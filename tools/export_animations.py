"""Invoke unmodified external exporters on OAT output. No native decoding lives here."""
import argparse
import json
import hashlib
import os
import shutil
import sys
from dataclasses import asdict
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--oat", type=Path, required=True)
parser.add_argument("--base", type=Path, action="append", default=[])
parser.add_argument("--overlay", type=Path, action="append", default=[])
parser.add_argument("--tool", type=Path, required=True)
parser.add_argument("--out", type=Path, required=True)
parser.add_argument("--report", type=Path, required=True)
parser.add_argument("--game-definition", type=Path)
parser.add_argument("--actor-bindings", type=Path, default=Path(__file__).with_name("origins_actor_bindings.json"))
parser.add_argument("--metadata-only", action="store_true", help="Refresh bindings from decoded source without rewriting rigs or clips")
parser.add_argument("--weapon-parts-only", action="store_true", help="Export declared weapon pieces and their clips into an existing asset set")
parser.add_argument("--zone", type=Path, default=Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Steam/steamapps/common/Call of Duty Black Ops II/zone/all/zm_tomb.ff")
args = parser.parse_args()
sys.path.insert(0, str(args.tool.resolve() / "scripts"))
from cod.animations import load_clips
from cod.viewmodel import parse_infostring, role_clips, hidden_bones, definition as weapon_definition
from cod.gltf import read_glb, write_glb
from cod.actors import clip_entry, _write_json, write_clip_glb
from cod.characters import skeleton_spec
from cod.t6.actors import read_asd

sources = [*args.base, args.oat, *args.overlay]
scripted_models = {
    "veh_t6_dlc_zm_robot_0", "veh_t6_dlc_zm_robot_1", "veh_t6_dlc_zm_robot_2",
    "veh_t6_dlc_zm_robot_foot_hatch", "c_zom_mech_body", "c_zom_mech_claw",
}
scripted_models.update(f"p6_anim_zm_barricade_board_{index:02}" for index in range(1, 7))
clips = {}
weapon_files = {}
for source in sources:
    clips.update(load_clips(str(source / "xanim")))
    weapon_files.update({p.name:p for p in (source / "weapons").glob("*") if p.is_file()})

def source_track(clip):
    return {
        "fps": clip.fps, "frames": clip.frames, "looped": clip.looped,
        "notetracks": clip.notetracks,
        "delta_translation": asdict(clip.delta_translation) if clip.delta_translation else None,
        "delta_rotation": asdict(clip.delta_rotation) if clip.delta_rotation else None,
    }

if args.metadata_only:
    path = args.out / "source-model-clips.json"
    records = json.loads(path.read_text())
    for record in records:
        if record["model"] not in scripted_models:
            continue
        for track in record["tracks"]:
            if track["name"] not in clips:
                raise ValueError(f"Required source animation {track['name']} is missing")
            track["source"] = source_track(clips[track["name"]])
    _write_json(str(path), records)
    print(json.dumps({"scenes": len(records), "decoded_clips": len(clips)}))
    sys.exit(0)
source_weapons = {}
hands_bindings = {}
models = args.oat / "model_export"
canonical_weapons = {}
presentations = []
if args.game_definition:
    game = json.loads(args.game_definition.read_text())
    presentations.extend(game["content"]["presentations"])
    for filename in ("weapons.json", "enemies.json"):
        domain = json.loads(args.game_definition.with_name(filename).read_text())
        presentations.extend(domain["presentations"])
        if filename == "weapons.json":
            canonical_weapons = {entry["id"].removeprefix("weapon."): entry for entry in domain["weapons"]}
def model_path(name):
    return next((source / "model_export" / (name + "_lod0.glb") for source in [*reversed(sources), args.out] if (source / "model_export" / (name + "_lod0.glb")).is_file()), None)
report = {"tool": "open-fastfile@088dac3bdee0e6e653871f877ad8ee4326d314a4", "decoded_clips": len(clips), "weapons": [], "machines": [], "errors": []}
(args.out / "model_export").mkdir(parents=True, exist_ok=True)

# Clip identity includes full joint paths and exact exported local bind frames.
# Geometry remains the upstream export; only the external writer creates clips.
model_report = {}
scene_sources = {}
written_clips = set()
def export_clips(original, names, scene=None):
    gltf, _ = read_glb(str(original))
    spec = skeleton_spec(gltf)
    paths = {}
    signature = []
    for name, parent, node in spec:
        path = (*paths[parent], name) if parent else (name,)
        paths[name] = path
        signature.append((path, node.get("translation", [0,0,0]),
                          node.get("rotation", [0,0,0,1]), node.get("scale", [1,1,1])))
    identity = hashlib.sha256(json.dumps(sorted(signature), separators=(",", ":")).encode()).hexdigest()
    scene = scene or "model_export/" + original.name
    scene_sources[scene] = original
    destination = args.out / scene
    destination.parent.mkdir(parents=True, exist_ok=True)
    if original.resolve() != destination.resolve():
        shutil.copyfile(original, destination)
    record = model_report.setdefault(scene, {"model":original.stem.removesuffix("_lod0"), "scene":scene, "tracks":{}})
    tracks = []
    for name in sorted(set(names)):
        if name not in clips:
            raise ValueError(f"Required source animation {name} for {scene} is missing")
        clip = clips[name]
        written = sum(bone.name in paths and bool(bone.rotation or bone.translation) for bone in clip.bones)
        unmatched = sum(bone.name not in paths for bone in clip.bones)
        track = {"name":name, "written":written, "unmatched":unmatched, "animation_path":None}
        # These values belong to the source clip, including scripted root offsets.
        # Do not use root_path(): it rounds keys and subtracts the initial offset.
        if record["model"] in scripted_models:
            track["source"] = source_track(clip)
        if written:
            relative = Path("animations") / identity / (name + ".glb")
            output = args.out / relative
            if relative not in written_clips:
                output.parent.mkdir(parents=True, exist_ok=True)
                problems = write_clip_glb(str(output), [spec], {name:clip})
                if problems:
                    report.setdefault("clip_warnings", []).append({"clip":name,"scene":scene,"problems":problems})
                written_clips.add(relative)
            track["animation_path"] = relative.as_posix() + "#Animation0"
        record["tracks"][name] = track
        tracks.append(track)
    return tracks

piece_walk = None

def export_weapon_parts(info, wanted):
    global piece_walk
    parts = []
    for index in range(1, 9):
        model = info.get(f"attachViewModel{index}")
        if not model:
            continue
        original = model_path(model)
        if original is None:
            raise ValueError(f"Required weapon piece {model} is missing")
        document, _ = read_glb(str(original))
        if not document.get("skins"):
            # The generic exporter omits a single rigid bone. These pieces need
            # that original bone for reload animation and ordinary native skinning.
            from cod import backends
            from cod.iw5.dumpers import xmodel
            from cod import xmodel_glb
            from collections import Counter
            if piece_walk is None:
                piece_walk = backends.for_fastfile(str(args.zone)).walk(str(args.zone))
                piece_walk.check()
            asset = next(asset for asset in piece_walk.by_type("xmodel") if asset.name == model)
            source_model = asset.record
            original = args.out / "model_export" / f"{model}_lod0.glb"
            xmodel_glb.write(str(original), xmodel.lod_data(
                piece_walk, source_model, model, 0, xmodel.surfaces(source_model, 0),
                xmodel.skeleton(piece_walk, source_model), Counter()))
            from strip_native_rig_materials import strip_materials
            document, binary = read_glb(str(original))
            strip_materials(document)
            write_glb(str(original), document, binary)
        export_clips(original, wanted)
        parts.append({
            "slot": index,
            "scene": f"model_export/{model}_lod0.glb#Scene0",
            "tag": info.get(f"attachViewModelTag{index}") or "j_gun",
            "offset": [float(info.get(f"attachViewModelOffset{axis}{index}", 0)) for axis in "XYZ"],
            "rotation": [float(info.get(f"attachViewModelOffset{axis}{index}", 0)) for axis in ("Pitch", "Yaw", "Roll")],
            "ads": index == 8,
        })
    return parts

if args.weapon_parts_only:
    source_weapons = json.loads((args.out / "source-weapons.json").read_text())
    records = {record["scene"]: record for record in json.loads((args.out / "source-model-clips.json").read_text())}
    for source in source_weapons.values():
        wanted = set(source["spec"]["clips"]) | {
            value.lower() for key, value in source["info"].items()
            if value and (key.endswith(("Anim", "AnimLeft", "AnimRight")) or key.startswith("dtp_"))}
        source["parts"] = export_weapon_parts(source["info"], wanted)
        tracks = export_clips(model_path(source["info"]["gunModel"]), wanted, source["scene"].split("#", 1)[0])
        source["clip_bindings"] = {track["name"]: track["animation_path"] for track in tracks if track["animation_path"]}
    for scene, record in model_report.items():
        records[scene] = {**record, "tracks": list(record["tracks"].values())}
    _write_json(str(args.out / "source-weapons.json"), source_weapons)
    _write_json(str(args.out / "source-model-clips.json"), list(records.values()))
    print(json.dumps({"weapons": len(source_weapons), "pieces": sum(len(w["parts"]) for w in source_weapons.values()), "models": len(model_report)}))
    sys.exit(0)

for weapon, source in sorted(weapon_files.items()):
    raw_info = parse_infostring(source.read_text())
    info = raw_info.copy()
    info = {key:(value.lower() if key.endswith(("Anim", "AnimLeft", "AnimRight")) else value) for key,value in info.items()}
    gun = model_path(info.get("gunModel", ""))
    if gun is None:
        model = info.get("gunModel", "")
        declaration = canonical_weapons.get(weapon)
        if not model or (model in ("defaultweapon", "viewmodel_default")
                         and declaration is not None and not declaration.get("presentation")):
            report.setdefault("weapons_without_geometry", []).append({
                "weapon": weapon, "source": str(source), "model": model,
                "reason": "Source gunModel is empty" if not model else "Source placeholder model; gameplay declares no first-person presentation",
                "info": raw_info,
            })
        else:
            report["errors"].append({"weapon":weapon,"error":"Missing exported source gunModel","model":model})
        continue
    wanted = sorted({v for k,v in info.items() if (k.endswith(("Anim", "AnimLeft", "AnimRight")) or k.startswith("dtp_")) and v in clips})
    missing = sorted({v for k,v in info.items() if k.endswith(("Anim", "AnimLeft", "AnimRight")) and v and v not in clips})
    if missing:
        report["errors"].append({"weapon":weapon,"missing_source_clips":missing})
    gltf, buf = read_glb(str(gun))
    joints = {gltf["nodes"][j].get("name") for skin in gltf.get("skins",[]) for j in skin["joints"]}
    roles, absent = role_clips(info, clips)
    spec = weapon_definition("t6",weapon,info,roles,clips,hidden_bones(info,joints),used=wanted)
    source_weapons[weapon] = {"info":raw_info,"spec":spec,"scene":f"model_export/weapon_{weapon}.glb#Scene0"}
    tracks = export_clips(gun, wanted, f"model_export/weapon_{weapon}.glb")
    source_weapons[weapon]["clip_bindings"] = {track["name"]:track["animation_path"] for track in tracks if track["animation_path"]}
    source_weapons[weapon]["parts"] = export_weapon_parts(info, wanted)
    report["weapons"].append({"weapon":weapon,"scene":f"model_export/weapon_{weapon}.glb","clips":spec["clips"],"roles":spec["roles"],"tracks":tracks,"source_model":info.get("gunModel")})
# Existing native_scene bindings use one hands model per character. Preserve that interface.
for character in ("dempsey", "nikolai", "richtofen", "takeo"):
    source = models / f"c_zom_{character}_viewhands_lod0.glb"
    tracks = export_clips(source, [name for name in clips if name.startswith("viewmodel_")])
    hands_bindings[character] = {track["name"]:track["animation_path"] for track in tracks if track["animation_path"]}
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
        tracks = export_clips(original, selected)
        report.setdefault("actors",[]).append({"model":model,"machine":source.stem,"scene":"model_export/"+original.name,"tracks":tracks})

# Source-owned animated gameplay props outside actor state machines.
for model,names in json.loads(args.actor_bindings.read_text()).get("clips",{}).items():
    original=model_path(model)
    if original is None:raise ValueError(f"Required animated source prop {model} is missing")
    export_clips(original, names)
# Committed source prop/model clip assignments; no native parsing or conversion here.
model_bindings=json.loads(Path(__file__).with_name("origins_model_animation_bindings.json").read_text())
for model,names in model_bindings.items():
    original=model_path(model)
    if original is None:
        report["errors"].append({"model":model,"error":"Missing source animated model"});continue
    export_clips(original, names)
# Existing game-owned presentation records supply exact scene-to-clip bindings.
# Never infer gameplay identity from a model name, bone overlap or file order.
if args.game_definition:
    report["presentations"] = []
    for presentation in presentations:
        if not presentation.get("scene") or not presentation.get("animations"):
            continue
        scene = Path(presentation["scene"].split("#", 1)[0])
        source = scene_sources.get(scene.as_posix()) or next(
            (root / "model_export" / scene.name for root in reversed(sources)
             if (root / "model_export" / scene.name).is_file()), None)
        if source is None:
            report["errors"].append({"presentation": presentation["id"], "error": "Missing original exported scene", "scene": scene.as_posix()})
            continue
        names = [ident.rsplit(".", 1)[-1].lower() for ident in presentation["animations"]]
        missing = [name for name in names if name not in clips]
        if missing:
            report["errors"].append({"presentation": presentation["id"], "missing_source_clips": missing})
            continue
        matched = export_clips(source, names, scene.as_posix())
        report["presentations"].append({"id": presentation["id"], "scene": str(scene), "clips": matched})
_write_json(str(args.out / "source-model-clips.json"), [
    {**record, "tracks":list(record["tracks"].values())}
    for _, record in sorted(model_report.items())
])
_write_json(str(args.out / "source-weapons.json"),source_weapons)
_write_json(str(args.out / "source-viewmodel-clips.json"),{name:clip_entry(clip,path=True) for name,clip in clips.items() if name.startswith("viewmodel_")})
_write_json(str(args.out / "source-hands.json"),hands_bindings)
_write_json(str(args.out / "source-actors.json"),json.loads(args.actor_bindings.read_text()))
_write_json(str(args.out / "source-actor-clips.json"),report.get("actors",[]))
args.report.parent.mkdir(parents=True, exist_ok=True)
args.report.write_text(json.dumps(report, indent=2))
print(json.dumps({"decoded_clips": len(clips), "weapon_exports": len(report["weapons"]), "machines": report["machines"], "errors": len(report["errors"])}))
