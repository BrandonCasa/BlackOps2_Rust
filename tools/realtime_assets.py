# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///
from pathlib import Path
import json
import os
import tempfile

try:
    from .texture_streaming import validate as validate_streaming
except ImportError:
    from texture_streaming import validate as validate_streaming


def replace_bytes(path, raw):
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.migration-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(raw)
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def strip_lighting(document):
    document.pop("extras", None)
    for material in document.get("materials", []):
        surface = material.get("extras", {}).get("surface", {})
        if surface.get("preparation", {}).get("kind") == "tile_macro_bake":
            surface["preparation"]["kind"] = "tile_macro_layers"
        if surface.get("version") == 2:
            surface["channels"] = [c for c in surface["channels"] if c["role"] != "baked_color"]
    for mesh in document.get("meshes", []):
        mesh.pop("extras", None)
        for primitive in mesh["primitives"]:
            primitive["attributes"].pop("TEXCOORD_1", None)
    for node in document.get("nodes", []):
        extras = node.get("extras", {})
        was_lit_instance = "lighting_id" in extras.get("lighting", {})
        extras.pop("lighting", None)
        if extras.get("static_instance") is True or was_lit_instance or node.get("name") == "static_instance":
            extras["static_instance"] = True
        if extras:
            node["extras"] = extras
        else:
            node.pop("extras", None)


def compact_geometry(document, pack):
    used = set()
    for mesh in document.get("meshes", []):
        for primitive in mesh["primitives"]:
            used.update(primitive["attributes"].values())
            if "indices" in primitive:
                used.add(primitive["indices"])
    for skin in document.get("skins", []):
        if "inverseBindMatrices" in skin:
            used.add(skin["inverseBindMatrices"])
    for animation in document.get("animations", []):
        for sampler in animation["samplers"]:
            used.update((sampler["input"], sampler["output"]))
    mapping = {old: new for new, old in enumerate(sorted(used))}
    accessors = [document["accessors"][old] for old in sorted(used)]
    views = sorted({a["bufferView"] for a in accessors})
    view_mapping = {old: new for new, old in enumerate(views)}
    raw = [(pack / b["uri"]).read_bytes() for b in document["buffers"]]
    output = bytearray()
    kept_views = []
    for old in views:
        view = document["bufferViews"][old]
        output.extend(b"\0" * (-len(output) % 4))
        offset = len(output)
        start = view.get("byteOffset", 0)
        output.extend(raw[view["buffer"]][start:start + view["byteLength"]])
        kept_views.append({**view, "buffer": 0, "byteOffset": offset})
    for accessor in accessors:
        accessor["bufferView"] = view_mapping[accessor["bufferView"]]
    for mesh in document.get("meshes", []):
        for primitive in mesh["primitives"]:
            primitive["attributes"] = {k: mapping[v] for k, v in primitive["attributes"].items()}
            if "indices" in primitive:
                primitive["indices"] = mapping[primitive["indices"]]
    for skin in document.get("skins", []):
        if "inverseBindMatrices" in skin:
            skin["inverseBindMatrices"] = mapping[skin["inverseBindMatrices"]]
    for animation in document.get("animations", []):
        for sampler in animation["samplers"]:
            sampler["input"], sampler["output"] = mapping[sampler["input"]], mapping[sampler["output"]]
    target = pack / document["buffers"][0]["uri"]
    before = sum(len(b) for b in raw)
    replace_bytes(target, output)
    document["buffers"] = [{"uri": document["buffers"][0]["uri"], "byteLength": len(output)}]
    document["bufferViews"], document["accessors"] = kept_views, accessors
    return before - len(output)


def migrate(pack):
    pack = Path(pack).resolve()
    kept = {(pack / uri).resolve() for uri in validate_streaming(pack)}
    removed_bytes = 0
    for path in sorted(pack.rglob("*.gltf")):
        document = json.loads(path.read_text())
        strip_lighting(document)
        if path.name == "map.gltf":
            removed_bytes += compact_geometry(document, pack)
        for image in document.get("images", []):
            kept.add((path.parent / image["uri"]).resolve())
        for material in document.get("materials", []):
            for channel in material.get("extras", {}).get("surface", {}).get("channels", []):
                if "uri" in channel:
                    kept.add((pack / channel["uri"]).resolve())
        replace_bytes(path, (json.dumps(document, separators=(",", ":")) + "\n").encode("utf-8"))
    def content_images(value):
        if isinstance(value, dict):
            for item in value.values():
                content_images(item)
        elif isinstance(value, list):
            for item in value:
                content_images(item)
        elif isinstance(value, str) and value.endswith((".png", ".jpg", ".jpeg", ".dds")):
            kept.add((pack / value).resolve())
    content_images(json.loads((pack / "game.json").read_text()))
    removed = []
    for path in pack.rglob("*"):
        if path.suffix.lower() in {".png", ".dds", ".jpg", ".jpeg"} and path.resolve() not in kept:
            removed.append(path.relative_to(pack).as_posix())
            removed_bytes += path.stat().st_size
            path.unlink()
    (pack / "preparation.json").unlink(missing_ok=True)
    return {"removed_images": removed, "removed_bytes": removed_bytes}


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description='Retire native incident-light data from a prepared runtime closure, preserving originals.')
    parser.add_argument("pack", type=Path)
    print(json.dumps(migrate(parser.parse_args().pack)))
