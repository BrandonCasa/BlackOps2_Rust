import argparse
import hashlib
import json
from pathlib import Path
import shutil

from import_origins import validate_pack


FORMATS = {28: "R8G8B8A8_UNORM", 29: "R8G8B8A8_UNORM_SRGB", 49: "R8G8_UNORM", 61: "R8_UNORM", 80: "BC4_UNORM",
           83: "BC5_UNORM", 98: "BC7_UNORM", 99: "BC7_UNORM_SRGB"}


def copy_verified(source, output, expected):
    shutil.copyfile(source, output)
    for path in (source, output):
        with path.open("rb") as stream:
            if hashlib.file_digest(stream, "sha256").hexdigest() != expected:
                raise ValueError(f"Source or copied data changed during snapshot: {source.name}")


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
    expected_copies = {}
    for uri, digest in hashes.items():
        if uri in retired:
            continue
        output = destination / uri
        output.parent.mkdir(parents=True, exist_ok=True)
        if uri in streams:
            full_uri = streams[uri]["full_uri"]
            expected_copies[uri] = hashes[full_uri]
            copy_verified(source / full_uri, output, expected_copies[uri])
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
            expected_copies[uri] = digest
            copy_verified(source / uri, output, digest)
        if hashlib.sha256((source / uri).read_bytes()).hexdigest() != digest:
            raise ValueError(f"Source changed during snapshot: {uri}")
    _, _, expanded = validate_pack(destination, exact=True)
    if any(expanded[uri] != expected for uri, expected in expected_copies.items()):
        raise ValueError("Recovered copied data differs from the initially validated source")
    unchanged = {uri: digest for uri, digest in hashes.items()
                 if uri.endswith(".bin") or uri == "game.json"}
    if any(expanded[uri] != digest for uri, digest in unchanged.items()):
        raise ValueError("Source recovery changed geometry/content")
    receipt.parent.mkdir(parents=True, exist_ok=True)
    receipt.write_text(json.dumps(dict(version=1, source_hashes=hashes,
                                      recovered_hashes=expanded, unchanged=unchanged), indent=2) + "\n")
    print(f"Recovered {len(streams)} full KTX2 chains; {len(expanded)} validated files", flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description='Recover an external full-resolution source from a validated streaming pack.\n\nOnly standard KTX2 tails and material descriptors are expanded. Mesh buffers,\nscene structure, content and the input pack remain unchanged.\n')
    for name in ("source", "destination", "receipt"):
        parser.add_argument(name, type=Path)
    args = parser.parse_args()
    prepare(args.source, args.destination, args.receipt)
