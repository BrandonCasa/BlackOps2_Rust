#![forbid(unsafe_code)]

use bo2_gameplay::{DamageError, LifeState, PlayerLethalPolicy};
use bo2_origins::*;

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
        Err(DamageError::InvalidCurrent)
    );
    assert_eq!(
        unperked_human(0, PlayerLethalPolicy::DownWithEligibleTeammate),
        Err(DamageError::InvalidCurrent)
    );
}
