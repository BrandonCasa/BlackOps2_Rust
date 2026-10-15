from test_support import asset_bytes
import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time
import traceback

from test_support import Client, capture, wait_for_slot, write_report
from test_brp import require
from test_weapon_materials import inputs
from test_environment import ending


def observe(client, predicate=lambda value: True, timeout=180):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        require(client.state()['state'] in ('Playing', 'Paused'), 'Atmosphere scenario left gameplay')
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
    require(camera['fog'] and camera['tonemapping'] == 'TonyMcMapface' and camera['sky']['loaded'],
            'World HDR, fog, or sky ownership changed')
    require(observed['fog_exposure'] == source['exposure'], 'Original fog exposure changed')
    scale = 2 ** -source['exposure']
    require(close(camera['fog_color'][:3], [channel * scale for channel in target['color']]),
            'Fog bypassed original exposure')
    require(close(camera['sun_fog_color'][:3], [channel * scale for channel in target['sun_color']]),
            'Sun fog bypassed original exposure')
    require(len(observed['cameras']) == 1, 'Expected one shared game camera')


def settled(value):
    return value['fog_elapsed'] >= value['fog_transition_seconds']


def run(client, output, report, asset_root, source, skip_views=False):
    client.until(lambda state: state['state'] == 'Playing' and state.get('player'), timeout=360)
    report['discovery'] = client.call('rpc.discover')
    report['schema'] = client.call('origins/schema')
    game = json.loads(asset_bytes(asset_root / 'game.json'))
    client.reset(timeout=90)
    client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300})
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
              *[[point[0], point[1] + 1.5, point[2]] for point in
                [*game['survey_views'][:3], *game['survey_views'][6:8]]]]
    report['weather_views'] = []
    for kind, bank in (() if skip_views else (('clear', 1), ('rain', 2), ('snow', 4))):
        for index, point in enumerate(points):
            client.reset(timeout=90)
            client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300})
            client.call('origins/environment/setup', {'weather': kind, 'intensity': 4})
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
    client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300})
    client.call('origins/environment/setup', {'weather': 'clear', 'overrides': {'3': 'snow', '4': 'clear'}})
    clear = observe(client, settled)
    before, after = ending(client, game, 3, 'snow')
    report['snow_round_ending'] = {'before': before['environment']['weather'], 'after': after['environment']['weather']}
    client.call('origins/setup', {'pending': 1, 'pause_spawning_seconds': 300})
    client.until(lambda state: state['environment']['weather']['presented'] == 'snow')
    partial = observe(client, lambda value: 0.01 < value['sun'][0]['color_linear'][0] - source['sun_color'][0]
                      < (source['snow_sun_color'][0] - source['sun_color'][0]) * .7)
    camera = next(camera for camera in partial['cameras'] if camera['world'])
    volume = volume_at(source, camera['position'], partial['bank'])
    target = volume['fog'] if volume else source['fog']
    fraction = min(partial['fog_elapsed'] / partial['fog_transition_seconds'], 1)
    blend = fraction * fraction * (3 - 2 * fraction)
    require(0 < fraction < 1, 'Missed the fog transition')
    for field in ('color', 'distance'):
        expected = [a + (b - a) * blend for a, b in zip(clear['fog'][field], target[field])]
        require(close(partial['fog'][field], expected), 'Fog transition differs from native smoothstep')
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
    before, after = ending(client, game, 4, 'clear')
    report['clear_round_ending'] = {'before': before['environment']['weather'], 'after': after['environment']['weather']}
    client.call('origins/setup', {'pending': 1, 'pause_spawning_seconds': 300})
    client.until(lambda state: state['environment']['weather']['presented'] == 'clear')
    restored = observe(client, lambda value: close(value['sun'][0]['color_linear'][:3], source['sun_color'])
                       and value['bank'] == 1 and value['vision_blend'] == 0 and settled(value))
    report['interrupted_transition'] = {'partial': partial, 'paused': paused, 'restored': restored}
    client.reset(timeout=90)
    reset = observe(client, lambda value: value['bank'] == 1 and value['vision_blend'] == 0 and settled(value))
    require(close(reset['sun'][0]['color_linear'][:3], source['sun_color']), 'Session retained old sunlight')
    report['reset'] = reset
    report['rain_rooms'] = []
    mound = game['environment']['digging']['mounds'][0]['position']
    outdoor_position = [mound[0], mound[1] + .9, mound[2]]
    for name, position in (('indoor', game['spawn']), ('outdoor', outdoor_position)):
        client.reset(timeout=90)
        client.call('origins/setup', {'position': position, 'clear_enemies': True, 'pending': 1,
                                     'pause_spawning_seconds': 300,
                                     'open_connections': [door['connection'] for door in game['access']['doors']]})
        client.call('origins/environment/setup', {'weather': 'rain', 'intensity': 4})
        state = client.until(lambda state: state['environment']['weather']['precipitation_bursts'] >= 10
                             and state['environment']['weather']['ambient_alias'] == f'amb_weather_2d_rain_{name}')
        require(math.dist(state['player']['position'][::2], position[::2]) < 2,
                'Rain scenario left its authored test position')
        value = observe(client, settled)
        assert_fog(source, value)
        samples = []
        for _ in range(10):
            state = client.wait_ticks(6)
            player = state['player']['position']
            drops = state['environment']['precipitation']
            nearby = [drop for drop in drops if math.dist(drop['position'][::2], player[::2]) < 2
                      and drop['position'][1] <= player[1] + 2]
            samples.append({'player': player, 'drops': drops, 'nearby': len(nearby)})
        report['rain_rooms'].append({'room': name, 'player': state['player'], 'samples': samples,
            'weather': state['environment']['weather'], 'observed': value,
            'capture': capture(client, output / f'rain-{name}.png')})
    indoor, outdoor = report['rain_rooms']
    require(sum(sample['nearby'] for sample in indoor['samples']) == 0, 'Rain reached the sheltered player')
    require(any(sample['nearby'] for sample in outdoor['samples']), 'Outdoor rain failed to reach the player')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bundle', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--port', type=int, default=15719)
    parser.add_argument('--skip-views', action='store_true')
    args = parser.parse_args()
    bundle, output = args.bundle.resolve(), args.output.resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parents[1]), 'Evidence must be external')
    output.mkdir(parents=True, exist_ok=False)
    source_path = bundle / 'origins.pack' / json.loads(asset_bytes(bundle / 'origins.pack/prepared/runtime.json'))['worlds'][0]['atmosphere_uri']
    report = {'passed': False, 'skip_views': args.skip_views, 'limitations': ['Camera placement and initial weather are scenario setup.', 'Controlled final enemies are killed through normal melee input to exercise round-ending weather.', 'Legacy film response and native photometry require visual acceptance.']}
    client = Client(f'http://127.0.0.1:{args.port}', request_timeout=30, shutdown_timeout=60)
    wait_for_slot()
    with (output / 'game.log').open('w', encoding='utf-8') as log:
        process = subprocess.Popen([str(bundle / 'origins.exe'), '--validation-remote', str(args.port)],
            cwd=bundle, stdout=log, stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW,
            env={**os.environ, 'WGPU_BACKEND': 'vulkan', 'RUST_LOG': 'info,wgpu=error,naga=warn', 'NO_COLOR': '1'})
        report['pid'] = process.pid
        try:
            run(client, output, report, Path(bundle / 'origins.pack'), json.loads(asset_bytes(source_path)), args.skip_views)
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
