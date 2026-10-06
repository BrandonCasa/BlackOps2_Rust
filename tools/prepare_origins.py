# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy>=2,<3", "pillow>=12,<13"]
# ///
"""Read retained external map exchanges and emit an external standard glTF pack.

Reads geometry/UV/packed corner streams, DDS images, material metadata and map
entities. Does not read captures or shaders and never modifies source inputs.
Run: uv run tools/prepare_origins.py --config config/local_assets.toml
"""
import argparse
import base64
import hashlib
import json
import math
from pathlib import Path
import re
import struct
import subprocess
import tomllib

import numpy as np
from PIL import Image

SCALE = 0.0254
WORKSPACE = Path(__file__).resolve().parents[1]


def external(path):
    path = Path(path).resolve()
    if path == WORKSPACE or WORKSPACE in path.parents:
        raise ValueError("licensed inputs and generated assets must stay outside the repository")
    return path


def dependency(root, record):
    path = (root / record["file"]).resolve()
    if root not in path.parents:
        raise ValueError("dependency escapes input root")
    data = path.read_bytes()
    if len(data) > 128 * 1024 * 1024:
        raise ValueError("input exceeds preparation limit")
    if record.get("sha256") and hashlib.sha256(data).hexdigest() != record["sha256"]:
        raise ValueError("input checksum mismatch")
    return data


def coordinates(vectors):
    return np.asarray(vectors, dtype=np.float32)[..., [0, 2, 1]] * [SCALE, SCALE, -SCALE]


def geometry(data):
    if data[:8] != b"BO2DGS\0\0" or struct.unpack_from("<II", data, 8) != (1, 0):
        raise ValueError("unsupported geometry exchange")
    positions, indices, surfaces, length = struct.unpack_from("<4Q", data, 16)
    if length != len(data) or length != 64 + positions * 12 + indices * 4 + surfaces * 48:
        raise ValueError("geometry length mismatch")
    xyz = np.frombuffer(data, "<f4", positions * 3, 64).reshape(-1, 3)
    refs = np.frombuffer(data, "<u4", indices, 64 + positions * 12)
    if indices % 3 or refs.max(initial=0) >= positions or not np.isfinite(xyz).all():
        raise ValueError("invalid geometry references or positions")
    records = [struct.unpack_from("<QQII6f", data, 64 + positions * 12 + indices * 4 + i * 48)
               for i in range(surfaces)]
    cursor = 0
    for first, count, *_ in records:
        if first != cursor or count % 3:
            raise ValueError("surface partition mismatch")
        cursor += count
    if cursor != indices:
        raise ValueError("incomplete surface partition")
    return coordinates(xyz[refs]), records


def entities(path):
    text = path.read_text(encoding="utf-8-sig")
    return [dict(re.findall(r'"([^"\n]+)"\s+"([^"\n]*)"', block))
            for block in re.findall(r"\{([^{}]*)\}", text)]


