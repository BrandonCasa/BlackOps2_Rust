from test_support import asset_bytes
import argparse
import json
import math
from pathlib import Path

from test_support import Client, write_report
from test_nuke_rewards import actor
from test_throwable_down import binding, check, edge, schedule


def prepare(client, game, health, nuked, double_points, insta_kill):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': ['weapon.c96_zm'], 'owned_perks': ['perk.electric_cherry'], 'pitch': 1.5}
    client.call('origins/setup', fixture)
    state = client.wait_ticks(12)
    pickup = list(state['player']['position'])
    pickup[1] -= state['player']['height'] / 2 - 0.25
    effects = ['powerup.zombie_blood']
    if double_points:
        effects.append('powerup.double_points')
    if insta_kill:
        effects.append('powerup.insta_kill')
    pickups = [{'id': effect, 'position': pickup} for effect in effects]
    client.call('origins/setup', {'pickups': pickups})
    state = client.until(lambda s: s['drops']['collected'] == len(effects))
    weapon = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.c96_zm')
    spacing = math.ceil(weapon['fire_interval'] / state['fixed_delta_seconds']) + 2
    fire = [event for i in range(4) for event in
            (edge(1 + i * spacing, binding(state, 'fire'), True),
             edge(2 + i * spacing, binding(state, 'fire'), False))]
    schedule(client, fire)
    magazine = weapon['magazine'] - 4
    state = client.until(lambda s: s['player']['weapons'][0]['magazine'] == magazine
                         and s['player']['action'] == 'Ready')
    home = state['player']['position']
    enemies = []
    if nuked:
        enemies = [{'position': [home[0] + x, home[1], home[2] + z], 'health': 1000000}
                   for x in (2.4, 3.0) for z in (-0.9, -0.3, 0.3, 0.9)]
    enemies.append({'position': [home[0] - 1.8, home[1], home[2]], 'health': health})
    entities = client.call('origins/setup', {'enemies': enemies})['enemies']
    if len(entities) != len(enemies):
        raise RuntimeError('Scenario target count mismatch')
    target = entities[-1]
    state = client.until(lambda s: all(actor(s, entity) and actor(s, entity)['ready']
                                      and actor(s, entity)['hit_regions'] for entity in entities), timeout=90)
    if nuked:
        nuke = [pickup[0] + 1.4, pickup[1], pickup[2]]
        client.call('origins/setup', {'pickups': [{'id': 'powerup.nuke', 'position': nuke}]})
        state = client.until(lambda s: s['drops']['collected'] == len(effects) + 1
                             and actor(s, target) and actor(s, target)['nuked']
                             and actor(s, target)['health'] == health)
        fixture['nuke_pickup'] = nuke
    cherry = next(p['cherry'] for p in game['content']['perks'] if p['id'] == 'perk.electric_cherry')
    fraction = magazine / weapon['magazine']
    damage = math.floor(cherry['damage'][1] + (cherry['damage'][0] - cherry['damage'][1]) * fraction + 0.5)
    fixture.update({'pickups': pickups, 'enemies': enemies, 'fire': fire,
                    'magazine': magazine, 'damage': damage})
    return state, target, fixture


def case(client, report, game, health, nuked, double_points, insta_kill):
    name = '_'.join(('claimed' if nuked else 'ordinary', 'natural_lethal' if health == 1 else 'base_nonlethal',
                     'double' if double_points else 'single', 'insta' if insta_kill else 'normal'))
    before, target, fixture = prepare(client, game, health, nuked, double_points, insta_kill)
    check(report, name + '_precondition',
          actor(before, target)['health'] == health and actor(before, target)['nuked'] == nuked
          and before['player']['life'] == 'Standing' and before['player']['action'] == 'Ready'
          and before['player']['weapons'][0]['magazine'] == fixture['magazine'] and before['session']['shots'] == 4
          and 'perk.electric_cherry' in before['player']['perks']
          and before['effects']['zombie_blood'] > 0
          and (before['effects']['double_points'] > 0) == double_points
          and (before['effects']['insta_kill'] > 0) == insta_kill, fixture=fixture, before=before)
    events = [edge(1, binding(before, 'reload'), True), edge(2, binding(before, 'reload'), False)]
    schedule(client, events)
    after = client.until(lambda s: any(r['source'] == 'perk.electric_cherry' and r['target'] == target
                                      for r in s['damage']['recent'][len(before['damage']['recent']):]))
    receipts = [r for r in after['damage']['recent'][len(before['damage']['recent']):]
                if r['source'] == 'perk.electric_cherry']
    receipt = receipts[0]
    natural = health <= fixture['damage']
    promoted = insta_kill and not nuked and not natural
    lethal = natural or promoted
    hit, kill, _, _, _ = game['tuning']['combat_points']
    regular = 0 if nuked else (kill + (hit if promoted else 0) if lethal else hit)
    multiplier = 2 if double_points else 1
    expected = regular * multiplier + (40 if natural else 0)
    check(report, name + '_native_reload_damage_and_score',
          len(receipts) == 1 and receipt['target'] == target and receipt['category'] == 'Lightning'
          and receipt['region'] == 'Torso' and receipt['base_amount'] == fixture['damage']
          and receipt['insta_kill'] == promoted and receipt['lethal'] == lethal
          and receipt['amount'] == (health if promoted else fixture['damage']) and receipt['points'] == expected
          and after['session']['shots'] == before['session']['shots']
          and after['session']['kills'] == before['session']['kills'] + int(lethal),
          inputs=events, expected_points=expected, receipt=receipt, before=before, after=after)
    if nuked:
        after = client.until(lambda s: not any(e.get('health') for e in s['enemies'])
                             and s['session']['score'] >= before['session']['score'] + 400 * multiplier)
    total = expected + (400 * multiplier if nuked else 0)
    check(report, name + '_payout_stats_and_input_release',
          after['session']['score'] == before['session']['score'] + total
          and after['session']['points'] == before['session']['points'] + total
          and after['session']['kills'] == before['session']['kills'] + int(lethal)
          and after['damage']['headshot_kills'] == before['damage']['headshot_kills']
          and after['damage']['melee_kills'] == before['damage']['melee_kills']
          and after['player']['life'] == 'Standing' and not after['held_inputs'] and not after['queued_inputs'],
          before=before, after=after)


def run(client, report, game):
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
    for nuked in (False, True):
        for health in (1000, 1):
            for double_points in (False, True):
                case(client, report, game, health, nuked, double_points, False)
    for nuked, health in ((False, 1000), (True, 1000), (False, 1)):
        for double_points in (False, True):
            case(client, report, game, health, nuked, double_points, True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'cherry_native_input_rewards',
              'limitations': ['Perk/weapon grants, actors, spawn delay, and powerup pickups are scenario setup.',
                              'Firing and reloading use native scheduled input; delayed damage, points, and kills are observed.',
                              'Native raw lethal bonus timing before the delayed shock remains an approximation.',
                              'Competing deaths during shock delay, perk acquisition, downed shocks, and audiovisual fidelity are outside this scenario.']}
    try:
        report['schema'] = client.call('origins/schema')
        game_path = Path(__file__).resolve().parent.parent / 'target/debug/origins.pack/game.json'
        run(client, report, json.loads(asset_bytes(game_path)))
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'cherry-rewards.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
