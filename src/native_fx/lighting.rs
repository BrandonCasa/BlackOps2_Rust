//! Particle light sampling (0044d160/005c4890) and PACKED32 color (005b4da0).
use crate::rendering::{model_lighting::ModelLightingSource, native_water::PrimaryLightDefinition};
use bevy::prelude::*;

#[derive(Resource)]
pub(crate) struct Source {
    pub primary: Vec<PrimaryLightDefinition>,
}

impl Source {
    pub(super) fn sample(
        &self,
        grid: &ModelLightingSource,
        weather: &crate::environment::weather::Weather,
        position: Vec3,
    ) -> Vec3 {
        // Ordinary world FX creation supplies a 64-inch Z sample offset and
        // primary-light scale 1 (005c4e60/005c4f60 -> 005c4890).
        let origin = Vec3::new(position.x, -position.z, position.y) / 0.0254 + Vec3::Z * 64.;
        let sample = grid.grid.sample_particle(origin);
        let ambient = grid
            .coefficients
            .particle_ambient(&sample.coefficients[..sample.count]);
        let direct = match sample.primary_light {
            0 => Vec3::ZERO,
            1 | 255 => weather.sun_diffuse().truncate(),
            index => {
                let light = &self.primary[usize::from(index)];
                let delta = light.origin - origin;
                let distance = delta.length();
                let mut attenuation = (1. - (distance / light.radius).min(1.)).powi(2);
                if light.kind == 2 {
                    let cosine = light
                        .dir
                        .dot(delta / if distance > 0. { distance } else { 1. })
                        .max(0.);
                    attenuation *= ((cosine - light.cos_half_fov_outer)
                        / (light.cos_half_fov_inner - light.cos_half_fov_outer))
                        .clamp(0., 1.);
                }
                light.diffuse_color.truncate() * attenuation
            }
        };
        ambient + direct * sample.visibility
    }
}

pub(super) fn pack(color: [f32; 4], fraction: u8, lighting: Vec3, exposure: f32) -> [u8; 4] {
    let bytes = color.map(|v| v.clamp(0., 255.) as u8);
    if fraction == 0 {
        return [bytes[0] >> 1, bytes[1] >> 1, bytes[2] >> 1, bytes[3]];
    }
    let fraction = f32::from(fraction) * 0.003921569;
    let rgb = lighting.to_array();
    let mut packed = bytes;
    for i in 0..3 {
        let original = f32::from(bytes[i]) * 0.003921569;
        let linear = original * original * rgb[i] * exposure;
        // Retain the source 0 * rsqrt(0) NaN, which packs to zero.
        let lit = linear * linear.sqrt().recip();
        let blended = original + fraction * (lit - original);
        packed[i] = ((blended * 0.5).clamp(0., 1.) * 255. + 0.5) as u8;
    }
    // Native shuffle 00b8f580 selects the original alpha, not the lit lane.
    packed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_lighting_preserves_alpha_and_source_packing() {
        assert_eq!(
            pack([255., 128., 1., 77.], 0, Vec3::ZERO, 1.),
            [127, 64, 0, 77]
        );
        assert_eq!(
            pack([128., 64., 32., 77.], 255, Vec3::splat(4.), 1.),
            [128, 64, 32, 77]
        );
        assert_eq!(
            pack([128., 64., 32., 77.], 255, Vec3::splat(4.), 0.25),
            [64, 32, 16, 77]
        );
        assert_eq!(
            pack([255., 255., 255., 77.], 191, Vec3::ZERO, 1.),
            [0, 0, 0, 77]
        );
        assert_eq!(pack([255.; 4], 255, Vec3::splat(100.), 1.), [255; 4]);
    }
}
