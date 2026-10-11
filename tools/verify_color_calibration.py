# /// script
# requires-python = ">=3.12"
# dependencies = ["pillow"]
# ///
import argparse
from functools import lru_cache
import itertools
import json
import math
import os
from pathlib import Path
import statistics
import subprocess
import traceback

from PIL import Image

from gameplay_remote import Client, write_report
from ktx2_textures import inspect_ktx2, payloads
from verify_atmosphere import observe, settled, run as atmosphere_run
from verify_brp import assets_match, digest, require
from verify_lantern_shadows import wait_for_slot
from verify_weapon_materials import capture, inputs, wait_materials


def film(base, vision):
    y = sum(a * b for a, b in zip(base, (.212585, .715195, .072220)))
    start, end = list(vision['vc_RS']), list(vision['vc_RE'])
    epsilon = 1 / 4096
    if start[0] >= end[0]: end[0] = start[0] + epsilon
    if start[1] >= end[1]: start[1] = end[1] - epsilon
    if start[2] >= end[2]: start[2] = end[2] - epsilon
    if end[2] >= start[3]: start[3] = end[2] + epsilon
    if start[3] >= end[3]: end[3] = start[3] + epsilon
    saturate = lambda x: min(1, max(0, x))
    weights = [saturate((y - end[0]) / (start[0] - end[0])),
               saturate((y - start[2]) / (end[2] - start[2])) * saturate((y - end[3]) / (start[3] - end[3])),
               saturate((y - start[1]) / (end[1] - start[1]))]
    weights = [x * x * (3 - 2 * x) for x in weights]
    total = sum(weights)
    weights = [x / total for x in weights]
    linear = [x * x for x in base]
    matrix = lambda row: sum(a * b for a, b in zip([*linear, 1], vision[row]))
    mapped = [saturate(sum(matrix('vc_' + band + 'M' + channel) * weight
                              for band, weight in zip('SMH', weights))) for channel in 'RGB']
    gamma = [x ** power for x, power in zip(mapped, vision['vc_FGM'])]
    saturation = vision['vc_FSM']
    luma = sum(x * y for x, y in zip(gamma, saturation[:3])) / sum(saturation[:3])
    saturated = [luma + saturation[3] * (x - luma) for x in gamma]
    return [math.sqrt(saturate(x + amount * (target - x)))
            for x, amount, target in zip(linear, vision['vc_FBM'], saturated)]


def reference(raw, source, blend):
    vision = {key: [a + blend * (b - a) for a, b in zip(value, source['vision']['_rain'][key])]
              for key, value in source['vision']['clear'].items()}

    @lru_cache(maxsize=None)
    def texel(x, y, z):
        offset = 4 * (y * 1024 + z * 32 + x)
        return film([v / 255 for v in raw[offset:offset + 3]], vision)

    def sample(stimulus):
        encoded = [math.sqrt(max(0, value)) for value in stimulus]
        softclip = source['softclip']
        pivot = 1 - softclip
        encoded = [x if x <= pivot else 1 - softclip * math.exp((pivot - x) / softclip) for x in encoded]
        coordinate = [min(1, max(0, value)) * 31 for value in encoded]
        low = [math.floor(value) for value in coordinate]
        fraction = [value - lo for value, lo in zip(coordinate, low)]
        result = [0., 0., 0.]
        for bits in itertools.product((0, 1), repeat=3):
            point = [min(31, lo + bit) for lo, bit in zip(low, bits)]
            weight = math.prod(f if bit else 1 - f for bit, f in zip(bits, fraction))
            for channel, value in enumerate(texel(*point)):
                result[channel] += weight * value
        return [value * 255 for value in result]
    return sample


