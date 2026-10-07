# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///
"""Inventory every installed BO2 texture/model, retaining original containers.

Extraction is resumable and external. The inspected library is copied into
assets/source/bo2 separately from the validated Origins runtime closure.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

try:
    from .export_instances import export
except ImportError:
    from export_instances import export

WORKSPACE = Path(__file__).resolve().parents[1]


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def copy_detached(source, target, expected, link_from=None):
    """Replace one validated output without modifying any linked working copy."""
    target.parent.mkdir(parents=True, exist_ok=True)
    descriptor, name = tempfile.mkstemp(prefix=target.name + ".", suffix=".tmp", dir=target.parent)
    os.close(descriptor)
    temporary = Path(name)
    try:
        if link_from is None:
            shutil.copyfile(source, temporary)
        else:
            # Only the identical, already-validated converted project output
            # may supply a link. Originals, external masters and LFS objects
            # remain independent. Replacing the name detaches future recooks.
            temporary.unlink()
            os.link(link_from, temporary)
        if digest(temporary) != expected:
            raise ValueError(f"library copy changed: {target.name}")
        os.replace(temporary, target)
        return link_from is not None
    finally:
        temporary.unlink(missing_ok=True)


def extract(config):
    installation = Path(config["assets"]["installation_root"]).resolve()
    inspection = Path(config["assets"]["inspection_output"]).resolve()
    extraction = inspection / "bo2-library-extraction"
    oat = Path(config["materials"].get("oat_source", inspection / "tooling/OAT/surface-7d027e8/source"))
    stock = oat / "build/bin/Release_x86/Unlinker.exe"
    callback = WORKSPACE / "tools/static_instances.cpp"
    zones = sorted((installation / "zone").rglob("*.ff"))
    if not zones or not stock.is_file():
        raise ValueError("installed fastfiles or the existing external OAT build are unavailable")
    os.environ["ORIGINS_INCLUDE_ALL_IMAGES"] = "1"
    first = extraction / zones[0].relative_to(installation).with_suffix("")
    # Compile once; all following zones use the same read-only callback binary.
    export(oat, zones[0], first, first / "images", True)
    reader = first / "Unlinker.exe"
    callback_hash = digest(callback)
    search = ";".join(str(installation / p) for p in ("zone/all", "zone/english"))
    failures, receipts = [], []
    for index, zone in enumerate(zones):
        relative = zone.relative_to(installation)
        output = extraction / relative.with_suffix("")
        output.mkdir(parents=True, exist_ok=True)
        stamp = output / "extraction-receipt.json"
        source_hash = digest(zone)
        expected = {"source_sha256": source_hash, "callback_sha256": callback_hash, "oat_sha256": digest(stock), "metadata_version": 2}
        previous = json.loads(stamp.read_text()) if stamp.exists() else {}
        if previous == expected:
            receipts.append({"zone": relative.as_posix(), **expected})
            continue
        env = dict(os.environ, ORIGINS_DATA_ROOT=str(installation / "zone/all"),
                   ORIGINS_IMAGE_OUTPUT=str(output / "images"), ORIGINS_INCLUDE_ACTOR_IMAGES="1")
        with (output / "metadata.log").open("wb") as log:
            result = subprocess.run([str(reader), "--no-color", "--list", str(zone)], cwd=output,
                                    env=env, stdout=log, stderr=log)
        if result.returncode:
            failures.append({"zone": relative.as_posix(), "phase": "metadata", "exit": result.returncode})
            print(f"Failed metadata {relative}", flush=True)
            continue
        if any(previous.get(k) != expected[k] for k in ("source_sha256", "oat_sha256")):
            with (output / "unlink.log").open("wb") as log:
                result = subprocess.run([str(stock), "--no-color", "--image-format", "DDS", "--model-format", "GLTF",
                    "--include-assets", "xmodel,material,techset", "--search-path", search,
                    "--output-folder", str(output), str(zone)], cwd=output, stdout=log, stderr=log)
            if result.returncode:
                failures.append({"zone": relative.as_posix(), "phase": "unlink", "exit": result.returncode})
                print(f"Failed unlink {relative}", flush=True)
                continue
        stamp.write_text(json.dumps(expected, indent=2) + "\n")
        receipts.append({"zone": relative.as_posix(), **expected})
        inventory = json.loads((output / "asset-inventory.json").read_text())
        print(f"Extracted {index + 1}/{len(zones)} {relative}: {len(inventory['models'])} models, {len(inventory['images'])} images", flush=True)
    (extraction / "extraction.json").write_text(json.dumps({"zones": receipts, "failures": failures}, indent=2) + "\n")
    if failures:
        raise RuntimeError(f"{len(failures)} fastfile extractions failed; see external extraction.json")


def retain(config):
    installation = Path(config["assets"]["installation_root"]).resolve()
    extraction = Path(config["assets"]["inspection_output"]).resolve() / "bo2-library-extraction"
    library = WORKSPACE / "assets/source/bo2"
    originals = library / "originals"
    files = []
    for source in sorted(installation.rglob("*")):
        if not source.is_file() or source.suffix.lower() not in {".ff", ".ipak", ".clump", ".bmp", ".png", ".jpg", ".jpeg", ".dds", ".iwi", ".tga", ".gltf", ".glb", ".obj", ".xmodel", ".xmodel_bin"}:
            continue
        relative = source.relative_to(installation)
        target = originals / "containers" / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        sha = digest(source)
        if not target.exists() or digest(target) != sha:
            shutil.copyfile(source, target)
        if digest(source) != sha or digest(target) != sha:
            raise ValueError(f"source/container changed during retention: {relative}")
        files.append({"uri": target.relative_to(library).as_posix(), "sha256": sha, "bytes": source.stat().st_size})
    if not files:
        raise ValueError("no original asset containers")
    (library / "containers.json").write_text(json.dumps({"version": 1, "files": files}, indent=2) + "\n")
    print(f"Retained {len(files)} untouched containers, {sum(f['bytes'] for f in files):,} bytes", flush=True)


def packages(config):
    installation = Path(config["assets"]["installation_root"]).resolve()
    inspection = Path(config["assets"]["inspection_output"]).resolve()
    oat = Path(config["materials"].get("oat_source", inspection / "tooling/OAT/surface-7d027e8/source"))
    export(oat, installation / "zone/all/code_pre_gfx_zm.ff", inspection / "bo2-package-export",
           inspection / "bo2-library-extraction/packages", True, WORKSPACE / "tools/package_images.cpp")


def check_library_index():
    try:
        from .import_origins import check_index, require
    except ImportError:
        from import_origins import check_index, require
    library = WORKSPACE / "assets/source/bo2"
    hashes = json.loads((library / "originals.json").read_text())["sha256"]
    ordinary = {"originals.json", "containers.json"}
    if (library / "receipt.json").is_file():
        receipt = json.loads((library / "receipt.json").read_text())
        hashes.update(receipt["originals_and_masters"])
        hashes.update({"converted/" + uri: sha for uri, sha in receipt["converted"].items()})
        ordinary.update({"receipt.json", "materials.json", "compression.json", "generated-targets.json", "converted/library.json"})
    hashes.update({r["uri"]: r["sha256"] for r in json.loads((library / "containers.json").read_text())["files"]})
    hashes.update({uri: digest(library / uri) for uri in ordinary})
    actual = {p.relative_to(library).as_posix() for p in library.rglob("*") if p.is_file()}
    require(actual == hashes.keys(), "source library files differ from receipts")
    print(json.dumps(check_index(library, hashes, ordinary)), flush=True)


def retain_extracted(config):
    """Retain native exchanges and mip chains; this performs no PBR cooking."""
    inspection = Path(config["assets"]["inspection_output"]).resolve()
    extraction, library = inspection / "bo2-library-extraction", WORKSPACE / "assets/source/bo2"
    receipt = json.loads((extraction / "extraction.json").read_text())
    if receipt["failures"]:
        raise ValueError("original extraction is incomplete")
    inventories = sorted(extraction.glob("zone/*/*/asset-inventory.json"))
    if len(inventories) != len(receipt["zones"]):
        raise ValueError("original zone inventory coverage differs")
    models, lods, authored, generated = 0, 0, 0, 0
    for path in inventories:
        inventory = json.loads(path.read_text())
        for model in inventory["models"]:
            for lod in range(model["lods"]):
                if not (path.parent / "model_export" / (model["name"] + f"_lod{lod}.gltf")).is_file():
                    raise ValueError("original model LOD exchange is missing")
            models += 1
            lods += model["lods"]
        for image in inventory["images"]:
            if image["name"].startswith("*"):
                generated += 1
            elif not (path.parent / "images" / (image["name"] + ".dds")).is_file():
                raise ValueError("original authored image is missing")
            else:
                authored += 1
    package_entries = 0
    for index in (extraction / "packages").glob("*/entries.json"):
        for entry in json.loads(index.read_text()):
            if not entry["decoded"] or any(not (index.parent / (entry["identity"] + suffix)).is_file() for suffix in (".dds", ".iwi")):
                raise ValueError("original package entry is missing")
            package_entries += 1
    hashes = {}
    selected = {"images", "model_export", "xmodel", "materials", "techsets", "techniques", "shader_bin", "packages", "loose"}
    for path in sorted(extraction.rglob("*")):
        relative = path.relative_to(extraction)
        if not path.is_file() or not (set(relative.parts) & selected or path.name in {"asset-inventory.json", "material-definitions.json", "extraction-receipt.json", "extraction.json"}):
            continue
        target = library / "originals/extracted" / relative
        sha = digest(path)
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.is_file() or digest(target) != sha:
            shutil.copyfile(path, target)
        if digest(target) != sha:
            raise ValueError("retained original exchange bytes differ")
        hashes[target.relative_to(library).as_posix()] = sha
        if len(hashes) % 10000 == 0:
            print(f"Retained original exchanges {len(hashes)}", flush=True)
    result = {"version": 2, "zones": len(inventories), "models": models, "model_lods": lods,
              "authored_named_images": authored, "generated_targets": generated,
              "package_entries": package_entries, "sha256": hashes}
    (library / "originals.json").write_text(json.dumps(result, indent=2) + "\n")
    print(f"Retained {len(hashes)} original exchanges; no PBR cooking", flush=True)
    return hashes


def import_library(config):
    try:
        from .import_origins import validate_gltf, safe_path, require
        from .material_textures import inspect_dds
    except ImportError:
        from import_origins import validate_gltf, safe_path, require
        from material_textures import inspect_dds
    inspection = Path(config["assets"]["inspection_output"]).resolve()
    prepared, extraction, masters = inspection / "bo2-library-prepared", inspection / "bo2-library-extraction", inspection / "bo2-library-cooking"
    manifest = json.loads((prepared / "library.json").read_text())
    require(manifest.get("qualified") is True, "library compression must be qualified before import")
    require(manifest.get("catalog_sha256") == digest(WORKSPACE / "config/bo2-materials.json"),
            "qualified library material catalog differs from the tracked recipes")
    require(manifest["zones"] == len(json.loads((extraction / "extraction.json").read_text())["zones"]), "library zone coverage mismatch")
    files = {"library.json"}
    for index, path in enumerate(sorted(prepared.rglob("*.gltf"))):
        uri = path.relative_to(prepared).as_posix()
        validate_gltf(prepared, uri, files, allow_native_transforms=True)
        if index % 500 == 0:
            print(f"Validated library models/material banks {index + 1}", flush=True)
    require(all(len(m["variants"]) == m["source_lods"] for m in manifest["models"]), "native model LOD coverage mismatch")
    models = {(m["zone"], m["identity"]): m for m in manifest["models"]}
    sources = {t["source"] for t in manifest["textures"]}
    declarations = []
    for inventory_path in sorted(extraction.glob("zone/*/*/asset-inventory.json")):
        zone = inventory_path.parent.relative_to(extraction).as_posix()
        inventory = json.loads(inventory_path.read_text())
        for model in inventory["models"]:
            record = models.get((zone, model["name"]))
            require(record is not None and record["source_lods"] == model["lods"], "native model declaration omitted")
        for image in inventory["images"]:
            name = zone + "/images/" + image["name"] + ".dds"
            if name not in sources:
                require(image["name"].startswith("*"), f"native image declaration omitted: {name}")
                declarations.append({"zone": zone, **image, "disposition": "generated target; declaration and original fastfile retained"})
    for index in sorted((extraction / "packages").glob("*/entries.json")):
        for entry in json.loads(index.read_text()):
            require(entry["decoded"] and (index.parent.relative_to(extraction).as_posix() + "/" + entry["identity"] + ".dds") in sources,
                    "package image conversion omitted")
    for texture in manifest["textures"]:
        original = extraction / texture["source"]
        require(digest(original) == texture["source_sha256"], "original texture changed")
        variant = texture["variant"]
        path = safe_path(prepared, variant["uri"], {".dds"})
        layout = inspect_dds(path.read_bytes(), allow_non_2d=variant.get("lossless", False))
        if "mip_count" in variant:
            require(layout["mip_count"] == variant["mip_count"] and layout["width"] == variant["width"] and layout["height"] == variant["height"], "texture variant layout mismatch")
            expected = {"BC7_UNORM_SRGB": 99, "BC7_UNORM": 98, "BC5_UNORM": 83, "BC4_UNORM": 80,
                        "R8G8B8A8_UNORM_SRGB": 29, "R8G8B8A8_UNORM": 28}
            require(layout["dxgi"] == expected.get(variant["format"]), "standalone texture format mismatch")
            require(layout["mip_count"] == max(layout["width"], layout["height"]).bit_length(), "standalone mip chain incomplete")
            require((variant["color_space"] == "srgb") == (layout["dxgi"] in {29, 99}), "standalone transfer mismatch")
        else:
            require(digest(path) == variant["sha256"], "precision texture changed")
        files.add(variant["uri"])
    actual = {p.relative_to(prepared).as_posix() for p in prepared.rglob("*") if p.is_file()}
    require(actual == files, f"library closure mismatch: {sorted(actual ^ files)[:20]}")
    hashes = {uri: digest(prepared / uri) for uri in sorted(files)}
    library = WORKSPACE / "assets/source/bo2"
    # Originals include named material/model exchanges and every package entry.
    original_files = {}
    selected = {"images", "model_export", "xmodel", "materials", "techsets", "techniques", "shader_bin", "packages", "loose"}
    for path in sorted(extraction.rglob("*")):
        relative = path.relative_to(extraction)
        if not path.is_file() or not (set(relative.parts) & selected or path.name in {"asset-inventory.json", "material-definitions.json", "extraction-receipt.json", "extraction.json"}):
            continue
        target = library / "originals/extracted" / relative
        sha = digest(path)
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists() or digest(target) != sha:
            shutil.copyfile(path, target)
        require(digest(target) == sha, "retained extraction bytes differ")
        original_files[target.relative_to(library).as_posix()] = sha
    authoring_links = 0
    for uri, sha in hashes.items():
        target = library / "converted" / uri
        copy_detached(prepared / uri, target, sha)
        if uri.startswith("textures/"):
            converted_target = target
            master = masters / Path(uri).name
            target = library / "authoring" / uri
            master_sha = digest(master)
            authoring_links += copy_detached(master, target, master_sha,
                                             converted_target if master_sha == sha else None)
            original_files[target.relative_to(library).as_posix()] = master_sha
    provenance = json.loads((masters / "material-provenance.json").read_text())
    (library / "materials.json").write_text(json.dumps(provenance, indent=2) + "\n")
    (library / "generated-targets.json").write_text(json.dumps(declarations, indent=2) + "\n")
    shutil.copyfile(masters / "compression-receipt.json", library / "compression.json")
    (library / "receipt.json").write_text(json.dumps({"version": 2, "originals_and_masters": original_files,
        "converted": hashes, "models": len(manifest["models"]), "model_lods": sum(m["source_lods"] for m in manifest["models"]),
        "textures": len(manifest["textures"]), "zones": manifest["zones"], "generated_targets": len(declarations)}, indent=2) + "\n")
    print(f"Imported complete library: {len(original_files)} original/master files, {len(hashes)} converted files; "
          f"{authoring_links} identical qualified authoring outputs share working-copy storage", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--retain", action="store_true")
    parser.add_argument("--retain-extracted", action="store_true")
    parser.add_argument("--packages", action="store_true")
    parser.add_argument("--import", dest="import_prepared", action="store_true")
    parser.add_argument("--check-index", action="store_true")
    args = parser.parse_args()
    config = tomllib.loads(args.config.read_text())
    if args.retain_extracted:
        retain_extracted(config)
    elif args.check_index:
        check_library_index()
    elif args.import_prepared:
        import_library(config)
    elif args.packages:
        packages(config)
    elif args.retain:
        retain(config)
    else:
        extract(config)
