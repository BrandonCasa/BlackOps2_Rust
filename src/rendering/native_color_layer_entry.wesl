import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
};
import package::rendering::native_world_color_layers::{
    NativeWorldColorLayerVaryings, native_world_color_layer_vertex,
    native_world_color_multiply_fragment, native_world_color_blend_fragment,
    native_world_color_add_fragment,
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

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    // Original PACKED_WORLD_T1N0 auxiliary stream: one half2, stride4.
    @location(6) layer_uv: vec2<f32>,
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
    @location(7) reflection_correction: vec3<f32>,
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let primary = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_color_layer_vertex(primary, input.layer_uv, scene, object);
    return Varyings(result.clip_position, result.uv_lightmap, result.normal_fog,
        result.fog_color, result.bitangent, result.tangent, result.eye_to_vertex,
        result.blend_uv, result.reflection_correction);
}

fn source_varyings(input: Varyings) -> NativeWorldColorLayerVaryings {
    return NativeWorldColorLayerVaryings(input.clip_position, input.uv_lightmap,
        input.normal_fog, input.fog_color, input.bitangent, input.tangent,
        input.eye_to_vertex, input.blend_uv, input.reflection_correction);
}

fn shade_multiply(input: Varyings) -> vec4<f32> {
    return native_world_color_multiply_fragment(source_varyings(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        layer_color_map, layer_color_sampler, lightmap_secondary,
        lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_blend(input: Varyings) -> vec4<f32> {
    return native_world_color_blend_fragment(source_varyings(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        layer_color_map, layer_color_sampler, lightmap_secondary,
        lightmap_sampler, reflection_probe, reflection_sampler);
}

fn shade_add(input: Varyings) -> vec4<f32> {
    return native_world_color_add_fragment(source_varyings(input), scene,
        color_map, color_sampler, normal_map, normal_sampler,
        layer_color_map, layer_color_sampler, lightmap_secondary,
        lightmap_sampler, reflection_probe, reflection_sampler);
}

// Preserve source encoded RGB and the selected program's alpha. Cancel the
// attachment's transfer only for an sRGB target.
fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}

@fragment
fn fragment_multiply(input: Varyings) -> @location(0) vec4<f32> {
    return shade_multiply(input);
}

@fragment
fn fragment_multiply_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_multiply(input));
}

@fragment
fn fragment_blend(input: Varyings) -> @location(0) vec4<f32> {
    return shade_blend(input);
}

@fragment
fn fragment_blend_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_blend(input));
}

@fragment
fn fragment_add(input: Varyings) -> @location(0) vec4<f32> {
    return shade_add(input);
}

@fragment
fn fragment_add_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_add(input));
}
