"""Assemble qualified candidates into the existing PBR pack without geometry edits."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import shutil

from material_textures import inspect_dds
from texture_streaming import assemble, validate
from preparation_resources import PreparationResources


def prepare(source, candidates, inventory, destination, receipt, pause_file=None):
    resources = PreparationResources(pause_file)
    source, candidates, destination = source.resolve(), candidates.resolve(), destination.resolve()
    workspace = Path(__file__).resolve().parents[1]
    if destination.is_relative_to(workspace) or destination.is_relative_to(source) or source.is_relative_to(destination):
        raise ValueError("Preparation output must be external and separate from input pack")
    if destination.exists():
        raise ValueError("Use a fresh preparation destination; source packs stay untouched")
    if (source/"texture-streaming.json").exists():
        raise ValueError("Candidate assembly requires the canonical unstreamed source pack")
    inventory_document = json.loads(inventory.read_text())
    entries = inventory_document["textures"]
    source_hashes = {**inventory_document["source_scenes"], **inventory_document["source_buffers"],
                     "game.json": inventory_document["source_game_sha256"]}
    for uri, digest in source_hashes.items():
        resources.wait()
        if hashlib.sha256((source/uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Canonical pack changed after density inventory: {uri}")
    metadata = {}
    provenance = {}
    for entry in entries:
        resources.wait()
        path = candidates / entry["uri"]
        record = json.loads(path.with_suffix(".json").read_text())
        if record["scale"] != entry["scale"] or record["uri"] != entry["uri"]:
            raise ValueError("Candidate does not match density inventory")
        source_hash = hashlib.sha256((source/entry["uri"]).read_bytes()).hexdigest()
        if record.get("source_runtime_sha256") != source_hash:
            raise ValueError(f"Source changed since candidate cooking: {entry['uri']}")
        if record["role"] != entry["role"] or record.get("upscaler") != "deepbump":
            raise ValueError("Migration requires the selected DeepBump material recipe")
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != record["output_sha256"]:
            raise ValueError(f"Candidate changed after cooking: {entry['uri']}")
        info = inspect_dds(raw)
        if (info["width"], info["height"]) != (entry["width"]*entry["scale"], entry["height"]*entry["scale"]):
            raise ValueError("Upscaled texture dimensions differ from the preparation contract")
        metadata[entry["uri"]] = info
        provenance[record["model_sha256"]] = dict(credit=record["credit"], upscaler=record["upscaler"])
    # Original glTF buffer bytes, transforms, skins and clips are copied without
    # conversion. Only prepared texture metadata is updated below.
    def copy_file(source_file, destination_file):
        resources.wait()
        return shutil.copy2(source_file, destination_file)
    shutil.copytree(source, destination, copy_function=copy_file)
    for entry in entries:
        resources.wait()
        shutil.copyfile(candidates / entry["uri"], destination / entry["uri"])
    stream_index = assemble(destination, entries, provenance=provenance, resources=resources)
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
                    format="R8G8B8A8_UNORM_SRGB" if channel["color_space"] == "srgb" else "R8G8B8A8_UNORM")
        for key, original_value in original_geometry.items():
            if document.get(key) != original_value:
                raise ValueError(f"Texture preparation changed scene geometry: {original.name}")
        relative = original.relative_to(source)
        (destination / relative).write_text(json.dumps(document, separators=(",", ":")) + "\n")
        scene_hashes[relative.as_posix()] = hashlib.sha256(original.read_bytes()).hexdigest()
    hashes = validate(destination, resources=resources)
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
    for name in ("source", "candidates", "inventory", "destination", "receipt"):
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--pause-file", type=Path)
    args = parser.parse_args()
    prepare(args.source, args.candidates, args.inventory, args.destination, args.receipt, args.pause_file)


if __name__ == "__main__":
    main()
