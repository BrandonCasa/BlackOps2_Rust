import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
    NativeWorldVaryings, native_world_vertex,
};
import package::rendering::native_world_plain::{
    NativeWorldGeometricVertexInput,
    native_world_diffuse_vertex, native_world_diffuse_fragment,
    native_world_normal_specular_fragment,
    native_world_geometric_specular_vertex, native_world_geometric_specular_fragment,
};

@group(0) @binding(0) var<uniform> scene: NativeWorldScene;
@group(1) @binding(0) var<uniform> object: NativeWorldObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var normal_map: texture_2d<f32>;
@group(2) @binding(3) var normal_sampler: sampler;
@group(2) @binding(4) var lightmap_secondary: texture_2d<f32>;
@group(2) @binding(5) var lightmap_sampler: sampler;
@group(2) @binding(6) var reflection_probe: texture_cube<f32>;
@group(2) @binding(7) var reflection_sampler: sampler;
@group(2) @binding(8) var specular_map: texture_2d<f32>;
@group(2) @binding(9) var specular_sampler: sampler;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
}

struct GeometricVertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
}

struct DiffuseVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
}

struct SpecularVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
    @location(7) reflection_correction: vec3<f32>,
}

struct GeometricSpecularVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
    @location(7) reflection_correction: vec3<f32>,
}

fn primary(input: VertexInput) -> NativeWorldVertexInput {
    return NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
}

@vertex
fn vertex_diffuse(input: VertexInput) -> DiffuseVaryings {
    let result = native_world_diffuse_vertex(primary(input), scene, object);
    return DiffuseVaryings(result.clip_position, result.color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex);
}

@vertex
fn vertex_specular(input: VertexInput) -> SpecularVaryings {
    // The selected source VS is the same b89a2252 binary as the original port.
    let result = native_world_vertex(primary(input), scene, object);
    return SpecularVaryings(result.clip_position, result.color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.reflection_correction);
}

@vertex
fn vertex_geometric_specular(input: GeometricVertexInput) -> GeometricSpecularVaryings {
    let source = NativeWorldGeometricVertexInput(input.position.xyz, input.color,
        input.uv, input.packed_normal.xyz, input.lightmap_uv);
    let result = native_world_geometric_specular_vertex(source, scene, object);
    return GeometricSpecularVaryings(result.clip_position, result.color,
        result.uv_lightmap, result.normal_fog, result.fog_color,
        result.eye_to_vertex, result.reflection_correction);
}

fn shade_diffuse(input: DiffuseVaryings) -> vec4<f32> {
    var source: NativeWorldVaryings;
    source.clip_position = input.clip_position;
    source.color = input.color;
    source.uv_lightmap = input.uv_lightmap;
    source.normal_fog = input.normal_fog;
    source.fog_color = input.fog_color;
    source.bitangent = input.bitangent;
    source.tangent = input.tangent;
    source.eye_to_vertex = input.eye_to_vertex;
    return native_world_diffuse_fragment(source, scene, color_map, color_sampler,
        normal_map, normal_sampler, lightmap_secondary, lightmap_sampler);
}

fn shade_specular(input: SpecularVaryings) -> vec4<f32> {
    let source = NativeWorldVaryings(input.clip_position, input.color,
        input.uv_lightmap, input.normal_fog, input.fog_color, input.bitangent,
        input.tangent, input.eye_to_vertex, input.reflection_correction);
    return native_world_normal_specular_fragment(source, scene,
        color_map, color_sampler, normal_map, normal_sampler, specular_map,
        specular_sampler, lightmap_secondary, lightmap_sampler,
        reflection_probe, reflection_sampler);
}

fn shade_geometric_specular(input: GeometricSpecularVaryings) -> vec4<f32> {
    var source: NativeWorldVaryings;
    source.clip_position = input.clip_position;
    source.color = input.color;
    source.uv_lightmap = input.uv_lightmap;
    source.normal_fog = input.normal_fog;
    source.fog_color = input.fog_color;
    source.eye_to_vertex = input.eye_to_vertex;
    source.reflection_correction = input.reflection_correction;
    return native_world_geometric_specular_fragment(source, scene,
        color_map, color_sampler, specular_map, specular_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

// Source programs return encoded RGB. Cancel an sRGB attachment's transfer
// only when that view is selected; preserve the source alpha.
fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

@fragment
fn fragment_diffuse(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return shade_diffuse(input);
}

@fragment
fn fragment_diffuse_srgb(input: DiffuseVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse(input));
}

@fragment
fn fragment_specular(input: SpecularVaryings) -> @location(0) vec4<f32> {
    return shade_specular(input);
}

@fragment
fn fragment_specular_srgb(input: SpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_specular(input));
}

@fragment
fn fragment_geometric_specular(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_specular(input);
}

@fragment
fn fragment_geometric_specular_srgb(input: GeometricSpecularVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_specular(input));
}
