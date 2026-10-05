#![forbid(unsafe_code)]

use bo2_gameplay::{LifeState, LifeStateError, PlayerLethalPolicy};
use bo2_origins::*;

#[test]
fn coop_restoration_uses_qualified_health_instead_of_downed_health() {
    let mut player = unperked_human(100, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
    player
        .apply(bo2_gameplay::ResolvedDamage::new(100).unwrap())
        .unwrap();
    assert_eq!(player.current(), Some(1));
    assert_eq!(
        ordinary_coop_revive_restoration(),
        bo2_gameplay::ReviveRestoration::new(100, 100)
    );
}

#[test]
fn configuration_preserves_explicit_current_health() {
    let human = unperked_human(40, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
    assert_eq!(
        (human.current(), human.maximum(), human.state()),
        (Some(40), 100, LifeState::Standing)
    );
    let actor = fallback_ordinary_actor(70).unwrap();
    assert_eq!((actor.current(), actor.maximum()), (Some(70), 150));
    assert_eq!(
        fallback_ordinary_actor(151),
        Err(LifeStateError::InvalidCurrent)
    );
    assert_eq!(
        unperked_human(0, PlayerLethalPolicy::DownWithEligibleTeammate),
        Err(LifeStateError::InvalidCurrent)
    );
}

#[test]
fn ordinary_coop_source_timing_is_sampled45_seconds_and100_millisecond_poll() {
    let timing = bo2_origins::ordinary_coop_bleedout_timing().unwrap();
    assert_eq!(timing.countdown(), std::time::Duration::from_secs(45));
    assert_eq!(
        timing.poll_interval(),
        std::time::Duration::from_millis(100)
    );
}

#[test]
fn ordinary_revive_timing_is_explicit_ideal_duration_adaptation() {
    let timing = bo2_origins::ordinary_coop_revive_timing().unwrap();
    assert_eq!(timing.duration(), std::time::Duration::from_secs(3));
    assert_eq!(timing.interval(), std::time::Duration::from_millis(50));
}
