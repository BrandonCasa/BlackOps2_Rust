//! Owned weapons and fixed-step action timing, independent of map configuration.
use crate::origins::content::{ContentId, FirePattern, WeaponDefinition, WeaponId};
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct OwnedWeapon {
    pub definition: WeaponId,
    pub magazine: u32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeaponAction {
    Ready,
    Equipping {
        remaining: f32,
    },
    Unequipping {
        remaining: f32,
        target: usize,
    },
    Reloading {
        remaining: f32,
        inserted: bool,
        empty: bool,
    },
    ShellReload {
        remaining: f32,
    },
    Melee {
        remaining: f32,
    },
}
#[derive(Clone, Default)]
struct AmmoPool {
    rounds: u32,
    capacity: u32,
}
#[derive(Component, Clone)]
pub struct WeaponInventory {
    slots: [Option<OwnedWeapon>; 3],
    active: usize,
    capacity: usize,
    ammunition: BTreeMap<ContentId, AmmoPool>,
    pub action: WeaponAction,
    cooldown: f32,
    burst: u32,
    pub sequence: u64,
    pub signal: WeaponSignal,
}
impl Default for WeaponInventory {
    fn default() -> Self {
        Self {
            slots: [None, None, None],
            active: 0,
            capacity: 2,
            ammunition: BTreeMap::new(),
            action: WeaponAction::Ready,
            cooldown: 0.,
            burst: 0,
            sequence: 0,
            signal: WeaponSignal::Idle,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeaponSignal {
    #[default]
    Idle,
    Equip,
    Unequip,
    Fire,
    DryFire,
    Reload,
    ReloadEmpty,
    ShellInsert,
    Melee,
}
#[derive(Clone, Copy, Default)]
pub struct WeaponIntent {
    pub pressed: bool,
    pub held: bool,
    pub reload: bool,
    pub melee: bool,
    pub cycle: bool,
    pub slot: Option<usize>,
}
#[derive(Clone, Debug)]
pub enum InventoryError {
    Full,
    AlreadyOwned,
    InvalidSlot,
    MissingDefinition,
    Busy,
}
#[derive(Clone)]
pub enum WeaponEmission {
    Shot(WeaponId),
    Melee,
}

#[derive(Clone, Copy)]
pub enum HitRegion {
    Head,
    UpperTorso,
    Torso,
    Arm,
    Leg,
}
pub fn damage_at(definition: &WeaponDefinition, distance: f32, region: HitRegion) -> u32 {
    let [near, far] = definition.mechanics.damage_range;
    let fraction = if far > near {
        ((distance - near) / (far - near)).clamp(0., 1.)
    } else {
        0.
    };
    let damage = definition.damage as f32
        + (definition.mechanics.minimum_damage as f32 - definition.damage as f32) * fraction;
    let index = match region {
        HitRegion::Head => 0,
        HitRegion::UpperTorso => 1,
        HitRegion::Torso => 2,
        HitRegion::Arm => 3,
        HitRegion::Leg => 4,
    };
    (damage * definition.mechanics.hit_multipliers[index])
        .round()
        .max(1.) as u32
}

impl WeaponInventory {
    pub fn active(&self) -> Option<&OwnedWeapon> {
        self.slots[self.active].as_ref()
    }
    pub fn active_slot(&self) -> usize {
        self.active
    }
    pub fn owned(&self) -> impl Iterator<Item = (usize, &OwnedWeapon)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, weapon)| weapon.as_ref().map(|w| (slot, w)))
    }
    pub fn definition<'a>(
        &self,
        definitions: &'a [WeaponDefinition],
    ) -> Option<&'a WeaponDefinition> {
        let weapon = self.active()?;
        definitions.iter().find(|d| d.id == weapon.definition)
    }
    fn group(definition: &WeaponDefinition) -> ContentId {
        definition
            .ammo_group
            .clone()
            .unwrap_or_else(|| ContentId(definition.id.0.clone()))
    }
    pub fn reserve(&self, definition: &WeaponDefinition) -> u32 {
        self.ammunition
            .get(&Self::group(definition))
            .map_or(0, |p| p.rounds)
    }
    pub fn grant(&mut self, definition: &WeaponDefinition) -> Result<usize, InventoryError> {
        if self.owned().any(|(_, w)| w.definition == definition.id) {
            return Err(InventoryError::AlreadyOwned);
        }
        let slot = self.slots[..self.capacity]
            .iter()
            .position(Option::is_none)
            .ok_or(InventoryError::Full)?;
        let pool = self.ammunition.entry(Self::group(definition)).or_default();
        pool.capacity = pool.capacity.max(definition.reserve);
        pool.rounds = pool
            .rounds
            .max(definition.mechanics.starting_reserve)
            .min(pool.capacity);
        self.slots[slot] = Some(OwnedWeapon {
            definition: definition.id.clone(),
            magazine: definition.magazine,
        });
        Ok(slot)
    }
    pub fn replace(
        &mut self,
        slot: usize,
        definition: &WeaponDefinition,
    ) -> Result<(), InventoryError> {
        if slot >= self.capacity {
            return Err(InventoryError::InvalidSlot);
        }
        if self.action != WeaponAction::Ready {
            return Err(InventoryError::Busy);
        }
        if self
            .owned()
            .any(|(i, w)| i != slot && w.definition == definition.id)
        {
            return Err(InventoryError::AlreadyOwned);
        }
        self.slots[slot] = Some(OwnedWeapon {
            definition: definition.id.clone(),
            magazine: definition.magazine,
        });
        let pool = self.ammunition.entry(Self::group(definition)).or_default();
        pool.capacity = pool.capacity.max(definition.reserve);
        pool.rounds = pool
            .rounds
            .max(definition.mechanics.starting_reserve)
            .min(pool.capacity);
        if slot == self.active {
            self.action = WeaponAction::Equipping {
                remaining: definition.mechanics.equip_seconds,
            };
            self.emit(WeaponSignal::Equip);
        }
        Ok(())
    }
    /// The third slot is always the perk-owned slot; loss removes that instance only.
    pub fn set_third_slot(&mut self, enabled: bool) {
        self.capacity = if enabled { 3 } else { 2 };
        if !enabled {
            self.slots[2] = None;
            if self.active == 2 {
                self.active = self.slots[..2]
                    .iter()
                    .position(Option::is_some)
                    .unwrap_or(0);
            }
            self.cancel_action();
        }
    }
    pub fn refill(&mut self, definitions: &[WeaponDefinition]) {
        for definition in definitions {
            if self.owned().any(|(_, w)| w.definition == definition.id) {
                let pool = self.ammunition.entry(Self::group(definition)).or_default();
                pool.capacity = pool.capacity.max(definition.reserve);
                pool.rounds = pool.capacity;
            }
        }
    }
    pub fn cancel_action(&mut self) {
        self.action = WeaponAction::Ready;
        self.burst = 0;
        self.emit(WeaponSignal::Idle);
    }
    fn emit(&mut self, signal: WeaponSignal) {
        self.signal = signal;
        self.sequence = self.sequence.wrapping_add(1);
    }
    fn insert(&mut self, definition: &WeaponDefinition, amount: u32) {
        let Some(weapon) = self.slots[self.active].as_mut() else {
            return;
        };
        let Some(pool) = self.ammunition.get_mut(&Self::group(definition)) else {
            return;
        };
        let transfer = definition
            .magazine
            .saturating_sub(weapon.magazine)
            .min(amount)
            .min(pool.rounds);
        weapon.magazine += transfer;
        pool.rounds -= transfer;
    }
    pub fn select(
        &mut self,
        slot: usize,
        definitions: &[WeaponDefinition],
    ) -> Result<(), InventoryError> {
        if slot >= self.capacity || self.slots[slot].is_none() {
            return Err(InventoryError::InvalidSlot);
        }
        if slot == self.active {
            return Ok(());
        }
        if self.action != WeaponAction::Ready {
            return Err(InventoryError::Busy);
        }
        let definition = self
            .definition(definitions)
            .ok_or(InventoryError::MissingDefinition)?;
        self.burst = 0;
        self.action = WeaponAction::Unequipping {
            remaining: definition.mechanics.unequip_seconds,
            target: slot,
        };
        self.emit(WeaponSignal::Unequip);
        Ok(())
    }
    pub fn tick(
        &mut self,
        dt: f32,
        intent: WeaponIntent,
        definitions: &[WeaponDefinition],
    ) -> Vec<WeaponEmission> {
        let mut emissions = Vec::new();
        let was_cooling = self.cooldown > 0.;
        self.cooldown = (self.cooldown - dt).max(0.);
        if was_cooling
            && self.cooldown == 0.
            && self.burst == 0
            && self.action == WeaponAction::Ready
            && matches!(self.signal, WeaponSignal::Fire | WeaponSignal::DryFire)
        {
            self.emit(WeaponSignal::Idle);
        }
        let Some(definition) = self.definition(definitions) else {
            return emissions;
        };
        match self.action {
            WeaponAction::Ready => {}
            WeaponAction::Equipping { remaining } | WeaponAction::Melee { remaining } => {
                if remaining > dt {
                    self.action = match self.action {
                        WeaponAction::Melee { .. } => WeaponAction::Melee {
                            remaining: remaining - dt,
                        },
                        _ => WeaponAction::Equipping {
                            remaining: remaining - dt,
                        },
                    };
                    return emissions;
                }
                self.action = WeaponAction::Ready;
                self.emit(WeaponSignal::Idle);
            }
            WeaponAction::Unequipping { remaining, target } => {
                if remaining > dt {
                    self.action = WeaponAction::Unequipping {
                        remaining: remaining - dt,
                        target,
                    };
                } else if let Some(next) = self.slots[target]
                    .as_ref()
                    .and_then(|w| definitions.iter().find(|d| d.id == w.definition))
                {
                    self.active = target;
                    self.cooldown = 0.;
                    self.action = WeaponAction::Equipping {
                        remaining: next.mechanics.equip_seconds,
                    };
                    self.emit(WeaponSignal::Equip);
                } else {
                    self.cancel_action();
                }
                return emissions;
            }
            WeaponAction::Reloading {
                remaining,
                mut inserted,
                empty,
            } => {
                let duration = if empty {
                    definition.mechanics.empty_reload
                } else {
                    definition.reload_seconds
                };
                let remaining = (remaining - dt).max(0.);
                if !inserted
                    && duration - remaining >= definition.mechanics.shell_seconds.min(duration)
                {
                    self.insert(definition, u32::MAX);
                    inserted = true;
                }
                if remaining > 0. {
                    self.action = WeaponAction::Reloading {
                        remaining,
                        inserted,
                        empty,
                    };
                    return emissions;
                }
                self.action = WeaponAction::Ready;
                self.emit(WeaponSignal::Idle);
            }
            WeaponAction::ShellReload { remaining } => {
                if (intent.pressed || intent.melee || intent.cycle || intent.slot.is_some())
                    && self.active().is_some_and(|w| w.magazine > 0)
                {
                    self.action = WeaponAction::Ready;
                    self.emit(WeaponSignal::Idle);
                } else if remaining > dt {
                    self.action = WeaponAction::ShellReload {
                        remaining: remaining - dt,
                    };
                    return emissions;
                } else {
                    self.insert(definition, 1);
                    self.emit(WeaponSignal::ShellInsert);
                    if self.reserve(definition) > 0
                        && self
                            .active()
                            .is_some_and(|w| w.magazine < definition.magazine)
                    {
                        self.action = WeaponAction::ShellReload {
                            remaining: definition.reload_seconds,
                        };
                        return emissions;
                    }
                    self.action = WeaponAction::Ready;
                    self.emit(WeaponSignal::Idle);
                }
            }
        }
        if intent.cycle {
            if let Some(slot) = (1..=self.capacity)
                .map(|n| (self.active + n) % self.capacity)
                .find(|&i| self.slots[i].is_some())
            {
                let _ = self.select(slot, definitions);
            }
        } else if let Some(slot) = intent.slot {
            let _ = self.select(slot, definitions);
        }
        if self.action != WeaponAction::Ready {
            return emissions;
        }
        if intent.melee && self.cooldown == 0. {
            self.burst = 0;
            self.action = WeaponAction::Melee {
                remaining: definition.mechanics.melee_seconds[1].max(0.5),
            };
            self.emit(WeaponSignal::Melee);
            emissions.push(WeaponEmission::Melee);
            return emissions;
        }
        if intent.reload
            && self.reserve(definition) > 0
            && self
                .active()
                .is_some_and(|w| w.magazine < definition.magazine)
        {
            self.burst = 0;
            let empty = self.active().is_some_and(|w| w.magazine == 0);
            self.action = if definition.mechanics.shell_reload {
                WeaponAction::ShellReload {
                    remaining: definition.reload_seconds,
                }
            } else {
                WeaponAction::Reloading {
                    remaining: if empty {
                        definition.mechanics.empty_reload
                    } else {
                        definition.reload_seconds
                    },
                    inserted: false,
                    empty,
                }
            };
            self.emit(if empty {
                WeaponSignal::ReloadEmpty
            } else {
                WeaponSignal::Reload
            });
            return emissions;
        }
        if intent.pressed
            && let FirePattern::Burst { shots } = definition.mechanics.fire
            && self.burst == 0
            && self.cooldown == 0.
        {
            self.burst = shots;
        }
        let firing = match definition.mechanics.fire {
            FirePattern::Semi => intent.pressed,
            FirePattern::Automatic => intent.held || intent.pressed,
            FirePattern::Burst { .. } => self.burst > 0,
            FirePattern::Charge => false, // Staff charge effects own this action in their dedicated system.
        };
        if firing && self.cooldown == 0. {
            let Some(weapon) = self.slots[self.active].as_mut() else {
                return emissions;
            };
            if weapon.magazine == 0 {
                self.burst = 0;
                self.cooldown = 0.18;
                self.emit(WeaponSignal::DryFire);
            } else {
                weapon.magazine -= 1;
                self.burst = self.burst.saturating_sub(1);
                self.cooldown = definition.fire_interval;
                self.emit(WeaponSignal::Fire);
                emissions.push(WeaponEmission::Shot(definition.id.clone()));
            }
        }
        emissions
    }
}
