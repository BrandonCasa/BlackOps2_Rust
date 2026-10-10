import argparse
import hashlib
import json
import math
from pathlib import Path
import time

from gameplay_remote import Client, write_report
from prepare_gameplay_content import external, info_string
from verify_nuke_rewards import actor, aim, prepare
from verify_throwable_down import binding, check, edge, schedule
from verify_weapon_materials import capture


def observe(client, predicate, timeout=30):
    deadline = time.monotonic() + timeout
    last = None
    while time.monotonic() < deadline:
        last = client.call('origins/animations')
        if predicate(last):
            return last
        time.sleep(0.025)
    raise TimeoutError(f'Animation condition timed out: {last}')


def weapon_rigs(observation, weapon):
    return [r for r in observation['weapons'] if r['weapon'] == weapon and not r['left']]


def pose_distance(before, after):
    return max((abs(x - y) for a, b in zip(before, after)
                for name in a['bones'].keys() & b['bones'].keys()
                for field in ('translation', 'rotation')
                for x, y in zip(a['bones'][name][field], b['bones'][name][field])), default=0)


def pause(client, resume=False, ticks=4):
    events = [edge(1, 'Escape', True), edge(2, 'Escape', False)]
    if resume:
        events += [edge(ticks + 1, 'Escape', True), edge(ticks + 2, 'Escape', False)]
    schedule(client, events)
    return client.until(lambda s: s['state'] == 'Paused', timeout=5)


def weapon_case(client, report, game, native_dir, name, aimed, empty=False, rechamber=False):
    client.reset()
    identity = 'weapon.' + name
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': [identity], 'pitch': 0}
    client.call('origins/setup', fixture)
    before = client.until(lambda s: s['player']['action'] == 'Ready')
    observe(client, lambda o: len(weapon_rigs(o, identity)) == 2
            and all(r['ready'] and r['clocks'] for r in weapon_rigs(o, identity)), timeout=90)
    weapon = next(w for w in game['content']['weapons'] if w['id'] == identity)
    native_path = native_dir / name
    fields = info_string(native_path, 'WEAPONFILE')['fields']
    shots = before['player']['weapons'][0]['magazine'] if empty else 1
    interval = math.ceil(weapon['fire_interval'] / before['fixed_delta_seconds']) + 4
    events = [edge(1, binding(before, 'aim'), True)] if aimed else []
    fire = binding(before, 'fire')
    for index in range(shots):
        tick = 120 + index * interval
        events += [edge(tick, fire, True), edge(tick + 1, fire, False)]
    final = 120 + (shots - 1) * interval
    if rechamber:
        fire_id = weapon['animation_actions']['ads_fire' if aimed else 'fire']
        clip = next(a for a in game['content']['animations'] if a['id'] == fire_id)
        final += math.ceil(clip['notifications'][-1]['seconds'] / before['fixed_delta_seconds'])
    events += [edge(final + 3, 'Escape', True), edge(final + 4, 'Escape', False)]
    if aimed:
        events += [edge(final + 30, binding(before, 'aim'), False)]
    schedule(client, events)
    after = client.until(lambda s: s['state'] == 'Paused', timeout=5)
    frozen = client.call('origins/animations')
    rigs = weapon_rigs(frozen, identity)
    key = ('adsRechamberAnim' if aimed else 'rechamberAnim') if rechamber else (
        ('adsLastShotAnim' if aimed else 'lastShotAnim') if empty else
        ('adsFireAnim' if aimed else 'fireAnim'))
    expected = fields[key]
    label = name + ('_ads' if aimed else '_hip') + ('_last' if empty else '') + ('_rechamber' if rechamber else '')
    report.setdefault('captures', {})[label] = capture(client, Path(report['output']) / (label + '.png'))
    check(report, label + '_native_clip_and_normal_firing',
          after['state'] == 'Paused' and after['session']['shots'] - before['session']['shots'] == shots
          and after['player']['weapons'][0]['magazine'] == before['player']['weapons'][0]['magazine'] - shots
          and len(rigs) == 2 and all(r['ready'] and r['bones'] and r['clocks']
                                   and all(c['clip'] == expected for c in r['clocks']) for r in rigs),
          native={'path': str(native_path), 'sha256': hashlib.sha256(native_path.read_bytes()).hexdigest(),
                  'field': key, 'clip': expected}, fixture=fixture, inputs=events, state=after, animation=frozen)
    client.wait_ticks(15)
    later = weapon_rigs(client.call('origins/animations'), identity)
    check(report, label + '_pause_freezes_clocks_and_pose',
          [r['clocks'] for r in rigs] == [r['clocks'] for r in later]
          and pose_distance(rigs, later) < 0.00001, animation=later)
    pause(client, resume=True, ticks=3)
    advanced = weapon_rigs(client.call('origins/animations'), identity)
    check(report, label + '_normal_resume_advances_actual_pose',
          len(advanced) == 2 and pose_distance(rigs, advanced) > 0.00001,
          pose_delta=pose_distance(rigs, advanced), animation=advanced)


