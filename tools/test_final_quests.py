import hashlib
import json
import math
from pathlib import Path
import re

from test_environment import aim as aim_point, tap
from test_nuke_rewards import actor
from test_throwable_down import binding, check, edge, schedule


def source_cycles(research):
    folder = Path(research) / 'source' / 'server-tomb' / 'maps' / 'mp'
    path = folder / 'zm_tomb_ee_lights.gsc'
    text = path.read_text(encoding='utf-8')
    morse = dict(re.findall(r'level\.morse_letters\["([A-Z])"\]\s*=\s*"([.\-]+)"', text))
    messages = re.findall(r'level\.morse_messages\[\d+\]\s*=\s*"([A-Z ]+)"', text)
    key = re.search(r'level\.cipher_key\s*=\s*"([A-Z]+)"', text).group(1)
    bellaso = re.search(r'light_show_morse\( lit_discs, "([A-Z ]+)" \)', text).group(1)
    if len(morse) != 26 or len(messages) != 8 or 'randomint( 100 ) < 10' not in text:
        raise RuntimeError('Recovered light source shape changed')

    def encrypt(phrase):
        answer = ''
        index = 0
        for letter in phrase:
            if letter in morse:
                answer += chr(65 + (ord(letter) + ord(key[index % len(key)]) - 130) % 26)
                index += 1
            else:
                answer += letter
        return answer

    def steps(rare):
        result = []

        def append(mode, on, seconds, message):
            result.append({'mode': mode, 'on': on, 'seconds': seconds, 'message': message})

        def phrase(message, mode):
            for letter in message:
                if letter in morse:
                    for symbol in morse[letter]:
                        append(mode, True, 0.2 if symbol == '.' else 1.0, message)
                        append(mode, False, 0.5, message)
                else:
                    append(mode, False, 2.0, message)
                append(mode, False, 1.5, message)

        if rare:
            append(1, False, 10.0, '')
            phrase(bellaso, 3)
        append(1, False, 10.0, '')
        phrase(key, 2)
        for message in messages:
            append(1, False, 10.0, '')
            phrase(encrypt(message), 1)
        return result

    return {'normal': steps(False), 'bellaso': steps(True)}, {
        'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
        'key': key, 'messages': messages, 'ciphertexts': [encrypt(message) for message in messages],
        'rare_probability': '10 of 100',
    }


def expected_color(lights):
    if lights['mode'] == 2:
        return [2.0 if lights['on'] else 0.5, 0.1, 0.1]
    if lights['mode'] == 3:
        return [0.1, 2.0 if lights['on'] else 0.5, 0.1]
    return [2.0 if lights['on'] else 0.25] * 3


def settled_materials(state):
    lights = state['staffs']['lights']
    materials = state['staffs']['light_materials']
    expected = expected_color(lights)
    return len(materials) == 4 and all(
        all(abs(actual - wanted) < 0.0001 for actual, wanted in zip(row['color'], expected))
        for row in materials)


def focus_access_target(client, point, predicate, offset):
    attempts = []
    for height in (.3, .91):
        for radius in (.8, 1.2, 1.7):
            for index in range(8):
                angle = index * math.tau / 8
                position = [point[0] + radius * math.sin(angle), point[1] + height,
                            point[2] + radius * math.cos(angle)]
                client.call('origins/setup', {'position': position})
                client.wait_ticks(3)
                aim_point(client, [point[0], point[1] + offset, point[2]])
                state = client.wait_ticks(3)
                attempts.append({'requested': position, 'position': state['player']['position'],
                                 'focus': state['focus']})
                if state['player']['life'] == 'Standing' and predicate(state['focus']):
                    return state, attempts
    raise RuntimeError('Could not focus Crazy Place prerequisite: ' + json.dumps(attempts[-8:]))


def open_crazy_place(client, report):
    for identity in ('part.gramophone_vinyl.player', 'part.gramophone_vinyl.fire'):
        part = next(item for item in client.call('origins/ui')['part_targets'] if item['id'] == identity)
        before, attempts = focus_access_target(client, part['position'],
                                               lambda focus: focus['target'] == part['entity'], .35)
        after = tap(client, 'interact')
        inventory = client.call('origins/ui')['inventory']
        collected = inventory['gramophone'] if identity.endswith('.player') else 'fire' in inventory['records']
        check(report, 'normal_crazy_place_prerequisite_collection_' + identity, collected,
              target=part, approaches=attempts, before=before['focus'],
              after=after['focus'], inventory=inventory)
    gramophone = [79.895700, -14.554200, -116.014500]
    before, attempts = focus_access_target(client, gramophone,
                                           lambda focus: 'place Gramophone' in focus['prompt'], 0)
    after = tap(client, 'interact')
    inventory = client.call('origins/ui')['inventory']
    check(report, 'normal_fire_gramophone_placement_unlocks_crazy_place_access',
          not inventory['gramophone'] and after['focus']['accepted'] > before['focus']['accepted'],
          source='src/portals/source.rs Fire gramophone; normal placement activates zone.chamber_*',
          approaches=attempts, before=before['focus'], after=after['focus'], inventory=inventory)


