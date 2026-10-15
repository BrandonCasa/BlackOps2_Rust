// Original transform-only VSd4f4e04b and null PS3b4a8bd3, used by both
// wpc_shadowcaster depth prepass and build-shadowmap-depth techniques.
// Supply each pass's ORIGINAL coordinate frame in these two matrices. The
// camera's eye-relative world rows cannot be reused for a light-space pass.
struct NativeDepthView { view_projection_rows: array<vec4<f32>, 4> }
struct NativeDepthObject { world_rows: array<vec4<f32>, 4> }
@group(0) @binding(0) var<uniform> view: NativeDepthView;
@group(1) @binding(0) var<uniform> object: NativeDepthObject;

@vertex
fn vertex(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    let source = vec4<f32>(position, 1.0);
    let world = vec4<f32>(dot(source, object.world_rows[0]), dot(source, object.world_rows[1]),
        dot(source, object.world_rows[2]), dot(source, object.world_rows[3]));
    return vec4<f32>(dot(world, view.view_projection_rows[0]), dot(world, view.view_projection_rows[1]),
        dot(world, view.view_projection_rows[2]), dot(world, view.view_projection_rows[3]));
}

// Source null PS writes white, but both depth techniques disable color writes.
// A depth-only render pipeline can omit the fragment stage altogether.
@fragment
fn fragment() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }
