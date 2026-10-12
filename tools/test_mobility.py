import argparse
import json
import math
from pathlib import Path

from test_support import Client, write_report
from test_support import source_mobility as native_tuning
from test_throwable_down import binding, check, edge, schedule


def prepare(client):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'weapons': ['weapon.mp40_zm'], 'owned_perks': [], 'pitch': 0}
    client.call('origins/setup', fixture)
    state = client.until(lambda s: s['player']['action'] == 'Ready'
                         and not s['player']['mobility']['airborne'])
    return state, fixture


def new_events(before, after, name):
    return [event for event in after['movement']['events'][len(before['movement']['events']):]
            if event['event'] == name]


def blocked_case(client, report, native, game):
    attempts = []
    for yaw in (0, math.pi / 2, math.pi, -math.pi / 2):
        before, fixture = prepare(client)
        fixture['yaw'] = yaw
        fixture['open_connections'] = [door['connection'] for door in game['access']['doors']]
        client.call('origins/setup', {'yaw': yaw, 'open_connections': fixture['open_connections']})
        forward, sprint, prone = [binding(before, key) for key in ('forward', 'sprint', 'prone')]
        press = math.ceil(2.5 / before['fixed_delta_seconds'])
        inputs = [edge(1, forward, True), edge(1, sprint, True), edge(press, prone, True),
                  edge(press + 1, prone, False), edge(press + 2, forward, False),
                  edge(press + 2, sprint, False)]
        batch = client.call('origins/input', {'events': inputs})
        client.wait_ticks(math.ceil(1.5 / before['fixed_delta_seconds']))
        stopped = client.state()
        velocity = stopped['player']['velocity']
        actual_speed = math.hypot(velocity[0], velocity[2])
        attempts.append({'yaw': yaw, 'speed': actual_speed, 'position': stopped['player']['position']})
        if (actual_speed <= native['dive_minimum_speed']
                and stopped['player']['sprint']['sprinting']
                and not stopped['player']['mobility']['airborne']):
            after = client.until(lambda s: s['fixed_tick'] >= batch['finish_tick']
                                 and not s['held_inputs'] and not s['queued_inputs'], timeout=30)
            entry = next(row for row in after['movement']['physics']
                         if row['seconds'] > stopped['movement']['physics'][-1]['seconds']
                         and row['prone'])
            check(report, 'solid_collision_stops_actual_velocity_and_rejects_dive',
                  after['player']['prone'] and not after['player']['mobility']['diving']
                  and not new_events(stopped, after, 'dive_started')
                  and math.hypot(*entry['velocity_before'][::2]) <= native['dive_minimum_speed'],
                  fixture=fixture, inputs=inputs, before=stopped, after=after,
                  prone_entry=entry, attempts=attempts)
            return
        client.release()
    check(report, 'find_authored_wall_for_minimum_speed_case', False, attempts=attempts)


