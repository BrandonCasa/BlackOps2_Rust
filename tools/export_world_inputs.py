"""Keep original world shader inputs through the existing typed zone exporter."""
import json
import struct
import sys
from pathlib import Path

root, external, zone = map(Path, sys.argv[1:])
sys.path.insert(0, str(external))
from scripts.cod import backends
from scripts.cod.t6.dumpers import gfxworld, material, rawfile
from scripts.cod.materials import material_file_stem
from scripts.cod.iw5.dumpers import xmodel
from native_model_inputs import export_model
from native_visibility import export_visibility
from native_images import source_walks, stage_source_images

walk = backends.for_fastfile(str(zone)).walk(str(zone))
walk.check()
material.write(walk, str(root / "target/assets"))
def stage_ordinary_visions(walks):
    required = {"vision/zm_tomb.vision", "vision/zm_tomb_rain.vision", "vision/zm_tomb_snow.vision"}
    found = set()
    for source_walk in walks:
        for source_asset in source_walk.by_type("rawfile"):
            if source_asset.name not in required:
                continue
            path = root / "target/assets" / source_asset.name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(rawfile.contents(source_asset))
            found.add(source_asset.name)
        yield source_walk
    assert found == required, f"Missing original ordinary vision files: {sorted(required - found)}"


image_report = stage_source_images(stage_ordinary_visions(source_walks(zone, walk)), root / "target/assets")
image_report_path = root / "target/reports/native-rgba-source-restage.json"
image_report_path.parent.mkdir(parents=True, exist_ok=True)
image_report_path.write_text(json.dumps(image_report, indent=2) + "\n", encoding="utf-8")
print("Restored source images:", image_report["counts"], flush=True)
asset, = walk.by_type("gfxworld")
world = asset.record
surfaces, _, indices = gfxworld.convert(world)
data = gfxworld.manifest(asset, surfaces, world["draw.vertexCount"])
# 0043fcf0 loads this exact GfxWorld+0x24 name as the camera-centered sky.
# It is separate from the BSP's static placements and must survive export.
sky_model = world["skyBoxModel"]
assert sky_model, "Original Origins GfxWorld has no authored sky model"
data["sky"] = {"model": sky_model}
export_visibility(world, root / "target/assets")

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
    output["flags"] = source["flags"] & 255
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
        "probeVolumes": [
            [list(volume.unpack("<4f", f"volumePlanes[{i}]")) for i in range(6)]
            for volume in probe["probeVolumes"] or ()
        ],
    }
    for probe in world["draw.reflectionProbes"]
]
# Dynamic reflection selection: explicit probe volumes (00409490), then the
# camera-space BSP cell (00408ef0) and that cell's ordered probe list (00409230).
# Outside the BSP, 00409410 considers only probes without authored volumes.
data["reflectionCells"] = []
for cell in world["cells"]:
    probes = cell["reflectionProbes"]
    indices = list(bytes(probes.bytes())) if probes is not None else []
    assert len(indices) == (cell["reflectionProbeCount"] & 255)
    assert all(i < len(data["reflectionProbes"]) for i in indices)
    data["reflectionCells"].append(indices)
assert len(data["reflectionCells"]) == world["dpvsPlanes.cellCount"]
data["reflectionPlanes"] = [
    [*plane.unpack("<3f", "normal"), plane["dist"]]
    for plane in world["dpvsPlanes.planes"]
]
assert len(data["reflectionPlanes"]) == world["planeCount"]
node_bytes = bytes(world["dpvsPlanes.nodes"].bytes())
assert len(node_bytes) == world["nodeCount"] * 2
data["reflectionNodes"] = [value for (value,) in struct.iter_unpack("<H", node_bytes)]

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

# Moving models sample the spatial grid; a static placement's colorsIndex is
# not a valid substitute. Preserve the loader's exact sparse rows and entries
# (GfxLightGrid / Load_GfxLightGrid0058a090), including legacy color volumes.
grid = {
    key: world[f"lightGrid.{key}"]
    for key in ("sunPrimaryLightIndex", "offset", "rowAxis", "colAxis",
                "rawRowDataSize", "entryCount", "colorCount", "coeffCount",
                "skyGridVolumeCount")
}
grid["mins"] = list(world.unpack("<3H", "lightGrid.mins"))
grid["maxs"] = list(world.unpack("<3H", "lightGrid.maxs"))
assert grid["rowAxis"] in range(3) and grid["colAxis"] in range(3)
assert grid["rowAxis"] != grid["colAxis"]
assert all(a <= b for a, b in zip(grid["mins"], grid["maxs"]))
row_count = grid["maxs"][grid["rowAxis"]] - grid["mins"][grid["rowAxis"]] + 1
grid_streams = {}
for field, size in (
    ("rowDataStart", row_count * 2),
    ("rawRowData", grid["rawRowDataSize"]),
    ("entries", grid["entryCount"] * 4),
    ("colors", grid["colorCount"] * 168),
    ("skyGridVolumes", grid["skyGridVolumeCount"] * 40),
):
    source = world[f"lightGrid.{field}"]
    packed = bytes(source.bytes()) if source is not None else b""
    assert len(packed) == size, f"Original light-grid {field} size"
    name = f"{asset.name}.lightgrid_{field}.bin"
    (assets / name).write_bytes(packed)
    grid[field + "File"] = name
    grid_streams[field] = packed

