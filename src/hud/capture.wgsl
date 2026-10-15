// hud_objective_circle_meter: sw4_2d_arc_outline_qez20962.
// Original material constants: Scale=1, Thickness=0.06, EdgeHarshness=12.
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius_x: vec4<f32>,
    @location(3) border_radius_y: vec4<f32>,
    @location(4) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> progress: f32;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let delta = in.uv - vec2<f32>(0.5);
    let radius = length(delta) * 2.0;
    let outer = clamp((radius - (1.0 - 1.0 / 12.0)) * 12.0, 0.0, 1.0);
    let inner = clamp(((1.0 - 1.0 / 12.0 - 0.06) - radius) * 12.0, 0.0, 1.0);
    let angle = atan2(delta.x, delta.y);
    let filled = select(0.0, 1.0, angle > 3.14159265359 - progress * 6.28318530718);
    return vec4<f32>(color.rgb, color.a * (1.0 - max(outer, inner)) * filled);
}
