use bevy::prelude::*;
use serde::Deserialize;

#[derive(Resource, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VisualProfile {
    pub ambient_color: [f32; 3],
    pub ambient_brightness: f32,
    pub environment_intensity: f32,
    pub sun_illuminance: f32,
    pub comparison_fov_degrees: f32,
    pub bloom_intensity: f32,
    pub lantern_color: [f32; 3],
    pub lantern_lumens: f32,
    pub lantern_range: f32,
    pub emissive_nits: f32,
    pub lantern_emissive_nits: f32,
}

impl Default for VisualProfile {
    fn default() -> Self {
        Self {
            ambient_color: [1.0, 1.0, 1.0],
            ambient_brightness: 30.0,
            environment_intensity: 0.02,
            sun_illuminance: 18000.0,
            comparison_fov_degrees: 55.0,
            lantern_color: [1.0, 0.71, 0.4],
            lantern_lumens: 6000.0,
            lantern_range: 10.0,
            emissive_nits: 150.0,
            lantern_emissive_nits: 4000.0,
            bloom_intensity: 0.14,
        }
    }
}

pub(super) fn color(rgb: [f32; 3]) -> Color {
    Color::srgb(rgb[0], rgb[1], rgb[2])
}
