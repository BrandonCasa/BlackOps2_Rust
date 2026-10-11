import argparse
import hashlib
import json
from pathlib import Path
import re
import struct

from PIL import Image

try:
    from .ktx2_textures import inspect_ktx2, payloads
except ImportError:
    from ktx2_textures import inspect_ktx2, payloads


GRADE_PATH = "sky/zm_tomb_grade.ktx2"
SOURCE_LUT_SHA256 = "887d2d470f0c07f2632979d689f83df974befbf13a6e0d25e6018400a423f2bd"
BASE_TEXELS_SHA256 = "a687b429bfa844cba5b7f075550203950dd9e2e6bedf143b52866a35a56c0bf5"


def validate_grade(raw):
    info = inspect_ktx2(raw)
    if (info["vk_format"], info["width"], info["height"], info["surfaces"],
            info["mip_count"], info["dimension"]) != (37, 1024, 64, 1, 11, "2d"):
        raise ValueError("Origins color lookup must retain both original 32 cubed tables")
    base = next(payloads(raw, info))
    if hashlib.sha256(base).hexdigest() != BASE_TEXELS_SHA256:
        raise ValueError("Origins color lookup differs from the original texels")
    if any(alpha != 255 for alpha in base[3::4]):
        raise ValueError("Origins color lookup has unexpected alpha")
    return info


def fog(value):
    if value["sun_angles"][2] != 0:
        raise ValueError("Origins fog expects a zero inner sun angle")
    return {**value, "distance": [number * 0.0254 for number in value["distance"]]}


def save_lookup(parts, output):
    samples = b"".join(struct.pack("<HBB4BII", channel * 8, 7, kind, 0, 0, 0, 0, 0, 255)
                       for channel, kind in enumerate((0, 1, 2, 15)))
    descriptor = struct.pack("<IHH4B4B8B", 0, 2, 24 + len(samples), 1, 1, 1, 0,
                             0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0) + samples
    descriptor = struct.pack("<I", 4 + len(descriptor)) + descriptor
    descriptor_offset = 80 + 24 * len(parts)
    offset = descriptor_offset + len(descriptor)
    indices, payload = [], bytearray()
    for part in parts:
        padding = -offset % 4
        payload.extend(bytes(padding))
        offset += padding
        indices.append(struct.pack("<3Q", offset, len(part), len(part)))
        payload.extend(part)
        offset += len(part)
    header = b"\xabKTX 20\xbb\r\n\x1a\n" + struct.pack("<9I4I2Q", 37, 1, 1024, 64, 0, 0,
        1, len(parts), 0, descriptor_offset, len(descriptor), 0, 0, 0, 0)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(header + b"".join(indices) + descriptor + payload)


def prepare(world, vision_root, source, output, config):
    native = json.loads(world.read_text())
    visions = {}
    provenance = {"atmosphere-world.json": hashlib.sha256(world.read_bytes()).hexdigest()}
    for suffix in ("", "_rain", "_snow"):
        path = vision_root / ("zm_tomb" + suffix + ".vision")
        visions[suffix or "clear"] = {
            name: [float(number) for number in values.split()]
            for name, values in re.findall(r'(vc_\w+)\s+"([^"]+)"', path.read_text())
        }
        provenance[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    if visions["_rain"] != visions["_snow"]:
        raise ValueError("Expected the shared original rain and snow film settings")
    volumes = []
    for order, volume in enumerate(native["volumes"]):
        control = volume["control"]
        first, count = control >> 16, ((control >> 8) & 255) + 1
        planes = native["planes"][first:first + count]
        if len(planes) != count:
            raise ValueError("Invalid original fog plane range")
        volumes.append({
            "order": order, "bank": control & 255, "script_id": volume["control_ex"] & 65535,
            "transition_seconds": 0.001 / volume["transition"] if volume["transition"] else 0,
            "planes": [[x, z, -y, distance * 0.0254] for x, y, z, distance in planes],
            "fog": fog(volume["fog"]),
        })
    provenance[source.name] = hashlib.sha256(source.read_bytes()).hexdigest()
    if provenance[source.name] != SOURCE_LUT_SHA256:
        raise ValueError("Expected the retained original Origins color lookup")
    data = {
        "source_sha256": provenance, "sun_angles": native["sun"]["angles"],
        "sun_color": native["sun"]["diffuse"][:3], "snow_sun_color": [0.769943, 0.894102, 1],
        "sun_brightness": native["sun"]["diffuse"][3],
        "fog_transition_seconds": 0.001 / native["fog_transition"] if native["fog_transition"] else 0,
        "fog": fog(native["fog"]), "volumes": volumes,
        "softclip": 0.25, "vision": visions,
    }
    if source.resolve() == output.resolve():
        raise ValueError("Runtime color lookup must preserve the original source")
    with Image.open(source) as image:
        image = image.convert("RGBA")
        if image.size != (1024, 64):
            raise ValueError("Unexpected original color lookup dimensions")
        original = image.tobytes()
        parts = [original]
        for level in range(1, 11):
            parts.append(image.resize((max(1, 1024 >> level), max(1, 64 >> level)), Image.Resampling.BOX).tobytes())
    save_lookup(parts, output)
    raw = output.read_bytes()
    info = validate_grade(raw)
    if next(payloads(raw, info)) != original:
        raise ValueError("Runtime lookup texels differ from the original")
    config.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"Prepared {len(volumes)} fog volumes and original color lookup {info['sha256']}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--world", required=True, type=Path)
    parser.add_argument("--vision-root", required=True, type=Path)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--config", required=True, type=Path)
    args = parser.parse_args()
    prepare(args.world, args.vision_root, args.source, args.output, args.config)
