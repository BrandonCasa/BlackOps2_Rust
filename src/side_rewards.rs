use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, SessionEntity},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;
const MEDALLIONS: [[f32; 3]; 4] = [
    [-11.1252, 11.5316, 1.5748],
    [-71.0438, 9.1694, -1.4732],
    [26.2636, 13.35405, 53.9242],
    [33.36925, -6.45795, -116.32565],
];
const REWARD: [f32; 3] = [-2.2352, 9.5504, 0.2032];
const BLOOD: [f32; 3] = [3.0988, 8.5344, 6.9088];

#[derive(Resource, Default)]
pub struct SideRewards {
    pub seals: [bool; 4],
    pub magna_collected: bool,
    pub fires_out: [bool; 3],
    fire_timer: f32,
    blood_round: Option<u32>,
    spare: std::collections::HashSet<Entity>,
    drone_target: Option<usize>,
    drone_wait: f32,
    delivery: bool,
    delivered: bool,
}
#[derive(Component)]
pub(crate) struct Reward;
pub struct SideRewardPlugin;
impl Plugin for SideRewardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SideRewards>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                reset.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (spare_change, fires, medallions)
                    .in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                collect
                    .in_set(GameStep::Interactions)
                    .after(crate::quest_recovery::recover)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn reset(mut state: ResMut<SideRewards>) {
    *state = default();
}
fn spare_change(
    mut commands: Commands,
    mut state: ResMut<SideRewards>,
    mut session: ResMut<Session>,
    spatial: SpatialQuery,
    players: Query<(Entity, &Player, &Transform, &crate::life::Vitality)>,
    machines: Query<(
        Entity,
        &Transform,
        Option<&crate::interactions::Usable>,
        Option<&crate::stations::Station>,
    )>,
) {
    let Ok((owner, player, pose, health)) = players.single() else {
        return;
    };
    if !player.prone || health.state() != crate::life::LifeState::Standing {
        return;
    }
    for (entity, location, usable, station) in &machines {
        let eligible = usable.is_some_and(|u| {
            matches!(
                u.source.action,
                crate::origins::content::InteractionAction::Perk { .. }
            )
        }) || matches!(station, Some(crate::stations::Station::PackAPunch));
        if !eligible || state.spare.contains(&entity) {
            continue;
        }
        let touching = spatial
            .shape_intersections(
                &Collider::cylinder(0.889, 1.6256),
                location.translation + Vec3::Y * 0.8128,
                Quat::IDENTITY,
                &SpatialQueryFilter::default(),
            )
            .contains(&owner);
        if touching {
            state.spare.insert(entity);
            session.award(25);
            crate::audio::emit(
                &mut commands,
                "purchase",
                crate::audio::SoundSource::World(pose.translation),
            );
        }
    }
}
fn fires(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<SideRewards>,
    session: Res<Session>,
    weather: Res<crate::environment::weather::Weather>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    triggers: Query<&crate::environment::triggers::MapTrigger>,
    players: Query<&crate::weapons::WeaponInventory, With<Player>>,
) {
    if let Some(round) = state.blood_round {
        if session.round <= round || weather.kind == crate::environment::weather::WeatherKind::Rain
        {
            return;
        }
        state.blood_round = None;
        state.fires_out = [false; 3];
    }
    state.fire_timer = (state.fire_timer - time.delta_secs()).max(0.);
    if state.fire_timer == 0. && state.fires_out.iter().any(|b| *b) {
        state.fires_out = [false; 3];
    }
    for hit in hits.read() {
        let Ok(inventory) = players.get(hit.attacker) else {
            continue;
        };
        if !inventory.active().is_some_and(|w| {
            matches!(
                w.definition.0.as_str(),
                "weapon.staff_water_zm" | "weapon.staff_water_upgraded_zm"
            )
        }) {
            continue;
        }
        let Ok(trigger) = triggers.get(hit.target) else {
            continue;
        };
        let Some(source) = game.environment.triggers.iter().find(|t| {
            t.id == trigger.0
                && t.source
                    .get("targetname")
                    .is_some_and(|n| n == "wagon_damage_trigger")
        }) else {
            continue;
        };
        let Some(index) = source
            .source
            .get("script_noteworthy")
            .and_then(|n| n.strip_prefix("wagon_"))
            .and_then(|n| n.strip_suffix("_fire"))
            .and_then(|n| n.parse::<usize>().ok())
            .and_then(|n| n.checked_sub(1))
            .filter(|i| *i < 3)
        else {
            continue;
        };
        if !state.fires_out[index] {
            state.fires_out[index] = true;
            if state.fire_timer == 0. {
                state.fire_timer = 30.;
            }
        }
    }
    if state.fires_out.iter().all(|b| *b) {
        state.blood_round = Some(session.round);
        state.fire_timer = 0.;
        crate::audio::emit(
            &mut commands,
            "mus_side_stinger_1",
            crate::audio::SoundSource::World(Vec3::from_array(BLOOD)),
        );
        if let Some(index) = game
            .content
            .powerups
            .iter()
            .position(|p| p.id.0.contains("zombie_blood"))
        {
            let id = commands
                .spawn((
                    SessionEntity,
                    crate::powerups::Pickup {
                        definition: index,
                        remaining: 26.5,
                    },
                    Transform::from_translation(Vec3::from_array(BLOOD)),
                    Visibility::Inherited,
                ))
                .id();
            if let Some(scene) = game
                .content
                .presentations
                .iter()
                .find(|p| Some(&p.id) == game.content.powerups[index].presentation.as_ref())
                .and_then(|p| p.scene.as_ref())
            {
                commands
                    .entity(id)
                    .insert(WorldAssetRoot(server.load(scene.clone())));
            }
        }
    }
}
fn medallions(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<SideRewards>,
    players: Query<&Transform, (With<Player>, Without<crate::buildables::Drone>)>,
    mut drones: Query<(&mut crate::buildables::Drone, &Transform), Without<Player>>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
) {
    if state.delivered {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    for (mut drone, pose) in &mut drones {
        if drone.returning {
            if state.drone_target.take().is_some() || state.delivery {
                drone.quest_goal = None;
                state.drone_wait = 0.;
            }
            continue;
        }
        if state.delivery {
            let goal = Vec3::from_array(REWARD) + Vec3::Y * 0.762;
            drone.quest_goal = Some(goal);
            if pose.translation.distance(goal) < 0.1016 {
                state.drone_wait += time.delta_secs();
                if state.drone_wait >= 1. {
                    state.delivered = true;
                    state.delivery = false;
                    drone.quest_goal = None;
                    crate::audio::emit(
                        &mut commands,
                        "zmb_perks_packa_ready",
                        crate::audio::SoundSource::World(goal),
                    );
                    crate::audio::emit(
                        &mut commands,
                        "mus_side_stinger_4",
                        crate::audio::SoundSource::World(goal),
                    );
                    let id = commands
                        .spawn((
                            SessionEntity,
                            Reward,
                            Transform::from_translation(Vec3::from_array(REWARD))
                                .with_rotation(Quat::from_rotation_y(90_f32.to_radians())),
                            Visibility::Inherited,
                        ))
                        .id();
                    if let Some(weapon) = game
                        .content
                        .weapons
                        .iter()
                        .find(|w| w.id.0 == "weapon.mg08_upgraded_zm")
                        && let Some(scene) = game
                            .content
                            .presentations
                            .iter()
                            .find(|p| Some(&p.id) == weapon.pickup.as_ref())
                            .and_then(|p| p.scene.as_ref())
                    {
                        commands
                            .entity(id)
                            .insert(WorldAssetRoot(server.load(scene.clone())));
                    }
                }
            }
            continue;
        }
        if state.drone_target.is_none() && drone.quest_goal.is_none() {
            state.drone_target = MEDALLIONS
                .iter()
                .enumerate()
                .filter(|(i, _)| !state.seals[*i])
                .filter(|(_, p)| player.translation.distance(Vec3::from_array(**p)) < 12.7)
                .min_by(|a, b| {
                    pose.translation
                        .distance_squared(Vec3::from_array(*a.1))
                        .total_cmp(&pose.translation.distance_squared(Vec3::from_array(*b.1)))
                })
                .map(|(i, _)| i);
        }
        if let Some(index) = state.drone_target {
            let goal = Vec3::from_array(MEDALLIONS[index]) + Vec3::Y * 0.762;
            drone.quest_goal = Some(goal);
            if pose.translation.distance(goal) < 0.1016 {
                state.drone_wait += time.delta_secs();
                if state.drone_wait >= 1. {
                    state.seals[index] = true;
                    state.drone_target = None;
                    state.drone_wait = 0.;
                    drone.quest_goal = None;
                    crate::audio::emit(
                        &mut commands,
                        "zmb_medallion_pickup",
                        crate::audio::SoundSource::World(goal),
                    );
                    let count = state.seals.iter().filter(|v| **v).count();
                    crate::audio::emit(
                        &mut commands,
                        format!("vox_maxi_drone_pickups_{}", count - 1),
                        crate::audio::SoundSource::World(goal),
                    );
                    if count == 4 {
                        state.delivery = true;
                        crate::audio::emit(
                            &mut commands,
                            "vox_maxi_drone_pickups_4",
                            crate::audio::SoundSource::World(goal),
                        );
                    }
                }
            }
        }
    }
}
pub(crate) fn collect(
    mut commands: Commands,
    input: Res<crate::input::GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut state: ResMut<SideRewards>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    game: Res<GameDefinition>,
    spatial: SpatialQuery,
    rewards: Query<(Entity, &Transform), With<Reward>>,
    mut players: Query<
        (
            Entity,
            &Player,
            &Transform,
            &crate::life::Vitality,
            &mut crate::weapons::WeaponInventory,
            Has<crate::perks::PerkDrinking>,
            Option<&crate::actions::HandReservation>,
        ),
        Without<Reward>,
    >,
) {
    if focus.target.is_some() {
        return;
    }
    let Ok((owner, player, pose, health, mut inventory, drinking, reservation)) =
        players.single_mut()
    else {
        return;
    };
    if health.state() != crate::life::LifeState::Standing {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    for (entity, location) in &rewards {
        let delta = location.translation - eye;
        let Ok(direction) = Dir3::new(delta) else {
            continue;
        };
        if delta.length() > 2.54
            || forward.dot(*direction) < 0.8
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
        focus.prompt = format!(
            "Press {} to take Magna Collider",
            settings.label(crate::input::Action::Interact)
        );
        if input.use_held
            && !drinking
            && reservation.is_none()
            && inventory.action == crate::weapons::WeaponAction::Ready
            && inventory.active().is_some()
            && let Some(weapon) = game
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.mg08_upgraded_zm")
        {
            if inventory.owned().any(|(_, w)| w.definition == weapon.id)
                || inventory.receive(weapon).is_ok()
            {
                inventory.fill_received(weapon);
                commands.entity(entity).despawn();
                state.magna_collected = true;
                focus.accepted += 1;
            }
        }
        break;
    }
}
