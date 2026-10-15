import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
};
import package::rendering::native_world_layers::{
    NativeWorldLayerVaryings, NativeWorldRevealMaterial,
    native_world_layer_vertex, native_world_layer_fragment,
    native_world_reveal_layer_fragment,
    native_world_reveal_specular_layer_fragment,
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
@group(2) @binding(8) var layer_color_map: texture_2d<f32>;
@group(2) @binding(9) var layer_color_sampler: sampler;
@group(2) @binding(10) var layer_normal_map: texture_2d<f32>;
@group(2) @binding(11) var layer_normal_sampler: sampler;
@group(2) @binding(12) var layer_specular_map: texture_2d<f32>;
@group(2) @binding(13) var layer_specular_sampler: sampler;
@group(2) @binding(14) var<uniform> reveal: NativeWorldRevealMaterial;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uv: vec2<f32>,
    @location(7) normal_transform: vec4<f32>,
}

struct Varyings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv_lightmap: vec4<f32>,
    @location(1) normal_fog: vec4<f32>,
    @location(2) fog_color: vec3<f32>,
    @location(3) bitangent: vec3<f32>,
    @location(4) tangent: vec3<f32>,
    @location(5) eye_to_vertex: vec3<f32>,
    @location(6) blend_uv: vec4<f32>,
    @location(7) normal_transform: vec4<f32>,
    @location(8) reflection_correction: vec3<f32>,
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let primary = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_layer_vertex(primary, input.layer_uv,
        input.normal_transform, scene, object);
    return Varyings(result.clip_position, result.uv_lightmap, result.normal_fog,
        result.fog_color, result.bitangent, result.tangent, result.eye_to_vertex,
        result.blend_uv, result.normal_transform, result.reflection_correction);
}

fn source_varyings(input: Varyings) -> NativeWorldLayerVaryings {
    return NativeWorldLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        input.eye_to_vertex, input.blend_uv, input.normal_transform,
        input.reflection_correction);
}

fn shade(input: Varyings) -> vec4<f32> {
    return native_world_layer_fragment(source_varyings(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        layer_color_map, layer_color_sampler, layer_normal_map, layer_normal_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_reveal(input: Varyings) -> vec4<f32> {
    return native_world_reveal_layer_fragment(source_varyings(input), scene, reveal,
        color_map, color_sampler, normal_map, normal_sampler,
        layer_color_map, layer_color_sampler, layer_normal_map, layer_normal_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_reveal_specular(input: Varyings) -> vec4<f32> {
    return native_world_reveal_specular_layer_fragment(source_varyings(input), scene, reveal,
        color_map, color_sampler, normal_map, normal_sampler,
        layer_color_map, layer_color_sampler, layer_normal_map, layer_normal_sampler,
        layer_specular_map, layer_specular_sampler, lightmap_secondary, lightmap_sampler,
        reflection_probe, reflection_sampler);
}

// The source shaders already encode RGB. Cancel the attachment's transfer only
// when the selected pipeline targets an sRGB view, preserving source alpha.
fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    return shade(input);
}

@fragment
fn fragment_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade(input));
}

@fragment
fn fragment_reveal(input: Varyings) -> @location(0) vec4<f32> {
    return shade_reveal(input);
}

@fragment
fn fragment_reveal_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_reveal(input));
}

@fragment
fn fragment_reveal_specular(input: Varyings) -> @location(0) vec4<f32> {
    return shade_reveal_specular(input);
}

@fragment
fn fragment_reveal_specular_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_reveal_specular(input));
}
