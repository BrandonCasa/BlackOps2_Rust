"""Thin invocation of unmodified upstream APIs; no decoding/conversion implementation."""
import os
import json
import shutil
import sys
from pathlib import Path
root = Path(sys.argv[1])
sys.path.insert(0, sys.argv[2])
from scripts.cod import backends
from scripts.cod.files import read_json
from scripts.cod import shell, brushmodels, gameobjects, metadata
from scripts.cod.volumes import hull_faces, hull_points
from scripts.cod.materials import classify_materials, material_file_stem, surface_images

manifest_path = root / "target/extracted/origins-collision-work/gw/maps/mp/zm_tomb.d3dbsp.gfxworld.json"
manifest = read_json(str(manifest_path))
clip = read_json(str(root / "target/assets/clipmap.json"))
ents = str(root / "target/extracted/origins-collision-work/ents/maps/mp/zm_tomb.d3dbsp.ents")
facts = gameobjects.facts_from_ents("zm_tomb", ents, "t6")
placements = brushmodels.preflight(manifest, clip, os.path.exists(ents), ents, facts)
matroot = root / "target/extracted/world-base-work/mat/materials"
zone = Path(sys.argv[3]) / "zm_tomb.ff"
backends.for_fastfile(str(zone)).dump(str(zone), str(matroot.parent), "material")
used = {surface["material"] for surface in manifest["surfaces"] if "material" in surface}
names = {str(path.relative_to(matroot))[:-5] for path in matroot.rglob("*.json")}
colormap, decal, drop, states = classify_materials(str(matroot), names, "t6")
normalize = lambda name: name.replace(chr(92), "/")
colormap = {normalize(key): value for key, value in colormap.items()}
states = {normalize(key): value for key, value in states.items()}
decal = {normalize(key) for key in decal}
drop = {normalize(key) for key in drop}
mapped = {name: colormap[material_file_stem(name)] for name in used if material_file_stem(name) in colormap}
boundstates = {name: states[material_file_stem(name)] for name in used if material_file_stem(name) in states}
bounddecal = {name for name in used if material_file_stem(name) in decal}
bounddrop = {name for name in used if material_file_stem(name) in drop}
textures = root / "target/extracted/images"
wanted = surface_images(mapped, bounddrop, boundstates)
missing = sorted(name for name in wanted if not (textures / (name + ".png")).is_file())
print("Material bindings", len(mapped), "of", len(used), "source textures missing", missing, flush=True)
if missing:
    raise SystemExit("Required world source images are missing: " + ", ".join(missing))
shell.export_shell(str(manifest_path), str(root / "target/assets/world.glb"), str(root / "target/assets/decals.glb"), str(textures), mapped, bounddecal, bounddrop, boundstates, placements, lightmap_uv=None)
print("DONE no baking; missing source images", missing, flush=True)

# Existing upstream hull exporter returns local Y-up metre vertices without placement.
hulls = {}
for index in sorted({index for model in clip["cmodels"][1:] for index in model.get("brushes", [])}):
    brush = clip["brushes"][index]
    faces = hull_faces(brush)
    if not faces:
        raise SystemExit(f"External brush hull export missing faces for brush {index}")
    hulls[str(index)] = {"contents": brush["contents"], "vertices": hull_points(faces)}
(root / "target/assets/brush_hulls.json").write_text(json.dumps(hulls), encoding="utf-8")
print("External movable brush hulls", len(hulls), flush=True)

# Keep external entity/PATHDATA records ordinary and unchanged for game-owned binding.
backend = backends.for_fastfile(str(zone))
work = root / "target/extracted/origins-collision-work"
backend.dump(str(zone), str(work / "raw"), "rawfile")
backend.dump(str(zone), str(work / "path"), "pathdata")
metadata_out = root / "target/extracted/origins-metadata"
metadata_out.mkdir(parents=True, exist_ok=True)
metadata_path, _ = metadata.write_metadata(str(metadata_out), "zm_tomb", "t6", ents,
    str(work / "raw"), str(work / "ents/zone_source/zm_tomb.zone"))
shutil.copyfile(metadata_path, root / "target/assets/entities.json")
pathdata = list((work / "path").rglob("*.pathdata.json"))
if len(pathdata) != 1:
    raise SystemExit(f"Expected one external Origins PATHDATA JSON, found {len(pathdata)}")
shutil.copyfile(pathdata[0], root / "target/assets/pathdata.json")
print("External entity metadata and native PATHDATA staged", flush=True)
