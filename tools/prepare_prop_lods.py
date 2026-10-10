import argparse
import base64
from collections import defaultdict
import copy
import json
from pathlib import Path
import struct


FORMATS = {5120: "b", 5121: "B", 5122: "h", 5123: "H", 5125: "I", 5126: "f"}
WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}
ALLOW = ("grass", "foliage", "sandbag", "barrel", "crate", "debris", "rubble", "rock", "tire", "tool")
EXCLUDE = ("fence", "wall", "door", "barrier", "generator", "perk", "light", "lamp", "blockade",
           "collision", "trigger", "building", "terrain", "interactive")


def values(document, index, root, cache):
    accessor = document["accessors"][index]
    view = document["bufferViews"][accessor["bufferView"]]
    buffer = view["buffer"]
    if buffer not in cache:
        uri = document["buffers"][buffer]["uri"]
        cache[buffer] = base64.b64decode(uri.split(",", 1)[1]) if uri.startswith("data:") else (root / uri).read_bytes()
    code = FORMATS[accessor["componentType"]]
    fmt = "<" + code * WIDTHS[accessor["type"]]
    size = struct.calcsize(fmt)
    stride = view.get("byteStride", size)
    start = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    result = [struct.unpack_from(fmt, cache[buffer], start + i * stride) for i in range(accessor["count"])]
    if accessor.get("normalized") and code != "f":
        maximum = {"b": 127, "B": 255, "h": 32767, "H": 65535, "I": 4294967295}[code]
        result = [tuple(max(-1.0, v / maximum) for v in row) for row in result]
    return result


def append_accessor(document, binary, rows, kind, indices=False):
    binary.extend(b"\0" * (-len(binary) % 4))
    offset = len(binary)
    fmt = "<" + ("I" if indices else "f") * WIDTHS[kind]
    for row in rows:
        binary.extend(struct.pack(fmt, *row))
    view = len(document["bufferViews"])
    document["bufferViews"].append(dict(buffer=0, byteOffset=offset, byteLength=len(binary) - offset))
    index = len(document["accessors"])
    accessor = dict(bufferView=view, componentType=5125 if indices else 5126, count=len(rows), type=kind)
    if kind == "VEC3":
        accessor.update(min=[min(row[i] for row in rows) for i in range(3)],
                        max=[max(row[i] for row in rows) for i in range(3)])
    document["accessors"].append(accessor)
    return index


def canonicalize(document):
    counts = []
    for table, owner, field in (("materials", "primitives", "material"), ("meshes", "nodes", "mesh")):
        unique, mapping, lookup = [], {}, {}
        for index, value in enumerate(document[table]):
            key = json.dumps(value, sort_keys=True, separators=(",", ":"))
            if key not in lookup:
                lookup[key] = len(unique)
                unique.append(value)
            mapping[index] = lookup[key]
        entries = document["nodes"] if owner == "nodes" else [p for m in document["meshes"] for p in m["primitives"]]
        for entry in entries:
            if field in entry:
                entry[field] = mapping[entry[field]]
        counts.append(len(document[table]) - len(unique))
        document[table] = unique
    return counts


