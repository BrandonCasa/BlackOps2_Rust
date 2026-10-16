use bevy::prelude::*;

pub(crate) fn source_shuffle<T>(items: &mut [T], session: &mut crate::gameplay::Session) {
    for index in 0..items.len() {
        let other = session.random() as usize % items.len();
        items.swap(index, other);
    }
}

pub(crate) fn record_downs(
    mut downs: MessageReader<crate::solo_revive::PlayerDowned>,
    mut deaths: MessageReader<crate::player_health::PlayerDeath>,
    players: Query<(Entity, &crate::life::Vitality), With<crate::player::Player>>,
    mut session: ResMut<crate::gameplay::Session>,
    mut counted: Local<Option<Entity>>,
) {
    if players
        .iter()
        .any(|(_, health)| health.state() == crate::life::LifeState::Standing)
    {
        *counted = None;
    }
    for target in downs
        .read()
        .map(|event| event.target)
        .chain(deaths.read().map(|event| event.target))
    {
        if players.contains(target) && *counted != Some(target) {
            session.downs = session.downs.saturating_add(1);
            // _zm_score::player_downed_penalty: truncate 5%, then round up to ten.
            let penalty = (session.points / 20).div_ceil(10) * 10;
            session.points = session.points.saturating_sub(penalty);
            *counted = Some(target);
        }
    }
}

impl crate::gameplay::Session {
    pub(crate) fn purchase<T>(
        &mut self,
        amount: u32,
        operation: impl FnOnce() -> Result<T, crate::interactions::InteractionRejection>,
    ) -> Result<T, crate::interactions::InteractionRejection> {
        if self.points < amount {
            return Err(crate::interactions::InteractionRejection::InsufficientPoints);
        }
        let item = operation()?;
        self.points -= amount;
        self.spent = self.spent.saturating_add(u64::from(amount));
        Ok(item)
    }
}

#[cfg(test)]
#[test]
fn native_down_penalty_rounds_up_once_without_changing_earned_or_spent_points() {
    use crate::{life::*, player::Player, player_health::PlayerDeath, solo_revive::PlayerDowned};
    let mut world = World::new();
    world.init_resource::<crate::gameplay::Session>();
    world.init_resource::<Messages<PlayerDowned>>();
    world.init_resource::<Messages<PlayerDeath>>();
    let mut health = Vitality::human(100, 100, 1, PlayerLethalPolicy::SoloSelfRevive).unwrap();
    let DamageOutcome::Downed { episode, .. } =
        health.apply(ResolvedDamage::new(100).unwrap()).unwrap()
    else {
        panic!("solo down")
    };
    let player = world
        .spawn((
            Player {
                yaw: 0.,
                pitch: 0.,
                velocity: Vec3::ZERO,
                height: 1.8,
                grounded: true,
                crouched: false,
                prone: false,
                eye_height: 1.62,
            },
            health,
        ))
        .id();
    let system = world.register_system(record_downs);
    world.write_message(PlayerDowned {
        target: player,
        attacker: player,
        episode,
    });
    world.run_system(system).unwrap();
    let session = world.resource::<crate::gameplay::Session>();
    assert_eq!(
        (session.points, session.score, session.spent, session.downs),
        (470, 500, 0, 1)
    );
    world.write_message(PlayerDeath {
        target: player,
        attacker: player,
    });
    world.run_system(system).unwrap();
    assert_eq!(world.resource::<crate::gameplay::Session>().points, 470);
    assert_eq!(world.resource::<crate::gameplay::Session>().downs, 1);
}
