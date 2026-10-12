import argparse
import json
from pathlib import Path
import time

try:
    from .test_support import Client, write_report
except ImportError:
    from test_support import Client, write_report


def wait_materials(client, first=True, world=False, timeout=45, expected=None):
    deadline = time.monotonic() + timeout
    last = None
    stable = None
    while time.monotonic() < deadline:
        last = client.call('origins/materials')
        if last['pipelines']['error']:
            raise RuntimeError(f"Rendering pipeline failed: {last['pipelines']['error']}")
        seen = {r['material'] for r in last['first_person']}
        if last['pending'] == 0 and last['pipelines']['pending'] == 0 and (not first or last['first_person']) and (not world or last['world']) and (not expected or seen.intersection(expected)):
            signature = [(r['entity'],r['material']) for r in last['first_person'] + last['world']]
            if signature == stable:
                return last
            stable = signature
        else:
            stable = None
        time.sleep(0.1)
    raise TimeoutError(f'Material preparation timed out: {last}')


def capture(client, path):
    client.call('origins/capture', {'path': str(path.resolve())})
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if path.is_file() and path.stat().st_size > 100:
            encoded = path.read_bytes()
            if encoded.startswith(b'\x89PNG\r\n\x1a\n') and encoded.endswith(b'\x00\x00\x00\x00IEND\xaeB\x60\x82'):
                return {'path': str(path)}
        time.sleep(0.1)
    raise TimeoutError(f'Screenshot did not complete: {path}')


def inputs(client, events, ticks):
    client.call('origins/input', {'events': events})
    state = client.wait_ticks(ticks)
    client.release()
    return state


def run(client, game, asset_root, output, selected, report, yaw):
    client.until(lambda s: s['state'] == 'Playing' and s.get('player'), timeout=120)
    report['discovery'] = client.call('rpc.discover')
    report['schema'] = client.call('origins/schema')
    content = game['content']
    presentations = {p['id']: p for p in content['presentations']}
    catalog = [w for w in content['weapons'] if w.get('presentation') and presentations[w['presentation']].get('scene')]
    if selected:
        requested = set(selected)
        catalog = [w for w in catalog if w['id'] in requested]
        if {w['id'] for w in catalog} != requested:
            raise ValueError('Requested weapon has no native first-person scene')
    report['catalog'] = [w['id'] for w in catalog]
    for weapon in catalog:
        client.reset(timeout=60)
        client.call('origins/setup', {'weapons': [weapon['id']], 'clear_enemies': True, 'pending': 0,
                                     'pause_spawning_seconds': 300, 'yaw': yaw, 'pitch': 0})
        client.until(lambda s: s.get('player') and s['player']['weapons'][0]['id'] == weapon['id'], timeout=30)
        client.until(lambda s: s['player']['action'] == 'Ready', timeout=30)
        scene = presentations[weapon['presentation']]['scene'].split('#')[0]
        expected = {m['name'] for m in json.loads((asset_root / scene).read_bytes()).get('materials', [])}
        camo_path = asset_root / 'weapon_camos.json'
        overrides = json.loads(camo_path.read_bytes())['weapons'].get(weapon['id'], {})
        expected = {overrides.get(name, name) for name in expected}
        ready = wait_materials(client, expected=expected)
        case = {'weapon': weapon['id'], 'materials': {k: v for k, v in ready.items() if k != 'shadows'},
                'captures': {}, 'actions': {}}
        report['cases'].append(case)
        folder = output / weapon['id'].replace('weapon.', '')
        folder.mkdir(parents=True, exist_ok=True)
        case['captures']['hip'] = capture(client, folder / 'hip.png')
        events = [{'tick': 1, 'key': 'MouseRight', 'pressed': True}, {'tick': 100, 'key': 'MouseRight', 'pressed': False}]
        client.call('origins/input', {'events': events})
        aimed = client.until(lambda s: s['player']['ads'] >= 0.99, timeout=5) if weapon['mechanics']['ads_enabled'] else client.wait_ticks(20)
        case['actions']['ads'] = aimed['player']['ads']
        case['captures']['ads'] = capture(client, folder / 'ads.png')
        client.release()
        client.wait_ticks(20)
        client.call('origins/input', {'events': [{'tick': 1, 'key': 'KeyA', 'pressed': True}, {'tick': 100, 'key': 'KeyA', 'pressed': False}]})
        moved = client.wait_ticks(10)
        case['actions']['movement'] = moved
        case['captures']['movement'] = capture(client, folder / 'movement.png')
        client.release()
        before = client.state()
        fired = inputs(client, [{'tick': 1, 'key': 'MouseLeft', 'pressed': True}, {'tick': 10, 'key': 'MouseLeft', 'pressed': False}], 15)
        case['actions']['fire'] = {'before': before['player']['weapons'], 'after': fired['player']['weapons'], 'shots': fired['session']['shots'] - before['session']['shots']}
        client.call('origins/input', {'events': [{'tick': 1, 'key': 'KeyR', 'pressed': True}, {'tick': 2, 'key': 'KeyR', 'pressed': False}]})
        client.wait_ticks(15)
        case['actions']['reload'] = client.state()['player']['action']
        case['captures']['reload'] = capture(client, folder / 'reload.png')
        client.release()
        if weapon.get('pickup') and presentations[weapon['pickup']].get('scene'):
            client.call('origins/preview', {'weapon': weapon['id']})
            case['world_materials'] = {k: v for k, v in wait_materials(client, world=True).items()
                                       if k != 'shadows'}
            case['captures']['world'] = capture(client, folder / 'world.png')
            client.call('origins/preview', {})
        if weapon['id'].startswith('weapon.staff_') and 'upgraded' in weapon['id']:
            client.call('origins/preview', {'weapon': weapon['id']})
            wait_materials(client, world=True)
            client.call('origins/setup', {'charge_level': 3})
            client.wait_ticks(5)
            charged = wait_materials(client, world=True)
            first = [r for r in charged['first_person'] if r['kind'] == 7]
            world = [r for r in charged['world'] if r['kind'] == 7]
            if not first or not world or not all(r['charge'] == 3 and r['charge_owner'] is not None for r in first) or not all(r['charge'] == 0 and r['charge_owner'] is None for r in world):
                raise RuntimeError('Staff charge leaked between held and world materials')
            case['charge_isolation'] = {'first_person': first, 'world': world, 'passed': True}
            case['captures']['charged_setup'] = capture(client, folder / 'charged-setup.png')
            client.call('origins/setup', {'charge_level': 0})
            client.call('origins/preview', {})
        case['passed'] = True
        write_report(output, 'weapon-materials.json', report)
        print(json.dumps({'weapon': weapon['id'], 'passed': True}), flush=True)


