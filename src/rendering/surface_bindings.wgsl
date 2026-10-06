#define_import_path origins::surface_bindings
struct OriginsSurfaceData {
    response: vec4<f32>,
    radiance: vec4<f32>,
    modes: vec4<u32>,
    lighting: vec4<f32>,
    sh_zero: vec4<f32>,
    sh_one: vec4<f32>,
    sh_two: vec4<f32>,
    layers: vec4<u32>,
    reveal: vec4<f32>,
    wetness: vec4<f32>,
    water: vec4<f32>,
    velocity: vec4<f32>,
}
struct AnimationClock { elapsed: vec4<f32>, }
#ifdef BINDLESS
#import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d, bindless_textures_cube}
struct SurfaceIndices {
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
    packed_lightmap: u32,
    lightmap_sampler: u32,
    reflection_map: u32,
    reflection_sampler: u32,
    color0: u32,
    color0_sampler: u32,
    color1: u32,
    color1_sampler: u32,
    color2: u32,
    color2_sampler: u32,
    color3: u32,
    color3_sampler: u32,
    third_normal: u32,
    third_normal_sampler: u32,
    third_specular: u32,
    third_specular_sampler: u32,
    instance_lighting: u32,
    animation_clock: u32,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(129) var<storage> surface_indices: array<SurfaceIndices>;
@group(#{MATERIAL_BIND_GROUP}) @binding(130) var<storage> surface_parameters: array<OriginsSurfaceData>;
@group(#{MATERIAL_BIND_GROUP}) @binding(132) var<storage, read> animation_clocks: binding_array<AnimationClock>;
#else
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> surface_uniform: OriginsSurfaceData;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var normal_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var second_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var second_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var specular_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var specular_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var second_specular: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var second_specular_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var control_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var control_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var packed_lightmap: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var lightmap_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(113) var reflection_map: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(114) var reflection_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(115) var color0: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(116) var color0_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(117) var color1: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(118) var color1_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(119) var color2: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(120) var color2_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(121) var color3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(122) var color3_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(123) var third_normal: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(124) var third_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(125) var third_specular: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(126) var third_specular_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(127) var instance_lighting: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(128) var<storage, read> animation_clock: AnimationClock;
#endif
fn parameters_for(slot: u32) -> OriginsSurfaceData {
#ifdef BINDLESS
    return surface_parameters[surface_indices[slot].parameters];
#else
    return surface_uniform;
#endif
}
fn sample_normal_map(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].normal_map], bindless_samplers_filtering[surface_indices[slot].normal_sampler], uv);
#else
    return textureSample(normal_map, normal_sampler, uv);
#endif
}
fn sample_second_normal(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].second_normal], bindless_samplers_filtering[surface_indices[slot].second_normal_sampler], uv);
#else
    return textureSample(second_normal, second_normal_sampler, uv);
#endif
}
fn sample_specular_map(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].specular_map], bindless_samplers_filtering[surface_indices[slot].specular_sampler], uv);
#else
    return textureSample(specular_map, specular_sampler, uv);
#endif
}
fn sample_second_specular(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].second_specular], bindless_samplers_filtering[surface_indices[slot].second_specular_sampler], uv);
#else
    return textureSample(second_specular, second_specular_sampler, uv);
#endif
}
fn sample_control_map(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].control_map], bindless_samplers_filtering[surface_indices[slot].control_sampler], uv);
#else
    return textureSample(control_map, control_sampler, uv);
#endif
}
fn sample_packed_lightmap(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].packed_lightmap], bindless_samplers_filtering[surface_indices[slot].lightmap_sampler], uv);
#else
    return textureSample(packed_lightmap, lightmap_sampler, uv);
#endif
}
fn sample_color0(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].color0], bindless_samplers_filtering[surface_indices[slot].color0_sampler], uv);
#else
    return textureSample(color0, color0_sampler, uv);
#endif
}
fn sample_color1(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].color1], bindless_samplers_filtering[surface_indices[slot].color1_sampler], uv);
#else
    return textureSample(color1, color1_sampler, uv);
#endif
}
fn sample_color2(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].color2], bindless_samplers_filtering[surface_indices[slot].color2_sampler], uv);
#else
    return textureSample(color2, color2_sampler, uv);
#endif
}
fn sample_color3(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].color3], bindless_samplers_filtering[surface_indices[slot].color3_sampler], uv);
#else
    return textureSample(color3, color3_sampler, uv);
#endif
}
fn sample_third_normal(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].third_normal], bindless_samplers_filtering[surface_indices[slot].third_normal_sampler], uv);
#else
    return textureSample(third_normal, third_normal_sampler, uv);
#endif
}
fn sample_third_specular(slot: u32, uv: vec2<f32>) -> vec4<f32> {
#ifdef BINDLESS
    return textureSample(bindless_textures_2d[surface_indices[slot].third_specular], bindless_samplers_filtering[surface_indices[slot].third_specular_sampler], uv);
#else
    return textureSample(third_specular, third_specular_sampler, uv);
#endif
}
fn atlas_dimensions(slot: u32) -> vec2<u32> {
#ifdef BINDLESS
    return textureDimensions(bindless_textures_2d[surface_indices[slot].packed_lightmap]);
#else
    return textureDimensions(packed_lightmap);
#endif
}
fn reflection_levels(slot: u32) -> u32 {
#ifdef BINDLESS
    return textureNumLevels(bindless_textures_cube[surface_indices[slot].reflection_map]);
#else
    return textureNumLevels(reflection_map);
#endif
}
fn sample_reflection(slot: u32, direction: vec3<f32>, level: f32) -> vec4<f32> {
#ifdef BINDLESS
    return textureSampleLevel(bindless_textures_cube[surface_indices[slot].reflection_map], bindless_samplers_filtering[surface_indices[slot].reflection_sampler], direction, level);
#else
    return textureSampleLevel(reflection_map, reflection_sampler, direction, level);
#endif
}
fn instance_color(slot: u32, index: u32) -> vec3<f32> {
#ifdef BINDLESS
    let width = textureDimensions(bindless_textures_2d[surface_indices[slot].instance_lighting]).x;
    return textureLoad(bindless_textures_2d[surface_indices[slot].instance_lighting], vec2<i32>(i32(index % width), i32(index / width)), 0).rgb;
#else
    let width = textureDimensions(instance_lighting).x;
    return textureLoad(instance_lighting, vec2<i32>(i32(index % width), i32(index / width)), 0).rgb;
#endif
}

fn animation_time(slot: u32) -> f32 {
#ifdef BINDLESS
    return animation_clocks[surface_indices[slot].animation_clock].elapsed.x;
#else
    return animation_clock.elapsed.x;
#endif
}
