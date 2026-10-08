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

# Allow the preparation math to use the installed host; explicit user-set
# BLAS/OpenMP limits still take precedence.
os.environ.setdefault("OMP_NUM_THREADS", str(os.cpu_count() or 1))
os.environ.setdefault("OPENBLAS_NUM_THREADS", str(os.cpu_count() or 1))
import numpy as np
from preparation_resources import PreparationResources
from material_textures import inspect_dds


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


def mesh_transforms(document):
    """Actual placement metrics, including nonuniform scale and parent shear."""
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
            gram = transform.T @ transform
            diagonal = np.trace(gram) / 3
            if np.allclose(gram, np.eye(3)*diagonal, rtol=1e-5, atol=max(diagonal, 1e-12)*1e-5):
                # Serialized rotations have tiny orthogonality error. Use an
                # upper singular scale so equivalent placements share work
                # without ever overstating their physical density.
                scale = float(np.linalg.svd(transform, compute_uv=False).max())
                scale = math.ceil(scale*1e6)/1e6
                metric, transform = ("uniform", scale), np.eye(3)*scale
            else:
                metric = tuple(gram.reshape(-1))
            result.setdefault(node["mesh"], {})[metric] = transform
        for child in node.get("children", []):
            visit(child, transform)
    for root in document["scenes"][document.get("scene", 0)].get("nodes", []):
        visit(root, np.eye(3))
    return result


def channel_uv(document, scene, primitive, channel, surface):
    attrs = primitive["attributes"]
    layer = channel["layer"]
    # Mirror the actual material shader, rather than assuming channel number
    # always names a secondary UV set. Tile control uses unscaled primary UV;
    # intrinsic tile colors/normals/response use primary UV times tile count.
    if surface["family"] == "tile_blend":
        uv = accessor(document, scene, attrs["TEXCOORD_0"])
        if channel["role"] != "control":
            uv = uv * surface.get("preparation", {}).get("tiles", 1)
        return uv
    if "effect" in surface:
        uv = accessor(document, scene, attrs["TEXCOORD_0"])
        return uv * surface["effect"]["layers"][layer]["scale"]
    if "layers" not in surface:
        return accessor(document, scene, attrs["TEXCOORD_0"])
    if layer == 0:
        uv = accessor(document, scene, attrs["TEXCOORD_0"])
    elif layer in (1, 2) and "_ORIGINS_LAYER_UV" in attrs:
        uv = accessor(document, scene, attrs["_ORIGINS_LAYER_UV"])
        uv = uv[:, (layer - 1)*2:layer*2]
    elif layer == 3 and "_ORIGINS_LAYER3_UV" in attrs:
        uv = accessor(document, scene, attrs["_ORIGINS_LAYER3_UV"])[:, :2]
    else:
        uv = accessor(document, scene, attrs["TEXCOORD_0"])
    return uv


def density(document, scene, primitive, channel, surface, transform, resources, regions=False):
    uv = channel_uv(document, scene, primitive, channel, surface)
    positions = accessor(document, scene, primitive["attributes"]["POSITION"])
    indices = accessor(document, scene, primitive["indices"]).reshape(-1, 3)
    minimum_axis, maximum, area, texel_area = math.inf, 0., 0., 0.
    degenerate, patches = 0, {}
    # Bound scratch RAM independently of the world mesh size.
    for start in range(0, len(indices), 8192):
        resources.wait()
        corners = indices[start:start + 8192]
        local = positions[corners].astype(np.float64)
        p = local @ transform.T
        t = uv[corners].astype(np.float64) * [channel["width"], channel["height"]]
        e1, e2 = p[:, 1]-p[:, 0], p[:, 2]-p[:, 0]
        normal = np.cross(e1, e2)
        triangle_area = np.linalg.norm(normal, axis=1) * .5
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
        singular = np.linalg.svd(jacobian, compute_uv=False)
        # Degenerate UVs are constant across one direction, not evidence of
        # low sampling density in the other direction.
        useful = np.where(singular[:, 1] > 1e-6, singular[:, 1], singular[:, 0])
        valid_uv = singular[:, 1] > 1e-6
        degenerate += int((~valid_uv).sum())
        texel_area += float((triangle_area[valid] * singular[:, 0] * singular[:, 1]).sum())
        if (useful > 1e-6).any():
            minimum_axis = min(minimum_axis, float(useful[useful > 1e-6].min()))
            maximum = max(maximum, float(singular[:, 0].max()))
        area += float(triangle_area[valid].sum())
        if regions:
            local = local[valid]
            cells = np.floor(local.mean(axis=1) / 8).astype(np.int64)
            unique, inverse = np.unique(cells, axis=0, return_inverse=True)
            for group, cell in enumerate(unique):
                selected = inverse == group
                key = tuple(cell)
                low, high = local[selected].min(axis=(0, 1)), local[selected].max(axis=(0, 1))
                # Native filtering is limited to 16x anisotropy. A nearly
                # collapsed UV triangle cannot benefit from mips finer than
                # the major axis divided by that limit; avoid loading a 16K
                # map because of one numerically tiny minor singular value.
                anisotropy = 16 if channel["sampler"] & 7 != 1 and channel["sampler"] & 24 else 1
                filter_density = np.maximum(singular[:, 1], singular[:, 0]/anisotropy)
                values = filter_density[selected]
                value = float(values[values > 1e-6].min()) if (values > 1e-6).any() else 0.
                previous = patches.get(key)
                if previous:
                    low, high = np.minimum(low, previous[0]), np.maximum(high, previous[1])
                    value = min(value, previous[2])
                patches[key] = (low, high, value)
    # Standard area-equivalent material density. Sliver UV triangles must not
    # turn a mostly well-unwrapped chart into an impossible multi-million-pixel
    # texture. Degenerate UVs still contribute physical area and are reported.
    equivalent = math.sqrt(texel_area / area) if area > 0 and texel_area > 0 else math.inf
    return equivalent, minimum_axis, maximum, area, degenerate, patches