def prepare(pack, models):
    pack, models = Path(pack).resolve(), Path(models).resolve()
    path = pack / "map.gltf"
    document = json.loads(path.read_text())
    if any("prop_lod" in n.get("extras", {}) for n in document["nodes"]):
        raise ValueError("Prepare prop LODs from a pack without existing LODs")
    binary = bytearray((pack / document["buffers"][0]["uri"]).read_bytes())
    cache = {0: bytes(binary)}
    groups = defaultdict(list)
    for index in document["scenes"][0]["nodes"]:
        node = document["nodes"][index]
        extras = node.get("extras", {})
        name = extras.get("source_model", "")
        if extras.get("static_instance") and not extras.get("fixture_emitter") and "mesh" in node:
            groups[name].append(index)
    materials = {m["name"]: i for i, m in enumerate(document["materials"])}
    providers = instances = 0
    for name, nodes in sorted(groups.items()):
        lower = name.lower()
        if not any(token in lower for token in ALLOW) or any(token in lower for token in EXCLUDE):
            continue
        original_meshes = {document["nodes"][i]["mesh"] for i in nodes}
        original_primitives = [p for i in original_meshes for p in document["meshes"][i]["primitives"]]
        allowed_materials = {p["material"] for p in original_primitives}
        if any(any(c.get("layer", 0) > 0 for c in document["materials"][i].get("extras", {}).get("surface", {}).get("channels", []))
               or document["materials"][i].get("extras", {}).get("surface", {}).get("effect") for i in allowed_materials):
            continue
        color_samples = defaultdict(list)
        for primitive in original_primitives:
            attributes = primitive["attributes"]
            if "COLOR_0" in attributes:
                positions = values(document, attributes["POSITION"], pack, cache)
                colors = values(document, attributes["COLOR_0"], pack, cache)
                color_samples[primitive["material"]].extend(zip(positions, colors))
        alternates = []
        for level in (1, 2):
            source_path = models / (name + f"_lod{level}.gltf")
            if not source_path.exists():
                break
            source = json.loads(source_path.read_text())
            primitives = [p for m in source["meshes"] for p in m["primitives"]]
            source_materials = [materials.get(source["materials"][p["material"]]["name"]) for p in primitives]
            if any(material not in allowed_materials for material in source_materials):
                break
            source_cache, output = {}, []
            for primitive, material in zip(primitives, source_materials):
                if primitive.get("mode", 4) != 4 or "targets" in primitive:
                    raise ValueError("Prop LOD requires ordinary authored triangles")
                attributes = {}
                positions = None
                for attribute, index in primitive["attributes"].items():
                    if attribute.startswith(("JOINTS_", "WEIGHTS_")):
                        continue
                    rows = values(source, index, source_path.parent, source_cache)
                    if attribute == "POSITION":
                        rows = [tuple(v * 0.0254 for v in row) for row in rows]
                        positions = rows
                    attributes[attribute] = append_accessor(document, binary, rows, source["accessors"][index]["type"])
                samples = color_samples[material]
                if "COLOR_0" not in attributes and samples and positions:
                    unique_colors = {tuple(color) for _, color in samples}
                    if len(unique_colors) == 1:
                        colors = [next(iter(unique_colors))] * len(positions)
                    else:
                        colors = [min(samples, key=lambda sample: sum((a - b) ** 2 for a, b in zip(point, sample[0])))[1] for point in positions]
                    attributes["COLOR_0"] = append_accessor(document, binary, colors, "VEC4")
                indices = values(source, primitive["indices"], source_path.parent, source_cache)
                index = append_accessor(document, binary, indices, "SCALAR", indices=True)
                mesh = len(document["meshes"])
                document["meshes"].append(dict(name=f"prop_lod{level}", primitives=[dict(attributes=attributes, indices=index, material=material, mode=4)]))
                output.append(mesh)
            alternates.append(output)
        if not alternates:
            continue
        placements = {}
        levels = len(alternates) + 1
        for index in nodes:
            node = document["nodes"][index]
            node["extras"]["prop_lod"] = dict(level=0, levels=levels)
            transform = {key: node[key] for key in ("matrix", "translation", "rotation", "scale") if key in node}
            placements[json.dumps(transform, sort_keys=True)] = transform
        for transform in placements.values():
            for level, meshes in enumerate(alternates, 1):
                for mesh in meshes:
                    index = len(document["nodes"])
                    document["nodes"].append(dict(mesh=mesh, name="static_instance", **copy.deepcopy(transform),
                        extras=dict(static_instance=True, source_model=name, fixture_emitter=False, prop_lod=dict(level=level, levels=levels))))
                    document["scenes"][0]["nodes"].append(index)
            instances += 1
        providers += 1
    deduplicated = canonicalize(document)
    document["buffers"][0]["byteLength"] = len(binary)
    (pack / document["buffers"][0]["uri"]).write_bytes(binary)
    path.write_text(json.dumps(document, separators=(",", ":")) + "\n")
    game_path = pack / "game.json"
    game = json.loads(game_path.read_text())
    stack = list(document["scenes"][0]["nodes"])
    count = 0
    while stack:
        node = document["nodes"][stack.pop()]
        count += "mesh" in node
        stack.extend(node.get("children", []))
    game["render_meshes"] = count
    game_path.write_text(json.dumps(game, indent=2) + "\n")
    return dict(lod_models=providers, lod_placements=instances, deduplicated_materials=deduplicated[0], deduplicated_meshes=deduplicated[1])


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("pack", type=Path)
    parser.add_argument("models", type=Path)
    arguments = parser.parse_args()
    print(prepare(arguments.pack, arguments.models))
