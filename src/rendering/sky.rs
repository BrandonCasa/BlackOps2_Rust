//! A practical storm sky, generated into a standard Bevy Skybox cube.
//! No captured image or additional rendering pipeline is involved.
use super::{GameCamera, profile::VisualProfile};
use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::Skybox,
    image::{ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
    },
};

fn noise(p: Vec3) -> f32 {
    let cell = p.floor();
    let f = p - cell;
    let u = f * f * (Vec3::splat(3.) - 2. * f);
    let hash = |x: f32, y: f32, z: f32| {
        let n = (cell.x + x) * 127.1 + (cell.y + y) * 311.7 + (cell.z + z) * 74.7;
        (n.sin() * 43758.547).fract().abs()
    };
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    mix(
        mix(
            mix(hash(0., 0., 0.), hash(1., 0., 0.), u.x),
            mix(hash(0., 1., 0.), hash(1., 1., 0.), u.x),
            u.y,
        ),
        mix(
            mix(hash(0., 0., 1.), hash(1., 0., 1.), u.x),
            mix(hash(0., 1., 1.), hash(1., 1., 1.), u.x),
            u.y,
        ),
        u.z,
    )
}

pub(super) fn setup(
    mut commands: Commands,
    cameras: Query<Entity, With<GameCamera>>,
    mut images: ResMut<Assets<Image>>,
    profile: Res<VisualProfile>,
) {
    let Ok(camera) = cameras.single() else { return };
    const SIZE: u32 = 128;
    let mut image = Image::new_fill(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        &[0; 8],
        TextureFormat::Rgba16Float,
        RenderAssetUsages::default(),
    );
    let sun = Vec3::from_array(profile.sky_sun_direction).normalize();
    for face in 0..6 {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let u = (x as f32 + 0.5) / SIZE as f32 * 2. - 1.;
                let v = (y as f32 + 0.5) / SIZE as f32 * 2. - 1.;
                let d = match face {
                    0 => Vec3::new(1., -v, -u),
                    1 => Vec3::new(-1., -v, u),
                    2 => Vec3::new(u, 1., v),
                    3 => Vec3::new(u, -1., -v),
                    4 => Vec3::new(u, -v, 1.),
                    _ => Vec3::new(-u, -v, -1.),
                }
                .normalize();
                let mut clouds = 0.;
                let mut frequency = 3.;
                let mut amplitude = 0.55;
                for _ in 0..5 {
                    clouds += noise(d * frequency + Vec3::splat(12.)) * amplitude;
                    frequency *= 2.;
                    amplitude *= 0.5;
                }
                let horizon = (1. - d.y.max(0.)).powf(4.);
                let cloud = ((clouds - 0.3) * 2.2).clamp(0., 1.);
                let clear = Vec3::from_array(profile.sky_upper)
                    .lerp(Vec3::from_array(profile.sky_horizon), horizon);
                let detail = ((clouds - 0.32) * 3.5).clamp(0., 1.);
                let detail = detail * detail * (3. - 2. * detail);
                let overcast = Vec3::from_array(profile.cloud_dark)
                    .lerp(Vec3::from_array(profile.cloud_bright), detail);
                let glow = d.dot(sun).max(0.).powf(48.) * (1. - cloud * 0.7);
                let disk = ((d.dot(sun) - 0.9985) / 0.0015).clamp(0., 1.) * (1. - cloud * 0.8);
                let color = (clear.lerp(overcast, cloud)
                    + Vec3::new(0.8, 0.85, 0.9) * glow
                    + Vec3::splat(disk * 2.))
                    * profile.sky_nits;
                image
                    .set_color_at_3d(x, y, face, Color::linear_rgb(color.x, color.y, color.z))
                    .expect("Sky cube pixel");
            }
        }
    }
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    commands.entity(camera).insert(Skybox {
        image: Some(images.add(image)),
        brightness: 1.,
        rotation: Quat::IDENTITY,
    });
}
