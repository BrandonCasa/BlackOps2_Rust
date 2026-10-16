"""Recontainer staged DDS textures with libktx; never decode or recompress texels."""
import argparse
import ctypes as c
import json
import os
import re
import struct
from pathlib import Path


class CreateInfo(c.Structure):
    _fields_ = [("glInternalformat", c.c_uint32), ("vkFormat", c.c_uint32),
                ("pDfd", c.c_void_p)] + [(name, c.c_uint32) for name in
                ("baseWidth", "baseHeight", "baseDepth", "numDimensions",
                 "numLevels", "numLayers", "numFaces")] + [
                ("isArray", c.c_bool), ("generateMipmaps", c.c_bool)]


# DXGI -> Vulkan format, block width, bytes per block. No format conversion.
FORMATS = {
    28: (37, 1, 4), 29: (43, 1, 4), 49: (16, 1, 2), 61: (9, 1, 1),
    # Bevy exposes legacy DDS A8 as R8Unorm as well.
    65: (9, 1, 1),
    10: (97, 1, 8), 2: (109, 1, 16), 41: (100, 1, 4),
    71: (133, 4, 8), 72: (134, 4, 8),
    74: (135, 4, 16), 75: (136, 4, 16),
    77: (137, 4, 16), 78: (138, 4, 16),
    80: (139, 4, 8), 81: (140, 4, 8),
    83: (141, 4, 16), 84: (142, 4, 16),
    87: (44, 1, 4), 91: (50, 1, 4),
    95: (143, 4, 16), 96: (144, 4, 16),
    98: (145, 4, 16), 99: (146, 4, 16),
}
FOURCC = {b"DXT1": 71, b"DXT3": 74, b"DXT5": 77,
          b"ATI1": 80, b"BC4U": 80, b"BC4S": 81,
          b"ATI2": 83, b"BC5U": 83, b"BC5S": 84}


