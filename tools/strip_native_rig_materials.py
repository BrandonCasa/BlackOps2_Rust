"""Remove redundant PBR texture requests from staged native-model rigs.

Native .native_model material files own rendering. Keep glTF geometry, skins,
animations and all binary chunks unchanged for Bevy's skeleton/scene loader.
"""
import argparse
import json
import os
import re
import struct
from pathlib import Path


def read_glb(data):
    magic, version, length = struct.unpack_from("<III", data)
    assert (magic, version, length) == (0x46546C67, 2, len(data))
    chunks = []
    cursor = 12
    while cursor < len(data):
        length, kind = struct.unpack_from("<II", data, cursor)
        end = cursor + 8 + length
        assert end <= len(data) and length % 4 == 0
        chunks.append((kind, data[cursor + 8:end]))
        cursor = end
    assert chunks and chunks[0][0] == 0x4E4F534A
    return json.loads(chunks[0][1]), chunks[1:]


def strip_materials(document):
    for key in ("materials", "textures", "images", "samplers"):
        document.pop(key, None)
    for mesh in document.get("meshes", []):
        for primitive in mesh["primitives"]:
            primitive.pop("material", None)
            extensions = primitive.get("extensions", {})
            extensions.pop("KHR_materials_variants", None)
    document.get("extensions", {}).pop("KHR_materials_variants", None)
    for key in ("extensionsUsed", "extensionsRequired"):
        if key in document:
            document[key] = [name for name in document[key]
                             if not name.startswith("KHR_materials_")]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    root = args.assets.resolve()
    changed = []
    checked = 0
    # The runtime also loads weapon scene aliases through this authoritative
    # scene-to-native-model map; those rigs do not have a _lod suffix.
    scenes = {}
    bindings = root / "source-model-clips.json"
    if bindings.is_file():
        for binding in json.loads(bindings.read_text(encoding="utf-8")):
            path = root / binding["scene"]
            scenes[path] = root / "model_export" / (binding["model"] + ".native_model")
    for path in sorted((root / "model_export").rglob("*_lod*.glb")):
        stem = re.sub(r"_lod\d+$", "", path.stem)
        scenes[path] = path.with_name(stem + ".native_model")
    for path, native_path in sorted(scenes.items()):
        if not native_path.is_file():
            continue
        native = json.loads(native_path.read_text(encoding="utf-8"))
        material_paths = {surface["materialFile"] for lod in native["lods"]
                          for surface in lod["surfaces"]}
        assert material_paths and all((root / name).is_file() for name in material_paths), native_path
        original, binary_chunks = read_glb(path.read_bytes())
        stripped = json.loads(json.dumps(original))
        strip_materials(stripped)
        checked += 1
        if original == stripped:
            continue
        payload = json.dumps(stripped, separators=(",", ":")).encode("utf-8")
        payload += b" " * (-len(payload) % 4)
        chunks = [(0x4E4F534A, payload), *binary_chunks]
        body = b"".join(struct.pack("<II", len(data), kind) + data for kind, data in chunks)
        output = struct.pack("<III", 0x46546C67, 2, len(body) + 12) + body
        verified, verified_chunks = read_glb(output)
        assert verified == stripped and verified_chunks == binary_chunks
        for key in ("nodes", "skins", "animations", "accessors", "bufferViews", "buffers", "scenes"):
            assert verified.get(key) == original.get(key), (path, key)
        temporary = path.with_suffix(".glb.native-rig-tmp")
        temporary.write_bytes(output)
        os.replace(temporary, path)
        changed.append({"scene": str(path.relative_to(root)),
                        "removed_images": len(original.get("images", [])),
                        "preserved_binary_bytes": sum(len(data) for _, data in binary_chunks)})
    assert checked, "No staged native model rigs found"
    args.report.write_text(json.dumps({"checked": checked, "changed": changed}, indent=2) + "\n", encoding="utf-8")
    print(f"Verified {checked} native rigs; removed PBR material references from {len(changed)}")


if __name__ == "__main__":
    main()
