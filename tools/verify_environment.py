import argparse
import hashlib
import json
import math
from pathlib import Path

from gameplay_remote import Client, write_report
from verify_throwable_down import binding, edge, schedule


def rng(seed, count=16):
    values = []
    for _ in range(count):
        seed = (seed*6364136223846793005+1) & ((1 << 64)-1)
        values.append((seed >> 32) & 0xffffffff)
    return values


def seed_for(predicate):
    return next(seed for seed in range(1000000) if predicate([v % 100 for v in rng(seed)]))


def seed_raw(predicate):
    return next(seed for seed in range(1000000) if predicate(rng(seed)))


def env(client, **fixture):
    return client.call('origins/environment/setup', fixture)


def target(state, kind, value=None):
    return next((t for t in state['environment']['targets'] if t['target']['kind'] == kind
                 and (value is None or t['target']['value'] == value)), None)


def tap(client, action):
    key = binding(client.state(), action)
    return schedule(client, [edge(1, key, True), edge(3, key, False)])


def aim(client, point):
    state = client.state()
    body = state['player']
    eye = list(body['position'])
    eye[1] += body['eye_height']-body['height']/2
    delta = [b-a for a, b in zip(eye, point)]
    client.call('origins/setup', {'yaw': math.atan2(-delta[0], -delta[2]),
                                 'pitch': math.atan2(delta[1], math.hypot(delta[0], delta[2]))})


def approach(client, kind, value=None):
    state = client.until(lambda s: target(s, kind, value) is not None and target(s, kind, value)['rising'] is None, timeout=15)
    item = target(state, kind, value)
    point = list(item['position'])
    point[1] += .8128 if kind == 'shovel' else .508 if kind in ('mound', 'hidden') else 0
    for dx, dz in ((0, 1.6), (1.6, 0), (0, -1.6), (-1.6, 0), (0, .9)):
        client.call('origins/setup', {'position': [point[0]+dx, item['position'][1]+.91, point[2]+dz]})
        client.wait_ticks(3)
        aim(client, point)
        state = client.wait_ticks(3)
        if state['focus']['target'] == item['entity'] and state['player']['life'] == 'Standing':
            return state, item
    raise RuntimeError(f'No usable authored {kind} {value}: {client.state()["focus"]}')


def fixture(client, game, **options):
    client.reset()
    client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300,
                                 'weapons': ['weapon.c96_zm'], 'open_connections': [d['connection'] for d in game['access']['doors']],
                                 'owned_perks': ['perk.phd_flopper']})
    env(client, **options)
    return client.wait_ticks(5)


def state_digger(state):
    return state['environment']['digger']['state']


def dig(client, index, seed=None):
    before, item = approach(client, 'mound', index)
    if seed is not None:
        env(client, seed=seed)
    tap(client, 'interact')
    after = client.until(lambda s: index not in s['environment']['digging']['active'], timeout=10)
    return before, after, after['environment']['digging']['events'][-1]


def ending(client, game, round_number, kind=None, seed=None):
    state = client.state()
    origin = list(game['spawn'])
    client.call('origins/setup', {'position': origin, 'yaw': 0, 'pitch': 0, 'clear_enemies': True,
                                 'pending': 0, 'pause_spawning_seconds': 300,
                                 'enemies': [{'position': [origin[0]+1.2, origin[1], origin[2]], 'health': 1}]})
    setup = {'round': round_number}
    if kind is not None:
        setup['overrides'] = {str(round_number): kind}
    env(client, **setup)
    ready = client.until(lambda s: len(s['enemies']) == 1 and s['enemies'][0]['ready'] and s['enemies'][0]['hit_regions'], timeout=30)
    chest = next(h for h in ready['enemies'][0]['hit_regions'] if h['id'] == 'chest')
    aim(client, [(a+b)/2 for a, b in zip(chest['start'], chest['end'])])
    before = client.state()
    if seed is not None:
        env(client, seed=seed)
    tap(client, 'melee')
    after = client.until(lambda s: s['session']['intermission'] and len(s['environment']['weather']['transitions']) > len(before['environment']['weather']['transitions']), timeout=15)
    return before, after


