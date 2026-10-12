import argparse
import json
import math
from pathlib import Path

from test_support import Client, write_report
from test_throwable_down import binding, check, edge, schedule


def actor(state, entity):
    return next((e for e in state['enemies'] if e['entity'] == entity), None)


def settled(client, target):
    previous = None
    frame = None
    stable = 0

    def ready(state):
        nonlocal previous, frame, stable
        enemy = actor(state, target)
        if not enemy or state['render_frame'] == frame:
            return False
        frame = state['render_frame']
        centers = state['player']['position'] + enemy['position']
        stable = stable + 1 if previous and all(abs(a - b) < 0.005 for a, b in zip(centers, previous)) else 0
        previous = centers
        return stable >= 3 and state['player']['action'] == 'Ready'

    return client.until(ready)


def prepare(client, health, nuked, insta_kill=False, double_points=False):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': ['weapon.c96_zm'], 'pitch': 0}
    client.call('origins/setup', fixture)
    state = client.wait_ticks(12)
    home = state['player']['position']
    pickup = list(home)
    pickup[1] -= state['player']['height'] / 2 - 0.25
    effects = ['powerup.zombie_blood']
    if insta_kill:
        effects.append('powerup.insta_kill')
    if double_points:
        effects.append('powerup.double_points')
    blood = {'pickups': [{'id': effect, 'position': pickup} for effect in effects]}
    client.call('origins/setup', blood)
    client.until(lambda s: s['drops']['collected'] == len(effects)
                 and s['effects']['zombie_blood'] > 0
                 and (not insta_kill or s['effects']['insta_kill'] > 0)
                 and (not double_points or s['effects']['double_points'] > 0))
    target = [home[0], home[1], home[2] + 4.5]
    enemies = []
    if nuked:
        enemies = [{'position': [home[0] + x, home[1], home[2] + z], 'health': 100}
                   for z in (2.2, 2.8) for x in (-3.0, -2.4, 2.4, 3.0)]
    enemies.append({'position': target, 'health': health})
    result = client.call('origins/setup', {'enemies': enemies})
    if len(result['enemies']) != len(enemies):
        raise RuntimeError('Scenario target count mismatch')
    target = result['enemies'][-1]
    ready = client.until(lambda s: all(actor(s, entity) and actor(s, entity)['ready']
                                      and actor(s, entity)['hit_regions'] for entity in result['enemies']), timeout=90)
    settled(client, target)
    if nuked:
        client.call('origins/setup', {'pickups': [{'id': 'powerup.nuke', 'position': pickup}]})
        client.until(lambda s: s['drops']['collected'] == len(effects) + 1)
        state = client.until(lambda s: actor(s, target) and actor(s, target).get('nuked')
                             and actor(s, target).get('health') == health)
    else:
        state = client.until(lambda s: actor(s, target) and actor(s, target).get('health') == health)
    fixture.update({'blood_setup': blood, 'enemies': enemies, 'nuke_pickup': nuked,
                    'audio_baseline': ready['audio']})
    return state, target, fixture


def aim(client, state, target, region=None):
    player = state['player']
    position = actor(state, target)['position']
    if region:
        volume = next(v for v in actor(state, target)['hit_regions'] if v['region'] == region)
        position = [(start + end) / 2 for start, end in zip(volume['start'], volume['end'])]
    dx = position[0] - player['position'][0]
    dz = position[2] - player['position'][2]
    eye = player['position'][1] + player['eye_height'] - player['height'] / 2
    dy = position[1] + (0 if region else 0.25) - eye
    setup = {'yaw': math.atan2(-dx, -dz), 'pitch': math.atan2(dy, math.hypot(dx, dz))}
    client.call('origins/setup', setup)
    return setup


def case(client, report, game, name, health, nuked, ads):
    before, target, fixture = prepare(client, health, nuked)
    aim_setup = aim(client, before, target)
    before = client.state()
    check(report, name + '_precondition',
          actor(before, target) is not None and actor(before, target)['health'] == health
          and actor(before, target)['nuked'] == nuked and before['player']['life'] == 'Standing'
          and before['player']['action'] == 'Ready', fixture=fixture, before=before)
    fire = binding(before, 'fire')
    shot = 1
    events = []
    if ads:
        weapon = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.c96_zm')
        shot = math.ceil(weapon['mechanics']['ads_seconds'][0] / before['fixed_delta_seconds']) + 3
        events.append(edge(1, binding(before, 'aim'), True))
        events.append(edge(shot + 3, binding(before, 'aim'), False))
    events.extend([edge(shot, fire, True), edge(shot + 1, fire, False)])
    after = schedule(client, events)
    after = client.until(lambda s: any(r['target'] == target and r['source'] == 'weapon.c96_zm'
                                      for r in s['damage']['recent'][len(before['damage']['recent']):]))
    receipts = [r for r in after['damage']['recent'][len(before['damage']['recent']):]
                if r['target'] == target and r['source'] == 'weapon.c96_zm']
    receipt = receipts[0]
    lethal = health == 1
    hit, kill, head, torso, _ = game['tuning']['combat_points']
    bonus = head if receipt['region'] == 'Head' else (torso if receipt['region'] in ('Torso', 'UpperTorso') else 0)
    expected = 0 if nuked and (lethal or not ads) else (kill + bonus if lethal else hit)
    check(report, name + '_native_hit_and_points',
          len(receipts) == 1 and receipt['lethal'] == lethal and receipt['aimed_bullet'] == ads
          and receipt['points'] == expected
          and after['session']['kills'] == before['session']['kills'] + int(lethal)
          and after['session']['shots'] == before['session']['shots'] + 1,
          aim_setup=aim_setup, inputs=events, receipt=receipt, before=before, after=after)
    total = receipt['points'] + (400 if nuked else 0)
    if nuked:
        after = client.until(lambda s: not any(e.get('health') for e in s['enemies'])
                             and s['session']['score'] >= before['session']['score'] + 400)
    check(report, name + '_final_payout_and_attributed_stats',
          after['session']['score'] == before['session']['score'] + total
          and after['session']['points'] == before['session']['points'] + total
          and after['session']['kills'] == before['session']['kills'] + int(lethal)
          and after['player']['life'] == 'Standing'
          and after['damage']['headshot_kills'] == before['damage']['headshot_kills']
              + int(lethal and receipt['region'] == 'Head'), before=before, after=after)


def run(client, report, game):
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
    for name, health, nuked, ads in (
        ('ordinary_hip_hit', 1000000, False, False),
        ('claimed_hip_hit', 1000000, True, False),
        ('claimed_ads_hit', 1000000, True, True),
        ('claimed_hip_kill', 1, True, False),
        ('claimed_ads_kill', 1, True, True),
        ('fresh_session_ordinary_kill', 1, False, False),
    ):
        case(client, report, game, name, health, nuked, ads)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'nuke_claimed_victim_scoring',
              'limitations': ['Weapons, enemies, aim, spawn delays, and Nuke/Zombie Blood pickups are direct scenario setup.',
                              'Firing and ADS use native scheduled input; accepted damage and rewards are observed.',
                              'Zombie Blood prevents enemy attacks during the scenarios.',
                              'Electric Cherry, Insta-Kill, melee, and voice behavior are outside this scenario.']}
    try:
        report['schema'] = client.call('origins/schema')
        game_path = Path(__file__).resolve().parent.parent / 'assets/origins/game.json'
        run(client, report, json.loads(game_path.read_bytes()))
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'nuke-rewards.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
