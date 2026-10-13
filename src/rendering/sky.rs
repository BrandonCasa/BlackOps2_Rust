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
    source: Option<Res<super::atmosphere::SourceAtmosphere>>,
) {
    let Some(source) = source else { return };
    let Ok(camera) = cameras.single() else { return };
    let image = server
        .load_builder()
        .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| settings.is_srgb = false)
        .load(source.sky_uri.clone());
    let rotation = source.sky_rotation();
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
