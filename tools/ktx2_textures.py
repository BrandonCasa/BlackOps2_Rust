import hashlib
import math
import struct


MAGIC = b'\xabKTX 20\xbb\r\n\x1a\n'
DXGI_TO_VK = {2: 109, 10: 97, 11: 91, 28: 37, 29: 43, 49: 16, 54: 76, 56: 70,
              61: 9, 65: 9, 71: 133, 72: 134, 74: 135, 75: 136, 77: 137, 78: 138,
              80: 139, 81: 140, 83: 141, 84: 142, 87: 44, 88: 44, 91: 50, 93: 50,
              95: 143, 96: 144, 98: 145, 99: 146}
VK_TO_DXGI = {v: k for k, v in DXGI_TO_VK.items() if k not in (65, 88, 93)}
SRGB = {43, 50, 134, 136, 138, 146}
TO_LINEAR = {43: 37, 50: 44, 134: 133, 136: 135, 138: 137, 146: 145}
TO_SRGB = {v: k for k, v in TO_LINEAR.items()}
FORMATS = {9: ('R8_UNORM', 1, False), 16: ('R8G8_UNORM', 2, False),
           37: ('R8G8B8A8_UNORM', 4, False), 43: ('R8G8B8A8_UNORM_SRGB', 4, False),
           44: ('B8G8R8A8_UNORM', 4, False), 50: ('B8G8R8A8_UNORM_SRGB', 4, False),
           70: ('R16_UNORM', 2, False), 76: ('R16_FLOAT', 2, False),
           91: ('R16G16B16A16_UNORM', 8, False), 97: ('R16G16B16A16_FLOAT', 8, False),
           109: ('R32G32B32A32_FLOAT', 16, False),
           133: ('BC1_UNORM', 8, True), 134: ('BC1_UNORM_SRGB', 8, True),
           135: ('BC2_UNORM', 16, True), 136: ('BC2_UNORM_SRGB', 16, True),
           137: ('BC3_UNORM', 16, True), 138: ('BC3_UNORM_SRGB', 16, True),
           139: ('BC4_UNORM', 8, True), 140: ('BC4_SNORM', 8, True),
           141: ('BC5_UNORM', 16, True), 142: ('BC5_SNORM', 16, True),
           143: ('BC6H_UF16', 16, True), 144: ('BC6H_SF16', 16, True),
           145: ('BC7_UNORM', 16, True), 146: ('BC7_UNORM_SRGB', 16, True)}


