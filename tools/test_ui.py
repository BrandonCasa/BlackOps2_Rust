import argparse
import json
import math
from pathlib import Path
import time

from test_support import Client, write_report
from test_brp import assets_match, digest
from test_environment import aim, approach, tap
from test_generator_population import focus_generator
from test_throwable_down import binding, check, edge, schedule


def observe(client, predicate=lambda ui: True, timeout=20):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        ui = client.call('origins/ui')
        if predicate(ui):
            return ui
        time.sleep(.05)
    raise TimeoutError('UI condition timed out: ' + json.dumps(ui))


def text(ui, value):
    return any(value in label['text'] for label in ui['labels'])


def capture(client, output, name):
    if output is None:
        return None
    path = output / (name + '.png')
    client.call('origins/capture', {'path': str(path)})
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if path.is_file() and path.stat().st_size > 100:
            return str(path)
        time.sleep(.1)
    raise TimeoutError('Screenshot did not finish: ' + str(path))


def hold(client, key):
    client.call('origins/input', {'events': [edge(1, key, True), edge(1800, key, False)]})


def key(client, value):
    return schedule(client, [edge(1, value, True), edge(4, value, False)])


def select_menu(client, index):
    for _ in range(20):
        if client.call('origins/ui')['selected_menu_row'] == index:
            return
        key(client, 'ArrowDown')
    raise TimeoutError('Menu selection did not reach row ' + str(index))


def setup_playing(client, options, stage):
    try:
        return client.call('origins/setup', options)
    except RuntimeError as error:
        state = client.state()
        player = state.get('player') or {}
        player = {name: player.get(name) for name in ('life', 'health', 'position', 'grounded', 'mobility')}
        raise RuntimeError(f'{stage}: {error}; state={state["state"]}; '
                           f'player={player}; '
                           f'landings={state.get("movement", {}).get("events", [])[-4:]}') from error


