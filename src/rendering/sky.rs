use super::{GameCamera, profile::VisualProfile};
use bevy::{
    light::{Atmosphere, AtmosphereEnvironmentMapLight, atmosphere::ScatteringMedium},
    pbr::{
        AtmosphereSettings, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
    },
    prelude::*,
};

pub(super) fn setup(
    mut commands: Commands,
    cameras: Query<Entity, With<GameCamera>>,
    mut media: ResMut<Assets<ScatteringMedium>>,
    profile: Res<VisualProfile>,
) {
    let Ok(camera) = cameras.single() else { return };
    let atmosphere = Atmosphere::earth(media.add(ScatteringMedium::earth(256, 256)));
    let center = GlobalTransform::from_translation(Vec3::Y * (-atmosphere.inner_radius - 64.));
    commands.spawn((atmosphere, center, Name::new("Origins atmosphere")));
    commands.entity(camera).insert((
        AtmosphereSettings::default(),
        AtmosphereEnvironmentMapLight {
            intensity: profile.environment_intensity,
            ..default()
        },
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
            ..default()
        },
        DistanceFog {
            color: super::profile::color(profile.fog_color),
            falloff: FogFalloff::Exponential {
                density: profile.fog_density,
            },
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
        if current.is_none_or(|current| current.specular_map != environment.specular_map) {
            commands.entity(entity).insert(environment.clone());
        }
    }
}
