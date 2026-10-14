import math
import re
import struct
import http.client
import itertools
import json
from pathlib import Path
import socket
import time
import urllib.error
import urllib.parse
import urllib.request


def asset_bytes(path):
    path = Path(path)
    pack = next((parent for parent in path.parents if parent.suffix == '.pack'), None)
    if pack is None:
        return path.read_bytes()
    wanted = path.relative_to(pack).as_posix()
    with pack.open('rb') as stream:
        if stream.read(8) != b'ORIGPAK1':
            raise ValueError('Unsupported runtime pack')
        count_bytes = stream.read(4)
        if len(count_bytes) != 4:
            raise ValueError('Truncated runtime pack')
        count, = struct.unpack('<I', count_bytes)
        for _ in range(count):
            header = stream.read(12)
            if len(header) != 12:
                raise ValueError('Truncated runtime pack entry')
            name_size, size = struct.unpack('<IQ', header)
            if name_size > 4096 or size > pack.stat().st_size - stream.tell() - name_size:
                raise ValueError('Invalid runtime pack entry length')
            name = stream.read(name_size).decode('utf-8')
            if name == wanted:
                data = stream.read(size)
                if len(data) != size:
                    raise ValueError('Truncated runtime asset')
                return data
            stream.seek(size, 1)
    raise FileNotFoundError('Runtime pack is missing ' + wanted)


def initial_zones(game):
    content = game['content']
    enabled = {zone['id'] for zone in content['zones'] if zone['initially_open']}
    if not enabled:
        raise ValueError('Packed content has no initially open zone')
    while True:
        previous = enabled.copy()
        for connection in content['zone_connections']:
            endpoints = {connection['from'], connection['to']}
            if connection['initially_open'] and enabled.intersection(endpoints):
                enabled.update(endpoints)
        if enabled == previous:
            return enabled


class Client:
    def __init__(self, url="http://127.0.0.1:15702", request_timeout=5, shutdown_timeout=15):
        parsed = urllib.parse.urlparse(url)
        if parsed.scheme != "http" or parsed.hostname != "127.0.0.1":
            raise ValueError("Validation requires an HTTP loopback endpoint")
        self.url = url
        self.ids = itertools.count(1)
        self.request_timeout = request_timeout
        self.shutdown_timeout = shutdown_timeout

    def call(self, method, params=None):
        identity = next(self.ids)
        body = {"jsonrpc": "2.0", "id": identity, "method": method}
        if params is not None:
            body["params"] = params
        request = urllib.request.Request(
            self.url, json.dumps(body).encode(), {"Content-Type": "application/json"}
        )
        with urllib.request.urlopen(request, timeout=self.request_timeout) as response:
            message = json.load(response)
        if message.get("id") != identity:
            raise RuntimeError("Mismatched BRP response")
        if "error" in message:
            raise RuntimeError(f"{method}: {message['error']}")
        return message["result"]

    def state(self):
        return self.call("origins/state")

    def until(self, predicate, timeout=30):
        deadline = time.monotonic() + timeout
        last = None
        while time.monotonic() < deadline:
            try:
                last = self.state()
                if predicate(last):
                    return last
            except http.client.RemoteDisconnected:
                raise
            except (urllib.error.URLError, TimeoutError, ConnectionError) as error:
                last = str(error)
            time.sleep(0.025)
        raise TimeoutError(f"Gameplay condition timed out after {timeout}s: {last}")

    def wait_ticks(self, count, timeout=30):
        target = self.state()["fixed_tick"] + count
        return self.until(lambda state: state["fixed_tick"] >= target, timeout)

    def release(self):
        return self.call("origins/release")

    def quit(self, timeout=None):
        timeout = self.shutdown_timeout if timeout is None else timeout
        endpoint = urllib.parse.urlparse(self.url)
        address = (endpoint.hostname, endpoint.port or 80)
        with socket.create_connection(address, timeout=3):
            pass
        try:
            result = self.call("origins/quit")
        except (TimeoutError, ConnectionError, urllib.error.URLError, http.client.RemoteDisconnected) as error:
            result = {"exit_requested": None, "response_error": f"{type(error).__name__}: {error}"}
        else:
            if not result.get("exit_requested"):
                raise RuntimeError(f"Game did not accept shutdown: {result}")
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                with socket.create_connection(address, timeout=min(3, max(0.01, deadline - time.monotonic()))):
                    pass
            except ConnectionRefusedError:
                return {**result, "endpoint_closed": True}
            except (TimeoutError, ConnectionResetError):
                pass
            time.sleep(0.05)
        raise TimeoutError(f"BRP endpoint remained open after shutdown: {self.url}")

    def shutdown(self, report):
        try:
            report["shutdown"] = self.quit()
        except Exception as error:
            report["passed"] = False
            report["shutdown_error"] = f"{type(error).__name__}: {error}"
            report.setdefault("error", report["shutdown_error"])

    def reset(self, timeout=30, difficulty=None):
        before = (self.state().get("player") or {}).get("entity")
        self.release()
        if difficulty is not None and difficulty not in ("original", "easy"):
            raise ValueError("Difficulty must be original or easy")
        self.call("origins/reset", {"difficulty": difficulty} if difficulty is not None else None)
        return self.until(
            lambda state: state["state"] == "Playing"
            and state.get("player")
            and state["player"]["entity"] != before,
            timeout,
        )


