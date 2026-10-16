@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var depth: texture_depth_2d;
struct Reprojection {
    world_from_clip: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    previous_clip_from_world: mat4x4<f32>,
    render_size: vec4<f32>,
}
@group(0) @binding(2) var<uniform> reprojection: Reprojection;
struct Input { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
struct Output {
    @location(0) color: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @builtin(frag_depth) depth: f32,
}
@fragment
fn fragment(input: Input) -> Output {
    let pixel = vec2<i32>(input.position.xy);
    let encoded = textureLoad(scene, pixel, 0);
    let linear = encoded.rgb * encoded.rgb;
    let z = textureLoad(depth, pixel, 0);
    let ndc = input.position.xy / reprojection.render_size.xy * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    // Homogeneous reconstruction also handles the sky (reverse-Z zero).
    let world = reprojection.world_from_clip * vec4<f32>(ndc, z, 1.0);
    let current = reprojection.clip_from_world * world;
    let previous = reprojection.previous_clip_from_world * world;
    let velocity = (current.xy / current.w - previous.xy / previous.w) * vec2<f32>(0.5, -0.5);
    return Output(vec4<f32>(linear, encoded.a), velocity, z);
}
