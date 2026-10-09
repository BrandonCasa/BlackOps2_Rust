import bevy_pbr::render::{
    mesh_functions, skinning,
    mesh::morph_vertex,
    forward_io::decompress_vertex,
    view_transformations::position_world_to_clip,
};
import bevy_pbr::render::forward_io::{Vertex, VertexOutput};

struct SurfaceVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @if(ORIGINS_UV_B) @location(3) uv_b: vec2<f32>,
    @if(ORIGINS_TANGENT) @location(4) tangent: vec4<f32>,
    @if(ORIGINS_COLOR) @location(5) color: vec4<f32>,
    @if(SKINNED) @location(6) joint_indices: vec4<u32>,
    @if(SKINNED) @location(7) joint_weights: vec4<f32>,
    @if(MORPH_TARGETS) @builtin(vertex_index) index: u32,
    @if(ORIGINS_LAYER_UV) @location(8) layer_uv: vec4<f32>,
    @if(ORIGINS_LAYER3_UV) @location(9) layer3_uv: vec2<f32>,
    @if(ORIGINS_ROTATION1) @location(10) rotation_first: vec4<f32>,
    @if(ORIGINS_ROTATION2) @location(11) rotation_second: vec4<f32>,
}
struct SurfaceOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) uv_b: vec2<f32>,
    @location(4) world_tangent: vec4<f32>,
    @location(5) color: vec4<f32>,
    @if(VERTEX_OUTPUT_INSTANCE_INDEX) @location(6) @interpolate(flat) instance_index: u32,
    @if(VISIBILITY_RANGE_DITHER) @location(7) @interpolate(flat) visibility_range_dither: i32,
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
    @if(ORIGINS_UV_B) standard.uv_b = input.uv_b;
    @if(!ORIGINS_UV_B) standard.uv_b = vec2(0.0);
    @if(ORIGINS_TANGENT) standard.tangent = input.tangent;
    @if(!ORIGINS_TANGENT) standard.tangent = vec4(1.0, 0.0, 0.0, 1.0);
    @if(ORIGINS_COLOR) standard.color = input.color;
    @if(!ORIGINS_COLOR) standard.color = vec4(1.0);
    @if(SKINNED) standard.joint_indices = input.joint_indices;
    @if(SKINNED) standard.joint_weights = input.joint_weights;
    @if(MORPH_TARGETS) standard.index = input.index;
    // Use native mesh/skin/morph helpers without calling an @vertex entry point.
    // WESL entry points cannot be called as functions by a material extension.
    let uncompressed = decompress_vertex(standard, input.instance_index);
    @if(MORPH_TARGETS) let vertex = morph_vertex(uncompressed, input.instance_index);
    @if(!MORPH_TARGETS) let vertex = uncompressed;
    let mesh_world = mesh_functions::get_world_from_local(input.instance_index);
    @if(SKINNED) let world = skinning::skin_model(vertex.joint_indices, vertex.joint_weights, input.instance_index);
    @if(!SKINNED) let world = mesh_world;
    var base: VertexOutput;
    base.world_position = mesh_functions::mesh_position_local_to_world(world, vec4(vertex.position, 1.0));
    base.position = position_world_to_clip(base.world_position.xyz);
    @if(SKINNED) base.world_normal = skinning::skin_normals(world, vertex.normal);
    @if(!SKINNED) base.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, input.instance_index);
    base.world_tangent = mesh_functions::mesh_tangent_local_to_world(world, vertex.tangent, input.instance_index);
    base.uv = vertex.uv;
    base.uv_b = vertex.uv_b;
    base.color = vertex.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX) base.instance_index = input.instance_index;
    @if(VISIBILITY_RANGE_DITHER) base.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(input.instance_index, mesh_world[3]);
    var out: SurfaceOutput;
    out.position = base.position;
    out.world_position = base.world_position;
    out.world_normal = base.world_normal;
    out.uv = base.uv;
    out.uv_b = base.uv_b;
    out.world_tangent = base.world_tangent;
    out.color = base.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX) out.instance_index = base.instance_index;
    @if(VISIBILITY_RANGE_DITHER) out.visibility_range_dither = base.visibility_range_dither;
    @if(ORIGINS_LAYER_UV) out.layer_uv = input.layer_uv;
    @if(!ORIGINS_LAYER_UV) out.layer_uv = vec4(input.uv, input.uv);
    @if(ORIGINS_LAYER3_UV) out.layer3_uv = input.layer3_uv;
    @if(!ORIGINS_LAYER3_UV) out.layer3_uv = input.uv;
    @if(ORIGINS_ROTATION1) out.rotation_first = input.rotation_first;
    @if(!ORIGINS_ROTATION1) out.rotation_first = vec4(1.0, 0.0, 0.0, 1.0);
    @if(ORIGINS_ROTATION2) out.rotation_second = input.rotation_second;
    @if(!ORIGINS_ROTATION2) out.rotation_second = vec4(1.0, 0.0, 0.0, 1.0);
    return out;
}
