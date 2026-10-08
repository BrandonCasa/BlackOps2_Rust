# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13", "soundfile>=0.13,<0.14"]
# ///
"""Cook the existing complete pack into Bevy PBR metadata and native GPU DDS.

Original data is read only. All work/receipts stay external; import validates the
runtime closure. Source recipes and exact material classifications stay in Git.
"""
import argparse
import copy
from functools import lru_cache
import hashlib
import json
import os
from pathlib import Path
import shutil
import tempfile
import tomllib

import numpy as np
from PIL import Image

try:
    from .material_textures import canonical_normal, color_levels, inspect_dds, normal_levels, response_levels, rgba_dds, rgba_levels
    from .prepare_origins import material_family, presentation
    from .material_semantics import resolve_bindings, meanings, physical_response, uv_selection
except ImportError:
    from material_textures import canonical_normal, color_levels, inspect_dds, normal_levels, response_levels, rgba_dds, rgba_levels
    from prepare_origins import material_family, presentation
    from material_semantics import resolve_bindings, meanings, physical_response, uv_selection

WORKSPACE = Path(__file__).resolve().parents[1]
EFFECT_CONSTANT_HASHES = {
    "hdrAmount": 14836402, "FalloffPower": 3185247991, "FalloffBoost": 3173127725,
    "Reflection_Amount": 211931156, "Gloss_Amount": 3432544439,
    "Layer1Scale": 3320325482, "Layer2Scale": 3354717129, "Layer3Scale": 3398313000,
    "Layer1Origin": 3272022534, "Layer2Origin": 114353733, "Layer3Origin": 1550647684,
    "Layer1Offset": 3286424575, "Layer2Offset": 128962428, "Layer3Offset": 1565193213,
    "Layer1Scroll": 2196952607, "Layer2Scroll": 3332229724, "Layer3Scroll": 475721245,
    "Layer1Rotation": 1886964550, "Layer2Rotation": 1519946245, "Layer3Rotation": 1339484996,
    "Layer1Alpha": 3337236870, "Layer2Alpha": 3376302501, "Layer3Alpha": 3410903492,
    "Layer1Depth": 3342767871, "Layer2Depth": 3379389788, "Layer3Depth": 3416155453,
}


def prepared_effect(original, surface):
    """Retain native effect parameters without altering images or geometry."""
    if not original or "unlit_3layer_nofog_" not in original["family"] or "zfeather" in original["family"]:
        return None
    constants = {c.get("hash", EFFECT_CONSTANT_HASHES.get(c.get("name"))):
                 c.get("value", c.get("literal")) for c in original.get("constants", [])}

    def value(name, count):
        result = constants.get(EFFECT_CONSTANT_HASHES[name])
        if result is None or len(result) < count or not np.isfinite(result).all():
            raise ValueError(f"Missing native effect constant {name}")
        return result[:count] if count > 1 else result[0]

    layers = [{**{key: value(f"Layer{i}{key.title()}", 2) for key in
                 ("scale", "origin", "offset", "scroll", "rotation")},
               "alpha": value(f"Layer{i}Alpha", 1), "depth": value(f"Layer{i}Depth", 1)}
              for i in (1, 2, 3)]
    coverage = [0, 1, 2]
    # Older opaque cooking replaced base alpha with one. A second native binding
    # to the same image retains its coverage; use that existing qualified map.
    bindings = [b for b in original.get("textures", []) if b["semantic"] == 2]
    if surface["presentation"]["alpha"] == "opaque" and len(bindings) == 3:
        duplicate = next((i for i in (1, 2) if bindings[i]["image"] == bindings[0]["image"]), None)
        if duplicate is None:
            raise ValueError("Opaque effect needs a retained base-coverage binding")
        coverage[0] = duplicate
    return {"kind": "native_unlit_3layer", "layers": layers, "coverage_layers": coverage,
            "hdr": value("hdrAmount", 1),
            "falloff": [value("FalloffPower", 1), value("FalloffBoost", 1)]
                       if "falloff" in original["family"] else [0.0, 1.0],
            "reflection": value("Reflection_Amount", 1), "gloss": value("Gloss_Amount", 1)}


