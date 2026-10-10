import argparse
import json
from pathlib import Path
import shutil

def assemble(pack, textures, provenance=None, resources=None, **unused):
    try:
        from .material_textures import inspect_dds
    except ImportError:
        from material_textures import inspect_dds
    entries = []
    for texture in textures:
        if resources is not None:
            resources.wait()
        info = inspect_dds((pack / texture["uri"]).read_bytes())
        entries.append(dict(uri=texture["uri"], width=info["width"], height=info["height"],
                            format=info["dxgi"], mip_count=info["mip_count"]))
    return dict(textures=entries)


def migrate(pack):
    pack = Path(pack).resolve()
    index = pack / "texture-streaming.json"
    if not index.exists():
        return 0
    entries = json.loads(index.read_text())["textures"]
    by_uri = {entry["uri"]: entry for entry in entries}
    for entry in entries:
        source = (pack / entry["full_uri"]).resolve()
        target = (pack / entry["uri"]).resolve()
        if not source.is_relative_to(pack) or not target.is_relative_to(pack) or source == target:
            raise ValueError("Resident texture migration requires distinct pack-local paths")
        shutil.copyfile(source, target)
    formats = {28: "R8G8B8A8_UNORM", 29: "R8G8B8A8_UNORM_SRGB", 80: "BC4_UNORM",
               83: "BC5_UNORM", 98: "BC7_UNORM", 99: "BC7_UNORM_SRGB"}
    for path in sorted(pack.rglob("*.gltf")):
        document = json.loads(path.read_text())
        changed = False
        for material in document.get("materials", []):
            for channel in material.get("extras", {}).get("surface", {}).get("channels", []):
                entry = by_uri.get(channel.get("uri"))
                if entry is not None:
                    channel.update(width=entry["width"], height=entry["height"],
                                   mip_count=len(entry["mips"]), format=formats[entry["format"]])
                    changed = True
        if changed:
            path.write_text(json.dumps(document, separators=(",", ":")) + "\n")
    for entry in entries:
        (pack / entry["full_uri"]).unlink()
    index.unlink()
    return len(entries)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("pack", type=Path)
    print(dict(resident_textures=migrate(parser.parse_args().pack)))
