"""Inventory the complete runtime texture closure and its physical UV density.

Reads prepared scenes only. Geometry and original files are never modified.
The receipt lives outside the repository and drives DeepBump preparation.
"""
import argparse
from collections import Counter
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath

# Inventory is preparation, not a benchmark; use one CPU thread alongside the
# game's shared validation slot. This does not alter density calculations.
os.environ["OMP_NUM_THREADS"] = "1"
os.environ["OPENBLAS_NUM_THREADS"] = "1"
import numpy as np
from preparation_resources import PreparationResources


def contained(root, path):
    resolved = path.resolve()
    if not resolved.is_relative_to(root.resolve()):
        raise ValueError(f"Dependency escapes pack: {path}")
    return resolved


def accessor(document, scene, index):
    item = document["accessors"][index]
    if "sparse" in item:
        raise ValueError("Sparse runtime accessor needs explicit density support")
    view = document["bufferViews"][item["bufferView"]]
    buffer = document["buffers"][view.get("buffer", 0)]
    path = contained(scene.parent, scene.parent / buffer["uri"])
    dtype = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}[item["componentType"]]
    channels = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[item["type"]]
    width = np.dtype(dtype).itemsize
    offset = view.get("byteOffset", 0) + item.get("byteOffset", 0)
    raw = np.memmap(path, dtype="u1", mode="r")
    return np.ndarray((item["count"], channels), dtype=dtype, buffer=raw,
                      offset=offset, strides=(view.get("byteStride", width * channels), width))


def mesh_scales(document):
    """Conservative largest singular scale over every scene placement."""
    result = {}
    def visit(index, parent):
        node = document["nodes"][index]
        if "matrix" in node:
            local = np.asarray(node["matrix"], np.float64).reshape(4, 4).T[:3, :3]
        else:
            x, y, z, w = node.get("rotation", [0, 0, 0, 1])
            local = np.asarray([
                [1 - 2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w)],
                [2*(x*y+z*w), 1 - 2*(x*x+z*z), 2*(y*z-x*w)],
                [2*(x*z-y*w), 2*(y*z+x*w), 1 - 2*(x*x+y*y)]])
            local = local @ np.diag(node.get("scale", [1, 1, 1]))
        transform = parent @ local
        if "mesh" in node:
            scale = float(np.linalg.svd(transform, compute_uv=False).max())
            result[node["mesh"]] = max(result.get(node["mesh"], 0), scale)
        for child in node.get("children", []):
            visit(child, transform)
    for scene in document.get("scenes", []):
        for root in scene.get("nodes", []):
            visit(root, np.eye(3))
    return result


def density(document, scene, primitive, width, height, layer, scale, resources):
    attrs = primitive["attributes"]
    if layer == 0:
        uv = accessor(document, scene, attrs["TEXCOORD_0"])
    elif layer in (1, 2) and "_ORIGINS_LAYER_UV" in attrs:
        uv = accessor(document, scene, attrs["_ORIGINS_LAYER_UV"])
        uv = uv[:, (layer - 1)*2:layer*2]
    elif layer == 3 and "_ORIGINS_LAYER3_UV" in attrs:
        uv = accessor(document, scene, attrs["_ORIGINS_LAYER3_UV"])[:, :2]
    else:
        uv = accessor(document, scene, attrs["TEXCOORD_0"])
    positions = accessor(document, scene, attrs["POSITION"])
    indices = accessor(document, scene, primitive["indices"]).reshape(-1, 3)
    minimum, maximum, area = math.inf, 0., 0.
    # Bound scratch RAM independently of the world mesh size.
    for start in range(0, len(indices), 8192):
        resources.wait()
        corners = indices[start:start + 8192]
        p = positions[corners].astype(np.float64)
        t = uv[corners].astype(np.float64) * [width, height]
        e1, e2 = p[:, 1]-p[:, 0], p[:, 2]-p[:, 0]
        normal = np.cross(e1, e2)
        triangle_area = np.linalg.norm(normal, axis=1) * .5 * scale * scale
        a = np.linalg.norm(e1, axis=1)
        b = np.sum(e1 * e2, axis=1) / np.maximum(a, 1e-12)
        c = np.sqrt(np.maximum(np.sum(e2 * e2, axis=1) - b*b, 0))
        valid = (a > 1e-9) & (c > 1e-9)
        if not valid.any():
            continue
        # UV Jacobian in an orthonormal surface frame. The smaller singular
        # value covers anisotropic stretches rather than trusting area alone.
        u1, u2 = t[:, 1]-t[:, 0], t[:, 2]-t[:, 0]
        first = u1[valid] / a[valid, None]
        second = (u2[valid] - first*b[valid, None]) / c[valid, None]
        jacobian = np.stack((first, second), axis=2)
        singular = np.linalg.svd(jacobian, compute_uv=False) / max(scale, 1e-9)
        # Degenerate UVs are constant across one direction, not evidence of
        # low sampling density in the other direction.
        useful = np.where(singular[:, 1] > 1e-6, singular[:, 1], singular[:, 0])
        useful = useful[useful > 1e-6]
        if len(useful):
            minimum = min(minimum, float(useful.min()))
            maximum = max(maximum, float(singular[:, 0].max()))
        area += float(triangle_area[valid].sum())
    return minimum, maximum, area