def gameplay_definition(records):
    starts = [e for e in records if e.get("targetname") == "initial_spawn_points"]
    if not starts:
        raise ValueError("map entities have no initial spawn points")
    source = starts[0]
    spawn = coordinates([float(x) for x in source["origin"].split()]).tolist()
    angles = [float(x) for x in source.get("angles", "0 0 0").split()]
    nodes = [coordinates([float(x) for x in e["origin"].split()]).tolist()
             for e in records if e.get("classname") == "node_pathnode" and "origin" in e]
    local = [p for p in nodes if np.linalg.norm(np.asarray(p) - spawn) < 45]
    spawners = [e for e in records if e.get("targetname") == "zone_start_spawners"
                and e.get("script_noteworthy") in ("spawn_location", "riser_location")]
    points = []
    for entry in spawners:
        point = coordinates([float(x) for x in entry["origin"].split()])
        # Use the nearest walkable map path node instead of implementing climb animations.
        if local:
            point = np.asarray(min(local, key=lambda p: np.linalg.norm(np.asarray(p) - point)))
        if not any(np.linalg.norm(np.asarray(p) - point) < 0.8 for p in points):
            points.append(point.tolist())
    if not points:
        raise ValueError("map entities have no starting-zone zombie entry points")
    return {"version": 1, "scene": "map.gltf#Scene0", "collision_scene": "map.gltf#Scene1",
            "spawn": spawn, "yaw": math.radians(90 - angles[1]), "spawn_points": points,
            "waypoints": local, "survey_views": [p for p in nodes[::max(1, len(nodes)//8)]][:8],
            "source_surfaces": 0, "render_meshes": 0, "collision_meshes": 0,
            "static_model_instances": 0}


class Gltf:
    def __init__(self):
        self.binary = bytearray()
        self.collidable = {}
        self.document = {"asset": {"version": "2.0", "generator": "Origins asset preparation"},
                         "scene": 0, "scenes": [{"nodes": []}, {"nodes": []}], "nodes": [],
                         "meshes": [], "materials": [], "textures": [], "images": [],
                         "samplers": [{"magFilter": 9729, "minFilter": 9987}],
                         "accessors": [], "bufferViews": [], "buffers": []}

    def accessor(self, values, kind, indices=False):
        values = np.ascontiguousarray(values, dtype="<u4" if indices else "<f4")
        while len(self.binary) % 4:
            self.binary.append(0)
        view = len(self.document["bufferViews"])
        self.document["bufferViews"].append({"buffer": 0, "byteOffset": len(self.binary),
                                            "byteLength": values.nbytes, "target": 34963 if indices else 34962})
        self.binary.extend(values.tobytes())
        index = len(self.document["accessors"])
        self.document["accessors"].append({"bufferView": view, "componentType": 5125 if indices else 5126,
                                           "count": len(values), "type": kind,
                                           "min": values.min(axis=0).tolist(),
                                           "max": values.max(axis=0).tolist()})
        return index

    def texture(self, uri):
        image = len(self.document["images"])
        self.document["images"].append({"uri": uri})
        self.document["textures"].append({"source": image, "sampler": 0})
        return {"index": len(self.document["textures"]) - 1}

    def mesh(self, arrays, material, name, collider=True, lightmap=None, visible=True):
        attributes = {key: self.accessor(data, kind) for key, data, kind in arrays}
        mesh = len(self.document["meshes"])
        points = next(data for key, data, _ in arrays if key == "POSITION").reshape(-1, 3, 3)
        self.collidable[mesh] = bool((np.linalg.norm(np.cross(points[:, 1] - points[:, 0], points[:, 2] - points[:, 0]), axis=1) > 1e-8).any())
        indexes = self.accessor(np.arange(len(points) * 3).reshape(-1, 1), "SCALAR", indices=True)
        self.document["meshes"].append({"name": name, "primitives": [
            {"attributes": attributes, "indices": indexes, "mode": 4, "material": material}]})
        if visible:
            self.node(mesh, name, 0, extras={"lightmap": lightmap} if lightmap else {})
        if collider and self.collidable[mesh]:
            self.node(mesh, "collision_" + name, 1)
        return mesh

    def node(self, mesh, name, scene, **properties):
        node = len(self.document["nodes"])
        self.document["nodes"].append({"mesh": mesh, "name": name, **properties})
        self.document["scenes"][scene]["nodes"].append(node)

    def save(self, output):
        self.document["buffers"] = [{"uri": "map.bin", "byteLength": len(self.binary)}]
        (output / "map.bin").write_bytes(self.binary)
        (output / "map.gltf").write_text(json.dumps(self.document, separators=(",", ":")), encoding="utf-8")


def model_accessor(document, index):
    accessor = document["accessors"][index]
    view = document["bufferViews"][accessor["bufferView"]]
    uri = document["buffers"][view["buffer"]]["uri"]
    if not uri.startswith("data:application/octet-stream;base64,"):
        raise ValueError("expected embedded OAT model buffer")
    buffers = document.setdefault("_decoded", {})
    if view["buffer"] not in buffers:
        buffers[view["buffer"]] = base64.b64decode(uri.split(",", 1)[1], validate=True)
    raw = buffers[view["buffer"]]
    dtype = {5126: "<f4", 5123: "<u2", 5125: "<u4"}[accessor["componentType"]]
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[accessor["type"]]
    offset = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    stride = view.get("byteStride", np.dtype(dtype).itemsize * width)
    return np.ndarray((accessor["count"], width), dtype, raw, offset,
                      (stride, np.dtype(dtype).itemsize)).copy()


def static_models(gltf, instances_path, model_root, output):
    instances = json.loads(external(instances_path).read_text())
    providers = {}
    textures = {}
    missing = set()
    basis = np.array([[1, 0, 0], [0, 0, 1], [0, -1, 0]], dtype=float)
    for item in instances:
        name = item["model"]
        if name not in providers:
            path = external(model_root / "model_export" / (name + "_lod0.gltf"))
            source = json.loads(path.read_text())
            materials = []
            for mat in source.get("materials", []):
                material = {"doubleSided": True, "pbrMetallicRoughness": {
                    "metallicFactor": 0, "roughnessFactor": 0.85}}
                for binding, normal in (("baseColorTexture", False), ("normalTexture", True)):
                    original = mat.get(binding) if normal else mat.get("pbrMetallicRoughness", {}).get(binding)
                    if original is None:
                        continue
                    image = source["images"][source["textures"][original["index"]]["source"]]["uri"]
                    # OAT uses URI escapes such as %26 in image names.
                    from urllib.parse import unquote
                    image_path = external(model_root / "images" / unquote(image).removeprefix("../images/"))
                    if image_path.name == "$identitynormalmap.dds":
                        continue  # StandardMaterial's default is the identity normal.
                    if not image_path.is_file():
                        missing.add(image_path.name)
                        continue
                    key = (image_path, normal)
                    if key not in textures:
                        pixels = np.array(Image.open(image_path).convert("RGBA"))
                        if normal:
                            xy = pixels[..., :2].astype(np.float32) / 127.5 - 1
                            pixels[..., 1] = 255 - pixels[..., 1]
                            pixels[..., 2] = (np.sqrt(np.maximum(0, 1 - (xy * xy).sum(axis=2))) * 127.5 + 127.5).astype(np.uint8)
                            pixels[..., 3] = 255
                        filename = f"model-{len(textures)}.png"
                        Image.fromarray(pixels).save(output / "textures" / filename)
                        textures[key] = gltf.texture("textures/" + filename)
                    if normal:
                        material[binding] = textures[key]
                    else:
                        material["pbrMetallicRoughness"][binding] = textures[key]
                materials.append(len(gltf.document["materials"]))
                gltf.document["materials"].append(material)
            provider = []
            for mesh in source["meshes"]:
                for primitive in mesh["primitives"]:
                    indexes = model_accessor(source, primitive["indices"]).flatten()
                    arrays = []
                    for attribute, kind in (("POSITION", "VEC3"), ("NORMAL", "VEC3"), ("TEXCOORD_0", "VEC2")):
                        values = model_accessor(source, primitive["attributes"][attribute])[indexes]
                        if attribute == "POSITION":
                            values *= SCALE  # OAT already converted to Y-up, in source units.
                        arrays.append((attribute, values, kind))
                    provider.append(gltf.mesh(arrays, materials[primitive["material"]],
                                              "static_provider", collider=False, visible=False))
            providers[name] = provider
        matrix = np.eye(4)
        matrix[:3, :3] = basis @ np.asarray(item["axis"]).reshape(3, 3).T @ basis.T * item["scale"]
        matrix[:3, 3] = coordinates(item["origin"])
        if not np.isfinite(matrix).all() or item["scale"] <= 0:
            raise ValueError("invalid static model placement")
        for mesh in providers[name]:
            gltf.node(mesh, "static_instance", 0, matrix=matrix.T.flatten().tolist())
            # Foliage and hanging cloth are visible decorations, not solid walls.
            if gltf.collidable[mesh] and not any(token in name.lower() for token in ("grass", "foliage", "vine", "cloth", "flag")):
                gltf.node(mesh, "collision_static", 1, matrix=matrix.T.flatten().tolist())
    if missing:
        raise ValueError(f"{len(missing)} required model textures unavailable: {sorted(missing)[:5]}")
    return len(instances), len(providers), len(textures)


def prepare(scene_path, entity_path, output, instances_path, model_root):
    root = external(scene_path).parent
    output = external(output)
    manifest = json.loads(scene_path.read_text())
    positions, surfaces = geometry(dependency(root, {"file": manifest["geometry"],
                                                     "sha256": manifest["geometry_sha256"]}))
    uv_data = dependency(root, {"file": manifest["corner_uv"], "sha256": manifest["corner_uv_sha256"]})
    uv = np.frombuffer(uv_data, "<f4").reshape(-1, 2)
    packed = dependency(root, manifest["corner_vertices"])
    if len(packed) != len(positions) * 36 or len(uv) != len(positions):
        raise ValueError("corner stream mismatch")
    layout = np.dtype({"names": ["p", "color", "uv", "normal", "tangent", "lm"],
                       "formats": [("<f4", 4), ("u1", 4), ("<f2", 2), "<u4", "<u4", ("<u2", 2)],
                       "offsets": [0, 16, 20, 24, 28, 32], "itemsize": 36})
    corners = np.frombuffer(packed, layout)
    normals = np.stack([(corners["normal"] >> shift & 1023) / 1023 for shift in (0, 10, 20)], axis=1)
    normals = np.where(normals < 0.5, normals * 2, normals * 2 - 2)
    normals = normals[:, [0, 2, 1]] * [1, 1, -1]
    normals /= np.maximum(np.linalg.norm(normals, axis=1, keepdims=True), 1e-8)
    color = corners["color"].astype(np.float32) / 255
    color[:, 3] = 1
    lm_uv = corners["lm"].astype(np.float32) / 65535
    definition = gameplay_definition(entities(external(entity_path)))
    output.mkdir(parents=True, exist_ok=True)
    texture_dir = output / "textures"
    texture_dir.mkdir(exist_ok=True)
    gltf = Gltf()
    cached = {}

    def texture(binding, normal=False, roughness=False):
        image_index = binding["image"]
        key = (image_index, normal, roughness)
        if key not in cached:
            record = manifest["images"][image_index]
            import io
            image = Image.open(io.BytesIO(dependency(root, record))).convert("RGBA")
            pixels = np.asarray(image).copy()
            if normal:
                xy = pixels[..., :2].astype(np.float32) / 127.5 - 1
                pixels[..., 1] = 255 - pixels[..., 1]
                pixels[..., 2] = (np.sqrt(np.maximum(0, 1 - (xy * xy).sum(axis=2))) * 127.5 + 127.5).astype(np.uint8)
                pixels[..., 3] = 255
            if roughness:
                pixels[..., 1] = np.clip(255 - pixels[..., 3], 65, 230)
                pixels[..., 2] = 0  # Most map surfaces are dielectric.
                pixels[..., 3] = 255
            filename = f"image-{image_index}{'-normal' if normal else '-roughness' if roughness else ''}.png"
            Image.fromarray(pixels).save(texture_dir / filename)
            cached[key] = gltf.texture("textures/" + filename)
        return cached[key]

    families = []
    for index, source in enumerate(manifest["materials"]):
        family = source["source_technique_set"].lower()
        families.append(family)
        bindings = source["bindings"]
        pbr = {"metallicFactor": 0, "roughnessFactor": 0.8}
        colors = [b for b in bindings if b["semantic"] == 2]
        normals_bindings = [b for b in bindings if b["semantic"] == 5]
        gloss = [b for b in bindings if b["semantic"] == 8]
        material = {"name": f"material_{index}", "doubleSided": True, "pbrMetallicRoughness": pbr}
        if colors:
            pbr["baseColorTexture"] = texture(colors[0])
        if normals_bindings:
            material["normalTexture"] = texture(normals_bindings[0], normal=True)
        if gloss:
            pbr["metallicRoughnessTexture"] = texture(gloss[0], roughness=True)
            pbr["roughnessFactor"] = 1
        if "unlit" in family:
            material["extensions"] = {"KHR_materials_unlit": {}}
            gltf.document.setdefault("extensionsUsed", ["KHR_materials_unlit"])
        if any(s in family for s in ("blend", "water", "effect", "burn")):
            material["alphaMode"] = "BLEND"
        elif "alpha" in family:
            material.update(alphaMode="MASK", alphaCutoff=0.5)
        if "water" in family:
            pbr.update(baseColorFactor=[0.12, 0.2, 0.23, 0.65], roughnessFactor=0.15)
        if family == "wpc_default":
            material = {"doubleSided": True, "pbrMetallicRoughness": {"baseColorFactor": [0.3,0.3,0.3,1], "metallicFactor":0, "roughnessFactor":1}}
        gltf.document["materials"].append(material)
    groups = {}
    shadow_only = 0
    for record, source in zip(surfaces, manifest["surfaces"], strict=True):
        first, count = record[:2]
        material = source["material"]
        family = families[material]
        if "shadowcaster" in family:
            shadow_only += 1
        center = positions[first:first+count].mean(axis=0)
        cell = tuple(np.floor(center / 16).astype(int))
        key = (material, source.get("lightmap"), cell)
        groups.setdefault(key, []).extend(range(first, first+count))
    for (material, lightmap, cell), indexes in groups.items():
        selection = np.asarray(indexes)
        family = families[material]
        if family == "wpc_default":
            color[selection] = 1
        lightmap_uri = None
        if lightmap is not None and lightmap < len(manifest["lightmaps"]):
            slot = manifest["lightmaps"][lightmap]
            image = slot.get("primary") or slot.get("secondary")
            if image is not None:
                texture({"image": image})
                lightmap_uri = f"textures/image-{image}.png"
        gltf.mesh([("POSITION", positions[selection], "VEC3"), ("NORMAL", normals[selection], "VEC3"),
                   ("TEXCOORD_0", uv[selection], "VEC2"), ("TEXCOORD_1", lm_uv[selection], "VEC2"),
                   ("COLOR_0", color[selection], "VEC4")], material,
                  f"world_{material}_{cell}", collider="water" not in family and "unlit" not in family,
                  lightmap=lightmap_uri, visible="shadowcaster" not in family)
    instance_count, model_count, model_textures = static_models(gltf, instances_path, external(model_root), output)
    gltf.save(output)
    definition.update(source_surfaces=len(surfaces), render_meshes=len(gltf.document["scenes"][0]["nodes"]),
                      collision_meshes=len(gltf.document["scenes"][1]["nodes"]), static_model_instances=instance_count)
    (output / "game.json").write_text(json.dumps(definition, indent=2), encoding="utf-8")
    receipt = {"source_surfaces": len(surfaces), "triangles": len(positions)//3,
               "render_meshes": definition["render_meshes"], "textures": len(cached),
               "shadow_helper_surfaces": shadow_only,
               "approximations": ["Standard PBR instead of native shaders", "first color/normal layer",
                                  "direct source lightmap", "nearest path-node zombie admission"],
               "static_instances": instance_count, "model_providers": model_count, "model_textures": model_textures}
    (output / "preparation.json").write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    print(json.dumps(receipt))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=WORKSPACE / "config/local_assets.toml")
    args = parser.parse_args()
    config = tomllib.loads(args.config.read_text(encoding="utf-8-sig"))
    root = Path(config["assets"]["inspection_output"])
    settings = config.get("preparation", {})
    entities_path = Path(settings.get("entities", root / "game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents"))
    model_root = Path(settings.get("models", root / "game-prepare/models"))
    instances_path = Path(settings.get("instances", root / "game-prepare/instances/static-instances.json"))
    zone = Path(config["assets"]["installation_root"]) / "zone/all/zm_tomb.ff"
    unlinker = external(Path(settings.get("unlinker", root / "tooling/OAT/v0.33.0/Unlinker.exe")))
    if not entities_path.exists() or not model_root.exists():
        external(model_root).mkdir(parents=True, exist_ok=True)
        subprocess.run([str(unlinker), "--no-color", "--include-assets", "xmodel,image,mapents",
                        "--model-format", "GLTF", "-o", str(model_root), str(zone)], check=True,
                       stdout=subprocess.DEVNULL)
        entities_path = model_root / "maps/mp/zm_tomb.d3dbsp.ents"
    if not instances_path.exists() or not (model_root / ".streamed-images-ready").exists():
        from export_instances import export
        source = external(Path(settings.get("oat_source", root / "tooling/OAT/surface-7d027e8/source")))
        instances_path = export(source, zone, external(instances_path.parent), external(model_root / "images"))
        (model_root / ".streamed-images-ready").touch()
    prepare(Path(settings.get("scene", root / "full-scene-assets/texture-preview.json")),
            entities_path, Path(config["game"]["asset_pack"]), instances_path, model_root)


if __name__ == "__main__":
    main()
