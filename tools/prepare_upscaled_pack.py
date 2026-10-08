"""Assemble qualified candidates into the existing PBR pack without geometry edits."""
import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
import shutil

from material_textures import inspect_dds
from texture_streaming import assemble, validate
from preparation_resources import PreparationResources
from import_origins import validate_pack
from upscale_textures import DEEPBUMP_UPSCALE_SHA256, RECIPE_VERSION, source_levels
from compressed_texture_cache import ENCODER_SHA256, FORMATS, materialize, target_format


def prepare(source, candidates, inventory, destination, receipt, pause_file=None, allow_partial=False, masters=None):
    resources = PreparationResources(pause_file, saturate=True)
    source, candidates, destination = source.resolve(), candidates.resolve(), destination.resolve()
    workspace = Path(__file__).resolve().parents[1]
    if destination.is_relative_to(workspace) or destination.is_relative_to(source) or source.is_relative_to(destination):
        raise ValueError("Preparation output must be external and separate from input pack")
    if destination.exists():
        raise ValueError("Use a fresh preparation destination; source packs stay untouched")
    if receipt.resolve().is_relative_to(workspace) or any(receipt.resolve().is_relative_to(root) for root in (source, candidates, destination)):
        raise ValueError("Assembly receipt must remain external and separate from its inputs/output")
    if (source/"texture-streaming.json").exists():
        raise ValueError("Candidate assembly requires the canonical unstreamed source pack")
    inventory_document = json.loads(inventory.read_text())
    entries = copy.deepcopy(inventory_document["textures"])
    source_hashes = {**inventory_document["source_scenes"], **inventory_document["source_buffers"],
                     "game.json": inventory_document["source_game_sha256"]}
    for uri, digest in source_hashes.items():
        resources.wait()
        if hashlib.sha256((source/uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Canonical pack changed after density inventory: {uri}")
    metadata = {}
    provenance = {}
    adopted = []
    pending = []
    for entry in entries:
        resources.wait()
        path = candidates / entry["uri"]
        if allow_partial and not path.with_suffix(".json").exists():
            pending.append(entry["uri"])
            entry["scale"] = 1
            entry["output_texels_per_meter"] = entry["minimum_texels_per_meter"]
            entry["density_exception"] = False
            continue
        record = json.loads(path.with_suffix(".json").read_text())
        if record["scale"] != entry["scale"] or record["uri"] != entry["uri"]:
            raise ValueError("Candidate does not match density inventory")
        source_hash = hashlib.sha256((source/entry["uri"]).read_bytes()).hexdigest()
        if record.get("source_runtime_sha256") != source_hash:
            raise ValueError(f"Source changed since candidate cooking: {entry['uri']}")
        if record["role"] != entry["role"] or record.get("upscaler") != "deepbump":
            raise ValueError("Migration requires the selected DeepBump material recipe")
        recipe = record.get("recipe", {})
        if recipe.get("version") != RECIPE_VERSION or record.get("density_floor") != entry.get("density_floor") or \
                record.get("density_metric") != entry.get("density_metric"):
            raise ValueError("Candidate requires the current strict-density recipe")
        if record.get("model_sha256") != DEEPBUMP_UPSCALE_SHA256 or recipe.get("model_sha256") != DEEPBUMP_UPSCALE_SHA256 or \
                recipe.get("encoder_sha256") != ENCODER_SHA256 or record.get("encoder_sha256") != ENCODER_SHA256 or \
                hashlib.sha256(json.dumps(recipe, sort_keys=True).encode()).hexdigest() != record.get("recipe_sha256"):
            raise ValueError("Candidate model, compression encoder or recipe checksum mismatch")
        if recipe.get("compression") != target_format(entry) or recipe.get("scale") != entry["scale"] or \
                recipe.get("role") != entry["role"] or recipe.get("color_space") != entry["color_space"] or \
                recipe.get("samplers") != sorted(entry.get("samplers", [20])):
            raise ValueError("Candidate compression or material recipe differs from inventory")
        if masters is None:
            raise ValueError("Assembly requires canonical masters to verify source mip receipts")
        levels, _, master_hash, _ = source_levels(source, masters, entry)
        if record.get("source_sha256") != master_hash or record.get("master_mip_sha256") != [
                hashlib.sha256(memoryview(level).cast("B")).hexdigest() for level in levels]:
            raise ValueError("Candidate source mip hashes differ from canonical masters")
        del levels
        if record.get("density_exception", False) != entry.get("density_exception", False):
            raise ValueError("Candidate density exception differs from inventory")
        if record.get("output_texels_per_meter") != entry.get("output_texels_per_meter"):
            raise ValueError("Candidate achieved density differs from inventory")
        if entry.get("density_exception") and (not inventory_document.get("cap_to_device") or
                max(entry["width"], entry["height"])*entry["scale"] != 16384):
            raise ValueError("Density exception requires the explicit device cap policy")
        if entry.get("density_applicable") and entry["minimum_texels_per_meter"] * entry["scale"] < entry["density_floor"] and not entry.get("density_exception"):
            raise ValueError(f"Candidate fails the strict density floor: {entry['uri']}")
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != record["output_sha256"]:
            raise ValueError(f"Candidate changed after cooking: {entry['uri']}")
        info = inspect_dds(raw)
        if info["dxgi"] != FORMATS[target_format(entry)] or record.get("format") != target_format(entry):
            raise ValueError("Candidate was not cooked to its required build-time block format")
        if (info["width"], info["height"]) != (entry["width"]*entry["scale"], entry["height"]*entry["scale"]):
            raise ValueError("Upscaled texture dimensions differ from the preparation contract")
        if info["mip_count"] != max(info["width"], info["height"]).bit_length():
            raise ValueError("Candidate requires a complete mip chain")
        prefix = int(math.log2(entry["scale"]))
        if [mip["sha256"] for mip in info["mips"][prefix:]] != record.get("retained_mip_sha256"):
            raise ValueError("Candidate changed retained source mip data")
        if record.get("source_mips_retained") and [mip["sha256"] for mip in info["mips"][prefix:]] != [
                mip["sha256"] for mip in inspect_dds((source/entry["uri"]).read_bytes())["mips"]]:
            raise ValueError("Candidate did not retain the original compressed mip tail")
        metadata[entry["uri"]] = info
        adopted.append(entry["uri"])
        provenance[record["model_sha256"]] = dict(credit=record["credit"], upscaler=record["upscaler"])
    # Original glTF buffer bytes, transforms, skins and clips are copied without
    # conversion. Only prepared texture metadata is updated below.
    def copy_file(source_file, destination_file):
        resources.wait()
        relative = Path(source_file).relative_to(source).as_posix()
        if relative in metadata:
            materialize(candidates/relative, Path(destination_file))
            return destination_file
        return shutil.copy2(source_file, destination_file)
    requested_bytes = sum(path.stat().st_size for path in source.rglob("*") if path.is_file()) + \
                      sum((candidates/uri).stat().st_size for uri in adopted)
    destination.parent.mkdir(parents=True, exist_ok=True)
    if requested_bytes > shutil.disk_usage(destination.parent).free:
        raise ValueError(f"Assembly needs at least {requested_bytes/2**30:.2f} GiB free")
    shutil.copytree(source, destination, copy_function=copy_file)
    policy = dict(target=inventory_document.get("world_density_floor"), metric=inventory_document.get("density_metric"),
                  cap_to_device=inventory_document.get("cap_to_device", False),
                  pending_textures=pending,
                  results=[dict(uri=entry["uri"], density=entry["output_texels_per_meter"],
                                applicable=entry["density_applicable"], exception=entry.get("density_exception", False))
                           for entry in entries if entry["uri"] in adopted])
    stream_index = assemble(destination, entries, provenance=provenance, resources=resources,
                            meshes=inventory_document.get("meshes"), density_policy=policy)
    bootstrap = {entry["uri"]: entry for entry in stream_index["textures"]}
    scene_hashes = {}
    for original in sorted(source.rglob("*.gltf")):
        resources.wait()
        document = json.loads(original.read_text())
        original_geometry = {key: copy.deepcopy(document.get(key)) for key in
            ("nodes", "meshes", "accessors", "bufferViews", "buffers", "skins", "animations", "scenes")}
        for material in document.get("materials", []):
            surface = material.get("extras", {}).get("surface", {})
            if surface.get("version") != 2:
                raise ValueError("Upscaling requires the canonical version-2 material pack")
            for channel in surface["channels"]:
                uri = channel["uri"]
                if uri not in bootstrap:
                    # Inventory covers placed geometry. Unused glTF materials
                    # retain their authored dependencies without neural work.
                    continue
                entry = bootstrap[uri]
                mip = entry["mips"][entry["bootstrap_mip"]]
                channel.update(width=mip["width"], height=mip["height"],
                    mip_count=len(entry["mips"])-entry["bootstrap_mip"],
                    format={28: "R8G8B8A8_UNORM", 29: "R8G8B8A8_UNORM_SRGB", 80: "BC4_UNORM",
                            83: "BC5_UNORM", 98: "BC7_UNORM", 99: "BC7_UNORM_SRGB"}[entry["format"]])
        for key, original_value in original_geometry.items():
            if document.get(key) != original_value:
                raise ValueError(f"Texture preparation changed scene geometry: {original.name}")
        relative = original.relative_to(source)
        (destination / relative).write_text(json.dumps(document, separators=(",", ":")) + "\n")
        scene_hashes[relative.as_posix()] = hashlib.sha256(original.read_bytes()).hexdigest()
    hashes = validate(destination, resources=resources)
    _, _, complete_closure = validate_pack(destination, exact=True)
    # Model buffers are unchanged byte-for-byte; preserve proof in the external
    # preparation receipt rather than adding research files to the runtime.
    buffer_hashes = {}
    for original in sorted(source.rglob("*.bin")):
        resources.wait()
        relative = original.relative_to(source)
        digest = hashlib.sha256(original.read_bytes()).hexdigest()
        if hashlib.sha256((destination / relative).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Mesh buffer changed: {relative}")
        buffer_hashes[relative.as_posix()] = digest
    result = dict(version=1, texture_count=len(entries), streaming_files=len(hashes),
                  validated_files=len(complete_closure),
                  adopted_textures=adopted, pending_textures=pending,
                  upgraded_textures=sum(entry["scale"] > 1 for entry in entries),
                  density_metric=inventory_document.get("density_metric"),
                  world_density_floor=inventory_document.get("world_density_floor"),
                  close_density_floor=inventory_document.get("close_density_floor"),
                  density_exceptions=[dict(uri=entry["uri"], achieved=entry["output_texels_per_meter"],
                                           requested=entry["density_floor"])
                                      for entry in entries if entry.get("density_exception") and entry["uri"] in adopted],
                  source_scenes=scene_hashes, unchanged_buffers=buffer_hashes,
                  source_game_sha256=hashlib.sha256((source/"game.json").read_bytes()).hexdigest(),
                  models=provenance)
    for uri, digest in source_hashes.items():
        resources.wait()
        if hashlib.sha256((source/uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Canonical source changed during assembly: {uri}")
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({key: result[key] for key in ("texture_count", "streaming_files")}), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "candidates", "inventory", "destination", "receipt", "masters"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--pause-file", type=Path)
    parser.add_argument("--allow-partial", action="store_true",
                        help="Qualification only: retain and explicitly report maps without candidates")
    args = parser.parse_args()
    prepare(args.source, args.candidates, args.inventory, args.destination, args.receipt, args.pause_file, args.allow_partial, args.masters)


if __name__ == "__main__":
    main()
