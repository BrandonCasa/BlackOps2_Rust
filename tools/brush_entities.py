from collections import Counter, defaultdict
import hashlib
import itertools
import json
import math
import re

import numpy as np

try:
    from .trigger_volumes import BASIS, SCALE, brush_vertices, piece, vector
except ImportError:
    from trigger_volumes import BASIS, SCALE, brush_vertices, piece, vector

CLASSES = {"script_brushmodel", "heli_height_lock"}


def convert(records, models):
    result, occurrences = [], {}
    for source in records:
        if not source.get("model", "").startswith("*"):
            continue
        if source.get("classname") not in CLASSES:
            if source.get("classname") not in {"trigger_multiple", "trigger_use", "trigger_damage", "trigger_lookat", "info_volume"}:
                raise ValueError("unsupported native brush entity class")
            continue
        digest = hashlib.sha256(json.dumps(source, sort_keys=True).encode()).hexdigest()[:24]
        identity = "brush." + source.get("guid", "anonymous").lower() + "." + digest
        occurrence = occurrences.get(identity, 0)
        occurrences[identity] = occurrence + 1
        identity += f".{occurrence}"
        model = models[source["model"][1:]]
        pitch, yaw, roll = np.radians(vector(source.get("angles", "0 0 0"))) / 2
        cp, sp, cy, sy, cr, sr = math.cos(pitch), math.sin(pitch), math.cos(yaw), math.sin(yaw), math.cos(roll), math.sin(roll)
        xyz = BASIS @ np.array([sr*cp*cy-cr*sp*sy, cr*sp*cy+sr*cp*sy, cr*cp*sy-sr*sp*cy])
        quaternion = [*xyz.tolist(), cr*cp*cy+sr*sp*sy]
        corners = np.array([BASIS @ np.array(p) * SCALE for p in itertools.product(*zip(model["min"], model["max"]))])
        result.append({"id": identity, "position": (BASIS @ vector(source["origin"]) * SCALE).tolist(),
                       "rotation": quaternion, "source": source,
                       "bounds": [corners.min(axis=0).tolist(), corners.max(axis=0).tolist()],
                       "volumes": [piece(identity + ".hull." + str(b["brush"]), brush_vertices(b), np.eye(3)) for b in model["hulls"]],
                       "visual_meshes": 0})
    return result


def validate(brushes, require):
    require(isinstance(brushes, list) and len(brushes) <= 4096, "invalid brush entity count")
    ids, pieces = set(), set()
    def identity(value, known):
        require(isinstance(value, str) and 0 < len(value) < 128 and re.fullmatch(r"[A-Za-z0-9_.-]+", value)
                and value not in known, "invalid or duplicate brush identity")
        known.add(value)
    def finite(value, width=3):
        require(isinstance(value, list) and len(value) == width
                and all(type(v) in (int, float) and math.isfinite(v) for v in value), "invalid brush vector")
    for brush in brushes:
        require(isinstance(brush, dict) and set(brush) == {"id", "position", "rotation", "bounds", "volumes", "source", "visual_meshes"}, "invalid brush fields")
        identity(brush["id"], ids)
        finite(brush["position"])
        finite(brush["rotation"], 4)
        require(abs(sum(v*v for v in brush["rotation"])-1) < 1e-5, "invalid brush quaternion")
        source = brush["source"]
        require(isinstance(source, dict) and 1 <= len(source) <= 64
                and all(isinstance(k, str) and re.fullmatch(r"[A-Za-z0-9_]+", k) and isinstance(v, str) and len(v) <= 4096 for k,v in source.items())
                and source.get("classname") in CLASSES and re.fullmatch(r"\*[1-9][0-9]*", source.get("model", "")), "invalid native brush source")
        require(isinstance(brush["bounds"], list) and len(brush["bounds"]) == 2, "invalid brush bounds")
        for bound in brush["bounds"]:
            finite(bound)
        require(all(a < b for a,b in zip(*brush["bounds"])), "empty brush bounds")
        require(type(brush["visual_meshes"]) is int and 0 <= brush["visual_meshes"] <= 4096, "invalid brush visual count")
        require(isinstance(brush["volumes"], list) and len(brush["volumes"]) <= 4096, "invalid brush hull count")
        for volume in brush["volumes"]:
            require(isinstance(volume, dict) and set(volume) == {"id", "position", "vertices"}, "invalid brush hull fields")
            identity(volume["id"], pieces)
            finite(volume["position"])
            require(isinstance(volume["vertices"], list) and 4 <= len(volume["vertices"]) <= 128, "invalid brush vertices")
            for point in volume["vertices"]:
                finite(point)
            require(np.linalg.matrix_rank(np.array(volume["vertices"])-volume["vertices"][0]) == 3, "flat brush hull")
    require(len(pieces) <= 32768, "too many brush hulls")


