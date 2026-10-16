"""Keep native XModel translation defaults separate from model-space skin binds."""
import argparse
import json
import math
import struct
import sys
from pathlib import Path

from native_model_inputs import local_translations


def correct_translations(gltf, binary, model, asset_type):
    from cod.animations import OFFSET_BONES, TRANSLATION_BY_ASSET_TYPE

    defaults = dict(zip(model["boneNames"], model["localTranslations"], strict=True))
    joints = {joint for skin in gltf["skins"] for joint in skin["joints"]}
    parents = {child: index for index, node in enumerate(gltf["nodes"]) for child in node.get("children", [])}
    absolute = TRANSLATION_BY_ASSET_TYPE.get(asset_type) == "absolute"
    changed = 0
    for animation in gltf.get("animations", []):
        extras = animation.setdefault("extras", {})
        if extras.get("sourceLocalTranslations"):
            continue
        for channel in animation["channels"]:
            if channel["target"]["path"] != "translation":
                continue
            index = channel["target"]["node"]
            node = gltf["nodes"][index]
            name = node["name"]
            # Roots have no XModel.trans entry. The external writer already
            # rotates root keys into the exported coordinate frame.
            desired = defaults[name] if parents.get(index) in joints else [0.0, 0.0, 0.0]
            previous = [0.0, 0.0, 0.0] if absolute and name not in OFFSET_BONES else node.get("translation", [0.0, 0.0, 0.0])
            offset = [wanted - old for wanted, old in zip(desired, previous, strict=True)]
            if not any(offset):
                continue
            sampler = animation["samplers"][channel["sampler"]]
            accessor = gltf["accessors"][sampler["output"]]
            assert accessor["componentType"] == 5126 and accessor["type"] == "VEC3" and "sparse" not in accessor
            view = gltf["bufferViews"][accessor["bufferView"]]
            start = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
            stride = view.get("byteStride", 12)
            assert sampler.get("interpolation", "LINEAR") in ("LINEAR", "STEP"), "Unexpected source translation interpolation"
            for key in range(accessor["count"]):
                at = start + key * stride
                values = [value + delta for value, delta in zip(struct.unpack_from("<3f", binary, at), offset, strict=True)]
                assert all(math.isfinite(value) for value in values), f"Nonfinite source translation: {name}"
                struct.pack_into("<3f", binary, at, *values)
            for bound in ("min", "max"):
                if bound in accessor:
                    accessor[bound] = [value + delta for value, delta in zip(accessor[bound], offset, strict=True)]
            changed += 1
        extras["sourceLocalTranslations"] = True
    return changed


def repair(assets, zone, report_path):
    from cod.gltf import read_glb, write_glb
    from scripts.cod.iw5.dumpers import xmodel
    from native_images import source_walks

    records = json.loads((assets / "source-model-clips.json").read_text())
    weapons = json.loads((assets / "weapons.json").read_text())
    scenes = {entry["scene"].split("#", 1)[0] for entry in weapons["presentations"] if entry.get("scene")}
    scenes.update(part["scene"].split("#", 1)[0] for weapon in weapons["weapons"] for part in weapon.get("model_parts", []))
    for weapon in json.loads((assets / "source-weapons.json").read_text()).values():
        scenes.add(weapon["scene"].split("#", 1)[0])
        scenes.update(part["scene"].split("#", 1)[0] for part in weapon["parts"])
    hands = {path for clips in json.loads((assets / "source-hands.json").read_text()).values() for path in clips.values()}
    records = [record for record in records if record["scene"] in scenes or any(track.get("animation_path") in hands for track in record["tracks"])]
    required = {record["model"] for record in records}
    wanted = {track["name"] for record in records for track in record["tracks"] if track.get("animation_path")}
    models, types = {}, {}
    for walk in source_walks(zone):
        for asset in walk.by_type("xmodel"):
            if asset.name in required:
                models[asset.name] = {"boneNames": xmodel.bone_names(walk, asset.record), "localTranslations": local_translations(asset.record)}
        for asset in walk.by_type("xanim"):
            if asset.name in wanted:
                types[asset.name] = asset.record["assetType"]
        print(f"Read {walk.path}", flush=True)
    assert required <= models.keys(), f"Missing source models: {sorted(required - models.keys())}"
    assert wanted <= types.keys(), f"Missing source clips: {sorted(wanted - types.keys())}"
    manifest = json.loads((assets / "native_models.json").read_text())
    for name, model in models.items():
        path = assets / "model_export" / (name + ".native_model")
        definition = json.loads(path.read_text())
        assert definition["boneNames"] == model["boneNames"], f"Source bone order changed: {name}"
        definition["localTranslations"] = model["localTranslations"]
        path.write_text(json.dumps(definition, separators=(",", ":")), encoding="utf-8")
        manifest[name] = definition
    (assets / "native_models.json").write_text(json.dumps(manifest, separators=(",", ":")), encoding="utf-8")
    repaired, checked = {}, {}
    for record in records:
        model = models[record["model"]]
        for track in record["tracks"]:
            if not track.get("animation_path"):
                continue
            path = track["animation_path"].split("#", 1)[0]
            if path in checked:
                assert checked[path] == model, f"Shared clip has different source defaults: {path}"
                continue
            checked[path] = model
            document, binary = read_glb(str(assets / path))
            if all(animation.get("extras", {}).get("sourceLocalTranslations") for animation in document["animations"]):
                continue
            binary = bytearray(binary)
            count = correct_translations(document, binary, model, types[track["name"]])
            write_glb(str(assets / path), document, binary)
            if count:
                repaired[path] = count
    report = {"models": len(models), "scenes": len(records), "clips_checked": len(checked), "clips_changed": len(repaired), "translation_channels_changed": sum(repaired.values()), "changed_clips": repaired}
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: value for key, value in report.items() if key != "changed_clips"}), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--tool", type=Path, required=True)
    parser.add_argument("--zone", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    sys.path[:0] = [str(args.tool), str(args.tool / "scripts")]
    repair(args.assets, args.zone, args.report)
