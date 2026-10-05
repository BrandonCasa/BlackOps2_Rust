#![forbid(unsafe_code)]

use bo2_gameplay::{LifeState, LifeStateError, PlayerLethalPolicy};
use bo2_origins::*;

#[test]
fn coop_restoration_uses_qualified_health_instead_of_downed_health() {
    let mut player = unperked_human(100, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
    player
        .apply(bo2_gameplay::ResolvedDamage::new(100).unwrap())
        .unwrap();
    assert_eq!(player.current(), 1);
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
        (40, 100, LifeState::Standing)
    );
    let actor = fallback_ordinary_actor(70).unwrap();
    assert_eq!((actor.current(), actor.maximum()), (70, 150));
    assert_eq!(
        fallback_ordinary_actor(151),
        Err(LifeStateError::InvalidCurrent)
    );
    assert_eq!(
        unperked_human(0, PlayerLethalPolicy::DownWithEligibleTeammate),
        Err(LifeStateError::InvalidCurrent)
    );
}
