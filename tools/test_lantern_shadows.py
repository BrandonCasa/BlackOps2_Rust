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


def observation(client, timeout=90):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = client.call('origins/materials')
        require(not result['pipelines']['error'], f"Pipeline failed: {result['pipelines']['error']}")
        shadows = result['shadows']
        if result['pending'] == 0 and result['pipelines']['pending'] == 0 and shadows['lights']:
            cameras = shadows['cameras']
            require(len(cameras) == 1 and cameras[0]['world'] and cameras[0]['active'], 'Expected one active game camera')
            require(not cameras[0]['contact_shadows'], 'Raster camera contact shadows enabled')
            for light in shadows['lights']:
                fixture = light['fixture']
                require(fixture is not None, 'Light has no source fixture')
                require(math.isclose(light['radius'], fixture['base_radius'] * fixture['scale'], rel_tol=1e-5), 'Incorrect scaled light radius')
                require(math.isclose(light['near_z'], .01 * fixture['scale'], rel_tol=1e-5), 'Incorrect scaled near plane')
                require(not light['contact_shadows'] and not light['shadow_maps'], 'Raster local shadows enabled')
            surfaces = shadows['surfaces']
            require(all(s['excluded'] for s in surfaces if s['expected_excluded']), 'Glass exclusion missing')
            housing = [s for s in surfaces if s['material'] in {
                'mc/mtl_p6_zm_tm_portable_lantern_on', 'mc/mtl_p6_zm_tm_lantern_01',
                'mc/mtl_p6_zm_tm_gas_lamp_on', 'mc/mtl_p6_zm_tm_robot_detail'}]
            require(housing and all(not s['excluded'] for s in housing), 'Housing shadow caster missing')
            return result
        time.sleep(.2)
    raise TimeoutError('Shadow/material readiness did not settle')


def summary(result):
    shadows = result['shadows']
    return {'cameras': shadows['cameras'], 'lights': len(shadows['lights']),
            'active_shadow_maps': sum(light['shadow_maps'] for light in shadows['lights']),
            'excluded_surfaces': sum(s['excluded'] for s in shadows['surfaces']),
            'masked_casters': sum(s['alpha'] == 'mask' and not s['excluded'] for s in shadows['surfaces']),
            'pipelines': result['pipelines']}


def view(client, eye, target):
    client.call('origins/rendering', {'view': [*eye, *target]})


def press(client, key, duration=2, after=5):
    return inputs(client, [{'tick': 1, 'key': key, 'pressed': True},
                           {'tick': duration + 1, 'key': key, 'pressed': False}], duration + after)



