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


SKY_PATH = "sky/zm_tomb.ktx2"
SOURCE_SHA256 = "91182038ee52785a60c37636e78dff756a1714701744608773c39229cb5bb6b3"


def validate_sky(raw):
    try:
        from .ktx2_textures import inspect_ktx2, payloads
    except ImportError:
        from ktx2_textures import inspect_ktx2, payloads
    layout = inspect_ktx2(raw)
    if (layout["dxgi"], layout["dimension"], layout["surfaces"],
            layout["width"], layout["height"], layout["mip_count"]) != (10, "cube", 6, 256, 256, 9):
        raise ValueError("Origins sky must be a complete 256px linear RGBA16F cubemap")
    for data in payloads(raw, layout):
        pixels = np.frombuffer(data, dtype="<f2").reshape(-1, 4)
        if not np.isfinite(pixels).all() or (pixels[:, :3] < 0).any() or not (pixels[:, 3] == 1).all():
            raise ValueError("invalid linear HDR sky pixels")
    return layout


def prepare(source, destination, executable, work):
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
    try:
        from .prepare_ktx2 import save_container
    except ImportError:
        from prepare_ktx2 import save_container
    parts = [b"".join(payloads[surface, level] for surface in range(6)) for level in range(9)]
    save_container(parts, 97, 256, 256, source, destination, executable, work, "sky", False, faces=6)
    result = destination.read_bytes()
    validate_sky(result)
    print(f"source SHA256 {hashlib.sha256(raw).hexdigest()}")
    print(f"runtime SHA256 {hashlib.sha256(result).hexdigest()}; {len(result)} bytes")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("--preparation-executable", required=True, type=Path)
    parser.add_argument("--work", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    prepare(args.source, args.output, args.preparation_executable, args.work)
