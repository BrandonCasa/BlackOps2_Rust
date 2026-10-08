"""Convert authored T6 trigger entities and clip-submodel brush unions."""
import hashlib
import itertools
import json
import math
from collections import Counter

import numpy as np

CLASSES = {"trigger_multiple", "trigger_use", "trigger_damage", "trigger_box",
           "trigger_radius", "trigger_lookat", "info_volume"}
BASIS = np.array([[1., 0., 0.], [0., 0., 1.], [0., -1., 0.]])
SCALE = .0254


def brush_vertices(brush):
    planes = [(np.array(p["normal"], dtype=float), p["distance"]) for p in brush["planes"]]
    for axis in range(3):
        normal = np.zeros(3)
        normal[axis] = 1
        planes.extend([(normal, brush["max"][axis]), (-normal, -brush["min"][axis])])
    normals = np.array([n for n, _ in planes])
    distances = np.array([d for _, d in planes])
    if not np.isfinite(normals).all() or not np.isfinite(distances).all():
        raise ValueError("non-finite native trigger brush")
    vertices = []
    for ids in itertools.combinations(range(len(planes)), 3):
        selected = normals[list(ids)]
        if abs(np.linalg.det(selected)) < 1e-8:
            continue
        point = np.linalg.solve(selected, distances[list(ids)])
        if np.all(normals @ point <= distances + .01) and not any(np.linalg.norm(point-v) < .01 for v in vertices):
            vertices.append(point)
    if not 4 <= len(vertices) <= 128:
        raise ValueError("empty or oversized native trigger convex brush")
    return vertices


def vector(value):
    result = np.array([float(v) for v in value.split()])
    if result.shape != (3,) or not np.isfinite(result).all():
        raise ValueError("invalid authored trigger vector")
    return result


def rotation(angles):
    pitch, yaw, roll = np.radians(vector(angles))
    cp, sp, cy, sy, cr, sr = math.cos(pitch), math.sin(pitch), math.cos(yaw), math.sin(yaw), math.cos(roll), math.sin(roll)
    # T6 forward is (cp*cy, cp*sy, -sp). Convert its Z-up basis once.
    return np.array([[cy, -sy, 0.], [sy, cy, 0.], [0., 0., 1.]]) @ np.array(
        [[cp, 0., sp], [0., 1., 0.], [-sp, 0., cp]]) @ np.array(
        [[1., 0., 0.], [0., cr, -sr], [0., sr, cr]])


def piece(identity, points, transform):
    points = np.array([BASIS @ transform @ p * SCALE for p in points])
    center = points.mean(axis=0)
    return {"id": identity, "position": center.tolist(), "vertices": (points-center).tolist()}


def convert(records, models):
    result, occurrences, cached_brushes = [], {}, {}
    for source in records:
        if source.get("classname") not in CLASSES:
            if source.get("classname", "").startswith("trigger_"):
                raise ValueError("unsupported native trigger class: " + source["classname"])
            continue
        digest = hashlib.sha256(json.dumps(source, sort_keys=True).encode()).hexdigest()[:24]
        identity = "trigger." + source.get("guid", "anonymous").lower() + "." + digest
        occurrence = occurrences.get(identity, 0)
        occurrences[identity] = occurrence + 1
        identity += f".{occurrence}"
        transform = rotation(source.get("angles", "0 0 0"))
        model = source.get("model", "")
        if model.startswith("*"):
            native = models[model[1:]]
            volumes = []
            for brush in native["hulls"]:
                key = (model, brush["brush"])
                if key not in cached_brushes:
                    cached_brushes[key] = brush_vertices(brush)
                volumes.append(piece(identity + ".brush." + str(brush["brush"]), cached_brushes[key], transform))
            if not volumes:
                raise ValueError("missing native trigger brush closure: " + identity)
            shape = {"kind": "convex", "volumes": volumes}
        elif source["classname"] == "trigger_box":
            extents = np.array([float(source[k]) for k in ("width", "length", "height")]) / 2
            if not np.isfinite(extents).all() or np.any(extents <= 0):
                raise ValueError("invalid native trigger box dimensions")
            corners = [np.array(signs) * extents for signs in itertools.product((-1., 1.), repeat=3)]
            shape = {"kind": "convex", "volumes": [piece(identity + ".box", corners, transform)]}
        elif source["classname"] == "trigger_radius":
            # Native unitrigger debug draws radius triggers from origin to origin+height.
            if np.any(vector(source.get("angles", "0 0 0")) != 0):
                raise ValueError("rotated radius trigger requires native orientation recovery")
            shape = {"kind": "cylinder", "radius": float(source["radius"]) * SCALE,
                     "height": float(source["height"]) * SCALE}
        else:
            raise ValueError("unsupported authored trigger shape: " + identity)
        result.append({"id": identity, "position": (BASIS @ vector(source["origin"]) * SCALE).tolist(),
                       "source": source, "shape": shape})
    return result


