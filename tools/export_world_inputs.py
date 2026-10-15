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
for surface_index, (output, source) in enumerate(zip(data["surfaces"], world["dpvs.surfaces"], strict=True)):
    output["materialFile"] = "materials/" + material_file_stem(output["material"]) + ".json"
    output["reflectionProbeIndex"] = source["reflectionProbeIndex"]
    output["primaryLightIndex"] = source["primaryLightIndex"] & 255
    output["sourceSurfaceIndex"] = surface_index
    output["materialDrawSurf"] = source["material"].record["info.drawSurf.packed"]
    output["materialSortKey"] = source["material"].record["info.sortKey"]
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
data["outdoorImage"] = world["outdoorImage"].name if world["outdoorImage"] is not None else None
data["outdoorLookupMatrix"] = [
    list(world.unpack("<4f", f"outdoorLookupMatrix[{row}]")) for row in range(4)
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
    output["cullDist"] = source["cullDist"]
    output["colorsIndex"] = source["colorsIndex"]
    output["visibility"] = source["visibility"] & 255
    output["reflectionProbeIndex"] = source["reflectionProbeIndex"] & 255

# ComWorld owns the authored primary-light array. GfxWorld retains its count
# and the sun override; R_InitPrimaryLights (00430300) builds GfxLight from these
# exact values, including its spot/square/round technique selection.
common_world, = walk.by_type("comworld")
primary_lights = list(common_world.record["primaryLights"] or ())
assert len(primary_lights) == common_world.record["primaryLightCount"]
assert len(primary_lights) == world["primaryLightCount"]


def primary_light(light):
    result = {name: light[name] for name in (
        "type", "canUseShadowMap", "exponent", "priority", "cullDist", "useCookie",
        "shadowmapVolume", "radius", "cosHalfFovOuter", "cosHalfFovInner",
        "cosHalfFovExpanded", "rotationLimit", "translationLimit", "mipDistance",
        "dAttenuation", "roundness", "defName",
    )}
    for name in ("color", "dir", "origin"):
        result[name] = list(light.unpack("<3f", name))
    for name in ("diffuseColor", "falloff", "angle", "aAbB", "cookieControl0",
                 "cookieControl1", "cookieControl2"):
        result[name] = list(light.unpack("<4f", name))
    return result


data["primaryLights"] = [primary_light(light) for light in primary_lights]
data["sunPrimaryLightIndex"] = world["sunPrimaryLightIndex"]
sun = world["sunLight"]
# This destination is initialized by R_UpdateWorldSun0043f830 from sunParse;
# retain its source bytes, while the existing data["sun"] owns initial values.
data["sunLight"] = {
    "type": sun["type"],
    "dir": list(sun.unpack("<3f", "dir")),
    "diffuseColor": list(sun.unpack("<4f", "diffuseColor")),
} if sun is not None else None
data["lightDefinitions"] = {}
light_definition_names = {light["defName"] for light in primary_lights if light["defName"] is not None}
for surface in data["surfaces"]:
    assert surface["primaryLightIndex"] < len(primary_lights), "World primary-light index"

# Preserve native technique slots and flags alongside water's material inputs.
# Flag1 requests resolvedPostSun; flag2 requests resolvedScene. Native004549a0
# aliases these logical inputs to one physical texture via00453ea0(6,7), and
# RB_DrawScene00424ff0 updates it before each corresponding draw group.
water_materials = [surface["material"] for surface in data["surfaces"]
                   if surface["material"] == "wpc/zm_tm_water_slow"]
if water_materials:
    water, = (item for item in walk.by_type("material") if item.name == water_materials[0])
    technique_set = water.record["techniqueSet"].record
    data["waterTechniques"] = []
    for slot in range(36):
        technique = technique_set[f"techniques[{slot}]"]
        if technique is not None:
            data["waterTechniques"].append({
                "slot": slot, "name": technique["name"], "flags": technique["flags"],
                "passCount": technique["passCount"],
            })

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

# FX references shared-zone materials as well as map-local ones. Export those
# original definitions through the same SDK, in the game's overlay order.
fx_catalog = json.loads((root / "gameplay/origins/fx_catalog.json").read_text(encoding="utf-8"))
fx_materials = set(fx_catalog["materials"])
exported_fx_materials = set()
material_names = material.ZoneNames()
for source_zone in ("code_post_gfx_zm", "common_zm", "patch_zm", "zm_tomb", "zm_tomb_patch"):
    source_path = zone.with_name(source_zone + ".ff")
    source_walk = walk if source_path == zone else backends.for_fastfile(str(source_path)).walk(str(source_path))
    source_walk.check()
    for light in source_walk.by_type("lightdef"):
        if light.name not in light_definition_names:
            continue
        data["lightDefinitions"][light.name] = {
            "attenuationImage": (light.record["attenuation.image"].name
                                 if light.record["attenuation.image"] is not None else None),
            "attenuationSamplerState": light.record["attenuation.samplerState"] & 255,
            "lmapLookupStart": light.record["lmapLookupStart"],
        }
    for source_material in source_walk.by_type("material"):
        if source_material.name not in fx_materials:
            continue
        destination = assets / "materials" / (material_file_stem(source_material.name) + ".json")
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(json.dumps(material.material_json(source_material, material_names),
            separators=(",", ":")), encoding="utf-8")
        exported_fx_materials.add(source_material.name)
assert exported_fx_materials == fx_materials, f"Missing original FX materials: {sorted(fx_materials - exported_fx_materials)}"
print(f"Original FX material definitions: {len(exported_fx_materials)}")


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
