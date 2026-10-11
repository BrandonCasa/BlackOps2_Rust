import argparse
import hashlib
import json
from pathlib import Path

from prepare_actors import append_clips
from prepare_arsenal import combat_weapon_fields
from xanim import decode


def prepare(base, inspection, output):
    game = json.loads((base / 'game.json').read_bytes())
    content = game['content']
    recovered = json.loads((inspection / 'solo-content/recovered-gameplay.json').read_bytes())
    source = inspection / 'solo-content/source/xanim'
    presentations = [p for p in content['presentations'] if p.get('scene')]
    def presentation(model):
        rows = [p for p in presentations if Path(p['scene'].split('#')[0]).name == model + '_lod0.gltf']
        rows.sort(key=lambda p: ('/zm_tomb/' not in p['scene'], p['id']))
        if not rows:
            raise ValueError('Missing original model: ' + model)
        return rows[0]
    patch = {'weapons': combat_weapon_fields(recovered, content), 'animations': [], 'enemies': [], 'presentations': [], 'files': []}
    prepared = {}
    def clips(model, names):
        row = presentation(model)
        relative = Path(row['scene'].split('#')[0])
        document = json.loads((base / relative).read_bytes())
        if len(document['buffers']) != 1:
            raise ValueError('Animation append requires single source buffer')
        binary_relative = relative.parent / document['buffers'][0]['uri']
        original = (base / binary_relative).read_bytes()
        binary = bytearray(original)
        existing = {a['name'] for a in document.get('animations', [])}
        append_clips(document, binary, [source / name for name in names if name not in existing], in_place=True)
        document['buffers'][0]['byteLength'] = len(binary)
        if binary[:len(original)] != original:
            raise ValueError('Changed original actor mesh buffer')
        target = output / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(document, separators=(',', ':')))
        (output / binary_relative).write_bytes(binary)
        for path in (relative, binary_relative):
            patch['files'].append({'path': path.as_posix(), 'before': hashlib.sha256((base / path).read_bytes()).hexdigest(), 'after': hashlib.sha256((output / path).read_bytes()).hexdigest()})
        additions = []
        for index, animation in enumerate(document.get('animations', [])):
            if animation['name'] not in names:
                continue
            name = animation['name']
            decoded = decode((source / name).read_bytes())
            identity = 'animation.' + model + '.' + name
            patch['animations'].append({'id': identity, 'path': relative.as_posix() + '#Animation' + str(index), 'skeleton': row['id'], 'notifications': decoded['notifications']})
            additions.append(identity)
            prepared[name] = decoded
        patch['presentations'].append({'id': row['id'], 'animations': additions})
        return row['id']
    panzer_names = ['ai_zombie_mech_intro_jump_in', 'ai_zombie_mech_walk_basic', 'ai_zombie_mech_run', 'ai_zombie_mech_idle', 'ai_zombie_mech_melee_a', 'ai_zombie_mech_ft_fire_start', 'ai_zombie_mech_ft_fire_loop', 'ai_zombie_mech_ft_fire_end', 'ai_zombie_mech_ft_burn_player', 'ai_zombie_mech_grapple_intro_sprint_to_aim_5', 'ai_zombie_mech_grapple_aim_5', 'ai_zombie_mech_stunned', 'ai_zombie_mech_death', 'ai_zombie_mech_death_explode', 'ai_zombie_mech_faceplate_pain', 'ai_zombie_mech_powercore_pain']
    body = clips('c_zom_mech_body', panzer_names)
    clips('fxanim_zom_tomb_portal_mod', ['fxanim_zom_tomb_portal_open_anim', 'fxanim_zom_tomb_portal_open_1frame_anim', 'fxanim_zom_tomb_portal_collapse_anim'])
    clips('p6_zm_tm_challenge_box', ['o_zombie_dlc4_challenge_box_open', 'o_zombie_dlc4_challenge_box_close'])
    robot_names = ['ai_zombie_giant_robot_walk_' + route + suffix for route in ('nml','trenches','village') for suffix in ('_intro','','_outtro')]
    for model in ('veh_t6_dlc_zm_robot','veh_t6_dlc_zm_robot_0','veh_t6_dlc_zm_robot_1','veh_t6_dlc_zm_robot_2'):
        clips(model, robot_names)
    clips('veh_t6_dlc_zm_robot_foot_hatch', ['ai_zombie_giant_robot_hatch_close','ai_zombie_giant_robot_hatch_open','ai_zombie_giant_robot_hatch_open_idle'])
    parts = [('c_zom_mech_faceplate','j_helmet'), ('c_zom_mech_powersupply_cap','tag_powersupply'), ('c_zom_mech_claw','tag_claw'), ('c_zom_mech_armor_knee_left','j_knee_attach_le'), ('c_zom_mech_armor_knee_right','j_knee_attach_ri'), ('c_zom_mech_armor_shoulder_left','j_shoulderarmor_le'), ('c_zom_mech_armor_shoulder_right','j_shoulderarmor_ri')]
    volumes = [('head','head','j_head','j_head_end',.14), ('chest','upper_torso','j_spineupper','j_spine4',.33), ('abdomen','torso','j_spinelower','j_spineupper',.3), ('pelvis','torso','j_mainroot','j_spinelower',.28), ('upperarm_le','left_arm','j_shoulder_le','j_elbow_le',.18), ('forearm_le','left_arm','j_elbow_le','j_wrist_le',.16), ('upperarm_ri','right_arm','j_shoulder_ri','j_elbow_ri',.18), ('forearm_ri','right_arm','j_elbow_ri','j_wrist_ri',.16), ('thigh_le','leg','j_hip_le','j_knee_le',.19), ('shin_le','leg','j_knee_le','j_ankle_le',.17), ('thigh_ri','leg','j_hip_ri','j_knee_ri',.19), ('shin_ri','leg','j_knee_ri','j_ankle_ri',.17), ('powerplant','powerplant','tag_powersupply','tag_powersupply',.19)]
    identity = lambda name: 'animation.c_zom_mech_body.' + name
    patch['enemies'].append({'id': 'enemy.panzer', 'health': 5000, 'speed': 1.65, 'damage': 75, 'presentation': body, 'parts': [{'presentation': presentation(model)['id'], 'socket': socket, 'root_joint': None} for model, socket in parts], 'animation_actions': {key: identity(name) for key, name in [('idle','ai_zombie_mech_idle'), ('walk','ai_zombie_mech_walk_basic'), ('run','ai_zombie_mech_run'), ('attack','ai_zombie_mech_melee_a'), ('death','ai_zombie_mech_death')]}, 'attack_seconds': prepared['ai_zombie_mech_melee_a']['duration'], 'attack_hits': [n['seconds'] for n in prepared['ai_zombie_mech_melee_a']['notifications'] if n['event'] == 'fire'], 'death_seconds': prepared['ai_zombie_mech_death']['duration'], 'corpse_seconds': 3., 'bounds_margin': 1.2, 'family': 'panzer', 'hit_regions': [{'id': name, 'region': region, 'start': start, 'end': end, 'radius': radius, 'presentation': body} for name, region, start, end, radius in volumes]})
    patch['durations'] = {name: data['duration'] for name, data in prepared.items()}
    output.mkdir(parents=True, exist_ok=True)
    (output / 'combat-patch.json').write_text(json.dumps(patch, indent=2) + '\n')
    return {'weapons': len(patch['weapons']), 'animations': len(patch['animations']), 'files': len(patch['files']), 'output': str(output)}


