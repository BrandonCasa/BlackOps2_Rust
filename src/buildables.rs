use crate::{
    GameState,
    assets::GameDefinition,
    crafting::{BuildableInventory, Crafting},
    damage::{DamageQueue, Impact, ImpactSource},
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    life::{LifeState, Vitality},
    origins::content::{DamageCategory, WeaponId},
    player::{Player, TickActions},
    weapons::HitRegion,
};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct ShieldAggro(pub Entity);
#[derive(Component)]
pub(crate) struct DeployedShield {
    pub owner: Entity,
}
#[derive(Component)]
pub struct Drone {
    pub owner: Entity,
    pub remaining: f32,
    pub returning: bool,
    pub upgraded: bool,
    pub quest_goal: Option<Vec3>,
    home: Vec3,
    cooldown: f32,
    burst: f32,
    pause: f32,
    revive: Option<(Entity, f32)>,
    crash: Option<f32>,
}
#[derive(Resource, Default)]
pub struct BuildableState {
    pub drone: Option<Entity>,
    pub drone_cooldown: f32,
    pub drone_upgraded: bool,
    pub drone_disabled: bool,
    melee_cooldown: f32,
    use_latched: bool,
}
pub struct BuildablePlugin;
impl Plugin for BuildablePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildableState>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                reset.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (actions, shield_targets)
                    .chain()
                    .before(crate::enemies::lifecycle)
                    .in_set(GameStep::EnemyIntent)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (drones, drone_fire)
                    .chain()
                    .in_set(GameStep::Combat)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                collect_shield
                    .in_set(GameStep::Interactions)
                    .after(crate::crafting::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                cleanup
                    .in_set(GameStep::Zombies)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn reset(mut state: ResMut<BuildableState>) {
    *state = BuildableState::default();
}
fn visual(
    commands: &mut Commands,
    entity: Entity,
    model: &str,
    game: &GameDefinition,
    server: &AssetServer,
) {
    if let Some(scene) = crate::assets::native_scene(game, model) {
        commands
            .entity(entity)
            .insert(WorldAssetRoot(server.load(scene.clone())));
    }
}
fn actions(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    input: Res<GameInput>,
    mut actions: ResMut<TickActions>,
    mut state: ResMut<BuildableState>,
    game: Res<GameDefinition>,
    crafting: Res<Crafting>,
    server: Res<AssetServer>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut BuildableInventory,
        &mut crate::weapons::WeaponInventory,
        &mut crate::equipment::GrenadeInventory,
        &mut crate::equipment::TacticalInventory,
        &mut crate::mines::MineInventory,
    )>,
    mut enemies: Query<
        (
            Entity,
            &Transform,
            &Vitality,
            &mut crate::enemies::Zombie,
            &mut crate::combat_effects::Wounds,
        ),
        Without<Player>,
    >,
    shields: Query<(Entity, &Transform), With<DeployedShield>>,
    reserved: Query<(), With<crate::actions::HandReservation>>,
    mut damage: ResMut<DamageQueue>,
    query: SpatialQuery,
) {
    let dt = time.delta_secs();
    state.melee_cooldown = (state.melee_cooldown - dt).max(0.);
    state.drone_cooldown = (state.drone_cooldown - dt).max(0.);
    let Ok((
        owner,
        player,
        pose,
        health,
        mut equipment,
        mut inventory,
        mut grenades,
        mut tactical,
        mut mines,
    )) = players.single_mut()
    else {
        return;
    };
    equipment.shield_attack = (equipment.shield_attack - dt).max(0.);
    if health.state() != LifeState::Standing {
        equipment.shield_held = false;
        return;
    }
    if reserved.contains(owner) {
        return;
    }
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    if input.drone
        && equipment.owned.contains("equip_dieseldrone_zm")
        && !state.drone_disabled
        && state.drone.is_none()
        && state.drone_cooldown <= 0.
    {
        if let Some(home) = crafting.drone_home() {
            let entity = commands
                .spawn((
                    Drone {
                        quest_goal: None,
                        owner,
                        remaining: 90.,
                        returning: false,
                        upgraded: state.drone_upgraded,
                        home,
                        cooldown: 0.,
                        burst: 0.,
                        pause: 0.,
                        revive: None,
                        crash: None,
                    },
                    SessionEntity,
                    Transform::from_translation(pose.translation + Vec3::Y * 2.),
                    Visibility::Inherited,
                    Vitality::ordinary_actor(200, 200).expect("drone health"),
                    Collider::sphere(0.4),
                    crate::hit_regions::Hitboxes::humanoid(0.8, 0.4),
                    RigidBody::Kinematic,
                    CustomPositionIntegration,
                ))
                .id();
            visual(
                &mut commands,
                entity,
                "veh_t6_dlc_zm_quadrotor",
                &game,
                &server,
            );
            crate::audio::emit_loop(
                &mut commands,
                "veh_qrdrone_idle_loop",
                crate::audio::SoundSource::attached(entity),
            );
            equipment.owned.remove("equip_dieseldrone_zm");
            state.drone = Some(entity);
            if equipment.equipped.as_deref() == Some("equip_dieseldrone_zm") {
                equipment.equipped = equipment
                    .owned
                    .contains("tomb_shield_zm")
                    .then(|| "tomb_shield_zm".into());
            }
        }
    }
    if input.buildable && equipment.owned.contains("tomb_shield_zm") && equipment.shield_health > 0
    {
        if equipment.shield_held && input.use_held {
            let facing = forward.with_y(0.).normalize_or_zero();
            let feet = pose.translation - Vec3::Y * player.height * 0.5;
            let start = feet + facing * 1.2 + Vec3::Y * 1.5;
            let ground = query.cast_ray(
                start,
                Dir3::NEG_Y,
                2.,
                true,
                &crate::environment::solids([owner]),
            );
            if player.grounded
                && let Some(hit) = ground
                && hit.normal.y >= 0.7
            {
                let position = start - Vec3::Y * hit.distance + Vec3::Y * 0.61;
                let rotation = Quat::from_rotation_y(player.yaw);
                let shape = Collider::cuboid(0.75, 1.2, 0.2);
                if shields
                    .iter()
                    .all(|(_, p)| p.translation.distance_squared(position) >= 2.54_f32.powi(2))
                    && query
                        .shape_intersections(
                            &shape,
                            position,
                            rotation,
                            &crate::environment::solids([owner]),
                        )
                        .is_empty()
                {
                    if let Some(previous) = equipment.deployed_shield.take() {
                        commands.entity(previous).despawn();
                    }
                    let entity = commands
                        .spawn((
                            DeployedShield { owner },
                            SessionEntity,
                            Transform::from_translation(position).with_rotation(rotation),
                            Visibility::Inherited,
                            RigidBody::Static,
                            shape,
                            Vitality::ordinary_actor(equipment.shield_health, 1500)
                                .expect("shield health"),
                        ))
                        .id();
                    visual(
                        &mut commands,
                        entity,
                        "t6_wpn_zmb_shield_dlc4_dmg0_world",
                        &game,
                        &server,
                    );
                    equipment.deployed_shield = Some(entity);
                    equipment.shield_health = 0;
                    equipment.shield_held = false;
                    equipment.owned.remove("tomb_shield_zm");
                    equipment.equipped = None;
                }
            }
        } else {
            equipment.shield_held = !equipment.shield_held;
            if equipment.shield_held {
                inventory.cancel_action();
                grenades.cancel();
                if let Some(g) = tactical.0.as_mut() {
                    g.cancel();
                }
                mines.cancel();
            }
        }
    }
    if !equipment.shield_held {
        return;
    }
    let attack = actions.fire || actions.melee;
    *actions = TickActions::default();
    inventory.cancel_action();
    if attack && state.melee_cooldown <= 0. {
        state.melee_cooldown = 0.8;
        equipment.shield_attack = 0.8;
        let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
        for (target, position, vitality, mut zombie, mut wounds) in &mut enemies {
            let delta = position.translation - eye;
            if vitality.state() != LifeState::ActorAlive
                || delta.length() > 2.286
                || delta.normalize_or_zero().dot(forward) <= 0.
            {
                continue;
            }
            let Some(direction) = Dir3::new(delta).ok() else {
                continue;
            };
            if query
                .cast_ray(
                    eye,
                    direction,
                    (delta.length() - 0.35).max(0.),
                    true,
                    &crate::environment::solids([owner, target]),
                )
                .is_some()
            {
                continue;
            }
            damage.0.push(Impact {
                point: None,
                attacker: owner,
                target,
                source: ImpactSource::Weapon(WeaponId("weapon.tomb_shield_zm".into())),
                category: DamageCategory::Melee,
                region: HitRegion::Torso,
                amount: 2500,
            });
            crate::combat_effects::knockdown(
                &mut wounds,
                &mut zombie,
                pose.translation,
                position.translation,
                0.8,
            );
            equipment.shield_health = equipment.shield_health.saturating_sub(100);
        }
        if equipment.shield_health == 0 {
            equipment.owned.remove("tomb_shield_zm");
            equipment.shield_held = false;
        }
    }
}
fn shield_targets(
    mut commands: Commands,
    shields: Query<(Entity, &Transform, &Vitality), With<DeployedShield>>,
    enemies: Query<
        (
            Entity,
            &Transform,
            &crate::rounds::PopulationKind,
            Option<&crate::enemies::EnemyGoal>,
            Has<ShieldAggro>,
        ),
        With<crate::enemies::Zombie>,
    >,
) {
    for (entity, pose, kind, goal, assigned) in &enemies {
        if *kind != crate::rounds::PopulationKind::Ordinary || (goal.is_some() && !assigned) {
            continue;
        }
        let selected = shields
            .iter()
            .filter(|(_, position, health)| {
                health.state() == LifeState::ActorAlive
                    && position.translation.distance_squared(pose.translation) < 12. * 12.
            })
            .min_by(|(_, a, _), (_, b, _)| {
                a.translation
                    .distance_squared(pose.translation)
                    .total_cmp(&b.translation.distance_squared(pose.translation))
            });
        if let Some((shield, position, _)) = selected {
            commands.entity(entity).insert((
                ShieldAggro(shield),
                crate::enemies::EnemyGoal {
                    position: position.translation,
                    face: position.translation,
                    radius: 1.65,
                },
            ));
        } else if assigned {
            commands
                .entity(entity)
                .remove::<(ShieldAggro, crate::enemies::EnemyGoal)>();
        }
    }
}
fn drones(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    query: SpatialQuery,
    players: Query<(Entity, &Player, &Transform, &Vitality), Without<Drone>>,
    pickups: Query<(Entity, &crate::powerups::Pickup, &Transform), Without<Drone>>,
    mut drones: Query<(Entity, &mut Drone, &mut Transform, &Vitality), Without<Player>>,
    mut state: ResMut<BuildableState>,
    mut revive: MessageWriter<crate::solo_revive::RemoteRevive>,
    mut collect: MessageWriter<crate::powerups::DronePickup>,
) {
    let dt = time.delta_secs();
    for (entity, mut drone, mut pose, health) in &mut drones {
        if health.state() != LifeState::ActorAlive {
            if drone.crash.is_none() {
                drone.crash = Some(3.);
                state.drone = None;
                state.drone_cooldown = 0.;
                commands.entity(entity).insert(ColliderDisabled);
            }
            let remaining = drone.crash.as_mut().unwrap();
            *remaining -= dt;
            pose.translation.y -= 4. * dt;
            pose.rotate_local_z(dt * 3.);
            if *remaining <= 0. {
                commands.entity(entity).despawn();
            }
            continue;
        }
        if drone.quest_goal.is_none() {
            drone.remaining -= dt;
        }
        drone.cooldown = (drone.cooldown - dt).max(0.);
        drone.pause = (drone.pause - dt).max(0.);
        let owner = players.get(drone.owner).ok();
        drone.returning |= drone.remaining <= 0.
            || owner.is_none_or(|(_, _, _, v)| v.state() == LifeState::HumanTerminal);
        if drone.quest_goal.is_none() && !drone.returning {
            if drone.revive.is_some_and(|(target, _)| {
                players
                    .get(target)
                    .is_ok_and(|(_, _, _, v)| v.state() == LifeState::Downed)
                    == false
            }) {
                drone.revive = None;
            }
            if drone.revive.is_none() {
                if let Some((target, _, _, _)) = players.iter().find(|(_, _, p, v)| {
                    v.state() == LifeState::Downed
                        && p.translation.distance_squared(pose.translation) < 12.7_f32.powi(2)
                }) {
                    drone.revive = Some((target, 0.));
                }
            }
        } else {
            drone.revive = None;
        }
        let retrieval = if drone.revive.is_none() && !drone.returning && drone.quest_goal.is_none()
        {
            owner.and_then(|(_, _, p, _)| {
                pickups
                    .iter()
                    .filter(|(_, drop, t)| {
                        drop.remaining > 0.
                            && t.translation.distance_squared(p.translation) < 12.7_f32.powi(2)
                    })
                    .min_by(|(_, _, a), (_, _, b)| {
                        a.translation
                            .distance_squared(p.translation)
                            .total_cmp(&b.translation.distance_squared(p.translation))
                    })
                    .map(|(id, _, p)| (id, p.translation))
            })
        } else {
            None
        };
        let goal = if let Some(goal) = drone.quest_goal {
            goal
        } else if drone.returning {
            drone.home + Vec3::Y
        } else if let Some((target, _)) = drone.revive {
            players.get(target).unwrap().2.translation + Vec3::Y * 3.81
        } else if let Some((_, point)) = retrieval {
            point
        } else {
            let (_, body, p, _) = owner.unwrap();
            p.translation
                + Quat::from_rotation_y(body.yaw) * Vec3::NEG_Z * 3.2512
                + Vec3::Y * 3.2512
        };
        let delta = goal - pose.translation;
        let step = delta.clamp_length_max(6. * dt);
        if let Ok(direction) = Dir3::new(step) {
            if drone.quest_goal.is_some()
                || query
                    .cast_ray(
                        pose.translation,
                        direction,
                        step.length(),
                        true,
                        &crate::environment::solids([entity]),
                    )
                    .is_none()
            {
                pose.translation += step;
            } else {
                pose.translation.y += dt * 2.;
            }
        }
        if drone.quest_goal.is_some() {
            continue;
        }
        if drone.returning {
            if delta.length() < 0.5 || drone.remaining < -30. {
                commands.entity(entity).despawn();
                state.drone = None;
                state.drone_cooldown = 60.;
            }
            continue;
        }
        if let Some((target, mut clock)) = drone.revive {
            if delta.length() < 0.8 {
                clock += dt;
            } else {
                clock = 0.;
            }
            if clock >= 1. {
                revive.write(crate::solo_revive::RemoteRevive { player: target });
                drone.revive = None;
                drone.pause = 1.;
            } else {
                drone.revive = Some((target, clock));
            }
            continue;
        }
        if let Some((pickup, _)) = retrieval {
            drone.pause = drone.pause.max(0.1);
            if delta.length() < 0.8 {
                collect.write(crate::powerups::DronePickup {
                    owner: drone.owner,
                    pickup,
                });
                drone.pause = 1.;
            }
            continue;
        }
    }
}
fn drone_fire(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    query: SpatialQuery,
    enemies: crate::hit_regions::RayTargets,
    mut drones: Query<(Entity, &mut Drone, &Transform, &Vitality)>,
    mut damage: ResMut<DamageQueue>,
    mut session: ResMut<crate::gameplay::Session>,
) {
    let dt = time.delta_secs();
    for (entity, mut drone, pose, health) in &mut drones {
        if drone.returning
            || drone.quest_goal.is_some()
            || drone.revive.is_some()
            || health.state() != LifeState::ActorAlive
        {
            continue;
        }
        if drone.pause > 0. {
            continue;
        }
        let weapon_id = if drone.upgraded {
            "weapon.quadrotorturret_upgraded_zm"
        } else {
            "weapon.quadrotorturret_zm"
        };
        let Some(weapon) = game.content.weapons.iter().find(|w| w.id.0 == weapon_id) else {
            continue;
        };
        let target = enemies
            .iter()
            .filter(|(target, v, p, _)| {
                *target != entity
                    && v.state() == LifeState::ActorAlive
                    && (2.4384..9.7536).contains(&p.translation.distance(pose.translation))
            })
            .filter_map(|(target, _, position, _)| {
                let direction = Dir3::new(position.translation - pose.translation).ok()?;
                let hit = crate::hit_regions::cast(
                    &enemies,
                    &query,
                    entity,
                    pose.translation,
                    direction,
                    9.7536,
                )?;
                (hit.target == target).then_some(hit)
            })
            .min_by(|a, b| a.hit.distance.total_cmp(&b.hit.distance));
        if let Some(hit) = target {
            if drone.burst <= 0. {
                drone.burst = 1.5 + (session.random() as f32 / u32::MAX as f32) * 1.5;
            }
            drone.burst -= dt;
            if drone.cooldown <= 0. {
                damage.0.push(Impact {
                    point: None,
                    attacker: drone.owner,
                    target: hit.target,
                    source: ImpactSource::Weapon(weapon.id.clone()),
                    category: weapon.category,
                    region: hit.hit.region,
                    amount: crate::weapons::damage_at(weapon, hit.hit.distance, hit.hit.region),
                });
                drone.cooldown = weapon.fire_interval.max(0.05);
                crate::audio::binding(
                    &mut commands,
                    &game,
                    &format!("{weapon_id}.fire"),
                    crate::audio::SoundSource::attached(entity),
                );
            }
            if drone.burst <= 0. {
                drone.pause = 0.5 + (session.random() as f32 / u32::MAX as f32) * 0.5;
            }
        } else {
            drone.burst = 0.;
        }
    }
}
pub(crate) fn collect_shield(
    mut commands: Commands,
    input: Res<GameInput>,
    mut state: ResMut<BuildableState>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    settings: Res<crate::input::ControlSettings>,
    shields: Query<(Entity, &Transform, &Vitality, &DeployedShield)>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut BuildableInventory,
    )>,
) {
    if !input.use_held {
        state.use_latched = false;
    }
    if focus.target.is_some() {
        return;
    }
    let Ok((owner, body, pose, health, mut equipment)) = players.single_mut() else {
        return;
    };
    if health.state() != LifeState::Standing {
        return;
    }
    let forward = Quat::from_rotation_y(body.yaw) * Vec3::NEG_Z;
    for (entity, position, vitality, shield) in &shields {
        if shield.owner != owner {
            continue;
        }
        let delta = position.translation - pose.translation;
        if vitality.state() != LifeState::ActorAlive
            || delta.length() > 2.
            || delta.normalize_or_zero().dot(forward) < 0.85
        {
            continue;
        }
        focus.target = Some(entity);
        focus.prompt = format!(
            "Press {} to take Zombie Shield",
            settings.label(crate::input::Action::Interact)
        );
        if input.use_held && !state.use_latched && equipment.shield_health == 0 {
            equipment.shield_health = vitality.current().unwrap_or(0);
            equipment.owned.insert("tomb_shield_zm".into());
            equipment.equipped = Some("tomb_shield_zm".into());
            equipment.deployed_shield = None;
            commands.entity(entity).despawn();
            state.use_latched = true;
            focus.accepted += 1;
        }
        break;
    }
}
fn cleanup(mut commands: Commands, shields: Query<(Entity, &Vitality), With<DeployedShield>>) {
    for (entity, health) in &shields {
        if health.state() != LifeState::ActorAlive {
            commands.entity(entity).despawn();
        }
    }
}