def retain_effects(config):
    """Enrich the external prepared pack; preserve every map and mesh byte."""
    pack = Path(config["game"]["asset_pack"]).resolve()
    if pack == WORKSPACE or WORKSPACE in pack.parents:
        raise ValueError("effect preparation must remain external")
    inspection = Path(config["assets"]["inspection_output"]).resolve()
    definitions = originals(inspection, Path(config["materials"]["extraction"]).resolve())
    candidates, changes = [], []
    for path in sorted(pack.rglob("*.gltf")):
        raw = path.read_bytes()
        document = json.loads(raw)
        identities = []
        for material in document.get("materials", []):
            surface = material.get("extras", {}).get("surface", {})
            effect = prepared_effect(definitions.get(material["name"]), surface)
            if effect is None or surface.get("effect") == effect:
                continue
            surface["effect"] = effect
            material.setdefault("extensions", {})["KHR_materials_unlit"] = {}
            document["extensionsUsed"] = sorted(set(document.get("extensionsUsed", []) + ["KHR_materials_unlit"]))
            identities.append(material["name"])
        if identities:
            updated = (json.dumps(document, separators=(",", ":")) + "\n").encode()
            candidates.append((path, raw, updated))
            changes.append({"uri": path.relative_to(pack).as_posix(), "materials": identities,
                            "before_sha256": hashlib.sha256(raw).hexdigest(),
                            "sha256": hashlib.sha256(updated).hexdigest(), "bytes": len(updated)})
    # Compute every candidate before mutation. Replace sibling entries atomically
    # so no external/source/worktree hardlink can be modified in place.
    for path, original, updated in candidates:
        if path.read_bytes() != original:
            raise ValueError(f"Prepared record changed during effect retention: {path.name}")
        descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.effect-", dir=path.parent)
        try:
            with os.fdopen(descriptor, "wb") as stream:
                stream.write(updated)
            os.replace(temporary, path)
        finally:
            if os.path.exists(temporary):
                os.unlink(temporary)
    return {"records": changes, "maps_changed": 0, "geometry_changed": 0,
            "approximation": "native nofog layer composition in linear color; source script suppression zero; legacy captured probe reflection omitted"}
ENCODER_SHA256 = "dcfdec10244e02cf5037fba089c55fb7e1326b1c8181742d77d15fa5cb5eef06"
CLASSES = {
    "concrete": [.84, .58, .98, 0, 0, .50], "stone": [.80, .50, .96, 0, 0, .55],
    "mud": [.63, .30, .90, 0, .18, .48], "wood": [.73, .45, .94, 0, 0, .65],
    "fabric": [.88, .70, .98, 0, 0, .65], "skin": [.59, .35, .80, 0, 0, .60],
    "glass": [.12, .05, .30, 0, 0, .60], "painted_metal": [.50, .22, .80, 0, .12, .65],
    "rust": [.87, .65, .98, 0, 0, .55], "metal": [.35, .12, .65, 1, 0, .65],
    "water": [.14, .08, .25, 0, 0, 1], "unclassified": [.80, .45, .98, 0, 0, .60],
}


def classification(name):
    name = name.lower()
    rules = [
        ("fabric", ("german_gear", "german_lower", "german_upper", "skirt", "strap", "coat_soldier", "boots", "hat", "fur", "tarp", "spiderweb", "wall_note", "blueprint", "schematic", "photo", "staff_note")),
        ("metal", ("crusader_body", "crusader_gear", "bullet", "bolt", "rail_", "rebar", "robot", "tripod")),
        ("skin", ("viewarm", "dead_body")),
        ("painted_metal", ("camo", "vending", "zapper", "power_station", "pump", "gas_can", "drone", "speaker", "fixture", "lantern", "gas_lamp", "desklamp", "bunkbed", "tool_chest", "bucket", "wheelbarrow", "pap_equip", "lighting_fixture")),
        ("wood", ("crate", "church_pew", "cork", "stick_trap", "work_bench", "wagon", "afr_door")),
        ("stone", ("ruins_floor", "church_column", "crypt", "statue", "plinth", "bones", "houses_dest")),
        ("glass", ("crystal", "church_window")),
        ("water", ("water",)), ("glass", ("glass", "bottle", "flask", "beaker")),
        ("skin", ("skin", "flesh", "head", "face", "hand", "body_bloody", "blood")),
        ("fabric", ("cloth", "fabric", "uniform", "german_body", "ger_body", "leather", "sack", "sandbag", "canopy", "foliage", "grass", "leaf", "tree")),
        ("mud", ("mud", "dirt", "ground", "terrain", "soil")),
        ("wood", ("wood", "plank", "lumber", "timber", "bark", "board")),
        ("rust", ("rust", "charred")),
        ("painted_metal", ("paint", "pipes", "pipe_", "tank", "radio", "generator", "machine", "metal_grey", "metal_green", "metal_red", "corrugated")),
        ("metal", ("metal", "steel", "iron", "brass", "copper", "chrome", "weapon", "wpn_", "attach_", "mauser", "m1911", "mp40", "remington", "b23r", "m14", "rebar", "wire", "fence", "gutter", "scope", "incense_burner", "torch", "wind_rings")),
        ("concrete", ("concrete", "cement", "bunker", "plaster", "rubble", "brick")),
        ("stone", ("stone", "rock", "marble", "pillar", "monolith", "ceramic", "roof", "tile")),
    ]
    return next((kind for kind, tokens in rules if any(t in name for t in tokens)), "unclassified")


def originals(inspection, extraction):
    result = {}
    # Later patch definitions win; retain zone identity in the external receipt.
    for zone in ("common_zm", "zm_tomb", "patch_zm", "zm_tomb_patch"):
        path = extraction / zone / "material-definitions.json"
        if path.exists():
            for name, definition in json.loads(path.read_text()).items():
                result[name] = dict(definition, zone=zone)
    inventory = json.loads((inspection / "full-scene-assets/inventory.private.json").read_text())
    for definition in inventory["materials"]:
        # The retained world exchange is joined by binding hash and native identity.
        if definition["name"] not in result:
            textures = [dict(t, image=inventory["images"][t["image"]]["name"])
                        for t in definition["textures"]]
            result[definition["name"]] = dict(definition, family=definition["techset"], textures=textures, zone="retained_world")
    return result


