@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var depth: texture_depth_2d;
struct Reprojection {
    world_from_clip: mat4x4<f32>,
    clip_from_world: mat4x4<f32>,
    previous_clip_from_world: mat4x4<f32>,
    render_size: vec4<f32>,
    jitter_delta: vec4<f32>,
}
@group(0) @binding(2) var<uniform> reprojection: Reprojection;
@group(0) @binding(3) var before_transparency: texture_2d<f32>;
@group(0) @binding(4) var previous_reactive: texture_2d<f32>;
struct Input { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
struct Output {
    @location(0) color: vec4<f32>,
    @location(1) motion: vec2<f32>,
    @location(2) bias: f32,
    @location(3) reactive: f32,
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
    let opaque = textureLoad(before_transparency, pixel, 0).rgb;
    let difference = abs(encoded.rgb - opaque);
    let brightness = max(max(max(encoded.r, encoded.g), encoded.b), max(max(opaque.r, opaque.g), opaque.b));
    let reactive = clamp(max(max(difference.r, difference.g), difference.b) / max(brightness, 1.0 / 255.0), 0.0, 1.0);
    // Use last frame's raw coverage once, so disappearing effects reject their
    // former footprint without feeding the rejection mask back indefinitely.
    let previous_uv = (input.position.xy + reprojection.jitter_delta.xy) / reprojection.render_size.xy - velocity;
    var old_reactive = 0.0;
    if reprojection.render_size.z > 0.0 && all(previous_uv >= vec2<f32>(0.0)) && all(previous_uv < vec2<f32>(1.0)) {
        old_reactive = textureLoad(previous_reactive, vec2<i32>(previous_uv * reprojection.render_size.xy), 0).r;
    }
    return Output(vec4<f32>(linear, encoded.a), velocity, max(reactive, old_reactive), reactive, z);
}
