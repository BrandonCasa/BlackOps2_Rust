"""Convert recovered OAT skinned glTF into self-contained, meter-scale actor assets."""
import base64
import copy
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image

try:
    from .prepare_origins import converted_pixels
    from .prepare_gameplay_content import external
    from .xanim import decode
except ImportError:
    from prepare_origins import converted_pixels
    from prepare_gameplay_content import external
    from xanim import decode


def convert(source_path, output, additional_images, clips, in_place=False, end_aligned_clips=()):
    source = json.loads(source_path.read_text(encoding="utf-8-sig"))
    document = copy.deepcopy(source)
    output = external(output)
    output.mkdir(parents=True, exist_ok=True)
    (output / "textures").mkdir(exist_ok=True)
    buffers = []
    for buffer in source["buffers"]:
        uri = buffer["uri"]
        if uri.startswith("data:application/octet-stream;base64,"):
            raw = base64.b64decode(uri.split(",", 1)[1], validate=True)
        else:
            raw = (source_path.parent / uri).read_bytes()
        if len(raw) != buffer["byteLength"]:
            raise ValueError("actor buffer length mismatch")
        buffers.append(raw)
    positions, weights = set(), set()
    for mesh in source["meshes"]:
        for primitive in mesh["primitives"]:
            positions.add(primitive["attributes"]["POSITION"])
            if "WEIGHTS_0" in primitive["attributes"]:
                weights.add(primitive["attributes"]["WEIGHTS_0"])
    inverse_binds = {s["inverseBindMatrices"] for s in source.get("skins", []) if "inverseBindMatrices" in s}
    types = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}
    widths = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
    binary = bytearray()
    views, accessors = [], []
    for index, accessor in enumerate(source["accessors"]):
        view = source["bufferViews"][accessor["bufferView"]]
        dtype = np.dtype(types[accessor["componentType"]])
        width = widths[accessor["type"]]
        values = np.ndarray((accessor["count"], width), dtype, buffers[view["buffer"]],
                            view.get("byteOffset", 0) + accessor.get("byteOffset", 0),
                            (view.get("byteStride", width * dtype.itemsize), dtype.itemsize)).copy()
        result = {"count": len(values), "type": accessor["type"], "componentType": accessor["componentType"], "bufferView": len(views)}
        if accessor.get("normalized"):
            values = (values.astype(np.float32) / np.iinfo(dtype).max).astype("<f4")
            result["componentType"] = 5126
        if index in positions:
            values *= 0.0254  # OAT already converted source coordinates to Y-up.
        if index in inverse_binds:
            values[:, 12:15] *= 0.0254  # Conjugate inverse binds by uniform unit scale.
        if index in weights:
            totals = values.sum(axis=1, keepdims=True)
            if (values < 0).any() or (totals <= 0).any():
                raise ValueError("invalid recovered skin weights")
            values = (values / totals).astype("<f4")
            result["componentType"] = 5126
        if not np.isfinite(values).all():
            raise ValueError("non-finite actor accessor")
        if index in positions or "min" in accessor:
            result["min"] = values.min(axis=0).tolist()
            result["max"] = values.max(axis=0).tolist()
        while len(binary) % 4:
            binary.append(0)
        raw = values.tobytes()
        views.append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(raw)})
        binary.extend(raw)
        accessors.append(result)
    for node in document["nodes"]:
        if "translation" in node:
            node["translation"] = [float(v) * 0.0254 for v in node["translation"]]
        if "matrix" in node:
            node["matrix"][12:15] = [float(v) * 0.0254 for v in node["matrix"][12:15]]
    normal_images = {document["textures"][m["normalTexture"]["index"]]["source"]
                     for m in document.get("materials", []) if "normalTexture" in m}
    for index, image in enumerate(document.get("images", [])):
        path = (source_path.parent / image["uri"]).resolve()
        # Only the recovered model export tree supplies texture input.
        model_root = source_path.parent.parent.resolve()
        if model_root not in path.parents:
            raise ValueError("actor texture escaped recovered model tree")
        if not path.is_file():
            path = additional_images / path.name
        mode = "normal" if index in normal_images else "raw"
        builtin_normal = not path.is_file() and path.name in ("$identitynormalmap.dds", ",$identitynormalmap.dds")
        raw = b"origins-native-identity-normal" if builtin_normal else path.read_bytes()
        digest = hashlib.sha256(raw + mode.encode()).hexdigest()[:24]
        relative = f"textures/actor-{digest}.png"
        if not (output / relative).exists():
            if builtin_normal:
                converted = np.array([[[128, 128, 255, 255]]], dtype=np.uint8)
            else:
                with Image.open(path) as pixels:
                    converted = converted_pixels(np.array(pixels.convert("RGBA")), mode)
            Image.fromarray(converted).save(output / relative)
        image["uri"] = relative
    name = source_path.stem
    document["accessors"], document["bufferViews"] = accessors, views
    skeleton_joints = {j for skin in document.get("skins", []) for j in skin["joints"]}
    children = {child for j in skeleton_joints for child in document["nodes"][j].get("children", [])}
    roots = skeleton_joints - children
    by_name = {node["name"]: i for i, node in enumerate(document["nodes"]) if "name" in node}
    animations, notifications = [], []

    def animation_accessor(values, kind):
        values = np.asarray(values, dtype="<f4")
        while len(binary) % 4:
            binary.append(0)
        raw = values.tobytes()
        view = len(views)
        views.append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(raw)})
        binary.extend(raw)
        index = len(accessors)
        accessors.append({"bufferView": view, "componentType": 5126, "count": len(values), "type": kind,
                          "min": values.min(axis=0).reshape(-1).tolist(), "max": values.max(axis=0).reshape(-1).tolist()})
        return index

    for clip_path in clips:
        clip = decode(clip_path.read_bytes())
        samplers, channels = [], []
        for bone, tracks in clip["tracks"].items():
            if in_place and bone == "tag_sync":
                continue  # Authoring synchronization marker is not an actor pose joint.
            if bone not in by_name or by_name[bone] not in skeleton_joints:
                continue
            node = by_name[bone]
            for target, (times, source_values) in tracks.items():
                values = source_values.copy()
                if in_place and not clip["channel_presence"][bone][target]:
                    bind = document["nodes"][node].get(target, [0,0,0] if target=="translation" else [0,0,0,1])
                    values = np.repeat(np.asarray(bind).reshape(1,-1),len(values),axis=0)
                elif target == "translation":
                    if node in roots:
                        values = values[:, [0, 2, 1]] * [1, 1, -1]
                    values *= 0.0254
                    if in_place:
                        # Retain bind offsets; native actor channels carry motion about that pose.
                        anchor = values[-1] if bone == "j_mainroot" and clip_path.name in end_aligned_clips else values[0]
                        values += np.asarray(document["nodes"][node].get("translation", [0,0,0])) - anchor
                elif node in roots:
                    # OAT premultiplies root orientations by a -90-degree X basis.
                    # Parent-relative rotations retain their native local basis.
                    x, y, z, w = values.T.copy()
                    c = np.sqrt(0.5)
                    values = np.stack((c * (x - w), c * (y + z), c * (z - y), c * (w + x)), axis=1)
                input_index = animation_accessor(times.reshape(-1, 1), "SCALAR")
                output_index = animation_accessor(values, "VEC3" if target == "translation" else "VEC4")
                channels.append({"sampler": len(samplers), "target": {"node": node, "path": target}})
                samplers.append({"input": input_index, "output": output_index, "interpolation": "LINEAR"})
        if not channels:
            continue  # Camera/socket-only clips have no channels for this mesh's rig.
        animations.append({"name": clip_path.name, "channels": channels, "samplers": samplers})
        notifications.append((clip_path.name, clip["notifications"]))
    if animations:
        document["animations"] = animations
    document["buffers"] = [{"byteLength": len(binary), "uri": name + ".bin"}]
    document["asset"]["generator"] = "Origins recovered skeletal preparation"
    (output / (name + ".bin")).write_bytes(binary)
    (output / (name + ".gltf")).write_text(json.dumps(document, separators=(",", ":")), encoding="utf-8")
    return name + ".gltf", notifications


