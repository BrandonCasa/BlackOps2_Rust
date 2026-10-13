use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize, serde::Serialize)]
pub(crate) enum Antialiasing {
    Dlss,
}

#[derive(Resource, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VisualProfile {
    pub ambient_color: [f32; 3],
    pub ambient_brightness: f32,
    pub environment_intensity: f32,
    pub sun_illuminance: f32,
    pub comparison_fov_degrees: f32,
    pub antialiasing: Antialiasing,
    pub bloom_intensity: f32,
    pub sun_shadow_map_size: usize,
    pub sun_shadow_distance: f32,
    pub sun_shadow_cascades: usize,
    pub sun_shadow_first_cascade: f32,
    pub lantern_color: [f32; 3],
    pub lantern_lumens: f32,
    pub lantern_range: f32,
    pub lantern_shadow_map_size: usize,
    pub max_local_lights: usize,
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
            antialiasing: Antialiasing::Dlss,
            lantern_color: [1.0, 0.71, 0.4],
            lantern_lumens: 6000.0,
            lantern_range: 10.0,
            lantern_shadow_map_size: 512,
            emissive_nits: 150.0,
            lantern_emissive_nits: 4000.0,
            sun_shadow_map_size: 4096,
            sun_shadow_distance: 60.0,
            sun_shadow_cascades: 4,
            sun_shadow_first_cascade: 8.0,
            max_local_lights: 8,
            bloom_intensity: 0.14,
        }
    }
}

pub(super) fn color(rgb: [f32; 3]) -> Color {
    Color::srgb(rgb[0], rgb[1], rgb[2])
}
