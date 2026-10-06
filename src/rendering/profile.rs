//! One embedded reconstruction profile. Surface/light colors are sRGB; the
//! generated HDR sky palette is linear radiance, scaled by sky_nits.
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Resource, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VisualProfile {
    pub ambient_color: [f32; 3],
    pub ambient_brightness: f32,
    pub sun_color: [f32; 3],
    pub sun_illuminance: f32,
    pub sun_rotation: [f32; 3],
    pub indoor_ev100: f32,
    pub outdoor_ev100: f32,
    pub adaptation_speed: f32,
    pub baked_radiance: f32,
    pub vertex_radiance: f32,
    pub probe_radiance: f32,
    pub instance_probe_fill: f32,
    pub transparent_opacity_scale: f32,
    pub reveal_softness: f32,
    pub post_saturation: f32,
    pub comparison_fov_degrees: f32,
    pub reflection_radiance: f32,
    pub normal_strength: f32,
    pub tile_normal_strength: f32,
    pub gloss_exponent: f32,
    pub specular_f0_scale: f32,
    pub bloom_intensity: f32,
    pub fog_color: [f32; 3],
    pub fog_density: f32,
    pub lantern_color: [f32; 3],
    pub lantern_lumens: f32,
    pub lantern_range: f32,
    pub emissive_nits: f32,
    pub lantern_emissive_nits: f32,
    pub water_color: [f32; 3],
    pub water_opacity: f32,
    pub water_roughness: f32,
    pub water_normal_strength: f32,
    pub water_uv_scale: f32,
    pub water_velocity: [f32; 4],
    pub wetness: f32,
    pub wet_darkening: f32,
    pub wet_roughness: f32,
    pub sky_nits: f32,
    pub sky_upper: [f32; 3],
    pub sky_horizon: [f32; 3],
    pub cloud_dark: [f32; 3],
    pub cloud_bright: [f32; 3],
    pub sky_sun_direction: [f32; 3],
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
