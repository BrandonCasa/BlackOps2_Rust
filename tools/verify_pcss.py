import argparse
import copy
import json
import math
import os
from pathlib import Path
import re
import subprocess
import time
import traceback

from gameplay_remote import Client, write_report
from verify_atmosphere import observe, settled
from verify_brp import assets_match, digest, require
from verify_lantern_shadows import wait_for_slot, processes, press
from verify_weapon_materials import capture, inputs, wait_materials


def diagnostic(client, output, target):
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        modes = re.findall(r'Material diagnostic: (\d+)', (output / 'game.log').read_text(encoding='utf-8'))
        mode = int(modes[-1]) if modes else 0
        if mode == target:
            wait_materials(client, first=False, timeout=180)
            client.wait_ticks(90, timeout=90)
            return mode
        press(client, 'F11', duration=8, after=8)
    raise TimeoutError(f'Material diagnostic did not reach {target}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--port', type=int, default=15727)
    args = parser.parse_args()
    bundle, output = args.bundle.resolve(), args.output.resolve()
    root = Path(__file__).resolve().parents[1]
    require(not output.is_relative_to(root), 'Evidence must stay external')
    output.mkdir(parents=True, exist_ok=False)
    identity = json.loads((bundle / 'identity.json').read_bytes())
    for name, expected in identity['binaries'].items():
        require(digest(bundle / name) == expected, f'Pinned binary changed: {name}')
    assets = copy.deepcopy(identity)
    pack = Path(assets['asset_root'])
    paths = {path.relative_to(pack).as_posix(): path for path in pack.rglob('*') if path.is_file()}
    changes = []
    for name in sorted(set(paths) | set(assets['assets'])):
        path, old = paths.get(name), assets['assets'].get(name)
        if path is None:
            changes.append({'path': name, 'before': old, 'after': None})
            del assets['assets'][name]
            continue
        stat = path.stat()
        if old and old['size'] == stat.st_size and old['mtime_ns'] == stat.st_mtime_ns:
            continue
        current = {'sha256': digest(path), 'size': stat.st_size, 'mtime_ns': stat.st_mtime_ns}
        assets['assets'][name] = current
        if old is None or old['sha256'] != current['sha256']:
            changes.append({'path': name, 'before': old, 'after': current})
    asset_manifest = output / 'runtime-assets.json'
    asset_manifest.write_text(json.dumps({'asset_root': str(pack), 'assets': assets['assets']}, indent=2) + '\n')
    report = {'passed': False, 'identity_sha256': digest(bundle / 'identity.json'),
              'bundle': str(bundle), 'source_revision': identity['source']['revision'],
              'source_snapshot': identity['source'], 'binaries': identity['binaries'],
              'script_sha256': digest(Path(__file__)), 'runtime_assets_sha256': digest(asset_manifest),
              'asset_changes_since_build': changes,
              'assets_before': {'mode': 'paths and size; SHA256 when mtime differs', 'files': len(paths)},
              'views': [],
              'limitations': ['Camera placement and weather are scenario setup.',
                              'Captures require visual review; development frame samples are not release benchmarks.']}
    game = json.loads((pack / 'game.json').read_bytes())
    atmosphere = json.loads((root / 'config/origins-atmosphere.json').read_bytes())
    yaw = math.radians(atmosphere['sun_angles'][1])
    points = [game['survey_views'][1], game['survey_views'][6],
              game['environment']['digging']['mounds'][0]['position']]
    client = Client(f'http://127.0.0.1:{args.port}', request_timeout=30, shutdown_timeout=60)
    wait_for_slot()
    report['compiler_load'] = processes(('cargo', 'rustc'))
    with (output / 'game.log').open('w', encoding='utf-8') as log:
        process = subprocess.Popen([str(bundle / 'origins.exe'), '--validation-remote', str(args.port)],
            cwd=bundle, stdout=log, stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW,
            env={**os.environ, 'WGPU_BACKEND': 'dx12', 'RUST_LOG': 'info,wgpu=error,naga=warn', 'NO_COLOR': '1'})
        report['pid'] = process.pid
        try:
            def ready(state):
                require(state['state'] != 'LoadError', f'Loading failed: {state}')
                return state['state'] == 'Playing' and state.get('player')

            client.until(ready, timeout=360)
            report['discovery'] = client.call('rpc.discover')
            report['game_schema'] = client.call('origins/schema')
            report['schema'] = client.call('registry.schema', {'with_crates': ['bevy_light']})
            for index, point in enumerate(points):
                client.reset(timeout=90)
                client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300})
                client.call('origins/environment/setup', {'weather': 'clear', 'intensity': 1})
                eye = [point[0], point[1] + 1.5, point[2]]
                target = [eye[0] + math.cos(yaw) * 6, eye[1] - 2.5, eye[2] - math.sin(yaw) * 6]
                view = [*eye, *target]
                client.call('origins/rendering', {'view': view})
                observed = observe(client, lambda value: settled(value) and value['vision_blend'] == 0)
                wait_materials(client, first=False, timeout=180)
                client.wait_ticks(120, timeout=120)
                case = {'index': index, 'view': view, 'sun': observed['sun'],
                        'normal': capture(client, output / f'normal-{index}.png')}
                report['views'].append(case)
                case['diagnostic_mode'] = diagnostic(client, output, 10)
                case['sun_shadow'] = capture(client, output / f'sun-shadow-{index}.png')
                diagnostic(client, output, 0)
                print(f'Captured PCSS view {index}', flush=True)
            client.call('origins/rendering', {'restore_player_view': True})
            client.reset(timeout=90)
            client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300})
            wait_materials(client, first=False, timeout=180)
            client.wait_ticks(120, timeout=120)
            report['moving_before'] = client.state()['player']['position']
            inputs(client, [{'tick': 1, 'key': 'KeyW', 'pressed': True},
                            {'tick': 91, 'key': 'KeyW', 'pressed': False}], 120)
            report['moving_after'] = client.state()['player']['position']
            require(math.dist(report['moving_before'], report['moving_after']) > .1, 'Player did not move')
            report['moving'] = capture(client, output / 'after-movement.png')
            report['materials'] = {key: value for key, value in client.call('origins/materials').items()
                                   if key in ('pipelines', 'preparation', 'preprocessing', 'shadows')}
            report['assets_after'] = assets_match(assets)
            report['passed'] = True
        except Exception as error:
            report['error'] = f'{type(error).__name__}: {error}'
            report['traceback'] = traceback.format_exc()
        finally:
            client.shutdown(report)
            path = write_report(output, 'pcss.json', report)
        process.wait(timeout=20)
    print(json.dumps({'passed': report['passed'], 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
