import argparse
import json
from pathlib import Path

from test_support import Client, write_report
from test_throwable_down import binding, edge, schedule


def check(report, name, condition, **evidence):
    report['cases'].append({'case': name, 'passed': bool(condition), **evidence})
    if not condition:
        raise RuntimeError(name)


def press(client, *keys):
    client.call('origins/input', {'events': [
        {'tick': index * 10 + offset, 'key': key, 'pressed': pressed}
        for index, key in enumerate(keys)
        for offset, pressed in ((1, True), (2, False))
    ]})


def run(client, report):
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=120)
    client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'pause_spawning_seconds': 30,
                                  'weapons': ['weapon.c96_zm']})
    client.until(lambda s: s['player']['action'] == 'Ready')
    press(client, 'MouseLeft', 'KeyR', 'Escape')
    paused = client.until(lambda s: s['state'] == 'Paused')
    check(report, 'pause_during_native_reload', paused['player']['action'].startswith('Reloading'),
          before=paused)
    later = client.wait_ticks(120)
    check(report, 'paused_reload_and_round_timer',
          paused['player']['action'] == later['player']['action']
          and paused['player']['weapons'] == later['player']['weapons']
          and paused['session']['spawn_remaining'] == later['session']['spawn_remaining'],
          after=later)
    check(report, 'paused_player_pose', paused['player']['position'] == later['player']['position'])
    check(report, 'scheduled_inputs_released', not later['held_inputs'] and not later['queued_inputs'])
    press(client, 'Escape')
    resumed = client.until(lambda s: s['state'] == 'Playing' and s['player']['action'] == 'Ready')
    check(report, 'resume_completes_reload', resumed['player']['weapons'][0]['magazine'] == 8
          and resumed['session']['spawn_remaining'] < later['session']['spawn_remaining'], after=resumed)
    before = client.state()
    restarted = client.reset()
    check(report, 'restart_replaces_player', restarted['player']['entity'] != before['player']['entity'])
    check(report, 'restart_resets_round_and_score', restarted['session']['round'] == 1
          and restarted['session']['points'] == 500 and restarted['session']['score'] == 500
          and restarted['session']['kills'] == 0 and restarted['session']['shots'] == 0, after=restarted)
    check(report, 'restart_clears_inputs', not restarted['held_inputs'] and not restarted['queued_inputs'])
    check(report, 'session_identity_changes_only_on_restart',
          paused['session_id'] == resumed['session_id']
          and restarted['session_id'] > before['session_id'])
    client.call('origins/setup', {'clear_enemies': True, 'pause_spawning_seconds': 30})
    baseline = client.state()
    blast = {'session_id': before['session_id'], 'center': baseline['player']['position'],
             'damage': 10, 'radius': 2}
    client.call('origins/setup', {'blast': blast,
                                'round_event': {'session_id': before['session_id'], 'round': 2}})
    rejected = client.wait_ticks(3)
    check(report, 'retired_blast_and_round_event_rejected',
          rejected['equipment']['retired_blasts'] == baseline['equipment']['retired_blasts'] + 1
          and rejected['equipment']['self_impacts'] == baseline['equipment']['self_impacts']
          and rejected['equipment']['round_awards'] == baseline['equipment']['round_awards']
          and rejected['player']['health'] == baseline['player']['health'],
          pipeline_setup={'blast': blast}, before=baseline, after=rejected)
    blast['session_id'] = restarted['session_id']
    blast['center'] = rejected['player']['position']
    client.call('origins/setup', {'blast': blast,
                                'round_event': {'session_id': restarted['session_id'], 'round': 2}})
    accepted = client.wait_ticks(3)
    check(report, 'current_blast_and_round_event_accepted',
          accepted['equipment']['retired_blasts'] == rejected['equipment']['retired_blasts']
          and accepted['equipment']['self_impacts'] == rejected['equipment']['self_impacts'] + 1
          and accepted['equipment']['round_awards'] == rejected['equipment']['round_awards'] + 1
          and accepted['player']['health'] < rejected['player']['health'], after=accepted)


