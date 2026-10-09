import json, math
import numpy as np
from prepare_actors import convert
from prepare_gameplay_content import external
from prepare_origins import entities
from xanim import decode

TEARS = [f"ai_zombie_boardtear_aligned_m_{i}_{phase}" for i in range(1,7) for phase in ("grab","pull")]

def prepare(config):
    base=external(config['assets']['inspection_output']);pack=external(config['game']['asset_pack'])
    game=json.loads((pack/'game.json').read_text());c=game['content'];source=base/'solo-content/source/xanim'
    records=entities(base/'game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents')
    coord=lambda p:(np.array(list(map(float,p.split())))[[0,2,1]]*[.0254,.0254,-.0254]).tolist()
    boards=[]
    for i in range(1,7):
        model=f'p6_anim_zm_barricade_board_{i:02}';identity='presentation.barrier.'+model
        names=[f'o_zombie_board_{i}_{p}' for p in ('repair','pull')]
        relative,notes=convert(base/'game-prepare/models/model_export'/(model+'_lod0.gltf'),pack/'props',base/'solo-content/actor-images',[source/n for n in names],board_delta=True)
        c['presentations'][:]=[p for p in c['presentations'] if p['id']!=identity]
        c['animations'][:]=[a for a in c['animations'] if a['skeleton']!=identity]
        refs=[]
        for index,(name,notifications) in enumerate(notes):
            aid='animation.barrier.'+name;refs.append(aid)
            c['animations'].append({'id':aid,'path':'props/'+relative+f'#Animation{index}','skeleton':identity,'notifications':notifications})
        assert len(refs)==2,model
        c['presentations'].append({'id':identity,'scene':'props/'+relative+'#Scene0','animations':refs,'sounds':[]})
        grab=decode((source/TEARS[(i-1)*2]).read_bytes());pull=decode((source/TEARS[(i-1)*2+1]).read_bytes())
        def motion(clip):
            times,values=clip['delta']['translation']
            values=values[:,[0,2,1]]*[.0254,.0254,-.0254]
            return [[float(t),*map(float,p)] for t,p in zip(times,values)]
        boards.append({'presentation':identity,'repair':refs[0],'pull':refs[1],
            'repair_seconds':decode((source/names[0]).read_bytes())['duration'],
            'pull_seconds':decode((source/names[1]).read_bytes())['duration'],
            'grab_seconds':grab['duration'],'tear_seconds':pull['duration'],
            'destroy_seconds':next(n['seconds'] for n in pull['notifications'] if n['event']=='destroy_piece'),
            'grab_clip':TEARS[(i-1)*2],'tear_clip':TEARS[(i-1)*2+1],
            'grab_motion':motion(grab),'tear_motion':motion(pull)})
    for table,prefix in (('placements','placement.barrier.'),('interactions','interaction.barrier.')):
        c[table][:]=[p for p in c[table] if not p['id'].startswith(prefix)]
    native=json.loads((base/'solo-content/navigation-recovery/map-navigation.json').read_text())
    c['barriers']=[]
    for e in records:
        if e.get('classname')!='zbarrier_zmcore_BasicWoodBarrier':continue
        assert int(e['zbarriernumboards'])==6
        name=e['targetname'];identity='barrier.'+name;position=coord(e['origin'])
        goals=[r for r in records if r.get('targetname')=='exterior_goal' and r.get('target')==name]
        assert len(goals)==1,name
        candidates=[n for n in native if n['animation']=='zm_mantle_over_40' and np.linalg.norm(np.array(coord(' '.join(map(str,n['position']))))-position)<.01]
        assert len(candidates)==1,name
        node=candidates[0];links=[l for l in node['links'] if l['negotiation']];assert len(links)==1
        entry='node.native.'+str(node['index']);exit='node.native.'+str(links[0]['to'])
        interior=next(n for n in c['navigation_nodes'] if n['id']==exit);z=interior['zone'];assert z!='zone.unassigned'
        next(n for n in c['navigation_nodes'] if n['id']==entry)['zone']=z
        placement='placement.barrier.'+name
        c['placements'].append({'id':placement,'definition':identity,'position':position,'yaw':math.radians(float(e['angles'].split()[1])),'zone':z})
        c['interactions'].append({'id':'interaction.barrier.'+name,'placement':placement,'action':{'kind':'barrier','barrier':identity},'reach':94.21*.0254,'hold_seconds':1.,'hidden_prompt':False})
        c['barriers'].append({'id':identity,'placement':placement,'entry':entry,'exit':exit,'boards':boards,'repair_sound':'sound.barrier.repair','break_sound':'sound.barrier.break'})
    (pack/'game.json').write_text(json.dumps(game,indent=2)+'\n')
    from prepare_audio import prepare as prepare_audio
    prepare_audio(config)
    report={'windows':len(c['barriers']),'boards':len(boards),'native_tear_clips':TEARS,'reward':'10; counter increments before strict < min(50*round,500) cap check',
        'approximation':'Middle attack slot only; window collision is a reconstructed full opening blocker. Native placements, ingress links, board meshes and animation clocks retained.'}
    (base/'solo-content/barriers-preparation.json').write_text(json.dumps(report,indent=2));print(report)
