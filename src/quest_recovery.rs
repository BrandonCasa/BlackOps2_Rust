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
struct Plane {
    quest: bool,
    node: usize,
}
#[derive(Component)]
struct Pilot {
    node: usize,
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
    }
    crate::audio::emit_loop(
        commands,
        "zmb_zombieblood_3rd_plane_loop",
        crate::audio::SoundSource::attached(entity),
    );
}
fn movement(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut planes: Query<(&mut Plane, &mut Transform), Without<Pilot>>,
    mut pilots: Query<
        (
            Entity,
            &mut Pilot,
            &mut Transform,
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
    for (entity, mut pilot, mut pose, landing) in &mut pilots {
        if let Some(mut landing) = landing {
            landing.remaining = (landing.remaining - time.delta_secs()).max(0.);
            pose.translation = landing.from.lerp(landing.to, 1. - landing.remaining / 3.);
            if landing.remaining == 0. {
                commands.entity(entity).remove::<PilotLanding>();
                crate::audio::emit_loop(
                    &mut commands,
                    "zmb_zombieblood_3rd_loop_other",
                    crate::audio::SoundSource::attached(entity),
                );
            }
            continue;
        }
        let destination = Vec3::from_array(source::PILOT[pilot.node]) + Vec3::Y * 0.9;
        let delta = destination - pose.translation;
        pose.translation += delta.clamp_length_max(5. * time.delta_secs());
        if delta.length() < 0.3 {
            pilot.node = (pilot.node + 1) % source::PILOT.len();
        }
        if delta.with_y(0.).length_squared() > 0.01 {
            pose.look_to(delta.with_y(0.), Vec3::Y);
        }
    }
}
fn restricted_visibility(
    effects: Res<crate::powerups::PowerupEffects>,
    mut entities: Query<&mut Visibility, With<BloodRestricted>>,
) {
    for mut visible in &mut entities {
        *visible = if effects.zombie_blood > 0. {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
fn observe(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<Recovery>,
    quest: ResMut<MainQuest>,
    session: Res<crate::gameplay::Session>,
    access: Res<crate::access::ZoneAccess>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    effects: Res<crate::powerups::PowerupEffects>,
    mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    planes: Query<(Entity, &Plane, &Transform)>,
    mut applied: MessageReader<crate::damage::DamageApplied>,
    pilots: Query<&Transform, With<Pilot>>,
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
            let feet = Vec3::from_array(source::PILOT[node]);
            let health = game.rounds.health(session.round);
            let pilot = commands
                .spawn((
                    SessionEntity,
                    Pilot { node },
                    PilotLanding {
                        remaining: 3.,
                        from: plane_pose.translation,
                        to: feet + Vec3::Y * 0.9,
                    },
                    BloodRestricted,
                    Transform::from_translation(plane_pose.translation),
                    Visibility::Inherited,
                    Vitality::ordinary_actor(health, health).expect("native pilot health"),
                    crate::hit_regions::Hitboxes::humanoid(1.8, 0.3),
                    Collider::capsule(0.3, 1.2),
                ))
                .id();
            if let Some(id) = game
                .content
                .enemies
                .iter()
                .find_map(|enemy| enemy.presentation.as_ref())
                && let Some(scene) = game
                    .content
                    .presentations
                    .iter()
                    .find(|p| p.id == *id)
                    .and_then(|p| p.scene.as_ref())
            {
                commands
                    .entity(pilot)
                    .insert(WorldAssetRoot(server.load(scene.clone())));
            }
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
            && let Ok(pose) = pilots.get(hit.impact.target)
        {
            state.pilot_down = true;
            let entity = commands
                .spawn((
                    SessionEntity,
                    DronePickup,
                    Transform::from_translation(pose.translation + Vec3::Y * 0.762)
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
            commands.entity(hit.impact.target).despawn();
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
