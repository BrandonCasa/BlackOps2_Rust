import argparse
import hashlib
import json
import math
import re
from pathlib import Path

from gameplay_remote import Client, write_report


def binding(state, action):
    for item in state['controls']['bindings'][action]:
        if 'key' in item:
            return item['key']
        if item.get('mouse') in ('Left', 'Right', 'Middle'):
            return 'Mouse' + item['mouse']
    raise RuntimeError('Unsupported binding: ' + action)


def check(report, name, passed, **evidence):
    report['cases'].append({'case': name, 'passed': bool(passed), **evidence})
    if not passed:
        raise RuntimeError(name)


def prepare(client, equipment=None):
    client.reset()
    fixture = {'clear_enemies': True, 'pending': 6, 'pause_spawning_seconds': 300,
               'owned_perks': ['perk.quick_revive'], 'pitch': -1.5}
    if equipment:
        fixture['equipment'] = equipment
    client.call('origins/setup', fixture)
    return client.wait_ticks(12), fixture


def schedule(client, events):
    batch = client.call('origins/input', {'events': sorted(events, key=lambda e: e['tick'])})
    return client.until(lambda s: s['fixed_tick'] >= batch['finish_tick']
                        and not s['held_inputs'] and not s['queued_inputs'], timeout=45)


def edge(tick, key, pressed):
    return {'tick': tick, 'key': key, 'pressed': pressed}


