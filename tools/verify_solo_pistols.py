import argparse
import json
from pathlib import Path

from gameplay_remote import Client, write_report
from prepare_solo_pistols import native_pistols
from verify_brp import assets_match, digest
from verify_throwable_down import binding, check, edge, schedule


def weapons(state):
    return {weapon['id']: weapon for weapon in state['player']['weapons']}


def active(state):
    return next(weapon for weapon in state['player']['weapons']
                if weapon['slot'] == state['player']['active_slot'])


def run_case(client, report, game, held, expected):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': held, 'owned_perks': ['perk.quick_revive'], 'pitch': 1.2}
    client.call('origins/setup', fixture)
    before = client.until(lambda state: state['player']['action'] == 'Ready'
                          and not state['player']['mobility']['airborne'])
    original = weapons(before)
    definition = next(weapon for weapon in game['content']['weapons'] if weapon['id'] == expected)
    grenade = next(weapon['throwable'] for weapon in game['content']['weapons']
                   if weapon['id'] == 'weapon.frag_grenade_zm')
    lethal = binding(before, 'lethal')
    end = round((grenade['prime_seconds'] + grenade['fuse_seconds'] + 2)
                / before['fixed_delta_seconds'])
    down_inputs = [edge(1, lethal, True), edge(end, lethal, False)]
    client.call('origins/input', {'events': down_inputs})
    admitted = client.until(lambda state: state['player']['life'] == 'Downed', timeout=30)
    downed = client.until(lambda state: state['player']['life'] == 'Downed'
                          and state['player']['action'] == 'Ready', timeout=30)
    client.release()
    downed_weapon = active(downed)
    check(report, 'native_downed_weapon_' + '_'.join(held),
          downed_weapon['id'] == expected and downed['session']['downs'] == 1
          and not downed['player']['perks'],
          fixture=fixture, inputs=down_inputs, before=before['player'], downed=downed['player'])
    saved = original.get(expected)
    if saved and expected != game['solo_revive']['downed_weapon']:
        if expected in ('weapon.ray_gun_zm', 'weapon.ray_gun_upgraded_zm'):
            magazine = min(saved['magazine'] + (saved['left_magazine'] or 0)
                           + saved['reserve'], definition['magazine'])
            reserve = 0
        else:
            magazine = saved['magazine']
            reserve = min(saved['reserve'], definition['magazine'] * 2)
    else:
        magazine = saved['magazine'] if saved else definition['magazine']
        reserve = definition['magazine'] * 2
    check(report, 'native_downed_ammunition_' + expected,
          downed_weapon['magazine'] == magazine and downed_weapon['reserve'] == reserve
          and downed_weapon['left_magazine'] == (saved['left_magazine'] if saved else None),
          expected_magazine=magazine, expected_left=saved['left_magazine'] if saved else None,
          expected_reserve=reserve, actual=downed_weapon)
    fire = binding(downed, 'fire')
    inputs = [event for tick in (1, 26, 51)
              for event in (edge(tick, fire, True), edge(tick + 1, fire, False))]
    after_firing = schedule(client, inputs)
    fired_weapon = active(after_firing)
    used = (magazine + (downed_weapon['left_magazine'] or 0) + reserve
            - fired_weapon['magazine'] - (fired_weapon['left_magazine'] or 0)
            - fired_weapon['reserve'])
    check(report, 'normal_input_can_fire_downed_' + expected,
          after_firing['player']['life'] == 'Downed' and used > 0,
          inputs=inputs, consumed=used, player=after_firing['player'])
    revived = client.until(lambda state: state['player']['life'] == 'Standing'
                           and state['player']['solo_charges'] == 0, timeout=30)
    elapsed = (revived['fixed_tick'] - admitted['fixed_tick']) * revived['fixed_delta_seconds']
    check(report, 'native_solo_revive_duration_' + expected,
          abs(elapsed - game['solo_revive']['seconds']) <= .2,
          observed_seconds=elapsed, expected_seconds=game['solo_revive']['seconds'],
          admission_tick=admitted['fixed_tick'], revived_tick=revived['fixed_tick'])
    restored = weapons(revived)
    expected_ammo = {key: {'magazine': value['magazine'], 'left_magazine': value['left_magazine'],
                           'reserve': value['reserve']}
                     for key, value in original.items()}
    if saved and expected != game['solo_revive']['downed_weapon']:
        expected_ammo[expected]['reserve'] = max(0, saved['reserve'] - used)
        expected_ammo[expected]['magazine'] = max(0, saved['magazine'] - max(0, used - saved['reserve']))
    actual_ammo = {key: {'magazine': value['magazine'], 'left_magazine': value['left_magazine'],
                         'reserve': value['reserve']}
                   for key, value in restored.items()}
    check(report, 'native_revived_ammunition_' + expected,
          actual_ammo == expected_ammo and revived['player']['health'] == game['player_health']['maximum']
          and not revived['player']['perks'],
          expected=expected_ammo, actual=actual_ammo, player=revived['player'])


