"""Prepare native zombie negotiation clips and collider-owned root-motion samples."""
import json
import numpy as np
from prepare_actors import convert
from prepare_enemies import CLIPS, skin_margin
from prepare_gameplay_content import external
from xanim import decode

def clip_name(alias):
    short = alias.removeprefix("zm_")
    if short.startswith(("alcove_", "scaffold_")):
        return "ai_zombie_traverse_" + short
    if "dlc4_" in short:
        short = short.replace("dlc4_", "dlc4_trench_wall_")
    if short.startswith("vault_barbedwire_"):
        return "ai_zombie_vaultover_barbedwire" + ("_24" if short.endswith("_24") else "")
    if short.startswith("mantle_over_40"):
        return "ai_zombie_traverse_v1"
    return "ai_zombie_" + short

def prepare(config):
    base=external(config["assets"]["inspection_output"]);pack=external(config["game"]["asset_pack"])
    path=pack/"game.json";game=json.loads(path.read_text());content=game["content"]
    source=base/"solo-content/source/xanim"
    aliases=sorted({l["source_animation"] for l in content["navigation_links"] if l["source_animation"]})
    names={a:clip_name(a) for a in aliases};names["rise"]="ai_zombie_traverse_ground_dugup"
    profiles={}
    for alias,name in names.items():
        clip=decode((source/name).read_bytes());times,values=clip["delta"]["translation"]
        values=np.asarray(values);values=(values-values[0])[:,[0,2,1]]*[.0254,.0254,-.0254]
        profiles[alias]={"seconds":clip["duration"],"motion":[[float(t),*map(float,p)] for t,p in zip(times,values)]}
    models={e["presentation"].removeprefix("actor.") for e in content["enemies"]}
    models.update(p["presentation"].removeprefix("actor.") for e in content["enemies"] for p in e["parts"])
    from prepare_barriers import TEARS
    clips=list(dict.fromkeys([*CLIPS.values(),*names.values(),*TEARS]));margins={}
    for model in sorted(models):
        identity="actor."+model
        relative,notes=convert(base/"game-prepare/models/model_export"/(model+"_lod0.gltf"),pack/"actors",base/"solo-content/actor-images",[source/n for n in clips],in_place=True,end_aligned_clips={names["rise"]})
        content["animations"][:]=[a for a in content["animations"] if a["skeleton"]!=identity]
        refs=[]
        for index,(name,notifications) in enumerate(notes):
            aid="animation."+model+"."+name;refs.append(aid)
            content["animations"].append({"id":aid,"path":"actors/"+relative+f"#Animation{index}","skeleton":identity,"notifications":notifications})
        presentation=next(p for p in content["presentations"] if p["id"]==identity);presentation["animations"]=refs
        margins[model]=skin_margin(pack/"actors"/relative)
    for enemy in content["enemies"]:
        model=enemy["presentation"].removeprefix("actor.")
        enemy["traversals"]={a:{"animation":"animation."+model+"."+names[a],**profile} for a,profile in profiles.items()}
        enemy["bounds_margin"]=max(margins[model],*(margins[p["presentation"].removeprefix("actor.")] for p in enemy["parts"]))
    path.write_text(json.dumps(game,indent=2)+"\n")
    report={"aliases":names,"models":len(models),"clips_per_rig":len(clips),"profiles":profiles,"approximation":"Original negotiation root deltas fitted to authored endpoints; barrier mantle uses recovered traverse_v1 pending gait-specific state aliases. Collider follows root motion; native scripts use noclip/nogravity during traversal."}
    (base/"solo-content/traversal-preparation.json").write_text(json.dumps(report,indent=2));print({k:v for k,v in report.items() if k!="profiles"})
