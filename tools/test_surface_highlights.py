import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import traceback

from test_support import Client, write_report
from test_atmosphere import observe, settled
from test_brp import require
from test_lantern_shadows import wait_for_slot
from test_weapon_materials import capture, wait_materials


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bundle', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--port', type=int, default=15725)
    args = parser.parse_args()
    bundle, output = args.bundle.resolve(), args.output.resolve()
    root = Path(__file__).resolve().parents[1]
    require(not output.is_relative_to(root), 'Evidence must stay external')
    output.mkdir(parents=True, exist_ok=False)
    report = {'passed': False, 'views': []}
    game = json.loads((Path(bundle / 'assets/origins') / 'game.json').read_bytes())
    atmosphere = json.loads((bundle / 'assets/origins' / json.loads((bundle / 'assets/origins/prepared/runtime.json').read_bytes())['worlds'][0]['atmosphere_uri']).read_bytes())
    yaw = math.radians(atmosphere['sun_angles'][1])
    direction = [math.cos(yaw), -math.sin(yaw)]
    points = [(1, game['survey_views'][1]), (2, game['survey_views'][6]),
              (3, game['environment']['digging']['mounds'][0]['position'])]
    client = Client(f'http://127.0.0.1:{args.port}', request_timeout=30, shutdown_timeout=60)
    wait_for_slot()
    with (output / 'game.log').open('w', encoding='utf-8') as log:
        process = subprocess.Popen([str(bundle / 'origins.exe'), '--validation-remote', str(args.port)],
            cwd=bundle, stdout=log, stderr=subprocess.STDOUT, creationflags=subprocess.CREATE_NO_WINDOW,
            env={**os.environ, 'WGPU_BACKEND': 'vulkan', 'RUST_LOG': 'info,wgpu=error,naga=warn', 'NO_COLOR': '1'})
        report['pid'] = process.pid
        try:
            client.until(lambda s: s['state'] == 'Playing' and s.get('player'), timeout=360)
            report['discovery'] = client.call('rpc.discover')
            report['schema'] = client.call('origins/schema')
            for weather in ('clear', 'rain'):
                client.reset(timeout=90)
                client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300})
                client.call('origins/environment/setup', {'weather': weather, 'intensity': 1})
                for index, point in points:
                    eye = [point[0], point[1] + 1.5, point[2]]
                    target = [eye[0] + direction[0] * 6, eye[1] - 2.5, eye[2] + direction[1] * 6]
                    view = [*eye, *target]
                    client.call('origins/rendering', {'view': view})
                    observed = observe(client, lambda v: settled(v) and v['vision_blend'] == (0 if weather == 'clear' else 1))
                    materials = wait_materials(client, first=False, timeout=90)
                    client.wait_ticks(45)
                    name = f'{weather}-{index}'
                    report['views'].append({'name': name, 'view': view, 'observed': observed,
                        'pending': materials['pending'], 'pipelines_pending': materials['pipelines']['pending'],
                        'capture': capture(client, output / f'{name}.png')})
                    print(f'Captured {name}', flush=True)
            client.call('origins/rendering', {'restore_player_view': True})
            client.reset(timeout=90)
            wait_materials(client, first=False, timeout=90)
            report['passed'] = True
        except Exception as error:
            report['error'] = f'{type(error).__name__}: {error}'
            report['traceback'] = traceback.format_exc()
        finally:
            client.shutdown(report)
            path = write_report(output, 'surface-highlights.json', report)
        process.wait(timeout=20)
    print(json.dumps({'passed': report['passed'], 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
