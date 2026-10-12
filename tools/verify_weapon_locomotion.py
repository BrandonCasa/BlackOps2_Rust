import argparse
import json
import math
from pathlib import Path

from gameplay_remote import Client, write_report
from verify_animation_fixes import observe, pose_distance, weapon_rigs
from verify_brp import assets_match, digest
from verify_throwable_down import binding, check, edge, schedule


def action_name(action):
    return ''.join(part.title() for part in action.split('_'))


def source_bindings(report, game, pack, inspection, preparation):
    recovered = inspection / 'solo-content/recovered-gameplay.json'
    if digest(recovered) != preparation['source']['sha256']:
        raise RuntimeError('Recovered weapon fields differ from the preparation receipt')
    fields = {'weapon.' + row['source']: row['fields']
              for row in json.loads(recovered.read_bytes())['weapons']}
    presentations = {row['id']: row for row in game['content']['presentations']}
    actions, timings = preparation['action_fields'], preparation['timing_fields']
    scenes = {}
    count = 0
    for weapon in game['content']['weapons']:
        native = fields[weapon['id']]
        for timing, names in timings.items():
            if weapon['mechanics'][timing] != [float(native.get(name) or 0) for name in names]:
                raise RuntimeError('Native locomotion timing mismatch: ' + weapon['id'])
        for action, field in actions.items():
            clip = native.get(field)
            if not clip or not weapon.get('presentation'):
                continue
            identity = weapon['animation_actions'].get(action, '')
            if identity.rsplit('.', 1)[-1] != clip:
                raise RuntimeError('Native action binding mismatch: ' + weapon['id'] + ':' + action)
            rigs = [weapon.get(mode) for mode in ('presentation', 'ads_presentation')]
            rigs += ['actor.c_zom_' + name + '_viewhands'
                     for name in ('dempsey', 'nikolai', 'richtofen', 'takeo')]
            for rig in filter(None, rigs):
                relative = presentations[rig]['scene'].split('#')[0]
                if relative not in scenes:
                    scenes[relative] = {animation['name'] for animation in
                                        json.loads((pack / relative).read_bytes()).get('animations', [])}
                if clip not in scenes[relative]:
                    raise RuntimeError('Prepared rig lacks native clip: ' + rig + ':' + clip)
                count += 1
    check(report, 'all_weapon_locomotion_fields_and_rig_bindings_match_recovered_source', True,
          weapons=len(game['content']['weapons']), rig_files=len(scenes), checked_bindings=count,
          recovered={'path': str(recovered), 'sha256': digest(recovered)})


def freeze(client, state, held, ticks):
    ticks = max(4, ticks)
    events = []
    if state['state'] == 'Paused':
        events += [edge(1, 'Escape', True), edge(2, 'Escape', False)]
    for action in held:
        events += [edge(1, binding(state, action), True),
                   edge(ticks + 2, binding(state, action), False)]
    events += [edge(ticks, 'Escape', True), edge(ticks + 1, 'Escape', False)]
    result = schedule(client, events)
    if result['state'] != 'Paused':
        raise RuntimeError('Scheduled animation observation did not pause the game')
    return result, events


def prepare(client, weapon, empty, floor=None):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': [weapon], 'pitch': 0, 'owned_perks': []}
    if floor:
        fixture.update(position=floor['position'], yaw=0,
                       open_connections=floor['open_connections'])
    client.call('origins/setup', fixture)
    state = client.until(lambda s: s['player']['action'] == 'Ready'
                         and not s['player']['mobility']['airborne'])
    observe(client, lambda o: len(weapon_rigs(o, weapon)) >= 2
            and all(r['ready'] and r['clocks'] for r in weapon_rigs(o, weapon)), 90)
    if empty:
        active = next(w for w in state['player']['weapons'] if w['slot'] == state['player']['active_slot'])
        interval = 30
        events = [event for tick in range(1, active['magazine'] * interval + 1, interval)
                  for event in (edge(tick, binding(state, 'fire'), True),
                                edge(tick + 1, binding(state, 'fire'), False))]
        state = schedule(client, events)
        state = client.until(lambda s: s['player']['action'] == 'Ready')
        if next(w for w in state['player']['weapons'] if w['slot'] == state['player']['active_slot'])['magazine']:
            raise RuntimeError('Normal firing did not empty the weapon')
        fixture['emptying_inputs'] = events
    return state, fixture


