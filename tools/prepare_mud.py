"""Prepare the authored Origins slow-area collision brush union and script rules."""
import hashlib,itertools,json,re
from pathlib import Path
import numpy as np
from prepare_gameplay_content import external
from prepare_origins import entities

def prepare(config):
    inspection=external(config["assets"]["inspection_output"]);pack=external(config["game"]["asset_pack"])
    table=inspection/"solo-content/mud-triggers-recovery/map-triggers.json"
    records=entities(inspection/"game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents")
    models={str(row["model"]):row for row in json.loads(table.read_text())}
    hulls=[]
    matrix=np.array([[1,0,0],[0,0,1],[0,-1,0]],dtype=float)
    for e in records:
        if e.get("targetname")!="player_slow_area":continue
        origin=np.array([float(x) for x in e["origin"].split()]);model=models[e["model"].lstrip("*")]
        for brush in model["hulls"]:
            planes=[(np.array(p["normal"]),p["distance"]) for p in brush["planes"]]
            for axis in range(3):
                n=np.zeros(3);n[axis]=1;planes.extend([(n,brush["max"][axis]),(-n,-brush["min"][axis])])
            normals=np.array([n for n,d in planes]);distances=np.array([d for n,d in planes]);vertices=[]
            for ids in itertools.combinations(range(len(planes)),3):
                selected=normals[list(ids)]
                if abs(np.linalg.det(selected))<1e-8:continue
                point=np.linalg.solve(selected,distances[list(ids)])
                if np.all(normals@point<=distances+0.01) and not any(np.linalg.norm(point-v)<0.01 for v in vertices):vertices.append(point)
            if len(vertices)<4:raise ValueError("empty native slow-area convex brush")
            points=np.array([matrix@(p+origin)*0.0254 for p in vertices]);center=points.mean(axis=0)
            hulls.append({"id":"mud."+e["guid"].lower()+"."+str(brush["brush"]),"position":center.tolist(),"vertices":(points-center).tolist()})
    if not hulls:raise ValueError("missing native slow area")
    script=inspection/"mechanics-world-02/decompiled/t6/zm_tomb_utility.gsc"
    text=script.read_text();section=text[text.index("player_slow_movement_speed_monitor()"):text.index("dug_zombie_spawn_init(")]
    scales=[float(x) for x in re.findall(r'n_new_move_scale = (0\.\d+);',section)]
    assert scales==[.7,.6] and 'wait 0.1;' in section and '0.4 / 4.0' in section and '0.3 / 6.0' in section
    profile={"scale":.6,"stamin_scale":.7,"step":.1,"stamin_step":.05,"interval":.1,"volumes":hulls}
    game=json.loads((pack/"game.json").read_text());game["environment"]={"mud":profile};(pack/"game.json").write_text(json.dumps(game,indent=2)+"\n")
    receipt={"native_volumes":len(hulls),"script_sha256":hashlib.sha256(script.read_bytes()).hexdigest(),"brushes_sha256":hashlib.sha256(table.read_bytes()).hexdigest(),"rules":{k:v for k,v in profile.items() if k!="volumes"},"approximation":"Convex vertices solve retained brush planes with 0.01 source-unit tolerance; runtime touching uses Avian capsules. Original mud sound/VO pending."}
    (inspection/"solo-content/mud-preparation.json").write_text(json.dumps(receipt,indent=2));print(receipt)