def catalog_for(pack, definitions):
    identities = {}
    for path in sorted(pack.rglob("*.gltf")):
        for material in json.loads(path.read_text())["materials"]:
            name = material["name"]
            tokens = name.split("(")[-1].rstrip(")").split(":")
            identities[name] = {"class": classification(tokens[0]),
                                "layer_classes": [classification(t) for t in tokens],
                                "source_family": definitions.get(name, {}).get("family", "unrecovered")}
    return {"version": 2, "classes": {k: dict(zip(
        ("roughness", "minimum", "maximum", "metallic", "coating", "normal_strength"), v)) for k, v in CLASSES.items()},
        "materials": dict(sorted(identities.items()))}


class Cooker:
    def __init__(self, pack, work, encoder, catalog, inspection, extraction):
        self.materials = []
        self.pack, self.work, self.encoder, self.catalog = pack, work, encoder, catalog
        self.inspection, self.extraction = inspection, extraction
        self.pending, self.sources, self.entries = {}, {}, {}
        self.image_paths = {}
        for root in [inspection / "game-prepare/models/images", *[extraction / z / "images" for z in
                    ("common_zm", "zm_tomb", "patch_zm", "zm_tomb_patch")]]:
            if root.exists():
                self.image_paths.update({p.stem: p for p in root.rglob("*.dds")})
        inventory = json.loads((inspection / "full-scene-assets/inventory.private.json").read_text())
        for image in inventory["images"]:
            if image.get("file"):
                self.image_paths.setdefault(image["name"], inspection / "full-scene-assets" / image["file"])
        self.work.mkdir(parents=True, exist_ok=True)
        (pack / "textures").mkdir(exist_ok=True)

    @lru_cache(maxsize=20)
    def pixels(self, path):
        if path.suffix.lower() == ".dds":
            raw = path.read_bytes()
            record = inspect_dds(raw, allow_non_2d=True)
            if record["dimension"] != "2d" or record["surfaces"] != 1 or record["dxgi"] in {2, 10, 11, 54, 56}:
                return None
            if record["dxgi"] in {61, 65}:
                offset = 148 if raw[84:88] == b"DX10" else 128
                plane = np.frombuffer(raw, np.uint8, record["width"] * record["height"], offset).reshape(record["height"], record["width"])
                result = np.full((*plane.shape, 4), 255, np.uint8)
                if record["dxgi"] == 65:
                    result[..., 3] = plane
                else:
                    result[..., :3] = plane[..., None]
                return result
        with Image.open(path) as image:
            return np.array(image.convert("RGBA"))

    def source(self, binding):
        if binding is None:
            return None
        path = self.image_paths.get(binding["image"])
        if path is None:
            raise ValueError(f"Native image {binding['image']} is missing; recover the declared zone DDS")
        # A patch may reuse a name with different bytes. Provenance follows the
        # resolved zone image, rather than silently keeping the first revision.
        key = str(path.resolve())  # Receipt only; never exported to runtime descriptors.
        if key not in self.sources:
            try:
                self.sources[key] = dict(inspect_dds(path.read_bytes(), allow_non_2d=True),
                    identity=binding["image"], sampler=binding.get("sampler", 20), resolution=key)
            except ValueError as error:
                raise ValueError(f"Source image {binding['image']} ({path.name}): {error}") from error
        if self.sources[key]["dimension"] != "2d" or self.sources[key]["surfaces"] != 1 or self.sources[key]["dxgi"] in {2, 10, 11, 54, 56}:
            return None  # Precision data is retained separately, never quantized as albedo.
        return self.pixels(path)

    def texture(self, levels, role, srgb=False):
        if all(np.all(level == levels[0][0, 0]) for level in levels):
            levels = [levels[0][:1, :1].copy()]
        raw = rgba_dds(levels, srgb)
        digest = hashlib.sha256(raw + role.encode()).hexdigest()[:32]
        name = f"pbr-{digest}.dds"
        w, h = levels[0].shape[1], levels[0].shape[0]
        fmt = "BC7_UNORM_SRGB" if srgb else "BC5_UNORM" if role == "normal" else "BC4_UNORM" if role in ("coating", "mask") else "BC7_UNORM"
        # Block encoders require aligned top levels. Preserve tiny/odd native sizes.
        if w % 4 or h % 4:
            fmt = "R8G8B8A8_UNORM_SRGB" if srgb else "R8G8B8A8_UNORM"
        master = self.work / name
        if master.exists():
            if master.read_bytes() != raw:
                raise ValueError(f"Canonical master cache is corrupt: {name}; remove only this keyed master and resume")
        else:
            temporary = master.with_suffix(".dds.tmp")
            temporary.write_bytes(raw)
            temporary.replace(master)
        self.pending[name] = fmt
        uri = "textures/" + name
        self.entries[uri] = {"uri": uri, "role": role, "color_space": "srgb" if srgb else "linear",
                             "format": fmt, "width": w, "height": h, "mip_count": len(levels)}
        return self.entries[uri].copy()

    def material(self, material, document, path, original):
        surface = material.setdefault("extras", {}).setdefault("surface", {})
        identity = material["name"]
        entry = self.catalog["materials"][identity]
        family = original.get("family", "unrecovered") if original else entry["source_family"]
        old = surface.get("channels", [])
        by_role = {(c["role"], c["layer"]): c for c in old}
        native = resolve_bindings(original, surface, identity)

        def old_pixels(role, layer):
            c = by_role.get((role, layer))
            if c:
                if "uri" in c:
                    return rgba_levels((self.work / Path(c["uri"]).name).read_bytes())[0]
                i = document["textures"][c["texture"]]["source"]
                source_path = path.parent / document["images"][i]["uri"]
                return self.pixels(source_path) if source_path.is_file() else None
            key = {"color": "baseColorTexture", "normal": "normalTexture", "emissive": "emissiveTexture"}.get(role)
            pbr = material.get("pbrMetallicRoughness", {})
            ref = material.get(key) if key != "baseColorTexture" else pbr.get(key)
            if layer == 0 and ref:
                i = document["textures"][ref["index"]]["source"]
                source_path = path.parent / document["images"][i]["uri"]
                return self.pixels(source_path) if source_path.is_file() else None
            return None

        def pixels(role, layer):
            source = self.source(native.get((role, layer)))
            return source if source is not None else old_pixels(role, layer)

        presentation_approximation = None
        if original and "presentation" not in surface:
            try:
                surface["presentation"] = presentation(original, family)
            except ValueError:
                presentation_approximation = "native technique has no TECH4/TECH2; source glTF blend/cull fallback"
                surface["presentation"] = {"alpha": {"MASK": "mask", "BLEND": "blend"}.get(material.get("alphaMode"), "opaque"),
                    "cutoff": material.get("alphaCutoff", .5), "cull": "none" if material.get("doubleSided") else "back",
                    "emissive": "unlit" in family, "fog": "nofog" not in family, "visible": "shadowcaster" not in family,
                    "wettable": False, "solid": "unlit" not in family}
            surface["presentation"]["lamp"] = False
        state = surface.get("presentation", {"alpha": "opaque"})
        surface.update(identity=identity, family=surface.get("family", material_family(family)),
                       version=2, physical_class="fabric" if entry["class"] == "leather" else entry["class"], coating_roughness=.12)
        cooked, semantics = [], []
        layers = sorted({layer for role, layer in set(native) | set(by_role) if role == "color"} | {0})
        for layer in layers:
            kind = entry["layer_classes"][layer] if layer < len(entry["layer_classes"]) else entry["class"]
            recipe = self.catalog["classes"][kind]
            color, normal, specular = pixels("color", layer), pixels("normal", layer), pixels("specular_gloss", layer)
            if color is None and normal is None and specular is None:
                continue
            interpretation = meanings(family, surface, layer, specular is not None)
            alpha_gloss = interpretation["color_alpha"] == "gloss"
            gloss = (specular if specular is not None else color) if (specular is not None or alpha_gloss) else None
            if color is not None:
                base = color.copy()
                if alpha_gloss or layer == 0 and state["alpha"] == "opaque":
                    base[..., 3] = 255
                c = self.texture(color_levels(base, True, state.get("cutoff", .5) if state["alpha"] == "mask" and layer == 0 else None), "color", True)
                cooked.append(dict(c, layer=layer, channels="rgb:base_color,a:coverage"))
                if state["alpha"] == "mask" and layer == 0:
                    self.entries[c["uri"]]["mask_cutoffs"] = [state.get("cutoff", .5)]
            if normal is not None:
                if ("normal", layer) in native and self.source(native["normal", layer]) is not None:
                    unit = canonical_normal(normal, recipe["normal_strength"])
                else:
                    unit = normal[..., :3].astype(np.float32) / 127.5 - 1
                    unit /= np.maximum(np.linalg.norm(unit, axis=2, keepdims=True), 1e-6)
                normal_mips, variance = normal_levels(unit)
                n = self.texture(normal_mips, "normal")
                cooked.append(dict(n, layer=layer, channels="rg:unit_normal_xy"))
            else:
                variance = []
            response = physical_response(color, gloss, normal, recipe,
                                         entry.get("physical_response", {}).get(str(layer)))
            response_mips = response_levels(response, variance)
            orm = self.texture(response_mips, "response")
            cooked.append(dict(orm, layer=layer, channels="r:occlusion,g:perceptual_roughness,b:metallic,a:coating"))
            if ("coating", layer) in by_role:
                coating_mips = [np.dstack((level[..., 3], np.zeros(level.shape[:2], np.uint8),
                                          np.zeros(level.shape[:2], np.uint8), np.full(level.shape[:2], 255, np.uint8)))
                                for level in response_mips]
                coating = self.texture(coating_mips, "coating")
                cooked.append(dict(coating, layer=layer, channels="r:coating"))
            semantics.append({"layer": layer, "class": kind, "interpretation": interpretation,
                              "roughness_source": "specular_alpha" if specular is not None else "color_alpha" if alpha_gloss else "authored_family",
                              "bindings": [dict(native[role, layer]) for role in ("color", "normal", "specular_gloss") if (role, layer) in native]})
        emissive = pixels("emissive", 0)
        if emissive is not None:
            c = self.texture(color_levels(emissive, True), "emissive", True)
            cooked.append(dict(c, layer=0, channels="rgb:emissive,a:unused"))
        # Macro-mask R/G is data. Replace the duplicate sRGB entry with linear BC7.
        if surface.get("preparation") and pixels("color", 2) is not None:
            mask = pixels("color", 2)
            cooked = [c for c in cooked if not (c["role"] == "color" and c["layer"] == 2)]
            c = self.texture(color_levels(mask, False), "control")
            cooked.append(dict(c, layer=2, channels="rg:core_coating_weights"))
        # Water uses a second normal even when it has no matching color layer.
        for layer in (1, 2):
            if not any(c["role"] == "normal" and c["layer"] == layer for c in cooked):
                normal = pixels("normal", layer)
                if normal is not None:
                    levels, _ = normal_levels(canonical_normal(normal, self.catalog["classes"][entry["class"]]["normal_strength"]))
                    c = self.texture(levels, "normal")
                    cooked.append(dict(c, layer=layer, channels="rg:unit_normal_xy"))
        surface["channels"] = sorted(cooked, key=lambda c: (c["role"], c["layer"]))
        for c in surface["channels"]:
            role = "specular_gloss" if c["role"] in ("response", "coating") else "color" if c["role"] == "control" else c["role"]
            binding = native.get((role, c["layer"]), native.get(("color", c["layer"]), {}))
            c["sampler"] = binding.get("sampler", 20)
        if "presentation" in surface:
            material["doubleSided"] = state["cull"] == "none"
            material["alphaMode"] = "MASK" if state["alpha"] == "mask" else "OPAQUE" if state["alpha"] == "opaque" else "BLEND"
            if state["alpha"] == "mask":
                material["alphaCutoff"] = state.get("cutoff", .5)
        surface["provenance"] = {"source_family": family, "source_zone": (original or {}).get("zone", "retained_export"),
                                 "recipe_sha256": hashlib.sha256(json.dumps(entry, sort_keys=True).encode()).hexdigest(),
                                 "approximation": "bounded native-gloss GGX fit; explicit catalog substrate and reviewed atlas masks; neutral AO"}
        effect = prepared_effect(original, surface)
        if effect:
            surface["effect"] = effect
            material.setdefault("extensions", {})["KHR_materials_unlit"] = {}
            document.setdefault("extensionsUsed", []).append("KHR_materials_unlit")
            document["extensionsUsed"] = sorted(set(document["extensionsUsed"]))
        if presentation_approximation:
            surface["provenance"]["approximation"] += "; " + presentation_approximation
        self.materials.append({"identity": identity, "source_family": family, "channels": semantics,
                               "physical_class": entry["class"], "unresolved": original is None or entry["class"] == "unclassified"})
        for key in ("normalTexture", "emissiveTexture", "occlusionTexture"):
            material.pop(key, None)
        pbr = material.setdefault("pbrMetallicRoughness", {})
        pbr.pop("baseColorTexture", None)
        pbr.pop("metallicRoughnessTexture", None)
        pbr["metallicFactor"] = self.catalog["classes"][entry["class"]]["metallic"]
        pbr["roughnessFactor"] = self.catalog["classes"][entry["class"]]["roughness"]

    def compress(self):
        try:
            from .compressed_texture_cache import CompressedTextureCache, materialize
            from .preparation_resources import PreparationResources
        except ImportError:
            from compressed_texture_cache import CompressedTextureCache, materialize
            from preparation_resources import PreparationResources
        resources = PreparationResources(exit_for_shared_slot=True, saturate=True)
        cache = CompressedTextureCache(self.work / "compressed-cache", self.encoder, "cuda", resources)
        for name in self.pending:
            resources.wait()
            raw = (self.work / name).read_bytes()
            entry = dict(self.entries["textures/" + name], scale=1,
                         density_applicable=max(self.entries["textures/" + name]["width"],
                                                self.entries["textures/" + name]["height"]) > 1)
            key = hashlib.sha256(raw + entry["role"].encode() + ENCODER_SHA256.encode() + b"cuda" +
                                 json.dumps(entry.get("mask_cutoffs", [])).encode()).hexdigest()
            found = cache.find(key)
            if found is None:
                found = cache.store(key, rgba_levels(raw), entry, raw)
            materialize(found[0], self.pack / "textures" / name)
            self.entries[entry["uri"]].update(format=found[1]["format"],
                dds=inspect_dds(found[0].read_bytes()))


