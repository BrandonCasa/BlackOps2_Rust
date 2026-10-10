import origins::surface_effect::Effect;
@if(BINDLESS) import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d};

enable wgpu_binding_array;
var<private> active_surface_slot: u32;
fn set_surface_slot(slot: u32) { active_surface_slot = slot; }
fn current_surface_slot() -> u32 { return active_surface_slot; }
struct OriginsSurfaceData {
    response: vec4<f32>,
    modes: vec4<u32>,
    layers: vec4<u32>,
    reveal: vec4<f32>,
    wetness: vec4<f32>,
    water: vec4<f32>,
    velocity: vec4<f32>,
    effect: Effect,
    native: NativeWeaponParameters,
}
struct NativeWeaponParameters {
    response: vec4<f32>,
    detail: vec4<f32>,
    ember: vec4<f32>,
    motion: vec4<f32>,
    flicker: vec4<f32>,
    layer_scale_offset: array<vec4<f32>,3>,
    layer_scroll_rotation: array<vec4<f32>,3>,
    layer_origin_depth: array<vec4<f32>,3>,
    layer_alpha: vec4<f32>,
    staff: vec4<f32>,
    staff_emission: vec4<f32>,
    camo: vec4<f32>,
    camo_extra: vec4<f32>,
    camo_flags: vec4<u32>,
    weapon_detail: vec4<f32>,
    kind: vec4<u32>,
}
struct AnimationClock { elapsed: vec4<f32>, }
@if(BINDLESS) struct SurfaceIndices {
    parameters: u32,
    normal_map: u32,
    normal_sampler: u32,
    second_normal: u32,
    second_normal_sampler: u32,
    specular_map: u32,
    specular_sampler: u32,
    second_specular: u32,
    second_specular_sampler: u32,
    control_map: u32,
    control_sampler: u32,
    color_base: u32,
    color0_sampler: u32,
    color_first: u32,
    color1_sampler: u32,
    color_second: u32,
    color2_sampler: u32,
    color_third: u32,
    color3_sampler: u32,
    third_normal: u32,
    third_normal_sampler: u32,
    third_specular: u32,
    third_specular_sampler: u32,
    animation_clock: u32,
}
@if(BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(124) var<storage> surface_indices: array<SurfaceIndices>;
@if(BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(125) var<storage> surface_parameters: array<OriginsSurfaceData>;
@if(BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(126) var<storage, read> animation_clocks: binding_array<AnimationClock>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> surface_uniform: OriginsSurfaceData;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(101) var normal_map: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(102) var normal_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(103) var second_normal: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(104) var second_normal_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(105) var specular_map: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(106) var specular_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(107) var second_specular: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(108) var second_specular_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(109) var control_map: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(110) var control_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(111) var color_base: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(112) var color0_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(113) var color_first: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(114) var color1_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(115) var color_second: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(116) var color2_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(117) var color_third: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(118) var color3_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(119) var third_normal: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(120) var third_normal_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(121) var third_specular: texture_2d<f32>;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(122) var third_specular_sampler: sampler;
@if(!BINDLESS) @group(constants::MATERIAL_BIND_GROUP) @binding(123) var<storage, read> animation_clock: AnimationClock;
fn parameters_for(slot: u32) -> OriginsSurfaceData {
    @if(BINDLESS) return surface_parameters[surface_indices[slot].parameters];
    @if(!BINDLESS) return surface_uniform;
}
fn sample_normal_map(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].normal_map], bindless_samplers_filtering[surface_indices[slot].normal_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(normal_map, normal_sampler, uv, gradient[0], gradient[1]);
}
fn sample_second_normal(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].second_normal], bindless_samplers_filtering[surface_indices[slot].second_normal_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(second_normal, second_normal_sampler, uv, gradient[0], gradient[1]);
}
fn sample_specular_map(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].specular_map], bindless_samplers_filtering[surface_indices[slot].specular_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(specular_map, specular_sampler, uv, gradient[0], gradient[1]);
}
fn sample_second_specular(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].second_specular], bindless_samplers_filtering[surface_indices[slot].second_specular_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(second_specular, second_specular_sampler, uv, gradient[0], gradient[1]);
}
fn sample_control_map(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].control_map], bindless_samplers_filtering[surface_indices[slot].control_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(control_map, control_sampler, uv, gradient[0], gradient[1]);
}
fn sample_color_base_grad(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].color_base], bindless_samplers_filtering[surface_indices[slot].color0_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(color_base, color0_sampler, uv, gradient[0], gradient[1]);
}
fn sample_color_first_grad(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].color_first], bindless_samplers_filtering[surface_indices[slot].color1_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(color_first, color1_sampler, uv, gradient[0], gradient[1]);
}
fn sample_color_second_grad(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].color_second], bindless_samplers_filtering[surface_indices[slot].color2_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(color_second, color2_sampler, uv, gradient[0], gradient[1]);
}
fn sample_color_third_grad(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].color_third], bindless_samplers_filtering[surface_indices[slot].color3_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(color_third, color3_sampler, uv, gradient[0], gradient[1]);
}
fn sample_third_normal(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].third_normal], bindless_samplers_filtering[surface_indices[slot].third_normal_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(third_normal, third_normal_sampler, uv, gradient[0], gradient[1]);
}
fn sample_third_specular(slot: u32, uv: vec2<f32>, gradient: mat2x2<f32>) -> vec4<f32> {
    @if(BINDLESS) return textureSampleGrad(bindless_textures_2d[surface_indices[slot].third_specular], bindless_samplers_filtering[surface_indices[slot].third_specular_sampler], uv, gradient[0], gradient[1]);
    @if(!BINDLESS) return textureSampleGrad(third_specular, third_specular_sampler, uv, gradient[0], gradient[1]);
}
fn animation_time(slot: u32) -> f32 {
    @if(BINDLESS) return animation_clocks[surface_indices[slot].animation_clock].elapsed.x;
    @if(!BINDLESS) return animation_clock.elapsed.x;
}
fn diagnostic_mode(slot: u32) -> u32 {
    @if(BINDLESS) return u32(animation_clocks[surface_indices[slot].animation_clock].elapsed.y);
    @if(!BINDLESS) return u32(animation_clock.elapsed.y);
}
fn reflection_diagnostic_enabled() -> bool {
    return diagnostic_mode(current_surface_slot()) == 6u;
}

