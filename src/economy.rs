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
