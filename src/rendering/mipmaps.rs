//! Reconstruct filtered levels from retained top mips; Bevy owns GPU sampling.
//! Packed atlases and reflection cubes deliberately use their own filtering.
use super::opaque::SurfaceLibrary;
use bevy::{
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::TextureFormat,
};
use std::collections::HashSet;

pub(super) fn prepare(
    library: Option<Res<SurfaceLibrary>>,
    materials: Res<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut processed: Local<HashSet<AssetId<Image>>>,
) {
    let Some(library) = library else { return };
    let mut handles = Vec::new();
    for definition in library.definitions.values() {
        handles.extend(definition.channels.values().cloned());
    }
    for (_, material) in materials.iter() {
        handles.extend(
            [
                material.base_color_texture.clone(),
                material.normal_map_texture.clone(),
                material.metallic_roughness_texture.clone(),
                material.emissive_texture.clone(),
            ]
            .into_iter()
            .flatten(),
        );
    }
    // Lookup tables filter sRGB in linear light without per-pixel pow.
    let linear: [f32; 256] = std::array::from_fn(|i| {
        let x = i as f32 / 255.;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    });
    let encoded: [u8; 4096] = std::array::from_fn(|i| {
        let x = i as f32 / 4095.;
        let y = if x <= 0.0031308 {
            x * 12.92
        } else {
            1.055 * x.powf(1. / 2.4) - 0.055
        };
        (y * 255.).round() as u8
    });
    for handle in handles {
        if processed.contains(&handle.id()) {
            continue;
        }
        let Some(mut image) = images.get_mut(&handle) else {
            continue;
        };
        let size = image.texture_descriptor.size;
        let format = image.texture_descriptor.format;
        if size.depth_or_array_layers != 1
            || !matches!(
                format,
                TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
            )
        {
            processed.insert(handle.id());
            continue;
        }
        let Some(data) = image.data.as_mut() else {
            continue;
        };
        let (mut w, mut h) = (size.width as usize, size.height as usize);
        if data.len() != w * h * 4 {
            processed.insert(handle.id());
            continue;
        }
        let srgb = format == TextureFormat::Rgba8UnormSrgb;
        let mut offset = 0;
        let mut levels = 1;
        while w > 1 || h > 1 {
            let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
            let mut next = vec![0; nw * nh * 4];
            for y in 0..nh {
                for x in 0..nw {
                    for channel in 0..4 {
                        let mut total = 0.;
                        for dy in 0..2 {
                            for dx in 0..2 {
                                let v = data[offset
                                    + (((2 * y + dy).min(h - 1) * w + (2 * x + dx).min(w - 1))
                                        * 4)
                                    + channel];
                                total += if srgb && channel < 3 {
                                    linear[v as usize]
                                } else {
                                    v as f32 / 255.
                                };
                            }
                        }
                        next[(y * nw + x) * 4 + channel] = if srgb && channel < 3 {
                            encoded[(total * 0.25 * 4095.).round().clamp(0., 4095.) as usize]
                        } else {
                            (total * 0.25 * 255.).round() as u8
                        };
                    }
                }
            }
            offset = data.len();
            data.extend(next);
            (w, h) = (nw, nh);
            levels += 1;
        }
        image.texture_descriptor.mip_level_count = levels;
        let mut sampler = match &image.sampler {
            ImageSampler::Descriptor(s) => s.clone(),
            ImageSampler::Default => ImageSamplerDescriptor::linear(),
        };
        sampler.mag_filter = ImageFilterMode::Linear;
        sampler.min_filter = ImageFilterMode::Linear;
        sampler.mipmap_filter = ImageFilterMode::Linear;
        sampler.anisotropy_clamp = 8;
        image.sampler = ImageSampler::Descriptor(sampler);
        processed.insert(handle.id());
    }
}
