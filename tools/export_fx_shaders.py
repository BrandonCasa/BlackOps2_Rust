"""Compile recovered particle DXBC instructions into static WESL entrypoints."""
import argparse
import ctypes as C
import hashlib
import json
import re
import struct
from pathlib import Path

import dxbc_to_wesl as dxbc


def disassemble(path):
    raw = path.read_bytes()
    dll = C.WinDLL("d3dcompiler_47.dll")
    function = dll.D3DDisassemble
    function.argtypes = [C.c_void_p, C.c_size_t, C.c_uint, C.c_char_p, C.POINTER(C.c_void_p)]
    function.restype = C.c_long
    blob = C.c_void_p()
    assert function(raw, len(raw), 0, None, C.byref(blob)) == 0, path
    table = C.cast(blob, C.POINTER(C.POINTER(C.c_void_p))).contents
    pointer = C.WINFUNCTYPE(C.c_void_p, C.c_void_p)(table[3])(blob)
    size = C.WINFUNCTYPE(C.c_size_t, C.c_void_p)(table[4])(blob)
    text = C.string_at(pointer, size).decode().rstrip("\0")
    C.WINFUNCTYPE(C.c_ulong, C.c_void_p)(table[2])(blob)
    return text


def icb(path):
    data = path.read_bytes()
    for offset in struct.unpack_from("<" + "I" * struct.unpack_from("<I", data, 28)[0], data, 32):
        if data[offset:offset + 4] not in (b"SHDR", b"SHEX"):
            continue
        size = struct.unpack_from("<I", data, offset + 4)[0]
        words = struct.unpack_from("<" + "I" * (size // 4), data, offset + 8)
        i = 2
        while i < len(words):
            if words[i] & 0x7ff == 53:
                length = words[i + 1]
                if words[i] >> 11 == 3:
                    return [dxbc.literal(words[j:j + 4]) for j in range(i + 2, i + length, 4)]
            else:
                length = words[i] >> 24 & 127
            assert length
            i += length
    return []


class ParticlePort(dxbc.Port):
    code_constants = {
        20: 'frame.scene.hdr_control0', 26: 'frame.scene.fog_color',
        27: 'frame.scene.fog_consts', 28: 'frame.scene.fog_consts2',
        29: 'frame.scene.sun_fog_dir', 30: 'frame.scene.sun_fog_color',
        31: 'frame.scene.sun_fog', 58: 'frame.z_near', 95: 'frame.render_target_size',
        **{i: f'frame.scene.view_projection_rows[{i-36}]' for i in range(36,40)},
    }

    def operand(self, arg, integer=False):
        match = re.fullmatch(r"(-?)(\|?)(icb\[r(\d+)\.([xyzw]) \+ (\d+)\])\.([xyzw]+)(\|?)", arg)
        if match:
            neg, absolute, _, index, component, offset, swizzle, _ = match.groups()
            value = f"original_icb[i32(bitcast<u32>(r{index}.{component})) + {offset}]"
            value = f"({value}).{swizzle.ljust(4, swizzle[-1])}"
            if integer:
                value = f"bitcast<vec4<i32>>({value})"
            if absolute:
                value = f"abs({value})"
            return f"-({value})" if neg else value
        match = re.fullmatch(r"(-?)(\|?)cb1\[(\d+)\](?:\.([xyzw]+))?(\|?)", arg)
        if match:
            neg, absolute, index, swizzle, _ = match.groups()
            index = int(index)
            if 4 <= index <= 7:
                value = f"source_clip_row({index - 4}u)"
            elif index == 16:
                value = "vec4<f32>(0.5, -0.5, 1.0, 1.0)"
            elif index == 17:
                value = "vec4<f32>(0.5, 0.5, 0.0, 0.0)"
            else:
                assert 46 <= index <= 53, (self.stage, arg)
                value = f"material.source[{index - 46}]"
            if swizzle:
                value = f"({value}).{swizzle.ljust(4, swizzle[-1])}"
            if integer:
                value = f"bitcast<vec4<i32>>({value})"
            if absolute:
                value = f"abs({value})"
            return f"-({value})" if neg else value
        match = re.fullmatch(r"(-?)(\|?)cb2\[(\d+)\](?:\.([xyzw]+))?(\|?)", arg)
        if match:
            neg, absolute, index, swizzle, _ = match.groups()
            index = int(index)
            value = "frame.scene.game_time" if index == 4 else f"material.source[{8 + index}]"
            assert index <= 7, arg
            if swizzle:
                value = f"({value}).{swizzle.ljust(4, swizzle[-1])}"
            if integer:
                value = f"bitcast<vec4<i32>>({value})"
            if absolute:
                value = f"abs({value})"
            return f"-({value})" if neg else value
        if arg.lstrip("-|").startswith("v") and self.stage == "vs":
            match = re.fullmatch(r"(-?)(\|?)v(\d+)(?:\.([xyzw]+))?(\|?)", arg)
            if match:
                neg, absolute, index, swizzle, _ = match.groups()
                name, number, _, _ = self.inputs[int(index)]
                field = self.input_fields[(name, number)]
                value = f"input.{field}" if field in ("position", "color") else f"vec4<f32>(input.uv, 0.0, 0.0)" if field == "uv" else f"source_normal(input.{field})"
                if swizzle:
                    value = f"({value}).{swizzle.ljust(4, swizzle[-1])}"
                if absolute:
                    value = f"abs({value})"
                return f"-({value})" if neg else value
        return super().operand(arg, integer)


HEADER = """// Generated from the original DXBC. See tools/export_fx_shaders.py.
import package::rendering::native_fx::NativeFxFrame;
struct SourceTransform { world_rows: array<vec4<f32>, 4>, outdoor_rows: array<vec4<f32>, 3> }
struct SourceConstants {
    feather_parms: vec4<f32>, additive_parms: array<vec4<f32>, 4>,
    alpha_dissolve_parms: vec4<f32>, source: array<vec4<f32>, 16>, alpha_test: vec4<f32>,
}
@group(0) @binding(0) var<uniform> frame: NativeFxFrame;
@group(0) @binding(1) var float_z: texture_2d<f32>;
@group(0) @binding(2) var float_z_sampler: sampler;
@group(1) @binding(0) var<uniform> transform: SourceTransform;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var<uniform> material: SourceConstants;
@group(2) @binding(3) var secondary_map: texture_2d<f32>;
@group(2) @binding(4) var secondary_sampler: sampler;
@group(3) @binding(0) var resolved_scene: texture_2d<f32>;
@group(3) @binding(1) var resolved_sampler: sampler;
// Soft optical flare sprites, filtered within their own atlas tile.
fn soft_flare_sprite(uv: vec2<f32>, local_uv: vec2<f32>) -> vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(secondary_map));
    let extent = max(fwidth(uv) / max(fwidth(local_uv), vec2<f32>(0.000001)), texel);
    let tile_min = uv - local_uv * extent;
    let low = tile_min + texel * 0.5;
    let high = tile_min + extent - texel * 0.5;
    let radius = max(texel * 1.5, extent * 0.015);
    let weights = array<f32, 3>(1.0, 2.0, 1.0);
    var blurred = vec4<f32>(0.0);
    for (var y = 0; y < 3; y += 1) {
        for (var x = 0; x < 3; x += 1) {
            let offset = vec2<f32>(f32(x - 1), f32(y - 1)) * radius;
            blurred += textureSampleLevel(secondary_map, secondary_sampler,
                clamp(uv + offset, low, high), 0.0) * weights[x] * weights[y];
        }
    }
    let edge = min(local_uv, vec2<f32>(1.0) - local_uv);
    let feather = smoothstep(vec2<f32>(0.0), vec2<f32>(0.12), edge);
    return blurred * (feather.x * feather.y / 16.0);
}
struct SourceInput {
    @location(0) position: vec4<f32>, @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>, @location(3) normal: u32, @location(4) tangent: u32,
}
fn source_normal(packed: u32) -> vec4<f32> {
    return vec4<f32>(vec3<f32>(vec3<u32>(packed, packed >> 10u, packed >> 20u) & vec3<u32>(1023u)) / 1023.0, 1.0);
}
fn source_clip_row(index: u32) -> vec4<f32> {
    let row = frame.scene.view_projection_rows[index];
    return vec4<f32>(
        dot(row, vec4<f32>(transform.world_rows[0].x, transform.world_rows[1].x, transform.world_rows[2].x, transform.world_rows[3].x)),
        dot(row, vec4<f32>(transform.world_rows[0].y, transform.world_rows[1].y, transform.world_rows[2].y, transform.world_rows[3].y)),
        dot(row, vec4<f32>(transform.world_rows[0].z, transform.world_rows[1].z, transform.world_rows[2].z, transform.world_rows[3].z)),
        dot(row, vec4<f32>(transform.world_rows[0].w, transform.world_rows[1].w, transform.world_rows[2].w, transform.world_rows[3].w)));
}
"""


def export(research, checkout):
    roots = research / "evidence/rendering-20261012/source/all"
    output = checkout / "target/reports/fx-complete-shaders"
    output.mkdir(parents=True, exist_ok=True)
    def find(kind, name):
        return next((roots / zone / kind / name for zone in ("zm_tomb_patch", "zm_tomb", "patch_zm", "common_zm", "code_post_gfx_zm") if (roots / zone / kind / name).is_file()))
    catalog = json.loads((checkout / "gameplay/origins/fx_catalog.json").read_text())
    names = {v["material"].lstrip(",") for effect in catalog["effects"].values() for e in effect["elements"] if e["type"] <= 6 for v in e["visuals"] if v and v.get("material")}
    families = {}
    for name in names:
        data = json.loads((checkout / "target/assets/materials" / (name.replace("*", "_") + ".json")).read_text())
        families.setdefault(data["techniqueSet"], []).append(data)
    current = (checkout / "src/native_fx/material.rs").read_text().split("let program = match")[1].split("if self.camera_region")[0]
    supported = set(re.findall(r'"([^"]+)"', current))
    selected = sorted(name for name in families if name not in supported and not name.startswith("particlecloud_"))
    shader = HEADER
    contracts = []
    for index, family in enumerate(selected):
        techset = find("techsets", family + ".techset").read_text()
        technique_name = re.search(r'"emissive":\s*(?:"[^"\n]+":\s*)*(\w+);', techset)[1]
        technique = find("techniques", technique_name + ".tech").read_text()
        records = {}
        constants = []
        for stage, name in re.findall(r'(vertex|pixel)Shader\s+4.0\s+"([^"]+)"', technique):
            prefix = "vs" if stage == "vertex" else "ps"
            path = find("shader_bin", prefix + "_" + name + ".cso")
            asm = output / (prefix + "_" + name + ".asm")
            asm.write_text(disassemble(path))
            records[prefix] = {"path": str(path), "asm": str(asm), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
        vertex, pixel = (ParticlePort(records[s], s) for s in ("vs", "ps"))
        vertex.body = re.sub(r'dcl_immediateConstantBuffer\s*\{.*?(?=\ndcl_)', '', vertex.body, flags=re.S)
        vertex.input_fields = {}
        for destination, component, field in re.findall(r'vertex\.(\w+)(?:\[(\d+)\])?\s*=\s*code\.(\w+);', technique):
            vertex.input_fields[(destination.upper(), int(component or 0))] = {"texcoord":"uv"}.get(field, field)
        # DXBC semantics use NORMAL0 even when the source declaration labels
        # the packed normal as the engine's generic normal channel.
        for register, (name, number, _, _) in vertex.inputs.items():
            if (name, number) not in vertex.input_fields:
                vertex.input_fields[(name, number)] = {"POSITION":"position", "COLOR":"color", "NORMAL":"normal", "TEXCOORD":"uv"}[name]
        textures = []
        pixel.texture_vars, pixel.sampler_vars = {}, {}
        for register, (name, _) in pixel.resources.items():
            if not register.startswith("t"):
                continue
            assignment = re.search(r'\b' + re.escape(name) + r'\s*=\s*(material|sampler)\.(\w+);', technique)
            if name == "floatZSampler":
                texture, sampler = "float_z", "float_z_sampler"
            elif assignment and assignment.groups() == ("sampler", "resolvedPostSun"):
                texture, sampler = "resolved_scene", "resolved_sampler"
            else:
                assert assignment and assignment[1] == "material", (family, name)
                textures.append(assignment[2])
                assert len(textures) <= 2, family
                texture, sampler = ("color_map", "color_sampler") if len(textures) == 1 else ("secondary_map", "secondary_sampler")
            pixel.texture_vars[register] = texture
            for sr, (sn, _) in pixel.resources.items():
                if sr.startswith("s") and sn == name:
                    pixel.sampler_vars[sr] = sampler
        for stage in (vertex, pixel):
            for shader_name, material_name in re.findall(r'\b(\w+)\s*=\s*material\.(\w+);', technique):
                declaration = re.search(r'//\s+float(\d?)\s+' + re.escape(shader_name) + r';\s*// Offset:\s*(\d+) Size:\s*(\d+)([^\n]*)', stage.text)
                if not declaration or "unused" in declaration[4]:
                    continue
                # Identify the declaring constant buffer's native register.
                before = stage.text[:declaration.start()]
                buffer = re.findall(r'// cbuffer ([\w$]+)', before)[-1]
                binding = re.search(r'^//\s+' + re.escape(buffer) + r'\s+cbuffer\s+\w+\s+\w+\s+cb(\d+)', stage.text, re.M)
                assert binding, (family, buffer)
                cb, offset, width = int(binding[1]), int(declaration[2]), int(declaration[3]) // 4
                if cb == 1:
                    offset -= 46 * 16
                else:
                    assert cb == 2, (family, buffer)
                    offset += 8 * 16
                assert 0 <= offset < 16 * 16 and 1 <= width <= 4, (family, offset, width)
                value = {"offset": offset // 4, "width": width, "name": material_name}
                if value not in constants:
                    constants.append(value)
        varying = f"struct SourceVaryings{index} {{\n    @builtin(position) position: vec4<f32>,\n"
        for i in vertex.outputs:
            if i != vertex.output_builtin and re.search(r'\bv' + str(i) + r'\b', pixel.body):
                varying += f"    @location({i}) v{i}: vec4<f32>,\n"
        if "superflare" in family:
            varying += "    @location(4) flare_uv: vec2<f32>,\n"
        varying += "}\n"
        body = vertex.translate()
        values = icb(Path(records["vs"]["path"]))
        if values:
            body = "    let original_icb = array<vec4<f32>, " + str(len(values)) + ">(" + ", ".join(values) + ");\n" + body
        body += f"\n    var result: SourceVaryings{index};\n    result.position = o{vertex.output_builtin};"
        for i in vertex.outputs:
            if f" v{i}:" in varying:
                body += f"\n    result.v{i} = o{i};"
        if "superflare" in family:
            body += "\n    result.flare_uv = input.uv;"
        body += "\n    return result;"
        pixel_body = pixel.translate()
        if "superflare" in family:
            original_sample = "textureSample(secondary_map, secondary_sampler, ((input.v3).xyxx).xy)"
            assert pixel_body.count(original_sample) == 1, family
            pixel_body = pixel_body.replace(original_sample, "soft_flare_sprite(input.v3.xy, input.flare_uv)")
        pixel_body += "\n    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }\n    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }\n    return o0;"
        shader += varying + f"@vertex\nfn source_vertex_{index}(input: SourceInput) -> SourceVaryings{index} {{\n{body}\n}}\n"
        shader += f"@fragment\nfn source_fragment_{index}(input: SourceVaryings{index}) -> @location(0) vec4<f32> {{\n{pixel_body}\n}}\n"
        provenance = {stage: {"source": Path(record["path"]).relative_to(research).as_posix(),
                              "sha256": record["sha256"]} for stage, record in records.items()}
        contracts.append({"family": family, "textures": textures, "constants": constants, "shaders": provenance,
                          "distortion": "resolved_scene" in pixel.texture_vars.values(), "superflare": "superflare" in family})
        print(index, family, len(body.splitlines()), textures, constants)
    (checkout / "src/rendering/native_fx_source.wesl").write_text(shader)
    (checkout / "gameplay/origins/fx_shaders.json").write_text(json.dumps(contracts, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--research", type=Path, required=True)
    args = parser.parse_args()
    export(args.research, Path(__file__).resolve().parent.parent)
