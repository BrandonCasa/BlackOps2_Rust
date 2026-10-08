#import "embedded://bevy_pbr/render/mesh.wgsl"::vertex as bevy_vertex
#import bevy_pbr::forward_io::{Vertex, VertexOutput}
struct SurfaceVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef ORIGINS_UV_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef ORIGINS_TANGENT
    @location(4) tangent: vec4<f32>,
#endif
#ifdef ORIGINS_COLOR
    @location(5) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(6) joint_indices: vec4<u32>,
    @location(7) joint_weights: vec4<f32>,
#endif
#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif
#ifdef ORIGINS_LAYER_UV
    @location(8) layer_uv: vec4<f32>,
#endif
#ifdef ORIGINS_LAYER3_UV
    @location(9) layer3_uv: vec2<f32>,
#endif
#ifdef ORIGINS_ROTATION1
    @location(10) rotation_first: vec4<f32>,
#endif
#ifdef ORIGINS_ROTATION2
    @location(11) rotation_second: vec4<f32>,
#endif
}
struct SurfaceOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv_b: vec2<f32>,
    @location(4) world_tangent: vec4<f32>,
    @location(5) color: vec4<f32>,
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
    @location(8) layer_uv: vec4<f32>,
    @location(9) layer3_uv: vec2<f32>,
    @location(10) rotation_first: vec4<f32>,
    @location(11) rotation_second: vec4<f32>,
}
@vertex
fn vertex(input: SurfaceVertex) -> SurfaceOutput {
    var standard: Vertex;
    standard.instance_index = input.instance_index;
    standard.position = input.position;
    standard.normal = input.normal;
    standard.uv = input.uv;
#ifdef ORIGINS_UV_B
    standard.uv_b = input.uv_b;
#else
    standard.uv_b = vec2(0.0);
#endif
#ifdef ORIGINS_TANGENT
    standard.tangent = input.tangent;
#else
    standard.tangent = vec4(1.0, 0.0, 0.0, 1.0);
#endif
#ifdef ORIGINS_COLOR
    standard.color = input.color;
#else
    standard.color = vec4(1.0);
#endif
#ifdef SKINNED
    standard.joint_indices = input.joint_indices;
    standard.joint_weights = input.joint_weights;
#endif
#ifdef MORPH_TARGETS
    standard.index = input.index;
#endif
    let base = bevy_vertex(standard);
    var out: SurfaceOutput;
    out.position = base.position;
    out.world_position = base.world_position;
    out.world_normal = base.world_normal;
    out.uv = base.uv;
    out.uv_b = base.uv_b;
    out.world_tangent = base.world_tangent;
    out.color = base.color;
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = base.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = base.visibility_range_dither;
#endif
#ifdef ORIGINS_LAYER_UV
    out.layer_uv = input.layer_uv;
#else
    out.layer_uv = vec4(input.uv, input.uv);
#endif
#ifdef ORIGINS_LAYER3_UV
    out.layer3_uv = input.layer3_uv;
#else
    out.layer3_uv = input.uv;
#endif
#ifdef ORIGINS_ROTATION1
    out.rotation_first = input.rotation_first;
#else
    out.rotation_first = vec4(1.0, 0.0, 0.0, 1.0);
#endif
#ifdef ORIGINS_ROTATION2
    out.rotation_second = input.rotation_second;
#else
    out.rotation_second = vec4(1.0, 0.0, 0.0, 1.0);
#endif
    return out;
}
