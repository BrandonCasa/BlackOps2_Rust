import argparse
import hashlib
import json
import struct
from pathlib import Path

from ktx2_textures import MAGIC, decoded_levels, inspect_ktx2


def prepare(pack, extraction, output):
    if output.resolve() == pack.resolve():
        raise ValueError("Prepare in a separate output directory")
    source = extraction / "images/mtl_c_zom_zombie_barbwire_head_char_m.dds"
    native = json.loads((extraction / "materials/mc/mtl_p6_zm_tm_note_rock_01_anim.json").read_text())
    constants = {c["name"]: c["literal"] for c in native["constants"]}
    levels = decoded_levels(source.read_bytes())
    template = next(p for p in (pack / "library/textures").glob("*.ktx2") if inspect_ktx2(p.read_bytes())["vk_format"] == 37)
    raw = template.read_bytes()
    start, size = struct.unpack_from("<2I", raw, 48)
    dfd = raw[start:start + size]
    offset = 80 + len(levels) * 24
    header = MAGIC + struct.pack("<9I", 37, 1, levels[0].shape[1], levels[0].shape[0], 0, 0, 1, len(levels), 0)
    header += struct.pack("<4I2Q", offset, len(dfd), 0, 0, 0, 0)
    offset += len(dfd)
    index, payload = bytearray(), bytearray(dfd)
    for level in levels:
        padding = -offset % 4
        payload.extend(bytes(padding))
        offset += padding
        data = level.tobytes()
        index.extend(struct.pack("<3Q", offset, len(data), len(data)))
        payload.extend(data)
        offset += len(data)
    cooked = header + index + payload
    info = inspect_ktx2(cooked)
    uri = "library/textures/frozen-" + hashlib.sha256(cooked).hexdigest()[:32] + ".ktx2"
    target = output / uri
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(cooked)
    files = []
    for path in (pack / "library/models").rglob("p6_zm_tm_note_rock_01_anim_lod*.gltf"):
        document = json.loads(path.read_text())
        for material in document["materials"]:
            if material["name"] != "mc/mtl_p6_zm_tm_note_rock_01_anim":
                continue
            surface = material["extras"]["surface"]
            surface["frozen_stone"] = {"constants": constants, "temporal_source_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}
            surface["channels"] = [c for c in surface["channels"] if c["role"] != "control"]
            surface["channels"].append({"uri": uri, "role": "control", "layer": 0, "color_space": "linear", "format": "R8G8B8A8_UNORM", "width": info["width"], "height": info["height"], "mip_count": len(levels), "channels": "r:native_temporal_reveal", "sampler": 20})
        relative = path.relative_to(pack)
        target = output / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(document, separators=(",", ":")) + "\n")
        files.append(relative.as_posix())
    return {"models": files, "texture": uri, "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--pack", type=Path, required=True)
    parser.add_argument("--extraction", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args.pack, args.extraction, args.output)))