def run_initial(client, report):
    client.reset(difficulty='original')
    before = client.until(lambda s: s['player']['grounded'] and s['player']['action'] == 'Ready')
    fire, reload, pause = [binding(before, action) for action in ('fire', 'reload', 'pause')]
    schedule(client, [edge(1, fire, True), edge(2, fire, False)])
    client.until(lambda s: s['player']['action'] == 'Ready')
    client.call('origins/input', {'events': [edge(1, reload, True), edge(2, reload, False),
                                           edge(3, pause, True), edge(4, pause, False)]})
    paused = client.until(lambda s: s['state'] == 'Paused' and s['player']['action'].startswith('Reloading'))
    client.until(lambda s: not s['held_inputs'] and not s['queued_inputs'])
    later = client.wait_ticks(120)
    check(report, 'initial_pause_freezes_player_reload_and_spawn_timer',
          paused['player']['position'] == later['player']['position']
          and paused['player']['weapons'] == later['player']['weapons']
          and paused['player']['action'] == later['player']['action']
          and paused['session']['spawn_remaining'] == later['session']['spawn_remaining'],
          before=paused, after=later)
    schedule(client, [edge(1, pause, True), edge(2, pause, False)])
    resumed = client.until(lambda s: s['state'] == 'Playing' and s['player']['action'] == 'Ready')
    check(report, 'initial_resume_finishes_reload_in_same_session',
          resumed['session_id'] == before['session_id']
          and resumed['player']['weapons'][0]['magazine'] == before['player']['weapons'][0]['magazine']
          and resumed['player']['weapons'][0]['reserve'] == before['player']['weapons'][0]['reserve'] - 1,
          after=resumed)
    forward, lethal = [binding(resumed, action) for action in ('forward', 'lethal')]
    client.call('origins/input', {'events': [edge(1, forward, True), edge(1, fire, True),
                                           edge(1, lethal, True), edge(3, pause, True),
                                           edge(4, pause, False),
                                           edge(120, 'ArrowDown', True), edge(121, 'ArrowDown', False),
                                           edge(140, 'ArrowDown', True), edge(141, 'ArrowDown', False),
                                           edge(160, 'ArrowDown', True), edge(161, 'ArrowDown', False),
                                           edge(180, 'Enter', True), edge(181, 'Enter', False),
                                           edge(1800, forward, False),
                                           edge(1800, fire, False), edge(1800, lethal, False)]})
    held = client.until(lambda s: s['state'] == 'Paused' and bool(s['held_inputs']))
    from test_ui import key, observe, select_menu
    check(report, 'initial_paused_restart_begins_with_held_native_inputs',
          all(key in held['held_inputs'] for key in (forward, fire, lethal)), state=held)
    restarted = client.until(lambda s: s['state'] == 'Playing' and s.get('player')
                             and s['session_id'] > resumed['session_id'], timeout=120)
    check(report, 'initial_paused_menu_restart_clears_session_and_inputs',
          restarted['player']['entity'] != resumed['player']['entity']
          and restarted['session']['round'] == 1 and restarted['session']['kills'] == 0
          and restarted['session']['shots'] == 0 and restarted['session']['downs'] == 0
          and restarted['player']['trigger_client_mode'] == 0
          and not restarted['held_inputs'] and not restarted['queued_inputs']
          and restarted['equipment']['active_grenades'] == []
          and restarted['barriers']['repaired'] == 0 and restarted['barriers']['destroyed'] == 0,
          after=restarted)
    schedule(client, [edge(1, pause, True), edge(2, pause, False)])
    client.until(lambda s: s['state'] == 'Paused')
    select_menu(client, 4)
    key(client, 'Enter')
    menu = client.until(lambda s: s['state'] == 'MainMenu')
    check(report, 'initial_quit_to_menu_removes_session_objects',
          not (menu.get('player') or {}).get('entity') and not menu['enemies']
          and not menu['equipment']['active_grenades'] and not menu['zones']['contacts'], after=menu)
    observe(client, lambda h: h.get('selected_frontend_row') == 0)
    key(client, 'Enter')
    started = client.until(lambda s: s['state'] == 'Playing' and (s.get('player') or {}).get('entity')
                           and s['session_id'] > restarted['session_id'], timeout=120)
    check(report, 'initial_normal_menu_start_creates_fresh_session',
          started['session']['round'] == 1 and started['session']['points'] == 500
          and not started['held_inputs'] and not started['queued_inputs']
          and started['player']['trigger_client_mode'] == 0, after=started)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'session_pause_restart',
              'injected_events_are_pipeline_checks_not_gameplay': True}
    try:
        client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=120)
        report['schema'] = client.call('origins/schema')
        run(client, report)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        write_report(args.output, 'session.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
