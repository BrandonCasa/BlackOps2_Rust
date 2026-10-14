# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///

from test_support import asset_bytes

import argparse
import json
import math
from pathlib import Path

import numpy as np

from test_support import gltf_accessor as accessor
from test_support import Client, initial_zones, write_report
from test_support import source_physics as native_tuning
from test_throwable_down import binding, check as record_check, edge, schedule


def check(report, name, passed, **evidence):
    print(json.dumps({'case': name, 'passed': bool(passed)}), flush=True)
    record_check(report, name, passed, **evidence)


def surfaces(pack, game):
    path, scene = game['collision_scene'].split('#Scene')
    document = json.loads(asset_bytes(pack / path))
    binary = asset_bytes((pack / path).parent / document['buffers'][0]['uri'])
    candidates = []
    stair_candidates = []
    for index in document['scenes'][int(scene)]['nodes']:
        node = document['nodes'][index]
        if 'mesh' not in node or any(key in node for key in ('matrix', 'translation', 'rotation', 'scale')):
            continue
        for primitive in document['meshes'][node['mesh']]['primitives']:
            material = document.get('materials', [])[primitive['material']].get('name', '')
            if any(name in material.lower() for name in ('mud', 'water', 'clip', 'caulk')):
                continue
            points = accessor(document, binary, primitive['attributes']['POSITION'])
            if primitive.get('mode', 4) != 4:
                raise ValueError('Physics scenarios require source triangle-list collision meshes')
            if 'indices' in primitive:
                indices = accessor(document, binary, primitive['indices']).reshape(-1, 3)
                triangles = points[indices]
            else:
                if len(points) % 3:
                    raise ValueError('Non-indexed collision triangle list has incomplete vertices')
                triangles = points.reshape(-1, 3, 3)
            cross = np.cross(triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 0])
            length = np.linalg.norm(cross, axis=1)
            normals = cross / np.maximum(length[:, None], 1e-12)
            edges = np.stack([triangles[:, 1] - triangles[:, 0], triangles[:, 2] - triangles[:, 1],
                              triangles[:, 0] - triangles[:, 2]], axis=1)
            radius = length / np.maximum(np.linalg.norm(edges, axis=2).sum(axis=1), 1e-12)
            flat = triangles[(normals[:, 1] > .9999) & (length > .2)].mean(axis=1)
            if len(flat) >= 8:
                heights = np.unique(np.round(flat[:, 1], 3))
                for start in range(max(0, len(heights) - 3)):
                    flight = heights[start:start + 4]
                    if np.all((np.diff(flight) > .06) & (np.diff(flight) <= .458)):
                        low = flat[np.abs(flat[:, 1] - flight[0]) < .002].mean(axis=0)
                        high = flat[np.abs(flat[:, 1] - flight[-1]) < .002].mean(axis=0)
                        if .8 < np.linalg.norm((high - low)[[0, 2]]) < 6:
                            stair_candidates.append({'node': int(index), 'center': low.tolist(),
                                                     'high': high.tolist(), 'material': material,
                                                     'distance': float(np.linalg.norm(low - game['spawn']))})
            for row in np.flatnonzero((normals[:, 1] > .71) & (radius > 1.1)):
                center = triangles[row].mean(axis=0)
                candidates.append({'node': int(index), 'triangle': int(row), 'center': center.tolist(),
                                   'normal': normals[row].tolist(), 'radius': float(radius[row]),
                                   'material': material,
                                   'distance': float(np.linalg.norm(center - game['spawn']))})
    return (sorted(candidates, key=lambda item: item['distance']),
            sorted(stair_candidates, key=lambda item: item['distance']))


