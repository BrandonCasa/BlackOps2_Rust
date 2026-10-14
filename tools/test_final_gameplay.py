# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///

from test_support import asset_bytes

import argparse
from collections import Counter
import csv
from datetime import datetime, timezone
import json
import math
from pathlib import Path
import subprocess
import threading
import time

from test_support import Client, initial_zones, write_report
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


INITIAL_SECTIONS = ('initial_start', 'initial_controls', 'initial_combat', 'initial_grenade_motion', 'initial_melee',
                    'initial_life', 'initial_revive', 'initial_barriers', 'initial_rounds')


def sample_gpu_memory(pid, trace, stop):
    def process_state():
        result = subprocess.run(['tasklist', '/FI', f'PID eq {pid}', '/FO', 'CSV', '/NH'],
                                capture_output=True, text=True, check=True, timeout=2,
                                creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        row = next((row for row in csv.reader(result.stdout.splitlines())
                    if len(row) > 1 and row[1] == str(pid)), None)
        return {'running': row is not None, 'image': row[0] if row else None}

    while not stop.is_set():
        sample = {'started_utc': datetime.now(timezone.utc).isoformat()}
        try:
            sample['owned_process_before'] = process_state()
            result = subprocess.run(['nvidia-smi', '--query-gpu=timestamp,index,name,memory.total,memory.used',
                                     '--format=csv,noheader,nounits'],
                                    capture_output=True, text=True, check=True, timeout=3,
                                    creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
            sample['gpus'] = [{'driver_timestamp_local': row[0], 'index': int(row[1]), 'name': row[2],
                               'total_mib': float(row[3]), 'used_mib': float(row[4])}
                              for row in csv.reader(result.stdout.splitlines(), skipinitialspace=True)
                              if len(row) == 5]
            if not sample['gpus']:
                raise RuntimeError('nvidia-smi returned no GPU memory samples')
            sample['owned_process_after'] = process_state()
        except Exception as error:
            sample['error'] = f'{type(error).__name__}: {error}'
        sample['finished_utc'] = datetime.now(timezone.utc).isoformat()
        trace['samples'].append(sample)
        if 'error' in sample:
            trace['error'] = sample['error']
            return
        stop.wait(trace['interval_seconds'])


def normal_action(client, action, ticks=2):
    key = binding(client.state(), action)
    return schedule(client, [edge(1, key, True), edge(ticks + 1, key, False)])


def normal_look(client, point):
    state = client.state()
    player = state['player']
    eye = player['position'].copy()
    eye[1] += player['eye_height'] - player['height'] / 2
    delta = [target - origin for origin, target in zip(eye, point)]
    yaw = math.atan2(-delta[0], -delta[2])
    pitch = max(-1.5, min(1.5, math.atan2(delta[1], math.hypot(delta[0], delta[2]))))
    for _ in range(3):
        player = state['player']
        horizontal = (yaw - player['yaw'] + math.pi) % math.tau - math.pi
        if abs(horizontal) < .02 and abs(pitch - player['pitch']) < .02:
            return state
        vertical = -1 if state['controls']['invert_mouse_y'] else 1
        probe_y = (1 if player['pitch'] >= 0 else -1) * vertical
        probe = schedule(client, [{'tick': 1, 'motion': [1, probe_y]}])
        yaw_step = (player['yaw'] - probe['player']['yaw'] + math.pi) % math.tau - math.pi
        pitch_step = (player['pitch'] - probe['player']['pitch']) / probe_y
        if not all(math.isfinite(step) and abs(step) > 1e-6 for step in (yaw_step, pitch_step)):
            raise RuntimeError('Normal mouse probe produced no measurable look response')
        horizontal = (yaw - probe['player']['yaw'] + math.pi) % math.tau - math.pi
        state = schedule(client, [{'tick': 1, 'motion': [-horizontal / yaw_step,
                             -(pitch - probe['player']['pitch']) / pitch_step]}])
    if (abs((state['player']['yaw'] - yaw + math.pi) % math.tau - math.pi) >= .02
            or abs(state['player']['pitch'] - pitch) >= .02):
        raise RuntimeError('Normal mouse look did not reach the requested direction')
    return state


def native_cook_off(client, grenade):
    before = client.until(lambda s: s['player']['lethal']['state'] == 'Ready')
    end = math.ceil((grenade['prime_seconds'] + grenade['fuse_seconds'] + 1)
                    / before['fixed_delta_seconds'])
    key = binding(before, 'lethal')
    client.call('origins/input', {'events': [edge(1, key, True), edge(end, key, False)]})
    try:
        return client.until(lambda s: s['equipment']['cooked_off'] > before['equipment']['cooked_off']
                            and (s['player']['health'] != before['player']['health']
                                 or s['player']['life'] != before['player']['life']), timeout=30)
    finally:
        client.release()


def camera_snapshot(client):
    state = client.state()
    registered = client.call('world.list_components')
    paths = {}
    for name in ('Camera3d', 'Transform', 'GlobalTransform', 'Projection'):
        matches = [path for path in registered if path.startswith('bevy_') and path.endswith('::' + name)]
        if len(matches) != 1:
            raise RuntimeError('Camera snapshot requires one registered ' + name + ': ' + str(matches))
        paths[name] = matches[0]
    rows = client.call('world.query', {
        'data': {'components': [paths[name] for name in ('Transform', 'GlobalTransform', 'Projection')]},
        'filter': {'with': [paths['Camera3d']]}, 'strict': True})
    cameras = [row for row in rows if any(path.endswith('::GameCamera') for path in
               client.call('world.list_components', {'entity': row['entity']}))]
    if len(cameras) != 1:
        raise RuntimeError('Camera snapshot requires one GameCamera: ' + str(cameras))
    return {'state_before_query': {'fixed_tick': state['fixed_tick'], 'player': state['player']},
            'cameras': cameras}


def initial_start(client, report, game, output):
    state = client.until(lambda s: s['state'] == 'Playing' and s.get('player')
                         and s['player']['grounded'] and s['player']['action'] == 'Ready', timeout=300)
    pistol = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.c96_zm')
    frag = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == 'weapon.frag_grenade_zm')
    owned = state['player']['weapons']
    awards = [event for event in state['equipment']['events'] if event.get('event') == 'round_award'
              and event.get('round') == 1 and event.get('weapon') == 'weapon.frag_grenade_zm']
    expected_frag = min(frag['initial'] + len(awards) * frag['round_award'], frag['capacity'])
    check(report, 'initial_pack_declares_scoped_content', game['delivery_scope'] == 'first_playable_solo'
          and game['render_meshes'] > 0 and game['collision_meshes'] > 0,
          scene=game['scene'], collision_scene=game['collision_scene'])
    check(report, 'initial_source_loadout_without_grants', len(owned) == 1
          and owned[0]['id'] == pistol['id'] and owned[0]['magazine'] == pistol['magazine']
          and owned[0]['reserve'] == pistol['mechanics']['starting_reserve']
          and len(awards) <= 1 and state['player']['lethal']['count'] == expected_frag
          and not state['player']['perks'] and state['player']['solo_charges'] == 0,
          player=state['player'], expected_lethal_count=expected_frag, natural_first_round_awards=awards)
    check(report, 'initial_grounded_in_authored_spawn_area', state['session']['round'] == 1
          and state['player']['health'] == game['player_health']['maximum']
          and min(math.dist(state['player']['position'], spawn['position'])
                  for spawn in game['initial_spawns']) < 2, state=state)
    materials = client.call('origins/materials')
    check(report, 'initial_render_pipelines_ready', materials['pipelines']['pending'] == 0
          and materials['pipelines']['error'] is None, materials=materials)
    check(report, 'initial_first_person_model_and_textures_loaded', materials['pending'] == 0
          and bool(materials['first_person'])
          and all(row['textures_ready'] for row in materials['first_person']), materials=materials)
    report['initial_scene'] = {'scene': game['scene'], 'collision_scene': game['collision_scene'],
                               'materials': materials, 'animations': client.call('origins/animations')}
    try:
        report['initial_scene']['camera_before_capture'] = camera_snapshot(client)
    except RuntimeError as error:
        report['initial_scene']['camera_before_capture_error'] = str(error)
    report['initial_scene']['capture'] = ui.capture(client, output, 'initial-start')
    try:
        report['initial_scene']['camera_after_capture'] = camera_snapshot(client)
    except RuntimeError as error:
        report['initial_scene']['camera_after_capture_error'] = str(error)
    paths = {asset['id']: asset['path'] for asset in game['content']['ui_assets']}
    ammo = owned[0]
    expected = Counter(paths['ui.hud_zm_num_tomb_' + ('slash' if char == '/' else char)]
                       for char in f"{ammo['magazine']}/{ammo['reserve']}")
    expected[paths['ui.chalkmarks_hellcatraz_' + str(state['session']['round'])]] += 1
    hud = ui.observe(client, lambda h: h['visible_images'] > 0
                     and h['loaded_images'] == h['visible_images'] and h['crosshair']
                     and ui.text(h, str(state['session']['points'])))
    check(report, 'initial_hud_images_loaded', ui.text(hud, str(state['session']['points']))
          and all(hud['images'].get(path, 0) >= count for path, count in expected.items()),
          expected_glyph_images=dict(expected), expected_points=state['session']['points'], ui=hud)
    session.first_round_grenades(client, report, game, 'initial_natural_first_round_grenade_award')


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
    shot_alias = game['audio']['bindings']['weapon.c96_zm.shot']
    audible = client.until(lambda s: s['audio']['sinks'].get(shot_alias, 0)
                           > before['audio']['sinks'].get(shot_alias, 0), timeout=10)
    check(report, 'initial_pistol_creates_native_audio_sink', True,
          alias=shot_alias, sinks=audible['audio']['sinks'])
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
    explode_alias = game['audio']['bindings']['weapon.frag_grenade_zm.explode']
    heard = client.until(lambda s: s['audio']['sinks'].get(explode_alias, 0)
                         > before['audio']['sinks'].get(explode_alias, 0), timeout=10)
    check(report, 'initial_frag_creates_native_explosion_audio_sink', True,
          alias=explode_alias, sinks=heard['audio']['sinks'])
    report['initial_combat_presentation'] = {'audio': blast['audio'],
                                            'animations': client.call('origins/animations'),
                                            'capture': ui.capture(client, output, 'initial-combat')}


def initial_melee(client, report, game):
    client.reset(difficulty='original')
    before = client.until(lambda s: s['player']['grounded'] and s['player']['action'] == 'Ready')
    player = before['player']
    position = [player['position'][0] - math.sin(player['yaw']) * 1.1,
                player['position'][1], player['position'][2] - math.cos(player['yaw']) * 1.1]
    fixture = {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300,
               'enemies': [{'position': position, 'health': game['rounds']['health_start']}]}
    admitted = client.call('origins/setup', fixture)['enemies'][0]
    ready = client.until(lambda s: any(e['entity'] == admitted and e['ready'] for e in s['enemies']))
    target = next(e for e in ready['enemies'] if e['entity'] == admitted)
    point = target['position'].copy()
    normal_look(client, point)
    normal_action(client, 'melee')
    after = client.until(lambda s: any(r['target'] == admitted and r['category'] == 'Melee'
                                      for r in s['damage']['recent']))
    hits = [r for r in after['damage']['recent'] if r['target'] == admitted and r['category'] == 'Melee']
    check(report, 'initial_normal_knife_damages_source_health_enemy',
          all(r['base_amount'] > 0 and r['points'] > 0 for r in hits)
          and after['session']['points'] > ready['session']['points'],
          fixture=fixture, hits=hits, before=ready['session'], after=after['session'])


def initial_grenade_motion(client, report, game, pack, research):
    candidates, _ = physics.surfaces(pack, game)
    site = physics.select_flat(client, report, game, candidates, native_tuning(research)[0])
    before, fixture = physics.prepare(client, game, site, yaw=0)
    check(report, 'initial_grenade_fixture_retains_starting_zone',
          before['zones']['current'] in initial_zones(game) and not fixture['open_connections'],
          fixture=fixture, zones=before['zones'])
    point = site['center'].copy()
    point[2] -= .2
    normal_look(client, point)
    before = client.state()
    normal_action(client, 'lethal')
    observed = []

    def detonated(state):
        if len(observed) < 512 and (not observed or observed[-1]['fixed_tick'] != state['fixed_tick']):
            observed.extend({'fixed_tick': state['fixed_tick'], **grenade}
                            for grenade in state['equipment']['active_grenades'])
        return state['equipment']['detonations'] > before['equipment']['detonations']

    after = client.until(detonated, timeout=30)
    events = after['equipment']['events'][len(before['equipment']['events']):]
    contacts = [event for event in events if event.get('event') == 'bounce' and event['normal'][1] > .999]
    check(report, 'initial_grenade_hits_authored_flat_floor', bool(contacts),
          fixture=fixture, contacts=contacts, events=events)
    first = next((event for event in contacts if not event['resting']), None)
    check(report, 'initial_grenade_retains_motion_at_first_floor_contact', first is not None,
          contacts=contacts)
    following = contacts[contacts.index(first) + 1:]
    travel = max((math.dist(first['position'][::2], row['position'][::2]) for row in following), default=0)
    check(report, 'initial_grenade_rolls_beyond_first_floor_contact', travel > .001
          and any(row['motion'] == 'rolling' for row in contacts),
          horizontal_travel=travel, first_contact=first, following_contacts=following)
    floor = [row for row in contacts if row.get('ground_point') is not None]
    check(report, 'initial_grenade_preserves_native_floor_clearance', bool(floor)
          and all(abs(row['position_after'][1] - row['ground_point'][1] - 1.5 * .0254) < .004 for row in floor),
          contacts=floor)
    rest = next((row for row in contacts if row['resting']), None)
    native = next(weapon['throwable']['native_launch'] for weapon in game['content']['weapons']
                  if weapon['id'] == 'weapon.frag_grenade_zm')
    report['initial_grenade_orientation'] = {'preserve_rotation': native['preserve_rotation'],
                                             'rotation_mode': native['rotation_mode'], 'rest': rest,
                                             'observed': observed}
    if rest is None:
        report.setdefault('limitations', []).append('The open-floor frag did not settle before its fuse; rest orientation remains unobserved in this case.')
        return
    if not native['preserve_rotation'] and native['rotation_mode'] in (1, 2):
        x, y, z, w = rest['rotation']
        axis = [2 * (x * z + w * y), 2 * (y * z - w * x), 1 - 2 * (x * x + y * y)]
        alignment = abs(sum(a * b for a, b in zip(axis, rest['normal'])))
        check(report, 'initial_grenade_source_enabled_floor_alignment', alignment > .999,
              normal_alignment=alignment, rest=rest)
    settled = [grenade for grenade in observed if grenade['entity'] == rest['entity'] and grenade['settled']]
    if len(settled) < 2:
        report.setdefault('limitations', []).append('Fewer than two fixed ticks of settled frag orientation were observed; frozen orientation remains unverified.')
        return
    reference = settled[0]['rotation']
    check(report, 'initial_grenade_settled_orientation_remains_frozen',
          all(abs(sum(a * b for a, b in zip(reference, grenade['rotation']))) > .99999
              and math.dist(settled[0]['position'], grenade['position']) < .0001 for grenade in settled[1:]),
          settled=settled)


def initial_rounds(client, report, game, output, timeout):
    client.reset(difficulty='original')
    state = client.until(lambda s: s['player']['grounded'] and s['player']['action'] == 'Ready')
    profile = game['rounds']
    enabled = initial_zones(game)
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
    emerged = set()
    round_log = []
    report['initial_rounds'] = round_log
    while time.monotonic() < deadline:
        state = client.state()
        display_round = state['session']['round']
        finished_five = display_round == 5 and state['session']['pending'] == 0 and not any(
            enemy.get('health') for enemy in state['enemies'])
        if (state['state'] != 'Playing' and not finished_five) or state['player']['life'] != 'Standing':
            raise RuntimeError('Normal-input survival ended: ' + json.dumps(state['session']))
        number = display_round - 1 if state['session']['intermission'] else display_round
        if (state['session']['intermission'] or finished_five) and number not in completed:
            admissions = [row for row in state['round_evidence']['admitted'] if row['round'] == number]
            check(report, 'initial_round_' + str(number) + '_population_health_and_kills',
                  len(admissions) == expected[number]['population']
                  and all(row['maximum_health'] == expected[number]['health']
                          and row['spawn_zone'] in enabled for row in admissions)
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
        if target['entity'] not in emerged:
            animation_state = client.call('origins/animations')
            observed = next((enemy for enemy in animation_state['enemies']
                             if enemy['entity'] == target['entity']), None)
            if observed is None or not observed['completed_emerging']:
                client.wait_ticks(12)
                continue
            emerged.add(target['entity'])
        point = target['position'].copy()
        heads = [region for region in target['hit_regions'] if region['region'] == 'Head']
        if heads:
            point = [(a + b) / 2 for a, b in zip(heads[0]['start'], heads[0]['end'])]
        else:
            point[1] += 1.2
        normal_look(client, point)
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
    injured = native_cook_off(client, frag)
    ended = client.until(lambda s: s['state'] == 'GameOver', timeout=10)
    check(report, 'initial_unpurchased_revive_does_not_admit_downed',
          ended['player']['life'] == 'HumanTerminal' and ended['session']['downs'] == 1,
          injury=injured, after=ended)
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
    report['initial_life_limitations'] = ['Ordinary solo automatic revival precedes source bleedout; no unreachable bleedout claim is made.']


def initial_revive(client, report, game, output):
    client.reset(difficulty='original')
    perk = next(p for p in game['content']['perks'] if p['id'] == game['solo_revive']['perk'])
    interaction = next(i for i in game['content']['interactions']
                       if i['action'].get('perk') == perk['id'])
    placement = next(p for p in game['content']['placements'] if p['id'] == interaction['placement'])
    point = placement['position'].copy()
    point[1] += 1
    generator = next(g for g in game['content']['generators'] if g['id'] == perk['generator'])
    fixture = {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300,
               'points': perk['price'] + generator['price']}
    client.call('origins/setup', fixture)
    from test_generator_population import focus_generator
    site = next(site for site in client.state()['generators']['sites'] if site['id'] == generator['id'])
    generator_focus = focus_generator(client, site)
    normal_action(client, 'interact')
    capturing = client.until(lambda s: next(g for g in s['generators']['sites']
                                            if g['id'] == site['id'])['state'] == 'Capturing')
    check(report, 'initial_generator_activation_charges_source_cost',
          capturing['session']['points'] == generator_focus['session']['points'] - generator['price'],
          fixture=fixture, before=generator_focus['session'], after=capturing['session'])
    deadline = time.monotonic() + 60
    captured = None
    while time.monotonic() < deadline:
        state = client.state()
        if state['state'] != 'Playing' or state['player']['life'] != 'Standing':
            raise RuntimeError('Generator prerequisite capture ended: ' + json.dumps(state['session']))
        current = next(g for g in state['generators']['sites'] if g['id'] == site['id'])
        if current['state'] == 'Active':
            captured = state
            break
        enemies = [enemy for enemy in state['enemies'] if enemy.get('health') and enemy['ready']]
        if enemies and state['player']['action'] == 'Ready':
            target = min(enemies, key=lambda e: math.dist(e['position'], state['player']['position']))
            target_point = target['position'].copy()
            target_point[1] += 1.2
            normal_look(client, target_point)
            weapon = client.state()['player']['weapons'][0]
            normal_action(client, 'fire' if weapon['magazine'] else 'reload')
        client.wait_ticks(6)
    if captured is None:
        raise TimeoutError('Normal-input starting generator capture did not complete')
    check(report, 'initial_generator_capture_enables_quick_revive_prerequisite',
          captured['generators']['completions'] > generator_focus['generators']['completions'],
          after=captured)
    focused = None
    approaches = []
    for radius in (1.2, .8, 1.6):
        for angle in range(8):
            position = [point[0] + radius * math.sin(angle * math.tau / 8),
                        placement['position'][1] + game['tuning']['standing_height'] / 2 + .02,
                        point[2] + radius * math.cos(angle * math.tau / 8)]
            client.call('origins/setup', {'position': position})
            client.wait_ticks(3)
            normal_look(client, point)
            state = client.wait_ticks(3)
            approaches.append({'position': position, 'focus': state['focus']})
            if perk['name'].lower() in (state['focus']['prompt'] or '').lower():
                focused = state
                break
        if focused:
            break
    if focused is None:
        raise RuntimeError('Authored Quick Revive prompt unavailable: ' + json.dumps(approaches))
    check(report, 'initial_quick_revive_focus_after_generator_capture',
          focused['focus']['rejection'] is None and focused['player']['solo_charges'] == 0
          and focused['session']['points'] >= perk['price'], fixture=fixture, approaches=approaches,
          focus=focused['focus'], capture=ui.capture(client, output, 'initial-quick-revive-prompt'))
    ticks = math.ceil((interaction['hold_seconds'] + .5) / focused['fixed_delta_seconds'])
    normal_action(client, 'interact', max(2, ticks))
    bought = client.until(lambda s: perk['id'] in s['player']['perks']
                          and s['player']['solo_charges'] == 1 and not s['player']['drinking'])
    check(report, 'initial_normal_quick_revive_purchase',
          bought['session']['points'] == focused['session']['points'] - perk['price'],
          before=focused['player'], after=bought['player'], session=bought['session'])
    grenade = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == 'weapon.frag_grenade_zm')
    native_cook_off(client, grenade)
    downed = client.until(lambda s: s['player']['life'] == 'Downed'
                          and s['player']['trigger_client_mode'] == 7
                          and math.isclose(s['player']['height'], game['solo_revive']['downed_height'], abs_tol=1e-4)
                          and math.isclose(s['player']['eye_height'], game['solo_revive']['downed_eye'], abs_tol=1e-4))
    check(report, 'initial_purchased_revive_admits_native_downed_state',
          downed['player']['trigger_client_mode'] == 7
          and not downed['player']['perks'] and downed['session']['downs'] == 1
          and math.isclose(downed['player']['height'], game['solo_revive']['downed_height'], abs_tol=1e-4)
          and math.isclose(downed['player']['eye_height'], game['solo_revive']['downed_eye'], abs_tol=1e-4)
          and any(w['id'] == game['solo_revive']['downed_weapon'] for w in downed['player']['weapons']),
          downed=downed['player'])
    client.until(lambda s: s['player']['life'] == 'Downed' and s['player']['action'] == 'Ready')
    normal_action(client, 'fire')
    fired = client.until(lambda s: s['session']['shots'] > downed['session']['shots'])
    check(report, 'initial_downed_pistol_fires_from_input', fired['session']['shots'] > downed['session']['shots'],
          before=downed['player'], after=fired['player'])
    revived = client.until(lambda s: s['player']['life'] == 'Standing' and s['player']['trigger_client_mode'] == 0
                           and s['player']['solo_charges'] == 0, timeout=30)
    event = next(event for event in reversed(revived['solo_revive_evidence']['events'])
                 if event.get('event') == 'revived' and event['target'] == downed['player']['entity'])
    elapsed = event['seconds']
    check(report, 'initial_purchased_quick_revive_recovers_on_source_schedule',
          abs(elapsed - game['solo_revive']['seconds']) <= 2 * revived['fixed_delta_seconds']
          and revived['player']['trigger_client_mode'] == 0
          and revived['player']['health'] == game['player_health']['maximum']
          and not revived['player']['perks'], elapsed=elapsed, player=revived['player'])


def initial_barriers(client, report, game):
    client.reset(difficulty='original')
    state = client.until(lambda s: s.get('barriers') and s['player']['grounded'])
    baseline = state['barriers']
    damaged = client.until(lambda s: s['barriers']['destroyed'] > baseline['destroyed']
                           and any(window['repairable'] for window in s['barriers']['windows']), timeout=180)
    entered = client.until(lambda s: s['barriers']['entries'] > baseline['entries'], timeout=120)
    check(report, 'initial_natural_zombie_barrier_destruction_and_entry',
          entered['barriers']['destroyed'] > baseline['destroyed'],
          before=baseline, damaged=damaged['barriers'], entered=entered['barriers'])
    attacked = client.until(lambda s: s['enemy_evidence']['attack_hits'] > state['enemy_evidence']['attack_hits']
                            and s['player']['health'] < state['player']['health'], timeout=90)
    check(report, 'initial_natural_zombie_pursuit_and_attack_damage',
          attacked['enemy_evidence']['attacks_started'] > state['enemy_evidence']['attacks_started']
          and attacked['health_evidence']['nonterminal_hits'] > state['health_evidence']['nonterminal_hits'],
          before=state['player'], after=attacked['player'], enemy_evidence=attacked['enemy_evidence'])
    window = next(window for window in entered['barriers']['windows'] if window['repairable'])
    definition = next(b for b in game['content']['barriers'] if b['id'] == window['id'])
    position = window['repair_origin'].copy()
    position[1] += game['tuning']['standing_height'] / 2 + .02
    fixture = {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300, 'position': position}
    client.call('origins/setup', fixture)
    before = client.wait_ticks(3)
    normal_action(client, 'interact', math.ceil((definition['repair_interval'] + 2)
                                              / before['fixed_delta_seconds']))
    repaired = client.until(lambda s: s['barriers']['repaired'] > before['barriers']['repaired'])
    awarded = repaired['barriers']['awarded'] - before['barriers']['awarded']
    check(report, 'initial_normal_barrier_repair_awards_source_points',
          repaired['session']['points'] - before['session']['points'] == awarded
          and awarded > 0 and awarded % definition['repair_points'] == 0,
          fixture=fixture, before=before['barriers'], after=repaired['barriers'], session=repaired['session'])


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
    parser.add_argument('--gpu-memory-pid', type=int)
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
    memory_stop = threading.Event()
    memory_thread = None
    try:
        if args.gpu_memory_pid is not None:
            if args.gpu_memory_pid <= 0:
                raise ValueError('GPU memory trace requires the positive owned game PID')
            trace = {'owned_pid': args.gpu_memory_pid, 'interval_seconds': 1,
                     'scope': 'Whole-device memory including other processes; sampled maxima are not exact peaks or performance results.',
                     'samples': []}
            report['gpu_memory'] = trace
            memory_thread = threading.Thread(target=sample_gpu_memory,
                                             args=(args.gpu_memory_pid, trace, memory_stop), daemon=True)
            memory_thread.start()
        if args.milestone == 'initial' and any(name not in (*INITIAL_SECTIONS, 'session') for name in selected):
            raise ValueError('Initial milestone sections must stay within initial content')
        if not math.isfinite(args.round_timeout) or args.round_timeout <= 0:
            raise ValueError('Round timeout must be positive and finite')
        game = json.loads(asset_bytes(Path(args.assets.resolve()) / 'game.json'))
        ready = client.until(lambda s: s['state'] == 'LoadError'
                             or s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), 300)
        report['initial_state'] = ready
        if ready['state'] == 'LoadError':
            raise RuntimeError('Game asset loading failed: ' + json.dumps(ready))
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
                elif name == 'initial_revive':
                    initial_revive(client, report, game, capture_output)
                elif name == 'initial_barriers':
                    initial_barriers(client, report, game)
                elif name == 'initial_melee':
                    initial_melee(client, report, game)
                elif name == 'initial_grenade_motion':
                    initial_grenade_motion(client, report, game, args.assets.resolve(), args.research)
                elif name == 'ui':
                    ui.run(client, report, None if args.no_captures else output, game)
                elif name == 'session':
                    if args.milestone == 'initial':
                        session.run_initial(client, report, game)
                    else:
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
        if memory_thread is not None:
            memory_stop.set()
            memory_thread.join(timeout=8)
            trace['sampler_stopped'] = not memory_thread.is_alive()
            maxima = {}
            for sample in trace['samples']:
                if (sample.get('owned_process_before', {}).get('running')
                        and sample.get('owned_process_after', {}).get('running')):
                    for gpu in sample.get('gpus', []):
                        index = str(gpu['index'])
                        maxima[index] = max(maxima.get(index, 0), gpu['used_mib'])
            trace['sampled_running_max_mib_by_gpu'] = maxima
        target = write_report(output, 'final-gameplay.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'report': str(target)}))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
