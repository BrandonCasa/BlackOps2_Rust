use super::*;
use crate::equipment::{GrenadeInventory, TacticalInventory, ThrowState};
use crate::mines::MineInventory;

pub(super) type Owners<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static WeaponInventory,
        &'static GrenadeInventory,
        &'static TacticalInventory,
        &'static MineInventory,
        Option<&'static crate::perks::PerkDrinking>,
        Option<&'static crate::crafting::BuildableInventory>,
    ),
    With<Player>,
>;
pub(super) type Inventories<'w, 's> = Query<
    'w,
    's,
    (
        &'static WeaponInventory,
        &'static GrenadeInventory,
        &'static TacticalInventory,
        &'static MineInventory,
        Option<&'static crate::perks::PerkDrinking>,
        Option<&'static crate::crafting::BuildableInventory>,
    ),
>;
pub(crate) struct Selection {
    pub(crate) weapon: WeaponId,
    pub(crate) action: WeaponAnimation,
    pub(crate) timing: Option<(f32, f32)>,
}
pub(super) fn owned(
    inventory: &WeaponInventory,
    lethal: &GrenadeInventory,
    tactical: &TacticalInventory,
    mines: &MineInventory,
) -> Vec<WeaponId> {
    let mut ids: Vec<_> = inventory
        .owned()
        .map(|(_, w)| w.definition.clone())
        .collect();
    ids.push(lethal.definition.clone());
    if let Some(t) = &tactical.0 {
        ids.push(t.definition.clone());
    }
    if let Some(id) = &mines.definition {
        ids.push(id.clone());
    }
    ids.sort();
    ids.dedup();
    ids
}
pub(crate) fn selection(
    lethal: &GrenadeInventory,
    tactical: &TacticalInventory,
    mines: &MineInventory,
    definition: &GameDefinition,
) -> Option<Selection> {
    for inventory in std::iter::once(lethal).chain(tactical.0.iter()) {
        if !inventory.busy() {
            continue;
        }
        let profile = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == inventory.definition)?
            .throwable
            .as_ref()?;
        let (action, timing) = match inventory.state {
            ThrowState::Priming { remaining } => (
                WeaponAnimation::Prime,
                Some((profile.prime_seconds, remaining)),
            ),
            ThrowState::Cooking { .. } => (WeaponAnimation::Idle, None),
            ThrowState::Releasing { remaining, .. } => (
                WeaponAnimation::Fire,
                Some((
                    profile.throw_seconds,
                    profile.throw_seconds - profile.release_delay + remaining,
                )),
            ),
            ThrowState::Recovery { remaining } => (
                WeaponAnimation::Fire,
                Some((profile.throw_seconds, remaining)),
            ),
            ThrowState::Ready => continue,
        };
        return Some(Selection {
            weapon: inventory.definition.clone(),
            action,
            timing,
        });
    }
    let (placing, remaining) = mines.presentation_phase()?;
    let id = mines.definition.as_ref()?;
    let profile = definition
        .content
        .weapons
        .iter()
        .find(|w| w.id == *id)?
        .mine
        .as_ref()?;
    Some(Selection {
        weapon: id.clone(),
        action: if placing {
            WeaponAnimation::Prime
        } else {
            WeaponAnimation::Fire
        },
        timing: Some((
            if placing {
                profile.place_seconds
            } else {
                profile.recovery_seconds
            },
            remaining,
        )),
    })
}
pub(super) fn buildable_selection(
    equipment: Option<&crate::crafting::BuildableInventory>,
) -> Option<Selection> {
    let equipment = equipment.filter(|e| e.shield_held)?;
    Some(Selection {
        weapon: WeaponId("weapon.tomb_shield_zm".into()),
        action: if equipment.shield_attack > 0. {
            WeaponAnimation::Melee
        } else {
            WeaponAnimation::Idle
        },
        timing: None,
    })
}
pub(super) fn select_visible(
    definition: Option<Res<GameDefinition>>,
    owners: Owners,
    mut views: Query<(&FirstPerson, &mut Visibility, &mut Transform)>,
    mut motions: Query<(
        &crate::player::Player,
        &crate::movement::SprintState,
        &crate::movement::Mobility,
        &mut crate::movement::MotionFeedback,
    )>,
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    state: Res<State<GameState>>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (owner, primary, lethal, tactical, mines, drinking, buildables) in &owners {
        let selected = drinking
            .map(|d| d.selection())
            .or_else(|| buildable_selection(buildables))
            .or_else(|| selection(lethal, tactical, mines, &definition));
        let visible = selected
            .as_ref()
            .map(|s| &s.weapon)
            .or_else(|| primary.active().map(|w| &w.definition));
        let weapon = visible.and_then(|id| definition.content.weapons.iter().find(|w| &w.id == id));
        let feedback = if let (Some(weapon), Ok((player, sprint, mobility, mut motion))) =
            (weapon, motions.get_mut(owner))
        {
            motion.weapon_pose(
                player,
                sprint,
                mobility,
                &weapon.mechanics.view_motion,
                if selected.is_some() { 0. } else { primary.ads },
                if **state == GameState::Playing {
                    time.delta_secs()
                } else {
                    0.
                },
                fixed.overstep_fraction(),
            )
        } else {
            Transform::IDENTITY
        };
        for (view, mut visibility, mut pose) in &mut views {
            if view.owner == owner {
                if let Some(weapon) = definition
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id == view.weapon)
                {
                    pose.translation = Vec3::from_array(weapon.view_offset) + feedback.translation;
                    pose.rotation =
                        feedback.rotation * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
                }
                *visibility = if visible == Some(&view.weapon) {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}
