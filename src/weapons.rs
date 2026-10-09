use crate::origins::content::{ContentId, FirePattern, WeaponDefinition, WeaponId};
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct OwnedWeapon {
    pub definition: WeaponId,
    pub magazine: u32,
    pub left_magazine: Option<u32>,
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
#[derive(Clone, Default)]
struct WeaponHand {
    cooldown: f32,
    reload: Option<(f32, f32)>,
    signal: WeaponSignal,
    sequence: u64,
    shot_elapsed: Option<f32>,
}
#[derive(Component, Clone)]
pub struct WeaponInventory {
    slots: [Option<OwnedWeapon>; 3],
    active: usize,
    capacity: usize,
    ammunition: BTreeMap<ContentId, AmmoPool>,
    pub action: WeaponAction,
    cooldown: f32,
    hands: [WeaponHand; 2],
    next_hand: usize,
    burst: u32,
    pub sequence: u64,
    pub signal: WeaponSignal,
    shot_elapsed: Option<f32>,
    pub ads: f32,
    ads_leaving: bool,
    pub recoil: Vec2,
    pending: WeaponIntent,
    reload_duration_scale: f32,
    pub firing: crate::origins::content::PerkFiringDefinition,
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
            hands: default(),
            next_hand: 0,
            burst: 0,
            sequence: 0,
            signal: WeaponSignal::Idle,
            shot_elapsed: None,
            ads: 0.,
            ads_leaving: false,
            recoil: Vec2::ZERO,
            pending: WeaponIntent::default(),
            reload_duration_scale: 1.,
            firing: default(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeaponSignal {
    Prime,
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
    ReloadStarted {
        weapon: WeaponId,
        magazine: u32,
        capacity: u32,
        seconds: f32,
    },
    ReloadCompleted(WeaponId),
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
    pub fn ads_zoom(&self, mechanics: &crate::origins::content::WeaponMechanics) -> f32 {
        let fraction = mechanics.ads_zoom_fraction[usize::from(self.ads_leaving)];
        if fraction <= 0. {
            return if self.ads >= 1. { 1. } else { 0. };
        }
        ((self.ads - (1. - fraction)) / fraction).clamp(0., 1.)
    }
    pub fn ads_fov(
        &self,
        hip_vertical: f32,
        mechanics: &crate::origins::content::WeaponMechanics,
    ) -> f32 {
        let horizontal = |vertical: f32| 2. * (vertical.to_radians() * 0.5).tan().atan2(0.75);
        let hip = horizontal(hip_vertical);
        let aimed = horizontal(mechanics.ads_fov);
        2. * (((hip + (aimed - hip) * self.ads_zoom(mechanics)) * 0.5).tan() * 0.75).atan()
    }
    pub fn set_reload_duration_scale(&mut self, scale: f32) {
        if scale.is_finite() && scale > 0. && scale <= 1. {
            self.reload_duration_scale = scale;
        }
    }
    pub fn reload_duration_scale(&self) -> f32 {
        self.reload_duration_scale
    }
    pub fn reload_timing(&self, definition: &WeaponDefinition) -> Option<(f32, f32)> {
        match self.action {
            WeaponAction::Reloading { remaining, empty } => Some((
                if empty {
                    definition.mechanics.empty_reload
                } else {
                    definition.reload_seconds
                },
                remaining,
            )),
            WeaponAction::ShellReload {
                remaining, phase, ..
            } => Some((
                match phase {
                    ShellPhase::Start => definition.mechanics.reload_start_seconds,
                    ShellPhase::Loop => definition.reload_seconds,
                    ShellPhase::End => definition.mechanics.reload_end_seconds,
                },
                remaining,
            )),
            _ => None,
        }
        .filter(|(duration, _)| *duration > 0.)
    }
    pub fn shot_elapsed(&self) -> Option<f32> {
        self.shot_elapsed
    }
    pub(crate) fn survey_signal(&mut self, signal: WeaponSignal) {
        self.emit(signal);
        for hand in &mut self.hands {
            hand.signal = signal;
            hand.sequence = self.sequence;
        }
    }
    pub(crate) fn survey_shot_pose(&mut self, elapsed: f32, empty: bool) {
        self.shot_elapsed = Some(elapsed);
        self.signal = WeaponSignal::Fire;
        self.hands[0].shot_elapsed = Some(elapsed);
        self.hands[0].signal = WeaponSignal::Fire;
        if let Some(weapon) = &mut self.slots[self.active] {
            weapon.magazine = if empty { 0 } else { 2 };
        }
    }
    pub fn presentation_timing(
        &self,
        definition: &WeaponDefinition,
        melee: &WeaponDefinition,
    ) -> Option<(f32, f32)> {
        self.reload_timing(definition).or_else(|| {
            match self.action {
                WeaponAction::Equipping { remaining } => {
                    Some((definition.mechanics.equip_seconds, remaining))
                }
                WeaponAction::Unequipping { remaining, .. } => {
                    Some((definition.mechanics.unequip_seconds, remaining))
                }
                WeaponAction::Melee { remaining, .. } => Some((
                    melee.mechanics.melee_seconds[1].max(melee.mechanics.melee_seconds[0]),
                    remaining,
                )),
                _ => None,
            }
            .filter(|(duration, _)| *duration > 0.)
        })
    }
    pub fn active(&self) -> Option<&OwnedWeapon> {
        self.slots[self.active].as_ref()
    }
    pub fn active_slot(&self) -> usize {
        self.active
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn perk_weapon(&self) -> Option<&OwnedWeapon> {
        (self.capacity == 3)
            .then(|| self.slots[2].as_ref())
            .flatten()
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
            left_magazine: definition.dual_weapon.as_ref().map(|_| definition.magazine),
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
            left_magazine: definition.dual_weapon.as_ref().map(|_| definition.magazine),
        });
        let pool = self.ammunition.entry(Self::group(definition)).or_default();
        pool.capacity = pool.capacity.max(definition.reserve);
        pool.rounds = pool
            .rounds
            .max(definition.mechanics.starting_reserve)
            .min(pool.capacity);
        if slot == self.active {
            self.hands = default();
            self.next_hand = 0;
            self.action = WeaponAction::Equipping {
                remaining: definition.mechanics.equip_seconds,
            };
            self.emit(WeaponSignal::Equip);
        }
        Ok(())
    }
    pub fn set_third_slot(
        &mut self,
        enabled: bool,
        definitions: &[WeaponDefinition],
    ) -> Option<OwnedWeapon> {
        if (self.capacity == 3) == enabled {
            return None;
        }
        self.capacity = if enabled { 3 } else { 2 };
        if !enabled {
            let removed = self.slots[2].take();
            if let Some(definition) = removed
                .as_ref()
                .and_then(|w| definitions.iter().find(|d| d.id == w.definition))
            {
                let group = Self::group(definition);
                let shared = self
                    .owned()
                    .filter_map(|(_, w)| definitions.iter().find(|d| d.id == w.definition))
                    .any(|d| Self::group(d) == group);
                if !shared {
                    self.ammunition.remove(&group);
                }
            }
            if self.active == 2 {
                self.active = self.slots[..2]
                    .iter()
                    .position(Option::is_some)
                    .unwrap_or(0);
            }
            self.cancel_action();
            return removed;
        }
        None
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
        self.hands = default();
        self.next_hand = 0;
        self.cooldown = 0.;
        self.burst = 0;
        self.action = WeaponAction::Equipping {
            remaining: definition.mechanics.equip_seconds,
        };
        self.emit(WeaponSignal::Equip);
        Ok(slot)
    }
    pub fn cancel_action(&mut self) {
        self.hands = default();
        self.next_hand = 0;
        self.shot_elapsed = None;
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
        if signal == WeaponSignal::Fire {
            self.shot_elapsed = Some(0.);
        } else if signal != WeaponSignal::Idle {
            self.shot_elapsed = None;
        }
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
    pub fn hand_signal(&self, left: bool) -> WeaponSignal {
        if self.active().is_some_and(|w| w.left_magazine.is_some())
            && self.action == WeaponAction::Ready
        {
            self.hands[usize::from(left)].signal
        } else {
            self.signal
        }
    }
    pub fn hand_sequence(&self, left: bool) -> u64 {
        if self.active().is_some_and(|w| w.left_magazine.is_some())
            && self.action == WeaponAction::Ready
        {
            self.hands[usize::from(left)].sequence
        } else {
            self.sequence
        }
    }
    pub fn hand_shot_elapsed(&self, left: bool) -> Option<f32> {
        if self.active().is_some_and(|w| w.left_magazine.is_some()) {
            self.hands[usize::from(left)].shot_elapsed
        } else {
            self.shot_elapsed
        }
    }
    pub fn hand_reload_timing(&self, left: bool) -> Option<(f32, f32)> {
        self.hands[usize::from(left)].reload
    }
    fn tick_dual(
        &mut self,
        dt: f32,
        intent: WeaponIntent,
        definitions: [&WeaponDefinition; 2],
        emissions: &mut Vec<WeaponEmission>,
    ) {
        let Some(weapon) = self.slots[self.active].as_mut() else {
            return;
        };
        let Some(left) = weapon.left_magazine.as_mut() else {
            return;
        };
        let magazines = [&mut weapon.magazine, left];
        let pool = self
            .ammunition
            .entry(Self::group(definitions[0]))
            .or_default();
        for (hand, (magazine, definition)) in self
            .hands
            .iter_mut()
            .zip(magazines.into_iter().zip(definitions))
        {
            hand.cooldown = (hand.cooldown - dt).max(0.);
            if let Some(age) = &mut hand.shot_elapsed {
                *age += dt;
            }
            if let Some((_, remaining)) = &mut hand.reload {
                *remaining = (*remaining - dt / self.reload_duration_scale).max(0.);
                if *remaining == 0. {
                    let transfer = definition
                        .magazine
                        .saturating_sub(*magazine)
                        .min(pool.rounds);
                    *magazine += transfer;
                    pool.rounds -= transfer;
                    hand.reload = None;
                    hand.signal = WeaponSignal::Idle;
                    hand.sequence = hand.sequence.wrapping_add(1);
                    emissions.push(WeaponEmission::ReloadCompleted(definition.id.clone()));
                }
            }
            if intent.reload
                && hand.reload.is_none()
                && *magazine < definition.magazine
                && pool.rounds > 0
            {
                let empty = *magazine == 0;
                let duration = if empty {
                    definition.mechanics.empty_reload
                } else {
                    definition.reload_seconds
                };
                hand.reload = Some((duration, duration));
                hand.signal = if empty {
                    WeaponSignal::ReloadEmpty
                } else {
                    WeaponSignal::Reload
                };
                hand.sequence = hand.sequence.wrapping_add(1);
                hand.shot_elapsed = None;
                emissions.push(WeaponEmission::ReloadStarted {
                    weapon: definition.id.clone(),
                    magazine: *magazine,
                    capacity: definition.magazine,
                    seconds: duration * self.reload_duration_scale,
                });
            }
        }
        let firing = intent.pressed
            || intent.held && matches!(definitions[0].mechanics.fire, FirePattern::Automatic);
        if !firing || self.cooldown > 0. {
            return;
        }
        let available =
            |index: usize| self.hands[index].reload.is_none() && self.hands[index].cooldown == 0.;
        let loaded = |index: usize| {
            if index == 0 {
                weapon.magazine > 0
            } else {
                weapon.left_magazine.is_some_and(|n| n > 0)
            }
        };
        let selected = [self.next_hand, 1 - self.next_hand]
            .into_iter()
            .find(|&i| available(i) && loaded(i));
        let Some(index) = selected else {
            if available(self.next_hand) {
                self.hands[self.next_hand].signal = WeaponSignal::DryFire;
                self.hands[self.next_hand].sequence =
                    self.hands[self.next_hand].sequence.wrapping_add(1);
                self.cooldown = 0.18;
                self.emit(WeaponSignal::DryFire);
            }
            return;
        };
        if index == 0 {
            weapon.magazine -= 1;
        } else if let Some(n) = &mut weapon.left_magazine {
            *n -= 1;
        }
        let hand = &mut self.hands[index];
        hand.cooldown = definitions[index].fire_interval * self.firing.interval_scale;
        hand.signal = WeaponSignal::Fire;
        hand.sequence = hand.sequence.wrapping_add(1);
        hand.shot_elapsed = Some(0.);
        self.cooldown = hand.cooldown;
        self.next_hand = 1 - index;
        self.emit(WeaponSignal::Fire);
        emissions.push(WeaponEmission::Shot(definitions[index].id.clone()));
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
        self.hands = default();
        self.next_hand = 0;
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
        if let Some(elapsed) = &mut self.shot_elapsed {
            *elapsed = (*elapsed + dt).min(60.);
        }
        let reload_dt = dt / self.reload_duration_scale;
        if self.action == WeaponAction::Ready {
            intent.pressed |= self.pending.pressed;
            intent.melee |= self.pending.melee;
            intent.cycle |= self.pending.cycle;
            intent.slot = intent.slot.or(self.pending.slot);
            self.pending = WeaponIntent::default();
        }
        let was_cooling = self.cooldown > 0.;
        let fire_overrun = if was_cooling {
            (dt - self.cooldown).max(0.)
        } else {
            0.
        };
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
        if self.ads == 0. {
            self.ads_leaving = false;
        } else if self.ads == 1. {
            self.ads_leaving = true;
        }
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
                let remaining = (remaining - reload_dt).max(0.);
                if remaining > 0. {
                    self.action = WeaponAction::Reloading { remaining, empty };
                    return emissions;
                }
                self.insert(definition, u32::MAX);
                emissions.push(WeaponEmission::ReloadCompleted(definition.id.clone()));
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
                let remaining = (remaining - reload_dt).max(0.);
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
                    emissions.push(WeaponEmission::ReloadCompleted(definition.id.clone()));
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
            self.hands = default();
            self.burst = 0;
            self.action = WeaponAction::Melee {
                remaining: melee_definition.mechanics.melee_seconds[1]
                    .max(melee_definition.mechanics.melee_seconds[0]),
                hit: false,
            };
            self.emit(WeaponSignal::Melee);
            return emissions;
        }
        if let Some(partner) = definition
            .dual_weapon
            .as_ref()
            .and_then(|id| definitions.iter().find(|w| w.id == *id))
        {
            self.tick_dual(dt, intent, [definition, partner], &mut emissions);
            return emissions;
        }
        if intent.reload
            && self.reserve(definition) > 0
            && self
                .active()
                .is_some_and(|w| w.magazine < definition.magazine)
        {
            self.burst = 0;
            emissions.push(WeaponEmission::ReloadStarted {
                weapon: definition.id.clone(),
                magazine: self.active().map_or(0, |w| w.magazine),
                capacity: definition.magazine,
                seconds: definition.reload_seconds * self.reload_duration_scale,
            });
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
            FirePattern::Charge => false,
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
                self.cooldown = (self.cooldown * self.firing.interval_scale - fire_overrun).max(0.);
                self.emit(WeaponSignal::Fire);
                emissions.push(WeaponEmission::Shot(definition.id.clone()));
            }
        }
        emissions
    }
}
