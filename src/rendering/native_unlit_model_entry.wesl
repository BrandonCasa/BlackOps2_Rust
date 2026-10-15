// Original model VS4876d431, replace PS49c3292c and multiply PS from
// mc_unlit_multiply_97923qz1. See model-layered-unlit-source.json for hashes.
import package::rendering::native_model::{NativeModelScene, NativeModelObject};
import package::rendering::native_world::{NativeWorldObject};
import package::rendering::native_world_plain::{NativeWorldGeometricVertexInput, native_world_position_fog};
@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(2) @binding(0) var color_map: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
struct UnlitScale { scale_rgb: vec4<f32> }
@group(2) @binding(2) var<uniform> material: UnlitScale;
struct Input {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}
struct Varyings {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv_fog: vec3<f32>,
    @location(2) fog_color: vec3<f32>,
}
@vertex
fn vertex_unlit(input: Input) -> Varyings {
    // The shared transform/fog helper consumes only world_rows, position and UV.
    let output = native_world_position_fog(
        NativeWorldGeometricVertexInput(input.position, input.color, input.uv, vec3<f32>(0.0), vec2<f32>(0.0)),
        scene.world, NativeWorldObject(object.world_rows, vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0)));
    return Varyings(output.clip_position, input.color, vec3<f32>(input.uv, output.normal_fog.w), output.fog_color);
}
fn shade_replace(input: Varyings) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_fog.xy) * input.color;
    let delta = fma(color.rgb * color.rgb, vec3<f32>(material.scale_rgb.x), -input.fog_color);
    let fogged = fma(vec3<f32>(input.uv_fog.z), delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), color.a);
}
fn shade_multiply(input: Varyings) -> vec4<f32> {
    let color = textureSample(color_map, color_sampler, input.uv_fog.xy) * input.color;
    let delta = fma(color.rgb, color.rgb, vec3<f32>(-1.0));
    return vec4<f32>(fma(vec3<f32>(input.uv_fog.z), delta, vec3<f32>(1.0)), color.a);
}
fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}
@fragment
fn fragment_replace(input: Varyings) -> @location(0) vec4<f32> { return shade_replace(input); }
@fragment
fn fragment_replace_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_replace(input)); }
@fragment
fn fragment_multiply(input: Varyings) -> @location(0) vec4<f32> { return shade_multiply(input); }
@fragment
fn fragment_multiply_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_multiply(input)); }
