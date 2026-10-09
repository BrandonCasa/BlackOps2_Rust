import argparse
import copy
import hashlib
import json
from pathlib import Path
import re
import shutil

from compressed_texture_cache import materialize
from import_origins import validate_pack, validate_gltf
from material_semantics import CHANNELS, uv_selection
from prepare_materials import Cooker, originals
from preparation_resources import PreparationResources


def atomic_json(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, separators=(",", ":")) + "\n")
    temporary.replace(path)


def repair(source, destination, masters, inspection, extraction, encoder, catalog_path, select, receipt, source_receipt=None, resume=False):
    workspace = Path(__file__).resolve().parents[1]
    source, destination, masters = source.resolve(), destination.resolve(), masters.resolve()
    if (destination.exists() and not resume) or any(destination.is_relative_to(p) or p.is_relative_to(destination)
                                   for p in (source, workspace, masters, inspection.resolve(), extraction.resolve())):
        raise ValueError("Use a fresh external destination separate from all inputs, masters and workspace")
    if any(masters.is_relative_to(p) or p.is_relative_to(masters) for p in (source, workspace)):
        raise ValueError("Canonical masters must stay external and separate from the source pack")
    if any(receipt.resolve().is_relative_to(p) for p in (source, destination, workspace)):
        raise ValueError("Repair receipt must remain separate from the packs and workspace")
    if (source / "texture-streaming.json").exists():
        raise ValueError("Expand the input with prepare_texture_source.py before native repair")
    if source_receipt is None:
        _, _, before = validate_pack(source, exact=True)
    else:
        before = json.loads(source_receipt.read_text())["recovered_hashes"]
        actual = {p.relative_to(source).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in source.rglob("*") if p.is_file()}
        if actual != before:
            raise ValueError("Recovered pack differs from its complete validated closure receipt")
    catalog_bytes = catalog_path.read_bytes()
    catalog = json.loads(catalog_bytes)
    definitions = originals(inspection, extraction)
    resources = PreparationResources(exit_for_shared_slot=True, saturate=True)
    resources.wait()
    if not destination.exists():
        shutil.copytree(source, destination, copy_function=lambda a, b: materialize(Path(a), Path(b)))
    else:
        for uri, digest in before.items():
            if uri.endswith(".gltf") or uri.startswith("textures/pbr-"):
                continue
            path = destination / uri
            if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
                raise ValueError(f"Cannot resume: nonmaterial dependency differs from source: {uri}")
    cooker = Cooker(destination, masters, encoder, catalog, inspection, extraction)
    updates, identities = {}, set()
    for scene in sorted(source.rglob("*.gltf")):
        resources.wait()
        document = json.loads(scene.read_text())
        structural = {key: copy.deepcopy(value) for key, value in document.items() if key != "materials"}
        changed = False
        for material in document.get("materials", []):
            identity = material["name"]
            if not re.search(select, identity):
                continue
            entry = catalog["materials"].get(identity)
            if entry is None or "unclassified" in entry.get("layer_classes", [entry["class"]]):
                raise ValueError(f"{identity}: assign reviewed physical classes in {catalog_path.name} first")
            original_surface = copy.deepcopy(material["extras"]["surface"])
            if original_surface.get("version") != 2:
                raise ValueError(f"{identity}: selected repair requires existing version-two declarations")
            candidate = copy.deepcopy(material)
            cooker.material(candidate, document, scene, definitions.get(identity))
            proposed = candidate["extras"]["surface"]
            channels = {(c["role"], c["layer"]): c for c in proposed["channels"]}
            repaired = []
            for old in original_surface["channels"]:
                key = old["role"], old["layer"]
                replacement = channels.pop(key, old)
                if old["role"] == "control":
                    replacement = old
                repaired.append(copy.deepcopy(replacement))
            for key, channel in channels.items():
                if key[0] == "emissive":
                    raise ValueError(f"{identity}: recovered emission has no runtime declaration; resolve explicitly")
            material["extras"]["surface"]["channels"] = repaired
            material["extras"]["surface"]["physical_class"] = proposed["physical_class"]
            material["extras"]["surface"]["provenance"] = proposed["provenance"]
            for channel in repaired:
                if channel["channels"] != CHANNELS[channel["role"]]:
                    raise ValueError(f"{identity}: prepared {channel['role']} has contradictory channel meanings")
            identities.add(identity)
            changed = True
        if changed:
            if any(document.get(key) != value for key, value in structural.items()):
                raise ValueError(f"Native material repair changed nonmaterial scene declarations: {scene.name}")
            updates[scene.relative_to(source).as_posix()] = document
    if not identities:
        raise ValueError("Selection matched no native material identities")
    cooker.compress()
    for uri, document in updates.items():
        for material in document["materials"]:
            for channel in material["extras"]["surface"]["channels"]:
                metadata = cooker.entries.get(channel["uri"])
                if metadata:
                    channel.update({key: metadata[key] for key in ("format", "width", "height", "mip_count")})
        atomic_json(destination / uri, document)
    used = {c["uri"] for scene in destination.rglob("*.gltf")
            for m in json.loads(scene.read_text()).get("materials", [])
            for c in m.get("extras", {}).get("surface", {}).get("channels", [])}
    old_materials = {c["uri"] for scene in source.rglob("*.gltf")
                     for m in json.loads(scene.read_text()).get("materials", [])
                     for c in m.get("extras", {}).get("surface", {}).get("channels", [])}
    prepared = {p.relative_to(destination).as_posix() for p in (destination / "textures").glob("pbr-*.dds")
                if re.fullmatch(r"pbr-[0-9a-f]{32}\.dds", p.name)}
    for uri in (old_materials | prepared) - used:
        (destination / uri).unlink(missing_ok=True)
    for uri in updates:
        validate_gltf(destination, uri, set())
    after = {p.relative_to(destination).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
             for p in destination.rglob("*") if p.is_file()}
    unchanged = {}
    for uri, digest in before.items():
        if uri not in updates and uri not in old_materials:
            if after.get(uri) != digest:
                raise ValueError(f"Nonmaterial dependency changed: {uri}")
            unchanged[uri] = digest
        if hashlib.sha256((source / uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Input changed during preparation: {uri}")
    if catalog_path.read_bytes() != catalog_bytes:
        raise ValueError("Physical catalog changed during cooking; resume with one immutable recipe revision")
    result = dict(version=2, selected_materials=sorted(identities), updated_scenes=sorted(updates),
                  native_interpretation=cooker.materials, native_sources=cooker.sources,
                  catalog_sha256=hashlib.sha256(catalog_bytes).hexdigest(),
                  unchanged_nonmaterial=unchanged, sha256=after,
                  files=len(after), bytes=sum((destination / uri).stat().st_size for uri in after),
                  visual_status="pending actual-game review", bulk_cook_complete=False)
    receipt.parent.mkdir(parents=True, exist_ok=True)
    atomic_json(receipt, result)
    print(json.dumps({k: result[k] for k in ("selected_materials", "files", "bytes", "visual_status")}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Repair selected native material sets in an external, otherwise unchanged pack.\n\nRecover full chains first, then repair, measure, cook, assemble and visually\nqualify. All four steps reuse the common semantic/DDS/cache helpers.\n')
    for name in ("source", "destination", "masters", "inspection", "extraction", "encoder", "receipt"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--catalog", type=Path, default=Path(__file__).resolve().parents[1]/"config/origins-materials.json")
    parser.add_argument("--select", required=True, help="Native identity regex; never a four-material production list")
    parser.add_argument("--source-receipt", type=Path, help="Reuse the exact fully validated recovery closure by hash")
    parser.add_argument("--resume", action="store_true", help="Resume an external native cook after verifying immutable dependencies")
    args = parser.parse_args()
    repair(args.source, args.destination, args.masters, args.inspection, args.extraction,
           args.encoder, args.catalog, args.select, args.receipt, args.source_receipt, args.resume)
