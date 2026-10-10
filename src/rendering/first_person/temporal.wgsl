@group(0) @binding(0) var world_depth: texture_depth_2d;
@group(0) @binding(1) var world_motion: texture_2d<f32>;
@group(0) @binding(2) var model_depth: texture_depth_2d;
@group(0) @binding(3) var model_motion: texture_2d<f32>;

struct TemporalInputs {
    @builtin(frag_depth) depth: f32,
    @location(0) motion: vec2<f32>,
}

@fragment
fn fragment(@builtin(position) position: vec4<f32>) -> TemporalInputs {
    let pixel = vec2<i32>(position.xy);
    let foreground = textureLoad(model_depth, pixel, 0);
    if foreground > 0.0 {
        return TemporalInputs(0.5 + 0.5 * foreground, textureLoad(model_motion, pixel, 0).xy);
    }
    return TemporalInputs(
        0.5 * textureLoad(world_depth, pixel, 0),
        textureLoad(world_motion, pixel, 0).xy,
    );
}
