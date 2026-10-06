//! Fixed-step aiming/reach checks and one accepted action per held-use gesture.
use crate::{
    GameState,
    assets::GameDefinition,
    economy::{PointLedger, PurchaseError},
    gameplay::{GameStep, Session, SessionEntity},
    input::GameInput,
    life::{LifeState, Vitality},
    origins::content::{InteractionAction, InteractionPlacement},
    player::Player,
    weapons::{InventoryError, WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use std::collections::HashMap;

#[derive(Component)]
pub struct Usable {
    pub source: InteractionPlacement,
}
#[derive(Resource, Default)]
pub struct InteractionFocus {
    pub target: Option<Entity>,
    pub progress: f32,
    pub prompt: String,
    pub rejection: Option<InteractionRejection>,
    latched: bool,
    pub accepted: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionRejection {
    Busy,
    Downed,
    InsufficientPoints,
    FullAmmo,
    Unavailable,
    Duplicate,
}
pub struct InteractionPlugin;
#[derive(Resource, Default)]
struct WallMaterials(HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>);
impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InteractionFocus>()
            .init_resource::<WallMaterials>()
            .init_resource::<PointLedger>()
            .add_systems(OnEnter(GameState::Playing), assemble)
            .add_observer(wall_ready)
            .add_systems(
                FixedUpdate,
                interact
                    .in_set(GameStep::Interactions)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn assemble(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    existing: Query<(), With<Usable>>,
    mut focus: ResMut<InteractionFocus>,
    mut ledger: ResMut<PointLedger>,
    players: Query<(), With<Player>>,
    server: Res<AssetServer>,
) {
    if !players.is_empty() || !existing.is_empty() {
        return;
    }
    *focus = InteractionFocus::default();
    *ledger = PointLedger::default();
    for source in &definition.content.interactions {
        let Some(placement) = definition
            .content
            .placements
            .iter()
            .find(|p| p.id == source.placement)
        else {
            continue;
        };
        let mut entity = commands.spawn((
            Usable {
                source: source.clone(),
            },
            SessionEntity,
            Transform::from_translation(Vec3::from_array(placement.position))
                .with_rotation(Quat::from_rotation_y(placement.yaw)),
            Visibility::Inherited,
        ));
        if let InteractionAction::Perk { perk } = &source.action {
            let scene = definition
                .content
                .perks
                .iter()
                .find(|p| p.id == *perk)
                .and_then(|p| p.presentation.as_ref())
                .and_then(|id| {
                    definition
                        .content
                        .presentations
                        .iter()
                        .find(|p| p.id == *id)
                })
                .and_then(|p| p.scene.as_ref());
            if let Some(scene) = scene {
                entity.insert(WorldAssetRoot(server.load(scene.clone())));
            }
            entity.with_children(|parent| {
                parent.spawn((
                    RigidBody::Static,
                    Collider::cuboid(1.66, 2.17, 0.9),
                    Transform::from_xyz(-0.203, 1.085, 0.152),
                ));
            });
        }
        if let InteractionAction::Weapon { weapon, .. } = &source.action {
            let scene = definition
                .content
                .weapons
                .iter()
                .find(|w| w.id == *weapon)
                .and_then(|w| w.pickup.as_ref())
                .and_then(|id| {
                    definition
                        .content
                        .presentations
                        .iter()
                        .find(|p| p.id == *id)
                })
                .and_then(|p| p.scene.as_ref());
            if let Some(scene) = scene {
                entity.insert(WorldAssetRoot(server.load(scene.clone())));
            }
        }
    }
}
fn wall_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    walls: Query<(), Or<(With<Usable>, With<crate::access::DoorProp>)>>,
    children: Query<&Children>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>, With<Mesh3d>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<WallMaterials>,
) {
    if !walls.contains(event.entity) {
        return;
    }
    for entity in children.iter_descendants(event.entity) {
        if let Ok(material) = meshes.get(entity) {
            let handle = if let Some(handle) = cache.0.get(&material.0.id()) {
                handle.clone()
            } else {
                let Some(source) = materials.get(&material.0) else {
                    continue;
                };
                let source = crate::rendering::with_actor_fill(source);
                let handle = materials.add(source);
                cache.0.insert(material.0.id(), handle.clone());
                handle
            };
            commands.entity(entity).insert(MeshMaterial3d(handle));
        }
    }
}
pub(crate) fn interact(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    input: Res<GameInput>,
    definition: Res<GameDefinition>,
    query: SpatialQuery,
    targets: Query<(Entity, &Usable, &Transform), Without<Player>>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut WeaponInventory,
        &mut crate::equipment::GrenadeInventory,
        &mut crate::mines::MineInventory,
        &mut crate::equipment::TacticalInventory,
        &mut crate::solo_revive::PerkInventory,
        Option<&crate::perks::PerkDrinking>,
    )>,
    mut focus: ResMut<InteractionFocus>,
    mut ledger: ResMut<PointLedger>,
    mut session: ResMut<Session>,
    mut access: ResMut<crate::access::ZoneAccess>,
    blockers: Query<(Entity, &crate::access::DoorBlocker)>,
) {
    let Ok((
        player_entity,
        player,
        body,
        vitality,
        mut inventory,
        mut grenades,
        mut mines,
        mut tactical,
        perks,
        drinking,
    )) = players.single_mut()
    else {
        return;
    };
    if !input.use_held {
        focus.latched = false;
        focus.progress = 0.;
    }
    if vitality.state() != LifeState::Standing {
        focus.target = None;
        focus.progress = 0.;
        focus.prompt.clear();
        focus.rejection = Some(InteractionRejection::Downed);
        return;
    }
    let eye = player.eye_height;
    let origin = body.translation + Vec3::Y * (eye - player.height * 0.5);
    let forward = Quat::from_rotation_y(player.yaw + inventory.recoil.x)
        * Quat::from_rotation_x((player.pitch + inventory.recoil.y).clamp(-1.5, 1.5))
        * Vec3::NEG_Z;
    let filter = crate::environment::solids([player_entity]);
    let candidate = targets
        .iter()
        .filter_map(|(entity, usable, transform)| {
            if matches!(&usable.source.action, InteractionAction::Door { connection, .. } if access.opened(connection)) { return None; }
            if matches!(&usable.source.action, InteractionAction::Perk { perk } if *perk == definition.solo_revive.perk)
                && (perks.solo_charges > 0 || perks.solo_purchases >= definition.solo_revive.maximum_purchases) { return None; }
            let endpoint = if matches!(usable.source.action, InteractionAction::Perk { .. }) { transform.translation + Vec3::Y + transform.rotation * Vec3::new(0., 0., 0.8) } else { transform.translation };
            let filter = if let InteractionAction::Door { connection, .. } = &usable.source.action {
                crate::environment::solids(std::iter::once(player_entity).chain(blockers.iter().filter(|(_,b)|b.0==*connection).map(|(e,_)|e)))
            } else {filter.clone()};
            let delta = endpoint - origin;
            let distance = delta.length();
            if distance > usable.source.reach || distance < 0.01 {
                return None;
            }
            let direction = Dir3::new(delta).ok()?;
            let dot = forward.dot(direction.as_vec3());
            if dot < 0.85
                || query
                    .cast_ray(origin, direction, (distance - 0.12).max(0.), true, &filter)
                    .is_some()
            {
                return None;
            }
            Some((dot, distance, entity, usable))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.total_cmp(&a.1)));
    let Some((_, _, entity, usable)) = candidate else {
        focus.target = None;
        focus.progress = 0.;
        focus.prompt.clear();
        return;
    };
    if focus.target != Some(entity) {
        focus.progress = 0.;
        focus.rejection = None;
    }
    focus.target = Some(entity);
    if let InteractionAction::Door { connection, price } = &usable.source.action {
        let debris = definition
            .access
            .doors
            .iter()
            .find(|d| d.connection == *connection)
            .is_some_and(|d| d.debris);
        focus.prompt = if usable.source.hidden_prompt {
            String::new()
        } else {
            format!(
                "Press F to {} [Cost: {}]",
                if debris { "clear debris" } else { "open door" },
                price
            )
        };
        if !input.use_held || focus.latched {
            return;
        }
        focus.progress += time.delta_secs();
        if focus.progress < usable.source.hold_seconds {
            return;
        }
        focus.latched = true;
        focus.progress = 0.;
        let purchase = ledger.command(*price);
        match ledger.accept(&mut session.points, purchase, || {
            access
                .unlock(&definition, connection)
                .map_err(|()| InteractionRejection::Unavailable)
        }) {
            Ok(()) => {
                focus.accepted += 1;
                focus.rejection = None;
                focus.prompt.clear();
            }
            Err(PurchaseError::InsufficientPoints) => {
                focus.rejection = Some(InteractionRejection::InsufficientPoints)
            }
            Err(PurchaseError::Duplicate) => {
                focus.rejection = Some(InteractionRejection::Duplicate)
            }
            Err(PurchaseError::Rejected(reason)) => focus.rejection = Some(reason),
        }
        return;
    }
    if let InteractionAction::Perk { perk } = &usable.source.action {
        let Some(perk) = definition.content.perks.iter().find(|p| p.id == *perk) else {
            return;
        };
        if perk.id != definition.solo_revive.perk && !perk.has_supported_effect() {
            focus.prompt.clear();
            return;
        }
        focus.prompt = format!("Press F to buy {} [{}]", perk.name, perk.price);
        if !input.use_held || focus.latched {
            return;
        }
        focus.progress += time.delta_secs();
        if focus.progress < usable.source.hold_seconds {
            return;
        }
        focus.latched = true;
        let purchase = ledger.command(perk.price);
        let result = ledger.accept(&mut session.points, purchase, || {
            if inventory.action != WeaponAction::Ready
                || grenades.busy()
                || mines.busy()
                || tactical.busy()
                || drinking.is_some()
            {
                return Err(InteractionRejection::Busy);
            }
            if perk.id == definition.solo_revive.perk {
                perks.can_buy_solo(&definition.solo_revive)?;
            } else if perks.owned.contains(&perk.id) {
                return Err(InteractionRejection::Duplicate);
            }
            let drink = crate::perks::PerkDrinking::new(perk, &inventory, &definition)?;
            inventory.cancel_action();
            commands.entity(player_entity).insert(drink);
            Ok(())
        });
        focus.rejection = match result {
            Ok(()) => {
                focus.accepted += 1;
                None
            }
            Err(PurchaseError::InsufficientPoints) => {
                Some(InteractionRejection::InsufficientPoints)
            }
            Err(PurchaseError::Duplicate) => Some(InteractionRejection::Duplicate),
            Err(PurchaseError::Rejected(reason)) => Some(reason),
        };
        return;
    }
    let InteractionAction::Weapon {
        weapon,
        price,
        ammo_price,
    } = &usable.source.action
    else {
        focus.prompt.clear();
        focus.rejection = Some(InteractionRejection::Unavailable);
        return;
    };
    let Some(weapon) = definition.content.weapons.iter().find(|w| w.id == *weapon) else {
        return;
    };
    let owned = inventory
        .owned()
        .find(|(_, w)| w.definition == weapon.id)
        .map(|(slot, _)| slot);
    let cost = if weapon.throwable.is_some() {
        *price
    } else if owned.is_some() {
        ammo_price.unwrap_or(price / 2)
    } else {
        *price
    };
    focus.prompt = if usable.source.hidden_prompt {
        String::new()
    } else {
        format!(
            "{} F to buy {}{} [{}]",
            if usable.source.hold_seconds > 0. {
                "Hold"
            } else {
                "Press"
            },
            weapon.name,
            if owned.is_some() { " ammo" } else { "" },
            cost
        )
    };
    if !input.use_held || focus.latched {
        return;
    }
    focus.progress += time.delta_secs();
    if focus.progress < usable.source.hold_seconds {
        return;
    }
    focus.latched = true;
    let purchase = ledger.command(cost);
    let result = ledger.accept(&mut session.points, purchase, || {
        if inventory.action != WeaponAction::Ready
            || grenades.busy()
            || mines.busy()
            || tactical.busy()
            || drinking.is_some()
        {
            return Err(InteractionRejection::Busy);
        }
        if weapon.mine.is_some() {
            mines.receive(weapon)?;
        } else if weapon.throwable.is_some() {
            let result = if weapon
                .throwable
                .as_ref()
                .is_some_and(|t| t.slot == crate::equipment::ThrowableSlot::Tactical)
            {
                tactical.receive(weapon)
            } else {
                grenades.receive(weapon)
            };
            result.map_err(|e| match e {
                crate::equipment::EquipmentError::Busy => InteractionRejection::Busy,
                crate::equipment::EquipmentError::Full => InteractionRejection::FullAmmo,
                crate::equipment::EquipmentError::Unsupported => InteractionRejection::Unavailable,
            })?;
        } else if owned.is_some() {
            if !inventory.refill_weapon(weapon) {
                return Err(InteractionRejection::FullAmmo);
            }
        } else {
            inventory.receive(weapon).map_err(|error| match error {
                InventoryError::Busy => InteractionRejection::Busy,
                _ => InteractionRejection::Unavailable,
            })?;
        }
        Ok(())
    });
    focus.rejection = match result {
        Ok(()) => {
            focus.accepted += 1;
            None
        }
        Err(PurchaseError::InsufficientPoints) => Some(InteractionRejection::InsufficientPoints),
        Err(PurchaseError::Duplicate) => Some(InteractionRejection::Duplicate),
        Err(PurchaseError::Rejected(reason)) => Some(reason),
    };
}