def run(client, report, native, game):
    client.until(lambda s: s['state'] == 'LoadError' or s['state'] in ('Playing', 'Paused', 'GameOver')
                 and s.get('player'), timeout=240)
    report['protocol'] = client.call('rpc.discover')
    report['schema'] = client.call('origins/schema')
    before, fixture = prepare(client)
    check(report, 'scenario_uses_hold_sprint_controls', before['controls']['sprint_mode'] == 'hold',
          controls=before['controls'])
    dt = before['fixed_delta_seconds']
    ticks = lambda seconds: math.ceil(seconds / dt)
    forward, sprint, prone = [binding(before, key) for key in ('forward', 'sprint', 'prone')]
    early = max(3, ticks(native['dive_startup'] / 2))
    inputs = [edge(1, forward, True), edge(1, sprint, True), edge(early, prone, True),
              edge(early + 1, prone, False), edge(early + 4, forward, False),
              edge(early + 4, sprint, False)]
    after = schedule(client, inputs)
    check(report, 'short_sprint_enters_prone_without_diving',
          after['player']['prone'] and not after['player']['mobility']['diving']
          and not new_events(before, after, 'dive_started'),
          fixture=fixture, inputs=inputs, after=after)

    before, fixture = prepare(client)
    start = ticks(native['dive_startup']) + 8
    inputs = [edge(1, forward, True), edge(1, sprint, True), edge(start, forward, False),
              edge(start, sprint, False), edge(start, prone, True), edge(start + 1, prone, False)]
    after = schedule(client, inputs)
    check(report, 'released_sprint_does_not_launch_from_previous_tick_flag',
          after['player']['prone'] and not new_events(before, after, 'dive_started'),
          fixture=fixture, inputs=inputs, after=after)

    before, fixture = prepare(client)
    start = ticks(native['dive_startup']) + 8
    fire = binding(before, 'fire')
    end = start + ticks(2)
    inputs = [edge(1, forward, True), edge(1, sprint, True), edge(start, prone, True),
              edge(start + 1, prone, False), edge(start + 2, fire, True),
              edge(start + 3, forward, False), edge(start + 3, sprint, False),
              edge(end, fire, False)]
    after = schedule(client, inputs)
    starts = new_events(before, after, 'dive_started')
    lands = [event for event in new_events(before, after, 'landed') if event['diving']]
    finishes = new_events(before, after, 'dive_finished')
    check(report, 'eligible_sprint_completes_one_dive_and_slide',
          len(starts) == len(lands) == len(finishes) == 1
          and starts[0]['sprint_seconds'] > native['dive_startup']
          and starts[0]['speed'] > native['dive_minimum_speed']
          and 0 < finishes[0]['seconds'] - lands[0]['seconds'] <= native['dive_slide'] + 2 * dt
          and after['player']['prone'] and not after['player']['mobility']['busy'],
          fixture=fixture, inputs=inputs, starts=starts, lands=lands, finishes=finishes, after=after)
    shots = after['firing']['shots'][len(before['firing']['shots']):]
    check(report, 'held_fire_waits_for_slide_and_native_post_move_pause',
          len(shots) > 0 and all(shot['seconds'] >= finishes[0]['seconds']
                               + native['dive_recovery'] - 2 * dt for shot in shots),
          shots=shots, finishes=finishes)

    before, fixture = prepare(client)
    repeat_home = {'position': before['player']['position'], 'yaw': before['player']['yaw']}
    inputs = [edge(1, forward, True), edge(1, sprint, True), edge(start, prone, True),
              edge(start + 1, prone, False), edge(start + 2, forward, False),
              edge(start + 2, sprint, False)]
    schedule(client, inputs)
    first = client.until(lambda s: len(new_events(before, s, 'dive_finished')) == 1
                         and not s['player']['mobility']['busy'])
    repeat_home['position'] = repeat_home['position'].copy()
    repeat_home['position'][1] -= (before['player']['height'] - first['player']['height']) * .5
    client.call('origins/setup', repeat_home)
    client.wait_ticks(2)
    first = client.until(lambda s: s['player']['grounded'] and not s['player']['mobility']['airborne'])
    end = ticks(native['dive_startup']) + 10
    inputs = [edge(1, prone, True), edge(2, prone, False), edge(3, forward, True),
              edge(3, sprint, True), edge(end, prone, True), edge(end + 1, prone, False),
              edge(end + 2, forward, False), edge(end + 2, sprint, False)]
    after = schedule(client, inputs)
    sprint_samples = [sample for sample in after['movement']['physics'][len(first['movement']['physics']):]
                      if sample['sprinting'] and not sample['prone']]
    check(report, 'native_exhaustion_window_rejects_second_dive',
          after['player']['prone'] and not after['player']['mobility']['diving']
          and after['player']['mobility']['since_dive'] < native['dive_exhaustion']
          and not new_events(first, after, 'dive_started')
          and len(sprint_samples) > ticks(native['dive_startup'])
          and math.hypot(*sprint_samples[-1]['velocity_before'][::2]) > native['dive_minimum_speed'],
          fixture=repeat_home, inputs=inputs, before=first, after=after, sprint_samples=sprint_samples)
    client.until(lambda s: s['player']['mobility']['since_dive'] > native['dive_exhaustion'] + dt)
    client.call('origins/setup', repeat_home)
    client.wait_ticks(2)
    client.until(lambda s: s['player']['grounded'] and not s['player']['mobility']['airborne'])
    before = client.state()
    after = schedule(client, inputs)
    check(report, 'dive_returns_after_native_exhaustion_window',
          len(new_events(before, after, 'dive_started')) == 1
          and before['player']['mobility']['since_dive'] > native['dive_exhaustion'],
          fixture=repeat_home, inputs=inputs, before=before, after=after)
    reset = client.reset()
    check(report, 'fresh_session_clears_dive_and_cooldown',
          not reset['player']['mobility']['busy'] and reset['player']['mobility']['since_dive'] is None
          and not reset['held_inputs'] and not reset['queued_inputs'], after=reset)
    blocked_case(client, report, native, game)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--assets', type=Path, required=True)
    parser.add_argument('--research', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url, shutdown_timeout=45)
    report = {'passed': False, 'cases': [], 'scenario': 'native_dive_gates',
              'limitations': ['Weapon grants and enemy/spawn controls are scenario setup.',
                              'Repeat attempts restore the starting surface while retaining the mobility cooldown.',
                              'Movement, prone, sprint and firing use scheduled normal input.',
                              'This scenario checks entry and recovery gates; trajectory and damage use separate scenarios.']}
    try:
        source = args.research / 'exports/functions/008a/008ad170.c'
        report['native_source'] = {'path': str(source.resolve())}
        native = native_tuning(args.research)
        game = json.loads((Path(args.assets.resolve()) / 'game.json').read_bytes())
        check(report, 'pack_preserves_native_dive_defaults',
              all(math.isclose(game['tuning']['mobility'][key], value, abs_tol=0.0000001)
                  for key, value in native.items()), native=native, pack=game['tuning']['mobility'])
        run(client, report, native, game)
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
        path = write_report(args.output, 'mobility.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']),
                      'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
