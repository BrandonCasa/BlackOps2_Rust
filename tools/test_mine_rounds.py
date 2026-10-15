from test_support import asset_bytes
import argparse
import json
import math
from pathlib import Path

from test_support import Client, write_report
from test_max_ammo_mines import prepare
from test_throwable_down import binding, check, edge, schedule


def place(client, stock):
    before = client.state()
    key = binding(before, 'mine')
    schedule(client, [edge(1, key, True), edge(2, key, False)])
    return client.until(lambda s: s['equipment']['mine_placed'] == before['equipment']['mine_placed'] + 1
                        and s['player']['mine']['count'] == stock - 1
                        and not s['player']['mine']['busy'])


def run(client, report, game):
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
    retired = client.state()['session_id']
    profile = next(w['mine'] for w in game['content']['weapons'] if w['id'] == 'weapon.claymore_zm')
    stock = profile['stock']
    before, fixture = prepare(client, stock)
    check(report, 'native_placement_spends_mine_in_new_session',
          before['session_id'] > retired and before['player']['mine']['count'] == stock - 1
          and before['player']['life'] == 'Standing', fixture=fixture, after=before)
    injected = {'round_event': {'session_id': retired, 'round': 2}}
    client.call('origins/setup', injected)
    after = client.wait_ticks(3)
    check(report, 'retired_round_event_cannot_refill_current_mines',
          after['player']['mine']['count'] == before['player']['mine']['count']
          and after['equipment']['round_awards'] == before['equipment']['round_awards']
          and after['session']['round'] == before['session']['round'],
          pipeline_setup=injected, before=before, after=after)
    injected['round_event']['session_id'] = after['session_id']
    client.call('origins/setup', injected)
    current = client.wait_ticks(3)
    check(report, 'current_round_event_refills_mines',
          current['player']['mine']['count'] == stock
          and current['equipment']['round_awards'] == after['equipment']['round_awards'] + 1,
          pipeline_setup=injected, before=after, after=current)
    spent = place(client, stock)
    client.call('origins/setup', {'clear_enemies': True, 'pending': 0})
    advanced = client.until(lambda s: s['session']['round'] == spent['session']['round'] + 1,
                            timeout=game['rounds']['between_round_seconds'] + 30)
    check(report, 'real_round_transition_refills_native_spent_mine',
          advanced['player']['mine']['count'] == stock
          and advanced['equipment']['round_awards'] == spent['equipment']['round_awards'] + 1
          and advanced['session']['pending'] > 0
          and advanced['round_evidence']['transitions'] == spent['round_evidence']['transitions'] + 1,
          before=spent, after=advanced)
    before, fixture = prepare(client, stock)
    client.call('origins/setup', {'pitch': -1.5})
    frag = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == 'weapon.frag_grenade_zm')
    key = binding(before, 'lethal')
    end = math.ceil((frag['prime_seconds'] + frag['fuse_seconds'] + 0.7) / before['fixed_delta_seconds'])
    events = [edge(1, key, True), edge(end, key, False)]
    downed = schedule(client, events)
    check(report, 'native_cooked_frag_down_keeps_spent_mine',
          downed['player']['life'] == 'Downed' and downed['player']['mine']['count'] == stock - 1,
          fixture=fixture, inputs=events, before=before, after=downed)
    injected = {'round_event': {'session_id': retired, 'round': 2}}
    client.call('origins/setup', injected)
    rejected = client.wait_ticks(3)
    check(report, 'retired_round_cannot_refill_downed_mines',
          rejected['player']['life'] == 'Downed'
          and rejected['player']['mine']['count'] == stock - 1, pipeline_setup=injected, after=rejected)
    injected['round_event']['session_id'] = rejected['session_id']
    client.call('origins/setup', injected)
    refilled = client.wait_ticks(3)
    check(report, 'native_current_round_refills_downed_mines_but_not_grenades',
          refilled['player']['life'] == 'Downed' and refilled['player']['mine']['count'] == stock
          and refilled['player']['lethal']['count'] == rejected['player']['lethal']['count']
          and refilled['equipment']['round_awards'] == rejected['equipment']['round_awards']
          and not refilled['held_inputs'] and not refilled['queued_inputs'],
          pipeline_setup=injected, before=rejected, after=refilled)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'claymore_round_session_isolation',
              'limitations': ['Perk/equipment grants and empty intermission conditions are scenario setup.',
                              'Placement and self-down use native scheduled input; inventory changes are observed.',
                              'Injected round messages check session isolation, not player-driven round completion.',
                              'The empty round transition exercises the real round scheduler and refill consumers.']}
    try:
        report['schema'] = client.call('origins/schema')
        path = Path(__file__).resolve().parent.parent / 'target/assets/game.json'
        run(client, report, json.loads(asset_bytes(path)))
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'mine-rounds.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