def validate_scene(brushes, document, require):
    entries = [(n["extras"]["brush_entity"], (i,n)) for i,n in enumerate(document["nodes"]) if "brush_entity" in n.get("extras", {})]
    roots = dict(entries)
    require(len(roots) == len(entries), "duplicate brush scene root")
    require(len(roots) == len(brushes), "brush scene root count mismatch")
    for brush in brushes:
        require(brush["id"] in roots, "missing brush scene root")
        index, node = roots[brush["id"]]
        require(index in document["scenes"][0]["nodes"] and node.get("translation") == brush["position"]
                and node.get("rotation") == brush["rotation"] and len(node.get("children", [])) == brush["visual_meshes"], "brush scene binding mismatch")
        require(all("mesh" in document["nodes"][child] for child in node.get("children", [])), "brush child has no mesh")


def separate_geometry(pack, brushes, ownership, scene):
    from prepare_origins import dependency, geometry
    manifest = json.loads(scene.read_text())
    inventory = json.loads((scene.parent / "inventory.private.json").read_text())
    positions, surfaces = geometry(dependency(scene.parent, {"file": manifest["geometry"], "sha256": manifest["geometry_sha256"]}))
    uv = np.frombuffer(dependency(scene.parent, {"file": manifest["corner_uv"], "sha256": manifest["corner_uv_sha256"]}), "<f4").reshape(-1, 2)
    if len(surfaces) != ownership["surfaces"] or len(manifest["surfaces"]) != len(surfaces):
        raise ValueError("brush/world surface correspondence mismatch")
    by_model = {int(b["source"]["model"][1:]): b for b in brushes}
    wanted, owned_surfaces = defaultdict(list), set()
    for model in ownership["models"][1:]:
        for i in range(model["first_surface"], model["first_surface"] + model["surface_count"]):
            if i in owned_surfaces or model["model"] not in by_model:
                raise ValueError("ambiguous or missing authored brush surface owner")
            owned_surfaces.add(i)
            first, count = surfaces[i][:2]
            material = inventory["materials"][manifest["surfaces"][i]["material"]]["name"]
            for corners in np.arange(first, first+count).reshape(-1,3)[:,[0,2,1]]:
                key = (material, positions[corners].astype("<f4").tobytes(), uv[corners].tobytes())
                wanted[key].append(model["model"])
    document = json.loads((pack / "map.gltf").read_text())
    if document["buffers"] != [{"uri": "map.bin", "byteLength": (pack / "map.bin").stat().st_size}]:
        raise ValueError("brush conversion requires the canonical map buffer")
    binary = bytearray((pack / "map.bin").read_bytes())
    def read(index):
        accessor = document["accessors"][index]; view = document["bufferViews"][accessor["bufferView"]]
        width = {"SCALAR":1,"VEC2":2,"VEC3":3,"VEC4":4}[accessor["type"]]
        dtype = np.dtype({5121:"u1",5123:"<u2",5125:"<u4",5126:"<f4"}[accessor["componentType"]])
        return np.ndarray((accessor["count"],width),dtype,binary,view.get("byteOffset",0)+accessor.get("byteOffset",0),
                          (view.get("byteStride",dtype.itemsize*width),dtype.itemsize)).copy()
    def append(values, kind, indices=False):
        values = np.ascontiguousarray(values, dtype="<u4" if indices else "<f4")
        while len(binary) % 4:
            binary.append(0)
        vi = len(document["bufferViews"])
        document["bufferViews"].append({"buffer":0,"byteOffset":len(binary),"byteLength":values.nbytes,"target":34963 if indices else 34962})
        binary.extend(values.tobytes())
        index = len(document["accessors"])
        document["accessors"].append({"bufferView":vi,"componentType":5125 if indices else 5126,"count":len(values),"type":kind,
                                      "min":values.min(axis=0).tolist(),"max":values.max(axis=0).tolist()})
        return index
    extracted, retired, moved = defaultdict(list), set(), 0
    for mesh_id, mesh in enumerate(document["meshes"]):
        if not mesh.get("name", "").startswith("world_"):
            continue
        remaining = []
        for primitive in mesh["primitives"]:
            attributes = {k:read(v) for k,v in primitive["attributes"].items()}
            triangles = read(primitive["indices"]).reshape(-1,3)
            material = document["materials"][primitive["material"]]["extras"]["surface"]["identity"]
            keep, selections = [], defaultdict(list)
            for corners in triangles:
                key = (material, attributes["POSITION"][corners].tobytes(), attributes["TEXCOORD_0"][corners].tobytes())
                if wanted.get(key):
                    selections[wanted[key].pop()].extend(corners)
                    moved += 1
                else:
                    keep.extend(corners)
            for model, selected in selections.items():
                extracted[model].append((primitive["material"], {k:array[selected] for k,array in attributes.items()}))
            if keep:
                if selections:
                    primitive["indices"] = append(np.array(keep).reshape(-1,1), "SCALAR", True)
                remaining.append(primitive)
        if remaining:
            mesh["primitives"] = remaining
        else:
            retired.add(mesh_id)
    if any(wanted.values()):
        raise ValueError("canonical world does not contain every original brush triangle")
    for world_scene in document["scenes"]:
        world_scene["nodes"] = [i for i in world_scene["nodes"] if document["nodes"][i].get("mesh") not in retired]
    for model, brush in by_model.items():
        children = []
        for material, arrays in extracted[model]:
            attributes = {k:append(a, {1:"SCALAR",2:"VEC2",3:"VEC3",4:"VEC4"}[a.shape[1]]) for k,a in arrays.items()}
            indices = append(np.arange(len(arrays["POSITION"])).reshape(-1,1), "SCALAR", True)
            mesh = len(document["meshes"])
            document["meshes"].append({"name":brush["id"],"primitives":[{"attributes":attributes,"indices":indices,"mode":4,"material":material}]})
            children.append(len(document["nodes"]))
            document["nodes"].append({"name":brush["id"]+".surface."+str(len(children)),"mesh":mesh})
        brush["visual_meshes"] = len(children)
        root = len(document["nodes"])
        document["nodes"].append({"name":brush["id"],"translation":brush["position"],"rotation":brush["rotation"],
                                  "children":children,"extras":{"brush_entity":brush["id"]}})
        document["scenes"][0]["nodes"].append(root)
    document["buffers"][0]["byteLength"] = len(binary)
    (pack / "map.bin").write_bytes(binary)
    (pack / "map.gltf").write_text(json.dumps(document,separators=(",", ":")), encoding="utf-8")
    return document, {"owned_surfaces":len(owned_surfaces),"moved_triangles":moved,"visual_meshes":sum(b["visual_meshes"] for b in brushes),
                      "preservation":"Canonical vertex attributes and material indexes copied unchanged; every owned triangle removed from static scene instances and admitted once under its authored root."}


