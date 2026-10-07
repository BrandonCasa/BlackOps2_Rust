"""Read licensed SAB banks, resolve native aliases, and prepare current-game audio.

SAB v14 layouts/hash follow retained OAT SoundBankTypes/CommonT6. Originals are
read only. Alias CSVs come from OAT soundbank extraction, samples from all native
bank dependencies (including common banks not loaded by a single zone export).
"""
import argparse
import csv
import hashlib
import io
import json
import math
from pathlib import Path
import re
import struct
import subprocess
import tomllib

import numpy as np
import soundfile as sf
from prepare_gameplay_content import external
from prepare_origins import entities

RATES = [8000,12000,16000,24000,32000,44100,48000,96000,192000]


def recover_aliases(config):
    base=external(config['assets']['inspection_output'])
    installation=Path(config['assets']['installation_root'])
    output=base/'audio-recovery';output.mkdir(parents=True,exist_ok=True)
    unlinker=Path(config.get('preparation',{}).get('unlinker',base/'tooling/OAT/v0.33.0/Unlinker.exe'))
    common=installation/'zone/all/common_zm.ff';patch=installation/'zone/all/patch_zm.ff'
    for zone,path in [('common_zm',common),('zm_tomb',installation/'zone/all/zm_tomb.ff'),('en_zm_tomb',installation/'zone/english/en_zm_tomb.ff')]:
        command=[str(unlinker),'--no-color','--include-assets','soundbank,soundpatch','--search-path',str(installation),'-o',str(output/'?zone?')]
        if zone!='common_zm':command+=['--load',str(common),'--load',str(patch)]
        with (output/(zone+'-extract.log')).open('wb') as log:
            subprocess.run(command+[str(path)],stdout=log,stderr=subprocess.STDOUT,check=True)
        text=(output/(zone+'-extract.log')).read_text(errors='replace')
        if not re.search(r'Finished with \d+ warnings, 0 errors',text):
            raise ValueError(f'OAT soundbank extraction failed: {zone}')


def sound_hash(name):
    result = 0x1505
    for char in name.lower():
        result = (ord(char) + 0x1003f * result) & 0xffffffff
    return result or 1


def bank_index(installation):
    result = {}
    paths = sorted((installation/'sound').glob('*.sab*'))
    # Tomb/patch variants override shared entries, with both stream/load banks retained.
    paths.sort(key=lambda p: ('tomb' in p.name, 'patch' in p.name, p.name))
    for path in paths:
        with path.open('rb') as stream:
            header = stream.read(0x48)
            magic,version,size,_,_,count,_ = struct.unpack_from('<7I',header)
            length,offset = struct.unpack_from('<qq',header,0x20)
            if magic != 0x23585532 or version != 14 or size != 20 or length != path.stat().st_size:
                raise ValueError(f'invalid SAB header: {path.name}')
            if offset < 0 or offset + count*20 > length:
                raise ValueError('SAB table outside file')
            stream.seek(offset)
            for identity,amount,start,frames,rate,channels,looping,kind in struct.iter_unpack('<4I4B',stream.read(count*20)):
                if start < 0x800 or start+amount>length or rate>=len(RATES) or not 1<=channels<=8:
                    raise ValueError('invalid SAB entry')
                result[identity] = (path,amount,start,frames,rate,channels,looping,kind)
    return result


