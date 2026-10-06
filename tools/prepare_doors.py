"""Prepare native solo door groups, collision hulls and zone adjacency."""
import hashlib,json,math,re
import numpy as np
import soundfile as sf
from prepare_gameplay_content import external
from prepare_origins import entities
from prepare_actors import convert
from prepare_mud import brush_vertices

def prepare(config):
    inspection=external(config['assets']['inspection_output']);pack=external(config['game']['asset_pack'])
    records=entities(inspection/'game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents')
    table=inspection/'solo-content/mud-triggers-recovery/map-triggers.json'
    models={str(row['model']):row for row in json.loads(table.read_text())}
    matrix=np.array([[1,0,0],[0,0,1],[0,-1,0]],dtype=float)
    coordinate=lambda v:(matrix@np.array([float(x) for x in v.split()])*.0254).tolist()
    def hulls(e):
        result=[];origin=np.array([float(x) for x in e['origin'].split()])
        for brush in models[e['model'].lstrip('*')]['hulls']:
            points=np.array([matrix@(p+origin)*.0254 for p in brush_vertices(brush)])
            center=points.mean(axis=0)
            result.append({'id':'volume.'+e['guid'].lower()+'.'+str(brush['brush']),'position':center.tolist(),'vertices':(points-center).tolist()})
        if not result:raise ValueError('missing native blocker hull '+e['guid'])
        return result
    game=json.loads((pack/'game.json').read_text());content=game['content']
    script=inspection/'mechanics-rounds-01/decompile/decompiled/t6/zm_tomb.gsc'
    text=script.read_text();start=text.index('working_zone_init()\n{');end=text.index('\n}\n',start)
    links=re.findall(r'add_adjacent_zone\( "([^"]+)", "([^"]+)", "([^"]+)"',text[start:end])
    zone=lambda s:'zone.start' if s=='zone_start' else 'zone.'+s.removeprefix('zone_')
    zones=sorted({zone(s) for a,b,f in links for s in (a,b)})
    content['zones']=[{'id':s,'initially_open':s=='zone.start'} for s in zones]
    connections=[];flags={}
    for a,b,f in links:
        identity='connection.'+f+'.'+str(len(flags.get(f,[])))
        flags.setdefault(f,[]).append(identity)
        connections.append({'id':identity,'from':zone(a),'to':zone(b),'initially_open':f=='always_on'})
    content['zone_connections']=connections
    prefixes=('placement.door.','interaction.door.')
    for name,prefix in zip(('placements','interactions'),prefixes):content[name][:]=[x for x in content[name] if not x['id'].startswith(prefix)]
    props={}
    for model in ('p6_zm_tm_bunker_door','p6_zm_tm_rock_blockade'):
        relative,_=convert(inspection/'game-prepare/models/model_export'/f'{model}_lod0.gltf',pack/'props',inspection/'solo-content/actor-images',[])
        identity='presentation.door.'+model;props[model]=identity
        content['presentations'][:]=[x for x in content['presentations'] if x['id']!=identity]
        content['presentations'].append({'id':identity,'scene':'props/'+relative+'#Scene0','animations':[],'sounds':[]})
    # Deterministic mechanical/stone cues are explicit reconstructions, not native audio.
    (pack/'audio').mkdir(exist_ok=True);rate=24000
    for name in ('bunker','debris'):
        t=np.arange(rate)/rate;rng=np.random.default_rng(2606)
        noise=rng.normal(0,.15,len(t));noise=np.convolve(noise,np.ones(9)/9,'same')
        wave=(noise+(.05*np.sin(2*np.pi*(75*t+30*t*t))))*np.sin(np.pi*t)**.5
        if name=='debris':wave*=np.exp(-4*t)
        relative='audio/door-'+name+'.ogg';sf.write(pack/relative,wave,rate,format='OGG',subtype='VORBIS')
        identity='sound.door.'+name
        content['sounds'][:]=[x for x in content['sounds'] if x['id']!=identity]
        content['sounds'].append({'id':identity,'path':relative,'volume':.7,'pitch':[1.,1.],'distance':16.,'looping':False})
    groups={};triggers=[e for e in records if e.get('targetname') in ('zombie_door','zombie_debris')]
    for e in triggers:groups.setdefault(e['target'],[]).append(e)
    doors=[]
    for target,trigs in groups.items():
        first=trigs[0];flag=first['script_flag'];edges=flags[flag]
        price=int(first['zombie_cost'])-250
        if price>=2500:price-=250
        debris=first['targetname']=='zombie_debris';visuals=[];blocks=[]
        for e in records:
            if e.get('targetname')!=target:continue
            if e.get('classname')=='script_brushmodel':blocks+=hulls(e)
            elif e.get('classname')=='script_model':
                visuals.append({'presentation':props[e['model']],'position':coordinate(e['origin']),'yaw':math.radians(float(e.get('angles','0 0 0').split()[1])),
                    'offset':coordinate(e.get('script_vector','0 0 0'))})
        doors.append({'connection':edges[0],'connections':edges,'debris':debris,'seconds':1.,'props':visuals,'blocks':blocks,'sound':'sound.door.'+('debris' if debris else 'bunker')})
        for e in trigs:
            identity=e['guid'].lower();placement='placement.door.'+identity
            content['placements'].append({'id':placement,'definition':edges[0],'position':coordinate(e['origin']),'yaw':0.,'zone':connections[next(i for i,c in enumerate(connections) if c['id']==edges[0])]['from']})
            content['interactions'].append({'id':'interaction.door.'+identity,'placement':placement,'action':{'kind':'door','connection':edges[0],'price':price},'reach':2.3,'hold_seconds':0.,'hidden_prompt':False})
    game['access']={'doors':doors}
    (pack/'game.json').write_text(json.dumps(game,indent=2)+'\n')
    receipt={'groups':len(doors),'triggers':len(triggers),'connections':len(connections),'zones':len(zones),'props':sum(len(d['props']) for d in doors),'blocks':sum(len(d['blocks']) for d in doors),
        'script_sha256':hashlib.sha256(script.read_bytes()).hexdigest(),'collision_sha256':hashlib.sha256(table.read_bytes()).hexdigest(),
        'approximation':'Opening sounds reconstructed; native models, placements, collision brush planes, solo prices, slide vector and adjacency retained. Tank/quest flags remain closed until their systems implement them.'}
    (inspection/'solo-content/doors-preparation.json').write_text(json.dumps(receipt,indent=2));print(receipt)