def check_isolation(client, asset_root, output, report):
    base = 'weapon.c96_zm'
    upgraded = 'weapon.c96_upgraded_zm'
    replacements = json.loads((asset_root / 'weapon_camos.json').read_bytes())['weapons'][upgraded]
    assert len(set(replacements.values())) == 2
    cases = []
    report['camo_isolation'] = {'passed': False, 'cases': cases}
    for first, world in ((base, upgraded), (upgraded, base)):
        client.reset(timeout=60)
        client.call('origins/setup', {'weapons': [first], 'clear_enemies': True, 'pending': 0,
                                     'pause_spawning_seconds': 300, 'yaw': 1.5707963267948966, 'pitch': 0})
        client.until(lambda s: s['player']['weapons'][0]['id'] == first and s['player']['action'] == 'Ready')
        expected_first = set(replacements) if first == base else set(replacements.values())
        expected_world = set(replacements) if world == base else set(replacements.values())
        wait_materials(client, expected=expected_first)
        client.call('origins/preview', {'weapon': world})
        ready = wait_materials(client, world=True, expected=expected_first)
        for rows, expected, kind in ((ready['first_person'], expected_first, 1 if first == base else 10),
                                     (ready['world'], expected_world, 1 if world == base else 10)):
            found = {r['material'] for r in rows if r['material'] in expected and r['kind'] == kind}
            if found != expected:
                raise RuntimeError(f'Camo instance isolation failed: {first}, {world}, {found}, {expected}')
        shot = capture(client, output / f'isolation-{first.removeprefix("weapon.")}.png')
        cases.append({'first_person': first, 'world': world,
                      'materials': {k: v for k, v in ready.items() if k != 'shadows'}, 'capture': shot})
        client.call('origins/preview', {})
    report['camo_isolation']['passed'] = True


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    parser.add_argument('--weapon', action='append')
    parser.add_argument('--yaw', type=float, default=1.5707963267948966)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'limitations': ['Weapon grants and world previews are scenario setup.', 'Readiness and screenshot capture do not establish visual fidelity.', 'Charge-shot gameplay is currently unfinished; a firing request need not produce a shot.']}
    try:
        game = json.loads((Path(args.assets.resolve()) / 'game.json').read_bytes())
        report['view_yaw'] = args.yaw
        run(client, game, Path(args.assets.resolve()), args.output.resolve(), args.weapon, report, args.yaw)
        if not args.weapon:
            check_isolation(client, Path(args.assets.resolve()), args.output.resolve(), report)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'weapon-materials.json', report)
    print(json.dumps({'passed': report['passed'], 'cases': len(report['cases']), 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
