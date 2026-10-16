"""Change GLB image URIs while preserving all other JSON and binary chunks."""
import argparse
import hashlib
import json
import os
import re
import shutil
import struct
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument("--assets", type=Path, required=True)
p.add_argument("--model", type=Path)
p.add_argument("--audit", action="store_true")
p.add_argument("--ktx2", action="store_true", help="Point external DDS images at verified KTX2 files")
a = p.parse_args()

paths = [a.model] if a.model else sorted((a.assets / "model_export").rglob("*.glb"))
changed = []
missing = []
pattern = rb'"uri"\s*:\s*"[^"]*\.dds"' if a.ktx2 else rb'"uri"\s*:\s*"[^"]*[#?\\][^"]*"'
for path in paths:
    # Inspect only the JSON chunk, not every model's geometry and animation data.
    with path.open("rb") as stream:
        magic, version, _, length, kind = struct.unpack("<5I", stream.read(20))
        if magic != 0x46546C67 or version != 2 or kind != 0x4E4F534A:
            raise ValueError(f"Invalid GLB JSON header: {path}")
        metadata = stream.read(length)
    if not re.search(pattern, metadata):
        continue
    document = json.loads(metadata)
    edits = []
    for image in document.get("images", []):
        uri = image.get("uri")
        if not uri or uri.startswith("data:"):
            continue
        if a.ktx2 and not uri.endswith(".dds"):
            continue
        if not a.ktx2 and not any(c in uri for c in "#?\\"):
            continue
        source = (path.parent / uri.replace("\\", "/")).resolve()
        if not source.is_relative_to(a.assets.resolve()):
            raise RuntimeError(f"Image URI escapes isolated asset root: {path}: {uri}")
        destination = a.assets.resolve() / "images" / source.with_suffix(".ktx2").name if a.ktx2 else source
        if not destination.is_file():
            missing.append({"model":str(path),"uri":uri})
            continue
        if not a.ktx2:
            safe = "uri_" + hashlib.sha256(source.name.encode()).hexdigest()[:20] + source.suffix
            destination = source.with_name(safe)
        normalized = Path(os.path.relpath(destination, path.parent)).as_posix()
        edits.append({"from":uri,"to":normalized})
        if not a.audit:
            if not a.ktx2:
                shutil.copyfile(source, destination)
            image["uri"] = normalized
            if a.ktx2:
                image["mimeType"] = "image/ktx2"
    if edits:
        changed.append({"model":str(path),"images":edits})
        if not a.audit:
            # Retain the complete original tail, including chunk headers/padding.
            original = path.read_bytes()
            tail = original[20 + length:]
            payload = json.dumps(document, separators=(",", ":")).encode("utf-8")
            payload += b" " * (-len(payload) % 4)
            output = struct.pack("<5I", magic, version, 20 + len(payload) + len(tail),
                                 len(payload), kind) + payload + tail
            if output[20 + len(payload):] != tail or json.loads(payload) != document:
                raise RuntimeError(f"GLB image URI edit verification failed: {path}")
            temporary = path.with_suffix(".glb.uri-tmp")
            temporary.write_bytes(output)
            os.replace(temporary, path)
print(json.dumps({"models":len(paths),"changed":changed,"missing":missing},indent=2))
if missing: raise SystemExit("Required source images missing; see URI report")
