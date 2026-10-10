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
    pub hold_seconds: f32,
    pub prompt: String,
    pub rejection: Option<InteractionRejection>,
    latched: bool,
    pub accepted: u32,
}
impl InteractionFocus {
    fn cancel(&mut self) {
        self.target = None;
        self.progress = 0.;
        self.hold_seconds = 0.;
        self.prompt.clear();
        self.rejection = None;
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractionRejection {
    Busy,
    Downed,
    InsufficientPoints,
    FullAmmo,
    PerkLimit,
    NoPower,
    Unavailable,
    Duplicate,
}
impl From<PurchaseError<Self>> for InteractionRejection {
    fn from(error: PurchaseError<Self>) -> Self {
        match error {
            PurchaseError::InsufficientPoints => Self::InsufficientPoints,
            PurchaseError::Duplicate => Self::Duplicate,
            PurchaseError::Rejected(reason) => reason,
        }
    }
}
pub struct InteractionPlugin;
#[derive(Resource, Default)]
struct WallMaterials(HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>);
impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InteractionFocus>()
            .init_resource::<WallMaterials>()
            .init_resource::<PointLedger>()
            .add_systems(
                OnEnter(GameState::Playing),
                assemble.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(OnExit(GameState::Playing), clear_focus)
            .add_systems(
                FixedUpdate,
                suppress_focus
                    .after(GameStep::PlayerLife)
                    .before(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_observer(wall_ready)
            .add_systems(
                FixedUpdate,
                interact
                    .in_set(GameStep::Interactions)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn clear_focus(mut focus: ResMut<InteractionFocus>) {
    focus.cancel();
    focus.latched = false;
}
fn suppress_focus(
    mut focus: ResMut<InteractionFocus>,
    input: Res<GameInput>,
    players: Query<&Vitality, With<Player>>,
    targets: Query<&Usable>,
) {
    if !players
        .single()
        .is_ok_and(|v| v.state() == LifeState::Standing)
    {
        focus.cancel();
        focus.latched = input.use_held;
        focus.rejection = Some(InteractionRejection::Downed);
    } else if focus
        .target
        .and_then(|e| targets.get(e).ok())
        .is_some_and(|u| u.source.hidden_prompt)
    {
        focus.prompt.clear();
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
            placement.id.clone(),
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
                entity.insert((
                    WorldAssetRoot(server.load(scene.clone())),
                    crate::presentation::camo::WeaponMaterialContext(weapon.clone()),
                ));
            }
        }
    }
}
fn wall_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    walls: Query<
        (),
        Or<(
            With<Usable>,
            With<crate::access::DoorProp>,
            With<crate::barriers::BoardVisual>,
            With<crate::generators::GeneratorVisual>,
        )>,
    >,
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
                let source = source.clone();
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
    settings: Res<crate::input::ControlSettings>,
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
        &crate::solo_revive::PerkInventory,
        Has<crate::perks::PerkDrinking>,
    )>,
    mut focus: ResMut<InteractionFocus>,
    mut ledger: ResMut<PointLedger>,
    mut session: ResMut<Session>,
    mut access: ResMut<crate::access::ZoneAccess>,
    blockers: Query<(Entity, &crate::access::DoorBlocker)>,
    mut barriers: ResMut<crate::barriers::Barriers>,
    windows: Query<(Entity, &crate::barriers::BarrierBlocker)>,
    mut generators: ResMut<crate::generators::Generators>,
) {
    let use_key = settings.label(crate::input::Action::Interact);
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
        focus.cancel();
        return;
    };
    if !input.use_held {
        focus.latched = false;
        focus.progress = 0.;
    }
    if vitality.state() != LifeState::Standing {
        focus.cancel();
        focus.latched = input.use_held;
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
            if matches!(&usable.source.action, InteractionAction::Generator { generator } if !generators.available(generator)) { return None; }
            if matches!(&usable.source.action, InteractionAction::Barrier { barrier } if !barriers.repairable(barrier)) {return None;}
            if matches!(&usable.source.action, InteractionAction::Door { connection, .. } if access.opened(connection)) { return None; }
            if matches!(&usable.source.action, InteractionAction::Perk { perk } if *perk == definition.solo_revive.perk)
                && (perks.solo_charges > 0 || perks.solo_purchases >= definition.solo_revive.maximum_purchases) { return None; }
            if matches!(usable.source.action, InteractionAction::Generator { .. }) && origin.distance(transform.translation+Vec3::Y*usable.source.reach*0.5)>usable.source.reach {return None;}
            let endpoint = if let InteractionAction::Barrier { barrier } = &usable.source.action {
                let d = definition.content.barriers.iter().find(|d| d.id == *barrier)?;
                let exit = definition.content.navigation_nodes.iter().find(|n| n.id == d.exit)?;
                let inward = (Vec3::from_array(exit.position) - transform.translation).with_y(0.).normalize_or_zero();
                transform.translation + inward * 0.8 + Vec3::Y * 0.7
            } else if matches!(usable.source.action, InteractionAction::Generator { .. }) {crate::generators::use_point(transform.translation,origin,usable.source.reach)} else if matches!(usable.source.action, InteractionAction::Perk { .. }) { transform.translation + Vec3::Y + transform.rotation * Vec3::new(0., 0., 0.8) } else { transform.translation };
            let filter = if let InteractionAction::Door { connection, .. } = &usable.source.action {
                crate::environment::solids(std::iter::once(player_entity).chain(blockers.iter().filter(|(_,b)|b.0==*connection).map(|(e,_)|e)))
            } else if let InteractionAction::Barrier { barrier }=&usable.source.action {
                crate::environment::solids(std::iter::once(player_entity).chain(windows.iter().filter(|(_,b)|b.0==*barrier).map(|(e,_)|e)))
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
        focus.cancel();
        return;
    };
    if focus.target != Some(entity) {
        focus.progress = 0.;
        focus.rejection = None;
    }
    focus.target = Some(entity);
    focus.hold_seconds = usable.source.hold_seconds;
    let busy = inventory.action != WeaponAction::Ready
        || grenades.busy()
        || mines.busy()
        || tactical.busy()
        || drinking;
    if let InteractionAction::Generator { generator } = &usable.source.action {
        let Some(d) = definition
            .content
            .generators
            .iter()
            .find(|d| d.id == *generator)
        else {
            return;
        };
        focus.prompt = if generators.busy() {
            "Generator capture in progress".into()
        } else {
            format!(
                "Press {use_key} to activate Generator {} [Cost: {}]",
                d.number, d.price
            )
        };
        if !input.use_held || focus.latched {
            return;
        }
        focus.latched = true;
        let purchase = ledger.command(d.price);
        let result = ledger.accept(&mut session.points, purchase, || {
            if busy {
                return Err(InteractionRejection::Busy);
            }
            generators.activate(generator, player_entity)
        });
        focus.rejection = match result {
            Ok(()) => {
                focus.accepted += 1;
                None
            }
            Err(error) => Some(error.into()),
        };
        return;
    }
    if let InteractionAction::Barrier { barrier } = &usable.source.action {
        focus.prompt = format!("Hold {use_key} to rebuild barrier");
        if !input.use_held {
            return;
        }
        if busy || input.fire_held {
            focus.progress = 0.;
            focus.rejection = Some(InteractionRejection::Busy);
            return;
        }
        focus.rejection = None;
        focus.progress += time.delta_secs();
        if focus.progress >= usable.source.hold_seconds {
            focus.progress = 0.;
            if let Some(index) = barriers.repair(barrier, player_entity) {
                let source = barriers.source(&definition, barrier, index);
                crate::audio::emit(&mut commands, "zmb_boards_float", source);
                crate::audio::emit(&mut commands, "zmb_repair_boards", source);
                focus.accepted += 1;
                focus.rejection = None;
            }
        }
        return;
    }
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
                "Press {use_key} to {} [Cost: {}]",
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
                let position = if debris {
                    targets
                        .get(entity)
                        .map_or(origin, |(_, _, pose)| pose.translation)
                } else {
                    definition
                        .access
                        .doors
                        .iter()
                        .find(|d| d.connection == *connection)
                        .and_then(|d| d.props.first())
                        .map_or(origin, |p| Vec3::from_array(p.position))
                };
                crate::audio::emit(
                    &mut commands,
                    "zmb_cha_ching",
                    crate::audio::SoundSource::World(position),
                );
                focus.accepted += 1;
                focus.rejection = None;
                focus.prompt.clear();
            }
            Err(error) => focus.rejection = Some(error.into()),
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
        focus.prompt = format!("Press {use_key} to buy {} [{}]", perk.name, perk.price);
        if !input.use_held || focus.latched {
            return;
        }
        if busy {
            focus.progress = 0.;
            focus.rejection = Some(InteractionRejection::Busy);
            return;
        }
        focus.rejection = None;
        focus.progress += time.delta_secs();
        if focus.progress < usable.source.hold_seconds {
            return;
        }
        focus.latched = true;
        focus.progress = 0.;
        let purchase = ledger.command(perk.price);
        let result = ledger.accept(&mut session.points, purchase, || {
            if perk.id == definition.solo_revive.perk {
                perks.can_buy_solo(&definition.solo_revive)?;
            } else if perks.owned.contains(&perk.id) {
                return Err(InteractionRejection::Duplicate);
            }
            if perk
                .generator
                .as_ref()
                .is_some_and(|generator| !generators.powered(generator))
            {
                return Err(InteractionRejection::NoPower);
            }
            if perks.owned.len() >= 4 {
                return Err(InteractionRejection::PerkLimit);
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
            Err(error) => Some(error.into()),
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
            "{} {use_key} to buy {}{} [{}]",
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
    if busy {
        focus.progress = 0.;
        focus.rejection = Some(InteractionRejection::Busy);
        return;
    }
    focus.rejection = None;
    focus.progress += time.delta_secs();
    if focus.progress < usable.source.hold_seconds {
        return;
    }
    focus.latched = true;
    focus.progress = 0.;
    let purchase = ledger.command(cost);
    let result = ledger.accept(&mut session.points, purchase, || {
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
            crate::audio::binding(
                &mut commands,
                &definition,
                &format!(
                    "{}.{}",
                    weapon.id.0,
                    if owned.is_some() { "ammo" } else { "purchase" }
                ),
                crate::audio::SoundSource::attached(player_entity),
            );
            crate::audio::emit(
                &mut commands,
                "zmb_cha_ching",
                crate::audio::SoundSource::attached(player_entity),
            );
            crate::audio::voice(
                &mut commands,
                &definition,
                &session,
                "generic_wall_buy",
                player_entity,
                session.kills,
            );
            None
        }
        Err(error) => Some(error.into()),
    };
}
