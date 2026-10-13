# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///

from test_support import asset_bytes

import argparse
import json
from pathlib import Path

from test_support import Client, write_report
from test_support import source_mobility as mobility_tuning, source_gravity as native_physics
from test_support import source_physics as native_tuning
import test_dive_physics as dive
import test_environment as environment
import test_mobility as mobility
import test_nuke_instakill as instakill
import test_phase_fixes as phase
import test_physics as physics
import test_session as session
import test_solo_pistols as pistols
import test_ui as ui
import test_weapon_locomotion as animation


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--assets', required=True, type=Path)
    parser.add_argument('--research', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--inspection', type=Path)
    parser.add_argument('--no-captures', action='store_true')
    parser.add_argument('--fail-fast', action='store_true')
    parser.add_argument('--section', action='append', choices=(
        'ui', 'session', 'projectiles', 'physics', 'mobility', 'dive', 'pistols',
        'instakill', 'environment', 'stations', 'landing', 'quests', 'animation'))
    args = parser.parse_args()
    output = args.output.resolve()
    if output.is_relative_to(Path(__file__).resolve().parents[1]):
        raise ValueError('Validation output must be external')
    output.mkdir(parents=True, exist_ok=True)
    game = json.loads(asset_bytes(Path(args.assets.resolve()) / 'game.json'))
    client = Client(args.url, request_timeout=20, shutdown_timeout=45)
    report = {'passed': False, 'cases': [], 'sections': [], 'limitations': ['Fixtures establish preconditions; scheduled input and observed outcomes establish behavior.', 'This bounded integration suite does not establish exhaustive original-game parity or performance.']}
    selected = args.section or ['ui', 'session', 'projectiles', 'physics', 'mobility',
                                'dive', 'pistols', 'instakill', 'environment', 'stations', 'landing', 'quests', 'animation']
    site = None
    try:
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), 300)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        for name in selected:
            section = {'name': name, 'passed': False, 'first_case': len(report['cases'])}
            report['sections'].append(section)
            print(json.dumps({'section': name, 'status': 'running'}), flush=True)
            try:
                client.reset()
                if name in ('dive', 'landing', 'animation') and report.get('floor') is None:
                    candidates, _ = physics.surfaces(Path(args.assets.resolve()), game)
                    site = physics.select_flat(client, report, game, candidates,
                                               native_tuning(args.research)[0])
                    report['floor'] = site
                if name == 'ui':
                    ui.run(client, report, None if args.no_captures else output, game)
                elif name == 'session':
                    session.run(client, report)
                elif name == 'projectiles':
                    phase.run(client, report, None if args.no_captures else output)
                elif name == 'physics':
                    motor, movement = native_tuning(args.research)
                    candidates, stairs = physics.surfaces(Path(args.assets.resolve()), game)
                    site = physics.select_flat(client, report, game, candidates, motor)
                    report['floor'] = site
                    physics.directions(client, report, game, site, motor, movement)
                    physics.air(client, report, game, site, motor)
                    physics.slopes(client, report, game, candidates, motor)
                    physics.stairs(client, report, game, stairs, motor)
                    for weapon, action, gravity in [('weapon.m32_zm', 'fire', game['tuning']['gravity']),
                                                     ('weapon.ray_gun_zm', 'fire', 0),
                                                     ('weapon.mp40_zm', 'lethal', game['tuning']['gravity'])]:
                        physics.projectile_case(client, report, game, site, weapon, action, gravity)
                elif name == 'mobility':
                    mobility.run(client, report, mobility_tuning(args.research), game)
                elif name == 'dive':
                    native = mobility_tuning(args.research)
                    vertical = native_physics(args.research)
                    motor = native_tuning(args.research)[0]
                    floor = next((row['fixture'] for row in report['cases']
                                  if row['case'] == 'authored_open_floor_supports_native_jump' and row['passed']), None)
                    dive.flat_motion(client, report, native, vertical, floor)
                    dive.held_slide(client, report, native, motor, floor)
                    base = dive.fall_site(client, report, game)
                    for height, diving, perks in ((1.5, True, []), (2.5, True, []), (5.4, True, []),
                                                  (2.5, True, ['perk.juggernog']),
                                                  (5.4, True, ['perk.phd_flopper']),
                                                  (2.5, False, []), (4.5, False, []), (8, False, [])):
                        dive.fall_case(client, report, native, game, base, height, diving, perks)
                    dive.wall_dive(client, report, native, game)
                elif name == 'pistols':
                    for held, expected in ((['weapon.mp40_zm'], 'weapon.c96_upgraded_zm'),
                                           (['weapon.python_upgraded_zm'], 'weapon.python_upgraded_zm'),
                                           (['weapon.fivesevendw_upgraded_zm'], 'weapon.fivesevendw_upgraded_zm'),
                                           (['weapon.python_upgraded_zm', 'weapon.ray_gun_zm'], 'weapon.ray_gun_zm'),
                                           (['weapon.raygun_mark2_zm'], 'weapon.raygun_mark2_zm'),
                                           (['weapon.raygun_mark2_zm', 'weapon.ray_gun_upgraded_zm'], 'weapon.ray_gun_upgraded_zm')):
                        pistols.run_case(client, report, game, held, expected)
                    pistols.without_quick_revive(client, report, game)
                elif name == 'instakill':
                    instakill.run(client, report, game)
                elif name == 'environment':
                    def record(case, passed, **evidence):
                        phase.check(report, case, passed, **evidence)
                    environment.run(client, game, record, ['shovel', 'weather', 'progression', 'hidden'], output)
                elif name == 'stations':
                    from test_final_stations import run
                    if args.inspection is None:
                        raise ValueError('Station source checks require --inspection')
                    run(client, report, game, args.research, args.inspection)
                elif name == 'landing':
                    from test_landing_surface import run
                    run(client, report, game, args.research)
                elif name == 'quests':
                    from test_final_quests import run
                    run(client, report, args.research, game)
                elif name == 'animation':
                    from test_weapon_locomotion import ACTIONS, TIMINGS
                    if args.inspection is None:
                        raise ValueError('Animation source checks require --inspection')
                    recovered = args.inspection / 'solo-content/recovered-gameplay.json'
                    animation.source_bindings(report, game, Path(args.assets.resolve()), args.inspection, {'action_fields': ACTIONS, 'timing_fields': TIMINGS})
                    floor = next((row['fixture'] for row in report['cases']
                                  if row['case'] == 'authored_open_floor_supports_native_jump' and row['passed']), None)
                    presentation = next(w['presentation'] for w in game['content']['weapons'] if w['id'] == 'weapon.mp40_zm')
                    row = next(p for p in game['content']['presentations'] if p['id'] == presentation)
                    doc = json.loads(asset_bytes(Path(args.assets.resolve()) / row['scene'].split('#')[0]))
                    clips = {a['name']: max(doc['accessors'][s['input']]['max'][0] for s in a['samplers'])
                             for a in doc['animations']}
                    for empty in (False, True):
                        animation.crawl_case(client, report, game, 'weapon.mp40_zm', empty, clips)
                        animation.dive_case(client, report, game, 'weapon.mp40_zm', empty, clips, floor)
                    animation.crawl_fire_case(client, report, game, 'weapon.mp40_zm', clips)
                section['passed'] = True
            except Exception as error:
                section['error'] = f'{type(error).__name__}: {str(error)[:1200]}'
                try:
                    section['failure_state'] = client.state()
                except Exception as diagnostic_error:
                    section['diagnostic_error'] = str(diagnostic_error)[:300]
            finally:
                section['case_count'] = len(report['cases']) - section['first_case']
                client.release()
                write_report(output, 'final-gameplay-progress.json', report)
                print(json.dumps({key: value for key, value in section.items()
                                  if key != 'failure_state'}), flush=True)
            if args.fail_fast and not section['passed']:
                break
        report['passed'] = all(s['passed'] for s in report['sections'])
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {str(error)[:1200]}'
    finally:
        client.shutdown(report)
        target = write_report(output, 'final-gameplay.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'report': str(target)}))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
