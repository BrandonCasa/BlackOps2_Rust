struct MotionObject {
    clip: mat4x4<f32>,
    current: mat4x4<f32>,
    previous: mat4x4<f32>,
    reactive: vec4<f32>,
}
@group(0) @binding(0) var<uniform> object: MotionObject;
@group(0) @binding(1) var depth: texture_depth_2d;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) current: vec4<f32>,
    @location(1) previous: vec4<f32>,
}
@vertex
fn vertex(@location(0) position: vec3<f32>, @location(1) previous: vec3<f32>) -> VertexOutput {
    return VertexOutput(object.clip * vec4<f32>(position, 1.0),
        object.current * vec4<f32>(position, 1.0),
        object.previous * vec4<f32>(previous, 1.0));
}
struct Output {
    @location(0) motion: vec2<f32>,
    @location(1) bias: f32,
    @location(2) reactive: f32,
}
@fragment
fn fragment(input: VertexOutput) -> Output {
    let z = textureLoad(depth, vec2<i32>(input.position.xy), 0);
    // Match the final source depth, including alpha-cutout holes and occlusion.
    // A small relative tolerance accounts for matrix/dot-product rounding.
    if abs(z - input.position.z) > max(z * 0.0001, 0.0000001) { discard; }
    if input.current.w <= 0.0 || input.previous.w <= 0.0 {
        return Output(vec2<f32>(0.0), 1.0, 1.0);
    }
    let velocity = (input.current.xy / input.current.w - input.previous.xy / input.previous.w) * vec2<f32>(0.5, -0.5);
    return Output(velocity, object.reactive.x, object.reactive.x);
}