def prepare(client, game, site=None, yaw=None, weapon=None, elevated=False):
    client.reset()
    initial = game.get('delivery_scope') == 'first_playable_solo'
    if weapon is None:
        weapon = 'weapon.c96_zm' if initial else 'weapon.mp40_zm'
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': [weapon], 'owned_perks': [], 'pitch': 0,
               'open_connections': [] if initial else [door['connection'] for door in game['access']['doors']]}
    if site:
        fixture['position'] = [site['center'][0], site['center'][1] + game['tuning']['standing_height'] / 2 + .02,
                               site['center'][2]]
        if elevated:
            fixture['position'][1] += 20
    if yaw is not None:
        fixture['yaw'] = yaw
    client.call('origins/setup', fixture)
    state = client.until(lambda s: s['player']['action'] == 'Ready')
    if not elevated:
        state = client.until(lambda s: s['player']['grounded'], timeout=5)
        if 'jump_recovery' in state['player']['mobility']:
            state = client.until(lambda s: s['player']['grounded']
                                 and not s['player']['mobility']['airborne']
                                 and s['player']['mobility']['jump_recovery'] == 0
                                 and s['player']['action'] == 'Ready', timeout=5)
        else:
            recovery = game['tuning']['mobility'].get('jump_recovery', .25)
            state = client.wait_ticks(math.ceil(recovery / state['fixed_delta_seconds']) + 2)
    return state, fixture


def samples(state, before):
    return [row for row in state['movement']['physics'] if row['seconds'] > before]


def speed(row, key='velocity_after'):
    return math.hypot(row[key][0], row[key][2])


def select_flat(client, report, game, candidates, native):
    attempts = []
    bounds = []
    initial = game.get('delivery_scope') == 'first_playable_solo'
    enabled = initial_zones(game) if initial else set()
    if initial:
        for row in game['access']['volumes']:
            if row['zone'] not in enabled:
                continue
            volume = row['volume']
            points = np.array(volume['vertices']) + np.array(volume['position'])
            bounds.append((points.min(axis=0), points.max(axis=0)))
    else:
        for trigger in game['environment']['triggers']:
            if trigger.get('source', {}).get('script_noteworthy') not in ('player_volume', 'life_brush'):
                continue
            for volume in trigger.get('shape', {}).get('volumes', []):
                points = np.array(volume['vertices']) + np.array(volume['position']) + np.array(trigger['position'])
                bounds.append((points.min(axis=0), points.max(axis=0)))
    offsets = ([-1, game['tuning']['standing_height'] / 2, 1],
               [1, game['tuning']['standing_height'] / 2, -1]) if initial else ([-2, .9, 2], [2, .9, 2], [0, .9, -8])
    eligible = [c for c in candidates if c['normal'][1] > .9999 and c['radius'] > 1.1
                and any(all(np.all(np.array(c['center']) + offset >= low)
                            and np.all(np.array(c['center']) + offset <= high)
                            for offset in offsets)
                        for low, high in bounds)]
    for site in eligible[:24]:
        try:
            before, fixture = prepare(client, game, site)
            if initial and before['zones']['current'] not in enabled:
                attempts.append({'site': site, 'rejected_zone': before['zones']['current']})
                continue
            if abs(before['player']['position'][1] - before['player']['height'] / 2 - site['center'][1]) > .08:
                continue
            jump = binding(before, 'jump')
            marker = before['movement']['physics'][-1]['seconds']
            after = schedule(client, [edge(1, jump, True), edge(2, jump, False), edge(50, jump, False)])
            rows = samples(after, marker)
            height = max(row['position_after'][1] for row in rows) - rows[0]['position_before'][1]
            attempts.append({'site': site, 'jump_height': height})
            settled = client.wait_ticks(math.ceil(4 / after['fixed_delta_seconds']))
            attempts[-1]['boundary_life'] = settled['player']['life']
            if abs(height - .0254 * 39) < .025 and settled['player']['life'] == 'Standing':
                check(report, 'authored_open_floor_supports_native_jump', True, fixture=fixture,
                      site=site, height=height, attempts=attempts)
                return site
        except (RuntimeError, TimeoutError) as error:
            attempts.append({'site': site, 'error': str(error)[:200]})
            client.release()
    check(report, 'authored_open_floor_supports_native_jump', False, attempts=attempts)


