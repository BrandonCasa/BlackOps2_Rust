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
        empty: bool,
    },
    ShellReload {
        remaining: f32,
        phase: ShellPhase,
        inserted: bool,
    },
    Melee {
        remaining: f32,
        hit: bool,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPhase {
    Start,
    Loop,
    End,
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
    pub ads: f32,
    pub recoil: Vec2,
    pending: WeaponIntent,
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
            ads: 0.,
            recoil: Vec2::ZERO,
            pending: WeaponIntent::default(),
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
    ReloadStart,
    ReloadEnd,
    Melee,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct WeaponIntent {
    pub pressed: bool,
    pub held: bool,
    pub reload: bool,
    pub melee: bool,
    pub cycle: bool,
    pub slot: Option<usize>,
    pub ads: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
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
pub fn sample(seed: u64) -> f32 {
    let mut value = seed.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((value ^ (value >> 31)) >> 40) as f32 / (1_u32 << 24) as f32
}
pub fn spread_direction(rotation: Quat, degrees: f32, seed: u64) -> Dir3 {
    let radius = sample(seed).sqrt() * degrees.to_radians().tan();
    let angle = sample(seed.wrapping_add(1)) * std::f32::consts::TAU;
    Dir3::new(rotation * Vec3::new(radius * angle.cos(), radius * angle.sin(), -1.))
        .unwrap_or(Dir3::NEG_Z)
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
    /// An episode-owned weapon has its own ammo pool and never mutates the saved inventory.
    pub fn temporary(definition: &WeaponDefinition, reserve_clips: u32) -> Self {
        let mut inventory = Self::default();
        inventory
            .grant(definition)
            .expect("empty temporary inventory");
        let reserve = definition.magazine.saturating_mul(reserve_clips);
        inventory.ammunition.insert(
            Self::group(definition),
            AmmoPool {
                rounds: reserve,
                capacity: reserve,
            },
        );
        inventory
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
        if self
            .owned()
            .any(|(i, w)| i != slot && w.definition == definition.id)
        {
            return Err(InventoryError::AlreadyOwned);
        }
        if matches!(
            self.action,
            WeaponAction::Reloading { .. } | WeaponAction::ShellReload { .. }
        ) {
            self.cancel_action();
        }
        if self.action != WeaponAction::Ready {
            return Err(InventoryError::Busy);
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
    pub fn refill_weapon(&mut self, definition: &WeaponDefinition) -> bool {
        let Some(pool) = self.ammunition.get_mut(&Self::group(definition)) else {
            return false;
        };
        if pool.rounds >= pool.capacity {
            return false;
        }
        pool.rounds = pool.capacity;
        true
    }
    pub fn receive(&mut self, definition: &WeaponDefinition) -> Result<usize, InventoryError> {
        if self.action != WeaponAction::Ready {
            return Err(InventoryError::Busy);
        }
        let slot = match self.grant(definition) {
            Ok(slot) => slot,
            Err(InventoryError::Full) => {
                self.replace(self.active, definition)?;
                self.active
            }
            Err(error) => return Err(error),
        };
        self.active = slot;
        self.cooldown = 0.;
        self.burst = 0;
        self.action = WeaponAction::Equipping {
            remaining: definition.mechanics.equip_seconds,
        };
        self.emit(WeaponSignal::Equip);
        Ok(slot)
    }
    pub fn cancel_action(&mut self) {
        self.action = WeaponAction::Ready;
        self.burst = 0;
        self.pending = WeaponIntent::default();
        self.emit(WeaponSignal::Idle);
    }
    pub fn recoil_impulse(&mut self, definition: &WeaponDefinition, seed: u64) {
        let yaw = definition.mechanics.recoil_yaw;
        let pitch = definition.mechanics.recoil_pitch;
        self.recoil += Vec2::new(
            yaw[0] + (yaw[1] - yaw[0]) * sample(seed),
            pitch[0] + (pitch[1] - pitch[0]) * sample(seed.wrapping_add(1)),
        )
        .map(f32::to_radians);
        self.recoil = self.recoil.clamp(Vec2::splat(-0.4), Vec2::splat(0.4));
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
        mut intent: WeaponIntent,
        definitions: &[WeaponDefinition],
    ) -> Vec<WeaponEmission> {
        let mut emissions = Vec::new();
        if self.action == WeaponAction::Ready {
            intent.pressed |= self.pending.pressed;
            intent.melee |= self.pending.melee;
            intent.cycle |= self.pending.cycle;
            intent.slot = intent.slot.or(self.pending.slot);
            self.pending = WeaponIntent::default();
        }
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
        let aiming =
            intent.ads && definition.mechanics.ads_enabled && self.action == WeaponAction::Ready;
        let target = if aiming { 1. } else { 0. };
        let step = dt / definition.mechanics.ads_seconds[if aiming { 0 } else { 1 }];
        self.ads = if self.ads < target {
            (self.ads + step).min(target)
        } else {
            (self.ads - step).max(target)
        };
        let recovery = definition.mechanics.recoil_recovery.to_radians() * dt;
        self.recoil = self
            .recoil
            .map(|value| value.signum() * (value.abs() - recovery).max(0.));
        let melee_definition = definitions
            .iter()
            .find(|w| w.id.0 == "weapon.knife_zm")
            .unwrap_or(definition);
        match self.action {
            WeaponAction::Ready => {}
            WeaponAction::Melee { remaining, mut hit } => {
                let remaining = (remaining - dt).max(0.);
                if !hit
                    && melee_definition.mechanics.melee_seconds[1]
                        .max(melee_definition.mechanics.melee_seconds[0])
                        - remaining
                        >= melee_definition.mechanics.melee_seconds[0]
                {
                    emissions.push(WeaponEmission::Melee);
                    hit = true;
                }
                if remaining > 0. {
                    self.action = WeaponAction::Melee { remaining, hit };
                    return emissions;
                }
                self.action = WeaponAction::Ready;
                self.emit(WeaponSignal::Idle);
            }
            WeaponAction::Equipping { remaining } => {
                if remaining > dt {
                    self.action = WeaponAction::Equipping {
                        remaining: remaining - dt,
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
            WeaponAction::Reloading { remaining, empty } => {
                let remaining = (remaining - dt).max(0.);
                if remaining > 0. {
                    self.action = WeaponAction::Reloading { remaining, empty };
                    return emissions;
                }
                self.insert(definition, u32::MAX);
                self.action = WeaponAction::Ready;
                self.emit(WeaponSignal::Idle);
            }
            WeaponAction::ShellReload {
                remaining,
                phase,
                mut inserted,
            } => {
                let interrupt = intent.cycle
                    || intent.slot.is_some()
                    || intent.melee
                    || ((intent.pressed || intent.held)
                        && self.active().is_some_and(|w| w.magazine > 0));
                if phase != ShellPhase::End && interrupt {
                    self.pending = intent;
                    self.pending.pressed |= intent.held;
                    self.action = WeaponAction::ShellReload {
                        remaining: definition.mechanics.reload_end_seconds,
                        phase: ShellPhase::End,
                        inserted: true,
                    };
                    self.emit(WeaponSignal::ReloadEnd);
                    return emissions;
                }
                let remaining = (remaining - dt).max(0.);
                let duration = if phase == ShellPhase::Start {
                    definition.mechanics.reload_start_seconds
                } else {
                    definition.reload_seconds
                };
                let at = if phase == ShellPhase::Start {
                    definition.mechanics.reload_start_insert
                } else {
                    definition.mechanics.shell_seconds
                };
                if phase != ShellPhase::End && !inserted && duration - remaining >= at.min(duration)
                {
                    self.insert(definition, 1);
                    inserted = true;
                }
                if remaining > 0. {
                    self.action = WeaponAction::ShellReload {
                        remaining,
                        phase,
                        inserted,
                    };
                    return emissions;
                }
                if phase == ShellPhase::End {
                    self.action = WeaponAction::Ready;
                    self.emit(WeaponSignal::Idle);
                    return emissions;
                }
                if self.reserve(definition) > 0
                    && self
                        .active()
                        .is_some_and(|w| w.magazine < definition.magazine)
                {
                    self.action = WeaponAction::ShellReload {
                        remaining: definition.reload_seconds,
                        phase: ShellPhase::Loop,
                        inserted: false,
                    };
                    self.emit(WeaponSignal::ShellInsert);
                } else {
                    self.action = WeaponAction::ShellReload {
                        remaining: definition.mechanics.reload_end_seconds,
                        phase: ShellPhase::End,
                        inserted: true,
                    };
                    self.emit(WeaponSignal::ReloadEnd);
                }
                return emissions;
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
        if intent.melee {
            self.burst = 0;
            self.action = WeaponAction::Melee {
                remaining: melee_definition.mechanics.melee_seconds[1]
                    .max(melee_definition.mechanics.melee_seconds[0]),
                hit: false,
            };
            self.emit(WeaponSignal::Melee);
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
                    remaining: definition.mechanics.reload_start_seconds,
                    phase: ShellPhase::Start,
                    inserted: definition.mechanics.reload_start_insert
                        > definition.mechanics.reload_start_seconds,
                }
            } else {
                WeaponAction::Reloading {
                    remaining: if empty {
                        definition.mechanics.empty_reload
                    } else {
                        definition.reload_seconds
                    },
                    empty,
                }
            };
            self.emit(if definition.mechanics.shell_reload {
                WeaponSignal::ReloadStart
            } else if empty {
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
                self.cooldown = definition.fire_interval
                    + if matches!(definition.mechanics.fire, FirePattern::Burst { .. })
                        && self.burst == 0
                    {
                        definition.mechanics.burst_delay
                    } else {
                        0.
                    };
                self.emit(WeaponSignal::Fire);
                emissions.push(WeaponEmission::Shot(definition.id.clone()));
            }
        }
        emissions
    }
}
