"""Recover an external full-resolution source from a validated streaming pack.

Only standard DDS tails and material descriptors are expanded. Mesh buffers,
scene structure, content and the input pack remain unchanged.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil

from import_origins import validate_pack


FORMATS = {28: "R8G8B8A8_UNORM", 29: "R8G8B8A8_UNORM_SRGB", 80: "BC4_UNORM",
           83: "BC5_UNORM", 98: "BC7_UNORM", 99: "BC7_UNORM_SRGB"}


def prepare(source, destination, receipt):
    source, destination = source.resolve(), destination.resolve()
    workspace = Path(__file__).resolve().parents[1]
    if destination.exists() or destination.is_relative_to(workspace) or \
            destination.is_relative_to(source) or source.is_relative_to(destination):
        raise ValueError("Use a fresh external destination separate from the source")
    if receipt.resolve().is_relative_to(workspace) or receipt.resolve().is_relative_to(source) or receipt.resolve().is_relative_to(destination):
        raise ValueError("Recovery receipt must remain external and separate from both packs")
    _, _, hashes = validate_pack(source, exact=True)
    destination.mkdir(parents=True)
    index_path = source / "texture-streaming.json"
    index = json.loads(index_path.read_text()) if index_path.exists() else {"textures": []}
    streams = {entry["uri"]: entry for entry in index["textures"]}
    retired = {entry["full_uri"] for entry in index["textures"]} | {"texture-streaming.json"}
    for uri, digest in hashes.items():
        # Snapshotting bytes is CPU/file work and does not own the GPU slot.
        if uri in retired:
            continue
        output = destination / uri
        output.parent.mkdir(parents=True, exist_ok=True)
        if uri in streams:
            shutil.copyfile(source / streams[uri]["full_uri"], output)
        elif output.suffix == ".gltf":
            document = json.loads((source / uri).read_text())
            for material in document.get("materials", []):
                for channel in material.get("extras", {}).get("surface", {}).get("channels", []):
                    entry = streams.get(channel["uri"])
                    if entry:
                        channel.update(width=entry["width"], height=entry["height"],
                                       mip_count=len(entry["mips"]), format=FORMATS[entry["format"]])
            output.write_text(json.dumps(document, separators=(",", ":")) + "\n")
        else:
            shutil.copyfile(source / uri, output)
        if hashlib.sha256((source / uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Source changed during snapshot: {uri}")
    _, _, expanded = validate_pack(destination, exact=True)
    unchanged = {uri: digest for uri, digest in hashes.items()
                 if uri.endswith(".bin") or uri == "game.json"}
    if any(expanded[uri] != digest for uri, digest in unchanged.items()):
        raise ValueError("Source recovery changed geometry/content")
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.write_text(json.dumps(dict(version=1, source_hashes=hashes,
                                      recovered_hashes=expanded, unchanged=unchanged), indent=2) + "\n")
    print(f"Recovered {len(streams)} full DDS chains; {len(expanded)} validated files", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("source", "destination", "receipt"):
        parser.add_argument(name, type=Path)
    args = parser.parse_args()
    prepare(args.source, args.destination, args.receipt)
