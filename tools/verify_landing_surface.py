import math
from pathlib import Path

from prepare_physics import native_tuning
from verify_brp import digest
from verify_dive_physics import events, fall_site
from verify_mobility import prepare
from verify_throwable_down import binding, check, edge, schedule


def magnitude(vector):
    return math.hypot(vector[0], vector[2])


def ground_velocity(vector, normal):
    if magnitude(vector) == 0:
        return vector.copy()
    tangent = [vector[0], -(vector[0] * normal[0] + vector[2] * normal[2]) / normal[1], vector[2]]
    scale = math.sqrt(sum(v * v for v in vector) / sum(v * v for v in tangent))
    return [v * scale for v in tangent] if scale < 1 or tangent[1] < 0 or vector[1] > 0 else vector.copy()


def expected_landing(height, game, flags, protected):
    tuning = game['tuning']['mobility']
    percent = int(max(0, min(1, (height - tuning.get('fall_minimum', 3.2512))
                              / (tuning.get('fall_fatal', 7.62) - tuning.get('fall_minimum', 3.2512)))) * 100)
    if flags & 1:
        percent = 0
    elif protected and percent:
        percent = 1
    if percent == 0:
        return percent, (.67 if height >= 12 * .0254 else 1), 0
    if percent >= 100 or flags & 2:
        return percent, .67, 0
    milliseconds = min(2000, 500 + 35 * percent)
    scale = .2 if milliseconds > 1499 else .5 - (milliseconds - 500) * .001 * .3
    return percent, scale, milliseconds * .001


def landing_case(client, report, game, base, height, flags=0, protected=False):
    client.call('origins/surfaces', {'flags': None})
    before, fixture = prepare(client)
    fixture.update(position=[base[0], base[1] + height + before['player']['height'] * .5, base[2]],
                   open_connections=[door['connection'] for door in game['access']['doors']],
                   owned_perks=['perk.phd_flopper'] if protected else [], yaw=0)
    client.call('origins/surfaces', {'flags': flags})
    client.call('origins/setup', fixture)
    marker = before['movement']['physics'][-1]['seconds']
    inputs = [edge(1, binding(before, 'forward'), True),
              edge(56, binding(before, 'forward'), False)]
    after = schedule(client, inputs)
    landed = [event for event in events(after, 'landed') if event['seconds'] > marker]
    check(report, f'landing_observed_{height}_{flags}_{protected}', bool(landed),
          fixture=fixture, flags=flags, inputs=inputs, events=landed)
    landing = landed[0]
    check(report, f'landing_fixture_preserves_requested_drop_{height}_{flags}_{protected}',
          abs(landing['impact_height'] - height) < .2,
          requested_height=height, landing=landing, fixture=fixture)
    percent, scale, recovery = expected_landing(landing['impact_height'], game, flags, protected)
    rows = [row for row in after['movement']['physics'] if row['seconds'] > marker]
    landing_row = next(row for row in rows if abs(row['seconds'] - landing['seconds']) < .000001)
    previous = next((row for row in reversed(rows) if row['seconds'] < landing['seconds']), None)
    ratio_ok = previous is None or magnitude(previous['velocity_after']) < .01 or math.dist(
        landing_row['velocity_before'][::2], [value * scale for value in previous['velocity_after'][::2]]) < .002
    check(report, f'native_landing_impact_scale_and_recovery_{height}_{flags}_{protected}',
          landing['percent'] == percent and abs(landing['impact_scale'] - scale) < .00001
          and abs(landing['landing_recovery'] - recovery) < .00001 and ratio_ok,
          expected={'percent': percent, 'scale': scale, 'recovery': recovery},
          landing=landing, previous=previous, first_grounded=landing_row)
    return after, rows