def prepare(config):
    inspection = external(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    models = Path(settings.get("models", inspection / "game-prepare/models"))
    pack = external(config["game"]["asset_pack"])
    image_root = inspection / "solo-content/actor-images"
    if not (image_root / "~-gviewarm_zom_dempsey_c.dds").is_file():
        from export_instances import export
        oat_source = external(settings.get("oat_source", inspection / "tooling/OAT/surface-7d027e8/source"))
        zone = Path(config["assets"]["installation_root"]) / "zone/all/zm_tomb.ff"
        export(oat_source, zone, inspection / "solo-content/actor-image-export", image_root, include_actor_images=True)
    shared_image = image_root / "~-gmtl_t6_wpn_zmb_raygun2_col1.dds"
    # A separate callback reads shared patch images even though that zone has no world.
    if not shared_image.is_file():
        from export_instances import export
        oat_source = external(settings.get("oat_source", inspection / "tooling/OAT/surface-7d027e8/source"))
        zone = Path(config["assets"]["installation_root"]) / "zone/all/patch_zm.ff"
        export(oat_source, zone, inspection / "solo-content/shared-image-export", image_root, include_actor_images=True)
    from prepare_arsenal import ACTIONS
    recovered = json.loads((inspection / "solo-content/recovered-gameplay.json").read_text())
    model_clips = {}
    shared_models = inspection / "solo-content/shared/model_export"
    source_models = {}
    arm_clips = set()
    missing_models = []
    for weapon in recovered["weapons"]:
        fields = weapon["fields"]
        clips = {fields.get(field) for field in ACTIONS.values()} - {None, ""}
        arm_clips.update(clips)
        for field in ("gunModel", "worldModel"):
            model = fields.get(field)
            if model:
                model_clips.setdefault(model, set()).update(clips if field == "gunModel" else ())
    names = [f"c_zom_{character}_viewhands" for character in ("dempsey", "nikolai", "richtofen", "takeo")]
    names += ["c_zom_tomb_german_body_1a", "c_zom_tomb_german_head1"]
    for model in sorted(model_clips):
        candidates = [root / (model + "_lod0.gltf") for root in (models / "model_export", shared_models)]
        found = next((path for path in candidates if path.is_file()), None)
        if found:
            source_models[model] = found
            names.append(model)
        else:
            missing_models.append(model)
    definition = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    presentations = definition["content"]["presentations"]
    animation_definitions = definition["content"]["animations"]
    animation_source = inspection / "solo-content/source/xanim"
    prepared_clips = 0
    for name in names:
        if "german" in name:
            clip_names = ["ai_zombie_idle_v1_delta", "ai_zombie_walk_v1", "ai_zombie_run_v2", "ai_zombie_attack_v1", "ai_zombie_crawl"]
        elif name.endswith("_viewhands"):
            clip_names = sorted(arm_clips)
        else:
            clip_names = sorted(model_clips[name])
        missing_clips = [clip for clip in clip_names if not (animation_source / clip).is_file()]
        if missing_clips:
            raise ValueError(f"Missing recovered animations for {name}: {missing_clips}")
        clips = [animation_source / clip for clip in clip_names]
        relative, notifications = convert(source_models.get(name, models / "model_export" / (name + "_lod0.gltf")), pack / "actors", image_root, clips)
        identity = "actor." + name
        presentations[:] = [p for p in presentations if p["id"] != identity]
        animation_definitions[:] = [a for a in animation_definitions if a["skeleton"] != identity]
        references = []
        for index, (clip_name, notes) in enumerate(notifications):
            clip_id = "animation." + name + "." + clip_name
            animation_definitions.append({"id": clip_id, "path": "actors/" + relative + f"#Animation{index}",
                "skeleton": identity, "notifications": notes})
            references.append(clip_id)
        prepared_clips += len(notifications)
        presentations.append({"id": identity, "scene": "actors/" + relative + "#Scene0", "animations": references, "sounds": []})
    (pack / "game.json").write_text(json.dumps(definition, indent=2) + "\n", encoding="utf-8")
    report = {"prepared_actors": len(names), "animation_clips": prepared_clips, "missing_models": missing_models}
    (inspection / "solo-content/actor-preparation.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report))