def prepare(config, source_pack=None, write_catalog=False):
    inspection = Path(config["assets"]["inspection_output"]).resolve()
    settings = config["materials"]
    source = Path(source_pack or settings["source_pack"]).resolve()
    output = Path(config["game"]["asset_pack"]).resolve()
    extraction = Path(settings["extraction"]).resolve()
    work = Path(settings["work"]).resolve()
    for p in (source, output, work, extraction):
        if p == WORKSPACE or WORKSPACE in p.parents:
            raise ValueError("material preparation inputs/outputs must be external")
    definitions = originals(inspection, extraction)
    catalog_path = WORKSPACE / "config/origins-materials.json"
    if write_catalog:
        catalog_path.write_text(json.dumps(catalog_for(source, definitions), indent=2) + "\n")
        return
    catalog = json.loads(catalog_path.read_text())
    if source == output or source in output.parents or output in source.parents:
        raise ValueError("material source and destination overlap")
    if not output.exists():
        shutil.copytree(source, output)
    cooker = Cooker(output, work, Path(settings["encoder"]), catalog, inspection, extraction)
    cooker.materials = []
    # Convert documents in memory first; leave readable source pack untouched.
    documents = []
    for path in sorted(output.rglob("*.gltf")):
        doc = json.loads(path.read_text())
        if any(m.get("extras", {}).get("surface", {}).get("version") == 2 for m in doc.get("materials", [])):
            raise ValueError("output already cooked; choose a fresh external destination")
        for material in doc.get("materials", []):
            cooker.material(material, doc, path, definitions.get(material["name"]))
        print(f"Prepared {path.relative_to(output)}", flush=True)
        doc["textures"], doc["images"], doc["samplers"] = [], [], []
        documents.append((path, doc))
    cooker.compress()
    for path, doc in documents:
        path.write_text(json.dumps(doc, separators=(",", ":")) + "\n")
    try:
        from .surface_frames import repair_document
    except ImportError:
        from surface_frames import repair_document
    for path, _ in documents:
        repair_document(output, path, catalog)
    # Remove only copied, now-unreferenced images inside this external destination.
    kept = set(cooker.entries)
    game = json.loads((output / "game.json").read_text())
    # Content may contain UI images independent of glTF materials.
    def image_refs(value):
        if isinstance(value, dict):
            for v in value.values(): image_refs(v)
        elif isinstance(value, list):
            for v in value: image_refs(v)
        elif isinstance(value, str) and value.endswith((".png", ".jpg", ".jpeg")):
            kept.add(value)
    image_refs(game)
    for image in output.rglob("*"):
        if image.suffix.lower() in (".png", ".jpg", ".jpeg") and image.relative_to(output).as_posix() not in kept:
            image.unlink()
    (work / "materials-receipt.json").write_text(json.dumps({"version": 2, "encoder_sha256": ENCODER_SHA256,
        "source_images": cooker.sources, "materials": cooker.materials, "textures": cooker.entries,
        "catalog_sha256": hashlib.sha256(catalog_path.read_bytes()).hexdigest()}, indent=2) + "\n")
    print(f"PBR pack: {len(cooker.materials)} assignments; {len(cooker.entries)} DDS textures", flush=True)
    try:
        from .realtime_assets import migrate
    except ImportError:
        from realtime_assets import migrate
    migrate(output)
    qualify(config)


