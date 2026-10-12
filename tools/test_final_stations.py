import json
import math
from pathlib import Path
import re
import time

from test_environment import aim, tap
from test_throwable_down import check


def fixture(client, generators, box=None):
    client.call('origins/stations/setup', {'active_generators': generators, 'normal_box': box})


def station(client):
    return client.call('origins/stations')


def wait_station(client, predicate, timeout=20):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        state = station(client)
        if predicate(state):
            return state
        time.sleep(.05)
    raise TimeoutError('Station condition timed out: ' + str(state))


def approach(client, point, prompt):
    height = client.state()['player']['height']
    attempts = []
    for radius in (1.2, 1.7, .8):
        for index in range(8):
            angle = index * math.tau / 8
            client.call('origins/setup', {'position': [point[0] + math.sin(angle) * radius,
                        point[1] + height / 2 + .08, point[2] + math.cos(angle) * radius]})
            client.wait_ticks(3)
            aim(client, [point[0], point[1] + .8, point[2]])
            state = client.wait_ticks(3)
            attempts.append(state['focus'])
            if prompt in state['focus']['prompt']:
                return state
    raise RuntimeError('Could not focus station ' + prompt + ': ' + str(attempts[-4:]))


def run(client, report, game, research, inspection):
    source = Path(research) / 'source/server-tomb/maps/mp/zm_tomb_capture_zones.gsc'
    entities_path = Path(inspection) / 'solo-content/doors-recovery/entities.json'
    bindings = dict((box, zone) for zone, box in re.findall(
        r'register_mystery_box_for_zone\( "([^"]+)", "([^"]+)" \)', source.read_text()))
    entities = json.loads(entities_path.read_bytes())
    native = []
    for entity in entities:
        name = entity.get('script_noteworthy')
        if entity.get('targetname') != 'treasure_chest_use' or name not in bindings:
            continue
        x, y, z = map(float, entity['origin'].split())
        generator = next(g['number'] for g in game['content']['generators']
                         if g['id'] == 'generator.' + bindings[name])
        native.append(([x * .0254, z * .0254, -y * .0254], generator))
    snapshot = station(client)
    expected = [next(number for point, number in native if math.dist(point, actual) < .0001)
                for actual in snapshot['box_positions']]
    check(report, 'all_box_positions_preserve_source_generator_binding', len(expected) == 6 and snapshot['box_generators'] == expected, expected=expected, observed=snapshot['box_generators'])
    client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'pending': 6,
                'pause_spawning_seconds': 300, 'points': 20000, 'weapons': ['weapon.mp40_zm'],
                'owned_perks': [], 'open_connections': [d['connection'] for d in game['access']['doors']]})
    fixture(client, [], 0)
    box = station(client)['box_positions'][0]
    before = approach(client, box, 'Mystery Box')
    tap(client, 'interact')
    after = client.wait_ticks(3)
    check(report, 'unpowered_mystery_box_rejects_normal_purchase',
          after['focus']['rejection'] == 'NoPower' and after['session']['points'] == before['session']['points']
          and station(client)['purchases'] == 0, before=before, after=after)
    fixture(client, [1])
    before = approach(client, box, 'Mystery Box')
    tap(client, 'interact')
    after = client.wait_ticks(3)
    check(report, 'unrelated_generator_does_not_power_box', after['focus']['rejection'] == 'NoPower'
          and after['session']['points'] == before['session']['points'], after=after)
    fixture(client, [3])
    approach(client, box, 'Mystery Box')
    client.until(lambda s: s['player']['action'] == 'Ready')
    before = client.state()
    tap(client, 'interact')
    wait_station(client, lambda s: s['boxes'][0]['phase'] == 'Rolling')
    check(report, 'local_generator_enables_950_point_box_purchase',
          client.state()['session']['points'] == before['session']['points'] - 950)
    fixture(client, [])
    reward = wait_station(client, lambda s: s['boxes'][0]['phase'] == 'Reward')['boxes'][0]['weapon']
    approach(client, box, 'take')
    tap(client, 'interact')
    state = client.until(lambda s: any(w['id'] == reward for w in s['player']['weapons'])
                         or (s['player'].get('tactical') or {}).get('id') == reward)
    check(report, 'power_loss_preserves_box_reward_collection', station(client)['collections'] == 1,
          reward=reward, after=state)

    client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'pending': 6,
                'pause_spawning_seconds': 300, 'points': 20000, 'weapons': ['weapon.mp40_zm'],
                'owned_perks': [], 'open_connections': [d['connection'] for d in game['access']['doors']]})
    fixture(client, [1, 2, 3, 4, 5, 6])
    point = station(client)['upgrade_position']
    before = approach(client, point, 'Pack-a-Punch')
    client.until(lambda s: s['player']['action'] == 'Ready')
    tap(client, 'interact')
    processing = wait_station(client, lambda s: s['upgrade']['phase'] == 'Processing')
    upgraded = processing['upgrade']['weapon']
    fixture(client, [1, 2, 3, 4, 5])
    ready = wait_station(client, lambda s: s['upgrade']['phase'] == 'Ready')
    check(report, 'power_loss_does_not_destroy_deposited_pack_a_punch_weapon',
          client.state()['session']['points'] == before['session']['points'] - 5000,
          processing=processing, ready=ready)
    approach(client, point, 'collect')
    client.until(lambda s: s['player']['action'] == 'Ready')
    tap(client, 'interact')
    after = client.until(lambda s: any(w['id'] == upgraded for w in s['player']['weapons']))
    check(report, 'normal_input_collects_upgrade_after_generator_loss', station(client)['collections'] == 1,
          upgraded=upgraded, after=after)
    before = approach(client, point, 'Pack-a-Punch')
    tap(client, 'interact')
    after = client.wait_ticks(3)
    check(report, 'generator_loss_blocks_next_pack_a_punch_purchase',
          after['focus']['rejection'] == 'NoPower' and after['session']['points'] == before['session']['points'])