def run(client, report, game, research_path):
    native = native_tuning(research_path)[0]
    report['landing_sources'] = {name: digest(Path(research_path) / 'exports/functions' / name)
                                 for name in ('008b/008bf460.c', '008b/008be7e0.c',
                                              '008b/008bebf0.c', '008b/008b3c10.c',
                                              '008c/008c64b0.c')}
    imported = client.call('origins/surfaces', {'flags': None})
    check(report, 'native_material_surface_flags_reach_map_colliders', imported['colliders'] > 0,
          imported=imported)
    report.setdefault('limitations', []).append(
        'Origins source materials contain no authored slick flag; slick cases explicitly set imported collider flags as scenario setup.')
    try:
        floor = report.get('floor')
        base = fall_site(client, report, game)
        for height, flags, protected in ((.2, 0, False), (2.5, 0, False), (4.5, 0, False),
                                         (6.5, 0, False), (4.5, 0, True), (4.5, 1, False)):
            after, rows = landing_case(client, report, game, base, height, flags, protected)
            if height == 4.5 and flags == 0 and not protected:
                active = [row for row in rows if row['grounded'] and row['landing_recovery'] > 0
                          and row['ground_normal'][1] > .9999]
                errors = []
                for row in active:
                    old = row['velocity_before']
                    speed = magnitude(old)
                    retained = max(0, speed - max(speed, native['physics']['stop_speed'])
                                   * native['physics']['friction'] * .3 * row['dt']) / speed if speed >= .0254 else 0
                    expected = [value * retained for value in old]
                    wish = row['wish']
                    wishspeed = math.sqrt(sum(value * value for value in wish))
                    if wishspeed:
                        direction = [value / wishspeed for value in wish]
                        addition = max(0, wishspeed - sum(a * b for a, b in zip(expected, direction)))
                        addition = min(addition, native['physics']['ground_acceleration'] * .25
                                       * max(wishspeed, native['physics']['stop_speed']) * row['dt'])
                        expected = [value + axis * addition for value, axis in zip(expected, direction)]
                    if math.dist(expected[::2], row['move_velocity'][::2]) > .002:
                        errors.append({'sample': row, 'expected': expected})
                check(report, 'native_landing_recovery_reduces_ground_friction_and_acceleration',
                      len(active) > 6 and not errors, samples=len(active), errors=errors)
                cleared = client.until(lambda state: state['player']['mobility']['landing_recovery'] == 0)
                check(report, 'landing_movement_penalty_expires',
                      cleared['player']['mobility']['landing_recovery'] == 0,
                      player=cleared['player'])
        landing_case(client, report, game, base, 4.5, 2, False)
        client.call('origins/surfaces', {'flags': None})
        before, fixture = prepare(client)
        slick_base = floor['center'] if floor else base
        fixture.update(position=[slick_base[0], slick_base[1] + before['player']['height'] * .5 + .01, slick_base[2]], yaw=0,
                       open_connections=[door['connection'] for door in game['access']['doors']])
        client.call('origins/setup', fixture)
        before = client.until(lambda state: state['player']['grounded']
                              and not state['player']['mobility']['airborne'])
        client.call('origins/surfaces', {'flags': 2})
        marker = before['movement']['physics'][-1]['seconds']
        inputs = [edge(1, binding(before, 'forward'), True), edge(21, binding(before, 'forward'), False),
                  edge(45, binding(before, 'forward'), False)]
        after = schedule(client, inputs)
        rows = [row for row in after['movement']['physics'] if row['seconds'] > marker
                and row['grounded'] and row['surface_flags'] & 2 and row['ground_normal'][1] > .9999]
        errors = []
        moving = []
        braking = []
        for row in rows:
            normal = row['ground_normal']
            old = ground_velocity(row['velocity_before'], normal)
            speed = magnitude(old)
            retained = max(0, 1 - 1.5 * row['dt']) if speed >= .0254 else 0
            expected = [value * retained for value in old]
            wish = ground_velocity(row['wish'], normal)
            wishspeed = math.sqrt(sum(value * value for value in wish))
            if wishspeed:
                moving.append(row)
                direction = [value / wishspeed for value in wish]
                addition = min(max(0, wishspeed - sum(a * b for a, b in zip(expected, direction))),
                               2 * max(wishspeed, native['physics']['stop_speed']) * row['dt'])
                expected = [value + axis * addition for value, axis in zip(expected, direction)]
            elif speed > .1:
                braking.append(row)
            slope = math.hypot(normal[0], normal[2])
            direction = [normal[0] / slope, 0, normal[2] / slope] if slope else [0, 0, 0]
            wishspeed = 4000 * .0254
            addition = min(max(0, wishspeed - sum(a * b for a, b in zip(expected, direction))),
                           2 * wishspeed * row['dt'])
            expected = [value + axis * addition for value, axis in zip(expected, direction)]
            speed3d = math.sqrt(sum(value * value for value in expected))
            if speed3d > 500 * 500 * .0254:
                expected = [value / speed3d * 500 * .0254 for value in expected]
            expected = ground_velocity(expected, normal)
            if math.dist(expected[::2], row['move_velocity'][::2]) > .002:
                errors.append({'sample': row, 'expected': expected})
        check(report, 'slick_surface_uses_native_friction_and_acceleration',
              len(moving) >= 10 and len(braking) >= 5 and not errors,
              fixture=fixture, inputs=inputs, moving=len(moving), braking=len(braking), errors=errors)
        client.call('origins/surfaces', {'flags': None})
        before, fixture = prepare(client)
        fixture.update(position=[slick_base[0], slick_base[1] + before['player']['height'] * .5 + .01, slick_base[2]],
                       open_connections=[door['connection'] for door in game['access']['doors']])
        client.call('origins/setup', fixture)
        client.wait_ticks(4)
        before = client.until(lambda state: state['player']['grounded']
                              and not state['player']['mobility']['airborne'])
        client.call('origins/surfaces', {'flags': 2})
        inputs = [edge(1, binding(before, 'jump'), True), edge(2, binding(before, 'jump'), False),
                  edge(4, binding(before, 'jump'), False)]
        after = schedule(client, inputs)
        check(report, 'slick_surface_rejects_jump_input',
              not after['player']['mobility']['airborne']
              and after['player']['position'][1] - before['player']['position'][1] < .05,
              fixture=fixture, inputs=inputs, before=before['player'], player=after['player'])
    finally:
        client.release()
        client.call('origins/surfaces', {'flags': None})