def qualify(config, extra_channels=None, document_paths=None, prune=True):
    """Qualify legacy callers through the same GPU semantic cache, without fallback."""
    pack = Path(config["game"]["asset_pack"]).resolve()
    work = Path(config["materials"]["work"]).resolve()
    if pack.is_relative_to(WORKSPACE) or work.is_relative_to(WORKSPACE):
        raise ValueError("Material qualification must remain external")
    paths = sorted(pack.rglob("*.gltf")) if document_paths is None else document_paths
    documents = [(p, json.loads(p.read_text())) for p in paths]
    inspection = Path(config["assets"]["inspection_output"])
    cooker = Cooker(pack, work, Path(config["materials"]["encoder"]), {}, inspection,
                    Path(config["materials"]["extraction"]))
    for _, document in documents:
        for material in document.get("materials", []):
            surface = material.get("extras", {}).get("surface")
            if not surface:
                continue
            response = next((c for c in surface["channels"] if c["role"] == "response" and c["layer"] == 0), None)
            if response and not any(c["role"] == "coating" for c in surface["channels"]):
                levels = rgba_levels((work / Path(response["uri"]).name).read_bytes())
                if levels[0][..., 3].any():
                    coat = [np.dstack((x[..., 3], np.zeros(x.shape[:2], np.uint8),
                                      np.zeros(x.shape[:2], np.uint8), np.full(x.shape[:2], 255, np.uint8)))
                            for x in levels]
                    channel = cooker.texture(coat, "coating")
                    surface["channels"].append(dict(channel, layer=0, sampler=response["sampler"], channels="r:coating"))
            for channel in surface["channels"]:
                cooker.entries[channel["uri"]] = dict(channel)
                if channel["role"] == "color" and channel["layer"] == 0 and material.get("alphaMode") == "MASK":
                    cooker.entries[channel["uri"]].setdefault("mask_cutoffs", []).append(material.get("alphaCutoff", .5))
    for channel in extra_channels or []:
        cooker.entries.setdefault(channel["uri"], dict(channel))
    for uri, channel in cooker.entries.items():
        channel["mask_cutoffs"] = sorted(set(channel.get("mask_cutoffs", [])))
        cooker.pending[Path(uri).name] = channel["format"]
    cooker.compress()
    for path, document in documents:
        for material in document.get("materials", []):
            for channel in material.get("extras", {}).get("surface", {}).get("channels", []):
                metadata = cooker.entries[channel["uri"]]
                channel.update({key: metadata[key] for key in ("format", "width", "height", "mip_count")})
        normal_scaffold(path, document)
        temporary = path.with_suffix(".gltf.tmp")
        temporary.write_text(json.dumps(document, separators=(",", ":")) + "\n")
        temporary.replace(path)
    for channel in extra_channels or []:
        metadata = cooker.entries[channel["uri"]]
        channel.update({key: metadata[key] for key in ("format", "width", "height", "mip_count")})
    if prune:
        for path in (pack / "textures").glob("*.dds"):
            if path.relative_to(pack).as_posix() not in cooker.entries:
                path.unlink()
    receipt = work / "compression-receipt.json"
    temporary = receipt.with_suffix(".json.tmp")
    temporary.write_text(json.dumps(dict(encoder_sha256=ENCODER_SHA256, compression_device="cuda",
                                        textures=cooker.entries), indent=2) + "\n")
    temporary.replace(receipt)
    print(f"GPU-qualified {len(cooker.entries)} complete semantic mip chains", flush=True)


