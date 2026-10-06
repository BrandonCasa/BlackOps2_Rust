# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///
"""Read retained external map exchanges and emit an external standard glTF pack.

Reads geometry/UV/packed corner streams, DDS images, material metadata and map
entities. Does not read captures or shaders and never modifies source inputs.
Run: uv run tools/prepare_origins.py --config config/local_assets.toml
"""
import argparse
import base64
import hashlib
import io
import json
import math
from pathlib import Path
import re
import struct
import subprocess
import tomllib

try:
    from .content_contract import initial_catalog
except ImportError:
    from content_contract import initial_catalog

import numpy as np
from PIL import Image

SCALE = 0.0254
WORKSPACE = Path(__file__).resolve().parents[1]


def material_family(technique, layered=False):
    technique = technique.lower()
    if "tile_blend" in technique:
        return "tile_blend"
    for token, family in (("shadowcaster", "shadow_helper"), ("water", "water"), ("unlit", "unlit"),
                          ("alpha", "masked"), ("blend", "blended"), ("effect", "effect"), ("burn", "effect"), ("smoke", "effect")):
        if token in technique:
            return family
    return "default" if technique == "wpc_default" else "layered_lit" if layered else "lit"


def presentation(source, technique):
    """Readable dispositions from the retained T6 TECH4 (or TECH2) state.

    These are asset metadata, not runtime filename guesses. Tree-canopy coverage
    is represented by a cutout in Bevy rather than sorted transparent cards.
    """
    entries = source["state_entries"]
    index = next((entries[k] for k in (4, 2) if entries[k] != 255), None)
    if index is None or index >= len(source["states"]):
        raise ValueError("missing source presentation state")
    bits = source["states"][index][0]
    src, dst, op = bits & 15, (bits >> 4) & 15, (bits >> 8) & 7
    mode = "opaque"
    if op and dst != 1:
        mode = "add" if dst == 2 else "multiply" if src == 1 and dst == 3 else "premultiplied" if src == 2 and dst == 6 else "blend"
    elif not (bits >> 11) & 1:
        mode = "mask"
    if "treecanopy" in technique:
        mode = "mask"
    return {"alpha": mode, "cutoff": 0.5, "cull": {1: "none", 2: "back", 3: "front"}.get((bits >> 14) & 3, "back"),
            "emissive": "emissive" in technique or "unlit" in technique and mode == "add",
            "fog": "nofog" not in technique, "visible": "shadowcaster" not in technique and technique != "wpc_default" and "smoke_pillar_cheap" not in technique,
            "solid": mode in ("opaque", "mask") and not any(token in technique for token in ("unlit", "water", "smoke")),
            "wettable": "lit_sm" in technique and mode == "opaque"}


def apply_presentation(material, state):
    # Explicit scene-light providers, distinct from radios, glyphs and other glow.
    state["lamp"] = material["name"] in {"mc/mtl_p6_zm_tm_gas_lamp_glass_on", "mc/mtl_p6_zm_tm_lantern_glass_on_01",
        "mc/mtl_p6_zm_tm_hanging_light_fixture_on", "mc/mtl_p6_zm_tm_portable_lantern_on"}
    material["extras"]["surface"]["presentation"] = state
    material["doubleSided"] = state["cull"] == "none"
    material["alphaMode"] = "MASK" if state["alpha"] == "mask" else "OPAQUE" if state["alpha"] == "opaque" else "BLEND"
    if state["alpha"] == "mask":
        material["alphaCutoff"] = state["cutoff"]


def world_layer_layout(technique):
    # Authored operations refer to layer numbers, not texture-table order.
    return ["base"] + [dict(b="blend", m="multiply", a="add", t="blend")[m.group(1)]
                         for m in re.finditer(r"_([bmat])[123]c[123]", technique)]


def secondary_attributes(root, manifest, uv, ranges):
    metadata = json.loads((root / "secondary-corners.json").read_text())
    if metadata["geometry_sha256"] != manifest["geometry_sha256"]:
        raise ValueError("secondary stream geometry mismatch")
    raw = dependency(root, metadata["stream"])
    local = np.frombuffer(dependency(root, metadata["corner_local_indices"]), "<u2")
    layer_uv = np.column_stack((uv, uv)).astype(np.float32)
    layer3 = uv.copy()
    rotation1 = np.tile([1, 0, 0, 1], (len(uv), 1)).astype(np.float32)
    rotation2 = rotation1.copy()
    # Half UVs followed by normalized signed 2x2 normal-basis rotations.
    layouts = {0: (0, 0, 0), 1: (4, 1, 0), 2: (8, 1, 1), 3: (8, 2, 0),
               4: (12, 2, 1), 5: (16, 2, 2), 7: (16, 3, 1)}
    for record, surface in zip(metadata["surfaces"], ranges, strict=True):
        first, count = record["first_corner"], record["corners"]
        if (first, count) != surface[:2]:
            raise ValueError("secondary corner correspondence mismatch")
        stride, uvs, rotations = layouts[record["source_format"]]
        if not stride:
            continue
        addresses = record["byte_offset"] + local[first:first+count].astype(np.int64) * stride
        if (addresses + stride > len(raw)).any():
            raise ValueError("secondary corner outside buffer")
        values = np.frombuffer(raw, np.uint8)[addresses[:, None] + np.arange(stride)]
        decoded = values[:, :uvs*4].copy().view("<f2").astype(np.float32)
        if not np.isfinite(decoded).all():
            raise ValueError("non-finite layer UV")
        layer_uv[first:first+count, :2] = decoded[:, :2]
        if uvs >= 2:
            layer_uv[first:first+count, 2:] = decoded[:, 2:4]
        if uvs == 3:
            layer3[first:first+count] = decoded[:, 4:6]
        if rotations:
            rotation1[first:first+count] = values[:, uvs*4:uvs*4+4] / 127.5 - 1
        if rotations == 2:
            rotation2[first:first+count] = values[:, uvs*4+4:uvs*4+8] / 127.5 - 1
    return layer_uv, layer3, rotation1, rotation2


