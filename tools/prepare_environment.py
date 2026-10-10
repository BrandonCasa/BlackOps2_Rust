import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import tomllib

import numpy as np
from prepare_actors import convert
from prepare_gameplay_content import external
from prepare_origins import entities
from trigger_volumes import BASIS, SCALE, rotation


def prepare(config, gsc, csc, report_dir):
    inspection = Path(config['assets']['inspection_output'])
    pack = external(config['game']['asset_pack'])
    game = json.loads((pack/'game.json').read_text())
    content, environment = game['content'], game['environment']
    source_path = inspection/'game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents'
    records = entities(source_path)
    native_models = inspection/'game-prepare/models/model_export'
    extra_images = inspection/'solo-content/actor-images'

    def presentation(model):
        identity = 'presentation.environment.'+model
        if any(p['id'] == identity for p in content['presentations']):
            return identity
        name, _ = convert(native_models/(model+'_lod0.gltf'), pack/'props/environment', extra_images, [])
        content['presentations'].append({'id': identity, 'scene': 'props/environment/'+name+'#Scene0', 'animations': [], 'sounds': []})
        return identity

    regions = []
    for region in game['access']['volumes']:
        volume = region['volume']
        points = np.array(volume['vertices'])
        from scipy.spatial import ConvexHull
        hull = ConvexHull(points)
        regions.append((region['zone'], np.array(volume['position']), hull.equations))

    def location(e):
        position = BASIS @ np.array(list(map(float, e['origin'].split()))) * SCALE
        point = position + np.array([0, .8128, 0])
        zones = [zone for zone, center, planes in regions if np.all(planes[:, :3] @ (point-center)+planes[:, 3] <= .001)]
        if not zones:
            raise ValueError('dig location outside authored zone: '+e['guid'])
        matrix = BASIS @ rotation(e.get('angles', '0 0 0')) @ BASIS.T
        from scipy.spatial.transform import Rotation
        quat = Rotation.from_matrix(matrix).as_quat().tolist()
        return {'id': 'digsite.'+e['guid'].lower(), 'position': position.tolist(), 'rotation': quat, 'zone': zones[0], 'group': e.get('script_noteworthy', e['targetname']), 'initial': e.get('script_noteworthy') == 'initial_spot'}

    environment['digging'] = {'shovels': [location(e) for e in records if e.get('targetname') == 'shovel_location'],
                              'mounds': [location(e) for e in records if e.get('targetname') == 'dig_spot'],
                              'hidden': [location(e) for e in records if e.get('targetname') == 'zombie_blood_dig_spot'],
                              'shovel': presentation('p6_zm_tm_shovel'), 'mound': presentation('p6_zm_tm_dig_mound'),
                              'blood_mound': presentation('p6_zm_tm_dig_mound_blood'), 'perk_bottle': presentation('zombie_pickup_perk_bottle')}
    parts = []
    for element in ('water', 'air', 'lightning'):
        for index, piece in enumerate(('lower_staff', 'middle_staff', 'upper_staff')):
            record = next(e for e in records if e.get('targetname') == f'elemental_staff_{element}_{piece}')
            identity = f'part.elemental_staff_{element}.{piece}'
            if identity not in content['definitions']:
                content['definitions'].append(identity)
            parts.append({'id': identity, 'family': 'elemental_staff_'+element, 'presentation': presentation(record['model']),
                          'dig_zone': ('bunker', 'nml', 'village')[index] if element == 'water' else None,
                          'initial_misses': 2 if element == 'water' and index == 2 else 0})
    environment['parts'] = parts
    ownership = []
    for record in environment['triggers']+environment['brush_entities']:
        s = record['source']
        target, noteworthy = s.get('targetname', ''), s.get('script_noteworthy', '')
        if target == 'ambient_package' or target.startswith('snd'):
            owner = 'ambience'
        elif target.startswith(('trig_stomp_', 'clip_foot_')):
            owner = 'robot'
        elif 'tank' in target or target == 'wagon_damage_trigger':
            owner = 'vehicle'
        elif s.get('classname') == 'info_volume' and noteworthy == 'player_volume' or target in ('zombie_door', 'zombie_debris') or target.startswith(('door_bunker_', 'junk_')):
            owner = 'access'
        elif target == 'player_slow_area' or target == 'traversal_blocker':
            owner = 'movement'
        elif noteworthy in ('kill', 'kill_player', 'player_out_of_bounds', 'life_brush', 'kill_brush') or s.get('classname') == 'trigger_hurt':
            owner = 'boundary'
        elif 'craftable' in target:
            owner = 'crafting'
        elif target:
            owner = 'quest'
        else:
            owner = 'deferred'
        ownership.append({'id': record['id'], 'owner': owner, 'shootable': s.get('classname') == 'trigger_damage' or s.get('classname') == 'script_brushmodel' and bool(target) and owner == 'quest'})
    environment['ownership'] = ownership
    for powerup, model in [('bonus_points_player', 'zombie_z_money_icon'), ('bonus_points_team', 'zombie_z_money_icon'), ('fire_sale', 'zombie_firesale')]:
        identity = 'powerup.'+powerup
        if not any(p['id'] == identity for p in content['powerups']):
            content['powerups'].append({'id': identity, 'duration': 30 if powerup == 'fire_sale' else 0, 'presentation': presentation(model)})
    (pack/'game.json').write_text(json.dumps(game, separators=(',', ':'))+'\n')
    from prepare_audio import prepare as audio
    additions = {f'amb_weather_2d_{weather}_{where}' for weather in ('default', 'rain', 'snow') for where in ('indoor', 'outdoor')}
    additions.update(('amb_thunder_clap_zm', 'amb_thunder_flash_zm', 'amb_cave_wind_2d', 'amb_spiral_rm_air_2d', 'amb_robot_head_2d', 'evt_dig', 'evt_grenade_digup', 'evt_zombie_dig_dirt', 'evt_weapon_digup', 'evt_staff_digup', 'evt_staff_digup_lp', 'zmb_craftable_pickup', 'zmb_squest_golden_anything', 'vox_zmba_powerup_blood_money_0'))
    import csv
    for path in (inspection/'audio-recovery').rglob('*.aliases.csv'):
        additions.update(row['Name'] for row in csv.DictReader(path.open(encoding='utf-8-sig')) if re_match_dig_voice(row['Name']))
    audio_receipt = audio(config, additions)
    files = [gsc/name for name in ('zm_tomb_utility.gsc', 'zm_tomb_dig.gsc', 'zm_tomb_main_quest.gsc', 'zm_tomb_craftables.gsc', 'zombies/_zm_craftables.gsc')]
    files.extend(csc/name for name in ('zm_tomb.csc', 'zm_tomb_amb.csc', 'zm_tomb_fx.csc', 'zm_tomb_giant_robot.csc', 'zm_tomb_tank.csc'))
    receipt = {'sources': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in files},
               'entities_sha256': hashlib.sha256(source_path.read_bytes()).hexdigest(),
               'locations': {k: len(v) for k, v in environment['digging'].items() if isinstance(v, list)},
               'owners': dict(Counter(o['owner'] for o in ownership)), 'parts': parts, 'audio': audio_receipt,
               'approximations': ['Bevy particle meshes replace native FX emitters and native weather fog banks remain unreconstructed.', 'Native room volumes select ambience with source fades; native room reverb remains unreconstructed.', 'Mound rise uses linear interpolation; source easing is not reconstructed.', 'Dug zombies use the reusable prepared native rise traversal.']}
    report_dir = external(report_dir)
    report_dir.mkdir(parents=True, exist_ok=True)
    (report_dir/'preparation.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps({k: v for k, v in receipt.items() if k in ('locations', 'owners')}))


def re_match_dig_voice(name):
    import re
    return re.match(r'vox_plr_[0-3]_(dig_|pickup_shovel_)', name) is not None


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--gsc', type=Path, required=True)
    parser.add_argument('--csc', type=Path, required=True)
    parser.add_argument('--report-dir', type=Path, required=True)
    args = parser.parse_args()
    prepare(tomllib.loads(args.config.read_text(encoding='utf-8-sig')), args.gsc, args.csc, args.report_dir)
