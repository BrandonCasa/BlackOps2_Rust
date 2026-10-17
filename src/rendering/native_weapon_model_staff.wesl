import package::rendering::native_world::sample_material;
// Original staff LIT PS469eefdb. Charge is scriptVector1.y >= Charge_Level;
// source chooses the two layer3 textures before composing layer2/layer1.
import package::rendering::native_model::NativeModelScene;
import package::rendering::native_scripted_model::{NativeScriptedVaryings, NativeModelScriptedScene,
    NativeModelScriptInputs, NativeModelParallaxConstants, native_scripted_parallax_basis,
    native_scripted_three_layer_uvs, native_scripted_reflection_finish};
fn native_staff_fragment(input: NativeScriptedVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, scripts: NativeModelScriptInputs,
    material: NativeModelParallaxConstants, emission_on: f32,
    normal_map: texture_2d<f32>, normal_sampler: sampler,
    layer1_map: texture_2d<f32>, layer1_sampler: sampler,
    layer2_map: texture_2d<f32>, layer2_sampler: sampler,
    layer3_on: texture_2d<f32>, layer3_on_sampler: sampler,
    layer3_off: texture_2d<f32>, layer3_off_sampler: sampler,
    probe: texture_cube<f32>, probe_sampler: sampler) -> vec4<f32> {
    let basis = native_scripted_parallax_basis(input);
    let c = material.source;
    let uv = native_scripted_three_layer_uvs(input.uv, basis.ray, material, view.game_time.w);
    let on = sample_material(scene.world.texture_mip_scale, layer3_on, layer3_on_sampler, uv.layer3);
    let off = sample_material(scene.world.texture_mip_scale, layer3_off, layer3_off_sampler, uv.layer3);
    let charged = select(0.0, 1.0, scripts.vectors[1].y >= c[10].w);
    let layer3 = fma(vec4<f32>(charged), on - off, off);
    let layer2 = sample_material(scene.world.texture_mip_scale, layer2_map, layer2_sampler, uv.layer2);
    let combined2 = fma(vec3<f32>(layer2.a * c[4].w),
        fma(-layer3.rgb, vec3<f32>(layer3.a), layer2.rgb), layer3.a * layer3.rgb);
    let layer1 = sample_material(scene.world.texture_mip_scale, layer1_map, layer1_sampler, uv.layer1);
    let composed = fma(vec3<f32>(layer1.a * c[4].z), layer1.rgb - combined2, combined2);
    let amount = fma(charged, emission_on - c[9].z, c[9].z);
    let gamma_emission = amount * (composed * input.color.rgb);
    let normal_sample = sample_material(scene.world.texture_mip_scale, normal_map, normal_sampler, input.uv * c[10].xy);
    let slope = fma(normal_sample.rg, vec2<f32>(4.015748023986816), vec2<f32>(-2.0157480239868164)) * c[10].z;
    let mapped = fma(vec3<f32>(slope.y), basis.bitangent,
        fma(vec3<f32>(slope.x), input.tangent_sign.xyz, basis.normal));
    let normal = inverseSqrt(dot(mapped, mapped)) * mapped;
    return native_scripted_reflection_finish(input, scene, normal, gamma_emission * gamma_emission,
        c[9].y, c[9].w, probe, probe_sampler);
}