def prepare_actor_states(base, inspection, output, state_definition, clone_only=False):
    import re
    import numpy as np
    game = json.loads((base / 'game.json').read_bytes())
    content = game['content']
    source = inspection / 'solo-content/source/xanim'
    text = state_definition.read_text() if not clone_only else ""
    def state(name):
        if clone_only: return []
        matches = re.findall(r'(?m)^' + name + r'\s*:[^\n]*\s*\{([^}]+)\}', text)
        if len(matches) != 1:
            raise ValueError('Missing original state: ' + name)
        return re.sub(r'//[^\n]*', '', matches[0]).split()
    deaths = state('zm_death_fire')
    burned = state('zm_move_burned')
    tank = {}
    for direction in ('up','down'):
        aliases = state('zm_tank_jump_' + direction)
        if len(aliases) % 2:
            raise ValueError('Invalid original tank aliases')
        tank.update({'tank_' + direction + '_' + alias: name for alias,name in zip(aliases[::2],aliases[1::2])})
    if not clone_only:
        tank['tank_super_sprint'] = state('zm_move_super_sprint')[0]
    names = ['ch_dazed_a_death','ai_zombie_crawl_death_v1'] if clone_only else list(dict.fromkeys([*deaths,*burned,*tank.values()]))
    decoded = {n:decode((source/n).read_bytes()) for n in names}
    patch = {'animations': [], 'enemies': [], 'presentations': [], 'files': [], 'durations':{n:d['duration'] for n,d in decoded.items()}}
    rigs = {e['presentation'] for e in content['enemies'] if e['id'] != 'enemy.panzer'}
    rigs.update(p['presentation'] for e in content['enemies'] if e['id'] != 'enemy.panzer' for p in e.get('parts',[]))
    robot_names = ['ai_zombie_giant_robot_bunker_intro']
    if not clone_only: rigs.update(p['id'] for p in content['presentations'] if p.get('scene') and Path(p['scene'].split('#')[0]).name.removesuffix('_lod0.gltf') in ('veh_t6_dlc_zm_robot','veh_t6_dlc_zm_robot_0','veh_t6_dlc_zm_robot_1','veh_t6_dlc_zm_robot_2'))
    for rig in sorted(rigs):
        row = next(p for p in content['presentations'] if p['id'] == rig)
        relative = Path(row['scene'].split('#')[0])
        doc = json.loads((base / relative).read_bytes())
        binary_relative = relative.parent / doc['buffers'][0]['uri']
        original = (base / binary_relative).read_bytes()
        binary = bytearray(original)
        clips = robot_names if relative.name.startswith('veh_t6_dlc_zm_robot') else names
        existing = {a['name'] for a in doc.get('animations',[])}
        missing = [n for n in clips if n not in existing]
        appended = append_clips(doc,binary,[source/n for n in missing],in_place=True)
        if [n for n,_ in appended] != missing or binary[:len(original)] != original:
            raise ValueError('Original actor append did not bind or changed original buffer: ' + rig)
        doc['buffers'][0]['byteLength'] = len(binary)
        target = output / relative
        target.parent.mkdir(parents=True,exist_ok=True)
        target.write_text(json.dumps(doc,separators=(',',':')))
        (output / binary_relative).write_bytes(binary)
        for path in (relative,binary_relative):
            patch['files'].append({'path':path.as_posix(),'before':hashlib.sha256((base/path).read_bytes()).hexdigest(),'after':hashlib.sha256((output/path).read_bytes()).hexdigest()})
        additions = []
        model = rig.removeprefix('actor.')
        for index,animation in enumerate(doc.get('animations',[])):
            n = animation['name']
            if n not in clips: continue
            aid='animation.' + model + '.' + n
            d=decoded.get(n) or decode((source/n).read_bytes())
            patch['animations'].append({'id':aid,'path':relative.as_posix()+'#Animation'+str(index),'skeleton':rig,'notifications':d['notifications']})
            additions.append(aid)
            patch['durations'][n]=d['duration']
        patch['presentations'].append({'id':rig,'animations':additions})
    for enemy in ([] if clone_only else content['enemies']):
        if enemy['id']=='enemy.panzer': continue
        model=enemy['presentation'].removeprefix('actor.')
        traversal={}
        for alias,n in tank.items():
            d=decoded[n]
            if d['delta'] is None: raise ValueError('Native tank animation lacks root delta: '+n)
            times,values=d['delta']['translation']
            values=np.asarray(values)[:,[0,2,1]]*[.0254,.0254,-.0254]
            if alias=='tank_super_sprint': values-=values[0]
            traversal[alias]={'animation':'animation.'+model+'.'+n,'seconds':d['duration'],'motion':[[float(time),*map(float,p)] for time,p in zip(times,values)]}
        patch['enemies'].append({'id':enemy['id'],'combat_states':{'fire_death':[{'animation':'animation.'+model+'.'+n,'seconds':decoded[n]['duration'],'hits':[]} for n in deaths]},'locomotion_states':{'burned':['animation.'+model+'.'+n for n in burned]},'traversals':traversal})
    output.mkdir(parents=True,exist_ok=True)
    (output/('clone-patch.json' if clone_only else 'actor-state-patch.json')).write_text(json.dumps(patch,indent=2)+'\n')
    return {'files':len(patch['files']),'animations':len(patch['animations']),'enemies':len(patch['enemies']),'output':str(output)}


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--base-pack', type=Path, required=True)
    parser.add_argument('--inspection', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--state-definition', type=Path)
    parser.add_argument('--clone-clips', action='store_true')
    arguments = parser.parse_args()
    if arguments.state_definition or arguments.clone_clips:
        result=prepare_actor_states(arguments.base_pack,arguments.inspection,arguments.output,arguments.state_definition,arguments.clone_clips)
    else:
        result=prepare(arguments.base_pack, arguments.inspection, arguments.output)
    print(json.dumps(result))
