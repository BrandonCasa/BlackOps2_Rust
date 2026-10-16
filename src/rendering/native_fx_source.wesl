// Generated from the original DXBC. See tools/export_fx_shaders.py.
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
struct SourceVaryings0 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) v4: vec4<f32>,
    @location(5) v5: vec4<f32>,
}
@vertex
fn source_vertex_0(input: SourceInput) -> SourceVaryings0 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var o5: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    var r4: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb1[4].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((source_clip_row(0u)).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb1[5].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((source_clip_row(1u)).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb1[6].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((source_clip_row(2u)).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb1[7].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((source_clip_row(3u)).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // mov o0.xyzw, r1.xyzw
    let instruction_6 = (r1).xyzw;
    o0 = instruction_6;
    // mov o1.xyzw, v1.zzzw
    let instruction_7 = (input.color).zzzw;
    o1 = instruction_7;
    // mov o2.z, r1.z
    let instruction_8 = (r1).zzzz;
    o2 = vec4<f32>(o2.x, o2.y, instruction_8.z, o2.w);
    // mov o2.xy, v2.xyxx
    let instruction_9 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o2 = vec4<f32>(instruction_9.x, instruction_9.y, o2.z, o2.w);
    // mov r0.xzw, l(1.000000,0,1.000000,1.000000)
    let instruction_10 = vec4<f32>(1.0, 0.0, 1.0, 1.0);
    r0 = vec4<f32>(instruction_10.x, r0.y, instruction_10.z, instruction_10.w);
    // mov r0.y, cb1[16].y
    let instruction_11 = (vec4<f32>(0.5, -0.5, 1.0, 1.0)).yyyy;
    r0 = vec4<f32>(r0.x, instruction_11.y, r0.z, r0.w);
    // mul r0.xyzw, r0.xyzw, cb2[1].xxxx
    let instruction_12 = ((r0).xyzw) * ((material.source[9]).xxxx);
    r0 = instruction_12;
    // mov r2.xzw, cb1[16].xxzw
    let instruction_13 = (vec4<f32>(0.5, -0.5, 1.0, 1.0)).xxzw;
    r2 = vec4<f32>(instruction_13.x, r2.y, instruction_13.z, instruction_13.w);
    // mov r2.y, l(-1.000000)
    let instruction_14 = vec4<f32>(-1.0);
    r2 = vec4<f32>(r2.x, instruction_14.y, r2.z, r2.w);
    // mul r0.xyzw, r0.xyzw, r2.xyzw
    let instruction_15 = ((r0).xyzw) * ((r2).xyzw);
    r0 = instruction_15;
    // add r2.xyz, v4.zxyz, v4.zxyz
    let instruction_16 = ((source_normal(input.tangent)).zxyz) + ((source_normal(input.tangent)).zxyz);
    r2 = vec4<f32>(instruction_16.x, instruction_16.y, instruction_16.z, r2.w);
    // ge r3.xyz, v4.zxyz, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_17 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((source_normal(input.tangent)).zxyz) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r3 = vec4<f32>(instruction_17.x, instruction_17.y, instruction_17.z, r3.w);
    // mad r4.xyz, v4.zxyz, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_18 = fma((source_normal(input.tangent)).zxyz, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r4 = vec4<f32>(instruction_18.x, instruction_18.y, instruction_18.z, r4.w);
    // movc r2.xyz, r3.xyzx, r4.xyzx, r2.xyzx
    let instruction_19 = select((r2).xyzx, (r4).xyzx, bitcast<vec4<u32>>((r3).xyzx) != vec4<u32>(0u));
    r2 = vec4<f32>(instruction_19.x, instruction_19.y, instruction_19.z, r2.w);
    // dp3 r3.x, r2.yzxy, cb1[4].xyzx
    let instruction_20 = vec4<f32>(dot(((r2).yzxy).xyz, ((source_clip_row(0u)).xyzx).xyz));
    r3 = vec4<f32>(instruction_20.x, r3.y, r3.z, r3.w);
    // dp3 r3.y, r2.yzxy, cb1[5].xyzx
    let instruction_21 = vec4<f32>(dot(((r2).yzxy).xyz, ((source_clip_row(1u)).xyzx).xyz));
    r3 = vec4<f32>(r3.x, instruction_21.y, r3.z, r3.w);
    // dp3 r3.z, r2.yzxy, cb1[6].xyzx
    let instruction_22 = vec4<f32>(dot(((r2).yzxy).xyz, ((source_clip_row(2u)).xyzx).xyz));
    r3 = vec4<f32>(r3.x, r3.y, instruction_22.z, r3.w);
    // dp3 r3.w, r2.yzxy, cb1[7].xyzx
    let instruction_23 = vec4<f32>(dot(((r2).yzxy).xyz, ((source_clip_row(3u)).xyzx).xyz));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_23.w);
    // mul r0.xyzw, r0.xyzw, r3.xyzw
    let instruction_24 = ((r0).xyzw) * ((r3).xyzw);
    r0 = instruction_24;
    // mul o3.xyzw, r0.xyzw, v1.xxxx
    let instruction_25 = ((r0).xyzw) * ((input.color).xxxx);
    o3 = instruction_25;
    // add r0.xyz, v3.yzxy, v3.yzxy
    let instruction_26 = ((source_normal(input.normal)).yzxy) + ((source_normal(input.normal)).yzxy);
    r0 = vec4<f32>(instruction_26.x, instruction_26.y, instruction_26.z, r0.w);
    // ge r3.xyz, v3.yzxy, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_27 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((source_normal(input.normal)).yzxy) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r3 = vec4<f32>(instruction_27.x, instruction_27.y, instruction_27.z, r3.w);
    // mad r4.xyz, v3.yzxy, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_28 = fma((source_normal(input.normal)).yzxy, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r4 = vec4<f32>(instruction_28.x, instruction_28.y, instruction_28.z, r4.w);
    // movc r0.xyz, r3.xyzx, r4.xyzx, r0.xyzx
    let instruction_29 = select((r0).xyzx, (r4).xyzx, bitcast<vec4<u32>>((r3).xyzx) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_29.x, instruction_29.y, instruction_29.z, r0.w);
    // mul r3.xyz, r0.xyzx, r2.xyzx
    let instruction_30 = ((r0).xyzx) * ((r2).xyzx);
    r3 = vec4<f32>(instruction_30.x, instruction_30.y, instruction_30.z, r3.w);
    // mad r0.xyz, r2.zxyz, r0.yzxy, -r3.xyzx
    let instruction_31 = fma((r2).zxyz, (r0).yzxy, -((r3).xyzx));
    r0 = vec4<f32>(instruction_31.x, instruction_31.y, instruction_31.z, r0.w);
    // dp3 r2.x, r0.xyzx, cb1[4].xyzx
    let instruction_32 = vec4<f32>(dot(((r0).xyzx).xyz, ((source_clip_row(0u)).xyzx).xyz));
    r2 = vec4<f32>(instruction_32.x, r2.y, r2.z, r2.w);
    // dp3 r2.y, r0.xyzx, cb1[5].xyzx
    let instruction_33 = vec4<f32>(dot(((r0).xyzx).xyz, ((source_clip_row(1u)).xyzx).xyz));
    r2 = vec4<f32>(r2.x, instruction_33.y, r2.z, r2.w);
    // dp3 r2.z, r0.xyzx, cb1[6].xyzx
    let instruction_34 = vec4<f32>(dot(((r0).xyzx).xyz, ((source_clip_row(2u)).xyzx).xyz));
    r2 = vec4<f32>(r2.x, r2.y, instruction_34.z, r2.w);
    // dp3 r2.w, r0.xyzx, cb1[7].xyzx
    let instruction_35 = vec4<f32>(dot(((r0).xyzx).xyz, ((source_clip_row(3u)).xyzx).xyz));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_35.w);
    // mul r0.xyzw, cb2[1].yyyy, l(-1.000000, 1.000000, -1.000000, -1.000000)
    let instruction_36 = ((material.source[9]).yyyy) * (vec4<f32>(-1.0, 1.0, -1.0, -1.0));
    r0 = instruction_36;
    // mul r0.xyzw, r0.xyzw, cb1[16].xyzw
    let instruction_37 = ((r0).xyzw) * ((vec4<f32>(0.5, -0.5, 1.0, 1.0)).xyzw);
    r0 = instruction_37;
    // mul r0.xyzw, r2.xyzw, r0.xyzw
    let instruction_38 = ((r2).xyzw) * ((r0).xyzw);
    r0 = instruction_38;
    // mul o4.xyzw, r0.xyzw, v1.yyyy
    let instruction_39 = ((r0).xyzw) * ((input.color).yyyy);
    o4 = instruction_39;
    // mul r0.xyzw, r1.wwww, cb1[17].xyzw
    let instruction_40 = ((r1).wwww) * ((vec4<f32>(0.5, 0.5, 0.0, 0.0)).xyzw);
    r0 = instruction_40;
    // mad r0.xyzw, r1.xyzw, cb1[16].xyzw, r0.xyzw
    let instruction_41 = fma((r1).xyzw, (vec4<f32>(0.5, -0.5, 1.0, 1.0)).xyzw, (r0).xyzw);
    r0 = instruction_41;
    // mov o5.zw, r0.zzzw
    let instruction_42 = (r0).zzzw;
    o5 = vec4<f32>(o5.x, o5.y, instruction_42.z, instruction_42.w);
    // div r0.xy, r0.xyxx, r0.wwww
    let instruction_43 = ((r0).xyxx) / ((r0).wwww);
    r0 = vec4<f32>(instruction_43.x, instruction_43.y, r0.z, r0.w);
    // mad o5.xy, r0.xyxx, cb2[2].zwzz, cb2[2].xyxx
    let instruction_44 = fma((r0).xyxx, (material.source[10]).zwzz, (material.source[10]).xyxx);
    o5 = vec4<f32>(instruction_44.x, instruction_44.y, o5.z, o5.w);
    var result: SourceVaryings0;
    result.position = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.v4 = o4;
    result.v5 = o5;
    return result;
}
@fragment
fn source_fragment_0(input: SourceVaryings0) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    // sample r0.xyzw, v2.xyxx, t1.xyzw, s1
    let instruction_0 = textureSample(color_map, color_sampler, ((input.v2).xyxx).xy);
    r0 = instruction_0;
    // mad r0.xy, r0.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_1 = fma((r0).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r0 = vec4<f32>(instruction_1.x, instruction_1.y, r0.z, r0.w);
    // mul o0.w, r0.w, v1.w
    let instruction_2 = ((r0).wwww) * ((input.v1).wwww);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_2.w);
    // div r0.xy, r0.xyxx, v5.wwww
    let instruction_3 = ((r0).xyxx) / ((input.v5).wwww);
    r0 = vec4<f32>(instruction_3.x, instruction_3.y, r0.z, r0.w);
    // mul r0.zw, v0.xxxy, cb0[95].zzzw
    let instruction_4 = ((input.position).xxxy) * ((frame.render_target_size).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_4.z, instruction_4.w);
    // mad r1.xy, r0.xxxx, v3.xyxx, r0.zwzz
    let instruction_5 = fma((r0).xxxx, (input.v3).xyxx, (r0).zwzz);
    r1 = vec4<f32>(instruction_5.x, instruction_5.y, r1.z, r1.w);
    // mad r0.xy, r0.yyyy, v4.xyxx, r1.xyxx
    let instruction_6 = fma((r0).yyyy, (input.v4).xyxx, (r1).xyxx);
    r0 = vec4<f32>(instruction_6.x, instruction_6.y, r0.z, r0.w);
    // sample r1.xyzw, r0.zwzz, t0.xyzw, s0
    let instruction_7 = textureSample(resolved_scene, resolved_sampler, ((r0).zwzz).xy);
    r1 = instruction_7;
    // sample r2.xyzw, r0.xyxx, t2.xyzw, s2
    let instruction_8 = textureSample(float_z, float_z_sampler, ((r0).xyxx).xy);
    r2 = instruction_8;
    // sample r0.xyzw, r0.xyxx, t0.xyzw, s0
    let instruction_9 = textureSample(resolved_scene, resolved_sampler, ((r0).xyxx).xy);
    r0 = instruction_9;
    // max r0.w, |r2.x|, l(0.000000)
    let instruction_10 = max(abs((r2).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_10.w);
    // div r0.w, cb0[58].x, r0.w
    let instruction_11 = ((frame.z_near).xxxx) / ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_11.w);
    // lt r0.w, r0.w, v2.z
    let instruction_12 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).wwww) < ((input.v2).zzzz)));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_12.w);
    // movc r0.xyz, r0.wwww, r1.xyzx, r0.xyzx
    let instruction_13 = select((r0).xyzx, (r1).xyzx, bitcast<vec4<u32>>((r0).wwww) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_13.x, instruction_13.y, instruction_13.z, r0.w);
    // add r1.xyz, v1.xyzx, v1.xyzx
    let instruction_14 = ((input.v1).xyzx) + ((input.v1).xyzx);
    r1 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, r1.w);
    // mul o0.xyz, r0.xyzx, r1.xyzx
    let instruction_15 = ((r0).xyzx) * ((r1).xyzx);
    o0 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, o0.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings1 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
}
@vertex
fn source_vertex_1(input: SourceInput) -> SourceVaryings1 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_6.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_7.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_8.z, o0.w);
    // dp4 o0.w, r1.xyzw, cb0[39].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_9.w);
    // mov o1.xyzw, v1.xyzw
    let instruction_10 = (input.color).xyzw;
    o1 = instruction_10;
    // mov o2.xy, v2.xyxx
    let instruction_11 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o2 = vec4<f32>(instruction_11.x, instruction_11.y, o2.z, o2.w);
    var result: SourceVaryings1;
    result.position = o0;
    result.v1 = o1;
    result.v2 = o2;
    return result;
}
@fragment
fn source_fragment_1(input: SourceVaryings1) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    // sample r0.xyzw, v2.xyxx, t0.xyzw, s0
    let instruction_0 = textureSample(color_map, color_sampler, ((input.v2).xyxx).xy);
    r0 = instruction_0;
    // mul o0.xyzw, r0.xyzw, v1.xyzw
    let instruction_1 = ((r0).xyzw) * ((input.v1).xyzw);
    o0 = instruction_1;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings2 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(3) v3: vec4<f32>,
}
@vertex
fn source_vertex_2(input: SourceInput) -> SourceVaryings2 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_6.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_7.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_8.z, o0.w);
    // dp4 r0.x, r1.xyzw, cb0[39].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_9.x, r0.y, r0.z, r0.w);
    // mov o0.w, r0.x
    let instruction_10 = (r0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_10.w);
    // mul_sat r0.y, r0.x, cb1[46].x
    let instruction_11 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_11.y, r0.z, r0.w);
    // mov o3.z, r0.x
    let instruction_12 = (r0).xxxx;
    o3 = vec4<f32>(o3.x, o3.y, instruction_12.z, o3.w);
    // mul o1.w, r0.y, v1.w
    let instruction_13 = ((r0).yyyy) * ((input.color).wwww);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_13.w);
    // mov o1.xyz, v1.xyzx
    let instruction_14 = (input.color).xyzx;
    o1 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, o1.w);
    // mov o2.xyz, l(0,0,0,0)
    let instruction_15 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, o2.w);
    // mov o3.xy, v2.xyxx
    let instruction_16 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o3 = vec4<f32>(instruction_16.x, instruction_16.y, o3.z, o3.w);
    // mov o3.w, l(1.000000)
    let instruction_17 = vec4<f32>(1.0);
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_17.w);
    var result: SourceVaryings2;
    result.position = o0;
    result.v1 = o1;
    result.v3 = o3;
    return result;
}
@fragment
fn source_fragment_2(input: SourceVaryings2) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mul r0.xy, v0.xyxx, cb0[95].zwzz
    let instruction_0 = ((input.position).xyxx) * ((frame.render_target_size).zwzz);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t1.xyzw, s1
    let instruction_1 = textureSample(float_z, float_z_sampler, ((r0).xyxx).xy);
    r0 = instruction_1;
    // max r0.x, |r0.x|, l(0.000000)
    let instruction_2 = max(abs((r0).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // div r0.x, cb0[58].x, r0.x
    let instruction_3 = ((frame.z_near).xxxx) / ((r0).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -v3.z
    let instruction_4 = ((r0).xxxx) + (-((input.v3).zzzz));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, cb1[46].x
    let instruction_5 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t0.xyzw, s0
    let instruction_6 = textureSample(color_map, color_sampler, ((input.v3).xyxx).xy);
    r1 = instruction_6;
    // add r0.y, r1.w, v1.w
    let instruction_7 = ((r1).wwww) + ((input.v1).wwww);
    r0 = vec4<f32>(r0.x, instruction_7.y, r0.z, r0.w);
    // mul r1.xyz, r1.xyzx, v1.xyzx
    let instruction_8 = ((r1).xyzx) * ((input.v1).xyzx);
    r1 = vec4<f32>(instruction_8.x, instruction_8.y, instruction_8.z, r1.w);
    // mad_sat r0.y, r0.y, cb1[51].x, -cb1[51].x
    let instruction_9 = clamp(fma((r0).yyyy, (material.source[5]).xxxx, -((material.source[5]).xxxx)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_9.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.y
    let instruction_10 = ((r0).xxxx) * ((r0).yyyy);
    r0 = vec4<f32>(instruction_10.x, r0.y, r0.z, r0.w);
    // mul o0.xyz, r0.xxxx, r1.xyzx
    let instruction_11 = ((r0).xxxx) * ((r1).xyzx);
    o0 = vec4<f32>(instruction_11.x, instruction_11.y, instruction_11.z, o0.w);
    // mov o0.w, r0.x
    let instruction_12 = (r0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_12.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings3 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(3) v3: vec4<f32>,
}
@vertex
fn source_vertex_3(input: SourceInput) -> SourceVaryings3 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_6.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_7.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_8.z, o0.w);
    // dp4 r0.x, r1.xyzw, cb0[39].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_9.x, r0.y, r0.z, r0.w);
    // mov o0.w, r0.x
    let instruction_10 = (r0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_10.w);
    // mul_sat r0.y, r0.x, cb1[46].x
    let instruction_11 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_11.y, r0.z, r0.w);
    // mov o3.z, r0.x
    let instruction_12 = (r0).xxxx;
    o3 = vec4<f32>(o3.x, o3.y, instruction_12.z, o3.w);
    // mul o1.w, r0.y, v1.w
    let instruction_13 = ((r0).yyyy) * ((input.color).wwww);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_13.w);
    // mov o1.xyz, v1.xyzx
    let instruction_14 = (input.color).xyzx;
    o1 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, o1.w);
    // mov o2.xyz, l(0,0,0,0)
    let instruction_15 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, o2.w);
    // mov o3.xy, v2.xyxx
    let instruction_16 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o3 = vec4<f32>(instruction_16.x, instruction_16.y, o3.z, o3.w);
    // mov o3.w, l(0)
    let instruction_17 = vec4<f32>(0.0);
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_17.w);
    var result: SourceVaryings3;
    result.position = o0;
    result.v1 = o1;
    result.v3 = o3;
    return result;
}
@fragment
fn source_fragment_3(input: SourceVaryings3) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mul r0.xy, v0.xyxx, cb0[95].zwzz
    let instruction_0 = ((input.position).xyxx) * ((frame.render_target_size).zwzz);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t1.xyzw, s1
    let instruction_1 = textureSample(float_z, float_z_sampler, ((r0).xyxx).xy);
    r0 = instruction_1;
    // max r0.x, |r0.x|, l(0.000000)
    let instruction_2 = max(abs((r0).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // div r0.x, cb0[58].x, r0.x
    let instruction_3 = ((frame.z_near).xxxx) / ((r0).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -v3.z
    let instruction_4 = ((r0).xxxx) + (-((input.v3).zzzz));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, cb1[46].x
    let instruction_5 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t0.xyzw, s0
    let instruction_6 = textureSample(color_map, color_sampler, ((input.v3).xyxx).xy);
    r1 = instruction_6;
    // add r0.y, r1.w, v1.w
    let instruction_7 = ((r1).wwww) + ((input.v1).wwww);
    r0 = vec4<f32>(r0.x, instruction_7.y, r0.z, r0.w);
    // mul r1.xyz, r1.xyzx, v1.xyzx
    let instruction_8 = ((r1).xyzx) * ((input.v1).xyzx);
    r1 = vec4<f32>(instruction_8.x, instruction_8.y, instruction_8.z, r1.w);
    // mad_sat r0.y, r0.y, cb1[51].x, -cb1[51].x
    let instruction_9 = clamp(fma((r0).yyyy, (material.source[5]).xxxx, -((material.source[5]).xxxx)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_9.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.y
    let instruction_10 = ((r0).xxxx) * ((r0).yyyy);
    r0 = vec4<f32>(instruction_10.x, r0.y, r0.z, r0.w);
    // mul o0.xyz, r0.xxxx, r1.xyzx
    let instruction_11 = ((r0).xxxx) * ((r1).xyzx);
    o0 = vec4<f32>(instruction_11.x, instruction_11.y, instruction_11.z, o0.w);
    // mov o0.w, r0.x
    let instruction_12 = (r0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_12.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings4 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
}
@vertex
fn source_vertex_4(input: SourceInput) -> SourceVaryings4 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_2.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_3.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_4.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_5.z, r1.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_6.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_7.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_8.z, o0.w);
    // dp4 o0.w, r1.xyzw, cb0[39].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_9.w);
    // dp3 r0.x, r1.xyzx, r1.xyzx
    let instruction_10 = vec4<f32>(dot(((r1).xyzx).xyz, ((r1).xyzx).xyz));
    r0 = vec4<f32>(instruction_10.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_11 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_11.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, r1.xyzx
    let instruction_12 = ((r0).xxxx) * ((r1).xyzx);
    r0 = vec4<f32>(instruction_12.x, instruction_12.y, instruction_12.z, r0.w);
    // add r1.xyz, v3.xyzx, v3.xyzx
    let instruction_13 = ((source_normal(input.normal)).xyzx) + ((source_normal(input.normal)).xyzx);
    r1 = vec4<f32>(instruction_13.x, instruction_13.y, instruction_13.z, r1.w);
    // ge r2.xyz, v3.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_14 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((source_normal(input.normal)).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r2 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, r2.w);
    // mad r3.xyz, v3.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_15 = fma((source_normal(input.normal)).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r3 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, r3.w);
    // movc r1.xyz, r2.xyzx, r3.xyzx, r1.xyzx
    let instruction_16 = select((r1).xyzx, (r3).xyzx, bitcast<vec4<u32>>((r2).xyzx) != vec4<u32>(0u));
    r1 = vec4<f32>(instruction_16.x, instruction_16.y, instruction_16.z, r1.w);
    // dp3 r2.x, r1.xyzx, cb3[0].xyzx
    let instruction_17 = vec4<f32>(dot(((r1).xyzx).xyz, ((transform.world_rows[0]).xyzx).xyz));
    r2 = vec4<f32>(instruction_17.x, r2.y, r2.z, r2.w);
    // dp3 r2.y, r1.xyzx, cb3[1].xyzx
    let instruction_18 = vec4<f32>(dot(((r1).xyzx).xyz, ((transform.world_rows[1]).xyzx).xyz));
    r2 = vec4<f32>(r2.x, instruction_18.y, r2.z, r2.w);
    // dp3 r2.z, r1.xyzx, cb3[2].xyzx
    let instruction_19 = vec4<f32>(dot(((r1).xyzx).xyz, ((transform.world_rows[2]).xyzx).xyz));
    r2 = vec4<f32>(r2.x, r2.y, instruction_19.z, r2.w);
    // dp3 r0.x, r0.xyzx, r2.xyzx
    let instruction_20 = vec4<f32>(dot(((r0).xyzx).xyz, ((r2).xyzx).xyz));
    r0 = vec4<f32>(instruction_20.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_21 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_21.x, r0.y, r0.z, r0.w);
    // mad_sat r0.x, r0.x, cb1[47].z, cb1[47].w
    let instruction_22 = clamp(fma((r0).xxxx, (material.source[1]).zzzz, (material.source[1]).wwww), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_22.x, r0.y, r0.z, r0.w);
    // mul r0.yzw, r0.xxxx, v1.xxyz
    let instruction_23 = ((r0).xxxx) * ((input.color).xxyz);
    r0 = vec4<f32>(r0.x, instruction_23.y, instruction_23.z, instruction_23.w);
    // add r1.xyz, cb1[48].xyzx, -cb1[49].xyzx
    let instruction_24 = ((material.source[2]).xyzx) + (-((material.source[3]).xyzx));
    r1 = vec4<f32>(instruction_24.x, instruction_24.y, instruction_24.z, r1.w);
    // mad r1.xyz, r0.xxxx, r1.xyzx, cb1[49].xyzx
    let instruction_25 = fma((r0).xxxx, (r1).xyzx, (material.source[3]).xyzx);
    r1 = vec4<f32>(instruction_25.x, instruction_25.y, instruction_25.z, r1.w);
    // mul o1.xyz, r0.yzwy, r1.xyzx
    let instruction_26 = ((r0).yzwy) * ((r1).xyzx);
    o1 = vec4<f32>(instruction_26.x, instruction_26.y, instruction_26.z, o1.w);
    // mov o1.w, v1.w
    let instruction_27 = (input.color).wwww;
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_27.w);
    // mov o2.xy, v2.xyxx
    let instruction_28 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o2 = vec4<f32>(instruction_28.x, instruction_28.y, o2.z, o2.w);
    var result: SourceVaryings4;
    result.position = o0;
    result.v1 = o1;
    result.v2 = o2;
    return result;
}
@fragment
fn source_fragment_4(input: SourceVaryings4) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    // sample r0.xyzw, v2.xyxx, t0.xyzw, s0
    let instruction_0 = textureSample(color_map, color_sampler, ((input.v2).xyxx).xy);
    r0 = instruction_0;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_1 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_1;
    // mul o0.xyz, r0.wwww, r0.xyzx
    let instruction_2 = ((r0).wwww) * ((r0).xyzx);
    o0 = vec4<f32>(instruction_2.x, instruction_2.y, instruction_2.z, o0.w);
    // mov o0.w, r0.w
    let instruction_3 = (r0).wwww;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_3.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings5 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(3) v3: vec4<f32>,
}
@vertex
fn source_vertex_5(input: SourceInput) -> SourceVaryings5 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_6.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_7.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_8.z, o0.w);
    // dp4 r0.x, r1.xyzw, cb0[39].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_9.x, r0.y, r0.z, r0.w);
    // mov o0.w, r0.x
    let instruction_10 = (r0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_10.w);
    // mul_sat r0.y, r0.x, cb1[46].x
    let instruction_11 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_11.y, r0.z, r0.w);
    // mov o3.z, r0.x
    let instruction_12 = (r0).xxxx;
    o3 = vec4<f32>(o3.x, o3.y, instruction_12.z, o3.w);
    // mul o1.w, r0.y, v1.w
    let instruction_13 = ((r0).yyyy) * ((input.color).wwww);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_13.w);
    // mov o1.xyz, v1.xyzx
    let instruction_14 = (input.color).xyzx;
    o1 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, o1.w);
    // mov o2.xyz, l(0,0,0,0)
    let instruction_15 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, o2.w);
    // mov o3.xy, v2.xyxx
    let instruction_16 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o3 = vec4<f32>(instruction_16.x, instruction_16.y, o3.z, o3.w);
    // mov o3.w, l(0)
    let instruction_17 = vec4<f32>(0.0);
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_17.w);
    var result: SourceVaryings5;
    result.position = o0;
    result.v1 = o1;
    result.v3 = o3;
    return result;
}
@fragment
fn source_fragment_5(input: SourceVaryings5) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mul r0.xy, v0.xyxx, cb0[95].zwzz
    let instruction_0 = ((input.position).xyxx) * ((frame.render_target_size).zwzz);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t1.xyzw, s1
    let instruction_1 = textureSample(float_z, float_z_sampler, ((r0).xyxx).xy);
    r0 = instruction_1;
    // max r0.x, |r0.x|, l(0.000000)
    let instruction_2 = max(abs((r0).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // div r0.x, cb0[58].x, r0.x
    let instruction_3 = ((frame.z_near).xxxx) / ((r0).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -v3.z
    let instruction_4 = ((r0).xxxx) + (-((input.v3).zzzz));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, cb1[46].x
    let instruction_5 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t0.xyzw, s0
    let instruction_6 = textureSample(color_map, color_sampler, ((input.v3).xyxx).xy);
    r1 = instruction_6;
    // add r0.y, r1.w, v1.w
    let instruction_7 = ((r1).wwww) + ((input.v1).wwww);
    r0 = vec4<f32>(r0.x, instruction_7.y, r0.z, r0.w);
    // mul o0.xyz, r1.xyzx, v1.xyzx
    let instruction_8 = ((r1).xyzx) * ((input.v1).xyzx);
    o0 = vec4<f32>(instruction_8.x, instruction_8.y, instruction_8.z, o0.w);
    // mad_sat r0.y, r0.y, cb1[51].x, -cb1[51].x
    let instruction_9 = clamp(fma((r0).yyyy, (material.source[5]).xxxx, -((material.source[5]).xxxx)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_9.y, r0.z, r0.w);
    // mul o0.w, r0.x, r0.y
    let instruction_10 = ((r0).xxxx) * ((r0).yyyy);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_10.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings6 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(3) v3: vec4<f32>,
}
@vertex
fn source_vertex_6(input: SourceInput) -> SourceVaryings6 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 r0.w, r1.xyzw, cb0[39].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_6.w);
    // add r0.z, r0.w, -cb1[50].x
    let instruction_7 = ((r0).wwww) + (-((material.source[4]).xxxx));
    r0 = vec4<f32>(r0.x, r0.y, instruction_7.z, r0.w);
    // dp4 r2.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_8.z, r2.w);
    // max r3.w, r0.z, r2.z
    let instruction_9 = max((r0).zzzz, (r2).zzzz);
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_9.w);
    // div r0.z, r3.w, r0.w
    let instruction_10 = ((r3).wwww) / ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, instruction_10.z, r0.w);
    // max r0.z, r0.z, l(0.500000)
    let instruction_11 = max((r0).zzzz, vec4<f32>(0.5));
    r0 = vec4<f32>(r0.x, r0.y, instruction_11.z, r0.w);
    // dp4 r0.x, r1.xyzw, cb0[36].xyzw
    let instruction_12 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_12.x, r0.y, r0.z, r0.w);
    // dp4 r0.y, r1.xyzw, cb0[37].xyzw
    let instruction_13 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_13.y, r0.z, r0.w);
    // mul r3.xy, r0.zzzz, r0.xyxx
    let instruction_14 = ((r0).zzzz) * ((r0).xyxx);
    r3 = vec4<f32>(instruction_14.x, instruction_14.y, r3.z, r3.w);
    // lt r0.z, r2.z, r0.w
    let instruction_15 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r2).zzzz) < ((r0).wwww)));
    r0 = vec4<f32>(r0.x, r0.y, instruction_15.z, r0.w);
    // movc r2.xyw, r0.zzzz, r3.xyxw, r0.xyxw
    let instruction_16 = select((r0).xyxw, (r3).xyxw, bitcast<vec4<u32>>((r0).zzzz) != vec4<u32>(0u));
    r2 = vec4<f32>(instruction_16.x, instruction_16.y, r2.z, instruction_16.w);
    // mov o0.xyzw, r2.xyzw
    let instruction_17 = (r2).xyzw;
    o0 = instruction_17;
    // mul_sat r0.x, r2.w, cb1[46].x
    let instruction_18 = clamp(((r2).wwww) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_18.x, r0.y, r0.z, r0.w);
    // mov o3.z, r2.w
    let instruction_19 = (r2).wwww;
    o3 = vec4<f32>(o3.x, o3.y, instruction_19.z, o3.w);
    // mul o1.w, r0.x, v1.w
    let instruction_20 = ((r0).xxxx) * ((input.color).wwww);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_20.w);
    // mov o1.xyz, v1.xyzx
    let instruction_21 = (input.color).xyzx;
    o1 = vec4<f32>(instruction_21.x, instruction_21.y, instruction_21.z, o1.w);
    // mov o2.xyz, l(0,0,0,0)
    let instruction_22 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = vec4<f32>(instruction_22.x, instruction_22.y, instruction_22.z, o2.w);
    // mov o3.xy, v2.xyxx
    let instruction_23 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o3 = vec4<f32>(instruction_23.x, instruction_23.y, o3.z, o3.w);
    // mov o3.w, l(0)
    let instruction_24 = vec4<f32>(0.0);
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_24.w);
    var result: SourceVaryings6;
    result.position = o0;
    result.v1 = o1;
    result.v3 = o3;
    return result;
}
@fragment
fn source_fragment_6(input: SourceVaryings6) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mul r0.xy, v0.xyxx, cb0[95].zwzz
    let instruction_0 = ((input.position).xyxx) * ((frame.render_target_size).zwzz);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t1.xyzw, s1
    let instruction_1 = textureSample(float_z, float_z_sampler, ((r0).xyxx).xy);
    r0 = instruction_1;
    // max r0.x, |r0.x|, l(0.000000)
    let instruction_2 = max(abs((r0).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // div r0.x, cb0[58].x, r0.x
    let instruction_3 = ((frame.z_near).xxxx) / ((r0).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -v3.z
    let instruction_4 = ((r0).xxxx) + (-((input.v3).zzzz));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, cb1[46].x
    let instruction_5 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t0.xyzw, s0
    let instruction_6 = textureSample(color_map, color_sampler, ((input.v3).xyxx).xy);
    r1 = instruction_6;
    // mul r1.xyzw, r1.xyzw, v1.xyzw
    let instruction_7 = ((r1).xyzw) * ((input.v1).xyzw);
    r1 = instruction_7;
    // mul o0.w, r0.x, r1.w
    let instruction_8 = ((r0).xxxx) * ((r1).wwww);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_8.w);
    // mov o0.xyz, r1.xyzx
    let instruction_9 = (r1).xyzx;
    o0 = vec4<f32>(instruction_9.x, instruction_9.y, instruction_9.z, o0.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings7 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
}
@vertex
fn source_vertex_7(input: SourceInput) -> SourceVaryings7 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    var r4: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_2.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_3.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_4.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_5.z, r1.w);
    // dp4 r0.x, r1.xyzw, cb0[39].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_6.x, r0.y, r0.z, r0.w);
    // mov o0.w, r0.x
    let instruction_7 = (r0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_7.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_8.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_9.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_10 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_10.z, o0.w);
    // dp3 r0.y, r1.xyzx, r1.xyzx
    let instruction_11 = vec4<f32>(dot(((r1).xyzx).xyz, ((r1).xyzx).xyz));
    r0 = vec4<f32>(r0.x, instruction_11.y, r0.z, r0.w);
    // rsq r0.z, r0.y
    let instruction_12 = inverseSqrt((r0).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_12.z, r0.w);
    // sqrt r0.y, r0.y
    let instruction_13 = sqrt((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_13.y, r0.z, r0.w);
    // mul r1.xyw, r0.zzzz, r1.xyxz
    let instruction_14 = ((r0).zzzz) * ((r1).xyxz);
    r1 = vec4<f32>(instruction_14.x, instruction_14.y, r1.z, instruction_14.w);
    // add r2.xyz, v3.xyzx, v3.xyzx
    let instruction_15 = ((source_normal(input.normal)).xyzx) + ((source_normal(input.normal)).xyzx);
    r2 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, r2.w);
    // ge r3.xyz, v3.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_16 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((source_normal(input.normal)).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r3 = vec4<f32>(instruction_16.x, instruction_16.y, instruction_16.z, r3.w);
    // mad r4.xyz, v3.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_17 = fma((source_normal(input.normal)).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r4 = vec4<f32>(instruction_17.x, instruction_17.y, instruction_17.z, r4.w);
    // movc r2.xyz, r3.xyzx, r4.xyzx, r2.xyzx
    let instruction_18 = select((r2).xyzx, (r4).xyzx, bitcast<vec4<u32>>((r3).xyzx) != vec4<u32>(0u));
    r2 = vec4<f32>(instruction_18.x, instruction_18.y, instruction_18.z, r2.w);
    // dp3 r3.x, r2.xyzx, cb3[0].xyzx
    let instruction_19 = vec4<f32>(dot(((r2).xyzx).xyz, ((transform.world_rows[0]).xyzx).xyz));
    r3 = vec4<f32>(instruction_19.x, r3.y, r3.z, r3.w);
    // dp3 r3.y, r2.xyzx, cb3[1].xyzx
    let instruction_20 = vec4<f32>(dot(((r2).xyzx).xyz, ((transform.world_rows[1]).xyzx).xyz));
    r3 = vec4<f32>(r3.x, instruction_20.y, r3.z, r3.w);
    // dp3 r3.z, r2.xyzx, cb3[2].xyzx
    let instruction_21 = vec4<f32>(dot(((r2).xyzx).xyz, ((transform.world_rows[2]).xyzx).xyz));
    r3 = vec4<f32>(r3.x, r3.y, instruction_21.z, r3.w);
    // dp3 r0.z, r1.xywx, r3.xyzx
    let instruction_22 = vec4<f32>(dot(((r1).xywx).xyz, ((r3).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, instruction_22.z, r0.w);
    // dp3 r0.w, cb0[29].xyzx, r1.xywx
    let instruction_23 = vec4<f32>(dot(((frame.scene.sun_fog_dir).xyzx).xyz, ((r1).xywx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_23.w);
    // mad_sat r0.w, cb0[31].y, r0.w, cb0[31].x
    let instruction_24 = clamp(fma((frame.scene.sun_fog).yyyy, (r0).wwww, (frame.scene.sun_fog).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_24.w);
    // mul r0.z, r0.z, r0.z
    let instruction_25 = ((r0).zzzz) * ((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_25.z, r0.w);
    // mad_sat r0.z, r0.z, cb1[47].z, cb1[47].w
    let instruction_26 = clamp(fma((r0).zzzz, (material.source[1]).zzzz, (material.source[1]).wwww), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_26.z, r0.w);
    // mul_sat r1.x, r0.x, cb1[46].x
    let instruction_27 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(instruction_27.x, r1.y, r1.z, r1.w);
    // mov o3.z, r0.x
    let instruction_28 = (r0).xxxx;
    o3 = vec4<f32>(o3.x, o3.y, instruction_28.z, o3.w);
    // mul r0.x, r1.x, v1.w
    let instruction_29 = ((r1).xxxx) * ((input.color).wwww);
    r0 = vec4<f32>(instruction_29.x, r0.y, r0.z, r0.w);
    // mul o1.w, r0.z, r0.x
    let instruction_30 = ((r0).zzzz) * ((r0).xxxx);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_30.w);
    // add r1.xyw, cb1[48].xyxz, -cb1[49].xyxz
    let instruction_31 = ((material.source[2]).xyxz) + (-((material.source[3]).xyxz));
    r1 = vec4<f32>(instruction_31.x, instruction_31.y, r1.z, instruction_31.w);
    // mad r1.xyw, r0.zzzz, r1.xyxw, cb1[49].xyxz
    let instruction_32 = fma((r0).zzzz, (r1).xyxw, (material.source[3]).xyxz);
    r1 = vec4<f32>(instruction_32.x, instruction_32.y, r1.z, instruction_32.w);
    // mul o1.xyz, r1.xywx, v1.xyzx
    let instruction_33 = ((r1).xywx) * ((input.color).xyzx);
    o1 = vec4<f32>(instruction_33.x, instruction_33.y, instruction_33.z, o1.w);
    // add r2.xyzw, -cb0[26].xyzw, cb0[30].xyzw
    let instruction_34 = (-((frame.scene.fog_color).xyzw)) + ((frame.scene.sun_fog_color).xyzw);
    r2 = instruction_34;
    // mad r2.xyzw, r0.wwww, r2.xyzw, cb0[26].xyzw
    let instruction_35 = fma((r0).wwww, (r2).xyzw, (frame.scene.fog_color).xyzw);
    r2 = instruction_35;
    // mul_sat r0.xzw, r2.xxyz, cb0[20].xxxx
    let instruction_36 = clamp(((r2).xxyz) * ((frame.scene.hdr_control0).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_36.x, r0.y, instruction_36.z, instruction_36.w);
    // sqrt o2.xyz, r0.xzwx
    let instruction_37 = sqrt((r0).xzwx);
    o2 = vec4<f32>(instruction_37.x, instruction_37.y, instruction_37.z, o2.w);
    // mad r0.x, cb0[27].w, r1.z, cb0[27].x
    let instruction_38 = fma((frame.scene.fog_consts).wwww, (r1).zzzz, (frame.scene.fog_consts).xxxx);
    r0 = vec4<f32>(instruction_38.x, r0.y, r0.z, r0.w);
    // mul r0.z, r1.z, cb0[27].w
    let instruction_39 = ((r1).zzzz) * ((frame.scene.fog_consts).wwww);
    r0 = vec4<f32>(r0.x, r0.y, instruction_39.z, r0.w);
    // min r0.w, r0.x, l(64.000000)
    let instruction_40 = min((r0).xxxx, vec4<f32>(64.0));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_40.w);
    // mul r0.w, r0.w, l(1.442695)
    let instruction_41 = ((r0).wwww) * (vec4<f32>(1.4426950216293335));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_41.w);
    // exp r0.w, r0.w
    let instruction_42 = exp2((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_42.w);
    // lt r1.x, r0.x, l(0.000000)
    let instruction_43 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(0.0))));
    r1 = vec4<f32>(instruction_43.x, r1.y, r1.z, r1.w);
    // add r0.x, r0.x, l(1.000000)
    let instruction_44 = ((r0).xxxx) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_44.x, r0.y, r0.z, r0.w);
    // movc r0.x, r1.x, r0.w, r0.x
    let instruction_45 = select((r0).xxxx, (r0).wwww, bitcast<vec4<u32>>((r1).xxxx) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_45.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -cb0[28].x
    let instruction_46 = ((r0).xxxx) + (-((frame.scene.fog_consts2).xxxx));
    r0 = vec4<f32>(instruction_46.x, r0.y, r0.z, r0.w);
    // lt r0.w, |r0.z|, l(0.000100)
    let instruction_47 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (abs((r0).zzzz)) < (vec4<f32>(9.999999747378752e-05))));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_47.w);
    // movc r0.z, r0.w, l(1.000000), r0.z
    let instruction_48 = select((r0).zzzz, vec4<f32>(1.0), bitcast<vec4<u32>>((r0).wwww) != vec4<u32>(0u));
    r0 = vec4<f32>(r0.x, r0.y, instruction_48.z, r0.w);
    // div r0.x, r0.x, r0.z
    let instruction_49 = ((r0).xxxx) / ((r0).zzzz);
    r0 = vec4<f32>(instruction_49.x, r0.y, r0.z, r0.w);
    // mov_sat r0.z, cb0[28].x
    let instruction_50 = clamp((frame.scene.fog_consts2).xxxx, vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_50.z, r0.w);
    // movc r0.x, r0.w, r0.z, r0.x
    let instruction_51 = select((r0).xxxx, (r0).zzzz, bitcast<vec4<u32>>((r0).wwww) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_51.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, cb0[27].y
    let instruction_52 = ((r0).xxxx) * ((frame.scene.fog_consts).yyyy);
    r0 = vec4<f32>(instruction_52.x, r0.y, r0.z, r0.w);
    // mad r0.x, r0.x, r0.y, cb0[27].z
    let instruction_53 = fma((r0).xxxx, (r0).yyyy, (frame.scene.fog_consts).zzzz);
    r0 = vec4<f32>(instruction_53.x, r0.y, r0.z, r0.w);
    // exp r0.x, r0.x
    let instruction_54 = exp2((r0).xxxx);
    r0 = vec4<f32>(instruction_54.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_55 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_55.x, r0.y, r0.z, r0.w);
    // add r0.x, -r0.x, l(1.000000)
    let instruction_56 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_56.x, r0.y, r0.z, r0.w);
    // mad r0.x, -r0.x, r2.w, l(1.000000)
    let instruction_57 = fma(-((r0).xxxx), (r2).wwww, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_57.x, r0.y, r0.z, r0.w);
    // mul o3.w, r0.x, r0.x
    let instruction_58 = ((r0).xxxx) * ((r0).xxxx);
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_58.w);
    // mov o3.xy, v2.xyxx
    let instruction_59 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o3 = vec4<f32>(instruction_59.x, instruction_59.y, o3.z, o3.w);
    var result: SourceVaryings7;
    result.position = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    return result;
}
@fragment
fn source_fragment_7(input: SourceVaryings7) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mul r0.xy, v0.xyxx, cb0[95].zwzz
    let instruction_0 = ((input.position).xyxx) * ((frame.render_target_size).zwzz);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t1.xyzw, s1
    let instruction_1 = textureSample(float_z, float_z_sampler, ((r0).xyxx).xy);
    r0 = instruction_1;
    // max r0.x, |r0.x|, l(0.000000)
    let instruction_2 = max(abs((r0).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // div r0.x, cb0[58].x, r0.x
    let instruction_3 = ((frame.z_near).xxxx) / ((r0).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -v3.z
    let instruction_4 = ((r0).xxxx) + (-((input.v3).zzzz));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, cb1[46].x
    let instruction_5 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t0.xyzw, s0
    let instruction_6 = textureSample(color_map, color_sampler, ((input.v3).xyxx).xy);
    r1 = instruction_6;
    // add r0.y, r1.w, v1.w
    let instruction_7 = ((r1).wwww) + ((input.v1).wwww);
    r0 = vec4<f32>(r0.x, instruction_7.y, r0.z, r0.w);
    // mad r1.xyz, v1.xyzx, r1.xyzx, -v2.xyzx
    let instruction_8 = fma((input.v1).xyzx, (r1).xyzx, -((input.v2).xyzx));
    r1 = vec4<f32>(instruction_8.x, instruction_8.y, instruction_8.z, r1.w);
    // mad o0.xyz, v3.wwww, r1.xyzx, v2.xyzx
    let instruction_9 = fma((input.v3).wwww, (r1).xyzx, (input.v2).xyzx);
    o0 = vec4<f32>(instruction_9.x, instruction_9.y, instruction_9.z, o0.w);
    // mad_sat r0.y, r0.y, cb1[51].x, -cb1[51].x
    let instruction_10 = clamp(fma((r0).yyyy, (material.source[5]).xxxx, -((material.source[5]).xxxx)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // mul o0.w, r0.x, r0.y
    let instruction_11 = ((r0).xxxx) * ((r0).yyyy);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_11.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings8 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
}
@vertex
fn source_vertex_8(input: SourceInput) -> SourceVaryings8 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_2.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_3.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_4.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_5.z, r1.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_6.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_7.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_8.z, o0.w);
    // dp4 o0.w, r1.xyzw, cb0[39].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_9.w);
    // mov o4.xyz, r1.xyzx
    let instruction_10 = (r1).xyzx;
    o4 = vec4<f32>(instruction_10.x, instruction_10.y, instruction_10.z, o4.w);
    // mov o1.xyzw, v1.xyzw
    let instruction_11 = (input.color).xyzw;
    o1 = instruction_11;
    // mov o2.xy, v2.xyxx
    let instruction_12 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o2 = vec4<f32>(instruction_12.x, instruction_12.y, o2.z, o2.w);
    // mov o2.z, l(1.000000)
    let instruction_13 = vec4<f32>(1.0);
    o2 = vec4<f32>(o2.x, o2.y, instruction_13.z, o2.w);
    // mov o3.xyz, l(0,0,0,0)
    let instruction_14 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, o3.w);
    var result: SourceVaryings8;
    result.position = o0;
    result.v1 = o1;
    result.v2 = o2;
    return result;
}
@fragment
fn source_fragment_8(input: SourceVaryings8) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    // sample r0.xyzw, v2.xyxx, t0.xyzw, s0
    let instruction_0 = textureSample(color_map, color_sampler, ((input.v2).xyxx).xy);
    r0 = instruction_0;
    // mul r0.xyz, r0.xyzx, v1.xyzx
    let instruction_1 = ((r0).xyzx) * ((input.v1).xyzx);
    r0 = vec4<f32>(instruction_1.x, instruction_1.y, instruction_1.z, r0.w);
    // add r0.w, r0.w, v1.w
    let instruction_2 = ((r0).wwww) + ((input.v1).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_2.w);
    // mad_sat r0.w, r0.w, cb1[51].x, -cb1[51].x
    let instruction_3 = clamp(fma((r0).wwww, (material.source[5]).xxxx, -((material.source[5]).xxxx)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_3.w);
    // mul o0.xyz, r0.wwww, r0.xyzx
    let instruction_4 = ((r0).wwww) * ((r0).xyzx);
    o0 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, o0.w);
    // mov o0.w, r0.w
    let instruction_5 = (r0).wwww;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_5.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings9 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
}
@vertex
fn source_vertex_9(input: SourceInput) -> SourceVaryings9 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_6.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_7.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_8.z, o0.w);
    // dp4 o0.w, r1.xyzw, cb0[39].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_9.w);
    // mov o1.xyzw, v1.xyzw
    let instruction_10 = (input.color).xyzw;
    o1 = instruction_10;
    // mov o2.xy, v2.xyxx
    let instruction_11 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o2 = vec4<f32>(instruction_11.x, instruction_11.y, o2.z, o2.w);
    var result: SourceVaryings9;
    result.position = o0;
    result.v1 = o1;
    result.v2 = o2;
    return result;
}
@fragment
fn source_fragment_9(input: SourceVaryings9) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    // sample r0.xyzw, v2.xyxx, t0.xyzw, s0
    let instruction_0 = textureSample(color_map, color_sampler, ((input.v2).xyxx).xy);
    r0 = instruction_0;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_1 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_1;
    // mul o0.xyz, r0.wwww, r0.xyzx
    let instruction_2 = ((r0).wwww) * ((r0).xyzx);
    o0 = vec4<f32>(instruction_2.x, instruction_2.y, instruction_2.z, o0.w);
    // mov o0.w, r0.w
    let instruction_3 = (r0).wwww;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_3.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings10 {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(3) v3: vec4<f32>,
}
@vertex
fn source_vertex_10(input: SourceInput) -> SourceVaryings10 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 o0.x, r1.xyzw, cb0[36].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    o0 = vec4<f32>(instruction_6.x, o0.y, o0.z, o0.w);
    // dp4 o0.y, r1.xyzw, cb0[37].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, instruction_7.y, o0.z, o0.w);
    // dp4 o0.z, r1.xyzw, cb0[38].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[2]).xyzw).xyzw));
    o0 = vec4<f32>(o0.x, o0.y, instruction_8.z, o0.w);
    // dp4 r0.x, r1.xyzw, cb0[39].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_9.x, r0.y, r0.z, r0.w);
    // mov o0.w, r0.x
    let instruction_10 = (r0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_10.w);
    // mul_sat r0.y, r0.x, cb1[46].x
    let instruction_11 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_11.y, r0.z, r0.w);
    // mov o3.z, r0.x
    let instruction_12 = (r0).xxxx;
    o3 = vec4<f32>(o3.x, o3.y, instruction_12.z, o3.w);
    // mul o1.w, r0.y, v1.w
    let instruction_13 = ((r0).yyyy) * ((input.color).wwww);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_13.w);
    // mov o1.xyz, v1.xyzx
    let instruction_14 = (input.color).xyzx;
    o1 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, o1.w);
    // mov o2.xyz, l(0,0,0,0)
    let instruction_15 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, o2.w);
    // mov o3.xy, v2.xyxx
    let instruction_16 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o3 = vec4<f32>(instruction_16.x, instruction_16.y, o3.z, o3.w);
    // mov o3.w, l(0)
    let instruction_17 = vec4<f32>(0.0);
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_17.w);
    var result: SourceVaryings10;
    result.position = o0;
    result.v1 = o1;
    result.v3 = o3;
    return result;
}
@fragment
fn source_fragment_10(input: SourceVaryings10) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mul r0.xy, v0.xyxx, cb0[95].zwzz
    let instruction_0 = ((input.position).xyxx) * ((frame.render_target_size).zwzz);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t1.xyzw, s1
    let instruction_1 = textureSample(float_z, float_z_sampler, ((r0).xyxx).xy);
    r0 = instruction_1;
    // max r0.x, |r0.x|, l(0.000000)
    let instruction_2 = max(abs((r0).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // div r0.x, cb0[58].x, r0.x
    let instruction_3 = ((frame.z_near).xxxx) / ((r0).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -v3.z
    let instruction_4 = ((r0).xxxx) + (-((input.v3).zzzz));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, cb1[46].x
    let instruction_5 = clamp(((r0).xxxx) * ((material.source[0]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t0.xyzw, s0
    let instruction_6 = textureSample(color_map, color_sampler, ((input.v3).xyxx).xy);
    r1 = instruction_6;
    // mul r1.xyzw, r1.xyzw, v1.xyzw
    let instruction_7 = ((r1).xyzw) * ((input.v1).xyzw);
    r1 = instruction_7;
    // mul o0.w, r0.x, r1.w
    let instruction_8 = ((r0).xxxx) * ((r1).wwww);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_8.w);
    // mov o0.xyz, r1.xyzx
    let instruction_9 = (r1).xyzx;
    o0 = vec4<f32>(instruction_9.x, instruction_9.y, instruction_9.z, o0.w);
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings11 {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) flare_uv: vec2<f32>,
}
@vertex
fn source_vertex_11(input: SourceInput) -> SourceVaryings11 {
    let original_icb = array<vec4<f32>, 8>(vec4<f32>(0.8958399891853333, 0.9639580249786377, 0.20123900473117828, 0.7453539967536926), vec4<f32>(0.9982600212097168, 0.8725240230560303, 0.25034299492836, 0.37711700797080994), vec4<f32>(0.6162599921226501, 0.8287299871444702, 0.03543199971318245, 0.49665799736976624), vec4<f32>(0.7577139735221863, 0.2090820074081421, 0.0, 0.8285160064697266), vec4<f32>(0.814844012260437, 0.6909390091896057, 0.0, 0.35975199937820435), vec4<f32>(0.658132016658783, 0.12857399880886078, 0.0, 0.0), vec4<f32>(0.8834189772605896, 0.6419870257377625, 0.0, 0.0), vec4<f32>(0.6458020210266113, 0.18726199865341187, 0.0, 0.0));
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    var r4: vec4<f32>;
    var r5: vec4<f32>;
    var r6: vec4<f32>;
    var r7: vec4<f32>;
    // mul r0.x, cb3[0].x, l(0.000001)
    let instruction_0 = ((transform.world_rows[0]).xxxx) * (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // mad r0.x, v1.x, l(255.000000), r0.x
    let instruction_1 = fma((input.color).xxxx, vec4<f32>(255.0), (r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // round_ni r0.x, r0.x
    let instruction_2 = floor((r0).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // lt r0.y, r0.x, l(14.000000)
    let instruction_3 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(14.0))));
    r0 = vec4<f32>(r0.x, instruction_3.y, r0.z, r0.w);
    // if_nz r0.y
    if bitcast<vec4<u32>>((r0).yyyy).x != 0u {
    // mov r1.xyz, v0.xyzx
    let instruction_4 = (input.position).xyzx;
    r1 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_5 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 r2.x, r1.xyzw, cb3[0].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(instruction_6.x, r2.y, r2.z, r2.w);
    // dp4 r2.y, r1.xyzw, cb3[1].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, instruction_7.y, r2.z, r2.w);
    // dp4 r2.z, r1.xyzw, cb3[2].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_8.z, r2.w);
    // mov r2.w, l(1.000000)
    let instruction_9 = vec4<f32>(1.0);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_9.w);
    // dp4 r0.y, r2.xyzw, cb0[39].xyzw
    let instruction_10 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // ge r0.z, r0.y, l(0.000000)
    let instruction_11 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).yyyy) >= (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_11.z, r0.w);
    // if_nz r0.z
    if bitcast<vec4<u32>>((r0).zzzz).x != 0u {
    // dp4 r1.x, r2.xyzw, cb0[36].xyzw
    let instruction_12 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_12.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r2.xyzw, cb0[37].xyzw
    let instruction_13 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_13.y, r1.z, r1.w);
    // div r0.zw, r1.xxxy, r0.yyyy
    let instruction_14 = ((r1).xxxy) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_14.z, instruction_14.w);
    // mul r1.xy, r0.zwzz, l(1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_15 = ((r0).zwzz) * (vec4<f32>(1.0, -1.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_15.x, instruction_15.y, r1.z, r1.w);
    // div r1.z, cb0[95].y, cb0[95].x
    let instruction_16 = ((frame.render_target_size).yyyy) / ((frame.render_target_size).xxxx);
    r1 = vec4<f32>(r1.x, r1.y, instruction_16.z, r1.w);
    // max r0.x, r0.x, l(0.000000)
    let instruction_17 = max((r0).xxxx, vec4<f32>(0.0));
    r0 = vec4<f32>(instruction_17.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(13.000000)
    let instruction_18 = min((r0).xxxx, vec4<f32>(13.0));
    r0 = vec4<f32>(instruction_18.x, r0.y, r0.z, r0.w);
    // lt r2.xyzw, r0.xxxx, l(1.000000, 2.000000, 3.000000, 4.000000)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(1.0, 2.0, 3.0, 4.0))));
    r2 = instruction_19;
    // if_nz r2.x
    if bitcast<vec4<u32>>((r2).xxxx).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.000000, 0.250000, 0.000000, 0.000000)
    let instruction_20 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.0, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_20.x, instruction_20.y, r3.z, r3.w);
    // mul r3.zw, r0.wwwz, l(0.000000, 0.000000, -0.200000, 0.200000)
    let instruction_21 = ((r0).wwwz) * (vec4<f32>(0.0, 0.0, -0.20000000298023224, 0.20000000298023224));
    r3 = vec4<f32>(r3.x, r3.y, instruction_21.z, instruction_21.w);
    // mul r2.x, cb2[5].y, l(10.000000)
    let instruction_22 = ((material.source[13]).yyyy) * (vec4<f32>(10.0));
    r2 = vec4<f32>(instruction_22.x, r2.y, r2.z, r2.w);
    // add r4.x, |r3.w|, l(0.500000)
    let instruction_23 = (abs((r3).wwww)) + (vec4<f32>(0.5));
    r4 = vec4<f32>(instruction_23.x, r4.y, r4.z, r4.w);
    // min r4.x, r4.x, l(1.000000)
    let instruction_24 = min((r4).xxxx, vec4<f32>(1.0));
    r4 = vec4<f32>(instruction_24.x, r4.y, r4.z, r4.w);
    // mad r4.y, r4.x, l(-2.000000), l(3.000000)
    let instruction_25 = fma((r4).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r4 = vec4<f32>(r4.x, instruction_25.y, r4.z, r4.w);
    // mul r4.x, r4.x, r4.x
    let instruction_26 = ((r4).xxxx) * ((r4).xxxx);
    r4 = vec4<f32>(instruction_26.x, r4.y, r4.z, r4.w);
    // mul r4.x, r4.x, r4.y
    let instruction_27 = ((r4).xxxx) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_27.x, r4.y, r4.z, r4.w);
    // min r4.x, r4.x, l(1.000000)
    let instruction_28 = min((r4).xxxx, vec4<f32>(1.0));
    r4 = vec4<f32>(instruction_28.x, r4.y, r4.z, r4.w);
    // mad r4.x, r4.x, l(4.000000), l(1.000000)
    let instruction_29 = fma((r4).xxxx, vec4<f32>(4.0), vec4<f32>(1.0));
    r4 = vec4<f32>(instruction_29.x, r4.y, r4.z, r4.w);
    // mul r2.x, r2.x, r4.x
    let instruction_30 = ((r2).xxxx) * ((r4).xxxx);
    r2 = vec4<f32>(instruction_30.x, r2.y, r2.z, r2.w);
    // div r4.y, r2.x, r0.y
    let instruction_31 = ((r2).xxxx) / ((r0).yyyy);
    r4 = vec4<f32>(r4.x, instruction_31.y, r4.z, r4.w);
    // mul r4.x, r1.z, r4.y
    let instruction_32 = ((r1).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_32.x, r4.y, r4.z, r4.w);
    // max r2.x, |r1.x|, |r1.y|
    let instruction_33 = max(abs((r1).xxxx), abs((r1).yyyy));
    r2 = vec4<f32>(instruction_33.x, r2.y, r2.z, r2.w);
    // add r4.z, r2.x, r2.x
    let instruction_34 = ((r2).xxxx) + ((r2).xxxx);
    r4 = vec4<f32>(r4.x, r4.y, instruction_34.z, r4.w);
    // min r4.z, r4.z, l(1.000000)
    let instruction_35 = min((r4).zzzz, vec4<f32>(1.0));
    r4 = vec4<f32>(r4.x, r4.y, instruction_35.z, r4.w);
    // mad r4.w, r4.z, l(-2.000000), l(3.000000)
    let instruction_36 = fma((r4).zzzz, vec4<f32>(-2.0), vec4<f32>(3.0));
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_36.w);
    // mul r4.z, r4.z, r4.z
    let instruction_37 = ((r4).zzzz) * ((r4).zzzz);
    r4 = vec4<f32>(r4.x, r4.y, instruction_37.z, r4.w);
    // mul r4.z, r4.z, r4.w
    let instruction_38 = ((r4).zzzz) * ((r4).wwww);
    r4 = vec4<f32>(r4.x, r4.y, instruction_38.z, r4.w);
    // min r4.z, r4.z, l(1.000000)
    let instruction_39 = min((r4).zzzz, vec4<f32>(1.0));
    r4 = vec4<f32>(r4.x, r4.y, instruction_39.z, r4.w);
    // add r2.x, r2.x, l(-0.800000)
    let instruction_40 = ((r2).xxxx) + (vec4<f32>(-0.800000011920929));
    r2 = vec4<f32>(instruction_40.x, r2.y, r2.z, r2.w);
    // mul_sat r2.x, r2.x, l(5.000000)
    let instruction_41 = clamp(((r2).xxxx) * (vec4<f32>(5.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r2 = vec4<f32>(instruction_41.x, r2.y, r2.z, r2.w);
    // mad r4.w, r2.x, l(-2.000000), l(3.000000)
    let instruction_42 = fma((r2).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_42.w);
    // mul r2.x, r2.x, r2.x
    let instruction_43 = ((r2).xxxx) * ((r2).xxxx);
    r2 = vec4<f32>(instruction_43.x, r2.y, r2.z, r2.w);
    // mul r2.x, r2.x, r4.w
    let instruction_44 = ((r2).xxxx) * ((r4).wwww);
    r2 = vec4<f32>(instruction_44.x, r2.y, r2.z, r2.w);
    // min r2.x, r2.x, l(1.000000)
    let instruction_45 = min((r2).xxxx, vec4<f32>(1.0));
    r2 = vec4<f32>(instruction_45.x, r2.y, r2.z, r2.w);
    // add r2.x, -r2.x, l(1.000000)
    let instruction_46 = (-((r2).xxxx)) + (vec4<f32>(1.0));
    r2 = vec4<f32>(instruction_46.x, r2.y, r2.z, r2.w);
    // mul r2.x, r2.x, r4.z
    let instruction_47 = ((r2).xxxx) * ((r4).zzzz);
    r2 = vec4<f32>(instruction_47.x, r2.y, r2.z, r2.w);
    // mul r5.xyz, r2.xxxx, l(0.408297, 0.408297, 0.204149, 0.000000)
    let instruction_48 = ((r2).xxxx) * (vec4<f32>(0.4082973599433899, 0.4082973599433899, 0.20414867997169495, 0.0));
    r5 = vec4<f32>(instruction_48.x, instruction_48.y, instruction_48.z, r5.w);
    // mov r4.zw, r3.wwwz
    let instruction_49 = (r3).wwwz;
    r4 = vec4<f32>(r4.x, r4.y, instruction_49.z, instruction_49.w);
    // mov o0.xy, l(0.800000,0,0,0)
    let instruction_50 = vec4<f32>(0.800000011920929, 0.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_50.x, instruction_50.y, o0.z, o0.w);
    // mov r2.x, l(1.000000)
    let instruction_51 = vec4<f32>(1.0);
    r2 = vec4<f32>(instruction_51.x, r2.y, r2.z, r2.w);
    // mov r5.w, l(1.000000)
    let instruction_52 = vec4<f32>(1.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_52.w);
    } else {
    // mov o0.xy, l(0,0,0,0)
    let instruction_53 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_53.x, instruction_53.y, o0.z, o0.w);
    // mov r4.xyzw, l(0,0,0,0)
    let instruction_54 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r4 = instruction_54;
    // mov r2.x, l(0)
    let instruction_55 = vec4<f32>(0.0);
    r2 = vec4<f32>(instruction_55.x, r2.y, r2.z, r2.w);
    // mov r3.xyzw, l(0,0,0,1.000000)
    let instruction_56 = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    r3 = instruction_56;
    // mov r5.xyzw, l(0,0,0,0)
    let instruction_57 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r5 = instruction_57;
    }
    // eq r6.x, r5.w, l(0.000000)
    let instruction_58 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r5).wwww) == (vec4<f32>(0.0))));
    r6 = vec4<f32>(instruction_58.x, r6.y, r6.z, r6.w);
    // and r2.y, r2.y, r6.x
    let instruction_59 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).yyyy) & bitcast<vec4<u32>>((r6).xxxx));
    r2 = vec4<f32>(r2.x, instruction_59.y, r2.z, r2.w);
    // if_nz r2.y
    if bitcast<vec4<u32>>((r2).yyyy).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.000000, 0.250000, 0.000000, 0.000000)
    let instruction_60 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.0, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_60.x, instruction_60.y, r3.z, r3.w);
    // mul r2.y, cb2[5].y, l(30.000000)
    let instruction_61 = ((material.source[13]).yyyy) * (vec4<f32>(30.0));
    r2 = vec4<f32>(r2.x, instruction_61.y, r2.z, r2.w);
    // max r6.x, |r1.x|, |r1.y|
    let instruction_62 = max(abs((r1).xxxx), abs((r1).yyyy));
    r6 = vec4<f32>(instruction_62.x, r6.y, r6.z, r6.w);
    // add r6.yz, r6.xxxx, l(0.000000, -0.300000, -0.800000, 0.000000)
    let instruction_63 = ((r6).xxxx) + (vec4<f32>(0.0, -0.30000001192092896, -0.800000011920929, 0.0));
    r6 = vec4<f32>(r6.x, instruction_63.y, instruction_63.z, r6.w);
    // mul_sat r6.yz, r6.yyzy, l(0.000000, 1.428571, 5.000000, 0.000000)
    let instruction_64 = clamp(((r6).yyzy) * (vec4<f32>(0.0, 1.4285714626312256, 5.0, 0.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r6 = vec4<f32>(r6.x, instruction_64.y, instruction_64.z, r6.w);
    // mad r7.xy, r6.yzyy, l(-2.000000, -2.000000, 0.000000, 0.000000), l(3.000000, 3.000000, 0.000000, 0.000000)
    let instruction_65 = fma((r6).yzyy, vec4<f32>(-2.0, -2.0, 0.0, 0.0), vec4<f32>(3.0, 3.0, 0.0, 0.0));
    r7 = vec4<f32>(instruction_65.x, instruction_65.y, r7.z, r7.w);
    // mul r6.yz, r6.yyzy, r6.yyzy
    let instruction_66 = ((r6).yyzy) * ((r6).yyzy);
    r6 = vec4<f32>(r6.x, instruction_66.y, instruction_66.z, r6.w);
    // mul r6.yz, r6.yyzy, r7.xxyx
    let instruction_67 = ((r6).yyzy) * ((r7).xxyx);
    r6 = vec4<f32>(r6.x, instruction_67.y, instruction_67.z, r6.w);
    // min r6.yz, r6.yyzy, l(0.000000, 1.000000, 1.000000, 0.000000)
    let instruction_68 = min((r6).yyzy, vec4<f32>(0.0, 1.0, 1.0, 0.0));
    r6 = vec4<f32>(r6.x, instruction_68.y, instruction_68.z, r6.w);
    // mad r6.y, r6.y, l(4.000000), l(1.000000)
    let instruction_69 = fma((r6).yyyy, vec4<f32>(4.0), vec4<f32>(1.0));
    r6 = vec4<f32>(r6.x, instruction_69.y, r6.z, r6.w);
    // mul r2.y, r2.y, r6.y
    let instruction_70 = ((r2).yyyy) * ((r6).yyyy);
    r2 = vec4<f32>(r2.x, instruction_70.y, r2.z, r2.w);
    // div r4.y, r2.y, r0.y
    let instruction_71 = ((r2).yyyy) / ((r0).yyyy);
    r4 = vec4<f32>(r4.x, instruction_71.y, r4.z, r4.w);
    // mul r4.x, r1.z, r4.y
    let instruction_72 = ((r1).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_72.x, r4.y, r4.z, r4.w);
    // add r2.y, r6.x, r6.x
    let instruction_73 = ((r6).xxxx) + ((r6).xxxx);
    r2 = vec4<f32>(r2.x, instruction_73.y, r2.z, r2.w);
    // min r2.y, r2.y, l(1.000000)
    let instruction_74 = min((r2).yyyy, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_74.y, r2.z, r2.w);
    // mad r6.x, r2.y, l(-2.000000), l(3.000000)
    let instruction_75 = fma((r2).yyyy, vec4<f32>(-2.0), vec4<f32>(3.0));
    r6 = vec4<f32>(instruction_75.x, r6.y, r6.z, r6.w);
    // mul r2.y, r2.y, r2.y
    let instruction_76 = ((r2).yyyy) * ((r2).yyyy);
    r2 = vec4<f32>(r2.x, instruction_76.y, r2.z, r2.w);
    // mul r2.y, r2.y, r6.x
    let instruction_77 = ((r2).yyyy) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, instruction_77.y, r2.z, r2.w);
    // min r2.y, r2.y, l(1.000000)
    let instruction_78 = min((r2).yyyy, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_78.y, r2.z, r2.w);
    // add r6.x, -r6.z, l(1.000000)
    let instruction_79 = (-((r6).zzzz)) + (vec4<f32>(1.0));
    r6 = vec4<f32>(instruction_79.x, r6.y, r6.z, r6.w);
    // mul r2.y, r2.y, r6.x
    let instruction_80 = ((r2).yyyy) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, instruction_80.y, r2.z, r2.w);
    // mul r5.xyz, r2.yyyy, l(0.578421, 0.272198, 0.272198, 0.000000)
    let instruction_81 = ((r2).yyyy) * (vec4<f32>(0.5784212350845337, 0.27219823002815247, 0.27219823002815247, 0.0));
    r5 = vec4<f32>(instruction_81.x, instruction_81.y, instruction_81.z, r5.w);
    // mov r4.zw, -r1.xxxy
    let instruction_82 = -((r1).xxxy);
    r4 = vec4<f32>(r4.x, r4.y, instruction_82.z, instruction_82.w);
    // mov o0.xy, l(1.000000,1.000000,0,0)
    let instruction_83 = vec4<f32>(1.0, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_83.x, instruction_83.y, o0.z, o0.w);
    // mov r2.x, l(1.000000)
    let instruction_84 = vec4<f32>(1.0);
    r2 = vec4<f32>(instruction_84.x, r2.y, r2.z, r2.w);
    // mov r5.w, l(1.000000)
    let instruction_85 = vec4<f32>(1.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_85.w);
    }
    // eq r2.y, r5.w, l(0.000000)
    let instruction_86 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r5).wwww) == (vec4<f32>(0.0))));
    r2 = vec4<f32>(r2.x, instruction_86.y, r2.z, r2.w);
    // and r2.y, r2.z, r2.y
    let instruction_87 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).zzzz) & bitcast<vec4<u32>>((r2).yyyy));
    r2 = vec4<f32>(r2.x, instruction_87.y, r2.z, r2.w);
    // if_nz r2.y
    if bitcast<vec4<u32>>((r2).yyyy).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.250000, 0.250000, 0.000000, 0.000000)
    let instruction_88 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.25, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_88.x, instruction_88.y, r3.z, r3.w);
    // mul r4.zw, r0.zzzw, l(0.000000, 0.000000, -0.400000, 0.400000)
    let instruction_89 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, -0.4000000059604645, 0.4000000059604645));
    r4 = vec4<f32>(r4.x, r4.y, instruction_89.z, instruction_89.w);
    // mul r2.y, cb2[5].y, l(20.000000)
    let instruction_90 = ((material.source[13]).yyyy) * (vec4<f32>(20.0));
    r2 = vec4<f32>(r2.x, instruction_90.y, r2.z, r2.w);
    // max r2.z, |r4.z|, |r4.w|
    let instruction_91 = max(abs((r4).zzzz), abs((r4).wwww));
    r2 = vec4<f32>(r2.x, r2.y, instruction_91.z, r2.w);
    // add r2.z, r2.z, l(-1.000000)
    let instruction_92 = ((r2).zzzz) + (vec4<f32>(-1.0));
    r2 = vec4<f32>(r2.x, r2.y, instruction_92.z, r2.w);
    // max r2.z, -r2.z, l(0.000000)
    let instruction_93 = max(-((r2).zzzz), vec4<f32>(0.0));
    r2 = vec4<f32>(r2.x, r2.y, instruction_93.z, r2.w);
    // mad r6.x, r2.z, l(-2.000000), l(3.000000)
    let instruction_94 = fma((r2).zzzz, vec4<f32>(-2.0), vec4<f32>(3.0));
    r6 = vec4<f32>(instruction_94.x, r6.y, r6.z, r6.w);
    // mul r2.z, r2.z, r2.z
    let instruction_95 = ((r2).zzzz) * ((r2).zzzz);
    r2 = vec4<f32>(r2.x, r2.y, instruction_95.z, r2.w);
    // mul r2.z, r2.z, r6.x
    let instruction_96 = ((r2).zzzz) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, instruction_96.z, r2.w);
    // min r2.z, r2.z, l(1.000000)
    let instruction_97 = min((r2).zzzz, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, instruction_97.z, r2.w);
    // mul r2.y, r2.z, r2.y
    let instruction_98 = ((r2).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(r2.x, instruction_98.y, r2.z, r2.w);
    // div r4.y, r2.y, r0.y
    let instruction_99 = ((r2).yyyy) / ((r0).yyyy);
    r4 = vec4<f32>(r4.x, instruction_99.y, r4.z, r4.w);
    // mul r4.x, r1.z, r4.y
    let instruction_100 = ((r1).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_100.x, r4.y, r4.z, r4.w);
    // max r2.y, |r1.x|, |r1.y|
    let instruction_101 = max(abs((r1).xxxx), abs((r1).yyyy));
    r2 = vec4<f32>(r2.x, instruction_101.y, r2.z, r2.w);
    // add r2.z, r2.y, r2.y
    let instruction_102 = ((r2).yyyy) + ((r2).yyyy);
    r2 = vec4<f32>(r2.x, r2.y, instruction_102.z, r2.w);
    // min r2.z, r2.z, l(1.000000)
    let instruction_103 = min((r2).zzzz, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, instruction_103.z, r2.w);
    // mad r6.x, r2.z, l(-2.000000), l(3.000000)
    let instruction_104 = fma((r2).zzzz, vec4<f32>(-2.0), vec4<f32>(3.0));
    r6 = vec4<f32>(instruction_104.x, r6.y, r6.z, r6.w);
    // mul r2.z, r2.z, r2.z
    let instruction_105 = ((r2).zzzz) * ((r2).zzzz);
    r2 = vec4<f32>(r2.x, r2.y, instruction_105.z, r2.w);
    // mul r2.z, r2.z, r6.x
    let instruction_106 = ((r2).zzzz) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, instruction_106.z, r2.w);
    // add r2.y, r2.y, l(-0.800000)
    let instruction_107 = ((r2).yyyy) + (vec4<f32>(-0.800000011920929));
    r2 = vec4<f32>(r2.x, instruction_107.y, r2.z, r2.w);
    // mul_sat r2.y, r2.y, l(5.000000)
    let instruction_108 = clamp(((r2).yyyy) * (vec4<f32>(5.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_108.y, r2.z, r2.w);
    // mad r6.x, r2.y, l(-2.000000), l(3.000000)
    let instruction_109 = fma((r2).yyyy, vec4<f32>(-2.0), vec4<f32>(3.0));
    r6 = vec4<f32>(instruction_109.x, r6.y, r6.z, r6.w);
    // mul r2.y, r2.y, r2.y
    let instruction_110 = ((r2).yyyy) * ((r2).yyyy);
    r2 = vec4<f32>(r2.x, instruction_110.y, r2.z, r2.w);
    // mul r2.y, r2.y, r6.x
    let instruction_111 = ((r2).yyyy) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, instruction_111.y, r2.z, r2.w);
    // min r2.yz, r2.yyzy, l(0.000000, 1.000000, 1.000000, 0.000000)
    let instruction_112 = min((r2).yyzy, vec4<f32>(0.0, 1.0, 1.0, 0.0));
    r2 = vec4<f32>(r2.x, instruction_112.y, instruction_112.z, r2.w);
    // add r2.y, -r2.y, l(1.000000)
    let instruction_113 = (-((r2).yyyy)) + (vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_113.y, r2.z, r2.w);
    // mul r2.y, r2.y, r2.z
    let instruction_114 = ((r2).yyyy) * ((r2).zzzz);
    r2 = vec4<f32>(r2.x, instruction_114.y, r2.z, r2.w);
    // mul r5.xyz, r2.yyyy, l(0.543485, 0.537592, 0.000000, 0.000000)
    let instruction_115 = ((r2).yyyy) * (vec4<f32>(0.5434852838516235, 0.5375915169715881, 0.0, 0.0));
    r5 = vec4<f32>(instruction_115.x, instruction_115.y, instruction_115.z, r5.w);
    // mov o0.xy, l(0.500000,0.500000,0,0)
    let instruction_116 = vec4<f32>(0.5, 0.5, 0.0, 0.0);
    o0 = vec4<f32>(instruction_116.x, instruction_116.y, o0.z, o0.w);
    // mov r2.x, l(1.000000)
    let instruction_117 = vec4<f32>(1.0);
    r2 = vec4<f32>(instruction_117.x, r2.y, r2.z, r2.w);
    // mov r5.w, l(1.000000)
    let instruction_118 = vec4<f32>(1.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_118.w);
    }
    // eq r2.y, r5.w, l(0.000000)
    let instruction_119 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r5).wwww) == (vec4<f32>(0.0))));
    r2 = vec4<f32>(r2.x, instruction_119.y, r2.z, r2.w);
    // and r2.y, r2.w, r2.y
    let instruction_120 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).wwww) & bitcast<vec4<u32>>((r2).yyyy));
    r2 = vec4<f32>(r2.x, instruction_120.y, r2.z, r2.w);
    // if_nz r2.y
    if bitcast<vec4<u32>>((r2).yyyy).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.000000, 0.250000, 0.000000, 0.000000)
    let instruction_121 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.0, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_121.x, instruction_121.y, r3.z, r3.w);
    // mul r4.zw, r0.zzzw, l(0.000000, 0.000000, -0.850000, 0.850000)
    let instruction_122 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, -0.8500000238418579, 0.8500000238418579));
    r4 = vec4<f32>(r4.x, r4.y, instruction_122.z, instruction_122.w);
    // mul r2.y, cb2[5].y, l(35.000000)
    let instruction_123 = ((material.source[13]).yyyy) * (vec4<f32>(35.0));
    r2 = vec4<f32>(r2.x, instruction_123.y, r2.z, r2.w);
    // max r2.z, |r4.z|, |r4.w|
    let instruction_124 = max(abs((r4).zzzz), abs((r4).wwww));
    r2 = vec4<f32>(r2.x, r2.y, instruction_124.z, r2.w);
    // mul r2.z, r2.z, l(1.428571)
    let instruction_125 = ((r2).zzzz) * (vec4<f32>(1.4285714626312256));
    r2 = vec4<f32>(r2.x, r2.y, instruction_125.z, r2.w);
    // min r2.z, r2.z, l(1.000000)
    let instruction_126 = min((r2).zzzz, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, instruction_126.z, r2.w);
    // mad r2.w, r2.z, l(-2.000000), l(3.000000)
    let instruction_127 = fma((r2).zzzz, vec4<f32>(-2.0), vec4<f32>(3.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_127.w);
    // mul r2.z, r2.z, r2.z
    let instruction_128 = ((r2).zzzz) * ((r2).zzzz);
    r2 = vec4<f32>(r2.x, r2.y, instruction_128.z, r2.w);
    // mul r2.z, r2.z, r2.w
    let instruction_129 = ((r2).zzzz) * ((r2).wwww);
    r2 = vec4<f32>(r2.x, r2.y, instruction_129.z, r2.w);
    // min r2.z, r2.z, l(1.000000)
    let instruction_130 = min((r2).zzzz, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, instruction_130.z, r2.w);
    // mul r2.y, r2.z, r2.y
    let instruction_131 = ((r2).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(r2.x, instruction_131.y, r2.z, r2.w);
    // div r4.y, r2.y, r0.y
    let instruction_132 = ((r2).yyyy) / ((r0).yyyy);
    r4 = vec4<f32>(r4.x, instruction_132.y, r4.z, r4.w);
    // mul r4.x, r1.z, r4.y
    let instruction_133 = ((r1).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_133.x, r4.y, r4.z, r4.w);
    // max r2.y, |r1.x|, |r1.y|
    let instruction_134 = max(abs((r1).xxxx), abs((r1).yyyy));
    r2 = vec4<f32>(r2.x, instruction_134.y, r2.z, r2.w);
    // add r2.z, r2.y, r2.y
    let instruction_135 = ((r2).yyyy) + ((r2).yyyy);
    r2 = vec4<f32>(r2.x, r2.y, instruction_135.z, r2.w);
    // min r2.z, r2.z, l(1.000000)
    let instruction_136 = min((r2).zzzz, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, instruction_136.z, r2.w);
    // mad r2.w, r2.z, l(-2.000000), l(3.000000)
    let instruction_137 = fma((r2).zzzz, vec4<f32>(-2.0), vec4<f32>(3.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_137.w);
    // mul r2.z, r2.z, r2.z
    let instruction_138 = ((r2).zzzz) * ((r2).zzzz);
    r2 = vec4<f32>(r2.x, r2.y, instruction_138.z, r2.w);
    // mul r2.z, r2.z, r2.w
    let instruction_139 = ((r2).zzzz) * ((r2).wwww);
    r2 = vec4<f32>(r2.x, r2.y, instruction_139.z, r2.w);
    // add r2.y, r2.y, l(-0.800000)
    let instruction_140 = ((r2).yyyy) + (vec4<f32>(-0.800000011920929));
    r2 = vec4<f32>(r2.x, instruction_140.y, r2.z, r2.w);
    // mul_sat r2.y, r2.y, l(5.000000)
    let instruction_141 = clamp(((r2).yyyy) * (vec4<f32>(5.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_141.y, r2.z, r2.w);
    // mad r2.w, r2.y, l(-2.000000), l(3.000000)
    let instruction_142 = fma((r2).yyyy, vec4<f32>(-2.0), vec4<f32>(3.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_142.w);
    // mul r2.y, r2.y, r2.y
    let instruction_143 = ((r2).yyyy) * ((r2).yyyy);
    r2 = vec4<f32>(r2.x, instruction_143.y, r2.z, r2.w);
    // mul r2.y, r2.y, r2.w
    let instruction_144 = ((r2).yyyy) * ((r2).wwww);
    r2 = vec4<f32>(r2.x, instruction_144.y, r2.z, r2.w);
    // min r2.yz, r2.yyzy, l(0.000000, 1.000000, 1.000000, 0.000000)
    let instruction_145 = min((r2).yyzy, vec4<f32>(0.0, 1.0, 1.0, 0.0));
    r2 = vec4<f32>(r2.x, instruction_145.y, instruction_145.z, r2.w);
    // add r2.y, -r2.y, l(1.000000)
    let instruction_146 = (-((r2).yyyy)) + (vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_146.y, r2.z, r2.w);
    // mul r2.y, r2.y, r2.z
    let instruction_147 = ((r2).yyyy) * ((r2).zzzz);
    r2 = vec4<f32>(r2.x, instruction_147.y, r2.z, r2.w);
    // mul r5.xyz, r2.yyyy, l(0.475436, 0.680496, 0.437461, 0.000000)
    let instruction_148 = ((r2).yyyy) * (vec4<f32>(0.475435733795166, 0.6804956197738647, 0.43746134638786316, 0.0));
    r5 = vec4<f32>(instruction_148.x, instruction_148.y, instruction_148.z, r5.w);
    // mov o0.xy, l(0.800000,1.000000,0,0)
    let instruction_149 = vec4<f32>(0.800000011920929, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_149.x, instruction_149.y, o0.z, o0.w);
    // mov r2.xyz, l(1.000000,0.010000,0.010000,0)
    let instruction_150 = vec4<f32>(1.0, 0.009999999776482582, 0.009999999776482582, 0.0);
    r2 = vec4<f32>(instruction_150.x, instruction_150.y, instruction_150.z, r2.w);
    // mov r5.w, l(1.000000)
    let instruction_151 = vec4<f32>(1.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_151.w);
    } else {
    // mov r2.yz, l(0,0,0,0)
    let instruction_152 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r2 = vec4<f32>(r2.x, instruction_152.y, instruction_152.z, r2.w);
    }
    // eq r2.w, r5.w, l(0.000000)
    let instruction_153 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r5).wwww) == (vec4<f32>(0.0))));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_153.w);
    // lt r6.xyz, r0.xxxx, l(7.000000, 12.000000, 13.000000, 0.000000)
    let instruction_154 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(7.0, 12.0, 13.0, 0.0))));
    r6 = vec4<f32>(instruction_154.x, instruction_154.y, instruction_154.z, r6.w);
    // and r2.w, r2.w, r6.x
    let instruction_155 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).wwww) & bitcast<vec4<u32>>((r6).xxxx));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_155.w);
    // if_nz r2.w
    if bitcast<vec4<u32>>((r2).wwww).x != 0u {
    // add r2.w, r0.x, l(-4.000000)
    let instruction_156 = ((r0).xxxx) + (vec4<f32>(-4.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_156.w);
    // ftoi r2.w, r2.w
    let instruction_157 = bitcast<vec4<f32>>(vec4<i32>((r2).wwww));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_157.w);
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.125000, 0.250000, 0.000000, 0.000000)
    let instruction_158 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.125, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_158.x, instruction_158.y, r3.z, r3.w);
    // mad r6.x, icb[r2.w + 0].x, l(-1.700000), l(0.700000)
    let instruction_159 = fma((original_icb[i32(bitcast<u32>(r2.w)) + 0]).xxxx, vec4<f32>(-1.7000000476837158), vec4<f32>(0.699999988079071));
    r6 = vec4<f32>(instruction_159.x, r6.y, r6.z, r6.w);
    // mad r6.x, r6.x, l(0.500000), l(0.500000)
    let instruction_160 = fma((r6).xxxx, vec4<f32>(0.5), vec4<f32>(0.5));
    r6 = vec4<f32>(instruction_160.x, r6.y, r6.z, r6.w);
    // mul r6.xw, r0.zzzw, r6.xxxx
    let instruction_161 = ((r0).zzzw) * ((r6).xxxx);
    r6 = vec4<f32>(instruction_161.x, r6.y, r6.z, instruction_161.w);
    // mad r4.zw, r6.xxxw, l(0.000000, 0.000000, 2.000000, -2.000000), -r1.xxxy
    let instruction_162 = fma((r6).xxxw, vec4<f32>(0.0, 0.0, 2.0, -2.0), -((r1).xxxy));
    r4 = vec4<f32>(r4.x, r4.y, instruction_162.z, instruction_162.w);
    // mad r6.x, icb[r2.w + 0].y, l(6.000000), l(2.000000)
    let instruction_163 = fma((original_icb[i32(bitcast<u32>(r2.w)) + 0]).yyyy, vec4<f32>(6.0), vec4<f32>(2.0));
    r6 = vec4<f32>(instruction_163.x, r6.y, r6.z, r6.w);
    // mul r6.x, r6.x, cb2[5].y
    let instruction_164 = ((r6).xxxx) * ((material.source[13]).yyyy);
    r6 = vec4<f32>(instruction_164.x, r6.y, r6.z, r6.w);
    // max r6.w, |r1.x|, |r1.y|
    let instruction_165 = max(abs((r1).xxxx), abs((r1).yyyy));
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_165.w);
    // min r7.x, r6.w, l(1.000000)
    let instruction_166 = min((r6).wwww, vec4<f32>(1.0));
    r7 = vec4<f32>(instruction_166.x, r7.y, r7.z, r7.w);
    // mad r7.y, r7.x, l(-2.000000), l(3.000000)
    let instruction_167 = fma((r7).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r7 = vec4<f32>(r7.x, instruction_167.y, r7.z, r7.w);
    // mul r7.x, r7.x, r7.x
    let instruction_168 = ((r7).xxxx) * ((r7).xxxx);
    r7 = vec4<f32>(instruction_168.x, r7.y, r7.z, r7.w);
    // mul r7.x, r7.x, r7.y
    let instruction_169 = ((r7).xxxx) * ((r7).yyyy);
    r7 = vec4<f32>(instruction_169.x, r7.y, r7.z, r7.w);
    // min r7.x, r7.x, l(1.000000)
    let instruction_170 = min((r7).xxxx, vec4<f32>(1.0));
    r7 = vec4<f32>(instruction_170.x, r7.y, r7.z, r7.w);
    // mul r6.x, r6.x, r7.x
    let instruction_171 = ((r6).xxxx) * ((r7).xxxx);
    r6 = vec4<f32>(instruction_171.x, r6.y, r6.z, r6.w);
    // mul r6.x, r6.x, l(0.800000)
    let instruction_172 = ((r6).xxxx) * (vec4<f32>(0.800000011920929));
    r6 = vec4<f32>(instruction_172.x, r6.y, r6.z, r6.w);
    // div r4.y, r6.x, r0.y
    let instruction_173 = ((r6).xxxx) / ((r0).yyyy);
    r4 = vec4<f32>(r4.x, instruction_173.y, r4.z, r4.w);
    // mul r4.x, r1.z, r4.y
    let instruction_174 = ((r1).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_174.x, r4.y, r4.z, r4.w);
    // mad r2.w, icb[r2.w + 0].z, l(0.400000), l(0.600000)
    let instruction_175 = fma((original_icb[i32(bitcast<u32>(r2.w)) + 0]).zzzz, vec4<f32>(0.4000000059604645), vec4<f32>(0.6000000238418579));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_175.w);
    // mul r7.xyz, r2.wwww, l(0.698661, 1.000000, 0.642857, 0.000000)
    let instruction_176 = ((r2).wwww) * (vec4<f32>(0.6986610293388367, 1.0, 0.642857015132904, 0.0));
    r7 = vec4<f32>(instruction_176.x, instruction_176.y, instruction_176.z, r7.w);
    // add r2.w, r6.w, r6.w
    let instruction_177 = ((r6).wwww) + ((r6).wwww);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_177.w);
    // min r2.w, r2.w, l(1.000000)
    let instruction_178 = min((r2).wwww, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_178.w);
    // mad r6.x, r2.w, l(-2.000000), l(3.000000)
    let instruction_179 = fma((r2).wwww, vec4<f32>(-2.0), vec4<f32>(3.0));
    r6 = vec4<f32>(instruction_179.x, r6.y, r6.z, r6.w);
    // mul r2.w, r2.w, r2.w
    let instruction_180 = ((r2).wwww) * ((r2).wwww);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_180.w);
    // mul r2.w, r2.w, r6.x
    let instruction_181 = ((r2).wwww) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_181.w);
    // min r2.w, r2.w, l(1.000000)
    let instruction_182 = min((r2).wwww, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_182.w);
    // add r6.x, r6.w, l(-0.800000)
    let instruction_183 = ((r6).wwww) + (vec4<f32>(-0.800000011920929));
    r6 = vec4<f32>(instruction_183.x, r6.y, r6.z, r6.w);
    // mul_sat r6.x, r6.x, l(5.000000)
    let instruction_184 = clamp(((r6).xxxx) * (vec4<f32>(5.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r6 = vec4<f32>(instruction_184.x, r6.y, r6.z, r6.w);
    // mad r6.w, r6.x, l(-2.000000), l(3.000000)
    let instruction_185 = fma((r6).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_185.w);
    // mul r6.x, r6.x, r6.x
    let instruction_186 = ((r6).xxxx) * ((r6).xxxx);
    r6 = vec4<f32>(instruction_186.x, r6.y, r6.z, r6.w);
    // mul r6.x, r6.x, r6.w
    let instruction_187 = ((r6).xxxx) * ((r6).wwww);
    r6 = vec4<f32>(instruction_187.x, r6.y, r6.z, r6.w);
    // min r6.x, r6.x, l(1.000000)
    let instruction_188 = min((r6).xxxx, vec4<f32>(1.0));
    r6 = vec4<f32>(instruction_188.x, r6.y, r6.z, r6.w);
    // mad r6.x, r6.x, l(-0.500000), l(1.000000)
    let instruction_189 = fma((r6).xxxx, vec4<f32>(-0.5), vec4<f32>(1.0));
    r6 = vec4<f32>(instruction_189.x, r6.y, r6.z, r6.w);
    // mul r2.w, r2.w, r6.x
    let instruction_190 = ((r2).wwww) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_190.w);
    // mul r5.xyz, r2.wwww, r7.xyzx
    let instruction_191 = ((r2).wwww) * ((r7).xyzx);
    r5 = vec4<f32>(instruction_191.x, instruction_191.y, instruction_191.z, r5.w);
    // mov o0.xy, l(2.000000,6.000000,0,0)
    let instruction_192 = vec4<f32>(2.0, 6.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_192.x, instruction_192.y, o0.z, o0.w);
    // mov r2.xyz, l(1.000000,0,0,0)
    let instruction_193 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    r2 = vec4<f32>(instruction_193.x, instruction_193.y, instruction_193.z, r2.w);
    // mov r5.w, l(1.000000)
    let instruction_194 = vec4<f32>(1.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_194.w);
    }
    // eq r2.w, r5.w, l(0.000000)
    let instruction_195 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r5).wwww) == (vec4<f32>(0.0))));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_195.w);
    // and r2.w, r6.y, r2.w
    let instruction_196 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r6).yyyy) & bitcast<vec4<u32>>((r2).wwww));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_196.w);
    // if_nz r2.w
    if bitcast<vec4<u32>>((r2).wwww).x != 0u {
    // add r0.x, r0.x, l(-7.000000)
    let instruction_197 = ((r0).xxxx) + (vec4<f32>(-7.0));
    r0 = vec4<f32>(instruction_197.x, r0.y, r0.z, r0.w);
    // ftoi r0.x, r0.x
    let instruction_198 = bitcast<vec4<f32>>(vec4<i32>((r0).xxxx));
    r0 = vec4<f32>(instruction_198.x, r0.y, r0.z, r0.w);
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.125000, 0.250000, 0.000000, 0.000000)
    let instruction_199 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.125, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_199.x, instruction_199.y, r3.z, r3.w);
    // mad r2.w, icb[r0.x + 0].w, l(-2.500000), l(1.250000)
    let instruction_200 = fma((original_icb[i32(bitcast<u32>(r0.x)) + 0]).wwww, vec4<f32>(-2.5), vec4<f32>(1.25));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_200.w);
    // mad r2.w, r2.w, l(0.500000), l(0.500000)
    let instruction_201 = fma((r2).wwww, vec4<f32>(0.5), vec4<f32>(0.5));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_201.w);
    // mul r0.zw, r0.zzzw, r2.wwww
    let instruction_202 = ((r0).zzzw) * ((r2).wwww);
    r0 = vec4<f32>(r0.x, r0.y, instruction_202.z, instruction_202.w);
    // mad r4.zw, r0.zzzw, l(0.000000, 0.000000, 2.000000, -2.000000), -r1.xxxy
    let instruction_203 = fma((r0).zzzw, vec4<f32>(0.0, 0.0, 2.0, -2.0), -((r1).xxxy));
    r4 = vec4<f32>(r4.x, r4.y, instruction_203.z, instruction_203.w);
    // mad r0.z, icb[r0.x + 3].x, l(2.500000), l(1.500000)
    let instruction_204 = fma((original_icb[i32(bitcast<u32>(r0.x)) + 3]).xxxx, vec4<f32>(2.5), vec4<f32>(1.5));
    r0 = vec4<f32>(r0.x, r0.y, instruction_204.z, r0.w);
    // mul r0.z, r0.z, cb2[5].y
    let instruction_205 = ((r0).zzzz) * ((material.source[13]).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_205.z, r0.w);
    // max r0.w, |r1.x|, |r1.y|
    let instruction_206 = max(abs((r1).xxxx), abs((r1).yyyy));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_206.w);
    // mul r2.w, r0.w, l(4.000000)
    let instruction_207 = ((r0).wwww) * (vec4<f32>(4.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_207.w);
    // min r2.w, r2.w, l(1.000000)
    let instruction_208 = min((r2).wwww, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_208.w);
    // mad r6.x, r2.w, l(-2.000000), l(3.000000)
    let instruction_209 = fma((r2).wwww, vec4<f32>(-2.0), vec4<f32>(3.0));
    r6 = vec4<f32>(instruction_209.x, r6.y, r6.z, r6.w);
    // mul r2.w, r2.w, r2.w
    let instruction_210 = ((r2).wwww) * ((r2).wwww);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_210.w);
    // mul r2.w, r2.w, r6.x
    let instruction_211 = ((r2).wwww) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_211.w);
    // min r2.w, r2.w, l(1.000000)
    let instruction_212 = min((r2).wwww, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_212.w);
    // mul r0.z, r0.z, r2.w
    let instruction_213 = ((r0).zzzz) * ((r2).wwww);
    r0 = vec4<f32>(r0.x, r0.y, instruction_213.z, r0.w);
    // div r4.y, r0.z, r0.y
    let instruction_214 = ((r0).zzzz) / ((r0).yyyy);
    r4 = vec4<f32>(r4.x, instruction_214.y, r4.z, r4.w);
    // mul r4.x, r1.z, r4.y
    let instruction_215 = ((r1).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_215.x, r4.y, r4.z, r4.w);
    // mad r0.x, icb[r0.x + 3].y, l(0.500000), l(0.400000)
    let instruction_216 = fma((original_icb[i32(bitcast<u32>(r0.x)) + 3]).yyyy, vec4<f32>(0.5), vec4<f32>(0.4000000059604645));
    r0 = vec4<f32>(instruction_216.x, r0.y, r0.z, r0.w);
    // mul r6.xyw, r0.xxxx, l(1.000000, 0.732143, 0.000000, 0.571429)
    let instruction_217 = ((r0).xxxx) * (vec4<f32>(1.0, 0.732142984867096, 0.0, 0.5714290142059326));
    r6 = vec4<f32>(instruction_217.x, instruction_217.y, r6.z, instruction_217.w);
    // add r0.x, r0.w, r0.w
    let instruction_218 = ((r0).wwww) + ((r0).wwww);
    r0 = vec4<f32>(instruction_218.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_219 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_219.x, r0.y, r0.z, r0.w);
    // mad r0.z, r0.x, l(-2.000000), l(3.000000)
    let instruction_220 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_220.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_221 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_221.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.z
    let instruction_222 = ((r0).xxxx) * ((r0).zzzz);
    r0 = vec4<f32>(instruction_222.x, r0.y, r0.z, r0.w);
    // add r0.z, r0.w, l(-0.800000)
    let instruction_223 = ((r0).wwww) + (vec4<f32>(-0.800000011920929));
    r0 = vec4<f32>(r0.x, r0.y, instruction_223.z, r0.w);
    // mul_sat r0.z, r0.z, l(5.000000)
    let instruction_224 = clamp(((r0).zzzz) * (vec4<f32>(5.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_224.z, r0.w);
    // mad r0.w, r0.z, l(-2.000000), l(3.000000)
    let instruction_225 = fma((r0).zzzz, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_225.w);
    // mul r0.z, r0.z, r0.z
    let instruction_226 = ((r0).zzzz) * ((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_226.z, r0.w);
    // mul r0.z, r0.z, r0.w
    let instruction_227 = ((r0).zzzz) * ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, instruction_227.z, r0.w);
    // min r0.xz, r0.xxzx, l(1.000000, 0.000000, 1.000000, 0.000000)
    let instruction_228 = min((r0).xxzx, vec4<f32>(1.0, 0.0, 1.0, 0.0));
    r0 = vec4<f32>(instruction_228.x, r0.y, instruction_228.z, r0.w);
    // add r0.z, -r0.z, l(1.000000)
    let instruction_229 = (-((r0).zzzz)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_229.z, r0.w);
    // mul r0.x, r0.z, r0.x
    let instruction_230 = ((r0).zzzz) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_230.x, r0.y, r0.z, r0.w);
    // mul r5.xyz, r0.xxxx, r6.xywx
    let instruction_231 = ((r0).xxxx) * ((r6).xywx);
    r5 = vec4<f32>(instruction_231.x, instruction_231.y, instruction_231.z, r5.w);
    // mov o0.xy, l(4.000000,1.000000,0,0)
    let instruction_232 = vec4<f32>(4.0, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_232.x, instruction_232.y, o0.z, o0.w);
    // mov r2.xyz, l(1.000000,0,0,0)
    let instruction_233 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    r2 = vec4<f32>(instruction_233.x, instruction_233.y, instruction_233.z, r2.w);
    // mov r5.w, l(1.000000)
    let instruction_234 = vec4<f32>(1.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_234.w);
    }
    // eq r0.x, r5.w, l(0.000000)
    let instruction_235 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r5).wwww) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_235.x, r0.y, r0.z, r0.w);
    // and r0.x, r6.z, r0.x
    let instruction_236 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r6).zzzz) & bitcast<vec4<u32>>((r0).xxxx));
    r0 = vec4<f32>(instruction_236.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.500000, 0.500000, 0.000000, 0.000000), l(0.500000, 0.000000, 0.000000, 0.000000)
    let instruction_237 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.5, 0.5, 0.0, 0.0), vec4<f32>(0.5, 0.0, 0.0, 0.0));
    r3 = vec4<f32>(instruction_237.x, instruction_237.y, r3.z, r3.w);
    // mul r0.x, cb2[5].y, l(25.000000)
    let instruction_238 = ((material.source[13]).yyyy) * (vec4<f32>(25.0));
    r0 = vec4<f32>(instruction_238.x, r0.y, r0.z, r0.w);
    // div r4.y, r0.x, r0.y
    let instruction_239 = ((r0).xxxx) / ((r0).yyyy);
    r4 = vec4<f32>(r4.x, instruction_239.y, r4.z, r4.w);
    // mul r4.x, r1.z, r4.y
    let instruction_240 = ((r1).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_240.x, r4.y, r4.z, r4.w);
    // max r0.x, |r1.x|, |r1.y|
    let instruction_241 = max(abs((r1).xxxx), abs((r1).yyyy));
    r0 = vec4<f32>(instruction_241.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, l(-0.650000)
    let instruction_242 = ((r0).xxxx) + (vec4<f32>(-0.6499999761581421));
    r0 = vec4<f32>(instruction_242.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, l(2.857143)
    let instruction_243 = clamp(((r0).xxxx) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_243.x, r0.y, r0.z, r0.w);
    // mad r0.z, r0.x, l(-2.000000), l(3.000000)
    let instruction_244 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_244.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_245 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_245.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.z
    let instruction_246 = ((r0).xxxx) * ((r0).zzzz);
    r0 = vec4<f32>(instruction_246.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_247 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_247.x, r0.y, r0.z, r0.w);
    // add r0.x, -r0.x, l(1.000000)
    let instruction_248 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_248.x, r0.y, r0.z, r0.w);
    // mul r5.xyz, r0.xxxx, l(0.750000, 0.682895, 0.375000, 0.000000)
    let instruction_249 = ((r0).xxxx) * (vec4<f32>(0.75, 0.6828945279121399, 0.375, 0.0));
    r5 = vec4<f32>(instruction_249.x, instruction_249.y, instruction_249.z, r5.w);
    // mov o2.xyzw, l(0,0,1.000000,0)
    let instruction_250 = vec4<f32>(0.0, 0.0, 1.0, 0.0);
    o2 = instruction_250;
    // mov r4.zw, r1.xxxy
    let instruction_251 = (r1).xxxy;
    r4 = vec4<f32>(r4.x, r4.y, instruction_251.z, instruction_251.w);
    // mov o0.xy, l(6.000000,5.000000,0,0)
    let instruction_252 = vec4<f32>(6.0, 5.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_252.x, instruction_252.y, o0.z, o0.w);
    // mov r2.xyz, l(1.000000,0,0,0)
    let instruction_253 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    r2 = vec4<f32>(instruction_253.x, instruction_253.y, instruction_253.z, r2.w);
    // mov r5.w, l(1.000000)
    let instruction_254 = vec4<f32>(1.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_254.w);
    } else {
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_255 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_255;
    }
    // eq r0.x, r5.w, l(0.000000)
    let instruction_256 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r5).wwww) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_256.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mul r3.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000)
    let instruction_257 = ((vec4<f32>(input.uv, 0.0, 0.0)).xyxx) * (vec4<f32>(0.25, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_257.x, instruction_257.y, r3.z, r3.w);
    // mul r0.x, cb2[5].y, l(200.000000)
    let instruction_258 = ((material.source[13]).yyyy) * (vec4<f32>(200.0));
    r0 = vec4<f32>(instruction_258.x, r0.y, r0.z, r0.w);
    // div r4.y, r0.x, r0.y
    let instruction_259 = ((r0).xxxx) / ((r0).yyyy);
    r4 = vec4<f32>(r4.x, instruction_259.y, r4.z, r4.w);
    // mul r4.x, r1.z, r4.y
    let instruction_260 = ((r1).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(instruction_260.x, r4.y, r4.z, r4.w);
    // max r0.x, |r1.x|, |r1.y|
    let instruction_261 = max(abs((r1).xxxx), abs((r1).yyyy));
    r0 = vec4<f32>(instruction_261.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, l(-0.650000)
    let instruction_262 = ((r0).xxxx) + (vec4<f32>(-0.6499999761581421));
    r0 = vec4<f32>(instruction_262.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, l(2.857143)
    let instruction_263 = clamp(((r0).xxxx) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_263.x, r0.y, r0.z, r0.w);
    // mad r0.y, r0.x, l(-2.000000), l(3.000000)
    let instruction_264 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, instruction_264.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_265 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_265.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.y
    let instruction_266 = ((r0).xxxx) * ((r0).yyyy);
    r0 = vec4<f32>(instruction_266.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_267 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_267.x, r0.y, r0.z, r0.w);
    // add r0.x, -r0.x, l(1.000000)
    let instruction_268 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_268.x, r0.y, r0.z, r0.w);
    // mul r5.xyz, r0.xxxx, l(0.811763, 0.820000, 0.556429, 0.000000)
    let instruction_269 = ((r0).xxxx) * (vec4<f32>(0.8117629885673523, 0.8199999928474426, 0.5564290285110474, 0.0));
    r5 = vec4<f32>(instruction_269.x, instruction_269.y, instruction_269.z, r5.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_270 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_270;
    // mov r4.zw, r1.xxxy
    let instruction_271 = (r1).xxxy;
    r4 = vec4<f32>(r4.x, r4.y, instruction_271.z, instruction_271.w);
    // mov o0.xy, l(0,6.000000,0,0)
    let instruction_272 = vec4<f32>(0.0, 6.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_272.x, instruction_272.y, o0.z, o0.w);
    // mov r2.xyz, l(1.000000,0,0,0)
    let instruction_273 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    r2 = vec4<f32>(instruction_273.x, instruction_273.y, instruction_273.z, r2.w);
    }
    // mul r0.xyz, r5.xyzx, cb2[5].xxxx
    let instruction_274 = ((r5).xyzx) * ((material.source[13]).xxxx);
    r0 = vec4<f32>(instruction_274.x, instruction_274.y, instruction_274.z, r0.w);
    // mul r0.xyz, r0.xyzx, v1.wwww
    let instruction_275 = ((r0).xyzx) * ((input.color).wwww);
    r0 = vec4<f32>(instruction_275.x, instruction_275.y, instruction_275.z, r0.w);
    // eq r0.w, r2.x, l(0.000000)
    let instruction_276 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r2).xxxx) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_276.w);
    // dp3 r1.x, r0.xzyx, l(0.333000, 0.333000, 0.334000, 0.000000)
    let instruction_277 = vec4<f32>(dot(((r0).xzyx).xyz, (vec4<f32>(0.3330000042915344, 0.3330000042915344, 0.33399999141693115, 0.0)).xyz));
    r1 = vec4<f32>(instruction_277.x, r1.y, r1.z, r1.w);
    // lt r1.x, r1.x, l(0.003800)
    let instruction_278 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r1).xxxx) < (vec4<f32>(0.003800000064074993))));
    r1 = vec4<f32>(instruction_278.x, r1.y, r1.z, r1.w);
    // or r0.w, r0.w, r1.x
    let instruction_279 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).wwww) | bitcast<vec4<u32>>((r1).xxxx));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_279.w);
    // if_nz r0.w
    if bitcast<vec4<u32>>((r0).wwww).x != 0u {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_280 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_280;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_281 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_281;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_282 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_282;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_283 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_283;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_284 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_284;
    } else {
    // mul r0.w, r1.z, r3.z
    let instruction_285 = ((r1).zzzz) * ((r3).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_285.w);
    // min r1.x, |r3.w|, |r0.w|
    let instruction_286 = min(abs((r3).wwww), abs((r0).wwww));
    r1 = vec4<f32>(instruction_286.x, r1.y, r1.z, r1.w);
    // max r1.y, |r3.w|, |r0.w|
    let instruction_287 = max(abs((r3).wwww), abs((r0).wwww));
    r1 = vec4<f32>(r1.x, instruction_287.y, r1.z, r1.w);
    // div r1.y, l(1.000000, 1.000000, 1.000000, 1.000000), r1.y
    let instruction_288 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r1).yyyy);
    r1 = vec4<f32>(r1.x, instruction_288.y, r1.z, r1.w);
    // mul r1.x, r1.y, r1.x
    let instruction_289 = ((r1).yyyy) * ((r1).xxxx);
    r1 = vec4<f32>(instruction_289.x, r1.y, r1.z, r1.w);
    // mul r1.y, r1.x, r1.x
    let instruction_290 = ((r1).xxxx) * ((r1).xxxx);
    r1 = vec4<f32>(r1.x, instruction_290.y, r1.z, r1.w);
    // mad r2.x, r1.y, l(0.020835), l(-0.085133)
    let instruction_291 = fma((r1).yyyy, vec4<f32>(0.02083509974181652), vec4<f32>(-0.08513300120830536));
    r2 = vec4<f32>(instruction_291.x, r2.y, r2.z, r2.w);
    // mad r2.x, r1.y, r2.x, l(0.180141)
    let instruction_292 = fma((r1).yyyy, (r2).xxxx, vec4<f32>(0.18014100193977356));
    r2 = vec4<f32>(instruction_292.x, r2.y, r2.z, r2.w);
    // mad r2.x, r1.y, r2.x, l(-0.330299)
    let instruction_293 = fma((r1).yyyy, (r2).xxxx, vec4<f32>(-0.3302994966506958));
    r2 = vec4<f32>(instruction_293.x, r2.y, r2.z, r2.w);
    // mad r1.y, r1.y, r2.x, l(0.999866)
    let instruction_294 = fma((r1).yyyy, (r2).xxxx, vec4<f32>(0.9998660087585449));
    r1 = vec4<f32>(r1.x, instruction_294.y, r1.z, r1.w);
    // mul r2.x, r1.y, r1.x
    let instruction_295 = ((r1).yyyy) * ((r1).xxxx);
    r2 = vec4<f32>(instruction_295.x, r2.y, r2.z, r2.w);
    // lt r2.w, |r3.w|, |r0.w|
    let instruction_296 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (abs((r3).wwww)) < (abs((r0).wwww))));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_296.w);
    // mad r2.x, r2.x, l(-2.000000), l(1.570796)
    let instruction_297 = fma((r2).xxxx, vec4<f32>(-2.0), vec4<f32>(1.5707963705062866));
    r2 = vec4<f32>(instruction_297.x, r2.y, r2.z, r2.w);
    // and r2.x, r2.w, r2.x
    let instruction_298 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).wwww) & bitcast<vec4<u32>>((r2).xxxx));
    r2 = vec4<f32>(instruction_298.x, r2.y, r2.z, r2.w);
    // mad r1.x, r1.x, r1.y, r2.x
    let instruction_299 = fma((r1).xxxx, (r1).yyyy, (r2).xxxx);
    r1 = vec4<f32>(instruction_299.x, r1.y, r1.z, r1.w);
    // lt r1.y, r3.w, -r3.w
    let instruction_300 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).wwww) < (-((r3).wwww))));
    r1 = vec4<f32>(r1.x, instruction_300.y, r1.z, r1.w);
    // and r1.y, r1.y, l(0xc0490fdb)
    let instruction_301 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r1).yyyy) & bitcast<vec4<u32>>(vec4<f32>(-3.1415927410125732)));
    r1 = vec4<f32>(r1.x, instruction_301.y, r1.z, r1.w);
    // add r1.x, r1.y, r1.x
    let instruction_302 = ((r1).yyyy) + ((r1).xxxx);
    r1 = vec4<f32>(instruction_302.x, r1.y, r1.z, r1.w);
    // min r1.y, r3.w, r0.w
    let instruction_303 = min((r3).wwww, (r0).wwww);
    r1 = vec4<f32>(r1.x, instruction_303.y, r1.z, r1.w);
    // max r0.w, r3.w, r0.w
    let instruction_304 = max((r3).wwww, (r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_304.w);
    // lt r1.y, r1.y, -r1.y
    let instruction_305 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r1).yyyy) < (-((r1).yyyy))));
    r1 = vec4<f32>(r1.x, instruction_305.y, r1.z, r1.w);
    // ge r0.w, r0.w, -r0.w
    let instruction_306 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).wwww) >= (-((r0).wwww))));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_306.w);
    // and r0.w, r0.w, r1.y
    let instruction_307 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).wwww) & bitcast<vec4<u32>>((r1).yyyy));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_307.w);
    // movc r0.w, r0.w, -r1.x, r1.x
    let instruction_308 = select((r1).xxxx, -((r1).xxxx), bitcast<vec4<u32>>((r0).wwww) != vec4<u32>(0u));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_308.w);
    // mad r1.xy, v2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_309 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_309.x, instruction_309.y, r1.z, r1.w);
    // sincos r2.x, r5.x, -r0.w
    let angle310 = -((r0).wwww);
    let instruction_310 = sin(angle310);
    r2 = vec4<f32>(instruction_310.x, r2.y, r2.z, r2.w);
    let instruction_311 = cos(angle310);
    r5 = vec4<f32>(instruction_311.x, r5.y, r5.z, r5.w);
    // mov r1.w, l(1.000000)
    let instruction_312 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_312.w);
    // div r1.xy, r1.xyxx, r1.zwzz
    let instruction_313 = ((r1).xyxx) / ((r1).zwzz);
    r1 = vec4<f32>(instruction_313.x, instruction_313.y, r1.z, r1.w);
    // mul r1.xy, r4.xyxx, r1.xyxx
    let instruction_314 = ((r4).xyxx) * ((r1).xyxx);
    r1 = vec4<f32>(instruction_314.x, instruction_314.y, r1.z, r1.w);
    // mov r5.y, r5.x
    let instruction_315 = (r5).xxxx;
    r5 = vec4<f32>(r5.x, instruction_315.y, r5.z, r5.w);
    // mov r5.z, r2.x
    let instruction_316 = (r2).xxxx;
    r5 = vec4<f32>(r5.x, r5.y, instruction_316.z, r5.w);
    // dp2 r3.z, r5.yzyy, r1.xyxx
    let instruction_317 = vec4<f32>(dot(((r5).yzyy).xy, ((r1).xyxx).xy));
    r3 = vec4<f32>(r3.x, r3.y, instruction_317.z, r3.w);
    // mov r5.x, -r2.x
    let instruction_318 = -((r2).xxxx);
    r5 = vec4<f32>(instruction_318.x, r5.y, r5.z, r5.w);
    // dp2 r3.w, r5.xyxx, r1.xyxx
    let instruction_319 = vec4<f32>(dot(((r5).xyxx).xy, ((r1).xyxx).xy));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_319.w);
    // mad r1.xy, r3.zwzz, r1.zwzz, r2.yzyy
    let instruction_320 = fma((r3).zwzz, (r1).zwzz, (r2).yzyy);
    r1 = vec4<f32>(instruction_320.x, instruction_320.y, r1.z, r1.w);
    // add r1.xy, r4.zwzz, r1.xyxx
    let instruction_321 = ((r4).zwzz) + ((r1).xyxx);
    r1 = vec4<f32>(instruction_321.x, instruction_321.y, r1.z, r1.w);
    // mov o4.y, -r1.y
    let instruction_322 = -((r1).yyyy);
    o4 = vec4<f32>(o4.x, instruction_322.y, o4.z, o4.w);
    // mad o0.zw, r1.xxxy, l(0.000000, 0.000000, 0.500000, 0.500000), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_323 = fma((r1).xxxy, vec4<f32>(0.0, 0.0, 0.5, 0.5), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o0 = vec4<f32>(o0.x, o0.y, instruction_323.z, instruction_323.w);
    // mov o1.xyz, r0.xyzx
    let instruction_324 = (r0).xyzx;
    o1 = vec4<f32>(instruction_324.x, instruction_324.y, instruction_324.z, o1.w);
    // mov o1.w, l(0)
    let instruction_325 = vec4<f32>(0.0);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_325.w);
    // mov o3.xy, r3.xyxx
    let instruction_326 = (r3).xyxx;
    o3 = vec4<f32>(instruction_326.x, instruction_326.y, o3.z, o3.w);
    // mov o3.zw, l(0,0,0,0)
    let instruction_327 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(o3.x, o3.y, instruction_327.z, instruction_327.w);
    // mov o4.x, r1.x
    let instruction_328 = (r1).xxxx;
    o4 = vec4<f32>(instruction_328.x, o4.y, o4.z, o4.w);
    // mov o4.zw, l(0,0,1.000000,1.000000)
    let instruction_329 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    o4 = vec4<f32>(o4.x, o4.y, instruction_329.z, instruction_329.w);
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_330 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_330;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_331 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_331;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_332 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_332;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_333 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_333;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_334 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_334;
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_335 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_335;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_336 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_336;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_337 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_337;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_338 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_338;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_339 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_339;
    }
    var result: SourceVaryings11;
    result.position = o4;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.flare_uv = input.uv;
    return result;
}
@fragment
fn source_fragment_11(input: SourceVaryings11) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // ne r0.x, l(0.000000, 0.000000, 0.000000, 0.000000), v1.w
    let instruction_0 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0, 0.0, 0.0, 0.0)) != ((input.v1).wwww)));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t1.xyzw, s2
    let instruction_1 = soft_flare_sprite(input.v3.xy, input.flare_uv);
    r1 = instruction_1;
    // dp4 r0.y, v2.xyzw, r1.xyzw
    let instruction_2 = vec4<f32>(dot(((input.v2).xyzw).xyzw, ((r1).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_2.y, r0.z, r0.w);
    // movc r0.xyzw, r0.xxxx, r1.xyzw, r0.yyyy
    let instruction_3 = select((r0).yyyy, (r1).xyzw, bitcast<vec4<u32>>((r0).xxxx) != vec4<u32>(0u));
    r0 = instruction_3;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_4 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_4;
    // sample r1.xyzw, v0.zwzz, t0.xyzw, s1
    let instruction_5 = textureSample(color_map, color_sampler, ((input.v0).zwzz).xy);
    r1 = instruction_5;
    // mad r1.xyzw, r1.xyzw, v0.yyyy, v0.xxxx
    let instruction_6 = fma((r1).xyzw, (input.v0).yyyy, (input.v0).xxxx);
    r1 = instruction_6;
    // mul_sat r0.xyzw, r0.xyzw, r1.xyzw
    let instruction_7 = clamp(((r0).xyzw) * ((r1).xyzw), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = instruction_7;
    // mul o0.xyzw, r0.xyzw, l(0.200000, 0.200000, 0.200000, 0.200000)
    let instruction_8 = ((r0).xyzw) * (vec4<f32>(0.20000000298023224, 0.20000000298023224, 0.20000000298023224, 0.20000000298023224));
    o0 = instruction_8;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings12 {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) flare_uv: vec2<f32>,
}
@vertex
fn source_vertex_12(input: SourceInput) -> SourceVaryings12 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // mul r0.x, cb3[0].x, l(0.000001)
    let instruction_0 = ((transform.world_rows[0]).xxxx) * (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // mad r0.x, v1.x, l(255.000000), r0.x
    let instruction_1 = fma((input.color).xxxx, vec4<f32>(255.0), (r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // round_ni r0.x, r0.x
    let instruction_2 = floor((r0).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // lt r0.y, r0.x, l(3.000000)
    let instruction_3 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(3.0))));
    r0 = vec4<f32>(r0.x, instruction_3.y, r0.z, r0.w);
    // if_nz r0.y
    if bitcast<vec4<u32>>((r0).yyyy).x != 0u {
    // mov r1.xyz, v0.xyzx
    let instruction_4 = (input.position).xyzx;
    r1 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_5 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 r2.x, r1.xyzw, cb3[0].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(instruction_6.x, r2.y, r2.z, r2.w);
    // dp4 r2.y, r1.xyzw, cb3[1].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, instruction_7.y, r2.z, r2.w);
    // dp4 r2.z, r1.xyzw, cb3[2].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_8.z, r2.w);
    // mov r2.w, l(1.000000)
    let instruction_9 = vec4<f32>(1.0);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_9.w);
    // dp4 r0.y, r2.xyzw, cb0[39].xyzw
    let instruction_10 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // ge r0.z, r0.y, l(0.000000)
    let instruction_11 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).yyyy) >= (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_11.z, r0.w);
    // if_nz r0.z
    if bitcast<vec4<u32>>((r0).zzzz).x != 0u {
    // dp4 r1.x, r2.xyzw, cb0[36].xyzw
    let instruction_12 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_12.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r2.xyzw, cb0[37].xyzw
    let instruction_13 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_13.y, r1.z, r1.w);
    // div r0.zw, r1.xxxy, r0.yyyy
    let instruction_14 = ((r1).xxxy) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_14.z, instruction_14.w);
    // mul r0.zw, r0.zzzw, l(0.000000, 0.000000, 1.000000, -1.000000)
    let instruction_15 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, 1.0, -1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_15.z, instruction_15.w);
    // div r1.z, cb0[95].y, cb0[95].x
    let instruction_16 = ((frame.render_target_size).yyyy) / ((frame.render_target_size).xxxx);
    r1 = vec4<f32>(r1.x, r1.y, instruction_16.z, r1.w);
    // max r0.x, r0.x, l(0.000000)
    let instruction_17 = max((r0).xxxx, vec4<f32>(0.0));
    r0 = vec4<f32>(instruction_17.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(2.000000)
    let instruction_18 = min((r0).xxxx, vec4<f32>(2.0));
    r0 = vec4<f32>(instruction_18.x, r0.y, r0.z, r0.w);
    // lt r0.x, r0.x, l(2.000000)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(2.0))));
    r0 = vec4<f32>(instruction_19.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r1.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.125000, 0.125000, 0.000000, 0.000000)
    let instruction_20 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.125, 0.125, 0.0, 0.0));
    r1 = vec4<f32>(instruction_20.x, instruction_20.y, r1.z, r1.w);
    // mul r0.x, cb2[5].y, l(25.000000)
    let instruction_21 = ((material.source[13]).yyyy) * (vec4<f32>(25.0));
    r0 = vec4<f32>(instruction_21.x, r0.y, r0.z, r0.w);
    // div r2.y, r0.x, r0.y
    let instruction_22 = ((r0).xxxx) / ((r0).yyyy);
    r2 = vec4<f32>(r2.x, instruction_22.y, r2.z, r2.w);
    // mul r2.x, r1.z, r2.y
    let instruction_23 = ((r1).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(instruction_23.x, r2.y, r2.z, r2.w);
    // mov r2.zw, r0.zzzw
    let instruction_24 = (r0).zzzw;
    r2 = vec4<f32>(r2.x, r2.y, instruction_24.z, instruction_24.w);
    // mov o0.xy, l(0.250000,2.500000,0,0)
    let instruction_25 = vec4<f32>(0.25, 2.5, 0.0, 0.0);
    o0 = vec4<f32>(instruction_25.x, instruction_25.y, o0.z, o0.w);
    // mov r0.x, l(1.000000)
    let instruction_26 = vec4<f32>(1.0);
    r0 = vec4<f32>(instruction_26.x, r0.y, r0.z, r0.w);
    // mov r3.xyzw, l(1.500000,1.500000,1.500000,1.000000)
    let instruction_27 = vec4<f32>(1.5, 1.5, 1.5, 1.0);
    r3 = instruction_27;
    } else {
    // mov o0.xy, l(0,0,0,0)
    let instruction_28 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_28.x, instruction_28.y, o0.z, o0.w);
    // mov r1.xy, l(0,0,0,0)
    let instruction_29 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r1 = vec4<f32>(instruction_29.x, instruction_29.y, r1.z, r1.w);
    // mov r2.xyzw, l(0,0,0,0)
    let instruction_30 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r2 = instruction_30;
    // mov r0.x, l(0)
    let instruction_31 = vec4<f32>(0.0);
    r0 = vec4<f32>(instruction_31.x, r0.y, r0.z, r0.w);
    // mov r3.xyzw, l(0,0,0,0)
    let instruction_32 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r3 = instruction_32;
    }
    // eq r3.w, r3.w, l(0.000000)
    let instruction_33 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).wwww) == (vec4<f32>(0.0))));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_33.w);
    // if_nz r3.w
    if bitcast<vec4<u32>>((r3).wwww).x != 0u {
    // mad r1.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000), l(0.250000, 0.000000, 0.000000, 0.000000)
    let instruction_34 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.25, 0.0, 0.0), vec4<f32>(0.25, 0.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_34.x, instruction_34.y, r1.z, r1.w);
    // mul r3.w, cb2[5].y, l(250.000000)
    let instruction_35 = ((material.source[13]).yyyy) * (vec4<f32>(250.0));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_35.w);
    // div r2.y, r3.w, r0.y
    let instruction_36 = ((r3).wwww) / ((r0).yyyy);
    r2 = vec4<f32>(r2.x, instruction_36.y, r2.z, r2.w);
    // mul r2.x, r1.z, r2.y
    let instruction_37 = ((r1).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(instruction_37.x, r2.y, r2.z, r2.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_38 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_38;
    // mov r3.xyz, l(1.000000,1.000000,1.000000,0)
    let instruction_39 = vec4<f32>(1.0, 1.0, 1.0, 0.0);
    r3 = vec4<f32>(instruction_39.x, instruction_39.y, instruction_39.z, r3.w);
    // mov r2.zw, r0.zzzw
    let instruction_40 = (r0).zzzw;
    r2 = vec4<f32>(r2.x, r2.y, instruction_40.z, instruction_40.w);
    // mov o0.xy, l(0,1.000000,0,0)
    let instruction_41 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_41.x, instruction_41.y, o0.z, o0.w);
    // mov r0.x, l(1.000000)
    let instruction_42 = vec4<f32>(1.0);
    r0 = vec4<f32>(instruction_42.x, r0.y, r0.z, r0.w);
    } else {
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_43 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_43;
    }
    // mul r0.yzw, r3.xxyz, cb2[5].xxxx
    let instruction_44 = ((r3).xxyz) * ((material.source[13]).xxxx);
    r0 = vec4<f32>(r0.x, instruction_44.y, instruction_44.z, instruction_44.w);
    // mul r0.yzw, r0.yyzw, v1.wwww
    let instruction_45 = ((r0).yyzw) * ((input.color).wwww);
    r0 = vec4<f32>(r0.x, instruction_45.y, instruction_45.z, instruction_45.w);
    // eq r0.x, r0.x, l(0.000000)
    let instruction_46 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_46.x, r0.y, r0.z, r0.w);
    // dp3 r3.x, r0.ywzy, l(0.333000, 0.333000, 0.334000, 0.000000)
    let instruction_47 = vec4<f32>(dot(((r0).ywzy).xyz, (vec4<f32>(0.3330000042915344, 0.3330000042915344, 0.33399999141693115, 0.0)).xyz));
    r3 = vec4<f32>(instruction_47.x, r3.y, r3.z, r3.w);
    // lt r3.x, r3.x, l(0.003800)
    let instruction_48 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).xxxx) < (vec4<f32>(0.003800000064074993))));
    r3 = vec4<f32>(instruction_48.x, r3.y, r3.z, r3.w);
    // or r0.x, r0.x, r3.x
    let instruction_49 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).xxxx) | bitcast<vec4<u32>>((r3).xxxx));
    r0 = vec4<f32>(instruction_49.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_50 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_50;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_51 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_51;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_52 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_52;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_53 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_53;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_54 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_54;
    } else {
    // mad r3.xy, v2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_55 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r3 = vec4<f32>(instruction_55.x, instruction_55.y, r3.z, r3.w);
    // mov r1.w, l(1.000000)
    let instruction_56 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_56.w);
    // div r3.xy, r3.xyxx, r1.zwzz
    let instruction_57 = ((r3).xyxx) / ((r1).zwzz);
    r3 = vec4<f32>(instruction_57.x, instruction_57.y, r3.z, r3.w);
    // mul r2.xy, r2.xyxx, r3.xyxx
    let instruction_58 = ((r2).xyxx) * ((r3).xyxx);
    r2 = vec4<f32>(instruction_58.x, instruction_58.y, r2.z, r2.w);
    // mad r1.zw, r2.xxxy, r1.zzzw, r2.zzzw
    let instruction_59 = fma((r2).xxxy, (r1).zzzw, (r2).zzzw);
    r1 = vec4<f32>(r1.x, r1.y, instruction_59.z, instruction_59.w);
    // mov o4.y, -r1.w
    let instruction_60 = -((r1).wwww);
    o4 = vec4<f32>(o4.x, instruction_60.y, o4.z, o4.w);
    // mad o0.zw, r1.zzzw, l(0.000000, 0.000000, 0.500000, 0.500000), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_61 = fma((r1).zzzw, vec4<f32>(0.0, 0.0, 0.5, 0.5), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o0 = vec4<f32>(o0.x, o0.y, instruction_61.z, instruction_61.w);
    // mov o1.xyz, r0.yzwy
    let instruction_62 = (r0).yzwy;
    o1 = vec4<f32>(instruction_62.x, instruction_62.y, instruction_62.z, o1.w);
    // mov o1.w, l(0)
    let instruction_63 = vec4<f32>(0.0);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_63.w);
    // mov o3.xy, r1.xyxx
    let instruction_64 = (r1).xyxx;
    o3 = vec4<f32>(instruction_64.x, instruction_64.y, o3.z, o3.w);
    // mov o3.zw, l(0,0,0,0)
    let instruction_65 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(o3.x, o3.y, instruction_65.z, instruction_65.w);
    // mov o4.x, r1.z
    let instruction_66 = (r1).zzzz;
    o4 = vec4<f32>(instruction_66.x, o4.y, o4.z, o4.w);
    // mov o4.zw, l(0,0,1.000000,1.000000)
    let instruction_67 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    o4 = vec4<f32>(o4.x, o4.y, instruction_67.z, instruction_67.w);
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_68 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_68;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_69 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_69;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_70 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_70;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_71 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_71;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_72 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_72;
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_73 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_73;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_74 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_74;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_75 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_75;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_76 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_76;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_77 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_77;
    }
    var result: SourceVaryings12;
    result.position = o4;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.flare_uv = input.uv;
    return result;
}
@fragment
fn source_fragment_12(input: SourceVaryings12) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // ne r0.x, l(0.000000, 0.000000, 0.000000, 0.000000), v1.w
    let instruction_0 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0, 0.0, 0.0, 0.0)) != ((input.v1).wwww)));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t1.xyzw, s2
    let instruction_1 = soft_flare_sprite(input.v3.xy, input.flare_uv);
    r1 = instruction_1;
    // dp4 r0.y, v2.xyzw, r1.xyzw
    let instruction_2 = vec4<f32>(dot(((input.v2).xyzw).xyzw, ((r1).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_2.y, r0.z, r0.w);
    // movc r0.xyzw, r0.xxxx, r1.xyzw, r0.yyyy
    let instruction_3 = select((r0).yyyy, (r1).xyzw, bitcast<vec4<u32>>((r0).xxxx) != vec4<u32>(0u));
    r0 = instruction_3;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_4 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_4;
    // sample r1.xyzw, v0.zwzz, t0.xyzw, s1
    let instruction_5 = textureSample(color_map, color_sampler, ((input.v0).zwzz).xy);
    r1 = instruction_5;
    // mad r1.xyzw, r1.xyzw, v0.yyyy, v0.xxxx
    let instruction_6 = fma((r1).xyzw, (input.v0).yyyy, (input.v0).xxxx);
    r1 = instruction_6;
    // mul_sat r0.xyzw, r0.xyzw, r1.xyzw
    let instruction_7 = clamp(((r0).xyzw) * ((r1).xyzw), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = instruction_7;
    // mul o0.xyzw, r0.xyzw, l(0.200000, 0.200000, 0.200000, 0.200000)
    let instruction_8 = ((r0).xyzw) * (vec4<f32>(0.20000000298023224, 0.20000000298023224, 0.20000000298023224, 0.20000000298023224));
    o0 = instruction_8;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings13 {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) flare_uv: vec2<f32>,
}
@vertex
fn source_vertex_13(input: SourceInput) -> SourceVaryings13 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    // mul r0.x, cb3[0].x, l(0.000001)
    let instruction_0 = ((transform.world_rows[0]).xxxx) * (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // mad r0.x, v1.x, l(255.000000), r0.x
    let instruction_1 = fma((input.color).xxxx, vec4<f32>(255.0), (r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // round_ni r0.x, r0.x
    let instruction_2 = floor((r0).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // lt r0.x, r0.x, l(1.000000)
    let instruction_3 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(1.0))));
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mov r0.xyz, v0.xyzx
    let instruction_4 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_5 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_5.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_6 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_6.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_7 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_7.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_8 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_8.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_9 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_9.w);
    // dp4 r0.x, r1.xyzw, cb0[39].xyzw
    let instruction_10 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_10.x, r0.y, r0.z, r0.w);
    // ge r0.y, r0.x, l(0.000000)
    let instruction_11 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) >= (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, instruction_11.y, r0.z, r0.w);
    // if_nz r0.y
    if bitcast<vec4<u32>>((r0).yyyy).x != 0u {
    // mov r2.xz, l(1.000000,0,1.000000,0)
    let instruction_12 = vec4<f32>(1.0, 0.0, 1.0, 0.0);
    r2 = vec4<f32>(instruction_12.x, r2.y, instruction_12.z, r2.w);
    // mov r2.y, v1.w
    let instruction_13 = (input.color).wwww;
    r2 = vec4<f32>(r2.x, instruction_13.y, r2.z, r2.w);
    // mul r0.yzw, r2.xxyz, cb2[5].xxxx
    let instruction_14 = ((r2).xxyz) * ((material.source[13]).xxxx);
    r0 = vec4<f32>(r0.x, instruction_14.y, instruction_14.z, instruction_14.w);
    // mov r2.xz, v1.wwww
    let instruction_15 = (input.color).wwww;
    r2 = vec4<f32>(instruction_15.x, r2.y, instruction_15.z, r2.w);
    // mov r2.y, l(0.666000)
    let instruction_16 = vec4<f32>(0.6660000085830688);
    r2 = vec4<f32>(r2.x, instruction_16.y, r2.z, r2.w);
    // mul r0.yzw, r0.yyzw, r2.xxyz
    let instruction_17 = ((r0).yyzw) * ((r2).xxyz);
    r0 = vec4<f32>(r0.x, instruction_17.y, instruction_17.z, instruction_17.w);
    // dp3 r2.x, r0.wwzw, l(0.333000, 0.333000, 0.334000, 0.000000)
    let instruction_18 = vec4<f32>(dot(((r0).wwzw).xyz, (vec4<f32>(0.3330000042915344, 0.3330000042915344, 0.33399999141693115, 0.0)).xyz));
    r2 = vec4<f32>(instruction_18.x, r2.y, r2.z, r2.w);
    // lt r2.x, r2.x, l(0.003800)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r2).xxxx) < (vec4<f32>(0.003800000064074993))));
    r2 = vec4<f32>(instruction_19.x, r2.y, r2.z, r2.w);
    // if_nz r2.x
    if bitcast<vec4<u32>>((r2).xxxx).x != 0u {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_20 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_20;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_21 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_21;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_22 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_22;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_23 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_23;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_24 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_24;
    } else {
    // dp4 r2.z, r1.xyzw, cb0[36].xyzw
    let instruction_25 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_25.z, r2.w);
    // dp4 r2.w, r1.xyzw, cb0[37].xyzw
    let instruction_26 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_26.w);
    // div r1.xy, r2.zwzz, r0.xxxx
    let instruction_27 = ((r2).zwzz) / ((r0).xxxx);
    r1 = vec4<f32>(instruction_27.x, instruction_27.y, r1.z, r1.w);
    // mul r1.xy, r1.xyxx, l(1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_28 = ((r1).xyxx) * (vec4<f32>(1.0, -1.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_28.x, instruction_28.y, r1.z, r1.w);
    // div r1.z, cb0[95].y, cb0[95].x
    let instruction_29 = ((frame.render_target_size).yyyy) / ((frame.render_target_size).xxxx);
    r1 = vec4<f32>(r1.x, r1.y, instruction_29.z, r1.w);
    // mul r2.x, cb2[5].y, l(75.000000)
    let instruction_30 = ((material.source[13]).yyyy) * (vec4<f32>(75.0));
    r2 = vec4<f32>(instruction_30.x, r2.y, r2.z, r2.w);
    // div r2.w, r2.x, r0.x
    let instruction_31 = ((r2).xxxx) / ((r0).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_31.w);
    // mul r2.z, r1.z, r2.w
    let instruction_32 = ((r1).zzzz) * ((r2).wwww);
    r2 = vec4<f32>(r2.x, r2.y, instruction_32.z, r2.w);
    // mad r2.xy, v2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_33 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r2 = vec4<f32>(instruction_33.x, instruction_33.y, r2.z, r2.w);
    // mov r1.w, l(1.000000)
    let instruction_34 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_34.w);
    // div r2.xy, r2.xyxx, r1.zwzz
    let instruction_35 = ((r2).xyxx) / ((r1).zwzz);
    r2 = vec4<f32>(instruction_35.x, instruction_35.y, r2.z, r2.w);
    // mul r2.xy, r2.zwzz, r2.xyxx
    let instruction_36 = ((r2).zwzz) * ((r2).xyxx);
    r2 = vec4<f32>(instruction_36.x, instruction_36.y, r2.z, r2.w);
    // mad r1.xy, r2.xyxx, r1.zwzz, r1.xyxx
    let instruction_37 = fma((r2).xyxx, (r1).zwzz, (r1).xyxx);
    r1 = vec4<f32>(instruction_37.x, instruction_37.y, r1.z, r1.w);
    // mov o4.y, -r1.y
    let instruction_38 = -((r1).yyyy);
    o4 = vec4<f32>(o4.x, instruction_38.y, o4.z, o4.w);
    // mad o0.zw, r1.xxxy, l(0.000000, 0.000000, 0.500000, 0.500000), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_39 = fma((r1).xxxy, vec4<f32>(0.0, 0.0, 0.5, 0.5), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o0 = vec4<f32>(o0.x, o0.y, instruction_39.z, instruction_39.w);
    // mov o0.xy, l(0,1.000000,0,0)
    let instruction_40 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_40.x, instruction_40.y, o0.z, o0.w);
    // mov o1.xyz, r0.yzwy
    let instruction_41 = (r0).yzwy;
    o1 = vec4<f32>(instruction_41.x, instruction_41.y, instruction_41.z, o1.w);
    // mov o1.w, l(0)
    let instruction_42 = vec4<f32>(0.0);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_42.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_43 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_43;
    // mad o3.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000), l(0.250000, 0.000000, 0.000000, 0.000000)
    let instruction_44 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.25, 0.0, 0.0), vec4<f32>(0.25, 0.0, 0.0, 0.0));
    o3 = vec4<f32>(instruction_44.x, instruction_44.y, o3.z, o3.w);
    // mov o3.zw, l(0,0,0,0)
    let instruction_45 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(o3.x, o3.y, instruction_45.z, instruction_45.w);
    // mov o4.x, r1.x
    let instruction_46 = (r1).xxxx;
    o4 = vec4<f32>(instruction_46.x, o4.y, o4.z, o4.w);
    // mov o4.zw, l(0,0,1.000000,1.000000)
    let instruction_47 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    o4 = vec4<f32>(o4.x, o4.y, instruction_47.z, instruction_47.w);
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_48 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_48;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_49 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_49;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_50 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_50;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_51 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_51;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_52 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_52;
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_53 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_53;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_54 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_54;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_55 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_55;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_56 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_56;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_57 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_57;
    }
    var result: SourceVaryings13;
    result.position = o4;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.flare_uv = input.uv;
    return result;
}
@fragment
fn source_fragment_13(input: SourceVaryings13) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // ne r0.x, l(0.000000, 0.000000, 0.000000, 0.000000), v1.w
    let instruction_0 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0, 0.0, 0.0, 0.0)) != ((input.v1).wwww)));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t1.xyzw, s2
    let instruction_1 = soft_flare_sprite(input.v3.xy, input.flare_uv);
    r1 = instruction_1;
    // dp4 r0.y, v2.xyzw, r1.xyzw
    let instruction_2 = vec4<f32>(dot(((input.v2).xyzw).xyzw, ((r1).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_2.y, r0.z, r0.w);
    // movc r0.xyzw, r0.xxxx, r1.xyzw, r0.yyyy
    let instruction_3 = select((r0).yyyy, (r1).xyzw, bitcast<vec4<u32>>((r0).xxxx) != vec4<u32>(0u));
    r0 = instruction_3;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_4 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_4;
    // sample r1.xyzw, v0.zwzz, t0.xyzw, s1
    let instruction_5 = textureSample(color_map, color_sampler, ((input.v0).zwzz).xy);
    r1 = instruction_5;
    // mad r1.xyzw, r1.xyzw, v0.yyyy, v0.xxxx
    let instruction_6 = fma((r1).xyzw, (input.v0).yyyy, (input.v0).xxxx);
    r1 = instruction_6;
    // mul_sat r0.xyzw, r0.xyzw, r1.xyzw
    let instruction_7 = clamp(((r0).xyzw) * ((r1).xyzw), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = instruction_7;
    // mul o0.xyzw, r0.xyzw, l(0.200000, 0.200000, 0.200000, 0.200000)
    let instruction_8 = ((r0).xyzw) * (vec4<f32>(0.20000000298023224, 0.20000000298023224, 0.20000000298023224, 0.20000000298023224));
    o0 = instruction_8;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings14 {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) flare_uv: vec2<f32>,
}
@vertex
fn source_vertex_14(input: SourceInput) -> SourceVaryings14 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // mul r0.x, cb3[0].x, l(0.000001)
    let instruction_0 = ((transform.world_rows[0]).xxxx) * (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // mad r0.x, v1.x, l(255.000000), r0.x
    let instruction_1 = fma((input.color).xxxx, vec4<f32>(255.0), (r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // round_ni r0.x, r0.x
    let instruction_2 = floor((r0).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // lt r0.y, r0.x, l(3.000000)
    let instruction_3 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(3.0))));
    r0 = vec4<f32>(r0.x, instruction_3.y, r0.z, r0.w);
    // if_nz r0.y
    if bitcast<vec4<u32>>((r0).yyyy).x != 0u {
    // mov r1.xyz, v0.xyzx
    let instruction_4 = (input.position).xyzx;
    r1 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_5 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 r2.x, r1.xyzw, cb3[0].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(instruction_6.x, r2.y, r2.z, r2.w);
    // dp4 r2.y, r1.xyzw, cb3[1].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, instruction_7.y, r2.z, r2.w);
    // dp4 r2.z, r1.xyzw, cb3[2].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_8.z, r2.w);
    // mov r2.w, l(1.000000)
    let instruction_9 = vec4<f32>(1.0);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_9.w);
    // dp4 r0.y, r2.xyzw, cb0[39].xyzw
    let instruction_10 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // ge r0.z, r0.y, l(0.000000)
    let instruction_11 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).yyyy) >= (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_11.z, r0.w);
    // if_nz r0.z
    if bitcast<vec4<u32>>((r0).zzzz).x != 0u {
    // dp4 r1.x, r2.xyzw, cb0[36].xyzw
    let instruction_12 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_12.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r2.xyzw, cb0[37].xyzw
    let instruction_13 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_13.y, r1.z, r1.w);
    // div r0.zw, r1.xxxy, r0.yyyy
    let instruction_14 = ((r1).xxxy) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_14.z, instruction_14.w);
    // mul r0.zw, r0.zzzw, l(0.000000, 0.000000, 1.000000, -1.000000)
    let instruction_15 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, 1.0, -1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_15.z, instruction_15.w);
    // div r1.z, cb0[95].y, cb0[95].x
    let instruction_16 = ((frame.render_target_size).yyyy) / ((frame.render_target_size).xxxx);
    r1 = vec4<f32>(r1.x, r1.y, instruction_16.z, r1.w);
    // max r0.x, r0.x, l(0.000000)
    let instruction_17 = max((r0).xxxx, vec4<f32>(0.0));
    r0 = vec4<f32>(instruction_17.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(2.000000)
    let instruction_18 = min((r0).xxxx, vec4<f32>(2.0));
    r0 = vec4<f32>(instruction_18.x, r0.y, r0.z, r0.w);
    // lt r0.x, r0.x, l(2.000000)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(2.0))));
    r0 = vec4<f32>(instruction_19.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r1.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.125000, 0.125000, 0.000000, 0.000000)
    let instruction_20 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.125, 0.125, 0.0, 0.0));
    r1 = vec4<f32>(instruction_20.x, instruction_20.y, r1.z, r1.w);
    // mul r0.x, cb2[5].y, l(25.000000)
    let instruction_21 = ((material.source[13]).yyyy) * (vec4<f32>(25.0));
    r0 = vec4<f32>(instruction_21.x, r0.y, r0.z, r0.w);
    // div r2.y, r0.x, r0.y
    let instruction_22 = ((r0).xxxx) / ((r0).yyyy);
    r2 = vec4<f32>(r2.x, instruction_22.y, r2.z, r2.w);
    // mul r2.x, r1.z, r2.y
    let instruction_23 = ((r1).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(instruction_23.x, r2.y, r2.z, r2.w);
    // mov r2.zw, r0.zzzw
    let instruction_24 = (r0).zzzw;
    r2 = vec4<f32>(r2.x, r2.y, instruction_24.z, instruction_24.w);
    // mov o0.xy, l(0.250000,2.500000,0,0)
    let instruction_25 = vec4<f32>(0.25, 2.5, 0.0, 0.0);
    o0 = vec4<f32>(instruction_25.x, instruction_25.y, o0.z, o0.w);
    // mov r0.x, l(1.000000)
    let instruction_26 = vec4<f32>(1.0);
    r0 = vec4<f32>(instruction_26.x, r0.y, r0.z, r0.w);
    // mov r3.xyzw, l(1.500000,1.000000,1.500000,1.000000)
    let instruction_27 = vec4<f32>(1.5, 1.0, 1.5, 1.0);
    r3 = instruction_27;
    } else {
    // mov o0.xy, l(0,0,0,0)
    let instruction_28 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_28.x, instruction_28.y, o0.z, o0.w);
    // mov r1.xy, l(0,0,0,0)
    let instruction_29 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r1 = vec4<f32>(instruction_29.x, instruction_29.y, r1.z, r1.w);
    // mov r2.xyzw, l(0,0,0,0)
    let instruction_30 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r2 = instruction_30;
    // mov r0.x, l(0)
    let instruction_31 = vec4<f32>(0.0);
    r0 = vec4<f32>(instruction_31.x, r0.y, r0.z, r0.w);
    // mov r3.xyzw, l(0,0,0,0)
    let instruction_32 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r3 = instruction_32;
    }
    // eq r3.w, r3.w, l(0.000000)
    let instruction_33 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).wwww) == (vec4<f32>(0.0))));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_33.w);
    // if_nz r3.w
    if bitcast<vec4<u32>>((r3).wwww).x != 0u {
    // mad r1.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000), l(0.250000, 0.000000, 0.000000, 0.000000)
    let instruction_34 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.25, 0.0, 0.0), vec4<f32>(0.25, 0.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_34.x, instruction_34.y, r1.z, r1.w);
    // mul r3.w, cb2[5].y, l(250.000000)
    let instruction_35 = ((material.source[13]).yyyy) * (vec4<f32>(250.0));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_35.w);
    // div r2.y, r3.w, r0.y
    let instruction_36 = ((r3).wwww) / ((r0).yyyy);
    r2 = vec4<f32>(r2.x, instruction_36.y, r2.z, r2.w);
    // mul r2.x, r1.z, r2.y
    let instruction_37 = ((r1).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(instruction_37.x, r2.y, r2.z, r2.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_38 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_38;
    // mov r3.xyz, l(1.000000,0.666000,1.000000,0)
    let instruction_39 = vec4<f32>(1.0, 0.6660000085830688, 1.0, 0.0);
    r3 = vec4<f32>(instruction_39.x, instruction_39.y, instruction_39.z, r3.w);
    // mov r2.zw, r0.zzzw
    let instruction_40 = (r0).zzzw;
    r2 = vec4<f32>(r2.x, r2.y, instruction_40.z, instruction_40.w);
    // mov o0.xy, l(0,1.000000,0,0)
    let instruction_41 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_41.x, instruction_41.y, o0.z, o0.w);
    // mov r0.x, l(1.000000)
    let instruction_42 = vec4<f32>(1.0);
    r0 = vec4<f32>(instruction_42.x, r0.y, r0.z, r0.w);
    } else {
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_43 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_43;
    }
    // mul r0.yzw, r3.xxyz, cb2[5].xxxx
    let instruction_44 = ((r3).xxyz) * ((material.source[13]).xxxx);
    r0 = vec4<f32>(r0.x, instruction_44.y, instruction_44.z, instruction_44.w);
    // mul r0.yzw, r0.yyzw, v1.wwww
    let instruction_45 = ((r0).yyzw) * ((input.color).wwww);
    r0 = vec4<f32>(r0.x, instruction_45.y, instruction_45.z, instruction_45.w);
    // eq r0.x, r0.x, l(0.000000)
    let instruction_46 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_46.x, r0.y, r0.z, r0.w);
    // dp3 r3.x, r0.ywzy, l(0.333000, 0.333000, 0.334000, 0.000000)
    let instruction_47 = vec4<f32>(dot(((r0).ywzy).xyz, (vec4<f32>(0.3330000042915344, 0.3330000042915344, 0.33399999141693115, 0.0)).xyz));
    r3 = vec4<f32>(instruction_47.x, r3.y, r3.z, r3.w);
    // lt r3.x, r3.x, l(0.003800)
    let instruction_48 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).xxxx) < (vec4<f32>(0.003800000064074993))));
    r3 = vec4<f32>(instruction_48.x, r3.y, r3.z, r3.w);
    // or r0.x, r0.x, r3.x
    let instruction_49 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).xxxx) | bitcast<vec4<u32>>((r3).xxxx));
    r0 = vec4<f32>(instruction_49.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_50 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_50;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_51 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_51;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_52 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_52;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_53 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_53;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_54 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_54;
    } else {
    // mad r3.xy, v2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_55 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r3 = vec4<f32>(instruction_55.x, instruction_55.y, r3.z, r3.w);
    // mov r1.w, l(1.000000)
    let instruction_56 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_56.w);
    // div r3.xy, r3.xyxx, r1.zwzz
    let instruction_57 = ((r3).xyxx) / ((r1).zwzz);
    r3 = vec4<f32>(instruction_57.x, instruction_57.y, r3.z, r3.w);
    // mul r2.xy, r2.xyxx, r3.xyxx
    let instruction_58 = ((r2).xyxx) * ((r3).xyxx);
    r2 = vec4<f32>(instruction_58.x, instruction_58.y, r2.z, r2.w);
    // mad r1.zw, r2.xxxy, r1.zzzw, r2.zzzw
    let instruction_59 = fma((r2).xxxy, (r1).zzzw, (r2).zzzw);
    r1 = vec4<f32>(r1.x, r1.y, instruction_59.z, instruction_59.w);
    // mov o4.y, -r1.w
    let instruction_60 = -((r1).wwww);
    o4 = vec4<f32>(o4.x, instruction_60.y, o4.z, o4.w);
    // mad o0.zw, r1.zzzw, l(0.000000, 0.000000, 0.500000, 0.500000), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_61 = fma((r1).zzzw, vec4<f32>(0.0, 0.0, 0.5, 0.5), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o0 = vec4<f32>(o0.x, o0.y, instruction_61.z, instruction_61.w);
    // mov o1.xyz, r0.yzwy
    let instruction_62 = (r0).yzwy;
    o1 = vec4<f32>(instruction_62.x, instruction_62.y, instruction_62.z, o1.w);
    // mov o1.w, l(0)
    let instruction_63 = vec4<f32>(0.0);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_63.w);
    // mov o3.xy, r1.xyxx
    let instruction_64 = (r1).xyxx;
    o3 = vec4<f32>(instruction_64.x, instruction_64.y, o3.z, o3.w);
    // mov o3.zw, l(0,0,0,0)
    let instruction_65 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(o3.x, o3.y, instruction_65.z, instruction_65.w);
    // mov o4.x, r1.z
    let instruction_66 = (r1).zzzz;
    o4 = vec4<f32>(instruction_66.x, o4.y, o4.z, o4.w);
    // mov o4.zw, l(0,0,1.000000,1.000000)
    let instruction_67 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    o4 = vec4<f32>(o4.x, o4.y, instruction_67.z, instruction_67.w);
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_68 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_68;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_69 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_69;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_70 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_70;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_71 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_71;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_72 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_72;
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_73 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_73;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_74 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_74;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_75 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_75;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_76 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_76;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_77 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_77;
    }
    var result: SourceVaryings14;
    result.position = o4;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.flare_uv = input.uv;
    return result;
}
@fragment
fn source_fragment_14(input: SourceVaryings14) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // ne r0.x, l(0.000000, 0.000000, 0.000000, 0.000000), v1.w
    let instruction_0 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0, 0.0, 0.0, 0.0)) != ((input.v1).wwww)));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t1.xyzw, s2
    let instruction_1 = soft_flare_sprite(input.v3.xy, input.flare_uv);
    r1 = instruction_1;
    // dp4 r0.y, v2.xyzw, r1.xyzw
    let instruction_2 = vec4<f32>(dot(((input.v2).xyzw).xyzw, ((r1).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_2.y, r0.z, r0.w);
    // movc r0.xyzw, r0.xxxx, r1.xyzw, r0.yyyy
    let instruction_3 = select((r0).yyyy, (r1).xyzw, bitcast<vec4<u32>>((r0).xxxx) != vec4<u32>(0u));
    r0 = instruction_3;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_4 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_4;
    // sample r1.xyzw, v0.zwzz, t0.xyzw, s1
    let instruction_5 = textureSample(color_map, color_sampler, ((input.v0).zwzz).xy);
    r1 = instruction_5;
    // mad r1.xyzw, r1.xyzw, v0.yyyy, v0.xxxx
    let instruction_6 = fma((r1).xyzw, (input.v0).yyyy, (input.v0).xxxx);
    r1 = instruction_6;
    // mul_sat r0.xyzw, r0.xyzw, r1.xyzw
    let instruction_7 = clamp(((r0).xyzw) * ((r1).xyzw), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = instruction_7;
    // mul o0.xyzw, r0.xyzw, l(0.200000, 0.200000, 0.200000, 0.200000)
    let instruction_8 = ((r0).xyzw) * (vec4<f32>(0.20000000298023224, 0.20000000298023224, 0.20000000298023224, 0.20000000298023224));
    o0 = instruction_8;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings15 {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) flare_uv: vec2<f32>,
}
@vertex
fn source_vertex_15(input: SourceInput) -> SourceVaryings15 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    // mul r0.x, cb3[0].x, l(0.000001)
    let instruction_0 = ((transform.world_rows[0]).xxxx) * (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // mad r0.x, v1.x, l(255.000000), r0.x
    let instruction_1 = fma((input.color).xxxx, vec4<f32>(255.0), (r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // round_ni r0.x, r0.x
    let instruction_2 = floor((r0).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // lt r0.x, r0.x, l(1.000000)
    let instruction_3 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(1.0))));
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mov r0.xyz, v0.xyzx
    let instruction_4 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_5 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_5.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_6 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_6.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_7 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_7.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_8 = vec4<f32>(dot(((r0).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_8.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_9 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_9.w);
    // dp4 r0.x, r1.xyzw, cb0[39].xyzw
    let instruction_10 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_10.x, r0.y, r0.z, r0.w);
    // ge r0.y, r0.x, l(0.000000)
    let instruction_11 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) >= (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, instruction_11.y, r0.z, r0.w);
    // if_nz r0.y
    if bitcast<vec4<u32>>((r0).yyyy).x != 0u {
    // mov r2.xy, v1.wwww
    let instruction_12 = (input.color).wwww;
    r2 = vec4<f32>(instruction_12.x, instruction_12.y, r2.z, r2.w);
    // mov r2.z, l(1.000000)
    let instruction_13 = vec4<f32>(1.0);
    r2 = vec4<f32>(r2.x, r2.y, instruction_13.z, r2.w);
    // mul r0.yzw, r2.xxyz, cb2[5].xxxx
    let instruction_14 = ((r2).xxyz) * ((material.source[13]).xxxx);
    r0 = vec4<f32>(r0.x, instruction_14.y, instruction_14.z, instruction_14.w);
    // mov r2.xy, l(0.750000,0.500000,0,0)
    let instruction_15 = vec4<f32>(0.75, 0.5, 0.0, 0.0);
    r2 = vec4<f32>(instruction_15.x, instruction_15.y, r2.z, r2.w);
    // mov r2.z, v1.w
    let instruction_16 = (input.color).wwww;
    r2 = vec4<f32>(r2.x, r2.y, instruction_16.z, r2.w);
    // mul r0.yzw, r0.yyzw, r2.xxyz
    let instruction_17 = ((r0).yyzw) * ((r2).xxyz);
    r0 = vec4<f32>(r0.x, instruction_17.y, instruction_17.z, instruction_17.w);
    // dp3 r2.x, r0.ywzy, l(0.333000, 0.333000, 0.334000, 0.000000)
    let instruction_18 = vec4<f32>(dot(((r0).ywzy).xyz, (vec4<f32>(0.3330000042915344, 0.3330000042915344, 0.33399999141693115, 0.0)).xyz));
    r2 = vec4<f32>(instruction_18.x, r2.y, r2.z, r2.w);
    // lt r2.x, r2.x, l(0.003800)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r2).xxxx) < (vec4<f32>(0.003800000064074993))));
    r2 = vec4<f32>(instruction_19.x, r2.y, r2.z, r2.w);
    // if_nz r2.x
    if bitcast<vec4<u32>>((r2).xxxx).x != 0u {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_20 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_20;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_21 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_21;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_22 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_22;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_23 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_23;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_24 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_24;
    } else {
    // dp4 r2.z, r1.xyzw, cb0[36].xyzw
    let instruction_25 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_25.z, r2.w);
    // dp4 r2.w, r1.xyzw, cb0[37].xyzw
    let instruction_26 = vec4<f32>(dot(((r1).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_26.w);
    // div r1.xy, r2.zwzz, r0.xxxx
    let instruction_27 = ((r2).zwzz) / ((r0).xxxx);
    r1 = vec4<f32>(instruction_27.x, instruction_27.y, r1.z, r1.w);
    // mul r1.xy, r1.xyxx, l(1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_28 = ((r1).xyxx) * (vec4<f32>(1.0, -1.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_28.x, instruction_28.y, r1.z, r1.w);
    // div r1.z, cb0[95].y, cb0[95].x
    let instruction_29 = ((frame.render_target_size).yyyy) / ((frame.render_target_size).xxxx);
    r1 = vec4<f32>(r1.x, r1.y, instruction_29.z, r1.w);
    // mul r2.x, cb2[5].y, l(500.000000)
    let instruction_30 = ((material.source[13]).yyyy) * (vec4<f32>(500.0));
    r2 = vec4<f32>(instruction_30.x, r2.y, r2.z, r2.w);
    // div r2.w, r2.x, r0.x
    let instruction_31 = ((r2).xxxx) / ((r0).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_31.w);
    // mul r2.z, r1.z, r2.w
    let instruction_32 = ((r1).zzzz) * ((r2).wwww);
    r2 = vec4<f32>(r2.x, r2.y, instruction_32.z, r2.w);
    // mad r2.xy, v2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_33 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r2 = vec4<f32>(instruction_33.x, instruction_33.y, r2.z, r2.w);
    // mov r1.w, l(1.000000)
    let instruction_34 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_34.w);
    // div r2.xy, r2.xyxx, r1.zwzz
    let instruction_35 = ((r2).xyxx) / ((r1).zwzz);
    r2 = vec4<f32>(instruction_35.x, instruction_35.y, r2.z, r2.w);
    // mul r2.xy, r2.zwzz, r2.xyxx
    let instruction_36 = ((r2).zwzz) * ((r2).xyxx);
    r2 = vec4<f32>(instruction_36.x, instruction_36.y, r2.z, r2.w);
    // mad r1.xy, r2.xyxx, r1.zwzz, r1.xyxx
    let instruction_37 = fma((r2).xyxx, (r1).zwzz, (r1).xyxx);
    r1 = vec4<f32>(instruction_37.x, instruction_37.y, r1.z, r1.w);
    // mov o4.y, -r1.y
    let instruction_38 = -((r1).yyyy);
    o4 = vec4<f32>(o4.x, instruction_38.y, o4.z, o4.w);
    // mad o0.zw, r1.xxxy, l(0.000000, 0.000000, 0.500000, 0.500000), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_39 = fma((r1).xxxy, vec4<f32>(0.0, 0.0, 0.5, 0.5), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o0 = vec4<f32>(o0.x, o0.y, instruction_39.z, instruction_39.w);
    // mov o0.xy, l(0,1.250000,0,0)
    let instruction_40 = vec4<f32>(0.0, 1.25, 0.0, 0.0);
    o0 = vec4<f32>(instruction_40.x, instruction_40.y, o0.z, o0.w);
    // mov o1.xyz, r0.yzwy
    let instruction_41 = (r0).yzwy;
    o1 = vec4<f32>(instruction_41.x, instruction_41.y, instruction_41.z, o1.w);
    // mov o1.w, l(0)
    let instruction_42 = vec4<f32>(0.0);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_42.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_43 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_43;
    // mad o3.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000), l(0.250000, 0.000000, 0.000000, 0.000000)
    let instruction_44 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.25, 0.0, 0.0), vec4<f32>(0.25, 0.0, 0.0, 0.0));
    o3 = vec4<f32>(instruction_44.x, instruction_44.y, o3.z, o3.w);
    // mov o3.zw, l(0,0,0,0)
    let instruction_45 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(o3.x, o3.y, instruction_45.z, instruction_45.w);
    // mov o4.x, r1.x
    let instruction_46 = (r1).xxxx;
    o4 = vec4<f32>(instruction_46.x, o4.y, o4.z, o4.w);
    // mov o4.zw, l(0,0,1.000000,1.000000)
    let instruction_47 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    o4 = vec4<f32>(o4.x, o4.y, instruction_47.z, instruction_47.w);
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_48 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_48;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_49 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_49;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_50 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_50;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_51 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_51;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_52 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_52;
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_53 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_53;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_54 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_54;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_55 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_55;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_56 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_56;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_57 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_57;
    }
    var result: SourceVaryings15;
    result.position = o4;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.flare_uv = input.uv;
    return result;
}
@fragment
fn source_fragment_15(input: SourceVaryings15) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // ne r0.x, l(0.000000, 0.000000, 0.000000, 0.000000), v1.w
    let instruction_0 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0, 0.0, 0.0, 0.0)) != ((input.v1).wwww)));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t1.xyzw, s2
    let instruction_1 = soft_flare_sprite(input.v3.xy, input.flare_uv);
    r1 = instruction_1;
    // dp4 r0.y, v2.xyzw, r1.xyzw
    let instruction_2 = vec4<f32>(dot(((input.v2).xyzw).xyzw, ((r1).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_2.y, r0.z, r0.w);
    // movc r0.xyzw, r0.xxxx, r1.xyzw, r0.yyyy
    let instruction_3 = select((r0).yyyy, (r1).xyzw, bitcast<vec4<u32>>((r0).xxxx) != vec4<u32>(0u));
    r0 = instruction_3;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_4 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_4;
    // sample r1.xyzw, v0.zwzz, t0.xyzw, s1
    let instruction_5 = textureSample(color_map, color_sampler, ((input.v0).zwzz).xy);
    r1 = instruction_5;
    // mad r1.xyzw, r1.xyzw, v0.yyyy, v0.xxxx
    let instruction_6 = fma((r1).xyzw, (input.v0).yyyy, (input.v0).xxxx);
    r1 = instruction_6;
    // mul_sat r0.xyzw, r0.xyzw, r1.xyzw
    let instruction_7 = clamp(((r0).xyzw) * ((r1).xyzw), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = instruction_7;
    // mul o0.xyzw, r0.xyzw, l(0.200000, 0.200000, 0.200000, 0.200000)
    let instruction_8 = ((r0).xyzw) * (vec4<f32>(0.20000000298023224, 0.20000000298023224, 0.20000000298023224, 0.20000000298023224));
    o0 = instruction_8;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings16 {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) flare_uv: vec2<f32>,
}
@vertex
fn source_vertex_16(input: SourceInput) -> SourceVaryings16 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // mul r0.x, cb3[0].x, l(0.000001)
    let instruction_0 = ((transform.world_rows[0]).xxxx) * (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // mad r0.x, v1.x, l(255.000000), r0.x
    let instruction_1 = fma((input.color).xxxx, vec4<f32>(255.0), (r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // round_ni r0.x, r0.x
    let instruction_2 = floor((r0).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // lt r0.y, r0.x, l(3.000000)
    let instruction_3 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(3.0))));
    r0 = vec4<f32>(r0.x, instruction_3.y, r0.z, r0.w);
    // if_nz r0.y
    if bitcast<vec4<u32>>((r0).yyyy).x != 0u {
    // mov r1.xyz, v0.xyzx
    let instruction_4 = (input.position).xyzx;
    r1 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_5 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 r2.x, r1.xyzw, cb3[0].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(instruction_6.x, r2.y, r2.z, r2.w);
    // dp4 r2.y, r1.xyzw, cb3[1].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, instruction_7.y, r2.z, r2.w);
    // dp4 r2.z, r1.xyzw, cb3[2].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_8.z, r2.w);
    // mov r2.w, l(1.000000)
    let instruction_9 = vec4<f32>(1.0);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_9.w);
    // dp4 r0.y, r2.xyzw, cb0[39].xyzw
    let instruction_10 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // ge r0.z, r0.y, l(0.000000)
    let instruction_11 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).yyyy) >= (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_11.z, r0.w);
    // if_nz r0.z
    if bitcast<vec4<u32>>((r0).zzzz).x != 0u {
    // dp4 r1.x, r2.xyzw, cb0[36].xyzw
    let instruction_12 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_12.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r2.xyzw, cb0[37].xyzw
    let instruction_13 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_13.y, r1.z, r1.w);
    // div r0.zw, r1.xxxy, r0.yyyy
    let instruction_14 = ((r1).xxxy) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_14.z, instruction_14.w);
    // mul r0.zw, r0.zzzw, l(0.000000, 0.000000, 1.000000, -1.000000)
    let instruction_15 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, 1.0, -1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_15.z, instruction_15.w);
    // div r1.z, cb0[95].y, cb0[95].x
    let instruction_16 = ((frame.render_target_size).yyyy) / ((frame.render_target_size).xxxx);
    r1 = vec4<f32>(r1.x, r1.y, instruction_16.z, r1.w);
    // max r0.x, r0.x, l(0.000000)
    let instruction_17 = max((r0).xxxx, vec4<f32>(0.0));
    r0 = vec4<f32>(instruction_17.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(2.000000)
    let instruction_18 = min((r0).xxxx, vec4<f32>(2.0));
    r0 = vec4<f32>(instruction_18.x, r0.y, r0.z, r0.w);
    // lt r0.x, r0.x, l(2.000000)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(2.0))));
    r0 = vec4<f32>(instruction_19.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r1.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.125000, 0.125000, 0.000000, 0.000000)
    let instruction_20 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.125, 0.125, 0.0, 0.0));
    r1 = vec4<f32>(instruction_20.x, instruction_20.y, r1.z, r1.w);
    // mul r0.x, cb2[5].y, l(25.000000)
    let instruction_21 = ((material.source[13]).yyyy) * (vec4<f32>(25.0));
    r0 = vec4<f32>(instruction_21.x, r0.y, r0.z, r0.w);
    // div r2.y, r0.x, r0.y
    let instruction_22 = ((r0).xxxx) / ((r0).yyyy);
    r2 = vec4<f32>(r2.x, instruction_22.y, r2.z, r2.w);
    // mul r2.x, r1.z, r2.y
    let instruction_23 = ((r1).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(instruction_23.x, r2.y, r2.z, r2.w);
    // mov r2.zw, r0.zzzw
    let instruction_24 = (r0).zzzw;
    r2 = vec4<f32>(r2.x, r2.y, instruction_24.z, instruction_24.w);
    // mov o0.xy, l(0,2.500000,0,0)
    let instruction_25 = vec4<f32>(0.0, 2.5, 0.0, 0.0);
    o0 = vec4<f32>(instruction_25.x, instruction_25.y, o0.z, o0.w);
    // mov r0.x, l(1.000000)
    let instruction_26 = vec4<f32>(1.0);
    r0 = vec4<f32>(instruction_26.x, r0.y, r0.z, r0.w);
    // mov r3.xyzw, l(1.666000,1.333000,1.000000,1.000000)
    let instruction_27 = vec4<f32>(1.6660000085830688, 1.3329999446868896, 1.0, 1.0);
    r3 = instruction_27;
    } else {
    // mov o0.xy, l(0,0,0,0)
    let instruction_28 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_28.x, instruction_28.y, o0.z, o0.w);
    // mov r1.xy, l(0,0,0,0)
    let instruction_29 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r1 = vec4<f32>(instruction_29.x, instruction_29.y, r1.z, r1.w);
    // mov r2.xyzw, l(0,0,0,0)
    let instruction_30 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r2 = instruction_30;
    // mov r0.x, l(0)
    let instruction_31 = vec4<f32>(0.0);
    r0 = vec4<f32>(instruction_31.x, r0.y, r0.z, r0.w);
    // mov r3.xyzw, l(0,0,0,0)
    let instruction_32 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r3 = instruction_32;
    }
    // eq r3.w, r3.w, l(0.000000)
    let instruction_33 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).wwww) == (vec4<f32>(0.0))));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_33.w);
    // if_nz r3.w
    if bitcast<vec4<u32>>((r3).wwww).x != 0u {
    // mad r1.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000), l(0.250000, 0.000000, 0.000000, 0.000000)
    let instruction_34 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.25, 0.0, 0.0), vec4<f32>(0.25, 0.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_34.x, instruction_34.y, r1.z, r1.w);
    // mul r3.w, cb2[5].y, l(250.000000)
    let instruction_35 = ((material.source[13]).yyyy) * (vec4<f32>(250.0));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_35.w);
    // div r2.y, r3.w, r0.y
    let instruction_36 = ((r3).wwww) / ((r0).yyyy);
    r2 = vec4<f32>(r2.x, instruction_36.y, r2.z, r2.w);
    // mul r2.x, r1.z, r2.y
    let instruction_37 = ((r1).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(instruction_37.x, r2.y, r2.z, r2.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_38 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_38;
    // mov r3.xyz, l(1.000000,0.666000,0.333000,0)
    let instruction_39 = vec4<f32>(1.0, 0.6660000085830688, 0.3330000042915344, 0.0);
    r3 = vec4<f32>(instruction_39.x, instruction_39.y, instruction_39.z, r3.w);
    // mov r2.zw, r0.zzzw
    let instruction_40 = (r0).zzzw;
    r2 = vec4<f32>(r2.x, r2.y, instruction_40.z, instruction_40.w);
    // mov o0.xy, l(0,1.000000,0,0)
    let instruction_41 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_41.x, instruction_41.y, o0.z, o0.w);
    // mov r0.x, l(1.000000)
    let instruction_42 = vec4<f32>(1.0);
    r0 = vec4<f32>(instruction_42.x, r0.y, r0.z, r0.w);
    } else {
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_43 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_43;
    }
    // mul r0.yzw, r3.xxyz, cb2[5].xxxx
    let instruction_44 = ((r3).xxyz) * ((material.source[13]).xxxx);
    r0 = vec4<f32>(r0.x, instruction_44.y, instruction_44.z, instruction_44.w);
    // mul r0.yzw, r0.yyzw, v1.wwww
    let instruction_45 = ((r0).yyzw) * ((input.color).wwww);
    r0 = vec4<f32>(r0.x, instruction_45.y, instruction_45.z, instruction_45.w);
    // eq r0.x, r0.x, l(0.000000)
    let instruction_46 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_46.x, r0.y, r0.z, r0.w);
    // dp3 r3.x, r0.ywzy, l(0.333000, 0.333000, 0.334000, 0.000000)
    let instruction_47 = vec4<f32>(dot(((r0).ywzy).xyz, (vec4<f32>(0.3330000042915344, 0.3330000042915344, 0.33399999141693115, 0.0)).xyz));
    r3 = vec4<f32>(instruction_47.x, r3.y, r3.z, r3.w);
    // lt r3.x, r3.x, l(0.003800)
    let instruction_48 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).xxxx) < (vec4<f32>(0.003800000064074993))));
    r3 = vec4<f32>(instruction_48.x, r3.y, r3.z, r3.w);
    // or r0.x, r0.x, r3.x
    let instruction_49 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).xxxx) | bitcast<vec4<u32>>((r3).xxxx));
    r0 = vec4<f32>(instruction_49.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_50 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_50;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_51 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_51;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_52 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_52;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_53 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_53;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_54 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_54;
    } else {
    // mad r3.xy, v2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_55 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r3 = vec4<f32>(instruction_55.x, instruction_55.y, r3.z, r3.w);
    // mov r1.w, l(1.000000)
    let instruction_56 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_56.w);
    // div r3.xy, r3.xyxx, r1.zwzz
    let instruction_57 = ((r3).xyxx) / ((r1).zwzz);
    r3 = vec4<f32>(instruction_57.x, instruction_57.y, r3.z, r3.w);
    // mul r2.xy, r2.xyxx, r3.xyxx
    let instruction_58 = ((r2).xyxx) * ((r3).xyxx);
    r2 = vec4<f32>(instruction_58.x, instruction_58.y, r2.z, r2.w);
    // mad r1.zw, r2.xxxy, r1.zzzw, r2.zzzw
    let instruction_59 = fma((r2).xxxy, (r1).zzzw, (r2).zzzw);
    r1 = vec4<f32>(r1.x, r1.y, instruction_59.z, instruction_59.w);
    // mov o4.y, -r1.w
    let instruction_60 = -((r1).wwww);
    o4 = vec4<f32>(o4.x, instruction_60.y, o4.z, o4.w);
    // mad o0.zw, r1.zzzw, l(0.000000, 0.000000, 0.500000, 0.500000), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_61 = fma((r1).zzzw, vec4<f32>(0.0, 0.0, 0.5, 0.5), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o0 = vec4<f32>(o0.x, o0.y, instruction_61.z, instruction_61.w);
    // mov o1.xyz, r0.yzwy
    let instruction_62 = (r0).yzwy;
    o1 = vec4<f32>(instruction_62.x, instruction_62.y, instruction_62.z, o1.w);
    // mov o1.w, l(0)
    let instruction_63 = vec4<f32>(0.0);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_63.w);
    // mov o3.xy, r1.xyxx
    let instruction_64 = (r1).xyxx;
    o3 = vec4<f32>(instruction_64.x, instruction_64.y, o3.z, o3.w);
    // mov o3.zw, l(0,0,0,0)
    let instruction_65 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(o3.x, o3.y, instruction_65.z, instruction_65.w);
    // mov o4.x, r1.z
    let instruction_66 = (r1).zzzz;
    o4 = vec4<f32>(instruction_66.x, o4.y, o4.z, o4.w);
    // mov o4.zw, l(0,0,1.000000,1.000000)
    let instruction_67 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    o4 = vec4<f32>(o4.x, o4.y, instruction_67.z, instruction_67.w);
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_68 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_68;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_69 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_69;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_70 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_70;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_71 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_71;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_72 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_72;
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_73 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_73;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_74 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_74;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_75 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_75;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_76 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_76;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_77 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_77;
    }
    var result: SourceVaryings16;
    result.position = o4;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.flare_uv = input.uv;
    return result;
}
@fragment
fn source_fragment_16(input: SourceVaryings16) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // ne r0.x, l(0.000000, 0.000000, 0.000000, 0.000000), v1.w
    let instruction_0 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0, 0.0, 0.0, 0.0)) != ((input.v1).wwww)));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t1.xyzw, s2
    let instruction_1 = soft_flare_sprite(input.v3.xy, input.flare_uv);
    r1 = instruction_1;
    // dp4 r0.y, v2.xyzw, r1.xyzw
    let instruction_2 = vec4<f32>(dot(((input.v2).xyzw).xyzw, ((r1).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_2.y, r0.z, r0.w);
    // movc r0.xyzw, r0.xxxx, r1.xyzw, r0.yyyy
    let instruction_3 = select((r0).yyyy, (r1).xyzw, bitcast<vec4<u32>>((r0).xxxx) != vec4<u32>(0u));
    r0 = instruction_3;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_4 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_4;
    // sample r1.xyzw, v0.zwzz, t0.xyzw, s1
    let instruction_5 = textureSample(color_map, color_sampler, ((input.v0).zwzz).xy);
    r1 = instruction_5;
    // mad r1.xyzw, r1.xyzw, v0.yyyy, v0.xxxx
    let instruction_6 = fma((r1).xyzw, (input.v0).yyyy, (input.v0).xxxx);
    r1 = instruction_6;
    // mul_sat r0.xyzw, r0.xyzw, r1.xyzw
    let instruction_7 = clamp(((r0).xyzw) * ((r1).xyzw), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = instruction_7;
    // mul o0.xyzw, r0.xyzw, l(0.200000, 0.200000, 0.200000, 0.200000)
    let instruction_8 = ((r0).xyzw) * (vec4<f32>(0.20000000298023224, 0.20000000298023224, 0.20000000298023224, 0.20000000298023224));
    o0 = instruction_8;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings17 {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) flare_uv: vec2<f32>,
}
@vertex
fn source_vertex_17(input: SourceInput) -> SourceVaryings17 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // mul r0.x, cb3[0].x, l(0.000001)
    let instruction_0 = ((transform.world_rows[0]).xxxx) * (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // mad r0.x, v1.x, l(255.000000), r0.x
    let instruction_1 = fma((input.color).xxxx, vec4<f32>(255.0), (r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // round_ni r0.x, r0.x
    let instruction_2 = floor((r0).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // lt r0.y, r0.x, l(3.000000)
    let instruction_3 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(3.0))));
    r0 = vec4<f32>(r0.x, instruction_3.y, r0.z, r0.w);
    // if_nz r0.y
    if bitcast<vec4<u32>>((r0).yyyy).x != 0u {
    // mov r1.xyz, v0.xyzx
    let instruction_4 = (input.position).xyzx;
    r1 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_5 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 r2.x, r1.xyzw, cb3[0].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(instruction_6.x, r2.y, r2.z, r2.w);
    // dp4 r2.y, r1.xyzw, cb3[1].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, instruction_7.y, r2.z, r2.w);
    // dp4 r2.z, r1.xyzw, cb3[2].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_8.z, r2.w);
    // mov r2.w, l(1.000000)
    let instruction_9 = vec4<f32>(1.0);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_9.w);
    // dp4 r0.y, r2.xyzw, cb0[39].xyzw
    let instruction_10 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // ge r0.z, r0.y, l(0.000000)
    let instruction_11 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).yyyy) >= (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_11.z, r0.w);
    // if_nz r0.z
    if bitcast<vec4<u32>>((r0).zzzz).x != 0u {
    // dp4 r1.x, r2.xyzw, cb0[36].xyzw
    let instruction_12 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_12.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r2.xyzw, cb0[37].xyzw
    let instruction_13 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_13.y, r1.z, r1.w);
    // div r0.zw, r1.xxxy, r0.yyyy
    let instruction_14 = ((r1).xxxy) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_14.z, instruction_14.w);
    // mul r0.zw, r0.zzzw, l(0.000000, 0.000000, 1.000000, -1.000000)
    let instruction_15 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, 1.0, -1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_15.z, instruction_15.w);
    // div r1.z, cb0[95].y, cb0[95].x
    let instruction_16 = ((frame.render_target_size).yyyy) / ((frame.render_target_size).xxxx);
    r1 = vec4<f32>(r1.x, r1.y, instruction_16.z, r1.w);
    // max r0.x, r0.x, l(0.000000)
    let instruction_17 = max((r0).xxxx, vec4<f32>(0.0));
    r0 = vec4<f32>(instruction_17.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(2.000000)
    let instruction_18 = min((r0).xxxx, vec4<f32>(2.0));
    r0 = vec4<f32>(instruction_18.x, r0.y, r0.z, r0.w);
    // lt r0.x, r0.x, l(2.000000)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(2.0))));
    r0 = vec4<f32>(instruction_19.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r1.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.125000, 0.125000, 0.000000, 0.000000)
    let instruction_20 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.125, 0.125, 0.0, 0.0));
    r1 = vec4<f32>(instruction_20.x, instruction_20.y, r1.z, r1.w);
    // mul r0.x, cb2[5].y, l(25.000000)
    let instruction_21 = ((material.source[13]).yyyy) * (vec4<f32>(25.0));
    r0 = vec4<f32>(instruction_21.x, r0.y, r0.z, r0.w);
    // div r2.y, r0.x, r0.y
    let instruction_22 = ((r0).xxxx) / ((r0).yyyy);
    r2 = vec4<f32>(r2.x, instruction_22.y, r2.z, r2.w);
    // mul r2.x, r1.z, r2.y
    let instruction_23 = ((r1).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(instruction_23.x, r2.y, r2.z, r2.w);
    // mov r2.zw, r0.zzzw
    let instruction_24 = (r0).zzzw;
    r2 = vec4<f32>(r2.x, r2.y, instruction_24.z, instruction_24.w);
    // mov o0.xy, l(0.250000,2.500000,0,0)
    let instruction_25 = vec4<f32>(0.25, 2.5, 0.0, 0.0);
    o0 = vec4<f32>(instruction_25.x, instruction_25.y, o0.z, o0.w);
    // mov r0.x, l(1.000000)
    let instruction_26 = vec4<f32>(1.0);
    r0 = vec4<f32>(instruction_26.x, r0.y, r0.z, r0.w);
    // mov r3.xyzw, l(1.000000,1.333000,1.666000,1.000000)
    let instruction_27 = vec4<f32>(1.0, 1.3329999446868896, 1.6660000085830688, 1.0);
    r3 = instruction_27;
    } else {
    // mov o0.xy, l(0,0,0,0)
    let instruction_28 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_28.x, instruction_28.y, o0.z, o0.w);
    // mov r1.xy, l(0,0,0,0)
    let instruction_29 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r1 = vec4<f32>(instruction_29.x, instruction_29.y, r1.z, r1.w);
    // mov r2.xyzw, l(0,0,0,0)
    let instruction_30 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r2 = instruction_30;
    // mov r0.x, l(0)
    let instruction_31 = vec4<f32>(0.0);
    r0 = vec4<f32>(instruction_31.x, r0.y, r0.z, r0.w);
    // mov r3.xyzw, l(0,0,0,0)
    let instruction_32 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r3 = instruction_32;
    }
    // eq r3.w, r3.w, l(0.000000)
    let instruction_33 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).wwww) == (vec4<f32>(0.0))));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_33.w);
    // if_nz r3.w
    if bitcast<vec4<u32>>((r3).wwww).x != 0u {
    // mad r1.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000), l(0.250000, 0.000000, 0.000000, 0.000000)
    let instruction_34 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.25, 0.0, 0.0), vec4<f32>(0.25, 0.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_34.x, instruction_34.y, r1.z, r1.w);
    // mul r3.w, cb2[5].y, l(250.000000)
    let instruction_35 = ((material.source[13]).yyyy) * (vec4<f32>(250.0));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_35.w);
    // div r2.y, r3.w, r0.y
    let instruction_36 = ((r3).wwww) / ((r0).yyyy);
    r2 = vec4<f32>(r2.x, instruction_36.y, r2.z, r2.w);
    // mul r2.x, r1.z, r2.y
    let instruction_37 = ((r1).zzzz) * ((r2).yyyy);
    r2 = vec4<f32>(instruction_37.x, r2.y, r2.z, r2.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_38 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_38;
    // mov r3.xyz, l(0.333000,0.666000,1.000000,0)
    let instruction_39 = vec4<f32>(0.3330000042915344, 0.6660000085830688, 1.0, 0.0);
    r3 = vec4<f32>(instruction_39.x, instruction_39.y, instruction_39.z, r3.w);
    // mov r2.zw, r0.zzzw
    let instruction_40 = (r0).zzzw;
    r2 = vec4<f32>(r2.x, r2.y, instruction_40.z, instruction_40.w);
    // mov o0.xy, l(0,1.000000,0,0)
    let instruction_41 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_41.x, instruction_41.y, o0.z, o0.w);
    // mov r0.x, l(1.000000)
    let instruction_42 = vec4<f32>(1.0);
    r0 = vec4<f32>(instruction_42.x, r0.y, r0.z, r0.w);
    } else {
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_43 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_43;
    }
    // mul r0.yzw, r3.xxyz, cb2[5].xxxx
    let instruction_44 = ((r3).xxyz) * ((material.source[13]).xxxx);
    r0 = vec4<f32>(r0.x, instruction_44.y, instruction_44.z, instruction_44.w);
    // mul r0.yzw, r0.yyzw, v1.wwww
    let instruction_45 = ((r0).yyzw) * ((input.color).wwww);
    r0 = vec4<f32>(r0.x, instruction_45.y, instruction_45.z, instruction_45.w);
    // eq r0.x, r0.x, l(0.000000)
    let instruction_46 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_46.x, r0.y, r0.z, r0.w);
    // dp3 r3.x, r0.ywzy, l(0.333000, 0.333000, 0.334000, 0.000000)
    let instruction_47 = vec4<f32>(dot(((r0).ywzy).xyz, (vec4<f32>(0.3330000042915344, 0.3330000042915344, 0.33399999141693115, 0.0)).xyz));
    r3 = vec4<f32>(instruction_47.x, r3.y, r3.z, r3.w);
    // lt r3.x, r3.x, l(0.003800)
    let instruction_48 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r3).xxxx) < (vec4<f32>(0.003800000064074993))));
    r3 = vec4<f32>(instruction_48.x, r3.y, r3.z, r3.w);
    // or r0.x, r0.x, r3.x
    let instruction_49 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).xxxx) | bitcast<vec4<u32>>((r3).xxxx));
    r0 = vec4<f32>(instruction_49.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_50 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_50;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_51 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_51;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_52 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_52;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_53 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_53;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_54 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_54;
    } else {
    // mad r3.xy, v2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_55 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r3 = vec4<f32>(instruction_55.x, instruction_55.y, r3.z, r3.w);
    // mov r1.w, l(1.000000)
    let instruction_56 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_56.w);
    // div r3.xy, r3.xyxx, r1.zwzz
    let instruction_57 = ((r3).xyxx) / ((r1).zwzz);
    r3 = vec4<f32>(instruction_57.x, instruction_57.y, r3.z, r3.w);
    // mul r2.xy, r2.xyxx, r3.xyxx
    let instruction_58 = ((r2).xyxx) * ((r3).xyxx);
    r2 = vec4<f32>(instruction_58.x, instruction_58.y, r2.z, r2.w);
    // mad r1.zw, r2.xxxy, r1.zzzw, r2.zzzw
    let instruction_59 = fma((r2).xxxy, (r1).zzzw, (r2).zzzw);
    r1 = vec4<f32>(r1.x, r1.y, instruction_59.z, instruction_59.w);
    // mov o4.y, -r1.w
    let instruction_60 = -((r1).wwww);
    o4 = vec4<f32>(o4.x, instruction_60.y, o4.z, o4.w);
    // mad o0.zw, r1.zzzw, l(0.000000, 0.000000, 0.500000, 0.500000), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_61 = fma((r1).zzzw, vec4<f32>(0.0, 0.0, 0.5, 0.5), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o0 = vec4<f32>(o0.x, o0.y, instruction_61.z, instruction_61.w);
    // mov o1.xyz, r0.yzwy
    let instruction_62 = (r0).yzwy;
    o1 = vec4<f32>(instruction_62.x, instruction_62.y, instruction_62.z, o1.w);
    // mov o1.w, l(0)
    let instruction_63 = vec4<f32>(0.0);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_63.w);
    // mov o3.xy, r1.xyxx
    let instruction_64 = (r1).xyxx;
    o3 = vec4<f32>(instruction_64.x, instruction_64.y, o3.z, o3.w);
    // mov o3.zw, l(0,0,0,0)
    let instruction_65 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(o3.x, o3.y, instruction_65.z, instruction_65.w);
    // mov o4.x, r1.z
    let instruction_66 = (r1).zzzz;
    o4 = vec4<f32>(instruction_66.x, o4.y, o4.z, o4.w);
    // mov o4.zw, l(0,0,1.000000,1.000000)
    let instruction_67 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    o4 = vec4<f32>(o4.x, o4.y, instruction_67.z, instruction_67.w);
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_68 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_68;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_69 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_69;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_70 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_70;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_71 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_71;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_72 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_72;
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_73 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_73;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_74 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_74;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_75 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_75;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_76 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_76;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_77 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_77;
    }
    var result: SourceVaryings17;
    result.position = o4;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.flare_uv = input.uv;
    return result;
}
@fragment
fn source_fragment_17(input: SourceVaryings17) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // ne r0.x, l(0.000000, 0.000000, 0.000000, 0.000000), v1.w
    let instruction_0 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0, 0.0, 0.0, 0.0)) != ((input.v1).wwww)));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t1.xyzw, s2
    let instruction_1 = soft_flare_sprite(input.v3.xy, input.flare_uv);
    r1 = instruction_1;
    // dp4 r0.y, v2.xyzw, r1.xyzw
    let instruction_2 = vec4<f32>(dot(((input.v2).xyzw).xyzw, ((r1).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_2.y, r0.z, r0.w);
    // movc r0.xyzw, r0.xxxx, r1.xyzw, r0.yyyy
    let instruction_3 = select((r0).yyyy, (r1).xyzw, bitcast<vec4<u32>>((r0).xxxx) != vec4<u32>(0u));
    r0 = instruction_3;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_4 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_4;
    // sample r1.xyzw, v0.zwzz, t0.xyzw, s1
    let instruction_5 = textureSample(color_map, color_sampler, ((input.v0).zwzz).xy);
    r1 = instruction_5;
    // mad r1.xyzw, r1.xyzw, v0.yyyy, v0.xxxx
    let instruction_6 = fma((r1).xyzw, (input.v0).yyyy, (input.v0).xxxx);
    r1 = instruction_6;
    // mul_sat r0.xyzw, r0.xyzw, r1.xyzw
    let instruction_7 = clamp(((r0).xyzw) * ((r1).xyzw), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = instruction_7;
    // mul o0.xyzw, r0.xyzw, l(0.200000, 0.200000, 0.200000, 0.200000)
    let instruction_8 = ((r0).xyzw) * (vec4<f32>(0.20000000298023224, 0.20000000298023224, 0.20000000298023224, 0.20000000298023224));
    o0 = instruction_8;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
struct SourceVaryings18 {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) flare_uv: vec2<f32>,
}
@vertex
fn source_vertex_18(input: SourceInput) -> SourceVaryings18 {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    var r4: vec4<f32>;
    var r5: vec4<f32>;
    var r6: vec4<f32>;
    var r7: vec4<f32>;
    // mul r0.x, cb3[0].x, l(0.000001)
    let instruction_0 = ((transform.world_rows[0]).xxxx) * (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // mad r0.x, v1.x, l(255.000000), r0.x
    let instruction_1 = fma((input.color).xxxx, vec4<f32>(255.0), (r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // round_ni r0.x, r0.x
    let instruction_2 = floor((r0).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // lt r0.y, r0.x, l(9.000000)
    let instruction_3 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(9.0))));
    r0 = vec4<f32>(r0.x, instruction_3.y, r0.z, r0.w);
    // if_nz r0.y
    if bitcast<vec4<u32>>((r0).yyyy).x != 0u {
    // mov r1.xyz, v0.xyzx
    let instruction_4 = (input.position).xyzx;
    r1 = vec4<f32>(instruction_4.x, instruction_4.y, instruction_4.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_5 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // dp4 r2.x, r1.xyzw, cb3[0].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(instruction_6.x, r2.y, r2.z, r2.w);
    // dp4 r2.y, r1.xyzw, cb3[1].xyzw
    let instruction_7 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, instruction_7.y, r2.z, r2.w);
    // dp4 r2.z, r1.xyzw, cb3[2].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((transform.world_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_8.z, r2.w);
    // mov r2.w, l(1.000000)
    let instruction_9 = vec4<f32>(1.0);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_9.w);
    // dp4 r0.y, r2.xyzw, cb0[39].xyzw
    let instruction_10 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // ge r0.z, r0.y, l(0.000000)
    let instruction_11 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).yyyy) >= (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_11.z, r0.w);
    // if_nz r0.z
    if bitcast<vec4<u32>>((r0).zzzz).x != 0u {
    // dp4 r1.x, r2.xyzw, cb0[36].xyzw
    let instruction_12 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_12.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r2.xyzw, cb0[37].xyzw
    let instruction_13 = vec4<f32>(dot(((r2).xyzw).xyzw, ((frame.scene.view_projection_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_13.y, r1.z, r1.w);
    // div r0.zw, r1.xxxy, r0.yyyy
    let instruction_14 = ((r1).xxxy) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_14.z, instruction_14.w);
    // mul r1.xy, r0.zwzz, l(1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_15 = ((r0).zwzz) * (vec4<f32>(1.0, -1.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_15.x, instruction_15.y, r1.z, r1.w);
    // div r1.z, cb0[95].y, cb0[95].x
    let instruction_16 = ((frame.render_target_size).yyyy) / ((frame.render_target_size).xxxx);
    r1 = vec4<f32>(r1.x, r1.y, instruction_16.z, r1.w);
    // max r0.x, r0.x, l(0.000000)
    let instruction_17 = max((r0).xxxx, vec4<f32>(0.0));
    r0 = vec4<f32>(instruction_17.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(8.000000)
    let instruction_18 = min((r0).xxxx, vec4<f32>(8.0));
    r0 = vec4<f32>(instruction_18.x, r0.y, r0.z, r0.w);
    // lt r2.xyzw, r0.xxxx, l(1.000000, 2.000000, 3.000000, 4.000000)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(1.0, 2.0, 3.0, 4.0))));
    r2 = instruction_19;
    // if_nz r2.x
    if bitcast<vec4<u32>>((r2).xxxx).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000), l(0.250000, 0.000000, 0.000000, 0.000000)
    let instruction_20 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.25, 0.0, 0.0), vec4<f32>(0.25, 0.0, 0.0, 0.0));
    r3 = vec4<f32>(instruction_20.x, instruction_20.y, r3.z, r3.w);
    // mul r3.zw, r0.zzzw, l(0.000000, 0.000000, -1.200000, 1.200000)
    let instruction_21 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, -1.2000000476837158, 1.2000000476837158));
    r3 = vec4<f32>(r3.x, r3.y, instruction_21.z, instruction_21.w);
    // mul r2.x, cb2[5].y, l(75.000000)
    let instruction_22 = ((material.source[13]).yyyy) * (vec4<f32>(75.0));
    r2 = vec4<f32>(instruction_22.x, r2.y, r2.z, r2.w);
    // max r4.x, |r1.x|, |r1.y|
    let instruction_23 = max(abs((r1).xxxx), abs((r1).yyyy));
    r4 = vec4<f32>(instruction_23.x, r4.y, r4.z, r4.w);
    // min r4.y, r4.x, l(1.000000)
    let instruction_24 = min((r4).xxxx, vec4<f32>(1.0));
    r4 = vec4<f32>(r4.x, instruction_24.y, r4.z, r4.w);
    // mad r4.z, r4.y, l(-2.000000), l(3.000000)
    let instruction_25 = fma((r4).yyyy, vec4<f32>(-2.0), vec4<f32>(3.0));
    r4 = vec4<f32>(r4.x, r4.y, instruction_25.z, r4.w);
    // mul r4.y, r4.y, r4.y
    let instruction_26 = ((r4).yyyy) * ((r4).yyyy);
    r4 = vec4<f32>(r4.x, instruction_26.y, r4.z, r4.w);
    // mul r4.y, r4.y, r4.z
    let instruction_27 = ((r4).yyyy) * ((r4).zzzz);
    r4 = vec4<f32>(r4.x, instruction_27.y, r4.z, r4.w);
    // mul r4.z, r4.y, r4.y
    let instruction_28 = ((r4).yyyy) * ((r4).yyyy);
    r4 = vec4<f32>(r4.x, r4.y, instruction_28.z, r4.w);
    // mul r4.z, r4.z, r4.z
    let instruction_29 = ((r4).zzzz) * ((r4).zzzz);
    r4 = vec4<f32>(r4.x, r4.y, instruction_29.z, r4.w);
    // mul r4.y, r4.z, r4.y
    let instruction_30 = ((r4).zzzz) * ((r4).yyyy);
    r4 = vec4<f32>(r4.x, instruction_30.y, r4.z, r4.w);
    // min r4.y, r4.y, l(1.000000)
    let instruction_31 = min((r4).yyyy, vec4<f32>(1.0));
    r4 = vec4<f32>(r4.x, instruction_31.y, r4.z, r4.w);
    // mul r2.x, r2.x, r4.y
    let instruction_32 = ((r2).xxxx) * ((r4).yyyy);
    r2 = vec4<f32>(instruction_32.x, r2.y, r2.z, r2.w);
    // div r5.y, r2.x, r0.y
    let instruction_33 = ((r2).xxxx) / ((r0).yyyy);
    r5 = vec4<f32>(r5.x, instruction_33.y, r5.z, r5.w);
    // mul r5.x, r1.z, r5.y
    let instruction_34 = ((r1).zzzz) * ((r5).yyyy);
    r5 = vec4<f32>(instruction_34.x, r5.y, r5.z, r5.w);
    // add r4.xy, r4.xxxx, l(-0.750000, -0.850000, 0.000000, 0.000000)
    let instruction_35 = ((r4).xxxx) + (vec4<f32>(-0.75, -0.8500000238418579, 0.0, 0.0));
    r4 = vec4<f32>(instruction_35.x, instruction_35.y, r4.z, r4.w);
    // mul_sat r4.xy, r4.xyxx, l(10.000000, 6.666667, 0.000000, 0.000000)
    let instruction_36 = clamp(((r4).xyxx) * (vec4<f32>(10.0, 6.666666507720947, 0.0, 0.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r4 = vec4<f32>(instruction_36.x, instruction_36.y, r4.z, r4.w);
    // mad r4.zw, r4.xxxy, l(0.000000, 0.000000, -2.000000, -2.000000), l(0.000000, 0.000000, 3.000000, 3.000000)
    let instruction_37 = fma((r4).xxxy, vec4<f32>(0.0, 0.0, -2.0, -2.0), vec4<f32>(0.0, 0.0, 3.0, 3.0));
    r4 = vec4<f32>(r4.x, r4.y, instruction_37.z, instruction_37.w);
    // mul r4.xy, r4.xyxx, r4.xyxx
    let instruction_38 = ((r4).xyxx) * ((r4).xyxx);
    r4 = vec4<f32>(instruction_38.x, instruction_38.y, r4.z, r4.w);
    // mul r4.xy, r4.xyxx, r4.zwzz
    let instruction_39 = ((r4).xyxx) * ((r4).zwzz);
    r4 = vec4<f32>(instruction_39.x, instruction_39.y, r4.z, r4.w);
    // mul r2.x, r4.x, r4.x
    let instruction_40 = ((r4).xxxx) * ((r4).xxxx);
    r2 = vec4<f32>(instruction_40.x, r2.y, r2.z, r2.w);
    // mul r2.x, r2.x, r2.x
    let instruction_41 = ((r2).xxxx) * ((r2).xxxx);
    r2 = vec4<f32>(instruction_41.x, r2.y, r2.z, r2.w);
    // mul r2.x, r2.x, r4.x
    let instruction_42 = ((r2).xxxx) * ((r4).xxxx);
    r2 = vec4<f32>(instruction_42.x, r2.y, r2.z, r2.w);
    // min r2.x, r2.x, l(1.000000)
    let instruction_43 = min((r2).xxxx, vec4<f32>(1.0));
    r2 = vec4<f32>(instruction_43.x, r2.y, r2.z, r2.w);
    // min r4.x, r4.y, l(1.000000)
    let instruction_44 = min((r4).yyyy, vec4<f32>(1.0));
    r4 = vec4<f32>(instruction_44.x, r4.y, r4.z, r4.w);
    // add r4.x, -r4.x, l(1.000000)
    let instruction_45 = (-((r4).xxxx)) + (vec4<f32>(1.0));
    r4 = vec4<f32>(instruction_45.x, r4.y, r4.z, r4.w);
    // mul r2.x, r2.x, r4.x
    let instruction_46 = ((r2).xxxx) * ((r4).xxxx);
    r2 = vec4<f32>(instruction_46.x, r2.y, r2.z, r2.w);
    // mul r4.xyz, r2.xxxx, l(1.500000, 2.250000, 3.000000, 0.000000)
    let instruction_47 = ((r2).xxxx) * (vec4<f32>(1.5, 2.25, 3.0, 0.0));
    r4 = vec4<f32>(instruction_47.x, instruction_47.y, instruction_47.z, r4.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_48 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_48;
    // mov o0.xy, l(0.100000,1.000000,0,0)
    let instruction_49 = vec4<f32>(0.10000000149011612, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_49.x, instruction_49.y, o0.z, o0.w);
    // mov r2.x, l(1.000000)
    let instruction_50 = vec4<f32>(1.0);
    r2 = vec4<f32>(instruction_50.x, r2.y, r2.z, r2.w);
    // mov r4.w, l(1.000000)
    let instruction_51 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_51.w);
    } else {
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_52 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_52;
    // mov o0.xy, l(0,0,0,0)
    let instruction_53 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_53.x, instruction_53.y, o0.z, o0.w);
    // mov r3.xyzw, l(0,0,0,0)
    let instruction_54 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r3 = instruction_54;
    // mov r5.xy, l(0,0,0,0)
    let instruction_55 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r5 = vec4<f32>(instruction_55.x, instruction_55.y, r5.z, r5.w);
    // mov r2.x, l(0)
    let instruction_56 = vec4<f32>(0.0);
    r2 = vec4<f32>(instruction_56.x, r2.y, r2.z, r2.w);
    // mov r4.xyzw, l(0,0,0,0)
    let instruction_57 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r4 = instruction_57;
    }
    // eq r5.w, r4.w, l(0.000000)
    let instruction_58 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r4).wwww) == (vec4<f32>(0.0))));
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_58.w);
    // and r2.y, r2.y, r5.w
    let instruction_59 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).yyyy) & bitcast<vec4<u32>>((r5).wwww));
    r2 = vec4<f32>(r2.x, instruction_59.y, r2.z, r2.w);
    // if_nz r2.y
    if bitcast<vec4<u32>>((r2).yyyy).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000), l(0.250000, 0.000000, 0.000000, 0.000000)
    let instruction_60 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.25, 0.0, 0.0), vec4<f32>(0.25, 0.0, 0.0, 0.0));
    r3 = vec4<f32>(instruction_60.x, instruction_60.y, r3.z, r3.w);
    // mul r2.y, cb2[5].y, l(75.000000)
    let instruction_61 = ((material.source[13]).yyyy) * (vec4<f32>(75.0));
    r2 = vec4<f32>(r2.x, instruction_61.y, r2.z, r2.w);
    // max r5.w, |r1.x|, |r1.y|
    let instruction_62 = max(abs((r1).xxxx), abs((r1).yyyy));
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_62.w);
    // min r6.x, r5.w, l(1.000000)
    let instruction_63 = min((r5).wwww, vec4<f32>(1.0));
    r6 = vec4<f32>(instruction_63.x, r6.y, r6.z, r6.w);
    // mad r6.y, r6.x, l(-2.000000), l(3.000000)
    let instruction_64 = fma((r6).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r6 = vec4<f32>(r6.x, instruction_64.y, r6.z, r6.w);
    // mul r6.x, r6.x, r6.x
    let instruction_65 = ((r6).xxxx) * ((r6).xxxx);
    r6 = vec4<f32>(instruction_65.x, r6.y, r6.z, r6.w);
    // mul r6.x, r6.x, r6.y
    let instruction_66 = ((r6).xxxx) * ((r6).yyyy);
    r6 = vec4<f32>(instruction_66.x, r6.y, r6.z, r6.w);
    // mul r6.y, r6.x, r6.x
    let instruction_67 = ((r6).xxxx) * ((r6).xxxx);
    r6 = vec4<f32>(r6.x, instruction_67.y, r6.z, r6.w);
    // mul r6.y, r6.y, r6.y
    let instruction_68 = ((r6).yyyy) * ((r6).yyyy);
    r6 = vec4<f32>(r6.x, instruction_68.y, r6.z, r6.w);
    // mul r6.x, r6.y, r6.x
    let instruction_69 = ((r6).yyyy) * ((r6).xxxx);
    r6 = vec4<f32>(instruction_69.x, r6.y, r6.z, r6.w);
    // min r6.x, r6.x, l(1.000000)
    let instruction_70 = min((r6).xxxx, vec4<f32>(1.0));
    r6 = vec4<f32>(instruction_70.x, r6.y, r6.z, r6.w);
    // mul r2.y, r2.y, r6.x
    let instruction_71 = ((r2).yyyy) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, instruction_71.y, r2.z, r2.w);
    // div r5.y, r2.y, r0.y
    let instruction_72 = ((r2).yyyy) / ((r0).yyyy);
    r5 = vec4<f32>(r5.x, instruction_72.y, r5.z, r5.w);
    // mul r5.x, r1.z, r5.y
    let instruction_73 = ((r1).zzzz) * ((r5).yyyy);
    r5 = vec4<f32>(instruction_73.x, r5.y, r5.z, r5.w);
    // add r6.xy, r5.wwww, l(-0.750000, -0.850000, 0.000000, 0.000000)
    let instruction_74 = ((r5).wwww) + (vec4<f32>(-0.75, -0.8500000238418579, 0.0, 0.0));
    r6 = vec4<f32>(instruction_74.x, instruction_74.y, r6.z, r6.w);
    // mul_sat r6.xy, r6.xyxx, l(10.000000, 6.666667, 0.000000, 0.000000)
    let instruction_75 = clamp(((r6).xyxx) * (vec4<f32>(10.0, 6.666666507720947, 0.0, 0.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r6 = vec4<f32>(instruction_75.x, instruction_75.y, r6.z, r6.w);
    // mad r6.zw, r6.xxxy, l(0.000000, 0.000000, -2.000000, -2.000000), l(0.000000, 0.000000, 3.000000, 3.000000)
    let instruction_76 = fma((r6).xxxy, vec4<f32>(0.0, 0.0, -2.0, -2.0), vec4<f32>(0.0, 0.0, 3.0, 3.0));
    r6 = vec4<f32>(r6.x, r6.y, instruction_76.z, instruction_76.w);
    // mul r6.xy, r6.xyxx, r6.xyxx
    let instruction_77 = ((r6).xyxx) * ((r6).xyxx);
    r6 = vec4<f32>(instruction_77.x, instruction_77.y, r6.z, r6.w);
    // mul r6.xy, r6.xyxx, r6.zwzz
    let instruction_78 = ((r6).xyxx) * ((r6).zwzz);
    r6 = vec4<f32>(instruction_78.x, instruction_78.y, r6.z, r6.w);
    // mul r2.y, r6.x, r6.x
    let instruction_79 = ((r6).xxxx) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, instruction_79.y, r2.z, r2.w);
    // mul r2.y, r2.y, r2.y
    let instruction_80 = ((r2).yyyy) * ((r2).yyyy);
    r2 = vec4<f32>(r2.x, instruction_80.y, r2.z, r2.w);
    // mul r2.y, r2.y, r6.x
    let instruction_81 = ((r2).yyyy) * ((r6).xxxx);
    r2 = vec4<f32>(r2.x, instruction_81.y, r2.z, r2.w);
    // min r2.y, r2.y, l(1.000000)
    let instruction_82 = min((r2).yyyy, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_82.y, r2.z, r2.w);
    // min r5.w, r6.y, l(1.000000)
    let instruction_83 = min((r6).yyyy, vec4<f32>(1.0));
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_83.w);
    // add r5.w, -r5.w, l(1.000000)
    let instruction_84 = (-((r5).wwww)) + (vec4<f32>(1.0));
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_84.w);
    // mul r2.y, r2.y, r5.w
    let instruction_85 = ((r2).yyyy) * ((r5).wwww);
    r2 = vec4<f32>(r2.x, instruction_85.y, r2.z, r2.w);
    // mul r4.xyz, r2.yyyy, l(1.500000, 2.250000, 3.000000, 0.000000)
    let instruction_86 = ((r2).yyyy) * (vec4<f32>(1.5, 2.25, 3.0, 0.0));
    r4 = vec4<f32>(instruction_86.x, instruction_86.y, instruction_86.z, r4.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_87 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_87;
    // mov r3.zw, r1.xxxy
    let instruction_88 = (r1).xxxy;
    r3 = vec4<f32>(r3.x, r3.y, instruction_88.z, instruction_88.w);
    // mov o0.xy, l(0.100000,1.000000,0,0)
    let instruction_89 = vec4<f32>(0.10000000149011612, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_89.x, instruction_89.y, o0.z, o0.w);
    // mov r2.x, l(1.000000)
    let instruction_90 = vec4<f32>(1.0);
    r2 = vec4<f32>(instruction_90.x, r2.y, r2.z, r2.w);
    // mov r4.w, l(1.000000)
    let instruction_91 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_91.w);
    }
    // eq r2.y, r4.w, l(0.000000)
    let instruction_92 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r4).wwww) == (vec4<f32>(0.0))));
    r2 = vec4<f32>(r2.x, instruction_92.y, r2.z, r2.w);
    // and r2.y, r2.z, r2.y
    let instruction_93 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).zzzz) & bitcast<vec4<u32>>((r2).yyyy));
    r2 = vec4<f32>(r2.x, instruction_93.y, r2.z, r2.w);
    // if_nz r2.y
    if bitcast<vec4<u32>>((r2).yyyy).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.250000, 0.125000, 0.000000, 0.000000), l(0.000000, 0.750000, 0.000000, 0.000000)
    let instruction_94 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.25, 0.125, 0.0, 0.0), vec4<f32>(0.0, 0.75, 0.0, 0.0));
    r3 = vec4<f32>(instruction_94.x, instruction_94.y, r3.z, r3.w);
    // mul r3.zw, r0.zzzw, l(0.000000, 0.000000, 0.750000, -1.100000)
    let instruction_95 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, 0.75, -1.100000023841858));
    r3 = vec4<f32>(r3.x, r3.y, instruction_95.z, instruction_95.w);
    // mul r2.y, r1.z, cb2[5].y
    let instruction_96 = ((r1).zzzz) * ((material.source[13]).yyyy);
    r2 = vec4<f32>(r2.x, instruction_96.y, r2.z, r2.w);
    // mul r5.x, r2.y, l(0.100000)
    let instruction_97 = ((r2).yyyy) * (vec4<f32>(0.10000000149011612));
    r5 = vec4<f32>(instruction_97.x, r5.y, r5.z, r5.w);
    // add r2.y, |r1.x|, l(-0.650000)
    let instruction_98 = (abs((r1).xxxx)) + (vec4<f32>(-0.6499999761581421));
    r2 = vec4<f32>(r2.x, instruction_98.y, r2.z, r2.w);
    // mul_sat r2.y, r2.y, l(2.857143)
    let instruction_99 = clamp(((r2).yyyy) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_99.y, r2.z, r2.w);
    // mad r2.z, r2.y, l(-2.000000), l(3.000000)
    let instruction_100 = fma((r2).yyyy, vec4<f32>(-2.0), vec4<f32>(3.0));
    r2 = vec4<f32>(r2.x, r2.y, instruction_100.z, r2.w);
    // mul r2.y, r2.y, r2.y
    let instruction_101 = ((r2).yyyy) * ((r2).yyyy);
    r2 = vec4<f32>(r2.x, instruction_101.y, r2.z, r2.w);
    // mul r2.y, r2.y, r2.z
    let instruction_102 = ((r2).yyyy) * ((r2).zzzz);
    r2 = vec4<f32>(r2.x, instruction_102.y, r2.z, r2.w);
    // min r2.y, r2.y, l(1.000000)
    let instruction_103 = min((r2).yyyy, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_103.y, r2.z, r2.w);
    // add r2.y, -r2.y, l(1.000000)
    let instruction_104 = (-((r2).yyyy)) + (vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, instruction_104.y, r2.z, r2.w);
    // mul r4.xyz, r2.yyyy, l(1.000000, 0.500000, 1.000000, 0.000000)
    let instruction_105 = ((r2).yyyy) * (vec4<f32>(1.0, 0.5, 1.0, 0.0));
    r4 = vec4<f32>(instruction_105.x, instruction_105.y, instruction_105.z, r4.w);
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_106 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_106;
    // mov o0.xy, l(0.250000,2.250000,0,0)
    let instruction_107 = vec4<f32>(0.25, 2.25, 0.0, 0.0);
    o0 = vec4<f32>(instruction_107.x, instruction_107.y, o0.z, o0.w);
    // mul r5.y, cb2[5].y, l(0.050000)
    let instruction_108 = ((material.source[13]).yyyy) * (vec4<f32>(0.05000000074505806));
    r5 = vec4<f32>(r5.x, instruction_108.y, r5.z, r5.w);
    // mov r5.w, l(-33.000000)
    let instruction_109 = vec4<f32>(-33.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_109.w);
    // mov r6.w, l(1.000000)
    let instruction_110 = vec4<f32>(1.0);
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_110.w);
    // mov r2.xyz, l(1.000000,-0.175000,0.375000,0)
    let instruction_111 = vec4<f32>(1.0, -0.17499999701976776, 0.375, 0.0);
    r2 = vec4<f32>(instruction_111.x, instruction_111.y, instruction_111.z, r2.w);
    // mov r4.w, l(1.000000)
    let instruction_112 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_112.w);
    } else {
    // mov r2.yz, l(0,0,0,0)
    let instruction_113 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    r2 = vec4<f32>(r2.x, instruction_113.y, instruction_113.z, r2.w);
    // mov r5.w, l(0)
    let instruction_114 = vec4<f32>(0.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_114.w);
    // mov r6.w, l(0)
    let instruction_115 = vec4<f32>(0.0);
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_115.w);
    }
    // eq r7.x, r4.w, l(0.000000)
    let instruction_116 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r4).wwww) == (vec4<f32>(0.0))));
    r7 = vec4<f32>(instruction_116.x, r7.y, r7.z, r7.w);
    // and r2.w, r2.w, r7.x
    let instruction_117 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).wwww) & bitcast<vec4<u32>>((r7).xxxx));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_117.w);
    // if_nz r2.w
    if bitcast<vec4<u32>>((r2).wwww).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.000000, 0.250000, 0.000000, 0.000000)
    let instruction_118 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.0, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_118.x, instruction_118.y, r3.z, r3.w);
    // mul r2.w, r1.z, cb2[5].y
    let instruction_119 = ((r1).zzzz) * ((material.source[13]).yyyy);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_119.w);
    // mul r5.x, r2.w, l(0.017500)
    let instruction_120 = ((r2).wwww) * (vec4<f32>(0.017500000074505806));
    r5 = vec4<f32>(instruction_120.x, r5.y, r5.z, r5.w);
    // add r2.w, |r1.x|, l(-0.650000)
    let instruction_121 = (abs((r1).xxxx)) + (vec4<f32>(-0.6499999761581421));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_121.w);
    // mul_sat r2.w, r2.w, l(2.857143)
    let instruction_122 = clamp(((r2).wwww) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_122.w);
    // mad r7.x, r2.w, l(-2.000000), l(3.000000)
    let instruction_123 = fma((r2).wwww, vec4<f32>(-2.0), vec4<f32>(3.0));
    r7 = vec4<f32>(instruction_123.x, r7.y, r7.z, r7.w);
    // mul r2.w, r2.w, r2.w
    let instruction_124 = ((r2).wwww) * ((r2).wwww);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_124.w);
    // mul r2.w, r2.w, r7.x
    let instruction_125 = ((r2).wwww) * ((r7).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_125.w);
    // min r2.w, r2.w, l(1.000000)
    let instruction_126 = min((r2).wwww, vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_126.w);
    // add r2.w, -r2.w, l(1.000000)
    let instruction_127 = (-((r2).wwww)) + (vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_127.w);
    // mul r4.xyz, r2.wwww, l(1.000000, 0.750000, 0.250000, 0.000000)
    let instruction_128 = ((r2).wwww) * (vec4<f32>(1.0, 0.75, 0.25, 0.0));
    r4 = vec4<f32>(instruction_128.x, instruction_128.y, instruction_128.z, r4.w);
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_129 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_129;
    // mov r3.zw, r1.xxxy
    let instruction_130 = (r1).xxxy;
    r3 = vec4<f32>(r3.x, r3.y, instruction_130.z, instruction_130.w);
    // mov o0.xy, l(0.250000,1.000000,0,0)
    let instruction_131 = vec4<f32>(0.25, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_131.x, instruction_131.y, o0.z, o0.w);
    // mul r5.y, cb2[5].y, l(0.015000)
    let instruction_132 = ((material.source[13]).yyyy) * (vec4<f32>(0.014999999664723873));
    r5 = vec4<f32>(r5.x, instruction_132.y, r5.z, r5.w);
    // mov r5.w, l(115.000000)
    let instruction_133 = vec4<f32>(115.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_133.w);
    // mov r6.w, l(0)
    let instruction_134 = vec4<f32>(0.0);
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_134.w);
    // mov r2.xyz, l(1.000000,0,0.125000,0)
    let instruction_135 = vec4<f32>(1.0, 0.0, 0.125, 0.0);
    r2 = vec4<f32>(instruction_135.x, instruction_135.y, instruction_135.z, r2.w);
    // mov r4.w, l(1.000000)
    let instruction_136 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_136.w);
    }
    // eq r2.w, r4.w, l(0.000000)
    let instruction_137 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r4).wwww) == (vec4<f32>(0.0))));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_137.w);
    // lt r7.xyzw, r0.xxxx, l(5.000000, 6.000000, 7.000000, 8.000000)
    let instruction_138 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(5.0, 6.0, 7.0, 8.0))));
    r7 = instruction_138;
    // and r0.x, r2.w, r7.x
    let instruction_139 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r2).wwww) & bitcast<vec4<u32>>((r7).xxxx));
    r0 = vec4<f32>(instruction_139.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.000000, 0.250000, 0.000000, 0.000000)
    let instruction_140 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.0, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_140.x, instruction_140.y, r3.z, r3.w);
    // mul r3.zw, r0.zzzw, l(0.000000, 0.000000, 0.900000, -1.250000)
    let instruction_141 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, 0.8999999761581421, -1.25));
    r3 = vec4<f32>(r3.x, r3.y, instruction_141.z, instruction_141.w);
    // mul r0.x, r1.z, cb2[5].y
    let instruction_142 = ((r1).zzzz) * ((material.source[13]).yyyy);
    r0 = vec4<f32>(instruction_142.x, r0.y, r0.z, r0.w);
    // mul r5.x, r0.x, l(0.032500)
    let instruction_143 = ((r0).xxxx) * (vec4<f32>(0.032499998807907104));
    r5 = vec4<f32>(instruction_143.x, r5.y, r5.z, r5.w);
    // add r0.x, |r1.x|, l(-0.650000)
    let instruction_144 = (abs((r1).xxxx)) + (vec4<f32>(-0.6499999761581421));
    r0 = vec4<f32>(instruction_144.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, l(2.857143)
    let instruction_145 = clamp(((r0).xxxx) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_145.x, r0.y, r0.z, r0.w);
    // mad r2.w, r0.x, l(-2.000000), l(3.000000)
    let instruction_146 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_146.w);
    // mul r0.x, r0.x, r0.x
    let instruction_147 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_147.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r2.w
    let instruction_148 = ((r0).xxxx) * ((r2).wwww);
    r0 = vec4<f32>(instruction_148.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_149 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_149.x, r0.y, r0.z, r0.w);
    // add r4.xyz, -r0.xxxx, l(1.000000, 1.000000, 1.000000, 0.000000)
    let instruction_150 = (-((r0).xxxx)) + (vec4<f32>(1.0, 1.0, 1.0, 0.0));
    r4 = vec4<f32>(instruction_150.x, instruction_150.y, instruction_150.z, r4.w);
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_151 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_151;
    // mov o0.xy, l(0.100000,0.900000,0,0)
    let instruction_152 = vec4<f32>(0.10000000149011612, 0.8999999761581421, 0.0, 0.0);
    o0 = vec4<f32>(instruction_152.x, instruction_152.y, o0.z, o0.w);
    // mul r5.y, cb2[5].y, l(0.017500)
    let instruction_153 = ((material.source[13]).yyyy) * (vec4<f32>(0.017500000074505806));
    r5 = vec4<f32>(r5.x, instruction_153.y, r5.z, r5.w);
    // mov r5.w, l(-28.392311)
    let instruction_154 = vec4<f32>(-28.392311096191406);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_154.w);
    // mov r6.w, l(0)
    let instruction_155 = vec4<f32>(0.0);
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_155.w);
    // mov r2.xyz, l(1.000000,-0.175000,-0.125000,0)
    let instruction_156 = vec4<f32>(1.0, -0.17499999701976776, -0.125, 0.0);
    r2 = vec4<f32>(instruction_156.x, instruction_156.y, instruction_156.z, r2.w);
    // mov r4.w, l(1.000000)
    let instruction_157 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_157.w);
    }
    // eq r0.x, r4.w, l(0.000000)
    let instruction_158 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r4).wwww) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_158.x, r0.y, r0.z, r0.w);
    // and r0.x, r7.y, r0.x
    let instruction_159 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r7).yyyy) & bitcast<vec4<u32>>((r0).xxxx));
    r0 = vec4<f32>(instruction_159.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.250000, 0.250000, 0.000000, 0.000000)
    let instruction_160 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.25, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_160.x, instruction_160.y, r3.z, r3.w);
    // mul r3.zw, r0.zzzw, l(0.000000, 0.000000, 1.175000, -1.175000)
    let instruction_161 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, 1.1749999523162842, -1.1749999523162842));
    r3 = vec4<f32>(r3.x, r3.y, instruction_161.z, instruction_161.w);
    // mul r5.yz, cb2[5].yyyy, l(0.000000, 0.125000, 0.125000, 0.000000)
    let instruction_162 = ((material.source[13]).yyyy) * (vec4<f32>(0.0, 0.125, 0.125, 0.0));
    r5 = vec4<f32>(r5.x, instruction_162.y, instruction_162.z, r5.w);
    // mul r5.x, r1.z, r5.z
    let instruction_163 = ((r1).zzzz) * ((r5).zzzz);
    r5 = vec4<f32>(instruction_163.x, r5.y, r5.z, r5.w);
    // add r0.x, |r1.x|, l(-0.650000)
    let instruction_164 = (abs((r1).xxxx)) + (vec4<f32>(-0.6499999761581421));
    r0 = vec4<f32>(instruction_164.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, l(2.857143)
    let instruction_165 = clamp(((r0).xxxx) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_165.x, r0.y, r0.z, r0.w);
    // mad r2.w, r0.x, l(-2.000000), l(3.000000)
    let instruction_166 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_166.w);
    // mul r0.x, r0.x, r0.x
    let instruction_167 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_167.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r2.w
    let instruction_168 = ((r0).xxxx) * ((r2).wwww);
    r0 = vec4<f32>(instruction_168.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_169 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_169.x, r0.y, r0.z, r0.w);
    // add r0.x, -r0.x, l(1.000000)
    let instruction_170 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_170.x, r0.y, r0.z, r0.w);
    // mul r4.xyz, r0.xxxx, l(0.450000, 0.300000, 0.750000, 0.000000)
    let instruction_171 = ((r0).xxxx) * (vec4<f32>(0.44999998807907104, 0.30000001192092896, 0.75, 0.0));
    r4 = vec4<f32>(instruction_171.x, instruction_171.y, instruction_171.z, r4.w);
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_172 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_172;
    // mov o0.xy, l(0.100000,0.900000,0,0)
    let instruction_173 = vec4<f32>(0.10000000149011612, 0.8999999761581421, 0.0, 0.0);
    o0 = vec4<f32>(instruction_173.x, instruction_173.y, o0.z, o0.w);
    // mov r5.w, l(-170.228897)
    let instruction_174 = vec4<f32>(-170.22889709472656);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_174.w);
    // mov r6.w, l(0)
    let instruction_175 = vec4<f32>(0.0);
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_175.w);
    // mov r2.xyz, l(1.000000,0.175000,-0.175000,0)
    let instruction_176 = vec4<f32>(1.0, 0.17499999701976776, -0.17499999701976776, 0.0);
    r2 = vec4<f32>(instruction_176.x, instruction_176.y, instruction_176.z, r2.w);
    // mov r4.w, l(1.000000)
    let instruction_177 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_177.w);
    }
    // eq r0.x, r4.w, l(0.000000)
    let instruction_178 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r4).wwww) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_178.x, r0.y, r0.z, r0.w);
    // and r0.x, r7.z, r0.x
    let instruction_179 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r7).zzzz) & bitcast<vec4<u32>>((r0).xxxx));
    r0 = vec4<f32>(instruction_179.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.125000, 0.125000, 0.000000, 0.000000), l(0.382812, 0.250000, 0.000000, 0.000000)
    let instruction_180 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.125, 0.125, 0.0, 0.0), vec4<f32>(0.3828125, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_180.x, instruction_180.y, r3.z, r3.w);
    // mul r3.zw, r0.zzzw, l(0.000000, 0.000000, 1.175000, -1.250000)
    let instruction_181 = ((r0).zzzw) * (vec4<f32>(0.0, 0.0, 1.1749999523162842, -1.25));
    r3 = vec4<f32>(r3.x, r3.y, instruction_181.z, instruction_181.w);
    // mul r5.yz, cb2[5].yyyy, l(0.000000, 0.050000, 0.050000, 0.000000)
    let instruction_182 = ((material.source[13]).yyyy) * (vec4<f32>(0.0, 0.05000000074505806, 0.05000000074505806, 0.0));
    r5 = vec4<f32>(r5.x, instruction_182.y, instruction_182.z, r5.w);
    // mul r5.x, r1.z, r5.z
    let instruction_183 = ((r1).zzzz) * ((r5).zzzz);
    r5 = vec4<f32>(instruction_183.x, r5.y, r5.z, r5.w);
    // add r0.x, |r1.x|, l(-0.650000)
    let instruction_184 = (abs((r1).xxxx)) + (vec4<f32>(-0.6499999761581421));
    r0 = vec4<f32>(instruction_184.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, l(2.857143)
    let instruction_185 = clamp(((r0).xxxx) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_185.x, r0.y, r0.z, r0.w);
    // mad r0.z, r0.x, l(-2.000000), l(3.000000)
    let instruction_186 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_186.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_187 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_187.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.z
    let instruction_188 = ((r0).xxxx) * ((r0).zzzz);
    r0 = vec4<f32>(instruction_188.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_189 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_189.x, r0.y, r0.z, r0.w);
    // add r0.x, -r0.x, l(1.000000)
    let instruction_190 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_190.x, r0.y, r0.z, r0.w);
    // mul r4.xyz, r0.xxxx, l(0.500000, 1.000000, 0.250000, 0.000000)
    let instruction_191 = ((r0).xxxx) * (vec4<f32>(0.5, 1.0, 0.25, 0.0));
    r4 = vec4<f32>(instruction_191.x, instruction_191.y, instruction_191.z, r4.w);
    // mov o2.xyzw, l(1.000000,0,0,0)
    let instruction_192 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    o2 = instruction_192;
    // mov o0.xy, l(0.100000,1.000000,0,0)
    let instruction_193 = vec4<f32>(0.10000000149011612, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_193.x, instruction_193.y, o0.z, o0.w);
    // mov r2.yz, l(0,0,-0.125000,0)
    let instruction_194 = vec4<f32>(0.0, 0.0, -0.125, 0.0);
    r2 = vec4<f32>(r2.x, instruction_194.y, instruction_194.z, r2.w);
    // mov r5.w, l(371.308472)
    let instruction_195 = vec4<f32>(371.3084716796875);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_195.w);
    // mov r6.w, l(0)
    let instruction_196 = vec4<f32>(0.0);
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_196.w);
    // mov r2.x, l(1.000000)
    let instruction_197 = vec4<f32>(1.0);
    r2 = vec4<f32>(instruction_197.x, r2.y, r2.z, r2.w);
    // mov r4.w, l(1.000000)
    let instruction_198 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_198.w);
    }
    // eq r0.x, r4.w, l(0.000000)
    let instruction_199 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r4).wwww) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_199.x, r0.y, r0.z, r0.w);
    // and r0.x, r7.w, r0.x
    let instruction_200 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r7).wwww) & bitcast<vec4<u32>>((r0).xxxx));
    r0 = vec4<f32>(instruction_200.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mad r3.xy, v2.xyxx, l(0.500000, 0.500000, 0.000000, 0.000000), l(0.500000, 0.000000, 0.000000, 0.000000)
    let instruction_201 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(0.5, 0.5, 0.0, 0.0), vec4<f32>(0.5, 0.0, 0.0, 0.0));
    r3 = vec4<f32>(instruction_201.x, instruction_201.y, r3.z, r3.w);
    // mul r0.x, cb2[5].y, l(25.000000)
    let instruction_202 = ((material.source[13]).yyyy) * (vec4<f32>(25.0));
    r0 = vec4<f32>(instruction_202.x, r0.y, r0.z, r0.w);
    // div r5.y, r0.x, r0.y
    let instruction_203 = ((r0).xxxx) / ((r0).yyyy);
    r5 = vec4<f32>(r5.x, instruction_203.y, r5.z, r5.w);
    // mul r5.x, r1.z, r5.y
    let instruction_204 = ((r1).zzzz) * ((r5).yyyy);
    r5 = vec4<f32>(instruction_204.x, r5.y, r5.z, r5.w);
    // max r0.x, |r1.x|, |r1.y|
    let instruction_205 = max(abs((r1).xxxx), abs((r1).yyyy));
    r0 = vec4<f32>(instruction_205.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, l(-0.650000)
    let instruction_206 = ((r0).xxxx) + (vec4<f32>(-0.6499999761581421));
    r0 = vec4<f32>(instruction_206.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, l(2.857143)
    let instruction_207 = clamp(((r0).xxxx) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_207.x, r0.y, r0.z, r0.w);
    // mad r0.z, r0.x, l(-2.000000), l(3.000000)
    let instruction_208 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_208.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_209 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_209.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.z
    let instruction_210 = ((r0).xxxx) * ((r0).zzzz);
    r0 = vec4<f32>(instruction_210.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_211 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_211.x, r0.y, r0.z, r0.w);
    // add r0.x, -r0.x, l(1.000000)
    let instruction_212 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_212.x, r0.y, r0.z, r0.w);
    // mul r4.xyz, r0.xxxx, l(0.750000, 0.675000, 0.450000, 0.000000)
    let instruction_213 = ((r0).xxxx) * (vec4<f32>(0.75, 0.675000011920929, 0.44999998807907104, 0.0));
    r4 = vec4<f32>(instruction_213.x, instruction_213.y, instruction_213.z, r4.w);
    // mov o2.xyzw, l(0,0,1.000000,0)
    let instruction_214 = vec4<f32>(0.0, 0.0, 1.0, 0.0);
    o2 = instruction_214;
    // mov r3.zw, r1.xxxy
    let instruction_215 = (r1).xxxy;
    r3 = vec4<f32>(r3.x, r3.y, instruction_215.z, instruction_215.w);
    // mov o0.xy, l(1.000000,1.000000,0,0)
    let instruction_216 = vec4<f32>(1.0, 1.0, 0.0, 0.0);
    o0 = vec4<f32>(instruction_216.x, instruction_216.y, o0.z, o0.w);
    // mov r5.w, l(0)
    let instruction_217 = vec4<f32>(0.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_217.w);
    // mov r6.w, l(0)
    let instruction_218 = vec4<f32>(0.0);
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_218.w);
    // mov r2.xyz, l(1.000000,0,0,0)
    let instruction_219 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    r2 = vec4<f32>(instruction_219.x, instruction_219.y, instruction_219.z, r2.w);
    // mov r4.w, l(1.000000)
    let instruction_220 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_220.w);
    }
    // eq r0.x, r4.w, l(0.000000)
    let instruction_221 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r4).wwww) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_221.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mul r3.xy, v2.xyxx, l(0.250000, 0.250000, 0.000000, 0.000000)
    let instruction_222 = ((vec4<f32>(input.uv, 0.0, 0.0)).xyxx) * (vec4<f32>(0.25, 0.25, 0.0, 0.0));
    r3 = vec4<f32>(instruction_222.x, instruction_222.y, r3.z, r3.w);
    // mul r0.x, cb2[5].y, l(66.000000)
    let instruction_223 = ((material.source[13]).yyyy) * (vec4<f32>(66.0));
    r0 = vec4<f32>(instruction_223.x, r0.y, r0.z, r0.w);
    // div r5.y, r0.x, r0.y
    let instruction_224 = ((r0).xxxx) / ((r0).yyyy);
    r5 = vec4<f32>(r5.x, instruction_224.y, r5.z, r5.w);
    // mul r5.x, r1.z, r5.y
    let instruction_225 = ((r1).zzzz) * ((r5).yyyy);
    r5 = vec4<f32>(instruction_225.x, r5.y, r5.z, r5.w);
    // max r0.x, |r1.x|, |r1.y|
    let instruction_226 = max(abs((r1).xxxx), abs((r1).yyyy));
    r0 = vec4<f32>(instruction_226.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, l(-0.650000)
    let instruction_227 = ((r0).xxxx) + (vec4<f32>(-0.6499999761581421));
    r0 = vec4<f32>(instruction_227.x, r0.y, r0.z, r0.w);
    // mul_sat r0.x, r0.x, l(2.857143)
    let instruction_228 = clamp(((r0).xxxx) * (vec4<f32>(2.857142925262451)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_228.x, r0.y, r0.z, r0.w);
    // mad r0.y, r0.x, l(-2.000000), l(3.000000)
    let instruction_229 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, instruction_229.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_230 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_230.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.y
    let instruction_231 = ((r0).xxxx) * ((r0).yyyy);
    r0 = vec4<f32>(instruction_231.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_232 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_232.x, r0.y, r0.z, r0.w);
    // add r0.x, -r0.x, l(1.000000)
    let instruction_233 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_233.x, r0.y, r0.z, r0.w);
    // mul r4.xyz, r0.xxxx, l(1.000000, 0.750000, 0.500000, 0.000000)
    let instruction_234 = ((r0).xxxx) * (vec4<f32>(1.0, 0.75, 0.5, 0.0));
    r4 = vec4<f32>(instruction_234.x, instruction_234.y, instruction_234.z, r4.w);
    // mov o2.xyzw, l(0,1.000000,0,0)
    let instruction_235 = vec4<f32>(0.0, 1.0, 0.0, 0.0);
    o2 = instruction_235;
    // mov r3.zw, r1.xxxy
    let instruction_236 = (r1).xxxy;
    r3 = vec4<f32>(r3.x, r3.y, instruction_236.z, instruction_236.w);
    // mov o0.xy, l(0,1.750000,0,0)
    let instruction_237 = vec4<f32>(0.0, 1.75, 0.0, 0.0);
    o0 = vec4<f32>(instruction_237.x, instruction_237.y, o0.z, o0.w);
    // mov r5.w, l(0)
    let instruction_238 = vec4<f32>(0.0);
    r5 = vec4<f32>(r5.x, r5.y, r5.z, instruction_238.w);
    // mov r6.w, l(0)
    let instruction_239 = vec4<f32>(0.0);
    r6 = vec4<f32>(r6.x, r6.y, r6.z, instruction_239.w);
    // mov r2.xyz, l(1.000000,0,0,0)
    let instruction_240 = vec4<f32>(1.0, 0.0, 0.0, 0.0);
    r2 = vec4<f32>(instruction_240.x, instruction_240.y, instruction_240.z, r2.w);
    }
    // mul r0.xyz, r4.xyzx, cb2[5].xxxx
    let instruction_241 = ((r4).xyzx) * ((material.source[13]).xxxx);
    r0 = vec4<f32>(instruction_241.x, instruction_241.y, instruction_241.z, r0.w);
    // mul r6.xyz, r0.xyzx, v1.wwww
    let instruction_242 = ((r0).xyzx) * ((input.color).wwww);
    r6 = vec4<f32>(instruction_242.x, instruction_242.y, instruction_242.z, r6.w);
    // eq r0.x, r2.x, l(0.000000)
    let instruction_243 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r2).xxxx) == (vec4<f32>(0.0))));
    r0 = vec4<f32>(instruction_243.x, r0.y, r0.z, r0.w);
    // dp3 r0.y, r6.xzyx, l(0.333000, 0.333000, 0.334000, 0.000000)
    let instruction_244 = vec4<f32>(dot(((r6).xzyx).xyz, (vec4<f32>(0.3330000042915344, 0.3330000042915344, 0.33399999141693115, 0.0)).xyz));
    r0 = vec4<f32>(r0.x, instruction_244.y, r0.z, r0.w);
    // lt r0.y, r0.y, l(0.003800)
    let instruction_245 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).yyyy) < (vec4<f32>(0.003800000064074993))));
    r0 = vec4<f32>(r0.x, instruction_245.y, r0.z, r0.w);
    // or r0.x, r0.y, r0.x
    let instruction_246 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).yyyy) | bitcast<vec4<u32>>((r0).xxxx));
    r0 = vec4<f32>(instruction_246.x, r0.y, r0.z, r0.w);
    // if_nz r0.x
    if bitcast<vec4<u32>>((r0).xxxx).x != 0u {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_247 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_247;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_248 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_248;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_249 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_249;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_250 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_250;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_251 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_251;
    } else {
    // mad r0.xy, v2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_252 = fma((vec4<f32>(input.uv, 0.0, 0.0)).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r0 = vec4<f32>(instruction_252.x, instruction_252.y, r0.z, r0.w);
    // mul r0.z, r5.w, l(0.017453)
    let instruction_253 = ((r5).wwww) * (vec4<f32>(0.01745329238474369));
    r0 = vec4<f32>(r0.x, r0.y, instruction_253.z, r0.w);
    // sincos r1.x, r2.x, r0.z
    let angle254 = (r0).zzzz;
    let instruction_254 = sin(angle254);
    r1 = vec4<f32>(instruction_254.x, r1.y, r1.z, r1.w);
    let instruction_255 = cos(angle254);
    r2 = vec4<f32>(instruction_255.x, r2.y, r2.z, r2.w);
    // mov r1.w, l(1.000000)
    let instruction_256 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_256.w);
    // div r0.xy, r0.xyxx, r1.zwzz
    let instruction_257 = ((r0).xyxx) / ((r1).zwzz);
    r0 = vec4<f32>(instruction_257.x, instruction_257.y, r0.z, r0.w);
    // mul r0.xy, r5.xyxx, r0.xyxx
    let instruction_258 = ((r5).xyxx) * ((r0).xyxx);
    r0 = vec4<f32>(instruction_258.x, instruction_258.y, r0.z, r0.w);
    // mov r4.y, r2.x
    let instruction_259 = (r2).xxxx;
    r4 = vec4<f32>(r4.x, instruction_259.y, r4.z, r4.w);
    // mov r4.z, r1.x
    let instruction_260 = (r1).xxxx;
    r4 = vec4<f32>(r4.x, r4.y, instruction_260.z, r4.w);
    // dp2 r0.z, r4.yzyy, r0.xyxx
    let instruction_261 = vec4<f32>(dot(((r4).yzyy).xy, ((r0).xyxx).xy));
    r0 = vec4<f32>(r0.x, r0.y, instruction_261.z, r0.w);
    // mov r4.x, -r1.x
    let instruction_262 = -((r1).xxxx);
    r4 = vec4<f32>(instruction_262.x, r4.y, r4.z, r4.w);
    // dp2 r0.w, r4.xyxx, r0.xyxx
    let instruction_263 = vec4<f32>(dot(((r4).xyxx).xy, ((r0).xyxx).xy));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_263.w);
    // mad r0.xy, r0.zwzz, r1.zwzz, r2.yzyy
    let instruction_264 = fma((r0).zwzz, (r1).zwzz, (r2).yzyy);
    r0 = vec4<f32>(instruction_264.x, instruction_264.y, r0.z, r0.w);
    // add r0.xy, r3.zwzz, r0.xyxx
    let instruction_265 = ((r3).zwzz) + ((r0).xyxx);
    r0 = vec4<f32>(instruction_265.x, instruction_265.y, r0.z, r0.w);
    // mov o4.y, -r0.y
    let instruction_266 = -((r0).yyyy);
    o4 = vec4<f32>(o4.x, instruction_266.y, o4.z, o4.w);
    // mad o0.zw, r0.xxxy, l(0.000000, 0.000000, 0.500000, 0.500000), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_267 = fma((r0).xxxy, vec4<f32>(0.0, 0.0, 0.5, 0.5), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o0 = vec4<f32>(o0.x, o0.y, instruction_267.z, instruction_267.w);
    // mov o1.xyzw, r6.xyzw
    let instruction_268 = (r6).xyzw;
    o1 = instruction_268;
    // mov o3.xy, r3.xyxx
    let instruction_269 = (r3).xyxx;
    o3 = vec4<f32>(instruction_269.x, instruction_269.y, o3.z, o3.w);
    // mov o3.zw, l(0,0,0,0)
    let instruction_270 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = vec4<f32>(o3.x, o3.y, instruction_270.z, instruction_270.w);
    // mov o4.x, r0.x
    let instruction_271 = (r0).xxxx;
    o4 = vec4<f32>(instruction_271.x, o4.y, o4.z, o4.w);
    // mov o4.zw, l(0,0,1.000000,1.000000)
    let instruction_272 = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    o4 = vec4<f32>(o4.x, o4.y, instruction_272.z, instruction_272.w);
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_273 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_273;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_274 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_274;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_275 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_275;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_276 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_276;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_277 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_277;
    }
    } else {
    // mov o0.xyzw, l(0,0,0,0)
    let instruction_278 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o0 = instruction_278;
    // mov o1.xyzw, l(0,0,0,0)
    let instruction_279 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o1 = instruction_279;
    // mov o2.xyzw, l(0,0,0,0)
    let instruction_280 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o2 = instruction_280;
    // mov o3.xyzw, l(0,0,0,0)
    let instruction_281 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    o3 = instruction_281;
    // mov o4.xyzw, l(-1000.000000,-1000.000000,-1000.000000,-1000.000000)
    let instruction_282 = vec4<f32>(-1000.0, -1000.0, -1000.0, -1000.0);
    o4 = instruction_282;
    }
    var result: SourceVaryings18;
    result.position = o4;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.flare_uv = input.uv;
    return result;
}
@fragment
fn source_fragment_18(input: SourceVaryings18) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // ne r0.x, l(0.000000, 0.000000, 0.000000, 0.000000), v1.w
    let instruction_0 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0, 0.0, 0.0, 0.0)) != ((input.v1).wwww)));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t1.xyzw, s2
    let instruction_1 = soft_flare_sprite(input.v3.xy, input.flare_uv);
    r1 = instruction_1;
    // dp4 r0.y, v2.xyzw, r1.xyzw
    let instruction_2 = vec4<f32>(dot(((input.v2).xyzw).xyzw, ((r1).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_2.y, r0.z, r0.w);
    // movc r0.xyzw, r0.xxxx, r1.xyzw, r0.yyyy
    let instruction_3 = select((r0).yyyy, (r1).xyzw, bitcast<vec4<u32>>((r0).xxxx) != vec4<u32>(0u));
    r0 = instruction_3;
    // mul r0.xyzw, r0.xyzw, v1.xyzw
    let instruction_4 = ((r0).xyzw) * ((input.v1).xyzw);
    r0 = instruction_4;
    // sample r1.xyzw, v0.zwzz, t0.xyzw, s1
    let instruction_5 = textureSample(color_map, color_sampler, ((input.v0).zwzz).xy);
    r1 = instruction_5;
    // mad r1.xyzw, r1.xyzw, v0.yyyy, v0.xxxx
    let instruction_6 = fma((r1).xyzw, (input.v0).yyyy, (input.v0).xxxx);
    r1 = instruction_6;
    // mul_sat r0.xyzw, r0.xyzw, r1.xyzw
    let instruction_7 = clamp(((r0).xyzw) * ((r1).xyzw), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = instruction_7;
    // mul o0.xyzw, r0.xyzw, l(0.200000, 0.200000, 0.200000, 0.200000)
    let instruction_8 = ((r0).xyzw) * (vec4<f32>(0.20000000298023224, 0.20000000298023224, 0.20000000298023224, 0.20000000298023224));
    o0 = instruction_8;
    if material.alpha_test.x == 1.0 && !(o0.a > 0.0) { discard; }
    if material.alpha_test.x == 2.0 && o0.a < 0.501960813999176 { discard; }
    return o0;
}