def run(client, game, record, selection, output):
    def enabled(name):
        return selection is None or name in selection

    if enabled('shovel'):
        before = fixture(client, game)
        before, item = approach(client, 'shovel')
        tap(client, 'interact')
        after = client.until(lambda s: state_digger(s)['shovel'])
        record('shovel_normal_use', target(after, 'shovel', item['target']['value']) is None, before=before, after=after)
        index = min(after['environment']['digging']['active'])
        before, after, result = dig(client, index)
        record('first_dig_good_and_depleted', result['outcome'] not in ('grenade', 'zombie') and state_digger(after)['qualifying_digs'] == 1, result=result, before=before, after=after)

    if enabled('no_shovel'):
        before = fixture(client, game)
        index = min(before['environment']['digging']['active'])
        before, _ = approach(client, 'mound', index)
        tap(client, 'interact')
        after = client.state()
        record('no_shovel_cannot_dig', index in after['environment']['digging']['active'] and state_digger(after)['qualifying_digs'] == 0, before=before, after=after)

    if enabled('weather'):
        initial = fixture(client, game)
        overrides = initial['environment']['weather']['overrides']
        record('source_early_weather_schedule', sum(overrides.get(str(r)) == 'snow' for r in (3, 4)) == 1
               and overrides.get('10') == 'snow' and all(overrides[str(r)] in ('clear', 'rain') for r in range(5, 10)), after=initial)
        for kind, expected in [('clear', 3), ('rain', 5), ('snow', 3)]:
            fixture(client, game, active_mounds=[])
            before, after = ending(client, game, 3, kind)
            record('round_weather_timing_'+kind, 1.8 < after['environment']['weather']['delay'] <= 2
                   and (after['environment']['weather']['intensity'] == 0 if kind == 'clear' else 1 <= after['environment']['weather']['intensity'] <= 4), after=after)
            after = client.until(lambda s: s['environment']['weather']['presented'] == kind and len(s['environment']['digging']['active']) == expected, timeout=20)
            transitions = len(after['environment']['weather']['transitions'])
            after = client.wait_ticks(80)
            record('round_ending_'+kind, after['session']['round'] == 3 and transitions == 1
                   and len(after['environment']['weather']['transitions']) == 1
                   and len(after['environment']['digging']['active']) == expected,
                   before=before, after=after)
            if kind == 'rain':
                after = client.until(lambda s: s['environment']['weather']['strikes'] > 0 and s['environment']['weather']['precipitation_bursts'] > 0)
                record('rain_lightning_and_precipitation', True, after=after)
                flash = client.until(lambda s: s['environment']['weather']['flash'] > 0)
                weather = flash['environment']['weather']
                record('source_lightning_flash_ranges', .1 <= weather['flash_duration'] < .35 and 28/30 <= weather['flash_peak'] < 32/30, after=flash)
            if kind == 'snow':
                record('snow_parts_region_replenishment', all(sum(region in game['environment']['digging']['mounds'][i]['zone'] for i in after['environment']['digging']['active']) == 1 for region in ('bunker', 'nml', 'village')) and after['environment']['weather']['strikes'] == 0, after=after)
            next_round = client.until(lambda s: s['session']['round'] == 4, timeout=25)
            record('real_next_round_'+kind, next_round['round_evidence']['transitions'] > before['round_evidence']['transitions'], after=next_round)
        reset = client.reset()
        record('session_weather_dig_parts_reset', reset['environment']['weather']['kind'] == 'clear'
               and reset['environment']['weather']['last_snow'] == 0 and reset['environment']['weather']['last_rain'] == 0
               and reset['environment']['weather']['strikes'] == 0 and not state_digger(reset)['shovel']
               and reset['environment']['parts']['pieces'] == {}, after=reset)
        for count in (15, 16):
            fixture(client, game, active_mounds=list(range(count)))
            before, after = ending(client, game, 1, 'clear')
            after = client.until(lambda s: any(e.get('event') == 'replenishment' for e in s['environment']['digging']['events']))
            after = client.wait_ticks(20)
            record('source_mound_cap_'+str(count), len(after['environment']['digging']['active']) == 16, before=before, after=after)
        state = fixture(client, game)
        forced = next(r for r in (3, 4) if state['environment']['weather']['overrides'].get(str(r)) == 'snow')
        before, after = ending(client, game, forced)
        record('natural_forced_early_snow_round', after['environment']['weather']['kind'] == 'snow' and after['environment']['weather']['last_snow'] == forced, before=before, after=after)

    if enabled('progression'):
        fixture(client, game, digger={'shovel': True, 'qualifying_digs': 28}, weather='clear')
        indices = sorted(client.state()['environment']['digging']['active'])[:3]
        seed = seed_for(lambda r: r[1] <= 50 and r[2] % 2 == 1)
        before, after, result = dig(client, indices[0], seed)
        record('dig_29_does_not_upgrade', state_digger(after)['qualifying_digs'] == 29 and not state_digger(after)['golden'], before=before, after=after)
        before, after, result = dig(client, indices[1], seed_for(lambda r: r[1] >= 95))
        record('dig_30_upgrades_after_reward_without_helmet_roll', state_digger(after)['qualifying_digs'] == 30 and state_digger(after)['golden'] and not state_digger(after)['helmet'] and result['outcome'] != 'golden_helmet', before=before, after=after)
        before, after, result = dig(client, indices[2], seed_for(lambda r: r[1] >= 95))
        record('eligible_5_percent_helmet_roll', state_digger(after)['helmet'] and result['outcome'] == 'golden_helmet' and state_digger(after)['qualifying_digs'] == 30, before=before, after=after)
        fixture(client, game, weather='clear', digger={'shovel': True, 'golden': True, 'qualifying_digs': 30})
        index = min(client.state()['environment']['digging']['active'])
        before, after, result = dig(client, index, seed_for(lambda r: r[1] == 94))
        record('helmet_roll_94_is_ineligible', not state_digger(after)['helmet'] and result['outcome'] != 'golden_helmet', before=before, after=after)

    if enabled('history'):
        for snow, rain, expected in [(0, 0, 'snow'), (7, 0, 'rain'), (7, 7, 'clear')]:
            fixture(client, game, overrides={}, last_snow=snow, last_rain=rain)
            before, after = ending(client, game, 7, seed=seed_for(lambda r: r[0] > 2 and r[1] >= 80))
            record('history_'+expected, after['environment']['weather']['kind'] == expected, before=before, after=after)

    if enabled('audio'):
        fixture(client, game, weather='rain')
        indoor = client.until(lambda s: s['environment']['weather']['ambient_alias'] == 'amb_weather_2d_rain_indoor'
                              and any(a['alias'] == 'amb_weather_2d_rain_indoor' and a['sink'] for a in s['environment']['audio_active']), timeout=30)
        record('rain_indoor_audio_sink', True, after=indoor)
        spot = game['environment']['digging']['mounds'][0]['position']
        client.call('origins/setup', {'position': [spot[0], spot[1]+.9, spot[2]]})
        outdoor = client.until(lambda s: s['environment']['weather']['ambient_alias'] == 'amb_weather_2d_rain_outdoor'
                               and any(a['alias'] == 'amb_weather_2d_rain_outdoor' and a['sink'] for a in s['environment']['audio_active'])
                               and not any(a['alias'] == 'amb_weather_2d_rain_indoor' for a in s['environment']['audio_active']))
        record('rain_outdoor_replaces_indoor_audio', not any(a['alias'] == 'amb_weather_2d_rain_indoor' for a in outdoor['environment']['audio_active']), before=indoor, after=outdoor)
        env(client, weather='snow')
        snow = client.until(lambda s: s['environment']['weather']['ambient_alias'] == 'amb_weather_2d_snow_outdoor'
                            and any(a['alias'] == 'amb_weather_2d_snow_outdoor' and a['sink'] for a in s['environment']['audio_active']))
        strikes = snow['environment']['weather']['strikes']
        snow = client.wait_ticks(180)
        record('snow_audio_and_no_new_thunder', snow['environment']['weather']['strikes'] == strikes and not any(a['alias'].startswith('amb_weather_2d_rain') for a in snow['environment']['audio_active']), after=snow)
        client.reset()
        clear = client.until(lambda s: any(a['alias'].startswith('amb_weather_2d_default') and a['sink'] for a in s['environment']['audio_active']))
        record('reset_stops_weather_audio', not any(a['alias'].startswith(('amb_weather_2d_snow', 'amb_weather_2d_rain')) for a in clear['environment']['audio_active']), after=clear)

    if enabled('ice'):
        fixture(client, game, weather='clear', digger={'shovel': True})
        indices = sorted(client.state()['environment']['digging']['active'])
        for i in indices[:2]:
            before, after, result = dig(client, i, seed_for(lambda r: r[1] <= 50))
        record('ice_misses_accumulate_without_snow', not after['environment']['parts']['pieces'] and after['environment']['digging']['misses']['part.elemental_staff_water.lower_staff'] >= 2, after=after)
        env(client, weather='snow', digger={'shovel': True, 'golden': True, 'qualifying_digs': 30})
        before, after, result = dig(client, indices[2], seed_for(lambda r: r[0] > 50 and r[1] >= 95))
        part = 'part.elemental_staff_water.lower_staff'
        record('ice_part_precedes_helmet_roll', result['outcome'] == part and not state_digger(after)['helmet'], before=before, after=after)
        reveal = target(after, 'part', part)
        record('ice_part_delays_reveal', reveal is not None and not reveal['visible'] and .4 < reveal['rising'] <= .55, after=after)
        before, item = approach(client, 'part', part)
        duplicate = env(client, parts=[{'id': part, 'position': item['position']}])
        record('available_part_cannot_be_revealed_twice', duplicate['revealed_parts'] == [None], after=client.state(), duplicate=duplicate)
        tap(client, 'interact')
        after = client.until(lambda s: s['environment']['parts']['pieces'][part]['state'] == 'owned')
        duplicate = env(client, parts=[{'id': part, 'position': item['position']}])
        record('shared_part_pickup_and_duplicate_prevention', duplicate['revealed_parts'] == [None] and target(client.state(), 'part', part) is None, before=before, after=after, duplicate=duplicate)

    if enabled('loot'):
        predicates = {
            'grenade': lambda r: r[1] > 50 and r[2] % 2 == 0,
            'zombie': lambda r: r[1] > 50 and r[2] % 2 == 1,
            'weapon': lambda r: r[1] <= 50 and r[2] % 2 == 1,
            'bonus_points_player': lambda r: r[1] <= 50 and r[2] % 2 == 0 and r[3] < 80 and r[4] <= 70,
            'zombie_blood': lambda r: r[1] <= 50 and r[2] % 2 == 0 and r[3] < 80 and r[4] > 70,
        }
        for outcome, predicate in predicates.items():
            fixture(client, game, weather='clear', digger={'shovel': True, 'qualifying_digs': 1})
            index = min(client.state()['environment']['digging']['active'])
            before, after, result = dig(client, index, seed_for(predicate))
            record('loot_'+outcome, result['outcome'].startswith('weapon.') if outcome == 'weapon' else result['outcome'] == outcome, before=before, after=after, result=result)
            if outcome == 'weapon':
                before, item = approach(client, 'weapon', result['outcome'])
                tap(client, 'interact')
                after = client.until(lambda s: any(e.get('event') == 'weapon_collected' for e in s['environment']['digging']['events']))
                record('weapon_normal_pickup', any(w['id'] == result['outcome'] for w in after['player']['weapons']), before=before, after=after)
            if outcome in ('bonus_points_player', 'zombie_blood'):
                pickup = client.until(lambda s: any(p['id'] == 'powerup.'+outcome for p in s['drops']['active']))
                active = next(p for p in pickup['drops']['active'] if p['id'] == 'powerup.'+outcome)
                client.call('origins/setup', {'position': [active['position'][0], active['position'][1]+.9, active['position'][2]+2.2]})
                aim(client, active['position'])
                before = client.wait_ticks(8)
                key = binding(before, 'forward')
                after = schedule(client, [edge(1, key, True), edge(25, key, False)])
                after = client.until(lambda s: s['drops']['collected'] > before['drops']['collected'])
                record('normal_movement_collects_'+outcome, after['session']['points'] > before['session']['points'] if outcome == 'bonus_points_player' else after['effects']['zombie_blood'] > 0, before=before, after=after)
                if outcome == 'bonus_points_player':
                    record('blood_money_does_not_announce_nuke', after['audio']['requests'].get('vox_zmba_powerup_nuke_0', 0) == before['audio']['requests'].get('vox_zmba_powerup_nuke_0', 0), before=before, after=after)
            if outcome == 'grenade':
                record('grenade_real_projectile', any(2.7 < g['remaining'] <= 3 for g in after['equipment']['active_grenades']), after=after)
                detonated = client.until(lambda s: s['equipment']['detonations'] > before['equipment']['detonations'])
                record('dug_grenade_detonates_in_real_game', True, before=before, after=detonated)
            if outcome == 'zombie':
                after = client.until(lambda s: any(e['ready'] and e['hit_regions'] for e in s['enemies']))
                record('dug_zombie_real_actor', True, after=after)

    if enabled('expiration'):
        fixture(client, game, weather='clear', digger={'shovel': True, 'qualifying_digs': 1})
        index = min(client.state()['environment']['digging']['active'])
        before, after, result = dig(client, index, seed_for(lambda r: r[1] <= 50 and r[2] % 2 == 1))
        expired = client.until(lambda s: s['environment']['digging']['expired_weapons'] == 1, timeout=30)
        record('weapon_expires_after_12_seconds', target(expired, 'weapon', result['outcome']) is None, before=after, after=expired)

    if enabled('ammo'):
        fixture(client, game, weather='clear', digger={'shovel': True, 'qualifying_digs': 1})
        index = min(client.state()['environment']['digging']['active'])
        approach(client, 'mound', index)
        tap(client, 'fire')
        client.until(lambda s: s['player']['action'] == 'Ready')
        tap(client, 'reload')
        weapon = next(w for w in game['content']['weapons'] if w['id'] == 'weapon.c96_zm')
        before = client.until(lambda s: s['player']['action'] == 'Ready' and s['player']['weapons'][0]['reserve'] < weapon['mechanics']['starting_reserve'])
        seed = seed_raw(lambda r: r[1] % 100 <= 50 and r[2] % 2 == 1 and r[3] % 100 < 90 and r[4] % 3 == 1)
        _, after, result = dig(client, index, seed)
        record('duplicate_weapon_ammunition_roll', result['outcome'] == 'weapon.c96_zm', after=after)
        approach(client, 'weapon', 'weapon.c96_zm')
        tap(client, 'interact')
        after = client.until(lambda s: target(s, 'weapon', 'weapon.c96_zm') is None)
        record('duplicate_weapon_refills_reserve', len(after['player']['weapons']) == 1 and after['player']['weapons'][0]['reserve'] == weapon['reserve'], before=before, after=after)

    if enabled('money'):
        fixture(client, game, weather='clear', digger={'shovel': True, 'qualifying_digs': 1})
        state = client.state()
        p = state['player']['position'][:]
        p[1] -= state['player']['height']/2
        client.call('origins/setup', {'pickups': [{'id': 'powerup.double_points', 'position': p}]})
        client.until(lambda s: s['effects']['double_points'] > 0)
        index = min(client.state()['environment']['digging']['active'])
        _, after, result = dig(client, index, seed_for(lambda r: r[1] <= 50 and r[2] % 2 == 0 and r[3] < 80 and r[4] <= 70))
        client.until(lambda s: any(p['id'] == 'powerup.bonus_points_player' and p['remaining'] < 26.4 for p in s['drops']['active']))
        active = next(p for p in client.state()['drops']['active'] if p['id'] == 'powerup.bonus_points_player')
        p = active['position']
        client.call('origins/setup', {'position': [p[0], p[1]-.624, p[2]+2.2]})
        aim(client, p)
        before = client.wait_ticks(4)
        env(client, seed=seed_raw(lambda r: r[1] % 5 == 4))
        key = binding(before, 'forward')
        schedule(client, [edge(1, key, True), edge(50, key, False)])
        after = client.until(lambda s: s['drops']['collected'] > before['drops']['collected'])
        record('blood_money_source_draw_and_double_points', after['session']['points']-before['session']['points'] == 500, before=before, after=after)

    if enabled('parts'):
        for part in (p for p in game['environment']['parts'] if p['dig_zone']):
            indices = [i for i, m in enumerate(game['environment']['digging']['mounds']) if part['dig_zone'] in m['zone']][:2]
            fixture(client, game, weather='snow', active_mounds=indices, digger={'shovel': True, 'qualifying_digs': 1})
            for index in indices:
                before, after, result = dig(client, index, seed_for(lambda r: r[0] > 50 and r[1] <= 50))
                if part['id'] in after['environment']['parts']['pieces']:
                    break
            record('ice_region_'+part['dig_zone'], result['outcome'] == part['id'], before=before, after=after)
            approach(client, 'part', part['id'])
            tap(client, 'interact')
            after = client.until(lambda s: s['environment']['parts']['pieces'][part['id']]['state'] == 'owned')
            record('ice_collection_'+part['dig_zone'], True, after=after)

    if enabled('golden_loot'):
        fixture(client, game, weather='clear', digger={'shovel': True, 'golden': True, 'helmet': True, 'qualifying_digs': 30})
        index = min(client.state()['environment']['digging']['active'])
        claymore_seed = seed_raw(lambda r: r[1] % 100 <= 70 and r[2] % 2 == 1 and r[3] % 100 >= 90 and r[4] % 7 == 2)
        before, after, result = dig(client, index, claymore_seed)
        record('golden_rare_claymore_roll', result['outcome'] == 'weapon.claymore_zm', before=before, after=after)
        before, _ = approach(client, 'weapon', 'weapon.claymore_zm')
        tap(client, 'interact')
        after = client.until(lambda s: s['player']['mine']['id'] == 'weapon.claymore_zm')
        record('dug_claymore_uses_equipment_inventory', after['player']['mine']['count'] > 0 and not any(w['id'] == 'weapon.claymore_zm' for w in after['player']['weapons']), before=before, after=after)
        for chosen, count, golden in [('nuke', 2, False), ('double_points', 2, False), ('insta_kill', 4, True), ('full_ammo', 4, True), ('fire_sale', 3, False)]:
            fixture(client, game, weather='clear', digger={'shovel': True, 'golden': golden, 'helmet': golden, 'qualifying_digs': 30 if golden else 1}, box_moved=chosen == 'fire_sale')
            options = ['nuke', 'double_points'] + (['fire_sale'] if chosen == 'fire_sale' else []) + (['insta_kill', 'full_ammo'] if golden else [])
            selected = options.index(chosen)
            seed = seed_raw(lambda r: r[1] % 100 <= (70 if golden else 50) and r[2] % 2 == 0 and r[3] % 100 >= 80 and r[4] % count == selected)
            index = min(client.state()['environment']['digging']['active'])
            before, after, result = dig(client, index, seed)
            record('rare_powerup_'+chosen, result['outcome'] == chosen and chosen in after['environment']['digging']['rare_spawned'], before=before, after=after)
            index = min(after['environment']['digging']['active'])
            before, after, result = dig(client, index, seed)
            record('consecutive_rare_prize_suppressed_'+chosen, result['outcome'] in ('bonus_points_player', 'zombie_blood'), before=before, after=after)

    if enabled('streak'):
        fixture(client, game, weather='clear', digger={'shovel': True, 'qualifying_digs': 1, 'losing_streak': 3})
        index = min(client.state()['environment']['digging']['active'])
        before, after, result = dig(client, index, seed_for(lambda r: r[1] > 90))
        record('third_loss_forces_good_next_dig', result['outcome'] not in ('grenade', 'zombie') and state_digger(after)['losing_streak'] == 0, before=before, after=after)
        env(client, digger={'shovel': True, 'qualifying_digs': 1, 'losing_streak': 1})
        index = min(after['environment']['digging']['active'])
        before, after, result = dig(client, index, seed_for(lambda r: r[1] <= 50))
        record('good_reward_preserves_losing_streak', state_digger(after)['losing_streak'] == 1, before=before, after=after)

    if enabled('powerup_caps'):
        fixture(client, game, weather='clear', active_mounds=list(range(9)), box_moved=True,
                digger={'shovel': True, 'golden': True, 'helmet': True, 'qualifying_digs': 30})
        seed = seed_raw(lambda r: r[1] % 100 <= 70 and r[2] % 2 == 0 and r[3] % 100 >= 80 and r[4] % 60 == 0)
        expected = ['nuke', 'zombie_blood', 'double_points', 'bonus_points_player', 'fire_sale',
                    'bonus_points_player', 'insta_kill', 'bonus_points_player', 'bonus_points_player']
        for index, outcome in enumerate(expected):
            before, after, result = dig(client, index, seed)
            record('round_powerup_sequence_'+str(index+1), result['outcome'] == outcome, before=before, after=after)
        record('five_powerups_cap_and_one_blood', after['environment']['digging']['powerups_this_round'] == 5
               and after['environment']['digging']['bloods_this_round'] == 1 and 'full_ammo' not in after['environment']['digging']['rare_spawned'], after=after)
        before, after = ending(client, game, 1, 'clear')
        record('round_ending_resets_dig_powerup_tracking', after['environment']['digging']['powerups_this_round'] == 0
               and after['environment']['digging']['bloods_this_round'] == 0 and not after['environment']['digging']['rare_spawned'], before=before, after=after)

    if enabled('hidden'):
        fixture(client, game, weather='clear', digger={'shovel': True, 'qualifying_digs': 29})
        client.call('origins/setup', {'owned_perks': ['perk.quick_revive']})
        index = min(client.state()['environment']['digging']['active'])
        _, after, _ = dig(client, index, seed_for(lambda r: r[1] <= 50))
        hidden = client.until(lambda s: target(s, 'hidden') is not None and target(s, 'hidden')['rising'] is None)
        record('golden_hidden_dig_requires_blood', not target(hidden, 'hidden')['visible'], after=hidden)
        position = hidden['player']['position'][:]
        position[1] -= hidden['player']['height']/2
        client.call('origins/setup', {'pickups': [{'id': 'powerup.zombie_blood', 'position': position}]})
        client.until(lambda s: s['effects']['zombie_blood'] > 0)
        before, item = approach(client, 'hidden')
        tap(client, 'interact')
        after = client.until(lambda s: state_digger(s)['hidden_found'] == 1)
        client.call('origins/setup', {'position': [item['position'][0], item['position'][1]+.9, item['position'][2]+2.3]})
        aim(client, [item['position'][0], item['position'][1]+.9, item['position'][2]])
        before = client.wait_ticks(5)
        key = binding(before, 'forward')
        schedule(client, [edge(1, key, True), edge(25, key, False)])
        after = client.until(lambda s: s['environment']['digger']['perk_limit'] == 5)
        record('hidden_dig_normal_use_and_bottle_touch_reward', state_digger(after)['hidden_wait_round'] and target(after, 'hidden') is None, before=before, after=after)
        before, after = ending(client, game, 1, 'clear')
        after = client.until(lambda s: target(s, 'hidden') is not None)
        record('hidden_next_site_waits_for_round_end', len(state_digger(after)['hidden_used']) == 2 and not state_digger(after)['hidden_wait_round'], before=before, after=after)
        for found in (2, 3, 4):
            state = client.state()
            if state['effects']['zombie_blood'] < 10:
                p = state['player']['position'][:]
                p[1] -= state['player']['height']/2
                client.call('origins/setup', {'pickups': [{'id': 'powerup.zombie_blood', 'position': p}]})
                client.until(lambda s: s['effects']['zombie_blood'] > 20)
            before, item = approach(client, 'hidden')
            tap(client, 'interact')
            after = client.until(lambda s: state_digger(s)['hidden_found'] == found)
            client.call('origins/setup', {'position': [item['position'][0], item['position'][1]+.9, item['position'][2]+2.3]})
            aim(client, [item['position'][0], item['position'][1]+.9, item['position'][2]])
            key = binding(client.wait_ticks(5), 'forward')
            schedule(client, [edge(1, key, True), edge(25, key, False)])
            after = client.until(lambda s: s['environment']['digger']['perk_limit'] == 4+found)
            record('hidden_slot_'+str(found), len(state_digger(after)['hidden_used']) == found, before=before, after=after)
            before, after = ending(client, game, found, 'clear')
        after = client.wait_ticks(30)
        record('four_hidden_digs_cap_at_eight_perks', target(after, 'hidden') is None and after['environment']['digger']['perk_limit'] == 8, after=after)
        index = min(after['environment']['digging']['active'])
        before, after, result = dig(client, index, seed_for(lambda r: r[1] < 95 and r[2] <= 70 and r[3] % 2 == 0 and r[4] < 80 and r[5] <= 70))
        record('blood_money_before_downing', result['outcome'] == 'bonus_points_player', after=after)
        client.until(lambda s: any(p['id'] == 'powerup.bonus_points_player' and p['remaining'] < 26.4 for p in s['drops']['active']))
        active = next(p for p in client.state()['drops']['active'] if p['id'] == 'powerup.bonus_points_player')
        p = active['position']
        client.call('origins/setup', {'clear_enemies': True, 'pending': 1, 'pause_spawning_seconds': 300,
                                     'position': [p[0], p[1]-.624, p[2]+2.2]})
        aim(client, p)
        key = binding(client.state(), 'lethal')
        schedule(client, [edge(1, key, True), edge(250, key, False)])
        down = client.until(lambda s: s['player']['life'] == 'Downed')
        record('earned_perk_slots_survive_downing', down['environment']['digger']['perk_limit'] == 8, after=down)
        key = binding(down, 'forward')
        schedule(client, [edge(1, key, True), edge(160, key, False)])
        collected = client.until(lambda s: s['drops']['collected'] > down['drops']['collected'])
        record('blood_money_does_not_award_downed_player', collected['session']['points'] == down['session']['points'], before=down, after=collected)
        reset = client.reset()
        record('earned_perk_slots_reset_in_new_session', reset['environment']['digger']['perk_limit'] == 4, after=reset)

    if enabled('support'):
        for vehicle in (False, True):
            fixture(client, game)
            origin = list(game['spawn'])
            location = [origin[0], origin[1], origin[2]-5]
            client.call('origins/setup', {'enemies': [{'position': location, 'health': 100}]})
            client.until(lambda s: s['enemies'] and s['enemies'][0]['ready'])
            before = client.state()
            location = before['enemies'][0]['position'][:]
            location[1] += .8
            env(client, contacts=[{'vehicle': vehicle, 'position': location, 'size': [1.2, 2, 1.2], 'amount': 1000}])
            after = client.until(lambda s: s['damage']['recent'] and s['damage']['recent'][-1]['lethal'])
            record('vehicle_attribution' if vehicle else 'robot_attribution', after['session']['kills'] == before['session']['kills'] and after['session']['points'] == before['session']['points'] and after['damage']['recent'][-1]['points'] == 0 and not after['damage']['recent'][-1]['soul_eligible'], before=before, after=after)
        fixture(client, game)
        state = client.state()
        p = state['player']['position'][:]
        p[1] += state['player']['eye_height']-state['player']['height']/2
        p[2] -= 2
        env(client, shootables=[p])
        aim(client, p)
        before = client.state()
        tap(client, 'fire')
        after = client.until(lambda s: len(s['environment']['support']['hits']) > 0)
        record('normal_weapon_hits_environment', after['session']['shots'] > before['session']['shots'] and after['environment']['support']['hits'][-1]['amount'] > 0, before=before, after=after)
        before = fixture(client, game)
        authored = next(t for t in game['environment']['triggers'] if t['source'].get('classname') == 'trigger_damage')
        point = authored['position']
        for dx, dz in ((0, 2), (2, 0), (0, -2), (-2, 0)):
            client.call('origins/setup', {'position': [point[0]+dx, point[1], point[2]+dz]})
            client.wait_ticks(5)
            aim(client, point)
            tap(client, 'fire')
            after = client.wait_ticks(20)
            if after['environment']['support']['hits']:
                break
        record('normal_weapon_hits_imported_damage_trigger', bool(after['environment']['support']['hits']), source=authored['id'], before=before, after=after)
        for helmet in (True, False):
            fixture(client, game, digger={'shovel': True, 'golden': True, 'helmet': helmet, 'qualifying_digs': 30})
            before = client.state()
            env(client, contacts=[{'vehicle': False, 'position': before['player']['position'], 'size': [1, 2, 1], 'amount': 1000}])
            if helmet:
                after = client.wait_ticks(20)
                record('helmet_robot_contact_immunity', after['player']['life'] == 'Standing' and after['player']['health'] == before['player']['health'] and after['environment']['support']['contacts'] > 0, before=before, after=after)
            else:
                after = client.until(lambda s: s['player']['life'] != 'Standing')
                record('unprotected_robot_contact_damages_player', True, before=before, after=after)

    if enabled('visuals'):
        def capture(name):
            path = output.resolve()/(name+'.png')
            client.call('origins/capture', {'path': str(path)})
            state = client.until(lambda s: path.exists() and path.stat().st_size > 0)
            record('window_capture_'+name, True, path=str(path), after=state)
        fixture(client, game)
        approach(client, 'shovel')
        capture('shovel')
        tap(client, 'interact')
        fixture(client, game, weather='clear', active_mounds=[0], digger={'shovel': True, 'qualifying_digs': 1})
        approach(client, 'mound', 0)
        capture('mound-clear')
        for weather in ('rain', 'snow'):
            bursts = client.state()['environment']['weather']['precipitation_bursts']
            env(client, weather=weather, intensity=4)
            client.until(lambda s: s['environment']['weather']['presented'] == weather and s['environment']['weather']['audio_delay'] == 0 and s['environment']['weather']['precipitation_bursts'] >= bursts+10)
            capture('mound-'+weather)
            p = client.state()['player']['position']
            aim(client, [p[0], p[1]+3, p[2]-20])
            capture('weather-'+weather)
            approach(client, 'mound', 0)
        env(client, weather='clear')
        client.until(lambda s: s['environment']['weather']['presented'] == 'clear')
        _, after, result = dig(client, 0, seed_for(lambda r: r[1] <= 50 and r[2] % 2 == 1))
        approach(client, 'weapon', result['outcome'])
        capture('dug-weapon')
        fixture(client, game, weather='snow', active_mounds=[0], digger={'shovel': True, 'qualifying_digs': 1})
        _, after, result = dig(client, 0, seed_for(lambda r: r[0] > 50 and r[1] <= 50 and r[2] % 2 == 1))
        if not result['outcome'].startswith('part.'):
            approach(client, 'weapon', result['outcome'])
            tap(client, 'interact')
            client.until(lambda s: target(s, 'weapon', result['outcome']) is None)
            env(client, active_mounds=[0])
            _, after, result = dig(client, 0, seed_for(lambda r: r[0] > 50 and r[1] <= 50 and r[2] % 2 == 1))
        approach(client, 'part', result['outcome'])
        capture('ice-part')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--url', default='http://127.0.0.1:15707')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--identity', type=Path, required=True)
    parser.add_argument('--game', type=Path, default=Path(__file__).resolve().parents[1]/'assets/origins/game.json')
    parser.add_argument('--case', action='append')
    args = parser.parse_args()
    client = Client(args.url)
    report = {'passed': False, 'cases': [], 'identity': json.loads(args.identity.read_text()),
              'script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'support_scripts_sha256': {name: hashlib.sha256((Path(__file__).parent/name).read_bytes()).hexdigest() for name in ('gameplay_remote.py', 'verify_throwable_down.py')},
              'limitations': ['Setup selects scenario state, PRNG seeds and authentic locations. Outcomes require native input and observations.',
                              'Boundary cases start at 28/29 qualifying digs; this does not constitute a sustained 30-dig playthrough.',
                              'Weather and digging visuals and audible quality require separate actual-window inspection.']}
    def record(name, passed, **evidence):
        report['cases'].append({'case': name, 'passed': bool(passed), **evidence})
        write_report(args.output, 'environment-gameplay.json', report)
        print(name, 'PASS' if passed else 'FAIL', flush=True)
        if not passed:
            raise RuntimeError(name)
    try:
        report['discovery'] = client.call('rpc.discover')
        report['schema'] = client.call('origins/environment/schema')
        if client.state()['state'] == 'GameOver':
            client.reset()
        client.until(lambda s: s['state'] == 'Playing' and s.get('player'), timeout=180)
        game_path = args.game.resolve()
        report['content_sha256'] = hashlib.sha256(game_path.read_bytes()).hexdigest()
        run(client, json.loads(game_path.read_text()), record, args.case, args.output)
        report['passed'] = True
    except Exception as error:
        report['error'] = str(error)
    finally:
        client.shutdown(report)
        path = write_report(args.output, 'environment-gameplay.json', report)
    print(json.dumps({'passed': report['passed'], 'checks': len(report['cases']), 'error': report.get('error'), 'report': str(path)}))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