fn surface_diagnostic_mode() -> u32 { return diagnostic_mode(current_surface_slot()); }
var<private> native_lighting_scales: vec3<f32>;
fn set_native_lighting_scales(scales: vec3<f32>) { native_lighting_scales = scales; }
fn native_direct_specular_scale() -> f32 {
    if parameters_for(current_surface_slot()).native.kind.x == 0u { return 1.0; }
    return native_lighting_scales.x;
}
fn native_indirect_diffuse_scale() -> f32 {
    if parameters_for(current_surface_slot()).native.kind.x == 0u { return 1.0; }
    return native_lighting_scales.y;
}
fn native_reflection_scale() -> f32 {
    if parameters_for(current_surface_slot()).native.kind.x == 0u { return 1.0; }
    return native_lighting_scales.z;
}
var<private> native_diffuse_direction: vec3<f32>;
fn set_native_diffuse_normal(normal: vec3<f32>) { native_diffuse_direction = normal; }
fn native_diffuse_normal(normal: vec3<f32>) -> vec3<f32> {
    if parameters_for(current_surface_slot()).native.kind.x == 0u { return normal; }
    return native_diffuse_direction;
}

var<private> native_gloss_value: f32;
fn set_native_gloss(gloss: f32) { native_gloss_value = gloss; }
fn native_enabled() -> bool { return parameters_for(current_surface_slot()).native.kind.x != 0u; }
fn native_gloss() -> f32 { return native_gloss_value; }
fn native_mip(fallback: f32) -> f32 {
    if !native_enabled() { return fallback; }
    return max(4.0*(1.0-native_gloss_value),0.0);
}
fn native_environment_fresnel(f0: vec3<f32>, nv: f32) -> vec3<f32> {
    let kind = parameters_for(current_surface_slot()).native.kind;
    if kind.x == 7u || (kind.x == 10u && parameters_for(current_surface_slot()).native.camo_flags.w != 0u) {
        return vec3(clamp(0.75+0.25*native_gloss_value,0.0,1.0));
    }
    let g = native_gloss_value;
    let bias = g*1.041667*min(exp2(-9.28*nv),g*0.475)+g*0.018229-0.015625;
    return clamp(f0*(0.75+0.25*g-bias)+vec3(bias),vec3(0.0),vec3(1.0));
}

fn native_specular(f0: vec3<f32>, nh: f32, lh: f32) -> vec3<f32> {
    let exponent = exp2(13.0*native_gloss_value);
    let distribution = pow(max(nh,0.0),exponent)*(0.25+0.125*exponent);
    let visibility = 1.0/(1.01+(lh*lh-1.0)*clamp(native_gloss_value+0.545,0.0,1.0));
    return (f0+(vec3(1.0)-f0)*exp2(-10.0*lh))*distribution*visibility;
}
