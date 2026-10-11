import argparse
import hashlib
import json
import math
from pathlib import Path
import time

from gameplay_remote import Client, write_report
from verify_animation_fixes import observe, pause, pose_distance, weapon_rigs
from verify_generator_population import focus_generator, interaction_key
from verify_throwable_down import binding, check, edge, schedule
from verify_weapon_materials import capture


def snapshot(client, predicate, timeout=30):
    return observe(client, lambda value: predicate(value['players']), timeout)['players']


def source_channels(pack, game, weapon_id):
    result = {}
    weapon = next(w for w in game['content']['weapons'] if w['id'] == weapon_id)
    names = {name.rsplit('.', 1)[-1] for name in weapon['animation_actions'].values()}
    paths = {animation['path'].split('#')[0] for animation in game['content']['animations']
             if animation['id'].rsplit('.', 1)[-1] in names}
    for name in sorted(paths):
        path = pack / name
        if path.suffix != '.gltf' or not path.is_file():
            continue
        document = json.loads(path.read_bytes())
        for animation in document.get('animations', []):
            channels = result.setdefault(animation['name'], set())
            for channel in animation['channels']:
                target = channel['target']
                channels.add((document['nodes'][target['node']].get('name'), target['path']))
    return result


def weapon_case(client, report, game, channels):
    client.reset()
    weapon = 'weapon.mp40_zm'
    client.call('origins/setup', {'clear_enemies': True, 'pending': 6,
                                'pause_spawning_seconds': 300, 'weapons': [weapon], 'pitch': 0})
    before = client.until(lambda s: s['player']['action'] == 'Ready')
    observe(client, lambda o: len(weapon_rigs(o, weapon)) == 2
            and all(r['ready'] and r['clocks'] for r in weapon_rigs(o, weapon)), 90)
    schedule(client, [edge(1, binding(before, 'fire'), True), edge(2, binding(before, 'fire'), False),
                      edge(6, binding(before, 'reload'), True), edge(7, binding(before, 'reload'), False),
                      edge(12, 'Escape', True), edge(13, 'Escape', False)])
    state = client.state()
    frozen = weapon_rigs(client.call('origins/animations'), weapon)
    check(report, 'normal_reload_uses_shared_sampled_playback', state['state'] == 'Paused'
          and state['player']['action'].startswith('Reloading')
          and all(r['action'] in ('Reload', 'ReloadEmpty') for r in frozen), state=state, rigs=frozen)
    client.wait_ticks(20)
    later = weapon_rigs(client.call('origins/animations'), weapon)
    check(report, 'reload_pause_restores_identical_pose', pose_distance(frozen, later) < 1e-5
          and [r['clocks'] for r in frozen] == [r['clocks'] for r in later])
    report['captures']['paused_reload'] = capture(client, Path(report['output']) / 'paused-reload.png')
    schedule(client, [edge(1, 'Escape', True), edge(2, 'Escape', False)])
    client.until(lambda s: s['state'] == 'Playing' and s['player']['action'] == 'Ready')
    idle = observe(client, lambda o: all(r['action'] == 'Idle' for r in weapon_rigs(o, weapon)))
    names = {clock['clip'] for rig in weapon_rigs(idle, weapon) for clock in rig['clocks']}
    players = [p for p in idle['players'] if p['enabled'] and p['layers']
               and all(layer['clip'] in names for layer in p['layers'])]
    checked, failures = 0, []
    for player in players:
        active_channels = set().union(*(channels.get(layer['clip'], set()) for layer in player['layers']))
        for bone in player['bones']:
            if bone['name'] == 'tag_torso':
                continue
            for field in ('translation', 'rotation'):
                if (bone['name'], field) in active_channels:
                    continue
                checked += 1
                a, b = bone[field], bone['rest_' + field]
                distance = max(abs(x - y) for x, y in zip(a, b))
                if field == 'rotation':
                    distance = min(distance, max(abs(x + y) for x, y in zip(a, b)))
                if distance > 1e-5:
                    failures.append({'bone': bone['name'], 'field': field, 'error': distance})
    check(report, 'sparse_idle_channels_return_to_bind_pose_after_reload', checked > 0 and not failures,
          checked_properties=checked, failures=failures)
    check(report, 'sampled_to_free_playback_restores_speed', players and all(
        layer['speed'] == 1 and layer['requested_seconds'] is None and not layer['paused']
        for p in players for layer in p['layers']))