def enemy_animation(client, entity):
    return next((e for e in client.call('origins/animations')['enemies'] if e['entity'] == entity), None)


def zombie_cases(client, report, game, states):
    client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'pending': 6,
                                'pause_spawning_seconds': 300, 'owned_perks': ['perk.juggernog']})
    state = client.until(lambda s: s['player']['health'] == 250
                         and abs(s['player']['velocity'][1]) < 0.001)
    home = state['player']['position']
    result = client.call('origins/setup', {'enemies': [
        {'position': [home[0], home[1], home[2] + 1.25], 'health': 1000000}]})
    target = result['enemies'][0]
    seen = []
    damage_samples = []
    previous_health = state['player']['health']
    deadline = time.monotonic() + 40
    while time.monotonic() < deadline:
        observation = enemy_animation(client, target)
        gameplay = client.state()
        if gameplay['player']['health'] != previous_health:
            damage_samples.append({'before': previous_health, 'after': gameplay['player']['health'],
                                   'animation': observation, 'tick': gameplay['fixed_tick']})
            previous_health = gameplay['player']['health']
        if observation and observation['action'] == 'Attack' and observation['rigs'] and all(
                r['ready'] and r['clocks'] for r in observation['rigs']):
            if not seen or seen[-1]['sequence'] != observation['sequence']:
                seen.append(observation)
            if len(seen) >= 2:
                pause(client)
                break
        time.sleep(0.025)
    frozen = enemy_animation(client, target)
    check(report, 'zombie_repeated_native_walk_melee', len(seen) >= 2
          and all(e['combat_state'] == 'WalkMelee'
                  and e['combat_clip'].rsplit('.', 1)[-1] in states['walk_melee']
                  and e['hits'] and all(r['clip'] == e['combat_clip'].rsplit('.', 1)[-1]
                                       and all(abs(c - min(e['elapsed'], e['seconds'])) < 0.00001
                                               for c in r['clocks']) for r in e['rigs']) for e in seen),
          attacks=seen)
    after = client.state()
    check(report, 'zombie_native_hit_notetrack_causes_melee_damage',
          after['player']['health'] < state['player']['health']
          and any(s['after'] < s['before'] and s['animation']['hits']
                  and s['animation']['elapsed'] >= s['animation']['hits'][0] - 2 * state['fixed_delta_seconds']
                  for s in damage_samples), baseline=state, state=after, samples=damage_samples)
    client.wait_ticks(15)
    later = enemy_animation(client, target)
    check(report, 'zombie_pause_freezes_body_and_head', frozen == later,
          before=frozen, after=later)
    report.setdefault('captures', {})['melee'] = capture(client, Path(report['output']) / 'melee.png')
    left = binding(after, 'left')
    events = [edge(1, 'Escape', True), edge(2, 'Escape', False), edge(3, left, True),
              edge(11, left, False), edge(13, 'Escape', True), edge(14, 'Escape', False)]
    schedule(client, events)
    moved = client.until(lambda s: s['state'] == 'Paused', timeout=5)
    facing = enemy_animation(client, target)
    delta = [moved['player']['position'][i] - facing['position'][i] for i in (0, 2)]
    length = math.hypot(*delta)
    alignment = sum(delta[j] * facing['forward'][i] for j, i in enumerate((0, 2))) / length
    check(report, 'melee_faces_player_after_normal_strafe', facing['action'] == 'Attack'
          and facing['sequence'] == frozen['sequence'] and alignment > 0.999
          and math.dist(after['player']['position'], moved['player']['position']) > 0.1,
          alignment=alignment, before=after, after=moved, animation=facing, inputs=events)
    before, target, fixture = prepare(client, 1, False)
    observe(client, lambda o: any(e['entity'] == target and e['rigs'] and all(
        r['ready'] and r['clocks'] for r in e['rigs']) for e in o['enemies']), timeout=90)
    aim_setup = aim(client, client.state(), target, 'Torso')
    before = client.state()
    events = [edge(1, binding(before, 'aim'), True), edge(120, binding(before, 'fire'), True),
              edge(121, binding(before, 'fire'), False), edge(125, 'Escape', True),
              edge(126, 'Escape', False), edge(150, binding(before, 'aim'), False)]
    schedule(client, events)
    after = client.until(lambda s: s['state'] == 'Paused', timeout=5)
    death = enemy_animation(client, target)
    report.setdefault('captures', {})['death'] = capture(client, Path(report['output']) / 'death.png')
    check(report, 'normal_bullet_kill_selects_native_death', death and death['action'] == 'Death'
          and death['combat_state'] == 'Death'
          and death['combat_clip'].rsplit('.', 1)[-1] in states['death']
          and any(r['target'] == target and r['lethal'] for r in after['damage']['recent'])
          and all(r['clip'] == death['combat_clip'].rsplit('.', 1)[-1] for r in death['rigs']),
          fixture=fixture, aim=aim_setup, inputs=events, state=after, animation=death)
    pause(client, resume=True, ticks=20)
    advanced = enemy_animation(client, target)
    check(report, 'death_body_and_head_advance_together', advanced['elapsed'] > death['elapsed']
          and all(all(abs(c - min(advanced['elapsed'], advanced['seconds'])) < 0.00001
                      for c in r['clocks']) for r in advanced['rigs'])
          and any(a['pose'] != b['pose'] for a, b in zip(death['rigs'], advanced['rigs'])), animation=advanced)
    pause(client, resume=True, ticks=math.ceil(death['seconds'] / after['fixed_delta_seconds']) + 8)
    end = enemy_animation(client, target)
    pause(client, resume=True, ticks=20)
    held = enemy_animation(client, target)
    check(report, 'native_death_holds_final_pose', end and held
          and all(all(abs(c - end['seconds']) < 0.00001 for c in r['clocks']) for r in end['rigs'])
          and end['rigs'] == held['rigs'], before=end, after=held)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--identity', required=True, type=Path)
    parser.add_argument('--preparation', required=True, type=Path)
    parser.add_argument('--native-weapons', required=True, type=Path)
    args = parser.parse_args()
    args.output = external(args.output)
    args.output.mkdir(parents=True, exist_ok=False)
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'native_weapon_and_zombie_animations',
              'output': str(args.output.resolve()),
              'limitations': ['Grants, actor placement, aim, spawn delay and Zombie Blood are scenario setup.',
                              'Firing, ADS and pause use scheduled normal input; melee is normal AI.',
                              'Runtime melee coverage is the walking German zombie; other state bindings use asset validation.',
                              'Mesh consistency checks cover GPU input uniforms, not GPU output uniforms or deformed vertices.',
                              'Native ragdoll handoff and engine RNG remain approximations.']}
    try:
        identity = json.loads(args.identity.read_bytes())
        report['identity'] = {'path': str(args.identity.resolve()),
                              'sha256': hashlib.sha256(args.identity.read_bytes()).hexdigest()}
        report['scripts'] = {name: hashlib.sha256(Path(__file__).with_name(name).read_bytes()).hexdigest()
                             for name in ('verify_animation_fixes.py', 'gameplay_remote.py',
                                          'verify_throwable_down.py', 'verify_nuke_rewards.py',
                                          'verify_weapon_materials.py')}
        pack = Path(identity['asset_root'])
        game_path = pack / 'game.json'
        check(report, 'runtime_content_matches_identity', hashlib.sha256(game_path.read_bytes()).hexdigest()
              == identity['assets']['game.json']['sha256'])
        game = json.loads(game_path.read_bytes())
        report['preparation'] = {'path': str(args.preparation.resolve()),
                                 'sha256': hashlib.sha256(args.preparation.read_bytes()).hexdigest()}
        states = json.loads(args.preparation.read_bytes())['states']
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        for name, aimed, empty, rechamber in (
                ('mp40_zm', False, False, False), ('mp40_zm', True, False, False),
                ('ballista_zm', True, False, False), ('ballista_zm', True, False, True),
                ('c96_zm', True, True, False)):
            weapon_case(client, report, game, args.native_weapons, name, aimed, empty, rechamber)
        zombie_cases(client, report, game, states)
        pipelines = client.call('origins/materials')['pipelines']
        mesh = pipelines['mesh_consistency']
        check(report, 'gpu_input_mesh_and_skin_offsets_match_allocator', pipelines['error'] is None
              and mesh['frames'] > 0 and mesh['checked'] > 0 and mesh['mismatch_frames'] == 0,
              pipelines=pipelines)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        try:
            client.release()
            client.reset()
            client.release()
        except Exception as error:
            report['passed'] = False
            report['cleanup_error'] = str(error)
        path = write_report(args.output, 'animations.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
