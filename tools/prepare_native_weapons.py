import argparse
import copy
import hashlib
import json
from pathlib import Path
import re

try:
    from .material_textures import inspect_dds
except ImportError:
    from material_textures import inspect_dds


FORMATS = {28: "R8G8B8A8_UNORM", 29: "R8G8B8A8_UNORM_SRGB", 49: "R8G8_UNORM", 61: "R8_UNORM",
           65: "A8_UNORM", 71: "BC1_UNORM", 72: "BC1_UNORM_SRGB",
           74: "BC2_UNORM", 75: "BC2_UNORM_SRGB", 77: "BC3_UNORM",
           78: "BC3_UNORM_SRGB", 80: "BC4_UNORM", 81: "BC4_SNORM",
           83: "BC5_UNORM", 84: "BC5_SNORM", 87: "B8G8R8A8_UNORM",
           88: "B8G8R8X8_UNORM", 91: "B8G8R8A8_UNORM_SRGB",
           93: "B8G8R8X8_UNORM_SRGB", 98: "BC7_UNORM", 99: "BC7_UNORM_SRGB"}
CHANNELS = {"color": "rgb:native_gamma2,a:source_alpha",
            "normal": "rg:native_signed_slopes_directx_y",
            "normal_detail": "rg:native_signed_slopes_directx_y",
            "specular_gloss": "rgb:native_reflectance,a:gloss",
            "specular_detail": "rgb:native_reflectance,a:gloss",
            "specular": "rgb:native_reflectance,a:source_alpha",
            "gloss": "r:native_gloss", "occlusion": "r:native_occlusion",
            "detail": "rgba:native_detail", "emissive": "rgb:native_gamma2,a:source_alpha",
            "reveal": "rgba:native_reveal", "heat": "rgba:native_heat",
            "rim": "rgba:native_rim_mask"}
NAMES = {"colorMap": ("color", 0), "Color_Map": ("color", 0),
         "Diffuse_Map": ("color", 0), "normalMap": ("normal", 0),
         "Normal_Map": ("normal", 0), "specularMap": ("specular_gloss", 0),
         "SpecularAndGloss": ("specular_gloss", 0), "Specular_Map": ("specular_gloss", 0),
         "Specular_Color_Map": ("specular", 0), "Specular_Gloss_Map": ("gloss", 0),
         "occlusionMap": ("occlusion", 0), "colorDetailMap": ("detail", 0),
         "Detail_Map": ("detail", 0), "NormalDetailMap": ("normal_detail", 0),
         "Glow_Map": ("emissive", 0), "Ember_Map": ("emissive", 0),
         "EmberGlow_Reveal_Map": ("reveal", 0), "Heat_Map": ("heat", 0),
         "Layer1Map": ("color", 0), "Layer2Map": ("color", 1),
         "Layer3Map_Off": ("color", 2), "Layer3MapOn": ("color", 3),
         "Weapon_Normal_Map": ("normal", 0), "Weapon_Specular_Map": ("specular_gloss", 0),
         "Weapon_Color_Map": ("color", 1), "Camo_Detail_Map": ("normal_detail", 0)}
KINDS = (("weapon_camo", "camo"), ("wpn_zmb_staff_3layer", "staff"),
         ("char_skin_emberglow", "ember"), ("phong_emissive", "phong"),
         ("viewmodel_transparent", "transparent"), ("optic_lens", "lens"),
         ("reticle_dynamic", "reticle"), ("clan_tag", "badge"),
         ("player_icon", "badge"), ("unlit", "unlit"),
         ("char_cloth", "cloth"), ("char_skin", "skin"), ("lit_sm_", "lit"))


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def discover(pack):
    game = json.loads((pack / "game.json").read_bytes())
    content = game["content"]
    presentation_ids = set()

    def references(value):
        if isinstance(value, dict):
            for item in value.values():
                references(item)
        elif isinstance(value, list):
            for item in value:
                references(item)
        elif isinstance(value, str) and value.startswith(("actor.", "presentation.")):
            presentation_ids.add(value)

    references(content["weapons"])
    paths = {pack / p["scene"].split("#")[0] for p in content["presentations"]
             if p["id"] in presentation_ids}
    paths.update(p for p in pack.rglob("*.gltf")
                 if re.search(r"weapon_|_wpn_|viewhands?|_attach_|offhand", p.name))
    return sorted(paths)