def inventory(root, high_density=2048., pause_file=None):
    if (root/"texture-streaming.json").exists():
        raise ValueError("Inventory the canonical full-resolution pack, not its streamed bootstrap tails")
    entries, categories = {}, Counter()
    resources = PreparationResources(pause_file)
    scenes, buffers = {}, {}
    for scene in sorted(root.rglob("*.gltf")):
        resources.wait()
        raw_scene = scene.read_bytes()
        scenes[scene.relative_to(root).as_posix()] = hashlib.sha256(raw_scene).hexdigest()
        document = json.loads(raw_scene)
        for buffer in document.get("buffers", []):
            path = contained(root, scene.parent/buffer["uri"])
            uri = path.relative_to(root).as_posix()
            if uri not in buffers:
                buffers[uri] = hashlib.sha256(path.read_bytes()).hexdigest()
        scales = mesh_scales(document)
        for mesh_index, mesh in enumerate(document.get("meshes", [])):
            if mesh_index not in scales:
                continue
            for primitive in mesh["primitives"]:
                if primitive.get("mode", 4) != 4:
                    raise ValueError("Only triangle runtime geometry has a density contract")
                material = document["materials"][primitive["material"]]
                surface = material.get("extras", {}).get("surface", {})
                measured = {}
                if surface.get("version") != 2:
                    raise ValueError(f"Scene requires canonical PBR migration: {scene}")
                for channel in surface["channels"]:
                    # Canonical surface URIs are AssetServer pack-relative,
                    # unlike standard glTF image/buffer dependencies.
                    path = contained(root, root / channel["uri"])
                    if not path.is_file():
                        raise ValueError(f"Missing prepared texture: {path}")
                    uri = path.relative_to(root).as_posix()
                    if uri not in entries:
                        entries[uri] = dict(uri=uri, role=channel["role"],
                            source_runtime_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                            color_space=channel["color_space"], width=channel["width"],
                            height=channel["height"], samplers=[], materials=[], scenes=[],
                            minimum_texels_per_meter=None, maximum_texels_per_meter=0.,
                            physical_area_m2=0., uses=0, protected_closeup=False)
                    entry = entries[uri]
                    if (entry["role"], entry["width"], entry["height"], entry["color_space"]) != (
                            channel["role"], channel["width"], channel["height"], channel["color_space"]):
                        raise ValueError(f"Conflicting texture semantics: {uri}")
                    footprint = (entry["width"], entry["height"], channel["layer"])
                    if footprint not in measured:
                        measured[footprint] = density(document, scene, primitive, *footprint,
                                                     scales[mesh_index], resources)
                    low, high, area = measured[footprint]
                    if math.isfinite(low):
                        old = entry["minimum_texels_per_meter"]
                        entry["minimum_texels_per_meter"] = low if old is None else min(old, low)
                    entry["maximum_texels_per_meter"] = max(entry["maximum_texels_per_meter"], high)
                    entry["physical_area_m2"] += area
                    entry["uses"] += 1
                    for key, value in [("materials", material["name"]),
                                       ("scenes", scene.relative_to(root).as_posix()),
                                       ("samplers", channel["sampler"])]:
                        if value not in entry[key]:
                            entry[key].append(value)
                    # The user explicitly prioritizes these close-view assets.
                    relative = scene.relative_to(root).as_posix()
                    entry["protected_closeup"] |= relative.startswith("actors/")
    for entry in entries.values():
        low = entry["minimum_texels_per_meter"]
        skip = (not entry["protected_closeup"] and low is not None and low >= high_density
                and max(entry["width"], entry["height"]) >= 2048)
        entry["scale"] = 1 if skip else 4
        entry["decision"] = "high physical UV density" if skip else "DeepBump 4x"
        categories[(entry["role"], entry["scale"])] += 1
    return dict(version=1, textures=list(entries.values()), source_scenes=scenes, source_buffers=buffers,
                source_game_sha256=hashlib.sha256((root/"game.json").read_bytes()).hexdigest(),
                summary={f"{role}:{scale}x": count for (role, scale), count in sorted(categories.items())})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack", type=Path)
    parser.add_argument("receipt", type=Path)
    parser.add_argument("--high-density", type=float, default=2048.)
    parser.add_argument("--pause-file", type=Path)
    args = parser.parse_args()
    root = args.pack.resolve()
    workspace = Path(__file__).resolve().parents[1]
    if args.receipt.resolve().is_relative_to(workspace):
        raise ValueError("Inventory receipt must remain external")
    result = inventory(root, args.high_density, args.pause_file)
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result["summary"]), flush=True)


if __name__ == "__main__":
    main()