# 0044c900: row offsets are uint16 word indices into four-byte units; each
# row starts with four uint16 bounds and a uint32 first-entry index. Runs
# store column count, sample height, and an 8/16-bit height offset.
raw_rows = grid_streams["rawRowData"]
for (row_offset,) in struct.iter_unpack("<H", grid_streams["rowDataStart"]):
    if row_offset == 0xffff:
        continue
    at = row_offset * 4
    assert at + 12 <= len(raw_rows), "Original light-grid row header range"
    column, width, z_min, z_count, first_entry = struct.unpack_from("<4HI", raw_rows, at)
    at += 12
    consumed = 0
    while consumed < width:
        assert at + 2 <= len(raw_rows), "Original light-grid run range"
        columns, height = struct.unpack_from("<BB", raw_rows, at)
        assert columns > 0 and consumed + columns <= width, "Original light-grid run width"
        if height:
            stride = 4 if z_count > 255 else 3
            assert at + stride <= len(raw_rows), "Original light-grid height range"
            z_offset = struct.unpack_from("<H" if z_count > 255 else "<B", raw_rows, at + 2)[0]
            assert z_offset + height <= z_count, "Original light-grid run height"
            assert first_entry + columns * height <= grid["entryCount"], "Original light-grid entries range"
        else:
            stride = 2
        first_entry += columns * height
        consumed += columns
        at += stride
for colors_index, primary_light, visibility in struct.iter_unpack("<HBB", grid_streams["entries"]):
    assert colors_index < max(grid["coeffCount"], grid["colorCount"]), "Original light-grid sample index"
grid["coefficientsFile"] = coefficient_name
data["lightGrid"] = grid
model_vertex_lighting = bytearray()
for output, source in zip(data["staticModels"], world["dpvs.smodelDrawInsts"], strict=True):
    output["cullDist"] = source["cullDist"]
    output["colorsIndex"] = source["colorsIndex"]
    output["visibility"] = source["visibility"] & 255
    output["reflectionProbeIndex"] = source["reflectionProbeIndex"] & 255
    output["primaryLightIndex"] = source["primaryLightIndex"] & 255
    # 00590e40/00589920: four independently serialized per-placement LOD
    # color streams, four bytes per vertex. The VB pointer is runtime-only.
    output["vertexLighting"] = []
    for lod in range(4):
        count = source[f"lmapVertexInfo[{lod}].numLmapVertexColors"]
        colors = source[f"lmapVertexInfo[{lod}].lmapVertexColors"]
        assert colors is not None or count == 0, "Missing static-model vertex lighting"
        packed = bytes(colors.bytes()) if colors is not None else b""
        assert len(packed) == count * 4, "Original model vertex-lighting byte count"
        output["vertexLighting"].append({"byteOffset": len(model_vertex_lighting), "vertexCount": count})
        model_vertex_lighting.extend(packed)
lighting_name = f"{asset.name}.model_vertex_lighting.bin"
(assets / lighting_name).write_bytes(model_vertex_lighting)
data["modelVertexLightingFile"] = lighting_name

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
for placement in data["staticModels"]:
    assert placement["primaryLightIndex"] < len(primary_lights), "Model primary-light index"

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
                # 0045a470 begins model draw keys with the material's packed
                # key, then supplies placement light/type and dynamic-light bits.
                "materialDrawSurf": source_material.record["info.drawSurf.packed"],
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

# baseVertIndex addresses the placement's LOD color array, independently of
# the exported PACKED32 byte offset. Reject incomplete source streams here.
for placement in data["staticModels"]:
    lods = data["modelGeometry"][placement["model"]]["lods"]
    for lod_index, lighting in enumerate(placement["vertexLighting"]):
        count = lighting["vertexCount"]
        if count == 0:
            continue
        assert lod_index < len(lods), "Vertex lighting references a missing model LOD"
        for surface in lods[lod_index]["surfaces"]:
            assert surface["baseVertIndex"] + surface["vertexCount"] <= count, (
                f"Incomplete vertex lighting: {placement['model']} LOD {lod_index} "
                f"surface {surface['material']}"
            )