def planar_staff_soul(client, report, game):
    client.reset()
    charger = [256.6416, -9.179052, 207.057752]
    fixture = {'weapons': ['weapon.staff_fire_zm'], 'clear_enemies': True,
               'pending': 6, 'pause_spawning_seconds': 300, 'owned_perks': ['perk.phd_flopper'],
               'open_connections': [door['connection'] for door in game['access']['doors']]}
    client.call('origins/setup', fixture)
    client.until(lambda state: state['player']['action'] == 'Ready')
    open_crazy_place(client, report)
    client.call('origins/setup', {'position': [charger[0], -9.53, charger[2] - 2.0]})
    client.call('origins/quests/setup', {'unlocked': [True, False, False, False]})
    before = client.until(lambda s: s['player']['action'] == 'Ready')
    eye = before['player']['position'][1] + before['player']['eye_height'] - before['player']['height'] / 2
    dx = charger[0] - before['player']['position'][0]
    dz = charger[2] - before['player']['position'][2]
    client.call('origins/setup', {'yaw': math.atan2(-dx, -dz),
                                 'pitch': math.atan2(charger[1] - eye, math.hypot(dx, dz))})
    before = client.until(lambda s: 'insert staff' in s['focus']['prompt'])
    interact = binding(before, 'interact')
    inputs = [edge(1, interact, True), edge(2, interact, False)]
    deposit_batch = client.call('origins/input', {'events': inputs})
    deposited = client.until(lambda s: s['staffs']['inserted'][0]
                             and s['fixed_tick'] >= deposit_batch['finish_tick']
                             and not s['held_inputs'] and not s['queued_inputs'])
    check(report, 'normal_interaction_deposits_unlocked_fire_staff',
          deposited['staffs']['inserted'][0] and not deposited['staffs']['charged'][0],
          fixture=fixture, inputs=inputs, after=deposited['staffs'])
    client.call('origins/setup', {'weapons': ['weapon.c96_zm']})
    before = client.until(lambda s: s['player']['action'] == 'Ready' and s['player']['grounded'])
    aim_key = binding(before, 'aim')
    aim_inputs = [edge(1, aim_key, True), edge(1800, aim_key, False)]
    client.call('origins/input', {'events': aim_inputs})
    before = client.until(lambda s: s['player']['ads'] >= 0.999)
    geometry = {'enemies': [{'position': [charger[0], 10000.0, charger[2]], 'health': 1}]}
    target = client.call('origins/setup', geometry)['enemies'][0]
    before = client.until(lambda s: actor(s, target) and actor(s, target)['ready']
                          and actor(s, target)['hit_regions'] and s['player']['action'] == 'Ready'
                          and s['player']['life'] == 'Standing', timeout=90)
    target_position = actor(before, target)['position']
    planar = math.hypot(target_position[0] - charger[0], target_position[2] - charger[2])
    vertical = abs(target_position[1] - charger[1])
    check(report, 'planar_staff_soul_live_actor_precondition',
          planar < 76.2 and vertical > 76.2 and actor(before, target)['health'] == 1
          and before['staffs']['inserted'][0] and before['player']['ads'] >= 0.999,
          target=target, horizontal_distance=planar, vertical_distance=vertical,
          target_state=actor(before, target), player=before['player'], geometry_fixture=geometry)
    fire = binding(before, 'fire')
    client.release()
    attempts = []
    interval = next(w['fire_interval'] for w in game['content']['weapons'] if w['id'] == 'weapon.c96_zm')
    for attempt in range(3):
        if attempt:
            schedule(client, [edge(1, 'Escape', True), edge(2, 'Escape', False)])
            client.wait_ticks(math.ceil(interval / before['fixed_delta_seconds']) + 2)
        live = client.state()
        enemy = actor(live, target)
        if live['player']['life'] != 'Standing' or not enemy or enemy['health'] != 1:
            raise RuntimeError('Planar soul fixture lost live participants: ' + json.dumps(live['player']))
        torso = next(v for v in enemy['hit_regions'] if v['region'] == 'Torso')
        point = [(a + b) / 2 for a, b in zip(torso['start'], torso['end'])]
        position = [point[0], point[1] - 8.0, point[2] - .65]
        eye = position[1] + live['player']['eye_height'] - live['player']['height'] / 2
        pose = {'position': position, 'yaw': math.pi,
                'pitch': math.atan2(point[1] - eye, .65)}
        client.call('origins/setup', pose)
        inputs = [edge(1, aim_key, True), edge(1, fire, True), edge(2, fire, False),
                  edge(2, 'Escape', True), edge(3, 'Escape', False), edge(4, aim_key, False)]
        after = schedule(client, inputs)
        receipt = next((r for r in after['damage']['recent'] if r['target'] == target
                        and r['lethal'] and r['source'] == 'weapon.c96_zm'), None)
        attempts.append({'pose_fixture': pose, 'inputs': inputs, 'before_target': enemy,
                         'after_target': actor(after, target), 'player': after['player'],
                         'firing': after['firing']['shots'][-3:], 'receipt': receipt})
        if receipt:
            break
    else:
        check(report, 'staff_charger_uses_native_horizontal_soul_radius', False,
              attempts=attempts, geometry_fixture=geometry)
    killed = actor(after, target)
    planar = math.hypot(killed['position'][0] - charger[0], killed['position'][2] - charger[2])
    vertical = abs(killed['position'][1] - charger[1])
    check(report, 'staff_charger_uses_native_horizontal_soul_radius',
          planar < 76.2 and vertical > 76.2 and after['staffs']['souls'][0] == 1
          and after['player']['life'] == 'Standing' and after['state'] == 'Paused',
          geometry_fixture=geometry, target=target, horizontal_distance=planar,
          vertical_distance=vertical, aim_inputs=aim_inputs,
          attempts=attempts, killed_target=killed, receipts=after['damage']['recent'],
          after=after['staffs'],
          source='zm_tomb_main_quest.gsc:1059-1089; distance2dsquared and 9000000 square-inch maximum')