def inventory(root, high_density=2048., pause_file=None, close_density=2048., metric="area", cap_to_device=False):
    if (root/"texture-streaming.json").exists():
        raise ValueError("Inventory the canonical full-resolution pack, not its streamed bootstrap tails")
    entries, categories = {}, Counter()
    resources = PreparationResources(pause_file, saturate=True)
    scenes, buffers, meshes = {}, {}, {}
    if not math.isfinite(high_density) or high_density <= 0 or not math.isfinite(close_density) or close_density <= 0:
        raise ValueError("Density floors must be finite and positive")
    for scene in sorted(root.rglob("*.gltf")):
        resources.wait()
        print(f"Measuring {scene.relative_to(root)}", flush=True)
        raw_scene = scene.read_bytes()
        scenes[scene.relative_to(root).as_posix()] = hashlib.sha256(raw_scene).hexdigest()
        document = json.loads(raw_scene)
        for buffer in document.get("buffers", []):
            path = contained(root, scene.parent/buffer["uri"])
            uri = path.relative_to(root).as_posix()
            if uri not in buffers:
                buffers[uri] = hashlib.sha256(path.read_bytes()).hexdigest()
        transforms = mesh_transforms(document)
        for mesh_index, mesh in enumerate(document.get("meshes", [])):
            if mesh_index not in transforms:
                continue
            for primitive_index, primitive in enumerate(mesh["primitives"]):
                if primitive.get("mode", 4) != 4:
                    raise ValueError("Only triangle runtime geometry has a density contract")
                material = document["materials"][primitive["material"]]
                surface = material.get("extras", {}).get("surface", {})
                measured, local_measurements, primitive_patches = {}, {}, {}
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
                        raw_texture = path.read_bytes()
                        layout = inspect_dds(raw_texture)
                        constant = layout["width"] == layout["height"] == 1
                        if layout["dxgi"] in (28, 29):
                            offset = 148 if raw_texture[84:88] == b"DX10" else 128
                            pixels = np.frombuffer(raw_texture, np.uint8, layout["width"]*layout["height"]*4,
                                                   offset).reshape(-1, 4)
                            constant = bool(np.all(pixels == pixels[0]))
                        entries[uri] = dict(uri=uri, role=channel["role"],
                            source_runtime_sha256=hashlib.sha256(raw_texture).hexdigest(),
                            color_space=channel["color_space"], width=channel["width"],
                            height=channel["height"], samplers=[], materials=[], scenes=[],
                            minimum_texels_per_meter=None, maximum_texels_per_meter=0.,
                            minimum_axis_texels_per_meter=None, degenerate_uv_triangles=0,
                            physical_area_m2=0., uses=0, protected_closeup=False,
                            constant_field=constant)
                    entry = entries[uri]
                    if (entry["role"], entry["width"], entry["height"], entry["color_space"]) != (
                            channel["role"], channel["width"], channel["height"], channel["color_space"]):
                        raise ValueError(f"Conflicting texture semantics: {uri}")
                    uv_kind = ("tile-control" if channel["role"] == "control" else "tile") if surface["family"] == "tile_blend" else \
                              "effect" if "effect" in surface else "layers" if "layers" in surface else "primary"
                    anisotropy = 16 if channel["sampler"] & 7 != 1 and channel["sampler"] & 24 else 1
                    footprint = (entry["width"], entry["height"], channel["layer"] if uv_kind in ("effect", "layers") else 0, uv_kind, anisotropy)
                    if footprint not in measured:
                        measurements = [density(document, scene, primitive, channel, surface, transform, resources)
                                        for transform in transforms[mesh_index].values()]
                        measured[footprint] = (min(v[0] for v in measurements), min(v[1] for v in measurements),
                                              max(v[2] for v in measurements), sum(v[3] for v in measurements),
                                              max(v[4] for v in measurements))
                        local_measurements[footprint] = density(document, scene, primitive, channel, surface,
                                                               np.eye(3), resources, regions=True)[5]
                    low, axis, high, area, degenerate = measured[footprint]
                    if metric == "minimum-axis":
                        low = axis
                    if math.isfinite(low):
                        old = entry["minimum_texels_per_meter"]
                        entry["minimum_texels_per_meter"] = low if old is None else min(old, low)
                    entry["maximum_texels_per_meter"] = max(entry["maximum_texels_per_meter"], high)
                    if math.isfinite(axis):
                        old = entry["minimum_axis_texels_per_meter"]
                        entry["minimum_axis_texels_per_meter"] = axis if old is None else min(old, axis)
                    entry["degenerate_uv_triangles"] += degenerate
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
                    for cell, (lo, hi, value) in local_measurements[footprint].items():
                        patch = primitive_patches.setdefault(cell, dict(center=((lo+hi)*.5).tolist(),
                                                                       half_extents=((hi-lo)*.5).tolist(), textures={}))
                        patch["textures"][uri] = value
                key = f"{scene.relative_to(root).as_posix()}#Mesh{mesh_index}/Primitive{primitive_index}"
                meshes[key] = list(primitive_patches.values())
    for entry in entries.values():
        low = entry["minimum_texels_per_meter"]
        target = close_density if entry["protected_closeup"] else high_density
        applicable = not entry["constant_field"] and low is not None
        required_scale = 1 if not applicable or low >= target else 2 ** math.ceil(math.log2(target / low))
        limit_scale = 2 ** math.floor(math.log2(16384 / max(entry["width"], entry["height"])))
        scale = min(required_scale, limit_scale) if cap_to_device else required_scale
        entry.update(scale=scale, density_floor=target, density_metric=metric,
                     density_applicable=applicable,
                     output_texels_per_meter=None if low is None else low*scale,
                     required_scale=required_scale, cap_to_device=cap_to_device,
                     achievable=required_scale <= limit_scale,
                     density_exception=applicable and low*scale < target and cap_to_device,
                     decision="constant material field" if entry["constant_field"] else
                              "unmapped UVs" if low is None else
                              "density satisfied" if scale == 1 else "DeepBump 4x stages then power-of-two reduction")
        categories[(entry["role"], entry["scale"])] += 1
    return dict(version=2, textures=list(entries.values()), meshes=meshes,
                density_metric=metric, world_density_floor=high_density, close_density_floor=close_density,
                cap_to_device=cap_to_device,
                source_scenes=scenes, source_buffers=buffers,
                source_game_sha256=hashlib.sha256((root/"game.json").read_bytes()).hexdigest(),
                summary={f"{role}:{scale}x": count for (role, scale), count in sorted(categories.items())})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack", type=Path)
    parser.add_argument("receipt", type=Path)
    parser.add_argument("--high-density", "--minimum-density", type=float, default=2048.)
    parser.add_argument("--close-density", type=float, default=2048.)
    parser.add_argument("--density-metric", choices=("area", "minimum-axis"), default="area")
    parser.add_argument("--cap-to-device", action="store_true",
                        help="Cap at 16384 pixels and explicitly retain every density exception")
    parser.add_argument("--pause-file", type=Path)
    args = parser.parse_args()
    root = args.pack.resolve()
    workspace = Path(__file__).resolve().parents[1]
    if args.receipt.resolve().is_relative_to(workspace) or args.receipt.resolve().is_relative_to(root):
        raise ValueError("Inventory receipt must remain external")
    result = inventory(root, args.high_density, args.pause_file, args.close_density, args.density_metric, args.cap_to_device)
    args.receipt.parent.mkdir(parents=True, exist_ok=True)
    args.receipt.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result["summary"]), flush=True)


if __name__ == "__main__":
    main()
