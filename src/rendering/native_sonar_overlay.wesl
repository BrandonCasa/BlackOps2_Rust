// Source MMS producer VS char_sonar_attachment_e7224d0f / PS char_sonar_attachment_4a7774e9; exact DXBC instruction translation.
import package::rendering::native_world::NativeWorldScene;
import package::rendering::native_sonar_common::NativeSonarDraw;
@group(0) @binding(0) var<uniform> scene: NativeWorldScene;
@group(1) @binding(0) var<uniform> draw: NativeSonarDraw;
struct Input {
    @location(0) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: vec4<f32>,
}
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
}
@vertex
fn vertex(input: Input) -> Varyings {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var o3: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    var r2: vec4<f32>;
    // add r0.xyz, v2.xyzx, v2.xyzx
    let instruction_0 = ((input.normal).xyzx) + ((input.normal).xyzx);
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // ge r1.xyz, v2.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_1 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.normal).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r1 = vec4<f32>(instruction_1.x, instruction_1.y, instruction_1.z, r1.w);
    // mad r2.xyz, v2.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_2 = fma((input.normal).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r2 = vec4<f32>(instruction_2.x, instruction_2.y, instruction_2.z, r2.w);
    // movc r0.xyz, r1.xyzx, r2.xyzx, r0.xyzx
    let instruction_3 = select((r0).xyzx, (r2).xyzx, bitcast<vec4<u32>>((r1).xyzx) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_3.x, instruction_3.y, instruction_3.z, r0.w);
    // dp3 r1.x, r0.xyzx, cb3[0].xyzx
    let instruction_4 = vec4<f32>(dot(((r0).xyzx).xyz, ((draw.world_rows[0]).xyzx).xyz));
    r1 = vec4<f32>(instruction_4.x, r1.y, r1.z, r1.w);
    // dp3 r1.y, r0.xyzx, cb3[1].xyzx
    let instruction_5 = vec4<f32>(dot(((r0).xyzx).xyz, ((draw.world_rows[1]).xyzx).xyz));
    r1 = vec4<f32>(r1.x, instruction_5.y, r1.z, r1.w);
    // dp3 r1.z, r0.xyzx, cb3[2].xyzx
    let instruction_6 = vec4<f32>(dot(((r0).xyzx).xyz, ((draw.world_rows[2]).xyzx).xyz));
    r1 = vec4<f32>(r1.x, r1.y, instruction_6.z, r1.w);
    // dp3 r0.x, r1.xyzx, r1.xyzx
    let instruction_7 = vec4<f32>(dot(((r1).xyzx).xyz, ((r1).xyzx).xyz));
    r0 = vec4<f32>(instruction_7.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_8 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_8.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, r1.xyzx
    let instruction_9 = ((r0).xxxx) * ((r1).xyzx);
    r0 = vec4<f32>(instruction_9.x, instruction_9.y, instruction_9.z, r0.w);
    // mov r1.xyz, v0.xyzx
    let instruction_10 = (input.position).xyzx;
    r1 = vec4<f32>(instruction_10.x, instruction_10.y, instruction_10.z, r1.w);
    // mov r1.w, l(1.000000)
    let instruction_11 = vec4<f32>(1.0);
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_11.w);
    // dp4 r2.x, r1.xyzw, cb3[0].xyzw
    let instruction_12 = vec4<f32>(dot(((r1).xyzw).xyzw, ((draw.world_rows[0]).xyzw).xyzw));
    r2 = vec4<f32>(instruction_12.x, r2.y, r2.z, r2.w);
    // dp4 r2.y, r1.xyzw, cb3[1].xyzw
    let instruction_13 = vec4<f32>(dot(((r1).xyzw).xyzw, ((draw.world_rows[1]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, instruction_13.y, r2.z, r2.w);
    // dp4 r2.z, r1.xyzw, cb3[2].xyzw
    let instruction_14 = vec4<f32>(dot(((r1).xyzw).xyzw, ((draw.world_rows[2]).xyzw).xyzw));
    r2 = vec4<f32>(r2.x, r2.y, instruction_14.z, r2.w);
    // mad r0.xyz, r0.xyzx, cb2[5].xxxx, r2.xyzx
    let instruction_15 = fma((r0).xyzx, (draw.material).xxxx, (r2).xyzx);
    r0 = vec4<f32>(instruction_15.x, instruction_15.y, instruction_15.z, r0.w);
    // dp3 r1.x, r0.xyzx, r0.xyzx
    let instruction_16 = vec4<f32>(dot(((r0).xyzx).xyz, ((r0).xyzx).xyz));
    r1 = vec4<f32>(instruction_16.x, r1.y, r1.z, r1.w);
    // rsq r1.y, r1.x
    let instruction_17 = inverseSqrt((r1).xxxx);
    r1 = vec4<f32>(r1.x, instruction_17.y, r1.z, r1.w);
    // sqrt r1.x, r1.x
    let instruction_18 = sqrt((r1).xxxx);
    r1 = vec4<f32>(instruction_18.x, r1.y, r1.z, r1.w);
    // mul r1.yzw, r0.xxyz, r1.yyyy
    let instruction_19 = ((r0).xxyz) * ((r1).yyyy);
    r1 = vec4<f32>(r1.x, instruction_19.y, instruction_19.z, instruction_19.w);
    // dp3 r1.y, cb0[29].xyzx, r1.yzwy
    let instruction_20 = vec4<f32>(dot(((scene.sun_fog_dir).xyzx).xyz, ((r1).yzwy).xyz));
    r1 = vec4<f32>(r1.x, instruction_20.y, r1.z, r1.w);
    // mad_sat r1.y, cb0[31].y, r1.y, cb0[31].x
    let instruction_21 = clamp(fma((scene.sun_fog).yyyy, (r1).yyyy, (scene.sun_fog).xxxx), vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, instruction_21.y, r1.z, r1.w);
    // add r2.xyzw, -cb0[26].xyzw, cb0[30].xyzw
    let instruction_22 = (-((scene.fog_color).xyzw)) + ((scene.sun_fog_color).xyzw);
    r2 = instruction_22;
    // mad r2.xyzw, r1.yyyy, r2.xyzw, cb0[26].xyzw
    let instruction_23 = fma((r1).yyyy, (r2).xyzw, (scene.fog_color).xyzw);
    r2 = instruction_23;
    // mov o0.xyz, r2.xyzx
    let instruction_24 = (r2).xyzx;
    o0 = vec4<f32>(instruction_24.x, instruction_24.y, instruction_24.z, o0.w);
    // mad r1.y, cb0[27].w, r0.z, cb0[27].x
    let instruction_25 = fma((scene.fog_consts).wwww, (r0).zzzz, (scene.fog_consts).xxxx);
    r1 = vec4<f32>(r1.x, instruction_25.y, r1.z, r1.w);
    // min r1.z, r1.y, l(64.000000)
    let instruction_26 = min((r1).yyyy, vec4<f32>(64.0));
    r1 = vec4<f32>(r1.x, r1.y, instruction_26.z, r1.w);
    // mul r1.z, r1.z, l(1.442695)
    let instruction_27 = ((r1).zzzz) * (vec4<f32>(1.4426950216293335));
    r1 = vec4<f32>(r1.x, r1.y, instruction_27.z, r1.w);
    // exp r1.z, r1.z
    let instruction_28 = exp2((r1).zzzz);
    r1 = vec4<f32>(r1.x, r1.y, instruction_28.z, r1.w);
    // lt r1.w, r1.y, l(0.000000)
    let instruction_29 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((r1).yyyy) < (vec4<f32>(0.0))));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_29.w);
    // add r1.y, r1.y, l(1.000000)
    let instruction_30 = ((r1).yyyy) + (vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, instruction_30.y, r1.z, r1.w);
    // movc r1.y, r1.w, r1.z, r1.y
    let instruction_31 = select((r1).yyyy, (r1).zzzz, bitcast<vec4<u32>>((r1).wwww) != vec4<u32>(0u));
    r1 = vec4<f32>(r1.x, instruction_31.y, r1.z, r1.w);
    // add r1.y, r1.y, -cb0[28].x
    let instruction_32 = ((r1).yyyy) + (-((scene.fog_consts2).xxxx));
    r1 = vec4<f32>(r1.x, instruction_32.y, r1.z, r1.w);
    // mul r1.z, r0.z, cb0[27].w
    let instruction_33 = ((r0).zzzz) * ((scene.fog_consts).wwww);
    r1 = vec4<f32>(r1.x, r1.y, instruction_33.z, r1.w);
    // lt r1.w, |r1.z|, l(0.000100)
    let instruction_34 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), (abs((r1).zzzz)) < (vec4<f32>(9.999999747378752e-05))));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_34.w);
    // movc r1.z, r1.w, l(1.000000), r1.z
    let instruction_35 = select((r1).zzzz, vec4<f32>(1.0), bitcast<vec4<u32>>((r1).wwww) != vec4<u32>(0u));
    r1 = vec4<f32>(r1.x, r1.y, instruction_35.z, r1.w);
    // div r1.y, r1.y, r1.z
    let instruction_36 = ((r1).yyyy) / ((r1).zzzz);
    r1 = vec4<f32>(r1.x, instruction_36.y, r1.z, r1.w);
    // mov_sat r1.z, cb0[28].x
    let instruction_37 = clamp((scene.fog_consts2).xxxx, vec4<f32>(0.0), vec4<f32>(1.0));
    r1 = vec4<f32>(r1.x, r1.y, instruction_37.z, r1.w);
    // movc r1.y, r1.w, r1.z, r1.y
    let instruction_38 = select((r1).yyyy, (r1).zzzz, bitcast<vec4<u32>>((r1).wwww) != vec4<u32>(0u));
    r1 = vec4<f32>(r1.x, instruction_38.y, r1.z, r1.w);
    // mul r1.y, r1.y, cb0[27].y
    let instruction_39 = ((r1).yyyy) * ((scene.fog_consts).yyyy);
    r1 = vec4<f32>(r1.x, instruction_39.y, r1.z, r1.w);
    // mad r1.x, r1.y, r1.x, cb0[27].z
    let instruction_40 = fma((r1).yyyy, (r1).xxxx, (scene.fog_consts).zzzz);
    r1 = vec4<f32>(instruction_40.x, r1.y, r1.z, r1.w);
    // exp r1.x, r1.x
    let instruction_41 = exp2((r1).xxxx);
    r1 = vec4<f32>(instruction_41.x, r1.y, r1.z, r1.w);
    // min r1.x, r1.x, l(1.000000)
    let instruction_42 = min((r1).xxxx, vec4<f32>(1.0));
    r1 = vec4<f32>(instruction_42.x, r1.y, r1.z, r1.w);
    // add r1.x, -r1.x, l(1.000000)
    let instruction_43 = (-((r1).xxxx)) + (vec4<f32>(1.0));
    r1 = vec4<f32>(instruction_43.x, r1.y, r1.z, r1.w);
    // mad o1.w, -r1.x, r2.w, l(1.000000)
    let instruction_44 = fma(-((r1).xxxx), (r2).wwww, vec4<f32>(1.0));
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_44.w);
    // mov o1.xyz, r0.xyzx
    let instruction_45 = (r0).xyzx;
    o1 = vec4<f32>(instruction_45.x, instruction_45.y, instruction_45.z, o1.w);
    // mov o2.xy, v1.xyxx
    let instruction_46 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o2 = vec4<f32>(instruction_46.x, instruction_46.y, o2.z, o2.w);
    // mov r0.w, l(1.000000)
    let instruction_47 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_47.w);
    // dp4 o3.x, r0.xyzw, cb0[36].xyzw
    let instruction_48 = vec4<f32>(dot(((r0).xyzw).xyzw, ((scene.view_projection_rows[0]).xyzw).xyzw));
    o3 = vec4<f32>(instruction_48.x, o3.y, o3.z, o3.w);
    // dp4 o3.y, r0.xyzw, cb0[37].xyzw
    let instruction_49 = vec4<f32>(dot(((r0).xyzw).xyzw, ((scene.view_projection_rows[1]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, instruction_49.y, o3.z, o3.w);
    // dp4 o3.z, r0.xyzw, cb0[38].xyzw
    let instruction_50 = vec4<f32>(dot(((r0).xyzw).xyzw, ((scene.view_projection_rows[2]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, o3.y, instruction_50.z, o3.w);
    // dp4 o3.w, r0.xyzw, cb0[39].xyzw
    let instruction_51 = vec4<f32>(dot(((r0).xyzw).xyzw, ((scene.view_projection_rows[3]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_51.w);
    var output: Varyings;
    output.position = o3;
    output.v0 = o0;
    output.v1 = o1;
    return output;
}
@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    // mad r0.xyz, cb1[30].xyzx, cb1[30].xyzx, -v0.xyzx
    let instruction_0 = fma((draw.script7).xyzx, (draw.script7).xyzx, -((input.v0).xyzx));
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mad o0.xyz, v1.wwww, r0.xyzx, v0.xyzx
    let instruction_1 = fma((input.v1).wwww, (r0).xyzx, (input.v0).xyzx);
    o0 = vec4<f32>(instruction_1.x, instruction_1.y, instruction_1.z, o0.w);
    // mov o0.w, cb1[30].w
    let instruction_2 = (draw.script7).wwww;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_2.w);
    return o0;
}