def dive_case(client, report, game, identity, empty, clips, floor):
    weapon = next(w for w in game['content']['weapons'] if w['id'] == identity)
    state, fixture = prepare(client, identity, empty, floor)
    dt = state['fixed_delta_seconds']
    start = math.ceil(game['tuning']['mobility']['dive_startup'] / dt) + 8

    def events(state, end, pause):
        result = [edge(1, binding(state, action), True) for action in ('forward', 'sprint')]
        result += [edge(start, binding(state, 'prone'), True),
                   edge(start + 1, binding(state, 'prone'), False)]
        result += [edge(start + 2, binding(state, action), False) for action in ('forward', 'sprint')]
        if pause:
            result += [edge(end, 'Escape', True), edge(end + 1, 'Escape', False)]
        return result

    inputs = events(state, start + math.ceil(2 / dt), False)
    schedule(client, inputs)
    probe = client.until(lambda s: any(e['event'] == 'dive_finished' for e in s['movement']['events'])
                         and not s['player']['mobility']['busy'])
    launch = next(e for e in probe['movement']['events'] if e['event'] == 'dive_started')
    landing = next(e for e in probe['movement']['events'] if e['event'] == 'landed' and e['diving'])
    landing_elapsed = landing['seconds'] - launch['seconds']
    check(report, identity + '_find_dive_landing_' + str(empty), 0 < landing_elapsed < 2,
          fixture=fixture, inputs=inputs, launch=launch, landing=landing)
    state, fixture = prepare(client, identity, empty, floor)
    capture = start + max(2, math.ceil(weapon['mechanics']['dive_seconds'][0] * .5 / dt))
    inputs = events(state, capture, True)
    state = schedule(client, inputs)
    suffix = '_empty' if empty else ''
    prefix = identity + suffix
    inspect_phase(client, report, weapon, 'dive_enter' + suffix, state, inputs, prefix + '_dive_enter', clips)
    state, inputs = freeze(client, state, [], math.ceil(weapon['mechanics']['dive_seconds'][0] / dt) + 4)
    inspect_phase(client, report, weapon, 'dive_loop' + suffix, state, inputs, prefix + '_dive_loop', clips)
    elapsed = state['movement']['motion'][-1]['elapsed']
    remaining = landing_elapsed - elapsed + weapon['mechanics']['dive_seconds'][2] * .5
    state, inputs = freeze(client, state, [], math.ceil(remaining / dt))
    observation = inspect_phase(client, report, weapon, 'dive_leave' + suffix, state, inputs,
                                prefix + '_dive_leave_on_landing', clips)
    check(report, prefix + '_dive_exit_starts_during_ground_slide',
          state['player']['mobility']['diving'] and state['player']['mobility']['slide_remaining'] is not None
          and all(r['locomotion']['landed'] for r in weapon_rigs(observation, identity)),
          inputs=inputs, player=state['player'])
    state, inputs = freeze(client, state, [], math.ceil(1 / dt))
    idle = observe(client, lambda o: all(r['action'] == ('IdleEmpty' if empty else 'Idle')
                   for r in weapon_rigs(o, identity)) and len(weapon_rigs(o, identity)) >= 2)
    check(report, prefix + '_dive_exit_returns_to_prone_idle', state['player']['prone']
          and not state['player']['mobility']['busy']
          and all(r['locomotion']['phase'] is None for r in weapon_rigs(idle, identity)),
          inputs=inputs, player=state['player'], animation=idle)


def inspect_phase(client, report, weapon, expected, state, inputs, label, clips):
    observation = observe(client, lambda o: all(r['action'] == action_name(expected)
                          for r in weapon_rigs(o, weapon['id']))
                          and len(weapon_rigs(o, weapon['id'])) >= 2, 10)
    rigs = weapon_rigs(observation, weapon['id'])
    clip = weapon['animation_actions'][expected].rsplit('.', 1)[-1]
    check(report, label + '_native_clip', all(r['bones'] and r['locomotion'] and r['clocks']
          and all(clock['clip'] == clip for clock in r['clocks']) for r in rigs),
          inputs=inputs, player=state['player'], animation=observation)
    phase = expected.removesuffix('_empty')
    looping = phase in ('prone_loop', 'prone_back', 'prone_right', 'prone_left', 'dive_loop')
    errors = []
    for rig in rigs:
        locomotion = rig['locomotion']
        duration = (weapon['mechanics']['crawl_seconds'][5] if phase == 'prone_leave'
                    else locomotion['duration']) or clips[clip]
        elapsed = locomotion['elapsed'] % duration if looping else min(locomotion['elapsed'], duration)
        seconds = elapsed / duration * clips[clip]
        if locomotion['phase'] != action_name(phase) or any(
                abs(clock['seconds'] - seconds) > 1e-4 for clock in rig['clocks']):
            errors.append({'rig': rig, 'expected_seconds': seconds})
    check(report, label + '_native_duration_samples_actual_clip', not errors, errors=errors)
    before = rigs
    client.wait_ticks(12)
    after = weapon_rigs(client.call('origins/animations'), weapon['id'])
    check(report, label + '_pause_freezes_pose_and_fixed_locomotion_clock',
          [r['clocks'] for r in before] == [r['clocks'] for r in after]
          and [r['locomotion'] for r in before] == [r['locomotion'] for r in after]
          and pose_distance(before, after) < 1e-5)
    return observation


