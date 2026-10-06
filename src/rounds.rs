//! Reusable solo round profile and population accounting; map preparation supplies tuning.
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoundProfile {
    pub health_start: u32,
    pub health_increase: u32,
    pub health_multiplier: f64,
    pub exponential_round: u32,
    pub population_base: u32,
    pub solo_population_increment: f64,
    pub population_round_divisor: u32,
    pub population_acceleration_round: u32,
    pub population_acceleration: f64,
    pub early_population: [f64; 5],
    pub ordinary_cap: u32,
    pub actor_cap: u32,
    pub spawn_seconds: f32,
    pub spawn_decay: f32,
    pub minimum_spawn_seconds: f32,
    pub first_spawn_seconds: f32,
    pub between_round_seconds: f32,
    pub speed_multiplier: u32,
    pub speed_random_band: u32,
    pub gait_thresholds: [u32; 2],
    pub gait_speeds: [f32; 3],
}
impl RoundProfile {
    pub fn valid(&self) -> bool {
        self.health_start > 0
            && self.health_start <= i32::MAX as u32
            && self.health_increase > 0
            && self.health_increase <= i32::MAX as u32
            && self.exponential_round >= 2
            && self.population_base > 0
            && self.population_round_divisor > 0
            && self.population_acceleration_round > 0
            && self.ordinary_cap > 0
            && self.actor_cap >= self.ordinary_cap
            && self.speed_random_band > 0
            && self.speed_multiplier > 0
            && self.gait_thresholds[0] < self.gait_thresholds[1]
            && [
                self.health_multiplier,
                self.solo_population_increment,
                self.population_acceleration,
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
            && self
                .early_population
                .iter()
                .all(|v| v.is_finite() && *v > 0. && *v <= 1.)
            && [
                self.spawn_seconds,
                self.spawn_decay,
                self.minimum_spawn_seconds,
                self.first_spawn_seconds,
                self.between_round_seconds,
            ]
            .iter()
            .chain(self.gait_speeds.iter())
            .all(|v| v.is_finite() && *v > 0.)
            && self.spawn_decay <= 1.
            && self.minimum_spawn_seconds <= self.spawn_seconds
            && self.gait_speeds.windows(2).all(|v| v[0] < v[1])
    }
    pub fn health(&self, round: u32) -> u32 {
        let mut health = self.health_start;
        for r in 2..=round.clamp(1, 255) {
            let increment = if r >= self.exponential_round {
                (health as f64 * self.health_multiplier) as u32
            } else {
                self.health_increase
            };
            // Native scripts retain the prior health on signed integer overflow.
            let Some(next) = health
                .checked_add(increment)
                .filter(|v| *v <= i32::MAX as u32)
            else {
                return health;
            };
            health = next;
        }
        health
    }
    pub fn population(&self, round: u32) -> u32 {
        let round = round.clamp(1, 255);
        let mut multiplier = (round as f64 / self.population_round_divisor as f64).max(1.);
        if round >= self.population_acceleration_round {
            multiplier *= round as f64 * self.population_acceleration;
        }
        let base = self
            .population_base
            .saturating_add((self.solo_population_increment * multiplier) as u32);
        if round <= self.early_population.len() as u32 {
            (base as f64 * self.early_population[(round - 1) as usize]) as u32
        } else {
            base
        }
    }
    pub fn spawn_delay(&self, round: u32) -> f32 {
        let mut delay = self.spawn_seconds;
        for _ in 1..round.clamp(1, 255) {
            delay = if delay > self.minimum_spawn_seconds {
                delay * self.spawn_decay
            } else {
                self.minimum_spawn_seconds
            };
        }
        delay
    }
    pub fn gait(&self, round: u32, random: u32) -> Gait {
        // The native end-of-round callback sets speed before incrementing the round.
        let base = if round <= 1 {
            1
        } else {
            round
                .clamp(1, 255)
                .saturating_sub(1)
                .saturating_mul(self.speed_multiplier)
        };
        let roll = base.saturating_add(random % self.speed_random_band);
        if roll <= self.gait_thresholds[0] {
            Gait::Walk
        } else if roll <= self.gait_thresholds[1] {
            Gait::Run
        } else {
            Gait::Sprint
        }
    }
    pub fn speed(&self, gait: Gait) -> f32 {
        self.gait_speeds[gait as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gait {
    Walk,
    Run,
    Sprint,
}
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopulationKind {
    #[default]
    Ordinary,
    Special,
    GeneratorDefender,
}
#[derive(Resource, Default)]
pub struct Population {
    pub ordinary: usize,
    pub special: usize,
    pub defenders: usize,
}
impl Population {
    pub fn total(&self) -> usize {
        self.ordinary + self.special + self.defenders
    }
}
#[derive(Resource, Default)]
pub struct RoundEvidence {
    pub attempts: u64,
    pub failed_admissions: u64,
    pub budget_consumed_on_failure: u64,
    pub peak_ordinary: usize,
    pub cap_waits: u64,
    pub transitions: u32,
    pub intermission_seconds: f32,
    pub admitted: Vec<serde_json::Value>,
}
