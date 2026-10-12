import argparse
import hashlib
import json
import math
from pathlib import Path

from test_support import Client, write_report
from test_throwable_down import check


def collect_nukes(client, count):
    before = client.state()
    position = list(before['player']['position'])
    position[1] -= before['player']['height'] / 2 - 0.25
    setup = {'pickups': [{'id': 'powerup.nuke', 'position': position} for _ in range(count)]}
    client.call('origins/setup', setup)
    after = client.until(lambda s: s['drops']['collected'] == before['drops']['collected'] + count
                         and s['session']['score'] == before['session']['score'] + 400 * count)
    return before, after, setup


def run(client, report):
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300}
    client.call('origins/setup', fixture)
    initial = client.wait_ticks(12)
    round_number = initial['session']['round']
    for expected in range(1, 6):
        before, armed, setup = collect_nukes(client, 16)
        armed = client.wait_ticks(2)
        check(report, f'native_nuke_payout_earns_drop_{expected}',
              not any(e.get('health') for e in armed['enemies'])
              and armed['session']['score'] == before['session']['score'] + 6400
              and armed['session']['points'] == before['session']['points'] + 6400
              and armed['session']['kills'] == before['session']['kills']
              and armed['drops']['spawned'] == min(expected - 1, 4),
              fixture=fixture, pickup_setup=setup, before=before, after=armed)
        player = armed['player']
        count = 1 if expected < 5 else 6
        positions = []
        for index in range(count):
            position = list(player['position'])
            distance = 5.5 + index * 0.4
            position[0] -= math.sin(player['yaw']) * distance
            position[2] -= math.cos(player['yaw']) * distance
            positions.append(position)
        enemy_setup = {'enemies': [{'position': p, 'health': 100} for p in positions]}
        targets = client.call('origins/setup', enemy_setup)['enemies']
        ready = client.until(lambda s: all(any(e['entity'] == target and e.get('health') == 100
                                               for e in s['enemies']) for target in targets))
        before, after, setup = collect_nukes(client, 1)
        after = client.wait_ticks(2)
        deaths = all(not any(e['entity'] == target and e.get('health') for e in after['enemies'])
                     for target in targets)
        check(report, f'native_nuke_kills_targets_{expected}',
              len(targets) == count and deaths and after['player']['life'] == 'Standing'
              and after['session']['shots'] == before['session']['shots']
              and after['session']['kills'] == before['session']['kills']
              and after['session']['points'] == before['session']['points'] + 400
              and after['round_evidence']['retired'] == ready['round_evidence']['retired']
              and after['session']['round'] == round_number,
              enemy_setup=enemy_setup, pickup_setup=setup, targets=targets, before=before, after=after)
        check(report, f'nuke_drop_{expected}' if expected < 5 else 'nuke_wave_respects_four_drop_round_cap',
              after['drops']['spawned'] == min(expected, 4), before=before, after=after)
    reset = client.reset()
    check(report, 'reset_clears_nuke_drop_counters_and_pickups',
          reset['session_id'] != after['session_id'] and reset['drops']['spawned'] == 0
          and reset['drops']['collected'] == 0 and not reset['drops']['active'], after=reset)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--identity', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'native_nuke_powerup_drops',
              'limitations': ['Enemies, Nuke pickups, and spawn delays are direct scenario setup.',
                              'Native proximity collection, Nuke waves, death messages, rewards, and drop spawning run normally.',
                              'The earned branch and four-drop round cap are covered; random-drop distribution is not measured.']}
    try:
        report['identity'] = {'path': str(args.identity.resolve()), 'sha256': hashlib.sha256(args.identity.read_bytes()).hexdigest()}
        report['script_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        report['helper_sha256'] = hashlib.sha256(Path(__file__).with_name('test_throwable_down.py').read_bytes()).hexdigest()
        report['schema'] = client.call('origins/schema')
        run(client, report)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'nuke-drops.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
