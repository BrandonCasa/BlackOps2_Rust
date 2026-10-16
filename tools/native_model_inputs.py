"""Preserve original T6 geometry and deformation inputs without glTF conversion."""
import struct
import json


def local_translations(model):
    """00785ea0 adds packed 12-byte XModel.trans XYZ to each non-root's keys."""
    roots, count = model["numRootBones"], model["numBones"]
    source = model["trans"]
    data = bytes(source.bytes()) if source is not None else b""
    assert 0 <= roots <= count and len(data) >= (count - roots) * 12, "Incomplete source local translations"
    return [[0.0, 0.0, 0.0] for _ in range(roots)] + [
        list(struct.unpack_from("<3f", data, index * 12)) for index in range(count - roots)
    ]


def export_model(walk, asset, destination):
    from scripts.cod.iw5.dumpers import xmodel
    from scripts.cod.materials import material_file_stem

    model = asset.record
    names = xmodel.bone_names(walk, model)
    assert len(names) == model["numBones"], f"{asset.name}: incomplete bone names"
    streams = {name: bytearray() for name in ("vertices", "indices", "blends", "tension")}
    result = {
        "bounds": [list(model.unpack("<3f", key)) for key in ("mins", "maxs")],
        "boneNames": names,
        "rootBoneCount": model["numRootBones"],
        "localTranslations": local_translations(model),
        "lodRampType": model["lodRampType"],
        "lods": [],
    }
    handles = list(model["materialHandles"])
    for lod in range(model["numLods"]):
        first = model[f"lodInfo[{lod}].surfIndex"]
        surfaces = xmodel.surfaces(model, lod)
        assert surfaces is not None, f"{asset.name}: missing LOD {lod} surfaces"
        output = {"dist": model[f"lodInfo[{lod}].dist"], "surfaces": []}
        for index, surface in enumerate(surfaces):
            vertex_count, triangle_count = surface["vertCount"], surface["triCount"]
            vertices = bytes(surface["verts0"].bytes())
            indices = bytes(surface["triIndices"].bytes())
            assert len(vertices) == vertex_count * 32, f"{asset.name}: PACKED32 size"
            assert len(indices) == triangle_count * 6, f"{asset.name}: index size"
            assert all(i < vertex_count for (i,) in struct.iter_unpack("<H", indices)), f"{asset.name}: triangle bounds"
            rigid = [
                {key: entry[key] for key in ("boneOffset", "vertCount", "triOffset", "triCount")}
                for entry in surface["vertList"] or ()
            ]
            assert len(rigid) == surface["vertListCount"], f"{asset.name}: rigid list count"
            counts = list(surface["vertInfo.vertCount"])
            assert len(counts) == 4 and all(n >= 0 for n in counts), f"{asset.name}: blend buckets"
            blend = surface["vertInfo.vertsBlend"]
            blends = bytes(blend.bytes()) if blend is not None else b""
            assert len(blends) == sum((2 * n + 1) * count * 2 for n, count in enumerate(counts)), f"{asset.name}: blend size"
            if rigid or blends:
                assert sum(r["vertCount"] for r in rigid) + sum(counts) == vertex_count, f"{asset.name}: skin coverage"
            for entry in rigid:
                assert entry["boneOffset"] % 64 == 0 and entry["boneOffset"] // 64 < len(names), f"{asset.name}: rigid bone"
                assert entry["triOffset"] + entry["triCount"] <= triangle_count, f"{asset.name}: rigid triangle range"
            at = 0
            for influence_count, count in enumerate(counts, start=1):
                for _ in range(count):
                    values = struct.unpack_from(f"<{2 * influence_count - 1}H", blends, at)
                    for offset in (values[0], *values[1::2]):
                        assert offset % 64 == 0 and offset // 64 < len(names), f"{asset.name}: blended bone"
                    at += 2 * len(values)
            tension = surface["vertInfo.tensionData"]
            tension_bytes = bytes(tension.bytes()) if tension is not None else b""
            material = handles[first + index]
            output["surfaces"].append({
                "vertexByteOffset": len(streams["vertices"]),
                "firstIndex": len(streams["indices"]) // 2,
                "vertexCount": vertex_count, "triCount": triangle_count,
                "baseVertIndex": surface["baseVertIndex"], "flags": surface["flags"],
                "material": material.name,
                "materialFile": "materials/" + material_file_stem(material.name) + ".json",
                "materialDrawSurf": material.record["info.drawSurf.packed"],
                "rigidLists": rigid, "blendVertexCounts": counts,
                "blendByteOffset": len(streams["blends"]), "blendByteLength": len(blends),
                "tensionByteOffset": len(streams["tension"]), "tensionByteLength": len(tension_bytes),
            })
            for key, value in (("vertices", vertices), ("indices", indices), ("blends", blends), ("tension", tension_bytes)):
                streams[key].extend(value)
        result["lods"].append(output)
    base = model["baseMat"]
    base_bytes = bytes(base.bytes()) if base is not None else b""
    assert len(base_bytes) == len(names) * 32, f"{asset.name}: bind matrix size"
    streams["bind"] = base_bytes
    for key, contents in streams.items():
        path = f"model_export/{asset.name}.native.{key}.bin"
        output_path = destination / path
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_bytes(contents)
        result[key + "File"] = path
    (destination / f"model_export/{asset.name}.native_model").write_text(
        json.dumps(result, separators=(",", ":")), encoding="utf-8"
    )
    return result