def prepare(config):
    from prepare_gameplay_content import external
    from prepare_origins import entities
    from export_instances import export
    from pathlib import Path
    inspection = external(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    pack = external(config["game"]["asset_pack"])
    entity_path = external(settings.get("entities", inspection / "game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents"))
    clip_path = external(settings.get("trigger_models", inspection / "solo-content/mud-triggers-recovery/map-triggers.json"))
    owner_path = external(settings.get("brush_models", inspection / "solo-content/brush-models-recovery/brush-models.json"))
    if not owner_path.exists():
        export(external(settings.get("oat_source", inspection / "tooling/OAT/surface-7d027e8/source")),
               Path(config["assets"]["installation_root"]) / "zone/all/zm_tomb.ff", owner_path.parent,
               callback=Path(__file__).with_name("brush_models.cpp"))
    brushes = convert(entities(entity_path), {str(m["model"]):m for m in json.loads(clip_path.read_text())})
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    def require(ok, reason):
        if not ok:
            raise ValueError(reason)
    require(bool(brushes), "empty authored brush catalog")
    if "brush_entities" in game.get("environment", {}):
        previous = game["environment"]["brush_entities"]
        for before, after in zip(previous, brushes, strict=True):
            after["visual_meshes"] = before["visual_meshes"]
        require(previous == brushes, "source brush catalog changed; prepare from an unsplit external map pack")
        document = json.loads((pack / "map.gltf").read_text())
        geometry_receipt = {"already_prepared":True,"visual_meshes":sum(b["visual_meshes"] for b in brushes)}
    else:
        document, geometry_receipt = separate_geometry(pack, brushes, json.loads(owner_path.read_text()),
                                                      external(settings.get("scene", inspection / "full-scene-assets/texture-preview.json")))
        game.setdefault("environment", {})["brush_entities"] = brushes
        def mesh_count(scene):
            pending = list(scene["nodes"])
            count = 0
            while pending:
                node = document["nodes"][pending.pop()]
                count += "mesh" in node
                pending.extend(node.get("children", []))
            return count
        game["render_meshes"], game["collision_meshes"] = [mesh_count(s) for s in document["scenes"]]
        (pack / "game.json").write_text(json.dumps(game,indent=2)+"\n",encoding="utf-8")
    validate(brushes, require)
    validate_scene(brushes, document, require)
    receipt = {"entities":len(brushes),"classes":dict(Counter(b["source"]["classname"] for b in brushes)),
               "convex_pieces":sum(len(b["volumes"]) for b in brushes),"without_hulls":sum(not b["volumes"] for b in brushes),
               "geometry":geometry_receipt,"source_sha256":{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in (entity_path,clip_path,owner_path)},
               "approximation":"Convex plane intersections retain 0.01 source-unit tolerance. Source bounds and empty hull sets are preserved; helicopter-height volumes have no invented collision. Script visibility, movement, damage and quest callbacks remain consumer responsibilities."}
    destination = external(settings.get("brush_receipt", inspection / "solo-content/brush-entities-preparation.json"))
    destination.parent.mkdir(parents=True,exist_ok=True)
    destination.write_text(json.dumps(receipt,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(receipt))
