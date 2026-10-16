// Original mc_sw4_3d_zm_sphere_elec_535e0f02; immutable DXBC hashes in effect source-inventory.json.
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
@group(3) @binding(0) var<uniform> frame: NativeFxFrame;
@group(3) @binding(1) var float_z: texture_2d<f32>;
@group(3) @binding(2) var float_z_sampler: sampler;
@group(2) @binding(28) var reflection_probe: texture_cube<f32>;
@group(2) @binding(29) var reflection_sampler: sampler;
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(4) v4: vec4<f32>,
    @location(5) v5: vec4<f32>,
    @location(6) v6: vec4<f32>,
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
    var o7: vec4<f32>;
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
    // mul r0.xyz, r0.xxxx, r2.xyzx
    let instruction_17 = ((r0).xxxx) * ((r2).xyzx);
    r0 = vec4<f32>(instruction_17.x, instruction_17.y, instruction_17.z, r0.w);
    // mov o2.xyz, r0.xyzx
    let instruction_18 = (r0).xyzx;
    o2 = vec4<f32>(instruction_18.x, instruction_18.y, instruction_18.z, o2.w);
    // mov o2.w, l(0)
    let instruction_19 = vec4<f32>(0.0);
    o2 = vec4<f32>(o2.x, o2.y, o2.z, instruction_19.w);
    // add r2.xyz, v4.xyzx, v4.xyzx
    let instruction_20 = ((input.tangent).xyzx) + ((input.tangent).xyzx);
    r2 = vec4<f32>(instruction_20.x, instruction_20.y, instruction_20.z, r2.w);
    // ge r3.xyz, v4.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_21 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.tangent).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r3 = vec4<f32>(instruction_21.x, instruction_21.y, instruction_21.z, r3.w);
    // mad r4.xyz, v4.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_22 = fma((input.tangent).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r4 = vec4<f32>(instruction_22.x, instruction_22.y, instruction_22.z, r4.w);
    // movc r2.xyz, r3.xyzx, r4.xyzx, r2.xyzx
    let instruction_23 = select((r2).xyzx, (r4).xyzx, bitcast<vec4<u32>>((r3).xyzx) != vec4<u32>(0u));
    r2 = vec4<f32>(instruction_23.x, instruction_23.y, instruction_23.z, r2.w);
    // dp3 r3.x, r2.xyzx, cb3[0].xyzx
    let instruction_24 = vec4<f32>(dot(((r2).xyzx).xyz, ((object.world_rows[0]).xyzx).xyz));
    r3 = vec4<f32>(instruction_24.x, r3.y, r3.z, r3.w);
    // dp3 r3.y, r2.xyzx, cb3[1].xyzx
    let instruction_25 = vec4<f32>(dot(((r2).xyzx).xyz, ((object.world_rows[1]).xyzx).xyz));
    r3 = vec4<f32>(r3.x, instruction_25.y, r3.z, r3.w);
    // dp3 r3.z, r2.xyzx, cb3[2].xyzx
    let instruction_26 = vec4<f32>(dot(((r2).xyzx).xyz, ((object.world_rows[2]).xyzx).xyz));
    r3 = vec4<f32>(r3.x, r3.y, instruction_26.z, r3.w);
    // dp3 r2.x, r3.xyzx, r3.xyzx
    let instruction_27 = vec4<f32>(dot(((r3).xyzx).xyz, ((r3).xyzx).xyz));
    r2 = vec4<f32>(instruction_27.x, r2.y, r2.z, r2.w);
    // rsq r2.x, r2.x
    let instruction_28 = inverseSqrt((r2).xxxx);
    r2 = vec4<f32>(instruction_28.x, r2.y, r2.z, r2.w);
    // mul o3.xyz, r2.xxxx, r3.xyzx
    let instruction_29 = ((r2).xxxx) * ((r3).xyzx);
    o3 = vec4<f32>(instruction_29.x, instruction_29.y, instruction_29.z, o3.w);
    // lt r2.x, l(0.000000), v0.w
    let instruction_30 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0)) < ((input.position).wwww)));
    r2 = vec4<f32>(instruction_30.x, r2.y, r2.z, r2.w);
    // lt r2.y, v0.w, l(0.000000)
    let instruction_31 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.position).wwww) < (vec4<f32>(0.0))));
    r2 = vec4<f32>(r2.x, instruction_31.y, r2.z, r2.w);
    // iadd r2.x, r2.y, -r2.x
    let instruction_32 = bitcast<vec4<f32>>(bitcast<vec4<i32>>((r2).yyyy) + -(bitcast<vec4<i32>>((r2).xxxx)));
    r2 = vec4<f32>(instruction_32.x, r2.y, r2.z, r2.w);
    // itof r2.x, r2.x
    let instruction_33 = vec4<f32>(bitcast<vec4<i32>>((r2).xxxx));
    r2 = vec4<f32>(instruction_33.x, r2.y, r2.z, r2.w);
    // lt r2.y, l(0.000000), r2.x
    let instruction_34 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0)) < ((r2).xxxx)));
    r2 = vec4<f32>(r2.x, instruction_34.y, r2.z, r2.w);
    // lt r2.x, r2.x, l(0.000000)
    let instruction_35 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r2).xxxx) < (vec4<f32>(0.0))));
    r2 = vec4<f32>(instruction_35.x, r2.y, r2.z, r2.w);
    // iadd r2.x, r2.x, -r2.y
    let instruction_36 = bitcast<vec4<f32>>(bitcast<vec4<i32>>((r2).xxxx) + -(bitcast<vec4<i32>>((r2).yyyy)));
    r2 = vec4<f32>(instruction_36.x, r2.y, r2.z, r2.w);
    // itof o3.w, r2.x
    let instruction_37 = vec4<f32>(bitcast<vec4<i32>>((r2).xxxx));
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_37.w);
    // mov o4.xy, v2.xyxx
    let instruction_38 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o4 = vec4<f32>(instruction_38.x, instruction_38.y, o4.z, o4.w);
    // mul r2.xy, r0.yzyy, r0.yzyy
    let instruction_39 = ((r0).yzyy) * ((r0).yzyy);
    r2 = vec4<f32>(instruction_39.x, instruction_39.y, r2.z, r2.w);
    // mad r3.w, r0.x, r0.x, -r2.x
    let instruction_40 = fma((r0).xxxx, (r0).xxxx, -((r2).xxxx));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_40.w);
    // mul r3.xyz, r0.zyxz, r0.xzyx
    let instruction_41 = ((r0).zyxz) * ((r0).xzyx);
    r3 = vec4<f32>(instruction_41.x, instruction_41.y, instruction_41.z, r3.w);
    // dp4 r2.x, cb3[7].xyzw, r3.xyzw
    let instruction_42 = vec4<f32>(dot(((object.grid_lighting_sh2).xyzw).xyzw, ((r3).xyzw).xyzw));
    r2 = vec4<f32>(instruction_42.x, r2.y, r2.z, r2.w);
    // dp4 r2.z, cb3[10].xyzw, r3.xyzw
    let instruction_43 = vec4<f32>(dot(((object.reflection_lighting_sh2).xyzw).xyzw, ((r3).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_43.z, r2.w);
    // mov r0.w, l(1.000000)
    let instruction_44 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_44.w);
    // dp4 r2.w, cb3[6].xyzw, r0.xyzw
    let instruction_45 = vec4<f32>(dot(((object.grid_lighting_sh1).xyzw).xyzw, ((r0).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_45.w);
    // dp4 r0.x, cb3[9].xyzw, r0.xyzw
    let instruction_46 = vec4<f32>(dot(((object.reflection_lighting_sh1).xyzw).xyzw, ((r0).xyzw).xyzw));
    r0 = vec4<f32>(instruction_46.x, r0.y, r0.z, r0.w);
    // add r0.x, r2.z, r0.x
    let instruction_47 = ((r2).zzzz) + ((r0).xxxx);
    r0 = vec4<f32>(instruction_47.x, r0.y, r0.z, r0.w);
    // mad r0.x, cb3[8].w, r2.y, r0.x
    let instruction_48 = fma((object.reflection_lighting_sh0).wwww, (r2).yyyy, (r0).xxxx);
    r0 = vec4<f32>(instruction_48.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, cb3[8].xyzx
    let instruction_49 = ((r0).xxxx) * ((object.reflection_lighting_sh0).xyzx);
    r0 = vec4<f32>(instruction_49.x, instruction_49.y, instruction_49.z, r0.w);
    // max r0.xyz, r0.xyzx, l(0.100000, 0.100000, 0.100000, 0.000000)
    let instruction_50 = max((r0).xyzx, vec4<f32>(0.10000000149011612, 0.10000000149011612, 0.10000000149011612, 0.0));
    r0 = vec4<f32>(instruction_50.x, instruction_50.y, instruction_50.z, r0.w);
    // add r0.w, r2.x, r2.w
    let instruction_51 = ((r2).xxxx) + ((r2).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_51.w);
    // mad r0.w, cb3[5].w, r2.y, r0.w
    let instruction_52 = fma((object.grid_lighting_sh0).wwww, (r2).yyyy, (r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_52.w);
    // mul r2.xyz, r0.wwww, cb3[5].xyzx
    let instruction_53 = ((r0).wwww) * ((object.grid_lighting_sh0).xyzx);
    r2 = vec4<f32>(instruction_53.x, instruction_53.y, instruction_53.z, r2.w);
    // max r2.xyz, r2.xyzx, l(0.100000, 0.100000, 0.100000, 0.000000)
    let instruction_54 = max((r2).xyzx, vec4<f32>(0.10000000149011612, 0.10000000149011612, 0.10000000149011612, 0.0));
    r2 = vec4<f32>(instruction_54.x, instruction_54.y, instruction_54.z, r2.w);
    // div o5.xyz, r2.xyzx, r0.xyzx
    let instruction_55 = ((r2).xyzx) / ((r0).xyzx);
    o5 = vec4<f32>(instruction_55.x, instruction_55.y, instruction_55.z, o5.w);
    // dp4 r0.x, r1.xyzw, cb0[36].xyzw
    let instruction_56 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[0]).xyzw).xyzw));
    r0 = vec4<f32>(instruction_56.x, r0.y, r0.z, r0.w);
    // dp4 r0.y, r1.xyzw, cb0[37].xyzw
    let instruction_57 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[1]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, instruction_57.y, r0.z, r0.w);
    // dp4 r0.z, r1.xyzw, cb0[38].xyzw
    let instruction_58 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[2]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, r0.y, instruction_58.z, r0.w);
    // dp4 r0.w, r1.xyzw, cb0[39].xyzw
    let instruction_59 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.world.view_projection_rows[3]).xyzw).xyzw));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_59.w);
    // mov o6.xyzw, r0.xyzw
    let instruction_60 = (r0).xyzw;
    o6 = instruction_60;
    // mov o7.xyzw, r0.xyzw
    let instruction_61 = (r0).xyzw;
    o7 = instruction_61;
    var result: Varyings;
    result.position = o7;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v4 = o4;
    result.v5 = o5;
    result.v6 = o6;
    return result;
}
fn shade(input: Varyings) -> vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // div r0.xy, v7.xyxx, cb0[95].xyxx
    let instruction_0 = ((input.position).xyxx) / ((frame.render_target_size).xyxx);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t3.xyzw, s1
    let instruction_1 = textureSample(float_z, float_z_sampler, ((r0).xyxx).xy);
    r0 = instruction_1;
    // max r0.x, |r0.x|, l(0.000000)
    let instruction_2 = max(abs((r0).xxxx), vec4<f32>(9.99999993922529e-09));
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // div r0.x, cb0[58].x, r0.x
    let instruction_3 = ((frame.z_near).xxxx) / ((r0).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, -v6.w
    let instruction_4 = ((r0).xxxx) + (-((input.v6).wwww));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // max r0.y, cb2[5].w, l(0.000100)
    let instruction_5 = max((material.source[0]).wwww, vec4<f32>(9.999999747378752e-05));
    r0 = vec4<f32>(r0.x, instruction_5.y, r0.z, r0.w);
    // div r0.y, l(1.000000, 1.000000, 1.000000, 1.000000), r0.y
    let instruction_6 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_6.y, r0.z, r0.w);
    // mul_sat r0.x, r0.y, r0.x
    let instruction_7 = clamp(((r0).yyyy) * ((r0).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_7.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, v1.w
    let instruction_8 = ((r0).xxxx) * ((input.v1).wwww);
    r0 = vec4<f32>(instruction_8.x, r0.y, r0.z, r0.w);
    // mul r0.yz, cb2[4].wwww, cb2[7].xxyx
    let instruction_9 = ((scene.world.game_time).wwww) * ((material.source[2]).xxyx);
    r0 = vec4<f32>(r0.x, instruction_9.y, instruction_9.z, r0.w);
    // mul r0.yz, r0.yyzy, cb2[7].zzwz
    let instruction_10 = ((r0).yyzy) * ((material.source[2]).zzwz);
    r0 = vec4<f32>(r0.x, instruction_10.y, instruction_10.z, r0.w);
    // frc r0.yz, r0.yyzy
    let instruction_11 = fract((r0).yyzy);
    r0 = vec4<f32>(r0.x, instruction_11.y, instruction_11.z, r0.w);
    // mad r0.yz, v4.xxyx, cb2[7].zzwz, r0.yyzy
    let instruction_12 = fma((input.v4).xxyx, (material.source[2]).zzwz, (r0).yyzy);
    r0 = vec4<f32>(r0.x, instruction_12.y, instruction_12.z, r0.w);
    // sample r1.xyzw, r0.yzyy, t0.xyzw, s2
    let instruction_13 = textureSample(texture_t0, sampler_t0, ((r0).yzyy).xy);
    r1 = instruction_13;
    // mad r0.yz, r1.xxyx, l(0.000000, 2.000000, 2.000000, 0.000000), l(0.000000, -1.000000, -1.000000, 0.000000)
    let instruction_14 = fma((r1).xxyx, vec4<f32>(0.0, 2.0, 2.0, 0.0), vec4<f32>(0.0, -1.0, -1.0, 0.0));
    r0 = vec4<f32>(r0.x, instruction_14.y, instruction_14.z, r0.w);
    // mad r0.yz, r0.yyzy, cb2[8].xxxx, v4.xxyx
    let instruction_15 = fma((r0).yyzy, (material.source[3]).xxxx, (input.v4).xxyx);
    r0 = vec4<f32>(r0.x, instruction_15.y, instruction_15.z, r0.w);
    // mul r1.xy, cb2[4].wwww, cb2[8].yzyy
    let instruction_16 = ((scene.world.game_time).wwww) * ((material.source[3]).yzyy);
    r1 = vec4<f32>(instruction_16.x, instruction_16.y, r1.z, r1.w);
    // mul r1.xy, r1.xyxx, cb2[9].xyxx
    let instruction_17 = ((r1).xyxx) * ((material.source[4]).xyxx);
    r1 = vec4<f32>(instruction_17.x, instruction_17.y, r1.z, r1.w);
    // frc r1.xy, r1.xyxx
    let instruction_18 = fract((r1).xyxx);
    r1 = vec4<f32>(instruction_18.x, instruction_18.y, r1.z, r1.w);
    // mad r0.yz, r0.yyzy, cb2[9].xxyx, r1.xxyx
    let instruction_19 = fma((r0).yyzy, (material.source[4]).xxyx, (r1).xxyx);
    r0 = vec4<f32>(r0.x, instruction_19.y, instruction_19.z, r0.w);
    // sample r1.xyzw, r0.yzyy, t1.xyzw, s4
    let instruction_20 = textureSample(texture_t1, sampler_t1, ((r0).yzyy).xy);
    r1 = instruction_20;
    // sample r2.xyzw, r0.yzyy, t2.xyzw, s3
    let instruction_21 = textureSample(texture_t2, sampler_t2, ((r0).yzyy).xy);
    r2 = instruction_21;
    // mul r0.y, r2.x, cb2[9].z
    let instruction_22 = ((r2).xxxx) * ((material.source[4]).zzzz);
    r0 = vec4<f32>(r0.x, instruction_22.y, r0.z, r0.w);
    // mul r1.xyz, r1.xyzx, cb2[9].zzzz
    let instruction_23 = ((r1).xyzx) * ((material.source[4]).zzzz);
    r1 = vec4<f32>(instruction_23.x, instruction_23.y, instruction_23.z, r1.w);
    // dp3 r0.z, v2.xyzx, v2.xyzx
    let instruction_24 = vec4<f32>(dot(((input.v2).xyzx).xyz, ((input.v2).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, instruction_24.z, r0.w);
    // rsq r0.z, r0.z
    let instruction_25 = inverseSqrt((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_25.z, r0.w);
    // mul r2.xyz, r0.zzzz, v2.xyzx
    let instruction_26 = ((r0).zzzz) * ((input.v2).xyzx);
    r2 = vec4<f32>(instruction_26.x, instruction_26.y, instruction_26.z, r2.w);
    // dp3 r0.z, v0.xyzx, v0.xyzx
    let instruction_27 = vec4<f32>(dot(((input.v0).xyzx).xyz, ((input.v0).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, instruction_27.z, r0.w);
    // rsq r0.z, r0.z
    let instruction_28 = inverseSqrt((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_28.z, r0.w);
    // mul r3.xyz, r0.zzzz, v0.xyzx
    let instruction_29 = ((r0).zzzz) * ((input.v0).xyzx);
    r3 = vec4<f32>(instruction_29.x, instruction_29.y, instruction_29.z, r3.w);
    // dp3_sat r0.z, -r3.xyzx, r2.xyzx
    let instruction_30 = clamp(vec4<f32>(dot((-((r3).xyzx)).xyz, ((r2).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_30.z, r0.w);
    // log r0.z, r0.z
    let instruction_31 = log2((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_31.z, r0.w);
    // mul r0.z, r0.z, cb2[5].y
    let instruction_32 = ((r0).zzzz) * ((material.source[0]).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_32.z, r0.w);
    // exp r0.z, r0.z
    let instruction_33 = exp2((r0).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_33.z, r0.w);
    // mul r0.z, r0.z, cb2[5].z
    let instruction_34 = ((r0).zzzz) * ((material.source[0]).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, instruction_34.z, r0.w);
    // mul r1.w, r0.z, r0.y
    let instruction_35 = ((r0).zzzz) * ((r0).yyyy);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_35.w);
    // mul r1.xyzw, r1.xyzw, v1.xyzw
    let instruction_36 = ((r1).xyzw) * ((input.v1).xyzw);
    r1 = instruction_36;
    // mul r1.w, r0.x, r1.w
    let instruction_37 = ((r0).xxxx) * ((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_37.w);
    // mul r0.xyzw, r1.xyzw, cb2[6].xyzw
    let instruction_38 = ((r1).xyzw) * ((material.source[1]).xyzw);
    r0 = instruction_38;
    // mul r0.xyz, r0.xyzx, cb2[5].xxxx
    let instruction_39 = ((r0).xyzx) * ((material.source[0]).xxxx);
    r0 = vec4<f32>(instruction_39.x, instruction_39.y, instruction_39.z, r0.w);
    // sqrt o0.w, r0.w
    let instruction_40 = sqrt((r0).wwww);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_40.w);
    // mul r0.xyz, r0.xyzx, r0.xyzx
    let instruction_41 = ((r0).xyzx) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_41.x, instruction_41.y, instruction_41.z, r0.w);
    // dp3 r0.w, r3.xyzx, r2.xyzx
    let instruction_42 = vec4<f32>(dot(((r3).xyzx).xyz, ((r2).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_42.w);
    // add r0.w, r0.w, r0.w
    let instruction_43 = ((r0).wwww) + ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_43.w);
    // mad r1.xyz, r2.xyzx, -r0.wwww, r3.xyzx
    let instruction_44 = fma((r2).xyzx, -((r0).wwww), (r3).xyzx);
    r1 = vec4<f32>(instruction_44.x, instruction_44.y, instruction_44.z, r1.w);
    // sample_l r1.xyzw, r1.xyzx, t15.xyzw, s15, l(0.800000)
    let instruction_45 = textureSampleLevel(reflection_probe, reflection_sampler, ((r1).xyzx).xyz, (vec4<f32>(0.800000011920929)).x);
    r1 = instruction_45;
    // add r0.w, r1.w, l(0.000001)
    let instruction_46 = ((r1).wwww) + (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_46.w);
    // div r0.w, l(1.000000, 1.000000, 1.000000, 1.000000), r0.w
    let instruction_47 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_47.w);
    // mul r1.xyz, r0.wwww, r1.xyzx
    let instruction_48 = ((r0).wwww) * ((r1).xyzx);
    r1 = vec4<f32>(instruction_48.x, instruction_48.y, instruction_48.z, r1.w);
    // mul r1.xyz, r1.xyzx, cb2[9].wwww
    let instruction_49 = ((r1).xyzx) * ((material.source[4]).wwww);
    r1 = vec4<f32>(instruction_49.x, instruction_49.y, instruction_49.z, r1.w);
    // mul r1.xyz, r1.xyzx, v5.xyzx
    let instruction_50 = ((r1).xyzx) * ((input.v5).xyzx);
    r1 = vec4<f32>(instruction_50.x, instruction_50.y, instruction_50.z, r1.w);
    // mad r0.xyz, r1.xyzx, l(0.950000, 0.950000, 0.950000, 0.000000), r0.xyzx
    let instruction_51 = fma((r1).xyzx, vec4<f32>(0.949999988079071, 0.949999988079071, 0.949999988079071, 0.0), (r0).xyzx);
    r0 = vec4<f32>(instruction_51.x, instruction_51.y, instruction_51.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_52 = ((r0).xyzx) * ((scene.world.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_52.x, instruction_52.y, instruction_52.z, r0.w);
    // sqrt o0.xyz, r0.xyzx
    let instruction_53 = sqrt((r0).xyzx);
    o0 = vec4<f32>(instruction_53.x, instruction_53.y, instruction_53.z, o0.w);
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
