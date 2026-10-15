"""Keep original world shader inputs through the existing typed zone exporter."""
import json
import sys
from pathlib import Path

root, external, zone = map(Path, sys.argv[1:])
sys.path.insert(0, str(external))
from scripts.cod import backends
from scripts.cod.t6.dumpers import gfxworld

walk = backends.for_fastfile(str(zone)).walk(str(zone))
walk.check()
asset, = walk.by_type("gfxworld")
world = asset.record
surfaces, _, _ = gfxworld.convert(world)
data = gfxworld.manifest(asset, surfaces, world["draw.vertexCount"])

# Keep the source surface order: material alone is not a complete draw binding.
for output, source in zip(data["surfaces"], world["dpvs.surfaces"], strict=True):
    output["reflectionProbeIndex"] = source["reflectionProbeIndex"]
    output["primaryLightIndex"] = source["primaryLightIndex"]
data["reflectionProbes"] = [
    {
        "origin": list(probe.unpack("<3f", "origin")),
        "lightingSH": list(probe.unpack("<12f", "lightingSH")),
        "reflectionImage": probe["reflectionImage"].name,
        "mipLodBias": probe["mipLodBias"],
    }
    for probe in world["draw.reflectionProbes"]
]


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
