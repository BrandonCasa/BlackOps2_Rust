import package::rendering::native_world::sample_material;
import package::rendering::native_world::{
    NativeWorldScene, NativeWorldObject, NativeWorldVertexInput, native_world_vertex,
};
import package::rendering::native_world_layers::{
    NativeWorldLayerVaryings, native_world_layer_probe, native_world_layer_lighting,
};

// T6 world LIT color layers, recovered rendering-20261012/{source,shaders}.
// Shared VS layer_lmap_9bf34854 SHA256:
// d3b6732db56f18a75132c31ece2597799a94d66b219ebc23f3e616cce050c84a.
// lit_sm_r0c0n0x0_m1c1: LIT layer_lmap_3696a21d, PS254af097 SHA256:
// 1c9d54db65a8c597cbc2149d5730cb016743d075896b5dd9e7f43a470a8e1836.
// lit_sm_r0c0n0x0_b1c1: LIT layer_lmap_3c46810f, PS9f70e132 SHA256:
// bdca2be08ba60858d7692223e9b16fd1e88a75e2447b79e846c09a1a45f5ca10.
// lit_sm_r0c0n0x0_a1c1: LIT layer_lmap_2ad17cfa, PSb50e886e SHA256:
// 7c6ce32896db8e07125e71468e82c1c06234c3222752a807227719acc59b9e2a.

struct NativeWorldColorLayerVaryings {
    clip_position: vec4<f32>,
    uv_lightmap: vec4<f32>,
    normal_fog: vec4<f32>,
    fog_color: vec3<f32>,
    bitangent: vec3<f32>,
    tangent: vec3<f32>,
    eye_to_vertex: vec3<f32>,
    blend_uv: vec4<f32>, // Native TEXCOORD6: primary RG, auxiliary UV.
    reflection_correction: vec3<f32>,
}

// PACKED_WORLD_T1N0: original primary stride36, auxiliary stride4 containing
// one Float16x2 UV. There is no normal transform and no second normal input.
// Source9bf34854 position/fog/basis/SH instructions match native_world_vertex;
// only its color outputs are replaced by primary RG and the layer UV.
fn native_world_color_layer_vertex(
    primary: NativeWorldVertexInput,
    layer_uv: vec2<f32>,
    scene: NativeWorldScene,
    object: NativeWorldObject,
) -> NativeWorldColorLayerVaryings {
    let world = native_world_vertex(primary, scene, object);
    return NativeWorldColorLayerVaryings(world.clip_position, world.uv_lightmap,
        world.normal_fog, world.fog_color, world.bitangent, world.tangent,
        world.eye_to_vertex, vec4<f32>(primary.color.rg, layer_uv),
        world.reflection_correction);
}

// Adapt common fields for the proven-identical probe/lightmap/fog/HDR tails.
// Those helpers never read normal_transform; no such source vertex attribute
// or GPU input exists in any of these color-only layer programs.
fn native_world_color_layer_lighting_input(
    input: NativeWorldColorLayerVaryings,
) -> NativeWorldLayerVaryings {
    var lighting_input: NativeWorldLayerVaryings;
    lighting_input.clip_position = input.clip_position;
    lighting_input.uv_lightmap = input.uv_lightmap;
    lighting_input.normal_fog = input.normal_fog;
    lighting_input.fog_color = input.fog_color;
    lighting_input.bitangent = input.bitangent;
    lighting_input.tangent = input.tangent;
    lighting_input.eye_to_vertex = input.eye_to_vertex;
    lighting_input.blend_uv = input.blend_uv;
    lighting_input.reflection_correction = input.reflection_correction;
    return lighting_input;
}

// All three PS programs use only the base normal-map slopes, independently
// of the layer weight. Their normal instructions are identical.
fn native_world_color_layer_normal(
    input: NativeWorldColorLayerVaryings,
    normal_sample: vec2<f32>,
) -> vec3<f32> {
    let slopes = fma(normal_sample, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164));
    let mapped_normal = fma(vec3<f32>(slopes.y), input.bitangent,
        fma(vec3<f32>(slopes.x), input.tangent, input.normal_fog.xyz));
    return inverseSqrt(dot(mapped_normal, mapped_normal)) * mapped_normal;
}

