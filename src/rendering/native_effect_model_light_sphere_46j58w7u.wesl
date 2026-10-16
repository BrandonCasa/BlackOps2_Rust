// Original mc_sw4_3d_light_sphere_46j58w7u; immutable DXBC hashes in effect source-inventory.json.
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
@group(3) @binding(0) var<uniform> frame: NativeFxFrame;
@group(3) @binding(1) var float_z: texture_2d<f32>;
@group(3) @binding(2) var float_z_sampler: sampler;
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(2) v2: vec4<f32>,
}
@vertex
fn vertex(input: Input) -> Varyings {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // dp3 r0.x, v0.xyzx, cb3[0].xyzx
    let instruction_0 = vec4<f32>(dot(((input.position).xyzx).xyz, ((object.world_rows[0]).xyzx).xyz));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // dp3 r0.y, v0.xyzx, cb3[1].xyzx
    let instruction_1 = vec4<f32>(dot(((input.position).xyzx).xyz, ((object.world_rows[1]).xyzx).xyz));
    r0 = vec4<f32>(r0.x, instruction_1.y, r0.z, r0.w);
    // dp3 r0.z, v0.xyzx, cb3[2].xyzx
    let instruction_2 = vec4<f32>(dot(((input.position).xyzx).xyz, ((object.world_rows[2]).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, instruction_2.z, r0.w);
    // dp3 r0.x, r0.xyzx, r0.xyzx
    let instruction_3 = vec4<f32>(dot(((r0).xyzx).xyz, ((r0).xyzx).xyz));
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // sqrt o0.z, r0.x
    let instruction_4 = sqrt((r0).xxxx);
    o0 = vec4<f32>(o0.x, o0.y, instruction_4.z, o0.w);
    // mov o0.xy, v1.xyxx
    let instruction_5 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o0 = vec4<f32>(instruction_5.x, instruction_5.y, o0.z, o0.w);
    // mul r0.xyz, cb0[41].xyzx, cb3[1].wwww
    let instruction_6 = ((scene.world.inverse_view_rows[1]).xyzx) * ((object.world_rows[1]).wwww);
    r0 = vec4<f32>(instruction_6.x, instruction_6.y, instruction_6.z, r0.w);
    // mad r0.xyz, cb3[0].wwww, cb0[40].xyzx, r0.xyzx
    let instruction_7 = fma((object.world_rows[0]).wwww, (scene.world.inverse_view_rows[0]).xyzx, (r0).xyzx);
    r0 = vec4<f32>(instruction_7.x, instruction_7.y, instruction_7.z, r0.w);
    // mad o2.xyz, cb3[2].wwww, cb0[42].xyzx, r0.xyzx
    let instruction_8 = fma((object.world_rows[2]).wwww, (scene.world.inverse_view_rows[2]).xyzx, (r0).xyzx);
    o2 = vec4<f32>(instruction_8.x, instruction_8.y, instruction_8.z, o2.w);
    // mov r0.xyz, v0.xyzx
    let instruction_9 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_9.x, instruction_9.y, instruction_9.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_10 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_10.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_11 = vec4<f32>(dot(((r0).xyzw).xyzw, ((object.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_11.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_12 = vec4<f32>(dot(((r0).xyzw).xyzw, ((object.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_12.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_13 = vec4<f32>(dot(((r0).xyzw).xyzw, ((object.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_13.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_14 = vec4<f32>(dot(((r0).xyzw).xyzw, ((object.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_14.w);
    // dp4 o3.x, r1.xyzw, cb0[36].xyzw
    let instruction_15 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[0]).xyzw).xyzw));
    o3 = vec4<f32>(instruction_15.x, o3.y, o3.z, o3.w);
    // dp4 o3.y, r1.xyzw, cb0[37].xyzw
    let instruction_16 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[1]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, instruction_16.y, o3.z, o3.w);
    // dp4 o3.z, r1.xyzw, cb0[38].xyzw
    let instruction_17 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[2]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, o3.y, instruction_17.z, o3.w);
    // dp4 o3.w, r1.xyzw, cb0[39].xyzw
    let instruction_18 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[3]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_18.w);
    var result: Varyings;
    result.position = o3;
    result.v0 = o0;
    result.v2 = o2;
    return result;
}
fn shade(input: Varyings) -> vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // mul r0.x, cb2[5].x, l(0.008727)
    let instruction_0 = ((material.source[0]).xxxx) * (vec4<f32>(0.008726646192371845));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sincos r0.x, r1.x, r0.x
    let angle1 = (r0).xxxx;
    let instruction_1 = sin(angle1);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    let instruction_2 = cos(angle1);
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // div r0.x, r0.x, r1.x
    let instruction_3 = ((r0).xxxx) / ((r1).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, l(1.500000)
    let instruction_4 = ((r0).xxxx) * (vec4<f32>(1.5));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // div r0.yz, v3.xxyx, cb0[95].xxyx
    let instruction_5 = ((input.position).xxyx) / ((frame.render_target_size).xxyx);
    r0 = vec4<f32>(r0.x, instruction_5.y, instruction_5.z, r0.w);
    // sample r1.xyzw, r0.yzyy, t0.xyzw, s1
    let instruction_6 = textureSample(float_z, float_z_sampler, ((r0).yzyy).xy);
    r1 = instruction_6;
    // add r0.yz, r0.yyzy, l(0.000000, -0.500000, -0.500000, 0.000000)
    let instruction_7 = ((r0).yyzy) + (vec4<f32>(0.0, -0.5, -0.5, 0.0));
    r0 = vec4<f32>(r0.x, instruction_7.y, instruction_7.z, r0.w);
    // max r0.w, |r1.x|, l(0.000000)
    let instruction_8 = max(abs((r1).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_8.w);
    // div r0.w, cb0[58].x, r0.w
    let instruction_9 = ((frame.z_near).xxxx) / ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_9.w);
    // mul r0.y, r0.w, r0.y
    let instruction_10 = ((r0).wwww) * ((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // mul r0.z, r0.w, -r0.z
    let instruction_11 = ((r0).wwww) * (-((r0).zzzz));
    r0 = vec4<f32>(r0.x, r0.y, instruction_11.z, r0.w);
    // add r1.z, -r0.w, v2.z
    let instruction_12 = (-((r0).wwww)) + ((input.v2).zzzz);
    r1 = vec4<f32>(r1.x, r1.y, instruction_12.z, r1.w);
    // mad r1.y, -r0.z, r0.x, v2.y
    let instruction_13 = fma(-((r0).zzzz), (r0).xxxx, (input.v2).yyyy);
    r1 = vec4<f32>(r1.x, instruction_13.y, r1.z, r1.w);
    // mul r0.x, r0.x, r0.y
    let instruction_14 = ((r0).xxxx) * ((r0).yyyy);
    r0 = vec4<f32>(instruction_14.x, r0.y, r0.z, r0.w);
    // mad r1.x, -r0.x, l(1.777778), v2.x
    let instruction_15 = fma(-((r0).xxxx), vec4<f32>(1.7777777910232544), (input.v2).xxxx);
    r1 = vec4<f32>(instruction_15.x, r1.y, r1.z, r1.w);
    // dp3 r0.x, r1.xyzx, r1.xyzx
    let instruction_16 = vec4<f32>(dot(((r1).xyzx).xyz, ((r1).xyzx).xyz));
    r0 = vec4<f32>(instruction_16.x, r0.y, r0.z, r0.w);
    // sqrt r0.x, r0.x
    let instruction_17 = sqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_17.x, r0.y, r0.z, r0.w);
    // add r0.y, v0.z, cb2[5].y
    let instruction_18 = ((input.v0).zzzz) + ((material.source[0]).yyyy);
    r0 = vec4<f32>(r0.x, instruction_18.y, r0.z, r0.w);
    // add r0.x, -r0.x, r0.y
    let instruction_19 = (-((r0).xxxx)) + ((r0).yyyy);
    r0 = vec4<f32>(instruction_19.x, r0.y, r0.z, r0.w);
    // div_sat r0.x, r0.x, r0.y
    let instruction_20 = clamp(((r0).xxxx) / ((r0).yyyy), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_20.x, r0.y, r0.z, r0.w);
    // lt r0.y, l(0.000000), r0.x
    let instruction_21 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(1.0000000116860974e-07)) < ((r0).xxxx)));
    r0 = vec4<f32>(r0.x, instruction_21.y, r0.z, r0.w);
    // mul r0.xzw, r0.xxxx, cb2[6].xxyz
    let instruction_22 = ((r0).xxxx) * ((material.source[1]).xxyz);
    r0 = vec4<f32>(instruction_22.x, r0.y, instruction_22.z, instruction_22.w);
    // and o0.xyz, r0.xzwx, r0.yyyy
    let instruction_23 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).xzwx) & bitcast<vec4<u32>>((r0).yyyy));
    o0 = vec4<f32>(instruction_23.x, instruction_23.y, instruction_23.z, o0.w);
    // mov o0.w, l(1.000000)
    let instruction_24 = vec4<f32>(1.0);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_24.w);
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
