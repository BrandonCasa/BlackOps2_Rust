import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import struct


NATIVE = {
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


def native_tuning(research):
    text = (research / 'exports/functions/008a/008ad170.c').read_text()
    values = {key: float_default(text, name) * scale for key, (name, scale) in NATIVE.items()}
    values['dive_apex_seconds'] = int(re.search(r'FUN_00757cf0\("dtp_max_apex_duration",([0-9]+),', text)[1]) * .001
    values['dive_jump_height'] = jump_height(research)
    return values


def native_physics(research):
    text = (research / 'exports/functions/008b/008b64d0.c').read_text()
    gravity = float_default(text, 'bg_gravity') * .0254
    return {'gravity': gravity, 'jump_speed': math.sqrt(2 * jump_height(research) * gravity)}


def native_perks(research):
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--research', type=Path, required=True)
    parser.add_argument('--pack', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    args = parser.parse_args()
    values = native_tuning(args.research)
    path = args.pack / 'game.json'
    game = json.loads(path.read_bytes())
    game['tuning'].setdefault('mobility', {}).update(values)
    game['tuning'].update(native_physics(args.research))
    path.write_text(json.dumps(game, separators=(',', ':')) + '\n', encoding='utf-8')
    entries = ('008a/008ad170', '008a/008ada20', '008a/008adac0', '008a/008ad690',
               '008a/008ad7f0', '008a/008ad8b0', '008b/008be7e0', '008b/008ba200',
               '008b/008b64d0', '008a/008ad570', '008a/008ad770', '008a/008ad4a0',
               '008b/008b3670', '008b/008bf460', '005d/005d2e00', '008b/008bef20',
               '008b/008bea40', '008c/008c2760', '008c/008c2a70', '008c/008c6170',
               '008c/008c64b0', '008c/008c9d80', '008b/008bd5b0', '008b/008bd520')
    evidence = {}
    for entry in entries:
        source = args.research / ('exports/functions/' + entry + '.c')
        evidence[entry.rsplit('/', 1)[1]] = {
            'path': str(source.resolve()), 'sha256': hashlib.sha256(source.read_bytes()).hexdigest()}
    receipt = {'native_tuning': values, 'physics': native_physics(args.research),
               'native_perks': native_perks(args.research), 'sources': evidence,
               'native_image_sha256': hashlib.sha256((args.research / 'input/t6zm.unpacked.exe').read_bytes()).hexdigest(),
               'game_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
               'limitations': ['Avian capsule contacts and stair handling reconstruct the original collision solver.',
                               'Native rules are sampled on the Bevy fixed timestep.',
                               'Imported slick flags and landing recovery are exercised by the separate final landing scenario.']}
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'native_tuning': values, 'receipt': str(args.receipt)}))


if __name__ == '__main__':
    main()
