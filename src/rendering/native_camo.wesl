import package::rendering::native_world::sample_material;
import package::rendering::native_model::{NativeModelScene};
import package::rendering::native_scripted_model::{
    NativeModelScriptedScene, NativeModelParallaxConstants, NativeScriptedVaryings,
    native_scripted_parallax_basis, native_scripted_three_layer_uvs,
    native_scripted_reflection_finish,
};

// Original material-specific weaponParam6 override, code0xb9 / cb1[37].
// 0052b7c0 delivers record.xyz and record.w + (preserveNormal ? 10 : 0).
struct NativeModelCamoParameters {
    weapon_param6: vec4<f32>,
}

// mc_sw4_3d_weapon_camo_3layer_w1w78372, LITaa15e2ab, PSbd6d3b56:
// 380c1ad22ebe16ff201fe6934c1538ff5895f8806dd0093efc728ae8d48c06ba.
// Canonical camo-3layer-20261013/{effective-bindings,channel-anchors}.json.
// All three view-dependent layers are composed in the original single draw.
fn native_model_camo_fragment(
    input: NativeScriptedVaryings,
    scene: NativeModelScene, view: NativeModelScriptedScene,
    material: NativeModelParallaxConstants, camo: NativeModelCamoParameters,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer1_map: texture_2d<f32>, layer1_sampler: sampler,
    layer2_map: texture_2d<f32>, layer2_sampler: sampler,
    layer3_map: texture_2d<f32>, layer3_sampler: sampler,
    weapon_normal_map: texture_2d<f32>, weapon_normal_sampler: sampler,
    reflection_probe: texture_cube<f32>, reflection_sampler: sampler,
) -> vec4<f32> {
    let basis = native_scripted_parallax_basis(input);
    let c = material.source;
    let p = camo.weapon_param6;
    let uv = native_scripted_three_layer_uvs(input.uv, basis.ray, material, view.game_time.w);
    let layer3 = sample_material(scene.world.texture_mip_scale, layer3_map, layer3_sampler, uv.layer3 * p.xy);
    let layer2 = sample_material(scene.world.texture_mip_scale, layer2_map, layer2_sampler, uv.layer2 * p.xy);
    let layer2_delta = fma(-layer3.rgb, vec3<f32>(layer3.a), layer2.rgb);
    let layer3_color = layer3.a * layer3.rgb;
    let combined2 = fma(vec3<f32>(layer2.a * c[4].w), layer2_delta, layer3_color);
    let layer1 = sample_material(scene.world.texture_mip_scale, layer1_map, layer1_sampler, uv.layer1 * p.xy);
    let composed = fma(vec3<f32>(layer1.a * c[4].z), layer1.rgb - combined2, combined2);
    let gamma_emission = (composed * input.color.rgb) * c[9].z;
    let emission = gamma_emission * gamma_emission;
    let normal_uv = (input.uv * p.xy) * c[10].xy;
    let normal_sample = sample_material(scene.world.texture_mip_scale, normal_map, normal_sampler, normal_uv);
    let camo_slope = fma(normal_sample.rg, vec2<f32>(4.015748023986816),
        vec2<f32>(-2.0157480239868164));
    let weapon_sample = sample_material(scene.world.texture_mip_scale, weapon_normal_map, weapon_normal_sampler, input.uv);
    let weapon_slope = fma(weapon_sample.rg, vec2<f32>(4.015748023986816),
        vec2<f32>(-2.0157480239868164)) * select(0.0, 1.0, p.w >= 10.0);
    let camo_weight = p.w + select(-0.0, -10.0, p.w >= 4.0);
    let slope = fma(vec2<f32>(camo_weight), camo_slope, weapon_slope) * c[10].z;
    let mapped = fma(vec3<f32>(slope.y), basis.bitangent,
        fma(vec3<f32>(slope.x), input.tangent_sign.xyz, basis.normal));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    return native_scripted_reflection_finish(input, scene, normal, emission, c[9].y, c[9].w,
        reflection_probe, reflection_sampler);
}
