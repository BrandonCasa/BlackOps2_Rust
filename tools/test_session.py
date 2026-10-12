import argparse
import json
from pathlib import Path

from test_support import Client, write_report


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
