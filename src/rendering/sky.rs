//! Physical Rayleigh/Mie sky and height fog through Bevy's atmosphere and volume passes.
use super::{GameCamera, profile::VisualProfile};
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageSampler, ImageSamplerDescriptor},
    light::{
        Atmosphere, AtmosphereEnvironmentMapLight, FogVolume, VolumetricFog,
        atmosphere::ScatteringMedium,
    },
    pbr::{
        AtmosphereSettings, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel,
    },
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

pub(super) fn setup(
    mut commands: Commands,
    cameras: Query<Entity, With<GameCamera>>,
    mut media: ResMut<Assets<ScatteringMedium>>,
    mut images: ResMut<Assets<Image>>,
    profile: Res<VisualProfile>,
) {
    let Ok(camera) = cameras.single() else { return };
    let atmosphere = Atmosphere::earth(media.add(ScatteringMedium::earth(256, 256)));
    // Native Origins geometry extends to -58.84 m. Put the planet surface
    // below it: negative level coordinates must not put the camera underground.
    let center = GlobalTransform::from_translation(Vec3::Y * (-atmosphere.inner_radius - 64.));
    commands.spawn((atmosphere, center, Name::new("Rayleigh and Mie atmosphere")));
    commands.entity(camera).insert((
        AtmosphereSettings::default(),
        AtmosphereEnvironmentMapLight::default(),
        ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Ultra,
            ..default()
        },
        VolumetricFog {
            ambient_color: super::profile::color(profile.fog_color),
            ambient_intensity: 0.15,
            step_count: 64,
            jitter: 0.,
        },
    ));
    // One smoothly sampled volume implements rho(y)=exp(-k*(y-base)).
    // The engine evaluates it against current-frame directional shadows.
    const BOTTOM: f32 = -32.;
    const TOP: f32 = 96.;
    const HEIGHT: u32 = 256;
    let mut density = Image::new_fill(
        Extent3d {
            width: 1,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D3,
        &[0; 2],
        TextureFormat::R16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    for index in 0..HEIGHT {
        let y = BOTTOM + (index as f32 + 0.5) / HEIGHT as f32 * (TOP - BOTTOM);
        let value = (-profile.fog_height_falloff * (y - profile.fog_base_height))
            .exp()
            .min(8.);
        density
            .set_color_at_3d(0, index, 0, Color::linear_rgb(value, value, value))
            .expect("Height fog density has a valid float format and coordinates");
    }
    density.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    commands.spawn((
        FogVolume {
            density_texture: Some(images.add(density)),
            density_factor: profile.fog_density,
            fog_color: super::profile::color(profile.fog_color),
            absorption: 0.02,
            scattering: 0.98,
            scattering_asymmetry: 0.65,
            ..default()
        },
        Transform::from_xyz(0., (TOP + BOTTOM) * 0.5, 0.).with_scale(Vec3::new(
            1600.,
            TOP - BOTTOM,
            1600.,
        )),
        Name::new("Exponential height fog"),
    ));
}

pub(super) fn viewmodel_environment(
    mut commands: Commands,
    world: Query<(&bevy::light::EnvironmentMapLight, &bevy::camera::Exposure), With<GameCamera>>,
    local: Query<&bevy::light::EnvironmentMapLight, With<super::static_gi::LocalReflection>>,
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
    let environment = local.single().unwrap_or(environment);
    for (entity, current, mut view_exposure) in &mut cameras {
        if view_exposure.ev100 != exposure.ev100 {
            view_exposure.ev100 = exposure.ev100;
        }
        if current.is_none_or(|current| current.specular_map != environment.specular_map) {
            commands.entity(entity).insert(environment.clone());
        }
    }
}
