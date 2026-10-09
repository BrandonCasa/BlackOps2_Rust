use bevy::prelude::*;
use std::collections::BTreeMap;

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