def without_quick_revive(client, report, game):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': ['weapon.mp40_zm'], 'owned_perks': []}
    client.call('origins/setup', fixture)
    before = client.wait_ticks(12)
    grenade = next(weapon['throwable'] for weapon in game['content']['weapons']
                   if weapon['id'] == 'weapon.frag_grenade_zm')
    lethal = binding(before, 'lethal')
    end = round((grenade['prime_seconds'] + grenade['fuse_seconds'] + 2)
                / before['fixed_delta_seconds'])
    inputs = [edge(1, lethal, True), edge(end, lethal, False)]
    client.call('origins/input', {'events': inputs})
    ended = client.until(lambda state: state['state'] == 'GameOver', timeout=30)
    client.release()
    elapsed = (ended['fixed_tick'] - before['fixed_tick']) * before['fixed_delta_seconds']
    check(report, 'native_solo_without_quick_revive_ends_match',
          ended['player']['life'] == 'HumanTerminal' and ended['session']['downs'] == 0
          and elapsed < grenade['prime_seconds'] + grenade['fuse_seconds'] + 1,
          fixture=fixture, inputs=inputs, elapsed=elapsed, before=before['player'],
          ended=ended['player'], session=ended['session'])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--identity', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--research', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url, shutdown_timeout=45)
    report = {'passed': False, 'cases': [], 'scenario': 'solo_pistol_selection_and_ammunition',
              'limitations': ['Perk ownership and weapon loadouts are scenario setup; purchases are not covered.',
                              'Downing, firing, and self-revival use normal game input and gameplay systems.',
                              'Initial-ammunition cases do not cover empty guns or reserve depletion.',
                              'Dual-wield cases exercise the fire binding and preserve both original magazines.']}
    try:
        identity = json.loads(args.identity.read_bytes())
        report['identity'] = {'path': str(args.identity.resolve()), 'sha256': digest(args.identity)}
        report['binaries'] = {name: digest(args.identity.parent / name) for name in identity['binaries']}
        if report['binaries'] != identity['binaries']:
            raise RuntimeError('Pinned binary identity changed')
        report['assets'] = assets_match(identity)
        report['script_hashes'] = {name: digest(Path(__file__).parent / name) for name in (
            'verify_solo_pistols.py', 'prepare_solo_pistols.py', 'gameplay_remote.py',
            'verify_brp.py', 'verify_throwable_down.py')}
        source = args.research / 'source/server-patch/maps/mp/zombies/_zm.gsc'
        report['native_source'] = {'path': str(source), 'sha256': digest(source)}
        game = json.loads((Path(identity['asset_root']) / 'game.json').read_bytes())
        ranks, replacement = native_pistols(args.research)
        loaded = {weapon['id'] for weapon in game['content']['weapons']}
        native_profile = {'pistol_ranks': [row for row in ranks if row['weapon'] in loaded],
                          'pistol_replace_below': replacement}
        check(report, 'native_pistol_selection_data',
              all(game['solo_revive'][key] == value for key, value in native_profile.items()),
              native_profile=native_profile, native_rank=ranks)
        client.until(lambda state: state['state'] in ('Playing', 'Paused', 'GameOver')
                     and state.get('player'), timeout=240)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        for held, expected in ((['weapon.mp40_zm'], 'weapon.c96_upgraded_zm'),
                               (['weapon.c96_zm'], 'weapon.c96_upgraded_zm'),
                               (['weapon.fiveseven_upgraded_zm'], 'weapon.c96_upgraded_zm'),
                               (['weapon.python_upgraded_zm'], 'weapon.python_upgraded_zm'),
                               (['weapon.fivesevendw_upgraded_zm'], 'weapon.fivesevendw_upgraded_zm'),
                               (['weapon.python_upgraded_zm', 'weapon.ray_gun_zm'], 'weapon.ray_gun_zm'),
                               (['weapon.raygun_mark2_zm'], 'weapon.raygun_mark2_zm'),
                               (['weapon.raygun_mark2_zm', 'weapon.ray_gun_upgraded_zm'], 'weapon.ray_gun_upgraded_zm'),
                               (['weapon.mp40_zm', 'weapon.c96_upgraded_zm'], 'weapon.c96_upgraded_zm')):
            run_case(client, report, game, held, expected)
        without_quick_revive(client, report, game)
        report['assets_after'] = assets_match(identity)
        report['passed'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        try:
            client.release()
        except Exception as error:
            report['release_error'] = str(error)
        finally:
            client.shutdown(report)
        path = write_report(args.output, 'solo-pistols.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
