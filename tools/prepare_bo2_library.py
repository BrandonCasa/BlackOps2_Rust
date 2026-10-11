# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13", "zstandard>=0.25,<1"]
# ///
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import tempfile
import tomllib

try:
    from .prepare_ktx2 import preserve_source, backend_identity
    from .compressed_texture_cache import materialize
except ImportError:
    from prepare_ktx2 import preserve_source, backend_identity
    from compressed_texture_cache import materialize

import numpy as np
from PIL import Image

try:
    from .bo2_library import WORKSPACE, digest
    from .prepare_materials import Cooker, classification, CLASSES, qualify
    from .prepare_actors import convert
    from .material_textures import canonical_normal, color_levels, normal_levels, inspect_dds, rgba_levels
    from .prepare_origins import tile_blend_color
except ImportError:
    from bo2_library import WORKSPACE, digest
    from prepare_materials import Cooker, classification, CLASSES, qualify
    from prepare_actors import convert
    from material_textures import canonical_normal, color_levels, normal_levels, inspect_dds, rgba_levels
    from prepare_origins import tile_blend_color


def material_converter_hash():
    names = ("prepare_materials.py", "material_textures.py", "material_semantics.py", "prepare_origins.py", "prepare_ktx2.py", "ktx2_textures.py")
    sources = {name: digest(Path(__file__).with_name(name)) for name in names}
    return hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest()


def revoke_qualification(pack):
    path = pack / "library.json"
    manifest = json.loads(path.read_text())
    manifest["qualified"] = False
    temporary = tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", newline="\n",
        dir=pack, prefix=".library-", suffix=".json", delete=False)
    temporary_path = Path(temporary.name)
    try:
        with temporary:
            json.dump(manifest, temporary, indent=2)
            temporary.write("\n")
        os.replace(temporary_path, path)
    finally:
        temporary_path.unlink(missing_ok=True)
    return manifest


