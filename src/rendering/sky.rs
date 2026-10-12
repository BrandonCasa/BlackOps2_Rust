use super::{GameCamera, profile::VisualProfile};
use bevy::{
    light::{GeneratedEnvironmentMapLight, Skybox},
    prelude::*,
};

pub(super) fn setup(
    mut commands: Commands,
    cameras: Query<Entity, With<GameCamera>>,
    server: Res<AssetServer>,
    profile: Res<VisualProfile>,
    source: Res<super::atmosphere::SourceAtmosphere>,
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
    let brightness = 1.0 / source.camera_exposure(&profile).exposure();
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
        source.fog.native(source.exposure),
    ));
}
