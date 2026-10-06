#import "embedded://bevy_pbr/render/mesh.wgsl"::vertex as bevy_vertex
#import bevy_pbr::forward_io::{Vertex, VertexOutput}
@group(#{MATERIAL_BIND_GROUP}) @binding(127) var instance_lighting: texture_2d<f32>;
struct SurfaceVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv_b: vec2<f32>,
    @location(4) tangent: vec4<f32>,
    @location(5) color: vec4<f32>,
    @location(8) layer_uv: vec4<f32>,
    @location(9) layer3_uv: vec2<f32>,
    @location(10) rotation1: vec4<f32>,
    @location(11) rotation2: vec4<f32>,
    @location(12) lighting_vertex: f32,
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
    @location(10) rotation1: vec4<f32>,
    @location(11) rotation2: vec4<f32>,
    @location(12) baked_color: vec3<f32>,
}
@vertex
fn vertex(input: SurfaceVertex) -> SurfaceOutput {
    var standard: Vertex;
    standard.instance_index = input.instance_index;
    standard.position = input.position;
    standard.normal = input.normal;
    standard.uv = input.uv;
    standard.uv_b = input.uv_b;
    standard.tangent = input.tangent;
    standard.color = input.color;
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
    out.layer_uv = input.layer_uv;
    out.layer3_uv = input.layer3_uv;
    out.rotation1 = input.rotation1;
    out.rotation2 = input.rotation2;
    let width = textureDimensions(instance_lighting).x;
    let index = u32(input.lighting_vertex);
    out.baked_color = textureLoad(instance_lighting, vec2<i32>(i32(index % width),i32(index / width)),0).rgb;
    return out;
}