def migrate_assemblies(config):
    """Bring newer native weapon assemblies into the existing PBR material pack.

    Join known identities to canonical maps; cook new native materials without
    touching assembly geometry, skeletons or animation data.
    """
    pack = Path(config["game"]["asset_pack"]).resolve()
    if pack == WORKSPACE or WORKSPACE in pack.parents:
        raise ValueError("assembly material preparation must remain external")
    settings = config["materials"]
    inspection = Path(config["assets"]["inspection_output"])
    extraction = Path(settings["extraction"])
    definitions = originals(inspection, extraction)
    catalog_path = WORKSPACE / "config/origins-materials.json"
    catalog = json.loads(catalog_path.read_text())
    documents = [(p, json.loads(p.read_text())) for p in sorted(pack.rglob("*.gltf"))]
    known = {m["name"]: m for _, doc in documents for m in doc.get("materials", [])
             if m.get("extras", {}).get("surface", {}).get("version") == 2}
    cooker = Cooker(pack, Path(settings["work"]), Path(settings["encoder"]), catalog, inspection, extraction)
    changed = []
    for path, doc in documents:
        if path.parent.name != "actors":
            continue
        edited = False
        for index, material in enumerate(doc.get("materials", [])):
            if material.get("extras", {}).get("surface", {}).get("version") == 2:
                continue
            name = material["name"]
            if name not in known:
                native = definitions.get(name)
                kind = "glass" if "lens" in name else classification(name)
                catalog["materials"].setdefault(name, {"class": kind, "layer_classes": [kind],
                    "source_family": (native or {}).get("family", "unrecovered")})
                cooker.material(material, doc, path, native)
                known[name] = copy.deepcopy(material)
            material = copy.deepcopy(known[name])
            # Image indexes belonged to another document. Canonical DDS bindings
            # are pack-relative; only the local tangent scaffold needs glTF images.
            for key in ("normalTexture", "emissiveTexture", "occlusionTexture"):
                material.pop(key, None)
            pbr = material.get("pbrMetallicRoughness", {})
            pbr.pop("baseColorTexture", None)
            pbr.pop("metallicRoughnessTexture", None)
            doc["materials"][index] = material
            edited = True
        if edited:
            for material in doc["materials"]:
                for key in ("normalTexture", "emissiveTexture", "occlusionTexture"):
                    material.pop(key, None)
                pbr = material.get("pbrMetallicRoughness", {})
                pbr.pop("baseColorTexture", None)
                pbr.pop("metallicRoughnessTexture", None)
            doc.pop("images", None)
            doc.pop("textures", None)
            doc.pop("samplers", None)
            normal_scaffold(path, doc)
            path.write_text(json.dumps(doc, separators=(",", ":")) + "\n")
            changed.append(path)
    catalog_path.write_text(json.dumps(catalog, indent=2) + "\n")
    cooker.compress()
    qualify(config, document_paths=changed, prune=False)
    try:
        from .realtime_assets import migrate
    except ImportError:
        from realtime_assets import migrate
    retired = migrate(pack)
    print(f"Migrated {len(changed)} native assemblies; cooked {len(cooker.materials)} new material identities; {retired}", flush=True)


