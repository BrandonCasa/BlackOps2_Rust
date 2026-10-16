//! Original T6 static-model directional lighting and object SH constants.
//!
//! Native recovery: 00466b50 builds the 56 cube directions; 00449b70/00449ad0
//! decode and evaluate light-grid coefficients; 00449f70 encodes atlas texels.
//! 0044b840 reduces RGB SH, 00457430/00457530 quantize/decode object constants,
//! and 004576d0/004189a0 supply atlas coordinates and lookup scale.
use bevy::prelude::{Vec3, Vec4};
use std::sync::LazyLock;

// Original cube-boundary order and texel indices, shared by both consumers.
static LIGHTING_DIRECTIONS: LazyLock<[(usize, Vec3); 56]> = LazyLock::new(|| {
    let mut directions = [(0, Vec3::ZERO); 56];
    let mut at = 0;
    for z in 0..4 {
        for y in 0..4 {
            for x in 0..4 {
                if (1..3).contains(&x) && (1..3).contains(&y) && (1..3).contains(&z) {
                    continue;
                }
                let direction = Vec3::new(x as f32, y as f32, z as f32) * 0.6666667 - Vec3::ONE;
                directions[at] = (z * 16 + y * 4 + x, direction.normalize());
                at += 1;
            }
        }
    }
    directions
});

pub(crate) struct LightingCoefficients {
    coefficients: Vec<[u16; 27]>,
}

pub(crate) struct StaticLighting {
    /// One original 4x4x4 RGBA8 block, X varying fastest, then Y, then Z.
    pub texels: [u8; 256],
    /// Original gridLightingSH0/1/2 after the native ushort round trip.
    pub sh: [Vec4; 3],
}

impl LightingCoefficients {
    /// 0044d160 averages the 56 unclipped cube-boundary samples after each
    /// weighted SH record has been evaluated and clamped independently.
    pub(crate) fn particle_ambient(&self, samples: &[(u16, f32)]) -> Vec3 {
        let mut directions = [Vec3::ZERO; 56];
        for &(index, weight) in samples {
            let packed = &self.coefficients[usize::from(index)];
            let coefficients = std::array::from_fn(|i| {
                Vec3::new(
                    decode(packed[i * 3]),
                    decode(packed[i * 3 + 1]),
                    decode(packed[i * 3 + 2]),
                ) * weight
            });
            for (rgb, &(_, direction)) in directions.iter_mut().zip(LIGHTING_DIRECTIONS.iter()) {
                *rgb += evaluate(&coefficients, direction);
            }
        }
        directions
            .into_iter()
            .fold(Vec3::ZERO, |sum, rgb| sum + rgb)
            * 0.017857144
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        assert!(
            !bytes.is_empty() && bytes.len().is_multiple_of(54),
            "Origins requires complete 54-byte GfxCompressedLightGridCoeffs records"
        );
        let coefficients = bytes
            .chunks_exact(54)
            .map(|record| {
                std::array::from_fn(|i| u16::from_le_bytes([record[i * 2], record[i * 2 + 1]]))
            })
            .collect();
        Self { coefficients }
    }

    pub(crate) fn static_lighting(&self, colors_index: usize) -> StaticLighting {
        let mut lighting =
            self.grid_lighting(&[(u16::try_from(colors_index).expect("source colorsIndex"), 1.)]);
        // Static placements store ushort SH (00457430/00457530); dynamic
        // 00457600 copies the original float SH directly from its 52-byte slot.
        lighting.sh = lighting
            .sh
            .map(|row| Vec4::from_array(row.to_array().map(quantize_sh)));
        lighting
    }

    /// Base grid contribution, before the separate native hero-light stage.
    /// 0044baf0 weights each decoded record, evaluates/clamps its 56 directions,
    /// then adds those results. Clamping after blending SH changes the lighting.
    pub(crate) fn grid_lighting(&self, samples: &[(u16, f32)]) -> StaticLighting {
        assert!(!samples.is_empty() && samples.len() <= 8);
        let mut decoded = [[Vec3::ZERO; 9]; 8];
        for (output, &(index, weight)) in decoded.iter_mut().zip(samples) {
            assert!(weight.is_finite() && weight >= 0.);
            let packed = &self.coefficients[usize::from(index)];
            *output = std::array::from_fn(|i| {
                Vec3::new(
                    decode(packed[i * 3]),
                    decode(packed[i * 3 + 1]),
                    decode(packed[i * 3 + 2]),
                ) * weight
            });
        }
        let mut coefficients = decoded[0];
        for record in &decoded[1..samples.len()] {
            for (sum, value) in coefficients.iter_mut().zip(record) {
                *sum += *value;
            }
        }

        let mut texels = [0; 256];
        // The eight interior texels remain zero, as in native 00449f70.
        for &(index, direction) in LIGHTING_DIRECTIONS.iter() {
            let mut rgb = evaluate(&decoded[0], direction);
            for record in &decoded[1..samples.len()] {
                rgb += evaluate(record, direction);
            }
            let offset = index * 4;
            texels[offset] = encode_texel(rgb.x);
            texels[offset + 1] = encode_texel(rgb.y);
            texels[offset + 2] = encode_texel(rgb.z);
            texels[offset + 3] = 255;
        }

        // 0044b840 fits one chromaticity and nine luminance coefficients into
        // the three vec4 object constants. Hero lighting changes directional
        // atlas values separately; it does not enter this grid SH reduction.
        let ambient = luminance(coefficients[0]) + 0.0001;
        let chromaticity = coefficients[0] * ambient.recip();
        let quadratic_z = luminance(coefficients[7]);
        let sh = [
            Vec4::new(
                chromaticity.x,
                chromaticity.y,
                chromaticity.z,
                quadratic_z * 3.0,
            ),
            Vec4::new(
                luminance(coefficients[1]),
                luminance(coefficients[2]),
                luminance(coefficients[3]),
                ambient - quadratic_z,
            ),
            Vec4::new(
                luminance(coefficients[4]),
                luminance(coefficients[5]),
                luminance(coefficients[6]),
                luminance(coefficients[8]),
            ),
        ];

        StaticLighting { texels, sh }
    }
}

