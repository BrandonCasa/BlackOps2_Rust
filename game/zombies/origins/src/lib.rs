#![forbid(unsafe_code)]

//! Scoped configuration from licensed build24784239 patch-script observations.
//! Current health is chosen by the caller; this does not establish spawn/round state.

use bo2_gameplay::{DamageError, PlayerLethalPolicy, Vitality};

/// Unperked health-reboot maximum, with no persistent health bonus.
pub const UNPERKED_MAXIMUM: u32 = 100;
/// Shared last-stand assignment, not terminal player death.
pub const DOWNED_HEALTH: u32 = 1;
/// Shared spawn fallback only when level health and saved respawn health do not apply.
pub const ACTOR_FALLBACK_HEALTH: u32 = 150;

pub fn unperked_human(current: u32, policy: PlayerLethalPolicy) -> Result<Vitality, DamageError> {
    Vitality::human(current, UNPERKED_MAXIMUM, DOWNED_HEALTH, policy)
}

pub fn fallback_ordinary_actor(current: u32) -> Result<Vitality, DamageError> {
    Vitality::ordinary_actor(current, ACTOR_FALLBACK_HEALTH)
}
