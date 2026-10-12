import argparse
import hashlib
import json
from pathlib import Path
import time

from gameplay_remote import Client, write_report
from verify_brp import assets_match
from verify_throwable_down import binding, check, edge


def prepare(client, weapon=None):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'owned_perks': [], 'pitch': -1.5}
    if weapon:
        fixture['weapons'] = [weapon]
    client.call('origins/setup', fixture)
    return client.until(lambda s: s['player']['action'] == 'Ready'), fixture


def fire_floor(client, report, weapon, expected_damage):
    before, fixture = prepare(client, weapon)
    fire = binding(before, 'fire')
    inputs = [edge(1, fire, True), edge(2, fire, False)]
    batch = client.call('origins/input', {'events': inputs})
    after = client.until(lambda s: s['fixed_tick'] >= batch['finish_tick']
                         and s['equipment']['self_impacts'] > before['equipment']['self_impacts'], timeout=15)
    receipts = [r for r in after['damage']['recent']
                if r['target'] == after['player']['entity']]
    loss = before['player']['health'] - after['player']['health']
    passed = (0 < loss <= expected_damage if expected_damage else loss == 0)
    check(report, weapon + '_native_player_splash',
          passed and after['player']['life'] == 'Standing'
          and after['session']['shots'] > before['session']['shots'],
          fixture=fixture, inputs=inputs, before=before, after=after, receipts=receipts)


def run(client, report, output):
    ready = client.until(lambda s: s['state'] == 'LoadError' or s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=240)
    if ready['state'] == 'LoadError':
        raise RuntimeError('Game asset loading failed')
    report['initial_state'] = ready
    report['methods'] = client.call('rpc.discover')
    report['schema'] = client.call('origins/schema')
    fire_floor(client, report, 'weapon.ray_gun_zm', 30)
    fire_floor(client, report, 'weapon.staff_fire_zm', 0)
    after = client.state()
    check(report, 'all_scheduled_inputs_released', not after['held_inputs'] and not after['queued_inputs'])
    client.call('origins/setup', {'pitch': 0})
    if output is None:
        return
    screenshot = output.resolve() / 'phase-fixes.png'
    screenshot.parent.mkdir(parents=True, exist_ok=True)
    client.call('origins/capture', {'path': str(screenshot)})
    deadline = time.monotonic() + 30
    while not screenshot.exists() and time.monotonic() < deadline:
        time.sleep(0.025)
    check(report, 'rendered_gameplay_frame_saved', screenshot.exists(), path=str(screenshot))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--identity', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'phase_fixes_gameplay',
              'limitations': ['The optional random-perk powerup is absent from the native catalog; that route is source-reviewed.',
                              'Full quest completion and performance are outside this scenario.']}
    try:
        report['identity'] = {'path': str(args.identity.resolve()),
                              'sha256': hashlib.sha256(args.identity.read_bytes()).hexdigest()}
        report['script_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        report['assets'] = assets_match(json.loads(args.identity.read_bytes()))
        run(client, report, args.output)
        report['passed'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        client.shutdown(report)
        write_report(args.output, 'phase-fixes.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
