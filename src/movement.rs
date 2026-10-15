use bevy::prelude::*;
pub mod physics;
pub mod platforms;
#[derive(Component, Default)]
pub struct SprintState {
    pub remaining: f32,
    pub capacity: f32,
    pub sprinting: bool,
    pub exhausted: bool,
    pub last_run_seconds: f32,
    active_seconds: f32,
    recovery_age: f32,
    restart_blocked: bool,
}
impl SprintState {
    pub fn continuous_seconds(&self) -> f32 {
        self.active_seconds
    }

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
        latch_interruptions: bool,
    ) {
        if self.capacity != capacity {
            self.remaining = (self.remaining + capacity - self.capacity).clamp(0., capacity);
            self.capacity = capacity;
            self.exhausted |= self.remaining == 0.;
        }
        if !requested {
            self.exhausted = false;
            self.restart_blocked = false;
        }
        let was_sprinting = self.sprinting;
        self.sprinting = requested
            && eligible
            && !self.exhausted
            && !self.restart_blocked
            && self.remaining > if was_sprinting { 0. } else { minimum };
        if self.sprinting {
            self.remaining = (self.remaining - dt).max(0.);
            self.active_seconds += dt;
            self.recovery_age = 0.;
            self.exhausted |= self.remaining == 0.;
        } else {
            if was_sprinting {
                self.restart_blocked = requested && latch_interruptions;
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
#[serde(default)]
pub struct MobilityTuning {
    pub prone_height: f32,
    pub prone_eye: f32,
    pub prone_speed: f32,
    pub prone_step: f32,
    pub stance: StanceTuning,
    pub jump_recovery: f32,
    pub dive_jump_height: f32,
    pub dive_launch_multiplier: f32,
    pub dive_apex_seconds: f32,
    pub dive_recovery: f32,
    pub dive_startup: f32,
    pub dive_minimum_speed: f32,
    pub dive_exhaustion: f32,
    pub dive_slide: f32,
    pub dive_fall_minimum: f32,
    pub dive_fall_fatal: f32,
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
            prone_speed: 190. * 0.15 * 0.0254,
            prone_step: 10. * 0.0254,
            stance: StanceTuning::default(),
            jump_recovery: 0.25,
            dive_jump_height: 39. * 0.0254,
            dive_launch_multiplier: 2.,
            dive_apex_seconds: 0.4,
            dive_recovery: 0.1,
            dive_startup: 0.25,
            dive_minimum_speed: 3.16 * 0.0254,
            dive_exhaustion: 1.5,
            dive_slide: 0.3,
            dive_fall_minimum: 65. * 0.0254,
            dive_fall_fatal: 200. * 0.0254,
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

#[derive(Clone, serde::Deserialize)]
pub struct StanceTuning {
    pub seconds: [f32; 4],
    pub curves: [Vec<[f32; 2]>; 4],
    pub other_speed: f32,
}
impl Default for StanceTuning {
    fn default() -> Self {
        Self {
            seconds: [0.2, 0.2, 0.6, 0.4],
            curves: [
                vec![
                    [0., 1.],
                    [1., 0.975],
                    [4., 0.925],
                    [30., 0.8],
                    [80., 0.2],
                    [90., 0.075],
                    [95., 0.025],
                    [100., 0.],
                ],
                vec![
                    [0., 0.],
                    [5., 0.025],
                    [10., 0.075],
                    [20., 0.2],
                    [70., 0.8],
                    [96., 0.925],
                    [99., 0.975],
                    [100., 1.],
                ],
                vec![
                    [0., 1.],
                    [11., 0.931],
                    [22., 0.759],
                    [34., 0.483],
                    [45., 0.172],
                    [50., 0.138],
                    [55., 0.172],
                    [70., 0.241],
                    [90., 0.207],
                    [100., 0.],
                ],
                vec![
                    [0., 0.],
                    [5., -0.034],
                    [30., 0.345],
                    [50., 0.483],
                    [67., 0.69],
                    [83., 0.79],
                    [100., 1.],
                ],
            ],
            other_speed: 180. * 0.0254,
        }
    }
}
impl StanceTuning {


    fn sample(&self, kind: usize, percent: f32) -> f32 {
        let points = &self.curves[kind];
        let Some(pair) = points.windows(2).find(|pair| percent <= pair[1][0]) else {
            return points.last().unwrap()[1];
        };
        pair[0][1] + (pair[1][1] - pair[0][1]) * (percent - pair[0][0]) / (pair[1][0] - pair[0][0])
    }
}

#[derive(Clone, Copy)]
struct EyeTransition {
    kind: usize,
    elapsed: f32,
}

#[derive(Default)]
pub struct StanceEye {
    transition: Option<EyeTransition>,
}
impl StanceEye {
    pub fn speed_scale(&self, prone: f32, crouch: f32, tuning: &StanceTuning) -> Option<f32> {
        let phase = self.transition?;
        if phase.kind < 2 {
            return None;
        }
        let fraction = (phase.elapsed / tuning.seconds[phase.kind]).clamp(0., 1.);
        Some(if phase.kind == 2 {
            crouch + (prone - crouch) * fraction
        } else {
            prone + (crouch - prone) * fraction
        })
    }

    pub fn update(
        &mut self,
        eye: f32,
        target: f32,
        standing: f32,
        crouch: f32,
        prone: f32,
        dt: f32,
        tuning: &StanceTuning,
    ) -> f32 {
        let canonical = target == standing || target == crouch || target == prone;
        if !canonical {
            self.transition = None;
            return eye + (target - eye).clamp(-tuning.other_speed * dt, tuning.other_speed * dt);
        }
        if let Some(mut phase) = self.transition {
            let downward = phase.kind == 0 || phase.kind == 2;
            let end = match phase.kind {
                0 | 3 => crouch,
                1 => standing,
                _ => prone,
            };
            if (downward && target > end) || (!downward && target < end) {
                let percent = (phase.elapsed / tuning.seconds[phase.kind] * 100. + 0.0001)
                    .trunc()
                    .clamp(0., 100.);
                phase.kind ^= 1;
                phase.elapsed = tuning.seconds[phase.kind] * (100. - percent) * 0.01;
            }
            phase.elapsed += dt;
            let percent = (phase.elapsed / tuning.seconds[phase.kind] * 100. + 0.0001)
                .trunc()
                .clamp(0., 100.);
            let factor = tuning.sample(phase.kind, percent);
            let value = if phase.kind < 2 {
                crouch + (standing - crouch) * factor
            } else {
                prone + (crouch - prone) * factor
            };
            self.transition = (percent < 100.).then_some(phase);
            return value;
        }
        if eye == target {
            return eye;
        }
        let kind = if target < eye {
            if eye > crouch { 0 } else { 2 }
        } else if eye < crouch {
            3
        } else {
            1
        };
        self.transition = Some(EyeTransition { kind, elapsed: 0. });
        eye
    }
}

#[cfg(test)]
mod stance_tests {
    use super::*;

    #[test]
    fn source_stance_eye_stages_through_crouch_and_preserves_prone_rebound() {
        let tuning = StanceTuning::default();
        let mut transition = StanceEye::default();
        let mut eye = 60.;
        let advance = |transition: &mut StanceEye, eye, dt| {
            transition.update(eye, 11., 60., 40., 11., dt, &tuning)
        };
        eye = advance(&mut transition, eye, 0.);
        eye = advance(&mut transition, eye, 0.2);
        assert_eq!(eye, 40.);
        eye = advance(&mut transition, eye, 0.);
        eye = advance(&mut transition, eye, 0.3);
        assert!((eye - (11. + 29. * 0.138)).abs() < 0.00001);
        let low = eye;
        eye = advance(&mut transition, eye, 0.12);
        assert!((eye - (11. + 29. * 0.241)).abs() < 0.00001 && eye > low);
        eye = advance(&mut transition, eye, 0.18);
        assert_eq!(eye, 11.);
    }

    #[test]
    fn source_prone_rise_keeps_authored_undershoot_and_downed_linear_rate() {
        let tuning = StanceTuning::default();
        let mut transition = StanceEye::default();
        transition.update(11., 60., 60., 40., 11., 0., &tuning);
        let eye = transition.update(11., 60., 60., 40., 11., 0.02, &tuning);
        assert!((eye - (11. - 29. * 0.034)).abs() < 0.00001);
        let units = 0.0254;
        let eye = transition.update(
            60. * units,
            22. * units,
            60. * units,
            40. * units,
            11. * units,
            0.1,
            &tuning,
        );
        assert!((eye - (60. - 180. * 0.1) * units).abs() < 0.00001);
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
    pub launch_height: f32,
    pub elapsed: f32,
    pub slide_remaining: Option<f32>,
}
#[derive(Component, Default)]
pub struct Mobility {
    pub stance: Stance,
    pub dive: Option<Dive>,
    pub recovery: f32,
    pub since_dive: Option<f32>,
    pub jump_recovery: f32,
    pub landing_recovery: f32,
    pub airborne: bool,
    pub peak: f32,
    pub fall_speed: f32,
    pub fall_origin: f32,
    pub fall_velocity: f32,
    pub previous_eye: f32,
    pub landing_dip: f32,
    pub stance_eye: StanceEye,
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
    pub impact_height: f32,
    pub speed: f32,
    pub diving: bool,
    pub surface_flags: u32,
}
#[derive(Resource, Default)]
pub struct MovementEvidence {
    pub events: Vec<serde_json::Value>,
    pub motion: Vec<serde_json::Value>,
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
    pub rumble: Vec2,
    pub bob: Vec2,
    pub previous_bob: Vec2,
    pub phase: f32,
    pub position: Option<Vec3>,
    pub look: Option<Vec2>,
    pub sway: Vec2,
}
