// Original mc_sw4_3d_water_on_surface_reveal_77320z68; immutable DXBC hashes in effect source-inventory.json.
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
@group(2) @binding(28) var reflection_probe: texture_cube<f32>;
@group(2) @binding(29) var reflection_sampler: sampler;
@group(3) @binding(0) var model_lighting: texture_3d<f32>;
@group(3) @binding(1) var model_lighting_sampler: sampler;
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
    @location(4) v4: vec4<f32>,
    @location(5) v5: vec4<f32>,
    @location(6) v6: vec4<f32>,
    @location(7) v7: vec4<f32>,
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
    var o8: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    var r4: vec4<f32>;
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
    // mul r0.xyz, r0.xxxx, r1.xyzx
    let instruction_46 = ((r0).xxxx) * ((r1).xyzx);
    r0 = vec4<f32>(instruction_46.x, instruction_46.y, instruction_46.z, r0.w);
    // mov o3.xyz, r0.xyzx
    let instruction_47 = (r0).xyzx;
    o3 = vec4<f32>(instruction_47.x, instruction_47.y, instruction_47.z, o3.w);
    // mov o3.w, l(0)
    let instruction_48 = vec4<f32>(0.0);
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_48.w);
    // add r1.xyz, v4.xyzx, v4.xyzx
    let instruction_49 = ((input.tangent).xyzx) + ((input.tangent).xyzx);
    r1 = vec4<f32>(instruction_49.x, instruction_49.y, instruction_49.z, r1.w);
    // ge r3.xyz, v4.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_50 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.tangent).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r3 = vec4<f32>(instruction_50.x, instruction_50.y, instruction_50.z, r3.w);
    // mad r4.xyz, v4.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_51 = fma((input.tangent).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r4 = vec4<f32>(instruction_51.x, instruction_51.y, instruction_51.z, r4.w);
    // movc r1.xyz, r3.xyzx, r4.xyzx, r1.xyzx
    let instruction_52 = select((r1).xyzx, (r4).xyzx, bitcast<vec4<u32>>((r3).xyzx) != vec4<u32>(0u));
    r1 = vec4<f32>(instruction_52.x, instruction_52.y, instruction_52.z, r1.w);
    // dp3 r3.x, r1.xyzx, cb3[0].xyzx
    let instruction_53 = vec4<f32>(dot(((r1).xyzx).xyz, ((object.world_rows[0]).xyzx).xyz));
    r3 = vec4<f32>(instruction_53.x, r3.y, r3.z, r3.w);
    // dp3 r3.y, r1.xyzx, cb3[1].xyzx
    let instruction_54 = vec4<f32>(dot(((r1).xyzx).xyz, ((object.world_rows[1]).xyzx).xyz));
    r3 = vec4<f32>(r3.x, instruction_54.y, r3.z, r3.w);
    // dp3 r3.z, r1.xyzx, cb3[2].xyzx
    let instruction_55 = vec4<f32>(dot(((r1).xyzx).xyz, ((object.world_rows[2]).xyzx).xyz));
    r3 = vec4<f32>(r3.x, r3.y, instruction_55.z, r3.w);
    // dp3 r1.x, r3.xyzx, r3.xyzx
    let instruction_56 = vec4<f32>(dot(((r3).xyzx).xyz, ((r3).xyzx).xyz));
    r1 = vec4<f32>(instruction_56.x, r1.y, r1.z, r1.w);
    // rsq r1.x, r1.x
    let instruction_57 = inverseSqrt((r1).xxxx);
    r1 = vec4<f32>(instruction_57.x, r1.y, r1.z, r1.w);
    // mul o4.xyz, r1.xxxx, r3.xyzx
    let instruction_58 = ((r1).xxxx) * ((r3).xyzx);
    o4 = vec4<f32>(instruction_58.x, instruction_58.y, instruction_58.z, o4.w);
    // lt r1.x, l(0.000000), v0.w
    let instruction_59 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0)) < ((input.position).wwww)));
    r1 = vec4<f32>(instruction_59.x, r1.y, r1.z, r1.w);
    // lt r1.y, v0.w, l(0.000000)
    let instruction_60 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.position).wwww) < (vec4<f32>(0.0))));
    r1 = vec4<f32>(r1.x, instruction_60.y, r1.z, r1.w);
    // iadd r1.x, r1.y, -r1.x
    let instruction_61 = bitcast<vec4<f32>>(bitcast<vec4<i32>>((r1).yyyy) + -(bitcast<vec4<i32>>((r1).xxxx)));
    r1 = vec4<f32>(instruction_61.x, r1.y, r1.z, r1.w);
    // itof r1.x, r1.x
    let instruction_62 = vec4<f32>(bitcast<vec4<i32>>((r1).xxxx));
    r1 = vec4<f32>(instruction_62.x, r1.y, r1.z, r1.w);
    // lt r1.y, l(0.000000), r1.x
    let instruction_63 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0)) < ((r1).xxxx)));
    r1 = vec4<f32>(r1.x, instruction_63.y, r1.z, r1.w);
    // lt r1.x, r1.x, l(0.000000)
    let instruction_64 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r1).xxxx) < (vec4<f32>(0.0))));
    r1 = vec4<f32>(instruction_64.x, r1.y, r1.z, r1.w);
    // iadd r1.x, r1.x, -r1.y
    let instruction_65 = bitcast<vec4<f32>>(bitcast<vec4<i32>>((r1).xxxx) + -(bitcast<vec4<i32>>((r1).yyyy)));
    r1 = vec4<f32>(instruction_65.x, r1.y, r1.z, r1.w);
    // itof o4.w, r1.x
    let instruction_66 = vec4<f32>(bitcast<vec4<i32>>((r1).xxxx));
    o4 = vec4<f32>(o4.x, o4.y, o4.z, instruction_66.w);
    // mov o5.xy, v2.xyxx
    let instruction_67 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o5 = vec4<f32>(instruction_67.x, instruction_67.y, o5.z, o5.w);
    // mov o6.xyzw, cb3[4].xyzw
    let instruction_68 = (object.grid_lighting_coords_and_vis).xyzw;
    o6 = instruction_68;
    // mul r1.xy, r0.yzyy, r0.yzyy
    let instruction_69 = ((r0).yzyy) * ((r0).yzyy);
    r1 = vec4<f32>(instruction_69.x, instruction_69.y, r1.z, r1.w);
    // mad r3.w, r0.x, r0.x, -r1.x
    let instruction_70 = fma((r0).xxxx, (r0).xxxx, -((r1).xxxx));
    r3 = vec4<f32>(r3.x, r3.y, r3.z, instruction_70.w);
    // mul r3.xyz, r0.zyxz, r0.xzyx
    let instruction_71 = ((r0).zyxz) * ((r0).xzyx);
    r3 = vec4<f32>(instruction_71.x, instruction_71.y, instruction_71.z, r3.w);
    // dp4 r1.x, cb3[7].xyzw, r3.xyzw
    let instruction_72 = vec4<f32>(dot(((object.grid_lighting_sh2).xyzw).xyzw, ((r3).xyzw).xyzw));
    r1 = vec4<f32>(instruction_72.x, r1.y, r1.z, r1.w);
    // dp4 r1.z, cb3[10].xyzw, r3.xyzw
    let instruction_73 = vec4<f32>(dot(((object.reflection_lighting_sh2).xyzw).xyzw, ((r3).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_73.z, r1.w);
    // mov r0.w, l(1.000000)
    let instruction_74 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_74.w);
    // dp4 r1.w, cb3[6].xyzw, r0.xyzw
    let instruction_75 = vec4<f32>(dot(((object.grid_lighting_sh1).xyzw).xyzw, ((r0).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_75.w);
    // dp4 r0.x, cb3[9].xyzw, r0.xyzw
    let instruction_76 = vec4<f32>(dot(((object.reflection_lighting_sh1).xyzw).xyzw, ((r0).xyzw).xyzw));
    r0 = vec4<f32>(instruction_76.x, r0.y, r0.z, r0.w);
    // add r0.x, r1.z, r0.x
    let instruction_77 = ((r1).zzzz) + ((r0).xxxx);
    r0 = vec4<f32>(instruction_77.x, r0.y, r0.z, r0.w);
    // mad r0.x, cb3[8].w, r1.y, r0.x
    let instruction_78 = fma((object.reflection_lighting_sh0).wwww, (r1).yyyy, (r0).xxxx);
    r0 = vec4<f32>(instruction_78.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, cb3[8].xyzx
    let instruction_79 = ((r0).xxxx) * ((object.reflection_lighting_sh0).xyzx);
    r0 = vec4<f32>(instruction_79.x, instruction_79.y, instruction_79.z, r0.w);
    // max r0.xyz, r0.xyzx, l(0.100000, 0.100000, 0.100000, 0.000000)
    let instruction_80 = max((r0).xyzx, vec4<f32>(0.10000000149011612, 0.10000000149011612, 0.10000000149011612, 0.0));
    r0 = vec4<f32>(instruction_80.x, instruction_80.y, instruction_80.z, r0.w);
    // add r0.w, r1.x, r1.w
    let instruction_81 = ((r1).xxxx) + ((r1).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_81.w);
    // mad r0.w, cb3[5].w, r1.y, r0.w
    let instruction_82 = fma((object.grid_lighting_sh0).wwww, (r1).yyyy, (r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_82.w);
    // mul r1.xyz, r0.wwww, cb3[5].xyzx
    let instruction_83 = ((r0).wwww) * ((object.grid_lighting_sh0).xyzx);
    r1 = vec4<f32>(instruction_83.x, instruction_83.y, instruction_83.z, r1.w);
    // max r1.xyz, r1.xyzx, l(0.100000, 0.100000, 0.100000, 0.000000)
    let instruction_84 = max((r1).xyzx, vec4<f32>(0.10000000149011612, 0.10000000149011612, 0.10000000149011612, 0.0));
    r1 = vec4<f32>(instruction_84.x, instruction_84.y, instruction_84.z, r1.w);
    // div o7.xyz, r1.xyzx, r0.xyzx
    let instruction_85 = ((r1).xyzx) / ((r0).xyzx);
    o7 = vec4<f32>(instruction_85.x, instruction_85.y, instruction_85.z, o7.w);
    // dp4 o8.x, r2.xyzw, cb0[36].xyzw
    let instruction_86 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[0]).xyzw).xyzw));
    o8 = vec4<f32>(instruction_86.x, o8.y, o8.z, o8.w);
    // dp4 o8.y, r2.xyzw, cb0[37].xyzw
    let instruction_87 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[1]).xyzw).xyzw));
    o8 = vec4<f32>(o8.x, instruction_87.y, o8.z, o8.w);
    // dp4 o8.z, r2.xyzw, cb0[38].xyzw
    let instruction_88 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[2]).xyzw).xyzw));
    o8 = vec4<f32>(o8.x, o8.y, instruction_88.z, o8.w);
    // dp4 o8.w, r2.xyzw, cb0[39].xyzw
    let instruction_89 = vec4<f32>(dot(((r2).xyzw).xyzw, ((scene.world.view_projection_rows[3]).xyzw).xyzw));
    o8 = vec4<f32>(o8.x, o8.y, o8.z, instruction_89.w);
    var result: Varyings;
    result.position = o8;
    result.v0 = o0;
    result.v1 = o1;
    result.v2 = o2;
    result.v3 = o3;
    result.v4 = o4;
    result.v5 = o5;
    result.v6 = o6;
    result.v7 = o7;
    return result;
}
fn shade(input: Varyings) -> vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    var r4: vec4<f32>;
    var r5: vec4<f32>;
    var r6: vec4<f32>;
    var r7: vec4<f32>;
    // mul r0.xy, cb2[4].wwww, l(0.000000, -0.142018, 0.000000, 0.000000)
    let instruction_0 = ((scene.world.game_time).wwww) * (vec4<f32>(0.0, -0.14201830327510834, 0.0, 0.0));
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // frc r0.xy, r0.xyxx
    let instruction_1 = fract((r0).xyxx);
    r0 = vec4<f32>(instruction_1.x, instruction_1.y, r0.z, r0.w);
    // mad r0.xy, v5.xyxx, l(2.840366, 2.840366, 0.000000, 0.000000), r0.xyxx
    let instruction_2 = fma((input.v5).xyxx, vec4<f32>(2.8403658866882324, 2.8403658866882324, 0.0, 0.0), (r0).xyxx);
    r0 = vec4<f32>(instruction_2.x, instruction_2.y, r0.z, r0.w);
    // mul r0.xy, r0.xyxx, l(0.997100, -0.090000, 0.000000, 0.000000)
    let instruction_3 = ((r0).xyxx) * (vec4<f32>(0.9970999956130981, -0.09000000357627869, 0.0, 0.0));
    r0 = vec4<f32>(instruction_3.x, instruction_3.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t2.xyzw, s3
    let instruction_4 = textureSample(texture_t2, sampler_t2, ((r0).xyxx).xy);
    r0 = instruction_4;
    // mad r0.x, r0.x, l(0.100000), l(1.000000)
    let instruction_5 = fma((r0).xxxx, vec4<f32>(0.10000000149011612), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_5.x, r0.y, r0.z, r0.w);
    // log r0.yz, v5.xxyx
    let instruction_6 = log2((input.v5).xxyx);
    r0 = vec4<f32>(r0.x, instruction_6.y, instruction_6.z, r0.w);
    // mul r0.xy, r0.yzyy, r0.xxxx
    let instruction_7 = ((r0).yzyy) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_7.x, instruction_7.y, r0.z, r0.w);
    // exp r0.xy, r0.xyxx
    let instruction_8 = exp2((r0).xyxx);
    r0 = vec4<f32>(instruction_8.x, instruction_8.y, r0.z, r0.w);
    // add r0.xy, r0.xyxx, l(-0.500000, -0.500000, 0.000000, 0.000000)
    let instruction_9 = ((r0).xyxx) + (vec4<f32>(-0.5, -0.5, 0.0, 0.0));
    r0 = vec4<f32>(instruction_9.x, instruction_9.y, r0.z, r0.w);
    // div r0.xy, r0.xyxx, cb2[8].yzyy
    let instruction_10 = ((r0).xyxx) / ((material.source[3]).yzyy);
    r0 = vec4<f32>(instruction_10.x, instruction_10.y, r0.z, r0.w);
    // add r0.xy, r0.xyxx, l(0.500000, 0.500000, 0.000000, 0.000000)
    let instruction_11 = ((r0).xyxx) + (vec4<f32>(0.5, 0.5, 0.0, 0.0));
    r0 = vec4<f32>(instruction_11.x, instruction_11.y, r0.z, r0.w);
    // mul r0.z, cb2[4].w, cb2[5].x
    let instruction_12 = ((scene.world.game_time).wwww) * ((material.source[0]).xxxx);
    r0 = vec4<f32>(r0.x, r0.y, instruction_12.z, r0.w);
    // mad r1.xy, -r0.zzzz, l(-0.008910, 0.799950, 0.000000, 0.000000), r0.xyxx
    let instruction_13 = fma(-((r0).zzzz), vec4<f32>(-0.008910338394343853, 0.799950361251831, 0.0, 0.0), (r0).xyxx);
    r1 = vec4<f32>(instruction_13.x, instruction_13.y, r1.z, r1.w);
    // mad r0.zw, r0.zzzz, l(0.000000, 0.000000, -0.014479, 1.299919), l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_14 = fma((r0).zzzz, vec4<f32>(0.0, 0.0, -0.014479300007224083, 1.2999193668365479), vec4<f32>(0.0, 0.0, 0.5, 0.5));
    r0 = vec4<f32>(r0.x, r0.y, instruction_14.z, instruction_14.w);
    // add r0.xy, -r0.zwzz, r0.xyxx
    let instruction_15 = (-((r0).zwzz)) + ((r0).xyxx);
    r0 = vec4<f32>(instruction_15.x, instruction_15.y, r0.z, r0.w);
    // ge r0.zw, r1.xxxy, -r1.xxxy
    let instruction_16 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r1).xxxy) >= (-((r1).xxxy))));
    r0 = vec4<f32>(r0.x, r0.y, instruction_16.z, instruction_16.w);
    // frc r1.xy, |r1.xyxx|
    let instruction_17 = fract(abs((r1).xyxx));
    r1 = vec4<f32>(instruction_17.x, instruction_17.y, r1.z, r1.w);
    // movc r0.zw, r0.zzzw, r1.xxxy, -r1.xxxy
    let instruction_18 = select(-((r1).xxxy), (r1).xxxy, bitcast<vec4<u32>>((r0).zzzw) != vec4<u32>(0u));
    r0 = vec4<f32>(r0.x, r0.y, instruction_18.z, instruction_18.w);
    // sample r1.xyzw, r0.zwzz, t3.xyzw, s4
    let instruction_19 = textureSample(texture_t3, sampler_t3, ((r0).zwzz).xy);
    r1 = instruction_19;
    // mad r0.zw, r1.xxxy, l(0.000000, 0.000000, 4.015748, 4.015748), l(0.000000, 0.000000, -2.015748, -2.015748)
    let instruction_20 = fma((r1).xxxy, vec4<f32>(0.0, 0.0, 4.015748023986816, 4.015748023986816), vec4<f32>(0.0, 0.0, -2.0157480239868164, -2.0157480239868164));
    r0 = vec4<f32>(r0.x, r0.y, instruction_20.z, instruction_20.w);
    // ge r1.xy, r0.xyxx, -r0.xyxx
    let instruction_21 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r0).xyxx) >= (-((r0).xyxx))));
    r1 = vec4<f32>(instruction_21.x, instruction_21.y, r1.z, r1.w);
    // frc r0.xy, |r0.xyxx|
    let instruction_22 = fract(abs((r0).xyxx));
    r0 = vec4<f32>(instruction_22.x, instruction_22.y, r0.z, r0.w);
    // movc r0.xy, r1.xyxx, r0.xyxx, -r0.xyxx
    let instruction_23 = select(-((r0).xyxx), (r0).xyxx, bitcast<vec4<u32>>((r1).xyxx) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_23.x, instruction_23.y, r0.z, r0.w);
    // sample r1.xyzw, r0.xyxx, t3.xyzw, s4
    let instruction_24 = textureSample(texture_t3, sampler_t3, ((r0).xyxx).xy);
    r1 = instruction_24;
    // mad r0.xy, r1.xyxx, l(4.015748, 4.015748, 0.000000, 0.000000), r0.zwzz
    let instruction_25 = fma((r1).xyxx, vec4<f32>(4.015748023986816, 4.015748023986816, 0.0, 0.0), (r0).zwzz);
    r0 = vec4<f32>(instruction_25.x, instruction_25.y, r0.z, r0.w);
    // add r0.xy, r0.xyxx, l(-2.015748, -2.015748, 0.000000, 0.000000)
    let instruction_26 = ((r0).xyxx) + (vec4<f32>(-2.0157480239868164, -2.0157480239868164, 0.0, 0.0));
    r0 = vec4<f32>(instruction_26.x, instruction_26.y, r0.z, r0.w);
    // mov r0.z, l(2.000000)
    let instruction_27 = vec4<f32>(2.0);
    r0 = vec4<f32>(r0.x, r0.y, instruction_27.z, r0.w);
    // dp3 r0.w, r0.xyzx, r0.xyzx
    let instruction_28 = vec4<f32>(dot(((r0).xyzx).xyz, ((r0).xyzx).xyz));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_28.w);
    // rsq r0.w, r0.w
    let instruction_29 = inverseSqrt((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_29.w);
    // mul r0.xyz, r0.wwww, r0.xyzx
    let instruction_30 = ((r0).wwww) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_30.x, instruction_30.y, instruction_30.z, r0.w);
    // div r0.xyz, r0.xyzx, r0.zzzz
    let instruction_31 = ((r0).xyzx) / ((r0).zzzz);
    r0 = vec4<f32>(instruction_31.x, instruction_31.y, instruction_31.z, r0.w);
    // dp2 r0.w, r0.xyxx, l(0.500000, 0.500000, 0.000000, 0.000000)
    let instruction_32 = vec4<f32>(dot(((r0).xyxx).xy, (vec4<f32>(0.5, 0.5, 0.0, 0.0)).xy));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_32.w);
    // lt r0.w, l(0.000000), r0.w
    let instruction_33 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0)) < ((r0).wwww)));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_33.w);
    // movc r1.x, r0.w, l(0.214286), l(0.285714)
    let instruction_34 = select(vec4<f32>(0.2857140004634857), vec4<f32>(0.21428599953651428), bitcast<vec4<u32>>((r0).wwww) != vec4<u32>(0u));
    r1 = vec4<f32>(instruction_34.x, r1.y, r1.z, r1.w);
    // and r0.w, r0.w, l(0x3f800000)
    let instruction_35 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).wwww) & bitcast<vec4<u32>>(vec4<f32>(1.0)));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_35.w);
    // sample r2.xyzw, v5.xyxx, t5.xyzw, s6
    let instruction_36 = textureSample(texture_t5, sampler_t5, ((input.v5).xyxx).xy);
    r2 = instruction_36;
    // add r1.xyz, r1.xxxx, r2.xyzx
    let instruction_37 = ((r1).xxxx) + ((r2).xyzx);
    r1 = vec4<f32>(instruction_37.x, instruction_37.y, instruction_37.z, r1.w);
    // add r2.xyz, -r1.xyzx, r2.xyzx
    let instruction_38 = (-((r1).xyzx)) + ((r2).xyzx);
    r2 = vec4<f32>(instruction_38.x, instruction_38.y, instruction_38.z, r2.w);
    // sample r3.xyzw, v5.xyxx, t0.xyzw, s1
    let instruction_39 = textureSample(texture_t0, sampler_t0, ((input.v5).xyxx).xy);
    r3 = instruction_39;
    // add r3.yz, cb1[26].zzzz, l(0.000000, 0.060000, -0.030000, 0.000000)
    let instruction_40 = ((scripts.vectors[3]).zzzz) + (vec4<f32>(0.0, 0.05999999865889549, -0.029999999329447746, 0.0));
    r3 = vec4<f32>(r3.x, instruction_40.y, instruction_40.z, r3.w);
    // add r3.xy, -r3.yzyy, r3.xxxx
    let instruction_41 = (-((r3).yzyy)) + ((r3).xxxx);
    r3 = vec4<f32>(instruction_41.x, instruction_41.y, r3.z, r3.w);
    // mul_sat r3.xy, r3.xyxx, l(-8.333333, 16.666666, 0.000000, 0.000000)
    let instruction_42 = clamp(((r3).xyxx) * (vec4<f32>(-8.333333015441895, 16.66666603088379, 0.0, 0.0)), vec4<f32>(0.0), vec4<f32>(1.0));
    r3 = vec4<f32>(instruction_42.x, instruction_42.y, r3.z, r3.w);
    // mad r3.zw, r3.xxxy, l(0.000000, 0.000000, -2.000000, -2.000000), l(0.000000, 0.000000, 3.000000, 3.000000)
    let instruction_43 = fma((r3).xxxy, vec4<f32>(0.0, 0.0, -2.0, -2.0), vec4<f32>(0.0, 0.0, 3.0, 3.0));
    r3 = vec4<f32>(r3.x, r3.y, instruction_43.z, instruction_43.w);
    // mul r3.xy, r3.xyxx, r3.xyxx
    let instruction_44 = ((r3).xyxx) * ((r3).xyxx);
    r3 = vec4<f32>(instruction_44.x, instruction_44.y, r3.z, r3.w);
    // mul r3.xy, r3.xyxx, r3.zwzz
    let instruction_45 = ((r3).xyxx) * ((r3).zwzz);
    r3 = vec4<f32>(instruction_45.x, instruction_45.y, r3.z, r3.w);
    // mad r1.xyz, r3.xxxx, r2.xyzx, r1.xyzx
    let instruction_46 = fma((r3).xxxx, (r2).xyzx, (r1).xyzx);
    r1 = vec4<f32>(instruction_46.x, instruction_46.y, instruction_46.z, r1.w);
    // mul r2.xyz, r1.xyzx, r1.xyzx
    let instruction_47 = ((r1).xyzx) * ((r1).xyzx);
    r2 = vec4<f32>(instruction_47.x, instruction_47.y, instruction_47.z, r2.w);
    // mad r1.xyz, -r1.xyzx, r1.xyzx, l(1.000000, 1.000000, 1.000000, 0.000000)
    let instruction_48 = fma(-((r1).xyzx), (r1).xyzx, vec4<f32>(1.0, 1.0, 1.0, 0.0));
    r1 = vec4<f32>(instruction_48.x, instruction_48.y, instruction_48.z, r1.w);
    // add r1.w, r2.w, l(-1.000000)
    let instruction_49 = ((r2).wwww) + (vec4<f32>(-1.0));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_49.w);
    // mad r0.w, r0.w, r1.w, l(1.000000)
    let instruction_50 = fma((r0).wwww, (r1).wwww, vec4<f32>(1.0));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_50.w);
    // add r1.w, -r0.w, r2.w
    let instruction_51 = (-((r0).wwww)) + ((r2).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_51.w);
    // mad r0.w, r3.x, r1.w, r0.w
    let instruction_52 = fma((r3).xxxx, (r1).wwww, (r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_52.w);
    // mad r4.xyzw, r0.wwww, l(1.041667, 0.475000, 0.018229, 0.250000), l(0.000000, 0.000000, -0.015625, 0.750000)
    let instruction_53 = fma((r0).wwww, vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25), vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    r4 = instruction_53;
    // mul r3.zw, r0.xxxy, cb2[8].xxxx
    let instruction_54 = ((r0).xxxy) * ((material.source[3]).xxxx);
    r3 = vec4<f32>(r3.x, r3.y, instruction_54.z, instruction_54.w);
    // sample r5.xyzw, v5.xyxx, t1.xyzw, s2
    let instruction_55 = textureSample(texture_t1, sampler_t1, ((input.v5).xyxx).xy);
    r5 = instruction_55;
    // mad r5.xy, r5.xyxx, l(4.015748, 4.015748, 0.000000, 0.000000), l(-2.015748, -2.015748, 0.000000, 0.000000)
    let instruction_56 = fma((r5).xyxx, vec4<f32>(4.015748023986816, 4.015748023986816, 0.0, 0.0), vec4<f32>(-2.0157480239868164, -2.0157480239868164, 0.0, 0.0));
    r5 = vec4<f32>(instruction_56.x, instruction_56.y, r5.z, r5.w);
    // mad r3.zw, r5.xxxy, r0.zzzz, r3.zzzw
    let instruction_57 = fma((r5).xxxy, (r0).zzzz, (r3).zzzw);
    r3 = vec4<f32>(r3.x, r3.y, instruction_57.z, instruction_57.w);
    // mad_sat r0.xy, -r0.xyxx, r0.zzzz, v5.xyxx
    let instruction_58 = clamp(fma(-((r0).xyxx), (r0).zzzz, (input.v5).xyxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_58.x, instruction_58.y, r0.z, r0.w);
    // sample r6.xyzw, r0.xyxx, t4.xyzw, s5
    let instruction_59 = textureSample(texture_t4, sampler_t4, ((r0).xyxx).xy);
    r6 = instruction_59;
    // mul r0.xyz, r6.xyzx, l(0.733418, 0.788225, 0.892857, 0.000000)
    let instruction_60 = ((r6).xyzx) * (vec4<f32>(0.7334179878234863, 0.788224995136261, 0.892857015132904, 0.0));
    r0 = vec4<f32>(instruction_60.x, instruction_60.y, instruction_60.z, r0.w);
    // add r5.xy, -r3.zwzz, r5.xyxx
    let instruction_61 = (-((r3).zwzz)) + ((r5).xyxx);
    r5 = vec4<f32>(instruction_61.x, instruction_61.y, r5.z, r5.w);
    // mad r3.zw, r3.xxxx, r5.xxxy, r3.zzzw
    let instruction_62 = fma((r3).xxxx, (r5).xxxy, (r3).zzzw);
    r3 = vec4<f32>(r3.x, r3.y, instruction_62.z, instruction_62.w);
    // dp3 r1.w, v3.xyzx, v3.xyzx
    let instruction_63 = vec4<f32>(dot(((input.v3).xyzx).xyz, ((input.v3).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_63.w);
    // rsq r1.w, r1.w
    let instruction_64 = inverseSqrt((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_64.w);
    // mul r5.xyz, r1.wwww, v3.xyzx
    let instruction_65 = ((r1).wwww) * ((input.v3).xyzx);
    r5 = vec4<f32>(instruction_65.x, instruction_65.y, instruction_65.z, r5.w);
    // mad r6.xyz, r3.zzzz, v4.xyzx, r5.xyzx
    let instruction_66 = fma((r3).zzzz, (input.v4).xyzx, (r5).xyzx);
    r6 = vec4<f32>(instruction_66.x, instruction_66.y, instruction_66.z, r6.w);
    // mul r7.xyz, r5.zxyz, v4.yzxy
    let instruction_67 = ((r5).zxyz) * ((input.v4).yzxy);
    r7 = vec4<f32>(instruction_67.x, instruction_67.y, instruction_67.z, r7.w);
    // mad r5.xyz, r5.yzxy, v4.zxyz, -r7.xyzx
    let instruction_68 = fma((r5).yzxy, (input.v4).zxyz, -((r7).xyzx));
    r5 = vec4<f32>(instruction_68.x, instruction_68.y, instruction_68.z, r5.w);
    // mul r5.xyz, r5.xyzx, v4.wwww
    let instruction_69 = ((r5).xyzx) * ((input.v4).wwww);
    r5 = vec4<f32>(instruction_69.x, instruction_69.y, instruction_69.z, r5.w);
    // mad r5.xyz, r3.wwww, r5.xyzx, r6.xyzx
    let instruction_70 = fma((r3).wwww, (r5).xyzx, (r6).xyzx);
    r5 = vec4<f32>(instruction_70.x, instruction_70.y, instruction_70.z, r5.w);
    // dp3 r1.w, r5.xyzx, r5.xyzx
    let instruction_71 = vec4<f32>(dot(((r5).xyzx).xyz, ((r5).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_71.w);
    // rsq r1.w, r1.w
    let instruction_72 = inverseSqrt((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_72.w);
    // mul r5.xyz, r1.wwww, r5.xyzx
    let instruction_73 = ((r1).wwww) * ((r5).xyzx);
    r5 = vec4<f32>(instruction_73.x, instruction_73.y, instruction_73.z, r5.w);
    // dp3 r1.w, v1.xyzx, v1.xyzx
    let instruction_74 = vec4<f32>(dot(((input.v1).xyzx).xyz, ((input.v1).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_74.w);
    // rsq r1.w, r1.w
    let instruction_75 = inverseSqrt((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_75.w);
    // mul r6.xyz, r1.wwww, v1.xyzx
    let instruction_76 = ((r1).wwww) * ((input.v1).xyzx);
    r6 = vec4<f32>(instruction_76.x, instruction_76.y, instruction_76.z, r6.w);
    // mad r7.xyz, -v1.xyzx, r1.wwww, cb0[18].xyzx
    let instruction_77 = fma(-((input.v1).xyzx), (r1).wwww, (scene.world.sun_position).xyzx);
    r7 = vec4<f32>(instruction_77.x, instruction_77.y, instruction_77.z, r7.w);
    // dp3_sat r1.w, r5.xyzx, -r6.xyzx
    let instruction_78 = clamp(vec4<f32>(dot(((r5).xyzx).xyz, (-((r6).xyzx)).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_78.w);
    // mul r1.w, r1.w, l(-9.280000)
    let instruction_79 = ((r1).wwww) * (vec4<f32>(-9.279999732971191));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_79.w);
    // exp r1.w, r1.w
    let instruction_80 = exp2((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_80.w);
    // min r1.w, r1.w, r4.y
    let instruction_81 = min((r1).wwww, (r4).yyyy);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_81.w);
    // mad r1.w, r4.x, r1.w, r4.z
    let instruction_82 = fma((r4).xxxx, (r1).wwww, (r4).zzzz);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_82.w);
    // add r2.w, -r1.w, r4.w
    let instruction_83 = (-((r1).wwww)) + ((r4).wwww);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_83.w);
    // mad_sat r4.xyz, r2.xyzx, r2.wwww, r1.wwww
    let instruction_84 = clamp(fma((r2).xyzx, (r2).wwww, (r1).wwww), vec4<f32>(0.0), vec4<f32>(1.0));
    r4 = vec4<f32>(instruction_84.x, instruction_84.y, instruction_84.z, r4.w);
    // dp3 r1.w, r6.xyzx, r5.xyzx
    let instruction_85 = vec4<f32>(dot(((r6).xyzx).xyz, ((r5).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_85.w);
    // add r1.w, r1.w, r1.w
    let instruction_86 = ((r1).wwww) + ((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_86.w);
    // mad r6.xyz, r5.xyzx, -r1.wwww, r6.xyzx
    let instruction_87 = fma((r5).xyzx, -((r1).wwww), (r6).xyzx);
    r6 = vec4<f32>(instruction_87.x, instruction_87.y, instruction_87.z, r6.w);
    // mad r1.w, r0.w, l(-4.000000), l(4.000000)
    let instruction_88 = fma((r0).wwww, vec4<f32>(-4.0), vec4<f32>(4.0));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_88.w);
    // sample_l r6.xyzw, r6.xyzx, t15.xyzw, s15, r1.w
    let instruction_89 = textureSampleLevel(reflection_probe, reflection_sampler, ((r6).xyzx).xyz, ((r1).wwww).x);
    r6 = instruction_89;
    // add r1.w, r6.w, l(0.000001)
    let instruction_90 = ((r6).wwww) + (vec4<f32>(9.999999974752427e-07));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_90.w);
    // div r1.w, l(1.000000, 1.000000, 1.000000, 1.000000), r1.w
    let instruction_91 = (vec4<f32>(1.0, 1.0, 1.0, 1.0)) / ((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_91.w);
    // mul r6.xyz, r1.wwww, r6.xyzx
    let instruction_92 = ((r1).wwww) * ((r6).xyzx);
    r6 = vec4<f32>(instruction_92.x, instruction_92.y, instruction_92.z, r6.w);
    // mul r4.xyz, r4.xyzx, r6.xyzx
    let instruction_93 = ((r4).xyzx) * ((r6).xyzx);
    r4 = vec4<f32>(instruction_93.x, instruction_93.y, instruction_93.z, r4.w);
    // mul r4.xyz, r4.xyzx, cb2[7].wwww
    let instruction_94 = ((r4).xyzx) * ((material.source[2]).wwww);
    r4 = vec4<f32>(instruction_94.x, instruction_94.y, instruction_94.z, r4.w);
    // dp3 r1.w, r7.xyzx, r7.xyzx
    let instruction_95 = vec4<f32>(dot(((r7).xyzx).xyz, ((r7).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_95.w);
    // rsq r1.w, r1.w
    let instruction_96 = inverseSqrt((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_96.w);
    // mul r6.xyz, r1.wwww, r7.xyzx
    let instruction_97 = ((r1).wwww) * ((r7).xyzx);
    r6 = vec4<f32>(instruction_97.x, instruction_97.y, instruction_97.z, r6.w);
    // dp3_sat r1.w, r6.xyzx, cb0[18].xyzx
    let instruction_98 = clamp(vec4<f32>(dot(((r6).xyzx).xyz, ((scene.world.sun_position).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_98.w);
    // dp3_sat r2.w, r5.xyzx, r6.xyzx
    let instruction_99 = clamp(vec4<f32>(dot(((r5).xyzx).xyz, ((r6).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_99.w);
    // log r2.w, r2.w
    let instruction_100 = log2((r2).wwww);
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_100.w);
    // mul r3.z, r1.w, l(-10.000000)
    let instruction_101 = ((r1).wwww) * (vec4<f32>(-10.0));
    r3 = vec4<f32>(r3.x, r3.y, instruction_101.z, r3.w);
    // mul r1.w, r1.w, r1.w
    let instruction_102 = ((r1).wwww) * ((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_102.w);
    // exp r3.z, r3.z
    let instruction_103 = exp2((r3).zzzz);
    r3 = vec4<f32>(r3.x, r3.y, instruction_103.z, r3.w);
    // mad r1.xyz, r1.xyzx, r3.zzzz, r2.xyzx
    let instruction_104 = fma((r1).xyzx, (r3).zzzz, (r2).xyzx);
    r1 = vec4<f32>(instruction_104.x, instruction_104.y, instruction_104.z, r1.w);
    // add_sat r2.x, r0.w, l(0.545000)
    let instruction_105 = clamp(((r0).wwww) + (vec4<f32>(0.5450000166893005)), vec4<f32>(0.0), vec4<f32>(1.0));
    r2 = vec4<f32>(instruction_105.x, r2.y, r2.z, r2.w);
    // mul r0.w, r0.w, l(13.000000)
    let instruction_106 = ((r0).wwww) * (vec4<f32>(13.0));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_106.w);
    // exp r0.w, r0.w
    let instruction_107 = exp2((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_107.w);
    // mad r1.w, r1.w, r2.x, -r2.x
    let instruction_108 = fma((r1).wwww, (r2).xxxx, -((r2).xxxx));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_108.w);
    // add r1.w, r1.w, l(1.010000)
    let instruction_109 = ((r1).wwww) + (vec4<f32>(1.0099999904632568));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_109.w);
    // dp3_sat r2.x, cb0[18].xyzx, r5.xyzx
    let instruction_110 = clamp(vec4<f32>(dot(((scene.world.sun_position).xyzx).xyz, ((r5).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r2 = vec4<f32>(instruction_110.x, r2.y, r2.z, r2.w);
    // div r1.w, r2.x, r1.w
    let instruction_111 = ((r2).xxxx) / ((r1).wwww);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_111.w);
    // mad r2.y, r0.w, l(0.125000), l(0.250000)
    let instruction_112 = fma((r0).wwww, vec4<f32>(0.125), vec4<f32>(0.25));
    r2 = vec4<f32>(r2.x, instruction_112.y, r2.z, r2.w);
    // mul r0.w, r2.w, r0.w
    let instruction_113 = ((r2).wwww) * ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_113.w);
    // exp r0.w, r0.w
    let instruction_114 = exp2((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_114.w);
    // mul r0.w, r0.w, r2.y
    let instruction_115 = ((r0).wwww) * ((r2).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_115.w);
    // mul r0.w, r1.w, r0.w
    let instruction_116 = ((r1).wwww) * ((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_116.w);
    // mul r1.xyz, r1.xyzx, r0.wwww
    let instruction_117 = ((r1).xyzx) * ((r0).wwww);
    r1 = vec4<f32>(instruction_117.x, instruction_117.y, instruction_117.z, r1.w);
    // mul r2.yzw, v6.wwww, cb0[19].xxyz
    let instruction_118 = ((input.v6).wwww) * ((scene.world.sun_diffuse).xxyz);
    r2 = vec4<f32>(r2.x, instruction_118.y, instruction_118.z, instruction_118.w);
    // mul r1.xyz, r1.xyzx, r2.yzwy
    let instruction_119 = ((r1).xyzx) * ((r2).yzwy);
    r1 = vec4<f32>(instruction_119.x, instruction_119.y, instruction_119.z, r1.w);
    // mul r1.xyz, r1.xyzx, l(0.250000, 0.250000, 0.250000, 0.000000)
    let instruction_120 = ((r1).xyzx) * (vec4<f32>(0.25, 0.25, 0.25, 0.0));
    r1 = vec4<f32>(instruction_120.x, instruction_120.y, instruction_120.z, r1.w);
    // mad r1.xyz, r4.xyzx, v7.xyzx, r1.xyzx
    let instruction_121 = fma((r4).xyzx, (input.v7).xyzx, (r1).xyzx);
    r1 = vec4<f32>(instruction_121.x, instruction_121.y, instruction_121.z, r1.w);
    // sample r4.xyzw, v5.xyxx, t4.xyzw, s5
    let instruction_122 = textureSample(texture_t4, sampler_t4, ((input.v5).xyxx).xy);
    r4 = instruction_122;
    // mad r4.xyz, cb2[7].xyzx, r4.xxxx, -r0.xyzx
    let instruction_123 = fma((material.source[2]).xyzx, (r4).xxxx, -((r0).xyzx));
    r4 = vec4<f32>(instruction_123.x, instruction_123.y, instruction_123.z, r4.w);
    // mad r0.xyz, r3.xxxx, r4.xyzx, r0.xyzx
    let instruction_124 = fma((r3).xxxx, (r4).xyzx, (r0).xyzx);
    r0 = vec4<f32>(instruction_124.x, instruction_124.y, instruction_124.z, r0.w);
    // mul r0.w, r3.y, r3.x
    let instruction_125 = ((r3).yyyy) * ((r3).xxxx);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_125.w);
    // mul r3.xyz, r0.wwww, cb2[6].xyzx
    let instruction_126 = ((r0).wwww) * ((material.source[1]).xyzx);
    r3 = vec4<f32>(instruction_126.x, instruction_126.y, instruction_126.z, r3.w);
    // mul r3.xyz, r3.xyzx, cb2[5].yyyy
    let instruction_127 = ((r3).xyzx) * ((material.source[0]).yyyy);
    r3 = vec4<f32>(instruction_127.x, instruction_127.y, instruction_127.z, r3.w);
    // mul r0.xyz, r0.xyzx, v2.xyzx
    let instruction_128 = ((r0).xyzx) * ((input.v2).xyzx);
    r0 = vec4<f32>(instruction_128.x, instruction_128.y, instruction_128.z, r0.w);
    // mul r0.xyz, r0.xyzx, r0.xyzx
    let instruction_129 = ((r0).xyzx) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_129.x, instruction_129.y, instruction_129.z, r0.w);
    // max r0.w, |r5.z|, |r5.y|
    let instruction_130 = max(abs((r5).zzzz), abs((r5).yyyy));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_130.w);
    // max r0.w, r0.w, |r5.x|
    let instruction_131 = max((r0).wwww, abs((r5).xxxx));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_131.w);
    // mul r4.xyz, r5.xyzx, cb0[56].xyzx
    let instruction_132 = ((r5).xyzx) * ((scene.lighting_lookup_scale).xyzx);
    r4 = vec4<f32>(instruction_132.x, instruction_132.y, instruction_132.z, r4.w);
    // div r4.xyz, r4.xyzx, r0.wwww
    let instruction_133 = ((r4).xyzx) / ((r0).wwww);
    r4 = vec4<f32>(instruction_133.x, instruction_133.y, instruction_133.z, r4.w);
    // add r4.xyz, r4.xyzx, v6.xyzx
    let instruction_134 = ((r4).xyzx) + ((input.v6).xyzx);
    r4 = vec4<f32>(instruction_134.x, instruction_134.y, instruction_134.z, r4.w);
    // sample r4.xyzw, r4.xyzx, t13.xyzw, s13
    let instruction_135 = textureSample(model_lighting, model_lighting_sampler, ((r4).xyzx).xyz);
    r4 = instruction_135;
    // mul r4.xyz, r4.xyzx, r4.xyzx
    let instruction_136 = ((r4).xyzx) * ((r4).xyzx);
    r4 = vec4<f32>(instruction_136.x, instruction_136.y, instruction_136.z, r4.w);
    // mul r4.xyz, r4.xyzx, l(32.000000, 32.000000, 32.000000, 0.000000)
    let instruction_137 = ((r4).xyzx) * (vec4<f32>(32.0, 32.0, 32.0, 0.0));
    r4 = vec4<f32>(instruction_137.x, instruction_137.y, instruction_137.z, r4.w);
    // mad r2.xyz, r2.xxxx, r2.yzwy, r4.xyzx
    let instruction_138 = fma((r2).xxxx, (r2).yzwy, (r4).xyzx);
    r2 = vec4<f32>(instruction_138.x, instruction_138.y, instruction_138.z, r2.w);
    // mad r0.xyz, r0.xyzx, r2.xyzx, r1.xyzx
    let instruction_139 = fma((r0).xyzx, (r2).xyzx, (r1).xyzx);
    r0 = vec4<f32>(instruction_139.x, instruction_139.y, instruction_139.z, r0.w);
    // mad r0.xyz, r3.xyzx, r3.xyzx, r0.xyzx
    let instruction_140 = fma((r3).xyzx, (r3).xyzx, (r0).xyzx);
    r0 = vec4<f32>(instruction_140.x, instruction_140.y, instruction_140.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_141 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_141.w);
    // dp4 r1.x, r0.xyzw, cb0[114].xyzw
    let instruction_142 = vec4<f32>(dot(((r0).xyzw).xyzw, ((view.hero_lighting[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_142.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb0[115].xyzw
    let instruction_143 = vec4<f32>(dot(((r0).xyzw).xyzw, ((view.hero_lighting[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_143.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb0[116].xyzw
    let instruction_144 = vec4<f32>(dot(((r0).xyzw).xyzw, ((view.hero_lighting[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_144.z, r1.w);
    // add r0.xyz, r1.xyzx, -v0.xyzx
    let instruction_145 = ((r1).xyzx) + (-((input.v0).xyzx));
    r0 = vec4<f32>(instruction_145.x, instruction_145.y, instruction_145.z, r0.w);
    // mad r0.xyz, v1.wwww, r0.xyzx, v0.xyzx
    let instruction_146 = fma((input.v1).wwww, (r0).xyzx, (input.v0).xyzx);
    r0 = vec4<f32>(instruction_146.x, instruction_146.y, instruction_146.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_147 = ((r0).xyzx) * ((scene.world.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_147.x, instruction_147.y, instruction_147.z, r0.w);
    // sqrt o0.xyz, r0.xyzx
    let instruction_148 = sqrt((r0).xyzx);
    o0 = vec4<f32>(instruction_148.x, instruction_148.y, instruction_148.z, o0.w);
    // mov o0.w, l(1.000000)
    let instruction_149 = vec4<f32>(1.0);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_149.w);
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
