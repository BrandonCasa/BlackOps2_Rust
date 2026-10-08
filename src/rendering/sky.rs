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
    shader::Shader,
};

// Atmosphere's aerial LUT assumes an unobstructed planetary medium. Applying
// it over indoor geometry adds unshadowed sky-colored haze on top of our
// shadowed height medium. Keep Bevy's physical sky and its environment input;
// finite scene rays have one participating medium, the native FogVolume.
pub(super) fn prepare(mut shaders: ResMut<Assets<Shader>>, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Some((id, source)) = shaders.iter().find_map(|(id, shader)| {
        let source = shader.source.as_str();
        source
            .contains("fn main(in: FullscreenVertexOutput) -> RenderSkyOutput")
            .then(|| (id, source.to_owned()))
    }) else {
        return;
    };
    let start = "        let t = ndc_to_camera_dist(vec3(uv_to_ndc(in.uv), depth));";
    let end = "    // exposure compensation";
    let (before, tail) = source
        .split_once(start)
        .expect("Pinned aerial perspective branch");
    let (_, after) = tail.split_once(end).expect("Pinned atmosphere exposure");
    let source = format!(
        "{before}        inscattering = vec3(0.0);\n        transmittance = vec3(1.0);\n    }}\n\n{end}{after}"
    );
    shaders
        .insert(
            id,
            Shader::from_wesl(source, "embedded://bevy_pbr/atmosphere/render_sky.wesl"),
        )
        .expect("Native sky shader");
    *done = true;
}

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
            // Constant sky scatter has no world visibility and fills enclosed
            // rooms with gray light. The sun's shadowed volume path supplies
            // actual illumination; the physical medium/height density remain.
            ambient_intensity: 0.0,
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
