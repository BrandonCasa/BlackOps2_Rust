use crate::origins::content::{ContentId, FirePattern, WeaponDefinition, WeaponId};
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct AmmoSnapshot {
    pub weapon: OwnedWeapon,
    pub reserve: u32,
    pub reserve_capacity: u32,
    pub magazine_capacity: u32,
}
#[derive(Clone, Debug)]
pub struct OwnedWeapon {
    pub definition: WeaponId,
    pub magazine: u32,
    pub left_magazine: Option<u32>,
    pub attachment: Option<String>,
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
    pub charge_level: u32,
    charging: f32,
    staff_backup: Option<OwnedWeapon>,
    revive_ammo: BTreeMap<WeaponId, (u32, AmmoPool)>,
    alternate_backup: Option<OwnedWeapon>,
    alternate_ammo: BTreeMap<WeaponId, OwnedWeapon>,
    melee_override: Option<WeaponId>,
    zoom_level: usize,
    stored_staff: BTreeMap<WeaponId, AmmoSnapshot>,
    weapon_ammo: BTreeMap<WeaponId, (ContentId, u32, u32)>,
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
            charge_level: 0,
            charging: 0.,
            staff_backup: None,
            revive_ammo: BTreeMap::new(),
            alternate_backup: None,
            alternate_ammo: BTreeMap::new(),
            melee_override: None,
            zoom_level: 0,
            stored_staff: BTreeMap::new(),
            weapon_ammo: BTreeMap::new(),
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
    DryFire(WeaponId),
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
    None,
    Head,
    UpperTorso,
    Torso,
    Arm,
    LeftArm,
    RightArm,
    Powerplant,
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
        HitRegion::None => return damage.round().max(1.) as u32,
        HitRegion::Head => 0,
        HitRegion::UpperTorso => 1,
        HitRegion::Torso => 2,
        HitRegion::Arm | HitRegion::LeftArm | HitRegion::RightArm => 3,
        HitRegion::Powerplant => 2,
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
    pub fn cancel_charge(&mut self) {
        self.charging = 0.;
        self.charge_level = 0;
    }
    pub fn set_melee_override(&mut self, weapon: Option<WeaponId>) {
        if !matches!(self.action, WeaponAction::Melee { .. }) {
            self.melee_override = weapon;
        }
    }
    pub fn melee_definition<'a>(
        &self,
        definitions: &'a [WeaponDefinition],
    ) -> Option<&'a WeaponDefinition> {
        if let Some(id) = &self.melee_override {
            return definitions.iter().find(|w| w.id == *id);
        }
        let active = self.active()?;
        if !active.definition.0.contains("_upgraded") {
            return None;
        }
        let family = active
            .definition
            .0
            .strip_prefix("weapon.staff_")?
            .split('_')
            .next()?;
        definitions
            .iter()
            .find(|w| w.id.0 == format!("weapon.staff_{family}_melee_zm"))
    }
    fn restore_staff_mode(&mut self) {
        if let Some(original) = self.staff_backup.take() {
            let magazine = self.active().map_or(1, |w| w.magazine);
            let reserve = self
                .ammunition
                .remove(&ContentId("ammo.staff_revive".into()))
                .unwrap_or(AmmoPool {
                    rounds: 3,
                    capacity: 3,
                });
            self.revive_ammo
                .insert(original.definition.clone(), (magazine, reserve));
            self.slots[self.active] = Some(original);
        }
        if let Some(mut original) = self.alternate_backup.take() {
            if let Some(alternate) = self.slots[self.active].take() {
                if self
                    .weapon_ammo
                    .get(&original.definition)
                    .zip(self.weapon_ammo.get(&alternate.definition))
                    .is_some_and(|(a, b)| a.0 == b.0)
                {
                    original.magazine = alternate.magazine;
                }
                self.alternate_ammo
                    .insert(alternate.definition.clone(), alternate);
            }
            self.slots[self.active] = Some(original);
        }
    }
    pub fn toggle_staff_mode(&mut self, definitions: &[WeaponDefinition]) -> bool {
        if self.action != WeaponAction::Ready {
            return false;
        }
        if self.staff_backup.is_some() || self.alternate_backup.is_some() {
            let alternate = self.active().cloned();
            self.restore_staff_mode();
            if let Some(previous) = alternate {
                if let Some(original) = self.slots[self.active].as_mut() {
                    if let (Some(a), Some(b)) = (
                        definitions.iter().find(|w| w.id == original.definition),
                        definitions.iter().find(|w| w.id == previous.definition),
                    ) {
                        if a.ammo_group == b.ammo_group {
                            original.magazine = previous.magazine;
                        }
                    }
                }
            }
            let duration = self
                .definition(definitions)
                .map_or(0.5, |w| w.mechanics.equip_seconds);
            self.cancel_action();
            self.action = WeaponAction::Equipping {
                remaining: duration,
            };
            self.emit(WeaponSignal::Equip);
            return true;
        }
        let Some(active) = self.active().cloned() else {
            return false;
        };
        let Some(current) = definitions.iter().find(|w| w.id == active.definition) else {
            return false;
        };
        let staff = active.definition.0.starts_with("weapon.staff_")
            && active.definition.0.contains("upgraded");
        let alternate = if staff {
            definitions
                .iter()
                .find(|w| w.id.0 == "weapon.staff_revive_zm")
        } else {
            current
                .attachments
                .get(self.attachment())
                .and_then(|a| a.alternate.as_ref())
                .or(current.mechanics.alternate.as_ref())
                .and_then(|id| definitions.iter().find(|w| w.id == *id))
        };
        let Some(alternate) = alternate else {
            return false;
        };
        let magazine = if staff {
            let (magazine, reserve) = self.revive_ammo.remove(&active.definition).unwrap_or((
                1,
                AmmoPool {
                    rounds: 3,
                    capacity: 3,
                },
            ));
            self.ammunition
                .insert(ContentId("ammo.staff_revive".into()), reserve);
            self.staff_backup = Some(active);
            magazine
        } else {
            let shared = current.ammo_group == alternate.ammo_group;
            let magazine = if shared {
                active.magazine
            } else {
                self.alternate_ammo
                    .remove(&alternate.id)
                    .map_or(alternate.magazine, |w| w.magazine)
            };
            self.weapon_ammo.insert(
                alternate.id.clone(),
                (
                    Self::group(alternate),
                    alternate.magazine,
                    alternate.reserve,
                ),
            );
            self.alternate_backup = Some(active);
            self.ammunition
                .entry(Self::group(alternate))
                .or_insert(AmmoPool {
                    rounds: alternate.mechanics.starting_reserve,
                    capacity: alternate.reserve,
                });
            magazine
        };
        self.slots[self.active] = Some(OwnedWeapon {
            definition: alternate.id.clone(),
            magazine,
            left_magazine: None,
            attachment: None,
        });
        self.cancel_action();
        self.action = WeaponAction::Equipping {
            remaining: alternate.mechanics.equip_seconds,
        };
        self.emit(WeaponSignal::Equip);
        true
    }
    pub fn toggle_zoom(&mut self, definition: &WeaponDefinition) {
        if definition
            .attachments
            .get(self.attachment())
            .is_some_and(|a| a.ads_fov.len() > 1)
        {
            self.zoom_level ^= 1;
        }
    }
    pub fn effective_ads_fov(&self, definition: &WeaponDefinition) -> f32 {
        definition
            .attachments
            .get(self.attachment())
            .and_then(|a| {
                a.ads_fov
                    .get(self.zoom_level.min(a.ads_fov.len().saturating_sub(1)))
            })
            .copied()
            .unwrap_or(definition.mechanics.ads_fov)
    }
    pub fn effective_ads_sight<'a>(
        &self,
        definition: &'a WeaponDefinition,
    ) -> Option<&'a crate::origins::content::AdsSight> {
        definition
            .attachments
            .get(self.attachment())
            .and_then(|a| a.ads_sight.as_ref())
            .or(definition.mechanics.ads_sight.as_ref())
    }
    pub fn ads_zoom(&self, mechanics: &crate::origins::content::WeaponMechanics) -> f32 {
        let fraction = mechanics.ads_zoom_fraction[usize::from(self.ads_leaving)];
        if fraction <= 0. {
            return if self.ads >= 1. { 1. } else { 0. };
        }
        ((self.ads - (1. - fraction)) / fraction).clamp(0., 1.)
    }
    pub fn ads_fov(&self, hip_vertical: f32, definition: &WeaponDefinition) -> f32 {
        let mechanics = &definition.mechanics;
        let horizontal = |vertical: f32| 2. * (vertical.to_radians() * 0.5).tan().atan2(0.75);
        let hip = horizontal(hip_vertical);
        let aimed = horizontal(self.effective_ads_fov(definition));
        let zoom = definition
            .attachments
            .get(self.attachment())
            .and_then(|a| a.ads_zoom_fraction)
            .map_or_else(
                || self.ads_zoom(mechanics),
                |fractions| {
                    let fraction = fractions[usize::from(self.ads_leaving)];
                    if fraction <= 0. {
                        if self.ads >= 1. { 1. } else { 0. }
                    } else {
                        ((self.ads - (1. - fraction)) / fraction).clamp(0., 1.)
                    }
                },
            );
        2. * (((hip + (aimed - hip) * zoom) * 0.5).tan() * 0.75).atan()
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
    pub fn attachment(&self) -> &str {
        self.active()
            .and_then(|w| w.attachment.as_deref())
            .unwrap_or("defaultattachment")
    }
    pub fn set_attachment(&mut self, id: &WeaponId, attachment: Option<String>) {
        if let Some(weapon) = self
            .slots
            .iter_mut()
            .flatten()
            .find(|w| w.definition == *id)
        {
            weapon.attachment = attachment;
        }
    }
    pub fn attachment_spread(&self) -> f32 {
        1.
    }
    pub fn attachment_range(&self) -> f32 {
        match self.attachment() {
            "extbarrel" => 1.16,
            "silencer" => 0.7,
            _ => 1.,
        }
    }
    pub fn exchange_staff(
        &mut self,
        definition: &WeaponDefinition,
    ) -> Result<usize, InventoryError> {
        if self.action != WeaponAction::Ready {
            return Err(InventoryError::Busy);
        }
        self.restore_staff_mode();
        for slot in &mut self.slots {
            if slot
                .as_ref()
                .is_some_and(|w| w.definition.0.starts_with("weapon.staff_"))
            {
                let weapon = slot.take().unwrap();
                let (group, magazine_capacity, reserve_capacity) = self
                    .weapon_ammo
                    .get(&weapon.definition)
                    .cloned()
                    .unwrap_or((ContentId(weapon.definition.0.clone()), weapon.magazine, 0));
                let pool = self.ammunition.get(&group).cloned().unwrap_or_default();
                self.stored_staff.insert(
                    weapon.definition.clone(),
                    AmmoSnapshot {
                        magazine_capacity,
                        weapon,
                        reserve: pool.rounds,
                        reserve_capacity,
                    },
                );
            }
        }
        let slot = self.receive(definition)?;
        if let Some(snapshot) = self.stored_staff.remove(&definition.id) {
            self.restore_received(definition, &snapshot);
        }
        Ok(slot)
    }
    pub fn deposit_active(&mut self) -> Result<OwnedWeapon, InventoryError> {
        if self.action != WeaponAction::Ready {
            return Err(InventoryError::Busy);
        }
        self.restore_staff_mode();
        let weapon = self.slots[self.active]
            .take()
            .ok_or(InventoryError::InvalidSlot)?;
        self.active = self.slots[..self.capacity]
            .iter()
            .position(Option::is_some)
            .unwrap_or(0);
        self.cancel_action();
        Ok(weapon)
    }
    pub fn ammo_snapshot(&self, definition: &WeaponDefinition) -> Option<AmmoSnapshot> {
        let weapon = self.active()?.clone();
        let pool = self.ammunition.get(&Self::group(definition));
        Some(AmmoSnapshot {
            weapon,
            reserve: pool.map_or(0, |p| p.rounds),
            reserve_capacity: definition.reserve,
            magazine_capacity: definition.magazine,
        })
    }
    pub fn restore_received(&mut self, definition: &WeaponDefinition, snapshot: &AmmoSnapshot) {
        for weapon in self
            .slots
            .iter_mut()
            .flatten()
            .filter(|w| w.definition == definition.id)
        {
            weapon.magazine = snapshot
                .weapon
                .magazine
                .saturating_add(
                    definition
                        .magazine
                        .saturating_sub(snapshot.magazine_capacity),
                )
                .min(definition.magazine);
            weapon.left_magazine = snapshot.weapon.left_magazine.map(|n| {
                n.saturating_add(
                    definition
                        .magazine
                        .saturating_sub(snapshot.magazine_capacity),
                )
                .min(definition.magazine)
            });
        }
        let pool = self.ammunition.entry(Self::group(definition)).or_default();
        pool.capacity = definition.reserve;
        pool.rounds = snapshot
            .reserve
            .saturating_add(definition.reserve.saturating_sub(snapshot.reserve_capacity))
            .min(pool.capacity);
    }
    pub fn fill_received(&mut self, definition: &WeaponDefinition) {
        for weapon in self
            .slots
            .iter_mut()
            .flatten()
            .filter(|w| w.definition == definition.id)
        {
            weapon.magazine = definition.magazine;
            weapon.left_magazine = definition.dual_weapon.as_ref().map(|_| definition.magazine);
        }
        let pool = self.ammunition.entry(Self::group(definition)).or_default();
        pool.capacity = definition.reserve;
        pool.rounds = definition.reserve;
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
        if definition.id.0 == "weapon.staff_revive_zm" {
            return ContentId("ammo.staff_revive".into());
        }
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
    pub fn total_ammunition(&self, definition: &WeaponDefinition) -> u32 {
        self.owned()
            .find(|(_, weapon)| weapon.definition == definition.id)
            .map_or(0, |(_, weapon)| {
                weapon
                    .magazine
                    .saturating_add(weapon.left_magazine.unwrap_or(0))
            })
            .saturating_add(self.reserve(definition))
    }
    pub fn last_stand(
        &self,
        definition: &WeaponDefinition,
        reserve_clips: u32,
        free_ammunition: bool,
    ) -> (Self, u32) {
        let mut inventory = Self::temporary(definition, reserve_clips);
        if let Some((_, weapon)) = self
            .owned()
            .find(|(_, weapon)| weapon.definition == definition.id)
        {
            inventory.slots[0] = Some(weapon.clone());
            if !free_ammunition {
                let reserve = if matches!(
                    definition.id.0.as_str(),
                    "weapon.ray_gun_zm" | "weapon.ray_gun_upgraded_zm"
                ) {
                    inventory.slots[0]
                        .as_mut()
                        .expect("selected last-stand weapon")
                        .magazine = self.total_ammunition(definition).min(definition.magazine);
                    0
                } else {
                    self.reserve(definition)
                        .min(definition.magazine.saturating_mul(reserve_clips))
                };
                inventory
                    .ammunition
                    .get_mut(&Self::group(definition))
                    .expect("temporary ammunition pool")
                    .rounds = reserve;
            }
        }
        let given = if free_ammunition {
            0
        } else {
            inventory.total_ammunition(definition)
        };
        (inventory, given)
    }
    pub fn debit_ammunition(&mut self, definition: &WeaponDefinition, amount: u32) {
        let stock = self.reserve(definition);
        if let Some(pool) = self.ammunition.get_mut(&Self::group(definition)) {
            pool.rounds = stock.saturating_sub(amount);
        }
        let magazine_used = amount.saturating_sub(stock);
        if let Some(weapon) = self
            .slots
            .iter_mut()
            .flatten()
            .find(|weapon| weapon.definition == definition.id)
        {
            weapon.magazine = weapon.magazine.saturating_sub(magazine_used);
        }
    }
    pub fn grant(&mut self, definition: &WeaponDefinition) -> Result<usize, InventoryError> {
        self.weapon_ammo.insert(
            definition.id.clone(),
            (
                Self::group(definition),
                definition.magazine,
                definition.reserve,
            ),
        );
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
            attachment: None,
        });
        Ok(slot)
    }
    pub fn replace(
        &mut self,
        slot: usize,
        definition: &WeaponDefinition,
    ) -> Result<(), InventoryError> {
        self.weapon_ammo.insert(
            definition.id.clone(),
            (
                Self::group(definition),
                definition.magazine,
                definition.reserve,
            ),
        );
        if slot >= self.capacity {
            return Err(InventoryError::InvalidSlot);
        }
        if self
            .owned()
            .any(|(i, w)| i != slot && w.definition == definition.id)
        {
            return Err(InventoryError::AlreadyOwned);
        }
        if !matches!(
            self.action,
            WeaponAction::Ready | WeaponAction::Reloading { .. } | WeaponAction::ShellReload { .. }
        ) {
            return Err(InventoryError::Busy);
        }
        self.slots[slot] = Some(OwnedWeapon {
            definition: definition.id.clone(),
            magazine: definition.magazine,
            left_magazine: definition.dual_weapon.as_ref().map(|_| definition.magazine),
            attachment: None,
        });
        let pool = self.ammunition.entry(Self::group(definition)).or_default();
        pool.capacity = pool.capacity.max(definition.reserve);
        pool.rounds = pool
            .rounds
            .max(definition.mechanics.starting_reserve)
            .min(pool.capacity);
        if slot == self.active {
            self.cancel_action();
            self.cooldown = 0.;
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
        if !enabled && self.active == 2 {
            self.restore_staff_mode();
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
        for snapshot in self.stored_staff.values_mut() {
            snapshot.reserve = snapshot.reserve_capacity;
        }
        for (_, pool) in self.revive_ammo.values_mut() {
            pool.rounds = pool.capacity;
        }
        if let Some(pool) = self
            .ammunition
            .get_mut(&ContentId("ammo.staff_revive".into()))
        {
            pool.rounds = pool.capacity;
        }
        for definition in definitions {
            if self
                .staff_backup
                .as_ref()
                .is_some_and(|w| w.definition == definition.id)
                || self.owned().any(|(_, w)| w.definition == definition.id)
            {
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
        if !matches!(
            self.action,
            WeaponAction::Ready | WeaponAction::Reloading { .. } | WeaponAction::ShellReload { .. }
        ) {
            return Err(InventoryError::Busy);
        }
        self.restore_staff_mode();
        for slot in &mut self.slots {
            if slot
                .as_ref()
                .is_some_and(|weapon| weapon.definition.0 == "weapon.zombie_fists_zm")
            {
                *slot = None;
            }
        }
        let slot = match self.grant(definition) {
            Ok(slot) => slot,
            Err(InventoryError::Full) => {
                self.replace(self.active, definition)?;
                return Ok(self.active);
            }
            Err(error) => return Err(error),
        };
        self.cancel_action();
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
        self.charging = 0.;
        self.charge_level = 0;
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
                emissions.push(WeaponEmission::DryFire(
                    definitions[self.next_hand].id.clone(),
                ));
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
        self.restore_staff_mode();
        self.charging = 0.;
        self.charge_level = 0;
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
        if intent.reload
            || intent.melee
            || intent.cycle
            || intent.slot.is_some()
            || self.action != WeaponAction::Ready
        {
            self.charging = 0.;
            self.charge_level = 0;
        }
        if let Some(elapsed) = &mut self.shot_elapsed {
            *elapsed = (*elapsed + dt).min(60.);
        }
        let reload_dt = dt / self.reload_duration_scale
            * if self.attachment() == "dualclip" {
                1. / 0.675
            } else {
                1.
            };
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
        let attachment_scale = if aiming && self.attachment() == "is" {
            0.85
        } else {
            1.
        };
        let step =
            dt / (definition.mechanics.ads_seconds[if aiming { 0 } else { 1 }] * attachment_scale);
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
        let center_scale = if self.ads > 0.5 && !definition.mechanics.dual_wield {
            match self.attachment() {
                "grip" | "acog" => 1.02,
                "silencer" => 0.98,
                _ => 1.,
            }
        } else {
            1.
        };
        let recovery = if definition.mechanics.dual_wield {
            definition.mechanics.recoil_recovery
        } else {
            definition
                .mechanics
                .ads_recoil_recovery
                .unwrap_or(definition.mechanics.recoil_recovery)
        };
        let recovery = recovery.to_radians() * dt * center_scale;
        self.recoil = self
            .recoil
            .map(|value| value.signum() * (value.abs() - recovery).max(0.));
        let melee_definition = self
            .melee_definition(definitions)
            .or_else(|| definitions.iter().find(|w| w.id.0 == "weapon.knife_zm"))
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
        if matches!(definition.mechanics.fire, FirePattern::Charge) {
            if self.cooldown > 0. {
                return emissions;
            }
            let magazine = self.active().map_or(0, |w| w.magazine);
            if intent.held || intent.pressed {
                if magazine == 0 {
                    if intent.pressed {
                        self.emit(WeaponSignal::DryFire);
                        emissions.push(WeaponEmission::DryFire(definition.id.clone()));
                    }
                    return emissions;
                }
                self.charging += dt;
                self.charge_level = if self.charging >= 1. {
                    3
                } else if self.charging >= 0.5 {
                    2
                } else {
                    1
                };
                while self.charge_level > 1 && magazine < [1, 3, 6][self.charge_level as usize - 1]
                {
                    self.charge_level -= 1;
                }
                return emissions;
            }
            if self.charging > 0. {
                let mut tier = self.charge_level.max(1);
                let costs = [1, 3, 6];
                while tier > 1 && magazine < costs[tier as usize - 1] {
                    tier -= 1;
                }
                let cost = costs[tier as usize - 1];
                if magazine >= cost {
                    let id = if tier == 1 {
                        definition.id.clone()
                    } else {
                        WeaponId(
                            definition
                                .id
                                .0
                                .replace("_upgraded_zm", &format!("_upgraded{tier}_zm")),
                        )
                    };
                    if definitions.iter().any(|w| w.id == id) {
                        if let Some(weapon) = self.slots[self.active].as_mut() {
                            weapon.magazine -= cost;
                        }
                        self.cooldown = definition.fire_interval;
                        self.emit(WeaponSignal::Fire);
                        emissions.push(WeaponEmission::Shot(id));
                    }
                }
                self.charging = 0.;
                self.charge_level = 0;
            }
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
                emissions.push(WeaponEmission::DryFire(definition.id.clone()));
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
