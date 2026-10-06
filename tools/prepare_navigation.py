"""Prepare compiled native routes, player zones and ordinary spawn identities."""
import hashlib,json
import numpy as np
from prepare_gameplay_content import external
from prepare_origins import entities
from prepare_mud import brush_vertices

def prepare(config):
    base=external(config['assets']['inspection_output']);pack=external(config['game']['asset_pack'])
    records=entities(base/'game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents')
    source=base/'solo-content/navigation-recovery/map-navigation.json';nodes=json.loads(source.read_text())
    table=base/'solo-content/mud-triggers-recovery/map-triggers.json';models={str(r['model']):r for r in json.loads(table.read_text())}
    game=json.loads((pack/'game.json').read_text());content=game['content'];known={z['id'] for z in content['zones']}
    matrix=np.array([[1,0,0],[0,0,1],[0,-1,0]],dtype=float)
    coordinate=lambda p:(matrix@np.asarray(p)*.0254)
    zone=lambda name:'zone.start' if name=='zone_start' else 'zone.'+name.removeprefix('zone_')
    volumes=[];regions=[]
    for e in records:
        if e.get('classname')!='info_volume' or e.get('script_noteworthy')!='player_volume':continue
        z=zone(e['targetname']);assert z in known;origin=np.array([float(x) for x in e['origin'].split()])
        for b in models[e['model'][1:]]['hulls']:
            vertices=np.array(brush_vertices(b));points=np.array([coordinate(p+origin) for p in vertices]);center=points.mean(axis=0)
            volumes.append({'zone':z,'volume':{'id':'zonevolume.'+e['guid'].lower()+'.'+str(b['brush']),'position':center.tolist(),'vertices':(points-center).tolist()}})
            planes=[(np.array(p['normal']),p['distance']) for p in b['planes']]
            for axis in range(3):
                n=np.zeros(3);n[axis]=1;planes.extend([(n,b['max'][axis]),(-n,-b['min'][axis])])
            normals=np.array([p[0] for p in planes]);distances=np.array([p[1] for p in planes]);regions.append((z,origin,normals,distances,vertices.mean(axis=0)))
    def classify(point):
        # Retain authored node origins for native zone classification.
        point=np.array(point);best=None
        for z,origin,normals,distances,center in regions:
            delta=point-origin;outside=max(0.,float(np.max(normals@delta-distances)))
            score=(outside,float(np.linalg.norm(delta-center)))
            if best is None or score<best[0]:best=(score,z)
        return best[1] if best[0][0] < 48 else None
    unassigned=sum(classify(n['position']) is None for n in nodes)
    retained={n['index']:n for n in nodes}
    if not any(z['id']=='zone.unassigned' for z in content['zones']):content['zones'].append({'id':'zone.unassigned','initially_open':False})
    content['navigation_nodes']=[{'id':'node.native.'+str(i),'position':coordinate(n['position']).tolist(),'zone':classify(n['position']) or 'zone.unassigned'} for i,n in retained.items()]
    # Identify crossings using native convex solid brushes, expanded by the actor radius.
    door_regions=[]
    for d in game['access']['doors']:
        from scipy.spatial import ConvexHull
        for b in d['blocks']:
            h=ConvexHull(np.array(b['vertices']));door_regions.append((d['connection'],np.array(b['position']),h.equations[:,:3],h.equations[:,3]))
    def crossings(a,b):
        result=set();a=coordinate(a)+[0,.85,0];b=coordinate(b)+[0,.85,0]
        for identity,center,normals,offset in door_regions:
            start=normals@(a-center)+offset-.3;velocity=normals@(b-a);low,high=0.,1.
            for s,v in zip(start,velocity):
                if abs(v)<1e-8:
                    if s>0:high=-1;break
                elif v<0:low=max(low,-s/v)
                else:high=min(high,-s/v)
            if low<=high:result.add(identity)
        return sorted(result)
    links=[]
    for i,n in retained.items():
        for ordinal,l in enumerate(n['links']):
            other=l['to']
            if other not in retained:continue
            animation=n['animation'] or None
            traversal='walk'
            if l['negotiation']:
                traversal='jump' if animation and ('jump_down' in animation or 'jump_across' in animation) else 'climb'
            blockers=crossings(n['position'],retained[other]['position'])
            links.append({'id':f'link.native.{i}.{ordinal}','from':f'node.native.{i}','to':f'node.native.{other}','traversal':traversal,'connection':None,
                'blockers':blockers,'source_animation':animation if l['negotiation'] else None})
    content['navigation_links']=links
    spawns=[]
    for e in records:
        if e.get('script_noteworthy') not in ('riser_location','spawn_location') or not e.get('targetname','').endswith('_spawners'):continue
        z=zone(e['targetname'].removesuffix('_spawners'))
        if z not in known:continue
        spawns.append({'id':'spawn.'+e['guid'].lower(),'zone':z,'position':coordinate([float(x) for x in e['origin'].split()]).tolist(),'rising':e['script_noteworthy']=='riser_location'})
    game['access']['volumes']=volumes;game['access']['spawns']=spawns
    (pack/'game.json').write_text(json.dumps(game,indent=2)+'\n')
    receipt={'native_nodes':len(nodes),'retained_nodes':len(retained),'unassigned_nodes':unassigned,'links':len(links),'negotiations':sum(l['traversal']!='walk' for l in links),'blocked_links':sum(bool(l['blockers']) for l in links),'zone_brushes':len(volumes),'spawns':len(spawns),
        'navigation_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'brushes_sha256':hashlib.sha256(table.read_bytes()).hexdigest(),
        'approximation':'Avian validates ordinary floor/capsule routes at assembly. Authored node origins classify zones; nearest plane fallback limited to 48 source units. Door crossings expand native convex brushes by 30 cm. Negotiations retained but traversal execution remains a later milestone.'}
    (base/'solo-content/navigation-preparation.json').write_text(json.dumps(receipt,indent=2));print(receipt)
