use bevy::prelude::*;
#[derive(Component, Default)]
pub struct SprintState {
    pub remaining: f32,
    pub capacity: f32,
    pub sprinting: bool,
    pub exhausted: bool,
    pub last_run_seconds: f32,
    active_seconds: f32,
    recovery_age: f32,
}
impl SprintState {
    pub fn new(capacity: f32) -> Self {
        Self {
            remaining: capacity,
            capacity,
            ..default()
        }
    }
    pub fn update(
        &mut self,
        dt: f32,
        requested: bool,
        eligible: bool,
        capacity: f32,
        minimum: f32,
        recovery_rate: f32,
        recovery_delay: f32,
    ) {
        if self.capacity != capacity {
            self.remaining = (self.remaining + capacity - self.capacity).clamp(0., capacity);
            self.capacity = capacity;
            self.exhausted |= self.remaining == 0.;
        }
        if !requested && self.remaining >= minimum {
            self.exhausted = false;
        }
        let was_sprinting = self.sprinting;
        self.sprinting = requested && eligible && !self.exhausted && self.remaining > 0.;
        if self.sprinting {
            self.remaining = (self.remaining - dt).max(0.);
            self.active_seconds += dt;
            self.recovery_age = 0.;
            self.exhausted |= self.remaining == 0.;
        } else {
            if was_sprinting {
                self.last_run_seconds = self.active_seconds;
                self.active_seconds = 0.;
            }
            let before = self.recovery_age;
            self.recovery_age += dt;
            let eligible_dt =
                (self.recovery_age - recovery_delay).max(0.) - (before - recovery_delay).max(0.);
            self.remaining = (self.remaining + eligible_dt * recovery_rate).min(capacity);
        }
    }
}

#[derive(Clone, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MobilityTuning {
    pub prone_height: f32,
    pub prone_eye: f32,
    pub prone_speed: f32,
    pub prone_step: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub air_acceleration: f32,
    pub air_deceleration: f32,
    pub stance_seconds: f32,
    pub jump_recovery: f32,
    pub dive_speed: f32,
    pub dive_lift: f32,
    pub dive_recovery: f32,
    pub fall_minimum: f32,
    pub fall_fatal: f32,
    pub landing_dip: f32,
    pub landing_seconds: f32,
    pub walk_bob: [f32; 2],
    pub sprint_bob: [f32; 2],
    pub stride: [f32; 2],
    pub sway_limit: f32,
    pub sway_response: f32,
}
impl Default for MobilityTuning {
    fn default() -> Self {
        Self {
            prone_height: 0.75,
            prone_eye: 0.3048,
            prone_speed: 1.1,
            prone_step: 0.12,
            acceleration: 40.,
            deceleration: 50.,
            air_acceleration: 8.,
            air_deceleration: 2.,
            stance_seconds: 0.18,
            jump_recovery: 0.25,
            dive_speed: 8.,
            dive_lift: 3.2,
            dive_recovery: 0.55,
            fall_minimum: 3.2512,
            fall_fatal: 7.62,
            landing_dip: 0.12,
            landing_seconds: 0.3,
            walk_bob: [0.016, 0.024],
            sprint_bob: [0.025, 0.04],
            stride: [2.4, 3.0],
            sway_limit: 0.04,
            sway_response: 14.,
        }
    }
}
impl MobilityTuning {
    pub fn valid(&self, radius: f32, crouch: f32) -> bool {
        [
            self.prone_height,
            self.prone_eye,
            self.prone_speed,
            self.prone_step,
            self.acceleration,
            self.deceleration,
            self.air_acceleration,
            self.air_deceleration,
            self.stance_seconds,
            self.jump_recovery,
            self.dive_speed,
            self.dive_lift,
            self.dive_recovery,
            self.fall_minimum,
            self.fall_fatal,
            self.landing_dip,
            self.landing_seconds,
            self.sway_limit,
            self.sway_response,
            self.walk_bob[0],
            self.walk_bob[1],
            self.sprint_bob[0],
            self.sprint_bob[1],
            self.stride[0],
            self.stride[1],
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.)
            && self.prone_height > 2. * radius
            && self.prone_height < crouch
            && self.prone_eye < self.prone_height
            && self.fall_minimum < self.fall_fatal
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stance {
    #[default]
    Standing,
    Crouched,
    Prone,
}
#[derive(Clone, Copy)]
pub struct Dive {
    pub direction: Vec3,
}
#[derive(Component, Default)]
pub struct Mobility {
    pub stance: Stance,
    pub dive: Option<Dive>,
    pub recovery: f32,
    pub jump_recovery: f32,
    pub airborne: bool,
    pub peak: f32,
    pub fall_speed: f32,
    pub previous_eye: f32,
    pub landing_dip: f32,
}
impl Mobility {
    pub fn new(eye: f32) -> Self {
        Self {
            previous_eye: eye,
            ..default()
        }
    }
    pub fn busy(&self) -> bool {
        self.dive.is_some() || self.recovery > 0.
    }
}
#[derive(Message, Clone)]
pub struct Landing {
    pub owner: Entity,
    pub position: Vec3,
    pub height: f32,
    pub speed: f32,
    pub diving: bool,
}
#[derive(Resource, Default)]
pub struct MovementEvidence {
    pub events: Vec<serde_json::Value>,
}
impl MovementEvidence {
    pub fn record(&mut self, event: serde_json::Value) {
        if self.events.len() < 128 {
            self.events.push(event);
        }
    }
}

#[derive(Component, Default)]
pub struct MotionFeedback {
    pub bob: Vec2,
    pub previous_bob: Vec2,
    pub phase: f32,
    pub position: Option<Vec3>,
    pub look: Option<Vec2>,
    pub sway: Vec2,
}
