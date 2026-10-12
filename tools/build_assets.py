# /// script
# requires-python = ">=3.13"
# dependencies = ["ijson==3.4.0.post0", "numpy==2.3.3", "pillow==11.3.0"]
# ///
from __future__ import annotations

import argparse
import base64
import collections
import concurrent.futures
import dataclasses
import decimal
import hashlib
import inspect
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import urllib.request
import urllib.parse
import zipfile
import wave
import zlib

import ijson
import numpy as np
from PIL import Image


OAT_REVISION = "068d4cd9f513ff0509b757498712b2ec3dee9463"
OAT_REPOSITORY = "https://github.com/Laupetin/OpenAssetTools.git"
PREMAKE_URL = "https://github.com/premake/premake-core/releases/download/v5.0.0-beta8/premake-5.0.0-beta8-windows.zip"
PREMAKE_SHA256 = "2301e3e23ff3074cb83a5ea6103d68c7ea81dad56b786807c84b0643cddea31b"
ZONE_ORDER = ("code_pre_gfx_zm", "code_post_gfx_zm", "common_zm", "weapons", "zm_tomb", "patch_zm", "zm_tomb_patch")
NATIVE_VERTEX_DECLARATIONS = json.loads(zlib.decompress(base64.b85decode('c-rmVUvJtv7zXg~lIvdhe++ls%C<@!OdZlD9aWLz*c(zBCIVBZY2SSTI!Z_(J3weS=edL6H&2YsuaQ;5&mgy7-MxvvyGcG8XF-=<1n$Av+0?|Ti}J6ANfmSh7keFZ#f6lXF_$oDIOB%Lh8Q6^)z*?EA)Qcl65%X~4fk<uY?u(rB(=ds;AfegE;0f_1XDaAYe%1=w9rgw87pmtkwjV_CX6^wL(YWYMumnFV~cD?6BX)&F{^Z}td}+<$`>b1&5!YH8ZG-^l4RoYd=YC^Q9?FbtfGxnia4b_^HhY1bTYKkiP*$KSi%Wop-mK}-g}xbZMlxAVM-P6i1JR)dzlySW;Pw!=fl0%MVrLg)J)90+r=yY{9Hc}bV)G+sk?XCbopXFo=vP<9A7?+GIL)%4?YY&UJtJa{o;(~yBVePz@R&w%-lsV{x(0ENrSF8X<i%{W#5Ya=IMC4I8*#%>Y}mtxm!L@9=?nsKc3v1;yL?u@f1rpV_y$$2Db-iQneYX*HhVM>ah3u^<Z#&6_JCZ*=B#KT94+N(fz^g`^%fdWA$c_#d@r$^!T#>w$~q)b02ed24}N0tr=mmo-DV%*R=U?wKbeIA0BQE=go)f=jT~8yuIvyd{aE$oa#+nM5p#5PA~^(g|=5Wz5Z47sdxSMT~i-`yZIHA;H7}~#PD))5_rWC;5|P0U>EFy-Gw+{7woo<-M@%tbrVa&T$tNJb2-e#8p9FI<?LkUR-)C;cN*vd-Iky$fG%!y&+|sN)_X32F3@cux)SIDU7-6fqiM~YE{`$9g}6`RDu@elAuhx{r2As$rwiOZaT|hLa0_ma%q_)|0&Z}(h^rwk#D%yJ_mJ*u+~EEtw;gZ`Zow_MZ4tM7TmfY}FQF}aAh7!k+CtlwXnO?c%Cie=32pNho6g`ZyoI-o-B?1~HEpri4BA3lXxkob_qhah7ApzZV+Lzs?SIEwivLPz^}Q{&m%&?j3vU~{v4pm3+G1ZBw1u|NwmsVJaS4o^#Y(~%?kH>GPhXSTR+hE)_`aGZ`iyVI#TO`GUExLbefbS$l%Ey9O@sqr01SZtp8&u7elvB7r3QAKr3UB=eW5S(J(j+m^V9e27a#Um>V>}07y6#NzIXE<N6V+9tn}k(s{>1QAAe!3OztR4+{0rhN)@zJv7%#Zdsb@yN7t>*`L5=)(p;39U-do=-|afPEH(f0QP@?XYAULi6xLL*N`<be;FXF_O@*iq8>uOg)e>4$qN*jVrX;ME@R|}`EfF;(ooY$l$kS(LdhOU(2cOoBKCKQvts8$@odC2v0~VO9ELlzFyI;Qnll|SM')))
KTX2_MAGIC = b"\xabKTX 20\xbb\r\n\x1a\n"


class AssetError(RuntimeError):
    pass


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        state = hashlib.sha256()
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            state.update(block)
        return state.hexdigest()


def pool_assets(path: Path):
    def native_numbers(value):
        if isinstance(value, decimal.Decimal):
            return float(value)
        if isinstance(value, list):
            return [native_numbers(item) for item in value]
        if isinstance(value, dict):
            return {key: native_numbers(item) for key, item in value.items()}
        return value
    with path.open("rb") as stream:
        for asset in ijson.items(stream, "assets.item", use_float=False):
            yield native_numbers(asset)


