use super::atmosphere::{AtmosphereState, SourceAtmosphere};
use bevy::{
    asset::{LoadState, RenderAssetUsages},
    core_pipeline::tonemapping::TonemappingLuts,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const SIZE: u32 = 48;
const TRANSITION_STEPS: usize = 32;

#[derive(Resource)]
pub(super) struct SourceGrade {
    image: Handle<Image>,
    variants: Vec<Handle<Image>>,
    selected: Option<usize>,
}

impl SourceGrade {
    pub(super) fn ready(&self) -> bool {
        !self.variants.is_empty()
    }
}

pub(super) fn setup(mut commands: Commands, server: Res<AssetServer>) {
    commands.insert_resource(SourceGrade {
        image: server
            .load_builder()
            .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| {
                settings.is_srgb = false
            })
            .load("sky/zm_tomb_grade.ktx2"),
        variants: Vec::new(),
        selected: None,
    });
}

pub(super) fn prepare(
    mut grade: ResMut<SourceGrade>,
    source: Res<SourceAtmosphere>,
    state: Res<AtmosphereState>,
    mut images: ResMut<Assets<Image>>,
    mut luts: ResMut<TonemappingLuts>,
    server: Res<AssetServer>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    if grade.variants.is_empty() {
        if matches!(server.load_state(grade.image.id()), LoadState::Failed(_)) {
            failure.0 = "Original Origins color lookup failed to load".into();
            next.set(crate::GameState::LoadError);
            return;
        }
        let Some(original) = images.get(&grade.image) else {
            return;
        };
        let size = original.texture_descriptor.size;
        if size.width != 1024
            || size.height != 64
            || original.texture_descriptor.format != TextureFormat::Rgba8Unorm
        {
            failure.0 = "Original Origins color lookup has an invalid format".into();
            next.set(crate::GameState::LoadError);
            return;
        }
        let pixels = original
            .data
            .as_ref()
            .expect("CPU-readable original color lookup")
            .clone();
        let mut template = Image::new_fill(
            Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: SIZE,
            },
            TextureDimension::D3,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::default(),
        );
        template.sampler = ImageSampler::linear();
        for step in 0..=TRANSITION_STEPS {
            let blend = step as f32 / TRANSITION_STEPS as f32;
            let mut lut = template.clone();
            lut.texture_descriptor.label = Some("Origins source color response");
            for z in 0..SIZE {
                for y in 0..SIZE {
                    for x in 0..SIZE {
                        let domain = Vec3::new(x as f32, y as f32, z as f32) / (SIZE - 1) as f32;
                        let stimulus = domain / (Vec3::ONE - domain).max(Vec3::splat(0.000001));
                        let result = response(stimulus, &pixels, &source, blend);
                        lut.set_color_at_3d(x, y, z, Color::srgb(result.x, result.y, result.z))
                            .expect("Native display LUT accepts linear color texels");
                    }
                }
            }
            grade.variants.push(images.add(lut));
        }
    }
    let selected = (state.vision_blend * TRANSITION_STEPS as f32).round() as usize;
    if grade.selected != Some(selected) {
        luts.tony_mc_mapface = grade.variants[selected].clone();
        grade.selected = Some(selected);
    }
}

fn response(stimulus: Vec3, pixels: &[u8], source: &SourceAtmosphere, blend: f32) -> Vec3 {
    let softclip = source.softclip.max(0.0009765625);
    let pivot = 1. - softclip;
    let clipped = stimulus.map(|value| {
        if value <= pivot {
            value.max(0.)
        } else {
            1. - softclip * ((pivot - value) / softclip).exp()
        }
    });
    let encoded = clipped.map(f32::sqrt);
    let base = sample(pixels, encoded);
    let linear = base * base;
    let clear = &source.vision["clear"];
    let rain = &source.vision["_rain"];
    let saturation =
        Vec4::from_array(clear["vc_FSM"]).lerp(Vec4::from_array(rain["vc_FSM"]), blend);
    let amount = Vec4::from_array(clear["vc_FGM"])
        .lerp(Vec4::from_array(rain["vc_FGM"]), blend)
        .truncate();
    let luma = linear.dot(saturation.truncate());
    let saturated = Vec3::splat(luma) + (linear - Vec3::splat(luma)) * saturation.w;
    (linear + amount * (saturated - linear))
        .clamp(Vec3::ZERO, Vec3::ONE)
        .map(f32::sqrt)
}

fn sample(pixels: &[u8], rgb: Vec3) -> Vec3 {
    let coordinate = rgb.clamp(Vec3::ZERO, Vec3::ONE) * 31.;
    let low = coordinate.floor().as_uvec3();
    let high = (low + UVec3::ONE).min(UVec3::splat(31));
    let fraction = coordinate - low.as_vec3();
    let texel = |x: u32, y: u32, z: u32| {
        let offset = ((y * 1024 + z * 32 + x) * 4) as usize;
        Vec3::new(
            pixels[offset] as f32,
            pixels[offset + 1] as f32,
            pixels[offset + 2] as f32,
        ) / 255.
    };
    let plane = |z| {
        texel(low.x, low.y, z)
            .lerp(texel(high.x, low.y, z), fraction.x)
            .lerp(
                texel(low.x, high.y, z).lerp(texel(high.x, high.y, z), fraction.x),
                fraction.y,
            )
    };
    plane(low.z).lerp(plane(high.z), fraction.z)
}
