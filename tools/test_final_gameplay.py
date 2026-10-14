# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///

from test_support import asset_bytes

import argparse
import json
import math
from pathlib import Path
import time

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
from test_throwable_down import binding, check, edge, schedule


INITIAL_SECTIONS = ('initial_start', 'initial_controls', 'initial_combat', 'initial_rounds', 'initial_life')


def normal_action(client, action, ticks=2):
    key = binding(client.state(), action)
    return schedule(client, [edge(1, key, True), edge(ticks + 1, key, False)])


def normal_look(client, game, point):
    state = client.state()
    player, controls = state['player'], state['controls']
    eye = player['position'].copy()
    eye[1] += player['eye_height'] - player['height'] / 2
    delta = [target - origin for origin, target in zip(eye, point)]
    yaw = math.atan2(-delta[0], -delta[2])
    pitch = math.atan2(delta[1], math.hypot(delta[0], delta[2]))
    hip = game['tuning']['mouse_sensitivity'] * controls['sensitivity']
    ads = game['tuning']['ads_mouse_sensitivity'] * controls['ads_sensitivity']
    sensitivity = hip + (ads - hip) * player['ads']
    horizontal = (yaw - player['yaw'] + math.pi) % math.tau - math.pi
    vertical = -1 if controls['invert_mouse_y'] else 1
    schedule(client, [{'tick': 1, 'motion': [-horizontal / sensitivity,
                                           -(pitch - player['pitch']) / sensitivity / vertical]}])
    return client.until(lambda s: abs((s['player']['yaw'] - yaw + math.pi) % math.tau - math.pi) < .02
                        and abs(s['player']['pitch'] - max(-1.5, min(1.5, pitch))) < .02, timeout=5)


def initial_start(client, report, game, output):
    state = client.until(lambda s: s['state'] == 'Playing' and s.get('player')
                         and s['player']['grounded'] and s['player']['action'] == 'Ready', timeout=300)
    pistol = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.c96_zm')
    frag = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == 'weapon.frag_grenade_zm')
    owned = state['player']['weapons']
    check(report, 'initial_source_loadout_without_grants', len(owned) == 1
          and owned[0]['id'] == pistol['id'] and owned[0]['magazine'] == pistol['magazine']
          and owned[0]['reserve'] == pistol['mechanics']['starting_reserve']
          and state['player']['lethal']['count'] == frag['initial']
          and not state['player']['perks'] and state['player']['solo_charges'] == 0,
          player=state['player'])
    check(report, 'initial_grounded_in_authored_spawn_area', state['session']['round'] == 1
          and state['player']['health'] == game['player_health']['maximum']
          and min(math.dist(state['player']['position'], spawn['position'])
                  for spawn in game['initial_spawns']) < 2, state=state)
    materials = client.call('origins/materials')
    check(report, 'initial_render_pipelines_ready', materials['pipelines']['pending'] == 0
          and materials['pipelines']['error'] is None, materials=materials)
    hud = ui.observe(client, lambda h: h['visible_images'] > 0
                     and h['loaded_images'] == h['visible_images'] and h['crosshair'])
    check(report, 'initial_hud_images_loaded', ui.text(hud, str(state['session']['points']))
          and any('hud_tomb_zom_num_' in name for name in hud['images']), ui=hud)
    report['initial_scene'] = {'scene': game['scene'], 'collision_scene': game['collision_scene'],
                               'materials': materials, 'animations': client.call('origins/animations'),
                               'capture': ui.capture(client, output, 'initial-start')}