class NativeWeapons:
    def __init__(self, pack, extraction, executable, work):
        self.pack, self.extraction = Path(pack), Path(extraction)
        self.executable, self.work = Path(executable), Path(work)
        self.definitions, self.images, self.material_cache = {}, {}, {}
        self.records, self.copied = {}, {}
        zones = sorted(p.parent for p in self.extraction.glob("zone/*/*/material-definitions.json"))
        priorities = {"weapons": 1, "common_zm": 2, "zm_tomb": 3, "patch_zm": 4, "zm_tomb_patch": 5}
        zones.sort(key=lambda p: (priorities.get(p.name, 0), p.as_posix()))
        for zone in zones:
            zone_id = zone.relative_to(self.extraction).as_posix()
            raw_definitions = json.loads((zone / "material-definitions.json").read_bytes())
            for name, raw_definition in raw_definitions.items():
                path = zone / "materials" / (name + ".json")
                if path.is_file():
                    raw = path.read_bytes()
                    self.definitions.setdefault(name, {})[zone_id] = (
                        json.loads(raw), raw_definition, digest(raw))
            for path in (zone / "images").rglob("*.dds"):
                self.images.setdefault(path.stem, {})[zone_id] = path

    def definition(self, identity, preferred=None):
        candidates = self.definitions.get(identity)
        if not candidates:
            raise ValueError(f"Native material definition unavailable: {identity}")
        zone = next((z for z in candidates if preferred and z.endswith("/" + preferred)), None)
        zone = zone or next(reversed(candidates))
        return zone, candidates[zone]

    def source_image(self, image, zone):
        candidates = self.images.get(image)
        if not candidates:
            raise ValueError(f"Native DDS unavailable: {image}")
        source_zone = zone if zone in candidates else next(reversed(candidates))
        return source_zone, candidates[source_zone]

    def texture(self, binding, zone, role, layer, sampler):
        source_zone, path = self.source_image(binding["image"], zone)
        key = (path, role, sampler)
        if key not in self.records:
            raw = path.read_bytes()
            record = inspect_dds(raw)
            if record["dxgi"] not in FORMATS:
                raise ValueError(f"Unsupported native DDS format: {path.name}: {record['dxgi']}")
            try:
                from .prepare_ktx2 import preserve_source
                from .compressed_texture_cache import materialize
            except ImportError:
                from prepare_ktx2 import preserve_source
                from compressed_texture_cache import materialize
            cached, receipt = preserve_source(path, role, False, self.executable, self.work)
            source_hash = record['sha256']
            record = receipt['output']
            uri = f"textures/native-{source_hash}-{sampler:02x}.ktx2"
            target = self.pack / uri
            target.parent.mkdir(parents=True, exist_ok=True)
            if not target.is_file():
                materialize(cached, target)
            elif digest(target.read_bytes()) != record['sha256']:
                raise ValueError(f"Native KTX2 identity collision: {uri}")
            self.records[key] = (record, uri)
            self.copied[uri] = {"sha256": record["sha256"], "source_zone": source_zone,
                                "source_sha256": source_hash, "image": binding["image"],
                                "mip_count": record["mip_count"], "recipe": receipt['recipe']}
        record, uri = self.records[key]
        channel = {"uri": uri, "role": role, "color_space": "linear", "format": record['format'],
                   "width": record["width"], "height": record["height"], "mip_count": record["mip_count"],
                   "layer": layer, "channels": CHANNELS[role], "sampler": sampler}
        provenance = {"uri": uri, "sha256": record["sha256"], "source_zone": source_zone}
        return channel, provenance

    def material(self, material, source_name=None, injections=None, constants=None):
        identity = material["name"]
        previous = material.get("extras", {}).get("surface", {})
        preferred = previous.get("native_weapon", previous.get("provenance", {})).get("source_zone", "").split("/")[-1]
        source_name = source_name or identity
        zone, (definition, raw_definition, source_hash) = self.definition(source_name, preferred)
        family = definition["techniqueSet"]
        kind = next((kind for token, kind in KINDS if token in family), None)
        if kind is None:
            raise ValueError(f"Unsupported native weapon family: {source_name}: {family}")
        native_constants = {c["name"]: c["literal"] for c in definition.get("constants", [])}
        native_constants.update(constants or {})
        native = {"kind": kind, "family": family, "constants": native_constants,
                  "source_zone": zone, "source_sha256": source_hash,
                  "material_identity": source_name, "bindings": []}
        channels, occupied = [], set()
        raw_bindings = raw_definition.get("textures", [])
        source_bindings = definition.get("textures", [])
        if len(raw_bindings) != len(source_bindings):
            raise ValueError(f"Native binding table mismatch: {source_name}")
        injections = injections or {}
        for binding, raw_binding in zip(source_bindings, raw_bindings, strict=True):
            name = binding.get("name")
            if binding["image"].lstrip(",") != raw_binding["image"].lstrip(","):
                raise ValueError(f"Native named/raw binding order mismatch: {source_name}")
            retained = copy.deepcopy(binding)
            retained["sampler"] = raw_binding["sampler"]
            retained["hash"] = raw_binding["hash"]
            if name == "radiantDiffuseMap":
                retained["active"] = False
                native["bindings"].append(retained)
                continue
            key = ("rim", 0) if binding.get("nameHash") == 2247950716 else NAMES.get(name)
            if key is None:
                raise ValueError(f"Unsupported native weapon binding: {source_name}: {name or binding.get('nameHash')}")
            role, layer = key
            if kind == "camo" and name == "normalMap":
                role = "normal_detail"
            if kind == "camo" and name == "SpecularAndGloss":
                role = "specular_detail"
            injection = injections.get(name, injections.get(role) if name and name.startswith("Weapon_") else None)
            resolved = copy.deepcopy(binding)
            texture_zone = zone
            if injection is not None:
                resolved["image"] = injection if isinstance(injection, str) else injection["image"]
                retained["injected_image"] = resolved["image"]
                if isinstance(injection, dict):
                    texture_zone = injection.get("source_zone", zone)
            if (role, layer) in occupied:
                raise ValueError(f"Duplicate native weapon channel: {source_name}: {(role, layer)}")
            occupied.add((role, layer))
            channel, provenance = self.texture(resolved, texture_zone, role, layer, raw_binding["sampler"])
            channels.append(channel)
            retained.update(provenance, active=True, role=role, layer=layer)
            native["bindings"].append(retained)
        surface = {"identity": identity, "family": previous.get("family", "lit"), "version": 2,
                   "physical_class": "unclassified", "coating_roughness": 0.0,
                   "channels": channels, "native_weapon": native}
        for key in ("presentation", "effect"):
            if key in previous:
                surface[key] = copy.deepcopy(previous[key])
        if "presentation" not in surface:
            try:
                from .prepare_origins import presentation
            except ImportError:
                from prepare_origins import presentation
            surface["presentation"] = dict(presentation(raw_definition, family), lamp=False)
        state = surface["presentation"]
        material.clear()
        material.update(name=identity, extras={"surface": surface},
                        pbrMetallicRoughness={"metallicFactor": 0.0, "roughnessFactor": 0.5},
                        doubleSided=state["cull"] == "none",
                        alphaMode="MASK" if state["alpha"] == "mask" else "OPAQUE" if state["alpha"] == "opaque" else "BLEND")
        if state["alpha"] == "mask":
            material["alphaCutoff"] = state["cutoff"]
        return material

    def prepare(self):
        changes = []
        for path in discover(self.pack):
            raw = path.read_bytes()
            document = json.loads(raw)
            for material in document.get("materials", []):
                identity = material["name"]
                if identity not in self.material_cache:
                    self.material_cache[identity] = copy.deepcopy(self.material(material))
                else:
                    material.clear()
                    material.update(copy.deepcopy(self.material_cache[identity]))
            for key in ("textures", "images", "samplers"):
                document[key] = []
            document.pop("extensionsUsed", None)
            document.pop("extensionsRequired", None)
            updated = (json.dumps(document, separators=(",", ":")) + "\n").encode()
            if raw != updated:
                path.write_bytes(updated)
                changes.append(path.relative_to(self.pack).as_posix())
        return {"gltfs": len(discover(self.pack)), "changed": changes,
                "materials": len(self.material_cache), "native_textures": len(self.copied),
                "families": sorted({m["extras"]["surface"]["native_weapon"]["family"] for m in self.material_cache.values()}),
                "textures": self.copied}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--pack", required=True, type=Path)
    parser.add_argument("--extraction", required=True, type=Path)
    parser.add_argument("--preparation-executable", required=True, type=Path)
    parser.add_argument("--work", required=True, type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    report = NativeWeapons(args.pack, args.extraction, args.preparation_executable, args.work).prepare()
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({k: v for k, v in report.items() if k not in {"changed", "textures"}}))


if __name__ == "__main__":
    main()
