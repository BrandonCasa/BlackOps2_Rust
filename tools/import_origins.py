# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13", "soundfile>=0.13,<0.14", "zstandard>=0.25,<1"]
# ///
import argparse
from concurrent.futures import ThreadPoolExecutor
from functools import lru_cache
import hashlib
import io
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import subprocess
import tempfile
import tomllib

try:
    from .fixture_emitters import FIXTURE_SURFACES
    from .content_contract import validate_content, validate_rounds, validate_player_health
    from .trigger_volumes import validate as validate_trigger_volumes
    from .brush_entities import validate as validate_brush_entities, validate_scene as validate_brush_scene
    from .prepare_sky import SKY_PATH, validate_sky
    from .prepare_atmosphere import GRADE_PATH, validate_grade
    from .environment_content import validate as validate_environment
except ImportError:
    from fixture_emitters import FIXTURE_SURFACES
    from content_contract import validate_content, validate_rounds, validate_player_health
    from trigger_volumes import validate as validate_trigger_volumes
    from brush_entities import validate as validate_brush_entities, validate_scene as validate_brush_scene
    from prepare_sky import SKY_PATH, validate_sky
    from prepare_atmosphere import GRADE_PATH, validate_grade
    from environment_content import validate as validate_environment

import numpy as np

WORKSPACE = Path(__file__).resolve().parents[1]
POINTER = b"version https://git-lfs.github.com/spec/v1"
RUNTIME_SUFFIXES = {".json", ".gltf", ".bin", ".ktx2", ".ogg", ".ttf", ".otf"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


@lru_cache(maxsize=65536)
def texture_layout(path, modified, size):
    try:
        from .ktx2_textures import inspect_ktx2
    except ImportError:
        from ktx2_textures import inspect_ktx2
    info = inspect_ktx2(read_bytes(path))
    require(info['dimension'] == '2d' and info['surfaces'] == 1,
            'runtime material and UI images must be single 2D textures')
    return info


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


def hash_closure(root, names):
    def hashed(name):
        with safe_path(root, name, RUNTIME_SUFFIXES).open('rb') as stream:
            return name, hashlib.file_digest(stream, 'sha256').hexdigest()
    with ThreadPoolExecutor(max_workers=8) as pool:
        return dict(pool.map(hashed, sorted(names)))


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
        require(not value.startswith("data:image/") and
                not re.search(r"\.(?:dds|png|jpe?g|basis|tga|bmp|webp|hdr|exr|tiff?)(?:#.*)?$", value, re.IGNORECASE),
                "non-KTX2 runtime image reference is forbidden")
        pattern = r"(?<![A-Za-z0-9_])[A-Za-z]:[\\/]|(?:^|\s)/(?:Users|home|mnt|tmp)/|file://"
        if not localized_text:
            pattern += r"|\\"
        require(not re.search(pattern, value),
                "absolute or machine-specific path in JSON")
        require(localized_text or not value.startswith("/"), "absolute path in JSON")
    elif isinstance(value, float):
        require(math.isfinite(value), "non-finite JSON number")


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


def validate_gltf(root, relative, files, *, allow_native_transforms=False):
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
                require(len(accessor[key]) == width and all(abs(a-b) <= 1e-7 + 1e-6 * abs(b)
                        for a, b in zip(accessor[key], actual.tolist())),
                        f"accessor {key} mismatch")
        arrays.append(values)
    for image in document.get("images", []):
        require(set(image) <= {"uri", "name"}, "embedded or extended images unsupported")
        suffix = PurePosixPath(image["uri"]).suffix
        require(suffix == ".ktx2", "every runtime glTF image must use KTX2")
        image_path = dependency(image["uri"], suffix)
        stat = image_path.stat()
        texture_layout(image_path, stat.st_mtime_ns, stat.st_size)
    for texture in document.get("textures", []):
        require(set(texture) <= {"source", "sampler", "name"}, "unsupported texture feature")
        ref(document.get("images", []), texture["source"], "image")
        if "sampler" in texture:
            ref(document.get("samplers", []), texture["sampler"], "sampler")
    extras = document.get("extras", {})
    require(set(extras) <= {'native_animation'}, "document lighting is retired; prepare a realtime pack")
    if 'native_animation' in extras:
        native = extras['native_animation']
        require(isinstance(native, dict) and set(native) == {'source_sha256', 'sources', 'asset_type', 'looped', 'duration', 'channel_presence', 'notifications'},
                'invalid native animation metadata')
        require(re.fullmatch(r'[0-9a-f]{64}', native['source_sha256']) is not None
                and type(native['asset_type']) is int and type(native['looped']) is bool
                and type(native['duration']) in (int, float) and native['duration'] > 0,
                'invalid native animation identity or timeline')
        require(isinstance(native['sources'], list) and native['sources']
                and all(isinstance(s, dict) and set(s) == {'zone', 'name'}
                        and all(isinstance(v, str) and v for v in s.values()) for s in native['sources']),
                'missing native animation source declarations')
        require(isinstance(native['channel_presence'], dict)
                and all(isinstance(v, dict) and set(v) == {'rotation', 'translation'}
                        and all(type(b) is bool for b in v.values()) for v in native['channel_presence'].values()),
                'invalid native animation sparse channels')

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
            require({"identity", "family", "channels"} <= set(surface) <= {"identity", "family", "channels", "preparation", "presentation", "layers", "effect", "version", "physical_class", "coating_roughness", "provenance", "native_weapon"}, "invalid surface metadata")
            require(surface.get("version") == 2, "prepare version-2 PBR materials before runtime import")
            native = surface.get("native_weapon")
            if native is not None:
                require({"kind", "family", "constants", "source_zone", "source_sha256", "material_identity", "bindings"} <= set(native), "invalid native weapon metadata")
                require(native["kind"] in {"lit", "cloth", "skin", "transparent", "unlit", "ember", "phong", "badge", "staff", "lens", "reticle", "camo"}, "unsupported native weapon kind")
                require(all(isinstance(native[key], str) and native[key] for key in ("family", "source_zone", "material_identity")), "invalid native weapon source")
                require(re.fullmatch(r"[0-9a-f]{64}", native["source_sha256"]) is not None, "invalid native material hash")
                require(isinstance(native["constants"], dict) and all(isinstance(name, str) and name and isinstance(values, list) and 1 <= len(values) <= 4 and all(type(value) in (int, float) and math.isfinite(value) for value in values) for name, values in native["constants"].items()), "invalid native weapon constants")
                require(isinstance(native["bindings"], list), "invalid native weapon binding table")
                for binding in native["bindings"]:
                    require(type(binding.get("active")) is bool and isinstance(binding.get("image"), str) and isinstance(binding.get("semantic"), str) and type(binding.get("hash")) is int and type(binding.get("sampler")) is int and 0 <= binding["sampler"] <= 255 and isinstance(binding.get("samplerState"), dict), "invalid retained native weapon binding")
                    if binding["active"]:
                        require(re.fullmatch(r"[0-9a-f]{64}", binding.get("sha256", "")) is not None and isinstance(binding.get("source_zone"), str), "invalid native texture provenance")
            if surface["version"] == 2:
                require(surface["physical_class"] in {"concrete", "stone", "mud", "wood", "fabric", "skin", "glass", "painted_metal", "rust", "metal", "water", "unclassified"}, "invalid physical material class")
                require(isinstance(surface["coating_roughness"], (int, float)) and 0 <= surface["coating_roughness"] <= 1, "invalid coating roughness")
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
                require(surface["family"] == "tile_blend" and surface["preparation"] == {"kind": "tile_macro_layers", "tiles": 4}, "invalid material preparation")
            if "effect" in surface and native is None:
                def effect_number(value):
                    return type(value) in (int, float) and math.isfinite(value) and abs(value) <= np.finfo(np.float32).max
                effect = surface["effect"]
                effect_family = surface["provenance"]["source_family"]
                require(surface["family"] == "unlit" and "unlit_3layer_nofog_" in effect_family and "zfeather" not in effect_family, "unsupported effect family")
                require(set(effect) == {"kind", "layers", "coverage_layers", "hdr", "falloff", "reflection", "gloss"} and effect["kind"] == "native_unlit_3layer", "invalid effect metadata")
                require(len(effect["layers"]) == 3 and len(effect["coverage_layers"]) == 3 and all(type(v) is int and 0 <= v < 3 for v in effect["coverage_layers"]), "invalid effect layers")
                require(len(effect["falloff"]) == 2 and all(effect_number(v) and v >= 0 for v in [effect["hdr"], effect["reflection"], effect["gloss"], *effect["falloff"]]), "invalid effect response")
                for layer in effect["layers"]:
                    require(set(layer) == {"scale", "origin", "offset", "scroll", "rotation", "alpha", "depth"}, "invalid effect layer")
                    require(all(len(layer[k]) == 2 and all(effect_number(v) for v in layer[k]) for k in ("scale", "origin", "offset", "scroll", "rotation")), "invalid effect UV transform")
                    require(all(effect_number(layer[k]) for k in ("alpha", "depth")) and layer["alpha"] >= 0, "invalid effect coverage/depth")
                require(all(("color", i) in {(c["role"], c["layer"]) for c in surface["channels"]} for i in range(3)), "missing effect color binding")
            require(surface["identity"] == material.get("name") and isinstance(surface["family"], str), "invalid material identity")
            bindings = set()
            for channel in surface["channels"]:
                binding = (channel.get("role"), channel.get("layer"))
                require(binding not in bindings, "duplicate material channel/layer")
                bindings.add(binding)
                require(set(channel) == {"uri", "role", "color_space", "format", "width", "height", "mip_count", "layer", "channels", "sampler"}, "invalid PBR texture metadata")
                require(type(channel["sampler"]) is int and 0 <= channel["sampler"] <= 255, "invalid native sampler bits")
                role = channel["role"]
                if native is not None:
                    try:
                        from .prepare_native_weapons import CHANNELS, FORMATS
                    except ImportError:
                        from prepare_native_weapons import CHANNELS, FORMATS
                    require(role in CHANNELS and channel["channels"] == CHANNELS[role], "invalid native texture interpretation")
                    require(type(channel["layer"]) is int and 0 <= channel["layer"] <= 31 and channel["color_space"] == "linear", "invalid native texture layer or transfer")
                    uri = channel["uri"]
                    texture = safe_path(root, uri, {".ktx2"})
                    files.add(uri)
                    stat = texture.stat()
                    dds = texture_layout(texture, stat.st_mtime_ns, stat.st_size)
                    require(FORMATS.get(dds["dxgi"]) == channel["format"], "native KTX2 format mismatch")
                    require((dds["width"], dds["height"], dds["mip_count"]) == (channel["width"], channel["height"], channel["mip_count"]), "native KTX2 layout mismatch")
                    retained = [b for b in native["bindings"] if b["active"] and (b.get("role"), b.get("layer")) == binding]
                    require(len(retained) == 1 and retained[0]["uri"] == uri and retained[0]["sha256"] == dds["sha256"] and retained[0]["sampler"] == channel["sampler"], "native KTX2 byte or sampler identity mismatch")
                    continue
                require(role in {"color", "normal", "response", "control", "emissive", "coating", "reflectance"}, "invalid PBR role")
                require(channel["channels"] == {"color": "rgb:base_color,a:coverage",
                    "normal": "rg:unit_normal_xy", "response": "r:occlusion,g:perceptual_roughness,b:metallic,a:coating",
                    "control": "rg:core_coating_weights", "emissive": "rgb:emissive,a:unused", "coating": "r:coating", "reflectance": "rgb:dielectric_reflectance,a:unused"}[role], "invalid PBR channel interpretation")
                require(type(channel["layer"]) is int and 0 <= channel["layer"] <= 31, "invalid PBR layer")
                require(channel["color_space"] == ("srgb" if role in {"color", "emissive"} else "linear"), "PBR color-space mismatch")
                uri = channel["uri"]
                texture = safe_path(root, uri, {".ktx2"})
                files.add(uri)
                stat = texture.stat()
                dds = texture_layout(texture, stat.st_mtime_ns, stat.st_size)
                expected_formats = {"BC7_UNORM_SRGB": 99, "BC7_UNORM": 98, "BC5_UNORM": 83,
                                    "BC4_UNORM": 80, "R8_UNORM": 61, "R8G8_UNORM": 49, "R8G8B8A8_UNORM_SRGB": 29, "R8G8B8A8_UNORM": 28}
                require(channel["format"] in expected_formats and dds["dxgi"] == expected_formats[channel["format"]], "PBR KTX2 format mismatch")
                allowed = {"normal": {83, 49, 28}, "coating": {80, 61, 28}}.get(role,
                    {99, 29} if role in {"color", "emissive"} else {98, 28})
                require(dds["dxgi"] in allowed, "PBR role/format mismatch")
                require((dds["width"], dds["height"], len(dds["mips"])) == (channel["width"], channel["height"], channel["mip_count"]), "PBR KTX2 layout mismatch")
                require(len(dds["mips"]) == max(dds["width"], dds["height"]).bit_length(), "incomplete PBR mip chain")
                require((role in {"color", "emissive"}) == (dds["dxgi"] in {29, 99}), "PBR KTX2 transfer mismatch")
            if "preparation" in surface:
                bindings = {(c["role"], c["layer"]) for c in surface["channels"]}
                require({("color", 0), ("color", 1), ("control", 2)} <= bindings, "tile material lacks intrinsic layers/control")
            if native is not None:
                require(bindings == {(b.get("role"), b.get("layer")) for b in native["bindings"] if b["active"]}, "native channel table mismatch")
    for mesh in document.get("meshes", []):
        require(set(mesh) <= {"name", "primitives", "extras"} and mesh["primitives"], "unsupported or empty mesh")
        metadata = mesh.get("extras", {})
        require(not metadata, "baked mesh lighting is retired")
        for primitive in mesh["primitives"]:
            require(set(primitive) <= {"attributes", "indices", "mode", "material"}, "unsupported primitive")
            require(primitive.get("mode", 4) == 4, "non-triangle primitive")
            position = ref(arrays, primitive["attributes"]["POSITION"], "position accessor")
            require(position.shape[1] == 3, "invalid position accessor")
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
                    require((weights >= 0).all() and (weights <= 1).all() and (np.abs(weights.sum(axis=1) - 1) <= 1.1e-4).all(), "invalid skin weight normalization")
                if semantic == "TANGENT":
                    require((np.abs(np.linalg.norm(values[:, :3], axis=1) - 1) <= 2e-5).all()
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
        allowed_extras = {"static_instance", "source_model", "fixture_emitter", "brush_entity", "prop_lod", "mesh_lod"}
        if allow_native_transforms:
            allowed_extras.update({"origins_native_rotation", "origins_native_translation"})
            for key, width in (("origins_native_rotation", 4), ("origins_native_translation", 3)):
                if key in extras:
                    require(isinstance(extras[key], list) and len(extras[key]) == width
                            and all(type(v) in (int, float) and math.isfinite(v) for v in extras[key]),
                            "invalid retained native joint transform")
        require(set(extras) <= allowed_extras, "baked node lighting is retired")
        if 'mesh_lod' in extras:
            lod = extras['mesh_lod']
            require(isinstance(lod, dict) and set(lod) == {'level', 'distances'}
                    and type(lod['level']) is int and isinstance(lod['distances'], list)
                    and 1 <= len(lod['distances']) <= 3 and 0 <= lod['level'] <= len(lod['distances'])
                    and all(type(v) in (int, float) and math.isfinite(v) and v >= 0 for v in lod['distances'])
                    and all(b >= a for a, b in zip(lod['distances'], lod['distances'][1:])),
                    'invalid authored mesh LOD metadata')
        if "prop_lod" in extras:
            lod = extras["prop_lod"]
            require(isinstance(lod, dict) and set(lod) == {"level", "levels"}
                    and type(lod["level"]) is int and type(lod["levels"]) is int
                    and 2 <= lod["levels"] <= 3 and 0 <= lod["level"] < lod["levels"]
                    and extras.get("static_instance") is True and extras.get("fixture_emitter") is False,
                    "invalid static prop LOD metadata")
        if "static_instance" in extras:
            require(extras["static_instance"] is True, "invalid static instance marker")
            require(isinstance(extras.get("source_model"), str) and re.fullmatch(r"[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*", extras["source_model"]), "missing or invalid fixture model identity")
            require(type(extras.get("fixture_emitter")) is bool, "missing or invalid fixture emitter ownership")
            if extras["fixture_emitter"]:
                mesh = ref(document.get("meshes", []), node.get("mesh"), "fixture mesh")
                require(len(mesh["primitives"]) == 1, "fixture emitter must own one prepared surface")
                material = document["materials"][mesh["primitives"][0]["material"]]
                require(material.get("extras", {}).get("surface", {}).get("presentation", {}).get("lamp") is True, "fixture emitter lost its emitting surface")
                require(FIXTURE_SURFACES.get(extras["source_model"]) == material.get("name"), "fixture emitter does not match its source model")
        else:
            require("source_model" not in extras and "fixture_emitter" not in extras, "fixture metadata requires a static instance")
        if "brush_entity" in extras:
            require(isinstance(extras["brush_entity"],str) and re.fullmatch(r"[A-Za-z0-9_.-]+",extras["brush_entity"]), "invalid brush entity marker")
        for child in node.get("children", []):
            ref(nodes, child, "child node")
    skins = document.get("skins", [])
    assembly_globals = None
    if document.get("asset", {}).get("generator", "").startswith("Origins native weapon assembly"):
        assembly_globals = {}
        def joint_matrix(node):
            if "matrix" in node:
                return np.asarray(node["matrix"]).reshape(4,4).T
            x,y,z,w = node.get("rotation", [0,0,0,1])
            result = np.eye(4)
            result[:3,:3] = [[1-2*(y*y+z*z),2*(x*y-z*w),2*(x*z+y*w)],
                [2*(x*y+z*w),1-2*(x*x+z*z),2*(y*z-x*w)],
                [2*(x*z-y*w),2*(y*z+x*w),1-2*(x*x+y*y)]]
            result[:3,:3] *= np.asarray(node.get("scale",[1,1,1]))
            result[:3,3] = node.get("translation",[0,0,0])
            return result
        def assembled_node(index, parent):
            require(index not in assembly_globals, "multiply parented assembly node")
            assembly_globals[index] = parent @ joint_matrix(nodes[index])
            for child in nodes[index].get("children",[]):
                assembled_node(child,assembly_globals[index])
        for index in document["scenes"][document.get("scene",0)]["nodes"]:
            assembled_node(index,np.eye(4))
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
            if assembly_globals is not None:
                require(all(j in assembly_globals and np.allclose(assembly_globals[j] @ matrix.reshape(4,4).T,np.eye(4),atol=2e-5)
                            for j,matrix in zip(joints,matrices)), "assembled weapon bind pose mismatch")
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
                require((np.abs(np.linalg.norm(quaternions, axis=1) - 1) <= 1.1e-4).all(), "invalid animation quaternion")
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


def validate_tuning(tuning):
    scalars = {"walk_speed": 4.8, "sprint_speed": 7.2, "sprint_seconds": 4.,
        "sprint_minimum": .25, "sprint_recovery_rate": 1., "sprint_recovery_delay": 0.,
        "crouch_speed": 2.4, "gravity": 20., "jump_speed": 6., "step_height": .45,
        "radius": .35, "standing_height": 1.8, "crouch_height": 1.15, "eye_height": 1.62,
        "mouse_sensitivity": .002, "ads_mouse_sensitivity": .0012, "fire_interval": .2,
        "reload_seconds": 1.5, "zombie_speed": 1.15}
    integers = {"pistol_damage": (50, 1, 2147483647), "melee_damage": (150, 1, 2147483647),
        "zombie_damage": (50, 1, 2147483647), "magazine": (8, 1, 4294967295),
        "reserve": (32, 0, 4294967295)}
    require(isinstance(tuning, dict) and set(tuning) <= scalars.keys() | integers.keys() | {"combat_points", "mobility"},
            "unknown gameplay tuning")

    def number(value, key, nonnegative=False):
        require(type(value) in (int, float), f"invalid tuning number: {key}")
        try:
            with np.errstate(over="ignore", under="ignore", invalid="ignore"):
                value = np.float32(value)
        except (OverflowError, ValueError):
            raise ValueError(f"invalid tuning number: {key}") from None
        require(np.isfinite(value) and (value >= 0 if nonnegative else value > 0),
                f"invalid tuning number: {key}")
        return value

    values = {key: number(tuning.get(key, default), key, key == "sprint_recovery_delay")
              for key, default in scalars.items()}
    for key, (default, minimum, maximum) in integers.items():
        value = tuning.get(key, default)
        require(type(value) is int and minimum <= value <= maximum, f"invalid tuning integer: {key}")
        values[key] = value
    points = tuning.get("combat_points", [10, 50, 50, 10, 80])
    require(isinstance(points, list) and len(points) == 5 and
            all(type(value) is int and 0 <= value <= 2147483647 for value in points), "invalid combat point tuning")
    values["combat_points"] = points
    require(values["sprint_minimum"] <= values["sprint_seconds"], "sprint minimum exceeds capacity")
    with np.errstate(over="ignore"):
        diameter = np.float32(2.) * values["radius"]
    require(values["standing_height"] > diameter and values["crouch_height"] > diameter and
            values["crouch_height"] <= values["standing_height"], "invalid player capsule heights")

    mobility_scalars = {"prone_height": .75, "prone_eye": .3048, "prone_speed": 1.1,
        "prone_step": .12, "acceleration": 40., "deceleration": 50., "air_acceleration": 8.,
        "air_deceleration": 2., "stance_seconds": .18, "jump_recovery": .25,
        "dive_speed": 8., "dive_lift": 3.2, "dive_recovery": .55, "fall_minimum": 3.2512,
        "fall_fatal": 7.62, "landing_dip": .12, "landing_seconds": .3,
        "sway_limit": .04, "sway_response": 14.}
    feedback = {"walk_bob": [.016, .024], "sprint_bob": [.025, .04], "stride": [2.4, 3.]}
    mobility = tuning.get("mobility", {})
    require(isinstance(mobility, dict) and set(mobility) <= mobility_scalars.keys() | feedback.keys(),
            "unknown mobility tuning")
    movement = {key: number(mobility.get(key, default), key) for key, default in mobility_scalars.items()}
    for key, default in feedback.items():
        value = mobility.get(key, default)
        require(isinstance(value, list) and len(value) == 2, f"invalid movement feedback tuning: {key}")
        movement[key] = [number(item, key) for item in value]
    require(diameter < movement["prone_height"] < values["crouch_height"], "invalid prone capsule")
    require(movement["prone_eye"] < movement["prone_height"], "invalid prone eye")
    require(movement["fall_minimum"] < movement["fall_fatal"], "unordered fall thresholds")
    values["mobility"] = movement
    return values


def validate_weapon_camos(root, files):
    path = root / "weapon_camos.json"
    if not path.is_file():
        return
    library = read_json(path)
    require(library.get("version") == 1 and library.get("camo_id") == 45, "invalid Origins camo library")
    files.add("weapon_camos.json")
    game = read_json(root / "game.json")["content"]
    weapons = {w["id"] for w in game["weapons"]}
    require(set(library["weapons"]) <= weapons, "unknown camo weapon")
    require({i for m in library["weapons"].values() for i in m.values()} == set(library["surfaces"]), "unreferenced or missing camo surface")
    for identity, surface in library["surfaces"].items():
        require(surface["identity"] == identity and surface["version"] == 2, "invalid camo identity")
        native = surface["native_weapon"]
        require(native["kind"] == "camo" and native["camo_id"] == 45, "invalid camo source")
        require(len(native["shader_consts"]) == 8 and all(type(v) in (int,float) and math.isfinite(v) for v in native["shader_consts"]), "invalid camo constants")
        require(set(native["replace_flags"]) == {"color","normal","specular"} and all(type(v) is bool for v in native["replace_flags"].values()), "invalid camo flags")
        for channel in surface["channels"]:
            uri = channel["uri"]
            raw = read_bytes(safe_path(root,uri,{".ktx2"}))
            try:
                from .prepare_native_weapons import CHANNELS, FORMATS
                from .ktx2_textures import inspect_ktx2
            except ImportError:
                from prepare_native_weapons import CHANNELS, FORMATS
                from ktx2_textures import inspect_ktx2
            image = inspect_ktx2(raw)
            require(channel["color_space"] == "linear" and channel["channels"] == CHANNELS[channel["role"]], "invalid camo channel transfer")
            require(channel["format"] == FORMATS[image["dxgi"]] and (channel["width"],channel["height"],channel["mip_count"]) == (image["width"],image["height"],image["mip_count"]), "invalid camo KTX2 metadata")
            bindings = [b for b in native["bindings"] if b["active"] and b["role"] == channel["role"] and b["layer"] == channel["layer"]]
            require(len(bindings) == 1 and bindings[0]["sha256"] == image["sha256"] and bindings[0]["uri"] == uri and bindings[0]["sampler"] == channel["sampler"], "camo KTX2 identity mismatch")
            files.add(uri)


def validate_pack(root, receipt=None, exact=False, progress=False):
    root = root.resolve()
    try:
        from .prepare_ktx2 import TEXTURES
    except ImportError:
        from prepare_ktx2 import TEXTURES
    for path in root.rglob('*'):
        if path.is_file():
            require(path.suffix.lower() not in TEXTURES - {'.ktx2'},
                    f"non-KTX2 runtime texture is forbidden: {path.relative_to(root)}")
            require(path.suffix.lower() != '.glb', "runtime model containers must externalize their KTX2 images")
    files = {"game.json", SKY_PATH, GRADE_PATH}
    validate_sky(read_bytes(safe_path(root, SKY_PATH, {".ktx2"})))
    validate_grade(read_bytes(safe_path(root, GRADE_PATH, {".ktx2"})))
    game = read_json(safe_path(root, "game.json", {".json"}))
    require(set(game) <= {"version", "scene", "collision_scene", "spawn", "yaw", "spawn_points",
                          "waypoints", "survey_views", "source_surfaces", "render_meshes",
                          "collision_meshes", "static_model_instances", "tuning", "content", "rounds", "player_health", "solo_revive", "environment", "access", "audio"}, "unknown gameplay configuration")
    audio=game.get('audio',{})
    require(isinstance(audio,dict) and set(audio)<={'aliases','bindings','timelines','emitters','character','surfaces'},'unknown audio configuration')
    aliases=audio.get('aliases',{});sounds_by_id={s['id']:s for s in game.get('content',{}).get('sounds',[])}
    require(isinstance(aliases,dict) and len(aliases)<=8192,'invalid audio aliases')
    require(type(audio.get('character',0)) is int and 0<=audio.get('character',0)<4,'invalid voice character')
    for name,a in aliases.items():
        require(isinstance(name,str) and name and set(a)=={'sounds','spatial','channel','subtitle','limit','priority','seconds','secondary'},'invalid audio alias fields')
        require(a['secondary'] is None or a['secondary'] in aliases,'missing secondary audio')
        require(a['sounds'] and all(s in sounds_by_id for s in a['sounds']),'missing audio variant')
        require(type(a['spatial']) is bool and a['channel'] in ('effects','music','ambience','dialogue') and isinstance(a['subtitle'],str),'invalid audio routing')
        require(type(a['limit']) is int and 1<=a['limit']<=32 and type(a['priority']) is int and a['priority']>=0,'invalid audio admission')
        require(type(a['seconds']) in (int,float) and math.isfinite(a['seconds']) and a['seconds']>0,'invalid audio duration')
    for name in aliases:
        seen=set();current=name
        while current is not None:
            require(current not in seen,'cyclic secondary audio');seen.add(current)
            current=aliases[current]['secondary']
    require(isinstance(audio.get('surfaces',{}),dict) and all(s in ('mud','dirt','wood','metal','brick','concrete','rock','gravel','grass','water','default') for s in audio.get('surfaces',{}).values()),'invalid audio surfaces')
    require(all(n in aliases for n in audio.get('bindings',{}).values()),'missing sound binding')
    animations={a['id']:a for a in game.get('content',{}).get('animations',[])}
    for key,notes in audio.get('timelines',{}).items():
        require(key in animations and isinstance(notes,list),'missing audio animation')
        for n in notes:
            require(set(n)=={'seconds','alias'} and n['alias'] in aliases and type(n['seconds']) in (int,float) and math.isfinite(n['seconds']) and 0<=n['seconds']<=max(a['seconds'] for a in animations[key]['notifications'])+.001,'invalid animation audio note')
    require(len(audio.get('emitters',[]))<=4096,'too many ambient emitters')
    for e in audio.get('emitters',[]):
        require(set(e)=={'alias','position'} and e['alias'] in aliases and len(e['position'])==3 and all(type(v) in (int,float) and math.isfinite(v) for v in e['position']),'invalid ambient emitter')
    tuning = validate_tuning(game.get("tuning", {}))
    environment = game.get("environment", {})
    require(isinstance(environment,dict) and set(environment)<={"mud","triggers","brush_entities","ownership","parts","digging"},"unknown environment fields")
    validate_environment(environment, game['content'], require)
    validate_trigger_volumes(environment.get("triggers", []), require)
    validate_brush_entities(environment.get("brush_entities", []), require)
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
    access=game.get("access",{})
    require(isinstance(access,dict) and set(access)<={"doors","volumes","spawns"},"unknown access fields")
    doors=access.get("doors",[]);require(isinstance(doors,list) and len(doors)<=256,"invalid door count")
    content=game.get("content",{});connections={c["id"] for c in content.get("zone_connections",[])}
    sounds={s["id"] for s in content.get("sounds",[]) if not s["looping"]}
    presentations={p["id"] for p in content.get("presentations",[]) if p["scene"] is not None}
    groups=set();volume_ids=set()
    for d in doors:
        require(isinstance(d,dict) and set(d)=={"connection","connections","debris","seconds","props","blocks","sound"},"invalid door fields")
        require(d["connection"] not in groups and d["connection"] in connections,"invalid door identity");groups.add(d["connection"])
        require(isinstance(d["connections"],list) and d["connection"] in d["connections"] and all(c in connections for c in d["connections"]),"invalid door edges")
        require(type(d["debris"]) is bool and type(d["seconds"]) in (int,float) and math.isfinite(d["seconds"]) and 0<d["seconds"]<=10,"invalid door timing")
        require(d["sound"] in sounds,"missing door sound")
        require(isinstance(d["props"],list) and 0<len(d["props"])<=64,"invalid door props")
        for p in d["props"]:
            require(isinstance(p,dict) and set(p)=={"presentation","position","yaw","offset"} and p["presentation"] in presentations,"invalid door visual")
            require(type(p["yaw"]) in (int,float) and math.isfinite(p["yaw"]),"invalid door yaw")
            require(all(isinstance(v,list) and len(v)==3 and all(type(x) in (int,float) and math.isfinite(x) for x in v) for v in [p["position"],p["offset"]]),"invalid door vectors")
        require(isinstance(d["blocks"],list) and 0<len(d["blocks"])<=256,"invalid door blocks")
        for v in d["blocks"]:
            require(isinstance(v,dict) and set(v)=={"id","position","vertices"},"invalid door volume")
            require(isinstance(v["id"],str) and re.fullmatch(r"[A-Za-z0-9_.-]+",v["id"]) and v["id"] not in volume_ids,"invalid blocker ID");volume_ids.add(v["id"])
            require(isinstance(v["vertices"],list) and 4<=len(v["vertices"])<=128,"invalid blocker vertices")
            require(all(isinstance(p,list) and len(p)==3 and all(type(x) in (int,float) and math.isfinite(x) for x in p) for p in [v["position"]]+v["vertices"]),"invalid blocker geometry")
            require(np.linalg.matrix_rank(np.array(v["vertices"])-v["vertices"][0])==3,"flat blocker")
    zones={z["id"] for z in content.get("zones",[])}
    zone_volumes=access.get("volumes",[]);require(isinstance(zone_volumes,list) and len(zone_volumes)<=4096,"invalid zone volumes")
    for row in zone_volumes:
        require(isinstance(row,dict) and set(row)=={"zone","volume"} and row["zone"] in zones,"invalid zone volume identity")
        v=row["volume"];require(isinstance(v,dict) and set(v)=={"id","position","vertices"},"invalid zone hull")
        require(isinstance(v["id"],str) and re.fullmatch(r"[A-Za-z0-9_.-]+",v["id"]) and v["id"] not in volume_ids,"invalid zone brush ID");volume_ids.add(v["id"])
        require(isinstance(v["vertices"],list) and 4<=len(v["vertices"])<=128,"invalid zone points")
        require(all(isinstance(p,list) and len(p)==3 and all(type(x) in (int,float) and math.isfinite(x) for x in p) for p in [v["position"]]+v["vertices"]),"invalid zone geometry")
        require(np.linalg.matrix_rank(np.array(v["vertices"])-v["vertices"][0])==3,"flat zone volume")
    spawn_ids=set();spawns=access.get("spawns",[]);require(isinstance(spawns,list) and len(spawns)<=4096,"invalid spawn count")
    for row in spawns:
        require(isinstance(row,dict) and set(row)=={"id","zone","position","rising"},"invalid spawn fields")
        require(isinstance(row["id"],str) and re.fullmatch(r"[A-Za-z0-9_.-]+",row["id"]) and row["id"] not in spawn_ids,"invalid spawn ID");spawn_ids.add(row["id"])
        require(row["zone"] in zones and type(row["rising"]) is bool,"invalid spawn zone/kind")
        require(isinstance(row["position"],list) and len(row["position"])==3 and all(type(v) in (int,float) and math.isfinite(v) for v in row["position"]),"invalid spawn origin")
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
    with np.errstate(over="ignore"):
        downed_height = np.float32(solo["downed_height"])
    require(np.float32(2.) * tuning["radius"] <= downed_height < tuning["standing_height"], "invalid downed capsule height")
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
    validate_brush_scene(environment.get("brush_entities", []), documents[game["scene"].split("#",1)[0]][0], require)
    map_documents = dict(documents)
    map_files = set(files)
    gltfs = {reference.split('#', 1)[0] for reference in dependencies
             if reference.split('#', 1)[0].endswith('.gltf')} - documents.keys()
    def validated_gltf(name):
        closure = set()
        try:
            document = validate_gltf(root, name, closure, allow_native_transforms=name.startswith('library/models/'))
        except ValueError as error:
            raise ValueError(name + ': ' + str(error)) from error
        return name, document, closure
    with ThreadPoolExecutor(max_workers=8) as pool:
        for index, (name, document, closure) in enumerate(pool.map(validated_gltf, sorted(gltfs)), 1):
            documents[name] = document
            files.update(closure)
            if progress and index % 1000 == 0:
                print(f'Validated glTF assets: {index}/{len(gltfs)}', flush=True)
    for reference in dependencies:
        name, _, label = reference.partition("#")
        dependency = safe_path(root, name, RUNTIME_SUFFIXES - {".json", ".bin"})
        suffix = dependency.suffix
        if suffix == ".gltf":
            if name not in documents:
                try:
                    documents[name] = validate_gltf(root, name, files, allow_native_transforms=name.startswith('library/models/'))
                except ValueError as error:
                    raise ValueError(name + ': ' + str(error)) from error
            if label:
                match = re.fullmatch(r"(Scene|Animation)(0|[1-9][0-9]*)", label)
                require(match is not None, "invalid glTF content label")
                doc, scenes = documents[name]
                ref(scenes if match[1] == "Scene" else doc.get("animations", []), int(match[2]), "glTF content label")
        else:
            require(not label, "unexpected media asset label")
            read_bytes(dependency)
            files.add(name)
            if suffix == ".ktx2":
                stat = dependency.stat()
                texture_layout(dependency, stat.st_mtime_ns, stat.st_size)
            elif suffix == ".ogg":
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
    texture_count = sum(name.endswith(".ktx2") for name in map_files)
    if receipt is not None:
        require(texture_count == receipt["textures"] + receipt["model_textures"], "texture receipt count mismatch")
        require(sum(name.endswith(".ktx2") and PurePosixPath(name).name.startswith("model-") for name in map_files)
                == receipt["model_textures"], "model texture receipt count mismatch")
    validate_weapon_camos(root, files)
    if exact:
        actual = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file()}
        require(actual == files, f"unexpected or missing runtime files: {sorted(actual ^ files)}")
    hashes = hash_closure(root, files)
    return game, documents, hashes


def external(path):
    path = Path(path).resolve()
    require(path != WORKSPACE and WORKSPACE not in path.parents, "preparation inputs must remain external")
    return path


def reconcile_sources(config, receipt, documents):
    from fixture_emitters import emitter_surface
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
    nodes = [n for n in render_doc["nodes"] if n.get("name") == "static_instance"]
    providers = {}
    fixture_surfaces = {}
    cursor = 0
    basis = np.array([[1, 0, 0], [0, 0, 1], [0, -1, 0]], dtype=float)
    for item in instances:
        name = item["model"]
        if name not in providers:
            model = json.loads(read_bytes(safe_path(model_root, f"model_export/{name}_lod0.gltf", {".gltf"})))
            fixture_surfaces[name] = [{"material": model["materials"][primitive["material"]]["name"]}
                                      for mesh in model["meshes"] for primitive in mesh["primitives"]]
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
        require(all(n.get("extras", {}).get("static_instance") is True for n in placed), "static instance marker lost")
        emitter = emitter_surface(name, fixture_surfaces[name])
        require(all(n["extras"]["source_model"] == name and n["extras"]["fixture_emitter"] == (index == emitter)
                    for index, n in enumerate(placed)), "fixture emitter ownership differs from source")
        cursor += len(shared)
    require(cursor == len(nodes), "extra model placements")
    all_meshes = [mesh for group in providers.values() for mesh in group]
    require(len(set(all_meshes)) == len(all_meshes), "distinct source providers unexpectedly merged")


def import_pack(source, destination, receipt, check=False, validated_hashes=None, predecessor_identity=None):
    source, destination = source.resolve(), destination.resolve()
    require(source != destination and source not in destination.parents and destination not in source.parents,
            "source and destination overlap")
    if validated_hashes is None:
        _, _, hashes = validate_pack(source, receipt, exact=receipt is None)
    else:
        hashes = validated_hashes
    require(hash_closure(source, hashes) == hashes, "qualified source changed before import")
    retired = set()
    if not check:
        if destination.exists():
            existing = {p.relative_to(destination).as_posix() for p in destination.rglob("*") if p.is_file()}
            retired = existing - hashes.keys()
            if retired:
                if predecessor_identity is None:
                    _, _, previous = validate_pack(destination, None, exact=True)
                    require(existing == previous.keys(), "destination contains unrelated files")
                else:
                    previous = json.loads(predecessor_identity.read_text(encoding='utf-8'))['assets']
                    require(existing == previous.keys(), "destination differs from the complete pinned predecessor")
                    for name, expected in previous.items():
                        path = safe_path(destination, name, RUNTIME_SUFFIXES | {'.dds', '.png', '.jpg', '.jpeg'})
                        raw = path.read_bytes()
                        if raw.startswith(POINTER):
                            pointer = POINTER + f"\noid sha256:{expected['sha256']}\nsize {expected['size']}\n".encode()
                            require(raw == pointer, f"predecessor LFS identity mismatch: {name}")
                        else:
                            require(len(raw) == expected['size'] and hashlib.sha256(raw).hexdigest() == expected['sha256'],
                                    f"predecessor asset changed: {name}")
        directories = {}
        for name in hashes:
            directories.setdefault(PurePosixPath(name).parent, name)
        for name in directories.values():
            safe_path(destination, name, RUNTIME_SUFFIXES).parent.mkdir(parents=True, exist_ok=True)
        def copy_runtime(name):
            target = safe_path(destination, name, RUNTIME_SUFFIXES)
            if not target.is_file() or hashlib.sha256(target.read_bytes()).hexdigest() != hashes[name]:
                descriptor, temporary = tempfile.mkstemp(prefix=f".{target.name}.import-", dir=target.parent)
                os.close(descriptor)
                try:
                    shutil.copyfile(safe_path(source, name, RUNTIME_SUFFIXES), temporary)
                    os.replace(temporary, target)
                finally:
                    if os.path.exists(temporary):
                        os.unlink(temporary)
        with ThreadPoolExecutor(max_workers=8) as pool:
            for _ in pool.map(copy_runtime, hashes):
                pass
        for name in retired:
            safe_path(destination, name, RUNTIME_SUFFIXES | {'.dds', '.png', '.jpg', '.jpeg'}).unlink()
    actual = {p.relative_to(destination).as_posix() for p in destination.rglob('*') if p.is_file()}
    require(actual == hashes.keys(), 'imported file list differs from validated source closure')
    imported = hash_closure(destination, hashes)
    require(imported == hashes, "imported file hashes differ from source")
    require(hash_closure(source, hashes) == hashes,
            "source changed during import")
    return {"runtime_files": len(hashes), "ktx2_textures": sum(n.endswith(".ktx2") for n in hashes),
            "runtime_bytes": sum((destination / n).stat().st_size for n in hashes), "sha256": hashes}


def check_index(destination, hashes, ordinary=None):
    def git(*args, data=None):
        return subprocess.run(["git", *args], input=data, cwd=WORKSPACE,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout
    prefix = destination.relative_to(WORKSPACE).as_posix() + "/"
    ordinary = {name for name in ("game.json", "weapon_camos.json") if name in hashes} if ordinary is None else set(ordinary)
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
    media = [line.removeprefix("LocalMediaDir=") for line in git("lfs", "env").decode().splitlines()
             if line.startswith("LocalMediaDir=")]
    require(len(media) == 1 and Path(media[0]).is_absolute(), "cannot resolve Git LFS storage")
    lfs_root = Path(media[0])
    verified = set()
    for name, oid in entries.items():
        header = batch.readline().decode().split()
        require(header[:2] == [oid, "blob"], "invalid staged object")
        raw = batch.read(int(header[2]))
        require(batch.read(1) == b"\n", "invalid Git batch response")
        actual_attrs = attrs[prefix + name]
        if name in ordinary:
            require(actual_attrs["filter"] in {"unspecified", "unset"} and actual_attrs["text"] == "unset",
                    "metadata must retain ordinary Git bytes")
            require(hashlib.sha256(raw).hexdigest() == hashes[name], "staged configuration hash mismatch")
        else:
            require(actual_attrs == {"filter": "lfs", "diff": "lfs", "merge": "lfs", "text": "unset"},
                    "runtime LFS attributes missing")
            size = (destination / name).stat().st_size
            expected = POINTER + f"\noid sha256:{hashes[name]}\nsize {size}\n".encode()
            require(raw == expected, "staged LFS pointer differs from validated source")
            digest = hashes[name]
            local = lfs_root / digest[:2] / digest[2:4] / digest
            if digest not in verified:
                with local.open("rb") as stream:
                    require(local.stat().st_size == size and hashlib.file_digest(stream, "sha256").hexdigest() == digest,
                            "local LFS object corrupt")
                verified.add(digest)
    return {"staged_files": len(entries), "lfs_pointers": len(entries) - len(ordinary), "ordinary_git_files": len(ordinary)}


def main():
    parser = argparse.ArgumentParser(description='Validate and copy the prepared runtime closure; never rewrite source assets.\n\nuv run tools/import_origins.py --config config/local_assets.toml\nAdd --check to validate an existing import against its external source.\nOnly the deliberately limited glTF profile emitted by prepare_origins is supported.\n')
    parser.add_argument("--config", type=Path, default=WORKSPACE / "config/local_assets.toml")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--check-index", action="store_true", help="also verify staged files and LFS objects; implies --check")
    parser.add_argument("--receipt", type=Path, help="optional external validation receipt")
    parser.add_argument("--predecessor-identity", type=Path)
    args = parser.parse_args()
    config = tomllib.loads(args.config.read_text(encoding="utf-8-sig"))
    source = external(config["game"]["asset_pack"])
    receipt = read_json(source / "preparation.json") if (source / "preparation.json").exists() else None
    _, documents, hashes = validate_pack(source, receipt, exact=receipt is None, progress=True)
    if receipt is not None:
        reconcile_sources(config, receipt, documents)
    result = import_pack(source, WORKSPACE / "assets/origins", receipt, args.check or args.check_index, hashes,
                         args.predecessor_identity)
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
