import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import time

from gameplay_remote import Client, write_report
from verify_brp import assets_match, digest, require
from verify_weapon_materials import capture, inputs


def processes(names):
    command = f"@(Get-Process -Name {','.join(names)} -ErrorAction SilentlyContinue | Select-Object ProcessName,Id) | ConvertTo-Json -Compress"
    value = subprocess.check_output(['powershell', '-NoProfile', '-NonInteractive', '-Command', command],
                                    text=True, creationflags=subprocess.CREATE_NO_WINDOW).strip()
    return json.loads(value) if value else []


def wait_for_slot():
    names = ('origins', 'origins_flythrough', 'capture', 'nsys', 'ncu', 'qrenderdoc', 'renderdoccmd',
             'tracy-capture', 'WinPixGpuCapturer', 'obs64', 'ffmpeg')
    while active := processes(names):
        print(f'Waiting for shared game/capture slot: {active}', flush=True)
        time.sleep(10)


def observation(client, timeout=90):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = client.call('origins/materials')
        require(not result['pipelines']['error'], f"Pipeline failed: {result['pipelines']['error']}")
        shadows = result['shadows']
        if result['pending'] == 0 and result['pipelines']['pending'] == 0 and shadows['lights']:
            cameras = shadows['cameras']
            require(any(c['world'] and c['active'] for c in cameras), 'No active world camera')
            require(any(c['viewmodel'] and c['active'] for c in cameras), 'No active viewmodel camera')
            require(all(not c['contact_shadows'] for c in cameras if not c['viewmodel']), 'World contact shadows enabled')
            require(all(c['contact_shadows'] for c in cameras if c['viewmodel']), 'Viewmodel contact shadows disabled')
            require(all(c['contact_settings']['steps'] == 16
                        and math.isclose(c['contact_settings']['length'], .3, rel_tol=1e-6)
                        and math.isclose(c['contact_settings']['thickness'], .1, rel_tol=1e-6)
                        for c in cameras if c['viewmodel']), 'Existing viewmodel contact settings changed')
            for light in shadows['lights']:
                fixture = light['fixture']
                require(fixture is not None, 'Light has no source fixture')
                require(math.isclose(light['radius'], fixture['base_radius'] * fixture['scale'], rel_tol=1e-5), 'Incorrect scaled light radius')
                require(math.isclose(light['near_z'], .01 * fixture['scale'], rel_tol=1e-5), 'Incorrect scaled near plane')
                require(light['contact_shadows'] and light['soft_shadows'], 'Shared viewmodel light settings lost')
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


def diagnostic(client, output, count, name):
    for _ in range(count):
        press(client, 'F11')
    client.wait_ticks(90)
    observed = observation(client)
    shot = capture(client, output / f'{name}.png')
    return {'capture': shot, 'observations': summary(observed)}


def run(client, output, report):
    initial = client.until(lambda s: s['state'] == 'Playing' and s.get('player'), timeout=180)
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
        require(light['shadow_maps'], f'Fixture shadow map was not admitted: {model}')
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
        case['local_shadow'] = diagnostic(client, output, 12, f'{label}-local-shadow')
        diagnostic(client, output, 1, f'{label}-restored')
        far = [target[0] + direction[0] * 20, eye[1], target[2] + direction[1] * 20]
        view(client, far, target)
        client.wait_ticks(60)
        distant = observation(client)
        require(not next(l for l in distant['shadows']['lights'] if l['entity'] == selected['entity'])['shadow_maps'],
                f'Fixture shadow map did not retire: {model}')
        view(client, eye, target)
        client.wait_ticks(60)
        require(next(l for l in observation(client)['shadows']['lights'] if l['entity'] == selected['entity'])['shadow_maps'],
                f'Fixture shadow map did not return: {model}')
        case['admission_retention_checked'] = True
        report['fixtures'].append(case)
    client.call('origins/rendering', {'restore_player_view': True})
    client.reset(timeout=60)
    client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300})
    client.wait_ticks(90)
    report['normal_play'] = capture(client, output / 'normal-play.png')
    for aa in ('Taa', 'Smaa', 'Msaa4'):
        print(f'Inspecting viewmodel contact shadows: {aa}', flush=True)
        client.reset(timeout=60)
        client.call('origins/setup', {'clear_enemies': True, 'pending': 0, 'pause_spawning_seconds': 300})
        client.call('origins/rendering', {'antialiasing': aa})
        client.wait_ticks(120)
        observed = observation(client)
        require(all(c['samples'] == (4 if aa == 'Msaa4' else 1) for c in observed['shadows']['cameras']), f'Wrong samples: {aa}')
        case = {'antialiasing': aa, 'observations': summary(observed),
                'contact_on': capture(client, output / f'viewmodel-{aa}-on.png')}
        client.call('origins/rendering', {'viewmodel_contact_shadows': False})
        client.wait_ticks(120)
        case['contact_off'] = capture(client, output / f'viewmodel-{aa}-off.png')
        client.call('origins/rendering', {'viewmodel_contact_shadows': True})
        client.wait_ticks(120)
        case['restored'] = capture(client, output / f'viewmodel-{aa}-restored.png')
        case['restored_observations'] = summary(observation(client))
        press(client, 'MouseRight', 30, 15)
        case['after_ads'] = capture(client, output / f'viewmodel-{aa}-after-ads.png')
        report['antialiasing'].append(case)
    client.call('origins/rendering', {'antialiasing': 'Taa'})
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
    args = parser.parse_args()
    bundle, output = args.bundle.resolve(), args.output.resolve()
    require(not output.is_relative_to(Path(__file__).resolve().parent.parent), 'Evidence must be external')
    output.mkdir(parents=True, exist_ok=False)
    identity = json.loads((bundle / 'identity.json').read_bytes())
    for name, expected in identity['binaries'].items():
        require(digest(bundle / name) == expected, f'Pinned binary changed: {name}')
    report = {'passed': False, 'structural_checks_passed': False, 'fixtures': [], 'antialiasing': [],
              'identity_sha256': digest(bundle / 'identity.json'), 'script_sha256': digest(Path(__file__)),
              'assets_before': assets_match(identity),
              'limitations': ['Camera poses, weapon grants, and actor placement are scenario setup.',
                              'Structural assertions and captures require separate human visual acceptance.']}
    client = Client(f'http://127.0.0.1:{args.port}')
    report['compiler_load'] = processes(('cargo', 'rustc'))
    wait_for_slot()
    with (output / 'game.log').open('w', encoding='utf-8') as log:
        process = subprocess.Popen([str(bundle / 'origins.exe'), '--validation-remote', str(args.port)],
                                   cwd=bundle, stdout=log, stderr=subprocess.STDOUT,
                                   env={**os.environ, 'RUST_LOG': 'info,wgpu=error,naga=warn', 'NO_COLOR': '1'},
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        report['pid'] = process.pid
        try:
            run(client, output, report)
            report['assets_after'] = assets_match(identity)
            report['passed'] = True
        except Exception as error:
            report['error'] = f'{type(error).__name__}: {error}'
        finally:
            client.shutdown(report)
            path = write_report(output, 'lantern-shadows.json', report)
        process.wait(timeout=20)
    print(json.dumps({'passed': report['passed'], 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
