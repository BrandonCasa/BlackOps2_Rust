import package::rendering::native_world::NativeWorldObject;
import package::rendering::native_water::{
    NativeWaterScene, NativeWaterMaterial, NativeWaterSunLight, NativeWaterPointLight,
    NativeWaterProjectedLight, NativeWaterSpotCookie, NativeWaterSpotCone,
    NativeWaterVertexInput, NativeWaterVaryings, native_water_vertex,
    native_water_fragment, native_water_sun_fragment, native_water_spot_fragment,
    native_water_square_fragment, native_water_round_fragment, native_water_omni_fragment,
};

@group(0) @binding(0) var<uniform> scene: NativeWaterScene;
@group(0) @binding(1) var float_z: texture_2d<f32>;
@group(0) @binding(2) var float_z_sampler: sampler;
@group(0) @binding(3) var resolved_post_sun: texture_2d<f32>;
@group(0) @binding(4) var resolved_post_sun_sampler: sampler;
@group(1) @binding(0) var<uniform> object: NativeWorldObject;
@group(2) @binding(0) var<uniform> material: NativeWaterMaterial;
@group(2) @binding(1) var normal_map00: texture_2d<f32>;
@group(2) @binding(2) var normal_sampler00: sampler;
@group(2) @binding(3) var normal_map01: texture_2d<f32>;
@group(2) @binding(4) var normal_sampler01: sampler;
@group(2) @binding(5) var lightmap_secondary: texture_2d<f32>;
@group(2) @binding(6) var lightmap_sampler: sampler;
@group(2) @binding(7) var reflection_probe: texture_cube<f32>;
@group(2) @binding(8) var reflection_sampler: sampler;

// This compact block contains exactly the selected source cb0 words. Each
// fragment entry reads only its original light inputs; unused words are padding.
struct WaterLightConstants {
    position: vec4<f32>,
    diffuse: vec4<f32>,
    attenuation: vec4<f32>,
    falloff_a: vec4<f32>,
    falloff_b: vec4<f32>,
    spot_matrix_rows: array<vec4<f32>, 3>,
    spot_direction: vec4<f32>,
    cookie_slide: vec4<f32>,
    cone_control1: vec4<f32>,
    cone_control2: vec4<f32>,
}
@group(3) @binding(0) var<uniform> light: WaterLightConstants;
// Only the SPOT and SQUARE entries read these bindings. LIT has no group3;
// SUN, ROUND and OMNI use a uniform-only group3, without a placeholder cookie.
@group(3) @binding(1) var attenuation_map: texture_2d<f32>;
@group(3) @binding(2) var attenuation_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) lightmap_uv: vec2<f32>,
}

struct Varyings {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) position_lightmap: vec4<f32>,
    @location(1) @interpolate(perspective, centroid) eye_and_clip_w: vec4<f32>,
    @location(2) normal_uv0: vec4<f32>,
    @location(3) normal_uv1: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) reflection_correction: vec3<f32>,
}

@vertex
fn vertex(input: VertexInput) -> Varyings {
    let result = native_water_vertex(NativeWaterVertexInput(input.position, input.color, input.lightmap_uv), scene, material, object);
    return Varyings(result.clip_position, result.position_lightmap, result.eye_and_clip_w,
        result.normal_uv0, result.normal_uv1, result.color, result.reflection_correction);
}

fn source_varyings(input: Varyings) -> NativeWaterVaryings {
    return NativeWaterVaryings(input.clip_position, input.position_lightmap, input.eye_and_clip_w,
        input.normal_uv0, input.normal_uv1, input.color, input.reflection_correction);
}

fn point_light() -> NativeWaterPointLight {
    return NativeWaterPointLight(light.position, light.diffuse, light.attenuation, light.falloff_a, light.falloff_b);
}

fn projected_light() -> NativeWaterProjectedLight {
    return NativeWaterProjectedLight(point_light(), light.spot_matrix_rows);
}

fn cookie() -> NativeWaterSpotCookie {
    return NativeWaterSpotCookie(light.spot_direction, light.cookie_slide);
}

@fragment
fn fragment(input: Varyings) -> @location(0) vec4<f32> {
    return native_water_fragment(source_varyings(input), scene, material,
        normal_map00, normal_sampler00, normal_map01, normal_sampler01,
        float_z, float_z_sampler, resolved_post_sun, resolved_post_sun_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

@fragment
fn fragment_sun(input: Varyings) -> @location(0) vec4<f32> {
    return native_water_sun_fragment(source_varyings(input), scene, material,
        NativeWaterSunLight(light.position, light.diffuse),
        normal_map00, normal_sampler00, normal_map01, normal_sampler01,
        float_z, float_z_sampler, resolved_post_sun, resolved_post_sun_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

@fragment
fn fragment_spot(input: Varyings) -> @location(0) vec4<f32> {
    return native_water_spot_fragment(source_varyings(input), scene, material,
        projected_light(), NativeWaterSpotCone(light.cone_control1, light.cone_control2), cookie(),
        normal_map00, normal_sampler00, normal_map01, normal_sampler01,
        float_z, float_z_sampler, resolved_post_sun, resolved_post_sun_sampler,
        attenuation_map, attenuation_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

@fragment
fn fragment_square(input: Varyings) -> @location(0) vec4<f32> {
    return native_water_square_fragment(source_varyings(input), scene, material, projected_light(), cookie(),
        normal_map00, normal_sampler00, normal_map01, normal_sampler01,
        float_z, float_z_sampler, resolved_post_sun, resolved_post_sun_sampler,
        attenuation_map, attenuation_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

@fragment
fn fragment_round(input: Varyings) -> @location(0) vec4<f32> {
    return native_water_round_fragment(source_varyings(input), scene, material, projected_light(),
        normal_map00, normal_sampler00, normal_map01, normal_sampler01,
        float_z, float_z_sampler, resolved_post_sun, resolved_post_sun_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}

@fragment
fn fragment_omni(input: Varyings) -> @location(0) vec4<f32> {
    return native_water_omni_fragment(source_varyings(input), scene, material, point_light(),
        normal_map00, normal_sampler00, normal_map01, normal_sampler01,
        float_z, float_z_sampler, resolved_post_sun, resolved_post_sun_sampler,
        lightmap_secondary, lightmap_sampler, reflection_probe, reflection_sampler);
}
