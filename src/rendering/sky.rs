use super::{GameCamera, profile::VisualProfile};
use bevy::{
    light::{GeneratedEnvironmentMapLight, Skybox},
    pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel},
    prelude::*,
};

pub(super) fn setup(
    mut commands: Commands,
    cameras: Query<Entity, With<GameCamera>>,
    server: Res<AssetServer>,
    profile: Res<VisualProfile>,
) {
    let Ok(camera) = cameras.single() else { return };
    let image = server
        .load_builder()
        .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| settings.is_srgb = false)
        .load("sky/zm_tomb.ktx2");
    let (sin, cos) = 48_f32.to_radians().sin_cos();
    let rotation = Quat::from_mat3(&Mat3::from_cols(
        Vec3::new(cos, 0., sin),
        Vec3::new(sin, 0., -cos),
        Vec3::Y,
    ));
    let brightness = 1.0
        / bevy::camera::Exposure {
            ev100: profile.outdoor_ev100,
        }
        .exposure();
    commands.entity(camera).insert((
        Skybox {
            image: Some(image.clone()),
            brightness,
            rotation,
        },
        GeneratedEnvironmentMapLight {
            environment_map: image,
            intensity: brightness * profile.environment_intensity,
            rotation,
            ..default()
        },
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            ..default()
        },
        DistanceFog {
            color: profile.fog_color(),
            falloff: profile.fog_falloff(),
            directional_light_color: Color::NONE,
            ..default()
        },
    ));
}

pub(super) fn viewmodel_environment(
    mut commands: Commands,
    world: Query<(&bevy::light::EnvironmentMapLight, &bevy::camera::Exposure), With<GameCamera>>,
    mut cameras: Query<
        (
            Entity,
            Option<&bevy::light::EnvironmentMapLight>,
            &mut bevy::camera::Exposure,
        ),
        (
            With<crate::presentation::ViewModelCamera>,
            Without<GameCamera>,
        ),
    >,
) {
    let Ok((environment, exposure)) = world.single() else {
        return;
    };
    for (entity, current, mut view_exposure) in &mut cameras {
        if view_exposure.ev100 != exposure.ev100 {
            view_exposure.ev100 = exposure.ev100;
        }
        if current.is_none_or(|current| {
            current.specular_map != environment.specular_map
                || current.diffuse_map != environment.diffuse_map
                || current.intensity != environment.intensity
                || current.rotation != environment.rotation
                || current.affects_lightmapped_mesh_diffuse
                    != environment.affects_lightmapped_mesh_diffuse
        }) {
            commands.entity(entity).insert(environment.clone());
        }
    }
}
