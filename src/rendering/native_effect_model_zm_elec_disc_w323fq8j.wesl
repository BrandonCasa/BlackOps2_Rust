// Original mc_sw4_3d_zm_elec_disc_w323fq8j; immutable DXBC hashes in effect source-inventory.json.
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
@group(2) @binding(6) var texture_t3: texture_2d<f32>;
@group(2) @binding(7) var sampler_t3: sampler;
@group(2) @binding(8) var texture_t4: texture_2d<f32>;
@group(2) @binding(9) var sampler_t4: sampler;
@group(2) @binding(10) var texture_t5: texture_2d<f32>;
@group(2) @binding(11) var sampler_t5: sampler;
@group(2) @binding(12) var texture_t6: texture_2d<f32>;
@group(2) @binding(13) var sampler_t6: sampler;
@group(3) @binding(0) var<uniform> frame: NativeFxFrame;
@group(3) @binding(1) var float_z: texture_2d<f32>;
@group(3) @binding(2) var float_z_sampler: sampler;
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) v4: vec4<f32>,
}
@vertex
fn vertex(input: Input) -> Varyings {
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
    // mov o1.xyzw, v1.xyzw
    let instruction_7 = (input.color).xyzw;
    o1 = instruction_7;
    // add r0.xyz, v3.xyzx, v3.xyzx
    let instruction_8 = ((input.normal).xyzx) + ((input.normal).xyzx);
    r0 = vec4<f32>(instruction_8.x, instruction_8.y, instruction_8.z, r0.w);
    // ge r2.xyz, v3.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_9 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.normal).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r2 = vec4<f32>(instruction_9.x, instruction_9.y, instruction_9.z, r2.w);
    // mad r3.xyz, v3.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_10 = fma((input.normal).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r3 = vec4<f32>(instruction_10.x, instruction_10.y, instruction_10.z, r3.w);
    // movc r0.xyz, r2.xyzx, r3.xyzx, r0.xyzx
    let instruction_11 = select((r0).xyzx, (r3).xyzx, bitcast<vec4<u32>>((r2).xyzx) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_11.x, instruction_11.y, instruction_11.z, r0.w);
    // dp3 r2.x, r0.xyzx, cb3[0].xyzx
    let instruction_12 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[0]).xyzx).xyz));
    r2 = vec4<f32>(instruction_12.x, r2.y, r2.z, r2.w);
    // dp3 r2.y, r0.xyzx, cb3[1].xyzx
    let instruction_13 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[1]).xyzx).xyz));
    r2 = vec4<f32>(r2.x, instruction_13.y, r2.z, r2.w);
    // dp3 r2.z, r0.xyzx, cb3[2].xyzx
    let instruction_14 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[2]).xyzx).xyz));
    r2 = vec4<f32>(r2.x, r2.y, instruction_14.z, r2.w);
    // dp3 r0.x, r2.xyzx, r2.xyzx
    let instruction_15 = vec4<f32>(dot(((r2).xyzx).xyz, ((r2).xyzx).xyz));
    r0 = vec4<f32>(instruction_15.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_16 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_16.x, r0.y, r0.z, r0.w);
    // mul o2.xyz, r0.xxxx, r2.xyzx
    let instruction_17 = ((r0).xxxx) * ((r2).xyzx);
    o2 = vec4<f32>(instruction_17.x, instruction_17.y, instruction_17.z, o2.w);
    // mov o2.w, l(0)
    let instruction_18 = vec4<f32>(0.0);
    o2 = vec4<f32>(o2.x, o2.y, o2.z, instruction_18.w);
    // mov o3.xy, v2.xyxx
    let instruction_19 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o3 = vec4<f32>(instruction_19.x, instruction_19.y, o3.z, o3.w);
    // dp4 r0.x, r1.xyzw, cb0[36].xyzw
    let instruction_20 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[0]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_20.x, r0.y, r0.z, r0.w);
    // dp4 r0.y, r1.xyzw, cb0[37].xyzw
    let instruction_21 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[1]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_21.y, r0.z, r0.w);
    // dp4 r0.z, r1.xyzw, cb0[38].xyzw
    let instruction_22 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[2]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, r0.y, instruction_22.z, r0.w);
    // dp4 r0.w, r1.xyzw, cb0[39].xyzw
    let instruction_23 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_23.w);
    // mov o4.xyzw, r0.xyzw
    let instruction_24 = (r0).xyzw;
    o4 = instruction_24;
    // mov o5.xyzw, r0.xyzw
    let instruction_25 = (r0).xyzw;
    o5 = instruction_25;
    var result: Varyings;
    result.position = o5;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.v4 = o4;
    return result;
}
fn shade(input: Varyings) -> vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // dp2 r0.x, v0.xyxx, v0.xyxx
    let instruction_0 = vec4<f32>(dot(((input.v0).xyxx).xy, ((input.v0).xyxx).xy));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // sqrt r0.x, r0.x
    let instruction_1 = sqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -cb2[15].x
    let instruction_2 = ((r0).xxxx) + (-((material.source[10]).xxxx));
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // add r0.y, -cb2[15].x, cb2[15].y
    let instruction_3 = (-((material.source[10]).xxxx)) + ((material.source[10]).yyyy);
    r0 = vec4<f32>(r0.x, instruction_3.y, r0.z, r0.w);
    // div r0.y, l(1.000000, 1.000000, 1.000000, 1.000000), r0.y
    let instruction_4 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_4.y, r0.z, r0.w);
    // mul_sat r0.x, r0.y, r0.x
    let instruction_5 = clamp(((r0).yyyy) * ((r0).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // mad r0.y, r0.x, l(-2.000000), l(3.000000)
    let instruction_6 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, instruction_6.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_7 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_7.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.y
    let instruction_8 = ((r0).xxxx) * ((r0).yyyy);
    r0 = vec4<f32>(instruction_8.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, cb2[14].w
    let instruction_9 = ((r0).xxxx) * ((material.source[9]).wwww);
    r0 = vec4<f32>(instruction_9.x, r0.y, r0.z, r0.w);
    // add r0.yz, -cb2[10].xxyx, cb2[15].zzwz
    let instruction_10 = (-((material.source[5]).xxyx)) + ((material.source[10]).zzwz);
    r0 = vec4<f32>(r0.x, instruction_10.y, instruction_10.z, r0.w);
    // mad r0.xy, r0.xxxx, r0.yzyy, cb2[10].xyxx
    let instruction_11 = fma((r0).xxxx, (r0).yzyy, (material.source[5]).xyxx);
    r0 = vec4<f32>(instruction_11.x, instruction_11.y, r0.z, r0.w);
    // mul r0.zw, cb2[4].wwww, cb2[8].yyyz
    let instruction_12 = ((scene.world.game_time).wwww) * ((material.source[3]).yyyz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_12.z, instruction_12.w);
    // mul r0.zw, r0.zzzw, cb2[9].xxxy
    let instruction_13 = ((r0).zzzw) * ((material.source[4]).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_13.z, instruction_13.w);
    // frc r0.zw, r0.zzzw
    let instruction_14 = fract((r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_14.z, instruction_14.w);
    // mad r0.zw, v3.xxxy, cb2[9].xxxy, r0.zzzw
    let instruction_15 = fma((input.v3).xxxy, (material.source[4]).xxxy, (r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_15.z, instruction_15.w);
    // sample r1.xyzw, r0.zwzz, t2.xyzw, s5
    let instruction_16 = textureSample(texture_t2, sampler_t2, ((r0).zwzz).xy);
    r1 = instruction_16;
    // mad r0.zw, r1.xxxy, l(0.000000, 0.000000, 2.000000, 2.000000), l(0.000000, 0.000000, -1.000000, -1.000000)
    let instruction_17 = fma((r1).xxxy, vec4<f32>(0.0, 0.0, 2.0, 2.0), vec4<f32>(0.0, 0.0, -1.0, -1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_17.z, instruction_17.w);
    // mad r0.zw, r0.zzzw, cb2[9].zzzz, v3.xxxy
    let instruction_18 = fma((r0).zzzw, (material.source[4]).zzzz, (input.v3).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_18.z, instruction_18.w);
    // add r0.xy, r0.xyxx, r0.zwzz
    let instruction_19 = ((r0).xyxx) + ((r0).zwzz);
    r0 = vec4<f32>(instruction_19.x, instruction_19.y, r0.z, r0.w);
    // mul r0.zw, cb2[4].wwww, cb2[5].zzzw
    let instruction_20 = ((scene.world.game_time).wwww) * ((material.source[0]).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_20.z, instruction_20.w);
    // mul r0.zw, r0.zzzw, cb2[5].xxxy
    let instruction_21 = ((r0).zzzw) * ((material.source[0]).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_21.z, instruction_21.w);
    // frc r0.zw, r0.zzzw
    let instruction_22 = fract((r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_22.z, instruction_22.w);
    // mad r0.zw, r0.xxxy, cb2[5].xxxy, r0.zzzw
    let instruction_23 = fma((r0).xxxy, (material.source[0]).xxxy, (r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_23.z, instruction_23.w);
    // sample r1.xyzw, r0.zwzz, t4.xyzw, s6
    let instruction_24 = textureSample(texture_t4, sampler_t4, ((r0).zwzz).xy);
    r1 = instruction_24;
    // dp3 r0.z, v2.xyzx, v2.xyzx
    let instruction_25 = vec4<f32>(dot(((input.v2).xyzx).xyz, ((input.v2).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, instruction_25.z, r0.w);
    // rsq r0.z, r0.z
    let instruction_26 = inverseSqrt((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_26.z, r0.w);
    // mul r1.yzw, r0.zzzz, v2.xxyz
    let instruction_27 = ((r0).zzzz) * ((input.v2).xxyz);
    r1 = vec4<f32>(r1.x, instruction_27.y, instruction_27.z, instruction_27.w);
    // dp3 r0.z, v0.xyzx, v0.xyzx
    let instruction_28 = vec4<f32>(dot(((input.v0).xyzx).xyz, ((input.v0).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, instruction_28.z, r0.w);
    // rsq r0.z, r0.z
    let instruction_29 = inverseSqrt((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_29.z, r0.w);
    // mul r2.xyz, r0.zzzz, v0.xyzx
    let instruction_30 = ((r0).zzzz) * ((input.v0).xyzx);
    r2 = vec4<f32>(instruction_30.x, instruction_30.y, instruction_30.z, r2.w);
    // dp3_sat r0.z, -r2.xyzx, r1.yzwy
    let instruction_31 = clamp(vec4<f32>(dot((-((r2).xyzx)).xyz, ((r1).yzwy).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_31.z, r0.w);
    // log r0.z, r0.z
    let instruction_32 = log2((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_32.z, r0.w);
    // mul r0.z, r0.z, cb2[6].y
    let instruction_33 = ((r0).zzzz) * ((material.source[1]).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_33.z, r0.w);
    // exp r0.z, r0.z
    let instruction_34 = exp2((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_34.z, r0.w);
    // mul r0.z, r0.z, cb2[6].z
    let instruction_35 = ((r0).zzzz) * ((material.source[1]).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_35.z, r0.w);
    // mul r1.yz, cb2[4].wwww, cb2[12].xxyx
    let instruction_36 = ((scene.world.game_time).wwww) * ((material.source[7]).xxyx);
    r1 = vec4<f32>(r1.x, instruction_36.y, instruction_36.z, r1.w);
    // mul r1.yz, r1.yyzy, cb2[12].zzwz
    let instruction_37 = ((r1).yyzy) * ((material.source[7]).zzwz);
    r1 = vec4<f32>(r1.x, instruction_37.y, instruction_37.z, r1.w);
    // frc r1.yz, r1.yyzy
    let instruction_38 = fract((r1).yyzy);
    r1 = vec4<f32>(r1.x, instruction_38.y, instruction_38.z, r1.w);
    // mad r1.yz, v3.xxyx, cb2[12].zzwz, r1.yyzy
    let instruction_39 = fma((input.v3).xxyx, (material.source[7]).zzwz, (r1).yyzy);
    r1 = vec4<f32>(r1.x, instruction_39.y, instruction_39.z, r1.w);
    // sample r2.xyzw, r1.yzyy, t0.xyzw, s2
    let instruction_40 = textureSample(texture_t0, sampler_t0, ((r1).yzyy).xy);
    r2 = instruction_40;
    // mad r1.yz, r2.xxyx, l(0.000000, 2.000000, 2.000000, 0.000000), l(0.000000, -1.000000, -1.000000, 0.000000)
    let instruction_41 = fma((r2).xxyx, vec4<f32>(0.0, 2.0, 2.0, 0.0), vec4<f32>(0.0, -1.0, -1.0, 0.0));
    r1 = vec4<f32>(r1.x, instruction_41.y, instruction_41.z, r1.w);
    // mad r1.yz, r1.yyzy, cb2[13].xxxx, v3.xxyx
    let instruction_42 = fma((r1).yyzy, (material.source[8]).xxxx, (input.v3).xxyx);
    r1 = vec4<f32>(r1.x, instruction_42.y, instruction_42.z, r1.w);
    // mul r2.xy, cb2[4].wwww, cb2[13].yzyy
    let instruction_43 = ((scene.world.game_time).wwww) * ((material.source[8]).yzyy);
    r2 = vec4<f32>(instruction_43.x, instruction_43.y, r2.z, r2.w);
    // mul r2.xy, r2.xyxx, cb2[14].xyxx
    let instruction_44 = ((r2).xyxx) * ((material.source[9]).xyxx);
    r2 = vec4<f32>(instruction_44.x, instruction_44.y, r2.z, r2.w);
    // frc r2.xy, r2.xyxx
    let instruction_45 = fract((r2).xyxx);
    r2 = vec4<f32>(instruction_45.x, instruction_45.y, r2.z, r2.w);
    // mad r1.yz, r1.yyzy, cb2[14].xxyx, r2.xxyx
    let instruction_46 = fma((r1).yyzy, (material.source[9]).xxyx, (r2).xxyx);
    r1 = vec4<f32>(r1.x, instruction_46.y, instruction_46.z, r1.w);
    // sample r2.xyzw, r1.yzyy, t6.xyzw, s3
    let instruction_47 = textureSample(texture_t6, sampler_t6, ((r1).yzyy).xy);
    r2 = instruction_47;
    // sample r3.xyzw, r1.yzyy, t1.xyzw, s8
    let instruction_48 = textureSample(texture_t1, sampler_t1, ((r1).yzyy).xy);
    r3 = instruction_48;
    // mul r0.z, r0.z, r2.x
    let instruction_49 = ((r0).zzzz) * ((r2).xxxx);
    r0 = vec4<f32>(r0.x, r0.y, instruction_49.z, r0.w);
    // mul r0.z, r0.z, cb2[14].z
    let instruction_50 = ((r0).zzzz) * ((material.source[9]).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_50.z, r0.w);
    // sample r2.xyzw, v3.xyxx, t5.xyzw, s4
    let instruction_51 = textureSample(texture_t5, sampler_t5, ((input.v3).xyxx).xy);
    r2 = instruction_51;
    // mad r1.w, r1.x, r2.x, r0.z
    let instruction_52 = fma((r1).xxxx, (r2).xxxx, (r0).zzzz);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_52.w);
    // mul r0.zw, cb2[4].wwww, cb2[7].xxxy
    let instruction_53 = ((scene.world.game_time).wwww) * ((material.source[2]).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_53.z, instruction_53.w);
    // mul r0.zw, r0.zzzw, cb2[7].zzzw
    let instruction_54 = ((r0).zzzw) * ((material.source[2]).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_54.z, instruction_54.w);
    // frc r0.zw, r0.zzzw
    let instruction_55 = fract((r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_55.z, instruction_55.w);
    // mad r0.xy, r0.xyxx, cb2[7].zwzz, r0.zwzz
    let instruction_56 = fma((r0).xyxx, (material.source[2]).zwzz, (r0).zwzz);
    r0 = vec4<f32>(instruction_56.x, instruction_56.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t3.xyzw, s7
    let instruction_57 = textureSample(texture_t3, sampler_t3, ((r0).xyxx).xy);
    r0 = instruction_57;
    // mad r1.xyz, r3.xyzx, cb2[14].zzzz, r0.xyzx
    let instruction_58 = fma((r3).xyzx, (material.source[9]).zzzz, (r0).xyzx);
    r1 = vec4<f32>(instruction_58.x, instruction_58.y, instruction_58.z, r1.w);
    // mul r0.xyzw, r1.xyzw, v1.xyzw
    let instruction_59 = ((r1).xyzw) * ((input.v1).xyzw);
    r0 = instruction_59;
    // div r1.xy, v5.xyxx, cb0[95].xyxx
    let instruction_60 = ((input.position).xyxx) / ((frame.render_target_size).xyxx);
    r1 = vec4<f32>(instruction_60.x, instruction_60.y, r1.z, r1.w);
    // sample r1.xyzw, r1.xyxx, t7.xyzw, s1
    let instruction_61 = textureSample(float_z, float_z_sampler, ((r1).xyxx).xy);
    r1 = instruction_61;
    // max r1.x, |r1.x|, l(0.000000)
    let instruction_62 = max(abs((r1).xxxx), vec4<f32>(9.99999993922529e-09));
    r1 = vec4<f32>(instruction_62.x, r1.y, r1.z, r1.w);
    // div r1.x, cb0[58].x, r1.x
    let instruction_63 = ((frame.z_near).xxxx) / ((r1).xxxx);
    r1 = vec4<f32>(instruction_63.x, r1.y, r1.z, r1.w);
    // add r1.x, r1.x, -v4.w
    let instruction_64 = ((r1).xxxx) + (-((input.v4).wwww));
    r1 = vec4<f32>(instruction_64.x, r1.y, r1.z, r1.w);
    // max r1.y, cb2[8].x, l(0.000100)
    let instruction_65 = max((material.source[3]).xxxx, vec4<f32>(9.999999747378752e-05));
    r1 = vec4<f32>(r1.x, instruction_65.y, r1.z, r1.w);
    // div r1.y, l(1.000000, 1.000000, 1.000000, 1.000000), r1.y
    let instruction_66 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r1).yyyy);
    r1 = vec4<f32>(r1.x, instruction_66.y, r1.z, r1.w);
    // mul_sat r1.x, r1.y, r1.x
    let instruction_67 = clamp(((r1).yyyy) * ((r1).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(instruction_67.x, r1.y, r1.z, r1.w);
    // mul r1.x, r1.x, v1.w
    let instruction_68 = ((r1).xxxx) * ((input.v1).wwww);
    r1 = vec4<f32>(instruction_68.x, r1.y, r1.z, r1.w);
    // mul r0.w, r0.w, r1.x
    let instruction_69 = ((r0).wwww) * ((r1).xxxx);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_69.w);
    // mul r0.xyzw, r0.xyzw, cb2[11].xyzw
    let instruction_70 = ((r0).xyzw) * ((material.source[6]).xyzw);
    r0 = instruction_70;
    // mul r0.xyz, r0.wwww, r0.xyzx
    let instruction_71 = ((r0).wwww) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_71.x, instruction_71.y, instruction_71.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb2[6].xxxx
    let instruction_72 = ((r0).xyzx) * ((material.source[1]).xxxx);
    r0 = vec4<f32>(instruction_72.x, instruction_72.y, instruction_72.z, r0.w);
    // mul r0.xyz, r0.xyzx, r0.xyzx
    let instruction_73 = ((r0).xyzx) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_73.x, instruction_73.y, instruction_73.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_74 = ((r0).xyzx) * ((scene.world.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_74.x, instruction_74.y, instruction_74.z, r0.w);
    // sqrt o0.xyz, r0.xyzx
    let instruction_75 = sqrt((r0).xyzx);
    o0 = vec4<f32>(instruction_75.x, instruction_75.y, instruction_75.z, o0.w);
    // mov o0.w, l(1.000000)
    let instruction_76 = vec4<f32>(1.0);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_76.w);
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
