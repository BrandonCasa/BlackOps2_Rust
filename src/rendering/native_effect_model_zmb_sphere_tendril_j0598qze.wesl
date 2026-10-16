// Original mc_sw4_3d_zmb_sphere_tendril_j0598qze; immutable DXBC hashes in effect source-inventory.json.
// Static instruction port. Source immediates are full binary f32 values.
import package::rendering::native_model::{NativeModelScene, NativeModelObject};
import package::rendering::native_scripted_model::{NativeModelScriptedScene, NativeModelScriptInputs};
import package::rendering::native_fx::NativeFxFrame;
struct NativeEffectModelConstants { source: array<vec4<f32>, 12> }
@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(0) @binding(1) var<uniform> view: NativeModelScriptedScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(1) @binding(1) var<uniform> scripts: NativeModelScriptInputs;
@group(2) @binding(31) var<uniform> material: NativeEffectModelConstants;
struct Input {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: vec4<f32>,
    @location(4) tangent: vec4<f32>,
}
@group(2) @binding(0) var texture_t0: texture_2d<f32>;
@group(2) @binding(1) var sampler_t0: sampler;
@group(2) @binding(2) var texture_t1: texture_2d<f32>;
@group(2) @binding(3) var sampler_t1: sampler;
@group(2) @binding(4) var texture_t2: texture_2d<f32>;
@group(2) @binding(5) var sampler_t2: sampler;
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(1) v1: vec4<f32>,
}
@vertex
fn vertex(input: Input) -> Varyings {
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
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((object.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((object.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((object.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((object.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // mov o0.xyz, r1.xyzx
    let instruction_6 = (r1).xyzx;
    o0 = vec4<f32>(instruction_6.x, instruction_6.y, instruction_6.z, o0.w);
    // mov o1.xy, v1.xyxx
    let instruction_7 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o1 = vec4<f32>(instruction_7.x, instruction_7.y, o1.z, o1.w);
    // dp4 o2.x, r1.xyzw, cb0[36].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[0]).xyzw).xyzw));
    o2 = vec4<f32>(instruction_8.x, o2.y, o2.z, o2.w);
    // dp4 o2.y, r1.xyzw, cb0[37].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[1]).xyzw).xyzw));
    o2 = vec4<f32>(o2.x, instruction_9.y, o2.z, o2.w);
    // dp4 o2.z, r1.xyzw, cb0[38].xyzw
    let instruction_10 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[2]).xyzw).xyzw));
    o2 = vec4<f32>(o2.x, o2.y, instruction_10.z, o2.w);
    // dp4 o2.w, r1.xyzw, cb0[39].xyzw
    let instruction_11 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[3]).xyzw).xyzw));
    o2 = vec4<f32>(o2.x, o2.y, o2.z, instruction_11.w);
    var result: Varyings;
    result.position = o2;
    result.v1 = o1;
    return result;
}
fn shade(input: Varyings) -> vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    // add r0.xy, v1.xyxx, cb2[7].xyxx
    let instruction_0 = ((input.v1).xyxx) + ((material.source[2]).xyxx);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // mul r0.xy, r0.xyxx, cb2[5].xyxx
    let instruction_1 = ((r0).xyxx) * ((material.source[0]).xyxx);
    r0 = vec4<f32>(instruction_1.x, instruction_1.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t1.xyzw, s2
    let instruction_2 = textureSample(texture_t1, sampler_t1, ((r0).xyxx).xy);
    r0 = instruction_2;
    // mul r0.yz, cb2[4].wwww, cb2[6].xxyx
    let instruction_3 = ((scene.world.game_time).wwww) * ((material.source[1]).xxyx);
    r0 = vec4<f32>(r0.x, instruction_3.y, instruction_3.z, r0.w);
    // mul r0.yz, r0.yyzy, cb2[6].zzwz
    let instruction_4 = ((r0).yyzy) * ((material.source[1]).zzwz);
    r0 = vec4<f32>(r0.x, instruction_4.y, instruction_4.z, r0.w);
    // frc r0.yz, r0.yyzy
    let instruction_5 = fract((r0).yyzy);
    r0 = vec4<f32>(r0.x, instruction_5.y, instruction_5.z, r0.w);
    // add r1.xy, v1.xyxx, cb2[9].xyxx
    let instruction_6 = ((input.v1).xyxx) + ((material.source[4]).xyxx);
    r1 = vec4<f32>(instruction_6.x, instruction_6.y, r1.z, r1.w);
    // mad r0.yz, r1.xxyx, cb2[6].zzwz, r0.yyzy
    let instruction_7 = fma((r1).xxyx, (material.source[1]).zzwz, (r0).yyzy);
    r0 = vec4<f32>(r0.x, instruction_7.y, instruction_7.z, r0.w);
    // sample r1.xyzw, r0.yzyy, t2.xyzw, s1
    let instruction_8 = textureSample(texture_t2, sampler_t2, ((r0).yzyy).xy);
    r1 = instruction_8;
    // sample r2.xyzw, r0.yzyy, t0.xyzw, s3
    let instruction_9 = textureSample(texture_t0, sampler_t0, ((r0).yzyy).xy);
    r2 = instruction_9;
    // mul r2.w, r0.x, r1.x
    let instruction_10 = ((r0).xxxx) * ((r1).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_10.w);
    // mul r0.xyzw, r2.xyzw, cb2[8].xyzw
    let instruction_11 = ((r2).xyzw) * ((material.source[3]).xyzw);
    r0 = instruction_11;
    // mul r0.xyz, r0.wwww, r0.xyzx
    let instruction_12 = ((r0).wwww) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_12.x, instruction_12.y, instruction_12.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb2[5].zzzz
    let instruction_13 = ((r0).xyzx) * ((material.source[0]).zzzz);
    r0 = vec4<f32>(instruction_13.x, instruction_13.y, instruction_13.z, r0.w);
    // mul r0.xyz, r0.xyzx, r0.xyzx
    let instruction_14 = ((r0).xyzx) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_15 = ((r0).xyzx) * ((scene.world.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, r0.w);
    // sqrt o0.xyz, r0.xyzx
    let instruction_16 = sqrt((r0).xyzx);
    o0 = vec4<f32>(instruction_16.x, instruction_16.y, instruction_16.z, o0.w);
    // mov o0.w, l(1.000000)
    let instruction_17 = vec4<f32>(1.0);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_17.w);
    return o0;
}
@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> { return shade(input); }
@fragment
fn fragment_srgb(input: Varyings) -> @location(0) vec4<f32> {
    let encoded = shade(input);
    let linear = select(encoded.rgb / vec3<f32>(12.92), pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)), encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}