fn decode(value: u16) -> f32 {
    // Original SSE transfer: multiply by the stored float reciprocal of 65535,
    // then by 32, then subtract 16. Keep the operation order and f32 precision.
    value as f32 * 1.5259022e-5 * 32.0 - 16.0
}

fn luminance(rgb: Vec3) -> f32 {
    rgb.x * 0.25 + rgb.y * 0.5 + rgb.z * 0.25
}

fn evaluate(c: &[Vec3; 9], n: Vec3) -> Vec3 {
    // Native 00449ad0 accumulation order; C7 uses 3*z*z-1 here, whereas the
    // reduced object SH above folds that basis into its Z^2 and constant terms.
    (c[1] * n.x
        + c[0]
        + c[2] * n.y
        + c[3] * n.z
        + c[4] * (n.z * n.x)
        + c[5] * (n.z * n.y)
        + c[6] * (n.y * n.x)
        + c[7] * (n.z * n.z * 3.0 - 1.0)
        + c[8] * (n.x * n.x - n.y * n.y))
        .max(Vec3::ZERO)
}

fn encode_texel(value: f32) -> u8 {
    // Native RGB8 conversion truncates; it does not add the SH quantizer's 0.5.
    ((value.min(32.0) * 0.03125).sqrt() * 255.0) as u8
}

fn quantize_sh(value: f32) -> f32 {
    let packed = ((value.clamp(-16.0, 16.0) + 16.0) * 2047.9688 + 0.5) as u16;
    decode(packed)
}

/// `slot` is a zero-based atlas block, rather than the native one-based handle.
pub(crate) fn grid_coordinates(slot: u16, visibility: u8) -> Vec4 {
    assert!(
        slot < 8192,
        "Original model-lighting atlas slot is out of range"
    );
    Vec4::new(
        ((slot & 127) * 4 + 2) as f32 * 0.001953125,
        ((slot >> 7) * 4 + 2) as f32 * 0.00390625,
        0.5,
        visibility as f32 * 0.003921569,
    )
}

pub(crate) fn lighting_lookup_scale() -> Vec4 {
    // 004189a0:1.5/width, 1.5/height, 1.5/depth for the 512x256x4 atlas.
    Vec4::new(0.0029296875, 0.005859375, 0.375, 0.0)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_directions_clamp_before_accumulation() {
        // Opposing constant fields cancel in SH, but the negative directional
        // sample is clamped before the native atlas adds the positive sample.
        let mut positive = [32768; 27];
        let mut negative = [32768; 27];
        positive[..3].fill(49151);
        negative[..3].fill(16384);
        let source = LightingCoefficients {
            coefficients: vec![positive, negative],
        };
        let result = source.grid_lighting(&[(0, 0.5), (1, 0.5)]);
        let ambient = source.particle_ambient(&[(0, 0.5), (1, 0.5)]);
        assert!((ambient - Vec3::splat(4.)).abs().max_element() < 0.001);
        for (index, texel) in result.texels.chunks_exact(4).enumerate() {
            let (x, y, z) = (index % 4, (index / 4) % 4, index / 16);
            if (1..3).contains(&x) && (1..3).contains(&y) && (1..3).contains(&z) {
                assert_eq!(texel, &[0; 4]);
            } else {
                assert_eq!(texel[3], 255);
                assert!(texel[..3].iter().all(|&value| (89..=91).contains(&value)));
            }
        }
    }

    #[test]
    fn dynamic_sh_retains_float_values_while_static_sh_round_trips() {
        let source = LightingCoefficients {
            coefficients: vec![[40000; 27]],
        };
        let dynamic = source.grid_lighting(&[(0, 1.)]);
        let static_light = source.static_lighting(0);
        assert_eq!(dynamic.texels, static_light.texels);
        assert_eq!(
            static_light.sh,
            dynamic
                .sh
                .map(|row| Vec4::from_array(row.to_array().map(quantize_sh)))
        );
        assert_ne!(dynamic.sh, static_light.sh);
    }
}
