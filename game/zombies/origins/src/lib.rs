#![forbid(unsafe_code)]

//! Scoped configuration from licensed build24784239 patch-script observations.
//! Current health is chosen by the caller; this does not establish spawn/round state.

use std::time::Duration;

use bo2_gameplay::{
    BleedoutClockError, BleedoutTiming, LifeStateError, PlayerLethalPolicy, ReviveClockError,
    ReviveRestoration, ReviveTiming, Vitality,
};

/// Unperked health-reboot maximum, with no persistent health bonus.
pub const UNPERKED_MAXIMUM: u32 = 100;
/// Shared last-stand assignment, not terminal player death.
pub const DOWNED_HEALTH: u32 = 1;
/// Shared spawn fallback only when level health and saved respawn health do not apply.
pub const ACTOR_FALLBACK_HEALTH: u32 = 150;

/// Ordinary no-bonus coop completion: Steam build24784239 script plus qualified
/// observed Plutonium native setters (task04). Live comparison remains pending.
pub fn ordinary_coop_revive_restoration() -> Result<ReviveRestoration, LifeStateError> {
    ReviveRestoration::new(UNPERKED_MAXIMUM, UNPERKED_MAXIMUM)
}

pub fn unperked_human(
    current: u32,
    policy: PlayerLethalPolicy,
) -> Result<Vitality, LifeStateError> {
    Vitality::human(current, UNPERKED_MAXIMUM, DOWNED_HEALTH, policy)
}

pub fn fallback_ordinary_actor(current: u32) -> Result<Vitality, LifeStateError> {
    Vitality::ordinary_actor(current, ACTOR_FALLBACK_HEALTH)
}

/// Shared source default sampled once for admitted ordinary coop, build24784239.
/// Runtime dvar equivalence and multiplayer timing comparison remain unqualified.
pub fn ordinary_coop_bleedout_timing() -> Result<BleedoutTiming, BleedoutClockError> {
    BleedoutTiming::new(Duration::from_secs(45), Duration::from_millis(100))
}

/// Ideal Duration adaptation of source03's3s threshold and0.05s waits.
/// Exact original float accumulation/VM step count and live coop timing unqualified.
pub fn ordinary_coop_revive_timing() -> Result<ReviveTiming, ReviveClockError> {
    ReviveTiming::new(Duration::from_secs(3), Duration::from_millis(50))
}
