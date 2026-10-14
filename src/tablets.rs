use crate::{
    GameState,
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    interactions::InteractionFocus,
    life::{LifeState, Vitality},
    player::Player,
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Component, Default)]
pub struct Tablet {
    pub stage: u8,
    pub souls: u32,
    pub reclean: bool,
    reward_delay: f32,
}
#[derive(Clone, Copy)]
enum Target {
    Bunker,
    Church,
}
#[derive(Component)]
struct TabletModel {
    index: usize,
    church: bool,
}
#[derive(Component, Default)]
struct Barrage {
    elapsed: f32,
    waiting: f32,
    shots: usize,
    launched: usize,
    launch: f32,
    authorized: bool,
    next_robot: usize,
    next_service: f32,
    started: bool,
    landings: Vec<Vec3>,
    glow: Option<Entity>,
    beep: f32,
}
#[derive(Component)]
struct RejectedBeacon {
    remaining: f32,
}
#[derive(Component)]
struct ShellFlight {
    elapsed: f32,
    from: Vec3,
    to: Vec3,
    incoming: bool,
}
#[derive(Component)]
pub(crate) struct BeaconAccepted;
#[derive(Resource, Default)]
pub struct TabletProgress {
    pub beacon_unlocked: bool,
    service: Option<Entity>,
    robots: [Option<Entity>; 3],
}
#[derive(Message, Clone, Copy)]
pub struct BeaconStrike {
    pub owner: Entity,
    pub point: Vec3,
}
#[derive(Resource, Default)]
pub(crate) struct TabletInput {
    held: bool,
}
pub struct TabletPlugin;
impl Plugin for TabletPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Player, Tablet>()
            .init_resource::<TabletInput>()
            .init_resource::<TabletProgress>()
            .add_message::<BeaconStrike>()
            .add_systems(
                FixedUpdate,
                beacon_presentation
                    .in_set(GameStep::Movement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                track
                    .in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                interact
                    .in_set(GameStep::Interactions)
                    .after(crate::staff_upgrades::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                bombard
                    .in_set(GameStep::Combat)
                    .after(crate::equipment::flight)
                    .before(crate::equipment::resolve_blasts)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut game: ResMut<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    mut input: ResMut<TabletInput>,
    mut progress: ResMut<TabletProgress>,
) {
    *input = default();
    *progress = default();
    if let Some(beacon) = game
        .content
        .weapons
        .iter_mut()
        .find(|w| w.id.0 == "weapon.beacon_zm")
    {
        beacon.throwable = Some(crate::equipment::ThrowableDefinition {
            slot: crate::equipment::ThrowableSlot::Tactical,
            initial: 2,
            capacity: 2,
            round_award: 0,
            prime_seconds: 1.55,
            throw_seconds: 0.7,
            release_delay: 0.,
            fuse_seconds: 10.,
            speed: 20.32,
            up_speed: 2.54,
            radius: 2.54,
            inner_radius: 0.,
            inner_damage: 5000,
            outer_damage: 2000,
            tangent_bounce: [0.5; 32],
            normal_bounce: [0.25; 32],
            collision_radius: 0.08,
            collision_half_extents: None,
            native_launch: None,
            cook_in_hand: false,
            sticky: false,
        });
    }
    let scene = game
        .content
        .presentations
        .iter()
        .filter_map(|p| p.scene.as_ref())
        .find(|p| p.contains("p6_zm_tm_tablet_muddy_lod0"));
    for (index, x) in [-3.6957, -3.4417, -4.0005, -3.1369].into_iter().enumerate() {
        let mut entity = commands.spawn((
            SessionEntity,
            TabletModel {
                index,
                church: false,
            },
            Transform::from_translation(Vec3::new(x, -8.1915, -113.3856)),
            Visibility::Inherited,
        ));
        if let Some(scene) = scene {
            entity.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
    let mut entity = commands.spawn((
        SessionEntity,
        TabletModel {
            index: 3,
            church: true,
        },
        Transform::from_translation(Vec3::new(23.3045, 8.8773, 67.3481)),
        Visibility::Hidden,
    ));
    if let Some(scene) = scene {
        entity.insert(WorldAssetRoot(server.load(scene.clone())));
    }
}
fn track(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    game: Res<crate::assets::GameDefinition>,
    spatial: SpatialQuery,
    volumes: Query<(
        &crate::environment::triggers::MapTrigger,
        &crate::environment::triggers::TriggerState,
    )>,
    enemies: Query<&Transform, With<crate::enemies::Zombie>>,
    mut players: Query<(&mut Tablet, &Vitality, &crate::environment::TerrainMotion), With<Player>>,
    mut models: Query<(&TabletModel, &mut Visibility)>,
) {
    let Ok((mut tablet, health, motion)) = players.single_mut() else {
        return;
    };
    if health.state() == LifeState::HumanTerminal && tablet.stage < 6 {
        *tablet = default();
    }
    if tablet.stage == 4 && motion.muddy {
        tablet.stage = 1;
        tablet.reclean = true;
        crate::audio::emit(
            &mut commands,
            "zmb_squest_oiptablet_dirtied",
            crate::audio::SoundSource::World(Vec3::ZERO),
        );
    }
    tablet.reward_delay = (tablet.reward_delay - time.delta_secs()).max(0.);
    for death in deaths.read() {
        if !matches!(tablet.stage, 2 | 5)
            || !death.0.can_supply_soul()
            || !matches!(
                death.0.impact.category,
                crate::origins::DamageCategory::Melee
            )
        {
            continue;
        }
        let Ok(pose) = enemies.get(death.0.impact.target) else {
            continue;
        };
        let name = if tablet.stage == 2 {
            "oneinchpunch_church_volume"
        } else {
            "oneinchpunch_bunker_volume"
        };
        let inside = spatial
            .shape_intersections(
                &Collider::sphere(0.3),
                pose.translation,
                Quat::IDENTITY,
                &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER),
            )
            .iter()
            .filter_map(|e| volumes.get(*e).ok())
            .filter(|(_, state)| state.enabled)
            .map(|(volume, _)| volume)
            .any(|v| {
                game.environment
                    .triggers
                    .iter()
                    .find(|t| t.id == v.0)
                    .is_some_and(|t| t.source.get("targetname").is_some_and(|n| n == name))
            });
        if inside {
            crate::quests::soul_effect(
                &mut commands,
                pose.translation,
                if tablet.stage == 2 {
                    Vec3::new(23.3045, 8.8773, 67.3481)
                } else {
                    Vec3::new(-3.1369, -8.1915, -113.3856)
                },
                0.,
                1.,
            );
            tablet.souls += 1;
            if tablet.souls >= 20 {
                tablet.stage += 1;
                tablet.souls = 0;
                crate::audio::emit(
                    &mut commands,
                    "zmb_squest_oiptablet_charged",
                    crate::audio::SoundSource::World(pose.translation),
                );
                if tablet.stage == 6 {
                    tablet.reward_delay = 4.;
                }
            }
        }
    }
    for (model, mut visibility) in &mut models {
        *visibility = if model.church && matches!(tablet.stage, 2 | 3)
            || !model.church && (model.index != 3 || matches!(tablet.stage, 0 | 5 | 6))
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
pub(crate) fn interact(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut latch: ResMut<TabletInput>,
    mut focus: ResMut<InteractionFocus>,
    game: Res<crate::assets::GameDefinition>,
    spatial: SpatialQuery,
    mut progress: ResMut<TabletProgress>,
    source_triggers: Query<(
        Entity,
        &crate::environment::triggers::MapTrigger,
        &Collider,
        &Transform,
        &crate::environment::triggers::TriggerState,
    )>,
    client_modes: Query<&crate::access::TriggerClientMode>,
    mut trigger_queue: ResMut<crate::environment::triggers::TriggerQueue>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &WeaponInventory,
        &mut Tablet,
        &mut crate::equipment::TacticalInventory,
        Has<crate::perks::PerkDrinking>,
        Option<&crate::actions::HandReservation>,
    )>,
) {
    if !input.use_held {
        latch.held = false;
    }
    let Ok((
        owner,
        player,
        pose,
        health,
        inventory,
        mut tablet,
        mut tactical,
        drinking,
        reservation,
    )) = players.single_mut()
    else {
        return;
    };
    if focus.target.is_some() || health.state() != LifeState::Standing {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let selected = source_triggers
        .iter()
        .filter_map(|(entity, trigger, shape, pose, state)| {
            let source = game
                .environment
                .triggers
                .iter()
                .find(|source| source.id == trigger.0)?;
            let target = match source.source.get("targetname")?.as_str() {
                "trigger_oneinchpunch_bunker_table" => Target::Bunker,
                "trigger_oneinchpunch_church_birdbath" => Target::Church,
                _ => return None,
            };
            let score = crate::environment::support::use_candidate(
                entity,
                shape,
                pose,
                state,
                &spatial,
                owner,
                client_modes.get(owner).ok()?.0,
                eye,
                forward,
                false,
                game.tuning.use_radius,
            )?;
            Some((entity, target, score))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((entity, target, _)) = selected else {
        return;
    };
    focus.target = Some(entity);
    focus.hold_seconds = 0.;
    focus.progress = 0.;
    let action = match (target, tablet.stage) {
        (Target::Bunker, 0) => "take muddy tablet",
        (Target::Church, 1) => "place tablet in basin",
        (Target::Church, 3) => "take cleansed tablet",
        (Target::Bunker, 4) => "return cleansed tablet",
        (Target::Bunker, 6) => "claim G-Strike",
        _ => "tablet ritual in progress",
    };
    focus.prompt = format!(
        "Press {} to {action}",
        settings.label(crate::input::Action::Interact)
    );
    if !input.use_held
        || latch.held
        || drinking
        || reservation.is_some()
        || inventory.action != WeaponAction::Ready
    {
        return;
    }
    let previous = tablet.stage;
    let reclean = tablet.reclean;
    match (target, tablet.stage) {
        (Target::Bunker, 0) => tablet.stage = 1,
        (Target::Church, 1) => {
            tablet.stage = if tablet.reclean { 4 } else { 2 };
            tablet.reclean = false;
        }
        (Target::Church, 3) => tablet.stage = 4,
        (Target::Bunker, 4) => tablet.stage = 5,
        (Target::Bunker, 6) if tablet.reward_delay <= 0. => {
            let Some(weapon) = game
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.beacon_zm")
            else {
                return;
            };
            if tactical.receive(weapon).is_err() {
                return;
            }
            tablet.stage = 7;
            progress.beacon_unlocked = true;
        }
        _ => return,
    }
    let alias = match previous {
        0 => "zmb_squest_oiptablet_pickup",
        1 => "zmb_squest_oiptablet_bathe",
        3 => "zmb_squest_oiptablet_pickup_clean",
        4 => "zmb_squest_oiptablet_place_table",
        6 => "zmb_squest_oiptablet_get_reward",
        _ => return,
    };
    latch.held = true;
    trigger_queue.push(entity, owner);
    focus.accepted += 1;
    crate::audio::emit(
        &mut commands,
        alias,
        crate::audio::SoundSource::attached(owner),
    );
    if previous == 1 && reclean {
        crate::audio::emit(
            &mut commands,
            "zmb_squest_oiptablet_pickup_clean",
            crate::audio::SoundSource::attached(owner),
        );
    }
}
fn bombard(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    robots: Res<crate::robots::Robots>,
    quest: Res<crate::quests::MainQuest>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    access: Res<crate::access::ZoneAccess>,
    spatial: SpatialQuery,
    volumes: Query<(
        &crate::environment::triggers::MapTrigger,
        Option<&crate::environment::hazards::BoundaryVolume>,
        &crate::environment::triggers::TriggerState,
    )>,
    mut progress: ResMut<TabletProgress>,
    mut grenades: Query<
        (
            Entity,
            &mut crate::equipment::Grenade,
            &Transform,
            Option<&mut Barrage>,
        ),
        Without<RejectedBeacon>,
    >,
    enemies: Query<(Entity, &Transform, &Vitality), With<crate::enemies::Zombie>>,
    mut damage: ResMut<crate::damage::DamageQueue>,
    mut strikes: MessageWriter<BeaconStrike>,
) {
    if progress.service.is_some_and(|e| !grenades.contains(e)) {
        progress.service = None;
    }
    for reservation in &mut progress.robots {
        if reservation.is_some_and(|e| !grenades.contains(e)) {
            *reservation = None;
        }
    }
    let dt = time.delta_secs();
    for (entity, mut grenade, pose, barrage) in &mut grenades {
        if grenade.definition.0 != "weapon.beacon_zm" || !grenade.settled() {
            continue;
        }
        let Some(mut barrage) = barrage else {
            let touching = spatial.shape_intersections(
                &Collider::sphere(0.1),
                pose.translation,
                Quat::IDENTITY,
                &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER),
            );
            let valid = touching
                .iter()
                .filter_map(|e| volumes.get(*e).ok())
                .filter(|(_, _, state)| state.enabled)
                .any(|(volume, boundary, _)| match boundary {
                    Some(crate::environment::hazards::BoundaryVolume::Playable(zone)) => {
                        access.reachable.contains(zone)
                    }
                    _ => {
                        quest.enabled
                            && quest.stage == crate::quests::QuestStage::BreakSeal
                            && game
                                .environment
                                .triggers
                                .iter()
                                .find(|t| t.id == volume.0)
                                .is_some_and(|t| {
                                    t.source
                                        .get("targetname")
                                        .is_some_and(|n| n == "fire_link_damage")
                                })
                    }
                });
            if !valid {
                grenade.lure_active = false;
                grenade.remaining = 100.;
                commands
                    .entity(entity)
                    .insert(RejectedBeacon { remaining: 1. });
                crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie/fx_zmb_blackhole_trap_end",
                    entity,
                    Transform::IDENTITY,
                );
                crate::audio::emit(
                    &mut commands,
                    "zmb_laugh_richtofen",
                    crate::audio::SoundSource::World(pose.translation),
                );
                continue;
            }
            grenade.remaining = 100.;
            let glow = crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_beacon_glow",
                entity,
                Transform::IDENTITY,
            );
            commands.entity(entity).insert((
                Barrage {
                    authorized: quest.strike_authorized(),
                    glow: Some(glow),
                    ..default()
                },
                BeaconAccepted,
            ));
            continue;
        };
        grenade.remaining = 100.;
        barrage.beep -= dt;
        if barrage.beep <= 0. {
            barrage.beep += 1.;
            crate::audio::emit(
                &mut commands,
                "evt_beacon_beep",
                crate::audio::SoundSource::attached(entity),
            );
        }
        if !barrage.started {
            barrage.waiting += dt;
            if barrage.waiting >= 10. {
                grenade.lure_active = false;
                commands.entity(entity).despawn();
                continue;
            }
            if progress.service.is_some_and(|e| e != entity) {
                continue;
            }
            let selected = if barrage.authorized && robots.triple_active {
                [1, 0, 2].get(barrage.next_robot).copied().filter(|i| {
                    robots.passes[*i].is_some_and(|p| p.delay <= 0.)
                        && progress.robots[*i].is_none()
                })
            } else {
                barrage.authorized = false;
                (0..3).find(|i| {
                    robots.passes[*i].is_some_and(|p| p.delay <= 0.)
                        && progress.robots[*i].is_none()
                })
            };
            let Some(robot) = selected else {
                continue;
            };
            progress.service = Some(entity);
            progress.robots[robot] = Some(entity);
            barrage.started = true;
            barrage.elapsed = 0.;
            barrage.next_robot = 1;
            barrage.next_service = 2.1;
            barrage.launch = if barrage.authorized {
                f32::INFINITY
            } else {
                3.5
            };
            crate::audio::emit(
                &mut commands,
                "zmb_homingbeacon_missiile_alarm",
                crate::audio::SoundSource::World(pose.translation),
            );
        }
        barrage.elapsed += dt;
        if barrage.authorized
            && !barrage.launch.is_finite()
            && barrage.elapsed >= barrage.next_service
        {
            let next = [1, 0, 2].get(barrage.next_robot).copied();
            if let Some(robot) = next.filter(|i| {
                robots.passes[*i].is_some_and(|p| p.delay <= 0.) && progress.robots[*i].is_none()
            }) {
                progress.robots[robot] = Some(entity);
                barrage.next_robot += 1;
                barrage.next_service += 2.1;
                crate::audio::emit(
                    &mut commands,
                    "zmb_homingbeacon_missiile_alarm",
                    crate::audio::SoundSource::World(pose.translation),
                );
            } else {
                barrage.launch = barrage.elapsed;
            }
        }
        let count = if barrage.authorized { 15 } else { 5 };
        if barrage.launch.is_finite()
            && barrage.elapsed >= barrage.launch
            && barrage.landings.is_empty()
        {
            let offsets = [
                Vec3::ZERO,
                Vec3::new(-1.8288, 0., -1.8288),
                Vec3::new(1.8288, 0., -1.8288),
                Vec3::new(1.8288, 0., 1.8288),
                Vec3::new(-1.8288, 0., 1.8288),
            ];
            for index in 0..count {
                let offset = if index == 0 {
                    Vec3::ZERO
                } else {
                    offsets[(index - 1) % 4 + 1]
                };
                let destination = pose.translation + offset;
                let start = pose.translation + shell_offset(index) + Vec3::Y * 88.9;
                let delta = destination - start;
                let landing = Dir3::new(delta)
                    .ok()
                    .and_then(|direction| {
                        spatial.cast_ray(
                            start,
                            direction,
                            delta.length(),
                            true,
                            &crate::environment::solids([grenade.owner]),
                        )
                    })
                    .map_or(destination, |h| {
                        start + delta.normalize_or_zero() * h.distance
                    });
                barrage.landings.push(landing);
            }
        }
        let first = barrage.launch + count as f32 * 0.05 + 3.;
        let launch_due = first - 3.
            + if barrage.launched == 0 {
                0.
            } else {
                1. + (barrage.launched - 1) as f32 * 0.25
            };
        if barrage.launched < count && barrage.elapsed >= launch_due {
            let index = barrage.launched;
            let to = barrage.landings[index];
            let from = pose.translation + shell_offset(index) + Vec3::Y * 215.9;
            let shell = commands
                .spawn((
                    SessionEntity,
                    Transform::from_translation(from)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    ShellFlight {
                        elapsed: 0.,
                        from,
                        to,
                        incoming: false,
                    },
                ))
                .id();
            if let Some(scene) = crate::assets::native_scene(&game, "jun_missile") {
                commands
                    .entity(shell)
                    .insert(WorldAssetRoot(server.load(scene.clone())));
            }
            crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_beacon_trail",
                shell,
                Transform::IDENTITY,
            );
            crate::audio::emit(
                &mut commands,
                "zmb_homingbeacon_missile_boom",
                crate::audio::SoundSource::attached(shell),
            );
            barrage.launched += 1;
        }
        let due = first
            + if barrage.shots == 0 {
                0.
            } else {
                1. + (barrage.shots - 1) as f32 * 0.25
            };
        if barrage.shots < count && barrage.elapsed >= due {
            let center = barrage.landings[barrage.shots];
            if barrage.shots == 0 {
                strikes.write(BeaconStrike {
                    owner: grenade.owner,
                    point: pose.translation,
                });
                grenade.lure_active = false;
                if let Some(glow) = barrage.glow.take() {
                    commands.entity(glow).try_despawn();
                }
            }
            barrage.shots += 1;
            for (enemy, p, health) in &enemies {
                let distance = p.translation.distance(center);
                if health.state() == LifeState::ActorAlive && distance <= 5.08 {
                    damage.0.push(crate::damage::Impact {
                        point: None,
                        attacker: grenade.owner,
                        target: enemy,
                        source: crate::damage::ImpactSource::Weapon(grenade.definition.clone()),
                        category: crate::origins::DamageCategory::Explosive,
                        region: crate::weapons::HitRegion::None,
                        amount: (8000. - distance / 5.08 * 1000.) as u32,
                    });
                }
            }
        }
        let launch_end = barrage.launch + count as f32 * 0.05 + 1. + (count - 1) as f32 * 0.25;
        if barrage.elapsed >= launch_end + 3. && progress.service == Some(entity) {
            progress.service = None;
            for reserved in &mut progress.robots {
                if *reserved == Some(entity) {
                    *reserved = None;
                }
            }
        }
        if barrage.elapsed >= launch_end + 6. {
            commands.entity(entity).despawn();
        }
    }
}
fn shell_offset(index: usize) -> Vec3 {
    if index == 0 {
        return Vec3::ZERO;
    }
    [
        Vec3::new(-165.1, 0., -165.1),
        Vec3::new(165.1, 0., -165.1),
        Vec3::new(165.1, 0., 165.1),
        Vec3::new(-165.1, 0., 165.1),
    ][(index - 1) % 4]
}
fn beacon_presentation(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut rejected: Query<(Entity, &mut Transform, &mut RejectedBeacon), Without<ShellFlight>>,
    mut shells: Query<(Entity, &mut Transform, &mut ShellFlight), Without<RejectedBeacon>>,
) {
    let dt = time.delta_secs();
    for (entity, mut pose, mut reject) in &mut rejected {
        reject.remaining -= dt;
        pose.translation.y += 1.524 * dt;
        if reject.remaining <= 0. {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut pose, mut shell) in &mut shells {
        shell.elapsed += dt;
        pose.translation = shell
            .from
            .lerp(shell.to, (shell.elapsed / 3.).clamp(0., 1.));
        if shell.elapsed >= 2. && !shell.incoming {
            shell.incoming = true;
            crate::audio::emit(
                &mut commands,
                "zmb_homingbeacon_missile_incoming",
                crate::audio::SoundSource::World(shell.to),
            );
        }
        if shell.elapsed >= 3. {
            crate::native_fx::emit(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_beacon_exp",
                Transform::from_translation(shell.to),
            );
            crate::audio::emit(
                &mut commands,
                "zmb_homingbeacon_missile_impact",
                crate::audio::SoundSource::World(shell.to),
            );
            commands.entity(entity).despawn();
        }
    }
}