def inspect_ktx2(raw, complete=True):
    if len(raw) < 80 or raw[:12] != MAGIC:
        raise ValueError('Invalid KTX2 identifier/header')
    fmt, type_size, width, height, depth, layers, faces, count, scheme = struct.unpack_from('<9I', raw, 12)
    dfd_offset, dfd_size, kv_offset, kv_size, sgd_offset, sgd_size = struct.unpack_from('<4I2Q', raw, 48)
    if fmt not in FORMATS or not width or not height or faces not in (1, 6) or scheme not in (0, 2):
        raise ValueError('Unsupported KTX2 format, dimensions, faces or supercompression')
    extent = max(width, height, depth)
    if extent > 16384 or not count or count > extent.bit_length() or (complete and count != extent.bit_length()):
        raise ValueError('Invalid KTX2 extent or incomplete mip chain')
    if faces == 6 and (width != height or depth):
        raise ValueError('Invalid KTX2 cubemap dimensions')
    sections = [(0, 80 + count*24)]
    for offset, size in ((dfd_offset, dfd_size), (kv_offset, kv_size), (sgd_offset, sgd_size)):
        if size:
            if offset % 4 or offset < 80+count*24 or offset+size > len(raw):
                raise ValueError('Invalid KTX2 section extent/alignment')
            sections.append((offset, offset+size))
    if dfd_size < 28 or struct.unpack_from('<I', raw, dfd_offset)[0] != dfd_size:
        raise ValueError('Invalid KTX2 data format descriptor')
    vendor, version, block_size = struct.unpack_from('<IHH', raw, dfd_offset+4)
    if vendor != 0 or version != 2 or block_size != dfd_size-4:
        raise ValueError('Unsupported KTX2 DFD block')
    transfer, alpha_flags = raw[dfd_offset+14:dfd_offset+16]
    if transfer != (2 if fmt in SRGB else 1) or alpha_flags & ~1:
        raise ValueError('KTX2 DFD transfer/alpha flags differ from VkFormat')
    _, stride, blocked = FORMATS[fmt]
    expected_type = 4 if fmt == 109 else 2 if fmt in (70, 76, 91, 97) else 1
    if type_size != expected_type:
        raise ValueError('KTX2 type size differs from its format')
    expected_dimensions = bytes((3, 3, 0, 0) if blocked else (0, 0, 0, 0))
    if raw[dfd_offset+16:dfd_offset+20] != expected_dimensions:
        raise ValueError('KTX2 DFD texel block dimensions differ from format')
    if raw[dfd_offset+20] not in (0 if scheme else stride, stride):
        raise ValueError('KTX2 DFD bytes per block differ from format')
    mips = []
    for level in range(count):
        offset, size, unpacked = struct.unpack_from('<3Q', raw, 80+24*level)
        w, h, d = max(1, width >> level), max(1, height >> level), max(1, depth >> level)
        expected = ((w+3)//4)*((h+3)//4)*stride if blocked else w*h*stride
        expected *= d*max(1, layers)*faces
        if offset < 80+count*24 or offset+size > len(raw) or unpacked != expected:
            raise ValueError(f'KTX2 mip {level} extent/payload size mismatch')
        if scheme == 0 and (size != unpacked or offset % math.lcm(stride, 4)):
            raise ValueError(f'KTX2 mip {level} payload/alignment mismatch')
        payload = raw[offset:offset+size]
        if scheme == 2:
            import zstandard
            payload = zstandard.ZstdDecompressor().decompress(payload, max_output_size=unpacked)
        if len(payload) != expected:
            raise ValueError(f'KTX2 mip {level} decompressed payload mismatch')
        sections.append((offset, offset+size))
        mips.append({'level': level, 'width': w, 'height': h, 'depth': d, 'bytes': unpacked,
                     'offset': offset, 'stored_bytes': size, 'sha256': hashlib.sha256(payload).hexdigest()})
    ordered = sorted(sections)
    if any(a[1] > b[0] for a, b in zip(ordered, ordered[1:])):
        raise ValueError('Overlapping KTX2 sections')
    if ordered[-1][1] != len(raw):
        raise ValueError('Trailing KTX2 payload')
    for a, b in zip(ordered, ordered[1:]):
        if any(raw[a[1]:b[0]]):
            raise ValueError('Nonzero KTX2 padding')
    return {'vk_format': fmt, 'dxgi': VK_TO_DXGI[fmt], 'format': FORMATS[fmt][0], 'width': width,
            'height': height, 'depth': depth, 'layers': layers, 'faces': faces,
            'dimension': 'cube' if faces == 6 else '3d' if depth else '2d',
            'surfaces': max(1, layers)*faces, 'mip_count': count, 'mips': mips,
            'color_space': 'srgb' if fmt in SRGB else 'linear', 'premultiplied': bool(alpha_flags & 1),
            'type_size': type_size, 'supercompression': scheme,
            'sha256': hashlib.sha256(raw).hexdigest()}


def payloads(raw, info=None):
    info = info or inspect_ktx2(raw)
    for mip in info['mips']:
        data = raw[mip['offset']:mip['offset']+mip['stored_bytes']]
        if info['supercompression'] == 2:
            import zstandard
            data = zstandard.ZstdDecompressor().decompress(data, max_output_size=mip['bytes'])
        yield data


def as_dds(raw, info=None):
    info = info or inspect_ktx2(raw)
    if info['dimension'] != '2d' or info['surfaces'] != 1:
        raise ValueError('Decoded material qualification expects one 2D texture')
    fmt, width, height, count = info['dxgi'], info['width'], info['height'], info['mip_count']
    header = [124, 0x2100f, height, width, info['mips'][0]['bytes'], 0, count, *([0]*11),
              32, 4, int.from_bytes(b'DX10', 'little'), 0, 0, 0, 0, 0,
              0x401008 if count > 1 else 0x1000, 0, 0, 0, 0]
    return b'DDS ' + struct.pack('<31I', *header) + struct.pack('<5I', fmt, 3, 0, 1, 0) + b''.join(payloads(raw, info))


def decoded_levels(raw):
    import io
    import numpy as np
    from PIL import Image
    try:
        from .material_textures import inspect_dds
    except ImportError:
        from material_textures import inspect_dds
    if raw.startswith(MAGIC):
        info = inspect_ktx2(raw)
        if info['surfaces'] != 1 or info['dimension'] != '2d':
            raise ValueError('Material decoding requires one 2D image')
        if info['vk_format'] in (9, 16, 37, 43, 44, 50):
            count = FORMATS[info['vk_format']][1]
            result = []
            for mip, part in zip(info['mips'], payloads(raw, info), strict=True):
                pixels = np.frombuffer(part, np.uint8).reshape(mip['height'], mip['width'], count)
                rgba = np.zeros((mip['height'], mip['width'], 4), np.uint8)
                rgba[..., 3] = 255
                rgba[..., :count] = pixels
                if info['vk_format'] in (44, 50):
                    rgba = rgba[..., [2, 1, 0, 3]]
                result.append(rgba)
            return result
        raw = as_dds(raw, info)
    info = inspect_dds(raw)
    header_size = 148 if raw[84:88] == b'DX10' else 128
    cursor = header_size
    result = []
    for mip in info['mips']:
        header = bytearray(raw[:header_size])
        struct.pack_into('<II', header, 12, mip['height'], mip['width'])
        struct.pack_into('<I', header, 28, 1)
        with Image.open(io.BytesIO(header + raw[cursor:cursor+mip['bytes']])) as image:
            image.load()
            result.append(np.asarray(image.convert('RGBA')))
        cursor += mip['bytes']
    return result