def run(client, report, game):
    client.until(lambda s: s['state'] in ('Playing', 'Paused', 'GameOver') and s.get('player'), timeout=180)
    frag = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == 'weapon.frag_grenade_zm')
    revive = game['solo_revive']
    before, fixture = prepare(client)
    dt = before['fixed_delta_seconds']
    ticks = lambda seconds: math.ceil(seconds / dt)
    lethal, fire = binding(before, 'lethal'), binding(before, 'fire')
    cook_end = ticks(frag['prime_seconds'] + frag['fuse_seconds']) + 6
    events = [edge(1, lethal, True), edge(cook_end + ticks(revive['seconds']) + 120, lethal, False)]
    for tick in (cook_end + 30, cook_end + 65):
        events += [edge(tick, fire, True), edge(tick + 1, fire, False)]
    batch = client.call('origins/input', {'events': sorted(events, key=lambda e: e['tick'])})
    after = client.until(lambda s: s['fixed_tick'] >= batch['start_tick'] + cook_end + 70, timeout=45)
    interruptions = [e for e in after['equipment']['events'][len(before['equipment']['events']):]
                     if e.get('event') == 'interrupted']
    check(report, 'cooked_frag_recovery_frees_downed_pistol',
          after['player']['life'] == 'Downed'
          and after['player']['weapons'][0]['id'] == revive['downed_weapon']
          and after['session']['shots'] >= 2
          and after['session']['downs'] == 1
          and after['player']['lethal']['state'] == 'Ready'
          and after['player']['lethal']['count'] == frag['initial'] - 1
          and len(interruptions) == 1 and interruptions[0]['state'].startswith('Recovery')
          and after['equipment']['cooked_off'] == before['equipment']['cooked_off'] + 1,
          fixture=fixture, inputs=events, before=before, after=after)
    revived = client.until(lambda s: s['player']['life'] == 'Standing'
                           and s['player']['solo_charges'] == 0, timeout=30)
    revived = client.wait_ticks(6)
    check(report, 'cooked_frag_does_not_resume_after_revive',
          revived['player']['lethal']['state'] == 'Ready'
          and revived['player']['lethal']['count'] == frag['initial'] - 1
          and lethal in revived['held_inputs'],
          after=revived)
    released = client.until(lambda s: s['fixed_tick'] >= batch['finish_tick']
                            and not s['held_inputs'] and not s['queued_inputs'], timeout=30)
    check(report, 'release_after_revive_keeps_throwable_ready',
          released['player']['lethal']['state'] == 'Ready'
          and released['player']['lethal']['count'] == frag['initial'] - 1,
          after=released)

    for phase in ('Priming', 'Cooking', 'Releasing', 'TacticalPriming'):
        equipment = ['weapon.cymbal_monkey_zm'] if phase == 'TacticalPriming' else None
        before, fixture = prepare(client, equipment)
        dt = before['fixed_delta_seconds']
        prime, release, fuse = ticks(frag['prime_seconds']), ticks(frag['release_delay']), ticks(frag['fuse_seconds'])
        first_detonation = 1 + prime + 1 + fuse
        second_key = binding(before, 'tactical') if equipment else lethal
        second_start = first_detonation - (prime // 2 if phase == 'Priming' else prime + 35)
        if equipment:
            monkey = next(w['throwable'] for w in game['content']['weapons'] if w['id'] == equipment[0])
            second_start = first_detonation - ticks(monkey['prime_seconds']) // 2
        events = [edge(1, lethal, True), edge(2, lethal, False), edge(second_start, second_key, True)]
        second_release = first_detonation - max(1, release // 2) if phase == 'Releasing' else first_detonation + 110
        events.append(edge(second_release, second_key, False))
        for tick in (first_detonation + 30, first_detonation + 65):
            events += [edge(tick, fire, True), edge(tick + 1, fire, False)]
        after = schedule(client, events)
        launches = [e for e in after['equipment']['events'][len(before['equipment']['events']):]
                    if e.get('event') == 'launch' and e.get('dropped')]
        interruptions = [e for e in after['equipment']['events'][len(before['equipment']['events']):]
                         if e.get('event') == 'interrupted']
        expected_phase = 'Priming' if equipment else phase
        source_fuse = None
        if len(interruptions) == 1:
            interrupted = interruptions[0]['state']
            if expected_phase == 'Priming':
                source_fuse = monkey['fuse_seconds'] if equipment else frag['fuse_seconds']
            else:
                field = 'remaining' if expected_phase == 'Cooking' else 'fuse'
                value = re.search(field + r': ([^ ,}]+)', interrupted)
                if value:
                    source_fuse = float(value[1])
        inventory = after['player']['tactical'] if equipment else after['player']['lethal']
        expected_count = monkey['initial'] - 1 if equipment else frag['initial'] - 2
        check(report, phase.lower() + '_drops_consumed_throwable_and_frees_pistol',
              after['player']['life'] == 'Downed' and after['session']['downs'] == 1
              and after['session']['shots'] >= 2 and inventory['state'] == 'Ready'
              and inventory['count'] == expected_count and len(launches) == 1
              and len(interruptions) == 1 and interruptions[0]['state'].startswith(expected_phase)
              and source_fuse is not None and math.isclose(launches[0]['fuse'], source_fuse, rel_tol=1e-6, abs_tol=1e-6)
              and launches[0]['fuse'] > 0 and launches[0]['velocity'] == [0.0, 0.0, 0.0]
              and after['equipment']['thrown'] == before['equipment']['thrown'] + 1
              and after['equipment']['cooked_off'] == before['equipment']['cooked_off'],
              fixture=fixture, intended_interrupted_phase=phase, inputs=events,
              expected_first_detonation_tick=first_detonation, before=before, after=after)
        revived = client.until(lambda s: s['player']['life'] == 'Standing'
                               and s['player']['solo_charges'] == 0, timeout=30)
        settled = client.wait_ticks(ticks(0.75))
        inventory = settled['player']['tactical'] if equipment else settled['player']['lethal']
        check(report, phase.lower() + '_does_not_resume_after_revive',
              inventory['state'] == 'Ready' and inventory['count'] == expected_count
              and not settled['equipment']['active_grenades']
              and settled['equipment']['detonations'] == before['equipment']['detonations'] + 2,
              revived=revived, after=settled)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15702')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--identity', type=Path, required=True)
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'scenario': 'throwable_down_interruptions',
              'limitations': ['Quick Revive ownership and equipment are scenario setup; acquisition is not covered.',
                              'Downing, throwing, cooking, firing and revival use the real gameplay paths.',
                              'Interrupted phase timing derives from runtime fixed delta and native content.']}
    try:
        report['identity'] = {'path': str(args.identity.resolve()), 'sha256': hashlib.sha256(args.identity.read_bytes()).hexdigest()}
        report['script_sha256'] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        report['schema'] = client.call('origins/schema')
        game_path = Path(__file__).resolve().parent.parent / 'assets/origins/game.json'
        report['content_sha256'] = hashlib.sha256(game_path.read_bytes()).hexdigest()
        run(client, report, json.loads(game_path.read_bytes()))
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
        path = write_report(args.output, 'throwable-down.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'report': str(path), 'error': report.get('error')}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