def dds(data):
    if len(data) < 128 or data[:4] != b"DDS " or struct.unpack_from("<I", data, 4)[0] != 124:
        raise ValueError("Invalid DDS header")
    height, width = struct.unpack_from("<2I", data, 12)
    depth = max(1, struct.unpack_from("<I", data, 24)[0])
    levels = max(1, struct.unpack_from("<I", data, 28)[0])
    caps2 = struct.unpack_from("<I", data, 112)[0]
    faces, layers, header = (6 if caps2 & 0x200 else 1), 1, 128
    is_array = False
    fourcc = data[84:88]
    if fourcc == b"DX10":
        fmt, dimension, misc, layers, alpha = struct.unpack_from("<5I", data, 128)
        if dimension not in (2, 3, 4) or not layers or alpha & 7 not in (0, 1):
            raise ValueError("Unsupported DDS dimensions or alpha mode")
        faces, header, is_array = (6 if misc & 4 else 1), 148, layers > 1
    elif fourcc in FOURCC:
        fmt = FOURCC[fourcc]
    else:
        bits, red, green, blue, alpha = struct.unpack_from("<5I", data, 88)
        masks = (bits, red, green, blue, alpha)
        fmt = {(32, 255, 65280, 16711680, 4278190080): 28,
               (32, 16711680, 65280, 255, 4278190080): 87,
               (8, 255, 0, 0, 0): 61, (8, 0, 0, 0, 255): 65}.get(masks)
    if fmt not in FORMATS or not width or not height:
        raise ValueError(f"Unsupported DDS format: {fourcc!r}, DXGI {fmt}")
    if faces == 6 and fourcc != b"DX10" and caps2 & 0xFC00 != 0xFC00:
        raise ValueError("Incomplete DDS cubemap")
    vk, block, size = FORMATS[fmt]
    dimensions = 3 if caps2 & 0x200000 or depth > 1 else 2
    info = CreateInfo(0, vk, None, width, height, depth, dimensions,
                      levels, layers, faces, is_array, False)
    images = []
    offset = header
    # DDS is layer/face-major; libktx arranges the KTX mip-major layout.
    for layer in range(layers):
        for face in range(faces):
            for level in range(levels):
                w, h, d = (max(1, value >> level) for value in (width, height, depth))
                length = ((w + block - 1) // block) * ((h + block - 1) // block) * d * size
                payload = data[offset:offset + length]
                if len(payload) != length:
                    raise ValueError("Truncated DDS mip")
                images.append((level, layer, face, payload))
                offset += length
    if offset != len(data):
        raise ValueError(f"DDS payload size mismatch: {offset} != {len(data)}")
    return info, images


class Ktx:
    def __init__(self, library):
        self.dll = c.CDLL(str(library.resolve()))
        for name, args, result in (
            ("ktxTexture2_Create", [c.POINTER(CreateInfo), c.c_int, c.POINTER(c.c_void_p)], c.c_int),
            ("ktxTexture_GetData", [c.c_void_p], c.c_void_p),
            ("ktxTexture_GetDataSize", [c.c_void_p], c.c_size_t),
            ("ktxTexture2_GetImageOffset", [c.c_void_p, c.c_uint32, c.c_uint32, c.c_uint32, c.POINTER(c.c_size_t)], c.c_int),
            ("ktxTexture2_WriteToNamedFile", [c.c_void_p, c.c_char_p], c.c_int),
            ("ktxTexture2_Destroy", [c.c_void_p], None),
        ):
            function = getattr(self.dll, name)
            function.argtypes, function.restype = args, result

    def write(self, info, images, destination):
        texture = c.c_void_p()
        check(self.dll.ktxTexture2_Create(c.byref(info), 1, c.byref(texture)))
        try:
            base = self.dll.ktxTexture_GetData(texture)
            capacity = self.dll.ktxTexture_GetDataSize(texture)
            for level, layer, face, payload in images:
                offset = c.c_size_t()
                check(self.dll.ktxTexture2_GetImageOffset(texture, level, layer, face, c.byref(offset)))
                if offset.value + len(payload) > capacity:
                    raise ValueError("KTX image exceeds allocated storage")
                c.memmove(base + offset.value, payload, len(payload))
            check(self.dll.ktxTexture2_WriteToNamedFile(texture, os.fsencode(destination)))
        finally:
            self.dll.ktxTexture2_Destroy(texture)


def check(code):
    if code:
        raise ValueError(f"libktx error {code}")


def verify(output, info, images):
    header = struct.unpack_from("<9I", output, 12)
    type_size = {97: 2, 109: 4, 100: 4}.get(info.vkFormat, 1)
    expected = (info.vkFormat, type_size, info.baseWidth, info.baseHeight,
                info.baseDepth if info.numDimensions == 3 else 0,
                info.numLayers if info.isArray else 0, info.numFaces, info.numLevels, 0)
    if output[:12] != b"\xabKTX 20\xbb\r\n\x1a\n" or header != expected:
        raise ValueError("KTX dimensions/format changed")
    for level in range(info.numLevels):
        offset, length, raw_length = struct.unpack_from("<3Q", output, 80 + level * 24)
        original = b"".join(payload for mip, _, _, payload in images if mip == level)
        if length != raw_length or output[offset:offset + length] != original:
            raise ValueError(f"KTX mip {level} bytes changed")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--assets", type=Path, required=True)
    parser.add_argument("--library", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    library = Ktx(args.library)
    count = total = 0
    for source in sorted((args.assets / "images").glob("*.dds")):
        try:
            info, images = dds(source.read_bytes())
            destination = source.with_suffix(".ktx2")
            temporary = destination.with_suffix(".ktx2.tmp")
            if destination.is_file():
                try:
                    verify(destination.read_bytes(), info, images)
                except ValueError:
                    library.write(info, images, temporary)
                    verify(temporary.read_bytes(), info, images)
                    os.replace(temporary, destination)
            else:
                library.write(info, images, temporary)
                verify(temporary.read_bytes(), info, images)
                os.replace(temporary, destination)
            count += 1
            total += sum(len(payload) for _, _, _, payload in images)
            if count % 500 == 0:
                print(f"Verified {count} KTX2 textures", flush=True)
        except Exception as error:
            raise RuntimeError(f"{source}: {error}") from error
    if not count:
        raise ValueError("No staged DDS textures")
    # Source extraction stays DDS. Only the runtime definition uses KTX2 paths.
    definition = args.assets / "game.json"
    original = definition.read_text(encoding="utf-8")
    text = re.sub(r'("path"\s*:\s*"images/[^"\n]*)\.dds"', r'\1.ktx2"', original)
    document = json.loads(text)
    for image in document.get("content", {}).get("ui_assets", []):
        if not (args.assets / image["path"]).is_file():
            raise ValueError(f"Missing runtime image: {image['path']}")
    if text != original:
        definition.write_text(text, encoding="utf-8")
    args.report.write_text(json.dumps({"textures": count, "verified_payload_bytes": total,
                                      "recompressed": 0}, indent=2) + "\n", encoding="utf-8")
    print(f"Verified {count} KTX2 textures; {total} payload bytes unchanged")


if __name__ == "__main__":
    main()
