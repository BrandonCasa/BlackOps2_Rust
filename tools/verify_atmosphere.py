import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time
import traceback

from gameplay_remote import Client, write_report
from verify_brp import assets_match, digest, require
from verify_lantern_shadows import wait_for_slot
from verify_weapon_materials import capture, inputs


def observe(client, predicate=lambda value: True, timeout=180):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        result = client.call('origins/materials')
        require(not result['pipelines']['error'], f"Pipeline failed: {result['pipelines']['error']}")
        last = result['atmosphere']
        if last['shader_ready'] and last['lut_ready'] and predicate(last):
            return last
        time.sleep(.05)
    raise TimeoutError(f'Atmosphere did not settle: {last}')


def close(actual, expected):
    return len(actual) == len(expected) and all(math.isclose(a, b, abs_tol=2e-4, rel_tol=1e-5)
                                               for a, b in zip(actual, expected))


def volume_at(source, point, bank):
    return next((volume for volume in source['volumes'] if volume['bank'] & bank
                 and all(sum(a * b for a, b in zip(plane[:3], point)) + plane[3] <= 0
                         for plane in volume['planes'])), None)


def assert_fog(source, observed):
    camera = next(camera for camera in observed['cameras'] if camera['world'])
    volume = volume_at(source, camera['position'], observed['bank'])
    require(observed['volume'] == (volume['order'] if volume else None), 'Original fog volume order changed')
    target = volume['fog'] if volume else source['fog']
    require(close(observed['fog']['distance'], target['distance']), 'Original fog distances changed')
    require(close(observed['fog']['color'], target['color']), 'Original fog color changed')
    require(math.isclose(observed['fog']['opacity'], target['opacity'], abs_tol=2e-4), 'Original fog opacity changed')
    require(camera['fog'] and camera['tonemapping'] == 'None' and camera['sky']['loaded'],
            'World HDR, fog, or sky ownership changed')
    final = next(camera for camera in observed['cameras'] if camera['viewmodel'])
    require(not final['fog'] and final['tonemapping'] == 'TonyMcMapface', 'Final composite tonemapping changed')


def settled(value):
    return value['fog_elapsed'] >= value['fog_transition_seconds']