def normal_scaffold(path, document):
    """Retain Bevy glTF's MikkTSpace preparation for tangentless source rigs.

    DDS isn't a standard glTF image. A valid neutral normal fallback requests
    Bevy's existing tangent generator; actual shading replaces it with the
    canonical DDS map before surface readiness. Authored tangents are untouched.
    """
    required = {p["material"] for mesh in document.get("meshes", []) for p in mesh["primitives"]
                if "TANGENT" not in p["attributes"] and "TEXCOORD_0" in p["attributes"]}
    required = {i for i in required if any(c["role"] == "normal" for c in
        document["materials"][i].get("extras", {}).get("surface", {}).get("channels", []))}
    if not required:
        return
    uri = "pbr-neutral-normal.png"
    Image.new("RGBA", (1, 1), (128, 128, 255, 255)).save(path.parent / uri)
    images = document.setdefault("images", [])
    image = next((i for i, entry in enumerate(images) if entry.get("uri") == uri), None)
    if image is None:
        image = len(images)
        images.append({"uri": uri})
    textures = document.setdefault("textures", [])
    texture = next((i for i, entry in enumerate(textures) if entry.get("source") == image), None)
    if texture is None:
        texture = len(textures)
        textures.append({"source": image})
    for index in required:
        document["materials"][index]["normalTexture"] = {"index": texture}