def directions(client, report, game, site, native, mobility):
    cases = [('forward', ['forward'], native['walk_speed']),
             ('strafe', ['right'], native['walk_speed'] * native['physics']['strafe_scale']),
             ('backward', ['backward'], native['walk_speed'] * native['physics']['backward_scale']),
             ('diagonal', ['forward', 'right'], native['walk_speed']),
             ('sprint', ['forward', 'sprint'], native['sprint_speed']),
             ('crouch', ['forward', 'crouch'], native['crouch_speed']),
             ('prone', ['forward', 'prone'], mobility['prone_speed'])]
    for name, actions, expected in cases:
        before, fixture = prepare(client, game, site, yaw=0)
        marker = before['movement']['physics'][-1]['seconds']
        inputs = [edge(1, binding(before, action), True) for action in actions]
        inputs += [edge(21, binding(before, action), False) for action in actions]
        inputs += [edge(55, binding(before, 'forward'), False)]
        after = schedule(client, inputs)
        rows = samples(after, marker)
        moving = [row for row in rows if any(row['input']) and row['grounded']]
        peak = max((speed(row) for row in moving), default=0)
        stopped = speed(rows[-1])
        check(report, 'native_' + name + '_speed_and_stop', len(moving) >= 16
              and abs(peak - expected) < .01 and stopped < .002,
              fixture=fixture, inputs=inputs, expected=expected, peak=peak, stopped=stopped,
              samples=rows)
        if name == 'forward':
            braking = [row for row in rows if not any(row['input']) and speed(row, 'velocity_before') > .03]
            residual = []
            for row in braking:
                old = speed(row, 'velocity_before')
                predicted = max(0, old - max(old, native['physics']['stop_speed'])
                                * native['physics']['friction'] * row['dt'])
                residual.append(abs(predicted - speed(row)))
            distance = sum(math.dist(row['position_before'][::2], row['position_after'][::2]) for row in braking)
            check(report, 'native_speed_dependent_ground_braking', len(braking) >= 10
                  and max(residual) < .002 and .4 < distance < 1.2,
                  samples=braking, maximum_error=max(residual), stopping_distance=distance)


def air(client, report, game, site, native):
    before, fixture = prepare(client, game, site, yaw=0)
    forward, sprint, jump = [binding(before, action) for action in ('forward', 'sprint', 'jump')]
    marker = before['movement']['physics'][-1]['seconds']
    inputs = [edge(1, forward, True), edge(1, sprint, True), edge(19, jump, True),
              edge(20, jump, False), edge(20, forward, False), edge(20, sprint, False),
              edge(50, forward, False)]
    after = schedule(client, inputs)
    rows = [row for row in samples(after, marker) if not row['grounded'] and not any(row['input'])]
    residual = [abs(speed(row) - speed(row, 'velocity_before')) for row in rows]
    check(report, 'released_air_input_preserves_sprint_momentum', len(rows) >= 20
          and max(residual) < .002 and min(speed(row) for row in rows) > native['sprint_speed'] * .98,
          fixture=fixture, inputs=inputs, samples=rows, maximum_error=max(residual, default=None))
    for action, axis, weapon in (('right', 0, 'weapon.mp40_zm'),
                                 ('backward', 2, 'weapon.mp40_zm'),
                                 ('right', 0, 'weapon.m32_zm')):
        before, fixture = prepare(client, game, site, yaw=0, weapon=weapon)
        jump, steer = binding(before, 'jump'), binding(before, action)
        marker = before['movement']['physics'][-1]['seconds']
        inputs = [edge(1, jump, True), edge(2, jump, False), edge(2, steer, True),
                  edge(18, steer, False), edge(30, steer, False)]
        after = schedule(client, inputs)
        all_rows = samples(after, marker)
        rows = [r for r in all_rows if not r['grounded'] and any(r['input'])]
        errors = [abs(r['velocity_after'][axis] - r['velocity_before'][axis]
                      - native['physics']['air_acceleration'] * native['walk_speed'] * r['dt']) for r in rows]
        check(report, 'native_air_' + action + '_' + weapon + '_acceleration', len(rows) >= 14
              and max(errors) < .002, fixture=fixture, inputs=inputs, samples=rows,
              maximum_error=max(errors, default=None), unfiltered=all_rows if not rows else None)