def run(client, report, research_path, game=None):
    if game is None:
        identity = report['identity']
        if 'asset_root' not in identity:
            identity = json.loads(Path(identity['path']).read_bytes())
        game = json.loads((Path(identity['asset_root']) / 'game.json').read_bytes())
    report['quest_methods'] = client.call('rpc.discover')
    schema = client.call('origins/quests/schema')
    expected, source = source_cycles(research_path)
    check(report, 'post_upgrade_morse_and_cipher_full_source_schedules',
          schema['light_cycles'] == expected,
          source=source,
          schedules={name: {'steps': len(steps), 'seconds': sum(s['seconds'] for s in steps),
                            'sha256': hashlib.sha256(json.dumps(steps, sort_keys=True).encode()).hexdigest()}
                     for name, steps in expected.items()},
          evidence_kind='Independent source comparison of complete schedules; runtime prefix follows')
    client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'pending': 6,
                                 'pause_spawning_seconds': 300})
    client.call('origins/quests/setup', {'upgraded_staffs': 3})
    before = client.wait_ticks(16)
    check(report, 'morse_waits_for_all_four_staff_upgrades',
          not before['staffs']['lights']['active'] and not before['staffs']['lights']['events'],
          fixture={'upgraded_staffs': 3}, after=before['staffs'])
    client.call('origins/quests/setup', {'upgraded_staffs': 4})
    started = client.until(lambda s: s['staffs']['lights']['active'])
    check(report, 'all_upgraded_gate_starts_source_ten_second_dark_interval',
          started['staffs']['lights']['events'][0] == {
              'sequence': 1, 'elapsed': 0.0, 'mode': 1, 'on': False, 'message': ''},
          fixture={'upgraded_staffs': 4}, after=started['staffs']['lights'])
    try:
        after = client.until(lambda s: s['staffs']['lights']['sequence'] >= 16
                             and settled_materials(s), timeout=120)
    except TimeoutError:
        report['light_timeout'] = client.state()['staffs']
        raise
    events = after['staffs']['lights']['events']
    branch = 'bellaso' if any(e['mode'] == 3 for e in events) else 'normal'
    steps = expected[branch]
    elapsed = 0.0
    expected_events = []
    for index, step in enumerate(steps[:len(events)]):
        expected_events.append({'sequence': index + 1, 'elapsed': elapsed,
                                'mode': step['mode'], 'on': step['on'], 'message': step['message']})
        elapsed += step['seconds']
    valid = all(actual['sequence'] == wanted['sequence']
                and abs(actual['elapsed'] - wanted['elapsed']) < 0.0001
                and all(actual[field] == wanted[field] for field in ('mode', 'on', 'message'))
                for actual, wanted in zip(events, expected_events))
    check(report, 'actual_fixed_step_morse_prefix_matches_native_timing',
          valid and len(events) >= 16 and any(e['on'] for e in events),
          branch=branch, observed=events, expected=expected_events,
          evidence_kind='Real fixed schedules after direct upgraded-staff gate fixture')
    materials = after['staffs']['light_materials']
    check(report, 'four_distinct_authored_disc_materials_follow_light_show',
          settled_materials(after) and len({m['material'] for m in materials}) == 4
          and all(max(m['emissive']) > min(m['emissive']) for m in materials),
          lights=after['staffs']['lights'], materials=materials,
          limitation='Native timing and color states are checked; absolute photometry remains a remaster approximation')
    reset = client.reset()
    check(report, 'fresh_session_resets_post_upgrade_light_sequence',
          not reset['staffs']['lights']['active'] and not reset['staffs']['lights']['events']
          and not reset['quest']['upgraded'], after=reset['staffs'])
    planar_staff_soul(client, report, game)
