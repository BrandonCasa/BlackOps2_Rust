//! Original T6 static-model directional lighting and object SH constants.
//!
//! Native recovery: 00466b50 builds the 56 cube directions; 00449b70/00449ad0
//! decode and evaluate light-grid coefficients; 00449f70 encodes atlas texels.
//! 0044b840 reduces RGB SH, 00457430/00457530 quantize/decode object constants,
//! and 004576d0/004189a0 supply atlas coordinates and lookup scale.
use bevy::prelude::{Vec3, Vec4};

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
        let packed = self.coefficients.get(colors_index).unwrap_or_else(|| {
            panic!(
                "Static model colorsIndex {colors_index} has no original light-grid coefficients"
            )
        });
        let coefficients: [Vec3; 9] = std::array::from_fn(|i| {
            Vec3::new(
                decode(packed[i * 3]),
                decode(packed[i * 3 + 1]),
                decode(packed[i * 3 + 2]),
            )
        });

        let mut texels = [0; 256];
        for z in 0..4 {
            for y in 0..4 {
                for x in 0..4 {
                    // The shader samples the cube boundary; all eight interior
                    // texels remain zero, as in native 00449f70.
                    if (1..3).contains(&x) && (1..3).contains(&y) && (1..3).contains(&z) {
                        continue;
                    }
                    let direction = Vec3::new(x as f32, y as f32, z as f32) * 0.6666667 - Vec3::ONE;
                    let direction = direction * direction.length().recip();
                    let rgb = evaluate(&coefficients, direction);
                    let offset = (z * 16 + y * 4 + x) * 4;
                    texels[offset] = encode_texel(rgb.x);
                    texels[offset + 1] = encode_texel(rgb.y);
                    texels[offset + 2] = encode_texel(rgb.z);
                    texels[offset + 3] = 255;
                }
            }
        }

        // 0044b840 fits one chromaticity and nine luminance coefficients into
        // the three vec4 object constants. Static models pass no hero-light
        // origin here; their authored colorsIndex owns this lighting sample.
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
        ]
        .map(|row| Vec4::from_array(row.to_array().map(quantize_sh)));

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