def retune_identities(config):
    """Apply explicit reviewed catalog edits without rewriting source geometry."""
    pack, settings = Path(config["game"]["asset_pack"]), config["materials"]
    inspection, extraction = Path(config["assets"]["inspection_output"]), Path(settings["extraction"])
    catalog = json.loads((WORKSPACE / "config/origins-materials.json").read_text())
    definitions = originals(inspection, extraction)
    cooker = Cooker(pack, Path(settings["work"]), Path(settings["encoder"]), catalog, inspection, extraction)
    cooker.materials = []
    changed = 0
    for path in sorted(pack.rglob("*.gltf")):
        document = json.loads(path.read_text())
        edited = False
        for material in document["materials"]:
            entry = catalog["materials"][material["name"]]
            recipe = hashlib.sha256(json.dumps(entry, sort_keys=True).encode()).hexdigest()
            if "review" in entry and material["extras"]["surface"]["provenance"]["recipe_sha256"] != recipe:
                cooker.material(material, document, path, definitions.get(material["name"]))
                edited = True
                changed += 1
        if edited:
            path.write_text(json.dumps(document, separators=(",", ":")) + "\n")
    cooker.compress()
    print(f"Retuned {changed} explicitly reviewed material assignments", flush=True)


def retune_response_mips(config):
    """Repair normal/response footprint alignment without changing top levels."""
    pack = Path(config["game"]["asset_pack"])
    work = Path(config["materials"]["work"])
    catalog = json.loads((WORKSPACE / "config/origins-materials.json").read_text())
    cooker = Cooker(pack, work, Path(config["materials"]["encoder"]), catalog,
        Path(config["assets"]["inspection_output"]), Path(config["materials"]["extraction"]))
    cache, changed = {}, 0
    for path in sorted(pack.rglob("*.gltf")):
        doc = json.loads(path.read_text())
        edited = False
        for material in doc["materials"]:
            channels = material["extras"]["surface"]["channels"]
            for response in channels:
                normal = next((n for n in channels if n["role"] == "normal" and n["layer"] == response["layer"]), None)
                if response["role"] != "response" or normal is None or (normal["width"], normal["height"]) == (response["width"], response["height"]):
                    continue
                key = (normal["uri"], response["uri"])
                if key not in cache:
                    units = rgba_levels((work / Path(normal["uri"]).name).read_bytes())[0][..., :3].astype(np.float32) / 127.5 - 1
                    units /= np.maximum(np.linalg.norm(units, axis=2, keepdims=True), 1e-6)
                    _, variance = normal_levels(units)
                    top = rgba_levels((work / Path(response["uri"]).name).read_bytes())[0]
                    cache[key] = cooker.texture(response_levels(top, variance), "response")
                    changed += 1
                response.update(cache[key])
                edited = True
        if edited:
            path.write_text(json.dumps(doc, separators=(",", ":")) + "\n")
    cooker.compress()
    print(f"Retuned {changed} response mip chains with different normal resolutions", flush=True)


def prune_runtime_pack(config):
    try:
        from .import_origins import validate_pack
    except ImportError:
        from import_origins import validate_pack
    pack = Path(config["game"]["asset_pack"]).resolve()
    if pack == WORKSPACE or WORKSPACE in pack.parents:
        raise ValueError("runtime closure assembly must remain external")
    for path in pack.rglob("*.gltf"):
        doc = json.loads(path.read_text())
        normal_scaffold(path, doc)
        path.write_text(json.dumps(doc, separators=(",", ":")) + "\n")
    _, _, hashes = validate_pack(pack)
    retired = []
    for path in pack.rglob("*"):
        if path.is_file() and path.relative_to(pack).as_posix() not in hashes:
            retired.append(path.relative_to(pack).as_posix())
            path.unlink()
    print(f"Assembled exact runtime closure: {len(hashes)} files; retired {retired}", flush=True)
    return hashes


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--write-catalog", action="store_true")
    parser.add_argument("--qualify-existing", action="store_true")
    parser.add_argument("--migrate-assemblies", action="store_true")
    parser.add_argument("--assemble-existing", action="store_true")
    parser.add_argument("--retune-response-mips", action="store_true")
    parser.add_argument("--retune-identities", action="store_true")
    parser.add_argument("--retain-effects", action="store_true")
    args = parser.parse_args()
    config = tomllib.loads(args.config.read_text())
    if args.retain_effects:
        print(json.dumps(retain_effects(config), indent=2))
    elif args.migrate_assemblies:
        migrate_assemblies(config)
        prune_runtime_pack(config)
    elif args.retune_identities:
        retune_identities(config)
        qualify(config)
        prune_runtime_pack(config)
    elif args.retune_response_mips:
        retune_response_mips(config)
        qualify(config)
        prune_runtime_pack(config)
    elif args.assemble_existing:
        prune_runtime_pack(config)
    elif args.qualify_existing:
        qualify(config)
    else:
        prepare(config, write_catalog=args.write_catalog)
        if not args.write_catalog:
            prune_runtime_pack(config)
