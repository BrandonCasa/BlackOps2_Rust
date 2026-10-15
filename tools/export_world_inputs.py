"""Keep original world shader inputs through the existing typed zone exporter."""
import json
import sys
from pathlib import Path

root, external, zone = map(Path, sys.argv[1:])
sys.path.insert(0, str(external))
from scripts.cod import backends
from scripts.cod.t6.dumpers import gfxworld, material
from scripts.cod.materials import material_file_stem
from scripts.cod.iw5.dumpers import xmodel

walk = backends.for_fastfile(str(zone)).walk(str(zone))
walk.check()
material.write(walk, str(root / "target/assets"))
asset, = walk.by_type("gfxworld")
world = asset.record
surfaces, _, indices = gfxworld.convert(world)
data = gfxworld.manifest(asset, surfaces, world["draw.vertexCount"])

# Original GPU formats already exist in wgpu. Keep the packed streams intact;
# the generic shell exporter discards tangents and generated-material inputs.
assets = root / "target/assets"
for stream in (0, 1):
    source = world[f"draw.vd{stream}.data"]
    packed = bytes(source.bytes()) if source is not None else b""
    assert len(packed) == world[f"draw.vertexDataSize{stream}"], f"World stream {stream} size"
    name = f"{asset.name}.gfxworld_vd{stream}.bin"
    destination = assets / name
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(packed)
    data[f"vertexDataFile{stream}"] = name
(assets / data["indexFile"]).write_bytes(indices)

# Keep the source surface order: material alone is not a complete draw binding.
for output, source in zip(data["surfaces"], world["dpvs.surfaces"], strict=True):
    output["materialFile"] = "materials/" + material_file_stem(output["material"]) + ".json"
    output["reflectionProbeIndex"] = source["reflectionProbeIndex"]
    output["primaryLightIndex"] = source["primaryLightIndex"]
    output["vertexDataOffset0"] = source["tris.vertexDataOffset0"]
    output["vertexDataOffset1"] = source["tris.vertexDataOffset1"]
    output["bounds"] = [list(source.unpack("<3f", f"bounds[{i}]")) for i in (0, 1)]
data["reflectionProbes"] = [
    {
        "origin": list(probe.unpack("<3f", "origin")),
        "lightingSH": list(probe.unpack("<12f", "lightingSH")),
        "reflectionImage": probe["reflectionImage"].name,
        "mipLodBias": probe["mipLodBias"],
    }
    for probe in world["draw.reflectionProbes"]
]

# Static-model lighting uses placement-selected SH coefficients. The placement's
# lightingSH/lightingHandle fields are runtime scratch, initialized on demand.
coefficients = world["lightGrid.coeffs"]
coefficient_bytes = bytes(coefficients.bytes()) if coefficients is not None else b""
assert len(coefficient_bytes) == world["lightGrid.coeffCount"] * 54
coefficient_name = f"{asset.name}.lightgrid_coeffs.bin"
(assets / coefficient_name).write_bytes(coefficient_bytes)
data["lightGridCoefficientsFile"] = coefficient_name
data["lightGridCoefficientCount"] = world["lightGrid.coeffCount"]
for output, source in zip(data["staticModels"], world["dpvs.smodelDrawInsts"], strict=True):
    output["colorsIndex"] = source["colorsIndex"]
    output["visibility"] = source["visibility"] & 255
    output["reflectionProbeIndex"] = source["reflectionProbeIndex"] & 255