def prepare(config, reuse_geometry=False, defer_qualification=False):
    inspection = Path(config["assets"]["inspection_output"]).resolve()
    source = inspection / "bo2-library-extraction"
    extraction = json.loads((source / "extraction.json").read_text())
    if extraction["failures"]:
        raise ValueError("complete fastfile extraction is required before library conversion")
    pack, work = inspection / "bo2-library-prepared", inspection / "bo2-library-cooking"
    pack.mkdir(exist_ok=True)
    previous_manifest = {}
    if (pack / "library.json").is_file():
        previous_manifest = revoke_qualification(pack)
    previous_sources = {(model["zone"], model["identity"], variant["lod"]): variant
        for model in previous_manifest.get("models", []) for variant in model["variants"]}
    zones = sorted(source.glob("zone/*/*/asset-inventory.json"))
    definitions, inventories, images = {}, {}, {}
    all_names = set()
    for path in zones:
        key = path.parent.relative_to(source).as_posix()
        inventories[key] = json.loads(path.read_text())
        definitions[key] = {name: dict(d, zone=key) for name, d in json.loads((path.parent / "material-definitions.json").read_text()).items()}
        images[key] = {p.stem: p for p in (path.parent / "images").rglob("*.dds")}
        all_names.update(definitions[key])
    catalog_path = WORKSPACE / "config/bo2-materials.json"
    if catalog_path.exists():
        catalog = json.loads(catalog_path.read_text())
    else:
        catalog = json.loads((WORKSPACE / "config/origins-materials.json").read_text())
    for name in sorted(all_names):
        tokens = name.split("(")[-1].rstrip(")").split(":")
        catalog["materials"].setdefault(name, {"class": classification(tokens[0]),
            "layer_classes": [classification(t) for t in tokens], "source_family": "native_zone"})
    catalog_path.write_text(json.dumps(catalog, indent=2) + "\n", newline="\n")
    cooker = Cooker(pack, work, Path(config["materials"]["preparation_executable"]), catalog, inspection, source)
    cooker.materials = []
    global_images = {name: path for mapping in images.values() for name, path in mapping.items()}
    global_definitions = {name: d for mapping in definitions.values() for name, d in mapping.items()}
    common_images = {name: path for key, mapping in images.items() if key.endswith(("/common_zm", "/patch_zm")) for name, path in mapping.items()}
    cache_path = work / "material-cache.json"
    material_cache = json.loads(cache_path.read_text()) if cache_path.is_file() else {}
    converter_hash = material_converter_hash()
    image_hashes = {}
    model_records, missing = [], []
    for key, inventory in inventories.items():
        zone = source / key
        cooker.image_paths = {**global_images, **common_images, **images[key]}
        native = {**global_definitions, **definitions[key]}
        def cook(material, doc, original_path):
            name = material["name"]
            original = native.get(name, {})
            resolved = {}
            for binding in original.get("textures", []):
                image = cooker.image_paths.get(binding["image"])
                if image is not None:
                    if image not in image_hashes:
                        image_hashes[image] = digest(image)
                    resolved[binding["image"]] = image_hashes[image]
            if name not in catalog["materials"]:
                catalog["materials"][name] = {"class": classification(name), "layer_classes": [classification(name)], "source_family": "unrecovered"}
            cache_key = hashlib.sha256(json.dumps({"converter": converter_hash, "identity": name,
                "native": {k: v for k, v in original.items() if k != "zone"}, "images": resolved,
                "recipe": catalog["materials"][name], "classes": catalog["classes"],
                "source": material, "backend": backend_identity(cooker.preparation_executable)}, sort_keys=True).encode()).hexdigest()
            if cache_key in material_cache:
                material.clear()
                material.update(copy.deepcopy(material_cache[cache_key]))
                material["extras"]["surface"]["provenance"]["source_zone"] = original.get("zone", key)
                for c in material["extras"]["surface"]["channels"]:
                    cooker.pending[Path(c["uri"]).name] = c["format"]
                    cooker.entries[c["uri"]] = {k: c[k] for k in ("uri", "role", "color_space", "format", "width", "height", "mip_count")}
                return
            cooker.material(material, doc, original_path, native.get(name))
            material_cache[cache_key] = copy.deepcopy(material)
        for model in inventory["models"]:
            lods = []
            for lod in range(model["lods"]):
                original = zone / "model_export" / (model["name"] + f"_lod{lod}.gltf")
                if not original.is_file():
                    missing.append({"zone": key, "model": model["name"], "lod": lod})
                    continue
                output = pack / "models" / key
                target = output / original.name
                source_hash = digest(original)
                previous_source = previous_sources.get((key, model["name"], lod), {})
                previous = json.loads(target.read_text()) if (reuse_geometry and target.is_file()
                    and previous_source.get("source_sha256") == source_hash
                    and previous_source.get("uri") == target.relative_to(pack).as_posix()) else {}
                if previous.get("asset", {}).get("generator") == "Origins recovered skeletal preparation" and all(
                        (output / b["uri"]).is_file() and (output / b["uri"]).stat().st_size == b["byteLength"] for b in previous.get("buffers", [])):
                    doc = previous
                else:
                    relative, _ = convert(original, output, zone / "images", [], prepare_images=False, preparation=config)
                    target = output / relative
                    doc = json.loads(target.read_text())
                source_document = json.loads(original.read_text())
                if any(m.get("extras", {}).get("surface", {}).get("version") == 2 for m in doc.get("materials", [])):
                    doc["materials"] = source_document.get("materials", [])
                    for section in ("textures", "images", "samplers"):
                        doc[section] = source_document.get(section, [])
                for material in doc.get("materials", []):
                    cook(material, doc, original)
                doc["textures"], doc["images"], doc["samplers"] = [], [], []
                target.write_text(json.dumps(doc, separators=(",", ":")) + "\n")
                lods.append({"lod": lod, "uri": target.relative_to(pack).as_posix(), "source_sha256": source_hash})
            model_records.append({"zone": key, "identity": model["name"], "source_lods": model["lods"], "variants": lods})
        bank = {"asset": {"version": "2.0"}, "scene": 0, "scenes": [{"nodes": []}], "nodes": [], "materials": []}
        for name in definitions[key]:
            material = {"name": name}
            cook(material, {"textures": [], "images": []}, zone / "material-bank.gltf")
            bank["materials"].append(material)
        target = pack / "materials" / (key.replace("/", "-") + ".gltf")
        target.parent.mkdir(exist_ok=True)
        target.write_text(json.dumps(bank, separators=(",", ":")) + "\n")
        cache_path.write_text(json.dumps(material_cache, separators=(",", ":")) + "\n")
        print(f"Prepared {key}: {len(inventory['models'])} models, {len(bank['materials'])} material identities", flush=True)
    if missing:
        (work / "missing-models.json").write_text(json.dumps(missing, indent=2) + "\n")
        raise ValueError(f"{len(missing)} native model LODs were not exported")
    semantic_by_name = {}
    for mapping in definitions.values():
        for material in mapping.values():
            for binding in material["textures"]:
                semantic_by_name.setdefault(binding["image"], set()).add(binding["semantic"])
    semantic_by_hash = {}
    for inventory in inventories.values():
        for image in inventory["images"]:
            semantic_by_hash.setdefault((image["name_hash"], image["data_hash"]), set()).update(semantic_by_name.get(image["name"], ()))
    texture_records, extra_channels = [], []
    textures = [(p, semantic_by_name.get(p.stem, set())) for mapping in images.values() for p in mapping.values()]
    for index in sorted((source / "packages").glob("*/entries.json")):
        for entry in json.loads(index.read_text()):
            if not entry["decoded"]:
                raise ValueError(f"package image was not decoded: {index.parent.name}/{entry['identity']}")
            textures.append((index.parent / (entry["identity"] + ".dds"), semantic_by_hash.get((entry["name_hash"], entry["data_hash"]), set())))
    cache = {}
    for index, (path, semantics) in enumerate(textures):
        raw_hash = digest(path)
        semantic = "normal" if 5 in semantics else "color" if 2 in semantics else "control"
        cache_key = (raw_hash, semantic)
        if cache_key not in cache:
            try:
                layout = inspect_dds(path.read_bytes())
            except ValueError:
                uri = "lighting/" + raw_hash[:32] + ".ktx2"
                target = pack / uri
                target.parent.mkdir(exist_ok=True)
                cached, retained = preserve_source(path, "precision_or_non_2d", False, cooker.preparation_executable, work/"retained-ktx2")
                materialize(cached, target)
                cache[cache_key] = {"uri": uri, "role": "precision_or_non_2d", "sha256": retained["output"]["sha256"], "source_sha256": raw_hash, "lossless": True}
            else:
                if layout["dxgi"] in {2, 10, 11, 54, 56}:
                    uri = "lighting/" + raw_hash[:32] + ".ktx2"
                    target = pack / uri
                    target.parent.mkdir(exist_ok=True)
                    cached, retained = preserve_source(path, "precision_or_non_2d", False, cooker.preparation_executable, work/"retained-ktx2")
                    materialize(cached, target)
                    cache[cache_key] = {"uri": uri, "role": "precision_or_non_2d", "sha256": retained["output"]["sha256"], "source_sha256": raw_hash, "lossless": True}
                    texture_records.append({"source": path.relative_to(source).as_posix(), "source_sha256": raw_hash, "semantics": sorted(semantics), "variant": cache[cache_key]})
                    continue
                pixels = cooker.pixels(path)
                if semantic == "normal":
                    levels, _ = normal_levels(canonical_normal(pixels))
                    channel = cooker.texture(levels, "normal")
                else:
                    channel = cooker.texture(color_levels(pixels, semantic == "color"), semantic, semantic == "color")
                cache[cache_key] = channel
                extra_channels.append(channel)
        texture_records.append({"source": path.relative_to(source).as_posix(), "source_sha256": raw_hash,
                                "semantics": sorted(semantics), "variant": cache[cache_key]})
        if index % 500 == 0:
            print(f"Prepared source textures {index + 1}/{len(textures)}", flush=True)
    cooker.compress()
    modified = copy.deepcopy(config)
    modified["game"]["asset_pack"] = str(pack)
    modified["materials"]["work"] = str(work)
    if not defer_qualification:
        qualify(modified, extra_channels)
    catalog_path.write_text(json.dumps(catalog, indent=2) + "\n", newline="\n")
    manifest = {"version": 2, "qualified": not defer_qualification, "zones": len(zones), "models": model_records, "textures": texture_records,
        "material_assignments": len(material_cache), "catalog_sha256": digest(catalog_path),
        "approximations": "GGX lobe fit and reviewed Origins family presets; new identities use explicit editable family classifications; generated reflection targets have declarations but no authored image payload"}
    (pack / "library.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (work / "material-provenance.json").write_text(json.dumps(cooker.materials, indent=2) + "\n")
    print(f"Full library: {len(model_records)} zone/model identities; {len(texture_records)} texture sources", flush=True)


def qualify_existing(config):
    inspection = Path(config["assets"]["inspection_output"]).resolve()
    pack, work = inspection / "bo2-library-prepared", inspection / "bo2-library-cooking"
    manifest = revoke_qualification(pack)
    modified = copy.deepcopy(config)
    modified["game"]["asset_pack"], modified["materials"]["work"] = str(pack), str(work)
    qualify(modified, [t["variant"] for t in manifest["textures"] if not t["variant"].get("lossless")])
    manifest["qualified"] = True
    (pack / "library.json").write_text(json.dumps(manifest, indent=2) + "\n")


def retune_unbound(config):
    inspection = Path(config["assets"]["inspection_output"])
    source, pack, work = inspection / "bo2-library-extraction", inspection / "bo2-library-prepared", inspection / "bo2-library-cooking"
    manifest = revoke_qualification(pack)
    catalog = json.loads((WORKSPACE / "config/bo2-materials.json").read_text())
    overrides = json.loads((WORKSPACE / "config/origins-materials.json").read_text())
    for identity, entry in overrides["materials"].items():
        if "review" in entry and identity in catalog["materials"]:
            catalog["materials"][identity].update({k: entry[k] for k in ("class", "layer_classes", "review")})
    cooker = Cooker(pack, work, Path(config["materials"]["preparation_executable"]), catalog, inspection, source)
    definitions, image_paths = {}, {}
    for inventory in sorted(source.glob("zone/*/*/asset-inventory.json")):
        zone = inventory.parent.relative_to(source).as_posix()
        definitions[zone] = json.loads((inventory.parent / "material-definitions.json").read_text())
        image_paths[zone] = {p.stem: p for p in (inventory.parent / "images").glob("*.dds")}
    global_images = {name: p for mapping in image_paths.values() for name, p in mapping.items()}
    common_images = {name: p for zone, mapping in image_paths.items() if zone.endswith(("/common_zm", "/patch_zm")) for name, p in mapping.items()}
    global_definitions = {name: d for mapping in definitions.values() for name, d in mapping.items()}
    provenance, resolved, tiles, control_sources = {}, {}, {}, set()
    for path in sorted(pack.rglob("*.gltf")):
        relative = path.relative_to(pack)
        zone = "/".join(relative.parts[1:4]) if relative.parts[0] == "models" else "/".join(path.stem.split("-", 2))
        available = {**global_images, **common_images, **image_paths[zone]}
        native = {**global_definitions, **definitions[zone]}
        doc = json.loads(path.read_text())
        edited = False
        for material in doc.get("materials", []):
            surface = material["extras"]["surface"]
            identity = material["name"]
            record_key = (zone, identity)
            original = native.get(identity, {})
            entry = catalog["materials"][identity]
            if "review" in entry and surface["physical_class"] != entry["class"]:
                cooker.image_paths = available
                cooker.material(material, doc, source / zone / "material-bank.gltf", dict(original, zone=zone))
                surface = material["extras"]["surface"]
            bindings = []
            for binding in original.get("textures", []):
                image = available.get(binding["image"])
                if image is not None:
                    key = image.relative_to(source).as_posix()
                    if key not in resolved:
                        record = inspect_dds(image.read_bytes(), allow_non_2d=True)
                        resolved[key] = {"source_uri": key, "source_sha256": record["sha256"], "source_format": record["dxgi"],
                                         "source_mips": record["mip_count"], "source_dimensions": [record["width"], record["height"]]}
                    bindings.append({**binding, **resolved[key]})
                else:
                    bindings.append({**binding, "approximation": "unresolved/generated binding; original declaration retained"})
            provenance.setdefault(record_key, {"zone": zone, "identity": identity, "physical_class": surface["physical_class"],
                "provenance": surface["provenance"], "bindings": bindings,
                "unresolved": not original or surface["physical_class"] == "unclassified"})
            family = surface["provenance"]["source_family"]
            if "tile_blend" in family:
                colors = [b for b in original.get("textures", []) if b["semantic"] == 2]
                if len(colors) == 3 and all(b["image"] in available for b in colors):
                    paths = [available[b["image"]] for b in colors]
                    tile_key = tuple(digest(p) for p in paths)
                    control_sources.add(tile_key[2])
                    if tile_key not in tiles:
                        pixels = [cooker.pixels(p) for p in paths]
                        if all(p is not None for p in pixels):
                            control = cooker.texture(color_levels(pixels[2], False), "control")
                            tiles[tile_key] = control
                    if tile_key in tiles:
                        control = tiles[tile_key]
                        surface["channels"] = [c for c in surface["channels"] if not (c["role"] in {"color", "control"} and c["layer"] == 2) and c["role"] != "baked_color"]
                        surface["channels"].extend([
                            {**control, "layer": 2, "channels": "rg:core_coating_weights", "sampler": colors[2]["sampler"]}])
                        surface["preparation"] = {"kind": "tile_macro_layers", "tiles": 4}
                        note = "fourfold tile/macro coverage with normalized weights; native detail bindings retained"
                        if note not in surface["provenance"]["approximation"]:
                            surface["provenance"]["approximation"] += "; " + note
            specular_layers = set()
            specular_hashes = {887934131: 0, 3532022562: 1, 3532022561: 2}
            for binding in original.get("textures", []):
                if binding["semantic"] == 8:
                    specular_layers.add(specular_hashes.get(binding["hash"], len(specular_layers)))
            for c in surface["channels"]:
                layer = c["layer"]
                separate_specular = layer in specular_layers
                if c["role"] != "color" or f"n{layer}x{layer}" not in family or separate_specular:
                    continue
                levels = rgba_levels((work / Path(c["uri"]).with_suffix('.dds').name).read_bytes())
                if all(np.all(level[..., 3] == 255) for level in levels):
                    continue
                for level in levels:
                    level[..., 3] = 255
                c.update(cooker.texture(levels, "color", True))
                edited = True
            surface["provenance"]["resolution_key"] = zone + "/" + identity
            edited = True
        if edited:
            path.write_text(json.dumps(doc, separators=(",", ":")) + "\n")
    cache, changed = {}, 0
    for texture in manifest["textures"]:
        if texture["variant"].get("lossless"):
            continue
        path = source / texture["source"]
        if path.suffix.lower() == ".bmp":
            continue
        record = inspect_dds(path.read_bytes())
        iwi = path.with_suffix(".iwi")
        gamma = 1.
        if iwi.is_file():
            with iwi.open("rb") as stream:
                header = stream.read(64)
            if header[:3] == b"IWi" and header[3] == 27:
                gamma = struct.unpack_from("<f", header, 12)[0]
        semantics = texture["semantics"]
        role = "control" if texture["source_sha256"] in control_sources else "normal" if 5 in semantics or record["dxgi"] == 83 else "color" if 2 in semantics or gamma > 1.1 else "mask" if record["dxgi"] in {61, 65, 80} else "control"
        if texture["variant"]["role"] == role:
            continue
        key = (texture["source_sha256"], role)
        if key not in cache:
            pixels = cooker.pixels(path)
            if role == "mask" and record["dxgi"] == 65:
                pixels = np.repeat(pixels[..., 3:4], 4, axis=2)
            levels = normal_levels(canonical_normal(pixels))[0] if role == "normal" else color_levels(pixels, role == "color")
            cache[key] = cooker.texture(levels, role, role == "color")
        texture["variant"] = cache[key]
        texture["interpretation"] = "native DXN slope normal" if role == "normal" else "native IWI gamma/color binding; physical identity unresolved" if role == "color" else "native scalar mask" if role == "mask" else "linear control data; physical identity unresolved"
        changed += 1
    installation = Path(config["assets"]["installation_root"])
    loose = installation / "codlogo.bmp"
    if loose.is_file() and not any(t["source"] == "loose/codlogo.bmp" for t in manifest["textures"]):
        target = source / "loose/codlogo.bmp"
        target.parent.mkdir(exist_ok=True)
        shutil.copyfile(loose, target)
        with Image.open(target) as image:
            pixels = np.array(image.convert("RGBA"))
        channel = cooker.texture(color_levels(pixels, True), "color", True)
        manifest["textures"].append({"source": "loose/codlogo.bmp", "source_sha256": digest(target), "semantics": [2], "variant": channel})
    cooker.compress()
    modified = copy.deepcopy(config)
    modified["game"]["asset_pack"], modified["materials"]["work"] = str(pack), str(work)
    qualify(modified, [t["variant"] for t in manifest["textures"] if not t["variant"].get("lossless")])
    catalog_path = WORKSPACE / "config/bo2-materials.json"
    catalog_path.write_text(json.dumps(catalog, indent=2) + "\n", newline="\n")
    manifest["catalog_sha256"] = digest(catalog_path)
    manifest["qualified"] = True
    manifest["approximations"] = "GGX lobe fit and editable physical-family recipes; explicit reviewed identity corrections; generated reflection targets have declarations but no authored image payload"
    (pack / "library.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (work / "material-provenance.json").write_text(json.dumps(list(provenance.values()), indent=2) + "\n")
    print(f"Retuned {changed} unbound image records from their native encoding/gamma", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Prepare every extracted BO2 model LOD and texture, independently of runtime use.')
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--retune-unbound", action="store_true")
    parser.add_argument("--reuse-geometry", action="store_true")
    parser.add_argument("--defer-qualification", action="store_true", help="assemble first; finish with --qualify-existing before import")
    parser.add_argument("--qualify-existing", action="store_true")
    args = parser.parse_args()
    config = tomllib.loads(args.config.read_text())
    if args.qualify_existing:
        qualify_existing(config)
    elif args.retune_unbound:
        retune_unbound(config)
    else:
        prepare(config, args.reuse_geometry, args.defer_qualification)