def run(client, report, output, game=None):
    if game is None:
        identity = report['identity']
        if 'asset_root' not in identity:
            identity = json.loads(Path(identity['path']).read_bytes())
        game = json.loads((Path(identity['asset_root']) / 'game.json').read_bytes())
    client.until(lambda state: state['state'] == 'Playing' and state.get('player'), timeout=300)
    report['protocol'] = client.call('rpc.discover')
    report['schema'] = client.call('origins/schema')
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300,
               'owned_perks': [], 'weapons': ['weapon.c96_zm']}
    client.call('origins/setup', fixture)
    before = client.wait_ticks(20)
    ui = observe(client, lambda ui: text(ui, '500') and ui['crosshair']
                 and ui['loaded_images'] == ui['visible_images'])
    check(report, 'original_hud_artwork_and_bitmap_font_loaded',
          'ui/hud_tomb_dpad.ktx2' in ui['images']
          and 'ui/gamefonts_pc_720.ktx2' in ui['images']
          and 'ui/chalkmarks_hellcatraz_1.ktx2' in ui['images']
          and any('hud_tomb_zom_num_' in path for path in ui['images'])
          and not text(ui, 'Health'), fixture=fixture, ui=ui,
          capture=capture(client, output, 'hud'))
    fire = binding(before, 'fire')
    schedule(client, [edge(1, fire, True), edge(4, fire, False)])
    after = client.until(lambda state: state['session']['shots'] > before['session']['shots'])
    ui = observe(client, lambda ui: ui['crosshair'])
    check(report, 'normal_fire_updates_original_ammo_digits',
          after['player']['weapons'][0]['magazine'] == before['player']['weapons'][0]['magazine'] - 1
          and any('hud_tomb_zom_num_' in path for path in ui['images']), before=before, after=after, ui=ui)
    hold(client, binding(after, 'aim'))
    aimed = client.until(lambda state: state['player']['ads'] > .95)
    ui = observe(client, lambda ui: not ui['crosshair'])
    check(report, 'normal_aim_hides_crosshair', True, after=aimed, ui=ui)
    client.release()
    if before['controls']['aim_mode'] == 'toggle':
        key(client, binding(before, 'aim'))
    client.until(lambda state: state['player']['ads'] < .05)
    hold(client, binding(client.state(), 'scoreboard'))
    ui = observe(client, lambda ui: text(ui, 'Musical Parts') and text(ui, 'Headshots'))
    check(report, 'normal_scoreboard_input_shows_inventory_and_stats',
          not ui['crosshair'] and 'ui/zm_capture_meter_dial.ktx2' in ui['images'], ui=ui,
          capture=capture(client, output, 'scoreboard'))
    client.release()
    observe(client, lambda ui: not text(ui, 'Headshots') and ui['crosshair'])

    position = client.state()['player']['position']
    client.call('origins/setup', {'pickups': [{'id': 'powerup.double_points', 'position': position}]})
    picked = client.until(lambda state: state['effects']['double_points'] > 0)
    ui = observe(client, lambda ui: 'ui/specialty_2x_zombies.ktx2' in ui['images'])
    check(report, 'normal_powerup_collection_shows_original_timed_icon', True, after=picked, ui=ui)
    collected = picked['drops']['collected']
    client.call('origins/setup', {'pickups': [{'id': 'powerup.full_ammo', 'position': picked['player']['position']}]})
    picked = client.until(lambda state: state['drops']['collected'] > collected)
    ui = observe(client, lambda ui: text(ui, 'Max Ammo!'))
    check(report, 'instant_powerup_shows_native_announcement', True, after=picked, ui=ui,
          capture=capture(client, output, 'max-ammo'))

    before, shovel = approach(client, 'shovel')
    ui = observe(client, lambda ui: any(label['text'] == before['focus']['prompt'] for label in ui['labels']))
    check(report, 'native_interaction_prompt_matches_gameplay_focus',
          bool(before['focus']['prompt']), before=before, ui=ui,
          capture=capture(client, output, 'prompt'))
    tap(client, 'interact')
    picked = client.until(lambda state: state['environment']['digger']['state']['shovel'])
    ui = observe(client, lambda ui: 'ui/zom_hud_craftable_tank_shovel.ktx2' in ui['images'])
    check(report, 'normal_shovel_interaction_updates_equipment_icon', True, after=picked, ui=ui)

    client.reset()
    collection_fixture = {**fixture, 'owned_perks': ['perk.phd_flopper'],
                          'open_connections': [door['connection'] for door in game['access']['doors']]}
    setup_playing(client, collection_fixture, 'gramophone collection fixture')
    ui = client.call('origins/ui')
    part = next(item for item in ui['part_targets'] if item['id'] == 'part.gramophone_vinyl.player')
    collected = False
    point = part['position']
    approaches = [
        [point[0] + radius * math.sin(index * math.tau / 8), point[1] + height,
         point[2] + radius * math.cos(index * math.tau / 8)]
        for height in (.3, .91) for radius in (.8, 1.2, 1.7) for index in range(8)]
    attempts = []
    for position in approaches:
        setup_playing(client, {'position': position}, f'gramophone approach {position}')
        state = client.wait_ticks(3)
        if state['state'] != 'Playing' or state['player']['life'] != 'Standing':
            raise RuntimeError('Gramophone approach left active gameplay before aim: '
                               + json.dumps({'state': state['state'],
                                             'life': state['player']['life'],
                                             'position': state['player']['position'],
                                             'movement': state['movement']['events'][-4:]}))
        aim(client, [point[0], point[1] + .35, point[2]])
        state = client.wait_ticks(3)
        player = state['player']
        eye = list(player['position'])
        eye[1] += player['eye_height'] - player['height'] / 2
        attempts.append({'requested_position': position, 'position': player['position'],
                         'grounded': player['grounded'], 'eye': eye,
                         'target_distance': math.dist(eye, [point[0], point[1] + .35, point[2]]),
                         'focus': state['focus']})
        if state['state'] != 'Playing' or state['player']['life'] != 'Standing':
            raise RuntimeError('Gramophone approach left active gameplay: '
                               + json.dumps({'state': state['state'], 'player': state['player'],
                                             'movement': state['movement']['events'][-4:]}))
        if state['focus']['target'] == part['entity']:
            tap(client, 'interact')
            collected = True
            break
    check(report, 'normal_gramophone_interaction_collects_item', collected, target=part,
          fixture=collection_fixture, approaches=attempts,
          fixture_note='Opened authored connections admit excavation approaches; PhD protects fixture teleports from landing damage.')
    ui = observe(client, lambda ui: ui['inventory']['gramophone'] and text(ui, 'Musical Parts'))
    check(report, 'collection_shows_original_inventory_preview',
          'ui/zom_hud_craftable_gramophone.ktx2' in ui['images'], ui=ui,
          capture=capture(client, output, 'inventory-preview'))

    client.reset()
    client.call('origins/setup', fixture)
    site = next(site for site in client.state()['generators']['sites'] if site['number'] == 1)
    before = focus_generator(client, site)
    tap(client, 'interact')
    after = client.until(lambda state: state['generators']['activations'] > before['generators']['activations'])
    ui = observe(client, lambda ui: 'ui/tomb_capture_spin.ktx2' in ui['images'])
    check(report, 'normal_generator_purchase_shows_capture_hud',
          after['session']['points'] == before['session']['points'] - site['price'], ui=ui,
          before=before, after=after, capture=capture(client, output, 'generator-capturing'))
    deadline = time.monotonic() + 45
    while time.monotonic() < deadline:
        state = client.wait_ticks(6)
        if next(item for item in state['generators']['sites'] if item['id'] == site['id'])['state'] == 'Active':
            break
        client.call('origins/setup', {'clear_enemies': True})
    else:
        raise TimeoutError('Generator capture did not complete')
    ui = observe(client, lambda ui: 'ui/tomb_capture_spin.ktx2' not in ui['images'])
    hold(client, binding(client.state(), 'scoreboard'))
    ui = observe(client, lambda ui: text(ui, 'Captures'))
    check(report, 'capture_completion_updates_native_inventory_wheel', True, ui=ui, after=state,
          fixture_note='Defenders cleared between observations; player remains inside the normal capture area.',
          capture=capture(client, output, 'generator-captured'))
    client.release()

    client.reset()
    perk_fixture = {'clear_enemies': True, 'pause_spawning_seconds': 300,
                    'owned_perks': ['perk.juggernog', 'perk.speed_cola', 'perk.stamin_up', 'perk.double_tap'],
                    'weapons': ['weapon.staff_fire_zm']}
    client.call('origins/setup', perk_fixture)
    client.wait_ticks(20)
    ui = observe(client, lambda ui: 'ui/hud_flamethrower_reticle.ktx2' in ui['images'])
    check(report, 'native_perk_icons_and_staff_center_reticle',
          all('ui/specialty_' + name + '_zombies.ktx2' in ui['images']
              for name in ['juggernaut', 'fastreload', 'marathon', 'doubletap']),
          fixture=perk_fixture, ui=ui, capture=capture(client, output, 'perks-and-staff'))
    hold(client, binding(client.state(), 'aim'))
    definition = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.staff_fire_zm')
    aimed = client.wait_ticks(math.ceil(definition['mechanics']['ads_seconds'][0]
                                       / client.state()['fixed_delta_seconds']) + 4)
    ui = observe(client, lambda ui: ui['crosshair'])
    check(report, 'native_staff_keeps_reticle_during_normal_aim_input',
          'ui/hud_flamethrower_reticle.ktx2' in ui['images']
          and (aimed['player']['ads'] > .95 if definition['mechanics']['ads_enabled']
               else aimed['player']['ads'] == 0),
          ads_enabled=definition['mechanics']['ads_enabled'], after=aimed, ui=ui)
    client.release()
    client.reset()
    client.call('origins/setup', fixture)

    original_audio = client.state()['controls']['audio'].copy()
    key(client, 'Escape')
    client.until(lambda state: state['state'] == 'Paused')
    ui = observe(client, lambda ui: text(ui, 'PAUSED'))
    check(report, 'normal_pause_opens_native_menu', not ui['crosshair'], ui=ui,
          capture=capture(client, output, 'pause'))
    select_menu(client, 2)
    key(client, 'Enter')
    ui = observe(client, lambda ui: text(ui, 'AUDIO') and text(ui, 'Master Volume'))
    select_menu(client, 0)
    adjust, restore = ('ArrowLeft', 'ArrowRight') if original_audio['master'] >= .05 else ('ArrowRight', 'ArrowLeft')
    key(client, adjust)
    changed = client.until(lambda state: abs(state['controls']['audio']['master'] - original_audio['master']) > .001)
    check(report, 'audio_menu_adjusts_actual_mixer_settings', True, ui=ui, after=changed)
    key(client, restore)
    key(client, 'Escape')
    observe(client, lambda ui: text(ui, 'PAUSED'))
    select_menu(client, 1)
    key(client, 'Enter')
    ui = observe(client, lambda ui: text(ui, 'CONTROLS') and text(ui, 'Mouse sensitivity'))
    check(report, 'normal_controls_opens_native_binding_menu', True, ui=ui,
          capture=capture(client, output, 'controls'))
    key(client, 'Escape')
    observe(client, lambda ui: text(ui, 'PAUSED'))
    key(client, 'Escape')
    client.until(lambda state: state['state'] == 'Playing')
    check(report, 'audio_back_resume_and_persistence',
          abs(client.state()['controls']['audio']['master'] - original_audio['master']) < .001)

    before = client.state()
    client.call('origins/setup', {'blast': {'session_id': before['session_id'],
                'center': before['player']['position'], 'radius': 2, 'damage': 1000000}})
    after = client.until(lambda state: state['state'] == 'GameOver', timeout=45)
    ui = observe(client, lambda ui: text(ui, 'GAME OVER') and text(ui, 'Headshots'))
    check(report, 'gameplay_death_shows_native_final_scoreboard',
          not ui['crosshair'], after=after, ui=ui, capture=capture(client, output, 'game-over'))
    client.reset()
    ui = observe(client, lambda ui: text(ui, '500') and not text(ui, 'GAME OVER') and ui['crosshair'])
    check(report, 'session_reset_clears_ui_state', True, ui=ui)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--identity', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    if output.is_relative_to(Path(__file__).resolve().parent.parent):
        raise ValueError('Validation output must be external')
    output.mkdir(parents=True, exist_ok=False)
    identity = json.loads(args.identity.read_bytes())
    report = {'passed': False, 'cases': [], 'identity': identity,
              'limitations': ['Solo gameplay only; no multiplayer or pixel-parity claim.',
                              'Advanced quests and completed recipes additionally require their gameplay scenarios.']}
    client = Client(args.url, request_timeout=20)
    try:
        report['assets_before'] = assets_match(identity)
        for name, expected in identity['binaries'].items():
            path = args.identity.parent / name
            if digest(path) != expected:
                raise ValueError('Pinned executable/DLL changed: ' + str(path))
        game = json.loads((Path(identity['asset_root']) / 'game.json').read_bytes())
        run(client, report, output, game)
        report['assets_after'] = assets_match(identity)
        report['passed'] = True
    except Exception as error:
        report['error'] = type(error).__name__ + ': ' + str(error)
    finally:
        client.shutdown(report)
        target = write_report(output, 'ui.json', report)
    print(json.dumps({'passed': report['passed'], 'report': str(target), 'error': report.get('error')}))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
