// Original mc_sw4_3d_scrolling_alpha2_distort_qwj8139f; immutable DXBC hashes in effect source-inventory.json.
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
    @location(5) v5: vec4<f32>,
}
@vertex
fn vertex(input: Input) -> Varyings {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var o4: vec4<f32>;
    var o5: vec4<f32>;
    var o6: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // add r0.xyzw, -cb0[26].xyzw, cb0[30].xyzw
    let instruction_0 = (-((scene.world.fog_color).xyzw)) + ((scene.world.sun_fog_color).xyzw);
    r0 = instruction_0;
    // mov r1.xyz, v0.xyzx
    let instruction_1 = (input.position).xyzx;
    r1 = vec4<f32>(instruction_1.x, instruction_1.y, instruction_1.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_2 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_2.w);
    // dp4 r2.x, r1.xyzw, cb3[0].xyzw
    let instruction_3 = vec4<f32>(dot(((r1).xyzw).xyzw, ((object.world_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(instruction_3.x, r2.y, r2.z, r2.w);
    // dp4 r2.y, r1.xyzw, cb3[1].xyzw
    let instruction_4 = vec4<f32>(dot(((r1).xyzw).xyzw, ((object.world_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, instruction_4.y, r2.z, r2.w);
    // dp4 r2.z, r1.xyzw, cb3[2].xyzw
    let instruction_5 = vec4<f32>(dot(((r1).xyzw).xyzw, ((object.world_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_5.z, r2.w);
    // dp4 r2.w, r1.xyzw, cb3[3].xyzw
    let instruction_6 = vec4<f32>(dot(((r1).xyzw).xyzw, ((object.world_rows[3]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_6.w);
    // dp3 r1.x, r2.xyzx, r2.xyzx
    let instruction_7 = vec4<f32>(dot(((r2).xyzx).xyz, ((r2).xyzx).xyz));
    r1 = vec4<f32>(instruction_7.x, r1.y, r1.z, r1.w);
    // rsq r1.y, r1.x
    let instruction_8 = inverseSqrt((r1).xxxx);
    r1 = vec4<f32>(r1.x, instruction_8.y, r1.z, r1.w);
    // sqrt r1.x, r1.x
    let instruction_9 = sqrt((r1).xxxx);
    r1 = vec4<f32>(instruction_9.x, r1.y, r1.z, r1.w);
    // mul r1.yzw, r1.yyyy, r2.xxyz
    let instruction_10 = ((r1).yyyy) * ((r2).xxyz);
    r1 = vec4<f32>(r1.x, instruction_10.y, instruction_10.z, instruction_10.w);
    // dp3 r1.y, cb0[29].xyzx, r1.yzwy
    let instruction_11 = vec4<f32>(dot(((scene.world.sun_fog_dir).xyzx).xyz, ((r1).yzwy).xyz));
    r1 = vec4<f32>(r1.x, instruction_11.y, r1.z, r1.w);
    // mad_sat r1.y, cb0[31].y, r1.y, cb0[31].x
    let instruction_12 = clamp(fma((scene.world.sun_fog).yyyy, (r1).yyyy, (scene.world.sun_fog).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, instruction_12.y, r1.z, r1.w);
    // mad r0.xyzw, r1.yyyy, r0.xyzw, cb0[26].xyzw
    let instruction_13 = fma((r1).yyyy, (r0).xyzw, (scene.world.fog_color).xyzw);
    r0 = instruction_13;
    // mov o0.xyz, r0.xyzx
    let instruction_14 = (r0).xyzx;
    o0 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, o0.w);
    // mad r0.x, cb0[27].w, r2.z, cb0[27].x
    let instruction_15 = fma((scene.world.fog_consts).wwww, (r2).zzzz, (scene.world.fog_consts).xxxx);
    r0 = vec4<f32>(instruction_15.x, r0.y, r0.z, r0.w);
    // min r0.y, r0.x, l(64.000000)
    let instruction_16 = min((r0).xxxx, vec4<f32>(64.0));
    r0 = vec4<f32>(r0.x, instruction_16.y, r0.z, r0.w);
    // mul r0.y, r0.y, l(1.442695)
    let instruction_17 = ((r0).yyyy) * (vec4<f32>(1.4426950216293335));
    r0 = vec4<f32>(r0.x, instruction_17.y, r0.z, r0.w);
    // exp r0.y, r0.y
    let instruction_18 = exp2((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_18.y, r0.z, r0.w);
    // lt r0.z, r0.x, l(0.000000)
    let instruction_19 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xxxx) < (vec4<f32>(0.0))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_19.z, r0.w);
    // add r0.x, r0.x, l(1.000000)
    let instruction_20 = ((r0).xxxx) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_20.x, r0.y, r0.z, r0.w);
    // movc r0.x, r0.z, r0.y, r0.x
    let instruction_21 = select((r0).xxxx, (r0).yyyy, bitcast<vec4<u32>>((r0).zzzz) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_21.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -cb0[28].x
    let instruction_22 = ((r0).xxxx) + (-((scene.world.fog_consts2).xxxx));
    r0 = vec4<f32>(instruction_22.x, r0.y, r0.z, r0.w);
    // mul r0.y, r2.z, cb0[27].w
    let instruction_23 = ((r2).zzzz) * ((scene.world.fog_consts).wwww);
    r0 = vec4<f32>(r0.x, instruction_23.y, r0.z, r0.w);
    // lt r0.z, |r0.y|, l(0.000100)
    let instruction_24 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (abs((r0).yyyy)) < (vec4<f32>(9.999999747378752e-05))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_24.z, r0.w);
    // movc r0.y, r0.z, l(1.000000), r0.y
    let instruction_25 = select((r0).yyyy, vec4<f32>(1.0), bitcast<vec4<u32>>((r0).zzzz) != vec4<u32>(0u));
    r0 = vec4<f32>(r0.x, instruction_25.y, r0.z, r0.w);
    // div r0.x, r0.x, r0.y
    let instruction_26 = ((r0).xxxx) / ((r0).yyyy);
    r0 = vec4<f32>(instruction_26.x, r0.y, r0.z, r0.w);
    // mov_sat r0.y, cb0[28].x
    let instruction_27 = clamp((scene.world.fog_consts2).xxxx, vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_27.y, r0.z, r0.w);
    // movc r0.x, r0.z, r0.y, r0.x
    let instruction_28 = select((r0).xxxx, (r0).yyyy, bitcast<vec4<u32>>((r0).zzzz) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_28.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, cb0[27].y
    let instruction_29 = ((r0).xxxx) * ((scene.world.fog_consts).yyyy);
    r0 = vec4<f32>(instruction_29.x, r0.y, r0.z, r0.w);
    // mad r0.x, r0.x, r1.x, cb0[27].z
    let instruction_30 = fma((r0).xxxx, (r1).xxxx, (scene.world.fog_consts).zzzz);
    r0 = vec4<f32>(instruction_30.x, r0.y, r0.z, r0.w);
    // exp r0.x, r0.x
    let instruction_31 = exp2((r0).xxxx);
    r0 = vec4<f32>(instruction_31.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(1.000000)
    let instruction_32 = min((r0).xxxx, vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_32.x, r0.y, r0.z, r0.w);
    // add r0.x, -r0.x, l(1.000000)
    let instruction_33 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_33.x, r0.y, r0.z, r0.w);
    // mad o1.w, -r0.x, r0.w, l(1.000000)
    let instruction_34 = fma(-((r0).xxxx), (r0).wwww, vec4<f32>(1.0));
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_34.w);
    // mov o1.xyz, r2.xyzx
    let instruction_35 = (r2).xyzx;
    o1 = vec4<f32>(instruction_35.x, instruction_35.y, instruction_35.z, o1.w);
    // mov o2.xyzw, v1.xyzw
    let instruction_36 = (input.color).xyzw;
    o2 = instruction_36;
    // add r0.xyz, v3.xyzx, v3.xyzx
    let instruction_37 = ((input.normal).xyzx) + ((input.normal).xyzx);
    r0 = vec4<f32>(instruction_37.x, instruction_37.y, instruction_37.z, r0.w);
    // ge r1.xyz, v3.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_38 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.normal).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r1 = vec4<f32>(instruction_38.x, instruction_38.y, instruction_38.z, r1.w);
    // mad r3.xyz, v3.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_39 = fma((input.normal).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r3 = vec4<f32>(instruction_39.x, instruction_39.y, instruction_39.z, r3.w);
    // movc r0.xyz, r1.xyzx, r3.xyzx, r0.xyzx
    let instruction_40 = select((r0).xyzx, (r3).xyzx, bitcast<vec4<u32>>((r1).xyzx) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_40.x, instruction_40.y, instruction_40.z, r0.w);
    // dp3 r1.x, r0.xyzx, cb3[0].xyzx
    let instruction_41 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[0]).xyzx).xyz));
    r1 = vec4<f32>(instruction_41.x, r1.y, r1.z, r1.w);
    // dp3 r1.y, r0.xyzx, cb3[1].xyzx
    let instruction_42 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[1]).xyzx).xyz));
    r1 = vec4<f32>(r1.x, instruction_42.y, r1.z, r1.w);
    // dp3 r1.z, r0.xyzx, cb3[2].xyzx
    let instruction_43 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[2]).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, instruction_43.z, r1.w);
    // dp3 r0.x, r1.xyzx, r1.xyzx
    let instruction_44 = vec4<f32>(dot(((r1).xyzx).xyz, ((r1).xyzx).xyz));
    r0 = vec4<f32>(instruction_44.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_45 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_45.x, r0.y, r0.z, r0.w);
    // mul o3.xyz, r0.xxxx, r1.xyzx
    let instruction_46 = ((r0).xxxx) * ((r1).xyzx);
    o3 = vec4<f32>(instruction_46.x, instruction_46.y, instruction_46.z, o3.w);
    // mov o3.w, l(0)
    let instruction_47 = vec4<f32>(0.0);
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_47.w);
    // mov o4.xy, v2.xyxx
    let instruction_48 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o4 = vec4<f32>(instruction_48.x, instruction_48.y, o4.z, o4.w);
    // dp4 r0.x, r2.xyzw, cb0[36].xyzw
    let instruction_49 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[0]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_49.x, r0.y, r0.z, r0.w);
    // dp4 r0.y, r2.xyzw, cb0[37].xyzw
    let instruction_50 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[1]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_50.y, r0.z, r0.w);
    // dp4 r0.z, r2.xyzw, cb0[38].xyzw
    let instruction_51 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[2]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, r0.y, instruction_51.z, r0.w);
    // dp4 r0.w, r2.xyzw, cb0[39].xyzw
    let instruction_52 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_52.w);
    // mov o5.xyzw, r0.xyzw
    let instruction_53 = (r0).xyzw;
    o5 = instruction_53;
    // mov o6.xyzw, r0.xyzw
    let instruction_54 = (r0).xyzw;
    o6 = instruction_54;
    var result: Varyings;
    result.position = o6;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.v4 = o4;
    result.v5 = o5;
    return result;
}
fn shade(input: Varyings) -> vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    // mul r0.xy, cb2[4].wwww, cb2[8].zwzz
    let instruction_0 = ((scene.world.game_time).wwww) * ((material.source[3]).zwzz);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // mul r0.xy, r0.xyxx, cb2[8].xyxx
    let instruction_1 = ((r0).xyxx) * ((material.source[3]).xyxx);
    r0 = vec4<f32>(instruction_1.x, instruction_1.y, r0.z, r0.w);
    // frc r0.xy, r0.xyxx
    let instruction_2 = fract((r0).xyxx);
    r0 = vec4<f32>(instruction_2.x, instruction_2.y, r0.z, r0.w);
    // mad r0.xy, v4.xyxx, cb2[8].xyxx, r0.xyxx
    let instruction_3 = fma((input.v4).xyxx, (material.source[3]).xyxx, (r0).xyxx);
    r0 = vec4<f32>(instruction_3.x, instruction_3.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t0.xyzw, s3
    let instruction_4 = textureSample(texture_t0, sampler_t0, ((r0).xyxx).xy);
    r0 = instruction_4;
    // mad r0.xy, r0.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_5 = fma((r0).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r0 = vec4<f32>(instruction_5.x, instruction_5.y, r0.z, r0.w);
    // mad r0.xy, r0.xyxx, cb2[7].wwww, v4.xyxx
    let instruction_6 = fma((r0).xyxx, (material.source[2]).wwww, (input.v4).xyxx);
    r0 = vec4<f32>(instruction_6.x, instruction_6.y, r0.z, r0.w);
    // mul r0.zw, cb2[4].wwww, cb2[6].zzzw
    let instruction_7 = ((scene.world.game_time).wwww) * ((material.source[1]).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_7.z, instruction_7.w);
    // mul r0.zw, r0.zzzw, cb2[6].xxxy
    let instruction_8 = ((r0).zzzw) * ((material.source[1]).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_8.z, instruction_8.w);
    // frc r0.zw, r0.zzzw
    let instruction_9 = fract((r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_9.z, instruction_9.w);
    // mad r0.xy, r0.xyxx, cb2[6].xyxx, r0.zwzz
    let instruction_10 = fma((r0).xyxx, (material.source[1]).xyxx, (r0).zwzz);
    r0 = vec4<f32>(instruction_10.x, instruction_10.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t0.xyzw, s3
    let instruction_11 = textureSample(texture_t0, sampler_t0, ((r0).xyxx).xy);
    r0 = instruction_11;
    // dp3 r1.x, v3.xyzx, v3.xyzx
    let instruction_12 = vec4<f32>(dot(((input.v3).xyzx).xyz, ((input.v3).xyzx).xyz));
    r1 = vec4<f32>(instruction_12.x, r1.y, r1.z, r1.w);
    // rsq r1.x, r1.x
    let instruction_13 = inverseSqrt((r1).xxxx);
    r1 = vec4<f32>(instruction_13.x, r1.y, r1.z, r1.w);
    // mul r1.xyz, r1.xxxx, v3.xyzx
    let instruction_14 = ((r1).xxxx) * ((input.v3).xyzx);
    r1 = vec4<f32>(instruction_14.x, instruction_14.y, instruction_14.z, r1.w);
    // dp3 r1.w, v1.xyzx, v1.xyzx
    let instruction_15 = vec4<f32>(dot(((input.v1).xyzx).xyz, ((input.v1).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_15.w);
    // rsq r2.x, r1.w
    let instruction_16 = inverseSqrt((r1).wwww);
    r2 = vec4<f32>(instruction_16.x, r2.y, r2.z, r2.w);
    // sqrt r1.w, r1.w
    let instruction_17 = sqrt((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_17.w);
    // div_sat r1.w, r1.w, cb2[9].x
    let instruction_18 = clamp(((r1).wwww) / ((material.source[4]).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_18.w);
    // mul r2.xyz, r2.xxxx, v1.xyzx
    let instruction_19 = ((r2).xxxx) * ((input.v1).xyzx);
    r2 = vec4<f32>(instruction_19.x, instruction_19.y, instruction_19.z, r2.w);
    // dp3_sat r1.x, -r2.xyzx, r1.xyzx
    let instruction_20 = clamp(vec4<f32>(dot((-((r2).xyzx)).xyz, ((r1).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(instruction_20.x, r1.y, r1.z, r1.w);
    // log r1.x, r1.x
    let instruction_21 = log2((r1).xxxx);
    r1 = vec4<f32>(instruction_21.x, r1.y, r1.z, r1.w);
    // mul r1.x, r1.x, cb2[7].y
    let instruction_22 = ((r1).xxxx) * ((material.source[2]).yyyy);
    r1 = vec4<f32>(instruction_22.x, r1.y, r1.z, r1.w);
    // exp r1.x, r1.x
    let instruction_23 = exp2((r1).xxxx);
    r1 = vec4<f32>(instruction_23.x, r1.y, r1.z, r1.w);
    // mul r1.x, r1.x, cb2[7].z
    let instruction_24 = ((r1).xxxx) * ((material.source[2]).zzzz);
    r1 = vec4<f32>(instruction_24.x, r1.y, r1.z, r1.w);
    // mul r1.yz, cb2[4].wwww, cb2[5].zzwz
    let instruction_25 = ((scene.world.game_time).wwww) * ((material.source[0]).zzwz);
    r1 = vec4<f32>(r1.x, instruction_25.y, instruction_25.z, r1.w);
    // mul r1.yz, r1.yyzy, cb2[5].xxyx
    let instruction_26 = ((r1).yyzy) * ((material.source[0]).xxyx);
    r1 = vec4<f32>(r1.x, instruction_26.y, instruction_26.z, r1.w);
    // frc r1.yz, r1.yyzy
    let instruction_27 = fract((r1).yyzy);
    r1 = vec4<f32>(r1.x, instruction_27.y, instruction_27.z, r1.w);
    // mad r1.yz, v4.xxyx, cb2[5].xxyx, r1.yyzy
    let instruction_28 = fma((input.v4).xxyx, (material.source[0]).xxyx, (r1).yyzy);
    r1 = vec4<f32>(r1.x, instruction_28.y, instruction_28.z, r1.w);
    // sample r2.xyzw, r1.yzyy, t1.xyzw, s2
    let instruction_29 = textureSample(texture_t1, sampler_t1, ((r1).yzyy).xy);
    r2 = instruction_29;
    // mul r0.w, r1.x, r2.y
    let instruction_30 = ((r1).xxxx) * ((r2).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_30.w);
    // mul r0.xyzw, r0.xyzw, v2.xyzw
    let instruction_31 = ((r0).xyzw) * ((input.v2).xyzw);
    r0 = instruction_31;
    // div r1.xy, v6.xyxx, cb0[95].xyxx
    let instruction_32 = ((input.position).xyxx) / ((frame.render_target_size).xyxx);
    r1 = vec4<f32>(instruction_32.x, instruction_32.y, r1.z, r1.w);
    // sample r2.xyzw, r1.xyxx, t2.xyzw, s1
    let instruction_33 = textureSample(float_z, float_z_sampler, ((r1).xyxx).xy);
    r2 = instruction_33;
    // max r1.x, |r2.x|, l(0.000000)
    let instruction_34 = max(abs((r2).xxxx), vec4<f32>(9.99999993922529e-09));
    r1 = vec4<f32>(instruction_34.x, r1.y, r1.z, r1.w);
    // div r1.x, cb0[58].x, r1.x
    let instruction_35 = ((frame.z_near).xxxx) / ((r1).xxxx);
    r1 = vec4<f32>(instruction_35.x, r1.y, r1.z, r1.w);
    // add r1.x, r1.x, -v5.w
    let instruction_36 = ((r1).xxxx) + (-((input.v5).wwww));
    r1 = vec4<f32>(instruction_36.x, r1.y, r1.z, r1.w);
    // max r1.y, cb2[9].y, l(0.000100)
    let instruction_37 = max((material.source[4]).yyyy, vec4<f32>(9.999999747378752e-05));
    r1 = vec4<f32>(r1.x, instruction_37.y, r1.z, r1.w);
    // div r1.y, l(1.000000, 1.000000, 1.000000, 1.000000), r1.y
    let instruction_38 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r1).yyyy);
    r1 = vec4<f32>(r1.x, instruction_38.y, r1.z, r1.w);
    // min r1.y, r1.y, l(1.000000)
    let instruction_39 = min((r1).yyyy, vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, instruction_39.y, r1.z, r1.w);
    // mul_sat r1.x, r1.y, r1.x
    let instruction_40 = clamp(((r1).yyyy) * ((r1).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(instruction_40.x, r1.y, r1.z, r1.w);
    // add r1.y, -r1.x, l(1.000000)
    let instruction_41 = (-((r1).xxxx)) + (vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, instruction_41.y, r1.z, r1.w);
    // mad r1.x, r1.w, r1.y, r1.x
    let instruction_42 = fma((r1).wwww, (r1).yyyy, (r1).xxxx);
    r1 = vec4<f32>(instruction_42.x, r1.y, r1.z, r1.w);
    // mul r0.w, r0.w, r1.x
    let instruction_43 = ((r0).wwww) * ((r1).xxxx);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_43.w);
    // mul r0.xyz, r0.xyzx, cb2[7].xxxx
    let instruction_44 = ((r0).xyzx) * ((material.source[2]).xxxx);
    r0 = vec4<f32>(instruction_44.x, instruction_44.y, instruction_44.z, r0.w);
    // mad r0.xyz, r0.xyzx, r0.xyzx, -v0.xyzx
    let instruction_45 = fma((r0).xyzx, (r0).xyzx, -((input.v0).xyzx));
    r0 = vec4<f32>(instruction_45.x, instruction_45.y, instruction_45.z, r0.w);
    // mad r0.xyz, v1.wwww, r0.xyzx, v0.xyzx
    let instruction_46 = fma((input.v1).wwww, (r0).xyzx, (input.v0).xyzx);
    r0 = vec4<f32>(instruction_46.x, instruction_46.y, instruction_46.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_47 = ((r0).xyzx) * ((scene.world.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_47.x, instruction_47.y, instruction_47.z, r0.w);
    // sqrt o0.xyzw, r0.xyzw
    let instruction_48 = sqrt((r0).xyzw);
    o0 = instruction_48;
    if !(o0.a > 0.0) { discard; }
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
