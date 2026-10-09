import argparse
import hashlib
import io
from pathlib import Path
import struct

import numpy as np
from PIL import Image

try:
    from .material_textures import inspect_dds
except ImportError:
    from material_textures import inspect_dds


SKY_PATH = "sky/zm_tomb.dds"
SOURCE_SHA256 = "91182038ee52785a60c37636e78dff756a1714701744608773c39229cb5bb6b3"


def validate_sky(raw):
    layout = inspect_dds(raw, allow_non_2d=True)
    if (layout["dxgi"], layout["dimension"], layout["surfaces"],
            layout["width"], layout["height"], layout["mip_count"]) != (10, "cube", 6, 256, 256, 9):
        raise ValueError("Origins sky must be a complete 256px linear RGBA16F cubemap")
    pixels = np.frombuffer(raw, dtype="<f2", offset=148).reshape(-1, 4)
    if not np.isfinite(pixels).all() or (pixels[:, :3] < 0).any() or not (pixels[:, 3] == 1).all():
        raise ValueError("invalid linear HDR sky pixels")
    return layout


def prepare(source, destination):
    if source.resolve() == destination.resolve():
        raise ValueError("sky output must preserve the original source")
    raw = source.read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("expected the retained Origins sky extraction with mip-major cubemap storage")
    layout = inspect_dds(raw, allow_non_2d=True)
    if (layout["dxgi"], layout["dimension"], layout["surfaces"],
            layout["width"], layout["height"], layout["mip_count"]) != (77, "cube", 6, 256, 256, 9):
        raise ValueError("expected the original Origins BC3 sky cubemap")
    offset = 148 if raw[84:88] == b"DX10" else 128
    payloads = {}
    for mip in sorted(layout["mips"], key=lambda value: (value["level"], value["surface"])):
        width, height, size = mip["width"], mip["height"], mip["bytes"]
        header = [124, 0x81007, height, width, size, 0, 1, *([0] * 11),
                  32, 4, int.from_bytes(b"DXT5", "little"), 0, 0, 0, 0, 0,
                  0x1000, 0, 0, 0, 0]
        image = b"DDS " + struct.pack("<31I", *header) + raw[offset:offset + size]
        with Image.open(io.BytesIO(image)) as decoded:
            pixels = np.asarray(decoded.convert("RGBA"), dtype=np.float32) / 255
        pixels[..., :3] /= pixels[..., 3:4] + np.float32(0.000001)
        pixels[..., 3] = 1
        if not np.isfinite(pixels).all() or np.max(pixels) > np.finfo(np.float16).max:
            raise ValueError("source sky exceeds finite RGBA16F range")
        surface = mip["surface"]
        pixels = pixels[::-1] if surface in (2, 3) else pixels[:, ::-1]
        surface = 9 - surface if surface in (4, 5) else surface
        payloads[surface, mip["level"]] = pixels.astype("<f2").tobytes()
        offset += size
    header = [124, 0x2100F, 256, 256, 256 * 8, 0, 9, *([0] * 11),
              32, 4, int.from_bytes(b"DX10", "little"), 0, 0, 0, 0, 0,
              0x401008, 0xFE00, 0, 0, 0]
    result = (b"DDS " + struct.pack("<31I", *header)
              + struct.pack("<5I", 10, 3, 4, 1, 0)
              + b"".join(payloads[mip["surface"], mip["level"]] for mip in layout["mips"]))
    validate_sky(result)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(result)
    print(f"source SHA256 {hashlib.sha256(raw).hexdigest()}")
    print(f"runtime SHA256 {hashlib.sha256(result).hexdigest()}; {len(result)} bytes")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("--output", type=Path,
                        default=Path(__file__).resolve().parents[1] / "assets/origins" / SKY_PATH)
    args = parser.parse_args()
    prepare(args.source, args.output)