# FX references shared-zone materials as well as map-local ones. Export those
# original definitions through the same SDK, in the game's overlay order.
fx_catalog = json.loads((root / "gameplay/origins/fx_catalog.json").read_text(encoding="utf-8"))
fx_materials = set(fx_catalog["materials"])
model_materials = set()
source_materials = {}
source_attachments = {}
material_names = material.ZoneNames()
dynamic_models = {}
dynamic_names = {
    path.stem.rsplit("_lod", 1)[0]
    for path in (assets / "model_export").glob("*_lod*.glb")
}
# Source models can serve both static placements and moving gameplay props.
# Every exported LOD scene needs native metadata when loaded as an instance.
dynamic_names.update(json.loads((root / "tools/origins_model_animation_bindings.json").read_text(encoding="utf-8")))
dynamic_names.add(sky_model)
for source_zone in ("code_post_gfx_zm", "common_zm", "patch_zm", "zm_tomb", "zm_tomb_patch"):
    source_path = zone.with_name(source_zone + ".ff")
    source_walk = walk if source_path == zone else backends.for_fastfile(str(source_path)).walk(str(source_path))
    source_walk.check()
    for model_asset in source_walk.by_type("xmodel"):
        if model_asset.name not in dynamic_names:
            continue
        dynamic_models[model_asset.name] = export_model(source_walk, model_asset, assets)
        for model_material in model_asset.record["materialHandles"]:
            model_materials.add(model_material.name.lstrip(","))
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
        # A comma record references an earlier zone; its empty body must never
        # overwrite the concrete material. Later concrete definitions do win.
        if source_material.name.startswith(","):
            continue
        source_materials[source_material.name] = material.material_json(source_material, material_names)
    for attachment in source_walk.by_type("attachment"):
        if attachment.name.startswith(","):
            continue
        record = attachment.record
        source_attachments[attachment.name] = {
            "mmsWeapon": bool(record["mmsWeapon"]),
            "mmsInScope": bool(record["mmsInScope"]),
            "mmsFOV": record["mmsFOV"],
            "mmsAspect": record["mmsAspect"],
            "mmsMaxDist": record["mmsMaxDist"],
        }
required_materials = fx_materials | model_materials | {
    surface["material"].lstrip(",") for surface in data["surfaces"]
}
assert required_materials <= source_materials.keys(), (
    f"Missing original material definitions: {sorted(required_materials - source_materials.keys())}"
)
for name in sorted(required_materials):
    destination = assets / "materials" / (material_file_stem(name) + ".json")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(source_materials[name], separators=(",", ":")), encoding="utf-8")
print(f"Original FX material definitions: {len(fx_materials)}")
assert "mms" in source_attachments, "Missing original MMS attachment inputs"
(assets / "source-attachments.json").write_text(
    json.dumps(source_attachments, separators=(",", ":")), encoding="utf-8"
)
assert set(dynamic_models) == dynamic_names, f"Missing native dynamic models: {sorted(dynamic_names - set(dynamic_models))}"
(assets / "native_models.json").write_text(json.dumps(dynamic_models, separators=(",", ":")), encoding="utf-8")
print(f"Original dynamic model inputs: {len(dynamic_models)}")


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

# Native00465600 consumes these original per-primary-light caster lists.
# Runtime surfaceCastsSunShadow/surfaceCastsShadow are rebuilt by00442ec0
# from material.gameFlags&0x40 and surface.flags&1; they are not fastfile data.
data["shadowGeometry"] = []
data["staticModelCount"] = world["dpvs.smodelCount"]
for geometry in world["shadowGeom"] or ():
    def indices(member, count):
        source = geometry[member]
        assert source is not None or count == 0, f"Missing shadow geometry {member}"
        return [value for (value,) in source.view(0, count).unpack("<H")] if count else []

    surface_indices = indices("sortedSurfIndex", geometry["surfaceCount"])
    model_indices = indices("smodelIndex", geometry["smodelCount"])
    assert all(index < world["surfaceCount"] for index in surface_indices)
    assert all(index < world["dpvs.smodelCount"] for index in model_indices)
    data["shadowGeometry"].append({"surfaceIndices": surface_indices, "staticModelIndices": model_indices})
assert len(data["shadowGeometry"]) == world["primaryLightCount"], "Original primary-light shadow geometry count"
data["shadowMapVolumes"] = [
    {"control": volume["control"]} for volume in world["shadowMapVolumes"] or ()
]
data["shadowMapVolumePlanes"] = [
    list(plane.unpack("<4f", "plane")) for plane in world["shadowMapVolumePlanes"] or ()
]

destination = root / "target/assets/world.json"
destination.write_text(json.dumps(data, separators=(",", ":")), encoding="utf-8")
print(f"World shader inputs: {len(surfaces)} surfaces, "
      f"{len(data['reflectionProbes'])} probes, {len(data['worldFogVolumes'])} fog volumes, "
      f"{len(data['worldFogModifierVolumes'])} fog modifiers, "
      f"{len(data['exposureVolumes'])} exposure volumes")