def write_report(output, name, report):
    root = Path(__file__).resolve().parent.parent
    output = Path(output).resolve()
    if output.is_relative_to(root) or Path(name).name != name:
        raise ValueError("Reports must use an external directory and a plain filename")
    output.mkdir(parents=True, exist_ok=True)
    target = output / name
    target.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return target



MOBILITY_SOURCE_FIELDS = {
    'dive_startup': ('dtp_startup_delay', .001),
    'dive_minimum_speed': ('dtp_min_speed', .0254),
    'dive_exhaustion': ('dtp_exhaustion_window', .001),
    'dive_slide': ('dtp_max_slide_duration', .001),
    'dive_recovery': ('dtp_post_move_pause', .001),
    'dive_launch_multiplier': ('dtp_new_trajectory_multiplier', 1.),
    'dive_fall_minimum': ('dtp_fall_damage_min_height', .0254),
    'dive_fall_fatal': ('dtp_fall_damage_max_height', .0254),
}


def float_default(text, name):
    match = re.search(r'FUN_00757df0\("' + name + r'",(0x[0-9a-f]+),', text)
    if match is None:
        raise ValueError('Missing native mobility default: ' + name)
    return struct.unpack('<f', struct.pack('<I', int(match[1], 16)))[0]


def jump_height(research):
    text = (research / 'exports/functions/008b/008b3670.c').read_text()
    return float(re.search(r'return \(float10\)([0-9.]+);', text)[1]) * .0254


def source_mobility(research):
    text = (research / 'exports/functions/008a/008ad170.c').read_text()
    values = {key: float_default(text, name) * scale for key, (name, scale) in MOBILITY_SOURCE_FIELDS.items()}
    values['dive_apex_seconds'] = int(re.search(r'FUN_00757cf0\("dtp_max_apex_duration",([0-9]+),', text)[1]) * .001
    values['dive_jump_height'] = jump_height(research)
    return values


def source_gravity(research):
    text = (research / 'exports/functions/008b/008b64d0.c').read_text()
    gravity = float_default(text, 'bg_gravity') * .0254
    return {'gravity': gravity, 'jump_speed': math.sqrt(2 * jump_height(research) * gravity)}


def source_fall_perks(research):
    image = (research / 'input/t6zm.unpacked.exe').read_bytes()
    pe = struct.unpack_from('<I', image, 0x3c)[0]
    optional = pe + 24
    base = struct.unpack_from('<I', image, optional + 28)[0]
    first = optional + struct.unpack_from('<H', image, pe + 20)[0]
    sections = [struct.unpack_from('<4I', image, first + index * 40 + 8)
                for index in range(struct.unpack_from('<H', image, pe + 6)[0])]

    def offset(address):
        return next(raw + address - base - rva for _, rva, size, raw in sections
                    if base + rva <= address < base + rva + size)

    source = (research / 'exports/functions/008b/008bd5b0.c').read_text()
    table = int(re.search(r'PTR_s_\w+_([0-9a-f]{8})\)\[param_1\]', source)[1], 16)
    count_source = (research / 'exports/functions/008b/008bd520.c').read_text()
    count = int(re.search(r'uVar2 < (0x[0-9a-f]+)', count_source)[1], 16) // 4
    names = []
    for index in range(count):
        pointer = struct.unpack_from('<I', image, offset(table) + index * 4)[0]
        start = offset(pointer)
        names.append(image[start:image.index(b'\0', start)].decode('ascii'))
    if names[14] != 'specialty_fallheight' or names[25] != 'specialty_flakjacket':
        raise ValueError('Native fall perk identities changed')
    return names