def initial_controls(client, report, game):
    before = client.reset(difficulty='original')
    before = client.until(lambda s: s['player']['grounded'] and s['player']['action'] == 'Ready')
    for action, field in (('crouch', 'crouched'), ('prone', 'prone')):
        key = binding(client.state(), action)
        batch = client.call('origins/input', {'events': [edge(1, key, True), edge(30, key, False)]})
        after = client.until(lambda s: s['player'][field], timeout=5)
        check(report, 'initial_normal_' + action, after['player'][field], after=after['player'])
        client.until(lambda s: s['fixed_tick'] >= batch['finish_tick']
                     and not s['held_inputs'] and not s['queued_inputs'])
        normal_action(client, 'jump')
        client.until(lambda s: s['player']['grounded'] and not s['player']['prone']
                     and not s['player']['crouched'])
    start = client.state()
    moved = normal_action(client, 'forward', 20)
    check(report, 'initial_normal_walk_on_authored_collision',
          math.dist(start['player']['position'], moved['player']['position']) > .05
          and moved['player']['grounded'] and moved['player']['life'] == 'Standing',
          before=start['player'], after=moved['player'])
    jump_start = client.state()
    normal_action(client, 'jump')
    landed = client.until(lambda s: s['player']['grounded']
                          and any(e.get('event') == 'landed' for e in s['movement']['events']
                                  [len(jump_start['movement']['events']):]))
    check(report, 'initial_normal_jump_lands', landed['player']['life'] == 'Standing',
          events=landed['movement']['events'][len(jump_start['movement']['events']):])
    client.reset(difficulty='original')
    ready = client.until(lambda s: s['player']['grounded'] and s['player']['action'] == 'Ready')
    dt = ready['fixed_delta_seconds']
    start = math.ceil(game['tuning']['mobility']['dive_startup'] / dt) + 8
    forward, sprint, prone = [binding(ready, key) for key in ('forward', 'sprint', 'prone')]
    inputs = [edge(1, forward, True), edge(1, sprint, True), edge(start, prone, True),
              edge(start + 1, prone, False), edge(start + 3, forward, False),
              edge(start + 3, sprint, False)]
    schedule(client, inputs)
    finished = client.until(lambda s: not s['player']['mobility']['busy'])
    events = finished['movement']['events'][len(ready['movement']['events']):]
    check(report, 'initial_normal_sprint_dive_and_landing',
          any(e.get('event') == 'dive_started' for e in events)
          and any(e.get('event') == 'dive_finished' for e in events)
          and finished['player']['grounded'], inputs=inputs, events=events, player=finished['player'])
    check(report, 'initial_controls_release_inputs', not finished['held_inputs']
          and not finished['queued_inputs'])


def initial_combat(client, report, game, output):
    client.reset(difficulty='original')
    fixture = {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300}
    client.call('origins/setup', fixture)
    before = client.until(lambda s: s['player']['grounded'] and s['player']['action'] == 'Ready')
    normal_action(client, 'fire')
    fired = client.until(lambda s: s['session']['shots'] == before['session']['shots'] + 1)
    check(report, 'initial_pistol_consumes_one_round',
          fired['player']['weapons'][0]['magazine'] == before['player']['weapons'][0]['magazine'] - 1,
          fixture=fixture, before=before['player'], after=fired['player'])
    normal_action(client, 'reload')
    after = client.until(lambda s: s['player']['action'] == 'Ready'
                         and s['player']['weapons'][0]['magazine'] == before['player']['weapons'][0]['magazine'])
    check(report, 'initial_pistol_reload_moves_reserve_ammo',
          after['player']['weapons'][0]['reserve'] == before['player']['weapons'][0]['reserve'] - 1,
          after=after['player'])
    hud = client.call('origins/ui')
    check(report, 'initial_ammunition_hud_stays_loaded', hud['loaded_images'] == hud['visible_images'], ui=hud)
    grenade = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == 'weapon.frag_grenade_zm')
    before = client.state()
    normal_action(client, 'lethal')
    launched = client.until(lambda s: s['equipment']['thrown'] > before['equipment']['thrown'])
    blast = client.until(lambda s: s['equipment']['detonations'] > before['equipment']['detonations'],
                         timeout=grenade['fuse_seconds'] + 15)
    events = blast['equipment']['events'][len(before['equipment']['events']):]
    check(report, 'initial_frag_launches_and_detonates_from_input',
          launched['player']['lethal']['count'] == before['player']['lethal']['count'] - 1
          and blast['equipment']['detonations'] == before['equipment']['detonations'] + 1,
          fixture=fixture, events=events, after=blast['player'])
    launches = [event for event in events if event.get('event') == 'launch']
    detonations = [event for event in events if event.get('event') == 'detonate']
    check(report, 'initial_frag_preserves_source_fuse', len(launches) == len(detonations) == 1
          and math.isclose(detonations[0]['flight_seconds'], launches[0]['fuse'],
                           abs_tol=2 * before['fixed_delta_seconds']), events=events)
    report['initial_combat_presentation'] = {'audio': blast['audio'],
                                            'animations': client.call('origins/animations'),
                                            'capture': ui.capture(client, output, 'initial-combat')}


