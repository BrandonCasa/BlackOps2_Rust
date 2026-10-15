import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput,
};
import package::rendering::native_world_three_layers::{
    NativeWorldThreeLayerVaryings, NativeWorldThreeLayerRevealMaterial,
    NativeWorldThreeLayerDoubleRevealMaterial,
    native_world_three_layer_vertex, native_world_three_layer_reveal_fragment,
    native_world_three_layer_double_reveal_fragment,
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
@group(2) @binding(8) var color_map1: texture_2d<f32>;
@group(2) @binding(9) var color_sampler1: sampler;
@group(2) @binding(10) var normal_map1: texture_2d<f32>;
@group(2) @binding(11) var normal_sampler1: sampler;
@group(2) @binding(12) var specular_map1: texture_2d<f32>;
@group(2) @binding(13) var specular_sampler1: sampler;
@group(2) @binding(14) var color_map2: texture_2d<f32>;
@group(2) @binding(15) var color_sampler2: sampler;
@group(2) @binding(16) var normal_map2: texture_2d<f32>;
@group(2) @binding(17) var normal_sampler2: sampler;
@group(2) @binding(18) var specular_map2: texture_2d<f32>;
@group(2) @binding(19) var specular_sampler2: sampler;
@group(2) @binding(20) var<uniform> reveal2: NativeWorldThreeLayerRevealMaterial;
@group(2) @binding(21) var<uniform> double_reveal: NativeWorldThreeLayerDoubleRevealMaterial;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lightmap_uv: vec2<f32>,
    @location(6) layer_uvs: vec4<f32>,
    @location(7) normal_transform1: vec4<f32>,
    @location(8) normal_transform2: vec4<f32>,
}

struct Varyings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) blend_color: vec3<f32>,
    @location(1) uv_lightmap: vec4<f32>,
    @location(2) normal_fog: vec4<f32>,
    @location(3) fog_color: vec3<f32>,
    @location(4) bitangent: vec3<f32>,
    @location(5) tangent: vec3<f32>,
    @location(6) eye_to_vertex: vec3<f32>,
    @location(7) layer_uvs: vec4<f32>,
    @location(8) normal_transform1: vec4<f32>,
    @location(9) normal_transform2: vec4<f32>,
    @location(10) reflection_correction: vec3<f32>,
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let primary = NativeWorldVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz, input.lightmap_uv);
    let result = native_world_three_layer_vertex(primary, input.layer_uvs,
        input.normal_transform1, input.normal_transform2, scene, object);
    return Varyings(result.clip_position, result.blend_color, result.uv_lightmap,
        result.normal_fog, result.fog_color, result.bitangent, result.tangent,
        result.eye_to_vertex, result.layer_uvs, result.normal_transform1,
        result.normal_transform2, result.reflection_correction);
}

fn source_varyings(input: Varyings) -> NativeWorldThreeLayerVaryings {
    return NativeWorldThreeLayerVaryings(input.clip_position, input.blend_color,
        input.uv_lightmap, input.normal_fog, input.fog_color, input.bitangent,
        input.tangent, input.eye_to_vertex, input.layer_uvs, input.normal_transform1,
        input.normal_transform2, input.reflection_correction);
}

fn shade(input: Varyings) -> vec4<f32> {
    return native_world_three_layer_reveal_fragment(source_varyings(input), scene,
        reveal2, color_map, color_sampler, normal_map, normal_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1,
        color_map2, color_sampler2, normal_map2, normal_sampler2,
        specular_map2, specular_sampler2, lightmap_secondary, lightmap_sampler,
        reflection_probe, reflection_sampler);
}

fn shade_double_reveal(input: Varyings) -> vec4<f32> {
    return native_world_three_layer_double_reveal_fragment(source_varyings(input),
        scene, double_reveal, color_map, color_sampler, normal_map, normal_sampler,
        color_map1, color_sampler1, normal_map1, normal_sampler1,
        specular_map1, specular_sampler1, color_map2, color_sampler2,
        normal_map2, normal_sampler2, specular_map2, specular_sampler2,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

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
fn fragment_double_reveal(input: Varyings) -> @location(0) vec4<f32> {
    return shade_double_reveal(input);
}

@fragment
fn fragment_double_reveal_srgb(input: Varyings) -> @location(0) vec4<f32> {
    return attachment_srgb(shade_double_reveal(input));
}
