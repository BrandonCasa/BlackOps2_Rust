use super::atmosphere::{AtmosphereState, SourceAtmosphere};
use bevy::{
    asset::{LoadState, RenderAssetUsages},
    core_pipeline::tonemapping::TonemappingLuts,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use std::sync::Arc;

const SIZE: u32 = 48;

#[derive(Resource)]
pub(super) struct SourceGrade {
    image: Handle<Image>,
    pixels: Option<Arc<[u8]>>,
    lut: Option<Handle<Image>>,
    selected: Option<f32>,
}

impl SourceGrade {
    pub(super) fn ready(&self) -> bool {
        self.lut.is_some()
    }
}

pub(super) fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    source: Res<SourceAtmosphere>,
) {
    commands.insert_resource(SourceGrade {
        image: server
            .load_builder()
            .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| {
                settings.is_srgb = false
            })
            .load(source.grade_uri.clone()),
        pixels: None,
        lut: None,
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
    let blend = state.vision_blend;
    if grade.selected == Some(blend) {
        return;
    }
    if grade.pixels.is_none() {
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
        grade.pixels = Some(
            original
                .data
                .as_ref()
                .expect("CPU-readable original color lookup")
                .clone()
                .into(),
        );
    }
    let pixels = grade
        .pixels
        .as_ref()
        .expect("Retained original color lookup")
        .clone();
    let mut lut = Image::new_fill(
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
    lut.texture_descriptor.label = Some("Origins source color response");
    lut.sampler = ImageSampler::linear();
    let film = Film::new(&source, blend);
    for z in 0..SIZE {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let encoded = Vec3::new(x as f32, y as f32, z as f32) / (SIZE - 1) as f32;
                let stimulus = encoded / (Vec3::ONE - encoded).max(Vec3::splat(1e-6));
                let native = stimulus.sqrt();
                let softclip = source.softclip.max(1. / 1024.);
                let pivot = 1. - softclip;
                let native = native
                    .map(|v| {
                        if v > pivot {
                            1. - softclip * ((pivot - v) / softclip).exp()
                        } else {
                            v
                        }
                    })
                    .clamp(Vec3::ZERO, Vec3::ONE)
                    * 31.;
                let lo = native.floor().as_uvec3();
                let hi = (lo + UVec3::ONE).min(UVec3::splat(31));
                let fraction = native - lo.as_vec3();
                let mut display = Vec3::ZERO;
                for corner in 0..8 {
                    let p = UVec3::new(
                        if corner & 1 == 0 { lo.x } else { hi.x },
                        if corner & 2 == 0 { lo.y } else { hi.y },
                        if corner & 4 == 0 { lo.z } else { hi.z },
                    );
                    let weight = (if corner & 1 == 0 {
                        1. - fraction.x
                    } else {
                        fraction.x
                    }) * (if corner & 2 == 0 {
                        1. - fraction.y
                    } else {
                        fraction.y
                    }) * (if corner & 4 == 0 {
                        1. - fraction.z
                    } else {
                        fraction.z
                    });
                    let offset = ((p.y * 1024 + p.z * 32 + p.x) * 4) as usize;
                    let base = Vec3::new(
                        pixels[offset] as f32,
                        pixels[offset + 1] as f32,
                        pixels[offset + 2] as f32,
                    ) / 255.;
                    display += film.apply(base) * weight;
                }
                let linear = display.map(|v| {
                    if v > 0.04045 {
                        ((v + 0.055) / 1.055).powf(2.4)
                    } else {
                        v / 12.92
                    }
                });
                lut.set_color_at_3d(x, y, z, Color::linear_rgb(linear.x, linear.y, linear.z))
                    .expect("Native display LUT texel");
            }
        }
    }
    if let Some(handle) = &grade.lut {
        images
            .insert(handle.id(), lut)
            .expect("Origins color LUT handle");
    } else {
        let handle = images.add(lut);
        luts.tony_mc_mapface = handle.clone();
        grade.lut = Some(handle);
    }
    grade.selected = Some(blend);
}