def initial_rounds(client, report, game, output, timeout):
    client.reset(difficulty='original')
    state = client.until(lambda s: s['player']['grounded'] and s['player']['action'] == 'Ready')
    profile = game['rounds']
    expected = {}
    health = profile['health_start']
    for number in range(1, 6):
        multiplier = max(1, number / profile['population_round_divisor'])
        if number >= profile['population_acceleration_round']:
            multiplier *= number * profile['population_acceleration']
        count = int((profile['population_base'] + int(profile['solo_population_increment'] * multiplier))
                    * profile['early_population'][number - 1])
        if number > 1:
            health += int(health * profile['health_multiplier']) if number >= profile['exponential_round'] else profile['health_increase']
        expected[number] = {'population': count, 'health': health}
    deadline = time.monotonic() + timeout
    completed = set()
    round_log = []
    report['initial_rounds'] = round_log
    while time.monotonic() < deadline:
        state = client.state()
        number = state['session']['round']
        finished_five = number == 5 and state['session']['pending'] == 0 and not any(
            enemy.get('health') for enemy in state['enemies'])
        if (state['state'] != 'Playing' and not finished_five) or state['player']['life'] != 'Standing':
            raise RuntimeError('Normal-input survival ended: ' + json.dumps(state['session']))
        if (state['session']['intermission'] or finished_five) and number not in completed:
            admissions = [row for row in state['round_evidence']['admitted'] if row['round'] == number]
            check(report, 'initial_round_' + str(number) + '_population_health_and_kills',
                  len(admissions) == expected[number]['population']
                  and all(row['maximum_health'] == expected[number]['health']
                          and row['spawn_zone'] == 'zone_start' for row in admissions)
                  and state['session']['kills'] == sum(expected[r]['population'] for r in range(1, number + 1))
                  and state['round_evidence']['budget_consumed_on_failure'] == 0,
                  expected=expected[number], admissions=admissions, session=state['session'])
            delay = profile['spawn_seconds']
            for _ in range(1, number):
                delay = delay * profile['spawn_decay'] if delay > profile['minimum_spawn_seconds'] else profile['minimum_spawn_seconds']
            dt = state['fixed_delta_seconds']
            check(report, 'initial_round_' + str(number) + '_source_spawn_cadence',
                  all(math.isclose(row['spawn_delay'], delay, abs_tol=2e-5) for row in admissions)
                  and all(b['time'] - a['time'] >= delay - 2 * dt for a, b in zip(admissions, admissions[1:]))
                  and state['population']['ordinary'] <= profile['ordinary_cap']
                  and state['round_evidence']['peak_ordinary'] <= profile['ordinary_cap'],
                  expected_delay=delay, admissions=admissions, population=state['population'])
            if number > 1:
                check(report, 'initial_round_' + str(number) + '_source_intermission',
                      math.isclose(state['round_evidence']['intermission_seconds'],
                                   profile['between_round_seconds'], abs_tol=2 * dt)
                      and state['round_evidence']['transitions'] == number - 1,
                      evidence=state['round_evidence'])
            round_log.append({'round': number, 'fixed_tick': state['fixed_tick'], 'session': state['session'],
                              'audio': state['audio'], 'barriers': state.get('barriers'),
                              'capture': ui.capture(client, output, 'initial-round-' + str(number))})
            completed.add(number)
            print(json.dumps({'initial_round_completed': number, 'kills': state['session']['kills']}), flush=True)
            if number == 5:
                check(report, 'initial_normal_survival_through_round_five', len(completed) == 5,
                      after=state)
                return
        enemies = [enemy for enemy in state['enemies'] if enemy.get('health')
                   and enemy['ready'] and not enemy['collider_disabled']]
        if not enemies:
            client.wait_ticks(12)
            continue
        player = state['player']
        target = min(enemies, key=lambda enemy: math.dist(player['position'], enemy['position']))
        point = target['position'].copy()
        heads = [region for region in target['hit_regions'] if region['region'] == 'Head']
        if heads:
            point = [(a + b) / 2 for a, b in zip(heads[0]['start'], heads[0]['end'])]
        else:
            point[1] += 1.2
        normal_look(client, game, point)
        state = client.state()
        distance = math.dist(state['player']['position'], target['position'])
        weapon = state['player']['weapons'][0]
        if state['player']['action'] != 'Ready':
            client.wait_ticks(4)
        elif distance < 1.5:
            normal_action(client, 'melee')
            normal_action(client, 'backward', 12)
        elif weapon['magazine']:
            normal_action(client, 'fire')
        elif weapon['reserve']:
            normal_action(client, 'reload')
        elif state['player']['lethal']['count'] and distance > 4:
            normal_action(client, 'lethal')
            normal_action(client, 'backward', 30)
        else:
            normal_action(client, 'forward', 10)
    raise TimeoutError('Normal-input rounds 1-5 timed out; completed ' + str(sorted(completed)))


