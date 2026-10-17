import package::rendering::native_world::sample_material;
// Original model VS4876d431, replace PS49c3292c and multiply PS from
// mc_unlit_multiply_97923qz1; decal blend VS/PS5c3f6d9c.
// See model-layered-unlit-source.json for the non-decal shader hashes.
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
        scene.world, NativeWorldObject(object.world_rows, vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(1.0)));
    return Varyings(output.clip_position, input.color, vec3<f32>(input.uv, output.normal_fog.w), output.fog_color);
}
// Exact source VS62b62074 supplies transmission=1; no fog constants are read.
@vertex
fn vertex_no_fog(input: Input) -> Varyings {
    let position = vec4<f32>(input.position, 1.0);
    let world = vec4<f32>(dot(position, object.world_rows[0]), dot(position, object.world_rows[1]),
        dot(position, object.world_rows[2]), dot(position, object.world_rows[3]));
    let rows = scene.world.view_projection_rows;
    let clip = vec4<f32>(dot(world, rows[0]), dot(world, rows[1]), dot(world, rows[2]), dot(world, rows[3]));
    return Varyings(clip, input.color, vec3<f32>(input.uv, 1.0), vec3<f32>(0.0));
}
// Exact PS62b62074 uses tinted source color and green-weighted luma;
// neither an HDR exposure read nor a color-space square/square-root exists.
fn shade_decal_add(input: Varyings) -> vec4<f32> {
    let color = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv_fog.xy) * input.color;
    let luma = dot(color.rbg, vec3<f32>(0.25, 0.25, 0.5));
    let alpha = (color.a * luma) * input.uv_fog.z;
    return vec4<f32>(alpha * color.rgb, alpha);
}
@fragment
fn fragment_decal_add(input: Varyings) -> @location(0) vec4<f32> { return shade_decal_add(input); }
@fragment
fn fragment_decal_add_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_decal_add(input)); }
fn shade_replace(input: Varyings) -> vec4<f32> {
    let color = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv_fog.xy) * input.color;
    let delta = fma(color.rgb * color.rgb, vec3<f32>(material.scale_rgb.x), -input.fog_color);
    let fogged = fma(vec3<f32>(input.uv_fog.z), delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), color.a);
}
// PS4ea12045 retains straight alpha. RGB receives that alpha before the
// source square-root encoding, after the ordinary unlit fog interpolation.
fn shade_blend(input: Varyings) -> vec4<f32> {
    let color = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv_fog.xy) * input.color;
    let delta = fma(color.rgb * color.rgb, vec3<f32>(material.scale_rgb.x), -input.fog_color);
    let fogged = fma(vec3<f32>(input.uv_fog.z), delta, input.fog_color);
    let covered = color.a * fogged;
    if !(color.a > 0.0) { discard; }
    return vec4<f32>(sqrt(covered * scene.world.hdr_control0.x), color.a);
}
// PS4b1d2e0b uses fog transmission only; fog color and alpha never add to
// or attenuate its emissive RGB. Original alpha blending preserves dest A.
fn shade_add(input: Varyings) -> vec4<f32> {
    let color = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv_fog.xy) * input.color;
    let lit = (color.rgb * color.rgb) * material.scale_rgb.x;
    let transmitted = lit * input.uv_fog.z;
    return vec4<f32>(sqrt(transmitted * scene.world.hdr_control0.x), color.a);
}
fn shade_multiply(input: Varyings) -> vec4<f32> {
    let color = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv_fog.xy) * input.color;
    let delta = fma(color.rgb, color.rgb, vec3<f32>(-1.0));
    return vec4<f32>(fma(vec3<f32>(input.uv_fog.z), delta, vec3<f32>(1.0)), color.a);
}
// PS5c3f6d9c: texture/vertex alpha and fog fade toward the multiply identity.
fn shade_decal_multiply(input: Varyings) -> vec4<f32> {
    let sample = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv_fog.xy);
    let delta = fma(sample.rgb, input.color.rgb, vec3<f32>(-1.0));
    let opacity = (sample.a * input.color.a) * input.uv_fog.z;
    return vec4<f32>(fma(vec3<f32>(opacity), delta, vec3<f32>(1.0)), opacity);
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
fn fragment_blend(input: Varyings) -> @location(0) vec4<f32> { return shade_blend(input); }
@fragment
fn fragment_blend_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_blend(input)); }
@fragment
fn fragment_add(input: Varyings) -> @location(0) vec4<f32> { return shade_add(input); }
@fragment
fn fragment_add_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_add(input)); }
@fragment
fn fragment_multiply(input: Varyings) -> @location(0) vec4<f32> { return shade_multiply(input); }
@fragment
fn fragment_multiply_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_multiply(input)); }
@fragment
fn fragment_decal_multiply(input: Varyings) -> @location(0) vec4<f32> { return shade_decal_multiply(input); }
@fragment
fn fragment_decal_multiply_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_decal_multiply(input)); }

// Source viewmodel-unlit VS89d1b67c ignores vertex color. Its four PS variants
// emit the square directly: no HDR factor, gamma-two encode, or fog omission.
fn shade_viewmodel(input: Varyings, premultiply: bool, alpha_mode: u32) -> vec4<f32> {
    let texel = sample_material(scene.world.texture_mip_scale, color_map, color_sampler, input.uv_fog.xy);
    var emission = texel.rgb * material.scale_rgb.x;
    if premultiply { emission *= texel.a; }
    let fogged = fma(vec3<f32>(input.uv_fog.z), fma(emission, emission, -input.fog_color), input.fog_color);
    var alpha = 1.0;
    if alpha_mode == 1u { alpha = sqrt(texel.a); }
    if alpha_mode == 2u { alpha = texel.a; }
    if alpha_mode != 0u && !(alpha > 0.0) { discard; }
    return vec4<f32>(fogged, alpha);
}
@fragment fn fragment_viewmodel_add(input: Varyings) -> @location(0) vec4<f32> { return shade_viewmodel(input, true, 0u); }
@fragment fn fragment_viewmodel_add_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_viewmodel(input, true, 0u)); }
@fragment fn fragment_viewmodel_sqrt_alpha(input: Varyings) -> @location(0) vec4<f32> { return shade_viewmodel(input, false, 1u); }
@fragment fn fragment_viewmodel_sqrt_alpha_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_viewmodel(input, false, 1u)); }
@fragment fn fragment_viewmodel_alpha(input: Varyings) -> @location(0) vec4<f32> { return shade_viewmodel(input, false, 2u); }
@fragment fn fragment_viewmodel_alpha_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_viewmodel(input, false, 2u)); }
@fragment fn fragment_viewmodel_opaque(input: Varyings) -> @location(0) vec4<f32> { return shade_viewmodel(input, false, 0u); }
@fragment fn fragment_viewmodel_opaque_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_viewmodel(input, false, 0u)); }
