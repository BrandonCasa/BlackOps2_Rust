use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize, serde::Serialize)]
pub(crate) enum Antialiasing {
    Msaa4,
    Smaa,
    Taa,
}

#[derive(Resource, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VisualProfile {
    pub ambient_color: [f32; 3],
    pub ambient_brightness: f32,
    pub environment_intensity: f32,
    pub sun_illuminance: f32,
    pub indoor_ev100: f32,
    pub outdoor_ev100: f32,
    pub adaptation_speed: f32,
    pub transparent_opacity_scale: f32,
    pub reveal_softness: f32,
    pub comparison_fov_degrees: f32,
    pub antialiasing: Antialiasing,
    pub bloom_intensity: f32,
    pub sun_shadow_map_size: usize,
    pub sun_soft_shadow_size: f32,
    pub sun_shadow_distance: f32,
    pub sun_shadow_cascades: usize,
    pub sun_shadow_first_cascade: f32,
    pub lantern_color: [f32; 3],
    pub lantern_lumens: f32,
    pub lantern_range: f32,
    pub lantern_shadow_map_size: usize,
    pub max_local_lights: usize,
    pub max_local_shadows: usize,
    pub local_shadow_admission_distance: f32,
    pub local_shadow_retention_distance: f32,
    pub local_light_distance: f32,
    pub local_light_fade_seconds: f32,
    pub emissive_nits: f32,
    pub lantern_emissive_nits: f32,
    pub water_color: [f32; 3],
    pub water_opacity: f32,
    pub water_roughness: f32,
    pub water_normal_strength: f32,
    pub water_uv_scale: f32,
    pub water_velocity: [f32; 4],
}

impl Default for VisualProfile {
    fn default() -> Self {
        serde_json::from_str(include_str!("../../config/origins-visual.json"))
            .expect("The committed visual profile must be valid")
    }
}

pub(super) fn color(rgb: [f32; 3]) -> Color {
    Color::srgb(rgb[0], rgb[1], rgb[2])
}