def initial_life(client, report, game):
    client.reset(difficulty='original')
    before = client.until(lambda s: s['player']['grounded'] and s['player']['action'] == 'Ready')
    check(report, 'initial_revive_requires_purchase', not before['player']['perks']
          and before['player']['solo_charges'] == 0, player=before['player'])
    frag = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == 'weapon.frag_grenade_zm')
    injuries = []
    for index in range(2):
        state = client.until(lambda s: s['player']['lethal']['state'] == 'Ready')
        end = math.ceil((frag['prime_seconds'] + frag['fuse_seconds'] + 1)
                        / state['fixed_delta_seconds'])
        key = binding(state, 'lethal')
        client.call('origins/input', {'events': [edge(1, key, True), edge(end, key, False)]})
        injured = client.until(lambda s: s['equipment']['cooked_off'] > state['equipment']['cooked_off']
                               and (s['player']['health'] != state['player']['health']
                                    or s['player']['life'] != state['player']['life']), timeout=30)
        client.release()
        injuries.append(injured)
        if index == 0:
            check(report, 'initial_first_frag_self_hit_is_nonlethal', injured['state'] == 'Playing'
                  and injured['player']['life'] == 'Standing'
                  and 0 < injured['player']['health'] < state['player']['health'], after=injured)
    ended = client.until(lambda s: s['state'] == 'GameOver', timeout=10)
    check(report, 'initial_unpurchased_revive_does_not_admit_downed',
          ended['player']['life'] == 'HumanTerminal' and ended['session']['downs'] == 1,
          injuries=injuries, after=ended)
    restart = binding(ended, 'restart')
    client.call('origins/input', {'events': [edge(1, restart, True), edge(2, restart, False)]})
    restarted = client.until(lambda s: s['state'] == 'Playing' and s.get('player')
                             and s['session_id'] > ended['session_id']
                             and s['player']['entity'] != ended['player']['entity'], timeout=120)
    check(report, 'initial_normal_restart_clears_session', restarted['session']['round'] == 1
          and restarted['session']['points'] == 500 and restarted['session']['kills'] == 0
          and restarted['session']['downs'] == 0 and not restarted['held_inputs']
          and not restarted['queued_inputs'] and restarted['player']['health'] == game['player_health']['maximum']
          and not restarted['player']['perks'], after=restarted)
    report['initial_life_limitations'] = ['Quick Revive purchase/revival still requires a powered authored station case.',
                                         'Ordinary solo automatic revival precedes source bleedout; no unreachable bleedout claim is made.']


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--assets', required=True, type=Path)
    parser.add_argument('--research', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--inspection', type=Path)
    parser.add_argument('--no-captures', action='store_true')
    parser.add_argument('--fail-fast', action='store_true')
    parser.add_argument('--milestone', choices=('full', 'initial'), default='full')
    parser.add_argument('--round-timeout', type=float, default=1200)
    parser.add_argument('--section', action='append', choices=(
        'ui', 'session', 'projectiles', 'physics', 'mobility', 'dive', 'pistols',
        'instakill', 'environment', 'stations', 'landing', 'quests', 'animation', *INITIAL_SECTIONS))
    args = parser.parse_args()
    output = args.output.resolve()
    if output.is_relative_to(Path(__file__).resolve().parents[1]):
        raise ValueError('Validation output must be external')
    output.mkdir(parents=True, exist_ok=True)
    client = Client(args.url, request_timeout=20, shutdown_timeout=45)
    report = {'passed': False, 'milestone': args.milestone, 'cases': [], 'sections': [], 'limitations': ['Fixtures establish preconditions; scheduled input and observed outcomes establish behavior.', 'This bounded integration suite does not establish exhaustive original-game parity or performance.', 'Read-only HUD/audio/animation observations and captures require separate visual and audible review.']}
    selected = args.section or (list(INITIAL_SECTIONS) + ['session'] if args.milestone == 'initial' else
                               ['ui', 'session', 'projectiles', 'physics', 'mobility',
                                'dive', 'pistols', 'instakill', 'environment', 'stations', 'landing', 'quests', 'animation'])
    site = None
    try:
        if args.milestone == 'initial' and any(name not in (*INITIAL_SECTIONS, 'session') for name in selected):
            raise ValueError('Initial milestone sections must stay within initial content')
        if not math.isfinite(args.round_timeout) or args.round_timeout <= 0:
            raise ValueError('Round timeout must be positive and finite')
        game = json.loads(asset_bytes(Path(args.assets.resolve()) / 'game.json'))
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), 300)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        for name in selected:
            section = {'name': name, 'passed': False, 'first_case': len(report['cases'])}
            report['sections'].append(section)
            print(json.dumps({'section': name, 'status': 'running'}), flush=True)
            try:
                if name != 'initial_start':
                    client.reset()
                if name in ('dive', 'landing', 'animation') and report.get('floor') is None:
                    candidates, _ = physics.surfaces(Path(args.assets.resolve()), game)
                    site = physics.select_flat(client, report, game, candidates,
                                               native_tuning(args.research)[0])
                    report['floor'] = site
                capture_output = None if args.no_captures else output
                if name == 'initial_start':
                    initial_start(client, report, game, capture_output)
                elif name == 'initial_controls':
                    initial_controls(client, report, game)
                elif name == 'initial_combat':
                    initial_combat(client, report, game, capture_output)
                elif name == 'initial_rounds':
                    initial_rounds(client, report, game, capture_output, args.round_timeout)
                elif name == 'initial_life':
                    initial_life(client, report, game)
                elif name == 'ui':
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
                try:
                    client.release()
                except Exception as error:
                    section['passed'] = False
                    section['cleanup_error'] = str(error)
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
