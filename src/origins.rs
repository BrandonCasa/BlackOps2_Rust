use bevy::prelude::*;
use serde::Deserialize;
use std::path::Path;
pub(crate) mod actors;
pub(crate) mod weapons;
pub(crate) mod audio;
pub mod content;
pub(crate) mod navigation;
pub(crate) mod zones;
pub use content::*;
#[derive(Resource, Deserialize, Clone)]
pub struct GameDefinition {
    pub version: u32,
    pub scene: String,
    #[serde(default)]
    pub additional_scenes: Vec<String>,
    pub collision_scene: String,
    pub collision_metadata: String,
    pub world_metadata: String,
    #[serde(default)]
    pub navigation_metadata: Option<String>,
    #[serde(default)]
    pub entity_metadata: Option<String>,
    #[serde(default)]
    pub brush_hulls: Option<String>,
    #[serde(default)]
    pub navigation_zones: std::collections::BTreeMap<u16, ZoneId>,
    #[serde(default)]
    pub camos: std::collections::BTreeMap<String, String>,
    pub spawn: [f32; 3],
    pub yaw: f32,
    pub spawn_points: Vec<[f32; 3]>,
    #[serde(default)]
    pub waypoints: Vec<[f32; 3]>,
    #[serde(default)]
    pub survey_views: Vec<[f32; 3]>,
    pub content: ContentCatalog,
    pub player_health: crate::player_health::HealthProfile,
    pub rounds: crate::rounds::RoundProfile,
    pub solo_revive: crate::solo_revive::SoloReviveProfile,
    #[serde(default)]
    pub generator_crystals: Option<crate::generators::CrystalProfile>,
    #[serde(default)]
    pub environment: crate::environment::EnvironmentDefinition,
    #[serde(default)]
    pub access: crate::access::AccessDefinition,
    #[serde(default)]
    pub audio: crate::audio::AudioDefinition,
    #[serde(default)]
    pub tuning: Tuning,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Tuning {
    pub physics: crate::movement::physics::PhysicsTuning,
    pub mobility: crate::movement::MobilityTuning,
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub sprint_seconds: f32,
    pub sprint_minimum: f32,
    pub sprint_recovery_rate: f32,
    pub sprint_recovery_delay: f32,
    pub crouch_speed: f32,
    pub gravity: f32,
    pub jump_speed: f32,
    pub step_height: f32,
    pub radius: f32,
    pub standing_height: f32,
    pub crouch_height: f32,
    pub crouch_eye: f32,
    pub eye_height: f32,
    pub use_radius: f32,
    pub mouse_sensitivity: f32,
    pub ads_mouse_sensitivity: f32,
    pub pistol_damage: u32,
    pub melee_damage: u32,
    pub combat_points: [u32; 5],
    pub combat_neck_points: u32,
    pub global_hit_multipliers: [f32; 21],
    pub magazine: u32,
    pub reserve: u32,
    pub fire_interval: f32,
    pub reload_seconds: f32,
    pub zombie_damage: u32,
    pub zombie_speed: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            physics: default(),
            mobility: default(),
            walk_speed: 190. * 0.0254,
            sprint_speed: 190. * 1.5 * 0.0254,
            sprint_seconds: 4.,
            sprint_minimum: 1.,
            sprint_recovery_rate: 1.,
            sprint_recovery_delay: 0.,
            crouch_speed: 190. * 0.65 * 0.0254,
            gravity: 800. * 0.0254,
            jump_speed: (2_f32 * 39. * 0.0254 * 800. * 0.0254).sqrt(),
            step_height: 18. * 0.0254,
            radius: 0.35,
            standing_height: 1.8,
            crouch_height: 1.15,
            crouch_eye: 40. * 0.0254,
            eye_height: 1.62,
            use_radius: 72. * 0.0254,
            mouse_sensitivity: 0.002,
            ads_mouse_sensitivity: 0.0012,
            pistol_damage: 50,
            melee_damage: 150,
            combat_points: [10, 50, 50, 10, 80],
            combat_neck_points: 20,
            global_hit_multipliers: [
                1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 1., 0., 1.,
            ],
            magazine: 8,
            reserve: 32,
            fire_interval: 0.2,
            reload_seconds: 1.5,
            zombie_damage: 50,
            zombie_speed: 1.15,
        }
    }
}

impl GameDefinition {
    pub fn load_from_root(root: &Path) -> Result<Self, ContentError> {
        let path = root.join("game.json");
        let bytes = std::fs::read(&path).map_err(|source| ContentError::Io { path, source })?;
        let mut definition = Self::decode(&bytes)?;
        weapons::bind(&mut definition, root).map_err(ContentError::InvalidDefinition)?;
        audio::bind(&mut definition, root);
        match (&definition.entity_metadata, &definition.brush_hulls) {
            (Some(entities), Some(hulls)) => {
                let read = |name: &str| -> Result<Vec<u8>, ContentError> {
                    let path = root.join(name);
                    std::fs::read(&path).map_err(|source| ContentError::Io { path, source })
                };
                let entities = read(entities)?;
                let hulls = read(hulls)?;
                let clip = read(&definition.collision_metadata)?;
                zones::bind(&mut definition, &entities, &clip, &hulls)
                    .map_err(ContentError::InvalidDefinition)?;
            }
            (None, None) => {}
            _ => {
                return Err(ContentError::InvalidDefinition(
                    "External zone binding requires both entity_metadata and brush_hulls".into(),
                ));
            }
        }
        crate::cinematics::bind_frames(&mut definition, root)
            .map_err(ContentError::InvalidDefinition)?;
        actors::bind(&mut definition, root).map_err(ContentError::InvalidDefinition)?;
        if let Some(path) = &definition.navigation_metadata {
            let path = root.join(path);
            let bytes = std::fs::read(&path).map_err(|source| ContentError::Io { path, source })?;
            let zones = navigation::derive_zones(&definition, &bytes, &definition.navigation_zones)
                .map_err(ContentError::InvalidDefinition)?;
            navigation::bind(&mut definition, &bytes, &zones)
                .map_err(ContentError::InvalidDefinition)?;
        }
        Ok(definition)
    }

    pub fn load(bytes: &[u8]) -> Result<Self, ContentError> {
        let definition = Self::decode(bytes)?;
        Ok(definition)
    }

    fn decode(bytes: &[u8]) -> Result<Self, ContentError> {
        serde_json::from_slice(bytes).map_err(|source| ContentError::Json {
            path: Path::new("game.json").into(),
            source,
        })
    }
}

