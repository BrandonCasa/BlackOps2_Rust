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
                OnEnter(GameState::PreparingShaders),
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
        if definition.delivery_scope == crate::origins::DeliveryScope::FirstPlayableSolo
            && !matches!(&source.action, InteractionAction::Barrier { .. })
            && !matches!(&source.action, InteractionAction::Perk { perk } if *perk == definition.solo_revive.perk)
            && !matches!(&source.action, InteractionAction::Generator { generator } if definition.content.perks.iter().any(|perk|perk.id==definition.solo_revive.perk && perk.generator.as_ref()==Some(generator)))
        {
            continue;
        }
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
            if source.source_trigger.is_some() {
                if let Some(profile) = definition.content.perks.iter().find(|p|p.id==*perk) {
                    entity.insert(crate::perks::PerkMachine::new(perk.clone()));
                    entity.with_children(|parent| {
                        for volume in &profile.collision {
                            let layers = (if volume.contents & 1 != 0 {1}else{0})
                                | (if volume.contents & 0x10000 != 0 {crate::environment::PLAYER_INGRESS_LAYER}else{0})
                                | (if volume.contents & 0x20000 != 0 {crate::environment::MONSTER_CLIP_LAYER}else{0});
                            let memberships = crate::assets::world_collision_layers(volume.contents).memberships
                                & LayerMask::from(crate::assets::RAGDOLL_WORLD_LAYER | crate::assets::DYNENT_WORLD_LAYER);
                            if let Some(shape)=Collider::convex_hull(&volume.vertices.iter().copied().map(Vec3::from_array).collect::<Vec<_>>()) {
                                parent.spawn((crate::perks::PerkMachineClip,RigidBody::Static,shape,CollisionLayers::new(LayerMask::from(layers) | memberships,LayerMask::ALL),Transform::from_translation(Vec3::from_array(volume.position))));
                            }
                        }
                    });
                }
            } else {
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
    mut barriers: ResMut<crate::barriers::Barriers>,
    (repair_triggers, player_shapes): (
        Query<&crate::barriers::BarrierRepairTrigger>,
        Query<&Collider, With<Player>>,
    ),
    mut generators: ResMut<crate::generators::Generators>,
    (native_triggers, client_modes, mut trigger_queue, mut native_flags): (
        Query<(
            Entity,
            &crate::environment::triggers::MapTrigger,
            &Collider,
            &Transform,
            &crate::environment::triggers::TriggerState,
        )>,
        Query<&crate::access::TriggerClientMode>,
        ResMut<crate::environment::triggers::TriggerQueue>,
        MessageWriter<crate::access::ZoneFlagSet>,
    ),
) {
    let use_key = settings.label(crate::input::Action::Interact);
    let barrier_use_held = barriers.update_use(input.use_held, time.delta_secs());
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
    let barrier_contacts = player_shapes
        .get(player_entity)
        .map(|shape| {
            query
                .shape_intersections(
                    shape,
                    body.translation,
                    body.rotation,
                    &SpatialQueryFilter::from_mask(crate::barriers::REPAIR_TRIGGER_LAYER),
                )
                .into_iter()
                .filter_map(|entity| {
                    repair_triggers
                        .get(entity)
                        .ok()
                        .map(|trigger| trigger.0.clone())
                })
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    let candidate = targets
        .iter()
        .filter_map(|(entity, usable, transform)| {
            if matches!(&usable.source.action, InteractionAction::Generator { generator } if !generators.available(generator)) { return None; }
            if matches!(&usable.source.action, InteractionAction::Barrier { barrier } if !barriers.repairable(barrier)) {return None;}
            if let InteractionAction::Barrier { barrier } = &usable.source.action {
                if !barrier_contacts.contains(barrier) {return None;}
                let distance = origin.distance_squared(transform.translation);
                return Some((-distance,distance,entity,usable,None));
            }
            if matches!(&usable.source.action, InteractionAction::Door { connection, .. } if access.opened(connection)) { return None; }
            if matches!(&usable.source.action, InteractionAction::Perk { perk } if *perk == definition.solo_revive.perk)
                && (perks.solo_charges > 0 || perks.solo_purchases >= definition.solo_revive.maximum_purchases) { return None; }
            if let Some(id) = &usable.source.source_trigger {
                let (trigger,_,shape,pose,state) = native_triggers.iter().find(|(_,native,_,_,_)|native.0==*id)?;
                let score = crate::environment::support::use_candidate(trigger,shape,pose,state,&query,
                    player_entity,client_modes.get(player_entity).ok()?.0,origin,forward,matches!(usable.source.action,InteractionAction::Generator {..}),definition.tuning.use_radius)?;
                return Some((-score,score,entity,usable,Some(trigger)));
            }
            if let InteractionAction::Door { connection, .. } = &usable.source.action {
                let (trigger, score) = native_triggers.iter().filter_map(|(trigger, native, shape, pose, state)| {
                    let source = definition.environment.triggers.iter().find(|source| source.id == native.0)?;
                    if !native_door_connection(&source.source, &definition.access.flag_links, connection) { return None; }
                    let score = crate::environment::support::use_candidate(trigger,shape,pose,state,&query,
                        player_entity,client_modes.get(player_entity).ok()?.0,origin,forward,false,definition.tuning.use_radius)?;
                    Some((trigger,score))
                }).min_by(|a,b|a.1.total_cmp(&b.1))?;
                return Some((-score,score,entity,usable,Some(trigger)));
            }
            if matches!(usable.source.action, InteractionAction::Generator { .. }) && origin.distance(transform.translation+Vec3::Y*usable.source.reach*0.5)>usable.source.reach {return None;}
            let endpoint = if matches!(usable.source.action, InteractionAction::Generator { .. }) {crate::generators::use_point(transform.translation,origin,usable.source.reach)} else if matches!(usable.source.action, InteractionAction::Perk { .. }) { transform.translation + Vec3::Y + transform.rotation * Vec3::new(0., 0., 0.8) } else { transform.translation };
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
            Some((dot, distance, entity, usable, None))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.total_cmp(&a.1)));
    let Some((_, _, entity, usable, native_trigger)) = candidate else {
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
    if usable
        .source
        .generator
        .as_ref()
        .is_some_and(|id| !generators.powered(id))
    {
        focus.prompt = "Requires generator power".into();
        focus.progress = 0.;
        focus.rejection = Some(InteractionRejection::NoPower);
        return;
    }
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
        focus.rejection = None;
        focus.progress = barriers.use_progress();
        if barrier_use_held {
            let Some(source_barrier) = definition
                .content
                .barriers
                .iter()
                .find(|definition| definition.id == *barrier)
            else {
                return;
            };
            if barriers
                .repair(
                    barrier,
                    player_entity,
                    &mut session,
                    source_barrier.repair_interval,
                )
                .is_some()
            {
                focus.accepted += 1;
                focus.rejection = None;
            }
        }
        return;
    }
    if let InteractionAction::Door { connection, .. } = &usable.source.action {
        let Some(native_trigger) = native_trigger else {
            return;
        };
        let Ok((_, native, _, _, _)) = native_triggers.get(native_trigger) else {
            return;
        };
        let Some(source) = definition
            .environment
            .triggers
            .iter()
            .find(|source| source.id == native.0)
        else {
            return;
        };
        let debris = source
            .source
            .get("targetname")
            .is_some_and(|name| name == "zombie_debris");
        let price = source
            .source
            .get("zombie_cost")
            .and_then(|cost| cost.parse::<u32>().ok())
            .unwrap_or(1000);
        focus.hold_seconds = 0.;
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
        focus.latched = true;
        focus.progress = 0.;
        let purchase = ledger.command(price);
        match ledger.accept(&mut session.points, purchase, || {
            access
                .unlock(&definition, connection)
                .map_err(|()| InteractionRejection::Unavailable)
        }) {
            Ok(()) => {
                for flag in source
                    .source
                    .get("script_flag")
                    .into_iter()
                    .flat_map(|flags| flags.split(','))
                    .map(str::trim)
                    .filter(|flag| !flag.is_empty())
                {
                    access.flags.insert(flag.into());
                    for link in definition
                        .access
                        .flag_links
                        .iter()
                        .filter(|link| link.flag == flag)
                    {
                        access.open.extend(link.connections.iter().cloned());
                    }
                    native_flags.write(crate::access::ZoneFlagSet(flag.into()));
                }
                access.refresh(&definition.content);
                if debris {
                    trigger_queue.push_and_delete(native_trigger, player_entity);
                } else {
                    trigger_queue.push(native_trigger, player_entity);
                }
                if let Some(target) = source.source.get("target") {
                    for (trigger, native, _, _, state) in &native_triggers {
                        if definition.environment.triggers.iter().any(|source| {
                            source.id == native.0 && source.source.get("target") == Some(target)
                        }) {
                            if debris && trigger != native_trigger {
                                commands.entity(trigger).despawn();
                            } else {
                                let mut state = state.clone();
                                state.enabled = false;
                                commands.entity(trigger).insert(state);
                            }
                        }
                    }
                }
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
            if perks.owned.len() >= perks.purchase_limit() {
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
                crate::audio::emit(
                    &mut commands,
                    "evt_bottle_dispense",
                    crate::audio::SoundSource::World(transform.translation),
                );
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
    let owned_variant = inventory.owned().find_map(|(_, owned)| {
        let candidate = definition
            .content
            .weapons
            .iter()
            .find(|w| w.id == owned.definition)?;
        (candidate.id == weapon.id
            || weapon.upgraded.as_ref() == Some(&candidate.id)
            || weapon
                .ammo_group
                .as_ref()
                .is_some_and(|group| candidate.ammo_group.as_ref() == Some(group)))
        .then_some(candidate)
    });
    let owned = owned_variant.is_some();
    let upgraded =
        owned_variant.is_some_and(|candidate| weapon.upgraded.as_ref() == Some(&candidate.id));
    let cost = if weapon.throwable.is_some() {
        *price
    } else if upgraded {
        4500
    } else if owned {
        ammo_price.unwrap_or((price / 2).div_ceil(10) * 10)
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
            if owned { " ammo" } else { "" },
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
        } else if let Some(owned_weapon) = owned_variant {
            if !inventory.refill_weapon(owned_weapon) {
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
                    if owned { "ammo" } else { "purchase" }
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

fn native_door_connection(
    source: &std::collections::BTreeMap<String, String>,
    links: &[crate::access::ZoneFlagLinkDefinition],
    connection: &crate::origins::ContentId,
) -> bool {
    source
        .get("targetname")
        .is_some_and(|name| matches!(name.as_str(), "zombie_door" | "zombie_debris"))
        && source.get("script_flag").is_some_and(|flags| {
            flags.split(',').map(str::trim).any(|flag| {
                links
                    .iter()
                    .any(|link| link.flag == flag && link.connections.contains(connection))
            })
        })
}

#[cfg(test)]
mod trigger_tests {
    use super::*;

    #[test]
    fn environment_trigger_query_access() {
        IntoSystem::into_system(interact).initialize(&mut World::new());
    }

    #[test]
    fn environment_trigger_door_uses_source_flags_instead_of_nearest_placement() {
        let connection = crate::origins::ContentId("connection.activate_zone_farm.0".into());
        let other = crate::origins::ContentId("connection.activate_zone_ruins.0".into());
        let links = vec![crate::access::ZoneFlagLinkDefinition {
            flag: "activate_zone_farm".into(),
            connections: vec![connection.clone()],
        }];
        let source = [
            ("targetname".into(), "zombie_debris".into()),
            ("script_flag".into(), "activate_zone_farm".into()),
        ]
        .into();
        assert!(native_door_connection(&source, &links, &connection));
        assert!(!native_door_connection(&source, &links, &other));
        let missing_flag = [("targetname".into(), "zombie_debris".into())].into();
        assert!(!native_door_connection(&missing_flag, &links, &connection));
    }
}