struct Film {
    range_start: Vec4,
    range_end: Vec4,
    matrices: [[Vec4; 3]; 3],
    gamma: Vec3,
    saturation: Vec4,
    amount: Vec3,
}

impl Film {
    fn new(source: &SourceAtmosphere, blend: f32) -> Self {
        let value = |key: &str| {
            Vec4::from_array(source.vision["clear"][key])
                .lerp(Vec4::from_array(source.vision["_rain"][key]), blend)
        };
        let mut start = value("vc_RS");
        let mut end = value("vc_RE");
        let epsilon = 1. / 4096.;
        if start.x >= end.x {
            end.x = start.x + epsilon;
        }
        if start.y >= end.y {
            start.y = end.y - epsilon;
        }
        if start.z >= end.z {
            start.z = end.z - epsilon;
        }
        if end.z >= start.w {
            start.w = end.z + epsilon;
        }
        if start.w >= end.w {
            end.w = start.w + epsilon;
        }
        let mut saturation = value("vc_FSM");
        let sum = saturation.x + saturation.y + saturation.z;
        saturation.x /= sum;
        saturation.y /= sum;
        saturation.z /= sum;
        Self {
            range_start: start,
            range_end: end,
            matrices: [
                [value("vc_SMR"), value("vc_MMR"), value("vc_HMR")],
                [value("vc_SMG"), value("vc_MMG"), value("vc_HMG")],
                [value("vc_SMB"), value("vc_MMB"), value("vc_HMB")],
            ],
            gamma: value("vc_FGM").truncate(),
            saturation,
            amount: value("vc_FBM").truncate(),
        }
    }

    fn apply(&self, base: Vec3) -> Vec3 {
        let y = base.dot(Vec3::new(0.212585, 0.715195, 0.072220));
        let a = self.range_start;
        let b = self.range_end;
        let ramp = |v: f32| v.clamp(0., 1.);
        let mut weights = Vec3::new(
            ramp((y - b.x) / (a.x - b.x)),
            ramp((y - a.z) / (b.z - a.z)) * ramp((y - b.w) / (a.w - b.w)),
            ramp((y - a.y) / (b.y - a.y)),
        );
        weights = weights * weights * (Vec3::splat(3.) - 2. * weights);
        weights /= weights.element_sum();
        let linear = base * base;
        let rgba = linear.extend(1.);
        let mapped = Vec3::from_array(std::array::from_fn(|channel| {
            let rows = self.matrices[channel];
            Vec3::new(rows[0].dot(rgba), rows[1].dot(rgba), rows[2].dot(rgba))
                .dot(weights)
                .clamp(0., 1.)
        }));
        let gamma = Vec3::new(
            mapped.x.powf(self.gamma.x),
            mapped.y.powf(self.gamma.y),
            mapped.z.powf(self.gamma.z),
        );
        let luma = Vec3::splat(gamma.dot(self.saturation.truncate()));
        let saturated = luma + (gamma - luma) * self.saturation.w;
        (linear + self.amount * (saturated - linear))
            .clamp(Vec3::ZERO, Vec3::ONE)
            .map(f32::sqrt)
    }
}

#[cfg(feature = "dev-tools")]
pub(super) fn snapshot(world: &World) -> serde_json::Value {
    let grade = world.resource::<SourceGrade>();
    let source = world.resource::<SourceAtmosphere>();
    let film = Film::new(source, grade.selected.unwrap_or(0.));
    let samples = [
        Vec3::ZERO,
        Vec3::splat(0.18),
        Vec3::splat(0.5),
        Vec3::ONE,
        Vec3::new(0.8, 0.4, 0.2),
        Vec3::new(0.2, 0.4, 0.8),
    ];
    serde_json::json!({"ready": grade.ready(), "size": SIZE, "vision_blend": grade.selected,
    "softclip": source.softclip, "samples": samples.map(|base| serde_json::json!({
        "base": base.to_array(), "film": film.apply(base).to_array()
    }))})
}
