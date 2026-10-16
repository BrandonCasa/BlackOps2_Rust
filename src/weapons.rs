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
    raised: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeaponAction {
    Ready,
    Equipping {
        remaining: f32,
        duration: f32,
    },
    Unequipping {
        remaining: f32,
        duration: f32,
        target: usize,
    },
    Reloading {
        remaining: f32,
        empty: bool,
        transferred: bool,
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
    reload: Option<HandReload>,
    signal: WeaponSignal,
    sequence: u64,
    shot_elapsed: Option<f32>,
}
#[derive(Clone)]
struct HandReload {
    duration: f32,
    remaining: f32,
    transfer: f32,
    transferred: bool,
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
    ads_requested: bool,
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
            ads_requested: false,
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
    FirstEquip,
    QuickEquip,
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
    pub suspended: bool,
    pub sprint: bool,
    pub pressed: bool,
    pub held: bool,
    pub secondary_pressed: bool,
    pub secondary_held: bool,
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
    Ads {
        weapon: WeaponId,
        aiming: bool,
    },
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
    Helmet,
    Head,
    Neck,
    UpperTorso,
    MidTorso,
    LowerTorso,
    RightUpperArm,
    LeftUpperArm,
    RightLowerArm,
    LeftLowerArm,
    RightHand,
    LeftHand,
    RightUpperLeg,
    LeftUpperLeg,
    RightLowerLeg,
    LeftLowerLeg,
    RightFoot,
    LeftFoot,
    Gun,
    Shield,
    Torso,
    Arm,
    LeftArm,
    RightArm,
    Powerplant,
    Leg,
}
impl HitRegion {
    pub fn native_index(self) -> usize {
        match self {
            Self::None => 0,
            Self::Helmet => 1,
            Self::Head => 2,
            Self::Neck => 3,
            Self::UpperTorso => 4,
            Self::MidTorso | Self::Torso | Self::Powerplant => 5,
            Self::LowerTorso => 6,
            Self::RightUpperArm | Self::RightArm | Self::Arm => 7,
            Self::LeftUpperArm | Self::LeftArm => 8,
            Self::RightLowerArm => 9,
            Self::LeftLowerArm => 10,
            Self::RightHand => 11,
            Self::LeftHand => 12,
            Self::RightUpperLeg | Self::Leg => 13,
            Self::LeftUpperLeg => 14,
            Self::RightLowerLeg => 15,
            Self::LeftLowerLeg => 16,
            Self::RightFoot => 17,
            Self::LeftFoot => 18,
            Self::Gun => 19,
            Self::Shield => 20,
        }
    }
    pub fn head(self) -> bool {
        matches!(self, Self::Head | Self::Helmet)
    }
    pub fn left_arm(self) -> bool {
        matches!(
            self,
            Self::LeftArm | Self::LeftUpperArm | Self::LeftLowerArm | Self::LeftHand
        )
    }
    pub fn right_arm(self) -> bool {
        matches!(
            self,
            Self::Arm
                | Self::RightArm
                | Self::RightUpperArm
                | Self::RightLowerArm
                | Self::RightHand
        )
    }
    pub fn leg(self) -> bool {
        matches!(
            self,
            Self::Leg
                | Self::RightUpperLeg
                | Self::LeftUpperLeg
                | Self::RightLowerLeg
                | Self::LeftLowerLeg
                | Self::RightFoot
                | Self::LeftFoot
        )
    }
}
pub fn damage_at(definition: &WeaponDefinition, distance: f32, region: HitRegion) -> u32 {
    if definition.mechanics.delivery == crate::origins::content::WeaponDelivery::Melee {
        return definition.damage;
    }
    let [near, far] = definition.mechanics.damage_range;
    let fraction = if far > near {
        ((distance - near) / (far - near)).clamp(0., 1.)
    } else {
        0.
    };
    let damage = definition.damage as f32
        + (definition.mechanics.minimum_damage as f32 - definition.damage as f32) * fraction;
    location_damage(definition, damage as u32, region)
}
pub fn location_damage(definition: &WeaponDefinition, damage: u32, region: HitRegion) -> u32 {
    if definition.mechanics.delivery == crate::origins::content::WeaponDelivery::Melee {
        return damage;
    }
    (damage as f64 * definition.mechanics.hit_multipliers[region.native_index()] as f64)
        .trunc()
        .max(1.) as u32
}

#[cfg(test)]
mod native_location_tests {
    use super::*;

    #[test]
    fn source_location_multipliers_preserve_sides_truncation_and_melee_bypass() {
        let mut weapon: WeaponDefinition = serde_json::from_value(serde_json::json!({"id":"weapon.test","name":"test","damage":101,"category":"bullet","magazine":1,"reserve":0,"fire_interval":0.1,"reload_seconds":1.,"mechanics":{"minimum_damage":101}})).unwrap();
        weapon.mechanics.hit_multipliers[9] = 0.51;
        weapon.mechanics.hit_multipliers[10] = 0.79;
        weapon.mechanics.hit_multipliers[0] = 0.8;
        assert_eq!(damage_at(&weapon, 0., HitRegion::RightLowerArm), 51);
        assert_eq!(damage_at(&weapon, 0., HitRegion::LeftLowerArm), 79);
        assert_eq!(damage_at(&weapon, 0., HitRegion::None), 80);
        weapon.mechanics.delivery = crate::origins::content::WeaponDelivery::Melee;
        assert_eq!(damage_at(&weapon, 0., HitRegion::Head), 101);
    }

    fn pistol() -> WeaponDefinition {
        serde_json::from_value(serde_json::json!({"id":"weapon.c96_zm","name":"C96",
            "damage":20,"category":"bullet","magazine":8,"reserve":40,
            "fire_interval":0.2,"reload_seconds":1.2,
            "mechanics":{"starting_reserve":40,"empty_reload":1.5,
                "reload_transfer_seconds":[0.4,0.6]}}))
        .unwrap()
    }

    #[test]
    fn empty_weapon_auto_reloads_after_cooldown_and_respects_hand_and_ads_locks() {
        let mut weapon = pistol();
        let mut inventory = WeaponInventory::default();
        inventory.grant(&weapon).unwrap();
        inventory.slots[0].as_mut().unwrap().magazine = 0;
        inventory.cooldown = 0.2;
        assert!(inventory.tick(0.1, default(), &[weapon.clone()]).is_empty());
        assert_eq!(inventory.action, WeaponAction::Ready);
        let events = inventory.tick(0.1, default(), &[weapon.clone()]);
        assert!(matches!(
            inventory.action,
            WeaponAction::Reloading { empty: true, .. }
        ));
        assert!(events.iter().any(
            |e| matches!(e, WeaponEmission::ReloadStarted { seconds, .. } if *seconds == 1.5)
        ));
        inventory.tick(1.5, default(), &[weapon.clone()]);
        assert_eq!(inventory.active().unwrap().magazine, 8);

        inventory.slots[0].as_mut().unwrap().magazine = 0;
        inventory.tick(
            0.1,
            WeaponIntent {
                suspended: true,
                ..default()
            },
            &[weapon.clone()],
        );
        assert_eq!(inventory.action, WeaponAction::Ready);
        weapon.mechanics.no_ads_auto_reload = true;
        inventory.tick(
            0.1,
            WeaponIntent {
                ads: true,
                ..default()
            },
            &[weapon.clone()],
        );
        assert_eq!(inventory.action, WeaponAction::Ready);
        inventory.tick(0.1, default(), &[weapon]);
        assert!(matches!(inventory.action, WeaponAction::Reloading { .. }));
    }

    #[test]
    fn ads_reversal_uses_out_fraction_and_emits_only_one_transition() {
        let mut weapon = pistol();
        weapon.mechanics.ads_zoom_fraction = [0.7, 0.1];
        let mut inventory = WeaponInventory::default();
        inventory.grant(&weapon).unwrap();
        inventory.ads = 0.9;
        inventory.ads_requested = true;
        let events = inventory.tick(0.01, default(), &[weapon.clone()]);
        assert!(inventory.ads_leaving);
        assert_eq!(inventory.ads_zoom(&weapon.mechanics), 0.);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, WeaponEmission::Ads { aiming: false, .. }))
                .count(),
            1
        );
        assert!(inventory.tick(0.01, default(), &[weapon]).is_empty());
    }

    #[test]
    fn reload_interruption_preserves_only_transferred_ammunition() {
        let weapon = pistol();
        for (age, expected) in [(0.3, 5), (0.5, 8)] {
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            inventory.slots[0].as_mut().unwrap().magazine = 5;
            inventory.tick(
                0.,
                WeaponIntent {
                    reload: true,
                    ..default()
                },
                &[weapon.clone()],
            );
            inventory.tick(age, default(), &[weapon.clone()]);
            inventory.tick(
                0.01,
                WeaponIntent {
                    melee: true,
                    ..default()
                },
                &[weapon.clone()],
            );
            assert!(matches!(inventory.action, WeaponAction::Melee { .. }));
            assert_eq!(inventory.active().unwrap().magazine, expected);
            assert_eq!(inventory.reserve(&weapon), 40 - (expected - 5));
        }
    }

    #[test]
    fn shell_reload_finishes_its_exit_and_does_not_fire_a_released_input() {
        let mut weapon = pistol();
        weapon.mechanics.shell_reload = true;
        weapon.mechanics.reload_start_seconds = 0.3;
        weapon.mechanics.reload_start_insert = 0.2;
        weapon.mechanics.reload_end_seconds = 0.2;
        for held in [false, true] {
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            inventory.slots[0].as_mut().unwrap().magazine = 0;
            inventory.tick(0., default(), &[weapon.clone()]);
            inventory.tick(0.2, default(), &[weapon.clone()]);
            assert_eq!(inventory.active().unwrap().magazine, 1);
            let events = inventory.tick(
                0.,
                WeaponIntent {
                    pressed: true,
                    held: true,
                    ..default()
                },
                &[weapon.clone()],
            );
            assert!(events.is_empty());
            assert!(matches!(
                inventory.action,
                WeaponAction::ShellReload {
                    phase: ShellPhase::End,
                    ..
                }
            ));
            inventory.tick(0.2, WeaponIntent { held, ..default() }, &[weapon.clone()]);
            let events =
                inventory.tick(0.01, WeaponIntent { held, ..default() }, &[weapon.clone()]);
            assert_eq!(
                events
                    .iter()
                    .filter(|e| matches!(e, WeaponEmission::Shot(_)))
                    .count(),
                usize::from(held)
            );
        }
    }

    #[test]
    fn new_and_restored_weapons_keep_their_own_equip_clock() {
        let mut weapon = pistol();
        weapon.mechanics.first_equip_seconds = 1.25;
        weapon.animation_actions.insert(
            crate::origins::WeaponAnimation::FirstEquip,
            ContentId("animation.first".into()),
        );
        let mut inventory = WeaponInventory::default();
        inventory.receive(&weapon).unwrap();
        assert_eq!(inventory.signal, WeaponSignal::FirstEquip);
        assert_eq!(
            inventory.presentation_timing(&weapon, &weapon),
            Some((1.25, 1.25))
        );
        inventory.restore_offhand(0.35);
        assert_eq!(inventory.signal, WeaponSignal::QuickEquip);
        assert_eq!(
            inventory.presentation_timing(&weapon, &weapon),
            Some((0.35, 0.35))
        );
        inventory.equip_new(&weapon);
        assert_eq!(inventory.signal, WeaponSignal::Equip);
        assert_eq!(
            inventory.presentation_timing(&weapon, &weapon),
            Some((
                weapon.mechanics.equip_seconds,
                weapon.mechanics.equip_seconds
            ))
        );
    }

    #[test]
    fn native_magazine_reload_transfers_before_animation_end_once() {
        let weapon = pistol();
        for (magazine, transfer_at) in [(5, 0.4), (0, 0.6)] {
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            inventory.slots[0].as_mut().unwrap().magazine = magazine;
            let definitions = [weapon.clone()];
            inventory.tick(
                0.,
                WeaponIntent {
                    reload: true,
                    ..default()
                },
                &definitions,
            );
            inventory.tick(transfer_at - 0.01, default(), &definitions);
            assert_eq!(inventory.active().unwrap().magazine, magazine);
            inventory.tick(0.01, default(), &definitions);
            assert_eq!(inventory.active().unwrap().magazine, 8);
            assert_eq!(inventory.reserve(&weapon), 40 - (8 - magazine));
            assert!(matches!(inventory.action, WeaponAction::Reloading { .. }));
            let emissions = inventory.tick(2., default(), &definitions);
            assert_eq!(
                emissions
                    .iter()
                    .filter(|event| matches!(event, WeaponEmission::ReloadCompleted(_)))
                    .count(),
                1
            );
            assert_eq!(inventory.reserve(&weapon), 40 - (8 - magazine));
        }
    }

    #[test]
    fn native_reload_cancellation_keeps_only_ammunition_already_transferred() {
        let weapon = pistol();
        for (age, expected_magazine, expected_reserve) in [(0.3, 5, 40), (0.5, 8, 37)] {
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            inventory.slots[0].as_mut().unwrap().magazine = 5;
            let definitions = [weapon.clone()];
            inventory.tick(
                0.,
                WeaponIntent {
                    reload: true,
                    ..default()
                },
                &definitions,
            );
            inventory.tick(age, default(), &definitions);
            inventory.cancel_action();
            inventory.tick(2., default(), &definitions);
            assert_eq!(inventory.active().unwrap().magazine, expected_magazine);
            assert_eq!(inventory.reserve(&weapon), expected_reserve);
        }
    }

    #[test]
    fn sprint_reload_cancel_preserves_the_transfer_on_the_current_tick() {
        for (before, step, expected) in [(0.98, 0.01, 1), (0.99, 0.01, 8), (1.01, 0.01, 8)] {
            let mut weapon = pistol();
            weapon.mechanics.reload_transfer_seconds = Some([1., 1.]);
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            inventory.slots[0].as_mut().unwrap().magazine = 1;
            let weapons = [weapon];
            inventory.tick(
                0.,
                WeaponIntent {
                    reload: true,
                    ..default()
                },
                &weapons,
            );
            inventory.tick(before, WeaponIntent::default(), &weapons);
            inventory.tick(
                step,
                WeaponIntent {
                    sprint: true,
                    ..default()
                },
                &weapons,
            );
            assert_eq!(inventory.action, WeaponAction::Ready);
            assert_eq!(inventory.active().unwrap().magazine, expected);
            let reserve = inventory.reserve(&weapons[0]);
            inventory.tick(
                0.25,
                WeaponIntent {
                    sprint: true,
                    ..default()
                },
                &weapons,
            );
            assert_eq!(inventory.reserve(&weapons[0]), reserve);
        }
    }

    fn dual_pistols() -> [WeaponDefinition; 2] {
        let mut right = pistol();
        let mut left = right.clone();
        left.id = WeaponId("weapon.left".into());
        right.dual_weapon = Some(left.id.clone());
        right.mechanics.dual_wield = true;
        left.mechanics.dual_wield = true;
        [right, left]
    }

    #[test]
    fn dual_triggers_have_independent_cooldowns_and_no_held_semi_repeat() {
        let definitions = dual_pistols();
        let mut inventory = WeaponInventory::default();
        inventory.grant(&definitions[0]).unwrap();
        let events = inventory.tick(
            0.,
            WeaponIntent {
                pressed: true,
                ..default()
            },
            &definitions,
        );
        assert!(matches!(&events[..], [WeaponEmission::Shot(id)] if *id == definitions[0].id));
        let events = inventory.tick(
            0.01,
            WeaponIntent {
                secondary_pressed: true,
                ..default()
            },
            &definitions,
        );
        assert!(matches!(&events[..], [WeaponEmission::Shot(id)] if *id == definitions[1].id));
        let events = inventory.tick(
            0.2,
            WeaponIntent {
                pressed: true,
                secondary_pressed: true,
                ads: true,
                ..default()
            },
            &definitions,
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, WeaponEmission::Shot(_)))
                .count(),
            2
        );
        assert_eq!(inventory.active().unwrap().magazine, 6);
        assert_eq!(inventory.active().unwrap().left_magazine, Some(6));
        assert_eq!(inventory.ads, 0.);
        assert!(
            inventory
                .tick(
                    0.3,
                    WeaponIntent {
                        held: true,
                        secondary_held: true,
                        ..default()
                    },
                    &definitions
                )
                .is_empty()
        );
        assert_eq!(inventory.hand_signal(false), WeaponSignal::Idle);
        assert_eq!(inventory.hand_signal(true), WeaponSignal::Idle);
    }

    #[test]
    fn dual_reload_transfers_at_each_source_time_and_cancellation_preserves_inserted_rounds() {
        let mut definitions = dual_pistols();
        definitions[1].mechanics.reload_transfer_seconds = Some([0.7, 0.7]);
        let mut inventory = WeaponInventory::default();
        inventory.grant(&definitions[0]).unwrap();
        let weapon = inventory.slots[0].as_mut().unwrap();
        weapon.magazine = 5;
        weapon.left_magazine = Some(6);
        inventory.set_reload_duration_scale(0.5);
        let events = inventory.tick(
            0.,
            WeaponIntent {
                reload: true,
                ..default()
            },
            &definitions,
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, WeaponEmission::ReloadStarted { .. }))
                .count(),
            2
        );
        inventory.tick(
            0.2,
            WeaponIntent {
                melee: true,
                ..default()
            },
            &definitions,
        );
        assert_eq!(inventory.active().unwrap().magazine, 8);
        assert_eq!(inventory.active().unwrap().left_magazine, Some(6));
        assert_eq!(inventory.reserve(&definitions[0]), 37);
        assert!(inventory.hand_reload_timing(false).is_none());
        assert!(inventory.hand_reload_timing(true).is_none());
    }

    #[test]
    fn dual_empty_hand_reloads_while_the_other_trigger_fires_its_own_weapon() {
        let definitions = dual_pistols();
        let mut inventory = WeaponInventory::default();
        inventory.grant(&definitions[0]).unwrap();
        inventory.slots[0].as_mut().unwrap().magazine = 0;
        let events = inventory.tick(
            0.,
            WeaponIntent {
                secondary_pressed: true,
                ..default()
            },
            &definitions,
        );
        assert!(events.iter().any(|e| matches!(e, WeaponEmission::ReloadStarted { weapon, .. } if *weapon == definitions[0].id)));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, WeaponEmission::Shot(id) if *id == definitions[1].id))
        );
        inventory.tick(0.6, default(), &definitions);
        assert_eq!(inventory.active().unwrap().magazine, 8);
        assert!(inventory.hand_reload_timing(false).is_some());
        assert_eq!(inventory.active().unwrap().left_magazine, Some(7));
    }

    #[test]
    fn no_op_weapon_selections_preserve_reload_and_staff_charge() {
        for command in [
            WeaponIntent {
                slot: Some(0),
                ..default()
            },
            WeaponIntent {
                slot: Some(1),
                ..default()
            },
            WeaponIntent {
                slot: Some(usize::MAX),
                ..default()
            },
            WeaponIntent {
                cycle: true,
                ..default()
            },
        ] {
            let mut weapon = pistol();
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            inventory.slots[0].as_mut().unwrap().magazine = 5;
            inventory.tick(
                0.,
                WeaponIntent {
                    reload: true,
                    ..default()
                },
                &[weapon.clone()],
            );
            inventory.tick(0.1, command, &[weapon.clone()]);
            assert!(matches!(inventory.action, WeaponAction::Reloading { .. }));
            inventory.cancel_action();
            weapon.mechanics.fire = FirePattern::Charge;
            inventory.tick(
                0.3,
                WeaponIntent {
                    held: true,
                    ..default()
                },
                &[weapon.clone()],
            );
            inventory.tick(
                0.3,
                WeaponIntent {
                    held: true,
                    ..command
                },
                &[weapon],
            );
            assert_eq!(inventory.charge_level, 2);
        }
    }

    #[test]
    fn reload_interruptions_account_for_transfer_on_the_current_tick() {
        for shell_reload in [false, true] {
            let mut weapon = pistol();
            weapon.mechanics.shell_reload = shell_reload;
            weapon.mechanics.reload_start_seconds = 0.6;
            weapon.mechanics.reload_start_insert = 0.4;
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            inventory.slots[0].as_mut().unwrap().magazine = 5;
            inventory.tick(
                0.,
                WeaponIntent {
                    reload: true,
                    ..default()
                },
                &[weapon.clone()],
            );
            inventory.tick(
                0.4,
                WeaponIntent {
                    melee: true,
                    ..default()
                },
                &[weapon.clone()],
            );
            let magazine = if shell_reload { 6 } else { 8 };
            assert_eq!(inventory.active().unwrap().magazine, magazine);
            assert_eq!(inventory.reserve(&weapon), 40 - (magazine - 5));
        }
    }

    #[test]
    fn direct_slot_wins_over_cycle_and_suspension_discards_both_triggers() {
        let mut definitions = dual_pistols().to_vec();
        let mut third = pistol();
        third.id = WeaponId("weapon.third".into());
        definitions.push(third);
        let mut inventory = WeaponInventory::default();
        inventory.capacity = 3;
        for weapon in &definitions {
            inventory.grant(weapon).unwrap();
        }
        inventory.tick(
            0.,
            WeaponIntent {
                slot: Some(2),
                cycle: true,
                ..default()
            },
            &definitions,
        );
        assert!(matches!(
            inventory.action,
            WeaponAction::Unequipping { target: 2, .. }
        ));
        inventory.cancel_action();
        let events = inventory.tick(
            0.,
            WeaponIntent {
                suspended: true,
                pressed: true,
                held: true,
                secondary_pressed: true,
                secondary_held: true,
                reload: true,
                melee: true,
                cycle: true,
                slot: Some(1),
                ads: true,
                sprint: true,
            },
            &definitions,
        );
        assert!(events.is_empty());
        assert_eq!(inventory.action, WeaponAction::Ready);
        assert_eq!(inventory.active().unwrap().magazine, 8);
        assert_eq!(inventory.active().unwrap().left_magazine, Some(8));
        assert!(inventory.tick(0.1, default(), &definitions).is_empty());
    }

    #[test]
    fn recoil_recenters_at_the_current_hip_or_ads_rate() {
        let mut weapon = pistol();
        weapon.mechanics.recoil_recovery = 10.;
        weapon.mechanics.ads_recoil_recovery = Some(30.);
        for ads in [0., 1.] {
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            inventory.ads = ads;
            inventory.recoil = Vec2::splat(0.5);
            inventory.tick(
                0.1,
                WeaponIntent {
                    ads: ads == 1.,
                    ..default()
                },
                &[weapon.clone()],
            );
            let expected = 0.5 - (10. + 20. * ads).to_radians() * 0.1;
            assert!((inventory.recoil.x - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn initial_knife_emits_one_hit_after_delay_and_keeps_recovery() {
        let weapon = pistol();
        let mut knife = weapon.clone();
        knife.id = WeaponId("weapon.knife_zm".into());
        knife.mechanics.delivery = crate::origins::content::WeaponDelivery::Melee;
        knife.mechanics.melee_seconds = [0.2, 0.6];
        let mut inventory = WeaponInventory::default();
        inventory.grant(&weapon).unwrap();
        let definitions = [weapon, knife];
        inventory.tick(
            0.,
            WeaponIntent {
                melee: true,
                ..default()
            },
            &definitions,
        );
        assert!(inventory.tick(0.19, default(), &definitions).is_empty());
        let emissions = inventory.tick(0.01, default(), &definitions);
        assert_eq!(
            emissions
                .iter()
                .filter(|event| matches!(event, WeaponEmission::Melee))
                .count(),
            1
        );
        assert!(matches!(
            inventory.action,
            WeaponAction::Melee { hit: true, .. }
        ));
        assert!(inventory.tick(0.4, default(), &definitions).is_empty());
        assert_eq!(inventory.action, WeaponAction::Ready);
    }
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
                duration,
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
            raised: true,
        });
        self.cancel_action();
        self.action = WeaponAction::Equipping {
            remaining: alternate.mechanics.equip_seconds,
            duration: alternate.mechanics.equip_seconds,
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
    pub(crate) fn native_ads_requested(&self) -> bool {
        self.ads_requested
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
            WeaponAction::Reloading {
                remaining, empty, ..
            } => Some((
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

    pub fn presentation_timing(
        &self,
        definition: &WeaponDefinition,
        melee: &WeaponDefinition,
    ) -> Option<(f32, f32)> {
        self.reload_timing(definition).or_else(|| {
            match self.action {
                WeaponAction::Equipping {
                    remaining,
                    duration,
                }
                | WeaponAction::Unequipping {
                    remaining,
                    duration,
                    ..
                } => Some((duration, remaining)),
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
            raised: false,
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
            raised: false,
        });
        let pool = self.ammunition.entry(Self::group(definition)).or_default();
        pool.capacity = pool.capacity.max(definition.reserve);
        pool.rounds = pool
            .rounds
            .max(definition.mechanics.starting_reserve)
            .min(pool.capacity);
        if slot == self.active {
            self.equip_new(definition);
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
        self.active = slot;
        self.equip_new(definition);
        Ok(slot)
    }
    pub(crate) fn equip_new(&mut self, definition: &WeaponDefinition) {
        self.cancel_action();
        self.cooldown = 0.;
        let first = self.first_raise(definition);
        let duration = if first {
            definition.mechanics.first_equip_seconds
        } else {
            definition.mechanics.equip_seconds
        };
        self.action = WeaponAction::Equipping {
            remaining: duration,
            duration,
        };
        self.emit(if first {
            WeaponSignal::FirstEquip
        } else {
            WeaponSignal::Equip
        });
    }
    fn first_raise(&mut self, definition: &WeaponDefinition) -> bool {
        let Some(weapon) = self.slots[self.active].as_mut() else {
            return false;
        };
        let first = !weapon.raised
            && definition.mechanics.first_equip_seconds > 0.
            && definition
                .animation_actions
                .contains_key(&crate::origins::WeaponAnimation::FirstEquip);
        weapon.raised = true;
        first
    }
    pub fn cancel_action(&mut self) {
        self.charging = 0.;
        self.charge_level = 0;
        self.hands = default();
        self.shot_elapsed = None;
        self.action = WeaponAction::Ready;
        self.burst = 0;
        self.pending = WeaponIntent::default();
        self.emit(WeaponSignal::Idle);
    }
    pub(crate) fn suspend(&mut self) {
        if self.action != WeaponAction::Ready
            || self.signal != WeaponSignal::Idle
            || self.shot_elapsed.is_some()
            || self
                .hands
                .iter()
                .any(|h| h.reload.is_some() || h.shot_elapsed.is_some())
        {
            self.cancel_action();
        }
    }
    pub(crate) fn restore_offhand(&mut self, seconds: f32) {
        self.cancel_action();
        self.action = WeaponAction::Equipping {
            remaining: seconds,
            duration: seconds,
        };
        self.emit(WeaponSignal::QuickEquip);
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
        self.hands[usize::from(left)]
            .reload
            .as_ref()
            .map(|reload| (reload.duration, reload.remaining))
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
        let triggers = [
            (intent.pressed, intent.held),
            (intent.secondary_pressed, intent.secondary_held),
        ];
        let mut fired = false;
        let mut dry_fire = false;
        let interrupted = intent.sprint || intent.melee || intent.slot.is_some();
        for ((hand, (magazine, definition)), (pressed, held)) in self
            .hands
            .iter_mut()
            .zip(magazines.into_iter().zip(definitions))
            .zip(triggers)
        {
            let overrun = if hand.cooldown > 0. {
                (dt - hand.cooldown).max(0.)
            } else {
                0.
            };
            hand.cooldown = (hand.cooldown - dt).max(0.);
            if let Some(age) = &mut hand.shot_elapsed {
                *age += dt;
            }
            if hand.cooldown == 0.
                && matches!(hand.signal, WeaponSignal::Fire | WeaponSignal::DryFire)
            {
                hand.signal = WeaponSignal::Idle;
                hand.sequence = hand.sequence.wrapping_add(1);
            }
            if let Some(reload) = &mut hand.reload {
                reload.remaining = (reload.remaining - dt / self.reload_duration_scale).max(0.);
                if !reload.transferred
                    && reload.duration - reload.remaining + 0.000001 >= reload.transfer
                {
                    let transfer = definition
                        .magazine
                        .saturating_sub(*magazine)
                        .min(pool.rounds);
                    *magazine += transfer;
                    pool.rounds -= transfer;
                    reload.transferred = true;
                }
                if reload.remaining == 0. {
                    hand.reload = None;
                    hand.signal = WeaponSignal::Idle;
                    hand.sequence = hand.sequence.wrapping_add(1);
                    emissions.push(WeaponEmission::ReloadCompleted(definition.id.clone()));
                }
            }
            if interrupted && hand.reload.take().is_some() {
                hand.signal = WeaponSignal::Idle;
                hand.sequence = hand.sequence.wrapping_add(1);
            }
            if !interrupted
                && (intent.reload || (!intent.suspended && *magazine == 0 && hand.cooldown == 0.))
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
                let transfer = definition
                    .mechanics
                    .reload_transfer_seconds
                    .map(|timing| timing[usize::from(empty)])
                    .filter(|seconds| *seconds > 0.)
                    .unwrap_or(duration)
                    .min(duration);
                hand.reload = Some(HandReload {
                    duration,
                    remaining: duration,
                    transfer,
                    transferred: false,
                });
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
            let firing =
                pressed || held && matches!(definition.mechanics.fire, FirePattern::Automatic);
            if interrupted
                || intent.suspended
                || !firing
                || hand.reload.is_some()
                || hand.cooldown > 0.
            {
                continue;
            }
            hand.sequence = hand.sequence.wrapping_add(1);
            if *magazine == 0 {
                hand.cooldown = 0.18;
                hand.signal = WeaponSignal::DryFire;
                emissions.push(WeaponEmission::DryFire(definition.id.clone()));
                dry_fire = true;
            } else {
                *magazine -= 1;
                hand.cooldown = (definition.fire_interval * self.firing.interval_scale
                    - if held && matches!(definition.mechanics.fire, FirePattern::Automatic) {
                        overrun
                    } else {
                        0.
                    })
                .max(0.);
                hand.signal = WeaponSignal::Fire;
                hand.shot_elapsed = Some(0.);
                emissions.push(WeaponEmission::Shot(definition.id.clone()));
                fired = true;
            }
        }
        self.cooldown = self
            .hands
            .iter()
            .map(|hand| hand.cooldown)
            .fold(0., f32::max);
        if fired {
            self.emit(WeaponSignal::Fire);
        } else if dry_fire {
            self.emit(WeaponSignal::DryFire);
        }
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
        self.burst = 0;
        let duration = if self.active().is_some_and(|w| w.magazine == 0)
            && definition.mechanics.empty_unequip_seconds > 0.
            && definition
                .animation_actions
                .contains_key(&crate::origins::WeaponAnimation::UnequipEmpty)
        {
            definition.mechanics.empty_unequip_seconds
        } else {
            definition.mechanics.unequip_seconds
        };
        self.action = WeaponAction::Unequipping {
            remaining: duration,
            duration,
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
        if intent.suspended {
            self.suspend();
            self.cancel_charge();
            self.pending = default();
            intent = WeaponIntent {
                suspended: true,
                ..default()
            };
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
            intent.pressed |= self.pending.pressed && intent.held && !intent.suspended;
            intent.secondary_pressed |=
                self.pending.secondary_pressed && intent.secondary_held && !intent.suspended;
            intent.melee |= self.pending.melee;
            intent.cycle |= self.pending.cycle;
            intent.slot = intent.slot.or(self.pending.slot);
            self.pending = WeaponIntent::default();
        } else if !intent.suspended {
            self.pending.pressed |= intent.pressed;
            self.pending.secondary_pressed |= intent.secondary_pressed;
        }
        let direct_selection = intent.slot.is_some();
        intent.slot = intent.slot.filter(|&slot| {
            slot != self.active && slot < self.capacity && self.slots[slot].is_some()
        });
        if !direct_selection && intent.cycle {
            intent.slot = (1..self.capacity)
                .map(|offset| (self.active + offset) % self.capacity)
                .find(|&slot| self.slots[slot].is_some());
        }
        intent.cycle = false;
        if intent.reload
            || intent.melee
            || intent.slot.is_some()
            || self.action != WeaponAction::Ready
        {
            self.cancel_charge();
        }
        let was_cooling = self.cooldown > 0.;
        let fire_overrun = if was_cooling {
            (dt - self.cooldown).max(0.)
        } else {
            0.
        };
        self.cooldown = if self.cooldown <= dt + 0.000001 {
            0.
        } else {
            self.cooldown - dt
        };
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
        intent.reload |= !intent.suspended
            && !intent.sprint
            && self.cooldown == 0.
            && self
                .active()
                .is_some_and(|w| w.magazine == 0 && w.left_magazine.is_none())
            && self.reserve(definition) > 0
            && !(intent.ads && definition.mechanics.no_ads_auto_reload);
        let reloading = intent.reload
            && self.reserve(definition) > 0
            && self
                .active()
                .is_some_and(|w| w.magazine < definition.magazine);
        let aiming = intent.ads
            && definition.mechanics.ads_enabled
            && definition.dual_weapon.is_none()
            && self.action == WeaponAction::Ready
            && !reloading
            && !intent.melee
            && intent.slot.is_none();
        if aiming != self.ads_requested {
            self.ads_leaving = !aiming;
            emissions.push(WeaponEmission::Ads {
                weapon: definition.id.clone(),
                aiming,
            });
        }
        self.ads_requested = aiming;
        let target = if aiming { 1. } else { 0. };
        let attachment_scale = if aiming && self.attachment() == "is" {
            0.85
        } else {
            1.
        };
        let step =
            dt / (definition.mechanics.ads_seconds[if aiming { 0 } else { 1 }] * attachment_scale);
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
        let hip_recovery = definition.mechanics.recoil_recovery;
        let ads_recovery = definition
            .mechanics
            .ads_recoil_recovery
            .unwrap_or(hip_recovery);
        let recovery = hip_recovery + (ads_recovery - hip_recovery) * self.ads;
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
                let remaining = if remaining <= dt + 0.000001 {
                    0.
                } else {
                    remaining - dt
                };
                if !hit
                    && melee_definition.mechanics.melee_seconds[1]
                        .max(melee_definition.mechanics.melee_seconds[0])
                        - remaining
                        + 0.000001
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
            WeaponAction::Equipping {
                remaining,
                duration,
            } => {
                if remaining > dt {
                    self.action = WeaponAction::Equipping {
                        remaining: remaining - dt,
                        duration,
                    };
                    return emissions;
                }
                self.action = WeaponAction::Ready;
                self.emit(WeaponSignal::Idle);
            }
            WeaponAction::Unequipping {
                remaining,
                duration,
                target,
            } => {
                if remaining > dt {
                    self.action = WeaponAction::Unequipping {
                        remaining: remaining - dt,
                        duration,
                        target,
                    };
                } else if let Some(next) = self.slots[target]
                    .as_ref()
                    .and_then(|w| definitions.iter().find(|d| d.id == w.definition))
                {
                    self.active = target;
                    self.cooldown = 0.;
                    let first = self.first_raise(next);
                    let duration = if first {
                        next.mechanics.first_equip_seconds
                    } else if self.slots[target].as_ref().is_some_and(|w| w.magazine == 0)
                        && next.mechanics.empty_equip_seconds > 0.
                        && next
                            .animation_actions
                            .contains_key(&crate::origins::WeaponAnimation::EquipEmpty)
                    {
                        next.mechanics.empty_equip_seconds
                    } else {
                        next.mechanics.equip_seconds
                    };
                    self.action = WeaponAction::Equipping {
                        remaining: duration,
                        duration,
                    };
                    self.emit(if first {
                        WeaponSignal::FirstEquip
                    } else {
                        WeaponSignal::Equip
                    });
                } else {
                    self.cancel_action();
                }
                return emissions;
            }
            WeaponAction::Reloading {
                remaining,
                empty,
                mut transferred,
            } => {
                let remaining = if remaining <= reload_dt + 0.000001 {
                    0.
                } else {
                    remaining - reload_dt
                };
                if !transferred && let Some(timing) = definition.mechanics.reload_transfer_seconds {
                    let duration = if empty {
                        definition.mechanics.empty_reload
                    } else {
                        definition.reload_seconds
                    };
                    let transfer = if timing[usize::from(empty)] > 0. {
                        timing[usize::from(empty)].min(duration)
                    } else {
                        duration
                    };
                    if duration - remaining + 0.000001 >= transfer {
                        self.insert(definition, u32::MAX);
                        transferred = true;
                    }
                }
                if intent.sprint || intent.melee || intent.slot.is_some() {
                    self.cancel_action();
                } else if remaining > 0. {
                    self.action = WeaponAction::Reloading {
                        remaining,
                        empty,
                        transferred,
                    };
                    return emissions;
                } else {
                    if !transferred {
                        self.insert(definition, u32::MAX);
                    }
                    emissions.push(WeaponEmission::ReloadCompleted(definition.id.clone()));
                    self.action = WeaponAction::Ready;
                    self.emit(WeaponSignal::Idle);
                }
            }
            WeaponAction::ShellReload {
                remaining,
                phase,
                mut inserted,
            } => {
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
                if phase != ShellPhase::End
                    && !inserted
                    && duration - remaining + 0.000001 >= at.min(duration)
                {
                    self.insert(definition, 1);
                    emissions.push(WeaponEmission::ReloadCompleted(definition.id.clone()));
                    inserted = true;
                }
                if intent.sprint {
                    self.cancel_action();
                    return emissions;
                }
                let interrupt = intent.slot.is_some()
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
        intent.pressed |= std::mem::take(&mut self.pending.pressed) && intent.held;
        intent.secondary_pressed |=
            std::mem::take(&mut self.pending.secondary_pressed) && intent.secondary_held;
        if let Some(partner) = definition
            .dual_weapon
            .as_ref()
            .and_then(|id| definitions.iter().find(|w| w.id == *id))
        {
            self.tick_dual(dt, intent, [definition, partner], &mut emissions);
            if !intent.melee && intent.slot.is_none() {
                return emissions;
            }
        }
        if let Some(slot) = intent.slot {
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
        if intent.reload
            && self.reserve(definition) > 0
            && self
                .active()
                .is_some_and(|w| w.magazine < definition.magazine)
        {
            self.burst = 0;
            let empty = self.active().is_some_and(|w| w.magazine == 0);
            let duration = if definition.mechanics.shell_reload {
                definition.mechanics.reload_start_seconds
            } else if empty {
                definition.mechanics.empty_reload
            } else {
                definition.reload_seconds
            };
            emissions.push(WeaponEmission::ReloadStarted {
                weapon: definition.id.clone(),
                magazine: self.active().map_or(0, |w| w.magazine),
                capacity: definition.magazine,
                seconds: duration
                    * self.reload_duration_scale
                    * if self.attachment() == "dualclip" {
                        0.675
                    } else {
                        1.
                    },
            });
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
                    transferred: false,
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
