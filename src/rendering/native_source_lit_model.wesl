import package::rendering::native_model::{NativeModelScene, NativeModelVaryings};
import package::rendering::native_scripted_model::NativeModelScriptedScene;
import package::rendering::native_model_variants::native_model_atlas_irradiance;
import package::rendering::native_world_plain::native_world_plain_specular_reflection;

// Source PSfa618ca5, PSc654bb10, PS3a1bfa95 and PSc793a237 apply
// cb0[114..116] to complete linear lighting before the ordinary fog/encode.
fn native_source_lit_finish(input: NativeModelVaryings, scene: NativeModelScene,
    view: NativeModelScriptedScene, lighting: vec3<f32>, alpha: f32,
    matrix: bool, premultiplied: bool) -> vec4<f32> {
    var lit = lighting;
    var fog = input.fog_color;
    var opacity = 1.0;
    if premultiplied {
        lit = alpha * lit;
        fog = alpha * fog;
        opacity = sqrt(alpha);
    }
    if matrix {
        let value = vec4<f32>(lit, 1.0);
        lit = vec3<f32>(dot(value, view.hero_lighting[0]),
            dot(value, view.hero_lighting[1]), dot(value, view.hero_lighting[2]));
    }
    let fogged = fma(vec3<f32>(input.normal_fog.w), lit - fog, fog);
    return vec4<f32>(sqrt(fogged * scene.world.hdr_control0.x), opacity);
}

fn native_source_lit_lighting(input: NativeModelVaryings, scene: NativeModelScene,
    normal: vec3<f32>, diffuse: vec3<f32>, specular: vec4<f32>, occlusion: vec4<f32>,
    atlas: texture_3d<f32>, atlas_sampler: sampler,
    probe: texture_cube<f32>, probe_sampler: sampler) -> vec3<f32> {
    let reflected = native_world_plain_specular_reflection(input.eye_to_vertex.xyz,
        normal, specular, probe, probe_sampler);
    let reflection = (reflected * occlusion.y) * input.reflection_correction;
    let irradiance = native_model_atlas_irradiance(normal, input.grid_coords_and_vis.xyz,
        scene, atlas, atlas_sampler) * occlusion.z;
    return fma(irradiance * (diffuse * diffuse), vec3<f32>(32.0), reflection);
}
