use bevy::prelude::*;
use serde::Deserialize;

#[derive(Deserialize, serde::Serialize)]
pub(crate) enum Antialiasing {
    Msaa4,
    Smaa,
    Taa,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ColorGrade {
    pub strength: f32,
    pub contrast: f32,
    pub vibrance: f32,
    pub shadow_tint: [f32; 3],
    pub highlight_tint: [f32; 3],
}

#[derive(Resource, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VisualProfile {
    pub ambient_color: [f32; 3],
    pub ambient_brightness: f32,
    pub environment_intensity: f32,
    pub sun_color: [f32; 3],
    pub sun_illuminance: f32,
    pub sun_rotation: [f32; 3],
    pub indoor_ev100: f32,
    pub outdoor_ev100: f32,
    pub adaptation_speed: f32,
    pub transparent_opacity_scale: f32,
    pub reveal_softness: f32,
    pub post_saturation: f32,
    pub color_grade: ColorGrade,
    pub comparison_fov_degrees: f32,
    pub antialiasing: Antialiasing,
    pub bloom_intensity: f32,
    pub fog_color_linear: [f32; 3],
    pub fog_brightness: f32,
    pub fog_base_distance: f32,
    pub fog_half_distance: f32,
    pub fog_distance_scale: f32,
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

impl VisualProfile {
    pub(super) fn fog_color(&self) -> Color {
        Color::linear_rgb(
            self.fog_color_linear[0] * self.fog_brightness,
            self.fog_color_linear[1] * self.fog_brightness,
            self.fog_color_linear[2] * self.fog_brightness,
        )
    }

    pub(super) fn fog_falloff(&self) -> FogFalloff {
        FogFalloff::ExponentialSquared {
            density: std::f32::consts::LN_2.sqrt()
                / ((self.fog_base_distance + self.fog_half_distance) * self.fog_distance_scale),
        }
    }
}

pub(super) fn color(rgb: [f32; 3]) -> Color {
    Color::srgb(rgb[0], rgb[1], rgb[2])
}
