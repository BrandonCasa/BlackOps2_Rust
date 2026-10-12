import argparse
import json
import math
from pathlib import Path
import struct

from test_support import Client, write_report
from test_support import source_fall_perks as native_perks, source_gravity as native_physics, source_mobility as native_tuning
from test_support import source_physics as native_motor
from test_mobility import prepare
from test_throwable_down import binding, check, edge, schedule


def events(state, name):
    return [event for event in state['movement']['events'] if event['event'] == name]


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def dive_inputs(state, native):
    forward, sprint, prone = [binding(state, name) for name in ('forward', 'sprint', 'prone')]
    start = math.ceil(native['dive_startup'] / state['fixed_delta_seconds']) + 8
    return [edge(1, forward, True), edge(1, sprint, True), edge(start, prone, True),
            edge(start + 1, prone, False), edge(start + 2, forward, False),
            edge(start + 2, sprint, False)]


def prepare_floor(client, floor):
    before, fixture = prepare(client)
    if floor:
        fixture.update(position=floor['position'], yaw=0,
                       open_connections=floor['open_connections'])
        client.call('origins/setup', fixture)
    before = client.until(lambda state: state['player']['grounded']
                          and not state['player']['mobility']['airborne']
                          and state['player']['mobility']['jump_recovery'] == 0
                          and state['player']['action'] == 'Ready')
    return before, fixture


def flat_motion(client, report, native, physics, floor):
    before, fixture = prepare_floor(client, floor)
    jump = binding(before, 'jump')
    baseline = len(events(before, 'landed'))
    inputs = [edge(1, jump, True), edge(2, jump, False)]
    schedule(client, inputs)
    after = client.until(lambda state: len(events(state, 'landed')) > baseline
                         and not state['player']['mobility']['airborne'])
    motion = after['movement']['motion'][len(before['movement']['motion']):]
    first = motion[0]
    height = max(sample['feet_after'][1] for sample in motion) - first['feet_before'][1]
    check(report, 'normal_jump_uses_native_launch_and_height',
          math.isclose(first['vertical_before_gravity'], physics['jump_speed'], abs_tol=.0001)
          and abs(height - native['dive_jump_height']) < .015,
          fixture=fixture, inputs=inputs, height=height, motion=motion,
          landing=events(after, 'landed'), player=after['player'])

    before, fixture = prepare_floor(client, floor)
    inputs = dive_inputs(before, native)
    schedule(client, inputs)
    after = client.until(lambda state: bool(events(state, 'dive_finished'))
                         and not state['player']['mobility']['busy'])
    motion = [sample for sample in after['movement']['motion'] if sample['diving']]
    launch = math.sqrt(2 * native['dive_jump_height'] * physics['gravity']) * native['dive_launch_multiplier']
    check(report, 'dive_uses_native_launch_velocity',
          math.isclose(motion[0]['vertical_before_gravity'], launch, abs_tol=.0001),
          expected=launch, first=motion[0], fixture=fixture, inputs=inputs)
    airborne = [sample for sample in motion if sample['slide_remaining'] is None]
    errors = []
    for sample in airborne[1:]:
        predicate = sample['feet_before'][1] >= sample['launch_height'] + native['dive_jump_height']
        expected = (0 if sample['elapsed'] < native['dive_apex_seconds']
                    else min(sample['velocity_before'][1], 0)) if predicate else sample['velocity_before'][1]
        if (sample['apex_clamped'] != predicate
                or not math.isclose(sample['vertical_before_gravity'], expected, abs_tol=.0001)):
            errors.append(sample)
    check(report, 'apex_uses_height_predicate_and_launch_relative_deadline',
          not errors and any(sample['apex_clamped'] for sample in airborne)
          and any(sample['elapsed'] >= native['dive_apex_seconds']
                  and sample['velocity_after'][1] < -.1 for sample in airborne),
          errors=errors, motion=motion, events=after['movement']['events'])
    motor_samples = {sample['seconds']: sample for sample in after['movement']['physics']}
    free = [sample for sample in airborne
            if motor_samples[sample['seconds']]['ground_normal'] is None
            and math.dist(sample['move_velocity'][::2], sample['velocity_after'][::2]) < .0001
            and abs(sample['move_velocity'][1] - sample['velocity_after'][1]
                    - sample['gravity_step'] * .5) < .0001]
    errors = [sample for sample in free
              if abs(sample['feet_after'][1] - sample['feet_before'][1]
                     - sample['move_velocity'][1] * sample['dt']) > .0001
              or abs(sample['gravity_step'] - physics['gravity'] * sample['dt']) > .0001]
    check(report, 'free_dive_motion_uses_native_midpoint_gravity',
          len(free) >= 12 and not errors, samples=len(free), errors=errors)
    slide = [sample for sample in motion if sample['slide_remaining'] is not None
             and motor_samples[sample['seconds']]['grounded']]
    check(report, 'grounded_dive_keeps_native_air_solver_gravity',
          len(slide) >= 3 and all(abs(sample['gravity_step'] - physics['gravity'] * sample['dt']) < .0001
                                  for sample in slide), samples=slide)
    errors = []
    for sample in slide:
        speed = math.hypot(*sample['velocity_before'][::2])
        retained = (max(0, speed - max(speed, 2.54) * 5.5
                        * (1 - sample['slide_remaining'] / native['dive_slide']) * sample['dt']) / speed
                    if speed >= .0254 else 0)
        expected = [value * retained for value in sample['velocity_before']]
        normal = motor_samples[sample['seconds']]['ground_normal']
        normal_speed = sum(value * axis for value, axis in zip(expected, normal))
        expected = [value - axis * (normal_speed - abs(normal_speed) * .001)
                    for value, axis in zip(expected, normal)]
        before_gravity = sample['move_velocity'].copy()
        before_gravity[1] += sample['gravity_step'] * .5
        if math.dist(before_gravity, expected) > .001:
            errors.append({'sample': sample, 'expected_velocity': expected})
    check(report, 'ground_slide_preserves_native_time_scaled_friction',
          len(slide) >= 3 and not errors, samples=len(slide), errors=errors)


