import argparse
import json
import time
from pathlib import Path

from test_support import Client, write_report
from test_nuke_rewards import actor, aim, prepare
from test_throwable_down import binding, check, edge


def requested(state, fragment):
    return sum(count for alias, count in state['audio']['requests'].items() if fragment in alias)


def case(client, report, game, name, health, nuked, ads, insta_kill=True,
         double_points=False, region=None):
    before, target, fixture = prepare(client, health, nuked, insta_kill, double_points)
    aim_setup = aim(client, before, target, region)
    before = client.state()
    check(report, name + '_precondition',
          actor(before, target)['health'] == health and actor(before, target)['nuked'] == nuked
          and actor(before, target)['ready'] and before['player']['life'] == 'Standing'
          and before['player']['action'] == 'Ready'
          and (before['effects']['insta_kill'] > 0) == insta_kill
          and (before['effects']['double_points'] > 0) == double_points,
          fixture=fixture, before=before)
    shot = 30 if ads else 1
    events = [edge(shot, binding(before, 'fire'), True), edge(shot + 1, binding(before, 'fire'), False)]
    if ads:
        events += [edge(1, binding(before, 'aim'), True), edge(shot + 3, binding(before, 'aim'), False)]
    batch = client.call('origins/input', {'events': sorted(events, key=lambda e: e['tick'])})
    aim_samples = []
    deadline = time.monotonic() + 30
    while ads:
        current = client.state()
        if current['fixed_tick'] >= batch['start_tick'] + shot:
            break
        if time.monotonic() >= deadline:
            raise TimeoutError('Scheduled aiming did not reach the firing tick')
        if current['player']['ads'] >= 0.999:
            aim_samples.append({'fixed_tick': current['fixed_tick'], 'render_frame': current['render_frame'],
                                'player': current['player']['position'], 'target': actor(current, target),
                                'setup': aim(client, current, target, region)})
        time.sleep(0.01)
    after = client.until(lambda s: s['fixed_tick'] >= batch['finish_tick']
                         and not s['held_inputs'] and not s['queued_inputs'], timeout=45)
    after = client.until(lambda s: any(r['target'] == target and r['source'] == 'weapon.c96_zm'
                                      for r in s['damage']['recent'][len(before['damage']['recent']):]))
    receipts = [r for r in after['damage']['recent'][len(before['damage']['recent']):]
                if r['target'] == target and r['source'] == 'weapon.c96_zm']
    receipt = receipts[0]
    promoted = insta_kill and health > receipt['base_amount'] and (not nuked or ads)
    lethal = health <= receipt['base_amount'] or promoted
    hit, kill, head, torso, _ = game['tuning']['combat_points']
    bonus = head if receipt['region'] == 'Head' else (torso if receipt['region'] in ('Torso', 'UpperTorso') else 0)
    if nuked:
        expected = hit if ads and (not lethal or promoted) else 0
    else:
        expected = kill + bonus + (hit if promoted else 0) if lethal else hit
    multiplier = 2 if double_points else 1
    expected *= multiplier
    check(report, name + '_native_damage_and_rewards',
          len(receipts) == 1 and receipt['lethal'] == lethal and receipt['insta_kill'] == promoted
          and receipt['aimed_bullet'] == ads and receipt['base_amount'] > 0
          and receipt['amount'] == (health if promoted else receipt['base_amount'])
          and receipt['points'] == expected and (region is None or receipt['region'] == region)
          and after['session']['kills'] == before['session']['kills'] + int(lethal)
          and after['session']['shots'] == before['session']['shots'] + 1,
          aim_setup=aim_setup, aim_samples=aim_samples, batch=batch, inputs=events, expected_points=expected, receipt=receipt,
          before=before, after=after)
    if nuked:
        after = client.until(lambda s: not any(e.get('health') for e in s['enemies'])
                             and s['session']['score'] >= before['session']['score'] + 400 * multiplier)
    else:
        after = client.wait_ticks(3)
    total = expected + (400 * multiplier if nuked else 0)
    check(report, name + '_payout_stats_and_input_release',
          after['session']['score'] == before['session']['score'] + total
          and after['session']['points'] == before['session']['points'] + total
          and after['session']['kills'] == before['session']['kills'] + int(lethal)
          and after['damage']['headshot_kills'] == before['damage']['headshot_kills']
              + int(lethal and receipt['region'] == 'Head')
          and not after['held_inputs'] and not after['queued_inputs']
          and after['player']['life'] == 'Standing', before=before, after=after)
    if region == 'Head':
        baseline = {'audio': fixture['audio_baseline']}
        expected_deaths = 9 if nuked else int(lethal)
        expected_voices = int(lethal and not nuked)
        after = client.until(lambda s: requested(s, 'zmb_vocals_zombie_death')
                             - requested(baseline, 'zmb_vocals_zombie_death') >= expected_deaths
                             and requested(s, 'kill_headshot') - requested(baseline, 'kill_headshot') >= expected_voices)
        head_voices = requested(after, 'kill_headshot') - requested(baseline, 'kill_headshot')
        all_kill_voices = requested(after, '_kill_') - requested(baseline, '_kill_')
        actor_deaths = requested(after, 'zmb_vocals_zombie_death') - requested(baseline, 'zmb_vocals_zombie_death')
        check(report, name + '_requested_voice_and_actor_death',
              head_voices == expected_voices and all_kill_voices == expected_voices
              and actor_deaths == expected_deaths,
              head_voices=head_voices, all_kill_voices=all_kill_voices, actor_deaths=actor_deaths,
              before=baseline['audio'], after=after['audio'])


def run(client, report, game):
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
    for options in (
        ('ordinary_hip_insta', 1000000, False, False),
        ('ordinary_ads_insta', 1000000, False, True),
        ('claimed_hip_insta', 1000000, True, False),
        ('claimed_ads_insta', 1000000, True, True),
        ('claimed_ads_natural_lethal_insta', 1, True, True),
        ('ordinary_natural_lethal_insta', 1, False, False),
    ):
        case(client, report, game, *options)
    case(client, report, game, 'claimed_ads_double_insta', 1000000, True, True, double_points=True)
    case(client, report, game, 'ordinary_head_insta_voice', 1000000, False, True, region='Head')
    case(client, report, game, 'ordinary_head_double_insta_voice', 1000000, False, True,
         double_points=True, region='Head')
    case(client, report, game, 'claimed_head_insta_voice', 1000000, True, True, region='Head')
    case(client, report, game, 'ordinary_head_voice', 1, False, True, insta_kill=False, region='Head')
    case(client, report, game, 'claimed_head_voice', 1, True, True, insta_kill=False, region='Head')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'nuke_instakill_points_and_requested_voices',
              'limitations': ['Enemies, aim, weapons, and effect pickups are direct scenario setup.',
                              'Firing and ADS use native scheduled input; damage, points, kills, and voice requests are observed.',
                              'Requested aliases establish event routing, not audible playback quality.',
                              'Melee, explosives, terminal-owner projectiles, and Electric Cherry are outside this scenario.']}
    try:
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
        report['schema'] = client.call('origins/schema')
        game_path = Path(__file__).resolve().parent.parent / 'assets/origins/game.json'
        run(client, report, json.loads(game_path.read_bytes()))
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'nuke-instakill.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
