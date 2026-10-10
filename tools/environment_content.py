import math
import re


OWNERS = {'boundary', 'movement', 'access', 'ambience', 'robot', 'vehicle', 'quest', 'crafting', 'deferred'}


def validate(environment, content, require):
    records = environment.get('triggers', []) + environment.get('brush_entities', [])
    ownership = environment.get('ownership', [])
    require(isinstance(ownership, list) and len(ownership) == len(records), 'environment ownership closure missing')
    require(all(isinstance(o, dict) and set(o) == {'id', 'owner', 'shootable'}
                and o['owner'] in OWNERS and type(o['shootable']) is bool for o in ownership), 'invalid environment owner')
    require(len({o['id'] for o in ownership}) == len(ownership)
            and {o['id'] for o in ownership} == {r['id'] for r in records}, 'environment ownership mismatch')
    presentations = {p['id'] for p in content['presentations'] if p['scene'] is not None}
    parts = environment.get('parts', [])
    require(isinstance(parts, list) and len(parts) <= 128, 'invalid part catalog')
    ids, zones = set(), set()
    for p in parts:
        require(isinstance(p, dict) and set(p) == {'id', 'family', 'presentation', 'dig_zone', 'initial_misses'}, 'invalid part fields')
        require(p['id'] not in ids and p['id'] in content['definitions'], 'invalid shared part identity')
        ids.add(p['id'])
        require(isinstance(p['family'], str) and 0 < len(p['family']) < 64 and p['presentation'] in presentations, 'invalid part presentation')
        require(type(p['initial_misses']) is int and 0 <= p['initial_misses'] <= 4, 'invalid initial part misses')
        require(p['dig_zone'] is None or p['dig_zone'] in ('bunker', 'nml', 'village') and p['dig_zone'] not in zones, 'invalid dig part region')
        if p['dig_zone'] is not None:
            zones.add(p['dig_zone'])
    digging = environment.get('digging')
    if digging is None:
        return
    require(isinstance(digging, dict) and set(digging) == {'shovels', 'mounds', 'hidden', 'shovel', 'mound', 'blood_mound', 'perk_bottle'}, 'invalid digging fields')
    for key in ('shovel', 'mound', 'blood_mound', 'perk_bottle'):
        require(digging[key] in presentations, 'missing dig presentation')
    ids = set()
    known_zones = {z['id'] for z in content['zones']}
    for table, limits in [('shovels', (1, 32)), ('mounds', (1, 256)), ('hidden', (4, 128))]:
        require(isinstance(digging[table], list) and limits[0] <= len(digging[table]) <= limits[1], 'invalid dig location count')
        for spot in digging[table]:
            require(isinstance(spot, dict) and set(spot) == {'id', 'position', 'rotation', 'zone', 'group', 'initial'}, 'invalid dig location fields')
            require(isinstance(spot['id'], str) and 0 < len(spot['id']) < 128 and re.fullmatch(r'[A-Za-z0-9_.-]+', spot['id']) and spot['id'] not in ids, 'invalid dig identity')
            ids.add(spot['id'])
            for key, size in [('position', 3), ('rotation', 4)]:
                require(isinstance(spot[key], list) and len(spot[key]) == size and all(type(n) in (float, int) and math.isfinite(n) for n in spot[key]), 'invalid dig pose')
            require(abs(sum(n*n for n in spot['rotation'])**0.5 - 1) < .001, 'invalid dig rotation')
            require(spot['zone'] in known_zones and type(spot['initial']) is bool and isinstance(spot['group'], str) and 0 < len(spot['group']) < 128, 'invalid dig location context')
    weapons = {w['id'] for w in content['weapons'] if w.get('pickup') is not None}
    require(all('weapon.'+w in weapons for w in ('ballista_zm', 'c96_zm', '870mcs_zm', 'dsr50_zm', 'srm1216_zm', 'claymore_zm', 'ak74u_zm', 'ksg_zm', 'mp40_zm', 'mp44_zm', 'frag_grenade_zm')), 'missing dig weapons')
    require(all('powerup.'+p in {p['id'] for p in content['powerups']} for p in ('nuke', 'double_points', 'fire_sale', 'insta_kill', 'full_ammo', 'zombie_blood', 'bonus_points_player')), 'missing dig powerups')
