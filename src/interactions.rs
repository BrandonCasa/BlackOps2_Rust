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
    walls: Query<(), With<Usable>>,
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
                let mut source = source.clone();
                // Reconstruct the native actor probe fill without changing shared
                // first-person materials or duplicating the world's baked lights.
                source.emissive_texture = source.base_color_texture.clone();
                source.emissive = LinearRgba::rgb(90., 90., 90.);
                source.emissive_exposure_weight = 1.;
                let handle = materials.add(source);
                cache.0.insert(material.0.id(), handle.clone());
                handle
            };
            commands.entity(entity).insert(MeshMaterial3d(handle));
        }
    }
}
fn interact(
    time: Res<Time<Fixed>>,
    input: Res<GameInput>,
    definition: Res<GameDefinition>,
    query: SpatialQuery,
    targets: Query<(Entity, &Usable, &Transform), Without<Player>>,
    mut players: Query<(Entity, &Player, &Transform, &Vitality, &mut WeaponInventory)>,
    mut focus: ResMut<InteractionFocus>,
    mut ledger: ResMut<PointLedger>,
    mut session: ResMut<Session>,
) {
    let Ok((player_entity, player, body, vitality, mut inventory)) = players.single_mut() else {
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
    let eye = if player.crouched {
        player.height * 0.8
    } else {
        definition.tuning.eye_height
    };
    let origin = body.translation + Vec3::Y * (eye - player.height * 0.5);
    let forward = Quat::from_rotation_y(player.yaw + inventory.recoil.x)
        * Quat::from_rotation_x((player.pitch + inventory.recoil.y).clamp(-1.5, 1.5))
        * Vec3::NEG_Z;
    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);
    let candidate = targets
        .iter()
        .filter_map(|(entity, usable, transform)| {
            let delta = transform.translation - origin;
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
    let cost = if owned.is_some() {
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
        if inventory.action != WeaponAction::Ready {
            return Err(InteractionRejection::Busy);
        }
        if owned.is_some() {
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
