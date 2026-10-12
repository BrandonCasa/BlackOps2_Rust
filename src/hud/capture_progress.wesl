import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> progress: vec4<f32>;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let delta = in.uv - vec2<f32>(0.5);
    let angle = atan2(delta.x, delta.y);
    let arc = select(0.0, 1.0, angle > 3.14159265359 - progress.x * 6.28318530718);
    let radius = length(delta) * 2.0;
    let outer = clamp((radius - (1.0 - 1.0 / 12.0)) * 12.0, 0.0, 1.0);
    let inner = clamp((1.0 - 1.0 / 12.0 - 0.06 - radius) * 12.0, 0.0, 1.0);
    return vec4<f32>(color.rgb, color.a * (1.0 - max(outer, inner)) * arc);
}