def crawl_case(client, report, game, identity, empty, clips):
    weapon = next(w for w in game['content']['weapons'] if w['id'] == identity)
    state, fixture = prepare(client, identity, empty)
    dt = state['fixed_delta_seconds']
    inputs = [edge(1, binding(state, 'prone'), True), edge(2, binding(state, 'prone'), False)]
    state = schedule(client, inputs)
    state = client.wait_ticks(math.ceil(.5 / dt))
    idle = observe(client, lambda o: all(r['action'] == ('IdleEmpty' if empty else 'Idle')
                   for r in weapon_rigs(o, identity)) and len(weapon_rigs(o, identity)) >= 2)
    check(report, identity + '_stationary_prone_keeps_idle_' + str(empty),
          state['player']['prone'] and all(r['locomotion']['phase'] is None
                                         for r in weapon_rigs(idle, identity)),
          fixture=fixture, inputs=inputs, player=state['player'], animation=idle)
    suffix = '_empty' if empty else ''
    prefix = identity + suffix
    state, inputs = freeze(client, state, ['forward'], math.ceil(weapon['mechanics']['crawl_seconds'][0] * .5 / dt))
    inspect_phase(client, report, weapon, 'prone_enter' + suffix, state, inputs, prefix + '_enter', clips)
    for direction, action, index in (('forward', 'prone_loop', 1), ('backward', 'prone_back', 2),
                                     ('right', 'prone_right', 3), ('left', 'prone_left', 4)):
        duration = weapon['mechanics']['crawl_seconds'][index]
        ticks = math.ceil(duration * .5 / dt) + 5
        if direction == 'forward':
            ticks += math.ceil(weapon['mechanics']['crawl_seconds'][0] / dt)
        state, inputs = freeze(client, state, [direction], ticks)
        inspect_phase(client, report, weapon, action + suffix, state, inputs, prefix + '_' + direction, clips)
    state, inputs = freeze(client, state, [], math.ceil(weapon['mechanics']['crawl_seconds'][5] * .5 / dt))
    inspect_phase(client, report, weapon, 'prone_leave' + suffix, state, inputs, prefix + '_leave', clips)
    state, inputs = freeze(client, state, [], math.ceil(weapon['mechanics']['crawl_seconds'][5] / dt) + 8)
    idle = observe(client, lambda o: all(r['action'] == ('IdleEmpty' if empty else 'Idle')
                   for r in weapon_rigs(o, identity)) and len(weapon_rigs(o, identity)) >= 2)
    check(report, prefix + '_crawl_exit_returns_to_prone_idle', state['player']['prone']
          and all(r['locomotion']['phase'] is None for r in weapon_rigs(idle, identity)),
          inputs=inputs, animation=idle)


