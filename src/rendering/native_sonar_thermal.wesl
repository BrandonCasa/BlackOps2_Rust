// Source MMS producer VS thermal_968212 / PS thermal_c85c8066; exact DXBC instruction translation.
import package::rendering::native_world::NativeWorldScene;
import package::rendering::native_sonar_common::NativeSonarDraw;
@group(0) @binding(0) var<uniform> scene: NativeWorldScene;
@group(1) @binding(0) var<uniform> draw: NativeSonarDraw;
struct Input {
    @location(0) position: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) normal: vec4<f32>,
}
@group(2) @binding(0) var texture_t0: texture_2d<f32>;
@group(2) @binding(1) var sampler_t0: sampler;
@group(2) @binding(2) var texture_t1: texture_2d<f32>;
@group(2) @binding(3) var sampler_t1: sampler;
@group(2) @binding(4) var texture_t2: texture_2d<f32>;
@group(2) @binding(5) var sampler_t2: sampler;
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) v0: vec4<f32>,
    @location(1) v1: vec4<f32>,
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
    var r2: vec4<f32>;
    var r3: vec4<f32>;
    // mov r0.xyz, v0.xyzx
    let instruction_0 = (input.position).xyzx;
    r0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, r0.w);
    // mov r0.w, l(1.000000)
    let instruction_1 = vec4<f32>(1.0);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_1.w);
    // dp4 r1.x, r0.xyzw, cb3[0].xyzw
    let instruction_2 = vec4<f32>(dot(((r0).xyzw).xyzw, ((draw.world_rows[0]).xyzw).xyzw));
    r1 = vec4<f32>(instruction_2.x, r1.y, r1.z, r1.w);
    // dp4 r1.y, r0.xyzw, cb3[1].xyzw
    let instruction_3 = vec4<f32>(dot(((r0).xyzw).xyzw, ((draw.world_rows[1]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, instruction_3.y, r1.z, r1.w);
    // dp4 r1.z, r0.xyzw, cb3[2].xyzw
    let instruction_4 = vec4<f32>(dot(((r0).xyzw).xyzw, ((draw.world_rows[2]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, instruction_4.z, r1.w);
    // dp4 r1.w, r0.xyzw, cb3[3].xyzw
    let instruction_5 = vec4<f32>(dot(((r0).xyzw).xyzw, ((draw.world_rows[3]).xyzw).xyzw));
    r1 = vec4<f32>(r1.x, r1.y, r1.z, instruction_5.w);
    // mov o0.xyz, r1.xyzx
    let instruction_6 = (r1).xyzx;
    o0 = vec4<f32>(instruction_6.x, instruction_6.y, instruction_6.z, o0.w);
    // add r0.xyz, v2.xyzx, v2.xyzx
    let instruction_7 = ((input.normal).xyzx) + ((input.normal).xyzx);
    r0 = vec4<f32>(instruction_7.x, instruction_7.y, instruction_7.z, r0.w);
    // ge r2.xyz, v2.xyzx, l(0.500000, 0.500000, 0.500000, 0.000000)
    let instruction_8 = bitcast<vec4<f32>>(select(vec4<u32>(0u), vec4<u32>(0xffffffffu), ((input.normal).xyzx) >= (vec4<f32>(0.5, 0.5, 0.5, 0.0))));
    r2 = vec4<f32>(instruction_8.x, instruction_8.y, instruction_8.z, r2.w);
    // mad r3.xyz, v2.xyzx, l(2.000000, 2.000000, 2.000000, 0.000000), l(-2.000000, -2.000000, -2.000000, 0.000000)
    let instruction_9 = fma((input.normal).xyzx, vec4<f32>(2.0, 2.0, 2.0, 0.0), vec4<f32>(-2.0, -2.0, -2.0, 0.0));
    r3 = vec4<f32>(instruction_9.x, instruction_9.y, instruction_9.z, r3.w);
    // movc r0.xyz, r2.xyzx, r3.xyzx, r0.xyzx
    let instruction_10 = select((r0).xyzx, (r3).xyzx, bitcast<vec4<u32>>((r2).xyzx) != vec4<u32>(0u));
    r0 = vec4<f32>(instruction_10.x, instruction_10.y, instruction_10.z, r0.w);
    // dp3 r2.x, r0.xyzx, cb3[0].xyzx
    let instruction_11 = vec4<f32>(dot(((r0).xyzx).xyz, ((draw.world_rows[0]).xyzx).xyz));
    r2 = vec4<f32>(instruction_11.x, r2.y, r2.z, r2.w);
    // dp3 r2.y, r0.xyzx, cb3[1].xyzx
    let instruction_12 = vec4<f32>(dot(((r0).xyzx).xyz, ((draw.world_rows[1]).xyzx).xyz));
    r2 = vec4<f32>(r2.x, instruction_12.y, r2.z, r2.w);
    // dp3 r2.z, r0.xyzx, cb3[2].xyzx
    let instruction_13 = vec4<f32>(dot(((r0).xyzx).xyz, ((draw.world_rows[2]).xyzx).xyz));
    r2 = vec4<f32>(r2.x, r2.y, instruction_13.z, r2.w);
    // dp3 r0.x, r2.xyzx, r2.xyzx
    let instruction_14 = vec4<f32>(dot(((r2).xyzx).xyz, ((r2).xyzx).xyz));
    r0 = vec4<f32>(instruction_14.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_15 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_15.x, r0.y, r0.z, r0.w);
    // mul o1.xyz, r0.xxxx, r2.xyzx
    let instruction_16 = ((r0).xxxx) * ((r2).xyzx);
    o1 = vec4<f32>(instruction_16.x, instruction_16.y, instruction_16.z, o1.w);
    // mov o1.w, l(0)
    let instruction_17 = vec4<f32>(0.0);
    o1 = vec4<f32>(o1.x, o1.y, o1.z, instruction_17.w);
    // mov o2.xy, v1.xyxx
    let instruction_18 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o2 = vec4<f32>(instruction_18.x, instruction_18.y, o2.z, o2.w);
    // dp4 o3.x, r1.xyzw, cb0[36].xyzw
    let instruction_19 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.view_projection_rows[0]).xyzw).xyzw));
    o3 = vec4<f32>(instruction_19.x, o3.y, o3.z, o3.w);
    // dp4 o3.y, r1.xyzw, cb0[37].xyzw
    let instruction_20 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.view_projection_rows[1]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, instruction_20.y, o3.z, o3.w);
    // dp4 o3.z, r1.xyzw, cb0[38].xyzw
    let instruction_21 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.view_projection_rows[2]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, o3.y, instruction_21.z, o3.w);
    // dp4 o3.w, r1.xyzw, cb0[39].xyzw
    let instruction_22 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.view_projection_rows[3]).xyzw).xyzw));
    o3 = vec4<f32>(o3.x, o3.y, o3.z, instruction_22.w);
    var output: Varyings;
    output.position = o3;
    output.v0 = o0;
    output.v1 = o1;
    output.v2 = o2;
    return output;
}
@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
    // dp3 r0.x, v1.xyzx, v1.xyzx
    let instruction_0 = vec4<f32>(dot(((input.v1).xyzx).xyz, ((input.v1).xyzx).xyz));
    r0 = vec4<f32>(instruction_0.x, r0.y, r0.z, r0.w);
    // rsq r0.x, r0.x
    let instruction_1 = inverseSqrt((r0).xxxx);
    r0 = vec4<f32>(instruction_1.x, r0.y, r0.z, r0.w);
    // mul r0.xyz, r0.xxxx, v1.xyzx
    let instruction_2 = ((r0).xxxx) * ((input.v1).xyzx);
    r0 = vec4<f32>(instruction_2.x, instruction_2.y, instruction_2.z, r0.w);
    // dp3 r0.w, -v0.xyzx, -v0.xyzx
    let instruction_3 = vec4<f32>(dot((-((input.v0).xyzx)).xyz, (-((input.v0).xyzx)).xyz));
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_3.w);
    // rsq r0.w, r0.w
    let instruction_4 = inverseSqrt((r0).wwww);
    r0 = vec4<f32>(r0.x, r0.y, r0.z, instruction_4.w);
    // mul r1.xyz, r0.wwww, -v0.xyzx
    let instruction_5 = ((r0).wwww) * (-((input.v0).xyzx));
    r1 = vec4<f32>(instruction_5.x, instruction_5.y, instruction_5.z, r1.w);
    // dp3_sat r0.x, r0.xyzx, r1.xyzx
    let instruction_6 = clamp(vec4<f32>(dot(((r0).xyzx).xyz, ((r1).xyzx).xyz)), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_6.x, r0.y, r0.z, r0.w);
    // add r0.x, r0.x, l(-0.646400)
    let instruction_7 = ((r0).xxxx) + (vec4<f32>(-0.646399974822998));
    r0 = vec4<f32>(instruction_7.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, l(-1.547030)
    let instruction_8 = ((r0).xxxx) * (vec4<f32>(-1.547029733657837));
    r0 = vec4<f32>(instruction_8.x, r0.y, r0.z, r0.w);
    // max r0.x, r0.x, l(0.000000)
    let instruction_9 = max((r0).xxxx, vec4<f32>(0.0));
    r0 = vec4<f32>(instruction_9.x, r0.y, r0.z, r0.w);
    // mad r0.y, r0.x, l(-2.000000), l(3.000000)
    let instruction_10 = fma((r0).xxxx, vec4<f32>(-2.0), vec4<f32>(3.0));
    r0 = vec4<f32>(r0.x, instruction_10.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.x
    let instruction_11 = ((r0).xxxx) * ((r0).xxxx);
    r0 = vec4<f32>(instruction_11.x, r0.y, r0.z, r0.w);
    // mul r0.x, r0.x, r0.y
    let instruction_12 = ((r0).xxxx) * ((r0).yyyy);
    r0 = vec4<f32>(instruction_12.x, r0.y, r0.z, r0.w);
    // sample r1.xyzw, v2.xyxx, t1.xyzw, s1
    let instruction_13 = textureSample(texture_t1, sampler_t1, ((input.v2).xyxx).xy);
    r1 = instruction_13;
    // mad r0.y, r1.x, l(4.015748), l(-2.015748)
    let instruction_14 = fma((r1).xxxx, vec4<f32>(4.015748023986816), vec4<f32>(-2.0157480239868164));
    r0 = vec4<f32>(r0.x, instruction_14.y, r0.z, r0.w);
    // sample r1.xyzw, v2.xyxx, t0.xyzw, s2
    let instruction_15 = textureSample(texture_t0, sampler_t0, ((input.v2).xyxx).xy);
    r1 = instruction_15;
    // mad r0.y, r0.y, cb2[5].y, r1.x
    let instruction_16 = fma((r0).yyyy, (draw.material).yyyy, (r1).xxxx);
    r0 = vec4<f32>(r0.x, instruction_16.y, r0.z, r0.w);
    // mad_sat r0.x, -r0.x, l(0.500000), r0.y
    let instruction_17 = clamp(fma(-((r0).xxxx), vec4<f32>(0.5), (r0).yyyy), vec4<f32>(0.0), vec4<f32>(1.0));
    r0 = vec4<f32>(instruction_17.x, r0.y, r0.z, r0.w);
    // mov r0.y, l(0.500000)
    let instruction_18 = vec4<f32>(0.5);
    r0 = vec4<f32>(r0.x, instruction_18.y, r0.z, r0.w);
    // sample r0.xyzw, r0.xyxx, t2.xyzw, s3
    let instruction_19 = textureSample(texture_t2, sampler_t2, ((r0).xyxx).xy);
    r0 = instruction_19;
    // mul r0.xyz, r0.xyzx, cb2[5].xxxx
    let instruction_20 = ((r0).xyzx) * ((draw.material).xxxx);
    r0 = vec4<f32>(instruction_20.x, instruction_20.y, instruction_20.z, r0.w);
    // mul r0.xyz, r0.xyzx, r0.xyzx
    let instruction_21 = ((r0).xyzx) * ((r0).xyzx);
    r0 = vec4<f32>(instruction_21.x, instruction_21.y, instruction_21.z, r0.w);
    // mul r0.xyz, r0.xyzx, cb0[20].xxxx
    let instruction_22 = ((r0).xyzx) * ((scene.hdr_control0).xxxx);
    r0 = vec4<f32>(instruction_22.x, instruction_22.y, instruction_22.z, r0.w);
    // sqrt o0.xyz, r0.xyzx
    let instruction_23 = sqrt((r0).xyzx);
    o0 = vec4<f32>(instruction_23.x, instruction_23.y, instruction_23.z, o0.w);
    // mov o0.w, cb1[31].x
    let instruction_24 = (draw.weapon0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_24.w);
    return o0;
}