def fall_site(client, report, game):
    before, fixture = prepare(client)
    baseline = len(events(before, 'landed'))
    surface = [-12.267935753, 1.016000032, -68.628486633]
    fixture.update(owned_perks=['perk.phd_flopper'],
                   open_connections=[door['connection'] for door in game['access']['doors']],
                   position=[surface[0], surface[1] + 20, surface[2]])
    client.call('origins/setup', fixture)
    after = client.until(lambda state: len(events(state, 'landed')) > baseline
                         and not state['player']['mobility']['airborne'], timeout=45)
    feet = after['player']['position'].copy()
    feet[1] -= after['player']['height'] * .5
    check(report, 'find_authored_surface_with_clear_overhead_for_fall_fixtures',
          after['player']['health'] == before['player']['health']
          and after['player']['life'] == 'Standing'
          and abs(feet[1] - surface[1]) < .15
          and events(after, 'landed')[-1]['impact_height'] > 8,
          fixture=fixture, surface=surface, observed_feet=feet, landing=events(after, 'landed')[-1],
          geometry={'node': 2729, 'triangle': 8, 'material': 'wpc/zm_tm_concrete_bunker01_bare'})
    return surface


def held_slide(client, report, native, motor, floor):
    before, fixture = prepare_floor(client, floor)
    client.call('origins/setup', {'yaw': 0})
    before = client.wait_ticks(1)
    inputs = dive_inputs(before, native)
    start = inputs[2]['tick']
    right = binding(before, 'right')
    inputs += [edge(start + math.ceil(.5 / before['fixed_delta_seconds']), right, True),
               edge(start + math.ceil(1.2 / before['fixed_delta_seconds']), right, False)]
    after = schedule(client, inputs)
    physics = {sample['seconds']: sample for sample in after['movement']['physics']}
    rows = [sample for sample in after['movement']['motion']
            if sample['diving'] and sample['slide_remaining'] is not None
            and physics[sample['seconds']]['grounded']
            and physics[sample['seconds']]['input'] == [1, 0]]
    errors = []
    additions = []
    tuning = motor['physics']
    for sample in rows:
        old = sample['velocity_before']
        speed = math.hypot(*old[::2])
        retained = (max(0, speed - max(speed, tuning['stop_speed']) * tuning['friction']
                        * (1 - sample['slide_remaining'] / native['dive_slide']) * sample['dt']) / speed
                    if speed >= .0254 else 0)
        expected = [value * retained for value in old]
        wish_speed = motor['walk_speed'] * .4
        addition = max(0, min(wish_speed - expected[0], tuning['air_acceleration']
                              * max(wish_speed, tuning['stop_speed']) * sample['dt']))
        expected[0] += addition
        additions.append(addition)
        normal = physics[sample['seconds']]['ground_normal']
        normal_speed = sum(value * axis for value, axis in zip(expected, normal))
        expected = [value - axis * (normal_speed - abs(normal_speed) * .001)
                    for value, axis in zip(expected, normal)]
        if math.dist(expected[::2], sample['move_velocity'][::2]) > .001:
            errors.append({'sample': sample, 'expected_velocity': expected})
    check(report, 'grounded_dive_keeps_native_air_command_scale_and_acceleration',
          len(rows) >= 4 and any(addition > 0 for addition in additions) and not errors,
          fixture=fixture, inputs=inputs, samples=rows, errors=errors)