def run(client, output, report, asset_root, source):
    client.until(lambda state: state['state'] == 'Playing' and state.get('player'), timeout=360)
    report['discovery'] = client.call('rpc.discover')
    report['schema'] = client.call('origins/schema')
    game = json.loads((asset_root / 'game.json').read_bytes())
    client.reset(timeout=90)
    client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300})
    initial = observe(client, settled)
    require(initial['fog_volume_count'] == len(source['volumes']) == 28, 'Missing original fog volumes')
    require(close(initial['sun'][0]['color_linear'][:3], source['sun_color']), 'Clear sunlight differs from source')
    assert_fog(source, initial)
    report['initial'] = initial
    before = client.state()['player']['position']
    inputs(client, [{'tick': 1, 'key': 'KeyW', 'pressed': True},
                    {'tick': 31, 'key': 'KeyW', 'pressed': False}], 60)
    after = client.state()['player']['position']
    require(math.dist(before, after) > .1, 'Normal player movement failed')
    report['movement'] = {'before': before, 'after': after}
    points = [next(camera['position'] for camera in initial['cameras'] if camera['world']),
              *[[point[0], point[1] + 1.5, point[2]] for point in game['survey_views'][:2]]]
    report['weather_views'] = []
    for kind, bank in (('clear', 1), ('rain', 2), ('snow', 4)):
        client.call('origins/environment/setup', {'weather': kind, 'intensity': 4})
        for index, point in enumerate(points):
            client.call('origins/rendering', {'view': [*point, point[0], point[1] + .2, point[2] - 5]})
            value = observe(client, lambda value: value['bank'] == bank and settled(value)
                            and math.isclose(value['vision_blend'], 0 if bank == 1 else 1, abs_tol=1e-5))
            assert_fog(source, value)
            expected = source['snow_sun_color'] if bank == 4 else source['sun_color']
            value = observe(client, lambda value: close(value['sun'][0]['color_linear'][:3], expected))
            report['weather_views'].append({'weather': kind, 'view': index, 'observed': value,
                'capture': capture(client, output / f'{kind}-{index}.png')})
    client.call('origins/rendering', {'restore_player_view': True})
    client.reset(timeout=90)
    client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300})
    client.call('origins/environment/setup', {'weather': 'clear', 'overrides': {'3': 'snow', '4': 'clear'}})
    state = client.state()
    client.call('origins/setup', {'round_event': {'session_id': state['session_id'], 'round': 3}})
    client.until(lambda state: client.call('origins/environment')['weather']['presented'] == 'snow')
    partial = observe(client, lambda value: 0.01 < value['sun'][0]['color_linear'][0] - source['sun_color'][0]
                      < (source['snow_sun_color'][0] - source['sun_color'][0]) * .7)
    inputs(client, [{'tick': 1, 'key': 'Escape', 'pressed': True},
                    {'tick': 2, 'key': 'Escape', 'pressed': False}], 3)
    client.until(lambda state: state['state'] == 'Paused')
    paused = observe(client)
    client.wait_ticks(30)
    frozen = observe(client)
    require(paused['fog'] == frozen['fog'] and paused['vision_blend'] == frozen['vision_blend']
            and paused['sun'] == frozen['sun'], 'Paused atmosphere continued changing')
    inputs(client, [{'tick': 1, 'key': 'Escape', 'pressed': True},
                    {'tick': 2, 'key': 'Escape', 'pressed': False}], 3)
    client.until(lambda state: state['state'] == 'Playing')
    state = client.state()
    client.call('origins/setup', {'round_event': {'session_id': state['session_id'], 'round': 4}})
    client.until(lambda state: client.call('origins/environment')['weather']['presented'] == 'clear')
    restored = observe(client, lambda value: close(value['sun'][0]['color_linear'][:3], source['sun_color'])
                       and value['bank'] == 1 and value['vision_blend'] == 0 and settled(value))
    report['interrupted_transition'] = {'partial': partial, 'paused': paused, 'restored': restored}
    client.reset(timeout=90)
    reset = observe(client, lambda value: value['bank'] == 1 and value['vision_blend'] == 0 and settled(value))
    require(close(reset['sun'][0]['color_linear'][:3], source['sun_color']), 'Session retained old sunlight')
    report['reset'] = reset


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bundle', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--port', type=int, default=15719)
    args = parser.parse_args()
    bundle, output = args.bundle.resolve(), args.output.resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parents[1]), 'Evidence must be external')
    output.mkdir(parents=True, exist_ok=False)
    identity = json.loads((bundle / 'identity.json').read_bytes())
    for name, expected in identity['binaries'].items():
        require(digest(bundle / name) == expected, f'Pinned binary changed: {name}')
    source_path = Path(__file__).resolve().parents[1] / 'config/origins-atmosphere.json'
    require(identity['source']['files'].get('config/origins-atmosphere.json') == digest(source_path),
            'Atmosphere config differs from the compiled snapshot')
    report = {'passed': False, 'identity_sha256': digest(bundle / 'identity.json'),
              'script_sha256': digest(Path(__file__)), 'assets_before': assets_match(identity),
              'limitations': ['Camera placement and initial weather are scenario setup.',
                              'Round messages exercise weather presentation; combat round completion is not covered.',
                              'Legacy film response and native photometry require visual acceptance.']}
    client = Client(f'http://127.0.0.1:{args.port}', request_timeout=30, shutdown_timeout=60)
    wait_for_slot()
    with (output / 'game.log').open('w', encoding='utf-8') as log:
        process = subprocess.Popen([str(bundle / 'origins.exe'), '--validation-remote', str(args.port)],
            cwd=bundle, stdout=log, stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW,
            env={**os.environ, 'WGPU_BACKEND': 'dx12', 'RUST_LOG': 'info,wgpu=error,naga=warn', 'NO_COLOR': '1'})
        report['pid'] = process.pid
        try:
            run(client, output, report, Path(identity['asset_root']), json.loads(source_path.read_bytes()))
            report['assets_after'] = assets_match(identity)
            report['passed'] = True
        except Exception as error:
            report['error'] = f'{type(error).__name__}: {error}'
            report['traceback'] = traceback.format_exc()
        finally:
            client.shutdown(report)
            path = write_report(output, 'atmosphere.json', report)
        process.wait(timeout=20)
    print(json.dumps({'passed': report['passed'], 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
