import argparse
import json
import math
from pathlib import Path

from test_support import Client, write_report
from test_support import info_string
from test_throwable_down import binding, check, edge, schedule


def native_spread(directory, name):
    path = directory / name
    record = info_string(path, 'WEAPONFILE')
    fields = record['fields']
    return ([float(fields[key]) for key in ('hipSpreadStandMin', 'hipSpreadDuckedMin', 'hipSpreadProneMin', 'adsSpread')], {'path': str(path.resolve())})


def case(client, report, game, name, natives, stance, aimed, deadshot):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': ['weapon.' + name], 'owned_perks': ['perk.deadshot'] if deadshot else []}
    client.call('origins/setup', fixture)
    before = client.until(lambda s: s['player']['action'] == 'Ready'
                          and not s['player']['prone'] and not s['player']['crouched']
                          and abs(s['player']['velocity'][0]) + abs(s['player']['velocity'][2]) < 0.001)
    weapon = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.' + name)
    dual = weapon.get('dual_weapon')
    count = 2 if dual else 1
    fire_tick = 120
    final_tick = 150 if dual else fire_tick
    events = []
    if stance != 'standing':
        key = binding(before, 'prone' if stance == 'prone' else 'crouch')
        events += [edge(1, key, True), edge(2 if stance == 'prone' else final_tick + 3, key, False)]
    if aimed:
        key = binding(before, 'aim')
        events += [edge(3, key, True), edge(final_tick + 2, key, False)]
    fire = binding(before, 'fire')
    events += [edge(120, fire, True), edge(121, fire, False)]
    if dual:
        events += [edge(150, fire, True), edge(151, fire, False)]
    after = schedule(client, events)
    shots = after['firing']['shots'][len(before['firing']['shots']):]
    label = f'{name}_{stance}_{"ads" if aimed else "hip"}_{"deadshot" if deadshot else "base"}'
    check(report, label + '_native_input_emits_expected_shots',
          len(shots) == count and after['session']['shots'] == before['session']['shots'] + count
          and after['player']['weapons'][0]['magazine'] == before['player']['weapons'][0]['magazine'] - 1
          and (not dual or after['player']['weapons'][0]['left_magazine']
               == before['player']['weapons'][0]['left_magazine'] - 1)
          and after['player']['life'] == 'Standing' and not after['held_inputs'] and not after['queued_inputs'],
          fixture=fixture, inputs=events, shots=shots)
    scale = next(p['firing']['hip_spread_scale'] for p in game['content']['perks']
                 if p['id'] == 'perk.deadshot') if deadshot else 1.0
    for index, shot in enumerate(shots):
        identity = dual if index else weapon['id']
        native = natives[identity.removeprefix('weapon.')]
        expected = native[3] if aimed else native[('standing', 'crouched', 'prone').index(stance)] * scale
        check(report, label + f'_shot_{index}_uses_native_stance_spread',
              shot['weapon'] == identity and shot['prone'] == (stance == 'prone')
              and shot['crouched'] == (stance != 'standing') and shot['speed'] < 0.001
              and math.isclose(shot['ads'], float(aimed), abs_tol=0.00001)
              and math.isclose(shot['hip_spread_scale'], scale, abs_tol=0.00001)
              and math.isclose(shot['spread'], expected, abs_tol=0.00001),
              expected_spread=expected, native_spread=native, shot=shot)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    parser.add_argument('--native-weapons', type=Path, required=True)
    parser.add_argument('--dual', action='store_true')
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'native_stance_firing_spread',
              'limitations': ['Weapon/perk grants and enemy/spawn controls are scenario setup.',
                              'Stance, ADS and firing use scheduled native input.',
                              'Checks emitted ray-cone spread, not statistical impact distribution.',
                              'Movement bloom and maximum spread behavior remain outside this scenario.']}
    try:
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        path = Path(__file__).resolve().parent.parent / 'assets/origins/game.json'
        game = json.loads(path.read_bytes())
        report['native_weapons'] = {}
        names = ('fivesevendw_zm', 'fivesevendw_upgraded_zm') if args.dual else ('mp40_zm', 'ballista_zm')
        for name in names:
            weapon = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.' + name)
            hand_names = [name]
            if weapon.get('dual_weapon'):
                hand_names.append(weapon['dual_weapon'].removeprefix('weapon.'))
            natives = {}
            for hand in hand_names:
                native, identity = native_spread(args.native_weapons, hand)
                natives[hand] = native
                report['native_weapons'][hand] = {**identity, 'spread': native}
                definition = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.' + hand)
                check(report, hand + '_pack_preserves_native_spread', definition['mechanics']['spread'] == native,
                      native=native, runtime=definition['mechanics']['spread'])
            for deadshot in (False, True):
                for stance in ('standing', 'crouched', 'prone'):
                    for aimed in ((False,) if args.dual else (False, True)):
                        case(client, report, game, name, natives, stance, aimed, deadshot)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'stance-spread.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
