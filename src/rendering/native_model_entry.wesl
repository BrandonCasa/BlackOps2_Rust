import package::rendering::native_model::{
    NativeModelScene, NativeModelObject, NativeModelVertexInput,
    NativeModelVaryings, native_model_vertex, native_model_fragment,
};
import package::rendering::native_model_variants::{
    NativeModelOcclusion, NativeModelGeometricVertexInput, NativeModelGeometricVaryings,
    native_model_geometric_vertex, native_model_diffuse_fragment,
    native_model_geometric_diffuse_fragment, native_model_alpha_diffuse_fragment,
    native_model_normal_specular_fragment, native_model_alpha_specular_fragment,
    native_model_vertex_occlusion_fragment,
};

@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var normal_map: texture_2d<f32>;
@group(2) @binding(3) var normal_sampler: sampler;
@group(2) @binding(4) var reflection_probe: texture_cube<f32>;
@group(2) @binding(5) var reflection_sampler: sampler;
@group(2) @binding(6) var specular_map: texture_2d<f32>;
@group(2) @binding(7) var specular_sampler: sampler;
@group(2) @binding(8) var<uniform> occlusion: NativeModelOcclusion;
@group(3) @binding(0) var model_lighting: texture_3d<f32>;
@group(3) @binding(1) var model_lighting_sampler: sampler;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
}

struct Varyings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec4<f32>,
    @location(7) grid_coords_and_vis: vec4<f32>,
    @location(8) reflection_correction: vec3<f32>,
}

struct GeometricVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
}

struct GeometricVaryings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(6) eye_to_vertex: vec4<f32>,
    @location(7) grid_coords_and_vis: vec4<f32>,
    @location(8) reflection_correction: vec3<f32>,
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let source = NativeModelVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz);
    let result = native_model_vertex(source, scene, object);
    return Varyings(result.clip_position, result.color, result.uv,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.grid_coords_and_vis, result.reflection_correction);
}

@vertex
fn vertex_geometric(input: GeometricVertexInput) -> GeometricVaryings {
    let result = native_model_geometric_vertex(
        NativeModelGeometricVertexInput(input.position, input.color, input.uv,
            input.packed_normal.xyz), scene, object);
    return GeometricVaryings(result.clip_position, result.color, result.uv,
        result.normal_fog, result.fog_color, result.eye_to_vertex,
        result.grid_coords_and_vis, result.reflection_correction);
}

fn source(input: Varyings) -> NativeModelVaryings {
    return NativeModelVaryings(input.clip_position, input.color, input.uv,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        input.eye_to_vertex, input.grid_coords_and_vis, input.reflection_correction);
}

fn shade(input: Varyings) -> vec4<f32> {
    return native_model_fragment(source(input), scene, color_map, color_sampler,
        normal_map, normal_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
}

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    return shade(input);
}

// The original shader already encodes its result. Undo the attachment transfer
// only when the actual color attachment will apply an sRGB encode itself.
@fragment
fn fragment_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade(input));
}

fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

fn shade_diffuse(input: Varyings) -> vec4<f32> {
    return native_model_diffuse_fragment(source(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        model_lighting, model_lighting_sampler);
}

fn shade_alpha_diffuse(input: Varyings) -> vec4<f32> {
    return native_model_alpha_diffuse_fragment(source(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        model_lighting, model_lighting_sampler);
}

fn shade_geometric_diffuse(input: GeometricVaryings) -> vec4<f32> {
    let geometric = NativeModelGeometricVaryings(input.clip_position, input.color,
        input.uv, input.normal_fog, input.fog_color, input.eye_to_vertex,
        input.grid_coords_and_vis, input.reflection_correction);
    return native_model_geometric_diffuse_fragment(geometric, scene,
        color_map, color_sampler, model_lighting, model_lighting_sampler);
}

fn shade_specular(input: Varyings) -> vec4<f32> {
    return native_model_normal_specular_fragment(source(input), scene, occlusion,
        color_map, color_sampler, normal_map, normal_sampler,
        specular_map, specular_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
}

fn shade_alpha_specular(input: Varyings) -> vec4<f32> {
    return native_model_alpha_specular_fragment(source(input), scene, occlusion,
        color_map, color_sampler, normal_map, normal_sampler,
        specular_map, specular_sampler, model_lighting, model_lighting_sampler,
        reflection_probe, reflection_sampler);
}

fn shade_vertex_occlusion(input: Varyings) -> vec4<f32> {
    return native_model_vertex_occlusion_fragment(source(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        model_lighting, model_lighting_sampler, reflection_probe, reflection_sampler);
}

@fragment
fn fragment_diffuse(input: Varyings) -> @location(0) vec4<f32> {
    return shade_diffuse(input);
}

@fragment
fn fragment_diffuse_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_diffuse(input));
}

@fragment
fn fragment_alpha_diffuse(input: Varyings) -> @location(0) vec4<f32> {
    return shade_alpha_diffuse(input);
}

@fragment
fn fragment_alpha_diffuse_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_alpha_diffuse(input));
}

@fragment
fn fragment_geometric_diffuse(input: GeometricVaryings) -> @location(0) vec4<f32> {
    return shade_geometric_diffuse(input);
}

@fragment
fn fragment_geometric_diffuse_srgb(input: GeometricVaryings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_geometric_diffuse(input));
}

@fragment
fn fragment_specular(input: Varyings) -> @location(0) vec4<f32> {
    return shade_specular(input);
}

@fragment
fn fragment_specular_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_specular(input));
}

@fragment
fn fragment_alpha_specular(input: Varyings) -> @location(0) vec4<f32> {
    return shade_alpha_specular(input);
}

@fragment
fn fragment_alpha_specular_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_alpha_specular(input));
}

@fragment
fn fragment_vertex_occlusion(input: Varyings) -> @location(0) vec4<f32> {
    return shade_vertex_occlusion(input);
}

@fragment
fn fragment_vertex_occlusion_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_vertex_occlusion(input));
}
