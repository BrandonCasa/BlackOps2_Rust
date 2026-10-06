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
except ImportError:
    from prepare_origins import converted_pixels
    from prepare_gameplay_content import external


def convert(source_path, output, additional_images):
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
        raw = path.read_bytes()
        mode = "normal" if index in normal_images else "raw"
        digest = hashlib.sha256(raw + mode.encode()).hexdigest()[:24]
        relative = f"textures/actor-{digest}.png"
        if not (output / relative).exists():
            with Image.open(path) as pixels:
                converted = converted_pixels(np.array(pixels.convert("RGBA")), mode)
            Image.fromarray(converted).save(output / relative)
        image["uri"] = relative
    name = source_path.stem
    document["accessors"], document["bufferViews"] = accessors, views
    document["buffers"] = [{"byteLength": len(binary), "uri": name + ".bin"}]
    document["asset"]["generator"] = "Origins recovered skeletal preparation"
    (output / (name + ".bin")).write_bytes(binary)
    (output / (name + ".gltf")).write_text(json.dumps(document, separators=(",", ":")), encoding="utf-8")
    return name + ".gltf"


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
    # First complete actor assembly: all four original arms, Mauser, and a German
    # zombie body/head. Remaining catalog assets are assembled in subsequent batches.
    names = [f"c_zom_{character}_viewhands" for character in ("dempsey", "nikolai", "richtofen", "takeo")]
    names += ["t6_wpn_zmb_mc96_view", "c_zom_tomb_german_body_1a", "c_zom_tomb_german_head1"]
    definition = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    presentations = definition["content"]["presentations"]
    for name in names:
        relative = convert(models / "model_export" / (name + "_lod0.gltf"), pack / "actors", image_root)
        identity = "actor." + name
        presentations[:] = [p for p in presentations if p["id"] != identity]
        presentations.append({"id": identity, "scene": "actors/" + relative + "#Scene0", "animations": [], "sounds": []})
    (pack / "game.json").write_text(json.dumps(definition, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"prepared_actors": len(names), "animation_clips": 0}))