def tile_blend_color(core, surface, control, tiles=4):
    """Bake the shared model tile/macro family to ordinary PBR, retaining raw inputs.

    Red weights the coated surface; green weights exposed core. Their magnitude
    retains macro occlusion. Tiling is a tunable
    reconstruction, not an attempt to reproduce the native shader constants.
    """
    size = 2048
    def tiled(pixels):
        tile = np.asarray(Image.fromarray(pixels).resize((size // tiles, size // tiles), Image.Resampling.LANCZOS))
        return np.tile(tile, (tiles, tiles, 1))[..., :3].astype(np.float32) / 255
    mask = np.asarray(Image.fromarray(control).resize((size, size), Image.Resampling.BILINEAR)).astype(np.float32) / 255
    # Blend diffuse reflectance in linear space, rather than blending sRGB bytes.
    linear = tiled(surface) ** 2.2 * mask[..., 0:1] + tiled(core) ** 2.2 * mask[..., 1:2]
    rgb = (np.maximum(linear, 0) ** (1 / 2.2) * 255).clip(0, 255).astype(np.uint8)
    return np.concatenate((rgb, np.full((size, size, 1), 255, np.uint8)), axis=2)


def unit_vectors(words):
    """T6 signed ten-bit direction, normalized for standard glTF."""
    values = np.stack([(words >> shift & 1023).astype(np.int32) for shift in (0, 10, 20)], axis=1)
    values = values - 2 * (values & 512)
    values = values[:, [0, 2, 1]].astype(np.float32) * [1, 1, -1]
    lengths = np.linalg.norm(values, axis=1, keepdims=True)
    if not np.isfinite(values).all() or (lengths < 1e-8).any():
        raise ValueError("invalid source direction")
    return values / lengths


def tangents(words, signs):
    if not np.isfinite(signs).all() or (signs == 0).any():
        raise ValueError("invalid source tangent handedness")
    # Original B=-sign(h)*cross(N,T); converted normal maps also flip Y.
    return np.column_stack((unit_vectors(words), np.sign(signs))).astype(np.float32)


def converted_pixels(pixels, mode):
    pixels = pixels.copy()
    if mode == "normal":
        xy = pixels[..., :2].astype(np.float32) / 127.5 - 1
        pixels[..., 1] = 255 - pixels[..., 1]
        pixels[..., 2] = (np.sqrt(np.maximum(0, 1 - (xy * xy).sum(axis=2))) * 127.5 + 127.5).astype(np.uint8)
        pixels[..., 3] = 255
    elif mode == "roughness":
        pixels[..., 1] = 255 - pixels[..., 3]
        pixels[..., 2] = 0
        pixels[..., 3] = 255
    return pixels


def cube_faces(raw):
    """Decode the six top-level DDS cube faces; keep native mip data external."""
    if raw[:4] != b"DDS " or not struct.unpack_from("<I", raw, 112)[0] & 0x200:
        raise ValueError("expected DDS cubemap")
    height, width = struct.unpack_from("<II", raw, 12)
    mips = max(1, struct.unpack_from("<I", raw, 28)[0])
    fourcc = raw[84:88]
    header_size = 148 if fourcc == b"DX10" else 128
    if fourcc == b"DX10":
        format_id = struct.unpack_from("<I", raw, 128)[0]
        block = 8 if format_id in (71, 72) else 16 if format_id in (74, 75, 77, 78, 83, 84) else None
        if block is None and format_id not in (28, 29):
            raise ValueError("unsupported cube format")
    else:
        block = 8 if fourcc == b"DXT1" else 16 if fourcc in (b"DXT3", b"DXT5", b"ATI2", b"BC5U") else None
        if block is None and struct.unpack_from("<I", raw, 88)[0] != 32:
            raise ValueError("unsupported cube pixel format")
    size = lambda w, h: max(1, (w + 3)//4) * max(1, (h + 3)//4) * block if block else w*h*4
    face_size = sum(size(max(1, width >> mip), max(1, height >> mip)) for mip in range(mips))
    if len(raw) != header_size + face_size * 6:
        raise ValueError("cube face payload mismatch")
    header = bytearray(raw[:header_size])
    struct.pack_into("<I", header, 28, 1)
    struct.pack_into("<I", header, 112, 0)
    if fourcc == b"DX10":
        struct.pack_into("<I", header, 136, 0)
        struct.pack_into("<I", header, 140, 1)
    return [np.array(Image.open(io.BytesIO(bytes(header) + raw[header_size+i*face_size:header_size+i*face_size+size(width, height)])).convert("RGBA")) for i in range(6)]


def external(path):
    path = Path(path).resolve()
    if path == WORKSPACE or WORKSPACE in path.parents:
        raise ValueError("licensed inputs and generated assets must stay outside the repository")
    return path


def dependency(root, record):
    path = (root / record["file"]).resolve()
    if root not in path.parents:
        raise ValueError("dependency escapes input root")
    data = path.read_bytes()
    if len(data) > 128 * 1024 * 1024:
        raise ValueError("input exceeds preparation limit")
    if record.get("sha256") and hashlib.sha256(data).hexdigest() != record["sha256"]:
        raise ValueError("input checksum mismatch")
    return data


def coordinates(vectors):
    return np.asarray(vectors, dtype=np.float32)[..., [0, 2, 1]] * [SCALE, SCALE, -SCALE]


def geometry(data):
    if data[:8] != b"BO2DGS\0\0" or struct.unpack_from("<II", data, 8) != (1, 0):
        raise ValueError("unsupported geometry exchange")
    positions, indices, surfaces, length = struct.unpack_from("<4Q", data, 16)
    if length != len(data) or length != 64 + positions * 12 + indices * 4 + surfaces * 48:
        raise ValueError("geometry length mismatch")
    xyz = np.frombuffer(data, "<f4", positions * 3, 64).reshape(-1, 3)
    refs = np.frombuffer(data, "<u4", indices, 64 + positions * 12)
    if indices % 3 or refs.max(initial=0) >= positions or not np.isfinite(xyz).all():
        raise ValueError("invalid geometry references or positions")
    records = [struct.unpack_from("<QQII6f", data, 64 + positions * 12 + indices * 4 + i * 48)
               for i in range(surfaces)]
    cursor = 0
    for first, count, *_ in records:
        if first != cursor or count % 3:
            raise ValueError("surface partition mismatch")
        cursor += count
    if cursor != indices:
        raise ValueError("incomplete surface partition")
    return coordinates(xyz[refs]), records


def entities(path):
    text = path.read_text(encoding="utf-8-sig")
    return [dict(re.findall(r'"([^"\n]+)"\s+"([^"\n]*)"', block))
            for block in re.findall(r"\{([^{}]*)\}", text)]


def gameplay_definition(records):
    starts = [e for e in records if e.get("targetname") == "initial_spawn_points"]
    if not starts:
        raise ValueError("map entities have no initial spawn points")
    source = starts[0]
    spawn = coordinates([float(x) for x in source["origin"].split()]).tolist()
    angles = [float(x) for x in source.get("angles", "0 0 0").split()]
    nodes = [coordinates([float(x) for x in e["origin"].split()]).tolist()
             for e in records if e.get("classname") == "node_pathnode" and "origin" in e]
    local = [p for p in nodes if np.linalg.norm(np.asarray(p) - spawn) < 45]
    spawners = [e for e in records if e.get("targetname") == "zone_start_spawners"
                and e.get("script_noteworthy") in ("spawn_location", "riser_location")]
    points = []
    for entry in spawners:
        point = coordinates([float(x) for x in entry["origin"].split()])
        # Use the nearest walkable map path node instead of implementing climb animations.
        if local:
            point = np.asarray(min(local, key=lambda p: np.linalg.norm(np.asarray(p) - point)))
        if not any(np.linalg.norm(np.asarray(p) - point) < 0.8 for p in points):
            points.append(point.tolist())
    if not points:
        raise ValueError("map entities have no starting-zone zombie entry points")
    return {"version": 2, "content": initial_catalog(), "scene": "map.gltf#Scene0", "collision_scene": "map.gltf#Scene1",
            "spawn": spawn, "yaw": math.radians(90 - angles[1]), "spawn_points": points,
            "waypoints": local, "survey_views": [p for p in nodes[::max(1, len(nodes)//8)]][:8],
            "source_surfaces": 0, "render_meshes": 0, "collision_meshes": 0,
            "static_model_instances": 0}


class Gltf:
    def __init__(self):
        self.binary = bytearray()
        self.collidable = {}
        self.document = {"asset": {"version": "2.0", "generator": "Origins asset preparation"},
                         "scene": 0, "scenes": [{"nodes": []}, {"nodes": []}], "nodes": [],
                         "meshes": [], "materials": [], "textures": [], "images": [],
                         "samplers": [{"magFilter": 9729, "minFilter": 9987}],
                         "accessors": [], "bufferViews": [], "buffers": []}

    def accessor(self, values, kind, indices=False):
        values = np.ascontiguousarray(values, dtype="<u4" if indices else "<f4")
        while len(self.binary) % 4:
            self.binary.append(0)
        view = len(self.document["bufferViews"])
        self.document["bufferViews"].append({"buffer": 0, "byteOffset": len(self.binary),
                                            "byteLength": values.nbytes, "target": 34963 if indices else 34962})
        self.binary.extend(values.tobytes())
        index = len(self.document["accessors"])
        self.document["accessors"].append({"bufferView": view, "componentType": 5125 if indices else 5126,
                                           "count": len(values), "type": kind,
                                           "min": values.min(axis=0).tolist(),
                                           "max": values.max(axis=0).tolist()})
        return index

    def texture(self, uri):
        image = len(self.document["images"])
        self.document["images"].append({"uri": uri})
        self.document["textures"].append({"source": image, "sampler": 0})
        return {"index": len(self.document["textures"]) - 1}

    def mesh(self, arrays, material, name, collider=True, lightmap=None, visible=True, lighting=None):
        attributes = {key: self.accessor(data, kind) for key, data, kind in arrays}
        mesh = len(self.document["meshes"])
        points = next(data for key, data, _ in arrays if key == "POSITION").reshape(-1, 3, 3)
        self.collidable[mesh] = bool((np.linalg.norm(np.cross(points[:, 1] - points[:, 0], points[:, 2] - points[:, 0]), axis=1) > 1e-8).any())
        indexes = self.accessor(np.arange(len(points) * 3).reshape(-1, 1), "SCALAR", indices=True)
        self.document["meshes"].append({"name": name, "primitives": [
            {"attributes": attributes, "indices": indexes, "mode": 4, "material": material}]})
        if lighting:
            self.document["meshes"][-1]["extras"] = {"lighting": lighting}
        if visible:
            extras = {"lighting": lighting} if lighting else {}
            if lightmap:
                extras["lightmap"] = lightmap
            self.node(mesh, name, 0, extras=extras)
        if collider and self.collidable[mesh]:
            self.node(mesh, "collision_" + name, 1)
        return mesh

    def node(self, mesh, name, scene, **properties):
        node = len(self.document["nodes"])
        self.document["nodes"].append({"mesh": mesh, "name": name, **properties})
        self.document["scenes"][scene]["nodes"].append(node)

    def save(self, output):
        self.document["buffers"] = [{"uri": "map.bin", "byteLength": len(self.binary)}]
        (output / "map.bin").write_bytes(self.binary)
        (output / "map.gltf").write_text(json.dumps(self.document, separators=(",", ":")), encoding="utf-8")


def model_accessor(document, index):
    accessor = document["accessors"][index]
    view = document["bufferViews"][accessor["bufferView"]]
    uri = document["buffers"][view["buffer"]]["uri"]
    if not uri.startswith("data:application/octet-stream;base64,"):
        raise ValueError("expected embedded OAT model buffer")
    buffers = document.setdefault("_decoded", {})
    if view["buffer"] not in buffers:
        buffers[view["buffer"]] = base64.b64decode(uri.split(",", 1)[1], validate=True)
    raw = buffers[view["buffer"]]
    dtype = {5120: "i1", 5121: "u1", 5122: "<i2", 5123: "<u2", 5125: "<u4", 5126: "<f4"}[accessor["componentType"]]
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[accessor["type"]]
    offset = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    stride = view.get("byteStride", np.dtype(dtype).itemsize * width)
    values = np.ndarray((accessor["count"], width), dtype, raw, offset,
                        (stride, np.dtype(dtype).itemsize)).copy()
    if accessor.get("normalized"):
        values = np.maximum(values.astype(np.float32) / np.iinfo(dtype).max, -1)
    if not np.isfinite(values).all():
        raise ValueError("non-finite model attribute")
    return values


def static_models(gltf, instances_path, model_root, output, world_root):
    instances = json.loads(external(instances_path).read_text())
    surface_data = json.loads(external(instances_path.parent / "model-surfaces.json").read_text())
    providers = {}
    textures = {}
    missing = set()
    basis = np.array([[1, 0, 0], [0, 0, 1], [0, -1, 0]], dtype=float)
    for item in instances:
        name = item["model"]
        if name not in providers:
            path = external(model_root / "model_export" / (name + "_lod0.gltf"))
            source = json.loads(path.read_text())
            materials = {}
            for source_index, mat in enumerate(source.get("materials", [])):
                original = next((s for s in surface_data[name] if s["material"] == mat["name"]), None)
                if original is None:
                    continue  # OAT includes unused material slots from other LODs.
                material = {"name": mat["name"], "extras": {"surface": {"identity": mat["name"],
                    "family": material_family(original["family"], sum(t["semantic"] == 2 for t in original["textures"]) > 1), "channels": []}}, "doubleSided": True, "pbrMetallicRoughness": {
                    "metallicFactor": 0, "roughnessFactor": 0.85}}
                layers = {}
                color_pixels = []
                state = presentation(original, original["family"])
                apply_presentation(material, state)
                for binding in original["textures"]:
                    role = {2: "color", 5: "normal", 8: "specular_gloss"}.get(binding["semantic"], "auxiliary")
                    if role == "color" and "phong_emissive" in original["family"] and not color_pixels:
                        role = "emissive"
                    layer = layers.get(role, 0)
                    layers[role] = layer + 1
                    image_path = external(model_root / "images" / (binding["image"] + ".dds"))
                    if not (model_root / "images").resolve() in image_path.parents:
                        raise ValueError("escaped model image")
                    approximation = None
                    if image_path.name in ("$identitynormalmap.dds", ",$identitynormalmap.dds") and not image_path.is_file():
                        pixels = np.array(Image.open(world_root / "builtin-identity-normal.dds").convert("RGBA"))
                    elif image_path.name == ",sw_radiant_default.dds" and not image_path.is_file():
                        pixels = np.array([[[128, 128, 128, 255]]], dtype=np.uint8)
                        approximation = "neutral placeholder for unresolved editor-default image"
                    elif not image_path.is_file():
                        missing.add(image_path.name)
                        continue
                    else:
                        pixels = np.array(Image.open(image_path).convert("RGBA"))
                    if role in ("color", "emissive"):
                        color_pixels.append(pixels)
                    modes = ["raw"] + (["normal"] if role == "normal" else ["roughness"] if role == "specular_gloss" else [])
                    for mode in modes:
                        key = (image_path, mode)
                        if key not in textures:
                            converted = converted_pixels(pixels, mode)
                            filename = f"model-{len(textures)}.png"
                            Image.fromarray(converted).save(output / "textures" / filename)
                            textures[key] = gltf.texture("textures/" + filename)
                    raw_texture = textures[(image_path, "raw")]
                    material["extras"]["surface"]["channels"].append({"role": role, "layer": layer,
                        "texture": raw_texture["index"], "channels": "rgb:specular,a:gloss" if role == "specular_gloss" else "rgba"})
                    if approximation:
                        material["extras"]["surface"]["channels"][-1]["approximation"] = approximation
                    if layer == 0:
                        if role == "normal":
                            material["normalTexture"] = textures[(image_path, "normal")]
                        elif role == "color":
                            material["pbrMetallicRoughness"]["baseColorTexture"] = raw_texture
                        elif role == "specular_gloss":
                            material["pbrMetallicRoughness"]["metallicRoughnessTexture"] = textures[(image_path, "roughness")]
                            material["pbrMetallicRoughness"]["roughnessFactor"] = 1
                        elif role == "emissive":
                            material["emissiveTexture"] = raw_texture
                            material["emissiveFactor"] = [1, 1, 1]
                if "tile_blend" in original["family"]:
                    if len(color_pixels) != 3:
                        raise ValueError("unsupported tile/macro material layout")
                    key = (tuple(t["image"] for t in original["textures"] if t["semantic"] == 2), "tile_blend")
                    if key not in textures:
                        filename = f"model-{len(textures)}.png"
                        Image.fromarray(tile_blend_color(*color_pixels)).save(output / "textures" / filename)
                        textures[key] = gltf.texture("textures/" + filename)
                    material["pbrMetallicRoughness"]["baseColorTexture"] = textures[key]
                    material["extras"]["surface"]["preparation"] = {"kind": "tile_macro_bake", "tiles": 4}
                materials[source_index] = len(gltf.document["materials"])
                gltf.document["materials"].append(material)
            provider = []
            surface_index = 0
            vertex_base = 0
            for mesh in source["meshes"]:
                for primitive in mesh["primitives"]:
                    indexes = model_accessor(source, primitive["indices"]).flatten()
                    retained = surface_data[name][surface_index]
                    vertices = np.asarray(retained["vertices"], dtype=np.float64)
                    if retained["material"] != source["materials"][primitive["material"]]["name"]:
                        raise ValueError("model surface/material correspondence mismatch")
                    local = indexes.astype(np.int64) - vertex_base
                    if sorted(local.tolist()) != sorted(retained["indices"]):
                        raise ValueError("model surface corner correspondence mismatch")
                    arrays = []
                    for attribute, accessor in primitive["attributes"].items():
                        if attribute.startswith(("JOINTS_", "WEIGHTS_")):
                            continue  # Static providers retain the exported rest pose.
                        kind = source["accessors"][accessor]["type"]
                        values = model_accessor(source, primitive["attributes"][attribute])[indexes]
                        if attribute == "POSITION":
                            values *= SCALE  # OAT already converted to Y-up, in source units.
                        arrays.append((attribute, values, kind))
                    if not any(key == "COLOR_0" for key, _, _ in arrays):
                        words = vertices[:, 2].astype(np.uint32)[local]
                        colors = np.stack([(words >> shift & 255) / 255 for shift in (0, 8, 16, 24)], axis=1)
                        arrays.append(("COLOR_0", colors, "VEC4"))
                    arrays.append(("TANGENT", tangents(vertices[:, 0].astype(np.uint32), vertices[:, 1])[local], "VEC4"))
                    provider.append(gltf.mesh(arrays, materials[primitive["material"]],
                                              "static_provider", collider=False, visible=False))
                    state = gltf.document["materials"][materials[primitive["material"]]]["extras"]["surface"]["presentation"]
                    gltf.collidable[provider[-1]] &= state["solid"]
                    # Expanded glTF corners still need their original LOD vertex identity
                    # to address each instance's separately retained baked color stream.
                    gltf.document["meshes"][provider[-1]]["extras"] = {
                        "lighting_vertices": gltf.accessor(indexes.reshape(-1, 1), "SCALAR", indices=True)}
                    surface_index += 1
                    vertex_base += len(vertices)
            if surface_index != len(surface_data[name]):
                raise ValueError("model surface count mismatch")
            providers[name] = provider
        matrix = np.eye(4)
        matrix[:3, :3] = basis @ np.asarray(item["axis"]).reshape(3, 3).T @ basis.T * item["scale"]
        matrix[:3, 3] = coordinates(item["origin"])
        if not np.isfinite(matrix).all() or item["scale"] <= 0:
            raise ValueError("invalid static model placement")
        lighting = {"probe": None if item["probe"] == 255 else item["probe"], "lighting_id": item["lighting_id"]}
        color_lods = []
        for source_colors in item["lightmap_colors"]:
            if source_colors:
                words = np.asarray(source_colors, dtype=np.uint32)
                colors = np.stack([(words >> shift & 255) / 255 for shift in (0, 8, 16, 24)], axis=1)
                color_lods.append(gltf.accessor(colors, "VEC4"))
            else:
                color_lods.append(None)
        if any(value is not None for value in color_lods):
            lighting["color_lods"] = color_lods
        for mesh in providers[name]:
            gltf.node(mesh, "static_instance", 0, matrix=matrix.T.flatten().tolist(), extras={"lighting": lighting})
            # Foliage and hanging cloth are visible decorations, not solid walls.
            if gltf.collidable[mesh] and not any(token in name.lower() for token in ("grass", "foliage", "vine", "cloth", "flag")):
                gltf.node(mesh, "collision_static", 1, matrix=matrix.T.flatten().tolist())
    if missing:
        raise ValueError(f"{len(missing)} required model textures unavailable: {sorted(missing)[:5]}")
    return len(instances), len(providers), len(textures)


def prepare(scene_path, entity_path, output, instances_path, model_root):
    root = external(scene_path).parent
    output = external(output)
    manifest = json.loads(scene_path.read_text())
    inventory = json.loads((root / "inventory.private.json").read_text())
    builtins = json.loads((root / "builtin-definitions.private.json").read_text())
    if len(inventory["materials"]) != len(manifest["materials"]):
        raise ValueError("material inventory mismatch")
    for source, retained in zip(manifest["materials"], inventory["materials"], strict=True):
        if source["source_technique_set"] != retained["techset"]:
            raise ValueError("material identity correspondence mismatch")
        bindings = {(b["semantic"], b["name_hash"]): b for b in source["bindings"]}
        ordered = []
        for binding in retained["textures"]:
            if binding["image"] is None:
                raise ValueError("material texture has no source identity")
            image = inventory["images"][binding["image"]]
            candidate = bindings.pop((binding["semantic"], binding["hash"]), None)
            if candidate is None:
                raise ValueError("missing retained material texture")
            record = manifest["images"][candidate["image"]]
            if "file" in image:
                matched = record["file"] == image["file"]
            else:
                definition = next((v for v in builtins.values() if v["source_sha256"] == hashlib.sha256(image["name"].encode()).hexdigest()), None)
                matched = definition is not None and definition["image_index"] == candidate["image"] and definition["dds_sha256"] == record["sha256"]
            if not matched:
                raise ValueError("material texture correspondence mismatch")
            ordered.append(candidate)
        if bindings:
            raise ValueError("unexpected retained material binding")
        source["bindings"] = ordered
    positions, surfaces = geometry(dependency(root, {"file": manifest["geometry"],
                                                     "sha256": manifest["geometry_sha256"]}))
    uv_data = dependency(root, {"file": manifest["corner_uv"], "sha256": manifest["corner_uv_sha256"]})
    uv = np.frombuffer(uv_data, "<f4").reshape(-1, 2)
    packed = dependency(root, manifest["corner_vertices"])
    if len(packed) != len(positions) * 36 or len(uv) != len(positions):
        raise ValueError("corner stream mismatch")
    layout = np.dtype({"names": ["p", "color", "uv", "normal", "tangent", "lm"],
                       "formats": [("<f4", 4), ("u1", 4), ("<f2", 2), "<u4", "<u4", ("<u2", 2)],
                       "offsets": [0, 16, 20, 24, 28, 32], "itemsize": 36})
    corners = np.frombuffer(packed, layout)
    normals = unit_vectors(corners["normal"])
    tangent = tangents(corners["tangent"], corners["p"][:, 3])
    color = corners["color"].astype(np.float32) / 255
    lm_uv = corners["lm"].astype(np.float32) / 65535
    layer_uv, layer3_uv, rotation1, rotation2 = secondary_attributes(root, manifest, uv, surfaces)
    definition = gameplay_definition(entities(external(entity_path)))
    previous_path = output / "game.json"
    if previous_path.is_file():
        previous = json.loads(previous_path.read_text(encoding="utf-8-sig"))
        if previous.get("version") == 2:
            definition["content"] = previous["content"]
    output.mkdir(parents=True, exist_ok=True)
    texture_dir = output / "textures"
    texture_dir.mkdir(exist_ok=True)
    gltf = Gltf()
    cached = {}

    def texture(binding, normal=False, roughness=False):
        image_index = binding["image"]
        key = (image_index, normal, roughness)
        if key not in cached:
            record = manifest["images"][image_index]
            image = Image.open(io.BytesIO(dependency(root, record))).convert("RGBA")
            pixels = converted_pixels(np.asarray(image), "normal" if normal else "roughness" if roughness else "raw")
            filename = f"image-{image_index}{'-normal' if normal else '-roughness' if roughness else ''}.png"
            Image.fromarray(pixels).save(texture_dir / filename)
            cached[key] = gltf.texture("textures/" + filename)
        return cached[key]

    families = []
    for index, source in enumerate(manifest["materials"]):
        family = source["source_technique_set"].lower()
        families.append(family)
        bindings = source["bindings"]
        pbr = {"metallicFactor": 0, "roughnessFactor": 0.8}
        colors = [b for b in bindings if b["semantic"] == 2]
        normals_bindings = [b for b in bindings if b["semantic"] == 5]
        gloss = [b for b in bindings if b["semantic"] == 8]
        identity = inventory["materials"][index]["name"]
        channels = []
        layers = {}
        source_layers = {"color": {2695565377: 0, 3054311504: 1, 3054311507: 2, 3054311506: 3},
                         "normal": {1507003663: 0, 2486480606: 1, 2486480605: 2},
                         "specular_gloss": {887934131: 0, 3532022562: 1, 3532022561: 2}}
        for binding in bindings:
            role = {2: "color", 5: "normal", 8: "specular_gloss"}.get(binding["semantic"], "auxiliary")
            layer = source_layers.get(role, {}).get(binding["name_hash"], layers.get(role, 0))
            layers[role] = layer + 1
            channels.append({"role": role, "layer": layer, "texture": texture(binding)["index"],
                             "channels": "rgb:specular,a:gloss" if role == "specular_gloss" else "rgba"})
        material = {"name": identity, "extras": {"surface": {"identity": identity, "family": material_family(family, len(colors) > 1), "channels": channels}},
                    "doubleSided": True, "pbrMetallicRoughness": pbr}
        channels.sort(key=lambda c: (c["role"], c["layer"]))
        apply_presentation(material, presentation(inventory["materials"][index], family))
        operations = world_layer_layout(family)
        if len(operations) > 1:
            constants = {c["hash"]: c["value"] for c in inventory["materials"][index]["constants"]}
            material["extras"]["surface"]["layers"] = {"operations": operations,
                "reveal": [constants.get(2294217777, [0.25, 1, 0, 0])[:2], constants.get(2294217778, [0.25, 1, 0, 0])[:2]],
                "controls": "vertex_color_gb", "uvs": "secondary_half", "normal_basis": "secondary_signed_rgba"}
        if colors:
            pbr["baseColorTexture"] = texture(colors[0])
        if normals_bindings:
            material["normalTexture"] = texture(normals_bindings[0], normal=True)
        if gloss:
            pbr["metallicRoughnessTexture"] = texture(gloss[0], roughness=True)
            pbr["roughnessFactor"] = 1
        if "unlit" in family:
            material["extensions"] = {"KHR_materials_unlit": {}}
            gltf.document.setdefault("extensionsUsed", ["KHR_materials_unlit"])
        if "water" in family:
            material["alphaMode"] = "BLEND"
            material["extras"]["surface"]["presentation"].update(alpha="blend", wettable=False)
            pbr.update(baseColorFactor=[0.12, 0.2, 0.23, 0.65], roughnessFactor=0.15)
        if family == "wpc_default":
            pbr.update(baseColorFactor=[0.3,0.3,0.3,1], metallicFactor=0, roughnessFactor=1)
        gltf.document["materials"].append(material)
    lighting = {"lightmaps": [], "probes": []}
    for slot in manifest["lightmaps"]:
        lighting["lightmaps"].append({key: gltf.document["images"][gltf.document["textures"][texture({"image": image})["index"]]["source"]]["uri"]
                                      for key, image in slot.items() if image is not None})
    for index, probe in enumerate(manifest["probes"]):
        faces = []
        for face, pixels in zip(("px", "nx", "py", "ny", "pz", "nz"), cube_faces(dependency(root, manifest["images"][probe["image"]])), strict=True):
            uri = f"textures/probe-{index}-{face}.png"
            Image.fromarray(pixels).save(output / uri)
            gltf.texture(uri)
            faces.append(uri)
        lighting["probes"].append({"faces": faces, "position": coordinates(probe["origin"]).tolist(),
                                   "mip_bias": probe["mip_bias"], "sh": probe["sh"], "sh_basis": "source_z_up"})
    gltf.document["extras"] = {"lighting": lighting}
    groups = {}
    shadow_only = 0
    for record, source in zip(surfaces, manifest["surfaces"], strict=True):
        first, count = record[:2]
        material = source["material"]
        family = families[material]
        if "shadowcaster" in family:
            shadow_only += 1
        center = positions[first:first+count].mean(axis=0)
        cell = tuple(np.floor(center / 16).astype(int))
        lightmap = source.get("lightmap")
        probe = source.get("probe")
        if lightmap in (31, 255):
            lightmap = None
        if probe == 255:
            probe = None
        key = (material, lightmap, probe, cell)
        groups.setdefault(key, []).extend(range(first, first+count))
    for (material, lightmap, probe, cell), indexes in groups.items():
        # T6's world triangles are clockwise; glTF/Bevy front faces are CCW.
        # Swap corners together (UVs/controls/bases included), preserving normals.
        selection = np.asarray(indexes).reshape(-1, 3)[:, [0, 2, 1]].reshape(-1)
        family = families[material]
        lightmap_uri = None
        if lightmap is not None and lightmap < len(manifest["lightmaps"]):
            slot = manifest["lightmaps"][lightmap]
            image = slot.get("primary") if slot.get("primary") is not None else slot.get("secondary")
            if image is not None:
                texture({"image": image})
                lightmap_uri = f"textures/image-{image}.png"
        gltf.mesh([("POSITION", positions[selection], "VEC3"), ("NORMAL", normals[selection], "VEC3"),
                   ("TANGENT", tangent[selection], "VEC4"),
                   ("TEXCOORD_0", uv[selection], "VEC2"), ("TEXCOORD_1", lm_uv[selection], "VEC2"),
                   ("COLOR_0", color[selection], "VEC4"),
                   ("_ORIGINS_LAYER_UV", layer_uv[selection], "VEC4"),
                   ("_ORIGINS_LAYER3_UV", layer3_uv[selection], "VEC2"),
                   ("_ORIGINS_ROTATION1", rotation1[selection], "VEC4"),
                   ("_ORIGINS_ROTATION2", rotation2[selection], "VEC4")], material,
                  f"world_{material}_{cell}", collider="water" not in family and "unlit" not in family,
                  lightmap=lightmap_uri, lighting={"lightmap": lightmap, "probe": probe}, visible="shadowcaster" not in family)
    instance_count, model_count, model_textures = static_models(gltf, instances_path, external(model_root), output, root)
    gltf.save(output)
    definition.update(source_surfaces=len(surfaces), render_meshes=len(gltf.document["scenes"][0]["nodes"]),
                      collision_meshes=len(gltf.document["scenes"][1]["nodes"]), static_model_instances=instance_count)
    (output / "game.json").write_text(json.dumps(definition, indent=2), encoding="utf-8")
    receipt = {"source_surfaces": len(surfaces), "triangles": len(positions)//3,
               "render_meshes": definition["render_meshes"], "textures": len(cached) + len(lighting["probes"]) * 6,
               "shadow_helper_surfaces": shadow_only,
               "approximations": ["Bevy PBR with parameterized layer/material response",
                                  "tile/macro diffuse bake with reconstructed fourfold tiling", "runtime-calibrated source-basis SH/probes",
                                  "unrecovered layer reveal constants use explicit defaults", "smoke proxy omitted in favor of single Bevy fog",
                                  "decoded top mip only", "neutral unresolved editor-default model image",
                                  "nearest path-node zombie admission"],
               "static_instances": instance_count, "model_providers": model_count, "model_textures": model_textures}
    (output / "preparation.json").write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    print(json.dumps(receipt))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=WORKSPACE / "config/local_assets.toml")
    parser.add_argument("--content-only", action="store_true", help="regenerate game.json from map entities, preserving prepared geometry coverage")
    parser.add_argument("--recover-gameplay-content", action="store_true", help="recover patch-resolved weapon/attachment definitions and map entities externally")
    parser.add_argument("--prepare-actors", action="store_true", help="convert recovered starter skeletal actors into the external runtime pack")
    parser.add_argument("--prepare-arsenal", action="store_true", help="convert native weapon tuning/localization into the external runtime pack")
    parser.add_argument("--prepare-enemies", action="store_true", help="prepare intact native zombie rigs and lifecycle tuning")
    parser.add_argument("--prepare-rounds", action="store_true", help="recover native solo Original round health, populations, pacing and locomotion")
    args = parser.parse_args()
    config = tomllib.loads(args.config.read_text(encoding="utf-8-sig"))
    if args.prepare_rounds:
        from prepare_rounds import prepare
        prepare(config)
        return
    if args.prepare_enemies:
        from prepare_enemies import prepare
        prepare(config)
        return
    if args.prepare_arsenal:
        from prepare_arsenal import prepare as prepare_arsenal
        prepare_arsenal(config)
        return
    if args.prepare_actors:
        from prepare_actors import prepare as prepare_actors
        prepare_actors(config)
        return
    if args.recover_gameplay_content:
        from prepare_gameplay_content import recover
        recover(config)
        return
    root = Path(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    entities_path = Path(settings.get("entities", root / "game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents"))
    if args.content_only:
        output = external(Path(config["game"]["asset_pack"]))
        previous = json.loads((output / "game.json").read_text(encoding="utf-8-sig"))
        definition = gameplay_definition(entities(external(entities_path)))
        if previous.get("version") == 2:
            definition["content"] = previous["content"]
        for key in ("source_surfaces", "render_meshes", "collision_meshes", "static_model_instances", "survey_views", "tuning"):
            if key in previous:
                definition[key] = previous[key]
        (output / "game.json").write_text(json.dumps(definition, indent=2) + "\n", encoding="utf-8")
        print("Regenerated version-two game.json; retained previously prepared geometry and texture bytes.")
        return
    model_root = Path(settings.get("models", root / "game-prepare/models"))
    instances_path = Path(settings.get("instances", root / "game-prepare/instances/static-instances.json"))
    zone = Path(config["assets"]["installation_root"]) / "zone/all/zm_tomb.ff"
    unlinker = external(Path(settings.get("unlinker", root / "tooling/OAT/v0.33.0/Unlinker.exe")))
    if not entities_path.exists() or not model_root.exists():
        external(model_root).mkdir(parents=True, exist_ok=True)
        subprocess.run([str(unlinker), "--no-color", "--include-assets", "xmodel,image,mapents",
                        "--model-format", "GLTF", "-o", str(model_root), str(zone)], check=True,
                       stdout=subprocess.DEVNULL)
        entities_path = model_root / "maps/mp/zm_tomb.d3dbsp.ents"
    surface_path = instances_path.parent / "model-surfaces.json"
    state_ready = surface_path.exists() and "states" in next(iter(json.loads(surface_path.read_text()).values()))[0]
    if not instances_path.exists() or not state_ready or not (model_root / ".surface-data-ready").exists():
        from export_instances import export
        source = external(Path(settings.get("oat_source", root / "tooling/OAT/surface-7d027e8/source")))
        instances_path = export(source, zone, external(instances_path.parent), external(model_root / "images"))
        (model_root / ".surface-data-ready").touch()
    prepare(Path(settings.get("scene", root / "full-scene-assets/texture-preview.json")),
            entities_path, Path(config["game"]["asset_pack"]), instances_path, model_root)


if __name__ == "__main__":
    main()
