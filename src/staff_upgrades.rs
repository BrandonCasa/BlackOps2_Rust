use crate::{
    GameState,
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    interactions::InteractionFocus,
    life::{LifeState, Vitality},
    player::Player,
    quests::{Element, MainQuest, QuestStage, SoulCollected, SoulCollector},
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;
pub const ELEMENTS: [Element; 4] = [
    Element::Fire,
    Element::Ice,
    Element::Wind,
    Element::Lightning,
];
pub const WEAPONS: [&str; 4] = [
    "weapon.staff_fire_zm",
    "weapon.staff_water_zm",
    "weapon.staff_air_zm",
    "weapon.staff_lightning_zm",
];
const CHARGERS: [Vec3; 4] = [
    Vec3::new(256.6416, -9.179052, 207.057752),
    Vec3::new(268.6304, -9.230106, 195.06946),
    Vec3::new(268.88186, -9.205468, 206.8068),
    Vec3::new(256.6416, -9.230868, 195.083176),
];
#[derive(Resource, Default)]
pub struct StaffUpgrades {
    pub rings: [u8; 4],
    pub orb_shot: [bool; 4],
    pub unlocked: [bool; 4],
    pub inserted: [bool; 4],
    pub charged: [bool; 4],
    pub souls: [u32; 4],
    latched: bool,
    ring_delay: f32,
    readiness: [f32; 4],
    pickup_delay: [f32; 4],
    deposited: [Option<crate::weapons::AmmoSnapshot>; 4],
    reload_ready: [bool; 4],
    arrivals: Vec<(usize, f32)>,
}
#[derive(Component)]
pub(crate) enum Target {
    Lever(usize),
    Charger(usize),
}
#[derive(Component)]
struct Orb(usize);
#[derive(Component, Default)]
struct OrbPresentation {
    glow: Option<Entity>,
    trail: Option<Entity>,
    sound: Option<Entity>,
    shot: bool,
    started: bool,
    arrived: bool,
}
#[derive(Component)]
struct OrbApproach {
    phase: u8,
    looking: f32,
    poll: f32,
    path: Vec<Vec3>,
    next: usize,
}
#[derive(Component)]
struct OrbTransit {
    path: Vec<Vec3>,
    next: usize,
    teleport: usize,
}
#[derive(Component)]
struct Disc(usize);
#[derive(Component, Default)]
struct ChargerPresentation {
    phase: u8,
    model: Option<Entity>,
    glow: Option<Entity>,
    sound: Option<Entity>,
}
pub struct StaffUpgradePlugin;
impl Plugin for StaffUpgradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StaffUpgrades>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (orbs, progress)
                    .chain()
                    .in_set(GameStep::Rewards)
                    .after(crate::quests::souls)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                charger_presentation
                    .in_set(GameStep::Rewards)
                    .after(progress)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                interact
                    .in_set(GameStep::Interactions)
                    .after(crate::staff_puzzles::relay)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn randomize(state: &mut StaffUpgrades, session: &mut crate::gameplay::Session) {
    let mut previous = 0;
    for ring in &mut state.rings {
        *ring = (previous + 1 + session.random() as u8 % 2) % 4;
        previous = *ring;
    }
}
fn setup(
    mut commands: Commands,
    mut state: ResMut<StaffUpgrades>,
    mut session: ResMut<crate::gameplay::Session>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    extras: Query<(Entity, &bevy::gltf::GltfExtras)>,
) {
    *state = default();
    randomize(&mut state, &mut session);
    for (index, point) in crate::staff_puzzles::source::LEVERS {
        commands.spawn((
            SessionEntity,
            Target::Lever(*index),
            Transform::from_translation(Vec3::from_array(*point)),
        ));
    }
    for index in 0..4 {
        commands.spawn((
            SessionEntity,
            Target::Charger(index),
            ChargerPresentation::default(),
            Transform::from_translation(CHARGERS[index]),
            SoulCollector {
                radius: 76.2,
                collected: 0,
                required: 20,
                active: false,
                ordinary_only: true,
            },
        ));
    }
    for (index, point) in [
        [0.8128, -14.3256, 0.8128],
        [-0.8128, -14.3256, -0.8128],
        [0.8128, -14.3256, -0.8128],
        [-0.8128, -14.3256, 0.8128],
    ]
    .into_iter()
    .enumerate()
    {
        let mut orb = commands.spawn((
            SessionEntity,
            Orb(index),
            OrbPresentation::default(),
            Transform::from_translation(Vec3::from_array(point)),
            Visibility::Hidden,
            ColliderDisabled,
            Collider::sphere(0.4),
            Sensor,
            CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE),
            crate::environment::support::EnvironmentOwner::Quest,
        ));
        let model = [
            "p6_zm_tm_orb_fire_lod0",
            "p6_zm_tm_orb_ice_lod0",
            "p6_zm_tm_orb_wind_lod0",
            "p6_zm_tm_orb_lightning_lod0",
        ][index];
        if let Some(scene) = game
            .content
            .presentations
            .iter()
            .filter_map(|p| p.scene.as_ref())
            .find(|s| s.contains(model))
        {
            orb.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
    let scene = game
        .content
        .presentations
        .iter()
        .filter_map(|p| p.scene.as_ref())
        .find(|s| s.contains("p6_zm_tm_chamber_mid_piece_lod0"));
    for (index, point) in [
        [-0.0254, -11.9888, 0.0127],
        [-0.0254, -10.5664, 0.0127],
        [-0.0254, -9.144, 0.0127],
        [-0.0508, -7.7216, 0.0254],
    ]
    .into_iter()
    .enumerate()
    {
        let mut disc = commands.spawn((
            SessionEntity,
            Disc(index),
            Transform::from_translation(Vec3::from_array(point)),
        ));
        if let Some(scene) = scene {
            disc.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
    for (entity, extras) in &extras {
        if serde_json::from_str::<serde_json::Value>(&extras.value)
            .ok()
            .is_some_and(|data| {
                data["static_instance"] == true
                    && data["source_model"]
                        .as_str()
                        .is_some_and(|model| model.contains("p6_zm_tm_chamber_mid_piece"))
            })
        {
            commands.entity(entity).insert(Visibility::Hidden);
        }
    }
}
fn charger_presentation(
    mut commands: Commands,
    state: Res<StaffUpgrades>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    mut chargers: Query<(Entity, &Target, &Transform, &mut ChargerPresentation)>,
) {
    for (entity, target, pose, mut visual) in &mut chargers {
        let Target::Charger(index) = target else {
            continue;
        };
        let phase = if !state.inserted[*index] {
            0
        } else if state.charged[*index] {
            2
        } else {
            1
        };
        if phase != visual.phase {
            if let Some(model) = visual.model.take() {
                commands.entity(model).try_despawn();
            }
            if let Some(glow) = visual.glow.take() {
                commands.entity(glow).try_despawn();
            }
            if let Some(sound) = visual.sound.take() {
                commands.entity(sound).try_despawn();
            }
            if phase > 0 {
                let id = if phase == 2 {
                    WEAPONS[*index].replace("_zm", "_upgraded_zm")
                } else {
                    WEAPONS[*index].into()
                };
                if let Some(weapon) = game.content.weapons.iter().find(|w| w.id.0 == id)
                    && let Some(scene) = game
                        .content
                        .presentations
                        .iter()
                        .find(|p| Some(&p.id) == weapon.pickup.as_ref())
                        .and_then(|p| p.scene.as_ref())
                {
                    visual.model = Some(
                        commands
                            .spawn((
                                SessionEntity,
                                WorldAssetRoot(server.load(scene.clone())),
                                *pose,
                            ))
                            .id(),
                    );
                }
                if phase == 2 {
                    visual.sound = Some(crate::audio::emit_loop(
                        &mut commands,
                        "zmb_squest_charge_soul_full_loop",
                        crate::audio::SoundSource::attached(entity),
                    ));
                }
            }
            visual.phase = phase;
        }
        if phase == 2 && state.pickup_delay[*index] <= 0. && visual.glow.is_none() {
            let family = ["fire", "ice", "air", "elec"][*index];
            visual.glow = Some(crate::native_fx::attach(
                &mut commands,
                format!("maps/zombie_tomb/fx_tomb_elem_reveal_{family}_glow"),
                entity,
                Transform::from_xyz(0., 0.2032, 0.),
            ));
        }
    }
}
fn orbs(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<StaffUpgrades>,
    puzzles: Res<crate::staff_puzzles::StaffPuzzles>,
    mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    mut orbs: Query<(
        Entity,
        &Orb,
        &mut Transform,
        &mut Visibility,
        Option<&mut OrbTransit>,
        Option<&mut OrbApproach>,
        &mut OrbPresentation,
    )>,
    players: Query<(&Player, &Transform), Without<Orb>>,
    mut discs: Query<(&Disc, &mut Transform), (Without<Orb>, Without<Player>)>,
    mut session: ResMut<crate::gameplay::Session>,
) {
    state.ring_delay = (state.ring_delay - time.delta_secs()).max(0.);
    for hit in hits.read() {
        if let Ok((_, orb, _, _, None, None, presentation)) = orbs.get(hit.target)
            && presentation.arrived
            && puzzles.first[orb.0]
            && puzzles.second[orb.0]
            && !state.unlocked[orb.0]
            && hit.weapon.0 == WEAPONS[orb.0]
        {
            state.orb_shot[orb.0] = true;
        }
    }
    for (entity, orb, mut pose, mut visibility, transit, approach, mut presentation) in &mut orbs {
        if puzzles.first[orb.0] && !presentation.started {
            presentation.started = true;
            let returning = crate::staff_puzzles::source::ORB_RETURN[orb.0];
            pose.translation = Vec3::from_array(returning[1]);
            *visibility = Visibility::Inherited;
            commands.entity(entity).insert(OrbApproach {
                phase: 1,
                looking: 0.,
                poll: 0.,
                path: Vec::new(),
                next: 0,
            });
            crate::audio::emit(
                &mut commands,
                "zmb_squest_crystal_leave",
                crate::audio::SoundSource::attached(entity),
            );
            presentation.sound = Some(crate::audio::emit_loop(
                &mut commands,
                "zmb_squest_crystal_loop",
                crate::audio::SoundSource::attached(entity),
            ));
            continue;
        }
        if let Some(mut approach) = approach {
            if approach.phase == 1 {
                approach.poll += time.delta_secs();
                while approach.poll >= 0.1 {
                    approach.poll -= 0.1;
                    let occupied = players.iter().any(|(_, body)| {
                        (body.translation - Vec3::new(262.636, -7.9756, 200.8124))
                            .with_y(0.)
                            .length_squared()
                            < 45.72_f32.powi(2)
                    });
                    let looking = players.iter().any(|(player, body)| {
                        let delta = pose.translation - (body.translation - Vec3::Y * 0.85);
                        let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.)
                            * Vec3::NEG_Z;
                        delta.with_y(0.).length_squared() < 4.572_f32.powi(2)
                            && delta.normalize_or_zero().dot(forward) > 30_f32.to_radians().cos()
                    });
                    approach.looking = if !occupied || looking {
                        approach.looking + 0.1
                    } else {
                        0.
                    };
                }
                if approach.looking >= 1. {
                    let returning = crate::staff_puzzles::source::ORB_RETURN[orb.0];
                    approach.path = vec![
                        Vec3::from_array(returning[0]),
                        Vec3::from_array(returning[0]) + Vec3::Y * 25.4,
                    ];
                    approach.phase = 2;
                    presentation.trail = Some(crate::native_fx::attach(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_puzzle_plinth_trail",
                        entity,
                        Transform::IDENTITY,
                    ));
                }
            } else if approach.phase == 3 {
                if puzzles.second[orb.0] {
                    approach.path = crate::staff_puzzles::source::ORB_DESCENT
                        .iter()
                        .map(|p| Vec3::from_array(*p))
                        .collect();
                    approach.path.push(Vec3::from_array(
                        [
                            [0.8128, -14.3256, 0.8128],
                            [-0.8128, -14.3256, -0.8128],
                            [0.8128, -14.3256, -0.8128],
                            [-0.8128, -14.3256, 0.8128],
                        ][orb.0],
                    ));
                    approach.next = 0;
                    approach.phase = 4;
                    *visibility = Visibility::Inherited;
                    state.ring_delay = f32::MAX;
                }
            } else if approach.next < approach.path.len() {
                let destination = approach.path[approach.next];
                let delta = destination - pose.translation;
                pose.translation += delta.clamp_length_max(7.62 * time.delta_secs());
                if pose.translation.distance(destination) < 0.001 {
                    approach.next += 1;
                }
            } else if approach.phase == 2 {
                pose.translation =
                    Vec3::from_array(crate::staff_puzzles::source::ORB_DESCENT[0]) + Vec3::Y * 76.2;
                *visibility = Visibility::Hidden;
                approach.phase = 3;
            } else {
                presentation.arrived = true;
                state.ring_delay = 0.;
                commands.entity(entity).remove::<OrbApproach>();
                if let Some(trail) = presentation.trail.take() {
                    commands.entity(trail).try_despawn();
                }
                crate::audio::emit(
                    &mut commands,
                    "zmb_squest_crystal_arrive",
                    crate::audio::SoundSource::attached(entity),
                );
            }
            continue;
        }
        if state.orb_shot[orb.0] && !presentation.shot {
            if let Some(sound) = presentation.sound.take() {
                commands.entity(sound).try_despawn();
            }
            presentation.shot = true;
            crate::audio::emit(
                &mut commands,
                "zmb_squest_crystal_charge",
                crate::audio::SoundSource::attached(entity),
            );
            presentation.sound = Some(crate::audio::emit_loop(
                &mut commands,
                "zmb_squest_crystal_charge_loop",
                crate::audio::SoundSource::attached(entity),
            ));
        }
        if let Some(mut transit) = transit {
            if transit.next < transit.path.len() {
                let destination = transit.path[transit.next];
                let delta = destination - pose.translation;
                if transit.next == transit.teleport {
                    pose.translation = destination;
                } else {
                    pose.translation += delta.clamp_length_max(7.62 * time.delta_secs());
                }
                if pose.translation.distance(destination) < 0.001 {
                    transit.next += 1;
                    if transit.next == 1 {
                        randomize(&mut state, &mut session);
                        state.ring_delay = 1.;
                    }
                }
            } else {
                if let Some(trail) = presentation.trail.take() {
                    commands.entity(trail).try_despawn();
                }
                state.unlocked[orb.0] = true;
                commands.entity(entity).remove::<OrbTransit>();
                crate::audio::emit(
                    &mut commands,
                    "zmb_squest_crystal_arrive",
                    crate::audio::SoundSource::attached(entity),
                );
            }
            continue;
        }
        let available = presentation.arrived && !state.unlocked[orb.0];
        *visibility = if available || state.unlocked[orb.0] {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if (presentation.shot || state.unlocked[orb.0]) && presentation.glow.is_none() {
            let name = ["fire", "ice", "air", "elec"][orb.0];
            presentation.glow = Some(crate::native_fx::attach(
                &mut commands,
                format!("maps/zombie_tomb/fx_tomb_elem_reveal_{name}_glow"),
                entity,
                Transform::IDENTITY,
            ));
        }
        if available {
            commands
                .entity(entity)
                .insert(crate::environment::support::ShootableEnvironment)
                .remove::<ColliderDisabled>();
        } else {
            commands
                .entity(entity)
                .remove::<crate::environment::support::ShootableEnvironment>()
                .insert(ColliderDisabled);
        }
        let orientation = [2, 0, 3, 1][orb.0];
        if available
            && state.orb_shot[orb.0]
            && state.ring_delay <= 0.
            && state.rings.into_iter().all(|r| r == orientation)
        {
            let ascent = crate::staff_puzzles::source::ORB_ASCENT;
            let mut path = vec![Vec3::new(
                pose.translation.x,
                ascent[0][1],
                pose.translation.z,
            )];
            path.extend(ascent.iter().map(|p| Vec3::from_array(*p)));
            path.push(*path.last().unwrap() + Vec3::Y * 50.8);
            let teleport = path.len();
            let returning = crate::staff_puzzles::source::ORB_RETURN[orb.0];
            path.push(Vec3::from_array(returning[0]) + Vec3::Y * 25.4);
            path.extend(returning.iter().map(|p| Vec3::from_array(*p)));
            commands
                .entity(entity)
                .insert((
                    OrbTransit {
                        path,
                        next: 0,
                        teleport,
                    },
                    ColliderDisabled,
                ))
                .remove::<crate::environment::support::ShootableEnvironment>();
            presentation.trail = Some(crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_puzzle_plinth_trail",
                entity,
                Transform::IDENTITY,
            ));
            state.ring_delay = f32::MAX;
            crate::audio::emit(
                &mut commands,
                "zmb_squest_crystal_leave",
                crate::audio::SoundSource::attached(entity),
            );
        }
    }
    for (disc, mut pose) in &mut discs {
        pose.rotation =
            Quat::from_rotation_y(state.rings[disc.0] as f32 * std::f32::consts::FRAC_PI_2);
    }
}
fn progress(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<crate::assets::GameDefinition>,
    mut state: ResMut<StaffUpgrades>,
    mut events: MessageReader<SoulCollected>,
    targets: Query<(Entity, &Target, &Transform)>,
    mut collectors: Query<&mut SoulCollector>,
    enemies: Query<&Transform, With<crate::enemies::Zombie>>,
    mut quest: ResMut<MainQuest>,
) {
    let dt = time.delta_secs();
    for event in events.read() {
        let Ok((_, Target::Charger(index), center)) = targets.get(event.collector) else {
            continue;
        };
        if let Ok(pose) = enemies.get(event.enemy) {
            crate::quests::soul_effect(
                &mut commands,
                pose.translation,
                center.translation + Vec3::Y * 0.127,
                1.,
                0.5,
            );
        }
        if state.charged[*index] {
            state.arrivals.push((*index, 1.5));
            continue;
        }
        state.souls[*index] = (state.souls[*index] + 1).min(20);
        if event.completed {
            state.readiness[*index] = 0.5;
        }
    }
    for (index, timer) in &mut state.arrivals {
        *timer -= dt;
        let _ = index;
    }
    let arrived: Vec<_> = state
        .arrivals
        .iter()
        .filter(|(_, t)| *t <= 0.)
        .map(|(i, _)| *i)
        .collect();
    state.arrivals.retain(|(_, t)| *t > 0.);
    for index in arrived {
        if let Some(snapshot) = state.deposited[index].as_mut()
            && let Some(weapon) = game
                .content
                .weapons
                .iter()
                .find(|w| w.id == snapshot.weapon.definition)
        {
            let reserve = snapshot.reserve.saturating_add(weapon.reserve / 20);
            snapshot.reserve = reserve.min(weapon.reserve);
            if reserve > weapon.reserve
                && let Some((entity, _, _)) = targets
                    .iter()
                    .find(|(_, target, _)| matches!(target, Target::Charger(i) if *i == index))
                && let Ok(mut collector) = collectors.get_mut(entity)
            {
                collector.active = false;
            }
            state.reload_ready[index] = true;
        }
    }
    for index in 0..4 {
        state.pickup_delay[index] = (state.pickup_delay[index] - dt).max(0.);
        if state.readiness[index] > 0. {
            state.readiness[index] = (state.readiness[index] - dt).max(0.);
            if state.readiness[index] == 0. {
                state.charged[index] = true;
                state.pickup_delay[index] = 0.65;
                quest.upgraded.insert(ELEMENTS[index]);
                if let Some((entity, _, _)) = targets
                    .iter()
                    .find(|(_, t, _)| matches!(t,Target::Charger(i) if *i==index))
                {
                    crate::audio::emit(
                        &mut commands,
                        "zmb_squest_charge_soul_full",
                        crate::audio::SoundSource::attached(entity),
                    );
                }
            }
        }
    }
    if quest.upgraded.len() == 4 {
        quest.transition(QuestStage::UpgradeStaffs, QuestStage::PlaceStaffs);
    }
}
pub(crate) fn interact(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut state: ResMut<StaffUpgrades>,
    mut focus: ResMut<InteractionFocus>,
    game: Res<crate::assets::GameDefinition>,
    spatial: SpatialQuery,
    mut targets: Query<(Entity, &Target, &Transform, Option<&mut SoulCollector>), Without<Player>>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut WeaponInventory,
        Has<crate::perks::PerkDrinking>,
        Option<&crate::actions::HandReservation>,
    )>,
) {
    if !input.use_held {
        state.latched = false;
    }
    let Ok((owner, player, pose, health, mut inventory, drinking, reservation)) =
        players.single_mut()
    else {
        return;
    };
    if health.state() != LifeState::Standing || focus.target.is_some() {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let selected = targets
        .iter()
        .filter_map(|(entity, _, pose, _)| {
            let delta = pose.translation - eye;
            let distance = delta.length();
            let direction = Dir3::new(delta).ok()?;
            (distance < 3.048
                && forward.dot(*direction) > 0.7
                && spatial
                    .cast_ray(
                        eye,
                        direction,
                        (distance - 0.1).max(0.),
                        true,
                        &crate::environment::solids([owner]),
                    )
                    .is_none())
            .then_some((entity, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(e, _)| e);
    let Some(entity) = selected else {
        return;
    };
    let Ok((_, target, _, collector)) = targets.get_mut(entity) else {
        return;
    };
    focus.target = Some(entity);
    focus.progress = 0.;
    focus.hold_seconds = 0.;
    let key = settings.label(crate::input::Action::Interact);
    let pressed = input.use_held
        && !state.latched
        && !drinking
        && reservation.is_none()
        && inventory.action == WeaponAction::Ready;
    match target {
        Target::Lever(index) => {
            focus.prompt = format!(
                "Press {key} to rotate excavation ring ({})",
                state.rings[*index]
            );
            if !pressed || state.ring_delay > 0. {
                return;
            }
            state.rings[*index] = (state.rings[*index] + 3) % 4;
            state.ring_delay = 1.;
            crate::audio::emit(
                &mut commands,
                "zmb_crypt_lever",
                crate::audio::SoundSource::attached(entity),
            );
        }
        Target::Charger(index) => {
            if !state.unlocked[*index] {
                focus.prompt = "Complete the elemental puzzles and excavation alignment".into();
                return;
            }
            if state.inserted[*index]
                && (!state.charged[*index]
                    || state.pickup_delay[*index] > 0.
                    || state.deposited[*index].is_some() && !state.reload_ready[*index])
            {
                focus.prompt = format!("Staff charging: {}/20", state.souls[*index]);
                return;
            }
            focus.prompt = if state.inserted[*index] {
                format!("Press {key} to retrieve upgraded staff")
            } else {
                format!("Press {key} to insert staff")
            };
            if !pressed {
                return;
            }
            if state.inserted[*index] {
                let id = WEAPONS[*index].replace("_zm", "_upgraded_zm");
                let Some(definition) = game.content.weapons.iter().find(|w| w.id.0 == id) else {
                    return;
                };
                if inventory.exchange_staff(definition).is_err() {
                    return;
                }
                if let Some(snapshot) = state.deposited[*index].take() {
                    inventory.restore_received(definition, &snapshot);
                } else {
                    inventory.fill_received(definition);
                }
                state.inserted[*index] = false;
                if let Some(mut collector) = collector {
                    collector.active = false;
                }
                state.arrivals.retain(|(i, _)| *i != *index);
            } else {
                let expected = if state.charged[*index] {
                    WEAPONS[*index].replace("_zm", "_upgraded_zm")
                } else {
                    WEAPONS[*index].to_string()
                };
                if !inventory
                    .active()
                    .is_some_and(|w| w.definition.0 == expected)
                {
                    return;
                }
                let snapshot = game
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id.0 == expected)
                    .and_then(|w| inventory.ammo_snapshot(w));
                if inventory.deposit_active().is_err() {
                    return;
                }
                if state.charged[*index] {
                    state.deposited[*index] = snapshot;
                    state.reload_ready[*index] = false;
                }
                state.inserted[*index] = true;
                if let Some(mut collector) = collector {
                    collector.active = true;
                    collector.collected = 0;
                    collector.required = if state.charged[*index] { u32::MAX } else { 20 };
                }
                if inventory.active().is_none()
                    && let Some(fists) = game
                        .content
                        .weapons
                        .iter()
                        .find(|w| w.id.0 == "weapon.zombie_fists_zm")
                {
                    let _ = inventory.grant(fists);
                }
            }
            crate::audio::emit(
                &mut commands,
                "zmb_squest_charge_place_staff",
                crate::audio::SoundSource::attached(entity),
            );
        }
    }
    state.latched = true;
    focus.accepted += 1;
}
