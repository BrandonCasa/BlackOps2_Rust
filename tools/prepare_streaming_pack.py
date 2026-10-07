# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13", "psutil>=7,<8", "soundfile>=0.13,<0.14"]
# ///
"""Stream a qualified canonical PBR pack without changing its authored detail.

DeepBump candidates retain their separate provenance-qualified assembly path.
This path integrates residency with the complete canonical runtime closure.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

from preparation_resources import PreparationResources
from texture_inventory import inventory
from texture_streaming import assemble, validate
from import_origins import validate_pack


def prepare(source, destination, receipt):
    source, destination = source.resolve(), destination.resolve()
    if destination.exists() or source == destination or destination.is_relative_to(source):
        raise ValueError("Use a fresh external destination separate from originals")
    workspace = Path(__file__).resolve().parents[1]
    if destination.is_relative_to(workspace):
        raise ValueError("Preparation must emit an external pack before validated import")
    resources = PreparationResources()
    _, _, closure = validate_pack(source)
    document = inventory(source)
    entries = document["textures"]
    for entry in entries:
        entry["scale"] = 1
    def copy_file(source_file, destination_file):
        resources.wait()
        return shutil.copy2(source_file, destination_file)
    shutil.copytree(source, destination, copy_function=copy_file)
    streams = assemble(destination, entries, resources=resources)
    by_uri = {entry["uri"]: entry for entry in streams["textures"]}
    for scene in destination.rglob("*.gltf"):
        resources.wait()
        content = json.loads(scene.read_text())
        for material in content.get("materials", []):
            for channel in material.get("extras", {}).get("surface", {}).get("channels", []):
                entry = by_uri.get(channel["uri"])
                if entry is None:
                    continue
                mip = entry["mips"][entry["bootstrap_mip"]]
                channel.update(width=mip["width"],height=mip["height"],
                    mip_count=len(entry["mips"])-entry["bootstrap_mip"])
        scene.write_text(json.dumps(content,separators=(",", ":"))+"\n")
    validate(destination, resources=resources)
    _, _, hashes = validate_pack(destination)
    unchanged = {**document["source_buffers"], "game.json": document["source_game_sha256"]}
    for uri,digest in unchanged.items():
        if hashlib.sha256((destination/uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Streaming changed geometry/content: {uri}")
    for uri,digest in closure.items():
        if hashlib.sha256((source/uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Canonical source changed during preparation: {uri}")
    receipt.parent.mkdir(parents=True,exist_ok=True)
    receipt.write_text(json.dumps(dict(version=1,textures=len(entries),
        source_scenes=document["source_scenes"],unchanged=unchanged,
        validated_files=len(hashes)),indent=2)+"\n")
    print(f"Prepared {len(entries)} shared streaming textures; {len(hashes)} validated runtime files",flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source",type=Path)
    parser.add_argument("destination",type=Path)
    parser.add_argument("receipt",type=Path)
    args = parser.parse_args()
    prepare(args.source,args.destination,args.receipt)
