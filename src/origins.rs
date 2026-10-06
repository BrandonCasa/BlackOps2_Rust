//! Origins content definition and tunable practical gameplay profile.
use bevy::prelude::*;
use serde::Deserialize;
use std::path::Path;
#[derive(Resource, Deserialize, Clone)]
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
    #[serde(default)]
    pub tuning: Tuning,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Tuning {
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub crouch_speed: f32,
    pub gravity: f32,
    pub jump_speed: f32,
    pub step_height: f32,
    pub radius: f32,
    pub standing_height: f32,
    pub crouch_height: f32,
    pub eye_height: f32,
    pub mouse_sensitivity: f32,
    pub pistol_damage: u32,
    pub melee_damage: u32,
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
            walk_speed: 4.8,
            sprint_speed: 7.2,
            crouch_speed: 2.4,
            gravity: 20.,
            jump_speed: 6.,
            step_height: 0.45,
            radius: 0.35,
            standing_height: 1.8,
            crouch_height: 1.15,
            eye_height: 1.62,
            mouse_sensitivity: 0.002,
            pistol_damage: 50,
            melee_damage: 150,
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
    pub fn validate(&self) -> Result<(), String> {
        for path in [&self.scene, &self.collision_scene] {
            let base = path.split('#').next().unwrap_or("");
            if base.is_empty()
                || Path::new(base).is_absolute()
                || Path::new(base)
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err("Scene references must be relative asset paths.".into());
            }
        }
        let values = [
            self.tuning.walk_speed,
            self.tuning.sprint_speed,
            self.tuning.crouch_speed,
            self.tuning.gravity,
            self.tuning.jump_speed,
            self.tuning.step_height,
            self.tuning.radius,
            self.tuning.standing_height,
            self.tuning.crouch_height,
            self.tuning.eye_height,
            self.tuning.mouse_sensitivity,
            self.tuning.fire_interval,
            self.tuning.reload_seconds,
            self.tuning.zombie_speed,
        ];
        if self.version != 1
            || self.spawn_points.is_empty()
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
            || self.tuning.standing_height <= 2. * self.tuning.radius
            || self.tuning.crouch_height <= 2. * self.tuning.radius
            || self.tuning.crouch_height > self.tuning.standing_height
            || self.tuning.magazine == 0
            || [
                self.tuning.pistol_damage,
                self.tuning.melee_damage,
                self.tuning.zombie_damage,
            ]
            .iter()
            .any(|value| *value == 0 || *value > i32::MAX as u32)
        {
            return Err(
                "Asset pack contains invalid gameplay parameters or incomplete scene coverage."
                    .into(),
            );
        }
        Ok(())
    }
}