def prepare(config):
    base = external(config['assets']['inspection_output'])
    pack = external(config['game']['asset_pack'])
    game = json.loads((pack/'game.json').read_text())
    content = game['content']
    rows = {}
    if any(not list((base/'audio-recovery'/zone/'soundbank').glob('*.aliases.csv')) for zone in ('common_zm','zm_tomb','en_zm_tomb')):
        recover_aliases(config)
    for zone in ('common_zm','zm_tomb','en_zm_tomb'):
        for path in sorted((base/'audio-recovery'/zone/'soundbank').glob('*.aliases.csv')):
            grouped = {}
            for row in csv.DictReader(path.open(encoding='utf-8-sig')):
                grouped.setdefault(row['Name'],[]).append(row)
            rows.update(grouped)
    if not rows:
        raise ValueError('extract OAT soundbank alias CSVs before preparing audio')
    banks = bank_index(Path(config['assets']['installation_root']))
    aliases, missing, provenance, samples, visiting, promoted = {}, [], [], {}, set(), []
    content['sounds'][:] = [s for s in content['sounds'] if not s['id'].startswith('sound.native.')]
    directory = pack/'audio/native';directory.mkdir(parents=True,exist_ok=True)

    def alias(name):
        if name in aliases: return name
        if name not in rows: return None
        if name in visiting: raise ValueError(f'cyclic native secondary audio: {name}')
        visiting.add(name)
        choices = [];durations=[]
        for index,row in enumerate(rows[name]):
            source = re.sub(r'\.(wav|flac)$','',row['FileSource'])
            if not source: continue
            identity = sound_hash(source)
            entry = banks.get(identity)
            if not entry:
                missing.append({'alias':name,'source':source});continue
            path,amount,start,frames,rate,channels,looping,kind = entry
            spatial=row['PanType']=='3d' or name.startswith('mus_perks_') and name.endswith('_jingle')
            sample_key=(identity,spatial)
            if sample_key not in samples:
                with path.open('rb') as stream:
                    stream.seek(start);raw = stream.read(amount)
                if kind == 0:
                    wave = np.frombuffer(raw,dtype='<i2').reshape(-1,channels).astype(np.float32)/32768
                    sample_rate = RATES[rate]
                elif kind == 8:
                    wave,sample_rate = sf.read(io.BytesIO(raw),dtype='float32',always_2d=True)
                else: raise ValueError(f'unsupported native audio format {kind}: {name}')
                if len(wave)!=frames or not np.isfinite(wave).all():
                    raise ValueError(f'invalid decoded sample: {name} ({len(wave)} != {frames})')
                # Bevy spatial stereo sums ears, so positional samples are prepared mono.
                digest = hashlib.sha256(raw).hexdigest()[:24]
                relative = f'audio/native/{digest}-'+('mono' if spatial else 'stereo')+'.ogg'
                if spatial:wave=wave.mean(axis=1)
                cached=pack/relative
                try:
                    info=sf.info(cached)
                    ready=info.format=='OGG' and info.subtype=='VORBIS' and info.frames>0
                except (sf.LibsndfileError,OSError):ready=False
                if not ready:
                    print('encode',name,sample_rate,wave.shape,flush=True)
                    subprocess.run(['ffmpeg','-v','error','-y','-f','f32le','-ar',str(sample_rate),'-ac',str(1 if wave.ndim==1 else wave.shape[1]),'-i','pipe:0','-ar',str(min(sample_rate,48000)),'-c:a','libvorbis','-q:a','5',str(pack/relative)],input=np.asarray(wave,dtype='<f4').tobytes(),check=True)
                samples[sample_key] = (relative,len(wave)/sample_rate,spatial)
                provenance.append({'source':source,'bank':path.name,'asset_id':identity,'sha256':hashlib.sha256(raw).hexdigest(),'prepared':relative})
            relative,duration,mono = samples[sample_key]
            durations.append(duration)
            sid = f'sound.native.{name}.{index}'
            content['sounds'].append({'id':sid,'path':relative,'volume':min(1.,10**((float(row['VolMax'])-100)/20)),
                'pitch':[2**(float(row['PitchMin'])/1200),2**(float(row['PitchMax'])/1200)],
                'distance':10. if name.startswith('mus_perks_') and name.endswith('_jingle') else max(.1,float(row['DistMaxDry'])*.0254),'looping':row['Looping']=='looping'})
            choices.append(sid)
        if not choices:
            visiting.remove(name)
            return None
        row=rows[name][0]
        secondary=alias(row['Secondary']) if row['Secondary'] else None
        aliases[name]={'sounds':choices,'spatial':row['PanType']=='3d' or name.startswith('mus_perks_') and name.endswith('_jingle'),'channel':'dialogue' if name.startswith('vox_') or row['Bus']=='bus_voice' else 'music' if row['IsMusic']=='yes' or name.startswith('mus_') else 'ambience' if row['VolumeGroup']=='grp_ambience' else 'effects',
            'secondary':secondary,'subtitle':row['Subtitle'],'limit':max(1,min(32,int(row['LimitCount']) or 8)),
            'priority':int(float(row['PriorityMax'])),'seconds':max(durations)}
        visiting.remove(name)
        return name

    bindings={};timelines={}
    wanted = {'zmb_board_slam','zmb_boards_float','zmb_repair_boards','zmb_break_boards','zmb_pneumatic_door','amb_rock_fall','zmb_vocals_zombie_ambience','zmb_vocals_zombie_attack','zmb_vocals_zombie_death','zmb_vocals_zombie_teardown','zmb_vocals_zombie_sprint',
        'zmb_attack_whoosh','zmb_monkey_song','wpn_semtex_alert','wpn_semtex_charge','mus_zombie_round_start','mus_zombie_round_over','chr_breathing_hurt','chr_breathing_hurt_start','chr_breathing_better','chr_revive_gasp','zmb_cherry_explode','zmb_phdflop_explo','amb_weather_2d_rain_outdoor','amb_weather_2d_rain_indoor','amb_cave_wind','zmb_cha_ching','zmb_perk_bottle_open','zmb_perk_bottle_drink'}
    # Retain all footstep surfaces; active collision metadata selects the surface.
    wanted.update(n for n in rows if re.match(r'fly_step_(walk|run|sprint|crouch_walk)_(plr|npc)_',n))
    wanted.update(n for n in rows if n.startswith(('fly_prone_','wpn_grenade_bounce_','wpn_sticky_gren_bounce_','zmb_monkey_land_')))
    wanted.update(n for n in rows if re.match(r'mus_perks_.*_(jingle|sting)$',n))
    wanted.add('mus_zombie_game_over')
    dialogue_events=('ammo_low','ammo_out','kill_headshot','kill_close','kill_explo','generic_wall_buy','exert_pain_low','exert_pain_high','heal_revived','perk_jugga','perk_mule','perk_revive','perk_speed','perk_stamine','during_round_1','end_round_1')
    wanted.update(n for n in rows if any(re.match(r'vox_plr_[0-3]_'+e+r'_',n) for e in dialogue_events))
    recovered=json.loads((base/'solo-content/recovered-gameplay.json').read_text())
    weapon_by_source={w['source']:w['fields'] for w in recovered['weapons']}
    for w in content['weapons']:
        fields=weapon_by_source.get(w['id'].removeprefix('weapon.'),{})
        for event,field in {'shot':'fireSoundPlayer','dry_fire':'emptyFireSoundPlayer','equip':'raiseSoundPlayer','unequip':'putawaySoundPlayer','melee':'meleeSwipeSoundPlayer','hit':'meleeHitSound','prime':'pullbackSoundPlayer','throw':'fireSoundPlayer','explode':'projExplosionSound','bounce':'bounceSound','purchase':'pickupSoundPlayer','ammo':'ammoPickupSoundPlayer'}.items():
            sound=fields.get(field)
            if sound and event in ('shot','throw') and sound.endswith('_LFE'):
                parents=[name for name,variants in rows.items() if name.endswith('_fire_plr') and any(r['Secondary']==sound for r in variants)]
                if len(parents)==1:
                    promoted.append({'weapon':w['id'],'exported':sound,'primary':parents[0]})
                    sound=parents[0]
            if sound: bindings[w['id']+'.'+event]=sound;wanted.add(sound)
    # Native sndnt animation notifies provide actual reload/equipment/perk sound timing.
    for animation in content['animations']:
        notes=[{'seconds':n['seconds'],'alias':n['event'].split('#',1)[1]} for n in animation['notifications'] if n['event'].startswith('sndnt#')]
        if notes:
            timelines[animation['id']]=notes;wanted.update(n['alias'] for n in notes)
    emitters=[]
    for row in entities(base/'game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents'):
        name=row.get('script_sound')
        if name and row.get('classname') in ('script_struct','script_origin') and name in rows and rows[name][0]['Looping']=='looping':
            xyz=list(map(float,row['origin'].split()))
            emitters.append({'alias':name,'position':[xyz[0]*.0254,xyz[2]*.0254,-xyz[1]*.0254]});wanted.add(name)
    for name in sorted(wanted):alias(name)
    bindings={key:value for key,value in bindings.items() if value in aliases}
    timelines={key:[n for n in notes if n['alias'] in aliases] for key,notes in timelines.items()}
    # Replace every previously generated current-game cue with native recordings.
    for old,new in {'sound.barrier.repair':'zmb_board_slam','sound.barrier.break':'zmb_break_boards','sound.door.bunker':'zmb_pneumatic_door','sound.door.debris':'amb_rock_fall'}.items():
        if new not in aliases:
            raise ValueError(f'required replacement alias missing: {new}')
        original=next(s for s in content['sounds'] if s['id']==aliases[new]['sounds'][0])
        content['sounds'][:]=[s for s in content['sounds'] if s['id']!=old]
        content['sounds'].append(dict(original,id=old))
    bindings.update({old:new for old,new in {'sound.barrier.repair':'zmb_board_slam','sound.barrier.break':'zmb_break_boards','sound.door.bunker':'zmb_pneumatic_door','sound.door.debris':'amb_rock_fall'}.items()})
    collision=json.loads((pack/game['collision_scene'].split('#')[0]).read_text())
    surfaces={}
    for material in collision.get('materials',[]):
        name=material['name'].lower()
        surfaces[material['name']]=next((surface for tokens,surface in [(('water','puddle'),'water'),(('mud',),'mud'),(('wood','board','plank'),'wood'),(('metal','steel','iron','pipe'),'metal'),(('brick',),'brick'),(('concrete','cement'),'concrete'),(('gravel',),'gravel'),(('grass',),'grass'),(('dirt','soil','earth'),'dirt'),(('rock','stone'),'rock')] if any(token in name for token in tokens)),'default')
    game['audio']={'aliases':aliases,'bindings':bindings,'timelines':timelines,'emitters':[e for e in emitters if e['alias'] in aliases],'character':0,'surfaces':surfaces}
    (pack/'game.json').write_text(json.dumps(game,indent=2)+'\n')
    report={'aliases':len(aliases),'samples':len(samples),'bindings':len(bindings),'timelines':len(timelines),'emitters':len(game['audio']['emitters']),'missing':missing,'missing_aliases':sorted(wanted-aliases.keys()),'provenance':provenance,'promoted_primary_layers':promoted,
        'approximations':['Linear attenuation and variant choice reconstruct native acoustic curves/weights.','Collision material names classify footsteps offline; exact native physics surface flags remain unrecovered.','Rain follows the current static rainy sky; dynamic weather gameplay is pending.','Perk machines use a bounded ten-meter music radius.','Native alias subtitles are empty; event captions do not invent speech transcripts.','Debris uses native rock-fall samples; script sound-family indirection remains unrecovered.']}
    (base/'audio-recovery/preparation.json').write_text(json.dumps(report,indent=2)+'\n')
    print({k:v for k,v in report.items() if k not in ('provenance','missing')},'missing samples',len(missing))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--config',type=Path,required=True);args=p.parse_args()
    prepare(tomllib.loads(args.config.read_text(encoding='utf-8-sig')))