def fall_case(client, report, native, game, base, height, diving, perks):
    before, fixture = prepare(client)
    if perks:
        fixture['owned_perks'] = perks
        client.call('origins/setup', {'owned_perks': perks})
        before = client.wait_ticks(3)
    if diving:
        inputs = dive_inputs(before, native)
        schedule(client, inputs)
        before = client.until(lambda state: state['player']['mobility']['diving'])
    else:
        inputs = []
    position = [base[0], base[1] + height + before['player']['height'] * .5, base[2]]
    fixture['position'] = position
    fixture['open_connections'] = [door['connection'] for door in game['access']['doors']]
    client.call('origins/setup', {'position': position, 'open_connections': fixture['open_connections']})
    baseline = len(events(before, 'landed'))
    after = client.until(lambda state: len(events(state, 'landed')) > baseline, timeout=45)
    landing = events(after, 'landed')[-1]
    minimum, fatal = ((native['dive_fall_minimum'], native['dive_fall_fatal']) if diving
                      else (game['tuning']['mobility'].get('fall_minimum', 128 * .0254),
                            game['tuning']['mobility'].get('fall_fatal', 300 * .0254)))
    percent = int(max(0, min(1, (landing['impact_height'] - minimum) / (fatal - minimum))) * 100)
    if percent > 0 and 'perk.phd_flopper' in perks:
        percent = 1
    fraction = f32(1.1) if percent == 100 else f32(percent * f32(.01))
    maximum = max([game['player_health']['maximum']] + [perk['maximum_health']
                  for perk in game['content']['perks']
                  if perk['id'] in perks and perk.get('maximum_health') is not None])
    expected = int(f32(maximum * fraction))
    impacts = [event for event in after['effects']['events'] if event.get('event') == 'fall_impact']
    if expected:
        check(report, f'fall_conversion_{diving}_{height}_{perks}',
              impacts and impacts[-1]['percent'] == percent and impacts[-1]['amount'] == expected,
              fixture=fixture, inputs=inputs, landing=landing, expected_percent=percent,
              expected_amount=expected, impacts=impacts)
    protected = any(perk in perks for perk in ('perk.phd_flopper',))
    protection = (after['player']['health'] == before['player']['health']
                  and any(event.get('event') == 'fall_protected' and event['amount'] == expected
                          for event in after['effects']['events'])
                  and any(event.get('event') == 'dive_explosion' for event in after['effects']['events']))
    check(report, f'actual_fall_outcome_{diving}_{height}_{perks}',
          landing['diving'] == diving
          and abs(landing['impact_height'] - height) < .2
          and (protection if protected
               else after['player']['life'] == 'HumanTerminal' if percent == 100
               else after['player']['health'] == before['player']['health'] - expected),
          fixture=fixture, landing=landing, before=before['player'], after=after['player'],
          health=after.get('health'), damage=after['damage'], effects=after['effects'])


