import hashlib
import struct

import numpy as np
from PIL import Image

FORMATS = {2: (16, False), 10: (8, False), 11: (8, False), 28: (4, False), 29: (4, False),
           49: (2, False), 54: (2, False), 56: (2, False), 61: (1, False), 65: (1, False),
           87: (4, False), 88: (4, False), 91: (4, False), 93: (4, False),
           71: (8, True), 72: (8, True), 74: (16, True), 75: (16, True),
           77: (16, True), 78: (16, True), 80: (8, True), 81: (8, True),
           83: (16, True), 84: (16, True), 95: (16, True), 96: (16, True),
           98: (16, True), 99: (16, True)}


def inspect_texture(raw, allow_non_2d=False):
    try:
        from .ktx2_textures import MAGIC, inspect_ktx2
    except ImportError:
        from ktx2_textures import MAGIC, inspect_ktx2
    if raw.startswith(MAGIC):
        result = inspect_ktx2(raw)
        if not allow_non_2d and (result['dimension'] != '2d' or result['surfaces'] != 1):
            raise ValueError('Material KTX2 must be one 2D image')
        return result
    return inspect_dds(raw, allow_non_2d=allow_non_2d)


def inspect_dds(raw, allow_non_2d=False):
    if raw[:4] != b"DDS " or len(raw) < 128 or struct.unpack_from("<I", raw, 4)[0] != 124:
        raise ValueError("invalid DDS header")
    height, width = struct.unpack_from("<II", raw, 12)
    levels = max(1, struct.unpack_from("<I", raw, 28)[0])
    fourcc = raw[84:88]
    offset = 128
    depth = max(1, struct.unpack_from("<I", raw, 24)[0])
    caps2 = struct.unpack_from("<I", raw, 112)[0]
    surfaces = 6 if caps2 & 512 else 1
    dimension = "cube" if surfaces == 6 else "3d" if caps2 & 0x200000 else "2d"
    if fourcc == b"DX10":
        if len(raw) < 148:
            raise ValueError("truncated DX10 DDS")
        fmt, dimension, misc, arrays, _ = struct.unpack_from("<5I", raw, 128)
        native_dimension = dimension
        dimension = "cube" if misc & 4 else "3d" if native_dimension == 4 else "2d"
        surfaces = arrays * (6 if misc & 4 else 1)
        if native_dimension not in (3, 4) or arrays < 1:
            raise ValueError("unsupported DDS image dimension")
        if not allow_non_2d and (native_dimension != 3 or arrays != 1 or misc & 4):
            raise ValueError("material DDS must be one 2D image")
        offset = 148
    else:
        fmt = {b"DXT1": 71, b"DXT3": 74, b"DXT5": 77, b"ATI1": 80, b"ATI2": 83}.get(fourcc)
        if fmt is None and struct.unpack_from("<I", raw, 88)[0] == 32:
            fmt = 28
        if fmt is None and struct.unpack_from("<I", raw, 88)[0] == 8:
            fmt = 65 if struct.unpack_from("<I", raw, 80)[0] & 2 else 61
    if fmt not in FORMATS or width < 1 or height < 1 or levels > max(width, height).bit_length():
        raise ValueError("unsupported DDS format or mip count")
    if not allow_non_2d and (dimension != "2d" or surfaces != 1):
        raise ValueError("material DDS must be one 2D image")
    stride, blocked = FORMATS[fmt]
    mips = []
    for surface in range(surfaces):
        for level in range(levels):
            w, h = max(1, width >> level), max(1, height >> level)
            size = ((w + 3) // 4) * ((h + 3) // 4) * stride if blocked else w * h * stride
            size *= max(1, depth >> level) if dimension == "3d" else 1
            payload = raw[offset:offset + size]
            if len(payload) != size:
                raise ValueError("truncated DDS mip payload")
            mips.append({"width": w, "height": h, "bytes": size, "surface": surface, "level": level,
                         "sha256": hashlib.sha256(payload).hexdigest()})
            offset += size
    if offset != len(raw):
        raise ValueError("extra DDS payload or unsupported array")
    return {"dxgi": fmt, "width": width, "height": height, "dimension": dimension, "surfaces": surfaces, "mip_count": levels, "mips": mips,
            "sha256": hashlib.sha256(raw).hexdigest()}


def rgba_dds(levels, srgb=False):
    height, width = levels[0].shape[:2]
    header = [124, 0x2100F, height, width, width * 4, 0, len(levels), *([0] * 11),
              32, 4, int.from_bytes(b"DX10", "little"), 0, 0, 0, 0, 0,
              0x401008 if len(levels) > 1 else 0x1000, 0, 0, 0, 0]
    return (b"DDS " + struct.pack("<31I", *header)
            + struct.pack("<5I", 29 if srgb else 28, 3, 0, 1, 0)
            + b"".join(np.asarray(x, dtype=np.uint8).tobytes() for x in levels))


def rgba_levels(raw):
    record = inspect_dds(raw)
    if record["dxgi"] not in (28, 29) or raw[84:88] != b"DX10":
        raise ValueError("canonical master must be DX10 RGBA8")
    offset, levels = 148, []
    for mip in record["mips"]:
        levels.append(np.frombuffer(raw, np.uint8, mip["bytes"], offset).reshape(mip["height"], mip["width"], 4).copy())
        offset += mip["bytes"]
    return levels


def canonical_normal(pixels, strength=1.0):
    slopes = pixels[..., :2].astype(np.float32) / 255 * 4.015748 - 2.015748
    slopes[..., 1] *= -1
    slopes *= strength
    normal = np.dstack((slopes, np.ones(pixels.shape[:2], np.float32)))
    return normal / np.linalg.norm(normal, axis=2, keepdims=True)


def reduced(values):
    h, w = values.shape[:2]
    nh, nw = max(1, h // 2), max(1, w // 2)
    ys = np.linspace(0, h, nh + 1, dtype=int)
    xs = np.linspace(0, w, nw + 1, dtype=int)
    if h % 2 == 0 and w % 2 == 0:
        return values.reshape(nh, 2, nw, 2, -1).mean(axis=(1, 3))
    return np.asarray([[values[ys[y]:ys[y + 1], xs[x]:xs[x + 1]].mean(axis=(0, 1))
                        for x in range(nw)] for y in range(nh)], np.float32)


def normal_levels(normal):
    levels, variances = [], []
    mean = normal
    while True:
        length = np.linalg.norm(mean, axis=2, keepdims=True)
        unit = mean / np.maximum(length, 1e-6)
        levels.append(np.dstack((unit * 0.5 + 0.5, np.ones(unit.shape[:2]))))
        variances.append(np.clip((1 - length[..., 0]) / np.maximum(length[..., 0], 1e-4), 0, 1))
        if mean.shape[:2] == (1, 1):
            break
        mean = reduced(mean)
    return [np.rint(np.clip(x, 0, 1) * 255).astype(np.uint8) for x in levels], variances


def color_levels(pixels, srgb, cutoff=None):
    values = pixels.astype(np.float32) / 255
    if srgb:
        rgb = values[..., :3]
        values[..., :3] = np.where(rgb <= .04045, rgb / 12.92, ((rgb + .055) / 1.055) ** 2.4)
    coverage = np.mean(values[..., 3] >= cutoff) if cutoff is not None else None
    levels = []
    while True:
        encoded = values.copy()
        if srgb:
            rgb = encoded[..., :3]
            encoded[..., :3] = np.where(rgb <= .0031308, rgb * 12.92, 1.055 * np.maximum(rgb, 0) ** (1 / 2.4) - .055)
        if cutoff is not None and levels and values.shape[0] * values.shape[1] >= 16:
            low, high = 0., 8.
            for _ in range(20):
                scale = (low + high) / 2
                if np.mean(values[..., 3] * scale >= cutoff) < coverage:
                    low = scale
                else:
                    high = scale
            encoded[..., 3] = np.clip(values[..., 3] * high, 0, 1)
        levels.append(np.rint(np.clip(encoded, 0, 1) * 255).astype(np.uint8))
        if values.shape[:2] == (1, 1):
            break
        values = reduced(values)
    return levels


def reflectance_levels(pixels):
    values = (pixels[..., :3].astype(np.float32) / 255) ** 2
    levels = []
    while True:
        rgb = np.rint(np.sqrt(np.clip(values, 0, 1)) * 255).astype(np.uint8)
        levels.append(np.dstack((rgb, np.full(rgb.shape[:2], 255, np.uint8))))
        if values.shape[:2] == (1, 1):
            return levels
        values = reduced(values)


def response_levels(pixels, variances):
    values = pixels.astype(np.float32) / 255
    values[..., 1] **= 4
    levels = []
    while True:
        result = values.copy()
        if variances:
            h, w = result.shape[:2]
            variance = min(variances, key=lambda v: abs(np.log2(v.shape[0] / h)) + abs(np.log2(v.shape[1] / w)))
            if variance.shape != (h, w):
                variance = np.asarray(Image.fromarray(variance).resize((w, h), Image.Resampling.BILINEAR))
            result[..., 1] += variance * .12
        result[..., 1] = np.clip(result[..., 1], 0, 1) ** .25
        levels.append(np.rint(np.clip(result, 0, 1) * 255).astype(np.uint8))
        if values.shape[:2] == (1, 1):
            break
        values = reduced(values)
    return levels
