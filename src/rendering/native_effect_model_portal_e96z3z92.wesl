// Original mc_sw4_3d_portal_e96z3z92; immutable DXBC hashes in effect source-inventory.json.
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
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(2) v2: vec4<f32>,
    @location(3) v3: vec4<f32>,
}
@vertex
fn vertex(input: Input) -> Varyings {
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
    // add r0.xy, -v2.xyxx, l(1.000000, 1.000000, 0.000000, 0.000000)
    let instruction_0 = (-((vec4<f32>(input.uv, 0.0, 0.0)).xyxx)) + (vec4<f32>(1.0, 1.0, 0.0, 0.0));
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, r0.z, r0.w);
    // mul r1.xyzw, r0.xyxy, l(3.623052, 0.253348, 8.250306, -1.454752)
    let instruction_1 = ((r0).xyxy) * (vec4<f32>(3.623051881790161, 0.2533484697341919, 8.250306129455566, -1.4547516107559204));
    r1 = instruction_1;
    // mul r0.xy, r0.xyxx, l(19.577713, 5.245832, 0.000000, 0.000000)
    let instruction_2 = ((r0).xyxx) * (vec4<f32>(19.577713012695312, 5.245832443237305, 0.0, 0.0));
    r0 = vec4<f32>(instruction_2.x, instruction_2.y, r0.z, r0.w);
    // add r0.x, r0.y, r0.x
    let instruction_3 = ((r0).yyyy) + ((r0).xxxx);
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // add r0.yz, r1.yywy, r1.xxzx
    let instruction_4 = ((r1).yywy) + ((r1).xxzx);
    r0 = vec4<f32>(r0.x, instruction_4.y, instruction_4.z, r0.w);
    // mul r0.w, cb2[4].w, cb2[16].z
    let instruction_5 = ((scene.world.game_time).wwww) * ((material.source[11]).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_5.w);
    // mad r0.yz, r0.wwww, l(0.000000, 5.654867, 9.424778, 0.000000), r0.yyzy
    let instruction_6 = fma((r0).wwww, vec4<f32>(0.0, 5.654866695404053, 9.42477798461914, 0.0), (r0).yyzy);
    r0 = vec4<f32>(r0.x, instruction_6.y, instruction_6.z, r0.w);
    // mad r0.x, r0.w, l(7.539823), r0.x
    let instruction_7 = fma((r0).wwww, vec4<f32>(7.539822578430176), (r0).xxxx);
    r0 = vec4<f32>(instruction_7.x, r0.y, r0.z, r0.w);
    // sincos r0.x, r1.x, r0.x
    let angle8 = (r0).xxxx;
    let instruction_8 = sin(angle8);
    r0 = vec4<f32>(instruction_8.x, r0.y, r0.z, r0.w);
    let instruction_9 = cos(angle8);
    r1 = vec4<f32>(instruction_9.x, r1.y, r1.z, r1.w);
    // sincos r2.xy, r3.xy, r0.yzyy
    let angle10 = (r0).yzyy;
    let instruction_10 = sin(angle10);
    r2 = vec4<f32>(instruction_10.x, instruction_10.y, r2.z, r2.w);
    let instruction_11 = cos(angle10);
    r3 = vec4<f32>(instruction_11.x, instruction_11.y, r3.z, r3.w);
    // mul r0.yz, r2.xxyx, l(0.000000, 7.500000, 3.000000, 0.000000)
    let instruction_12 = ((r2).xxyx) * (vec4<f32>(0.0, 7.5, 3.0, 0.0));
    r0 = vec4<f32>(r0.x, instruction_12.y, instruction_12.z, r0.w);
    // mul r2.xyzw, r3.xxyy, l(27.172890, 1.900113, 24.750919, -4.364254)
    let instruction_13 = ((r3).xxyy) * (vec4<f32>(27.172889709472656, 1.9001134634017944, 24.750919342041016, -4.364254474639893));
    r2 = instruction_13;
    // add r1.yz, r2.zzwz, r2.xxyx
    let instruction_14 = ((r2).zzwz) + ((r2).xxyx);
    r1 = vec4<f32>(r1.x, instruction_14.y, instruction_14.z, r1.w);
    // mad r1.xy, r1.xxxx, l(33.282112, 8.917915, 0.000000, 0.000000), r1.yzyy
    let instruction_15 = fma((r1).xxxx, vec4<f32>(33.28211212158203, 8.917915344238281, 0.0, 0.0), (r1).yzyy);
    r1 = vec4<f32>(instruction_15.x, instruction_15.y, r1.z, r1.w);
    // mul r1.xy, r1.xyxx, v1.wwww
    let instruction_16 = ((r1).xyxx) * ((input.color).wwww);
    r1 = vec4<f32>(instruction_16.x, instruction_16.y, r1.z, r1.w);
    // mul r1.xy, r1.xyxx, l(0.025000, 0.025000, 0.000000, 0.000000)
    let instruction_17 = ((r1).xyxx) * (vec4<f32>(0.02500000037252903, 0.02500000037252903, 0.0, 0.0));
    r1 = vec4<f32>(instruction_17.x, instruction_17.y, r1.z, r1.w);
    // add r0.y, r0.z, r0.y
    let instruction_18 = ((r0).zzzz) + ((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_18.y, r0.z, r0.w);
    // mad r0.x, r0.x, l(1.700000), r0.y
    let instruction_19 = fma((r0).xxxx, vec4<f32>(1.7000000476837158), (r0).yyyy);
    r0 = vec4<f32>(instruction_19.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, v1.w
    let instruction_20 = ((r0).xxxx) * ((input.color).wwww);
    r0 = vec4<f32>(instruction_20.x, r0.y, r0.z, r0.w);
    // add r0.yzw, v3.xxyz, v3.xxyz
    let instruction_21 = ((input.normal).xxyz) + ((input.normal).xxyz);
    r0 = vec4<f32>(r0.x, instruction_21.y, instruction_21.z, instruction_21.w);
    // ge r2.xyz, v3.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_22 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.normal).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r2 = vec4<f32>(instruction_22.x, instruction_22.y, instruction_22.z, r2.w);
    // mad r3.xyz, v3.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_23 = fma((input.normal).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r3 = vec4<f32>(instruction_23.x, instruction_23.y, instruction_23.z, r3.w);
    // movc r0.yzw, r2.xxyz, r3.xxyz, r0.yyzw
    let instruction_24 = select((r0).yyzw, (r3).xxyz, bitcast<vec4<u32>>((r2).xxyz) != vec4<u32>(0u));
    r0 = vec4<f32>(r0.x, instruction_24.y, instruction_24.z, instruction_24.w);
    // dp3 r2.x, r0.yzwy, cb3[0].xyzx
    let instruction_25 = vec4<f32>(dot(((r0).yzwy).xyz, ((object.world_rows[0]).xyzx).xyz));
    r2 = vec4<f32>(instruction_25.x, r2.y, r2.z, r2.w);
    // dp3 r2.y, r0.yzwy, cb3[1].xyzx
    let instruction_26 = vec4<f32>(dot(((r0).yzwy).xyz, ((object.world_rows[1]).xyzx).xyz));
    r2 = vec4<f32>(r2.x, instruction_26.y, r2.z, r2.w);
    // dp3 r2.z, r0.yzwy, cb3[2].xyzx
    let instruction_27 = vec4<f32>(dot(((r0).yzwy).xyz, ((object.world_rows[2]).xyzx).xyz));
    r2 = vec4<f32>(r2.x, r2.y, instruction_27.z, r2.w);
    // dp3 r0.y, r2.xyzx, r2.xyzx
    let instruction_28 = vec4<f32>(dot(((r2).xyzx).xyz, ((r2).xyzx).xyz));
    r0 = vec4<f32>(r0.x, instruction_28.y, r0.z, r0.w);
    // rsq r0.y, r0.y
    let instruction_29 = inverseSqrt((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_29.y, r0.z, r0.w);
    // mul r3.xyz, r0.yyyy, r2.xyzx
    let instruction_30 = ((r0).yyyy) * ((r2).xyzx);
    r3 = vec4<f32>(instruction_30.x, instruction_30.y, instruction_30.z, r3.w);
    // mov r4.xyz, v0.xyzx
    let instruction_31 = (input.position).xyzx;
    r4 = vec4<f32>(instruction_31.x, instruction_31.y, instruction_31.z, r4.w);
    // mov r4.w, l(1.000000)
    let instruction_32 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_32.w);
    // dp4 r5.x, r4.xyzw, cb3[0].xyzw
    let instruction_33 = vec4<f32>(dot(((r4).xyzw).xyzw, ((object.world_rows[0]).xyzw).xyzw));
    r5 = vec4<f32>(instruction_33.x, r5.y, r5.z, r5.w);
    // dp4 r5.y, r4.xyzw, cb3[1].xyzw
    let instruction_34 = vec4<f32>(dot(((r4).xyzw).xyzw, ((object.world_rows[1]).xyzw).xyzw));
    r5 = vec4<f32>(r5.x, instruction_34.y, r5.z, r5.w);
    // dp4 r5.z, r4.xyzw, cb3[2].xyzw
    let instruction_35 = vec4<f32>(dot(((r4).xyzw).xyzw, ((object.world_rows[2]).xyzw).xyzw));
    r5 = vec4<f32>(r5.x, r5.y, instruction_35.z, r5.w);
    // mad r4.xyz, r0.xxxx, r3.xyzx, r5.xyzx
    let instruction_36 = fma((r0).xxxx, (r3).xyzx, (r5).xyzx);
    r4 = vec4<f32>(instruction_36.x, instruction_36.y, instruction_36.z, r4.w);
    // mov o0.xyz, r4.xyzx
    let instruction_37 = (r4).xyzx;
    o0 = vec4<f32>(instruction_37.x, instruction_37.y, instruction_37.z, o0.w);
    // mov o1.xy, v2.xyxx
    let instruction_38 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o1 = vec4<f32>(instruction_38.x, instruction_38.y, o1.z, o1.w);
    // add r0.xzw, v4.xxyz, v4.xxyz
    let instruction_39 = ((input.tangent).xxyz) + ((input.tangent).xxyz);
    r0 = vec4<f32>(instruction_39.x, r0.y, instruction_39.z, instruction_39.w);
    // ge r5.xyz, v4.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_40 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.tangent).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r5 = vec4<f32>(instruction_40.x, instruction_40.y, instruction_40.z, r5.w);
    // mad r6.xyz, v4.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_41 = fma((input.tangent).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r6 = vec4<f32>(instruction_41.x, instruction_41.y, instruction_41.z, r6.w);
    // movc r0.xzw, r5.xxyz, r6.xxyz, r0.xxzw
    let instruction_42 = select((r0).xxzw, (r6).xxyz, bitcast<vec4<u32>>((r5).xxyz) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_42.x, r0.y, instruction_42.z, instruction_42.w);
    // dp3 r5.x, r0.xzwx, cb3[0].xyzx
    let instruction_43 = vec4<f32>(dot(((r0).xzwx).xyz, ((object.world_rows[0]).xyzx).xyz));
    r5 = vec4<f32>(instruction_43.x, r5.y, r5.z, r5.w);
    // dp3 r5.y, r0.xzwx, cb3[1].xyzx
    let instruction_44 = vec4<f32>(dot(((r0).xzwx).xyz, ((object.world_rows[1]).xyzx).xyz));
    r5 = vec4<f32>(r5.x, instruction_44.y, r5.z, r5.w);
    // dp3 r5.z, r0.xzwx, cb3[2].xyzx
    let instruction_45 = vec4<f32>(dot(((r0).xzwx).xyz, ((object.world_rows[2]).xyzx).xyz));
    r5 = vec4<f32>(r5.x, r5.y, instruction_45.z, r5.w);
    // dp3 r0.x, r5.xyzx, r5.xyzx
    let instruction_46 = vec4<f32>(dot(((r5).xyzx).xyz, ((r5).xyzx).xyz));
    r0 = vec4<f32>(instruction_46.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_47 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_47.x, r0.y, r0.z, r0.w);
    // mul r0.xzw, r0.xxxx, r5.xxyz
    let instruction_48 = ((r0).xxxx) * ((r5).xxyz);
    r0 = vec4<f32>(instruction_48.x, r0.y, instruction_48.z, instruction_48.w);
    // mul r5.xyz, r0.zwxz, r3.zxyz
    let instruction_49 = ((r0).zwxz) * ((r3).zxyz);
    r5 = vec4<f32>(instruction_49.x, instruction_49.y, instruction_49.z, r5.w);
    // mad r3.xyz, r3.yzxy, r0.wxzw, -r5.xyzx
    let instruction_50 = fma((r3).yzxy, (r0).wxzw, -((r5).xyzx));
    r3 = vec4<f32>(instruction_50.x, instruction_50.y, instruction_50.z, r3.w);
    // lt r1.z, l(0.000000), v0.w
    let instruction_51 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0)) < ((input.position).wwww)));
    r1 = vec4<f32>(r1.x, r1.y, instruction_51.z, r1.w);
    // lt r1.w, v0.w, l(0.000000)
    let instruction_52 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.position).wwww) < (vec4<f32>(0.0))));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_52.w);
    // iadd r1.z, r1.w, -r1.z
    let instruction_53 = bitcast<vec4<f32>>(bitcast<vec4<i32>>((r1).wwww) + -(bitcast<vec4<i32>>((r1).zzzz)));
    r1 = vec4<f32>(r1.x, r1.y, instruction_53.z, r1.w);
    // itof r1.z, r1.z
    let instruction_54 = vec4<f32>(bitcast<vec4<i32>>((r1).zzzz));
    r1 = vec4<f32>(r1.x, r1.y, instruction_54.z, r1.w);
    // mul r3.xyz, r1.zzzz, r3.xyzx
    let instruction_55 = ((r1).zzzz) * ((r3).xyzx);
    r3 = vec4<f32>(instruction_55.x, instruction_55.y, instruction_55.z, r3.w);
    // mul r3.xyz, r1.yyyy, r3.xyzx
    let instruction_56 = ((r1).yyyy) * ((r3).xyzx);
    r3 = vec4<f32>(instruction_56.x, instruction_56.y, instruction_56.z, r3.w);
    // mad r1.xyw, r1.xxxx, r0.xzxw, r3.xyxz
    let instruction_57 = fma((r1).xxxx, (r0).xzxw, (r3).xyxz);
    r1 = vec4<f32>(instruction_57.x, instruction_57.y, r1.z, instruction_57.w);
    // mad r1.xyw, r2.xyxz, r0.yyyy, r1.xyxw
    let instruction_58 = fma((r2).xyxz, (r0).yyyy, (r1).xyxw);
    r1 = vec4<f32>(instruction_58.x, instruction_58.y, r1.z, instruction_58.w);
    // dp3 r0.y, r1.xywx, r1.xywx
    let instruction_59 = vec4<f32>(dot(((r1).xywx).xyz, ((r1).xywx).xyz));
    r0 = vec4<f32>(r0.x, instruction_59.y, r0.z, r0.w);
    // rsq r0.y, r0.y
    let instruction_60 = inverseSqrt((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_60.y, r0.z, r0.w);
    // mul r1.xyw, r0.yyyy, r1.xyxw
    let instruction_61 = ((r0).yyyy) * ((r1).xyxw);
    r1 = vec4<f32>(instruction_61.x, instruction_61.y, r1.z, instruction_61.w);
    // mul r2.xyz, r0.zwxz, r1.wxyw
    let instruction_62 = ((r0).zwxz) * ((r1).wxyw);
    r2 = vec4<f32>(instruction_62.x, instruction_62.y, instruction_62.z, r2.w);
    // mad r2.xyz, r1.ywxy, r0.wxzw, -r2.xyzx
    let instruction_63 = fma((r1).ywxy, (r0).wxzw, -((r2).xyzx));
    r2 = vec4<f32>(instruction_63.x, instruction_63.y, instruction_63.z, r2.w);
    // mul r2.xyz, r1.zzzz, r2.xyzx
    let instruction_64 = ((r1).zzzz) * ((r2).xyzx);
    r2 = vec4<f32>(instruction_64.x, instruction_64.y, instruction_64.z, r2.w);
    // dp3 r0.y, -r4.xyzx, -r4.xyzx
    let instruction_65 = vec4<f32>(dot((-((r4).xyzx)).xyz, (-((r4).xyzx)).xyz));
    r0 = vec4<f32>(r0.x, instruction_65.y, r0.z, r0.w);
    // rsq r0.y, r0.y
    let instruction_66 = inverseSqrt((r0).yyyy);
    r0 = vec4<f32>(r0.x, instruction_66.y, r0.z, r0.w);
    // mul r3.xyz, r0.yyyy, -r4.xyzx
    let instruction_67 = ((r0).yyyy) * (-((r4).xyzx));
    r3 = vec4<f32>(instruction_67.x, instruction_67.y, instruction_67.z, r3.w);
    // dp3 r2.w, r3.xyzx, r2.xyzx
    let instruction_68 = vec4<f32>(dot(((r3).xyzx).xyz, ((r2).xyzx).xyz));
    r2 = vec4<f32>(r2.x, r2.y, r2.z, instruction_68.w);
    // dp3 r0.y, r3.xyzx, r1.xywx
    let instruction_69 = vec4<f32>(dot(((r3).xyzx).xyz, ((r1).xywx).xyz));
    r0 = vec4<f32>(r0.x, instruction_69.y, r0.z, r0.w);
    // dp3 r2.z, r3.xyzx, r0.xzwx
    let instruction_70 = vec4<f32>(dot(((r3).xyzx).xyz, ((r0).xzwx).xyz));
    r2 = vec4<f32>(r2.x, r2.y, instruction_70.z, r2.w);
    // div r0.xz, -r2.zzwz, r0.yyyy
    let instruction_71 = (-((r2).zzwz)) / ((r0).yyyy);
    r0 = vec4<f32>(instruction_71.x, r0.y, instruction_71.z, r0.w);
    // lt r0.y, l(0.000000), r0.y
    let instruction_72 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (vec4<f32>(0.0)) < ((r0).yyyy)));
    r0 = vec4<f32>(r0.x, instruction_72.y, r0.z, r0.w);
    // and r0.xy, r0.xzxx, r0.yyyy
    let instruction_73 = bitcast<vec4<f32>>(bitcast<vec4<u32>>((r0).xzxx) & bitcast<vec4<u32>>((r0).yyyy));
    r0 = vec4<f32>(instruction_73.x, instruction_73.y, r0.z, r0.w);
    // add r0.z, cb2[11].w, l(0.000010)
    let instruction_74 = ((material.source[6]).wwww) + (vec4<f32>(9.999999747378752e-06));
    r0 = vec4<f32>(r0.x, r0.y, instruction_74.z, r0.w);
    // mul r0.zw, r0.zzzz, r0.xxxy
    let instruction_75 = ((r0).zzzz) * ((r0).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_75.z, instruction_75.w);
    // mul r0.zw, r0.zzzw, cb2[10].zzzw
    let instruction_76 = ((r0).zzzw) * ((material.source[5]).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_76.z, instruction_76.w);
    // add r1.xy, v2.xyxx, l(-0.500000, -0.500000, 0.000000, 0.000000)
    let instruction_77 = ((vec4<f32>(input.uv, 0.0, 0.0)).xyxx) + (vec4<f32>(-0.5, -0.5, 0.0, 0.0));
    r1 = vec4<f32>(instruction_77.x, instruction_77.y, r1.z, r1.w);
    // mad r0.zw, r1.xxxy, cb2[10].zzzw, r0.zzzw
    let instruction_78 = fma((r1).xxxy, (material.source[5]).zzzw, (r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_78.z, instruction_78.w);
    // mad r0.zw, cb2[11].xxxy, cb2[10].zzzw, r0.zzzw
    let instruction_79 = fma((material.source[6]).xxxy, (material.source[5]).zzzw, (r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_79.z, instruction_79.w);
    // add o2.xy, r0.zwzz, l(0.500000, 0.500000, 0.000000, 0.000000)
    let instruction_80 = ((r0).zwzz) + (vec4<f32>(0.5, 0.5, 0.0, 0.0));
    o2 = vec4<f32>(instruction_80.x, instruction_80.y, o2.z, o2.w);
    // add r2.xyzw, cb2[10].yyxx, l(0.000010, 0.000010, 0.000010, 0.000010)
    let instruction_81 = ((material.source[5]).yyxx) + (vec4<f32>(9.999999747378752e-06, 9.999999747378752e-06, 9.999999747378752e-06, 9.999999747378752e-06));
    r2 = instruction_81;
    // mul r2.xyzw, r0.xyxy, r2.xyzw
    let instruction_82 = ((r0).xyxy) * ((r2).xyzw);
    r2 = instruction_82;
    // mul r0.xy, r0.xyxx, cb2[5].xyxx
    let instruction_83 = ((r0).xyxx) * ((material.source[0]).xyxx);
    r0 = vec4<f32>(instruction_83.x, instruction_83.y, r0.z, r0.w);
    // mul r0.xy, r0.xyxx, l(0.000010, 0.000010, 0.000000, 0.000000)
    let instruction_84 = ((r0).xyxx) * (vec4<f32>(9.999999747378752e-06, 9.999999747378752e-06, 0.0, 0.0));
    r0 = vec4<f32>(instruction_84.x, instruction_84.y, r0.z, r0.w);
    // mad r0.xy, r1.xyxx, cb2[5].xyxx, r0.xyxx
    let instruction_85 = fma((r1).xyxx, (material.source[0]).xyxx, (r0).xyxx);
    r0 = vec4<f32>(instruction_85.x, instruction_85.y, r0.z, r0.w);
    // mad r0.xy, cb2[5].zwzz, cb2[5].xyxx, r0.xyxx
    let instruction_86 = fma((material.source[0]).zwzz, (material.source[0]).xyxx, (r0).xyxx);
    r0 = vec4<f32>(instruction_86.x, instruction_86.y, r0.z, r0.w);
    // mul r0.zw, r2.xxxy, cb2[7].zzzw
    let instruction_87 = ((r2).xxxy) * ((material.source[2]).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_87.z, instruction_87.w);
    // mul r1.zw, r2.zzzw, cb2[6].zzzw
    let instruction_88 = ((r2).zzzw) * ((material.source[1]).zzzw);
    r1 = vec4<f32>(r1.x, r1.y, instruction_88.z, instruction_88.w);
    // mad r1.zw, r1.xxxy, cb2[6].zzzw, r1.zzzw
    let instruction_89 = fma((r1).xxxy, (material.source[1]).zzzw, (r1).zzzw);
    r1 = vec4<f32>(r1.x, r1.y, instruction_89.z, instruction_89.w);
    // mad r0.zw, r1.xxxy, cb2[7].zzzw, r0.zzzw
    let instruction_90 = fma((r1).xxxy, (material.source[2]).zzzw, (r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_90.z, instruction_90.w);
    // mad r0.zw, cb2[8].xxxy, cb2[7].zzzw, r0.zzzw
    let instruction_91 = fma((material.source[3]).xxxy, (material.source[2]).zzzw, (r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_91.z, instruction_91.w);
    // mad r1.xy, cb2[6].zwzz, l(-0.060000, 0.000000, 0.000000, 0.000000), r1.zwzz
    let instruction_92 = fma((material.source[1]).zwzz, vec4<f32>(-0.05999999865889549, 0.0, 0.0, 0.0), (r1).zwzz);
    r1 = vec4<f32>(instruction_92.x, instruction_92.y, r1.z, r1.w);
    // mul r1.zw, cb2[4].wwww, cb2[8].zzzw
    let instruction_93 = ((scene.world.game_time).wwww) * ((material.source[3]).zzzw);
    r1 = vec4<f32>(r1.x, r1.y, instruction_93.z, instruction_93.w);
    // mad r0.zw, r1.zzzw, cb2[7].zzzw, r0.zzzw
    let instruction_94 = fma((r1).zzzw, (material.source[2]).zzzw, (r0).zzzw);
    r0 = vec4<f32>(r0.x, r0.y, instruction_94.z, instruction_94.w);
    // add o2.zw, r0.zzzw, l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_95 = ((r0).zzzw) + (vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o2 = vec4<f32>(o2.x, o2.y, instruction_95.z, instruction_95.w);
    // mul r0.zw, cb2[4].wwww, cb2[7].xxxy
    let instruction_96 = ((scene.world.game_time).wwww) * ((material.source[2]).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_96.z, instruction_96.w);
    // mad r0.zw, r0.zzzw, cb2[6].zzzw, r1.xxxy
    let instruction_97 = fma((r0).zzzw, (material.source[1]).zzzw, (r1).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_97.z, instruction_97.w);
    // add o3.xy, r0.zwzz, l(0.500000, 0.500000, 0.000000, 0.000000)
    let instruction_98 = ((r0).zwzz) + (vec4<f32>(0.5, 0.5, 0.0, 0.0));
    o3 = vec4<f32>(instruction_98.x, instruction_98.y, o3.z, o3.w);
    // mul r0.zw, cb2[4].wwww, cb2[6].xxxy
    let instruction_99 = ((scene.world.game_time).wwww) * ((material.source[1]).xxxy);
    r0 = vec4<f32>(r0.x, r0.y, instruction_99.z, instruction_99.w);
    // mad r0.xy, r0.zwzz, cb2[5].xyxx, r0.xyxx
    let instruction_100 = fma((r0).zwzz, (material.source[0]).xyxx, (r0).xyxx);
    r0 = vec4<f32>(instruction_100.x, instruction_100.y, r0.z, r0.w);
    // add o3.zw, r0.xxxy, l(0.000000, 0.000000, 0.500000, 0.500000)
    let instruction_101 = ((r0).xxxy) + (vec4<f32>(0.0, 0.0, 0.5, 0.5));
    o3 = vec4<f32>(o3.x, o3.y, instruction_101.z, instruction_101.w);
    // mov r4.w, l(1.000000)
    let instruction_102 = vec4<f32>(1.0);
    r4 = vec4<f32>(r4.x, r4.y, r4.z, instruction_102.w);
    // dp4 o4.x, r4.xyzw, cb0[36].xyzw
    let instruction_103 = vec4<f32>(dot(((r4).xyzw).xyzw, ((scene.world.view_projection_rows[0]).xyzw).xyzw));
    o4 = vec4<f32>(instruction_103.x, o4.y, o4.z, o4.w);
    // dp4 o4.y, r4.xyzw, cb0[37].xyzw
    let instruction_104 = vec4<f32>(dot(((r4).xyzw).xyzw, ((scene.world.view_projection_rows[1]).xyzw).xyzw));
    o4 = vec4<f32>(o4.x, instruction_104.y, o4.z, o4.w);
    // dp4 o4.z, r4.xyzw, cb0[38].xyzw
    let instruction_105 = vec4<f32>(dot(((r4).xyzw).xyzw, ((scene.world.view_projection_rows[2]).xyzw).xyzw));
    o4 = vec4<f32>(o4.x, o4.y, instruction_105.z, o4.w);
    // dp4 o4.w, r4.xyzw, cb0[39].xyzw
    let instruction_106 = vec4<f32>(dot(((r4).xyzw).xyzw, ((scene.world.view_projection_rows[3]).xyzw).xyzw));
    o4 = vec4<f32>(o4.x, o4.y, o4.z, instruction_106.w);
    var result: Varyings;
    result.position = o4;
    result.v2 = o2;
    result.v3 = o3;
    return result;
}
fn shade(input: Varyings) -> vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    // add r0.x, cb2[4].w, l(1.875000)
    let instruction_0 = ((scene.world.game_time).wwww) + (vec4<f32>(1.875));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // div r0.x, r0.x, cb2[14].w
    let instruction_1 = ((r0).xxxx) / ((material.source[9]).wwww);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, cb2[15].x
    let instruction_2 = ((r0).xxxx) * ((material.source[10]).xxxx);
    r0 = vec4<f32>(instruction_2.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, l(6.000000)
    let instruction_3 = ((r0).xxxx) * (vec4<f32>(6.0));
    r0 = vec4<f32>(instruction_3.x, r0.y, r0.z, r0.w);
    // sincos r0.x, null, r0.x
    let angle4 = (r0).xxxx;
    let instruction_4 = sin(angle4);
    r0 = vec4<f32>(instruction_4.x, r0.y, r0.z, r0.w);
    // max r0.y, r0.x, cb2[14].z
    let instruction_5 = max((r0).xxxx, (material.source[9]).zzzz);
    r0 = vec4<f32>(r0.x, instruction_5.y, r0.z, r0.w);
    // div r0.z, cb2[4].w, cb2[14].w
    let instruction_6 = ((scene.world.game_time).wwww) / ((material.source[9]).wwww);
    r0 = vec4<f32>(r0.x, r0.y, instruction_6.z, r0.w);
    // mul r0.z, r0.z, cb2[15].x
    let instruction_7 = ((r0).zzzz) * ((material.source[10]).xxxx);
    r0 = vec4<f32>(r0.x, r0.y, instruction_7.z, r0.w);
    // mul r0.z, r0.z, l(6.000000)
    let instruction_8 = ((r0).zzzz) * (vec4<f32>(6.0));
    r0 = vec4<f32>(r0.x, r0.y, instruction_8.z, r0.w);
    // sincos r0.z, null, r0.z
    let angle9 = (r0).zzzz;
    let instruction_9 = sin(angle9);
    r0 = vec4<f32>(r0.x, r0.y, instruction_9.z, r0.w);
    // max r0.w, r0.z, cb2[14].z
    let instruction_10 = max((r0).zzzz, (material.source[9]).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_10.w);
    // add r1.xyz, r0.xxxx, r0.zzzz
    let instruction_11 = ((r0).xxxx) + ((r0).zzzz);
    r1 = vec4<f32>(instruction_11.x, instruction_11.y, instruction_11.z, r1.w);
    // min r0.xy, r0.wyww, cb2[14].yyyy
    let instruction_12 = min((r0).wyww, (material.source[9]).yyyy);
    r0 = vec4<f32>(instruction_12.x, instruction_12.y, r0.z, r0.w);
    // add r0.x, r0.y, r0.x
    let instruction_13 = ((r0).yyyy) + ((r0).xxxx);
    r0 = vec4<f32>(instruction_13.x, r0.y, r0.z, r0.w);
    // mad r0.x, r0.x, l(0.500000), l(0.500000)
    let instruction_14 = fma((r0).xxxx, vec4<f32>(0.5), vec4<f32>(0.5));
    r0 = vec4<f32>(instruction_14.x, r0.y, r0.z, r0.w);
    // add r0.yz, v2.xxyx, l(0.000000, -0.500000, -0.500000, 0.000000)
    let instruction_15 = ((input.v2).xxyx) + (vec4<f32>(0.0, -0.5, -0.5, 0.0));
    r0 = vec4<f32>(r0.x, instruction_15.y, instruction_15.z, r0.w);
    // mad r0.xy, r0.yzyy, r0.xxxx, l(0.500000, 0.500000, 0.000000, 0.000000)
    let instruction_16 = fma((r0).yzyy, (r0).xxxx, vec4<f32>(0.5, 0.5, 0.0, 0.0));
    r0 = vec4<f32>(instruction_16.x, instruction_16.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t0.xyzw, s6
    let instruction_17 = textureSample(texture_t0, sampler_t0, ((r0).xyxx).xy);
    r0 = instruction_17;
    // mul r0.xyzw, r0.xyzw, cb2[11].zzzz
    let instruction_18 = ((r0).xyzw) * ((material.source[6]).zzzz);
    r0 = instruction_18;
    // mul r0.xyz, r0.wwww, r0.xyzx
    let instruction_19 = ((r0).wwww) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_19.x, instruction_19.y, instruction_19.z, r0.w);
    // mul r2.xy, cb2[4].wwww, cb2[15].zwzz
    let instruction_20 = ((scene.world.game_time).wwww) * ((material.source[10]).zwzz);
    r2 = vec4<f32>(instruction_20.x, instruction_20.y, r2.z, r2.w);
    // mul r2.xy, r2.xyxx, cb2[16].xyxx
    let instruction_21 = ((r2).xyxx) * ((material.source[11]).xyxx);
    r2 = vec4<f32>(instruction_21.x, instruction_21.y, r2.z, r2.w);
    // frc r2.xy, r2.xyxx
    let instruction_22 = fract((r2).xyxx);
    r2 = vec4<f32>(instruction_22.x, instruction_22.y, r2.z, r2.w);
    // mad r2.xy, v2.zwzz, cb2[16].xyxx, r2.xyxx
    let instruction_23 = fma((input.v2).zwzz, (material.source[11]).xyxx, (r2).xyxx);
    r2 = vec4<f32>(instruction_23.x, instruction_23.y, r2.z, r2.w);
    // sample r2.xyzw, r2.xyxx, t1.xyzw, s4
    let instruction_24 = textureSample(texture_t1, sampler_t1, ((r2).xyxx).xy);
    r2 = instruction_24;
    // mad r2.xy, r2.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_25 = fma((r2).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r2 = vec4<f32>(instruction_25.x, instruction_25.y, r2.z, r2.w);
    // mad r2.xy, r2.xyxx, cb2[15].yyyy, v2.zwzz
    let instruction_26 = fma((r2).xyxx, (material.source[10]).yyyy, (input.v2).zwzz);
    r2 = vec4<f32>(instruction_26.x, instruction_26.y, r2.z, r2.w);
    // sample r2.xyzw, r2.xyxx, t2.xyzw, s5
    let instruction_27 = textureSample(texture_t2, sampler_t2, ((r2).xyxx).xy);
    r2 = instruction_27;
    // mov r1.w, l(2.000000)
    let instruction_28 = vec4<f32>(2.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_28.w);
    // max r1.xyzw, r1.xyzw, l(0.500000, 0.500000, 0.500000, 0.500000)
    let instruction_29 = max((r1).xyzw, vec4<f32>(0.5, 0.5, 0.5, 0.5));
    r1 = instruction_29;
    // min r1.xyzw, r1.xyzw, l(1.250000, 1.250000, 1.250000, 1.250000)
    let instruction_30 = min((r1).xyzw, vec4<f32>(1.25, 1.25, 1.25, 1.25));
    r1 = instruction_30;
    // mad r1.xyz, r2.xyzx, r1.xyzx, -r0.xyzx
    let instruction_31 = fma((r2).xyzx, (r1).xyzx, -((r0).xyzx));
    r1 = vec4<f32>(instruction_31.x, instruction_31.y, instruction_31.z, r1.w);
    // mul r0.w, r1.w, r2.w
    let instruction_32 = ((r1).wwww) * ((r2).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_32.w);
    // mul r0.w, r0.w, cb2[9].z
    let instruction_33 = ((r0).wwww) * ((material.source[4]).zzzz);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_33.w);
    // mad r0.xyz, r0.wwww, r1.xyzx, r0.xyzx
    let instruction_34 = fma((r0).wwww, (r1).xyzx, (r0).xyzx);
    r0 = vec4<f32>(instruction_34.x, instruction_34.y, instruction_34.z, r0.w);
    // sample r1.xyzw, v3.xyxx, t3.xyzw, s3
    let instruction_35 = textureSample(texture_t3, sampler_t3, ((input.v3).xyxx).xy);
    r1 = instruction_35;
    // add r1.xyz, -r0.xyzx, r1.xyzx
    let instruction_36 = (-((r0).xyzx)) + ((r1).xyzx);
    r1 = vec4<f32>(instruction_36.x, instruction_36.y, instruction_36.z, r1.w);
    // mul r0.w, r1.w, cb2[9].y
    let instruction_37 = ((r1).wwww) * ((material.source[4]).yyyy);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_37.w);
    // mad r0.xyz, r0.wwww, r1.xyzx, r0.xyzx
    let instruction_38 = fma((r0).wwww, (r1).xyzx, (r0).xyzx);
    r0 = vec4<f32>(instruction_38.x, instruction_38.y, instruction_38.z, r0.w);
    // mul r1.xy, cb2[4].wwww, cb2[13].xyxx
    let instruction_39 = ((scene.world.game_time).wwww) * ((material.source[8]).xyxx);
    r1 = vec4<f32>(instruction_39.x, instruction_39.y, r1.z, r1.w);
    // mul r1.xy, r1.xyxx, cb2[13].zwzz
    let instruction_40 = ((r1).xyxx) * ((material.source[8]).zwzz);
    r1 = vec4<f32>(instruction_40.x, instruction_40.y, r1.z, r1.w);
    // frc r1.xy, r1.xyxx
    let instruction_41 = fract((r1).xyxx);
    r1 = vec4<f32>(instruction_41.x, instruction_41.y, r1.z, r1.w);
    // mad r1.xy, v3.zwzz, cb2[13].zwzz, r1.xyxx
    let instruction_42 = fma((input.v3).zwzz, (material.source[8]).zwzz, (r1).xyxx);
    r1 = vec4<f32>(instruction_42.x, instruction_42.y, r1.z, r1.w);
    // sample r1.xyzw, r1.xyxx, t4.xyzw, s1
    let instruction_43 = textureSample(texture_t4, sampler_t4, ((r1).xyxx).xy);
    r1 = instruction_43;
    // mad r1.xy, r1.xyxx, l(2.000000, 2.000000, 0.000000, 0.000000), l(-1.000000, -1.000000, 0.000000, 0.000000)
    let instruction_44 = fma((r1).xyxx, vec4<f32>(2.0, 2.0, 0.0, 0.0), vec4<f32>(-1.0, -1.0, 0.0, 0.0));
    r1 = vec4<f32>(instruction_44.x, instruction_44.y, r1.z, r1.w);
    // mad r1.xy, r1.xyxx, cb2[14].xxxx, v3.zwzz
    let instruction_45 = fma((r1).xyxx, (material.source[9]).xxxx, (input.v3).zwzz);
    r1 = vec4<f32>(instruction_45.x, instruction_45.y, r1.z, r1.w);
    // sample r1.xyzw, r1.xyxx, t5.xyzw, s2
    let instruction_46 = textureSample(texture_t5, sampler_t5, ((r1).xyxx).xy);
    r1 = instruction_46;
    // add r1.xyz, -r0.xyzx, r1.xyzx
    let instruction_47 = (-((r0).xyzx)) + ((r1).xyzx);
    r1 = vec4<f32>(instruction_47.x, instruction_47.y, instruction_47.z, r1.w);
    // mul r0.w, r1.w, cb2[9].x
    let instruction_48 = ((r1).wwww) * ((material.source[4]).xxxx);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_48.w);
    // mad r0.xyz, r0.wwww, r1.xyzx, r0.xyzx
    let instruction_49 = fma((r0).wwww, (r1).xyzx, (r0).xyzx);
    r0 = vec4<f32>(instruction_49.x, instruction_49.y, instruction_49.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb2[12].xyzx
    let instruction_50 = ((r0).xyzx) * ((material.source[7]).xyzx);
    r0 = vec4<f32>(instruction_50.x, instruction_50.y, instruction_50.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb2[9].wwww
    let instruction_51 = ((r0).xyzx) * ((material.source[4]).wwww);
    r0 = vec4<f32>(instruction_51.x, instruction_51.y, instruction_51.z, r0.w);
    // mul r0.xyz, r0.xyzx, r0.xyzx
    let instruction_52 = ((r0).xyzx) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_52.x, instruction_52.y, instruction_52.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_53 = ((r0).xyzx) * ((scene.world.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_53.x, instruction_53.y, instruction_53.z, r0.w);
    // sqrt o0.xyz, r0.xyzx
    let instruction_54 = sqrt((r0).xyzx);
    o0 = vec4<f32>(instruction_54.x, instruction_54.y, instruction_54.z, o0.w);
    // mov o0.w, l(1.000000)
    let instruction_55 = vec4<f32>(1.0);
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_55.w);
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
