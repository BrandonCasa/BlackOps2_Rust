use crate::{
    GameState,
    gameplay::{GameStep, SessionEntity},
    life::{LifeState, Vitality},
    quests::{MainQuest, QuestStage},
};
use avian3d::prelude::*;
use bevy::prelude::*;
mod source;
#[derive(Component)]
pub struct BloodRestricted;
#[derive(Component)]
pub(crate) struct Plane {
    pub(crate) quest: bool,
    node: usize,
    glow: Option<Entity>,
}
#[derive(Component)]
pub(crate) struct Pilot {
    node: usize,
    destination: Option<Vec3>,
    route_revision: u32,
    vertical_velocity: f32,
    sprint_clip: usize,
    sound: Option<Entity>,
}
#[derive(Component)]
pub(crate) struct PilotLanding {
    remaining: f32,
    from: Vec3,
    to: Vec3,
}
#[derive(Component)]
struct FirePieceDrop {
    remaining: f32,
}
#[derive(Component)]
pub(crate) struct DronePickup;
#[derive(Resource, Default)]
pub struct Recovery {
    pub fire_down: bool,
    pub plane_down: bool,
    pub pilot_down: bool,
    fire_started: bool,
    quest_started: bool,
    village_round: Option<u32>,
}
pub struct RecoveryPlugin;
impl Plugin for RecoveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Recovery>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                reset.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                movement
                    .in_set(GameStep::Movement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                observe
                    .in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                restricted_visibility
                    .in_set(GameStep::Interactions)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                recover
                    .in_set(GameStep::Interactions)
                    .after(crate::main_quest::interact)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn reset(mut state: ResMut<Recovery>) {
    *state = default();
}
fn presentation(
    commands: &mut Commands,
    entity: Entity,
    model: &str,
    game: &crate::assets::GameDefinition,
    server: &AssetServer,
) {
    if let Some(scene) = game
        .content
        .presentations
        .iter()
        .filter_map(|p| p.scene.as_ref())
        .find(|s| s.contains(model))
    {
        commands
            .entity(entity)
            .insert(WorldAssetRoot(server.load(scene.clone())));
    }
}
fn spawn_plane(
    commands: &mut Commands,
    quest: bool,
    game: &crate::assets::GameDefinition,
    server: &AssetServer,
) {
    let entity = commands
        .spawn((
            SessionEntity,
            Plane {
                quest,
                node: 1 % source::PLANE.len(),
                glow: None,
            },
            Transform::from_translation(Vec3::from_array(source::PLANE[0])),
            Visibility::Inherited,
            Collider::cuboid(6., 1.5, 4.),
            Sensor,
            CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE),
            crate::environment::support::EnvironmentOwner::Quest,
            crate::environment::support::ShootableEnvironment,
        ))
        .id();
    presentation(commands, entity, "veh_t6_dlc_zm_biplane_lod0", game, server);
    if quest {
        commands.entity(entity).insert(BloodRestricted);
    } else {
        crate::native_fx::attach(
            commands,
            "maps/zombie_tomb/fx_tomb_elem_reveal_fire_glow",
            entity,
            Transform::IDENTITY,
        );
    }
    if quest {
        crate::audio::emit_loop(
            commands,
            "zmb_zombieblood_3rd_plane_loop",
            crate::audio::SoundSource::attached(entity),
        );
    }
}
fn movement(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<crate::assets::GameDefinition>,
    access: Res<crate::access::ZoneAccess>,
    mut navigation: ResMut<crate::navigation::Navigation>,
    spatial: SpatialQuery,
    slide: MoveAndSlide,
    mut planes: Query<(&mut Plane, &mut Transform), Without<Pilot>>,
    mut pilots: Query<
        (
            Entity,
            &mut Pilot,
            &mut crate::enemies::Zombie,
            &mut Transform,
            &Collider,
            &Vitality,
            Option<&mut PilotLanding>,
        ),
        Without<Plane>,
    >,
) {
    for (mut plane, mut pose) in &mut planes {
        let destination = Vec3::from_array(source::PLANE[plane.node]);
        let delta = destination - pose.translation;
        pose.translation += delta.clamp_length_max(33.528 * time.delta_secs());
        if delta.length() < 1. {
            plane.node = (plane.node + 1) % source::PLANE.len();
        }
        if delta.length_squared() > 0.01 {
            pose.look_to(delta, Vec3::Y);
        }
    }
    for (entity, mut pilot, mut zombie, mut pose, collider, health, landing) in &mut pilots {
        if health.state() != LifeState::ActorAlive {
            continue;
        }
        let enemy = game
            .content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
            .expect("native pilot actor definition");
        let center = crate::enemies::body_center(enemy, false);
        if let Some(mut landing) = landing {
            landing.remaining = (landing.remaining - time.delta_secs()).max(0.);
            pose.translation = landing.from.lerp(landing.to, 1. - landing.remaining / 3.);
            if landing.remaining == 0. {
                commands.entity(entity).remove::<PilotLanding>();
                zombie.action = crate::origins::content::EnemyAnimation::Sprint;
                zombie.elapsed = 0.;
                pilot.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_zombieblood_3rd_loop_other",
                    crate::audio::SoundSource::attached(entity),
                ));
            }
            continue;
        }
        let feet = pose.translation - center;
        let mut goal = Vec3::from_array(source::PILOT[pilot.node]);
        if feet.distance_squared(goal) < 0.3_f32.powi(2) {
            pilot.node = (pilot.node + 1) % source::PILOT.len();
            pilot.destination = None;
            goal = Vec3::from_array(source::PILOT[pilot.node]);
        }
        if pilot.route_revision != navigation.revision
            || pilot
                .destination
                .is_none_or(|point| feet.xz().distance_squared(point.xz()) < 0.3_f32.powi(2))
        {
            pilot.destination = Some(navigation.destination(feet, goal, &access));
            pilot.route_revision = navigation.revision;
        }
        let delta = (pilot.destination.expect("pilot route") - feet).with_y(0.);
        let motion = game.content.animations[pilot.sprint_clip]
            .root_motion
            .as_ref()
            .expect("native pilot sprint root motion");
        let previous = zombie.elapsed;
        zombie.elapsed += time.delta_secs();
        let distance = Vec3::from_array(motion.loop_displacement(previous, zombie.elapsed))
            .xz()
            .length();
        let direction = delta.normalize_or_zero();
        pilot.vertical_velocity -= game.tuning.gravity * time.delta_secs();
        let velocity = direction * distance.min(delta.length()) / time.delta_secs()
            + Vec3::Y * pilot.vertical_velocity;
        let filter = crate::environment::actor_solids([entity]);
        let grounded = spatial
            .cast_ray(feet + Vec3::Y * 0.1, Dir3::NEG_Y, 0.25, false, &filter)
            .is_some_and(|hit| hit.normal.y > 0.65);
        let result = crate::player::slide_character(
            &slide,
            &spatial,
            crate::player::MotionShape {
                collider,
                radius: enemy.physics.expect("native pilot actor physics").radius,
                step_height: game.tuning.step_height,
                grounded,
            },
            pose.translation,
            velocity,
            time.delta(),
            &filter,
        );
        pose.translation = result.position;
        pilot.vertical_velocity = result.projected_velocity.y;
        if direction.length_squared() > 0. {
            pose.rotation = Quat::from_rotation_y((-direction.z).atan2(direction.x));
        }
    }
}
fn restricted_visibility(
    mut commands: Commands,
    effects: Res<crate::powerups::PowerupEffects>,
    mut entities: Query<&mut Visibility, With<BloodRestricted>>,
    mut planes: Query<(Entity, &mut Plane)>,
) {
    for mut visible in &mut entities {
        *visible = if effects.zombie_blood > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (entity, mut plane) in &mut planes {
        if !plane.quest {
            continue;
        }
        if effects.zombie_blood > 0. && plane.glow.is_none() {
            plane.glow = Some(crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_elem_reveal_fire_glow",
                entity,
                Transform::IDENTITY,
            ));
        } else if effects.zombie_blood <= 0.
            && let Some(glow) = plane.glow.take()
        {
            crate::native_fx::stop(&mut commands, glow);
        }
    }
}
fn observe(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<Recovery>,
    quest: ResMut<MainQuest>,
    mut session: ResMut<crate::gameplay::Session>,
    access: Res<crate::access::ZoneAccess>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    effects: Res<crate::powerups::PowerupEffects>,
    mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    planes: Query<(Entity, &Plane, &Transform)>,
    mut applied: MessageReader<crate::damage::DamageApplied>,
    mut pilots: Query<(&Transform, &crate::enemies::Zombie, &mut Pilot)>,
    mut parts: ResMut<crate::environment::parts::Parts>,
    mut drops: Query<(Entity, &mut FirePieceDrop)>,
    players: Query<(), With<crate::player::Player>>,
) {
    if state.village_round.is_none()
        && access
            .open
            .iter()
            .any(|z| z.0 == "connection.activate_zone_village_0.0")
    {
        state.village_round = Some(session.round);
    }
    if !state.fire_started && state.village_round.is_some_and(|r| session.round > r) {
        spawn_plane(&mut commands, false, &game, &server);
        state.fire_started = true;
    }
    if quest.enabled && quest.stage == QuestStage::RecoverDrone && !state.quest_started {
        spawn_plane(&mut commands, true, &game, &server);
        state.quest_started = true;
    }
    for (entity, mut drop) in &mut drops {
        drop.remaining -= time.delta_secs();
        if drop.remaining <= 0. {
            crate::crafting::reveal_part(
                &mut commands,
                &mut parts,
                "part.elemental_staff_fire.middle_staff",
                Vec3::new(20.024852, 3.519932, -11.958066),
            );
            commands.entity(entity).despawn();
        }
    }
    for hit in hits.read() {
        let Ok((entity, plane, plane_pose)) = planes.get(hit.target) else {
            continue;
        };
        if !players.contains(hit.attacker)
            || hit.amount == 0
            || plane.quest && (effects.zombie_blood <= 0. || state.plane_down)
            || !plane.quest && state.fire_down
        {
            continue;
        }
        if plane.quest && !state.plane_down {
            state.plane_down = true;
            let node = source::PILOT
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    Vec3::from_array(*a.1)
                        .distance_squared(plane_pose.translation)
                        .total_cmp(&Vec3::from_array(*b.1).distance_squared(plane_pose.translation))
                })
                .map_or(0, |(i, _)| i);
            // Step 5 uses the first authored zombie_spawner_dig: basic German.
            // Its dug initializer has no rise animation; the script links it to
            // the falling plane position for three seconds before setting sprint.
            let count = game
                .content
                .enemies
                .iter()
                .filter(|enemy| enemy.archetype == "actor_zm_tomb_basic_german")
                .count();
            assert!(count > 0, "native pilot dig-spawner archetype");
            let enemy = game
                .content
                .enemies
                .iter()
                .filter(|enemy| enemy.archetype == "actor_zm_tomb_basic_german")
                .nth(session.random() as usize % count)
                .unwrap();
            let center = crate::enemies::body_center(enemy, false);
            let from = plane_pose.translation + center;
            let health = game.rounds.health(session.round);
            let mut zombie =
                crate::enemies::Zombie::new(from, enemy.id.clone(), crate::rounds::Gait::Sprint)
                    .with_locomotion_seed(session.random())
                    .dug();
            zombie.action = crate::origins::content::EnemyAnimation::Sprint;
            let sprint = zombie
                .locomotion_clip(enemy)
                .expect("native pilot sprint clip");
            let sprint_clip = game
                .content
                .animations
                .iter()
                .position(|clip| {
                    enemy.presentation.as_ref() == Some(&clip.skeleton)
                        && clip.id.0.rsplit('.').next() == Some(sprint)
                })
                .expect("native pilot sprint animation");
            assert!(
                game.content.animations[sprint_clip].root_motion.is_some(),
                "native pilot sprint root motion"
            );
            zombie.action = crate::origins::content::EnemyAnimation::Idle;
            commands.spawn((
                Name::new("zombie_blood_pilot"),
                SessionEntity,
                Pilot {
                    node,
                    destination: None,
                    route_revision: u32::MAX,
                    vertical_velocity: 0.,
                    sprint_clip,
                    sound: None,
                },
                PilotLanding {
                    remaining: 3.,
                    from,
                    to: Vec3::from_array(source::PILOT[node]) + center,
                },
                BloodRestricted,
                zombie,
                Transform::from_translation(from).with_rotation(plane_pose.rotation),
                Visibility::Inherited,
                Vitality::ordinary_actor(health, health).expect("native pilot health"),
                crate::enemies::body_shape(enemy, false),
            ));
        } else if !plane.quest && !state.fire_down {
            state.fire_down = true;
            commands.spawn((SessionEntity, FirePieceDrop { remaining: 5. }));
        }
        crate::native_fx::emit(
            &mut commands,
            "maps/zombie_tomb/fx_tomb_explo_airplane",
            *plane_pose,
        );
        commands.entity(entity).despawn();
        crate::audio::emit(
            &mut commands,
            "zmb_zombieblood_3rd_plane_explode",
            crate::audio::SoundSource::World(hit.point),
        );
    }
    for hit in applied.read() {
        if hit.lethal
            && !state.pilot_down
            && let Ok((pose, zombie, mut pilot)) = pilots.get_mut(hit.impact.target)
        {
            state.pilot_down = true;
            if let Some(sound) = pilot.sound.take() {
                commands.entity(sound).try_despawn();
            }
            let enemy = game
                .content
                .enemies
                .iter()
                .find(|enemy| enemy.id == zombie.definition)
                .expect("native pilot actor definition");
            let feet = pose.translation - crate::enemies::body_center(enemy, false);
            let entity = commands
                .spawn((
                    SessionEntity,
                    DronePickup,
                    Transform::from_translation(feet + Vec3::Y * 0.762)
                        .with_rotation(pose.rotation),
                    Visibility::Inherited,
                ))
                .id();
            presentation(
                &mut commands,
                entity,
                "veh_t6_dlc_zm_quadrotor_lod0",
                &game,
                &server,
            );
        }
    }
}
pub(crate) fn recover(
    mut commands: Commands,
    input: Res<crate::input::GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut quest: ResMut<MainQuest>,
    mut buildables: ResMut<crate::buildables::BuildableState>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    spatial: SpatialQuery,
    pickups: Query<(Entity, &Transform), With<DronePickup>>,
    mut players: Query<
        (
            Entity,
            &crate::player::Player,
            &Transform,
            &Vitality,
            &crate::weapons::WeaponInventory,
            Has<crate::perks::PerkDrinking>,
            Option<&crate::actions::HandReservation>,
        ),
        Without<DronePickup>,
    >,
) {
    if !quest.enabled || quest.stage != QuestStage::RecoverDrone || focus.target.is_some() {
        return;
    }
    let Ok((owner, player, pose, health, inventory, drinking, reservation)) = players.single_mut()
    else {
        return;
    };
    if health.state() != LifeState::Standing {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    for (entity, pickup) in &pickups {
        let delta = pickup.translation - eye;
        let Ok(direction) = Dir3::new(delta) else {
            continue;
        };
        let feet = pose.translation - Vec3::Y * (player.height * 0.5);
        let base = pickup.translation - Vec3::Y * 0.762;
        let offset = feet - base;
        if offset.xz().length_squared() > 0.9144_f32.powi(2)
            || offset.y < 0.
            || offset.y > 6.5024
            || forward.dot(*direction) < 0.7
            || spatial
                .cast_ray(
                    eye,
                    direction,
                    (delta.length() - 0.1).max(0.),
                    true,
                    &crate::environment::solids([owner]),
                )
                .is_some()
        {
            continue;
        }
        focus.target = Some(entity);
        focus.hold_seconds = 0.;
        focus.progress = 0.;
        focus.prompt = format!(
            "Press {} to recover Maxis Drone",
            settings.label(crate::input::Action::Interact)
        );
        if input.use_held
            && !drinking
            && reservation.is_none()
            && inventory.action == crate::weapons::WeaponAction::Ready
        {
            buildables.drone_upgraded = true;
            buildables.drone_disabled = false;
            buildables.drone_cooldown = 0.;
            crate::audio::emit(
                &mut commands,
                "vox_maxi_drone_upgraded_0",
                crate::audio::SoundSource::attached(owner),
            );
            commands.entity(entity).despawn();
            quest.transition(QuestStage::RecoverDrone, QuestStage::UpgradeFists);
            focus.accepted += 1;
        }
        break;
    }
}
