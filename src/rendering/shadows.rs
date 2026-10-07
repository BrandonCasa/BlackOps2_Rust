//! Correct directional PCSS in Bevy's existing shadow-map sampling path.
use bevy::{
    prelude::*,
    shader::{Shader, ShaderImport},
};

/// Retain Bevy's snapped XY fit and its native maps/frusta, but include
/// offscreen sun casters in a fixed light-space depth interval. A camera-fitted
/// depth interval clips approaching casters differently as the view rotates.
/// The 4 km slab covers the Origins runtime closure and moving actor bounds;
/// reverse-Z float depth preserves contact precision without a camera-dependent
/// near plane, depth scale or normal bias.
pub(super) fn stabilize(mut lights: Query<(&GlobalTransform, &mut bevy::light::Cascades)>) {
    const NEAR: f32 = 2048.;
    const DEPTH: f32 = 4096.;
    for (light, mut cascades) in &mut lights {
        let basis = Mat4::from_quat(light.rotation());
        let inverse = basis.transpose();
        for cascade in cascades.cascades.values_mut().flatten() {
            // Do not recover XY by transforming world_from_cascade back to
            // light space. Its camera-fitted Z participates in that round trip
            // and introduces tiny XY errors as the view rotates. Preserve the
            // engine's directly constructed, texel-snapped projection rows.
            let origin = Vec3::new(
                -cascade.clip_from_world.w_axis.x / cascade.clip_from_cascade.x_axis.x,
                -cascade.clip_from_world.w_axis.y / cascade.clip_from_cascade.y_axis.y,
                NEAR,
            );
            cascade.world_from_cascade.w_axis = basis * origin.extend(1.);
            cascade.clip_from_cascade.z_axis.z = DEPTH.recip();
            cascade.clip_from_cascade.w_axis.z = 1.;
            cascade.clip_from_world.x_axis.z = inverse.x_axis.z / DEPTH;
            cascade.clip_from_world.y_axis.z = inverse.y_axis.z / DEPTH;
            cascade.clip_from_world.z_axis.z = inverse.z_axis.z / DEPTH;
            cascade.clip_from_world.w_axis.z = 1. - NEAR / DEPTH;
        }
    }
}

pub(super) fn prepare(mut shaders: ResMut<Assets<Shader>>, mut done: Local<bool>) {
    if *done {
        return;
    }
    let native = |path: &str| {
        shaders.iter().find_map(|(id, shader)| {
            (shader.import_path == ShaderImport::Custom(path.into()))
                .then(|| (id, shader.source.as_str().to_owned()))
        })
    };
    let (Some((sampling_id, sampling)), Some((shadows_id, shadows))) = (
        native("bevy_pbr::shadow_sampling"),
        native("bevy_pbr::shadows"),
    ) else {
        return;
    };
    // Bevy 0.19.1's directional path applies a perspective depth ratio to
    // orthographic cascades. Their fitted near/far planes vary with camera
    // orientation, so that ratio changes a stationary blocker's penumbra.
    // Preserve native maps, comparisons and samples; use world distances and
    // the sun's angular radius. Keep the filter pattern in the light's plane.
    // The original function is also used by perspective spot lights.
    let corrected_sampling = sampling + "\n" + include_str!("sun_pcss.wgsl");
    let original = "            texel_size,\n            (*light).soft_shadow_size,\n        );";
    assert_eq!(
        shadows.matches(original).count(),
        1,
        "Pinned directional PCSS call"
    );
    let corrected_shadows = shadows
        .replacen("sample_shadow_map, sample_shadow_map_pcss,", "sample_shadow_map, sample_shadow_map_pcss, sample_directional_shadow_map_pcss, directional_receiver_gradient,", 1)
        .replacen("return sample_shadow_map_pcss(\n            light_local.xy,", "return sample_directional_shadow_map_pcss(\n            light_local.xy,", 1)
        .replacen(original,
        "            texel_size,\n            (*light).soft_shadow_size,\n            1.0 / length(vec3((*cascade).clip_from_world[0].z, (*cascade).clip_from_world[1].z, (*cascade).clip_from_world[2].z)),\n            (*light).cascades[(*light).num_cascades - 1u].far_bound,\n            vec2(dot(normalize(vec3((*cascade).clip_from_world[0].x, (*cascade).clip_from_world[1].x, (*cascade).clip_from_world[2].x)), frag_position.xyz), dot(normalize(vec3((*cascade).clip_from_world[0].y, (*cascade).clip_from_world[1].y, (*cascade).clip_from_world[2].y)), frag_position.xyz)),\n            directional_receiver_gradient((*cascade).clip_from_world, surface_normal),\n        );", 1);
    shaders
        .insert(
            sampling_id,
            Shader::from_wgsl(corrected_sampling, "origins::sun_sampling"),
        )
        .expect("Native shadow sampling handle");
    shaders
        .insert(
            shadows_id,
            Shader::from_wgsl(corrected_shadows, "origins::sun_shadows"),
        )
        .expect("Native shadow handle");
    *done = true;
}