def generator_case(client, report):
    state = client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'points': 5000, 'pending': 6,
                                'pause_spawning_seconds': 300})
    site = next(s for s in client.state()['generators']['sites'] if s['number'] == 1)
    state = focus_generator(client, site)
    schedule(client, [edge(1, interaction_key(state), True), edge(2, interaction_key(state), False)])
    state = client.until(lambda s: next(g for g in s['generators']['sites'] if g['id'] == site['id'])['progress'] > 0.2)
    players = snapshot(client, lambda ps: any(any(l['clip'] == 'fxanim_zom_tomb_generator_up_idle_anim'
                        for l in p['layers']) for p in ps))
    selected = next(p for p in players if any(l['clip'] == 'fxanim_zom_tomb_generator_up_idle_anim' for l in p['layers']))
    entity = selected['entity']
    layers = {l['clip']: l for l in selected['layers']}
    up, down = (layers['fxanim_zom_tomb_generator_fluid_' + n + '_anim'] for n in ('up', 'down'))
    check(report, 'generator_progress_blends_native_up_and_down', 0 < up['requested_weight'] < 1
          and abs(up['requested_weight'] + down['requested_weight'] - 1) < 1e-5
          and 0 < up['weight'] < 1 and abs(up['weight'] + down['weight'] - 1) < 1e-4,
          player=selected, state=state)
    pause(client)
    first = next(p for p in client.call('origins/animations')['players'] if p['entity'] == entity)
    client.wait_ticks(20)
    second = next(p for p in client.call('origins/animations')['players'] if p['entity'] == entity)
    check(report, 'generator_pause_freezes_pose_clocks_and_weight_blends', first == second)
    report['captures']['generator_blend'] = capture(client, Path(report['output']) / 'generator-blend.png')
    schedule(client, [edge(1, 'Escape', True), edge(2, 'Escape', False)])
    deadline = time.monotonic() + 180
    while time.monotonic() < deadline:
        client.call('origins/setup', {'clear_enemies': True})
        state = client.state()
        if next(s for s in state['generators']['sites'] if s['id'] == site['id'])['state'] == 'Active':
            break
        time.sleep(0.15)
    else:
        raise TimeoutError('Normal generator capture did not complete')
    players = snapshot(client, lambda ps: any(p['entity'] == entity and any(
        l['clip'] == 'fxanim_zom_tomb_generator_fluid_rotate_up_anim' for l in p['layers']) for p in ps))
    check(report, 'normal_capture_starts_native_rotate_up', state['generators']['completions'] > 0,
          player=next(p for p in players if p['entity'] == entity), state=state)
    client.reset()
    players = snapshot(client, lambda ps: any(any(l['clip'] == 'fxanim_zom_tomb_generator_fluid_down_anim'
                        for l in p['layers']) for p in ps))
    generators = [p for p in players if any((l['clip'] or '').startswith('fxanim_zom_tomb_generator_') for l in p['layers'])]
    check(report, 'fresh_session_clears_generator_layers_and_goal_weights', generators and all(
        not any('rotate_' in (l['clip'] or '') for l in p['layers'])
        and all(l['requested_weight'] == (0 if '_fluid_up_' in l['clip'] else 1) for l in p['layers'])
        for p in generators))


def lure_case(client, report, game):
    state = client.reset()
    weapon = next(w for w in game['content']['weapons'] if w.get('lure'))
    client.call('origins/setup', {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
                                'equipment': [weapon['id']], 'pitch': -1.3})
    state = client.until(lambda s: s['player']['action'] == 'Ready')
    hold = math.ceil(weapon['throwable']['prime_seconds'] / state['fixed_delta_seconds']) + 3
    schedule(client, [edge(1, binding(state, 'tactical'), True), edge(hold, binding(state, 'tactical'), False)])
    players = snapshot(client, lambda ps: any(p['enabled'] and p['layers'] and all(l['clip'] is None for l in p['layers']) for p in ps))
    player = next(p for p in players if p['enabled'] and p['layers'] and all(l['clip'] is None for l in p['layers']))
    pause(client)
    frozen = next(p for p in client.call('origins/animations')['players'] if p['entity'] == player['entity'])
    client.wait_ticks(15)
    later = next(p for p in client.call('origins/animations')['players'] if p['entity'] == player['entity'])
    check(report, 'normal_monkey_throw_pauses_shared_loop', frozen == later and frozen['bones']
          and all(l['paused'] for l in frozen['layers']), player=frozen)
    report['captures']['monkey'] = capture(client, Path(report['output']) / 'monkey.png')
    schedule(client, [edge(1, 'Escape', True), edge(2, 'Escape', False)])
    client.wait_ticks(12)
    resumed = next(p for p in client.call('origins/animations')['players'] if p['entity'] == player['entity'])
    check(report, 'monkey_resume_advances_native_loop', resumed['bones'] != frozen['bones']
          and all(not l['paused'] and l['speed'] == 1 for l in resumed['layers']))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--identity', required=True, type=Path)
    parser.add_argument('--reference', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    args.output = args.output.resolve()
    if args.output.is_relative_to(Path(__file__).resolve().parents[1]):
        parser.error('Output must be outside the repository')
    args.output.mkdir(parents=True, exist_ok=False)
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'captures': {}, 'output': str(args.output), 'limitations': [
        'Inventory, placement and enemy removal are scenario setup; firing, reload, pause, generator capture and throwing use normal input.',
        'This checks installed animation integration, not complete native animtree or ragdoll parity.']}
    try:
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), 180)
        report['protocol'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/schema')
        identity = json.loads(args.identity.read_bytes())
        report['identity'] = {'path': str(args.identity.resolve()), 'sha256': hashlib.sha256(args.identity.read_bytes()).hexdigest()}
        reference = args.reference / 'source/client-tomb/clientscripts/mp/zm_tomb_capture_zones.csc'
        report['reference'] = {'path': str(reference.resolve()), 'sha256': hashlib.sha256(reference.read_bytes()).hexdigest()}
        report['script_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        pack = Path(identity['asset_root'])
        game_bytes = (pack / 'game.json').read_bytes()
        game_hash = hashlib.sha256(game_bytes).hexdigest()
        if game_hash != identity['assets']['game.json']['sha256']:
            raise RuntimeError('Runtime gameplay data differs from the pinned asset identity')
        report['game_sha256'] = game_hash
        game = json.loads(game_bytes)
        weapon_case(client, report, game, source_channels(pack, game, 'weapon.mp40_zm'))
        generator_case(client, report)
        lure_case(client, report, game)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'animation-ecs.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'error': report.get('error'), 'report': str(path)}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
