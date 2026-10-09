import json
import numpy as np
from pathlib import Path
from prepare_actors import convert
from prepare_gameplay_content import external
from xanim import decode

CLIPS = {"idle":"ai_zombie_idle_v1_delta", "walk":"ai_zombie_walk_v1", "run":"ai_zombie_run_v2", "sprint":"ai_zombie_sprint_v1",
         "attack":"ai_zombie_attack_v1", "death":"ai_zombie_firestaff_death_collapse_a"}

def hit_regions(enemy):
    regions = [("head", "head", "j_head", "j_head_end", .105, enemy["parts"][0]["presentation"]),
        ("chest", "upper_torso", "j_spineupper", "j_spine4", .21, enemy["presentation"]),
        ("abdomen", "torso", "j_spinelower", "j_spineupper", .20, enemy["presentation"]),
        ("pelvis", "torso", "j_mainroot", "j_spinelower", .18, enemy["presentation"])]
    for side in ("le", "ri"):
        for label, region, start, end, radius in (
            ("upperarm", "arm", "shoulder", "elbow", .10),
            ("forearm", "arm", "elbow", "wrist", .08),
            ("hand", "arm", "wrist", "mid", .09),
            ("thigh", "leg", "hip", "knee", .12),
            ("shin", "leg", "knee", "ankle", .105),
            ("foot", "leg", "ankle", "ball", .10)):
            regions.append((label+"_"+side,region,"j_"+start+"_"+side,
                "j_"+end+"_"+side+("_1" if end=="mid" else ""),radius,enemy["presentation"]))
    return [dict(id=i,region=r,start=a,end=b,radius=v,presentation=p) for i,r,a,b,v,p in regions]

def prepare_hit_regions(config):
    pack = external(config["game"]["asset_pack"])
    path = pack / "game.json"
    game = json.loads(path.read_text(encoding="utf-8-sig"))
    for enemy in game["content"]["enemies"]:
        enemy["hit_regions"] = hit_regions(enemy)
    path.write_text(json.dumps(game,indent=2)+"\n",encoding="utf-8")
    print(json.dumps({"variants":len(game["content"]["enemies"]),"volumes_per_variant":16,
        "approximation":"Anatomical capsule radii, bound to original animated joints; not recovered native collision surfaces."}))

def skin_margin(path):
    doc = json.loads(path.read_text())
    buffers = [(path.parent / b["uri"]).read_bytes() for b in doc["buffers"]]
    def array(index):
        a = doc["accessors"][index]; v = doc["bufferViews"][a["bufferView"]]
        width = {"VEC3":3,"VEC4":4,"MAT4":16}[a["type"]]
        dtype = {5126:"<f4",5123:"<u2",5121:"u1"}[a["componentType"]]
        return np.frombuffer(buffers[v["buffer"]],dtype=dtype,count=a["count"]*width,
            offset=v.get("byteOffset",0)+a.get("byteOffset",0)).reshape(-1,width)
    maximum = 0.
    for node in doc["nodes"]:
        if node.get("scale",[1,1,1]) != [1,1,1]: raise ValueError("nonrigid actor scale needs a larger envelope")
        if "mesh" not in node or "skin" not in node: continue
        binds = array(doc["skins"][node["skin"]]["inverseBindMatrices"]).reshape(-1,4,4).transpose(0,2,1)
        for primitive in doc["meshes"][node["mesh"]]["primitives"]:
            attrs = primitive["attributes"]
            positions = array(attrs["POSITION"])
            joints,weights = array(attrs["JOINTS_0"]),array(attrs["WEIGHTS_0"])
            homogeneous = np.c_[positions,np.ones(len(positions))]
            for i in range(4):
                active = weights[:,i] > 0
                if active.any():
                    offsets = np.einsum("nij,nj->ni",binds[joints[:,i]],homogeneous)
                    maximum = max(maximum,float(np.linalg.norm(offsets[:,:3],axis=1)[active].max()))
    return maximum + 0.025

def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    pack = external(config["game"]["asset_pack"])
    models = inspection / "game-prepare/models/model_export"
    images = inspection / "solo-content/actor-images"
    source = inspection / "solo-content/source/xanim"
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    content = game["content"]
    families = {"german":(["1a","1b"],["1","2","3"]), "crusader":(["1a","1c","2b","2c"],["1","2"])}
    names = []
    for family, (bodies, heads) in families.items():
        names += [f"c_zom_tomb_{family}_body_{n}" for n in bodies]
        names += [f"c_zom_tomb_{family}_head{n}" for n in heads]
    margins = {}
    for name in names:
        relative, notifications = convert(models / (name + "_lod0.gltf"), pack / "actors", images,
            [source / n for n in CLIPS.values()], in_place=True)
        margins[name] = skin_margin(pack / "actors" / relative)
        identity = "actor." + name
        content["presentations"][:] = [p for p in content["presentations"] if p["id"] != identity]
        content["animations"][:] = [a for a in content["animations"] if a["skeleton"] != identity]
        refs = []
        for i,(clip,notes) in enumerate(notifications):
            clip_id = "animation." + name + "." + clip; refs.append(clip_id)
            content["animations"].append({"id":clip_id,"path":"actors/"+relative+f"#Animation{i}",
                "skeleton":identity,"notifications":notes})
        content["presentations"].append({"id":identity,"scene":"actors/"+relative+"#Scene0","animations":refs,"sounds":[]})
    attack = decode((source / CLIPS["attack"]).read_bytes())
    death = decode((source / CLIPS["death"]).read_bytes())
    enemies = []
    for family,(bodies,heads) in families.items():
        for body in bodies:
            for head in heads:
                model = f"c_zom_tomb_{family}_body_{body}"
                identity = "enemy.zombie" if family == "german" and body == "1a" and head == "1" else f"enemy.{family}.{body}.{head}"
                enemies.append({"id":identity,"health":150,"speed":1.15,"damage":50,"presentation":"actor."+model,
                    "parts":[{"presentation":f"actor.c_zom_tomb_{family}_head{head}","socket":"j_spine4","root_joint":"j_spine4"}],
                    "animation_actions":{action:"animation."+model+"."+clip for action,clip in CLIPS.items()},
                    "attack_seconds":attack["duration"],"attack_hits":[n["seconds"] for n in attack["notifications"] if n["event"]=="fire"],
                    "death_seconds":death["duration"],"corpse_seconds":8.,"bounds_margin":max(margins[model],margins[f"c_zom_tomb_{family}_head{head}"]),
                    "family":family})
    content["enemies"] = enemies
    for enemy in enemies:
        enemy["hit_regions"] = hit_regions(enemy)
    (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n",encoding="utf-8")
    report = {"models":len(names),"enemy_variants":len(enemies),"attack_hits":enemies[0]["attack_hits"],
        "death_seconds":death["duration"],"approximations":["Recovered fire-staff collapse provides ordinary corpse motion; actor translations are normalized to bind offsets, preserving omitted channels and collider-owned motion.",
        "Joint envelope uses the maximum positively weighted bind offset plus 2.5 cm; authored head roots inherit the torso socket.",
        "Headwear, gibs and zone-specific crusader admissions remain pending."]}
    (inspection / "solo-content/enemy-preparation.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
    print(json.dumps(report))