def viewport(image):
    width, height = image.size
    if width / height > 16 / 9:
        w = round(height * 16 / 9)
        return ((width - w) // 2, 0, w, height)
    h = round(width * 9 / 16)
    return (0, (height - h) // 2, width, h)


def chart(client, output, report, source, raw, weather):
    client.reset(timeout=90)
    client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300})
    client.call('origins/environment/setup', {'weather': weather, 'intensity': 4})
    blend = 0 if weather == 'clear' else 1
    observed = observe(client, lambda value: value['vision_blend'] == blend and settled(value))
    colors = [[x, x, x] for x in (0, .0001, .001, .003, .01, .03, .09, .18, .36, .5625, .75, 1, 1.5, 2, 4, 16)]
    colors += [[.18, .01, .01], [.01, .18, .01], [.01, .01, .18], [.36, .09, .03],
               [.03, .09, .36], [.36, .36, .03], [.03, .36, .36], [.36, .03, .36],
               [1, .1, .01], [.01, .1, 1], [.1, 1, .01], [2, .18, .03],
               [.03, .18, 2], [.18, 2, .03], [4, .36, .09], [.09, .36, 4]]
    layout = client.call('origins/rendering', {'color_patches': colors, 'bloom': False, 'antialiasing': 'Smaa'})['color_patches']
    wait_materials(client, first=False, timeout=90)
    client.wait_ticks(12)
    path = output / f'chart-{weather}.png'
    receipt = capture(client, path)
    image = Image.open(path).convert('RGB')
    x, y, width, height = viewport(image)
    expected = reference(raw, source, blend)
    samples = []
    for patch in layout:
        px, py = round(x + patch['uv'][0] * width), round(y + patch['uv'][1] * height)
        actual = [statistics.median(image.getpixel((px + dx, py + dy))[channel]
                  for dx in range(-2, 3) for dy in range(-2, 3)) for channel in range(3)]
        target = expected(patch['rgb'])
        error = max(abs(a - b) for a, b in zip(actual, target))
        samples.append({'stimulus': patch['rgb'], 'pixel': [px, py], 'actual_8bit': actual,
                        'native_8bit': target, 'maximum_error_8bit': error})
    result = {'weather': weather, 'capture': receipt, 'observed': observed,
              'samples': samples, 'maximum_error_8bit': max(row['maximum_error_8bit'] for row in samples)}
    report.setdefault('charts', []).append(result)
    require(result['maximum_error_8bit'] <= 2.5, f'Native {weather} color response mismatch: {result["maximum_error_8bit"]}')
    for camera in observed['cameras']:
        require(not camera['auto_exposure'], 'Histogram exposure is still active')
        profile = json.loads((Path(__file__).resolve().parents[1] / 'config/origins-visual.json').read_bytes())
        scale = profile['sun_illuminance'] / (source['sun_brightness'] * math.pi)
        require(math.isclose(camera['exposure_multiplier'], 2 ** -source['exposure'] / scale, rel_tol=2e-6),
                'Camera bypassed native exposure / diffuse radiance conversion')
    for sample in observed['grading']['samples']:
        target = film(sample['base'], source['vision']['clear' if blend == 0 else '_rain'])
        require(max(abs(a - b) for a, b in zip(sample['film'], target)) < 2e-6, 'Runtime film controls differ from native equations')
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--port', type=int, default=15724)
    parser.add_argument('--charts-only', action='store_true')
    args = parser.parse_args()
    bundle, output = args.bundle.resolve(), args.output.resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parents[1]), 'Evidence must be external')
    output.mkdir(parents=True, exist_ok=False)
    identity = json.loads((bundle / 'identity.json').read_bytes())
    for name, expected in identity['binaries'].items():
        require(digest(bundle / name) == expected, f'Pinned binary changed: {name}')
    root = Path(__file__).resolve().parents[1]
    for name in ('config/origins-atmosphere.json', 'config/origins-visual.json', 'src/rendering/grading.rs',
                 'src/rendering/grading.wesl', 'src/rendering/atmosphere.rs', 'src/presentation.rs', 'src/verification/rendering.rs'):
        require(identity['source']['files'].get(name) == digest(root / name), f'Compiled calibration source changed: {name}')
    source = json.loads((root / 'config/origins-atmosphere.json').read_bytes())
    asset_root = Path(identity['asset_root'])
    encoded = (asset_root / 'sky/zm_tomb_grade.ktx2').read_bytes()
    raw = next(payloads(encoded, inspect_ktx2(encoded)))
    report = {'passed': False, 'identity_sha256': digest(bundle / 'identity.json'), 'script_sha256': digest(Path(__file__)),
              'assets_before': assets_match(identity), 'source': identity['source'],
              'limitations': ['The color chart compares the actual final GPU composite with the recovered native equations.',
                              'Lighting transport, ambient reconstruction, physical material BRDFs, and Bevy bloom are not original renderer pixel parity.',
                              'No matched captures from a running original game are available.']}
    client = Client(f'http://127.0.0.1:{args.port}', request_timeout=30, shutdown_timeout=60)
    wait_for_slot()
    with (output / 'game.log').open('w', encoding='utf-8') as log:
        process = subprocess.Popen([str(bundle / 'origins.exe'), '--validation-remote', str(args.port)],
            cwd=bundle, stdout=log, stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW,
            env={**os.environ, 'WGPU_BACKEND': 'dx12', 'RUST_LOG': 'info,wgpu=error,naga=warn', 'NO_COLOR': '1'})
        report['pid'] = process.pid
        try:
            client.until(lambda value: value['state'] == 'Playing' and value.get('player'), timeout=360)
            report['discovery'] = client.call('rpc.discover')
            report['schema'] = client.call('origins/schema')
            for weather in ('clear', 'rain', 'snow'):
                chart(client, output, report, source, raw, weather)
            client.call('origins/rendering', {'color_patches': [], 'bloom': True, 'antialiasing': 'Taa'})
            if not args.charts_only:
                atmosphere_run(client, output, report, asset_root, source)
            report['assets_after'] = assets_match(identity)
            report['passed'] = True
        except Exception as error:
            report['error'] = f'{type(error).__name__}: {error}'
            report['traceback'] = traceback.format_exc()
        finally:
            client.shutdown(report)
            path = write_report(output, 'color-calibration.json', report)
        process.wait(timeout=20)
    print(json.dumps({'passed': report['passed'], 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