def slopes(client, report, game, candidates, native):
    attempts = []
    for site in [c for c in candidates if .74 < c['normal'][1] < .97][:32]:
        normal = site['normal']
        yaw = math.atan2(normal[0], normal[2])
        try:
            before, fixture = prepare(client, game, site, yaw=yaw)
            marker = before['movement']['physics'][-1]['seconds']
            forward = binding(before, 'forward')
            after = schedule(client, [edge(1, forward, True), edge(25, forward, False), edge(40, forward, False)])
            rows = [r for r in samples(after, marker) if any(r['input'])]
            rise = after['player']['position'][1] - before['player']['position'][1]
            grounded = sum(r['grounded'] for r in rows)
            errors = []
            slowed = 0
            for row in rows:
                height = abs(row['position_after'][1] - row['position_before'][1])
                scale = (max(0, 1 - height / native['step_height'])
                         * (1 - native['physics']['step_speed_min']) + native['physics']['step_speed_min']
                         if abs(row['step_adjustment']) > native['physics']['step_slowdown_height'] else 1)
                slowed += scale < 1
                expected = math.sqrt(sum(v * v for v in row['collision_velocity'])) * scale
                errors.append(abs(math.sqrt(sum(v * v for v in row['velocity_after'])) - expected))
            attempts.append({'site': site, 'rise': rise, 'grounded_samples': grounded,
                             'slowed_samples': slowed, 'maximum_step_speed_error': max(errors, default=None),
                             'samples': rows})
            if rise > .15 and grounded >= len(rows) - 1 and max(errors) < .002:
                check(report, 'authored_slope_contact_and_native_height_slowdown', True, fixture=fixture,
                      samples=rows, attempts=attempts)
                return
        except (RuntimeError, TimeoutError) as error:
            attempts.append({'site': site, 'error': str(error)[:200]})
            client.release()
    check(report, 'authored_walkable_slope_keeps_ground_contact', False, attempts=attempts)


def stairs(client, report, game, candidates, native):
    attempts = []
    for site in candidates[:24]:
        delta = [b - a for a, b in zip(site['center'], site['high'])]
        yaw = math.atan2(-delta[0], -delta[2])
        try:
            before, fixture = prepare(client, game, site, yaw=yaw)
            marker = before['movement']['physics'][-1]['seconds']
            forward = binding(before, 'forward')
            after = schedule(client, [edge(1, forward, True), edge(45, forward, False), edge(60, forward, False)])
            rows = [r for r in samples(after, marker) if any(r['input'])]
            steps = [r['position_after'][1] - r['position_before'][1] for r in rows]
            rise = sum(max(0, value) for value in steps)
            net_rise = rows[-1]['position_after'][1] - rows[0]['position_before'][1]
            progress = math.dist(rows[0]['position_before'][::2], rows[-1]['position_after'][::2])
            grounded = sum(r['grounded'] for r in rows)
            overshoot = max(math.dist(r['position_before'][::2], r['position_after'][::2])
                            - speed(r, 'move_velocity') * r['dt'] for r in rows)
            errors = []
            slowed = 0
            for row in rows:
                height = abs(row['position_after'][1] - row['position_before'][1])
                scale = (max(0, 1 - height / native['step_height'])
                         * (1 - native['physics']['step_speed_min']) + native['physics']['step_speed_min']
                         if abs(row['step_adjustment']) > native['physics']['step_slowdown_height'] else 1)
                slowed += scale < 1
                expected = math.sqrt(sum(v * v for v in row['collision_velocity'])) * scale
                errors.append(abs(math.sqrt(sum(v * v for v in row['velocity_after'])) - expected))
            attempts.append({'site': site, 'rise': rise, 'grounded_samples': grounded,
                             'net_rise': net_rise, 'horizontal_progress': progress,
                             'maximum_step': max(steps), 'overshoot': overshoot,
                             'slowed_samples': slowed, 'maximum_step_speed_error': max(errors),
                             'samples': rows})
            if (net_rise > .3 and progress > .4 and grounded >= len(rows) - 2 and max(steps) <= .48
                    and overshoot < .02 and slowed > 0 and max(errors) < .002):
                check(report, 'authored_stairs_preserve_fixed_tick_travel', True, fixture=fixture,
                      samples=rows, attempts=attempts)
                return
        except (RuntimeError, TimeoutError) as error:
            attempts.append({'site': site, 'error': str(error)[:200]})
            client.release()
    check(report, 'authored_stairs_preserve_fixed_tick_travel', False, attempts=attempts)


