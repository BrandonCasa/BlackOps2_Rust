//! Bounded sprint endurance, independent of map/perk identity.
use bevy::prelude::*;
#[derive(Component)]
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
            sprinting: false,
            exhausted: false,
            recovery_age: 0.,
            last_run_seconds: 0.,
            active_seconds: 0.,
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
            // Preserve spent seconds rather than refill endurance on perk changes.
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
