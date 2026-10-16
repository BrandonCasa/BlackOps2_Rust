// Source MMS producer VS thermal_solid_f24ed319 / PS thermal_solid_9e710634; exact DXBC instruction translation.
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
}
@vertex
fn vertex(input: Input) -> Varyings {
    var o0: vec4<f32>;
    var o1: vec4<f32>;
    var o2: vec4<f32>;
    var r0: vec4<f32>;
    var r1: vec4<f32>;
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
    // mov o1.xy, v1.xyxx
    let instruction_7 = (vec4<f32>(input.uv, 0.0, 0.0)).xyxx;
    o1 = vec4<f32>(instruction_7.x, instruction_7.y, o1.z, o1.w);
    // dp4 o2.x, r1.xyzw, cb0[36].xyzw
    let instruction_8 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.view_projection_rows[0]).xyzw).xyzw));
    o2 = vec4<f32>(instruction_8.x, o2.y, o2.z, o2.w);
    // dp4 o2.y, r1.xyzw, cb0[37].xyzw
    let instruction_9 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.view_projection_rows[1]).xyzw).xyzw));
    o2 = vec4<f32>(o2.x, instruction_9.y, o2.z, o2.w);
    // dp4 o2.z, r1.xyzw, cb0[38].xyzw
    let instruction_10 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.view_projection_rows[2]).xyzw).xyzw));
    o2 = vec4<f32>(o2.x, o2.y, instruction_10.z, o2.w);
    // dp4 o2.w, r1.xyzw, cb0[39].xyzw
    let instruction_11 = vec4<f32>(dot(((r1).xyzw).xyzw, ((scene.view_projection_rows[3]).xyzw).xyzw));
    o2 = vec4<f32>(o2.x, o2.y, o2.z, instruction_11.w);
    var output: Varyings;
    output.position = o2;
    return output;
}
@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    var o0: vec4<f32>;
    // mul o0.xyz, cb2[5].xyzx, cb2[5].xyzx
    let instruction_0 = ((draw.material).xyzx) * ((draw.material).xyzx);
    o0 = vec4<f32>(instruction_0.x, instruction_0.y, instruction_0.z, o0.w);
    // mov o0.w, cb1[31].x
    let instruction_1 = (draw.weapon0).xxxx;
    o0 = vec4<f32>(o0.x, o0.y, o0.z, instruction_1.w);
    return o0;
}
