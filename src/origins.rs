//! Origins content definition and tunable practical gameplay profile.
use bevy::prelude::*;
use serde::Deserialize;
use std::path::Path;
pub mod content;
pub use content::*;
#[derive(Resource, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct GameDefinition {
    pub version: u32,
    pub scene: String,
    pub collision_scene: String,
    pub spawn: [f32; 3],
    pub yaw: f32,
    pub spawn_points: Vec<[f32; 3]>,
    pub waypoints: Vec<[f32; 3]>,
    pub survey_views: Vec<[f32; 3]>,
    pub source_surfaces: usize,
    pub render_meshes: usize,
    pub collision_meshes: usize,
    pub static_model_instances: usize,
    pub content: ContentCatalog,
    pub player_health: crate::player_health::HealthProfile,
    pub rounds: crate::rounds::RoundProfile,
    pub solo_revive: crate::solo_revive::SoloReviveProfile,
    #[serde(default)]
    pub tuning: Tuning,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Tuning {
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
    pub eye_height: f32,
    pub mouse_sensitivity: f32,
    pub ads_mouse_sensitivity: f32,
    pub pistol_damage: u32,
    pub melee_damage: u32,
    pub combat_points: [u32; 5],
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
            mobility: default(),
            walk_speed: 4.8,
            sprint_speed: 7.2,
            sprint_seconds: 4.,
            sprint_minimum: 0.25,
            sprint_recovery_rate: 1.,
            sprint_recovery_delay: 0.,
            crouch_speed: 2.4,
            gravity: 20.,
            jump_speed: 6.,
            step_height: 0.45,
            radius: 0.35,
            standing_height: 1.8,
            crouch_height: 1.15,
            eye_height: 1.62,
            mouse_sensitivity: 0.002,
            ads_mouse_sensitivity: 0.0012,
            pistol_damage: 50,
            melee_damage: 150,
            combat_points: [10, 50, 50, 10, 80],
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
    pub fn load(path: &Path) -> Result<Self, ContentError> {
        let size = std::fs::metadata(path)
            .map_err(|source| ContentError::Io {
                path: path.into(),
                source,
            })?
            .len();
        if size > 32 * 1024 * 1024 {
            return Err(ContentError::InvalidDefinition(
                "Gameplay definition exceeds 32 MiB.".into(),
            ));
        }
        let bytes = std::fs::read(path).map_err(|source| ContentError::Io {
            path: path.into(),
            source,
        })?;
        if bytes.len() > 32 * 1024 * 1024 {
            return Err(ContentError::InvalidDefinition(
                "Gameplay definition exceeds 32 MiB.".into(),
            ));
        }
        // Version is checked before parsing a newer/older schema's required fields.
        let header: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|source| ContentError::Json {
                path: path.into(),
                source,
            })?;
        let version = header
            .get("version")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
            .ok_or_else(|| {
                ContentError::InvalidDefinition("Missing integer content version.".into())
            })?;
        if version != 2 {
            return Err(ContentError::UnsupportedVersion(version));
        }
        let definition: Self =
            serde_json::from_value(header).map_err(|source| ContentError::Json {
                path: path.into(),
                source,
            })?;
        definition.validate()?;
        Ok(definition)
    }
    pub fn validate(&self) -> Result<(), ContentError> {
        if self.version != 2 {
            return Err(ContentError::UnsupportedVersion(self.version));
        }
        self.content.validate()?;
        if !self.solo_revive.valid(self) {
            return Err(ContentError::InvalidDefinition(
                "Invalid solo revive profile.".into(),
            ));
        }
        if !self.player_health.valid() {
            return Err(ContentError::InvalidDefinition(
                "Invalid player health profile. Regenerate/import game.json.".into(),
            ));
        }
        if !self.rounds.valid() {
            return Err(ContentError::InvalidDefinition(
                "Invalid round profile. Regenerate/import game.json.".into(),
            ));
        }
        for path in [&self.scene, &self.collision_scene] {
            validate_asset_path(path)?;
            let base = path.split('#').next().unwrap_or("");
            if base.is_empty()
                || Path::new(base).is_absolute()
                || Path::new(base)
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(ContentError::InvalidPath(path.clone()));
            }
        }
        let values = [
            self.tuning.walk_speed,
            self.tuning.sprint_speed,
            self.tuning.sprint_seconds,
            self.tuning.sprint_minimum,
            self.tuning.sprint_recovery_rate,
            self.tuning.crouch_speed,
            self.tuning.gravity,
            self.tuning.jump_speed,
            self.tuning.step_height,
            self.tuning.radius,
            self.tuning.standing_height,
            self.tuning.crouch_height,
            self.tuning.eye_height,
            self.tuning.mouse_sensitivity,
            self.tuning.ads_mouse_sensitivity,
            self.tuning.fire_interval,
            self.tuning.reload_seconds,
            self.tuning.zombie_speed,
        ];
        if self.spawn_points.is_empty()
            || self.render_meshes == 0
            || self.collision_meshes == 0
            || !self.yaw.is_finite()
            || !self
                .spawn
                .iter()
                .chain(self.spawn_points.iter().flatten())
                .chain(self.waypoints.iter().flatten())
                .chain(self.survey_views.iter().flatten())
                .all(|v| v.is_finite())
            || !values.iter().all(|v| v.is_finite() && *v > 0.)
            || !self
                .tuning
                .mobility
                .valid(self.tuning.radius, self.tuning.crouch_height)
            || !self.tuning.sprint_recovery_delay.is_finite()
            || self.tuning.sprint_recovery_delay < 0.
            || self.tuning.sprint_minimum > self.tuning.sprint_seconds
            || self.tuning.standing_height <= 2. * self.tuning.radius
            || self.tuning.crouch_height <= 2. * self.tuning.radius
            || self.tuning.crouch_height > self.tuning.standing_height
            || self
                .tuning
                .combat_points
                .iter()
                .any(|v| *v > i32::MAX as u32)
            || self.tuning.magazine == 0
            || [
                self.tuning.pistol_damage,
                self.tuning.melee_damage,
                self.tuning.zombie_damage,
            ]
            .iter()
            .any(|value| *value == 0 || *value > i32::MAX as u32)
        {
            return Err(ContentError::InvalidDefinition(
                "Asset pack contains invalid gameplay parameters or incomplete scene coverage."
                    .into(),
            ));
        }
        Ok(())
    }
}