def run(client, output, report, startup_only=False):
    started = time.monotonic()
    startup = report.setdefault('startup', {'samples': []})

    def ready(state):
        require(state['state'] != 'LoadError', f'Loading failed: {state}')
        if state['state'] == 'PreparingShaders':
            require(not state['enemies'], 'Enemies spawned during shader preparation')
            progress = client.call('origins/materials')['preparation']
            signature = (progress['total'], progress['ready'], progress['error'])
            if not startup['samples'] or signature != startup['samples'][-1]['signature']:
                startup['samples'].append({'signature': signature,
                                           'elapsed_seconds': time.monotonic() - started,
                                           'progress': progress})
            if 0 < progress['ready'] < progress['total'] and 'capture' not in startup:
                startup['capture'] = capture(client, output / 'shader-preparation.png')
        return state['state'] == 'Playing' and state.get('player')

    initial = client.until(ready, timeout=360)
    startup['elapsed_seconds'] = time.monotonic() - started
    materials = client.call('origins/materials')
    startup['final'] = materials['preparation']
    startup['preprocessing'] = materials['preprocessing']
    require(startup['preprocessing']['active_mode'] == 'gpu_culling', 'GPU culling is not active')
    require(startup['final']['total'] > 0
            and startup['final']['ready'] == startup['final']['total']
            and startup['final']['error'] is None, 'Gameplay began before shaders were ready')
    if startup_only:
        report['initial'] = observation(client)
        report['normal_play'] = capture(client, output / 'normal-play.png')
        client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300})
        report['movement_before'] = client.state()['player']
        press(client, 'KeyW', 30, 30)
        report['movement_after'] = client.state()['player']
        require(math.dist(report['movement_before']['position'], report['movement_after']['position']) > .05,
                'Normal movement did not advance the player')
        report['after_movement'] = observation(client)
        report['normal_play_after_movement'] = capture(client, output / 'normal-play-after-movement.png')
        report['settled_capture'] = capture(client, output / 'normal-play-settled.png')
        return
    report['discovery'] = client.call('rpc.discover')
    methods = {method['name'] for method in report['discovery']['methods']}
    require({'origins/rendering', 'origins/materials', 'origins/capture', 'origins/setup'} <= methods, 'Required BRP methods missing')
    report['schemas'] = client.call('registry.schema', {'with_crates': ['bevy_light', 'bevy_pbr', 'origins']})
    client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300})
    ready = observation(client)
    report['initial'] = ready
    origin = initial['player']['position']
    models = sorted({light['fixture']['model'] for light in ready['shadows']['lights']})
    require(len(models) == 5, f'Expected all five fixture models: {models}')
    samples = [(model, index) for model in models for index in range(2 if model == 'p6_zm_tm_portable_lantern_on' else 1)]
    for model, sample in samples:
        label = f'{model}-{sample}'
        print(f'Inspecting fixture: {label}', flush=True)
        client.reset(timeout=60)
        client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300})
        selected = sorted((light for light in ready['shadows']['lights'] if light['fixture']['model'] == model),
                          key=lambda light: math.dist(light['position'], origin))[sample]
        target = selected['position']
        direction = [origin[0] - target[0], origin[2] - target[2]]
        length = math.hypot(*direction)
        direction = [x / max(length, .001) for x in direction]
        distance = 1.25 if 'robot' not in model else 1.8
        eye = [target[0] + direction[0] * distance, target[1] + .25,
               target[2] + direction[1] * distance]
        view(client, eye, target)
        client.wait_ticks(120)
        observed = observation(client)
        light = next(light for light in observed['shadows']['lights'] if light['entity'] == selected['entity'])
        require(light['intensity'] > 0, f'Fixture light is inactive: {model}')
        case = {'model': model, 'sample': sample, 'light': light, 'eye': eye, 'target': target,
                'settled': capture(client, output / f'{label}-settled.png'), 'observations': summary(observed)}
        for index in range(12):
            offset = -.3 + index * .6 / 11
            moved = [eye[0] - direction[1] * offset, eye[1], eye[2] + direction[0] * offset]
            view(client, moved, target)
            client.wait_ticks(3)
            if index in (3, 8):
                case[f'moving_{index}'] = capture(client, output / f'{label}-moving-{index}.png')
        view(client, eye, target)
        report['fixtures'].append(case)
    client.call('origins/rendering', {'restore_player_view': True})
    client.reset(timeout=60)
    client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300})
    client.wait_ticks(90)
    report['normal_play'] = capture(client, output / 'normal-play.png')
    client.reset(timeout=60)
    client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300,
                                 'yaw': 0, 'pitch': 0,
                                 'enemies': [{'position': [origin[0], origin[1], origin[2] - 2], 'health': 100000}]})
    client.wait_ticks(30)
    report['actor'] = capture(client, output / 'moving-actor.png')
    report['actor_before'] = client.state()['enemies']
    client.wait_ticks(60)
    report['actor_after'] = client.state()['enemies']
    report['actor_moved'] = capture(client, output / 'moving-actor-after.png')
    report['final'] = summary(observation(client))
    report['structural_checks_passed'] = True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bundle', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--port', type=int, default=15714)
    parser.add_argument('--startup-only', action='store_true')
    args = parser.parse_args()
    bundle, output = args.bundle.resolve(), args.output.resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parent.parent), 'Evidence must be external')
    output.mkdir(parents=True, exist_ok=False)
    report = {'passed': False, 'structural_checks_passed': False, 'fixtures': [], 'startup_only': args.startup_only, 'wgpu_environment': {name: os.environ.get(name) for name in ('WGPU_VALIDATION', 'WGPU_GPU_BASED_VALIDATION', 'WGPU_VALIDATION_INDIRECT_CALL')}, 'limitations': ['Camera poses, weapon grants, and actor placement are scenario setup.', 'Structural assertions and captures require separate human visual acceptance.']}
    client = Client(f'http://127.0.0.1:{args.port}', request_timeout=30, shutdown_timeout=60)
    report['compiler_load'] = processes(('cargo', 'rustc'))
    wait_for_slot()
    with (output / 'game.log').open('w', encoding='utf-8') as log:
        process = subprocess.Popen([str(bundle / 'origins.exe'), '--validation-remote', str(args.port)],
                                   cwd=bundle, stdout=log, stderr=subprocess.STDOUT,
                                   env={**os.environ, 'WGPU_BACKEND': 'vulkan', 'RUST_LOG': 'info,wgpu=error,naga=warn', 'NO_COLOR': '1'},
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        report['pid'] = process.pid
        try:
            run(client, output, report, args.startup_only)
            rendering_log = (output / 'game.log').read_text(encoding='utf-8')
            require('backend: Vulkan' in rendering_log, 'Expected the Vulkan backend')
            report['renderer'] = {'backend': 'Vulkan', 'lighting': 'Solari'}
            report['passed'] = True
        except Exception as error:
            report['error'] = f'{type(error).__name__}: {error}'
            report['traceback'] = traceback.format_exc()
        finally:
            client.shutdown(report)
            path = write_report(output, 'shader-startup.json' if args.startup_only else 'lantern-shadows.json', report)
        process.wait(timeout=20)
    print(json.dumps({'passed': report['passed'], 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