def crawl_fire_case(client, report, game, identity, clips):
    weapon = next(w for w in game['content']['weapons'] if w['id'] == identity)
    state, fixture = prepare(client, identity, False)
    dt = state['fixed_delta_seconds']
    prone = binding(state, 'prone')
    state = schedule(client, [edge(1, prone, True), edge(2, prone, False)])
    state, movement_inputs = freeze(client, state, ['forward'],
                                    math.ceil(weapon['mechanics']['crawl_seconds'][0] / dt) + 12)
    inspect_phase(client, report, weapon, 'prone_loop', state, movement_inputs,
                  identity + '_fire_setup', clips)
    fire, forward = binding(state, 'fire'), binding(state, 'forward')
    inputs = [edge(1, 'Escape', True), edge(2, 'Escape', False), edge(1, forward, True),
              edge(4, fire, True), edge(6, 'Escape', True), edge(7, 'Escape', False),
              edge(8, fire, False), edge(8, forward, False)]
    before = state
    state = schedule(client, inputs)
    observation = inspect_phase(client, report, weapon, 'prone_leave', state, inputs,
                                identity + '_fire_interrupt', clips)
    check(report, identity + '_crawl_fire_uses_short_native_exit_gate',
          state['session']['shots'] == before['session']['shots']
          and all(r['locomotion']['queued_fire'] and math.isclose(r['locomotion']['duration'],
                  weapon['mechanics']['crawl_seconds'][6], abs_tol=1e-6)
                  for r in weapon_rigs(observation, identity)),
          fixture=fixture, inputs=inputs, before=before['player'], player=state['player'])
    state, inputs = freeze(client, state, ['fire'],
                          math.ceil(weapon['mechanics']['crawl_seconds'][6] / dt) + 12)
    check(report, identity + '_normal_fire_resumes_after_crawl_exit',
          state['session']['shots'] > before['session']['shots'] and state['player']['prone'],
          inputs=inputs, before_shots=before['session']['shots'], after_shots=state['session']['shots'])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--identity', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--inspection', required=True, type=Path)
    parser.add_argument('--preparation', required=True, type=Path)
    parser.add_argument('--physics-report', required=True, type=Path)
    args = parser.parse_args()
    client = Client(args.url, shutdown_timeout=45)
    report = {'passed': False, 'cases': [], 'scenario': 'native_weapon_locomotion',
              'limitations': ['Weapon grants, enemy removal and spawn delay are scenario setup.',
                              'Movement, prone, empty magazines and pause use scheduled normal input.',
                              'Runtime crawl and dive coverage is MP40; all weapon bindings use prepared asset inspection.',
                              'The borrowed floor fixture requires matching collision assets; generic motor parity is a separate check.']}
    try:
        identity = json.loads(args.identity.read_bytes())
        report['identity'] = {'path': str(args.identity.resolve()), 'sha256': digest(args.identity)}
        report['binaries'] = {name: digest(args.identity.parent / name) for name in identity['binaries']}
        if report['binaries'] != identity['binaries']:
            raise RuntimeError('Pinned executable or DLL changed')
        report['assets'] = assets_match(identity)
        report['scripts'] = {name: digest(Path(__file__).with_name(name)) for name in (
            'verify_weapon_locomotion.py', 'prepare_mobility_animations.py', 'verify_animation_fixes.py',
            'verify_brp.py', 'verify_throwable_down.py', 'gameplay_remote.py')}
        pack = Path(identity['asset_root'])
        game = json.loads((pack / 'game.json').read_bytes())
        preparation = json.loads(args.preparation.read_bytes())
        report['preparation'] = {'path': str(args.preparation.resolve()), 'sha256': digest(args.preparation)}
        source_bindings(report, game, pack, args.inspection, preparation)
        physics = json.loads(args.physics_report.read_bytes())
        if not physics['passed']:
            raise RuntimeError('Authored floor evidence must pass')
        previous_identity = json.loads(Path(physics['identity']['path']).read_bytes())
        if digest(Path(physics['identity']['path'])) != physics['identity']['sha256']:
            raise RuntimeError('Authored floor identity changed')
        previous_game = json.loads((Path(previous_identity['asset_root']) / 'game.json').read_bytes())
        for field in ('collision_scene', 'collision_meshes', 'static_model_instances', 'access'):
            if previous_game[field] != game[field]:
                raise RuntimeError('Authored floor collision or access definitions differ')
        scene = game['collision_scene'].split('#')[0]
        scene_doc = json.loads((pack / scene).read_bytes())
        geometry = [scene] + [(Path(scene).parent / buffer['uri']).as_posix()
                              for buffer in scene_doc.get('buffers', [])]
        if any(identity['assets'][name]['sha256'] != previous_identity['assets'][name]['sha256']
               for name in geometry):
            raise RuntimeError('Authored floor geometry differs from the passing report')
        floor = next(case['fixture'] for case in physics['cases']
                     if case['case'] == 'authored_open_floor_supports_native_jump' and case['passed'])
        report['floor_reference'] = {'path': str(args.physics_report.resolve()),
                                     'sha256': digest(args.physics_report), 'geometry': geometry}
        clips = {}
        for row in game['content']['presentations']:
            if row['id'] != next(w['presentation'] for w in game['content']['weapons']
                                 if w['id'] == 'weapon.mp40_zm'):
                continue
            doc = json.loads((pack / row['scene'].split('#')[0]).read_bytes())
            clips = {animation['name']: max(doc['accessors'][sampler['input']]['max'][0]
                                           for sampler in animation['samplers'])
                     for animation in doc['animations']}
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), 240)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        for empty in (False, True):
            crawl_case(client, report, game, 'weapon.mp40_zm', empty, clips)
            dive_case(client, report, game, 'weapon.mp40_zm', empty, clips, floor)
        crawl_fire_case(client, report, game, 'weapon.mp40_zm', clips)
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
        path = write_report(args.output, 'weapon-locomotion.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
