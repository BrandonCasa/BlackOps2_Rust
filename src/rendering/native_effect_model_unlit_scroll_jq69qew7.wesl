// Original mc_sw4_3d_unlit_scroll_jq69qew7; immutable DXBC hashes in effect source-inventory.json.
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
@group(2) @binding(28) var reflection_probe: texture_cube<f32>;
@group(2) @binding(29) var reflection_sampler: sampler;
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
    // add r0.xyz, v2.xyzx, v2.xyzx
    let instruction_36 = ((input.normal).xyzx) + ((input.normal).xyzx);
    r0 = vec4<f32>(instruction_36.x, instruction_36.y, instruction_36.z, r0.w);
    // ge r1.xyz, v2.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_37 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.normal).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r1 = vec4<f32>(instruction_37.x, instruction_37.y, instruction_37.z, r1.w);
    // mad r3.xyz, v2.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_38 = fma((input.normal).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r3 = vec4<f32>(instruction_38.x, instruction_38.y, instruction_38.z, r3.w);
    // movc r0.xyz, r1.xyzx, r3.xyzx, r0.xyzx
    let instruction_39 = select((r0).xyzx, (r3).xyzx, bitcast<vec4<u32>>((r1).xyzx) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_39.x, instruction_39.y, instruction_39.z, r0.w);
    // dp3 r1.x, r0.xyzx, cb3[0].xyzx
    let instruction_40 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[0]).xyzx).xyz));
    r1 = vec4<f32>(instruction_40.x, r1.y, r1.z, r1.w);
    // dp3 r1.y, r0.xyzx, cb3[1].xyzx
    let instruction_41 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[1]).xyzx).xyz));
    r1 = vec4<f32>(r1.x, instruction_41.y, r1.z, r1.w);
    // dp3 r1.z, r0.xyzx, cb3[2].xyzx
    let instruction_42 = vec4<f32>(dot(((r0).xyzx).xyz, ((object.world_rows[2]).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, instruction_42.z, r1.w);
    // dp3 r0.x, r1.xyzx, r1.xyzx
    let instruction_43 = vec4<f32>(dot(((r1).xyzx).xyz, ((r1).xyzx).xyz));
    r0 = vec4<f32>(instruction_43.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_44 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_44.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, r1.xyzx
    let instruction_45 = ((r0).xxxx) * ((r1).xyzx);
    r0 = vec4<f32>(instruction_45.x, instruction_45.y, instruction_45.z, r0.w);
    // mov o2.xyz, r0.xyzx
    let instruction_46 = (r0).xyzx;
    o2 = vec4<f32>(instruction_46.x, instruction_46.y, instruction_46.z, o2.w);
    // mov o2.w, l(0)
    let instruction_47 = vec4<f32>(0.0);
    o2 = vec4<f32>(o2.x, o2.y, o2.z, instruction_47.w);
    // mov o3.xy, v1.xyxx
    let instruction_48 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o3 = vec4<f32>(instruction_48.x, instruction_48.y, o3.z, o3.w);
    // mov o4.xyzw, cb3[4].xyzw
    let instruction_49 = (object.grid_lighting_coords_and_vis).xyzw;
    o4 = instruction_49;
    // mul r1.xy, r0.yzyy, r0.yzyy
    let instruction_50 = ((r0).yzyy) * ((r0).yzyy);
    r1 = vec4<f32>(instruction_50.x, instruction_50.y, r1.z, r1.w);
    // mad r3.w, r0.x, r0.x, -r1.x
    let instruction_51 = fma((r0).xxxx, (r0).xxxx, -((r1).xxxx));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_51.w);
    // mul r3.xyz, r0.zyxz, r0.xzyx
    let instruction_52 = ((r0).zyxz) * ((r0).xzyx);
    r3 = vec4<f32>(instruction_52.x, instruction_52.y, instruction_52.z, r3.w);
    // dp4 r1.x, cb3[7].xyzw, r3.xyzw
    let instruction_53 = vec4<f32>(dot(((object.grid_lighting_sh2).xyzw).xyzw, ((r3).xyzw).xyzw));
    r1 = vec4<f32>(instruction_53.x, r1.y, r1.z, r1.w);
    // dp4 r1.z, cb3[10].xyzw, r3.xyzw
    let instruction_54 = vec4<f32>(dot(((object.reflection_lighting_sh2).xyzw).xyzw, ((r3).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_54.z, r1.w);
    // mov r0.w, l(1.000000)
    let instruction_55 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_55.w);
    // dp4 r1.w, cb3[6].xyzw, r0.xyzw
    let instruction_56 = vec4<f32>(dot(((object.grid_lighting_sh1).xyzw).xyzw, ((r0).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_56.w);
    // dp4 r0.x, cb3[9].xyzw, r0.xyzw
    let instruction_57 = vec4<f32>(dot(((object.reflection_lighting_sh1).xyzw).xyzw, ((r0).xyzw).xyzw));
    r0 = vec4<f32>(instruction_57.x, r0.y, r0.z, r0.w);
    // add r0.x, r1.z, r0.x
    let instruction_58 = ((r1).zzzz) + ((r0).xxxx);
    r0 = vec4<f32>(instruction_58.x, r0.y, r0.z, r0.w);
    // mad r0.x, cb3[8].w, r1.y, r0.x
    let instruction_59 = fma((object.reflection_lighting_sh0).wwww, (r1).yyyy, (r0).xxxx);
    r0 = vec4<f32>(instruction_59.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, cb3[8].xyzx
    let instruction_60 = ((r0).xxxx) * ((object.reflection_lighting_sh0).xyzx);
    r0 = vec4<f32>(instruction_60.x, instruction_60.y, instruction_60.z, r0.w);
    // max r0.xyz, r0.xyzx, l(0.100000, 0.100000, 0.100000, 0.000000)
    let instruction_61 = max((r0).xyzx, vec4<f32>(0.10000000149011612, 0.10000000149011612, 0.10000000149011612, 0.0));
    r0 = vec4<f32>(instruction_61.x, instruction_61.y, instruction_61.z, r0.w);
    // add r0.w, r1.x, r1.w
    let instruction_62 = ((r1).xxxx) + ((r1).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_62.w);
    // mad r0.w, cb3[5].w, r1.y, r0.w
    let instruction_63 = fma((object.grid_lighting_sh0).wwww, (r1).yyyy, (r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_63.w);
    // mul r1.xyz, r0.wwww, cb3[5].xyzx
    let instruction_64 = ((r0).wwww) * ((object.grid_lighting_sh0).xyzx);
    r1 = vec4<f32>(instruction_64.x, instruction_64.y, instruction_64.z, r1.w);
    // max r1.xyz, r1.xyzx, l(0.100000, 0.100000, 0.100000, 0.000000)
    let instruction_65 = max((r1).xyzx, vec4<f32>(0.10000000149011612, 0.10000000149011612, 0.10000000149011612, 0.0));
    r1 = vec4<f32>(instruction_65.x, instruction_65.y, instruction_65.z, r1.w);
    // div o5.xyz, r1.xyzx, r0.xyzx
    let instruction_66 = ((r1).xyzx) / ((r0).xyzx);
    o5 = vec4<f32>(instruction_66.x, instruction_66.y, instruction_66.z, o5.w);
    // dp4 o6.x, r2.xyzw, cb0[36].xyzw
    let instruction_67 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[0]).xyzw).xyzw));
    o6 = vec4<f32>(instruction_67.x, o6.y, o6.z, o6.w);
    // dp4 o6.y, r2.xyzw, cb0[37].xyzw
    let instruction_68 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[1]).xyzw).xyzw));
    o6 = vec4<f32>(o6.x, instruction_68.y, o6.z, o6.w);
    // dp4 o6.z, r2.xyzw, cb0[38].xyzw
    let instruction_69 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[2]).xyzw).xyzw));
    o6 = vec4<f32>(o6.x, o6.y, instruction_69.z, o6.w);
    // dp4 o6.w, r2.xyzw, cb0[39].xyzw
    let instruction_70 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[3]).xyzw).xyzw));
    o6 = vec4<f32>(o6.x, o6.y, o6.z, instruction_70.w);
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
    var r3: vec4<f32>;
    // dp3 r0.x, v1.xyzx, v1.xyzx
    let instruction_0 = vec4<f32>(dot(((input.v1).xyzx).xyz, ((input.v1).xyzx).xyz));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_1 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // mul r0.yzw, r0.xxxx, v1.xxyz
    let instruction_2 = ((r0).xxxx) * ((input.v1).xxyz);
    r0 = vec4<f32>(r0.x, instruction_2.y, instruction_2.z, instruction_2.w);
    // mad r1.xyz, -v1.xyzx, r0.xxxx, cb0[18].xyzx
    let instruction_3 = fma(-((input.v1).xyzx), (r0).xxxx, (scene.world.sun_position).xyzx);
    r1 = vec4<f32>(instruction_3.x, instruction_3.y, instruction_3.z, r1.w);
    // dp3 r0.x, v2.xyzx, v2.xyzx
    let instruction_4 = vec4<f32>(dot(((input.v2).xyzx).xyz, ((input.v2).xyzx).xyz));
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_5 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // mul r2.xyz, r0.xxxx, v2.xyzx
    let instruction_6 = ((r0).xxxx) * ((input.v2).xyzx);
    r2 = vec4<f32>(instruction_6.x, instruction_6.y, instruction_6.z, r2.w);
    // dp3 r0.x, r0.yzwy, r2.xyzx
    let instruction_7 = vec4<f32>(dot(((r0).yzwy).xyz, ((r2).xyzx).xyz));
    r0 = vec4<f32>(instruction_7.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, r0.x
    let instruction_8 = ((r0).xxxx) + ((r0).xxxx);
    r0 = vec4<f32>(instruction_8.x, r0.y, r0.z, r0.w);
    // mad r3.xyz, r2.xyzx, -r0.xxxx, r0.yzwy
    let instruction_9 = fma((r2).xyzx, -((r0).xxxx), (r0).yzwy);
    r3 = vec4<f32>(instruction_9.x, instruction_9.y, instruction_9.z, r3.w);
    // dp3_sat r0.x, r2.xyzx, -r0.yzwy
    let instruction_10 = clamp(vec4<f32>(dot(((r2).xyzx).xyz, (-((r0).yzwy)).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_10.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, l(-9.280000)
    let instruction_11 = ((r0).xxxx) * (vec4<f32>(-9.279999732971191));
    r0 = vec4<f32>(instruction_11.x, r0.y, r0.z, r0.w);
    // exp r0.x, r0.x
    let instruction_12 = exp2((r0).xxxx);
    r0 = vec4<f32>(instruction_12.x, r0.y, r0.z, r0.w);
    // min r0.x, r0.x, l(0.475000)
    let instruction_13 = min((r0).xxxx, vec4<f32>(0.4749999940395355));
    r0 = vec4<f32>(instruction_13.x, r0.y, r0.z, r0.w);
    // mad r0.x, r0.x, l(1.041667), l(0.002604)
    let instruction_14 = fma((r0).xxxx, vec4<f32>(1.0416667461395264), vec4<f32>(0.0026042000390589237));
    r0 = vec4<f32>(instruction_14.x, r0.y, r0.z, r0.w);
    // sample_l r3.xyzw, r3.xyzx, t15.xyzw, s15, l(0.000000)
    let instruction_15 = textureSampleLevel(reflection_probe, reflection_sampler, ((r3).xyzx).xyz, (vec4<f32>(0.0)).x);
    r3 = instruction_15;
    // add r0.y, r3.w, l(0.000001)
    let instruction_16 = ((r3).wwww) + (vec4<f32>(9.999999974752427e-07));
    r0 = vec4<f32>(r0.x, instruction_16.y, r0.z, r0.w);
    // div r0.y, l(1.000000, 1.000000, 1.000000, 1.000000), r0.y
    let instruction_17 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_17.y, r0.z, r0.w);
    // mul r0.yzw, r0.yyyy, r3.xxyz
    let instruction_18 = ((r0).yyyy) * ((r3).xxyz);
    r0 = vec4<f32>(r0.x, instruction_18.y, instruction_18.z, instruction_18.w);
    // add r1.w, -r0.x, l(1.000000)
    let instruction_19 = (-((r0).xxxx)) + (vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_19.w);
    // mad r0.x, r1.w, l(0.040000), r0.x
    let instruction_20 = fma((r1).wwww, vec4<f32>(0.03999999910593033), (r0).xxxx);
    r0 = vec4<f32>(instruction_20.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, r0.yzwy
    let instruction_21 = ((r0).xxxx) * ((r0).yzwy);
    r0 = vec4<f32>(instruction_21.x, instruction_21.y, instruction_21.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb2[5].wwww
    let instruction_22 = ((r0).xyzx) * ((material.source[0]).wwww);
    r0 = vec4<f32>(instruction_22.x, instruction_22.y, instruction_22.z, r0.w);
    // dp3 r0.w, r1.xyzx, r1.xyzx
    let instruction_23 = vec4<f32>(dot(((r1).xyzx).xyz, ((r1).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_23.w);
    // rsq r0.w, r0.w
    let instruction_24 = inverseSqrt((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_24.w);
    // mul r1.xyz, r0.wwww, r1.xyzx
    let instruction_25 = ((r0).wwww) * ((r1).xyzx);
    r1 = vec4<f32>(instruction_25.x, instruction_25.y, instruction_25.z, r1.w);
    // dp3_sat r0.w, r2.xyzx, r1.xyzx
    let instruction_26 = clamp(vec4<f32>(dot(((r2).xyzx).xyz, ((r1).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_26.w);
    // dp3_sat r1.x, r1.xyzx, cb0[18].xyzx
    let instruction_27 = clamp(vec4<f32>(dot(((r1).xyzx).xyz, ((scene.world.sun_position).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(instruction_27.x, r1.y, r1.z, r1.w);
    // dp3_sat r1.y, r2.xyzx, cb0[18].xyzx
    let instruction_28 = clamp(vec4<f32>(dot(((r2).xyzx).xyz, ((scene.world.sun_position).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, instruction_28.y, r1.z, r1.w);
    // log r0.w, r0.w
    let instruction_29 = log2((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_29.w);
    // mul r0.w, r0.w, l(8192.000000)
    let instruction_30 = ((r0).wwww) * (vec4<f32>(8192.0));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_30.w);
    // exp r0.w, r0.w
    let instruction_31 = exp2((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_31.w);
    // mul r0.w, r0.w, l(1024.250000)
    let instruction_32 = ((r0).wwww) * (vec4<f32>(1024.25));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_32.w);
    // mad r1.z, r1.x, r1.x, l(0.010000)
    let instruction_33 = fma((r1).xxxx, (r1).xxxx, vec4<f32>(0.009999999776482582));
    r1 = vec4<f32>(r1.x, r1.y, instruction_33.z, r1.w);
    // mul r1.x, r1.x, l(-10.000000)
    let instruction_34 = ((r1).xxxx) * (vec4<f32>(-10.0));
    r1 = vec4<f32>(instruction_34.x, r1.y, r1.z, r1.w);
    // exp r1.x, r1.x
    let instruction_35 = exp2((r1).xxxx);
    r1 = vec4<f32>(instruction_35.x, r1.y, r1.z, r1.w);
    // mad r1.x, r1.x, l(0.960000), l(0.040000)
    let instruction_36 = fma((r1).xxxx, vec4<f32>(0.9599999785423279), vec4<f32>(0.03999999910593033));
    r1 = vec4<f32>(instruction_36.x, r1.y, r1.z, r1.w);
    // div r1.y, r1.y, r1.z
    let instruction_37 = ((r1).yyyy) / ((r1).zzzz);
    r1 = vec4<f32>(r1.x, instruction_37.y, r1.z, r1.w);
    // mul r0.w, r0.w, r1.y
    let instruction_38 = ((r0).wwww) * ((r1).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_38.w);
    // mul r0.w, r1.x, r0.w
    let instruction_39 = ((r1).xxxx) * ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_39.w);
    // mul r1.xyz, v4.wwww, cb0[19].xyzx
    let instruction_40 = ((input.v4).wwww) * ((scene.world.sun_diffuse).xyzx);
    r1 = vec4<f32>(instruction_40.x, instruction_40.y, instruction_40.z, r1.w);
    // mul r1.xyz, r0.wwww, r1.xyzx
    let instruction_41 = ((r0).wwww) * ((r1).xyzx);
    r1 = vec4<f32>(instruction_41.x, instruction_41.y, instruction_41.z, r1.w);
    // mad r0.xyz, r0.xyzx, v5.xyzx, r1.xyzx
    let instruction_42 = fma((r0).xyzx, (input.v5).xyzx, (r1).xyzx);
    r0 = vec4<f32>(instruction_42.x, instruction_42.y, instruction_42.z, r0.w);
    // mul r1.xy, cb2[4].wwww, cb2[5].xyxx
    let instruction_43 = ((scene.world.game_time).wwww) * ((material.source[0]).xyxx);
    r1 = vec4<f32>(instruction_43.x, instruction_43.y, r1.z, r1.w);
    // mul r1.xy, r1.xyxx, cb2[5].zzzz
    let instruction_44 = ((r1).xyxx) * ((material.source[0]).zzzz);
    r1 = vec4<f32>(instruction_44.x, instruction_44.y, r1.z, r1.w);
    // mad r1.xy, v3.xyxx, cb2[5].zzzz, r1.xyxx
    let instruction_45 = fma((input.v3).xyxx, (material.source[0]).zzzz, (r1).xyxx);
    r1 = vec4<f32>(instruction_45.x, instruction_45.y, r1.z, r1.w);
    // sample r1.xyzw, r1.xyxx, t1.xyzw, s1
    let instruction_46 = textureSample(texture_t1, sampler_t1, ((r1).xyxx).xy);
    r1 = instruction_46;
    // sample r2.xyzw, v3.xyxx, t0.xyzw, s2
    let instruction_47 = textureSample(texture_t0, sampler_t0, ((input.v3).xyxx).xy);
    r2 = instruction_47;
    // mul r1.xyzw, r1.xyzw, r2.xyzw
    let instruction_48 = ((r1).xyzw) * ((r2).xyzw);
    r1 = instruction_48;
    // mul r1.xyzw, r1.xyzw, cb2[6].xxxx
    let instruction_49 = ((r1).xyzw) * ((material.source[1]).xxxx);
    r1 = instruction_49;
    // mad r0.xyz, r1.xyzx, r1.xyzx, r0.xyzx
    let instruction_50 = fma((r1).xyzx, (r1).xyzx, (r0).xyzx);
    r0 = vec4<f32>(instruction_50.x, instruction_50.y, instruction_50.z, r0.w);
    // sqrt o0.w, r1.w
    let instruction_51 = sqrt((r1).wwww);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_51.w);
    // add r0.xyz, r0.xyzx, -v0.xyzx
    let instruction_52 = ((r0).xyzx) + (-((input.v0).xyzx));
    r0 = vec4<f32>(instruction_52.x, instruction_52.y, instruction_52.z, r0.w);
    // mad r0.xyz, v1.wwww, r0.xyzx, v0.xyzx
    let instruction_53 = fma((input.v1).wwww, (r0).xyzx, (input.v0).xyzx);
    r0 = vec4<f32>(instruction_53.x, instruction_53.y, instruction_53.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_54 = ((r0).xyzx) * ((scene.world.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_54.x, instruction_54.y, instruction_54.z, r0.w);
    // sqrt o0.xyz, r0.xyzx
    let instruction_55 = sqrt((r0).xyzx);
    o0 = vec4<f32>(instruction_55.x, instruction_55.y, instruction_55.z, o0.w);
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
