from test_support import asset_bytes
import argparse
import json
import math
from pathlib import Path

from test_support import Client, write_report
from test_throwable_down import binding, check, edge, schedule


def prepare(client, stock):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'equipment': ['weapon.claymore_zm'], 'owned_perks': ['perk.quick_revive'],
               'weapons': ['weapon.c96_zm'], 'pitch': 0}
    client.call('origins/setup', fixture)
    before = client.wait_ticks(12)
    mine = binding(before, 'mine')
    schedule(client, [edge(1, mine, True), edge(2, mine, False)])
    spent = client.until(lambda s: s['equipment']['mine_placed'] == before['equipment']['mine_placed'] + 1
                         and s['player']['mine']['count'] == stock - 1 and not s['player']['mine']['busy'])
    return spent, fixture


def pickup(client, state, distance):
    player = state['player']
    position = list(player['position'])
    position[1] -= player['height'] / 2 - 0.25
    position[0] -= math.sin(player['yaw']) * distance
    position[2] -= math.cos(player['yaw']) * distance
    setup = {'pickups': [{'id': 'powerup.full_ammo', 'position': position}]}
    result = client.call('origins/setup', setup)
    return setup, result['pickups'][0]


def run(client, report, game):
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
    claymore = next(w['mine'] for w in game['content']['weapons'] if w['id'] == 'weapon.claymore_zm')
    stock = claymore['stock']
    before, fixture = prepare(client, stock)
    fire = binding(before, 'fire')
    schedule(client, [edge(1, fire, True), edge(2, fire, False)])
    before = client.state()
    setup, entity = pickup(client, before, 2.4)
    waiting = client.wait_ticks(2)
    check(report, 'max_ammo_waits_outside_collection_radius',
          any(p['entity'] == entity for p in waiting['drops']['active'])
          and waiting['drops']['collected'] == before['drops']['collected']
          and waiting['player']['mine']['count'] == stock - 1,
          fixture=fixture, pickup_setup=setup, before=before, after=waiting)
    forward = binding(waiting, 'forward')
    travel_ticks = math.ceil(0.6 / waiting['fixed_delta_seconds'])
    after = schedule(client, [edge(1, forward, True), edge(travel_ticks, forward, False)])
    after = client.until(lambda s: s['drops']['collected'] == before['drops']['collected'] + 1)
    check(report, 'native_movement_collects_max_ammo_and_refills_claymore',
          after['player']['life'] == 'Standing' and after['player']['mine']['count'] == stock
          and not any(p['entity'] == entity for p in after['drops']['active'])
          and after['player']['weapons'][0]['magazine'] == before['player']['weapons'][0]['magazine'],
          pickup_setup=setup, before=before, after=after)

    before, fixture = prepare(client, stock)
    client.call('origins/setup', {'pitch': -1.5})
    frag = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == 'weapon.frag_grenade_zm')
    lethal = binding(before, 'lethal')
    down_ticks = math.ceil((frag['prime_seconds'] + frag['fuse_seconds'] + 0.7) / before['fixed_delta_seconds'])
    downed = schedule(client, [edge(1, lethal, True), edge(down_ticks, lethal, False)])
    check(report, 'native_cooked_frag_down_preserves_spent_mines',
          downed['player']['life'] == 'Downed' and downed['player']['mine']['count'] == stock - 1,
          fixture=fixture, before=before, after=downed)
    setup, entity = pickup(client, downed, 0)
    after = client.until(lambda s: s['drops']['collected'] == downed['drops']['collected'] + 1)
    check(report, 'max_ammo_keeps_native_downed_exclusion',
          after['player']['life'] == 'Downed' and after['player']['mine']['count'] == stock - 1
          and not any(p['entity'] == entity for p in after['drops']['active']),
          pickup_setup=setup, before=downed, after=after)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'max_ammo_claymore_inventory',
              'limitations': ['Equipment, perks, and pickups are direct scenario setup; acquisition and random drops are not covered.',
                              'Claymores are spent through native input and Max Ammo uses normal proximity collection.',
                              'The standing pickup is reached by native movement; the downed pickup is placed at the player.']}
    try:
        report['schema'] = client.call('origins/schema')
        game_path = Path(__file__).resolve().parent.parent / 'target/assets/game.json'
        run(client, report, json.loads(asset_bytes(game_path)))
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'max-ammo-mines.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