def source_physics(research):
    def source(entry):
        return (research / ('exports/functions/' + entry + '.c')).read_text()
    dvars = source('008b/008b64d0')
    walk = int(re.search(r'FUN_00757cf0\("g_speed",(0x[0-9a-f]+),',
                         source('005f/005fced0'))[1], 16) * .0254
    ground = source('008c/008c64b0')
    for value in (9, 12, 19):
        if f'fVar12 = {value}.0;' not in ground:
            raise ValueError('Native stance acceleration changed')
    image = (research / 'input/t6zm.unpacked.exe').read_bytes()
    pe = struct.unpack_from('<I', image, 0x3c)[0]
    optional = pe + 24
    base = struct.unpack_from('<I', image, optional + 28)[0]
    sections = optional + struct.unpack_from('<H', image, pe + 20)[0]
    def scalar(address):
        for i in range(struct.unpack_from('<H', image, pe + 6)[0]):
            _, rva, size, offset = struct.unpack_from('<4I', image, sections + 40 * i + 8)
            if base + rva <= address < base + rva + size:
                return struct.unpack_from('<f', image, offset + address - base - rva)[0]
        raise ValueError('Native constant outside image')
    crouch, prone = scalar(0x00b15ce8), scalar(0x00b0f6f0)
    if not (.649 < crouch < .651 and .149 < prone < .151):
        raise ValueError('Native stance speed constants changed')
    steps = source('008c/008cb330')
    step_threshold = float(re.search(r'([0-9.]+) < ABS\(local_f4\)', steps)[1]) * .0254
    step_min = float(re.search(r'local_100\)\s*\*\s*0\.8\s*\+\s*([0-9.]+)', steps)[1])
    physics = {'ground_acceleration': 9., 'crouch_acceleration': 12., 'prone_acceleration': 19.,
               'air_acceleration': 1., 'friction': float_default(dvars, 'friction'),
               'stop_speed': float_default(dvars, 'stopspeed') * .0254,
               'strafe_scale': float_default(dvars, 'player_strafeSpeedScale'),
               'backward_scale': float_default(dvars, 'player_backSpeedScale'),
               'sprint_strafe_scale': float_default(dvars, 'player_sprintStrafeSpeedScale'),
               'ground_normal_min': struct.unpack('<f', struct.pack('<I', 0x3f333333))[0],
               'ground_probe': .0254, 'step_down': 9. * .0254, 'skin_width': .00635,
               'step_slowdown_height': step_threshold, 'step_speed_min': step_min}
    tuning = {'walk_speed': walk, 'sprint_speed': walk * float_default(dvars, 'player_sprintSpeedScale'),
              'crouch_speed': walk * crouch, 'step_height': 18. * .0254, 'physics': physics}
    mobility = {'prone_speed': walk * prone, 'prone_step': 10. * .0254}
    return tuning, mobility


def source_pistols(research):
    source = research / 'source/server-patch/maps/mp/zombies/_zm.gsc'
    return source_pistol_ranks(source.read_text())


def source_pistol_ranks(text):
    body = re.search(r'(?m)^last_stand_pistol_rank_init\(\)\s*\{(.*?)^\}', text, re.S)
    if body is None:
        raise ValueError('Missing native last-stand pistol initialization')
    ranks, replacement = [], None
    for line in body[1].splitlines():
        match = re.search(r'level\.pistol_values\[level\.pistol_values\.size\] = "([^"]+)";', line)
        if match:
            ranks.append({'weapon': 'weapon.' + match[1], 'rank': len(ranks)})
        if 'level.pistol_value_solo_replace_below = level.pistol_values.size - 1;' in line:
            replacement = len(ranks) - 1
    if replacement is None or replacement < 0 or replacement >= len(ranks) - 1:
        raise ValueError('Missing native solo pistol replacement threshold')
    if len({row['weapon'] for row in ranks}) != len(ranks):
        raise ValueError('Duplicate native last-stand pistol')
    return ranks, replacement


def external_output(path):
    path = Path(path).resolve()
    if path == Path(__file__).resolve().parents[1] or Path(__file__).resolve().parents[1] in path.parents:
        raise ValueError("Scenario output must remain external")
    return path


def info_string(path, expected):
    raw = path.read_bytes()
    tokens = raw.decode("utf-8-sig").rstrip("\r\n").split("\\")
    if tokens[0] != expected or len(tokens) % 2 != 1:
        raise ValueError(f"malformed {expected}: {path.name}")
    fields = {}
    for key, value in zip(tokens[1::2], tokens[2::2]):
        if not key or key in fields:
            raise ValueError(f"duplicate/empty source field: {path.name}/{key}")
        fields[key] = value
    return {'source': path.name, 'fields': fields}


def gltf_accessor(document, binary, index):
    import numpy as np
    item = document['accessors'][index]
    view = document['bufferViews'][item['bufferView']]
    dtype = np.dtype({5121: 'u1', 5123: '<u2', 5125: '<u4', 5126: '<f4'}[item['componentType']])
    width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[item['type']]
    return np.ndarray((item['count'], width), dtype, binary,
                      view.get('byteOffset', 0) + item.get('byteOffset', 0),
                      (view.get('byteStride', width * dtype.itemsize), dtype.itemsize)).copy()
