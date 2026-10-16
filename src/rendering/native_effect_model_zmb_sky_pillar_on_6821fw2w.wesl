// Original mc_sw4_3d_zmb_sky_pillar_on_6821fw2w; immutable DXBC hashes in effect source-inventory.json.
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
    // dp3 r0.x, v2.xyzx, v2.xyzx
    let instruction_0 = vec4<f32>(dot(((input.v2).xyzx).xyz, ((input.v2).xyzx).xyz));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_1 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, v2.xyzx
    let instruction_2 = ((r0).xxxx) * ((input.v2).xyzx);
    r0 = vec4<f32>(instruction_2.x, instruction_2.y, instruction_2.z, r0.w);
    // dp3 r0.w, v0.xyzx, v0.xyzx
    let instruction_3 = vec4<f32>(dot(((input.v0).xyzx).xyz, ((input.v0).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_3.w);
    // rsq r0.w, r0.w
    let instruction_4 = inverseSqrt((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_4.w);
    // mul r1.xyz, r0.wwww, v0.xyzx
    let instruction_5 = ((r0).wwww) * ((input.v0).xyzx);
    r1 = vec4<f32>(instruction_5.x, instruction_5.y, instruction_5.z, r1.w);
    // dp3 r0.x, -r1.xyzx, r0.xyzx
    let instruction_6 = vec4<f32>(dot((-((r1).xyzx)).xyz, ((r0).xyzx).xyz));
    r0 = vec4<f32>(instruction_6.x, r0.y, r0.z, r0.w);
    // log r0.x, |r0.x|
    let instruction_7 = log2(abs((r0).xxxx));
    r0 = vec4<f32>(instruction_7.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, cb2[8].x
    let instruction_8 = ((r0).xxxx) * ((material.source[3]).xxxx);
    r0 = vec4<f32>(instruction_8.x, r0.y, r0.z, r0.w);
    // exp r0.x, r0.x
    let instruction_9 = exp2((r0).xxxx);
    r0 = vec4<f32>(instruction_9.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, cb2[8].y
    let instruction_10 = ((r0).xxxx) * ((material.source[3]).yyyy);
    r0 = vec4<f32>(instruction_10.x, r0.y, r0.z, r0.w);
    // mul r0.yz, cb2[4].wwww, cb2[9].xxyx
    let instruction_11 = ((scene.world.game_time).wwww) * ((material.source[4]).xxyx);
    r0 = vec4<f32>(r0.x, instruction_11.y, instruction_11.z, r0.w);
    // mul r0.yz, r0.yyzy, cb2[9].zzwz
    let instruction_12 = ((r0).yyzy) * ((material.source[4]).zzwz);
    r0 = vec4<f32>(r0.x, instruction_12.y, instruction_12.z, r0.w);
    // frc r0.yz, r0.yyzy
    let instruction_13 = fract((r0).yyzy);
    r0 = vec4<f32>(r0.x, instruction_13.y, instruction_13.z, r0.w);
    // mad r0.yz, v3.xxyx, cb2[9].zzwz, r0.yyzy
    let instruction_14 = fma((input.v3).xxyx, (material.source[4]).zzwz, (r0).yyzy);
    r0 = vec4<f32>(r0.x, instruction_14.y, instruction_14.z, r0.w);
    // sample r1.xyzw, r0.yzyy, t0.xyzw, s3
    let instruction_15 = textureSample(texture_t0, sampler_t0, ((r0).yzyy).xy);
    r1 = instruction_15;
    // mad r0.yz, r1.xxyx, l(0.000000, 2.000000, 2.000000, 0.000000), l(0.000000, -1.000000, -1.000000, 0.000000)
    let instruction_16 = fma((r1).xxyx, vec4<f32>(0.0, 2.0, 2.0, 0.0), vec4<f32>(0.0, -1.0, -1.0, 0.0));
    r0 = vec4<f32>(r0.x, instruction_16.y, instruction_16.z, r0.w);
    // mad r0.yz, r0.yyzy, cb2[8].wwww, v3.xxyx
    let instruction_17 = fma((r0).yyzy, (material.source[3]).wwww, (input.v3).xxyx);
    r0 = vec4<f32>(r0.x, instruction_17.y, instruction_17.z, r0.w);
    // mul r1.xy, cb2[4].wwww, cb2[5].zwzz
    let instruction_18 = ((scene.world.game_time).wwww) * ((material.source[0]).zwzz);
    r1 = vec4<f32>(instruction_18.x, instruction_18.y, r1.z, r1.w);
    // mul r1.xy, r1.xyxx, cb2[5].xyxx
    let instruction_19 = ((r1).xyxx) * ((material.source[0]).xyxx);
    r1 = vec4<f32>(instruction_19.x, instruction_19.y, r1.z, r1.w);
    // frc r1.xy, r1.xyxx
    let instruction_20 = fract((r1).xyxx);
    r1 = vec4<f32>(instruction_20.x, instruction_20.y, r1.z, r1.w);
    // mad r0.yz, r0.yyzy, cb2[5].xxyx, r1.xxyx
    let instruction_21 = fma((r0).yyzy, (material.source[0]).xxyx, (r1).xxyx);
    r0 = vec4<f32>(r0.x, instruction_21.y, instruction_21.z, r0.w);
    // sample r1.xyzw, r0.yzyy, t2.xyzw, s4
    let instruction_22 = textureSample(texture_t2, sampler_t2, ((r0).yzyy).xy);
    r1 = instruction_22;
    // sample r2.xyzw, r0.yzyy, t1.xyzw, s5
    let instruction_23 = textureSample(texture_t1, sampler_t1, ((r0).yzyy).xy);
    r2 = instruction_23;
    // mul r0.x, r0.x, r1.x
    let instruction_24 = ((r0).xxxx) * ((r1).xxxx);
    r0 = vec4<f32>(instruction_24.x, r0.y, r0.z, r0.w);
    // add r0.yz, v3.xxyx, cb1[23].xxyx
    let instruction_25 = ((input.v3).xxyx) + ((scripts.vectors[0]).xxyx);
    r0 = vec4<f32>(r0.x, instruction_25.y, instruction_25.z, r0.w);
    // mul r0.yz, r0.yyzy, cb2[6].yyzy
    let instruction_26 = ((r0).yyzy) * ((material.source[1]).yyzy);
    r0 = vec4<f32>(r0.x, instruction_26.y, instruction_26.z, r0.w);
    // sample r1.xyzw, r0.yzyy, t3.xyzw, s2
    let instruction_27 = textureSample(texture_t3, sampler_t3, ((r0).yzyy).xy);
    r1 = instruction_27;
    // mul r0.x, r0.x, r1.x
    let instruction_28 = ((r0).xxxx) * ((r1).xxxx);
    r0 = vec4<f32>(instruction_28.x, r0.y, r0.z, r0.w);
    // dp2 r0.y, v0.xyxx, v0.xyxx
    let instruction_29 = vec4<f32>(dot(((input.v0).xyxx).xy, ((input.v0).xyxx).xy));
    r0 = vec4<f32>(r0.x, instruction_29.y, r0.z, r0.w);
    // sqrt r0.y, r0.y
    let instruction_30 = sqrt((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_30.y, r0.z, r0.w);
    // add r0.y, r0.y, -cb2[10].y
    let instruction_31 = ((r0).yyyy) + (-((material.source[5]).yyyy));
    r0 = vec4<f32>(r0.x, instruction_31.y, r0.z, r0.w);
    // add r0.z, -cb2[10].y, cb2[10].z
    let instruction_32 = (-((material.source[5]).yyyy)) + ((material.source[5]).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_32.z, r0.w);
    // div r0.z, l(1.000000, 1.000000, 1.000000, 1.000000), r0.z
    let instruction_33 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_33.z, r0.w);
    // mul_sat r0.y, r0.z, r0.y
    let instruction_34 = clamp(((r0).zzzz) * ((r0).yyyy), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_34.y, r0.z, r0.w);
    // mad r0.z, r0.y, l(-2.000000), l(3.000000)
    let instruction_35 = fma((r0).yyyy, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_35.z, r0.w);
    // mul r0.y, r0.y, r0.y
    let instruction_36 = ((r0).yyyy) * ((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_36.y, r0.z, r0.w);
    // mul r0.y, r0.y, r0.z
    let instruction_37 = ((r0).yyyy) * ((r0).zzzz);
    r0 = vec4<f32>(r0.x, instruction_37.y, r0.z, r0.w);
    // mul r0.y, r0.y, cb2[10].x
    let instruction_38 = ((r0).yyyy) * ((material.source[5]).xxxx);
    r0 = vec4<f32>(r0.x, instruction_38.y, r0.z, r0.w);
    // mul r0.x, r0.y, r0.x
    let instruction_39 = ((r0).yyyy) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_39.x, r0.y, r0.z, r0.w);
    // div r0.yz, v5.xxyx, cb0[95].xxyx
    let instruction_40 = ((input.position).xxyx) / ((frame.render_target_size).xxyx);
    r0 = vec4<f32>(r0.x, instruction_40.y, instruction_40.z, r0.w);
    // sample r1.xyzw, r0.yzyy, t4.xyzw, s1
    let instruction_41 = textureSample(float_z, float_z_sampler, ((r0).yzyy).xy);
    r1 = instruction_41;
    // max r0.y, |r1.x|, l(0.000000)
    let instruction_42 = max(abs((r1).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(r0.x, instruction_42.y, r0.z, r0.w);
    // div r0.y, cb0[58].x, r0.y
    let instruction_43 = ((frame.z_near).xxxx) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_43.y, r0.z, r0.w);
    // add r0.y, r0.y, -v4.w
    let instruction_44 = ((r0).yyyy) + (-((input.v4).wwww));
    r0 = vec4<f32>(r0.x, instruction_44.y, r0.z, r0.w);
    // max r0.z, cb2[8].z, l(0.000100)
    let instruction_45 = max((material.source[3]).zzzz, vec4<f32>(9.999999747378752e-05));
    r0 = vec4<f32>(r0.x, r0.y, instruction_45.z, r0.w);
    // div r0.z, l(1.000000, 1.000000, 1.000000, 1.000000), r0.z
    let instruction_46 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_46.z, r0.w);
    // mul_sat r0.y, r0.z, r0.y
    let instruction_47 = clamp(((r0).zzzz) * ((r0).yyyy), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, instruction_47.y, r0.z, r0.w);
    // mul r0.y, r0.y, v1.w
    let instruction_48 = ((r0).yyyy) * ((input.v1).wwww);
    r0 = vec4<f32>(r0.x, instruction_48.y, r0.z, r0.w);
    // mul r2.w, r0.y, r0.x
    let instruction_49 = ((r0).yyyy) * ((r0).xxxx);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_49.w);
    // mul r0.xyzw, r2.xyzw, cb2[7].xyzw
    let instruction_50 = ((r2).xyzw) * ((material.source[2]).xyzw);
    r0 = instruction_50;
    // mul r0.xyz, r0.wwww, r0.xyzx
    let instruction_51 = ((r0).wwww) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_51.x, instruction_51.y, instruction_51.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb2[6].xxxx
    let instruction_52 = ((r0).xyzx) * ((material.source[1]).xxxx);
    r0 = vec4<f32>(instruction_52.x, instruction_52.y, instruction_52.z, r0.w);
    // mul r0.xyz, r0.xyzx, r0.xyzx
    let instruction_53 = ((r0).xyzx) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_53.x, instruction_53.y, instruction_53.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_54 = ((r0).xyzx) * ((scene.world.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_54.x, instruction_54.y, instruction_54.z, r0.w);
    // sqrt o0.xyz, r0.xyzx
    let instruction_55 = sqrt((r0).xyzx);
    o0 = vec4<f32>(instruction_55.x, instruction_55.y, instruction_55.z, o0.w);
    // mov o0.w, l(1.000000)
    let instruction_56 = vec4<f32>(1.0);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_56.w);
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
