use crate::{
    GameState,
    assets::GameDefinition,
    economy::PointLedger,
    environment::support::DamageEmitter,
    gameplay::{GameStep, Session, SessionEntity},
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection},
    life::{LifeState, Vitality},
    movement::platforms::{MovingPlatform, PlatformSupport},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;
mod source;

#[derive(Component)]
pub(crate) struct TankBody;
#[derive(Component)]
pub(crate) struct CallBox(usize);
#[derive(Component)]
pub(crate) struct TankRescue {
    pub fade: f32,
    elapsed: f32,
    from: Vec3,
    to: Vec3,
    node: crate::origins::ContentId,
    disabled: bool,
}
#[derive(Component)]
struct TankBurn {
    remaining: f32,
    amount: u32,
}
#[derive(Component)]
pub(crate) struct TankMotion {
    tag: usize,
    alias: String,
    elapsed: f32,
    down: bool,
}
#[derive(Component)]
struct TankChase {
    tag: usize,
    remaining: f32,
    onboard: bool,
    previous: Transform,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TankPhase {
    #[default]
    Ready,
    Departing,
    Moving,
    Cooling,
}
#[derive(Default)]
struct FlameCycle {
    check: f32,
    empty: f32,
    interval: f32,
    active: f32,
    fx: Option<Entity>,
    sound: Option<Entity>,
}
#[derive(Resource, Default)]
pub struct Tank {
    pub entity: Option<Entity>,
    pub phase: TankPhase,
    pub station: usize,
    pub trips: u32,
    pub timer: f32,
    node: usize,
    speed: f32,
    rider_seconds: f32,
    free_next: bool,
    called: bool,
    latched: bool,
    activated_once: bool,
    engine: Option<Entity>,
    engine_delay: f32,
    flames: [FlameCycle; 3],
    drops: [u32; 3],
    drop_check: f32,
}
pub struct TankPlugin;
impl Plugin for TankPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tank>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (
                    movement.before(crate::movement::platforms::carry),
                    rescue.after(movement),
                )
                    .in_set(GameStep::Movement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (board.before(crate::enemies::lifecycle), flames)
                    .in_set(GameStep::EnemyIntent)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                drops
                    .in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                interact
                    .in_set(GameStep::Interactions)
                    .after(crate::buildables::collect_shield)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut tank: ResMut<Tank>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut parts: ResMut<crate::environment::parts::Parts>,
    mut session: ResMut<Session>,
) {
    *tank = Tank::default();
    let hull = Collider::compound(vec![(
        Vec3::new(0., 0.4914, -1.3585),
        Quat::IDENTITY,
        Collider::cuboid(6.1334, 2.9828, 10.9971),
    )]);
    let entity = commands
        .spawn((
            TankBody,
            MovingPlatform,
            SessionEntity,
            RigidBody::Kinematic,
            CustomPositionIntegration,
            hull,
            Transform::from_translation(Vec3::from_array(source::ROUTE[0].0) + Vec3::Y)
                .with_rotation(Quat::from_rotation_y((-104.2_f32).to_radians())),
            Visibility::Inherited,
            DamageEmitter::Vehicle("vehicle.tank".into()),
        ))
        .id();
    tank.entity = Some(entity);
    tank.node = 1;
    tank.drops = [session.round, session.round + 1, session.round + 2];
    for (i, flame) in tank.flames.iter_mut().enumerate() {
        flame.check = 1. + i as f32 * 0.25;
        flame.interval = 3. + (session.random() % 3001) as f32 / 1000.;
    }
    if let Some(scene) = crate::assets::native_scene(&game, "veh_t6_dlc_mkiv_tank") {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                WorldAssetRoot(server.load(scene.clone())),
                Transform::from_xyz(0., -1., 0.)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            ));
        });
    }
    for (site, position) in [[9.5758, 2.413, 75.819], [-6.9342, -6.4516, -115.2398]]
        .into_iter()
        .enumerate()
    {
        commands.spawn((
            CallBox(site),
            SessionEntity,
            Transform::from_translation(Vec3::from_array(position)),
        ));
    }
    for (piece, position) in [
        ("upper_staff", [58.019188, -2.85623, -76.949554]),
        ("middle_staff", [-3.69824, 5.207254, -10.920476]),
        ("lower_staff", [1.49987, 10.255504, 70.197218]),
    ] {
        crate::environment::digging::spawn_part(
            &mut commands,
            &server,
            &game,
            &mut parts,
            &crate::origins::ContentId(format!("part.elemental_staff_lightning.{piece}")),
            Vec3::from_array(position),
        );
    }
    tank.engine = Some(crate::audio::emit_loop(
        &mut commands,
        "zmb_tank_idle",
        crate::audio::SoundSource::attached(entity),
    ));
}
fn engine(commands: &mut Commands, tank: &mut Tank, alias: &str, entity: Entity) {
    if let Some(previous) = tank.engine.take() {
        commands.entity(previous).despawn();
    }
    tank.engine = Some(crate::audio::emit_loop(
        commands,
        alias,
        crate::audio::SoundSource::attached(entity),
    ));
}
fn movement(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut tank: ResMut<Tank>,
    game: Res<GameDefinition>,
    access: Res<crate::access::ZoneAccess>,
    spatial: SpatialQuery,
    mut bodies: Query<
        (Entity, &mut Transform, &DamageEmitter),
        (With<TankBody>, Without<Vitality>),
    >,
    actors: Query<
        (
            Entity,
            &Transform,
            &Vitality,
            Option<&PlatformSupport>,
            &Collider,
            Has<ColliderDisabled>,
        ),
        (With<Player>, Without<TankRescue>),
    >,
    rescues: Query<&TankRescue>,
    mut damage: ResMut<crate::damage::DamageQueue>,
) {
    let Ok((entity, mut pose, emitter)) = bodies.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    match tank.phase {
        TankPhase::Ready => return,
        TankPhase::Departing => tank.phase = TankPhase::Moving,
        TankPhase::Cooling => {
            tank.timer -= dt;
            if tank.timer <= 0. {
                tank.phase = TankPhase::Ready;
                crate::audio::emit(
                    &mut commands,
                    "zmb_tank_ready",
                    crate::audio::SoundSource::attached(entity),
                );
                engine(&mut commands, &mut tank, "zmb_tank_idle", entity);
            }
            return;
        }
        TankPhase::Moving => {}
    }
    if tank.engine_delay > 0. {
        tank.engine_delay -= dt;
        if tank.engine_delay <= 0. {
            engine(&mut commands, &mut tank, "zmb_tank_loop", entity);
        }
    }
    tank.rider_seconds += dt
        * actors
            .iter()
            .filter(|(_, _, health, support, _, _)| {
                matches!(health.state(), LifeState::Standing | LifeState::Downed)
                    && support.is_some_and(|s| s.entity == Some(entity))
            })
            .count() as f32;
    let (point, speed, stop) = source::ROUTE[tank.node];
    let destination = Vec3::from_array(point) + Vec3::Y;
    let delta = destination - pose.translation;
    let step = delta.clamp_length_max(tank.speed * dt);
    let before = pose.translation;
    pose.translation += step;
    let forward = delta.with_y(0.).normalize_or_zero();
    if forward.length_squared() > 0. {
        pose.rotation = Quat::from_rotation_y((-forward.x).atan2(-forward.z));
    }
    let mut reserved: std::collections::BTreeSet<_> =
        rescues.iter().map(|r| r.node.clone()).collect();
    for (target, position, health, support, collider, disabled) in &actors {
        if health.state() != LifeState::Standing
            || support.is_some_and(|s| s.entity == Some(entity))
        {
            continue;
        }
        let local = pose.rotation.inverse() * (position.translation - pose.translation);
        if local.x.abs() < 2.8
            && local.z > -6.86
            && local.z < 4.14
            && local.y.abs() < 1.5
            && (position.translation - before).dot(forward) > 0.
        {
            damage.0.push(crate::damage::Impact {
                attacker: entity,
                target,
                source: emitter.source(),
                category: crate::origins::DamageCategory::Crush,
                region: crate::weapons::HitRegion::Torso,
                amount: health.current().unwrap_or(0).saturating_add(1000),
            });
            let mut nodes: Vec<_> = game
                .content
                .navigation_nodes
                .iter()
                .filter(|n| {
                    access.reachable.contains(&n.zone)
                        && !reserved.contains(&n.id)
                        && Vec3::from_array(n.position).distance_squared(position.translation)
                            < 6.5024_f32.powi(2)
                })
                .collect();
            nodes.sort_by(|a, b| {
                Vec3::from_array(a.position)
                    .distance_squared(position.translation)
                    .total_cmp(&Vec3::from_array(b.position).distance_squared(position.translation))
            });
            if let Some(node) = nodes.into_iter().find(|n| {
                let point = Vec3::from_array(n.position) + Vec3::Y * (0.2032 + 0.9);
                let local = pose.rotation.inverse() * (point - pose.translation);
                (local.x.abs() > 3.2 || local.z < -7.2 || local.z > 4.5)
                    && spatial
                        .shape_intersections(
                            collider,
                            point,
                            Quat::IDENTITY,
                            &crate::environment::solids([target]),
                        )
                        .is_empty()
            }) {
                reserved.insert(node.id.clone());
                commands.entity(target).insert((
                    TankRescue {
                        fade: 0.,
                        elapsed: 0.,
                        from: position.translation,
                        to: Vec3::from_array(node.position) + Vec3::Y * (0.2032 + 0.9),
                        node: node.id.clone(),
                        disabled,
                    },
                    ColliderDisabled,
                    crate::actions::HandReservation::timed(
                        crate::actions::HandAction::Station,
                        4.2,
                    ),
                ));
            }
        }
    }
    if delta.length() <= step.length() + 0.001 {
        tank.speed = speed;
        tank.node = if tank.node + 1 >= source::ROUTE.len() {
            1
        } else {
            tank.node + 1
        };
        if stop {
            tank.station = 1 - tank.station;
            tank.trips += 1;
            tank.phase = TankPhase::Cooling;
            tank.timer = tank.rider_seconds.clamp(2., 120.);
            tank.rider_seconds = 0.;
            tank.free_next = tank.called;
            tank.called = false;
            crate::audio::emit(
                &mut commands,
                "zmb_tank_stop",
                crate::audio::SoundSource::attached(entity),
            );
            engine(&mut commands, &mut tank, "zmb_tank_idle", entity);
        }
    }
}
fn rescue(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut players: Query<(Entity, &mut Transform, &mut Player, &mut TankRescue), Without<TankBody>>,
) {
    for (entity, mut pose, mut player, mut rescue) in &mut players {
        rescue.elapsed += time.delta_secs();
        rescue.fade = if rescue.elapsed < 0.01 {
            rescue.elapsed / 0.01
        } else if rescue.elapsed < 4. {
            1.
        } else {
            (1. - (rescue.elapsed - 4.) / 0.2).clamp(0., 1.)
        };
        pose.translation = rescue.from.lerp(rescue.to, rescue.elapsed.min(1.));
        player.velocity = Vec3::ZERO;
        if rescue.elapsed >= 4.2 {
            if !rescue.disabled {
                commands.entity(entity).remove::<ColliderDisabled>();
            }
            commands
                .entity(entity)
                .remove::<(TankRescue, crate::actions::HandReservation)>();
        }
    }
}
fn tag_pose(pose: &Transform, index: usize) -> (Vec3, Vec3) {
    let (_, point, forward) = source::TAGS[index];
    (
        pose.transform_point(Vec3::from_array(point)),
        pose.rotation * Vec3::from_array(forward),
    )
}
fn valid_tag(tank: &Tank, index: usize, down: bool) -> bool {
    if tank.phase == TankPhase::Moving {
        index != 6 && (!down || !matches!(index,3..=5|7..=9))
    } else if tank.station == 0 {
        !matches!(index,3..=5|8..=9)
    } else {
        !matches!(index, 3..=5)
    }
}
fn board(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    tank: Res<Tank>,
    game: Res<GameDefinition>,
    body: Query<&Transform, With<TankBody>>,
    riders: Query<(&Vitality, &PlatformSupport), With<Player>>,
    mut zombies: Query<
        (
            Entity,
            &mut crate::enemies::Zombie,
            &mut Transform,
            &Vitality,
            &crate::combat_effects::Wounds,
            Option<&mut TankChase>,
            Option<&mut TankMotion>,
        ),
        (
            Without<TankBody>,
            Without<Player>,
            Without<crate::panzer::Panzer>,
        ),
    >,
) {
    let Ok(body) = body.single() else {
        return;
    };
    let dt = time.delta_secs();
    let riders = riders.iter().any(|(h, s)| {
        matches!(h.state(), LifeState::Standing | LifeState::Downed) && s.entity == tank.entity
    });
    let occupied: std::collections::BTreeSet<_> = zombies
        .iter()
        .filter_map(|(_, _, _, h, _, _, m)| {
            m.filter(|_| h.state() == LifeState::ActorAlive)
                .map(|m| m.tag)
        })
        .collect();
    let positions: Vec<_> = zombies
        .iter()
        .filter(|(_, _, _, h, _, _, _)| h.state() == LifeState::ActorAlive)
        .map(|(e, _, p, _, _, _, _)| (e, p.translation))
        .collect();
    let mut claimed = occupied;
    for (entity, mut zombie, mut pose, health, wounds, chase, motion) in &mut zombies {
        if health.state() != LifeState::ActorAlive {
            zombie.tank_animation = None;
            zombie.tank_chasing = false;
            commands
                .entity(entity)
                .remove::<(TankMotion, TankChase, crate::enemies::EnemyGoal)>();
            continue;
        }
        let Some(enemy) = game
            .content
            .enemies
            .iter()
            .find(|e| e.id == zombie.definition)
        else {
            continue;
        };
        if let Some(mut motion) = motion {
            let Some(profile) = enemy.traversals.get(&motion.alias) else {
                commands
                    .entity(entity)
                    .remove::<(TankMotion, ColliderDisabled)>();
                continue;
            };
            motion.elapsed += dt;
            zombie.elapsed = motion.elapsed;
            let (_, _, native_forward) = source::TAGS[motion.tag];
            let forward = Vec3::from_array(native_forward);
            let rotation = Quat::from_rotation_y((-forward.z).atan2(forward.x));
            let mut sample = profile.motion.last().copied().unwrap_or([0.; 4]);
            for pair in profile.motion.windows(2) {
                if motion.elapsed <= pair[1][0] {
                    let f =
                        ((motion.elapsed - pair[0][0]) / (pair[1][0] - pair[0][0])).clamp(0., 1.);
                    for i in 1..4 {
                        sample[i] = pair[0][i] + (pair[1][i] - pair[0][i]) * f;
                    }
                    break;
                }
            }
            pose.translation = body.transform_point(
                Vec3::from_array(source::TAGS[motion.tag].1)
                    + rotation * Vec3::new(sample[1], sample[2], sample[3])
                    + Vec3::Y * if wounds.crawler { 0.35 } else { 0.85 },
            );
            pose.rotation = body.rotation * rotation;
            if motion.elapsed >= profile.seconds {
                zombie.tank_animation = None;
                zombie.elapsed = 0.;
                zombie.action = crate::origins::EnemyAnimation::Idle;
                commands
                    .entity(entity)
                    .remove::<(TankMotion, ColliderDisabled)>();
                if motion.down {
                    commands
                        .entity(entity)
                        .remove::<(TankChase, crate::enemies::EnemyGoal)>();
                    zombie.tank_chasing = false;
                } else if let Some(mut chase) = chase {
                    chase.onboard = true;
                    chase.previous = *body;
                    commands
                        .entity(entity)
                        .remove::<crate::enemies::EnemyGoal>();
                }
            }
            continue;
        }
        let distance = pose.translation.distance(body.translation);
        if let Some(mut chase) = chase {
            if chase.onboard {
                let local = chase
                    .previous
                    .compute_affine()
                    .inverse()
                    .transform_point3(pose.translation);
                pose.translation = body.transform_point(local);
                chase.previous = *body;
                if riders {
                    continue;
                }
            } else if !riders || distance > 25.4 {
                zombie.tank_chasing = false;
                commands
                    .entity(entity)
                    .remove::<(TankChase, crate::enemies::EnemyGoal)>();
                continue;
            }
            chase.remaining -= dt;
            let down = chase.onboard;
            if chase.remaining <= 0. || !valid_tag(&tank, chase.tag, down) {
                if let Some(index) = (3..source::TAGS.len())
                    .filter(|i| valid_tag(&tank, *i, down) && !claimed.contains(i))
                    .min_by(|a, b| {
                        tag_pose(body, *a)
                            .0
                            .distance_squared(pose.translation)
                            .total_cmp(&tag_pose(body, *b).0.distance_squared(pose.translation))
                    })
                {
                    chase.tag = index;
                    chase.remaining = 6.;
                } else {
                    continue;
                }
            }
            let (point, _) = tag_pose(body, chase.tag);
            commands.entity(entity).insert(crate::enemies::EnemyGoal {
                position: point + Vec3::Y * 0.85,
                face: body.translation,
                radius: 0.,
            });
            zombie.tank_chasing = !wounds.crawler && tank.phase == TankPhase::Moving;
            let range = if tank.phase == TankPhase::Moving {
                1.6256
            } else {
                0.8128
            };
            let crowd = positions
                .iter()
                .filter(|(e, p)| {
                    *e != entity && p.xz().distance_squared(point.xz()) < 1.6256_f32.powi(2)
                })
                .count();
            if pose.translation.xz().distance_squared(point.xz()) < range * range
                && !claimed.contains(&chase.tag)
                && crowd < 4
                && zombie.completed_emerging()
            {
                let tag = source::TAGS[chase.tag].0;
                let tag = if !down
                    && tank.phase == TankPhase::Moving
                    && tag == "window_left_rear_jmp_jnt"
                {
                    "window_rear_long_jmp_jnt"
                } else {
                    tag
                };
                let alias = format!(
                    "tank_{}_{}{}",
                    if down { "down" } else { "up" },
                    tag,
                    if wounds.crawler { "_crawler" } else { "" }
                );
                if let Some(profile) = enemy.traversals.get(&alias) {
                    zombie.tank_animation =
                        profile.animation.0.rsplit('.').next().map(str::to_owned);
                    zombie.action = crate::origins::EnemyAnimation::Traverse;
                    zombie.elapsed = 0.;
                    claimed.insert(chase.tag);
                    commands
                        .entity(entity)
                        .remove::<crate::enemies::EnemyGoal>()
                        .insert((
                            TankMotion {
                                tag: chase.tag,
                                alias,
                                elapsed: 0.,
                                down,
                            },
                            ColliderDisabled,
                        ));
                }
            }
        } else if riders && distance < 12.7 && zombie.completed_emerging() {
            if let Some(tag) = (3..source::TAGS.len())
                .filter(|i| valid_tag(&tank, *i, false) && !claimed.contains(i))
                .min_by(|a, b| {
                    tag_pose(body, *a)
                        .0
                        .distance_squared(pose.translation)
                        .total_cmp(&tag_pose(body, *b).0.distance_squared(pose.translation))
                })
            {
                commands.entity(entity).insert(TankChase {
                    tag,
                    remaining: 6.,
                    onboard: false,
                    previous: *body,
                });
            }
        }
    }
}
fn flames(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut tank: ResMut<Tank>,
    mut session: ResMut<Session>,
    body: Query<(Entity, &Transform), With<TankBody>>,
    mut zombies: Query<
        (
            Entity,
            &Transform,
            &Vitality,
            &mut crate::combat_effects::Wounds,
            Option<&TankMotion>,
            Option<&mut TankBurn>,
        ),
        (Without<TankBody>, With<crate::enemies::Zombie>),
    >,
    mut damage: ResMut<crate::damage::DamageQueue>,
) {
    let Ok((owner, pose)) = body.single() else {
        return;
    };
    let dt = time.delta_secs();
    let moving = tank.phase == TankPhase::Moving;
    for (target, _, health, _, _, burn) in &mut zombies {
        if let Some(mut burn) = burn {
            burn.remaining -= dt;
            if burn.remaining <= 0. {
                if health.state() == LifeState::ActorAlive {
                    damage.0.push(crate::damage::Impact {
                        attacker: owner,
                        target,
                        source: crate::damage::ImpactSource::Vehicle("vehicle.tank".into()),
                        category: crate::origins::DamageCategory::Fire,
                        region: crate::weapons::HitRegion::Torso,
                        amount: burn.amount,
                    });
                }
                commands.entity(target).remove::<TankBurn>();
            }
        }
    }
    for index in 0..3 {
        let cycle = &mut tank.flames[index];
        cycle.check -= dt;
        if cycle.active > 0. {
            cycle.active -= dt;
            if !moving || cycle.active <= 0. {
                if let Some(e) = cycle.fx.take() {
                    commands.entity(e).despawn();
                }
                if let Some(e) = cycle.sound.take() {
                    commands.entity(e).despawn();
                }
                crate::audio::emit(
                    &mut commands,
                    "zmb_tank_flame_stop",
                    crate::audio::SoundSource::attached(owner),
                );
                cycle.active = 0.;
                cycle.empty = 0.;
                cycle.interval = 3. + (session.random() % 3001) as f32 / 1000.;
                cycle.check = 1.;
                continue;
            }
        } else if !moving || cycle.check > 0. {
            continue;
        }
        let (tag, forward) = tag_pose(pose, index);
        let kill = tag + forward * 2.032;
        let targets: Vec<_> = zombies
            .iter()
            .filter(|(_, p, h, _, m, _)| {
                h.state() == LifeState::ActorAlive
                    && m.is_none()
                    && p.translation.xz().distance_squared(kill.xz()) <= 2.032_f32.powi(2)
                    && forward.dot(p.translation) >= 0.95
            })
            .map(|(e, _, _, _, _, _)| e)
            .collect();
        if cycle.active <= 0. {
            cycle.check = 1.;
            if !targets.is_empty() || cycle.empty > cycle.interval {
                cycle.active = if targets.is_empty() { 3. } else { 6. };
                let offset = Transform::from_translation(Vec3::from_array(source::TAGS[index].1))
                    .with_rotation(Quat::from_rotation_arc(
                        Vec3::X,
                        Vec3::from_array(source::TAGS[index].2).normalize(),
                    ));
                cycle.fx = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_mech_wpn_flamethrower",
                    owner,
                    offset,
                ));
                crate::audio::emit(
                    &mut commands,
                    "zmb_tank_flame_start",
                    crate::audio::SoundSource::attached(owner),
                );
                cycle.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_tank_flame_loop",
                    crate::audio::SoundSource::attached(owner),
                ));
            } else {
                cycle.empty += 1.;
                continue;
            }
        }
        for target in targets {
            if let Ok((_, _, health, mut wounds, _, burn)) = zombies.get_mut(target) {
                if index == 0 {
                    damage.0.push(crate::damage::Impact {
                        attacker: owner,
                        target,
                        source: crate::damage::ImpactSource::Vehicle("vehicle.tank".into()),
                        category: crate::origins::DamageCategory::Fire,
                        region: crate::weapons::HitRegion::Torso,
                        amount: health.current().unwrap_or(0),
                    });
                    wounds.burned = true;
                    wounds.burning = 8.5;
                } else if burn.is_none() {
                    commands.entity(target).insert(TankBurn {
                        remaining: 0.5,
                        amount: health.current().unwrap_or(0),
                    });
                    wounds.burned = true;
                    wounds.burning = 8.5;
                }
            }
        }
    }
}
fn drops(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut tank: ResMut<Tank>,
    mut session: ResMut<Session>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    body: Query<&Transform, With<TankBody>>,
    riders: Query<(&Vitality, &PlatformSupport), With<Player>>,
    parts: Res<crate::environment::parts::Parts>,
) {
    tank.drop_check -= time.delta_secs();
    if tank.drop_check > 0. {
        return;
    }
    tank.drop_check = 2.;
    let Ok(pose) = body.single() else {
        return;
    };
    if !riders.iter().any(|(h, s)| {
        matches!(h.state(), LifeState::Standing | LifeState::Downed) && s.entity == tank.entity
    }) {
        return;
    }
    if !["upper_staff", "middle_staff", "lower_staff"]
        .iter()
        .all(|piece| {
            parts
                .pieces
                .get(&format!("part.elemental_staff_lightning.{piece}"))
                .is_some_and(|s| {
                    matches!(
                        s,
                        crate::environment::parts::PartState::Owned { .. }
                            | crate::environment::parts::PartState::Consumed
                    )
                })
        })
    {
        return;
    }
    for (index, (node, point)) in [
        (28, [-10.7696, 7.112, 68.072]),
        (8, [38.2016, -3.8608, -82.4992]),
        (22, [-14.6304, 6.7056, -8.3312]),
    ]
    .into_iter()
    .enumerate()
    {
        if session.round >= tank.drops[index]
            && pose
                .translation
                .xz()
                .distance_squared(Vec3::from_array(source::ROUTE[node].0).xz())
                < 12.7_f32.powi(2)
        {
            let name = [
                "nuke",
                "full_ammo",
                "zombie_blood",
                "insta_kill",
                "fire_sale",
                "double_points",
            ][(session.random() % 6) as usize];
            crate::powerups::spawn_named(
                &mut commands,
                &game,
                &server,
                name,
                Vec3::from_array(point),
            );
            tank.drops[index] = session.round + 8 + session.random() % 4;
        }
    }
}
pub(crate) fn interact(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    spatial: SpatialQuery,
    mut tank: ResMut<Tank>,
    bodies: Query<(Entity, &Transform), With<TankBody>>,
    calls: Query<(Entity, &CallBox, &Transform)>,
    players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &crate::weapons::WeaponInventory,
        &crate::equipment::GrenadeInventory,
        &crate::equipment::TacticalInventory,
        &crate::mines::MineInventory,
        Has<crate::perks::PerkDrinking>,
    )>,
    mut focus: ResMut<InteractionFocus>,
    mut ledger: ResMut<PointLedger>,
    mut session: ResMut<Session>,
) {
    if !input.use_held {
        tank.latched = false;
    }
    if focus.target.is_some() {
        return;
    }
    let Ok((owner, player, pose, health, inventory, grenades, tactical, mines, drinking)) =
        players.single()
    else {
        return;
    };
    if health.state() != LifeState::Standing {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward =
        Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch) * Vec3::NEG_Z;
    let Ok((body, body_pose)) = bodies.single() else {
        return;
    };
    let options = std::iter::once((body, body_pose.translation + Vec3::Y, false)).chain(
        calls
            .iter()
            .filter(|(_, call, _)| tank.activated_once && call.0 != tank.station)
            .map(|(entity, _, pose)| (entity, pose.translation, true)),
    );
    for (entity, point, called) in options {
        let delta = point - eye;
        let Ok(direction) = Dir3::new(delta) else {
            continue;
        };
        if delta.length() > 3.
            || forward.dot(*direction) < 0.8
            || spatial
                .cast_ray(
                    eye,
                    direction,
                    (delta.length() - 0.15).max(0.),
                    true,
                    &crate::environment::solids([owner, body]),
                )
                .is_some()
        {
            continue;
        }
        focus.target = Some(entity);
        let price = if tank.free_next && !called { 0 } else { 500 };
        focus.prompt = match tank.phase {
            TankPhase::Ready => format!(
                "Press {} to {} tank [Cost: {price}]",
                settings.label(crate::input::Action::Interact),
                if called { "call" } else { "activate" }
            ),
            TankPhase::Cooling => format!("Tank cooling down [{:.0}s]", tank.timer),
            _ => "Tank route in progress".into(),
        };
        if !input.use_held || tank.latched {
            return;
        }
        tank.latched = true;
        if tank.phase != TankPhase::Ready
            || inventory.action != crate::weapons::WeaponAction::Ready
            || grenades.busy()
            || tactical.busy()
            || mines.busy()
            || drinking
        {
            focus.rejection = Some(InteractionRejection::Busy);
            return;
        }
        let purchase = ledger.command(price);
        if let Err(error) = ledger.accept(&mut session.points, purchase, || {
            Ok::<_, InteractionRejection>(())
        }) {
            focus.rejection = Some(error.into());
            return;
        }
        tank.phase = TankPhase::Moving;
        tank.speed = 3.57632;
        tank.engine_delay = 0.4;
        tank.activated_once = true;
        tank.called = called;
        tank.free_next = false;
        focus.accepted += 1;
        crate::audio::emit(
            &mut commands,
            "zmb_tank_start",
            crate::audio::SoundSource::attached(body),
        );
        return;
    }
}