# Retain the model streams needed by the original PACKED vertex programs.
# The ordinary GLB scene remains available to animation/gameplay consumers.
requested_models = {placement["model"] for placement in data["staticModels"]}
data["modelGeometry"] = {}
for model_asset in walk.by_type("xmodel"):
    if model_asset.name not in requested_models:
        continue
    model = model_asset.record
    model_data = {
        "bounds": [list(model.unpack("<3f", key)) for key in ("mins", "maxs")],
        "lods": [],
    }
    vertices, triangles = bytearray(), bytearray()
    handles = list(model["materialHandles"])
    for lod in range(model["numLods"]):
        lod_data = {"dist": model[f"lodInfo[{lod}].dist"], "surfaces": []}
        first = model[f"lodInfo[{lod}].surfIndex"]
        for index, surface in enumerate(xmodel.surfaces(model, lod)):
            vertex_bytes = bytes(surface["verts0"].bytes())
            index_bytes = bytes(surface["triIndices"].bytes())
            assert len(vertex_bytes) == surface["vertCount"] * 32
            assert len(index_bytes) == surface["triCount"] * 6
            source_material = handles[first + index]
            lod_data["surfaces"].append({
                "vertexByteOffset": len(vertices), "firstIndex": len(triangles) // 2,
                "vertexCount": surface["vertCount"], "triCount": surface["triCount"],
                "baseVertIndex": surface["baseVertIndex"], "flags": surface["flags"],
                "material": source_material.name,
                "materialFile": "materials/" + material_file_stem(source_material.name) + ".json",
            })
            vertices.extend(vertex_bytes)
            triangles.extend(index_bytes)
        model_data["lods"].append(lod_data)
    for key, suffix, contents in (("vertexFile", "vertices", vertices), ("indexFile", "indices", triangles)):
        name = f"model_export/{model_asset.name}.{suffix}.bin"
        destination = assets / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(contents)
        model_data[key] = name
    assert model_asset.name not in data["modelGeometry"], "Duplicate original static model"
    data["modelGeometry"][model_asset.name] = model_data
assert set(data["modelGeometry"]) == requested_models, "Missing original static model streams"


def fog(record):
    result = {name: record[name] for name in (
        "baseDist", "halfDist", "baseHeight", "halfHeight", "sunFogPitch",
        "sunFogYaw", "sunFogInner", "sunFogOuter", "fogOpacity", "sunFogOpacity",
    )}
    for name in ("fogColor", "sunFogColor"):
        result[name] = list(record.unpack("<3f", name))
    return result


data["fogTransitionTime"] = world["sunParse.fogTransitionTime"]
data["initWorldFog"] = fog(world["sunParse.initWorldFog[0]"])
data["worldFogVolumes"] = [
    {
        "mins": list(volume.unpack("<3f", "mins")),
        "maxs": list(volume.unpack("<3f", "maxs")),
        "control": volume["control"],
        "controlEx": volume["controlEx"],
        "fogTransitionTime": volume["fogTransitionTime"],
        "volumeWorldFog": fog(volume["volumeWorldFog[0]"]),
    }
    for volume in world["worldFogVolumes"] or ()
]
data["worldFogModifierVolumes"] = [
    {
        **{name: volume[name] for name in (
            "control", "controlEx", "minX", "minY", "minZ", "maxX", "maxY", "maxZ",
            "transitionTime", "depthScale", "heightScale",
        )},
        "colorAdjust": list(volume.unpack("<4f", "colorAdjust")),
    }
    for volume in world["worldFogModifierVolumes"] or ()
]
data["exposureVolumes"] = [
    {name: volume[name] for name in (
        "control", "exposure", "luminanceIncreaseScale", "luminanceDecreaseScale",
        "featherRange", "featherAdjust",
    )}
    for volume in world["exposureVolumes"] or ()
]
for name in ("worldFog", "worldFogModifier", "exposure"):
    data[name + "VolumePlanes"] = [
        list(plane.unpack("<4f", "plane"))
        for plane in world[name + "VolumePlanes"] or ()
    ]

destination = root / "target/assets/world.json"
destination.write_text(json.dumps(data, separators=(",", ":")), encoding="utf-8")
print(f"World shader inputs: {len(surfaces)} surfaces, "
      f"{len(data['reflectionProbes'])} probes, {len(data['worldFogVolumes'])} fog volumes, "
      f"{len(data['worldFogModifierVolumes'])} fog modifiers, "
      f"{len(data['exposureVolumes'])} exposure volumes")
