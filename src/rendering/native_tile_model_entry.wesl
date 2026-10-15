// Original COD7 tile blend model LIT programs. Source hashes and disassemblies:
// target/reports/model-tile-blend-source.json. MC and MLV retain their distinct
// original per-vertex lighting inputs; specular R is gloss and GBA is reflectance.
import package::rendering::native_model::{NativeModelScene, NativeModelObject, NativeModelVertexInput};
import package::rendering::native_scripted_model::{
    NativeModelScriptedScene, native_scripted_model_vertex_lighting,
    native_scripted_bitangent,
};
import package::rendering::native_model_variants::{native_model_atlas_irradiance};

struct NativeModelTileConstants {
    scales: vec4<f32>,
    heights_ao: vec4<f32>, // micro1, macro, AO adjustment, micro2.
    reflection: vec4<f32>,
}
@group(0) @binding(0) var<uniform> scene: NativeModelScene;
@group(0) @binding(1) var<uniform> view: NativeModelScriptedScene;
@group(1) @binding(0) var<uniform> object: NativeModelObject;
@group(2) @binding(0) var macro_mix: texture_2d<f32>;
@group(2) @binding(1) var macro_mix_sampler: sampler;
@group(2) @binding(2) var macro_normal: texture_2d<f32>;
@group(2) @binding(3) var macro_normal_sampler: sampler;
@group(2) @binding(4) var micro_normal1: texture_2d<f32>;
@group(2) @binding(5) var micro_normal1_sampler: sampler;
@group(2) @binding(6) var micro_normal2: texture_2d<f32>;
@group(2) @binding(7) var micro_normal2_sampler: sampler;
@group(2) @binding(8) var micro_color1: texture_2d<f32>;
@group(2) @binding(9) var micro_color1_sampler: sampler;
@group(2) @binding(10) var micro_color2: texture_2d<f32>;
@group(2) @binding(11) var micro_color2_sampler: sampler;
@group(2) @binding(12) var micro_specular1: texture_2d<f32>;
@group(2) @binding(13) var micro_specular1_sampler: sampler;
@group(2) @binding(14) var micro_specular2: texture_2d<f32>;
@group(2) @binding(15) var micro_specular2_sampler: sampler;
@group(2) @binding(16) var probe: texture_cube<f32>;
@group(2) @binding(17) var probe_sampler: sampler;
@group(2) @binding(18) var<uniform> material: NativeModelTileConstants;
@group(3) @binding(0) var model_lighting: texture_3d<f32>;
@group(3) @binding(1) var model_lighting_sampler: sampler;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
}
struct VertexLitInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) packed_normal: vec4<f32>,
    @location(4) packed_tangent: vec4<f32>,
    @location(5) lighting: vec4<f32>,
}
struct Varyings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) fog_color: vec3<f32>,
    @location(1) eye_fog: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) normal: vec3<f32>,
    @location(4) tangent_sign: vec4<f32>,
    @location(5) uv: vec2<f32>,
    @location(6) grid_coords_and_vis: vec4<f32>,
    @location(7) reflection_correction: vec3<f32>,
    @location(8) grid_correction: vec3<f32>,
    @location(9) micro_uv: vec4<f32>,
}
fn tile_vertex(input: NativeModelVertexInput, lighting: vec4<f32>, baked: bool) -> Varyings {
    let result = native_scripted_model_vertex_lighting(input, scene, object, lighting, baked);
    return Varyings(result.clip_position, result.fog_color, result.eye_fog, result.color,
        result.normal, result.tangent_sign, result.uv, result.grid_coords_and_vis,
        result.reflection_correction, result.grid_correction, input.uv.xyxy * material.scales);
}
@vertex
fn vertex_scripted(input: VertexInput) -> Varyings {
    return tile_vertex(NativeModelVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz), vec4<f32>(0.0), false);
}
@vertex
fn vertex_scripted_mlv(input: VertexLitInput) -> Varyings {
    return tile_vertex(NativeModelVertexInput(input.position, input.color, input.uv,
        input.packed_normal.xyz, input.packed_tangent.xyz), input.lighting, true);
}
struct TileSurface {
    normal: vec3<f32>,
    color_squared: vec3<f32>,
    layer_weight: f32,
    ao: f32,
}
fn tile_surface(input: Varyings) -> TileSurface {
    let macro_sample = textureSample(macro_mix, macro_mix_sampler, input.uv);
    let weight = macro_sample.g * macro_sample.g;
    let ao = clamp(fma(macro_sample.r, macro_sample.r, material.heights_ao.z), 0.0, 1.0);
    let normal_scale = vec2<f32>(4.015748023986816);
    let normal_bias = vec2<f32>(-2.0157480239868164);
    let macro_slope = fma(textureSample(macro_normal, macro_normal_sampler, input.uv).xy, normal_scale, normal_bias);
    let slope1 = fma(textureSample(micro_normal1, micro_normal1_sampler, input.micro_uv.xy).xy, normal_scale, normal_bias);
    let slope2 = fma(textureSample(micro_normal2, micro_normal2_sampler, input.micro_uv.zw).xy, normal_scale, normal_bias);
    let weight1 = fma(-macro_sample.g, macro_sample.g, 1.0) * material.heights_ao.x;
    let slope = fma(vec2<f32>(material.heights_ao.y), macro_slope,
        fma(vec2<f32>(weight1), slope1, slope2 * (weight * material.heights_ao.w)));
    let geometric = input.normal * inverseSqrt(dot(input.normal, input.normal));
    let bitangent = native_scripted_bitangent(geometric, input.tangent_sign);
    let displaced = fma(vec3<f32>(slope.y), bitangent,
        fma(vec3<f32>(slope.x), input.tangent_sign.xyz, geometric));
    let normal = displaced * inverseSqrt(dot(displaced, displaced));
    let color1 = textureSample(micro_color1, micro_color1_sampler, input.micro_uv.xy).rgb;
    let color2 = textureSample(micro_color2, micro_color2_sampler, input.micro_uv.zw).rgb;
    let colored = fma(vec3<f32>(weight), color2 - color1, color1) * input.color.rgb;
    return TileSurface(normal, colored * colored, weight, ao);
}
fn tile_diffuse_light(input: Varyings, surface: TileSurface) -> vec3<f32> {
    let atlas = native_model_atlas_irradiance(surface.normal, input.grid_coords_and_vis.xyz,
        scene, model_lighting, model_lighting_sampler);
    let ambient = ((atlas * surface.ao) * input.grid_correction) * 32.0;
    let ndotl = clamp(dot(view.sun_position.xyz, surface.normal), 0.0, 1.0);
    let sun = input.grid_coords_and_vis.w * view.sun_diffuse.rgb;
    return fma(vec3<f32>(ndotl), sun, ambient);
}
fn tile_specular(input: Varyings, surface: TileSurface) -> vec3<f32> {
    let sample1 = textureSample(micro_specular1, micro_specular1_sampler, input.micro_uv.xy);
    let sample2 = textureSample(micro_specular2, micro_specular2_sampler, input.micro_uv.zw);
    let specular = fma(vec4<f32>(surface.layer_weight), sample2 - sample1, sample1);
    let gloss = specular.r;
    let reflectance = specular.gba * specular.gba;
    let eye = input.eye_fog.xyz * inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz));
    let reflected = fma(surface.normal, vec3<f32>(-2.0 * dot(eye, surface.normal)), eye);
    let probe_sample = textureSampleLevel(probe, probe_sampler, reflected, fma(gloss, -4.0, 4.0));
    let radiance = probe_sample.rgb * (1.0 / (probe_sample.a + 0.000001));
    let coefficients = fma(vec4<f32>(gloss), vec4<f32>(1.0416667461395264, 0.4749999940395355, 0.018229199573397636, 0.25),
        vec4<f32>(0.0, 0.0, -0.015625, 0.75));
    let grazing = exp2(-9.28 * clamp(dot(surface.normal, -eye), 0.0, 1.0));
    let fresnel = fma(coefficients.x, min(grazing, coefficients.y), coefficients.z);
    let response = clamp(fma(reflectance, vec3<f32>(coefficients.w - fresnel), vec3<f32>(fresnel)), vec3<f32>(0.0), vec3<f32>(1.0));
    let environment = (surface.ao * material.reflection.x) * (radiance * response);
    let half_unnormalized = fma(-input.eye_fog.xyz,
        vec3<f32>(inverseSqrt(dot(input.eye_fog.xyz, input.eye_fog.xyz))), view.sun_position.xyz);
    let half_vector = half_unnormalized * inverseSqrt(dot(half_unnormalized, half_unnormalized));
    let ndoth = clamp(dot(surface.normal, half_vector), 0.0, 1.0);
    let hdotl = clamp(dot(half_vector, view.sun_position.xyz), 0.0, 1.0);
    let exponent = exp2(gloss * 13.0);
    let lobe = exp2(log2(ndoth) * exponent) * fma(exponent, 0.125, 0.25);
    let sun_fresnel = fma(fma(-specular.gba, specular.gba, vec3<f32>(1.0)), vec3<f32>(exp2(-10.0 * hdotl)), reflectance);
    let gloss_factor = clamp(gloss + 0.545, 0.0, 1.0);
    let denominator = fma(hdotl * hdotl, gloss_factor, -gloss_factor) + 1.01;
    let ndotl = clamp(dot(view.sun_position.xyz, surface.normal), 0.0, 1.0);
    let direct = ((sun_fresnel * (lobe * (ndotl / denominator))) * surface.ao)
        * (input.grid_coords_and_vis.w * view.sun_diffuse.rgb);
    return fma(environment, input.reflection_correction, direct);
}
fn shade_diffuse(input: Varyings) -> vec4<f32> {
    let surface = tile_surface(input);
    let delta = fma(surface.color_squared, tile_diffuse_light(input, surface), -input.fog_color);
    let fogged = fma(vec3<f32>(input.eye_fog.w), delta, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}
fn shade_specular(input: Varyings) -> vec4<f32> {
    let surface = tile_surface(input);
    let lit = fma(surface.color_squared, tile_diffuse_light(input, surface), tile_specular(input, surface));
    let fogged = fma(vec3<f32>(input.eye_fog.w), lit - input.fog_color, input.fog_color);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), 1.0);
}
fn attachment_srgb(encoded: vec4<f32>) -> vec4<f32> {
    let linear = select(encoded.rgb / vec3<f32>(12.92),
        pow((encoded.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4)),
        encoded.rgb > vec3<f32>(0.04045));
    return vec4<f32>(linear, encoded.a);
}
@fragment
fn fragment_diffuse(input: Varyings) -> @location(0) vec4<f32> { return shade_diffuse(input); }
@fragment
fn fragment_diffuse_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_diffuse(input)); }
@fragment
fn fragment_specular(input: Varyings) -> @location(0) vec4<f32> { return shade_specular(input); }
@fragment
fn fragment_specular_srgb(input: Varyings) -> @location(0) vec4<f32> { return attachment_srgb(shade_specular(input)); }
