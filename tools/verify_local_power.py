import argparse
import hashlib
import json
from pathlib import Path

from gameplay_remote import Client, write_report
from verify_generator_population import capture_generator
from verify_perk_limit import approach, interact


def check(report, name, passed, **evidence):
    report['cases'].append({'case': name, 'passed': bool(passed), **evidence})
    if not passed:
        raise RuntimeError(name)


def attempt(client, content, perk_id):
    perk = next(p for p in content['perks'] if p['id'] == perk_id)
    before = approach(client, content, perk)
    interact(client, before)
    after = client.until(lambda s: s['focus']['rejection'] is not None
                         or s['focus']['accepted'] > before['focus']['accepted'])
    client.release()
    return before, after, perk


def denied(client, report, content, perk_id, name):
    before, after, _ = attempt(client, content, perk_id)
    check(report, name, after['focus']['rejection'] == 'NoPower'
          and after['session']['points'] == before['session']['points']
          and after['player']['perks'] == before['player']['perks']
          and not after['player']['drinking'], before=before, after=after)


def run(client, report, game):
    content = game['content']
    connections = [d['connection'] for d in game['access']['doors']]
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=120)
    client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'points': 10000, 'pause_spawning_seconds': 300, 'open_connections': connections})
    for perk in ('quick_revive', 'speed_cola', 'juggernog', 'stamin_up'):
        denied(client, report, content, 'perk.' + perk, perk + '_requires_local_power')
    home = client.reset()['player']['position']
    client.call('origins/setup', {'clear_enemies': True, 'points': 10000, 'pause_spawning_seconds': 300, 'open_connections': connections})
    before, after, perk = attempt(client, content, 'perk.mule_kick')
    client.call('origins/setup', {'position': home})
    after = client.until(lambda s: perk['id'] in s['player']['perks'] and not s['player']['drinking'])
    check(report, 'mule_kick_purchase_completes_without_power', after['session']['points'] == before['session']['points'] - perk['price']
          and after['focus']['accepted'] == before['focus']['accepted'] + 1, after=after)
    client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'points': 10000, 'pause_spawning_seconds': 300, 'open_connections': connections})
    capture_generator(client, 2)
    denied(client, report, content, 'perk.quick_revive', 'unrelated_generator_does_not_power_machine')
    powered = capture_generator(client, 1)
    check(report, 'native_capture_enables_local_power',
          next(g for g in powered['generators']['sites'] if g['number'] == 1)['state'] == 'Active', after=powered)
    before, after, perk = attempt(client, content, 'perk.quick_revive')
    after = client.until(lambda s: perk['id'] in s['player']['perks'] and not s['player']['drinking'])
    check(report, 'powered_purchase_grants_perk_once', after['session']['points'] == before['session']['points'] - perk['price']
          and after['focus']['accepted'] == before['focus']['accepted'] + 1
          and after['player']['solo_charges'] == 1, after=after)
    restarted = client.reset()
    check(report, 'restart_clears_local_power', all(g['state'] == 'Inactive' for g in restarted['generators']['sites']))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--identity', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'local_generator_power',
              'limitations': ['Connections opened and defenders removed as setup to isolate power and purchases; capture and purchase use native input.',
                              'Generator shutdown and attack waves remain unfinished.']}
    try:
        report['identity'] = {'path': str(args.identity.resolve()), 'sha256': hashlib.sha256(args.identity.read_bytes()).hexdigest()}
        game = Path(__file__).resolve().parent.parent / 'assets/origins/game.json'
        content = json.loads(game.read_bytes())
        report['content_sha256'] = hashlib.sha256(game.read_bytes()).hexdigest()
        run(client, report, content)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        write_report(args.output, 'local-power.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