def projectile_case(client, report, game, site, weapon, action, expected_gravity):
    before, fixture = prepare(client, game, site, yaw=0, weapon=weapon, elevated=True)
    key = binding(before, action)
    after = schedule(client, [edge(1, key, True), edge(2, key, False), edge(65, key, False)])
    rows = [r for r in after['movement']['projectiles'] if r['weapon'] == weapon or action == 'lethal']
    error = []
    for row in rows:
        dt = row['dt']
        predicted = row['velocity_after'][1] * dt + expected_gravity * dt * dt / 2
        error.append(abs(row['position_after'][1] - row['position_before'][1] - predicted))
    check(report, weapon + '_native_free_flight', len(rows) >= 3
          and all(abs(row['gravity'] - expected_gravity) < .00001 for row in rows)
          and max(error) < .0001, fixture=fixture, samples=rows, maximum_error=max(error, default=None))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--assets', type=Path, required=True)
    parser.add_argument('--research', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--focus', choices=('all', 'stairs_projectiles', 'projectiles'), default='all')
    args = parser.parse_args()
    client = Client(args.url, shutdown_timeout=45)
    report = {'passed': False, 'scenario': 'native_movement_physics', 'cases': [],
              'focus': args.focus,
              'limitations': ['Direct position, enemy, weapon and door changes are scenario setup.',
                              'Movement and projectile outcomes use scheduled native input on imported map geometry.',
                              'Imported slick flags and landing impact recovery are covered by the separate final landing scenario.',
                              'This measures native numeric contracts; subjective controller feel still needs player review.']}
    try:
        pack = Path(args.assets.resolve())
        game = json.loads(asset_bytes(pack / 'game.json'))
        native, mobility = native_tuning(args.research)
        check(report, 'pack_uses_native_motor_values', all(math.isclose(game['tuning'][key], value, abs_tol=1e-7)
              for key, value in native.items() if key != 'physics')
              and all(math.isclose(game['tuning']['physics'][key], value, abs_tol=1e-7)
                      for key, value in native['physics'].items()), native=native)
        candidates, stair_candidates = surfaces(pack, game)
        report['surface_candidates'] = len(candidates)
        report['stair_candidates'] = len(stair_candidates)
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=240)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        site = select_flat(client, report, game, candidates, native)
        if args.focus == 'all':
            directions(client, report, game, site, native, mobility)
            air(client, report, game, site, native)
            slopes(client, report, game, candidates, native)
        if args.focus != 'projectiles':
            stairs(client, report, game, stair_candidates, native)
        for weapon, action, gravity in [('weapon.m32_zm', 'fire', game['tuning']['gravity']),
                                         ('weapon.ray_gun_zm', 'fire', 0),
                                         ('weapon.mp40_zm', 'lethal', game['tuning']['gravity'])]:
            projectile_case(client, report, game, site, weapon, action, gravity)
        report['passed'] = True
    except Exception as error:
        report['error'] = f'{type(error).__name__}: {str(error)[:600]}'
    finally:
        try:
            client.release()
        except Exception as error:
            report['release_error'] = f'{type(error).__name__}: {error}'
        finally:
            client.shutdown(report)
        path = write_report(args.output, 'physics.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
