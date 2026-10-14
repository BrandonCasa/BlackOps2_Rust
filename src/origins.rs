use bevy::prelude::*;
use serde::Deserialize;
use std::path::Path;
pub mod camo;
pub mod content;
pub use content::*;
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryScope {
    #[default]
    FullOrigins,
    FirstPlayableSolo,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialSpawn {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub excluded_side: u8,
}

#[derive(Resource, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct GameDefinition {
    pub version: u32,
    #[serde(default)]
    pub delivery_scope: DeliveryScope,
    #[serde(default)]
    pub initial_spawns: Vec<InitialSpawn>,
    pub scene: String,
    pub collision_scene: String,
    pub spawn: [f32; 3],
    pub yaw: f32,
    pub spawn_points: Vec<[f32; 3]>,
    #[serde(default)]
    pub waypoints: Vec<[f32; 3]>,
    #[serde(default)]
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
    pub fn load(bytes: &[u8]) -> Result<Self, ContentError> {
        let path = Path::new("game.json");
        if bytes.len() > 32 * 1024 * 1024 {
            return Err(ContentError::InvalidDefinition(
                "Gameplay definition exceeds 32 MiB.".into(),
            ));
        }
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
        if header
            .get("delivery_scope")
            .is_none_or(|scope| scope == "full_origins")
        {
            for field in [
                "powerups",
                "generators",
                "robot_routes",
                "tank_routes",
                "weather",
                "digging",
                "recipes",
                "staff_puzzles",
                "quests",
                "quest_transitions",
                "cinematics",
            ] {
                if header["content"].get(field).is_none() {
                    return Err(ContentError::InvalidDefinition(format!(
                        "Full Origins catalog is missing {field}"
                    )));
                }
            }
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
        if self.delivery_scope == DeliveryScope::FirstPlayableSolo {
            self.content.validate_first_playable()?;
            if self.access.spawners.is_empty() || self.access.entrances.is_empty()
                || self.initial_spawns.is_empty()
                || self.initial_spawns.iter().any(|spawn| {
                    !spawn.position.iter().all(|value| value.is_finite())
                        || !spawn.yaw.is_finite()
                        || !spawn.pitch.is_finite()
                        || spawn.pitch.abs() > 1.5
                        || !(1..=2).contains(&spawn.excluded_side)
                })
                || !(1..=2).all(|side| {
                    self.initial_spawns
                        .iter()
                        .any(|spawn| spawn.excluded_side != side)
                })
            {
                return Err(ContentError::InvalidDefinition(
                    "First playable solo lacks valid source initial spawn candidates".into(),
                ));
            }
            if !self
                .content
                .perks
                .iter()
                .any(|perk| {
                    perk.id == self.solo_revive.perk
                        && !perk.collision.is_empty()
                        && perk.path_bounds_radius.is_some()
                })
                || self.content.sounds.is_empty()
                || self.audio.aliases.is_empty()
                || self.audio.bindings.is_empty()
            {
                return Err(ContentError::InvalidDefinition(
                    "First playable solo requires source Quick Revive and essential audio".into(),
                ));
            }
        }
        if !self.audio.valid(self) {
            return Err(ContentError::InvalidDefinition(
                "Invalid audio aliases or bindings.".into(),
            ));
        }
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
            self.tuning.crouch_eye,
            self.tuning.eye_height,
            self.tuning.use_radius,
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
            || !self.environment.valid(self)
            || self.content.barriers.iter().any(|barrier|!barrier.valid_actions())
            || self.content.generators.iter().any(|generator| generator.initial_reward.as_ref().is_some_and(|reward| !self.environment.triggers.iter().any(|trigger|trigger.id==reward.source_trigger)))
            || self.content.perks.iter().any(|perk| perk.bump_trigger.as_ref().is_some_and(|id| !self.environment.triggers.iter().any(|trigger| trigger.id == *id)))
            || self.content.interactions.iter().any(|interaction| {
                interaction.source_trigger.as_ref().is_some_and(|id| {
                    !self
                        .environment
                        .triggers
                        .iter()
                        .any(|trigger| trigger.id == *id)
                })
            })
            || !self.access.valid(&self.content, &self.environment)
            || !self.tuning.physics.valid()
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
            || self.tuning.crouch_eye >= self.tuning.crouch_height
            || self
                .tuning
                .combat_points
                .iter()
                .any(|v| *v > i32::MAX as u32)
            || self.tuning.combat_neck_points > i32::MAX as u32
            || self
                .tuning
                .global_hit_multipliers
                .iter()
                .any(|value| !value.is_finite() || *value < 0.)
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
