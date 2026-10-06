# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13", "soundfile>=0.13,<0.14"]
# ///
"""Validate and copy the prepared runtime closure; never rewrite source assets.

uv run tools/import_origins.py --config config/local_assets.toml
Add --check to validate an existing import against its external source.
Only the deliberately limited glTF profile emitted by prepare_origins is supported.
"""
import argparse
import hashlib
import io
import json
import math
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import subprocess
import tomllib

try:
    from .content_contract import validate_content, validate_rounds, validate_player_health
except ImportError:
    from content_contract import validate_content, validate_rounds, validate_player_health

import numpy as np
from PIL import Image

WORKSPACE = Path(__file__).resolve().parents[1]
POINTER = b"version https://git-lfs.github.com/spec/v1"
RUNTIME_SUFFIXES = {".json", ".gltf", ".bin", ".png", ".jpg", ".jpeg", ".ogg", ".ttf", ".otf"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def natural(value, label, minimum=0):
    require(type(value) is int and value >= minimum, f"invalid {label}")
    return value


def ref(items, index, label):
    natural(index, label)
    require(index < len(items), f"invalid {label} reference: {index}")
    return items[index]


def safe_path(root, relative, suffixes):
    require(isinstance(relative, str) and relative, "empty dependency path")
    require(not re.search(r"[:\\%?#\x00-\x1f]", relative), "unsafe dependency path")
    parts = relative.split("/")
    require(not any(p in ("", ".", "..") for p in parts), "dependency path escapes root")
    require(PurePosixPath(relative).suffix in suffixes, "forbidden dependency type")
    path = root.joinpath(*parts)
    require(root.resolve() in path.resolve().parents, "dependency path escapes root")
    require(not any(p.is_symlink() or (hasattr(p, "is_junction") and p.is_junction())
                    for p in [path, *path.parents] if p != root.parent), "linked dependency")
    return path


def read_bytes(path):
    require(path.is_file(), f"missing dependency: {path.name}")
    raw = path.read_bytes()
    require(not raw.startswith(POINTER), f"unresolved Git LFS pointer: {path.name}")
    return raw


def clean_strings(value, localized_text=False):
    if isinstance(value, dict):
        for key, item in value.items():
            clean_strings(key)
            is_localized = key == "text" and set(value) == {"id", "key", "text"} and str(value["id"]).startswith("string.")
            clean_strings(item, is_localized)
    elif isinstance(value, list):
        for item in value:
            clean_strings(item)
    elif isinstance(value, str):
        pattern = r"(?<![A-Za-z0-9_])[A-Za-z]:[\\/]|(?:^|\s)/(?:Users|home|mnt|tmp)/|file://"
        if not localized_text:
            pattern += r"|\\"
        require(not re.search(pattern, value),
                "absolute or machine-specific path in JSON")
        require(localized_text or not value.startswith("/"), "absolute path in JSON")
    elif isinstance(value, float):
        require(np.isfinite(value), "non-finite JSON number")


def read_json(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result
    value = json.loads(read_bytes(path), object_pairs_hook=unique)
    clean_strings(value)
    return value


def validate_gltf(root, relative, files):
    path = safe_path(root, relative, {".gltf"})
    document = read_json(path)
    files.add(relative)
    require(document.get("asset", {}).get("version") == "2.0", "unsupported glTF version")
    require(set(document) <= {"asset", "scene", "scenes", "nodes", "meshes", "materials",
                              "textures", "images", "samplers", "accessors", "bufferViews",
                              "buffers", "extensionsUsed", "extensionsRequired", "extras", "skins", "animations"},
            "unsupported glTF feature")
    require(set(document.get("extensionsUsed", []) + document.get("extensionsRequired", []))
            <= {"KHR_materials_unlit"}, "unsupported glTF extension")

    def dependency(uri, suffix):
        # Resolve each URI relative to its glTF while preserving the pack-relative path.
        safe_path(path.parent, uri, {suffix})
        name = (PurePosixPath(relative).parent / uri).as_posix()
        result = safe_path(root, name, {suffix})
        files.add(name)
        return result

    buffers = []
    for buffer in document.get("buffers", []):
        require(set(buffer) <= {"uri", "byteLength", "name"}, "unsupported buffer feature")
        raw = read_bytes(dependency(buffer["uri"], ".bin"))
        require(len(raw) == natural(buffer["byteLength"], "buffer length", 1), "buffer length mismatch")
        buffers.append(raw)
    views = document.get("bufferViews", [])
    for view in views:
        require(set(view) <= {"buffer", "byteOffset", "byteLength", "byteStride", "target", "name"},
                "unsupported buffer-view feature")
        raw = ref(buffers, view["buffer"], "buffer")
        offset = natural(view.get("byteOffset", 0), "buffer-view offset")
        length = natural(view["byteLength"], "buffer-view length", 1)
        require(offset + length <= len(raw), "buffer-view out of bounds")
    arrays = []
    for accessor in document.get("accessors", []):
        require(set(accessor) <= {"bufferView", "componentType", "count", "type", "min", "max",
                                  "byteOffset", "normalized", "name"}, "unsupported accessor feature")
        view = ref(views, accessor["bufferView"], "buffer-view")
        component = accessor["componentType"]
        types = {5120: "i1", 5121: "u1", 5122: "<i2", 5123: "<u2", 5125: "<u4", 5126: "<f4"}
        widths = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
        require(component in types and accessor["type"] in widths, "unsupported accessor layout")
        dtype = np.dtype(types[component])
        width = widths[accessor["type"]]
        size = dtype.itemsize * width
        count = natural(accessor["count"], "accessor count", 1)
        offset = natural(accessor.get("byteOffset", 0), "accessor offset")
        stride = natural(view.get("byteStride", size), "accessor stride", size)
        if "byteStride" in view:
            require(4 <= stride <= 252 and stride % 4 == 0, "invalid byte stride")
        require(offset % dtype.itemsize == 0 and view.get("byteOffset", 0) % dtype.itemsize == 0,
                "misaligned accessor")
        require(offset + (count - 1) * stride + size <= view["byteLength"], "accessor out of bounds")
        values = np.ndarray((count, width), dtype, buffers[view["buffer"]],
                            view.get("byteOffset", 0) + offset, (stride, dtype.itemsize))
        require(np.isfinite(values).all(), "non-finite accessor")
        for key, actual in (("min", values.min(axis=0)), ("max", values.max(axis=0))):
            if key in accessor:
                require(len(accessor[key]) == width and np.allclose(accessor[key], actual, rtol=1e-6, atol=1e-7),
                        f"accessor {key} mismatch")
        arrays.append(values)
    decoded = set()
    def decode_png(image_path):
        if image_path in decoded:
            return
        read_bytes(image_path)
        with Image.open(image_path) as pixels:
            require(pixels.format == "PNG", "non-PNG image")
            pixels.verify()
        with Image.open(image_path) as pixels:
            pixels.load()
        decoded.add(image_path)
    for image in document.get("images", []):
        require(set(image) <= {"uri", "name"}, "embedded or extended images unsupported")
        suffix = PurePosixPath(image["uri"]).suffix
        require(suffix in {".png", ".jpg", ".jpeg"}, "unsupported glTF image")
        image_path = dependency(image["uri"], suffix)
        if suffix == ".png":
            decode_png(image_path)
        else:
            read_bytes(image_path)
            with Image.open(image_path) as pixels:
                require(pixels.format == "JPEG", "non-JPEG image")
                pixels.verify()
            with Image.open(image_path) as pixels:
                pixels.load()
    for texture in document.get("textures", []):
        require(set(texture) <= {"source", "sampler", "name"}, "unsupported texture feature")
        ref(document.get("images", []), texture["source"], "image")
        if "sampler" in texture:
            ref(document.get("samplers", []), texture["sampler"], "sampler")
    extras = document.get("extras", {})
    require(set(extras) <= {"lighting"}, "unsupported document extras")
    lighting = extras.get("lighting", {"lightmaps": [], "probes": []})
    require(set(lighting) == {"lightmaps", "probes"}, "invalid lighting metadata")
    for slot in lighting["lightmaps"]:
        require(set(slot) <= {"primary", "secondary"}, "invalid lightmap metadata")
        for uri in slot.values():
            decode_png(dependency(uri, ".png"))
    for probe in lighting["probes"]:
        require(set(probe) == {"faces", "position", "mip_bias", "sh", "sh_basis"}, "invalid probe metadata")
        require(len(probe["faces"]) == 6 and len(set(probe["faces"])) == 6, "missing cube faces")
        require(len(probe["position"]) == 3 and len(probe["sh"]) == 12 and probe["sh_basis"] == "source_z_up", "invalid probe coefficients")
        dimensions = set()
        for uri in probe["faces"]:
            image_path = dependency(uri, ".png")
            decode_png(image_path)
            with Image.open(image_path) as pixels:
                dimensions.add(pixels.size)
        require(len(dimensions) == 1 and next(iter(dimensions))[0] == next(iter(dimensions))[1], "invalid cube face dimensions")

    def material_refs(value):
        for key, item in value.items():
            if key.endswith("Texture"):
                require(set(item) <= {"index", "texCoord", "scale", "strength"}, "unsupported texture binding")
                ref(document.get("textures", []), item["index"], "texture")
            elif isinstance(item, dict):
                material_refs(item)
    for material in document.get("materials", []):
        require(set(material) <= {"name", "pbrMetallicRoughness", "normalTexture", "occlusionTexture",
                                  "emissiveTexture", "emissiveFactor", "alphaMode", "alphaCutoff",
                                  "doubleSided", "extensions", "extras"}, "unsupported material feature")
        require(set(material.get("extensions", {})) <= {"KHR_materials_unlit"}, "unsupported material extension")
        require(set(material.get("pbrMetallicRoughness", {})) <= {"baseColorFactor", "baseColorTexture",
                                                                 "metallicFactor", "roughnessFactor",
                                                                 "metallicRoughnessTexture"}, "unsupported PBR feature")
        material_refs(material)
        metadata = material.get("extras", {})
        require(set(metadata) <= {"surface"}, "unsupported material extras")
        if "surface" in metadata:
            surface = metadata["surface"]
            require({"identity", "family", "channels"} <= set(surface) <= {"identity", "family", "channels", "preparation", "presentation", "layers"}, "invalid surface metadata")
            if "presentation" in surface:
                state = surface["presentation"]
                require(set(state) == {"alpha", "cutoff", "cull", "emissive", "fog", "visible", "wettable", "lamp", "solid"}, "invalid presentation metadata")
                require(state["alpha"] in ("opaque", "mask", "blend", "premultiplied", "add", "multiply") and state["cull"] in ("none", "back", "front"), "invalid surface state")
                require(isinstance(state["cutoff"], (int, float)) and 0 <= state["cutoff"] <= 1 and all(type(state[k]) is bool for k in ("emissive", "fog", "visible", "wettable", "lamp", "solid")), "invalid presentation parameters")
                require(material.get("doubleSided", False) == (state["cull"] == "none"), "cull disposition mismatch")
            if "layers" in surface:
                layout = surface["layers"]
                require(set(layout) == {"operations", "reveal", "controls", "uvs", "normal_basis"}, "invalid layered metadata")
                require(2 <= len(layout["operations"]) <= 4 and layout["operations"][0] == "base" and all(x in ("blend", "multiply", "add") for x in layout["operations"][1:]), "unsupported layer operation")
                require(layout["controls"] == "vertex_color_gb" and layout["uvs"] == "secondary_half" and layout["normal_basis"] == "secondary_signed_rgba", "unsupported layer controls")
                require(len(layout["reveal"]) == 2 and all(len(x) == 2 and all(isinstance(v, (int, float)) and math.isfinite(v) for v in x) for x in layout["reveal"]), "invalid blend parameters")
            if "preparation" in surface:
                require(surface["family"] == "tile_blend" and surface["preparation"] == {"kind": "tile_macro_bake", "tiles": 4}, "invalid material preparation")
            require(surface["identity"] == material.get("name") and isinstance(surface["family"], str), "invalid material identity")
            layers = {}
            for channel in surface["channels"]:
                require({"role", "layer", "texture", "channels"} <= set(channel)
                        <= {"role", "layer", "texture", "channels", "approximation"}, "invalid raw channel metadata")
                if "approximation" in channel:
                    require(channel["approximation"] == "neutral placeholder for unresolved editor-default image", "unknown texture approximation")
                role = channel["role"]
                require(role in ("color", "normal", "specular_gloss", "auxiliary", "emissive"), "invalid channel role")
                require(0 <= channel["layer"] <= 31 and channel["layer"] >= layers.get(role, 0), "unordered material layers")
                layers[role] = channel["layer"] + 1
                ref(document.get("textures", []), channel["texture"], "raw texture")
                require(channel["channels"] == ("rgb:specular,a:gloss" if role == "specular_gloss" else "rgba"), "invalid channel interpretation")
            if "preparation" in surface:
                colors = [c["texture"] for c in surface["channels"] if c["role"] == "color"]
                require(len(colors) == 3 and material["pbrMetallicRoughness"]["baseColorTexture"]["index"] not in colors,
                        "tile/macro material must use a derived diffuse texture, retaining all source layers")
    for mesh in document.get("meshes", []):
        require(set(mesh) <= {"name", "primitives", "extras"} and mesh["primitives"], "unsupported or empty mesh")
        metadata = mesh.get("extras", {})
        require(set(metadata) <= {"lighting", "lighting_vertices"}, "unsupported mesh extras")
        if "lighting" in metadata:
            assignment = metadata["lighting"]
            require(set(assignment) == {"lightmap", "probe"}, "invalid mesh lighting assignment")
            for key, table in (("lightmap", "lightmaps"), ("probe", "probes")):
                if assignment[key] is not None:
                    ref(lighting[table], assignment[key], key)
        for primitive in mesh["primitives"]:
            require(set(primitive) <= {"attributes", "indices", "mode", "material"}, "unsupported primitive")
            require(primitive.get("mode", 4) == 4, "non-triangle primitive")
            position = ref(arrays, primitive["attributes"]["POSITION"], "position accessor")
            require(position.shape[1] == 3, "invalid position accessor")
            if "lighting_vertices" in metadata:
                mapping = ref(arrays, metadata["lighting_vertices"], "instance lighting vertex mapping")
                require(mapping.shape[1] == 1 and mapping.dtype.kind == "u" and len(mapping) == len(position), "invalid instance lighting vertex mapping")
            for index in primitive["attributes"].values():
                require(len(ref(arrays, index, "attribute accessor")) == len(position), "attribute count mismatch")
            for semantic, index in primitive["attributes"].items():
                values = arrays[index]
                require(semantic in ("POSITION", "NORMAL", "TANGENT", "COLOR_0", "TEXCOORD_0", "TEXCOORD_1", "JOINTS_0", "WEIGHTS_0", "_ORIGINS_LAYER_UV", "_ORIGINS_LAYER3_UV", "_ORIGINS_ROTATION1", "_ORIGINS_ROTATION2"), "unsupported mesh attribute")
                require(values.shape[1] == (2 if semantic.startswith("TEXCOORD") or semantic == "_ORIGINS_LAYER3_UV" else 4 if semantic in ("TANGENT", "COLOR_0", "JOINTS_0", "WEIGHTS_0", "_ORIGINS_LAYER_UV", "_ORIGINS_ROTATION1", "_ORIGINS_ROTATION2") else 3), "attribute width mismatch")
                if semantic == "JOINTS_0":
                    require(values.dtype.kind == "u" and values.dtype.itemsize <= 2, "invalid joint indices")
                if semantic == "WEIGHTS_0":
                    accessor = document["accessors"][index]
                    weights = values.astype(float)
                    if values.dtype.kind == "u":
                        require(accessor.get("normalized") and values.dtype.itemsize <= 2, "unnormalized integer skin weights")
                        weights /= np.iinfo(values.dtype).max
                    else:
                        require(values.dtype.kind == "f", "invalid skin weight type")
                    require((weights >= 0).all() and (weights <= 1).all() and np.allclose(weights.sum(axis=1), 1, atol=1e-4), "invalid skin weight normalization")
                if semantic == "TANGENT":
                    require(np.allclose(np.linalg.norm(values[:, :3], axis=1), 1, atol=1e-5)
                            and np.isin(values[:, 3], [-1, 1]).all(), "invalid tangent or handedness")
                if semantic == "COLOR_0":
                    require(((values >= 0) & (values <= 1)).all(), "invalid vertex color")
            require(("JOINTS_0" in primitive["attributes"]) == ("WEIGHTS_0" in primitive["attributes"]), "incomplete skin attributes")
            if extras:
                require("TANGENT" in primitive["attributes"] and "COLOR_0" in primitive["attributes"], "missing retained surface attributes")
            indices = ref(arrays, primitive["indices"], "index accessor")
            require(indices.dtype.kind == "u" and indices.shape[1] == 1 and len(indices) % 3 == 0,
                    "invalid triangle indices")
            require(int(indices.max()) < len(position), "vertex index out of bounds")
            ref(document.get("materials", []), primitive["material"], "material")
    nodes = document.get("nodes", [])
    for node in nodes:
        require(set(node) <= {"name", "mesh", "skin", "children", "matrix", "translation", "rotation", "scale", "extras"},
                "unsupported node feature")
        if "mesh" in node:
            ref(document.get("meshes", []), node["mesh"], "mesh")
        require(not ("matrix" in node and any(k in node for k in ("translation", "rotation", "scale"))),
                "mixed node transforms")
        for key, width in (("matrix", 16), ("translation", 3), ("rotation", 4), ("scale", 3)):
            if key in node:
                require(len(node[key]) == width and all(type(x) in (int, float) for x in node[key]), "invalid transform")
        extras = node.get("extras", {})
        require(set(extras) <= {"lightmap", "lighting"}, "unsupported node extras")
        if extras.get("lightmap"):
            decode_png(dependency(extras["lightmap"], ".png"))
        if "lighting" in extras:
            assignment = extras["lighting"]
            require(set(assignment) <= {"lightmap", "probe", "lighting_id", "color_lods"}, "invalid lighting assignment")
            for key, table in (("lightmap", "lightmaps"), ("probe", "probes")):
                if assignment.get(key) is not None:
                    ref(lighting[table], assignment[key], key)
            if "lighting_id" in assignment:
                natural(assignment["lighting_id"], "lighting id")
            if "color_lods" in assignment:
                require(len(assignment["color_lods"]) == 4, "invalid instance lighting LODs")
                for index in assignment["color_lods"]:
                    if index is not None:
                        values = ref(arrays, index, "instance lightmap colors")
                        require(values.shape[1] == 4 and ((values >= 0) & (values <= 1)).all(), "invalid instance lighting colors")
        for child in node.get("children", []):
            ref(nodes, child, "child node")
    skins = document.get("skins", [])
    for skin in skins:
        require(set(skin) <= {"name", "joints", "skeleton", "inverseBindMatrices"}, "unsupported skin feature")
        joints = skin.get("joints", [])
        require(joints and len(set(joints)) == len(joints), "empty or duplicate skin joints")
        for joint in joints:
            node = ref(nodes, joint, "skin joint")
            require(isinstance(node.get("name"), str) and node["name"], "skin joint lost its name")
        require(len({nodes[j]["name"] for j in joints}) == len(joints), "ambiguous skin joint names")
        if "skeleton" in skin:
            ref(nodes, skin["skeleton"], "skeleton root")
        if "inverseBindMatrices" in skin:
            matrix_index = skin["inverseBindMatrices"]
            matrices = ref(arrays, matrix_index, "inverse bind matrices")
            accessor = document["accessors"][matrix_index]
            require(accessor["type"] == "MAT4" and accessor["componentType"] == 5126 and len(matrices) == len(joints), "invalid inverse bind matrix layout")
    for node in nodes:
        if "skin" in node:
            skin = ref(skins, node["skin"], "node skin")
            mesh = ref(document.get("meshes", []), node.get("mesh"), "skinned mesh")
            for primitive in mesh["primitives"]:
                attributes = primitive["attributes"]
                require({"JOINTS_0", "WEIGHTS_0"} <= attributes.keys(), "skinned node lacks joint/weight attributes")
                require(int(arrays[attributes["JOINTS_0"]].max()) < len(skin["joints"]), "skin joint index out of bounds")
    for animation in document.get("animations", []):
        require(set(animation) <= {"name", "channels", "samplers"}, "unsupported animation feature")
        samplers = animation.get("samplers", [])
        require(samplers and animation.get("channels"), "empty animation")
        for sampler in samplers:
            require(set(sampler) <= {"input", "output", "interpolation"}, "unsupported animation sampler")
            times = ref(arrays, sampler["input"], "animation times")
            values = ref(arrays, sampler["output"], "animation values")
            require(document["accessors"][sampler["input"]]["componentType"] == 5126 and times.shape[1] == 1 and (times >= 0).all() and (np.diff(times[:, 0]) > 0).all(), "invalid animation times")
            require(document["accessors"][sampler["output"]]["componentType"] == 5126, "invalid animation value type")
            mode = sampler.get("interpolation", "LINEAR")
            require(mode in {"LINEAR", "STEP", "CUBICSPLINE"}, "unsupported animation interpolation")
            require(len(values) == len(times) * (3 if mode == "CUBICSPLINE" else 1), "animation sample count mismatch")
        targets = set()
        for channel in animation["channels"]:
            require(set(channel) == {"sampler", "target"} and set(channel["target"]) == {"node", "path"}, "invalid animation channel")
            target = channel["target"]
            node = ref(nodes, target["node"], "animation target")
            require(node.get("name") and "matrix" not in node, "animation target lacks named TRS node")
            require(target["path"] in {"translation", "rotation", "scale"}, "unsupported animation target")
            key = (target["node"], target["path"])
            require(key not in targets, "duplicate animation target")
            targets.add(key)
            sampler = ref(samplers, channel["sampler"], "animation sampler")
            values = arrays[sampler["output"]]
            require(values.shape[1] == (4 if target["path"] == "rotation" else 3), "animation value width mismatch")
            if target["path"] == "rotation":
                quaternions = values[1::3] if sampler.get("interpolation") == "CUBICSPLINE" else values
                require(np.allclose(np.linalg.norm(quaternions, axis=1), 1, atol=1e-4), "invalid animation quaternion")
    scenes = document.get("scenes", [])
    if "scene" in document:
        ref(scenes, document["scene"], "default scene")
    scene_meshes = []
    for scene in scenes:
        require(set(scene) <= {"nodes", "name"}, "unsupported scene feature")
        visited = set()
        pending = list(scene.get("nodes", []))
        while pending:
            index = pending.pop()
            node = ref(nodes, index, "scene node")
            require(index not in visited, "cyclic or multiply-parented scene node")
            visited.add(index)
            pending.extend(node.get("children", []))
        scene_meshes.append([nodes[i] for i in sorted(visited) if "mesh" in nodes[i]])
    # Also check cycles in nodes not attached to a scene.
    done = set()
    def visit(index, active):
        require(index not in active, "cyclic node graph")
        if index in done:
            return
        for child in nodes[index].get("children", []):
            visit(child, active | {index})
        done.add(index)
    for index in range(len(nodes)):
        visit(index, set())
    return document, scene_meshes


def validate_pack(root, receipt=None, exact=False):
    root = root.resolve()
    files = {"game.json"}
    game = read_json(safe_path(root, "game.json", {".json"}))
    require(set(game) <= {"version", "scene", "collision_scene", "spawn", "yaw", "spawn_points",
                          "waypoints", "survey_views", "source_surfaces", "render_meshes",
                          "collision_meshes", "static_model_instances", "tuning", "content", "rounds", "player_health", "solo_revive", "environment"}, "unknown gameplay configuration")
    require(set(game.get("tuning", {})) <= {"walk_speed", "sprint_speed", "sprint_seconds", "sprint_minimum", "sprint_recovery_rate", "sprint_recovery_delay", "crouch_speed", "gravity",
        "jump_speed", "step_height", "radius", "standing_height", "crouch_height", "eye_height",
        "mouse_sensitivity", "pistol_damage", "melee_damage", "magazine", "reserve", "fire_interval",
        "reload_seconds", "zombie_damage", "zombie_speed", "combat_points", "mobility"}, "unknown gameplay tuning")
    environment = game.get("environment", {})
    require(isinstance(environment,dict) and set(environment)<={"mud"},"unknown environment fields")
    mud = environment.get("mud")
    if mud is not None:
        require(isinstance(mud,dict) and set(mud)=={"scale","stamin_scale","step","stamin_step","interval","volumes"},"invalid mud fields")
        require(all(type(mud[k]) in (int,float) and math.isfinite(mud[k]) and mud[k]>0 for k in ("scale","stamin_scale","step","stamin_step","interval")),"invalid mud tuning")
        require(mud["scale"]<=mud["stamin_scale"]<=1 and mud["stamin_step"]<=mud["step"]<=1,"invalid mud modifiers")
        volumes=mud["volumes"];require(isinstance(volumes,list) and 0<len(volumes)<=4096,"invalid mud volume count")
        identities=set()
        for volume in volumes:
            require(isinstance(volume,dict) and set(volume)=={"id","position","vertices"},"invalid volume fields")
            require(isinstance(volume["id"],str) and re.fullmatch(r"[A-Za-z0-9_.-]+",volume["id"]) and volume["id"] not in identities,"invalid volume ID");identities.add(volume["id"])
            points=volume["vertices"];require(isinstance(points,list) and 4<=len(points)<=128,"invalid convex volume")
            require(all(isinstance(v,list) and len(v)==3 and all(type(x) in (int,float) and math.isfinite(x) for x in v) for v in [volume["position"]]+points),"invalid volume vectors")
            require(np.linalg.matrix_rank(np.array(points)-points[0])==3,"flat volume vertices")
    mobility = game.get("tuning", {}).get("mobility", {})
    require(isinstance(mobility, dict) and set(mobility) <= {"prone_height","prone_eye","prone_speed","prone_step","acceleration","deceleration","air_acceleration","air_deceleration","stance_seconds","jump_recovery","dive_speed","dive_lift","dive_recovery","fall_minimum","fall_fatal","landing_dip","landing_seconds","walk_bob","sprint_bob","stride","sway_limit","sway_response"}, "unknown mobility tuning")
    require(all(type(v) in (int,float) and math.isfinite(v) and v > 0 for k,v in mobility.items() if k not in ("walk_bob","sprint_bob","stride")),"invalid mobility tuning")
    for key in ("walk_bob","sprint_bob","stride"):
        if key in mobility: require(isinstance(mobility[key],list) and len(mobility[key])==2 and all(type(x) in (int,float) and math.isfinite(x) and x>0 for x in mobility[key]),"invalid movement feedback tuning")
    require(game.get("tuning", {}).get("radius",.35)*2 < mobility.get("prone_height",.75) < game.get("tuning", {}).get("crouch_height",1.15),"invalid prone capsule")
    require(mobility.get("prone_eye",.3048) < mobility.get("prone_height",.75),"invalid prone eye")
    require(mobility.get("fall_minimum",3.2512)<mobility.get("fall_fatal",7.62),"unordered fall thresholds")
    sprint_tuning = game.get("tuning", {})
    for key, default in (("sprint_seconds", 4), ("sprint_minimum", 0.25), ("sprint_recovery_rate", 1), ("sprint_recovery_delay", 0)):
        value = sprint_tuning.get(key, default)
        require(type(value) in (int, float) and math.isfinite(value) and (value >= 0 if key == "sprint_recovery_delay" else value > 0), "invalid sprint tuning")
    require(sprint_tuning.get("sprint_minimum", 0.25) <= sprint_tuning.get("sprint_seconds", 4), "sprint minimum exceeds capacity")
    points = game.get("tuning", {}).get("combat_points", [10, 50, 50, 10, 80])
    require(isinstance(points, list) and len(points) == 5 and all(type(v) is int and 0 <= v <= 2147483647 for v in points), "invalid combat point tuning")
    require(game.get("version") == 2, "unsupported gameplay version; expected 2, regenerate/import the pack")
    validate_rounds(game.get("rounds"), require)
    validate_player_health(game.get("player_health"), require)
    dependencies = validate_content(game.get("content"), require)
    solo = game.get("solo_revive")
    require(isinstance(solo, dict) and set(solo) == {"perk", "seconds", "bleedout_seconds", "bleedout_poll_seconds", "maximum_purchases", "downed_weapon", "reserve_clips", "downed_height", "downed_speed"}, "invalid solo revive profile")
    require(all(type(solo[k]) in (int, float) and math.isfinite(solo[k]) and solo[k] > 0 for k in ("seconds", "bleedout_seconds", "bleedout_poll_seconds", "downed_height", "downed_speed")), "invalid solo revive timing/movement")
    require(solo["seconds"] < solo["bleedout_seconds"] and solo["bleedout_seconds"] % 1 == 0 and solo["bleedout_seconds"] <= 3600 and 0.001 <= solo["bleedout_poll_seconds"] <= solo["bleedout_seconds"], "invalid solo revive deadline")
    require(type(solo["maximum_purchases"]) is int and solo["maximum_purchases"] > 0 and type(solo["reserve_clips"]) is int and 0 <= solo["reserve_clips"] <= 100, "invalid solo charge/ammo count")
    require(solo["perk"] in {p["id"] for p in game["content"]["perks"]} and solo["downed_weapon"] in {w["id"] for w in game["content"]["weapons"]}, "unresolved solo revive content")
    require(game.get("tuning", {}).get("radius", .35) * 2 <= solo["downed_height"] < game.get("tuning", {}).get("standing_height", 1.8), "invalid downed capsule height")
    documents = {}
    for key, count_key in (("scene", "render_meshes"), ("collision_scene", "collision_meshes")):
        match = re.fullmatch(r"(.+\.gltf)#Scene(0|[1-9][0-9]*)", game[key])
        require(match is not None, "invalid gameplay scene label")
        name, index = match[1], int(match[2])
        if name not in documents:
            documents[name] = validate_gltf(root, name, files)
        _, scenes = documents[name]
        nodes = ref(scenes, index, "gameplay scene")
        require(len(nodes) == natural(game[count_key], count_key, 1), "gameplay scene count mismatch")
    for game_key, receipt_key in (("source_surfaces", "source_surfaces"), ("render_meshes", "render_meshes"),
                                   ("static_model_instances", "static_instances")):
        count = natural(game[game_key], game_key, 1)
        if receipt is not None:
            require(count == receipt[receipt_key], "preparation receipt count mismatch")
    # Additional presentation dependencies belong to the runtime closure. Raw source
    # inputs and preparation receipts are never traversed here.
    map_documents = dict(documents)
    map_files = set(files)
    for reference in dependencies:
        name, _, label = reference.partition("#")
        dependency = safe_path(root, name, RUNTIME_SUFFIXES - {".json", ".bin"})
        suffix = dependency.suffix
        if suffix == ".gltf":
            if name not in documents:
                documents[name] = validate_gltf(root, name, files)
            if label:
                match = re.fullmatch(r"(Scene|Animation)(0|[1-9][0-9]*)", label)
                require(match is not None, "invalid glTF content label")
                doc, scenes = documents[name]
                ref(scenes if match[1] == "Scene" else doc.get("animations", []), int(match[2]), "glTF content label")
        else:
            require(not label, "unexpected media asset label")
            read_bytes(dependency)
            files.add(name)
            if suffix in {".png", ".jpg", ".jpeg"}:
                with Image.open(dependency) as pixels:
                    require(pixels.format == ("PNG" if suffix == ".png" else "JPEG"), "image format mismatch")
                    pixels.verify()
                with Image.open(dependency) as pixels:
                    pixels.load()
            elif suffix == ".ogg":
                # Stream-decode rather than accepting a valid header with damaged packets.
                import soundfile
                with soundfile.SoundFile(dependency) as sound:
                    require(sound.format == "OGG" and sound.subtype == "VORBIS" and sound.frames > 0, "unsupported or empty runtime audio")
                    frames = 0
                    for block in sound.blocks(blocksize=65536):
                        require(np.isfinite(block).all(), "non-finite audio samples")
                        frames += len(block)
                    require(frames == sound.frames, "truncated audio stream")
            else:
                from PIL import ImageFont
                ImageFont.truetype(str(dependency), 16)
    presentations = {p["id"]:p for p in game["content"]["presentations"]}
    for enemy in game["content"]["enemies"]:
        for hit in enemy.get("hit_regions",[]):
            scene = presentations[hit["presentation"]]["scene"]
            require(scene is not None,"hit volume has no rig scene")
            doc = documents[scene.split("#",1)[0]][0]
            names = {doc["nodes"][i].get("name") for skin in doc.get("skins",[]) for i in skin["joints"]}
            require(hit["start"] in names and hit["end"] in names,"hit volume bone is absent from skin")
        if not enemy.get("parts"): continue
        scene = presentations[enemy["presentation"]]["scene"]
        require(scene is not None,"multipart enemy has no body scene")
        body = documents[scene.split("#",1)[0]][0]
        body_names = {body["nodes"][i].get("name") for skin in body.get("skins",[]) for i in skin["joints"]}
        for part in enemy["parts"]:
            require(part["socket"] in body_names,"enemy attachment socket is absent from body skeleton")
            part_scene = presentations[part["presentation"]]["scene"]
            require(part_scene is not None,"enemy attachment has no scene")
            doc = documents[part_scene.split("#",1)[0]][0]
            require(part["root_joint"] is None or any(n.get("name")==part["root_joint"] for n in doc["nodes"]),"enemy partial rig root is absent")
    triangles = sum(doc["accessors"][p["indices"]]["count"] // 3
                    for doc, _ in map_documents.values() for mesh in doc["meshes"]
                    if mesh.get("name", "").startswith("world_") for p in mesh["primitives"])
    if receipt is not None:
        require(triangles == receipt["triangles"], "world triangle receipt mismatch")
    pngs = sum(name.endswith(".png") for name in map_files)
    if receipt is not None:
        require(pngs == receipt["textures"] + receipt["model_textures"], "texture receipt count mismatch")
        require(sum(name.endswith(".png") and PurePosixPath(name).name.startswith("model-") for name in map_files)
                == receipt["model_textures"], "model texture receipt count mismatch")
    if exact:
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file()}
        require(actual == files, f"unexpected or missing runtime files: {sorted(actual ^ files)}")
    hashes = {name: hashlib.sha256(read_bytes(safe_path(root, name, RUNTIME_SUFFIXES))).hexdigest()
              for name in sorted(files)}
    return game, documents, hashes


def external(path):
    path = Path(path).resolve()
    require(path != WORKSPACE and WORKSPACE not in path.parents, "preparation inputs must remain external")
    return path


def reconcile_sources(config, receipt, documents):
    """Verify receipt facts that grouping removes from runtime glTF against retained inputs."""
    root = external(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    scene = external(settings.get("scene", root / "full-scene-assets/texture-preview.json"))
    manifest = json.loads(scene.read_text(encoding="utf-8-sig"))
    geometry_path = safe_path(scene.parent, manifest["geometry"], {Path(manifest["geometry"]).suffix})
    raw = read_bytes(geometry_path)
    require(hashlib.sha256(raw).hexdigest() == manifest["geometry_sha256"], "source geometry checksum mismatch")
    require(raw[:8] == b"BO2DGS\0\0" and struct.unpack_from("<II", raw, 8) == (1, 0), "invalid world exchange")
    vertices, indices, surfaces, length = struct.unpack_from("<4Q", raw, 16)
    require(length == len(raw) == 64 + vertices * 12 + indices * 4 + surfaces * 48, "world exchange length mismatch")
    require(surfaces == len(manifest["surfaces"]) == receipt["source_surfaces"] and indices == receipt["triangles"] * 3,
            "source world receipt mismatch")
    instances_path = external(settings.get("instances", root / "game-prepare/instances/static-instances.json"))
    instances = json.loads(instances_path.read_text(encoding="utf-8-sig"))
    require(len(instances) == receipt["static_instances"], "source placement receipt mismatch")
    require(len({item["model"] for item in instances}) == receipt["model_providers"], "source provider receipt mismatch")
    model_root = external(settings.get("models", root / "game-prepare/models"))
    render_doc = documents[next(iter(documents))][0]
    if render_doc.get("extras", {}).get("lighting"):
        inventory = json.loads((scene.parent / "inventory.private.json").read_text(encoding="utf-8-sig"))
        require([m["name"] for m in render_doc["materials"][:len(manifest["materials"])]]
                == [m["name"] for m in inventory["materials"]], "world material identity differs from source")
        expected = {}
        for surface_index, source in enumerate(manifest["surfaces"]):
            _, count = struct.unpack_from("<QQ", raw, 64 + vertices * 12 + indices * 4 + surface_index * 48)
            key = (source["material"], None if source.get("lightmap") in (31, 255) else source.get("lightmap"),
                   None if source.get("probe") == 255 else source.get("probe"))
            expected[key] = expected.get(key, 0) + count // 3
        actual = {}
        for index, mesh in enumerate(render_doc["meshes"]):
            if not mesh.get("name", "").startswith("world_"):
                continue
            primitive = mesh["primitives"][0]
            material = primitive["material"]
            require("lighting" in mesh.get("extras", {}), "world mesh lost its lighting assignment")
            assignment = mesh["extras"]["lighting"]
            key = (material, assignment.get("lightmap"), assignment.get("probe"))
            actual[key] = actual.get(key, 0) + render_doc["accessors"][primitive["indices"]]["count"] // 3
        require(actual == expected, "world material/lightmap/probe partition differs from source")
    nodes = [n for n in render_doc["nodes"] if n.get("name") == "static_instance"]
    providers = {}
    cursor = 0
    basis = np.array([[1, 0, 0], [0, 0, 1], [0, -1, 0]], dtype=float)
    for item in instances:
        name = item["model"]
        if name not in providers:
            model = json.loads(read_bytes(safe_path(model_root, f"model_export/{name}_lod0.gltf", {".gltf"})))
            count = sum(len(mesh["primitives"]) for mesh in model["meshes"])
            require(count > 0 and cursor + count <= len(nodes), "missing model placement meshes")
            providers[name] = [n["mesh"] for n in nodes[cursor:cursor + count]]
            require(len(set(providers[name])) == count, "collapsed model provider meshes")
        shared = providers[name]
        placed = nodes[cursor:cursor + len(shared)]
        require([n["mesh"] for n in placed] == shared, "shared model references differ from source placement order")
        matrix = np.eye(4)
        matrix[:3, :3] = basis @ np.asarray(item["axis"]).reshape(3, 3).T @ basis.T * item["scale"]
        matrix[:3, 3] = np.asarray(item["origin"])[[0, 2, 1]] * [0.0254, 0.0254, -0.0254]
        require(all(np.allclose(n["matrix"], matrix.T.flatten(), rtol=1e-7, atol=1e-7) for n in placed),
                "model placement transform differs from source")
        if "probe" in item:
            for node in placed:
                assignment = node.get("extras", {}).get("lighting", {})
                require(assignment.get("probe") == (None if item["probe"] == 255 else item["probe"])
                        and assignment.get("lighting_id") == item["lighting_id"], "model lighting assignment differs from source")
                require([index is not None for index in assignment.get("color_lods", [None] * 4)]
                        == [bool(colors) for colors in item["lightmap_colors"]], "model lightmap-color assignment differs from source")
        cursor += len(shared)
    require(cursor == len(nodes), "extra model placements")
    all_meshes = [mesh for group in providers.values() for mesh in group]
    require(len(set(all_meshes)) == len(all_meshes), "distinct source providers unexpectedly merged")


def import_pack(source, destination, receipt, check=False, validated_hashes=None):
    source, destination = source.resolve(), destination.resolve()
    require(source != destination and source not in destination.parents and destination not in source.parents,
            "source and destination overlap")
    if validated_hashes is None:
        _, _, hashes = validate_pack(source, receipt, exact=receipt is None)
    else:
        hashes = validated_hashes
    if not check:
        if destination.exists():
            existing = {p.relative_to(destination).as_posix() for p in destination.rglob("*") if p.is_file()}
            require(existing <= hashes.keys(), "destination contains unrelated files; refusing to replace them")
        for name in hashes:
            target = safe_path(destination, name, RUNTIME_SUFFIXES)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(safe_path(source, name, RUNTIME_SUFFIXES), target)
    _, _, imported = validate_pack(destination, receipt, exact=True)
    require(imported == hashes, "imported file hashes differ from source")
    require(all(hashlib.sha256(read_bytes(source / name)).hexdigest() == digest for name, digest in hashes.items()),
            "source changed during import")
    return {"runtime_files": len(hashes), "png_textures": sum(n.endswith(".png") for n in hashes),
            "runtime_bytes": sum((destination / n).stat().st_size for n in hashes), "sha256": hashes}


def check_index(destination, hashes):
    """Inspect every staged candidate, its attributes, pointer bytes and local LFS object."""
    def git(*args, data=None):
        return subprocess.run(["git", *args], input=data, cwd=WORKSPACE,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout
    prefix = destination.relative_to(WORKSPACE).as_posix() + "/"
    entries = {}
    for entry in git("ls-files", "--stage", "-z", "--", prefix).split(b"\0"):
        if not entry:
            continue
        metadata, name = entry.decode().split("\t")
        mode, oid, stage = metadata.split()
        require(mode == "100644" and stage == "0", "non-regular or conflicted staged asset")
        entries[name.removeprefix(prefix)] = oid
    require(entries.keys() == hashes.keys(), "staged runtime file list differs from validated closure")
    names = [prefix + name for name in entries]
    attributes = git("check-attr", "--cached", "-z", "--stdin", "filter", "diff", "merge", "text",
                     data=b"\0".join(n.encode() for n in names) + b"\0").decode().split("\0")
    attrs = {}
    for name, key, value in zip(attributes[0:-1:3], attributes[1:-1:3], attributes[2:-1:3]):
        attrs.setdefault(name, {})[key] = value
    batch = io.BytesIO(git("cat-file", "--batch", data="".join(oid + "\n" for oid in entries.values()).encode()))
    lfs_root = Path(git("rev-parse", "--git-path", "lfs/objects").decode().strip())
    if not lfs_root.is_absolute():
        lfs_root = WORKSPACE / lfs_root
    for name, oid in entries.items():
        header = batch.readline().decode().split()
        require(header[:2] == [oid, "blob"], "invalid staged object")
        raw = batch.read(int(header[2]))
        require(batch.read(1) == b"\n", "invalid Git batch response")
        actual_attrs = attrs[prefix + name]
        if name == "game.json":
            require(actual_attrs["filter"] == "unspecified" and actual_attrs["text"] == "unset",
                    "game.json must retain ordinary Git bytes")
            require(hashlib.sha256(raw).hexdigest() == hashes[name], "staged configuration hash mismatch")
        else:
            require(actual_attrs == {"filter": "lfs", "diff": "lfs", "merge": "lfs", "text": "unset"},
                    "runtime LFS attributes missing")
            size = (destination / name).stat().st_size
            expected = POINTER + f"\noid sha256:{hashes[name]}\nsize {size}\n".encode()
            require(raw == expected, "staged LFS pointer differs from validated source")
            digest = hashes[name]
            local = lfs_root / digest[:2] / digest[2:4] / digest
            payload = read_bytes(local)
            require(len(payload) == size and hashlib.sha256(payload).hexdigest() == digest, "local LFS object corrupt")
    return {"staged_files": len(entries), "lfs_pointers": len(entries) - 1, "ordinary_git_files": 1}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=WORKSPACE / "config/local_assets.toml")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--check-index", action="store_true", help="also verify staged files and LFS objects; implies --check")
    parser.add_argument("--receipt", type=Path, help="optional external validation receipt")
    args = parser.parse_args()
    config = tomllib.loads(args.config.read_text(encoding="utf-8-sig"))
    source = external(config["game"]["asset_pack"])
    receipt = read_json(source / "preparation.json")
    _, documents, hashes = validate_pack(source, receipt)
    reconcile_sources(config, receipt, documents)
    result = import_pack(source, WORKSPACE / "assets/origins", receipt, args.check or args.check_index, hashes)
    if args.check_index:
        result.update(check_index(WORKSPACE / "assets/origins", result["sha256"]))
    if args.receipt:
        target = external(args.receipt)
        require(source != target and source not in target.parents, "receipt must not modify source pack")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({k: v for k, v in result.items() if k != "sha256"}))


if __name__ == "__main__":
    main()