def bytes_digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write_json(path: Path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    data = json.dumps(value, ensure_ascii=False, allow_nan=False, separators=(",", ":")) + "\n"
    path.write_text(data, encoding="utf-8", newline="\n")


def inside(root: Path, relative: str) -> Path:
    name = PurePosixPath(relative.replace("\\", "/"))
    if name.is_absolute() or any(p in {"", ".", ".."} or ":" in p for p in name.parts):
        raise AssetError(f"Unsafe source/asset path: {relative}")
    result = root.joinpath(*name.parts).resolve()
    if not result.is_relative_to(root.resolve()):
        raise AssetError(f"Path escapes its root: {relative}")
    return result


def checked_hash(path: Path, expected: str, label: str) -> str:
    if not path.is_file():
        raise AssetError(f"Missing {label}: {path}")
    actual = digest(path)
    if actual != expected:
        raise AssetError(f"Changed {label}: {path}; expected {expected}, got {actual}")
    return actual


def run(arguments: list[str], *, cwd: Path | None = None, log: Path | None = None):
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        with log.open("wb") as stream:
            result = subprocess.run(arguments, cwd=cwd, stdout=stream, stderr=subprocess.STDOUT)
    else:
        result = subprocess.run(arguments, cwd=cwd)
    if result.returncode:
        raise AssetError(f"Command failed ({result.returncode}): {arguments[0]}; log: {log}")


@dataclasses.dataclass(frozen=True)
class IwiImage:
    width: int
    height: int
    depth: int
    faces: int
    format: int
    flags: int
    gamma: float
    gloss: tuple[int, ...]
    levels: tuple[bytes, ...]


IWI_FORMATS = {
    1: (37, 43, 1, 1, 4, 1),
    3: (16, 22, 1, 1, 2, 1),
    4: (9, 15, 1, 1, 1, 1),
    5: (9, 15, 1, 1, 1, 1),
    11: (133, 134, 4, 4, 8, 128),
    12: (135, 136, 4, 4, 16, 129),
    13: (137, 138, 4, 4, 16, 130),
    14: (141, 141, 4, 4, 16, 132),
    19: (97, 97, 1, 1, 8, 1),
}


def read_iwi(path: Path) -> IwiImage:
    data = path.read_bytes()
    if len(data) < 64 or data[:4] != b"IWi\x1b":
        raise AssetError(f"Expected original T6 IWI v27, never DDS: {path}")
    image_format, flags, width, height, depth, gamma = struct.unpack_from("<BBHHHf", data, 4)
    if image_format not in IWI_FORMATS:
        raise AssetError(f"Unsupported native IWI encoding {image_format}: {path}")
    if not width or not height or depth > 1 and not flags & 8:
        raise AssetError(f"Invalid IWI dimensions: {path}")
    faces = 6 if flags & 4 else 1
    if faces == 6 and (width != height or depth > 1):
        raise AssetError(f"Invalid native cubemap dimensions: {path}")
    level_count = 1 if flags & 2 else max(width, height, depth).bit_length()
    sizes = struct.unpack_from("<8I", data, 32)
    _, _, block_width, block_height, block_size, _ = IWI_FORMATS[image_format]
    levels = [b""] * level_count
    cursor = 64
    for level in reversed(range(level_count)):
        size = math.ceil(max(1, width >> level) / block_width) * math.ceil(max(1, height >> level) / block_height) * max(1, depth >> level) * faces * block_size
        end = cursor + size
        if end > len(data):
            raise AssetError(f"Truncated original IWI mip {level}: {path}")
        levels[level] = data[cursor:end]
        if level < 8 and sizes[level] not in {0, end}:
            raise AssetError(f"Native IWI picmip offset disagrees at level {level}: {path} ({sizes[level]} != {end})")
        cursor = end
    if cursor != len(data):
        raise AssetError(f"Unexpected bytes after native IWI mip chain: {path} ({len(data) - cursor})")
    return IwiImage(width, height, depth, faces, image_format, flags, gamma, tuple(data[16:32]), tuple(levels))


def dfd(image_format: int, srgb: bool) -> bytes:
    _, _, width, height, size, model = IWI_FORMATS[image_format]
    samples = []
    if model == 1:
        channels = 4 if image_format in {1, 19} else 3 if image_format == 2 else 2 if image_format == 3 else 1
        bits = 16 if image_format == 19 else 8
        for channel in range(channels):
            channel_type = 15 if channel == 3 or image_format == 5 else channel
            if image_format == 3 and channel == 1:
                channel_type = 15
            qualifier = 0xC0 if image_format == 19 else 0x10 if srgb and channel_type == 15 else 0
            low, high = (0xBF800000, 0x3F800000) if image_format == 19 else (0, (1 << bits) - 1)
            samples.append(struct.pack("<HBB4BII", channel * bits, bits - 1, channel_type | qualifier, 0, 0, 0, 0, low, high))
    else:
        channels = [(0, size * 8, 1)] if image_format == 11 else [(0, 64, 15), (64, 64, 0)] if image_format in {12, 13} else [(0, 64, 0), (64, 64, 1)]
        for offset, bits, channel in channels:
            samples.append(struct.pack("<HBB4BII", offset, bits - 1, channel | (0x10 if srgb and channel == 15 else 0), 0, 0, 0, 0, 0, 0xFFFFFFFF))
    descriptor = struct.pack("<IHH4B4B8B", 0, 2, 24 + len(samples) * 16, model, 1, 2 if srgb else 1, 0, width - 1, height - 1, 0, 0, size, 0, 0, 0, 0, 0, 0, 0) + b"".join(samples)
    return struct.pack("<I", len(descriptor) + 4) + descriptor


def write_ktx2(path: Path, image: IwiImage, *, srgb: bool):
    if image.format in {3, 4, 5}:
        converted = []
        for level in image.levels:
            expanded = bytearray()
            if image.format == 3:
                for luminance, alpha in zip(level[::2], level[1::2]):
                    expanded.extend((luminance, luminance, luminance, alpha))
            elif image.format == 4:
                for luminance in level:
                    expanded.extend((luminance, luminance, luminance, 255))
            else:
                for alpha in level:
                    expanded.extend((255, 255, 255, alpha))
            converted.append(bytes(expanded))
        image = dataclasses.replace(image, format=1, levels=tuple(converted))
    vk_linear, vk_srgb, _, _, block_size, _ = IWI_FORMATS[image.format]
    descriptor = dfd(image.format, srgb)
    level_count = len(image.levels)
    descriptor_offset = 80 + 24 * level_count
    key = b"KTXorientation\0rd\0"
    key_value = struct.pack("<I", len(key)) + key + b"\0" * (-len(key) % 4)
    key_offset = descriptor_offset + len(descriptor)
    offset = key_offset + len(key_value)
    alignment = math.lcm(4, block_size)
    layout = [None] * level_count
    payload = bytearray()
    for level in reversed(range(level_count)):
        padding = -offset % alignment
        payload.extend(b"\0" * padding)
        offset += padding
        data = image.levels[level]
        layout[level] = (offset, len(data), len(data))
        payload.extend(data)
        offset += len(data)
    header = KTX2_MAGIC + struct.pack("<13I2Q", vk_srgb if srgb else vk_linear, 2 if image.format == 19 else 1, image.width, image.height, image.depth if image.flags & 8 else 0, 0, image.faces, level_count, 0, descriptor_offset, len(descriptor), key_offset, len(key_value), 0, 0)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as stream:
        stream.write(header)
        for entry in layout:
            stream.write(struct.pack("<3Q", *entry))
        stream.write(descriptor)
        stream.write(key_value)
        stream.write(payload)
    validate_ktx2(path)


def validate_ktx2(path: Path) -> dict:
    data = path.read_bytes()
    if data[:12] != KTX2_MAGIC or len(data) < 104:
        raise AssetError(f"Invalid KTX2 header: {path}")
    values = struct.unpack_from("<13I2Q", data, 12)
    vk, type_size, width, height, depth, layers, faces, levels, compression, descriptor_offset, descriptor_size, kv_offset, kv_size, global_offset, global_size = values
    if not width or not height or faces not in {1, 6} or levels == 0 or layers or compression:
        raise AssetError(f"Unsupported generated KTX2 layout: {path}")
    if descriptor_offset != 80 + levels * 24 or descriptor_size < 44:
        raise AssetError(f"Missing generated KTX2 format descriptor: {path}")
    if descriptor_offset + descriptor_size > len(data) or kv_offset + kv_size > len(data):
        raise AssetError(f"Out-of-bounds KTX2 metadata: {path}")
    if struct.unpack_from("<I", data, descriptor_offset)[0] != descriptor_size:
        raise AssetError(f"Invalid KTX2 format descriptor size: {path}")
    known = {fmt[0]: fmt for fmt in IWI_FORMATS.values()} | {fmt[1]: fmt for fmt in IWI_FORMATS.values()}
    if vk not in known:
        raise AssetError(f"Unrecognized generated Vulkan texture format {vk}: {path}")
    _, _, bw, bh, bs, _ = known[vk]
    spans = []
    for level in range(levels):
        offset, size, raw = struct.unpack_from("<3Q", data, 80 + level * 24)
        expected = math.ceil(max(1, width >> level) / bw) * math.ceil(max(1, height >> level) / bh) * max(1, depth >> level) * faces * bs
        if size != expected or raw != size or offset < kv_offset + kv_size or offset + size > len(data) or offset % math.lcm(4, bs):
            raise AssetError(f"Invalid KTX2 mip {level}: {path}")
        spans.append((offset, offset + size))
    if any(a[1] > b[0] for a, b in zip(sorted(spans), sorted(spans)[1:])):
        raise AssetError(f"Overlapping KTX2 mip data: {path}")
    return {"vk_format": vk, "width": width, "height": height, "depth": depth, "faces": faces, "levels": levels, "sha256": digest(path)}


NATIVE_POOL_EXPORTER = r'''
#pragma once
#include "Dumping/IAssetDumper.h"
#include "Game/T6/T6.h"
#include "Game/IGame.h"
#include <nlohmann/json.hpp>
#include <cstring>
#include <stdexcept>
#include <type_traits>
#include <iostream>

namespace origins_native {
using J = nlohmann::json;
using namespace T6;
template<class T, size_t N> J array(const T (&a)[N]) { J j=J::array(); for(const auto& v:a) j.push_back(v); return j; }
inline J array(const vec2_t& a){return array(a.v);}
inline J array(const vec3_t& a){return array(a.v);}
inline J array(const vec4_t& a){return array(a.v);}
inline J string(const char* p) { return p ? J(p) : J(nullptr); }
template<class T> J range(const T& r) { return J{{"base",r.base},{"amplitude",r.amplitude}}; }
inline J vector_range(const FxElemVec3Range& r) { return J{{"base",array(r.base)},{"amplitude",array(r.amplitude)}}; }
inline J velocity(const FxElemVelStateInFrame& v) { return J{{"velocity",vector_range(v.velocity)},{"total_delta",vector_range(v.totalDelta)}}; }
inline J visual_state(const FxElemVisualState& v) { J c=J::array(); for(auto b:v.color)c.push_back(static_cast<unsigned char>(b)); return J{{"color_rgba8",c},{"rotation_delta",v.rotationDelta},{"rotation_total",v.rotationTotal},{"size",array(v.size)},{"scale",v.scale}}; }
inline void binary(AssetDumpingContext& c,const std::string& name,const void* p,size_t size) {
    if(size && !p) throw std::runtime_error("Native null payload: "+name);
    auto s=c.OpenAssetFile(name); if(!s)throw std::runtime_error("Cannot write "+name);
    s->write(static_cast<const char*>(p),static_cast<std::streamsize>(size));
}
inline J plane(const cplane_s* p) { return p ? J{{"normal",array(p->normal)},{"distance",p->dist},{"type",p->type},{"sign_bits",p->signbits}}:J(nullptr); }
inline J clip_info(const ClipInfo& c) {
    J j{{"planes",J::array()},{"materials",J::array()},{"brushes",J::array()},{"leaf_brush_nodes",J::array()},{"leaf_brushes",J::array()},{"brush_vertices",J::array()},{"uinds",J::array()}};
    for(int i=0;i<c.planeCount;i++)j["planes"].push_back(plane(&c.planes[i]));
    for(unsigned i=0;i<c.numMaterials;i++){const auto& m=c.materials[i];j["materials"].push_back(J{{"name",string(m.name)},{"surface_flags",m.surfaceFlags},{"content_flags",m.contentFlags}});}
    for(unsigned i=0;i<c.numBrushes;i++){
        const auto& b=c.brushes[i];J sides=J::array(),verts=J::array();
        for(unsigned k=0;k<b.numsides;k++)sides.push_back(J{{"plane",plane(b.sides[k].plane)},{"content_flags",b.sides[k].cflags},{"surface_flags",b.sides[k].sflags}});
        for(unsigned k=0;k<b.numverts;k++)verts.push_back(array(b.verts[k]));
        J axial_c=J::array(),axial_s=J::array();for(int k=0;k<2;k++){axial_c.push_back(array(b.axial_cflags[k]));axial_s.push_back(array(b.axial_sflags[k]));}
        j["brushes"].push_back(J{{"mins",array(b.mins)},{"maxs",array(b.maxs)},{"contents",b.contents},{"sides",sides},{"vertices",verts},{"axial_content_flags",axial_c},{"axial_surface_flags",axial_s}});
    }
    for(unsigned i=0;i<c.leafbrushNodesCount;i++){
        const auto& n=c.leafbrushNodes[i];J v{{"axis",n.axis},{"leaf_brush_count",n.leafBrushCount},{"contents",n.contents}};
        if(n.leafBrushCount>0){v["brushes"]=J::array();for(int k=0;k<n.leafBrushCount;k++)v["brushes"].push_back(n.data.leaf.brushes[k]);}
        else v["children"]=J{{"distance",n.data.children.dist},{"range",n.data.children.range},{"offsets",array(n.data.children.childOffset)}};
        j["leaf_brush_nodes"].push_back(v);
    }
    for(unsigned i=0;i<c.numLeafBrushes;i++)j["leaf_brushes"].push_back(c.leafbrushes[i]);
    for(unsigned i=0;i<c.numBrushVerts;i++)j["brush_vertices"].push_back(array(c.brushVerts[i]));
    for(unsigned i=0;i<c.nuinds;i++)j["uinds"].push_back(c.uinds[i]);
    return j;
}
inline J clip(AssetDumpingContext& ctx,const clipMap_t& c,const std::string& id) {
    J j{{"name",string(c.name)},{"checksum",c.checksum},{"info",clip_info(c.info)},{"terrain_vertices",J::array()},{"terrain_triangles",J::array()},{"submodels",J::array()},{"static_models",J::array()},{"triggers",J::array()}};
    for(unsigned i=0;i<c.vertCount;i++)j["terrain_vertices"].push_back(array(c.verts[i]));
    for(int i=0;i<c.triCount;i++)j["terrain_triangles"].push_back(array(c.triIndices[i]));
    for(unsigned i=0;i<c.numSubModels;i++){const auto& m=c.cmodels[i];j["submodels"].push_back(J{{"mins",array(m.mins)},{"maxs",array(m.maxs)},{"radius",m.radius},{"leaf_brush_node",m.leaf.leafBrushNode},{"brush_contents",m.leaf.brushContents},{"terrain_contents",m.leaf.terrainContents}});}
    for(unsigned i=0;i<c.numStaticModels;i++){const auto& m=c.staticModelList[i];J axis=J::array();for(auto& a:m.invScaledAxis)axis.push_back(array(a));j["static_models"].push_back(J{{"model",m.xmodel?string(m.xmodel->name):J(nullptr)},{"contents",m.contents},{"origin",array(m.origin)},{"inverse_scaled_axis",axis},{"mins",array(m.absmin)},{"maxs",array(m.absmax)}});}
    if(c.mapEnts){const auto& t=c.mapEnts->trigger;for(unsigned i=0;i<t.count;i++){const auto& m=t.models[i];J hulls=J::array();for(unsigned k=0;k<m.hullCount;k++){const auto& h=t.hulls[m.firstHull+k];J slabs=J::array();for(unsigned n=0;n<h.slabCount;n++){const auto& s=t.slabs[h.firstSlab+n];slabs.push_back(J{{"direction",array(s.dir)},{"midpoint",s.midPoint},{"half_size",s.halfSize}});}hulls.push_back(J{{"midpoint",array(h.bounds.midPoint)},{"half_size",array(h.bounds.halfSize)},{"contents",h.contents},{"slabs",slabs}});}j["triggers"].push_back(J{{"contents",m.contents},{"hulls",hulls}});}}
    return j;
}
inline J world(AssetDumpingContext& ctx,const GfxWorld& w,const std::string& id) {
    std::string p="native-binary/world-"+id;
    binary(ctx,p+"-stream0.bin",w.draw.vd0.data,w.draw.vertexDataSize0);
    binary(ctx,p+"-stream1.bin",w.draw.vd1.data,w.draw.vertexDataSize1);
    binary(ctx,p+"-indices.u16",w.draw.indices,w.draw.indexCount*sizeof(uint16_t));
    J j{{"name",string(w.name)},{"base_name",string(w.baseName)},{"checksum",w.checksum},{"mins",array(w.mins)},{"maxs",array(w.maxs)},{"skybox_model",string(w.skyBoxModel)},{"vertex_count",w.draw.vertexCount},{"stream0",p+"-stream0.bin"},{"stream1",p+"-stream1.bin"},{"indices",p+"-indices.u16"},{"surfaces",J::array()},{"static_models",J::array()},{"hero_lights",J::array()}};
    for(int i=0;i<w.surfaceCount;i++){const auto& s=w.dpvs.surfaces[i];J bounds=J::array();bounds.push_back(array(s.bounds[0]));bounds.push_back(array(s.bounds[1]));j["surfaces"].push_back(J{{"material",s.material?string(s.material->info.name):J(nullptr)},{"stream0_offset",s.tris.vertexDataOffset0},{"stream1_offset",s.tris.vertexDataOffset1},{"first_vertex",s.tris.firstVertex},{"vertex_count",s.tris.vertexCount},{"triangle_count",s.tris.triCount},{"base_index",s.tris.baseIndex},{"mins",array(s.tris.mins)},{"maxs",array(s.tris.maxs)},{"lightmap",s.lightmapIndex},{"reflection_probe",s.reflectionProbeIndex},{"primary_light",s.primaryLightIndex},{"flags",s.flags},{"bounds",bounds}});}
    for(unsigned i=0;i<w.dpvs.smodelCount;i++){const auto& s=w.dpvs.smodelDrawInsts[i];J axis=J::array();for(auto& a:s.placement.axis)axis.push_back(array(a));j["static_models"].push_back(J{{"model",s.model?string(s.model->name):J(nullptr)},{"origin",array(s.placement.origin)},{"axis",axis},{"scale",s.placement.scale},{"flags",s.flags},{"id",s.smid},{"primary_light",s.primaryLightIndex},{"reflection_probe",s.reflectionProbeIndex}});}
    for(unsigned i=0;i<w.heroLightCount;i++){const auto& l=w.heroLights[i];j["hero_lights"].push_back(J{{"type",l.type},{"color",array(l.color)},{"direction",array(l.dir)},{"origin",array(l.origin)},{"radius",l.radius},{"cos_half_fov_outer",l.cosHalfFovOuter},{"cos_half_fov_inner",l.cosHalfFovInner},{"exponent",l.exponent}});}
    const auto& s=w.sunParse.initWorldSun[0];const auto& f=w.sunParse.initWorldFog[0];
    j["initial_sun"]=J{{"control",s.control},{"angles",array(s.angles)},{"ambient_color",array(s.ambientColor)},{"diffuse",array(s.sunCd)},{"specular",array(s.sunCs)},{"sky_color",array(s.skyColor)},{"exposure",s.exposure}};
    j["initial_fog"]=J{{"base_distance",f.baseDist},{"half_distance",f.halfDist},{"base_height",f.baseHeight},{"half_height",f.halfHeight},{"sun_pitch",f.sunFogPitch},{"sun_yaw",f.sunFogYaw},{"sun_inner",f.sunFogInner},{"sun_outer",f.sunFogOuter},{"color",array(f.fogColor)},{"opacity",f.fogOpacity},{"sun_color",array(f.sunFogColor)},{"sun_opacity",f.sunFogOpacity},{"transition_time",w.sunParse.fogTransitionTime}};
    return j;
}
inline J shader_argument(const MaterialShaderArgument& a) {
    J j{{"type",a.type},{"location",a.location.offset},{"size",a.size},{"buffer",a.buffer}};
    switch(a.type){
    case MTL_ARG_LITERAL_VERTEX_CONST:case MTL_ARG_LITERAL_PIXEL_CONST:if(a.u.literalConst)j["literal"]=array(*a.u.literalConst);break;
    case MTL_ARG_CODE_VERTEX_CONST:case MTL_ARG_CODE_PIXEL_CONST:j["code_constant"]=J{{"index",a.u.codeConst.index},{"first_row",a.u.codeConst.firstRow},{"row_count",a.u.codeConst.rowCount}};break;
    case MTL_ARG_CODE_PIXEL_SAMPLER:j["code_sampler"]=a.u.codeSampler;break;
    default:j["name_hash"]=a.u.nameHash;break;
    }
    return j;
}
inline J material(const Material& m) {
    J j{{"name",string(m.info.name)},{"technique_set",m.techniqueSet?string(m.techniqueSet->name):J(nullptr)},{"state_flags",m.stateFlags},{"state_entries",array(m.stateBitsEntry)},{"textures",J::array()},{"constants",J::array()},{"state_bits",J::array()},{"techniques",J::array()}};
    for(unsigned i=0;i<m.textureCount;i++){const auto& t=m.textureTable[i];unsigned char sampler=0;std::memcpy(&sampler,&t.samplerState,1);j["textures"].push_back(J{{"name_hash",t.nameHash},{"name_start",t.nameStart},{"name_end",t.nameEnd},{"sampler",sampler},{"semantic",t.semantic},{"image",t.image?string(t.image->name):J(nullptr)},{"mature",t.isMatureContent}});}
    for(unsigned i=0;i<m.constantCount;i++){const auto& c=m.constantTable[i];j["constants"].push_back(J{{"name_hash",c.nameHash},{"name",std::string(c.name,strnlen(c.name,12))},{"literal",array(c.literal)}});}
    for(unsigned i=0;i<m.stateBitsCount;i++)j["state_bits"].push_back(array(m.stateBitsTable[i].loadBits.raw));
    if(m.techniqueSet){j["world_vertex_format"]=m.techniqueSet->worldVertFormat;for(unsigned i=0;i<std::extent_v<decltype(m.techniqueSet->techniques)>;i++){auto* t=m.techniqueSet->techniques[i];if(!t)continue;J passes=J::array();for(unsigned k=0;k<t->passCount;k++){const auto& p=t->passArray[k];J routing=J::array(),args=J::array();if(p.vertexDecl)for(unsigned v=0;v<p.vertexDecl->streamCount;v++)routing.push_back(J{{"source",p.vertexDecl->routing.data[v].source},{"destination",p.vertexDecl->routing.data[v].dest}});for(unsigned v=0;v<unsigned(p.perPrimArgCount)+p.perObjArgCount+p.stableArgCount;v++)args.push_back(shader_argument(p.args[v]));passes.push_back(J{{"vertex_shader",p.vertexShader?string(p.vertexShader->name):J(nullptr)},{"pixel_shader",p.pixelShader?string(p.pixelShader->name):J(nullptr)},{"material_type",p.materialType},{"routing",routing},{"arguments",args},{"per_primitive_argument_count",p.perPrimArgCount},{"per_object_argument_count",p.perObjArgCount},{"stable_argument_count",p.stableArgCount}});}j["techniques"].push_back(J{{"index",i},{"name",string(t->name)},{"flags",t->flags},{"passes",passes}});}}
    return j;
}
inline J model(AssetDumpingContext& ctx,const XModel& m,const std::string& id) {
    J j{{"name",string(m.name)},{"root_bones",m.numRootBones},{"lod_ramp_type",m.lodRampType},{"contents",m.contents},{"radius",m.radius},{"mins",array(m.mins)},{"maxs",array(m.maxs)},{"flags",m.flags},{"collision_lod",m.collLod},{"lighting_origin_offset",array(m.lightingOriginOffset)},{"lighting_origin_range",m.lightingOriginRange},{"physics_preset",m.physPreset?string(m.physPreset->name):J(nullptr)},{"physics_constraints",m.physConstraints?string(m.physConstraints->name):J(nullptr)},{"bones",J::array()},{"surfaces",J::array()},{"lods",J::array()},{"collision_surfaces",J::array()}};
    for(unsigned i=0;i<m.numBones;i++){
        const auto& a=m.baseMat[i];const auto& b=m.boneInfo[i];J bounds=J::array();bounds.push_back(array(b.bounds[0]));bounds.push_back(array(b.bounds[1]));
        J bone{{"name",ctx.m_zone.m_script_strings.Value(m.boneNames[i])},{"classification",m.partClassification?J(m.partClassification[i]):J(nullptr)},{"bind_rotation",array(a.quat)},{"bind_translation",array(a.trans)},{"bind_translation_weight",a.transWeight},{"bounds",bounds},{"bounds_offset",array(b.offset)},{"radius_squared",b.radiusSquared},{"collision_map",b.collmap}};
        if(i>=m.numRootBones){unsigned k=i-m.numRootBones;bone["parent_delta"]=m.parentList[k];bone["local_rotation_snorm16"]=array(m.quats[k].v);bone["local_translation"]=J::array({m.trans[k*3],m.trans[k*3+1],m.trans[k*3+2]});}else bone["parent_delta"]=nullptr;
        j["bones"].push_back(bone);
    }
    for(unsigned i=0;i<m.numsurfs;i++){
        const auto& s=m.surfs[i];J verts=J::array(),tris=J::array(),rigid=J::array(),blend=J::array(),tension=J::array();
        for(unsigned k=0;k<s.vertCount;k++){const auto& v=s.verts0[k];verts.push_back(J{{"position",array(v.xyz)},{"binormal_sign",v.binormalSign},{"color_rgba8",v.color.packed},{"texcoord_half2",v.texCoord.packed},{"normal_101010",v.normal.packed},{"tangent_101010",v.tangent.packed}});if(s.vertInfo.tensionData)tension.push_back(s.vertInfo.tensionData[k]);}
        for(unsigned k=0;k<s.triCount;k++)tris.push_back(array(s.triIndices[k].i));
        for(unsigned k=0;k<s.vertListCount;k++){const auto& r=s.vertList[k];rigid.push_back(J{{"bone_offset",r.boneOffset},{"vertex_count",r.vertCount},{"triangle_offset",r.triOffset},{"triangle_count",r.triCount}});}
        if(s.vertInfo.vertsBlend){unsigned count=s.vertInfo.vertCount[0]+s.vertInfo.vertCount[1]*3+s.vertInfo.vertCount[2]*5+s.vertInfo.vertCount[3]*7;for(unsigned k=0;k<count;k++)blend.push_back(s.vertInfo.vertsBlend[k]);}
        j["surfaces"].push_back(J{{"material",m.materialHandles[i]?string(m.materialHandles[i]->info.name):J(nullptr)},{"tile_mode",s.tileMode},{"flags",s.flags},{"base_vertex_index",s.baseVertIndex},{"part_bits",array(s.partBits)},{"vertices",verts},{"triangles",tris},{"rigid_vertex_groups",rigid},{"blend_vertex_counts",array(s.vertInfo.vertCount)},{"blend_data_u16",blend},{"tension",tension}});
    }
    for(unsigned i=0;i<m.numLods;i++){const auto& l=m.lodInfo[i];j["lods"].push_back(J{{"distance",l.dist},{"surface_count",l.numsurfs},{"surface_index",l.surfIndex},{"part_bits",array(l.partBits)}});}
    for(int i=0;i<m.numCollSurfs;i++){const auto& c=m.collSurfs[i];J tris=J::array();for(int k=0;k<c.numCollTris;k++){const auto& t=c.collTris[k];tris.push_back(J{{"plane",array(t.plane)},{"s_vector",array(t.svec)},{"t_vector",array(t.tvec)}});}j["collision_surfaces"].push_back(J{{"bone",c.boneIdx},{"contents",c.contents},{"surface_flags",c.surfFlags},{"mins",array(c.mins)},{"maxs",array(c.maxs)},{"triangles",tris}});}
    return j;
}
inline J animation(AssetDumpingContext& ctx,const XAnimParts& a,const std::string& id) {
    J names=J::array(),notifications=J::array(),data=J::object();
    for(unsigned i=0;i<a.boneCount[PART_TYPE_ALL];i++)names.push_back(ctx.m_zone.m_script_strings.Value(a.names[i]));
    for(unsigned i=0;i<a.notifyCount;i++)notifications.push_back(J{{"name",ctx.m_zone.m_script_strings.Value(a.notify[i].name)},{"normalized_time",a.notify[i].time}});
    auto payload=[&](const char* name,const void* p,size_t count,size_t item_size){std::string path="native-binary/animation-"+id+"-"+name+".bin";binary(ctx,path,p,count*item_size);data[name]=J{{"path",path},{"count",count},{"item_size",item_size}};};
    payload("data_byte",a.dataByte,a.dataByteCount,1);payload("data_short",a.dataShort,a.dataShortCount,2);payload("data_int",a.dataInt,a.dataIntCount,4);payload("random_byte",a.randomDataByte,a.randomDataByteCount,1);payload("random_short",a.randomDataShort,a.randomDataShortCount,2);payload("random_int",a.randomDataInt,a.randomDataIntCount,4);payload("indices",a.indices.data,a.indexCount,a.numframes<256?1:2);
    J j{{"name",string(a.name)},{"frame_count",a.numframes},{"loop",a.bLoop},{"delta",a.bDelta},{"delta_3d",a.bDelta3D},{"left_hand_grip_ik",a.bLeftHandGripIK},{"streamed_file_size",a.streamedFileSize},{"bone_channel_counts",array(a.boneCount)},{"asset_type",a.assetType},{"default",a.isDefault},{"framerate",a.framerate},{"frequency",a.frequency},{"primed_length",a.primedLength},{"loop_entry_time",a.loopEntryTime},{"bone_names",names},{"notifications",notifications},{"compressed_channels",data}};
    if(a.deltaPart){const auto& d=*a.deltaPart;J delta=J::object();auto index=[&](unsigned count,const void* p){J values=J::array();for(unsigned i=0;i<count;i++)values.push_back(a.numframes<256?static_cast<const unsigned char*>(p)[i]:static_cast<const uint16_t*>(p)[i]);return values;};
        if(d.trans){const auto& t=*d.trans;J v{{"size",t.size},{"small",t.smallTrans}};if(!t.size)v["frame0"]=array(t.u.frame0);else{v["mins"]=array(t.u.frames.mins);v["size_per_axis"]=array(t.u.frames.size);v["indices"]=index(t.size+1,&t.u.frames.indices);v["frames"]=J::array();for(unsigned k=0;k<=t.size;k++)v["frames"].push_back(t.smallTrans?array(t.u.frames.frames._1[k]):array(t.u.frames.frames._2[k]));}delta["translation"]=v;}
        if(d.quat2){const auto& q=*d.quat2;J v{{"size",q.size}};if(!q.size)v["frame0"]=array(q.u.frame0.value);else{v["indices"]=index(q.size+1,&q.u.frames.indices);v["frames"]=J::array();for(unsigned k=0;k<=q.size;k++)v["frames"].push_back(array(q.u.frames.frames[k].value));}delta["rotation_2"]=v;}
        if(d.quat){const auto& q=*d.quat;J v{{"size",q.size}};if(!q.size)v["frame0"]=array(q.u.frame0.value);else{v["indices"]=index(q.size+1,&q.u.frames.indices);v["frames"]=J::array();for(unsigned k=0;k<=q.size;k++)v["frames"].push_back(array(q.u.frames.frames[k].value));}delta["rotation_4"]=v;}
        j["root_motion"]=delta;
    }
    return j;
}
inline J fx_visual(const FxElemVisuals& v,FxElemType type) {
    if(!v.anonymous)return nullptr;
    if(type==FX_ELEM_TYPE_MODEL)return J{{"model",string(v.model->name)}};
    if(type==FX_ELEM_TYPE_SOUND)return J{{"sound",string(v.soundName)}};
    if(type==FX_ELEM_TYPE_RUNNER)return J{{"effect",string(v.effectDef.name)}};
    if(type==FX_ELEM_TYPE_OMNI_LIGHT)return J{{"type","omni_light"}};
    if(type==FX_ELEM_TYPE_SPOT_LIGHT)return J{{"light",string(v.lightDef->name)}};
    return J{{"material",string(v.material->info.name)}};
}
inline J effect(const FxEffectDef& f) {
    J j{{"name",string(f.name)},{"flags",f.flags},{"priority",f.efPriority},{"looping_count",f.elemDefCountLooping},{"one_shot_count",f.elemDefCountOneShot},{"emission_count",f.elemDefCountEmission},{"looping_life_ms",f.msecLoopingLife},{"non_looping_life_ms",f.msecNonLoopingLife},{"bounds_dimensions",array(f.boundingBoxDim)},{"bounds_center",array(f.boundingBoxCentre)},{"occlusion_depth_bias",f.occlusionQueryDepthBias},{"occlusion_fade_in",f.occlusionQueryFadeIn},{"occlusion_fade_out",f.occlusionQueryFadeOut},{"occlusion_scale",range(f.occlusionQueryScaleRange)},{"elements",J::array()}};
    for(int i=0;i<f.elemDefCountLooping+f.elemDefCountOneShot+f.elemDefCountEmission;i++){
        const auto& e=f.elemDefs[i];J origin=J::array(),angles=J::array(),angular=J::array(),vel=J::array(),vis=J::array(),visuals=J::array();
        for(int k=0;k<3;k++){origin.push_back(range(e.spawnOrigin[k]));angles.push_back(range(e.spawnAngles[k]));angular.push_back(range(e.angularVelocity[k]));}
        if(e.velSamples)for(int k=0;k<=static_cast<unsigned char>(e.velIntervalCount);k++)vel.push_back(J{{"local",velocity(e.velSamples[k].local)},{"world",velocity(e.velSamples[k].world)}});
        if(e.visSamples)for(int k=0;k<=static_cast<unsigned char>(e.visStateIntervalCount);k++)vis.push_back(J{{"base",visual_state(e.visSamples[k].base)},{"amplitude",visual_state(e.visSamples[k].amplitude)}});
        unsigned count=static_cast<unsigned char>(e.visualCount);
        if(e.elemType==FX_ELEM_TYPE_DECAL){if(e.visuals.markArray)for(unsigned k=0;k<count;k++){J a=J::array();for(auto* m:e.visuals.markArray[k].materials)a.push_back(m?string(m->info.name):J(nullptr));visuals.push_back(J{{"decal_materials",a}});}}
        else if(count==1)visuals.push_back(fx_visual(e.visuals.instance,e.elemType));
        else if(e.visuals.array)for(unsigned k=0;k<count;k++)visuals.push_back(fx_visual(e.visuals.array[k],e.elemType));
        auto effect_name=[](const FxEffectDefRef& r){return string(r.name);};
        J value{{"type",e.elemType},{"flags",e.flags},{"spawn",i<f.elemDefCountLooping?J{{"interval_ms",e.spawn.looping.intervalMsec},{"count",e.spawn.looping.count}}:J{{"count",range(e.spawn.oneShot.count)}}},{"spawn_range",range(e.spawnRange)},{"fade_in_range",range(e.fadeInRange)},{"fade_out_range",range(e.fadeOutRange)},{"frustum_cull_radius",e.spawnFrustumCullRadius},{"delay_ms",range(e.spawnDelayMsec)},{"life_ms",range(e.lifeSpanMsec)},{"origin",origin},{"offset_radius",range(e.spawnOffsetRadius)},{"offset_height",range(e.spawnOffsetHeight)},{"angles",angles},{"angular_velocity",angular},{"initial_rotation",range(e.initialRotation)},{"rotation_axis",e.rotationAxis},{"gravity",range(e.gravity)},{"reflection",range(e.reflectionFactor)},{"atlas",J{{"behavior",e.atlas.behavior},{"index",e.atlas.index},{"fps",e.atlas.fps},{"loop_count",e.atlas.loopCount},{"column_bits",e.atlas.colIndexBits},{"row_bits",e.atlas.rowIndexBits},{"entry_count_and_range",e.atlas.entryCountAndIndexRange}}},{"wind_influence",e.windInfluence},{"velocity_samples",vel},{"visual_samples",vis},{"visuals",visuals},{"collision_mins",array(e.collMins)},{"collision_maxs",array(e.collMaxs)},{"effect_on_impact",effect_name(e.effectOnImpact)},{"effect_on_death",effect_name(e.effectOnDeath)},{"effect_emitted",effect_name(e.effectEmitted)},{"effect_attached",effect_name(e.effectAttached)},{"emit_distance",range(e.emitDist)},{"emit_distance_variance",range(e.emitDistVariance)},{"sort_order",e.sortOrder},{"lighting_fraction",e.lightingFrac},{"alpha_fade_ms",e.alphaFadeTimeMsec},{"max_wind_strength",e.maxWindStrength},{"interval_at_max_wind",e.spawnIntervalAtMaxWind},{"life_at_max_wind",e.lifespanAtMaxWind},{"spawn_sound",string(e.spawnSound.spawnSound)},{"billboard_pivot",array(e.billboardPivot)}};
        if(e.elemType==FX_ELEM_TYPE_CLOUD)value["cloud_density"]=range(e.u.cloudDensityRange);
        else value["billboard_trim"]=J{{"top_width",e.u.billboard.topWidth},{"bottom_width",e.u.billboard.bottomWidth}};
        if(e.extended.spotLightDef&&e.elemType==FX_ELEM_TYPE_SPOT_LIGHT)value["spot_light"]=J{{"inner_fov_fraction",e.extended.spotLightDef->fovInnerFraction},{"start_radius",e.extended.spotLightDef->startRadius},{"end_radius",e.extended.spotLightDef->endRadius}};
        if(e.extended.trailDef&&e.elemType==FX_ELEM_TYPE_TRAIL){const auto& t=*e.extended.trailDef;J verts=J::array(),inds=J::array();for(int k=0;k<t.vertCount;k++)verts.push_back(J{{"position",array(t.verts[k].pos)},{"normal",array(t.verts[k].normal)},{"texcoord",t.verts[k].texCoord}});for(int k=0;k<t.indCount;k++)inds.push_back(t.inds[k]);value["trail"]=J{{"scroll_ms",t.scrollTimeMsec},{"repeat_distance",t.repeatDist},{"split_distance",t.splitDist},{"vertices",verts},{"indices",inds}};}
        j["elements"].push_back(value);
    }
    return j;
}

inline J sound_bank(AssetDumpingContext& ctx,const SndBank& b,const std::string& id){
    J lists=J::array(),reverbs=J::array(),ducks=J::array();
    for(unsigned i=0;i<b.aliasCount;i++){const auto& l=b.alias[i];J aliases=J::array();for(int k=0;k<l.count;k++){const auto& a=l.head[k];uint32_t flags[2];std::memcpy(flags,&a.flags,sizeof(flags));J v{{"name",string(a.name)},{"subtitle",string(a.subtitle)},{"secondary",string(a.secondaryName)},{"asset_file_name",string(a.assetFileName)},{"flags",array(flags)},{"id",a.id},{"assetId",a.assetId},{"duck",a.duck},{"contextType",a.contextType},{"contextValue",a.contextValue},{"stopOnPlay",a.stopOnPlay},{"futzPatch",a.futzPatch},{"fluxTime",a.fluxTime},{"startDelay",a.startDelay},{"reverbSend",a.reverbSend},{"centerSend",a.centerSend},{"volMin",a.volMin},{"volMax",a.volMax},{"pitchMin",a.pitchMin},{"pitchMax",a.pitchMax},{"distMin",a.distMin},{"distMax",a.distMax},{"distReverbMax",a.distReverbMax},{"envelopMin",a.envelopMin},{"envelopMax",a.envelopMax},{"envelopPercentage",a.envelopPercentage},{"fadeIn",a.fadeIn},{"fadeOut",a.fadeOut},{"dopplerScale",a.dopplerScale},{"minPriorityThreshold",a.minPriorityThreshold},{"maxPriorityThreshold",a.maxPriorityThreshold},{"probability",a.probability},{"occlusionLevel",a.occlusionLevel},{"minPriority",a.minPriority},{"maxPriority",a.maxPriority},{"pan",a.pan},{"limitCount",a.limitCount},{"entityLimitCount",a.entityLimitCount},{"duckGroup",a.duckGroup}};aliases.push_back(v);}lists.push_back(J{{"name",string(l.name)},{"id",l.id},{"sequence",l.sequence},{"aliases",aliases}});}
    for(unsigned i=0;i<b.radverbCount;i++){const auto& a=b.radverbs[i];reverbs.push_back(J{{"name",std::string(a.name,strnlen(a.name,32))},{"id",a.id},{"smoothing",a.smoothing},{"earlyTime",a.earlyTime},{"lateTime",a.lateTime},{"earlyGain",a.earlyGain},{"lateGain",a.lateGain},{"returnGain",a.returnGain},{"earlyLpf",a.earlyLpf},{"lateLpf",a.lateLpf},{"inputLpf",a.inputLpf},{"dampLpf",a.dampLpf},{"wallReflect",a.wallReflect},{"dryGain",a.dryGain},{"earlySize",a.earlySize},{"lateSize",a.lateSize},{"diffusion",a.diffusion},{"returnHighpass",a.returnHighpass}});}
    for(unsigned i=0;i<b.duckCount;i++){const auto& a=b.ducks[i];J attenuation=J::array(),filter=J::array();if(a.attenuation)for(int k=0;k<32;k++)attenuation.push_back(a.attenuation[k]);if(a.filter)for(int k=0;k<32;k++)filter.push_back(a.filter[k]);ducks.push_back(J{{"name",std::string(a.name,strnlen(a.name,32))},{"attenuation",attenuation},{"filter",filter},{"id",a.id},{"fadeIn",a.fadeIn},{"fadeOut",a.fadeOut},{"startDelay",a.startDelay},{"distance",a.distance},{"length",a.length},{"fadeInCurve",a.fadeInCurve},{"fadeOutCurve",a.fadeOutCurve},{"updateWhilePaused",a.updateWhilePaused}});}
    const auto& l=b.loadedAssets;J loaded=J::array();for(unsigned i=0;i<l.entryCount;i++){const auto& e=l.entries[i];loaded.push_back(J{{"id",e.id},{"offset",e.offset},{"size",e.size},{"frame_count",e.frameCount},{"rate_index",e.frameRateIndex},{"channels",e.channelCount},{"loop",e.looping},{"format",e.format}});}
    std::string path="native-binary/soundbank-"+id+"-loaded.bin";binary(ctx,path,l.data,l.dataSize);
    return J{{"name",string(b.name)},{"aliases",lists},{"reverbs",reverbs},{"ducks",ducks},{"stream",J{{"zone",string(b.streamAssetBank.zone)},{"language",string(b.streamAssetBank.language)}}},{"load",J{{"zone",string(b.loadAssetBank.zone)},{"language",string(b.loadAssetBank.language)}}},{"loaded_assets",J{{"zone",string(l.zone)},{"language",string(l.language)},{"data",path},{"data_size",l.dataSize},{"entries",loaded}}}};
}
inline J rect(const rectDef_s& r){return J{{"x",r.x},{"y",r.y},{"w",r.w},{"h",r.h},{"horzAlign",r.horzAlign},{"vertAlign",r.vertAlign}};}
inline J window(const windowDef_t& w){return J{{"name",string(w.name)},{"group",string(w.group)},{"rect",rect(w.rect)},{"client_rect",rect(w.rectClient)},{"foreground",array(w.foreColor)},{"background_color",array(w.backColor)},{"border_color",array(w.borderColor)},{"outline_color",array(w.outlineColor)},{"background_material",w.background?string(w.background->info.name):J(nullptr)},{"style",w.style},{"border",w.border},{"modal",w.modal},{"frameSides",w.frameSides},{"frameTexSize",w.frameTexSize},{"frameSize",w.frameSize},{"ownerDraw",w.ownerDraw},{"ownerDrawFlags",w.ownerDrawFlags},{"borderSize",w.borderSize},{"staticFlags",w.staticFlags},{"rotation",w.rotation}};}
inline J menu(const menuDef_t& m){J items=J::array();for(int i=0;i<m.itemCount;i++){const auto& a=*m.items[i];J v{{"window",window(a.window)},{"type",a.type},{"data_type",a.dataType},{"show_bits",a.showBits},{"hide_bits",a.hideBits}};bool has_text=a.type==ITEM_TYPE_TEXT||a.type==ITEM_TYPE_BUTTON||a.type==ITEM_TYPE_LISTBOX||a.type==ITEM_TYPE_EDITFIELD||a.type==ITEM_TYPE_NUMERICFIELD||a.type==ITEM_TYPE_SLIDER||a.type==ITEM_TYPE_YESNO||a.type==ITEM_TYPE_MULTI||a.type==ITEM_TYPE_DVARENUM||a.type==ITEM_TYPE_BIND||a.type==ITEM_TYPE_VALIDFILEFIELD||a.type==ITEM_TYPE_UPREDITFIELD||a.type==ITEM_TYPE_GAME_MESSAGE_WINDOW||a.type==ITEM_TYPE_BIND2||a.type==ITEM_TYPE_OWNERDRAW_TEXT||a.type==ITEM_TYPE_OD_TEXT_BUTTON||a.type==ITEM_TYPE_PROFILEMULTI;if(has_text&&a.typeData.textDef){const auto& t=*a.typeData.textDef;v["text"]=J{{"value",string(t.text)},{"rect",rect(t.textRect[0])},{"font_color",array(t.fontColor)},{"glow_color",array(t.glowColor)},{"shadow_color",array(t.shadowColor)},{"shadow_offset",array(t.shadowOffset)},{"alignment",t.alignment},{"fontEnum",t.fontEnum},{"itemFlags",t.itemFlags},{"textAlignMode",t.textAlignMode},{"textalignx",t.textalignx},{"textaligny",t.textaligny},{"textscale",t.textscale},{"textpadding",t.textpadding},{"textStyle",t.textStyle},{"fontBlur",t.fontBlur},{"glowSize",t.glowSize},{"shadowSize",t.shadowSize}};}items.push_back(v);}return J{{"window",window(m.window)},{"font",string(m.font)},{"sound",string(m.soundName)},{"focus_color",array(m.focusColor)},{"disabled_color",array(m.disableColor)},{"initial_rect",rect(m.initialRectInfo)},{"items",items},{"fullScreen",m.fullScreen},{"ui3dWindowId",m.ui3dWindowId},{"fontIndex",m.fontIndex},{"fadeCycle",m.fadeCycle},{"priority",m.priority},{"fadeClamp",m.fadeClamp},{"fadeAmount",m.fadeAmount},{"fadeInAmount",m.fadeInAmount},{"blurRadius",m.blurRadius},{"openSlideSpeed",m.openSlideSpeed},{"closeSlideSpeed",m.closeSlideSpeed},{"openSlideDirection",m.openSlideDirection},{"closeSlideDirection",m.closeSlideDirection},{"openFadingTime",m.openFadingTime},{"closeFadingTime",m.closeFadingTime},{"showBits",m.showBits},{"hideBits",m.hideBits},{"imageTrack",m.imageTrack},{"control",m.control}};}

class Dumper final:public IAssetDumper {
public:
    size_t GetProgressTotalCount(AssetDumpingContext& c)const override{return 1;}
    void Dump(AssetDumpingContext& c)override {
        auto s=c.OpenAssetFile("native-pools.json");if(!s)throw std::runtime_error("Cannot write native-pools.json");
        *s<<"{\"schema_version\":1,\"zone\":"<<J(c.m_zone.m_name).dump()<<",\"assets\":[";unsigned index=0;bool first=true;
        auto* game=IGame::GetGameById(c.m_zone.m_game_id);
        for(const auto* a:c.m_zone.m_pools){
            if(a->m_type==ASSET_TYPE_SCRIPTPARSETREE||a->m_type==ASSET_TYPE_GAMEWORLD_SP||a->m_type==ASSET_TYPE_GAMEWORLD_MP)continue;
            const std::string type(game->GetAssetTypeName(a->m_type).value_or("unknown"));
            std::cerr<<"Native art: "<<type<<' '<<a->m_name<<'\n'<<std::flush;
            J deps=J::array(),indirect=J::array(),strings=J::array();
            for(auto* d:a->m_dependencies)deps.push_back(J{{"type",std::string(game->GetAssetTypeName(d->m_type).value_or("unknown"))},{"name",d->m_name}});
            for(const auto& d:a->m_indirect_asset_references)indirect.push_back(J{{"type",std::string(game->GetAssetTypeName(d.m_type).value_or("unknown"))},{"name",d.m_name}});
            for(auto s:a->m_used_script_strings)strings.push_back(c.m_zone.m_script_strings.Value(s));
            J row{{"type",type},{"type_id",a->m_type},{"name",a->m_name},{"reference",a->IsReference()},{"dependencies",deps},{"indirect_references",indirect},{"script_strings",strings}};
            if(!a->IsReference()&&a->m_ptr){
                std::string id=std::to_string(index);
                switch(a->m_type){

                case ASSET_TYPE_SOUND:row["data"]=sound_bank(c,*static_cast<const SndBank*>(a->m_ptr),id);break;
                case ASSET_TYPE_MENU:row["data"]=menu(*static_cast<const menuDef_t*>(a->m_ptr));break;
                case ASSET_TYPE_MENULIST:{const auto& m=*static_cast<const MenuList*>(a->m_ptr);J menus=J::array();for(int i=0;i<m.menuCount;i++)if(m.menus[i])menus.push_back(menu(*m.menus[i]));row["data"]=J{{"name",string(m.name)},{"menus",menus}};break;}
                case ASSET_TYPE_IMAGE:{const auto& m=*static_cast<const GfxImage*>(a->m_ptr);row["data"]=J{{"name",string(m.name)},{"hash",m.hash},{"map_type",m.mapType},{"semantic",m.semantic},{"category",m.category},{"picmip",array(m.picmip.platform)},{"no_picmip",m.noPicmip},{"track",m.track},{"width",m.width},{"height",m.height},{"depth",m.depth},{"level_count",m.levelCount},{"streaming",m.streaming},{"base_size",m.baseSize},{"loaded_size",m.loadedSize},{"skipped_mip_levels",m.skippedMipLevels}};break;}
                case ASSET_TYPE_FONT:{const auto& m=*static_cast<const Font_s*>(a->m_ptr);J glyphs=J::array(),kernings=J::array();for(int i=0;i<m.glyphCount;i++){const auto& g=m.glyphs[i];glyphs.push_back(J{{"codepoint",g.letter},{"x0",g.x0},{"y0",g.y0},{"advance",g.dx},{"width",g.pixelWidth},{"height",g.pixelHeight},{"s0",g.s0},{"t0",g.t0},{"s1",g.s1},{"t1",g.t1}});}for(int i=0;i<m.kerningPairsCount;i++){const auto& k=m.kerningPairs[i];kernings.push_back(J{{"first",k.wFirst},{"second",k.wSecond},{"amount",k.iKernAmount}});}row["data"]=J{{"name",string(m.fontName)},{"pixel_height",m.pixelHeight},{"scaling_allowed",m.isScalingAllowed},{"material",m.material?string(m.material->info.name):J(nullptr)},{"glow_material",m.glowMaterial?string(m.glowMaterial->info.name):J(nullptr)},{"glyphs",glyphs},{"kerning_pairs",kernings}};break;}
                case ASSET_TYPE_IMPACT_FX:{const auto& m=*static_cast<const FxImpactTable*>(a->m_ptr);J rows=J::array();for(int i=0;i<21;i++){J nonflesh=J::array(),flesh=J::array();for(auto* f:m.table[i].nonflesh)nonflesh.push_back(f?string(f->name):J(nullptr));for(auto* f:m.table[i].flesh)flesh.push_back(f?string(f->name):J(nullptr));rows.push_back(J{{"nonflesh",nonflesh},{"flesh",flesh}});}row["data"]=J{{"name",string(m.name)},{"rows",rows}};break;}
                case ASSET_TYPE_FOOTSTEP_TABLE:{const auto& m=*static_cast<const FootstepTableDef*>(a->m_ptr);J rows=J::array();for(auto& r:m.sndAliasTable)rows.push_back(array(r));row["data"]=J{{"name",string(m.name)},{"sound_alias_ids",rows}};break;}
                case ASSET_TYPE_FOOTSTEPFX_TABLE:{const auto& m=*static_cast<const FootstepFXTableDef*>(a->m_ptr);J refs=J::array();for(auto* f:m.footstepFX)refs.push_back(f?string(f->name):J(nullptr));row["data"]=J{{"name",string(m.name)},{"effects",refs}};break;}
                case ASSET_TYPE_XMODELPIECES:{const auto& m=*static_cast<const XModelPieces*>(a->m_ptr);J pieces=J::array();for(int i=0;i<m.numpieces;i++)pieces.push_back(J{{"model",m.pieces[i].model?string(m.pieces[i].model->name):J(nullptr)},{"offset",array(m.pieces[i].offset)}});row["data"]=J{{"name",string(m.name)},{"pieces",pieces}};break;}

                case ASSET_TYPE_GFXWORLD:row["data"]=world(c,*static_cast<const GfxWorld*>(a->m_ptr),id);break;
                case ASSET_TYPE_CLIPMAP:case ASSET_TYPE_CLIPMAP_PVS:row["data"]=clip(c,*static_cast<const clipMap_t*>(a->m_ptr),id);break;
                case ASSET_TYPE_MATERIAL:row["data"]=material(*static_cast<const Material*>(a->m_ptr));break;
                case ASSET_TYPE_XMODEL:row["data"]=model(c,*static_cast<const XModel*>(a->m_ptr),id);break;
                case ASSET_TYPE_XANIMPARTS:row["data"]=animation(c,*static_cast<const XAnimParts*>(a->m_ptr),id);break;
                case ASSET_TYPE_FX:row["data"]=effect(*static_cast<const FxEffectDef*>(a->m_ptr));break;
                case ASSET_TYPE_COMWORLD:{const auto& w=*static_cast<const ComWorld*>(a->m_ptr);J lights=J::array();for(unsigned i=0;i<w.primaryLightCount;i++){const auto& l=w.primaryLights[i];lights.push_back(J{{"type",l.type},{"can_shadow",l.canUseShadowMap},{"exponent",l.exponent},{"priority",l.priority},{"cull_distance",l.cullDist},{"color",array(l.color)},{"direction",array(l.dir)},{"origin",array(l.origin)},{"radius",l.radius},{"outer_fov_cos",l.cosHalfFovOuter},{"inner_fov_cos",l.cosHalfFovInner},{"expanded_fov_cos",l.cosHalfFovExpanded},{"diffuse_color",array(l.diffuseColor)},{"falloff",array(l.falloff)},{"angle",array(l.angle)},{"attenuation",l.dAttenuation},{"roundness",l.roundness},{"definition",string(l.defName)}});}row["data"]=J{{"name",string(w.name)},{"lights",lights}};break;}
                case ASSET_TYPE_GAMEWORLD_SP:case ASSET_TYPE_GAMEWORLD_MP:{const auto& w=*static_cast<const GameWorldMp*>(a->m_ptr);J nodes=J::array();for(unsigned i=0;i<w.path.nodeCount;i++){const auto& n=w.path.nodes[i].constant;J links=J::array();for(unsigned k=0;k<n.totalLinkCount;k++){const auto& l=n.Links[k];links.push_back(J{{"distance",l.fDist},{"node",l.nodeNum},{"disconnect_count",l.disconnectCount},{"negotiation",l.negotiationLink}});}auto str=[&](unsigned s){return c.m_zone.m_script_strings.Value(s);};nodes.push_back(J{{"type",n.type},{"spawn_flags",n.spawnflags},{"origin",array(n.vOrigin)},{"angle",n.fAngle},{"forward",array(n.forward)},{"radius",n.fRadius},{"minimum_use_distance_squared",n.minUseDistSq},{"targetname",str(n.targetname)},{"script_linkname",str(n.script_linkName)},{"script_noteworthy",str(n.script_noteworthy)},{"target",str(n.target)},{"animscript",str(n.animscript)},{"links",links}});}row["data"]=J{{"name",string(w.name)},{"nodes",nodes},{"original_node_count",w.path.originalNodeCount}};break;}
                default:break;
                }
            }
            try{*s<<(first?"":",")<<row.dump();first=false;index++;}catch(const std::exception& e){std::cerr<<"Cannot serialize native art "<<type<<' '<<a->m_name<<": "<<e.what()<<'\n'<<std::flush;throw;}
        }
        *s<<"]}\n";c.IncrementProgress();
    }
};
}
'''


NATIVE_IPAK_EXPORTER = r'''
#pragma once
#include "ObjContainer/IPak/IPak.h"
#include <nlohmann/json.hpp>
#include <filesystem>
#include <fstream>
#include <format>
#include <iterator>
#include <iostream>
#include <stdexcept>
#include <vector>
int OriginsExportIPak(const char* source,const char* output) {
    namespace fs=std::filesystem;using J=nlohmann::json;
    try {
        auto input=std::make_unique<std::ifstream>(source,std::ios::binary);
        if(!*input)throw std::runtime_error("Cannot open original IPak");
        auto pak=IIPak::Create(source,std::move(input));
        if(!pak->Initialize())throw std::runtime_error("Invalid original IPak");
        fs::create_directories(output);J assets=J::array();
        for(const auto& e:pak->GetIndexEntries()){
            auto stream=pak->GetEntryStream(e.key.nameHash,e.key.dataHash);
            if(!stream)throw std::runtime_error("Cannot open native IPak entry");
            std::vector<char> bytes((std::istreambuf_iterator<char>(*stream)),std::istreambuf_iterator<char>());
            if(bytes.size()<4||bytes[0]!='I'||bytes[1]!='W'||bytes[2]!='i')throw std::runtime_error("IPak entry is not a native IWI");
            const auto actual=IIPak::HashData(bytes.data(),bytes.size());
            const auto name=std::format("{:08x}-{:08x}.iwi",e.key.nameHash,e.key.dataHash);
            std::ofstream out(fs::path(output)/name,std::ios::binary);out.write(bytes.data(),bytes.size());
            if(!out)throw std::runtime_error("Cannot write native IPak entry");
            assets.push_back(J{{"name_hash",e.key.nameHash},{"data_hash",e.key.dataHash},{"data_crc32",actual},{"offset",e.offset},{"stored_size",e.size},{"decoded_size",bytes.size()},{"path",name}});
        }
        std::ofstream manifest(fs::path(output)/"ipak-index.json");manifest<<J{{"schema_version",1},{"assets",assets}}.dump()<<'\n';
        return manifest?0:1;
    }catch(const std::exception& e){std::cerr<<e.what()<<'\n';return 1;}
}
'''


def bootstrap_oat(work: Path, source_root: Path | None = None) -> tuple[Path, dict]:
    source = source_root or work / "toolchain" / "OpenAssetTools"
    if source.exists():
        revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
        if revision != OAT_REVISION:
            raise AssetError(f"Unexpected upstream OAT revision in {source}: {revision}")
    else:
        source.parent.mkdir(parents=True, exist_ok=True)
        run(["git", "clone", "--no-checkout", OAT_REPOSITORY, str(source)])
        run(["git", "checkout", "--detach", OAT_REVISION], cwd=source)
        run(["git", "submodule", "update", "--init", "--recursive"], cwd=source)
    writer = source / "src/ObjWriting/Game/T6/ObjWriterT6.cpp"
    original = subprocess.check_output(["git", "show", f"{OAT_REVISION}:src/ObjWriting/Game/T6/ObjWriterT6.cpp"], cwd=source)
    anchor = b"void ObjWriter::RegisterAssetDumpers(AssetDumpingContext& context)\n{"
    if original.count(anchor) != 1:
        raise AssetError("Pinned upstream OAT writer registration changed")
    patched = b'#include "OriginsNativePools.h"\n' + original.replace(anchor, anchor + b"\n    RegisterAssetDumper(std::make_unique<origins_native::Dumper>());")
    patched = patched.replace(b'    RegisterAssetDumper(std::make_unique<script::DumperT6>());\n', b'')
    header = writer.with_name("OriginsNativePools.h")
    main_path = source / "src/UnlinkerCli/main.cpp"
    main_original = subprocess.check_output(["git", "show", f"{OAT_REVISION}:src/UnlinkerCli/main.cpp"], cwd=source)
    main_anchor = b"    con::init();"
    main_patched = b'#include <string>\nint OriginsExportIPak(const char*,const char*);\n' + main_original.replace(main_anchor, main_anchor + b'\n    if(argc==4&&std::string(argv[1])=="--origins-ipak")return OriginsExportIPak(argv[2],argv[3]);')
    ipak_header = main_path.with_name("OriginsIPak.h")
    existing = subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=all"], cwd=source, text=True).splitlines()
    allowed = {"src/ObjWriting/Game/T6/ObjWriterT6.cpp", "src/ObjWriting/Game/T6/OriginsNativePools.h", "src/UnlinkerCli/main.cpp", "src/UnlinkerCli/OriginsIPak.h"}
    for row in existing:
        if row[3:].replace("\\", "/") not in allowed:
            raise AssetError(f"Untrusted changes in fresh OAT checkout: {row}")
    if not writer.is_file() or writer.read_bytes() != patched:
        writer.write_bytes(patched)
    header_bytes = (NATIVE_POOL_EXPORTER + NATIVE_IPAK_EXPORTER).encode()
    if not header.is_file() or header.read_bytes() != header_bytes:
        header.write_bytes(header_bytes)
    if main_path.read_bytes() != main_patched:
        main_path.write_bytes(main_patched)
    if ipak_header.is_file():
        ipak_header.unlink()
    premake = source / "build/premake5.exe"
    if not premake.is_file():
        premake.parent.mkdir(parents=True, exist_ok=True)
        archive = premake.with_suffix(".zip")
        urllib.request.urlretrieve(PREMAKE_URL, archive)
        with zipfile.ZipFile(archive) as package:
            premake.write_bytes(package.read("premake5.exe"))
        archive.unlink()
    checked_hash(premake, PREMAKE_SHA256, "Premake binary")
    run([str(premake), "vs2026"], cwd=source, log=work / "logs/oat-generate.log")
    vswhere = Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Microsoft Visual Studio/Installer/vswhere.exe"
    if not vswhere.is_file():
        raise AssetError("Visual Studio C++ build tools are required to compile the source-derived native exporter")
    build = subprocess.check_output([str(vswhere), "-latest", "-products", "*", "-requires", "Microsoft.Component.MSBuild", "-find", "MSBuild/**/Bin/MSBuild.exe"], text=True).strip().splitlines()
    if not build:
        raise AssetError("MSBuild was not found by vswhere")
    solution = source / "build/OpenAssetTools.slnx"
    if not solution.is_file():
        solution = source / "build/OpenAssetTools.sln"
    run([build[0], str(solution), "/t:Tools\\UnlinkerCli", "/p:Configuration=Release", "/p:Platform=Win32", "/m:2", "/v:minimal", "/nologo"], cwd=source, log=work / "logs/oat-build.log")
    unlinker = source / "build/bin/Release_x86/Unlinker.exe"
    if not unlinker.is_file():
        raise AssetError(f"Fresh native extractor build did not produce {unlinker}")
    return unlinker, {"upstream": OAT_REPOSITORY, "revision": OAT_REVISION, "exporter_sha256": bytes_digest(header_bytes), "writer_sha256": digest(writer), "entrypoint_sha256": digest(main_path), "binary_sha256": digest(unlinker)}


def extraction_receipt(output: Path, source: dict, tool: dict, receipt_file: Path) -> bool:
    if not output.exists():
        return False
    if not receipt_file.is_file():
        raise AssetError(f"Unreceipted native output must be moved aside before retrying: {output}")
    receipt = read_json(receipt_file)
    if receipt["source_sha256"] != source["sha256"] or receipt["exporter_sha256"] != tool["exporter_sha256"] or receipt["upstream_revision"] != tool["revision"]:
        raise AssetError(f"Existing work output came from different original input or extractor: {output}")
    expected = receipt["files"]
    actual_paths = {path.relative_to(output).as_posix() for path in output.rglob("*") if path.is_file()}
    if actual_paths != set(expected):
        raise AssetError(f"Existing extraction file inventory changed: {output}")
    receipt_time = receipt_file.stat().st_mtime_ns
    for relative, entry in expected.items():
        path = output.joinpath(*PurePosixPath(relative).parts)
        if path.is_symlink():
            raise AssetError(f"External link in source-derived extraction: {path}")
        stat = path.stat()
        if isinstance(entry, dict):
            if stat.st_size == entry["size"] and stat.st_mtime_ns == entry["mtime_ns"]:
                continue
            sha = entry["sha256"]
        else:
            if stat.st_mtime_ns <= receipt_time:
                continue
            sha = entry
        checked_hash(path, sha, "receipted original-art extraction")
    return True


def record_extraction(output: Path, source: dict, tool: dict, receipt_file: Path):
    files = {}
    for path in sorted(output.rglob("*")):
        if path.is_file():
            stat = path.stat()
            files[path.relative_to(output).as_posix()] = {"sha256": digest(path), "size": stat.st_size, "mtime_ns": stat.st_mtime_ns}
    write_json(receipt_file, {"source_sha256": source["sha256"], "exporter_sha256": tool["exporter_sha256"], "upstream_revision": tool["revision"], "files": files})


def clump_members(path: Path, install: Path) -> list[dict]:
    size = path.stat().st_size
    container_sha = digest(path)
    members = []
    with path.open("rb") as stream:
        page = 0
        while True:
            stream.seek(page * 0x4000)
            table = stream.read(0x4000)
            if len(table) != 0x4000:
                raise AssetError(f"Truncated original clump index: {path.name}")
            finished = False
            for index in range(1024):
                name_hash, sector, stored_size, reserved = struct.unpack_from("<4I", table, index * 16)
                if name_hash == 0:
                    finished = True
                    break
                offset = sector << 12
                if stored_size < 56 or offset < 0x4000 or offset > size - stored_size:
                    raise AssetError(f"Invalid original clump entry {path.name}/{name_hash:08x}")
                stream.seek(offset)
                payload = stream.read(stored_size)
                magic = payload[:8]
                version = struct.unpack_from("<I", payload, 8)[0]
                if magic not in {b"TAff0100", b"TAffu100", b"TAffx100"} or version != 147:
                    raise AssetError(f"Unsupported art-clump member format {path.name}/{name_hash:08x}")
                native_name = payload[24:56].split(b"\0", 1)[0].decode("utf-8") if payload[12:20] == b"PHEEBs71" else f"{name_hash:08x}"
                members.append({"zone": native_name, "path": path.relative_to(install).as_posix(), "sha256": bytes_digest(payload), "container_sha256": container_sha, "language": path.parent.name, "embedded": {"name_hash": name_hash, "index_page": page, "index": index, "offset": offset, "size": stored_size, "reserved": reserved}})
            if finished:
                break
            page += 1
    return members


def extract_native(install: Path, work: Path, inventory: dict, *, oat_source: Path | None = None) -> dict:
    unlinker, tool = bootstrap_oat(work, oat_source)
    zone_files = list(inventory["zones"])
    zone_files.sort(key=lambda row: (ZONE_ORDER.index(row["zone"]) if row["zone"] in ZONE_ORDER else len(ZONE_ORDER), row["path"]))
    for container in inventory["containers"]:
        if container["kind"] == "clump":
            zone_files.extend(clump_members(inside(install, container["path"]), install))
    fresh = work / "native-extraction"
    fresh.mkdir(parents=True, exist_ok=True)
    clump_data = {}
    for row in zone_files:
        if "embedded" in row and row["path"] not in clump_data:
            original = inside(install, row["path"])
            checked_hash(original, row["container_sha256"], "original art clump")
            clump_data[row["path"]] = original.read_bytes()

    def extract_zone(indexed):
        index, row = indexed
        zone_id = f"{index:03d}-{Path(row['path']).parent.name}-{re.sub('[^a-zA-Z0-9_.-]', '_', row['zone'])}"
        output = fresh / zone_id
        ff = inside(install, row["path"])
        if "embedded" in row:
            payload = clump_data[row["path"]][row["embedded"]["offset"]:row["embedded"]["offset"] + row["embedded"]["size"]]
            ff = work / "clump-fastfiles" / f"{row['embedded']['name_hash']:08x}.ff"
            ff.parent.mkdir(parents=True, exist_ok=True)
            if not ff.is_file():
                ff.write_bytes(payload)
        checked_hash(ff, row["sha256"], "original fastfile before extraction")
        receipt = work / "receipts" / f"{zone_id}.json"
        if not extraction_receipt(output, row, tool, receipt):
            output.mkdir()
            search = [install, install / "sound"] + sorted(path for path in (install / "zone").iterdir() if path.is_dir())
            run([str(unlinker), "--no-color", "--image-format", "IWI", "--model-format", "GLTF", "--exclude-assets", "script,aitype,mptype,mpbody,mphead,character,xmodelalias,gameworldsp,gameworldmp,leaderboard,ddl", "--search-path", ";".join(str(path) for path in search), "-o", str(output), str(ff)], log=work / "logs" / f"{zone_id}.log")
            record_extraction(output, row, tool, receipt)
        pools = list(output.rglob("native-pools.json"))
        if len(pools) != 1:
            raise AssetError(f"Expected one fresh native pool export for {zone_id}, found {len(pools)}")
        log = work / "logs" / f"{zone_id}.log"
        with log.open("rb") as stream:
            asset_count = sum(line.startswith(b"Native art: ") for line in stream)
        receipt_data = read_json(receipt)
        pool_entry = receipt_data["files"][pools[0].relative_to(output).as_posix()]
        pool_sha = pool_entry["sha256"] if isinstance(pool_entry, dict) else pool_entry
        print(f"Extracted {zone_id}: {asset_count} native asset identities", flush=True)
        return {**row, "id": zone_id, "root": str(pools[0].parent), "pool_sha256": pool_sha, "asset_count": asset_count}
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
        results = list(executor.map(extract_zone, enumerate(zone_files)))
    ipaks = []
    for row in inventory["containers"]:
        if row["kind"] != "ipak":
            continue
        output = fresh / "ipak" / PurePosixPath(row["path"]).stem
        original = inside(install, row["path"])
        checked_hash(original, row["sha256"], "original image package")
        receipt = work / "receipts" / f"ipak-{original.stem}.json"
        if not extraction_receipt(output, row, tool, receipt):
            run([str(unlinker), "--origins-ipak", str(original), str(output)], log=work / "logs" / f"ipak-{original.stem}.log")
            record_extraction(output, row, tool, receipt)
        index_file = output / "ipak-index.json"
        index = read_json(index_file)
        ipaks.append({**row, "root": str(output), "index_sha256": digest(index_file), "asset_count": len(index["assets"])})
        print(f"Extracted every native image in {row['path']}: {len(index['assets'])}", flush=True)
    return {"tool": tool, "zones": results, "ipaks": ipaks, "containers": inventory["containers"]}


def installation_zones(install: Path) -> dict:
    zones = []
    zone_root = install / "zone"
    if not zone_root.is_dir():
        raise AssetError("Original installation has no zone directory")
    containers = []
    for directory in sorted(path for path in zone_root.iterdir() if path.is_dir()):
        for path in sorted(directory.glob("*.ff")):
            relative = path.relative_to(install).as_posix()
            row = {"zone": path.stem, "path": relative, "sha256": digest(path), "language": directory.name}
            zones.append(row)
            containers.append({**row, "kind": "fastfile", "size": path.stat().st_size})
        for path in sorted(directory.glob("*.ipak")):
            containers.append({"kind": "ipak", "path": path.relative_to(install).as_posix(), "size": path.stat().st_size, "sha256": digest(path)})
        for path in sorted(directory.glob("*.clump")):
            containers.append({"kind": "clump", "path": path.relative_to(install).as_posix(), "size": path.stat().st_size, "sha256": digest(path)})
    for directory, suffixes, kind in ((install / "sound", {".sabl", ".sabs"}, "soundbank"), (install / "video", {".webm", ".bik", ".bk2"}, "cinematic")):
        if directory.is_dir():
            for path in sorted(directory.rglob("*")):
                if path.is_file() and path.suffix.lower() in suffixes:
                    containers.append({"kind": kind, "path": path.relative_to(install).as_posix(), "size": path.stat().st_size, "sha256": digest(path)})
    for path in sorted(install.rglob("*")):
        if path.is_file() and path.suffix.lower() in {".bmp", ".png", ".jpg", ".jpeg", ".tga", ".iwi"}:
            containers.append({"kind": "loose_image", "path": path.relative_to(install).as_posix(), "size": path.stat().st_size, "sha256": digest(path)})
    if not zones or not any(zone["zone"] == "zm_tomb" for zone in zones):
        raise AssetError("The original installation must contain zone/all/zm_tomb.ff")
    return {"schema_version": 1, "zones": zones, "containers": containers}


def export_sound_bank(path: Path, install: Path, output: Path) -> dict:
    rates = (8000, 12000, 16000, 24000, 32000, 44100, 48000, 96000, 192000)
    size = path.stat().st_size
    with path.open("rb") as stream:
        raw = stream.read(584)
        if len(raw) != 584:
            raise AssetError(f"Truncated native sound bank: {path.name}")
        magic, version, entry_size, checksum_size, dependency_size, count, dependency_count, padding, file_size, entry_offset, checksum_offset, native_checksum, dependency_bytes = struct.unpack("<8I3q16s512s", raw)
        if magic != int.from_bytes(b"2UX#", "little") or version != 14 or entry_size != 20 or checksum_size != 16 or file_size != size:
            raise AssetError(f"Unsupported native sound bank header: {path.name}")
        if dependency_count * dependency_size > 512 or dependency_count and not dependency_size:
            raise AssetError(f"Invalid native sound-bank dependencies: {path.name}")
        if not 584 <= entry_offset <= size - count * 20 or not 584 <= checksum_offset <= size - count * 16:
            raise AssetError(f"Invalid native sound-bank tables: {path.name}")
        dependencies = [dependency_bytes[i * dependency_size:(i + 1) * dependency_size].split(b"\0", 1)[0].decode("utf-8") for i in range(dependency_count)]
        stream.seek(entry_offset)
        table = stream.read(count * 20)
        stream.seek(checksum_offset)
        checksums = stream.read(count * 16)
        assets = []
        for i in range(count):
            identity, stored_size, offset, frames, rate_index, channels, looping, image_format = struct.unpack_from("<4I4B", table, i * 20)
            if not 0 < offset <= size - stored_size or rate_index >= len(rates) or not channels:
                raise AssetError(f"Invalid native audio entry {path.name}/{identity:08x}")
            stream.seek(offset)
            payload = stream.read(stored_size)
            native_md5 = checksums[i * 16:(i + 1) * 16]
            actual_md5 = hashlib.md5(payload).digest()
            if actual_md5 != native_md5:
                raise AssetError(f"Native sound checksum mismatch: {path.name}/{identity:08x}")
            sha = bytes_digest(payload)
            extension = {0: "wav", 1: "wav", 2: "wav", 3: "wav", 5: "mp3", 8: "flac"}.get(image_format, "native-audio")
            target = output / "audio" / f"{sha}.{extension}"
            target.parent.mkdir(parents=True, exist_ok=True)
            if not target.is_file():
                if image_format in {0, 1, 2}:
                    sample_size = (2, 3, 4)[image_format]
                    if len(payload) != frames * channels * sample_size:
                        raise AssetError(f"Native PCM frame-size mismatch: {path.name}/{identity:08x}")
                    with wave.open(str(target), "wb") as wav:
                        wav.setnchannels(channels)
                        wav.setsampwidth(sample_size)
                        wav.setframerate(rates[rate_index])
                        wav.writeframes(payload)
                elif image_format == 3:
                    fmt = struct.pack("<HHIIHH", 3, channels, rates[rate_index], rates[rate_index] * channels * 4, channels * 4, 32)
                    target.write_bytes(b"RIFF" + struct.pack("<I", 36 + len(payload)) + b"WAVEfmt " + struct.pack("<I", 16) + fmt + b"data" + struct.pack("<I", len(payload)) + payload)
                else:
                    if image_format == 8 and not payload.startswith(b"fLaC"):
                        raise AssetError(f"Native FLAC signature mismatch: {path.name}/{identity:08x}")
                    target.write_bytes(payload)
            assets.append({"id": identity, "offset": offset, "stored_size": stored_size, "frame_count": frames, "sample_rate": rates[rate_index], "channel_count": channels, "looping": looping, "native_format": image_format, "native_md5": native_md5.hex(), "payload_sha256": sha, "uri": target.relative_to(output).as_posix(), "prepared": image_format in {0, 1, 2, 3, 5, 8}})
    return {"source": path.relative_to(install).as_posix(), "sha256": digest(path), "version": version, "native_checksum": native_checksum.hex(), "dependencies": dependencies, "assets": assets}


CODE_SUFFIXES = {".gsc", ".csc", ".gsh", ".csh", ".lua", ".luac", ".c", ".cpp", ".h", ".hpp", ".asm"}
NON_ART_TYPES = {"script", "aitype", "mptype", "mpbody", "mphead", "character", "xmodelalias", "gameworldsp", "gameworldmp", "leaderboard", "ddl", "memoryblock", "xglobals"}


def asset_key(zone: str, name: str) -> str:
    return bytes_digest((zone + "\0" + name).encode())[:24]


def reference_path(pack: Path, directory: Path, uri: str) -> Path:
    value = PurePosixPath(urllib.parse.unquote(uri.replace("\\", "/")))
    if value.is_absolute() or any(":" in part for part in value.parts):
        raise AssetError(f"Invalid prepared asset reference: {uri}")
    target = directory.joinpath(*value.parts).resolve()
    if target != pack.resolve() and pack.resolve() not in target.parents:
        raise AssetError(f"Prepared reference escapes pack: {uri}")
    return target


def game_half(value):
    value = np.asarray(value, dtype=np.uint32)
    shifted = value << np.uint32(14)
    bits = ((value << np.uint32(16)) & np.uint32(0x80000000)) | (((((shifted & np.uint32(0x0fffc000)).astype(np.int64) - ((~shifted) & np.uint32(0x10000000))) & 0xffffffff).astype(np.uint32) ^ np.uint32(0x80000000)) >> np.uint32(1))
    return np.where(value == 0, 0.0, bits.view(np.float32)).astype(np.float32)


def packed_normal(value):
    value = np.asarray(value, dtype=np.uint32)
    parts = np.stack([(value >> shift) & 1023 for shift in (0, 10, 20)], axis=-1)
    bits = (parts.astype(np.int64) - 2 * (parts & 512) + 0x40400000).astype(np.uint32)
    return (bits.view(np.float32) - 3.0) * np.float32(8208.0312)


def game_vector(values, *, position=False):
    result = np.asarray(values, dtype=np.float32)[..., [0, 2, 1]].copy()
    result[..., 2] *= -1
    if position:
        result *= 0.0254
    return result


def game_quaternion(values):
    values = np.asarray(values, dtype=np.float32)
    result = values[..., [0, 2, 1, 3]].copy()
    result[..., 2] *= -1
    norm = np.linalg.norm(result, axis=-1, keepdims=True)
    if np.any(norm < 1e-8):
        raise AssetError("Native zero quaternion cannot be converted")
    return result / norm


class GltfDocument:
    def __init__(self, name):
        self.data = {"asset": {"version": "2.0", "generator": "Origins original-art builder"}, "scene": 0, "scenes": [{"nodes": []}], "nodes": [], "meshes": [], "materials": [], "accessors": [], "bufferViews": [], "buffers": [{"byteLength": 0}], "extras": {"native_name": name, "units": "meters"}}
        self.binary = bytearray()

    def accessor(self, values, kind, *, target=None, bounds=False):
        values = np.ascontiguousarray(values)
        components = {np.dtype("<f4"): 5126, np.dtype("<u4"): 5125, np.dtype("<u2"): 5123, np.dtype("u1"): 5121}
        if values.dtype not in components:
            values = values.astype("<f4")
        self.binary.extend(b"\0" * (-len(self.binary) % 4))
        view = {"buffer": 0, "byteOffset": len(self.binary), "byteLength": values.nbytes}
        if target:
            view["target"] = target
        self.data["bufferViews"].append(view)
        self.binary.extend(values.tobytes())
        item = {"bufferView": len(self.data["bufferViews"]) - 1, "componentType": components[values.dtype], "count": len(values), "type": kind}
        if bounds and len(values):
            item["min"] = np.atleast_1d(values.min(axis=0)).tolist()
            item["max"] = np.atleast_1d(values.max(axis=0)).tolist()
        self.data["accessors"].append(item)
        return len(self.data["accessors"]) - 1

    def save(self, path):
        path.parent.mkdir(parents=True, exist_ok=True)
        self.data["buffers"][0].update(byteLength=len(self.binary), uri=path.with_suffix(".bin").name)
        path.with_suffix(".bin").write_bytes(self.binary)
        write_json(path, self.data)


def animation_curves(data: dict, root: Path) -> dict:
    pools = {}
    offsets = collections.Counter()
    formats = {"data_byte": "B", "data_short": "h", "data_int": "I", "random_byte": "B", "random_short": "h", "random_int": "I", "indices": "B" if data["frame_count"] < 256 else "H"}
    for name, item in data["compressed_channels"].items():
        raw = inside(root, item["path"]).read_bytes()
        if len(raw) != item["count"] * item["item_size"]:
            raise AssetError(f"Native animation pool size mismatch: {data['name']}/{name}")
        pools[name] = np.frombuffer(raw, dtype=np.dtype("<" + formats[name]))

    def take(name, count=1):
        start = offsets[name]
        if start + count > len(pools[name]):
            raise AssetError(f"Native animation pool exhausted: {data['name']}/{name}")
        offsets[name] += count
        return pools[name][start:start + count].copy()

    def indices(size):
        if data["frame_count"] < 256:
            return take("data_byte", size + 1).astype(np.uint16)
        if size >= 64:
            result = take("indices", size + 1)
            take("data_short", (size - 1) // 256 + 2)
            return result
        return take("data_short", size + 1).view(np.uint16)

    tracks = [{"name": name} for name in data["bone_names"]]
    counts = data["bone_channel_counts"]
    if sum(counts[:5]) != len(tracks) or sum(counts[5:9]) != len(tracks):
        raise AssetError(f"Native animation bone channel counts mismatch: {data['name']}")
    bone = 0
    for kind in range(5):
        for unused in range(counts[kind]):
            track = tracks[bone]
            bone += 1
            if kind == 0:
                continue
            width = 2 if kind in {1, 3} else 4
            if kind in {1, 2}:
                size = int(take("data_short")[0]) & 65535
                frames = indices(size)
                values = take("random_short", (size + 1) * width).reshape(-1, width)
            else:
                frames = np.array([0], dtype=np.uint16)
                values = take("data_short", width).reshape(1, width)
            values = values.astype(np.float32) / 32767.0
            if width == 2:
                values = np.column_stack((np.zeros((len(values), 2), dtype=np.float32), values))
            track["rotation"] = {"frames": frames.tolist(), "values": game_quaternion(values).tolist()}
    assigned = set()
    for kind in range(5, 9):
        for unused in range(counts[kind]):
            bone = int(take("data_byte")[0])
            if bone >= len(tracks) or bone in assigned:
                raise AssetError(f"Native animation translation bone assignment mismatch: {data['name']}")
            assigned.add(bone)
            if kind == 8:
                continue
            if kind == 7:
                values = take("data_int", 3).view(np.float32).reshape(1, 3)
                frames = np.array([0], dtype=np.uint16)
            else:
                size = int(take("data_short")[0]) & 65535
                minimum = take("data_int", 3).view(np.float32)
                scale = take("data_int", 3).view(np.float32)
                frames = indices(size)
                values = take("random_byte" if kind == 5 else "random_short", (size + 1) * 3)
                if kind == 6:
                    values = values.view(np.uint16)
                values = minimum + values.reshape(-1, 3).astype(np.float32) * scale
            tracks[bone]["translation"] = {"frames": frames.tolist(), "values": game_vector(values, position=True).tolist()}
    for name in ("data_byte", "data_short", "data_int", "random_byte", "random_short", "indices"):
        if offsets[name] != len(pools[name]):
            raise AssetError(f"Unconsumed native animation channel data: {data['name']}/{name}")
    return {"name": data["name"], "frame_count": data["frame_count"], "framerate": data["framerate"], "tracks": tracks, "loop": data["loop"], "notifications": data["notifications"], "root_motion": data.get("root_motion")}


def prepare_animation(data, root, path):
    curves = animation_curves(data, root)
    document = GltfDocument(data["name"])
    animation = {"name": data["name"], "samplers": [], "channels": [], "extras": {key: curves[key] for key in ("loop", "framerate", "frame_count", "notifications", "root_motion")}}
    if curves["framerate"] <= 0:
        raise AssetError(f"Native animation has invalid framerate: {data['name']}")
    for track in curves["tracks"]:
        node = len(document.data["nodes"])
        document.data["nodes"].append({"name": track["name"]})
        document.data["scenes"][0]["nodes"].append(node)
        for kind in ("rotation", "translation"):
            if kind not in track:
                continue
            channel = track[kind]
            times = np.asarray(channel["frames"], dtype="<f4") / curves["framerate"]
            if np.any(np.diff(times) <= 0) or (len(times) and times[-1] > curves["frame_count"] / curves["framerate"] + 1e-5):
                raise AssetError(f"Native animation has invalid keyframe indices: {data['name']}")
            animation["samplers"].append({"input": document.accessor(times, "SCALAR", bounds=True), "output": document.accessor(np.asarray(channel["values"], dtype="<f4"), "VEC4" if kind == "rotation" else "VEC3"), "interpolation": "LINEAR"})
            animation["channels"].append({"sampler": len(animation["samplers"]) - 1, "target": {"node": node, "path": kind}})
    document.data["animations"] = [animation]
    document.save(path)
    write_json(path.with_suffix(".native.json"), curves)
    return {"loop": curves["loop"], "framerate": curves["framerate"], "notifications": curves["notifications"], "root_motion": curves["root_motion"]}


def vertex_attributes(document, positions, normals, uv, colors, tangents=None):
    attributes = {"POSITION": document.accessor(game_vector(positions, position=True), "VEC3", target=34962, bounds=True), "NORMAL": document.accessor(game_vector(normals), "VEC3", target=34962), "TEXCOORD_0": document.accessor(np.asarray(uv, dtype="<f4"), "VEC2", target=34962), "COLOR_0": document.accessor(np.asarray(colors, dtype="<f4"), "VEC4", target=34962)}
    if tangents is not None:
        tangent = np.asarray(tangents, dtype="<f4").copy()
        tangent[:, :3] = game_vector(tangent[:, :3])
        attributes["TANGENT"] = document.accessor(tangent, "VEC4", target=34962)
    return attributes


def prepare_model(data, path, material_lookup, lod_index):
    document = GltfDocument(data["name"])
    roots = []
    for index, bone in enumerate(data["bones"]):
        parent = index - bone["parent_delta"] if bone["parent_delta"] is not None else None
        rotation = bone["bind_rotation"] if parent is None else np.asarray(bone["local_rotation_snorm16"], dtype=np.float32) / 32767.0
        translation = bone["bind_translation"] if parent is None else bone["local_translation"]
        node = {"name": bone["name"], "rotation": game_quaternion(rotation).tolist(), "translation": game_vector(translation, position=True).tolist()}
        document.data["nodes"].append(node)
        if parent is None:
            roots.append(index)
        elif 0 <= parent < index:
            document.data["nodes"][parent].setdefault("children", []).append(index)
        else:
            raise AssetError(f"Native model parent index invalid: {data['name']}/{bone['name']}")
    if data["bones"]:
        matrices = []
        for bone in data["bones"]:
            x, y, z, w = game_quaternion(bone["bind_rotation"])
            matrix = np.array([[1-2*(y*y+z*z), 2*(x*y-z*w), 2*(x*z+y*w), 0], [2*(x*y+z*w), 1-2*(x*x+z*z), 2*(y*z-x*w), 0], [2*(x*z-y*w), 2*(y*z+x*w), 1-2*(x*x+y*y), 0], [0,0,0,1]], dtype=np.float32)
            matrix[:3, 3] = game_vector(bone["bind_translation"], position=True)
            matrices.append(np.linalg.inv(matrix).T.reshape(16))
        document.data["skins"] = [{"name": data["name"], "joints": list(range(len(data["bones"]))), "inverseBindMatrices": document.accessor(np.asarray(matrices, dtype="<f4"), "MAT4")}]
    lod = data["lods"][lod_index]
    primitives = []
    for surface in data["surfaces"][lod["surface_index"]:lod["surface_index"] + lod["surface_count"]]:
        vertices = surface["vertices"]
        position = np.asarray([v["position"] for v in vertices], dtype=np.float32)
        normal = packed_normal([v["normal_101010"] for v in vertices])
        tangent = np.column_stack((packed_normal([v["tangent_101010"] for v in vertices]), [v["binormal_sign"] for v in vertices]))
        uv = np.asarray([v["texcoord_half2"] for v in vertices], dtype=np.uint32)
        uv = np.column_stack((game_half(uv & 65535), game_half(uv >> 16)))
        color = np.asarray([v["color_rgba8"] for v in vertices], dtype=np.uint32)
        color = np.stack([(color >> shift) & 255 for shift in (0,8,16,24)], axis=-1).astype(np.float32) / 255.0
        attributes = vertex_attributes(document, position, normal, uv, color, tangent)
        if data["bones"]:
            joints = np.zeros((len(vertices), 4), dtype="<u2")
            weights = np.zeros((len(vertices), 4), dtype="<f4")
            vertex = 0
            for group in surface["rigid_vertex_groups"]:
                end = vertex + group["vertex_count"]
                joints[vertex:end, 0] = group["bone_offset"] // 32
                weights[vertex:end, 0] = 1
                vertex = end
            blend = surface["blend_data_u16"]
            offset = 0
            for count, vertices_count in enumerate(surface["blend_vertex_counts"], 1):
                for unused in range(vertices_count):
                    if vertex >= len(vertices):
                        raise AssetError(f"Native skin weights exceed vertices: {data['name']}")
                    joints[vertex, 0] = blend[offset] // 32
                    total = 0
                    for influence in range(1, count):
                        joints[vertex, influence] = blend[offset + influence * 2 - 1] // 32
                        weight = blend[offset + influence * 2] / 65535.0
                        weights[vertex, influence] = weight
                        total += weight
                    weights[vertex, 0] = 1 - total
                    offset += count * 2 - 1
                    vertex += 1
            if vertex != len(vertices) or np.any(joints >= len(data["bones"])):
                raise AssetError(f"Incomplete native skin weights: {data['name']}")
            attributes["JOINTS_0"] = document.accessor(joints, "VEC4", target=34962)
            attributes["WEIGHTS_0"] = document.accessor(weights, "VEC4", target=34962)
        material = material_lookup(surface["material"], document, path.parent)
        triangles = np.asarray(surface["triangles"], dtype="<u2")
        if triangles.size and int(triangles.max()) >= len(vertices):
            raise AssetError(f"Native model triangle index invalid: {data['name']}")
        primitives.append({"attributes": attributes, "indices": document.accessor(triangles[:, [2,1,0]].reshape(-1), "SCALAR", target=34963), "material": material})
    document.data["meshes"].append({"name": data["name"], "primitives": primitives})
    mesh_node = {"name": data["name"], "mesh": 0}
    if data["bones"]:
        mesh_node["skin"] = 0
    document.data["nodes"].append(mesh_node)
    document.data["scenes"][0]["nodes"] = roots + [len(document.data["nodes"]) - 1]
    document.save(path)


def world_layer_geometry(native, stream, surface, geometry):
    vertex_format = int(native.get("world_vertex_format", 0))
    if not 0 <= vertex_format <= 8:
        raise AssetError(f"Unknown native world vertex format {vertex_format}")
    declaration = NATIVE_VERTEX_DECLARATIONS["declarations"][vertex_format + 4]
    elements = [entry for entry in declaration["elements"] if entry["available"] and entry["input_slot"] == 1]
    sizes = {10: 8, 28: 4, 34: 4}
    stride = max((entry["byte_offset"] + sizes[entry["dxgi_format"]] for entry in elements), default=0)
    count, offset = surface["vertex_count"], surface["stream1_offset"]
    if offset < 0 or offset + count * stride > len(stream):
        raise AssetError(f"Native world layer stream is truncated: {native['name']}")
    geometry["uvs"] = [geometry["uv"]]
    geometry["rotations"] = []
    for entry in elements:
        start = offset + entry["byte_offset"]
        if entry["source_name"].startswith("TEXCOORD"):
            components = sizes[entry["dxgi_format"]] // 2
            values = np.ndarray((count, components), dtype="<u2", buffer=stream, offset=start, strides=(stride, 2))
            values = game_half(values)
            geometry["uvs"].extend(values[:, index:index + 2] for index in range(0, components, 2))
        elif entry["source_name"].startswith("NORMAL_TRANSFORM"):
            values = np.ndarray((count, 4), dtype="u1", buffer=stream, offset=start, strides=(stride, 1))
            geometry["rotations"].append(values.astype(np.float32) * (2 / 255) - 1)


def material_geometry_atlas(geometry, dimensions):
    triangles = np.asarray(geometry["triangles"], dtype=np.int64)
    charts = []
    for face, indices in enumerate(triangles):
        lengths = np.ones(2, dtype=np.float32)
        for uv, size in zip(geometry["uvs"], dimensions):
            edges = (uv[indices[1:]] - uv[indices[0]]) * np.asarray(size)
            lengths = np.maximum(lengths, np.linalg.norm(edges, axis=1))
        charts.append([face, int(math.ceil(lengths[0])) + 9, int(math.ceil(lengths[1])) + 9])
    area = sum(width * height for _, width, height in charts)
    density = min(1., math.sqrt(8388608 / max(1, area)))
    for chart in charts:
        chart[1:] = [max(10, int((value - 8) * density) + 8) for value in chart[1:]]
    atlas_width = 1 << max(4, math.ceil(math.log2(max(max(c[1] for c in charts), math.sqrt(sum(c[1]*c[2] for c in charts))))))
    placements = {}
    x = y = row_height = 0
    for face, width, height in sorted(charts, key=lambda item: item[2], reverse=True):
        if x + width > atlas_width:
            x, y, row_height = 0, y + row_height, 0
        placements[face] = (x, y, width, height)
        x += width
        row_height = max(row_height, height)
    atlas_height = 1 << max(4, math.ceil(math.log2(max(1, y + row_height))))
    if atlas_width > 16384 or atlas_height > 16384:
        raise AssetError("Native layered surface exceeds the maximum atlas dimensions")
    sources = {f"uv{index}": values for index, values in enumerate(geometry["uvs"])}
    sources.update({f"rotation{index}": values for index, values in enumerate(geometry["rotations"])})
    sources["color"] = geometry["color"]
    grids = {key: np.zeros((atlas_height, atlas_width, values.shape[1]), dtype=np.float32) for key, values in sources.items()}
    atlas_uv = np.empty((len(triangles), 3, 2), dtype=np.float32)
    for face, indices in enumerate(triangles):
        x, y, width, height = placements[face]
        u, v = np.meshgrid((np.arange(width, dtype=np.float32) - 4) / (width - 9), (np.arange(height, dtype=np.float32) - 4) / (height - 9))
        barycentric = np.maximum(np.stack((1-u-v, u, v), axis=2), 0)
        barycentric /= np.maximum(barycentric.sum(axis=2, keepdims=True), 1e-8)
        for key, values in sources.items():
            grids[key][y:y+height, x:x+width] = np.einsum("hwk,kc->hwc", barycentric, values[indices])
        atlas_uv[face] = np.asarray(((x+4.5, y+4.5), (x+width-4.5, y+4.5), (x+4.5, y+height-4.5))) / (atlas_width, atlas_height)
    expanded = triangles.reshape(-1)
    for key in ("position", "normal", "tangent"):
        geometry[key] = geometry[key][expanded]
    geometry["uv"] = atlas_uv.reshape(-1, 2)
    geometry["color"] = np.ones((len(expanded), 4), dtype=np.float32)
    geometry["triangles"] = np.arange(len(expanded), dtype="<u4").reshape(-1, 3)
    grids["texel_density"] = density
    return grids


def prepare_world(data, root, path, material_lookup):
    document = GltfDocument(data["name"])
    stream = inside(root, data["stream0"]).read_bytes()
    layer_stream = inside(root, data["stream1"]).read_bytes()
    indices = np.frombuffer(inside(root, data["indices"]).read_bytes(), dtype="<u2")
    primitives = []
    attributes_cache = {}
    for index, surface in enumerate(data["surfaces"]):
        offset = surface["stream0_offset"]
        count = surface["vertex_count"]
        if count == 0 or surface["triangle_count"] == 0:
            continue
        if offset < 0 or offset + count * 36 > len(stream):
            raise AssetError(f"Native world vertex span invalid: {data['name']}/{index}")
        key = (offset, count)
        if key not in attributes_cache:
            position = np.ndarray((count, 3), dtype="<f4", buffer=stream, offset=offset, strides=(36, 4))
            color = np.ndarray((count, 4), dtype="u1", buffer=stream, offset=offset + 16, strides=(36, 1)).astype(np.float32) / 255.0
            uv = np.ndarray((count, 2), dtype="<u2", buffer=stream, offset=offset + 20, strides=(36, 2))
            uv = game_half(uv)
            normal = np.ndarray((count,), dtype="<u4", buffer=stream, offset=offset + 24, strides=(36,))
            tangent = np.ndarray((count,), dtype="<u4", buffer=stream, offset=offset + 28, strides=(36,))
            sign = np.ndarray((count,), dtype="<f4", buffer=stream, offset=offset + 12, strides=(36,))
            attributes_cache[key] = vertex_attributes(document, position, packed_normal(normal), uv, color, np.column_stack((packed_normal(tangent), sign)))
        start = surface["base_index"]
        end = start + surface["triangle_count"] * 3
        triangle = indices[start:end]
        if end > len(indices) or len(triangle) != end - start or int(triangle.max()) >= count:
            raise AssetError(f"Native world triangle span invalid: {data['name']}/{index}")
        geometry = {"position": position, "normal": packed_normal(normal), "tangent": np.column_stack((packed_normal(tangent), sign)), "uv": uv, "color": color, "triangles": triangle.reshape(-1, 3), "surface": surface, "layer_stream": layer_stream}
        material = material_lookup(surface["material"], document, path.parent, geometry)
        attributes = vertex_attributes(document, geometry["position"], geometry["normal"], geometry["uv"], geometry["color"], geometry["tangent"])
        primitives.append({"attributes": attributes, "indices": document.accessor(geometry["triangles"][:, [2,1,0]].reshape(-1), "SCALAR", target=34963), "material": material, "extras": {"native_surface": index, "native_material": surface["material"], "lightmap": surface["lightmap"], "reflection_probe": surface["reflection_probe"], "primary_light": surface["primary_light"], "flags": surface["flags"], "native_stream1_offset": surface["stream1_offset"]}})
    document.data["meshes"] = [{"name": data["name"], "primitives": primitives}]
    document.data["nodes"] = [{"name": data["name"], "mesh": 0}]
    document.data["scenes"][0]["nodes"] = [0]
    document.data["extras"].update(native_static_models=data["static_models"], native_hero_lights=data["hero_lights"], native_sun=data["initial_sun"], native_fog=data["initial_fog"])
    document.save(path)



def material_hash(name):
    result = 0
    for byte in name.encode("utf-8"):
        result = ((result * 33) ^ (byte | 0x20)) & 0xFFFFFFFF
    return result


def decode_iwi_pixels(image: IwiImage) -> np.ndarray:
    if image.faces != 1 or image.depth > 1:
        raise AssetError("Material baking needs a 2D image, not a native cube or volume")
    data = image.levels[0]
    width, height = image.width, image.height
    if image.format == 19:
        result = np.frombuffer(data, dtype="<f2").astype(np.float32).reshape(height, width, 4)
        if not np.isfinite(result).all():
            raise AssetError("Non-finite native RGBA16F texels")
        return result
    if image.format == 1:
        return np.frombuffer(data, dtype=np.uint8).reshape(height, width, 4).astype(np.float32) / 255
    if image.format in {3, 4, 5}:
        source = np.frombuffer(data, dtype=np.uint8).reshape(height, width, 2 if image.format == 3 else 1)
        result = np.ones((height, width, 4), dtype=np.float32)
        if image.format == 5:
            result[:, :, 3] = source[:, :, 0] / 255
        else:
            result[:, :, :3] = source[:, :, :1] / 255
            if image.format == 3:
                result[:, :, 3] = source[:, :, 1] / 255
        return result
    if image.format not in {11, 12, 13, 14}:
        raise AssetError(f"Cannot bake native IWI format {image.format}")
    block_size = 8 if image.format == 11 else 16
    blocks = np.frombuffer(data, dtype=np.uint8).reshape(-1, block_size)
    def bc_alpha(encoded):
        a = encoded[:, 0].astype(np.uint16)
        b = encoded[:, 1].astype(np.uint16)
        palette = np.empty((len(encoded), 8), dtype=np.uint16)
        palette[:, :2] = np.stack((a, b), axis=1)
        for index in range(2, 8):
            seven = ((8-index) * a + (index-1) * b) // 7
            five = ((6-index) * a + (index-1) * b) // 5 if index < 6 else np.full_like(a, 0 if index == 6 else 255)
            palette[:, index] = np.where(a > b, seven, five)
        indices = np.zeros(len(encoded), dtype=np.uint64)
        for index in range(6):
            indices |= encoded[:, 2 + index].astype(np.uint64) << (8 * index)
        lookup = (indices[:, None] >> (3*np.arange(16, dtype=np.uint64))) & 7
        return np.take_along_axis(palette, lookup.astype(np.int64), axis=1).astype(np.float32) / 255
    result = np.ones((len(blocks), 16, 4), dtype=np.float32)
    if image.format == 14:
        result[:, :, 0] = bc_alpha(blocks[:, :8])
        result[:, :, 1] = bc_alpha(blocks[:, 8:])
        result[:, :, 2] = 0
    else:
        color = blocks if image.format == 11 else blocks[:, 8:]
        endpoints = color[:, :4].copy().view("<u2").reshape(-1, 2)
        expanded = np.stack(((endpoints >> 11) * 255 // 31, ((endpoints >> 5) & 63) * 255 // 63, (endpoints & 31) * 255 // 31), axis=2)
        palette = np.ones((len(blocks), 4, 4), dtype=np.uint16) * 255
        palette[:, :2, :3] = expanded
        palette[:, 2, :3] = (2*expanded[:, 0] + expanded[:, 1]) // 3
        palette[:, 3, :3] = (expanded[:, 0] + 2*expanded[:, 1]) // 3
        if image.format == 11:
            three = endpoints[:, 0] <= endpoints[:, 1]
            palette[three, 2, :3] = (expanded[three, 0] + expanded[three, 1]) // 2
            palette[three, 3] = 0
        indices = color[:, 4:].copy().view("<u4").reshape(-1)
        lookup = (indices[:, None] >> (2*np.arange(16, dtype=np.uint32))) & 3
        result = palette[np.arange(len(blocks))[:, None], lookup].astype(np.float32) / 255
        if image.format == 12:
            alpha = blocks[:, :8].copy().view("<u8").reshape(-1)
            result[:, :, 3] = ((alpha[:, None] >> (4*np.arange(16, dtype=np.uint64))) & 15).astype(np.float32) / 15
        elif image.format == 13:
            result[:, :, 3] = bc_alpha(blocks[:, :8])
    return result.reshape(math.ceil(height/4), math.ceil(width/4), 4, 4, 4).transpose(0, 2, 1, 3, 4).reshape(math.ceil(height/4)*4, math.ceil(width/4)*4, 4)[:height, :width]


def material_sample(pixels, width, height, sampler, scale=(1., 1.)):
    source_height, source_width = pixels.shape[:2]
    u = (np.arange(width, dtype=np.float32) + .5) * (source_width * scale[0] / width) - .5
    v = (np.arange(height, dtype=np.float32) + .5) * (source_height * scale[1] / height) - .5
    if sampler & 7 == 1:
        x, y = np.floor(u + .5).astype(np.int64), np.floor(v + .5).astype(np.int64)
        x = np.clip(x, 0, source_width-1) if sampler & 32 else x % source_width
        y = np.clip(y, 0, source_height-1) if sampler & 64 else y % source_height
        return pixels[y[:, None], x[None, :]]
    x0, y0 = np.floor(u).astype(np.int64), np.floor(v).astype(np.int64)
    wx, wy = (u-x0).astype(np.float32)[None, :, None], (v-y0).astype(np.float32)[:, None, None]
    x1, y1 = x0+1, y0+1
    x0 = np.clip(x0, 0, source_width-1) if sampler & 32 else x0 % source_width
    x1 = np.clip(x1, 0, source_width-1) if sampler & 32 else x1 % source_width
    y0 = np.clip(y0, 0, source_height-1) if sampler & 64 else y0 % source_height
    y1 = np.clip(y1, 0, source_height-1) if sampler & 64 else y1 % source_height
    return ((pixels[y0[:, None], x0[None, :]]*(1-wx) + pixels[y0[:, None], x1[None, :]]*wx)*(1-wy) + (pixels[y1[:, None], x0[None, :]]*(1-wx) + pixels[y1[:, None], x1[None, :]]*wx)*wy).astype(np.float32)


def material_srgb(linear):
    linear = np.maximum(linear, 0)
    return np.where(linear <= .0031308, linear*12.92, 1.055*np.power(linear, 1/2.4)-.055)


def material_texture(output, identity, role, pixels, srgb, sampler, normal=False):
    pixels = np.asarray(pixels, dtype=np.float32)
    high_dynamic_range = bool(np.any(pixels[:, :, :3] > 1))
    levels = []
    current = pixels
    while True:
        encoded = current.copy()
        if normal:
            direction = encoded[:, :, :3] * 2 - 1
            direction /= np.maximum(np.linalg.norm(direction, axis=2, keepdims=True), 1e-8)
            encoded[:, :, :3] = direction * .5 + .5
        if srgb and not high_dynamic_range:
            encoded[:, :, :3] = material_srgb(encoded[:, :, :3])
        levels.append(encoded.astype("<f2").tobytes() if high_dynamic_range else np.rint(np.clip(encoded, 0, 1)*255).astype(np.uint8).tobytes())
        height, width = current.shape[:2]
        if height == 1 and width == 1 or sampler & 24 == 0:
            break
        new_height, new_width = max(1, height//2), max(1, width//2)
        y0, x0 = np.arange(new_height)*2, np.arange(new_width)*2
        y1, x1 = np.minimum(y0+1, height-1), np.minimum(x0+1, width-1)
        current = (current[y0[:, None], x0[None, :]] + current[y0[:, None], x1[None, :]] + current[y1[:, None], x0[None, :]] + current[y1[:, None], x1[None, :]]) * .25
    content = hashlib.sha256(b"".join(levels) + bytes((19 if high_dynamic_range else 1, int(srgb and not high_dynamic_range)))).hexdigest()
    uri = f"prepared/images/{content}-{role}.ktx2"
    path = inside(output, uri)
    if not path.is_file():
        image = IwiImage(pixels.shape[1], pixels.shape[0], 1, 1, 19 if high_dynamic_range else 1, 2 if len(levels)==1 else 0, 1., (), tuple(levels))
        write_ktx2(path, image, srgb=srgb and not high_dynamic_range)
    return {"uri": uri, "role": role, "layer": 0, "color_space": "srgb" if srgb and not high_dynamic_range else "linear", "sampler": sampler}


def material_presentation(native):
    entries, states = native.get("state_entries", []), native.get("state_bits", [])
    selected = next((entries[index] for index in (4, 2, 0) if index < len(entries) and 0 <= entries[index] < len(states)), None)
    if selected is None:
        raise AssetError(f"No native surface render state: {native['name']}")
    raw, depth = states[selected][:2]
    source, destination, operation = raw & 15, raw >> 4 & 15, raw >> 8 & 7
    cutoff, alpha = 0., "opaque"
    if operation not in {0, 1}:
        raise AssetError(f"Native RGB blend operation {operation} needs a dedicated bake: {native['name']}")
    if operation and (source, destination) not in {(2, 1), (0, 0)}:
        if (source, destination) == (5, 6):
            alpha = "blend"
        elif (source, destination) == (2, 6):
            alpha = "premultiplied"
        elif destination == 2 and source in {2, 5}:
            alpha = "add"
        elif (source, destination) in {(9, 1), (1, 3)}:
            alpha = "multiply"
        else:
            raise AssetError(f"Native blend factors {source}/{destination} need a dedicated bake: {native['name']}")
    if not raw & (1 << 11) and alpha == "opaque":
        alpha = "mask"
        cutoff = 128/255 if raw & (1 << 12) else 1/255
    cull = {1: "none", 2: "back", 3: "front"}.get(raw >> 14 & 3)
    if cull is None:
        raise AssetError(f"Invalid native cull state: {native['name']}")
    return {"alpha": alpha, "cutoff": cutoff, "cull": cull, "fog": "nofog" not in native.get("technique_set", "").lower(), "visible": bool(raw & (1 << 27)), "native_state_index": selected, "native_state_bits": [raw, depth]}



def material_uv_sample(pixels, uv, sampler):
    height, width = pixels.shape[:2]
    xy = np.asarray(uv, dtype=np.float32) * (width, height) - .5
    if sampler & 7 == 1:
        indices = np.floor(xy+.5).astype(np.int64)
        x, y = indices[:, :, 0], indices[:, :, 1]
        x = np.clip(x, 0, width-1) if sampler & 32 else x % width
        y = np.clip(y, 0, height-1) if sampler & 64 else y % height
        return pixels[y, x]
    floor = np.floor(xy).astype(np.int64)
    fraction = (xy-floor).astype(np.float32)
    x0, y0 = floor[:, :, 0], floor[:, :, 1]
    x1, y1 = x0+1, y0+1
    x0 = np.clip(x0, 0, width-1) if sampler & 32 else x0 % width
    x1 = np.clip(x1, 0, width-1) if sampler & 32 else x1 % width
    y0 = np.clip(y0, 0, height-1) if sampler & 64 else y0 % height
    y1 = np.clip(y1, 0, height-1) if sampler & 64 else y1 % height
    wx, wy = fraction[:, :, :1], fraction[:, :, 1:2]
    return ((pixels[y0,x0]*(1-wx)+pixels[y0,x1]*wx)*(1-wy)+(pixels[y1,x0]*(1-wx)+pixels[y1,x1]*wx)*wy).astype(np.float32)


def material_native_inputs(native, image_lookup):
    constants = {int(value["name_hash"]): value for value in native.get("constants", [])}
    textures = {int(value["name_hash"]): value for value in native.get("textures", [])}
    used_constants, used_textures, evidence = set(), set(), []
    def constant(name, default):
        hashed = material_hash(name)
        value = constants.get(hashed)
        if value:
            used_constants.add(hashed)
        return np.asarray(value["literal"] if value else default, dtype=np.float32)
    def texture(hashed, required=True):
        value = textures.get(hashed)
        if value is None:
            if required:
                raise AssetError(f"Missing native texture binding {hashed}: {native['name']}")
            return None
        used_textures.add(hashed)
        found = image_lookup(value["image"], "linear")
        if found is None:
            raise AssetError(f"Missing original native image {value['image']}: {native['name']}")
        evidence.append({"name":value["image"], "sha256":found["sha256"], "name_hash":hashed,"sampler":value["sampler"],"semantic":value["semantic"]})
        return {"pixels":decode_iwi_pixels(found["image"]),"sampler":int(value["sampler"])}
    def finish():
        remaining_textures = set(textures)-used_textures
        remaining_constants = set(constants)-used_constants
        if remaining_textures or remaining_constants:
            raise AssetError(f"Unbaked native layered inputs for {native['name']}: textures={sorted(remaining_textures)}, controls={[constants[key]['name'] for key in remaining_constants]}")
    return constant, texture, finish, evidence


def material_layer_result(native, channels, standard, evidence, approximations):
    presentation = material_presentation(native)
    surface = {"version":3,"identity":native["name"],"channels":channels,"standard":standard,"presentation":presentation,"native_technique_set":native.get("technique_set"),"native_images":evidence,"native_constants":native.get("constants",[]),"approximations":approximations}
    return {"name":native["name"],"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.,"roughnessFactor":1.},"alphaMode":"MASK" if presentation["alpha"]=="mask" else "OPAQUE" if presentation["alpha"]=="opaque" else "BLEND","alphaCutoff":presentation["cutoff"],"doubleSided":presentation["cull"]=="none","extras":{"surface":surface}}


def prepare_tile_material(native, image_lookup, output):
    constant, texture, finish, evidence = material_native_inputs(native, image_lookup)
    macro_normal, macro_ao = texture(329745474), texture(3976875907)
    color1, color2 = texture(3580385162), texture(2643103145)
    normal1, normal2 = texture(3069501348), texture(3400726183)
    specular1, specular2 = texture(830013099), texture(2415611304)
    scale1, scale2 = constant("Micro_1_Scale",(1,1,0,1))[:2], constant("Micro_2_Scale",(1,1,0,1))[:2]
    height_macro = float(constant("Macro_Height",(1,0,0,1))[0])
    height1 = float(constant("Micro_1_Height",(1,0,0,1))[0])
    height2 = float(constant("Micro_2_Height",(1,0,0,1))[0])
    ao_adjust = float(constant("AO_Diffuse_Adj",(0,0,0,1))[0])
    reflection = float(constant("reflection_amount",(1,0,0,1))[0])
    finish()
    sources = [(macro_normal,(1,1)),(macro_ao,(1,1)),(color1,scale1),(color2,scale2),(normal1,scale1),(normal2,scale2),(specular1,scale1),(specular2,scale2)]
    width=max(max(1,math.ceil(item['pixels'].shape[1]*abs(float(scale[0])))) for item,scale in sources)
    height=max(max(1,math.ceil(item['pixels'].shape[0]*abs(float(scale[1])))) for item,scale in sources)
    if width>16384 or height>16384 or width*height>67108864:
        raise AssetError(f"Native layered material frequencies exceed 64M bake texels: {native['name']} ({width}x{height})")
    def sample(item,scale=(1,1)):
        return material_sample(item['pixels'],width,height,item['sampler'],scale)
    ao_source=sample(macro_ao)
    blend=ao_source[:,:,1:2]**2
    base=sample(color1,scale1)
    base2=sample(color2,scale2)
    base[:,:,:3]=(base[:,:,:3]*(1-blend)+base2[:,:,:3]*blend)**2
    base[:,:,3]=1
    slopes=(sample(macro_normal)[:,:,:2]*(510/127)-(256/127))*height_macro
    slopes+=(sample(normal1,scale1)[:,:,:2]*(510/127)-(256/127))*(1-blend)*height1
    slopes+=(sample(normal2,scale2)[:,:,:2]*(510/127)-(256/127))*blend*height2
    normals=np.concatenate((slopes,np.ones((height,width,1),dtype=np.float32)),axis=2)
    normals/=np.maximum(np.linalg.norm(normals,axis=2,keepdims=True),1e-8)
    normal_pixels=np.concatenate((normals*.5+.5,np.ones((height,width,1),dtype=np.float32)),axis=2)
    specular=sample(specular1,scale1)*(1-blend)+sample(specular2,scale2)*blend
    ao=np.clip(ao_source[:,:,0]**2+ao_adjust,0,1)
    response=np.ones((height,width,4),dtype=np.float32)
    response[:,:,0]=ao
    response[:,:,1]=np.sqrt(1-np.clip(specular[:,:,0],0,1))
    response[:,:,2]=0
    reflectance=np.ones_like(response)
    reflectance[:,:,:3]=specular[:,:,1:4]**2*max(0,reflection)*ao[:,:,None]/.16
    sampler=color1['sampler']
    channels=[material_texture(output,native['name'],'color',base,True,sampler),material_texture(output,native['name'],'normal',normal_pixels,False,sampler,normal=True),material_texture(output,native['name'],'response',response,False,sampler),material_texture(output,native['name'],'reflectance',reflectance,False,sampler)]
    standard={"roughness":1.,"metallic":1.,"reflectance":1.,"specular_transmission":0.,"diffuse_transmission":0.,"ior":1.5,"thickness":0.,"unlit":False,"base_color":[1.,1.,1.,1.],"emissive":[0.,0.,0.]}
    return material_layer_result(native,channels,standard,evidence,["Native micro/macro tile textures, AO blend, scale and slope-height controls are flattened in UV0; the native specular lighting equation is approximated by StandardMaterial."])


def prepare_crystal_material(native, image_lookup, output):
    constant, texture, finish, evidence = material_native_inputs(native,image_lookup)
    staff="wpn_zmb_staff_3layer" in native.get('technique_set','')
    normal=texture(1507003663)
    layers=[texture(3714339118),texture(3714374925)]
    layers.append(texture(1507458012 if staff else 3714406764))
    charged=texture(3424736909) if staff else None
    controls=[]
    for index in range(1,4):
        controls.append({key:constant(f"Layer{index}{key}",default) for key,default in {"Scale":(1,1,0,1),"Offset":(0,0,0,1),"Scroll":(0,0,0,1),"Rotation":(0,0,0,1),"Origin":(0,0,0,1),"Depth":(0,0,0,1),"Alpha":(1,0,0,1)}.items()})
    warp=constant('WarpScale',(1,1,0,1))[:2]
    normal_height=float(constant('NormalHeight',(1,0,0,1))[0])
    gloss=float(constant('Gloss_Amount',(.5,0,0,1))[0])
    reflection=float(constant('Reflection_Amount',(0,0,0,1))[0])
    if staff:
        intensity_off=float(constant('Emissive_Amount_Off',(1,0,0,1))[0])
        intensity_on=float(constant('Emissive_Amount_On',(1,0,0,1))[0])
        charge_level=float(constant('Charge_Level',(1,0,0,1))[0])
    else:
        intensity_off=intensity_on=float(constant('hdrAmount',(1,0,0,1))[0])
        charge_level=1.
    finish()
    sources=[(normal,warp)]+[(layer,control['Scale'][:2]) for layer,control in zip(layers,controls)]
    if charged:sources.append((charged,controls[2]['Scale'][:2]))
    width=max(max(1,math.ceil(item['pixels'].shape[1]*abs(float(scale[0])))) for item,scale in sources)
    height=max(max(1,math.ceil(item['pixels'].shape[0]*abs(float(scale[1])))) for item,scale in sources)
    if width>16384 or height>16384 or width*height>67108864:
        raise AssetError(f"Native crystal material frequencies exceed 64M bake texels: {native['name']} ({width}x{height})")
    uv=np.stack(np.meshgrid((np.arange(width,dtype=np.float32)+.5)/width,(np.arange(height,dtype=np.float32)+.5)/height),axis=2)
    slopes=(material_uv_sample(normal['pixels'],uv*warp,normal['sampler'])[:,:,:2]*(510/127)-(256/127))*normal_height
    directions=np.concatenate((slopes,np.ones((height,width,1),dtype=np.float32)),axis=2)
    directions/=np.maximum(np.linalg.norm(directions,axis=2,keepdims=True),1e-8)
    normal_pixels=np.concatenate((directions*.5+.5,np.ones((height,width,1),dtype=np.float32)),axis=2)
    response=np.ones((height,width,4),dtype=np.float32)
    response[:,:,1]=math.sqrt(1-max(0,min(1,gloss)))
    response[:,:,2]=0
    reflectance=np.ones_like(response)
    reflectance[:,:,:3]=max(0,reflection)*max(0,min(1,.75+.25*gloss))/.16
    sampler=layers[0]['sampler']
    fixed=[material_texture(output,native['name'],'normal',normal_pixels,False,sampler,normal=True),material_texture(output,native['name'],'response',response,False,sampler),material_texture(output,native['name'],'reflectance',reflectance,False,sampler)]
    def frame(time,charge):
        sampled=[]
        for index,(layer,control) in enumerate(zip(layers,controls)):
            coordinates=(uv-control['Origin'][:2]+control['Offset'][:2])*control['Scale'][:2]
            angle=float(control['Rotation'][0])*(1+float(control['Rotation'][1])*time)*(2*math.pi)
            cosine,sine=math.cos(angle),math.sin(angle)
            coordinates=np.stack((coordinates[:,:,0]*cosine+coordinates[:,:,1]*sine,-coordinates[:,:,0]*sine+coordinates[:,:,1]*cosine),axis=2)
            coordinates+=time*control['Scroll'][:2]*control['Scale'][:2]+control['Origin'][:2]
            selected=charged if staff and index==2 and charge>=charge_level else layer
            sampled.append(material_uv_sample(selected['pixels'],coordinates,selected['sampler']))
        one,two,three=sampled
        composed=three[:,:,:3]*three[:,:,3:4]
        weight=two[:,:,3:4]*float(controls[1]['Alpha'][0])
        composed=composed+(two[:,:,:3]-composed)*weight
        weight=one[:,:,3:4]*float(controls[0]['Alpha'][0])
        composed=composed+(one[:,:,:3]-composed)*weight
        intensity=intensity_on if staff and charge>=charge_level else intensity_off
        emission=np.ones((height,width,4),dtype=np.float32)
        emission[:,:,:3]=np.maximum(composed*intensity,0)**2
        channels=[material_texture(output,native['name'],'emissive',emission,True,sampler)]+fixed
        standard={"roughness":1.,"metallic":1.,"reflectance":1.,"specular_transmission":0.,"diffuse_transmission":0.,"ior":1.5,"thickness":0.,"unlit":False,"base_color":[0.,0.,0.,1.],"emissive":[1.,1.,1.]}
        return {"channels":channels,"standard":standard}
    animated=any(np.any(control['Scroll'][:2]) or float(control['Rotation'][0]*control['Rotation'][1])!=0 for control in controls)
    first=frame(0,0)
    approximations=["Native layer transforms, alpha composition, HDR, normal height and gloss are baked at normal view incidence; view-dependent parallax and native reflection lighting are approximated by StandardMaterial."]
    result=material_layer_result(native,first['channels'],first['standard'],evidence,approximations)
    if staff:
        last=max(1,math.ceil(charge_level))
        on=frame(0,charge_level)
        frames=[first if index<charge_level else on for index in range(last+1)]
        result['extras']['surface']['animation']={"driver":"staff_charge","fps":1.,"frames":frames}
        if animated:approximations.append("Staff charge selects authored off/on compositions; independently scrolling layer controls are retained in native metadata and sampled at time zero.")
    elif animated:
        frames=[first]+[frame(index/8,0) for index in range(1,16)]
        result['extras']['surface']['animation']={"driver":"time","fps":8.,"frames":frames}
        approximations.append("Authored scrolling and rotation are sampled at 8 Hz in a repeating two-second clip; the long native cycle remains preserved in the native controls.")
    result['extras']['surface']['bake']={"layer_count":3,"view":"normal_incidence","native_depths":[float(control['Depth'][0]) for control in controls]}
    return result


def prepare_material(native: dict, image_lookup, output: Path) -> dict:
    technique_name = native.get("technique_set", "").lower()
    if "cod7_tile_blend_spec" in technique_name:
        return prepare_tile_material(native, image_lookup, output)
    if "unlit_3layer_normal_script" in technique_name or "wpn_zmb_staff_3layer" in technique_name:
        return prepare_crystal_material(native, image_lookup, output)
    name = native["name"]
    technique = native.get("technique_set", "").lower().lstrip(",")
    family = re.sub(r"^(?:mc|mlv|wpc)_", "", technique)
    generic_lit = bool(re.match(r"lit_sm_[a-z0-9]+_[a-z0-9]+$", family)) and not re.search(r"[bcnosx][1-9]", family.split("_")[2])
    generic_unlit = bool(re.match(r"unlit_(?:replace|add)_[a-z0-9]+$", family))
    emissive_lit = "phong_emissive_alcatraz" in family
    if not (generic_lit or generic_unlit or emissive_lit):
        raise AssetError(f"Native material technique requires authored geometry, time, or shader-specific baking: {name} ({technique})")
    constants = {int(value["name_hash"]): value for value in native.get("constants", [])}
    used_constants = set()
    def constant(names, default):
        for candidate in names:
            hashed = material_hash(candidate)
            if hashed in constants:
                used_constants.add(hashed)
                return np.asarray(constants[hashed]["literal"], dtype=np.float32)
        return np.asarray(default, dtype=np.float32)
    presentation = material_presentation(native)
    textures = native.get("textures", [])
    by_hash = {int(texture["name_hash"]): texture for texture in textures}
    used_textures = set()
    source_images = []
    def source(hashes, semantics=()):
        texture = next((by_hash[h] for h in hashes if h in by_hash), None)
        if texture is None:
            texture = next((item for item in textures if item["semantic"] in semantics and item["name_hash"] not in used_textures), None)
        if texture is None:
            return None
        used_textures.add(texture["name_hash"])
        result = image_lookup(texture["image"], "linear")
        if result is None:
            raise AssetError(f"Missing original native image {texture['image']}: {name}")
        source_images.append({"name": texture["image"], "sha256": result["sha256"], "semantic": texture["semantic"], "name_hash": texture["name_hash"], "sampler": texture["sampler"]})
        return {"pixels": decode_iwi_pixels(result["image"]), "sampler": int(texture["sampler"])}
    color = source((2695565377,), (2,))
    normal = source((1507003663, 2485960688), (5,))
    specular = source((887934131, 2351529600), (8,))
    emissive = source((1825069840,))
    detail = source((3948059469,))
    occlusion = source((), (10,))
    if any(texture["name_hash"] not in used_textures for texture in textures):
        missing = [str(texture["name_hash"]) for texture in textures if texture["name_hash"] not in used_textures]
        raise AssetError(f"Native material has unbaked texture inputs {','.join(missing)}: {name}")
    detail_scale = constant(("detailScale",), (1., 1., 1., 0.)) if detail else None
    available = [entry for entry in (color, normal, specular, emissive, detail, occlusion) if entry is not None]
    if not available:
        raise AssetError(f"Native material has no recovered surface image: {name}")
    width = max(entry["pixels"].shape[1] for entry in available)
    height = max(entry["pixels"].shape[0] for entry in available)
    if detail:
        width = max(width, math.ceil(detail["pixels"].shape[1] * abs(float(detail_scale[0]))))
        height = max(height, math.ceil(detail["pixels"].shape[0] * abs(float(detail_scale[1]))))
    if width > 16384 or height > 16384 or width * height > 67108864:
        raise AssetError(f"Authored material texture frequencies need a geometry atlas beyond 64M texels: {name} ({width}x{height})")
    sampler = color["sampler"] if color else available[0]["sampler"]
    def resample(entry, scale=(1., 1.)):
        return material_sample(entry["pixels"], width, height, entry["sampler"], scale)
    rgba = resample(color) if color else np.ones((height, width, 4), dtype=np.float32)
    tint = constant(("colorTint", "scaleRGB"), (1., 1., 1., 1.))
    base = rgba.copy()
    base[:, :, :3] = np.maximum(base[:, :, :3] * tint[:3], 0)**2
    base[:, :, 3] *= tint[3]
    if generic_lit and presentation["alpha"] in {"blend", "premultiplied", "add", "multiply"}:
        base[:, :, 3] = np.sqrt(np.maximum(base[:, :, 3], 0))
    hdr = float(constant(("hdrAmount",), (1., 0., 0., 1.))[0])
    specular_amount = float(constant(("SpecularAmount", "Specular_Amount"), (1., 0., 0., 1.))[0])
    specular_push = float(constant(("Specular_Push",), (1., 0., 0., 1.))[0])
    reflection_amount = float(constant(("ReflectionAmount", "Reflection_Amount"), (1., 0., 0., 1.))[0])
    gloss_push = float(constant(("Gloss_Push", "Gloss_Amount"), (1., 0., 0., 1.))[0])
    occlusion_amount = constant(("occlusionAmount",), (1., 1., 1., 1.))
    if generic_lit and not np.allclose(occlusion_amount, (1., 1., 1., 1.)):
        raise AssetError(f"Native lighting-specific occlusion controls need separate light response: {name}")
    channels = [material_texture(output, name, "color", base, True, sampler)]
    if normal:
        slopes = resample(normal)[:, :, :2] * (510/127) - (256/127)
        if detail:
            slopes += (resample(detail, tuple(detail_scale[:2]))[:, :, :2] * (510/127) - (256/127)) * detail_scale[2]
        direction = np.concatenate((slopes, np.ones((height, width, 1), dtype=np.float32)), axis=2)
        direction /= np.maximum(np.linalg.norm(direction, axis=2, keepdims=True), 1e-8)
        normal_pixels = np.concatenate((direction*.5+.5, np.ones((height, width, 1), dtype=np.float32)), axis=2)
        channels.append(material_texture(output, name, "normal", normal_pixels, False, normal["sampler"], normal=True))
    elif detail:
        raise AssetError(f"Native detail normal lacks a primary normal: {name}")
    response = np.ones((height, width, 4), dtype=np.float32)
    response[:, :, 2] = 0
    reflectance = np.ones((height, width, 4), dtype=np.float32)
    if specular:
        specular_pixels = resample(specular)
        gloss = np.clip(specular_pixels[:, :, 3] * gloss_push, 0, 1)
        native_f0 = np.maximum(specular_pixels[:, :, :3], 0)**2 * specular_amount * specular_push * reflection_amount * occlusion_amount[1]
        reflectance[:, :, :3] = native_f0 / .16
        if presentation["alpha"] == "opaque" and generic_lit:
            response[:, :, 0] = np.clip(rgba[:, :, 3], 0, 1)
    elif generic_lit and "x0" in family:
        gloss = np.clip(rgba[:, :, 3], 0, 1)
        reflectance[:, :, :3] = .04/.16
        base[:, :, 3] = 1
        channels[0] = material_texture(output, name, "color", base, True, sampler)
    else:
        gloss = np.zeros((height, width), dtype=np.float32)
        reflectance[:, :, :3] = .04/.16
    response[:, :, 1] = np.sqrt(1-gloss)
    if occlusion:
        response[:, :, 0] *= np.clip(resample(occlusion)[:, :, 0], 0, 1)
    if not generic_unlit:
        channels.append(material_texture(output, name, "response", response, False, sampler))
        channels.append(material_texture(output, name, "reflectance", reflectance, False, specular["sampler"] if specular else sampler))
    emissive_factor = [0., 0., 0.]
    if emissive:
        emission = resample(emissive)
        emission[:, :, :3] = np.maximum(emission[:, :, :3], 0)**2
        channels.append(material_texture(output, name, "emissive", emission, True, emissive["sampler"]))
        push = float(constant(("Emissive_Push", "Emissive_Amount"), (1., 0., 0., 1.))[0])
        emissive_factor = [max(0., hdr*push)] * 3
    extra_constants = [value["name"] for key, value in constants.items() if key not in used_constants and not (detail and key == material_hash("detailScale"))]
    if extra_constants:
        raise AssetError(f"Native material has unbaked authored controls {','.join(extra_constants)}: {name}")
    if generic_unlit:
        hdr_base = max(0., hdr)
    else:
        hdr_base = 1.
    standard = {"roughness": 1., "metallic": 1. if not generic_unlit else 0., "reflectance": 1., "specular_transmission": 0., "diffuse_transmission": 0., "ior": 1.5, "thickness": 0., "unlit": generic_unlit, "base_color": [hdr_base, hdr_base, hdr_base, 1.], "emissive": emissive_factor}
    surface = {"version": 3, "identity": name, "channels": channels, "standard": standard, "presentation": presentation, "native_technique_set": native.get("technique_set"), "native_images": source_images, "native_constants": native.get("constants", []), "approximations": ["Native environment/specular lighting is represented by the StandardMaterial BRDF; native gloss maps to perceptual roughness sqrt(1-gloss).", "Native material gamma2 and slope normals are converted offline; renderer lighting and vertex color interpolation remain engine-native."]}
    return {"name": name, "pbrMetallicRoughness": {"baseColorFactor": [1., 1., 1., 1.], "metallicFactor": 0., "roughnessFactor": 1.}, "alphaMode": "MASK" if presentation["alpha"] == "mask" else "OPAQUE" if presentation["alpha"] == "opaque" else "BLEND", "alphaCutoff": presentation["cutoff"], "doubleSided": presentation["cull"] == "none", "extras": {"surface": surface}}


def native_pack(extraction: dict, install: Path, output: Path) -> dict:
    signature = {"schema_version": 1, "exporter_sha256": extraction["tool"]["exporter_sha256"], "texture_encoder_sha256": bytes_digest((inspect.getsource(read_iwi) + inspect.getsource(write_ktx2) + inspect.getsource(dfd)).encode()), "original_containers": {row["path"]: row["sha256"] for row in extraction["containers"]}}
    stamp = output / ".source-build.json"
    if output.exists() and (not stamp.is_file() or read_json(stamp) != signature):
        raise AssetError(f"Output must be fresh or belong to this exact original-source build: {output}")
    output.mkdir(parents=True, exist_ok=True)
    write_json(stamp, signature)
    native = output / "native"
    manifest = {"schema_version": 1, "worlds": [], "models": [], "animations": [], "materials": [], "images": [], "audio": [], "cinematics": [], "native_art": {}}
    identities = []
    categories = collections.defaultdict(collections.Counter)
    issues = []
    definitions = collections.defaultdict(list)
    prepared_images = {}
    ipak_images = collections.defaultdict(list)
    zone_images = {}
    native_records = {}
    record_sources = {}

    def prepare_image(path, source):
        sha = digest(path)
        if sha not in prepared_images:
            image = read_iwi(path)
            outputs = {}
            for srgb in (False, True):
                if image.format in {14, 19} and srgb:
                    continue
                transfer = "srgb" if srgb else "linear"
                target = output / "images" / f"{sha[:24]}-{transfer}.ktx2"
                if not target.is_file():
                    write_ktx2(target, image, srgb=srgb)
                outputs[transfer] = target.relative_to(output).as_posix()
            prepared_images[sha] = {"path": path, "sha256": sha, "image": image, "ktx2": outputs}
        result = prepared_images[sha]
        return result

    for ipak in extraction["ipaks"]:
        root = Path(ipak["root"])
        index_file = root / "ipak-index.json"
        checked_hash(index_file, ipak["index_sha256"], "fresh original image-package inventory")
        index = read_json(index_file)
        for row in index["assets"]:
            path = inside(root, row["path"])
            sha = digest(path)
            row["sha256"] = sha
            preserved = native / "images" / f"{sha}.iwi"
            preserved.parent.mkdir(parents=True, exist_ok=True)
            if not preserved.is_file():
                shutil.copyfile(path, preserved)
            row["native_uri"] = preserved.relative_to(output).as_posix()
            try:
                image = prepare_image(path, ipak["path"])
                row["ktx2"] = image["ktx2"]
                ipak_images[row["name_hash"]].append(image)
                manifest["images"].append({"name_hash": row["name_hash"], "data_hash": row["data_hash"], "source": ipak["path"], "source_sha256": ipak["sha256"], "sha256": sha, "uris": image["ktx2"]})
            except AssetError as error:
                issues.append({"category": "image", "source": ipak["path"], "identity": row["name_hash"], "reason": str(error)})
        write_json(native / "ipak" / PurePosixPath(ipak["path"]).stem / "index.json", {"source": ipak["path"], "source_sha256": ipak["sha256"], "assets": index["assets"]})

    for zone in extraction["zones"]:
        root = Path(zone["root"])
        checked_hash(root / "native-pools.json", zone["pool_sha256"], "fresh native pool export")
        destination = native / "zones" / zone["id"]
        shutil.copytree(root, destination, dirs_exist_ok=True, ignore=lambda directory, names: [name for name in names if name in {"native-pools.json", "model_export"} or Path(name).suffix.lower() in CODE_SUFFIXES])
        for path in sorted(root.rglob("*.iwi")):
            try:
                item = prepare_image(path, zone["path"])
                zone_images[(zone["id"], path.relative_to(root).with_suffix("").as_posix().removeprefix("images/"))] = item
            except AssetError as error:
                issues.append({"category": "image", "source": zone["id"], "identity": path.relative_to(root).as_posix(), "reason": str(error)})
        pool_path = destination / "native-pools.json"
        with pool_path.open("w", encoding="utf-8", newline="\n") as pool_output:
            pool_output.write('{"schema_version":1,"zone":' + json.dumps(zone["zone"]) + ',"assets":[')
            first = True
            for asset in pool_assets(root / "native-pools.json"):
                if asset["type"] in NON_ART_TYPES or (asset["type"] == "rawfile" and PurePosixPath(asset["name"]).suffix.lower() in CODE_SUFFIXES):
                    continue
                if not first:
                    pool_output.write(",")
                first = False
                json.dump(asset, pool_output, allow_nan=False, separators=(",", ":"))
                category = categories[asset["type"]]
                category["identities"] += 1
                category["references" if asset["reference"] else "definitions"] += 1
                if "data" in asset:
                    category["structured_native_pool_exports"] += 1
                key = asset_key(zone["id"], asset["name"])
                record = native / "assets" / asset["type"] / f"{key}.json"
                write_json(record, asset)
                identity = {"zone": zone["id"], "type": asset["type"], "type_id": asset["type_id"], "name": asset["name"], "reference": asset["reference"], "dependencies": asset["dependencies"], "indirect_references": asset["indirect_references"], "native_record": record.relative_to(output).as_posix(), "source": zone["path"], "source_sha256": zone["sha256"]}
                identities.append(identity)
                record_sources[identity["native_record"]] = identity
                manifest["native_art"].setdefault(asset["type"], []).append(identity)
                if not asset["reference"]:
                    definitions[(asset["type"], asset["name"])].append(identity)
                    native_records[(zone["id"], asset["type"], asset["name"])] = record
                    if asset["type"] == "image" and "data" in asset:
                        candidates = ipak_images.get(asset["data"]["hash"], [])
                        candidates = [item for item in candidates if item["image"].width == asset["data"]["width"] and item["image"].height == asset["data"]["height"]]
                        unique = {item["sha256"]: item for item in candidates}
                        if (zone["id"], asset["name"]) not in zone_images and len(unique) == 1:
                            zone_images[(zone["id"], asset["name"])] = next(iter(unique.values()))
                if asset["type"] == "image":
                    image = zone_images.get((zone["id"], asset["name"]))
                    if image:
                        manifest["images"].append({**identity, "sha256": image["sha256"], "uris": image["ktx2"]})
            pool_output.write("]}\n")
        print(f"Preserved all fresh native art records for {zone['id']}", flush=True)

    def resolve_record(zone, kind, name):
        name = name.removeprefix(",") if name else name
        local = native_records.get((zone["id"], kind, name))
        if local:
            return local
        candidates = definitions.get((kind, name), [])
        if not candidates:
            return None
        if len(candidates) == 1:
            return inside(output, candidates[0]["native_record"])
        hashes = {digest(inside(output, item["native_record"])) for item in candidates}
        if len(hashes) == 1:
            return inside(output, candidates[0]["native_record"])
        return None

    material_cache = {}
    for zone in extraction["zones"]:
        def image_lookup(name, transfer="linear", source_id=None):
            name = name.removeprefix(",") if name else name
            image = zone_images.get((source_id or zone["id"], name))
            if image is None:
                candidates = {item["sha256"]: item for (source_zone, source_name), item in zone_images.items() if source_name == name}
                if len(candidates) != 1:
                    return None
                image = next(iter(candidates.values()))
            uri = image["ktx2"].get(transfer)
            return {**image, "uri": uri} if uri else None

        def material_lookup(name, document, directory):
            for index, material in enumerate(document.data["materials"]):
                if material.get("name") == name:
                    return index
            record_path = resolve_record(zone, "material", name)
            cache_key = str(record_path)
            if record_path and cache_key not in material_cache:
                asset = read_json(record_path)
                source_id = record_sources[record_path.relative_to(output).as_posix()]["zone"]
                source_zone = next(item for item in extraction["zones"] if item["id"] == source_id)
                data = {**asset["data"], "_source_root": Path(source_zone["root"]), "_zone": source_id}
                try:
                    material_cache[cache_key] = prepare_material(data, lambda name, transfer="linear": image_lookup(name, transfer, source_id), output)
                    target = output / "prepared" / "materials" / f"{asset_key(source_id, name)}.json"
                    write_json(target, material_cache[cache_key])
                    manifest["materials"].append({"name": name, "zone": source_id, "uri": target.relative_to(output).as_posix(), "native_record": record_path.relative_to(output).as_posix()})
                except AssetError as error:
                    issues.append({"category": "material", "zone": source_id, "identity": name, "reason": str(error)})
                    material_cache[cache_key] = {"name": name, "extras": {"native_record": record_path.relative_to(output).as_posix(), "preparation_incomplete": str(error)}}
            if record_path:
                material = json.loads(json.dumps(material_cache[cache_key]))
            else:
                material = {"name": name, "extras": {"preparation_incomplete": "Unresolved original material definition"}}
                issues.append({"category": "material", "zone": zone["id"], "identity": name, "reason": "Unresolved original material definition"})
            document.data["materials"].append(material)
            return len(document.data["materials"]) - 1

        root = Path(zone["root"])
        for asset in pool_assets(root / "native-pools.json"):
            if asset["reference"] or "data" not in asset:
                continue
            data = asset["data"]
            key = asset_key(zone["id"], asset["name"])
            base = {"name": asset["name"], "zone": zone["id"], "source": zone["path"], "source_sha256": zone["sha256"], "native_record": f"native/assets/{asset['type']}/{key}.json"}
            try:
                if asset["type"] == "material":
                    material_lookup(asset["name"], GltfDocument(asset["name"]), output / "prepared" / "materials")
                elif asset["type"] == "xanim":
                    target = output / "prepared" / "animations" / f"{key}.gltf"
                    metadata = prepare_animation(data, root, target)
                    manifest["animations"].append({**base, "uri": target.relative_to(output).as_posix(), "curves_uri": target.with_suffix(".native.json").relative_to(output).as_posix(), **metadata})
                elif asset["type"] == "xmodel":
                    lods = []
                    for lod_index, lod in enumerate(data["lods"]):
                        target = output / "prepared" / "models" / f"{key}-lod{lod_index}.gltf"
                        prepare_model(data, target, material_lookup, lod_index)
                        lods.append({"uri": target.relative_to(output).as_posix(), "lod": lod_index, "distance": lod["distance"] * 0.0254, "native_distance": lod["distance"]})
                    manifest["models"].append({**base, "uri": lods[0]["uri"] if lods else None, "lods": lods})
                elif asset["type"] == "gfxworld":
                    target = output / "prepared" / "worlds" / f"{key}.gltf"
                    prepare_world(data, root, target, material_lookup)
                    manifest["worlds"].append({**base, "uri": target.relative_to(output).as_posix(), "static_models": data["static_models"], "hero_lights": data["hero_lights"], "skybox_model": data["skybox_model"]})
            except (AssetError, ValueError, IndexError, KeyError) as error:
                issues.append({"category": asset["type"], "zone": zone["id"], "identity": asset["name"], "reason": str(error)})
        print(f"Prepared geometry and animation art for {zone['id']}", flush=True)

    for row in extraction["containers"]:
        original = inside(install, row["path"])
        if row["kind"] == "soundbank":
            checked_hash(original, row["sha256"], "original sound bank")
            bank = export_sound_bank(original, install, output)
            write_json(native / "audio" / f"{original.name}.json", bank)
            manifest["audio"].extend({"source": row["path"], **asset} for asset in bank["assets"])
        elif row["kind"] == "cinematic":
            target = output / "cinematics" / original.name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original, target)
            checked_hash(target, row["sha256"], "original cinematic")
            manifest["cinematics"].append({**row, "uri": target.relative_to(output).as_posix()})
        elif row["kind"] == "loose_image":
            checked_hash(original, row["sha256"], "original loose image")
            if original.suffix.lower() == ".iwi":
                prepared = prepare_image(original, row["path"])
                manifest["images"].append({"name": original.name, "source": row["path"], "sha256": row["sha256"], "uris": prepared["ktx2"]})
            else:
                decoded = Image.open(original).convert("RGBA")
                image = IwiImage(decoded.width, decoded.height, 1, 1, 1, 2, 2.2, (0,) * 16, (decoded.tobytes(),))
                target = output / "images" / f"{row['sha256'][:24]}-srgb.ktx2"
                write_ktx2(target, image, srgb=True)
                manifest["images"].append({"name": original.name, "source": row["path"], "sha256": row["sha256"], "uris": {"srgb": target.relative_to(output).as_posix()}})
    write_json(native / "vertex-declarations.json", NATIVE_VERTEX_DECLARATIONS)
    write_json(native / "containers.json", {"schema_version": 1, "containers": extraction["containers"]})
    write_json(native / "asset-index.json", {"schema_version": 1, "assets": identities})
    write_json(output / "prepared" / "index.json", manifest)
    report = {"schema_version": 1, "tool": extraction["tool"], "zones": [{key: value for key, value in zone.items() if key != "root"} for zone in extraction["zones"]], "categories": {name: dict(counts) for name, counts in sorted(categories.items())}, "prepared_counts": {key: len(value) for key, value in manifest.items() if isinstance(value, list)}, "unprepared_art": issues, "runtime_pack_complete": False, "remaining_runtime_decoders": ["Native multi-UV/vertex-controlled material bakes for unsupported shader families", "Native dynamic UI, destructible/glass and remaining art pool structured decoding"]}
    write_json(output / "build-report.json", report)
    return report


def dependency_closure(pack: Path) -> dict:
    issues = []
    counts = collections.Counter()
    for path in sorted(pack.rglob("*")):
        if path.is_symlink():
            issues.append({"file": path.relative_to(pack).as_posix(), "reason": "Pack must not contain external symlinks"})
            continue
        if not path.is_file():
            continue
        counts[path.suffix.lower()] += 1
        try:
            if path.suffix.lower() == ".ktx2":
                validate_ktx2(path)
            elif path.suffix.lower() == ".gltf":
                gltf = read_json(path)
                for item in gltf.get("buffers", []) + gltf.get("images", []):
                    uri = item.get("uri", "")
                    if not uri or uri.startswith("data:"):
                        continue
                    target = reference_path(pack, path.parent, uri)
                    if not target.is_file():
                        issues.append({"file": path.relative_to(pack).as_posix(), "uri": uri, "reason": "Missing glTF dependency"})
                for material in gltf.get("materials", []):
                    surface = material.get("extras", {}).get("surface")
                    if surface:
                        for channel in surface.get("channels", []):
                            if channel.get("layer", 0) != 0 or channel.get("role") not in {"color", "normal", "response", "emissive", "reflectance"}:
                                issues.append({"file": path.relative_to(pack).as_posix(), "reason": "Unsupported StandardMaterial channel"})
                            target = inside(pack, channel["uri"])
                            if not target.is_file():
                                issues.append({"file": path.relative_to(pack).as_posix(), "uri": channel["uri"], "reason": "Missing StandardMaterial texture"})
            elif path.name == "native-pools.json":
                for asset in pool_assets(path):
                    data = asset.get("data", {})
                    if asset["type"] == "gfxworld":
                        for key in ("stream0", "stream1", "indices"):
                            if not inside(path.parent, data[key]).is_file():
                                issues.append({"file": path.relative_to(pack).as_posix(), "uri": data[key], "reason": "Missing native world stream"})
        except (AssetError, KeyError, ValueError) as error:
            issues.append({"file": path.relative_to(pack).as_posix(), "reason": str(error)})
    return {"complete": not issues, "file_counts": dict(sorted(counts.items())), "issues": issues}


def command_line() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("inventory", "extract", "build"):
        command = sub.add_parser(name)
        command.add_argument("--install", type=Path, required=True)
        command.add_argument("--work", type=Path)
        if name in {"extract", "build"}:
            command.add_argument("--oat-source", type=Path)
        if name == "build":
            command.add_argument("--output", type=Path, required=True)
    image = sub.add_parser("texture")
    image.add_argument("input", type=Path)
    image.add_argument("output", type=Path)
    image.add_argument("--srgb", action="store_true")
    closure = sub.add_parser("closure")
    closure.add_argument("pack", type=Path)
    arguments = sys.argv[1:]
    if arguments and arguments[0].startswith("--"):
        arguments.insert(0, "build")
    return parser.parse_args(arguments)


def main() -> int:
    args = command_line()
    try:
        if args.command == "texture":
            if args.input.suffix.lower() != ".iwi":
                raise AssetError("Texture input must be an original IWI, never DDS or an existing export")
            write_ktx2(args.output, read_iwi(args.input), srgb=args.srgb)
            print(json.dumps(validate_ktx2(args.output)))
            return 0
        if args.command == "closure":
            result = dependency_closure(args.pack.resolve())
            print(json.dumps(result, separators=(",", ":")))
            return 0 if result["complete"] else 2
        install = args.install.resolve()
        work = args.work.resolve() if args.work else Path(tempfile.mkdtemp(prefix="origins-original-art-"))
        work.mkdir(parents=True, exist_ok=True)
        inventory = installation_zones(install)
        write_json(work / "installed-art-containers.json", inventory)
        print(f"Discovered {len(inventory['zones'])} original fastfiles and {len(inventory['containers'])} installed art containers", flush=True)
        if args.command == "inventory":
            return 0
        extraction = extract_native(install, work, inventory, oat_source=args.oat_source)
        write_json(work / "native-extraction.json", extraction)
        if args.command == "extract":
            return 0
        output = args.output.resolve()
        report = native_pack(extraction, install, output)
        closure = dependency_closure(output)
        write_json(output / "dependency-closure.json", closure)
        print(json.dumps({"output": str(output), "native_asset_identities": sum(value["identities"] for value in report["categories"].values()), "runtime_pack_complete": report["runtime_pack_complete"], "closure_complete": closure["complete"], "unprepared_art_count": len(report["unprepared_art"]), "remaining_runtime_decoders": report["remaining_runtime_decoders"]}, separators=(",", ":")))
        return 0 if report["runtime_pack_complete"] and closure["complete"] else 2
    except (AssetError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"Asset rebuild failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
