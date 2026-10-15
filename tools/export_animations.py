"""Invoke unmodified external exporters on OAT output. No native decoding lives here."""
import argparse
import json
import sys
from dataclasses import asdict
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--oat", type=Path, required=True)
parser.add_argument("--overlay", type=Path, action="append", default=[])
parser.add_argument("--tool", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--report", type=Path, required=True)
parser.add_argument("--game-definition", type=Path)
parser.add_argument("--actor-bindings", type=Path, default=Path(__file__).with_name("origins_actor_bindings.json"))
args = parser.parse_args()
sys.path.insert(0, str(args.tool.resolve() / "scripts"))
from cod.animations import load_clips, clip_to_gltf
from cod.viewmodel import parse_infostring, role_clips, hidden_bones, definition as weapon_definition
from cod.gltf import read_glb, write_glb
from cod.actors import clip_entry, _write_json, write_clip_glb
from cod.characters import skeleton_spec
from cod.t6.actors import read_asd

sources = [args.oat, *args.overlay]
clips = {}
weapon_files = {}
for source in sources:
    clips.update(load_clips(str(source / "xanim")))
    weapon_files.update({p.name:p for p in (source / "weapons").glob("*") if p.is_file()})
source_weapons = {}
hands_bindings = {}
models = args.oat / "model_export"
def model_path(name):
    return next((source / "model_export" / (name + "_lod0.glb") for source in reversed(sources) if (source / "model_export" / (name + "_lod0.glb")).is_file()), None)
report = {"tool": "open-fastfile@088dac3bdee0e6e653871f877ad8ee4326d314a4", "decoded_clips": len(clips), "weapons": [], "machines": [], "errors": []}
(args.out / "model_export").mkdir(parents=True, exist_ok=True)
for weapon, source in sorted(weapon_files.items()):
    raw_info = parse_infostring(source.read_text())
    info = raw_info.copy()
    info = {key:(value.lower() if key.endswith(("Anim", "AnimLeft", "AnimRight")) else value) for key,value in info.items()}
    gun = model_path(info.get("gunModel", ""))
    if gun is None:
        report["errors"].append({"weapon":weapon,"error":"Missing exported source gunModel","model":info.get("gunModel")})
        continue
    wanted = sorted({v for k,v in info.items() if k.endswith(("Anim", "AnimLeft", "AnimRight")) and v in clips})
    missing = sorted({v for k,v in info.items() if k.endswith(("Anim", "AnimLeft", "AnimRight")) and v and v not in clips})
    if missing:
        report["errors"].append({"weapon":weapon,"missing_source_clips":missing})
    gltf, buf = read_glb(str(gun))
    joints = {gltf["nodes"][j].get("name") for skin in gltf.get("skins",[]) for j in skin["joints"]}
    roles, absent = role_clips(info, clips)
    spec = weapon_definition("t6",weapon,info,roles,clips,hidden_bones(info,joints),used=wanted)
    source_weapons[weapon] = {"info":raw_info,"spec":spec,"scene":f"model_export/weapon_{weapon}.glb#Scene0"}
    tracks = []
    for name in wanted:
        written, unmatched = clip_to_gltf(gltf,buf,clips[name])
        tracks.append({"name":name,"written":written,"unmatched":unmatched})
    gltf["scenes"][0].setdefault("extras",{})["viewmodel"] = spec
    gltf["buffers"] = [{"byteLength":len(buf)}]
    output = args.out / "model_export" / f"weapon_{weapon}.glb"
    write_glb(str(output),gltf,buf)
    source_weapons[weapon]["clip_bindings"] = {animation["name"]:str(output.relative_to(args.out))+f"#Animation{index}" for index,animation in enumerate(gltf.get("animations",[]))}
    report["weapons"].append({"weapon":weapon,"scene":str(output.relative_to(args.out)),"clips":spec["clips"],"roles":spec["roles"],"tracks":tracks,"source_model":info.get("gunModel")})
# Existing native_scene bindings use one hands model per character. Preserve that interface.
for character in ("dempsey", "nikolai", "richtofen", "takeo"):
    source = models / f"c_zom_{character}_viewhands_lod0.glb"
    gltf, buf = read_glb(str(source))
    for name, clip in clips.items():
        if name.startswith("viewmodel_"):
            clip_to_gltf(gltf, buf, clip)
    gltf["buffers"] = [{"byteLength": len(buf)}]
    write_glb(str(args.out / "model_export" / source.name), gltf, buf)
    hands_bindings[character] = {animation["name"]:"model_export/"+source.name+f"#Animation{index}" for index,animation in enumerate(gltf.get("animations",[]))}
# Source state definitions retain all actual state/alias bindings. They are not mapped to game roles here.
for source in sorted((args.oat / "animstatedefs").glob("*.asd")):
    states = read_asd(source.read_text())
    names = {name for state in states.values() for alias, name in state["clips"]}
    selected = {name: clips[name] for name in sorted(names) if name in clips}
    output = args.out / "actors" / source.stem
    output.mkdir(parents=True, exist_ok=True)
    _write_json(str(output / "source-clips.json"), {name: clip_entry(clip, path=True) for name, clip in selected.items()})
    # Preserve decoded source tracks verbatim where the external summary intentionally rounds/drops them.
    _write_json(str(output / "source-tracks.json"), {name:{"fps":clip.fps,"frames":clip.frames,"notetracks":clip.notetracks,"delta_translation":asdict(clip.delta_translation) if clip.delta_translation else None,"delta_rotation":asdict(clip.delta_rotation) if clip.delta_rotation else None} for name,clip in selected.items()})
    _write_json(str(output / "source-states.json"), states)
    report["machines"].append({"source": source.name, "clips": len(selected), "missing": sorted(names - clips.keys())})
    binding = json.loads(args.actor_bindings.read_text())["machines"].get(source.stem, [])
    for model in binding:
        original = model_path(model)
        if original is None:
            report["errors"].append({"actor_model":model,"error":"Missing source actor model"})
            continue
        gltf, buf = read_glb(str(original))
        tracks = []
        for name, clip in selected.items():
            written, unmatched = clip_to_gltf(gltf, buf, clip)
            tracks.append({"name":name,"written":written,"unmatched":unmatched})
        gltf["buffers"] = [{"byteLength":len(buf)}]
        write_glb(str(args.out / "model_export" / original.name),gltf,buf)
        labels = {animation["name"]:"model_export/"+original.name+f"#Animation{index}" for index,animation in enumerate(gltf.get("animations",[]))}
        for track in tracks:
            track["animation_path"] = labels.get(track["name"])
        report.setdefault("actors",[]).append({"model":model,"machine":source.stem,"scene":"model_export/"+original.name,"tracks":tracks})

# Source-owned animated gameplay props outside actor state machines.
for model,names in json.loads(args.actor_bindings.read_text()).get("clips",{}).items():
    original=model_path(model)
    if original is None:raise ValueError(f"Required animated source prop {model} is missing")
    gltf,buf=read_glb(str(original))
    for name in names:
        if name not in clips:raise ValueError(f"Required source prop clip {name} is missing")
        clip_to_gltf(gltf,buf,clips[name])
    gltf["buffers"]=[{"byteLength":len(buf)}]
    write_glb(str(args.out / "model_export" / original.name),gltf,buf)
# Committed source prop/model clip assignments; no native parsing or conversion here.
model_bindings=json.loads(Path(__file__).with_name("origins_model_animation_bindings.json").read_text())
model_report=[]
for model,names in model_bindings.items():
    original=model_path(model)
    if original is None:
        report["errors"].append({"model":model,"error":"Missing source animated model"});continue
    gltf,buf=read_glb(str(original))
    tracks=[]
    for name in names:
        if name not in clips:
            report["errors"].append({"model":model,"missing_clip":name});continue
        written,unmatched=clip_to_gltf(gltf,buf,clips[name]);tracks.append({"name":name,"written":written,"unmatched":unmatched})
    if any(track["written"] for track in tracks):
        gltf["buffers"]=[{"byteLength":len(buf)}]
        write_glb(str(args.out / "model_export" / original.name),gltf,buf)
    labels={animation["name"]:"model_export/"+original.name+f"#Animation{index}" for index,animation in enumerate(gltf.get("animations",[]))}
    for track in tracks:track["animation_path"]=labels.get(track["name"])
    model_report.append({"model":model,"scene":"model_export/"+original.name,"tracks":tracks})
_write_json(str(args.out / "source-model-clips.json"),model_report)
# Existing game-owned presentation records supply exact scene-to-clip bindings.
# Never infer gameplay identity from a model name, bone overlap or file order.
if args.game_definition:
    game = json.loads(args.game_definition.read_text())
    animations = {entry["id"]: entry for entry in game["content"]["animations"]}
    report["presentations"] = []
    for presentation in game["content"]["presentations"]:
        if not presentation.get("scene") or not presentation.get("animations"):
            continue
        scene = Path(presentation["scene"].split("#", 1)[0])
        source = models / scene.name
        if not source.is_file():
            report["errors"].append({"presentation": presentation["id"], "error": "Missing original exported scene", "scene": str(source)})
            continue
        names = [ident.rsplit(".", 1)[-1] for ident in presentation["animations"]]
        missing = [name for name in names if name not in clips]
        if missing:
            report["errors"].append({"presentation": presentation["id"], "missing_source_clips": missing})
            continue
        gltf, buf = read_glb(str(source))
        matched = []
        for name in names:
            written, unmatched = clip_to_gltf(gltf, buf, clips[name])
            matched.append({"name": name, "written": written, "unmatched": unmatched})
        output = args.out / scene
        output.parent.mkdir(parents=True, exist_ok=True)
        gltf["buffers"] = [{"byteLength": len(buf)}]
        write_glb(str(output), gltf, buf)
        report["presentations"].append({"id": presentation["id"], "scene": str(scene), "clips": matched})
_write_json(str(args.out / "source-weapons.json"),source_weapons)
_write_json(str(args.out / "source-viewmodel-clips.json"),{name:clip_entry(clip,path=True) for name,clip in clips.items() if name.startswith("viewmodel_")})
_write_json(str(args.out / "source-hands.json"),hands_bindings)
_write_json(str(args.out / "source-actors.json"),json.loads(args.actor_bindings.read_text()))
_write_json(str(args.out / "source-actor-clips.json"),report.get("actors",[]))
args.report.parent.mkdir(parents=True, exist_ok=True)
args.report.write_text(json.dumps(report, indent=2))
print(json.dumps({"decoded_clips": len(clips), "weapon_exports": len(report["weapons"]), "machines": report["machines"], "errors": len(report["errors"])}))