// Required native slots for every function below: colorMap t0/s0,
// normalMap t1/s1, colorMap1 t2/s2, secondary lightmap t13/s13, probe t15/s15.
// Preserve original UNORM views and original sampler states. The selected
// source programs read cb0[20].x in PS and no material constants; stored tint,
// scaleRGB1 and other unbound material values are not applied.

fn native_world_color_multiply_fragment(
    input: NativeWorldColorLayerVaryings,
    scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = sample_material(scene.texture_mip_scale, normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_color_layer_normal(input, normal_sample.rg);
    let layer = sample_material(scene.texture_mip_scale, layer_color_map, layer_color_sampler, input.blend_uv.zw);
    let modulation = fma(vec3<f32>(input.blend_uv.y), layer.rgb - vec3<f32>(1.0), vec3<f32>(1.0));
    // PS254af097 modifies both color and gloss. Layer alpha is never read.
    let gloss_scale = dot(modulation, vec3<f32>(0.30000001192092896, 0.6000000238418579, 0.10000000149011612));
    let base = sample_material(scene.texture_mip_scale, color_map, color_sampler, input.uv_lightmap.xy);
    let color_gamma = modulation * base.rgb;
    let gloss = gloss_scale * base.a;
    let lighting_input = native_world_color_layer_lighting_input(input);
    let probe = native_world_layer_probe(lighting_input, normal, gloss, reflection_probe, reflection_sampler);
    let strength = fma(gloss, min(probe.fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    return native_world_layer_lighting(lighting_input, scene, normal, color_gamma,
        strength * probe.linear_color, lightmap_secondary, lightmap_sampler);
}

fn native_world_color_blend_fragment(
    input: NativeWorldColorLayerVaryings,
    scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = sample_material(scene.texture_mip_scale, normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_color_layer_normal(input, normal_sample.rg);
    let base = sample_material(scene.texture_mip_scale, color_map, color_sampler, input.uv_lightmap.xy);
    let gloss = base.a;
    let lighting_input = native_world_color_layer_lighting_input(input);
    let probe = native_world_layer_probe(lighting_input, normal, gloss, reflection_probe, reflection_sampler);
    let strength = fma(gloss, min(probe.fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    let layer = sample_material(scene.texture_mip_scale, layer_color_map, layer_color_sampler, input.blend_uv.zw);
    // PS9f70e132: alpha blend authored colors before their square transfer;
    // the base gloss and base normal remain unchanged by that blend.
    let weight = layer.a * input.blend_uv.y;
    let color_gamma = fma(vec3<f32>(weight), layer.rgb - base.rgb, base.rgb);
    return native_world_layer_lighting(lighting_input, scene, normal, color_gamma,
        strength * probe.linear_color, lightmap_secondary, lightmap_sampler);
}

fn native_world_color_add_fragment(
    input: NativeWorldColorLayerVaryings,
    scene: NativeWorldScene,
    color_map: texture_2d<f32>, color_sampler: sampler,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer_color_map: texture_2d<f32>, layer_color_sampler: sampler,
    lightmap_secondary: texture_2d<f32>, lightmap_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let normal_sample = sample_material(scene.texture_mip_scale, normal_map, normal_sampler, input.uv_lightmap.xy);
    let normal = native_world_color_layer_normal(input, normal_sample.rg);
    let base = sample_material(scene.texture_mip_scale, color_map, color_sampler, input.uv_lightmap.xy);
    let gloss = base.a;
    let lighting_input = native_world_color_layer_lighting_input(input);
    let probe = native_world_layer_probe(lighting_input, normal, gloss, reflection_probe, reflection_sampler);
    let strength = fma(gloss, min(probe.fresnel, gloss * 0.4749999940395355),
        fma(gloss, 0.027499999850988388, 0.014999999664723873));
    let layer = sample_material(scene.texture_mip_scale, layer_color_map, layer_color_sampler, input.blend_uv.zw);
    // PSb50e886e multiplies layer RGB by G first, then applies layer alpha in
    // the add. Preserve that sequence and the original saturated alpha write.
    let scaled_layer = layer.rgb * input.blend_uv.y;
    let color_gamma = fma(scaled_layer, vec3<f32>(layer.a), base.rgb);
    let alpha = clamp(fma(layer.a, input.blend_uv.y, 1.0), 0.0, 1.0);
    let result = native_world_layer_lighting(lighting_input, scene, normal, color_gamma,
        strength * probe.linear_color, lightmap_secondary, lightmap_sampler);
    return vec4<f32>(result.rgb, alpha);
}