def validate(triggers, require):
    import re
    require(isinstance(triggers, list) and len(triggers) <= 4096, "invalid trigger count")
    identities, pieces = set(), set()
    def identity(value, known):
        require(isinstance(value, str) and len(value) < 128 and re.fullmatch(r"[A-Za-z0-9_.-]+", value)
                and value not in known, "invalid or duplicate trigger ID")
        known.add(value)
    def finite_vector(value):
        require(isinstance(value, list) and len(value) == 3
                and all(type(v) in (int, float) and math.isfinite(v) for v in value), "invalid trigger vector")
    for trigger in triggers:
        require(isinstance(trigger, dict) and set(trigger) == {"id", "position", "source", "shape"}, "invalid trigger fields")
        identity(trigger["id"], identities)
        finite_vector(trigger["position"])
        source = trigger["source"]
        require(isinstance(source, dict) and 1 <= len(source) <= 64
                and all(isinstance(k, str) and re.fullmatch(r"[A-Za-z0-9_]+", k)
                        and isinstance(v, str) and len(v) <= 4096 for k, v in source.items())
                and source.get("classname") in CLASSES, "invalid retained trigger source")
        shape = trigger["shape"]
        require(isinstance(shape, dict), "invalid trigger shape")
        if shape.get("kind") == "convex":
            require(set(shape) == {"kind", "volumes"} and source["classname"] != "trigger_radius", "invalid convex trigger fields")
            volumes = shape["volumes"]
            require(isinstance(volumes, list) and 1 <= len(volumes) <= 4096, "invalid trigger brush union")
            for volume in volumes:
                require(isinstance(volume, dict) and set(volume) == {"id", "position", "vertices"}, "invalid trigger brush fields")
                identity(volume["id"], pieces)
                finite_vector(volume["position"])
                points = volume["vertices"]
                require(isinstance(points, list) and 4 <= len(points) <= 128, "invalid trigger convex points")
                for point in points:
                    finite_vector(point)
                require(np.linalg.matrix_rank(np.array(points)-points[0]) == 3, "flat trigger brush")
        else:
            require(set(shape) == {"kind", "radius", "height"} and shape.get("kind") == "cylinder"
                    and source["classname"] == "trigger_radius", "invalid cylinder trigger fields")
            require(all(type(shape[k]) in (int, float) and math.isfinite(shape[k]) and shape[k] > 0
                        for k in ("radius", "height")), "invalid cylinder trigger dimensions")
    require(len(pieces) <= 32768, "too many trigger brush pieces")


def prepare(config):
    from prepare_gameplay_content import external
    from prepare_origins import entities
    inspection = external(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    pack = external(config["game"]["asset_pack"])
    entity_path = external(settings.get("entities", inspection / "game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents"))
    brush_path = external(settings.get("trigger_models", inspection / "solo-content/mud-triggers-recovery/map-triggers.json"))
    models = {str(m["model"]): m for m in json.loads(brush_path.read_text())}
    triggers = convert(entities(entity_path), models)
    def require(ok, reason):
        if not ok:
            raise ValueError(reason)
    validate(triggers, require)
    if not triggers:
        raise ValueError("authored trigger catalog is empty")
    game = json.loads((pack / "game.json").read_text(encoding="utf-8-sig"))
    game.setdefault("environment", {})["triggers"] = triggers
    (pack / "game.json").write_text(json.dumps(game, indent=2) + "\n", encoding="utf-8")
    receipt = {"triggers": len(triggers), "classes": dict(Counter(t["source"]["classname"] for t in triggers)),
               "convex_pieces": sum(len(t["shape"].get("volumes", [])) for t in triggers),
               "cylinders": sum(t["shape"]["kind"] == "cylinder" for t in triggers),
               "entities_sha256": hashlib.sha256(entity_path.read_bytes()).hexdigest(),
               "brushes_sha256": hashlib.sha256(brush_path.read_bytes()).hexdigest(),
               "approximation": "Brush vertices use 0.01 source-unit tolerance; Avian supplies touching. Native box dimensions/yaw and radius-height extents follow retained unitrigger script debug geometry. Native engine touch tolerances and trigger callbacks are not reconstructed here."}
    destination = external(settings.get("trigger_receipt", inspection / "solo-content/trigger-volumes-preparation.json"))
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(receipt))
