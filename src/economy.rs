use bevy::prelude::*;
use std::collections::BTreeMap;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransactionId(pub u64);
#[derive(Clone, Copy, Debug)]
pub struct Purchase {
    pub id: TransactionId,
    pub amount: u32,
}
#[derive(Debug)]
pub enum PurchaseError<E> {
    InsufficientPoints,
    Duplicate,
    Rejected(E),
}
#[derive(Resource, Default)]
pub struct PointLedger {
    pub spent: u64,
    next: u64,
    debits: BTreeMap<TransactionId, u32>,
}
impl PointLedger {
    pub fn command(&mut self, amount: u32) -> Purchase {
        self.next = self.next.wrapping_add(1);
        Purchase {
            id: TransactionId(self.next),
            amount,
        }
    }
    pub fn accept<T, E>(
        &mut self,
        balance: &mut u32,
        purchase: Purchase,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, PurchaseError<E>> {
        if self.debits.contains_key(&purchase.id) {
            return Err(PurchaseError::Duplicate);
        }
        if *balance < purchase.amount {
            return Err(PurchaseError::InsufficientPoints);
        }
        let item = operation().map_err(PurchaseError::Rejected)?;
        *balance -= purchase.amount;
        self.spent = self.spent.saturating_add(u64::from(purchase.amount));
        self.debits.insert(purchase.id, purchase.amount);
        Ok(item)
    }
    pub fn refund(&mut self, balance: &mut u32, id: TransactionId) -> bool {
        let Some(amount) = self.debits.get_mut(&id) else {
            return false;
        };
        if *amount == 0 {
            return false;
        }
        *balance = balance.saturating_add(*amount);
        *amount = 0;
        true
    }
}