def wall_dive(client, report, native, game):
    connections = [door['connection'] for door in game['access']['doors']]
    attempts = []
    for position in ([67.0, -8.20, -135.8], [65.0, -8.40, -134.0], [67.0, -8.30, -134.5]):
        before, fixture = prepare(client)
        fixture.update(position=position, yaw=0, open_connections=connections)
        client.call('origins/setup', fixture)
        before = client.until(lambda state: state['player']['grounded']
                              and not state['player']['mobility']['airborne']
                              and state['player']['action'] == 'Ready', timeout=5)
        inputs = dive_inputs(before, native)
        entered = schedule(client, inputs)
        attempt = {'fixture': fixture, 'inputs': inputs, 'entry_player': entered['player'],
                   'events': entered['movement']['events']}
        attempts.append(attempt)
        if not events(entered, 'dive_started'):
            continue
        after = client.until(lambda state: bool(events(state, 'dive_finished'))
                             and not state['player']['mobility']['busy'], timeout=8)
        motion = [sample for sample in after['movement']['motion'] if sample['diving']]
        contacts = []
        for sample, following in zip(motion, motion[1:]):
            intended = math.hypot(*sample['move_velocity'][::2])
            projected = math.hypot(*sample['velocity_after'][::2])
            if (intended - projected > .1 and sample['elapsed'] > 3 * sample['dt']
                    and abs(sample['feet_after'][2] + 140.8176) < .8):
                contacts.append({'contact': sample, 'next': following,
                                 'retained': math.hypot(*following['move_velocity'][::2]) <= projected + .001})
        attempt.update(contacts=contacts, motion=motion, events=after['movement']['events'])
        if contacts:
            check(report, 'authored_wall_projection_survives_next_dive_tick',
                  all(contact['retained'] for contact in contacts), attempts=attempts,
                  authored_wall=[position[0], -8.0, -140.8176],
                  fixture_note='Grounded interior approaches avoid the spawn rubble; normal sprint and prone inputs enter the dive before the authored wall.')
            return
    check(report, 'authored_wall_projection_survives_next_dive_tick', False, attempts=attempts)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    parser.add_argument('--research', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url, shutdown_timeout=45)
    report = {'passed': False, 'cases': [], 'scenario': 'native_dive_physics',
              'limitations': ['Falls use airborne position fixtures after normal-input dive entry.',
                              'Fixture ledges do not establish authored ledge traversal or subjective feel.',
                              'Avian contacts and fixed ticks reconstruct the native collision solver.']}
    try:
        names = ('test_dive_physics.py', 'test_mobility.py',
                 'test_throwable_down.py', 'test_support.py', 'test_brp.py')
        native, physics = native_tuning(args.research), native_physics(args.research)
        motor = native_motor(args.research)[0]
        report['native'] = {'mobility': native, 'physics': physics, 'motor': motor,
                            'perks': native_perks(args.research)}
        game = json.loads((Path(args.assets.resolve()) / 'game.json').read_bytes())
        check(report, 'pack_preserves_native_vertical_physics',
              all(math.isclose(game['tuning'][key], value, abs_tol=.000001)
                  for key, value in physics.items()), pack=game['tuning'], expected=physics)
        client.until(lambda state: state['state'] in ('Playing', 'Paused', 'GameOver')
                     and state.get('player'), timeout=240)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        floor = None
        flat_motion(client, report, native, physics, floor)
        held_slide(client, report, native, motor, floor)
        base = fall_site(client, report, game)
        for height, diving, perks in ((1.5, True, []), (2.5, True, []), (5.4, True, []),
                                      (2.5, True, ['perk.juggernog']),
                                      (5.4, True, ['perk.phd_flopper']),
                                      (2.5, False, []), (4.5, False, []), (8, False, [])):
            fall_case(client, report, native, game, base, height, diving, perks)
        wall_dive(client, report, native, game)
        report['passed'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {error}'
    finally:
        try:
            client.release()
        except Exception as error:
            report['release_error'] = f'{type(error).__name__}: {error}'
        finally:
            client.shutdown(report)
        path = write_report(args.output, 'dive-physics.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
