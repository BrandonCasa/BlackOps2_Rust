"""URI-only staging through unchanged pygltflib; no geometry/material conversion.
https://pypi.org/project/pygltflib/1.16.5/
"""
import argparse
import hashlib
import json
import re
import shutil
import sys
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument("--sdk", type=Path, required=True)
p.add_argument("--assets", type=Path, required=True)
p.add_argument("--model", type=Path)
p.add_argument("--audit", action="store_true")
a = p.parse_args()
sys.path.insert(0, str(a.sdk))
from pygltflib import GLTF2

paths = [a.model] if a.model else sorted((a.assets / "model_export").rglob("*.glb"))
changed = []
missing = []
for path in paths:
    # Cheap text prefilter; the external SDK handles all glTF parsing and writing.
    if not re.search(rb'"uri"\s*:\s*"[^"]*[#?\\][^"]*"', path.read_bytes()):
        continue
    gltf = GLTF2.load_binary(str(path))
    edits = []
    for image in gltf.images:
        uri = image.uri
        if not uri or uri.startswith("data:") or not any(c in uri for c in "#?\\"):
            continue
        source = (path.parent / uri.replace("\\", "/")).resolve()
        if not source.is_relative_to(a.assets.resolve()):
            raise RuntimeError(f"Image URI escapes isolated asset root: {path}: {uri}")
        if not source.is_file():
            missing.append({"model":str(path),"uri":uri})
            continue
        safe = "uri_" + hashlib.sha256(source.name.encode()).hexdigest()[:20] + source.suffix
        destination = source.with_name(safe)
        normalized = Path(__import__("os").path.relpath(destination, path.parent)).as_posix()
        edits.append({"from":uri,"to":normalized})
        if not a.audit:
            shutil.copyfile(source, destination)
            image.uri = normalized
    if edits:
        changed.append({"model":str(path),"images":edits})
        if not a.audit:
            # The established SDK preserves the binary payload and all glTF objects.
            original_binary = gltf.binary_blob()
            gltf.save_binary(str(path))
            if GLTF2.load_binary(str(path)).binary_blob() != original_binary:
                raise RuntimeError(f"SDK unexpectedly changed binary payload: {path}")
print(json.dumps({"models":len(paths),"changed":changed,"missing":missing},indent=2))
if missing: raise SystemExit("Required source images missing; see URI report")
