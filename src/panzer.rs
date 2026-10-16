use crate::{
    GameState,
    assets::GameDefinition,
    combat_effects::Armor,
    damage::{DamageApplied, DamageQueue, EnemyDeath, Impact, ImpactSource},
    enemies::Zombie,
    gameplay::{GameStep, Session, SessionEntity},
    life::{LifeState, Vitality},
    origins::content::{ContentId, DamageCategory, WeaponId},
    player::Player,
    rounds::{Gait, PopulationKind},
    weapons::HitRegion,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
mod source;

#[derive(Component, Default)]
pub struct CapturedByPanzer(pub Option<Entity>);
#[derive(Clone, Copy, Debug)]
pub enum PanzerPhase {
    Landing,
    Exit {
        origin: Vec3,
        yaw: f32,
        landing: Vec3,
    },
    Hover {
        landing: Vec3,
    },
    Arrive {
        landing: Vec3,
    },
    Pursuit,
    LiftOff,
    Sprint,
    TouchDown,
    Melee,
    FlameWindup,
    Flame,
    FlameSweep,
    ClawWindup,
    ShieldBreak(f32),
    Claw {
        tip: Vec3,
        direction: Vec3,
        remaining: f32,
    },
    Pulling(f32),
    HeldBurn,
    Stunned(f32),
    Pain(PainKind),
    Knockdown {
        kind: KnockdownKind,
        stage: u8,
        remaining: f32,
    },
    Dead(f32),
}
#[derive(Component)]
#[require(PanzerFeedback)]
pub struct Panzer {
    pub phase: PanzerPhase,
    pub quest: bool,
    pub captured: Option<Entity>,
    flame_cooldown: f32,
    claw_cooldown: f32,
    flame_tick: f32,
    claw_baseline: u32,
    plate_rewarded: bool,
    core_rewarded: bool,
    cover_reacted: bool,
    vocal_timer: f32,
    drop_powerup: bool,
    death_attacker: Option<Entity>,
    pub animation_elapsed: f32,
    claw_intro_seconds: f32,
    exploded: bool,
    boosting: bool,
    firing: bool,
    aim: usize,
    melee: usize,
    anchor: Transform,
    tank_tag: Option<usize>,
    jump_origin: Option<usize>,
    jump_check: f32,
    failed_paths: u8,
    arrival_yaw: f32,
}
#[derive(Clone, Copy, Debug)]
pub enum KnockdownKind {
    Robot,
    Tank,
}
#[derive(Clone, Copy, Debug)]
pub enum PainKind {
    Head,
    Faceplate,
    Powercore,
}
#[derive(Component)]
pub(crate) struct KnockdownRequest(pub KnockdownKind);

#[derive(Resource)]
struct PanzerAnimations {
    clips: BTreeMap<String, usize>,
    underground: Vec<(Collider, Vec3)>,
    _handles: Vec<Handle<AnimationClip>>,
    spawns: Vec<source::Spawn>,
}
impl PanzerAnimations {
    fn clip<'a>(
        &self,
        game: &'a GameDefinition,
        name: &str,
    ) -> &'a crate::origins::AnimationDefinition {
        &game.content.animations[*self
            .clips
            .get(name)
            .unwrap_or_else(|| panic!("Required Panzer clip {name}"))]
    }
}
const FLAME_AIM: [&str; 9] = [
    "ai_zombie_mech_ft_aim_1",
    "ai_zombie_mech_ft_aim_2",
    "ai_zombie_mech_ft_aim_3",
    "ai_zombie_mech_ft_aim_4",
    "ai_zombie_mech_ft_aim_5",
    "ai_zombie_mech_ft_aim_6",
    "ai_zombie_mech_ft_aim_7",
    "ai_zombie_mech_ft_aim_8",
    "ai_zombie_mech_ft_aim_9",
];
const CLAW_AIM: [&str; 9] = [
    "ai_zombie_mech_grapple_aim_1",
    "ai_zombie_mech_grapple_aim_2",
    "ai_zombie_mech_grapple_aim_3",
    "ai_zombie_mech_grapple_aim_4",
    "ai_zombie_mech_grapple_aim_5",
    "ai_zombie_mech_grapple_aim_6",
    "ai_zombie_mech_grapple_aim_7",
    "ai_zombie_mech_grapple_aim_8",
    "ai_zombie_mech_grapple_aim_9",
];

fn aim_index(pose: Transform, target: Vec3, right_offset: f32) -> Option<usize> {
    let local = pose.rotation.inverse() * (target - pose.translation) - Vec3::X * right_offset;
    let yaw = local.x.atan2(-local.z).to_degrees();
    let pitch = local.y.atan2(local.xz().length()).to_degrees();
    if yaw.abs() > 45. || pitch.abs() > 60. {
        return None;
    }
    let column = if yaw.abs() < 22.5 {
        1
    } else if yaw > 0. {
        2
    } else {
        0
    };
    let row = if pitch.abs() < 30. {
        1
    } else if pitch > 0. {
        2
    } else {
        0
    };
    Some(row * 3 + column)
}

fn crossed_note(seconds: f32, start: f32, end: f32, duration: f32, looping: bool) -> bool {
    if looping {
        ((end - seconds) / duration).ceil().max(0.) > ((start - seconds) / duration).ceil().max(0.)
    } else {
        seconds >= start && seconds < end
    }
}
impl Panzer {
    fn can_interrupt(&self) -> bool {
        !matches!(
            self.phase,
            PanzerPhase::Landing
                | PanzerPhase::Exit { .. }
                | PanzerPhase::Hover { .. }
                | PanzerPhase::Arrive { .. }
                | PanzerPhase::Knockdown { .. }
                | PanzerPhase::Dead(_)
        )
    }
    pub fn animation(&self) -> (&'static str, bool) {
        // The claw flies on the muzzleflash note while the body finishes its intro.
        if self.claw_intro_seconds > 0.
            && matches!(
                self.phase,
                PanzerPhase::Claw { .. } | PanzerPhase::Pulling(_) | PanzerPhase::ShieldBreak(_)
            )
        {
            return ("ai_zombie_mech_grapple_intro_sprint_to_aim_5", false);
        }
        match self.phase {
            PanzerPhase::Landing => ("ai_zombie_mech_intro_jump_in", false),
            PanzerPhase::Exit { .. } => ("ai_zombie_mech_exit", false),
            PanzerPhase::Hover { .. } => ("ai_zombie_mech_exit_hover", true),
            PanzerPhase::Arrive { .. } => ("ai_zombie_mech_arrive", false),
            PanzerPhase::Pursuit => ("ai_zombie_mech_run", true),
            PanzerPhase::LiftOff => ("ai_zombie_mech_sprint_booster_liftoff", false),
            PanzerPhase::Sprint => ("ai_zombie_mech_sprint_booster_loop", true),
            PanzerPhase::TouchDown => ("ai_zombie_mech_sprint_booster_touchdown", false),
            PanzerPhase::Melee => (
                ["ai_zombie_mech_melee_a", "ai_zombie_mech_melee_b"][self.melee],
                false,
            ),
            PanzerPhase::FlameWindup => ("ai_zombie_mech_ft_intro_sprint_to_aim_5", false),
            PanzerPhase::Flame => (FLAME_AIM[self.aim], true),
            PanzerPhase::FlameSweep => ("ai_zombie_mech_ft_sweep_up", false),
            PanzerPhase::ClawWindup => ("ai_zombie_mech_grapple_intro_sprint_to_aim_5", false),
            PanzerPhase::Claw { .. } | PanzerPhase::Pulling(_) | PanzerPhase::ShieldBreak(_) => {
                (CLAW_AIM[self.aim], true)
            }
            PanzerPhase::HeldBurn => ("ai_zombie_mech_ft_burn_player", false),
            PanzerPhase::Stunned(_) => ("ai_zombie_mech_stunned", true),
            PanzerPhase::Pain(kind) => (
                match kind {
                    PainKind::Head => "ai_zombie_mech_head_pain",
                    PainKind::Faceplate => "ai_zombie_mech_faceplate_pain",
                    PainKind::Powercore => "ai_zombie_mech_powercore_pain",
                },
                false,
            ),
            PanzerPhase::Knockdown { kind, stage, .. } => (
                match kind {
                    KnockdownKind::Robot => [
                        "ai_zombie_mech_injury_hit_by_footstep",
                        "ai_zombie_mech_injury_down_by_footstep_loop",
                        "ai_zombie_mech_injury_recover_from_footstep",
                    ][stage as usize],
                    KnockdownKind::Tank => [
                        "ai_zombie_mech_injury_hit_by_tank",
                        "ai_zombie_mech_injury_down_by_tank_loop",
                        "ai_zombie_mech_injury_recover_from_tank",
                    ][stage as usize],
                },
                stage == 1,
            ),
            PanzerPhase::Dead(_) => ("ai_zombie_mech_death", false),
        }
    }
}
#[derive(Message)]
pub struct PanzerClawReleased {
    pub releaser: Entity,
    pub captive: Entity,
}
#[derive(Message)]
pub struct PanzerDestroyed {
    pub entity: Entity,
    pub position: Vec3,
    pub quest: bool,
    pub attacker: Entity,
}
#[derive(Component)]
pub struct PanzerNuke {
    remaining: f32,
    attacker: Entity,
}
pub fn queue_nuke(commands: &mut Commands, target: Entity, attacker: Entity, delay: f32) {
    commands.entity(target).insert(PanzerNuke {
        remaining: delay,
        attacker,
    });
}
#[derive(Component)]
struct FlameIgnited {
    remaining: f32,
    pulse: f32,
    sound: Entity,
}

#[derive(Component)]
struct FlameVolume {
    owner: Option<Entity>,
}
#[derive(Component)]
struct ClawTrigger(Entity);

fn bind_triggers(
    mut commands: Commands,
    game: Res<GameDefinition>,
    panzers: Query<(Entity, &Panzer)>,
    rigs: Query<&crate::enemies::ActorRig>,
    sockets: Query<&GlobalTransform>,
    mut volumes: Query<(
        Entity,
        &crate::environment::triggers::MapTrigger,
        &mut Transform,
        Option<&mut FlameVolume>,
        &mut crate::environment::triggers::TriggerState,
    )>,
    mut claws: Query<
        (
            Entity,
            &ClawTrigger,
            &mut Transform,
            &mut crate::environment::triggers::TriggerState,
        ),
        Without<crate::environment::triggers::MapTrigger>,
    >,
) {
    let mut occupied = BTreeSet::new();
    for (_, _, _, volume, _) in &volumes {
        if let Some(owner) = volume.and_then(|volume| volume.owner)
            && panzers
                .get(owner)
                .is_ok_and(|(_, panzer)| !matches!(panzer.phase, PanzerPhase::Dead(_)))
        {
            occupied.insert(owner);
        }
    }
    for (entity, trigger, mut pose, volume, mut state) in &mut volumes {
        let Some(source) = game.environment.triggers.iter().find(|source| {
            source.id == trigger.0
                && source
                    .source
                    .get("script_noteworthy")
                    .is_some_and(|name| name == "flamethrower_trigger")
        }) else {
            continue;
        };
        let mut owner = volume
            .as_ref()
            .and_then(|volume| volume.owner)
            .filter(|owner| occupied.contains(owner));
        if owner.is_none() {
            owner = panzers
                .iter()
                .find(|(owner, panzer)| {
                    !occupied.contains(owner)
                        && matches!(
                            panzer.phase,
                            PanzerPhase::FlameWindup
                                | PanzerPhase::Flame
                                | PanzerPhase::FlameSweep
                                | PanzerPhase::HeldBurn
                        )
                        && rigs
                            .iter()
                            .any(|rig| rig.socket(*owner, "tag_flamethrower_FX").is_some())
                })
                .map(|(owner, _)| owner);
            if let Some(owner) = owner {
                occupied.insert(owner);
            }
        }
        state.automatic = false;
        state.enabled = owner.is_some();
        if let Some(owner) = owner
            && let Some(socket) = rigs
                .iter()
                .find_map(|rig| rig.socket(owner, "tag_flamethrower_FX"))
            && let Ok(socket) = sockets.get(socket)
        {
            *pose = socket.compute_transform();
        } else {
            *pose = source.transform();
        }
        if let Some(mut volume) = volume {
            volume.owner = owner;
        } else {
            commands.entity(entity).insert(FlameVolume { owner });
        }
    }
    let mut claw_owners: BTreeSet<_> = claws.iter().map(|(_, claw, _, _)| claw.0).collect();
    for (entity, claw, mut pose, mut state) in &mut claws {
        let Ok((_, panzer)) = panzers.get(claw.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        if matches!(panzer.phase, PanzerPhase::Dead(_)) {
            commands.entity(entity).despawn();
            continue;
        }
        state.enabled = true;
        if let Some(socket) = rigs.iter().find_map(|rig| rig.claw_socket(claw.0))
            && let Ok(socket) = sockets.get(socket)
        {
            *pose = socket.compute_transform();
        }
    }
    for (owner, panzer) in &panzers {
        if claw_owners.contains(&owner) || matches!(panzer.phase, PanzerPhase::Dead(_)) {
            continue;
        }
        let Some(socket) = rigs.iter().find_map(|rig| rig.claw_socket(owner)) else {
            continue;
        };
        let Ok(socket) = sockets.get(socket) else {
            continue;
        };
        let definition = crate::environment::triggers::TriggerDefinition {
            id: format!("script.panzer.claw.{}", owner.to_bits()),
            position: socket.translation().to_array(),
            target_position: None,
            source: [
                ("classname".into(), "trigger_damage".into()),
                ("spawnflags".into(), "0".into()),
            ]
            .into(),
            shape: crate::environment::triggers::TriggerShape::Cylinder {
                radius: 3. * 0.0254,
                height: 15. * 0.0254,
            },
        };
        if let Some(entity) = crate::environment::triggers::spawn_owned(
            &mut commands,
            &definition,
            owner,
            crate::environment::support::EnvironmentOwner::Enemy,
        ) {
            commands
                .entity(entity)
                .remove::<crate::environment::triggers::MapTrigger>()
                .insert((ClawTrigger(owner), socket.compute_transform()));
            claw_owners.insert(owner);
        }
    }
}

fn claw_hits(
    mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    triggers: Query<(&ClawTrigger, &crate::environment::triggers::TriggerState)>,
    panzers: Query<&Panzer>,
    players: Query<&Vitality, With<Player>>,
    mut damage: ResMut<DamageQueue>,
) {
    for hit in hits.read() {
        let Ok((claw, state)) = triggers.get(hit.target) else {
            continue;
        };
        if !state.enabled
            || !players
                .get(hit.attacker)
                .is_ok_and(|health| health.state() == LifeState::Standing)
            || !panzers.get(claw.0).is_ok_and(|panzer| {
                matches!(
                    panzer.phase,
                    PanzerPhase::Claw { .. } | PanzerPhase::Pulling(_)
                )
            })
        {
            continue;
        }
        damage.0.push(Impact {
            point: None,
            attacker: hit.attacker,
            target: claw.0,
            source: ImpactSource::Weapon(hit.weapon.clone()),
            category: hit.category,
            region: HitRegion::LeftArm,
            amount: 1,
        });
    }
}

#[cfg(test)]
mod trigger_tests {
    use super::*;

    #[test]
    fn environment_trigger_query_access() {
        let mut world = World::new();
        IntoSystem::into_system(bind_triggers).initialize(&mut world);
        IntoSystem::into_system(claw_hits).initialize(&mut world);
        IntoSystem::into_system(behavior).initialize(&mut world);
    }

    #[test]
    fn notes_cross_once_and_repeat_on_later_animation_cycles() {
        assert!(!crossed_note(0.5, 0.4, 0.5, 1.5, false));
        assert!(crossed_note(0.5, 0.5, 0.6, 1.5, false));
        assert!(!crossed_note(0.5, 0.6, 0.7, 1.5, false));
        assert!(crossed_note(0.5, 1.99, 2.01, 1.5, true));
        assert!(!crossed_note(1.5, 0., 0.1, 1.5, true));
        assert!(crossed_note(0., 0., 0.1, 1.5, true));
    }

    #[test]
    fn directional_aim_uses_original_nine_way_arcs() {
        let pose = Transform::IDENTITY;
        assert_eq!(aim_index(pose, Vec3::new(0., 0., -10.), 0.), Some(4));
        assert_eq!(aim_index(pose, Vec3::new(6., 8., -10.), 0.), Some(8));
        assert_eq!(aim_index(pose, Vec3::new(-6., -8., -10.), 0.), Some(0));
        assert_eq!(aim_index(pose, Vec3::new(20., 0., -10.), 0.), None);
        assert_eq!(aim_index(pose, Vec3::new(0., 0., 10.), 0.), None);
    }

    #[test]
    fn wave_health_uses_scheduled_wave_and_ignores_actor_count() {
        let mut waves = PanzerWaves::default();
        waves.wave_count = 1;
        assert_eq!(health_for_wave(&waves), 5000);
        waves.spawned = 9;
        assert_eq!(health_for_wave(&waves), 5000);
        waves.wave_count = 2;
        assert_eq!(health_for_wave(&waves), 6000);
        waves.wave_count = 99;
        assert_eq!(health_for_wave(&waves), 22500);
    }

    #[test]
    fn orphaned_claw_capture_releases_player() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let missing = world.spawn_empty().id();
        let captive = world.spawn(CapturedByPanzer(Some(missing))).id();
        world.despawn(missing);
        world.run_system_once(clear_capture).unwrap();
        assert_eq!(world.get::<CapturedByPanzer>(captive).unwrap().0, None);
    }

    #[test]
    #[ignore = "requires exported installed Origins assets"]
    fn exported_panzer_spawns_keep_authored_angles() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets/entities.json");
        let root = path.parent().unwrap();
        let game = GameDefinition::load_from_root(root);
        let spawns = source::spawns(&std::fs::read(&path).unwrap(), &game);
        assert_eq!(spawns.len(), 63);
        assert_eq!(spawns[0].zone.0, "zone.bolt_stairs");
        assert!(
            spawns
                .iter()
                .any(|spawn| spawn.pose.rotation.angle_between(Quat::IDENTITY) > 1.)
        );
    }
}
#[derive(Resource)]
pub struct PanzerWaves {
    pub next_round: u32,
    pub pending: u32,
    pub spawned: u32,
    pub killed: u32,
    pub fire_part_dropped: bool,
    wave_count: u32,
    timer: f32,
    activated: bool,
    used: BTreeSet<usize>,
    jump_used_until: BTreeMap<usize, f32>,
}
impl Default for PanzerWaves {
    fn default() -> Self {
        Self {
            next_round: 8,
            pending: 0,
            spawned: 0,
            killed: 0,
            fire_part_dropped: false,
            wave_count: 0,
            timer: 0.,
            activated: false,
            used: BTreeSet::new(),
            jump_used_until: BTreeMap::new(),
        }
    }
}
pub fn health_for_wave(waves: &PanzerWaves) -> u32 {
    (5000 + 1000 * waves.wave_count.saturating_sub(1)).min(22500)
}
pub struct PanzerPlugin;
impl Plugin for PanzerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanzerWaves>()
            .add_message::<PanzerClawReleased>()
            .add_message::<PanzerDestroyed>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                reset.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                schedule
                    .in_set(GameStep::Rounds)
                    .run_if(crate::cheats::rounds_active)
                    .before(crate::rounds::advance)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (
                    clear_capture,
                    bind_triggers,
                    nukes,
                    behavior.run_if(crate::cheats::enemies_active),
                    burning,
                    feedback,
                    part_failsafe,
                )
                    .chain()
                    .in_set(GameStep::EnemyIntent)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                claw_hits
                    .in_set(GameStep::Combat)
                    .after(crate::gameplay::combat)
                    .after(crate::projectiles::flight)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                deaths
                    .in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn reset(
    mut commands: Commands,
    mut waves: ResMut<PanzerWaves>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    options: Res<crate::assets::LaunchOptions>,
) {
    *waves = PanzerWaves::default();
    let clips: BTreeMap<_, _> = game
        .content
        .animations
        .iter()
        .enumerate()
        .filter(|(_, a)| a.skeleton.0 == "actor.c_zom_mech_body")
        .map(|(i, a)| (a.id.0.rsplit('.').next().unwrap().to_owned(), i))
        .collect();
    let handles = clips
        .values()
        .map(|i| server.load(game.content.animations[*i].path.clone()))
        .collect();
    let underground = game
        .access
        .volumes
        .iter()
        .filter(|v| v.zone.0 == "zone.ug_bottom_zone")
        .map(|v| {
            (
                Collider::convex_hull(
                    &v.volume
                        .vertices
                        .iter()
                        .copied()
                        .map(Vec3::from_array)
                        .collect::<Vec<_>>(),
                )
                .expect("Panzer underground zone hull"),
                Vec3::from_array(v.volume.position),
            )
        })
        .collect();
    commands.insert_resource(PanzerAnimations {
        clips,
        underground,
        _handles: handles,
        spawns: source::spawns(
            &std::fs::read(
                options.asset_root.join(
                    game.entity_metadata
                        .as_ref()
                        .expect("Panzer entity metadata"),
                ),
            )
            .expect("Read Panzer source spawns"),
            &game,
        ),
    });
}
// zm_tomb.d3dbsp.ents mech_hole_spawner array order and linked landing structs.
pub(crate) fn spawn_quest(commands: &mut Commands, health: u32, hole: usize) -> Entity {
    let (origin, yaw, landing) = [
        ([-3160., 1160., -16.], 0., [-2248., 656., 184.]),
        ([-3184., 1064., -16.], 315., [-2456., 40., 232.]),
        ([-3104., 1080., -16.], 255., [-2712., 32., 232.]),
        ([-3080., 1168., -16.], 285., [-2464., 288., 232.]),
    ][hole];
    let world = |[x, y, z]: [f32; 3]| Vec3::new(x, z, -y) * 0.0254;
    let origin = world(origin);
    let yaw = f32::to_radians(yaw);
    spawn(
        commands,
        origin,
        health,
        true,
        PanzerPhase::Exit {
            origin,
            yaw,
            landing: world(landing),
        },
        Quat::from_rotation_y(yaw),
    )
}

pub(crate) fn spawn(
    commands: &mut Commands,
    position: Vec3,
    health: u32,
    quest: bool,
    phase: PanzerPhase,
    rotation: Quat,
) -> Entity {
    let entity = commands
        .spawn((
            Panzer {
                phase,
                quest,
                captured: None,
                flame_cooldown: 2.,
                claw_cooldown: 5.,
                flame_tick: 0.,
                claw_baseline: 0,
                plate_rewarded: false,
                core_rewarded: false,
                cover_reacted: false,
                vocal_timer: 3.,
                drop_powerup: false,
                death_attacker: None,
                animation_elapsed: 0.,
                claw_intro_seconds: 0.,
                exploded: false,
                boosting: false,
                firing: false,
                aim: 4,
                melee: 0,
                anchor: Transform::from_translation(position + Vec3::Y * 1.2)
                    .with_rotation(rotation),
                tank_tag: None,
                jump_origin: None,
                jump_check: 1.,
                failed_paths: 0,
                arrival_yaw: 0.,
            },
            Zombie::new(
                position + Vec3::Y * 1.2,
                ContentId("enemy.panzer".into()),
                Gait::Run,
            ),
            SessionEntity,
            Collider::capsule(0.5, 1.4),
            ColliderDisabled,
            Transform::from_translation(position + Vec3::Y * 1.2).with_rotation(rotation),
            Visibility::Inherited,
            Vitality::ordinary_actor(health, health).expect("source Panzer health"),
            Armor::panzer(health),
            crate::combat_effects::Wounds::default(),
            crate::hit_regions::Hitboxes::panzer(),
        ))
        .id();
    if !quest {
        commands.entity(entity).insert(PopulationKind::Special);
        crate::audio::emit(
            commands,
            "zmb_ai_mechz_incoming_alarm",
            crate::audio::SoundSource::attached(entity),
        );
    }
    entity
}
fn schedule(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut session: ResMut<Session>,
    mut waves: ResMut<PanzerWaves>,
    access: Res<crate::access::ZoneAccess>,
    query: SpatialQuery,
    players: Query<&Transform, With<Player>>,
    panzers: Query<&Vitality, With<Panzer>>,
    mut rounds: MessageReader<crate::rounds::RoundStarted>,
    animations: Res<PanzerAnimations>,
) {
    let round_started = rounds.read().count() > 0;
    if round_started {
        waves.used.clear();
    }
    if !access.reachable.iter().any(|z| z.0.starts_with("zone.nml")) {
        return;
    }
    let first_activation = !waves.activated;
    waves.activated = true;
    if (first_activation || round_started)
        && waves.pending == 0
        && session.round >= waves.next_round
    {
        if panzers.iter().any(|v| v.state() == LifeState::ActorAlive) {
            waves.next_round += 1;
        } else {
            waves.pending = 1;
            waves.wave_count += 1;
            waves.timer = 10. + session.random() as f32 / u32::MAX as f32 * 5.;
        }
    }
    if waves.pending == 0 {
        return;
    }
    waves.timer -= time.delta_secs();
    if waves.timer > 0. {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };
    let choices = animations
        .spawns
        .iter()
        .enumerate()
        .filter(|(index, spawn)| {
            !waves.used.contains(index) && access.reachable.contains(&spawn.zone)
        });
    let Some((index, spawn_point)) = choices
        .filter(|(_, spawn)| {
            query
                .shape_intersections(
                    &Collider::capsule(0.5, 1.4),
                    spawn.pose.translation + Vec3::Y * 1.2,
                    Quat::IDENTITY,
                    &crate::environment::actor_solids([]),
                )
                .is_empty()
        })
        .min_by(|a, b| {
            a.1.pose
                .translation
                .distance_squared(player.translation)
                .total_cmp(&b.1.pose.translation.distance_squared(player.translation))
        })
    else {
        waves.timer = 0.5;
        return;
    };
    let entity = spawn(
        &mut commands,
        spawn_point.pose.translation,
        health_for_wave(&waves),
        false,
        PanzerPhase::Landing,
        spawn_point.pose.rotation,
    );
    let later = waves.spawned > 0;
    commands.queue(move |world: &mut World| {
        if let Some(mut panzer) = world.get_mut::<Panzer>(entity) {
            panzer.drop_powerup = later;
        }
    });
    waves.used.insert(index);
    waves.spawned += 1;
    waves.pending -= 1;
    waves.next_round = session.round + 4 + session.random() % 2;
}
fn strike(
    damage: &mut DamageQueue,
    attacker: Entity,
    target: Entity,
    amount: u32,
    category: DamageCategory,
) {
    damage.0.push(Impact {
        point: None,
        attacker,
        target,
        source: ImpactSource::Enemy(ContentId("enemy.panzer".into())),
        category,
        region: HitRegion::Torso,
        amount,
    });
}
fn nukes(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut targets: Query<(Entity, &Vitality, &mut PanzerNuke)>,
    mut damage: ResMut<DamageQueue>,
) {
    for (target, health, mut nuke) in &mut targets {
        nuke.remaining -= time.delta_secs();
        if nuke.remaining <= 0. {
            if health.state() == LifeState::ActorAlive {
                damage.0.push(Impact {
                    point: None,
                    attacker: nuke.attacker,
                    target,
                    source: ImpactSource::Powerup,
                    category: DamageCategory::Explosive,
                    region: HitRegion::None,
                    amount: (health.current().unwrap_or(0) as f32 * 0.25).round() as u32,
                });
            }
            commands.entity(target).remove::<PanzerNuke>();
        }
    }
}
fn release(panzer: &mut Panzer, captured: &mut CapturedByPanzer) {
    captured.0 = None;
    panzer.captured = None;
    panzer.firing = false;
    panzer.phase = PanzerPhase::Pursuit;
}

fn clear_capture(
    mut players: Query<(Entity, &mut CapturedByPanzer)>,
    panzers: Query<(&Panzer, &Vitality)>,
) {
    for (player, mut capture) in &mut players {
        if capture.0.is_some_and(|owner| {
            !panzers.get(owner).is_ok_and(|(panzer, health)| {
                panzer.captured == Some(player) && health.state() == LifeState::ActorAlive
            })
        }) {
            capture.0 = None;
        }
    }
}
fn behavior(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    query: SpatialQuery,
    slide: MoveAndSlide,
    mut panzers: Query<
        (
            Entity,
            &mut Panzer,
            &mut Transform,
            &Collider,
            &Vitality,
            &mut Armor,
            &mut Visibility,
        ),
        (Without<Player>, Without<crate::cheats::physics::LivingRagdoll>),
    >,
    mut players: Query<
        (
            Entity,
            &mut Player,
            &mut Transform,
            &Collider,
            &Vitality,
            &mut CapturedByPanzer,
            &mut crate::crafting::BuildableInventory,
            &crate::solo_revive::PerkInventory,
            &mut crate::movement::Mobility,
            &crate::movement::platforms::PlatformSupport,
        ),
        (With<Player>, Without<Panzer>),
    >,
    mut damage: ResMut<DamageQueue>,
    mut received: MessageReader<DamageApplied>,
    mut navigation: ResMut<crate::navigation::Navigation>,
    access: Res<crate::access::ZoneAccess>,
    effects: Res<crate::powerups::PowerupEffects>,
    mut session: ResMut<Session>,
    game: Res<GameDefinition>,
    scripted: (
        Res<PanzerAnimations>,
        Query<&KnockdownRequest>,
        Query<&crate::enemies::ActorRig>,
        MessageWriter<crate::actions::Rumble>,
        Query<
            (Entity, &Transform),
            (
                With<crate::tank::TankBody>,
                Without<Panzer>,
                Without<Player>,
            ),
        >,
        Res<crate::tank::Tank>,
        ResMut<PanzerWaves>,
    ),
    mut released: MessageWriter<PanzerClawReleased>,
    (mut ignitable, flame_volumes): (
        Query<
            (
                Entity,
                &Transform,
                &Vitality,
                Has<FlameIgnited>,
                Has<crate::tank::TankMotion>,
            ),
            (With<Zombie>, Without<Player>, Without<Panzer>),
        >,
        Query<(&FlameVolume, &Collider, &Transform), (Without<Panzer>, Without<Player>)>,
    ),
) {
    let (animations, requests, rigs, mut rumble, tank_body, tank_state, mut waves) = scripted;
    let tank = tank_body
        .single()
        .ok()
        .map(|(entity, pose)| (entity, *pose));
    let mut tank_tags: BTreeSet<_> = panzers.iter().filter_map(|(_, p, ..)| p.tank_tag).collect();
    let dt = time.delta_secs();
    for event in received.read() {
        if let Ok((_, mut panzer, pose, _, health, armor, _)) = panzers.get_mut(event.impact.target)
        {
            let explosive = matches!(event.impact.category, DamageCategory::Explosive)
                || event.impact.source.id().contains("staff_");
            if panzer.captured.is_some()
                && (event.impact.region.left_arm()
                    || explosive
                        && armor.explosive_damage.saturating_sub(panzer.claw_baseline)
                            > health.maximum() / 10)
            {
                if let Some(owner) = panzer.captured
                    && let Ok((_, _, _, _, _, mut captured, _, _, _, _)) = players.get_mut(owner)
                {
                    release(&mut panzer, &mut captured);
                    panzer.firing = false;
                    panzer.boosting = false;
                    panzer.phase = PanzerPhase::Pain(PainKind::Head);
                    panzer.anchor = *pose;
                    panzer.animation_elapsed = 0.;
                    if event.impact.source.player_attributed() {
                        released.write(PanzerClawReleased {
                            releaser: event.impact.attacker,
                            captive: owner,
                        });
                    }
                }
            }
            if armor.broken && !panzer.plate_rewarded {
                panzer.plate_rewarded = true;
                if event.impact.source.player_attributed() {
                    session.award(100);
                }
                if panzer.can_interrupt() {
                    if let Some(owner) = panzer.captured
                        && let Ok((_, _, _, _, _, mut captured, _, _, _, _)) =
                            players.get_mut(owner)
                    {
                        release(&mut panzer, &mut captured);
                    }
                    panzer.firing = false;
                    panzer.anchor = *pose;
                    panzer.phase = PanzerPhase::Pain(PainKind::Faceplate);
                    panzer.boosting = false;
                    panzer.animation_elapsed = 0.;
                }
            }
            if armor.powerplant_cover == 0 && !panzer.cover_reacted {
                panzer.cover_reacted = true;
                if panzer.can_interrupt() {
                    if let Some(owner) = panzer.captured
                        && let Ok((_, _, _, _, _, mut captured, _, _, _, _)) =
                            players.get_mut(owner)
                    {
                        release(&mut panzer, &mut captured);
                    }
                    panzer.firing = false;
                    panzer.anchor = *pose;
                    panzer.phase = PanzerPhase::Pain(PainKind::Powercore);
                    panzer.boosting = false;
                    panzer.animation_elapsed = 0.;
                }
            }
            if armor.powerplant_destroyed && !panzer.core_rewarded {
                panzer.core_rewarded = true;
                if event.impact.source.player_attributed() {
                    session.award(100);
                }
                if let Some(owner) = panzer.captured
                    && let Ok((_, _, _, _, _, mut captured, _, _, _, _)) = players.get_mut(owner)
                {
                    release(&mut panzer, &mut captured);
                }
                if panzer.can_interrupt() {
                    panzer.firing = false;
                    panzer.boosting = false;
                    panzer.phase = PanzerPhase::Stunned(4.);
                    panzer.animation_elapsed = 0.;
                }
            }
        }
    }
    for (entity, mut panzer, mut pose, collider, health, mut armor, mut visible) in &mut panzers {
        armor.ice_cooldown = (armor.ice_cooldown - dt).max(0.);
        if health.state() != LifeState::ActorAlive {
            continue;
        }
        if !rigs.iter().any(|rig| rig.ready_for(entity)) {
            continue;
        }
        if let Ok(request) = requests.get(entity) {
            commands.entity(entity).remove::<KnockdownRequest>();
            if !matches!(
                panzer.phase,
                PanzerPhase::Knockdown { .. }
                    | PanzerPhase::Landing
                    | PanzerPhase::Exit { .. }
                    | PanzerPhase::Hover { .. }
                    | PanzerPhase::Arrive { .. }
            ) {
                if let Some(owner) = panzer.captured
                    && let Ok((_, _, _, _, _, mut captured, _, _, _, _)) = players.get_mut(owner)
                {
                    release(&mut panzer, &mut captured);
                }
                panzer.anchor = *pose;
                panzer.firing = false;
                panzer.boosting = false;
                panzer.phase = PanzerPhase::Knockdown {
                    kind: request.0,
                    stage: 0,
                    remaining: match request.0 {
                        KnockdownKind::Robot => 10.,
                        KnockdownKind::Tank => 5.,
                    },
                };
                panzer.animation_elapsed = 0.;
            }
        }
        if panzer.claw_intro_seconds > 0. && panzer.animation_elapsed >= panzer.claw_intro_seconds {
            panzer.claw_intro_seconds = 0.;
            panzer.animation_elapsed = 0.;
        }
        let previous = panzer.animation().0;
        panzer.flame_cooldown = (panzer.flame_cooldown - dt).max(0.);
        panzer.claw_cooldown = (panzer.claw_cooldown - dt).max(0.);
        let current_clip = animations.clip(&game, panzer.animation().0);
        let clip_start = panzer.animation_elapsed;
        let clip_end = clip_start + dt;
        let looping = panzer.animation().1;
        for note in current_clip.notifications.iter().filter(|n| {
            crossed_note(
                n.seconds,
                clip_start,
                clip_end,
                current_clip.seconds,
                looping,
            )
        }) {
            match note.event.as_str() {
                "booster_on" => panzer.boosting = true,
                "booster_off" => panzer.boosting = false,
                "start_ft" => {
                    panzer.firing = true;
                    panzer.flame_tick = 0.;
                }
                "stop_ft" => panzer.firing = false,
                "impact" => {
                    crate::native_fx::emit(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_mech_jump_landing",
                        Transform::from_translation(pose.translation - Vec3::Y * 1.2),
                    );
                }
                event if event.starts_with("footstep_") => {
                    let tag = if event.starts_with("footstep_left_") {
                        "tag_foot_steam_LE"
                    } else {
                        "tag_foot_steam_RI"
                    };
                    if let Some(socket) = rigs.iter().find_map(|rig| rig.socket(entity, tag)) {
                        crate::native_fx::attach(
                            &mut commands,
                            "maps/zombie_tomb/fx_tomb_mech_foot_step_steam",
                            socket,
                            Transform::IDENTITY,
                        );
                    }
                    if let Ok((_, _, player_pose, _, _, _, _, _, _, _)) = players.single()
                        && (player_pose.translation.y - pose.translation.y).abs() < 100. * 0.0254
                    {
                        rumble.write(crate::actions::Rumble {
                            origin: pose.translation,
                            radius: 1500. * 0.0254,
                            seconds: 0.1,
                            strength: if player_pose
                                .translation
                                .xz()
                                .distance(pose.translation.xz())
                                < 750. * 0.0254
                            {
                                0.35
                            } else {
                                0.25
                            },
                        });
                    }
                }
                event => {
                    if let Some(alias) = event.strip_prefix("sndnt#") {
                        crate::audio::emit(
                            &mut commands,
                            alias,
                            crate::audio::SoundSource::attached(entity),
                        );
                    }
                }
            }
        }
        panzer.vocal_timer -= dt;
        if panzer.vocal_timer <= 0. {
            panzer.vocal_timer = 3. + session.random() as f32 / u32::MAX as f32 * 3.;
            crate::audio::emit(
                &mut commands,
                "zmb_ai_mechz_vox_ambient",
                crate::audio::SoundSource::attached(entity),
            );
        }
        if matches!(
            panzer.phase,
            PanzerPhase::Exit { .. } | PanzerPhase::Hover { .. } | PanzerPhase::Arrive { .. }
        ) {
            // Step 4 animscripted uses authored anchors and native root motion,
            // with fx_cleanup's solo network frame, then a three-second hidden hover.
            let clip = current_clip;
            panzer.animation_elapsed += dt;
            let elapsed = panzer.animation_elapsed;
            let center = Vec3::Y * 1.2;
            match panzer.phase {
                PanzerPhase::Exit { origin, yaw, .. } => {
                    let motion = clip
                        .root_motion
                        .as_ref()
                        .expect("native Panzer exit root motion");
                    pose.translation = origin
                        + center
                        + Quat::from_rotation_y(yaw)
                            * Vec3::from_array(motion.translation_at(elapsed));
                }
                PanzerPhase::Arrive { landing } => {
                    let motion = clip
                        .root_motion
                        .as_ref()
                        .expect("native Panzer arrival root motion");
                    pose.translation = landing
                        + center
                        + Quat::from_rotation_y(panzer.arrival_yaw)
                            * Vec3::from_array(motion.translation_at(elapsed));
                    pose.rotation = Quat::from_rotation_y(panzer.arrival_yaw);
                }
                _ => {}
            }
            let finished = elapsed
                >= if matches!(panzer.phase, PanzerPhase::Hover { .. }) {
                    3.1
                } else {
                    clip.seconds
                };
            if finished {
                panzer.phase = match panzer.phase {
                    PanzerPhase::Exit { landing, .. } => {
                        *visible = Visibility::Hidden;
                        commands.entity(entity).insert(ColliderDisabled);
                        PanzerPhase::Hover { landing }
                    }
                    PanzerPhase::Hover { landing } => {
                        let arrival = animations
                            .clip(&game, "ai_zombie_mech_arrive")
                            .root_motion
                            .as_ref()
                            .expect("native Panzer arrival root motion");
                        pose.translation = landing
                            + center
                            + Quat::from_rotation_y(panzer.arrival_yaw)
                                * Vec3::from_array(arrival.translation_at(0.));
                        pose.rotation = Quat::from_rotation_y(panzer.arrival_yaw);
                        *visible = Visibility::Inherited;
                        PanzerPhase::Arrive { landing }
                    }
                    PanzerPhase::Arrive { .. } => {
                        commands.entity(entity).remove::<ColliderDisabled>();
                        PanzerPhase::Pursuit
                    }
                    _ => unreachable!(),
                };
                panzer.animation_elapsed = 0.;
            }
            continue;
        }
        let independent = match panzer.phase {
            PanzerPhase::Pain(_) => {
                if let Some(motion) = &current_clip.root_motion {
                    pose.translation = panzer.anchor.translation
                        + panzer.anchor.rotation
                            * Vec3::from_array(motion.translation_at(clip_end));
                }
                if clip_end >= current_clip.seconds {
                    panzer.phase = PanzerPhase::Pursuit;
                }
                true
            }
            PanzerPhase::Landing => {
                let motion = current_clip
                    .root_motion
                    .as_ref()
                    .expect("Panzer entrance source root motion");
                pose.translation = panzer.anchor.translation
                    + panzer.anchor.rotation * Vec3::from_array(motion.translation_at(clip_end));
                panzer.phase = if clip_end < current_clip.seconds {
                    PanzerPhase::Landing
                } else {
                    commands.entity(entity).remove::<ColliderDisabled>();
                    panzer.boosting = false;
                    crate::native_fx::emit(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_mech_jump_landing",
                        Transform::from_translation(pose.translation - Vec3::Y * 1.2),
                    );
                    PanzerPhase::Pursuit
                };
                true
            }
            PanzerPhase::Knockdown {
                kind,
                stage,
                remaining,
            } => {
                if let Some(motion) = &current_clip.root_motion {
                    pose.translation = panzer.anchor.translation
                        + panzer.anchor.rotation
                            * Vec3::from_array(motion.translation_at(clip_end));
                }
                if clip_end >= current_clip.seconds {
                    panzer.phase = match stage {
                        0 => PanzerPhase::Knockdown {
                            kind,
                            stage: 1,
                            remaining,
                        },
                        1 if remaining > current_clip.seconds => PanzerPhase::Knockdown {
                            kind,
                            stage: 1,
                            remaining: remaining - current_clip.seconds,
                        },
                        1 => PanzerPhase::Knockdown {
                            kind,
                            stage: 2,
                            remaining: 0.,
                        },
                        _ => PanzerPhase::Pursuit,
                    };
                    panzer.anchor = *pose;
                    panzer.animation_elapsed = 0.;
                } else {
                    panzer.animation_elapsed = clip_end;
                }
                continue;
            }
            PanzerPhase::Stunned(remaining) => {
                panzer.phase = if remaining > dt {
                    PanzerPhase::Stunned(remaining - dt)
                } else {
                    PanzerPhase::Pursuit
                };
                true
            }
            _ => false,
        };
        if independent {
            panzer.animation_elapsed = if previous == panzer.animation().0 {
                panzer.animation_elapsed + dt
            } else {
                0.
            };
            continue;
        }
        let Ok((
            owner,
            mut player,
            mut player_pose,
            player_collider,
            vitality,
            mut captured,
            mut shield,
            perks,
            mut mobility,
            support,
        )) = players.single_mut()
        else {
            continue;
        };
        if vitality.state() != LifeState::Standing {
            if panzer.captured == Some(owner) {
                release(&mut panzer, &mut captured);
            }
            panzer.firing = false;
            panzer.phase = PanzerPhase::Pursuit;
            continue;
        }
        if effects.zombie_blood > 0. && panzer.captured.is_none() {
            panzer.firing = false;
            panzer.phase = PanzerPhase::Pursuit;
            continue;
        }
        let delta = player_pose.translation - pose.translation;
        let distance = delta.length();
        let path_check = panzer.jump_check <= dt;
        if matches!(
            panzer.phase,
            PanzerPhase::Pursuit
                | PanzerPhase::LiftOff
                | PanzerPhase::Sprint
                | PanzerPhase::TouchDown
        ) {
            panzer.jump_check -= dt;
            if panzer.jump_check <= 0. {
                panzer.jump_check = 1.;
                if distance * distance >= 4410000. * 0.0254_f32.powi(2) || panzer.failed_paths >= 3
                {
                    panzer.jump_origin = animations
                        .spawns
                        .iter()
                        .enumerate()
                        .filter(|(_, spawn)| access.reachable.contains(&spawn.zone))
                        .min_by(|a, b| {
                            a.1.pose
                                .translation
                                .distance_squared(pose.translation)
                                .total_cmp(&b.1.pose.translation.distance_squared(pose.translation))
                        })
                        .map(|(i, _)| i);
                } else {
                    panzer.jump_origin = None;
                }
            }
            if let Some(index) = panzer.jump_origin {
                let origin = animations.spawns[index].pose.translation;
                if origin.distance_squared(pose.translation - Vec3::Y * 1.2)
                    <= (10_f32 * 0.0254).powi(2)
                {
                    if let Some((index, landing)) = animations
                        .spawns
                        .iter()
                        .enumerate()
                        .filter(|(i, spawn)| {
                            access.reachable.contains(&spawn.zone)
                                && waves
                                    .jump_used_until
                                    .get(i)
                                    .is_none_or(|until| *until <= time.elapsed_secs())
                        })
                        .min_by(|a, b| {
                            a.1.pose
                                .translation
                                .distance_squared(player_pose.translation)
                                .total_cmp(
                                    &b.1.pose
                                        .translation
                                        .distance_squared(player_pose.translation),
                                )
                        })
                    {
                        waves
                            .jump_used_until
                            .insert(index, time.elapsed_secs() + 5.);
                        panzer.arrival_yaw = landing.pose.rotation.to_euler(EulerRot::YXZ).0;
                        panzer.phase = PanzerPhase::Exit {
                            origin,
                            yaw: pose.rotation.to_euler(EulerRot::YXZ).0,
                            landing: landing.pose.translation,
                        };
                        panzer.jump_origin = None;
                        panzer.failed_paths = 0;
                        panzer.animation_elapsed = 0.;
                        continue;
                    }
                }
            }
        }
        let direction = delta.with_y(0.).normalize_or_zero();
        let forward = pose.rotation * Vec3::NEG_Z;
        let facing = forward.dot(direction);
        let visible = Dir3::new(delta).ok().is_some_and(|d| {
            query
                .cast_ray(
                    pose.translation,
                    d,
                    distance,
                    true,
                    &crate::environment::solids([entity, owner]),
                )
                .is_none()
        });
        let underground = animations.underground.iter().any(|(shape, position)| {
            shape.contains_point(*position, Quat::IDENTITY, pose.translation)
        });
        let on_tank = tank.is_some_and(|(entity, _)| support.entity == Some(entity));
        let tank_goal = if on_tank {
            let (_, tank_pose) = tank.unwrap();
            let goals = crate::tank::panzer_goals(tank_pose);
            let chosen = goals
                .iter()
                .enumerate()
                .filter(|(index, _)| panzer.tank_tag == Some(*index) || !tank_tags.contains(index))
                .min_by(|a, b| {
                    a.1.distance_squared(pose.translation)
                        .total_cmp(&b.1.distance_squared(pose.translation))
                });
            if let Some((index, point)) = chosen {
                if let Some(old) = panzer.tank_tag {
                    tank_tags.remove(&old);
                }
                panzer.tank_tag = Some(index);
                tank_tags.insert(index);
                Some(*point + Vec3::Y * 1.2)
            } else {
                None
            }
        } else {
            if let Some(old) = panzer.tank_tag.take() {
                tank_tags.remove(&old);
            }
            None
        };
        if let Some(aim) = aim_index(
            *pose,
            player_pose.translation,
            if matches!(panzer.phase, PanzerPhase::Flame | PanzerPhase::FlameWindup) {
                26. * 0.0254
            } else {
                0.
            },
        ) {
            panzer.aim = aim;
        }
        if panzer.firing {
            let touching = flame_volumes
                .iter()
                .find(|(volume, _, _)| volume.owner == Some(entity))
                .map(|(_, shape, pose)| {
                    query.shape_intersections(
                        shape,
                        pose.translation,
                        pose.rotation,
                        &SpatialQueryFilter::default().with_excluded_entities([entity]),
                    )
                })
                .unwrap_or_default();
            panzer.flame_tick -= dt;
            if panzer.flame_tick <= 0. {
                panzer.flame_tick += 0.5;
                if touching.contains(&owner)
                    || matches!(panzer.phase, PanzerPhase::FlameSweep)
                        && on_tank
                        && tank_state.phase != crate::tank::TankPhase::Moving
                {
                    let juggernog = perks.owned.iter().any(|id| id.0 == "perk.juggernog");
                    strike(
                        &mut damage,
                        entity,
                        owner,
                        if juggernog { 45 } else { 30 },
                        DamageCategory::Fire,
                    );
                }
            }
            for (target, _, vitality, burning, riding) in &mut ignitable {
                if !burning
                    && vitality.state() == LifeState::ActorAlive
                    && (touching.contains(&target)
                        || matches!(panzer.phase, PanzerPhase::FlameSweep)
                            && riding
                            && tank_state.phase != crate::tank::TankPhase::Moving)
                {
                    let sound = crate::audio::emit_loop(
                        &mut commands,
                        "zmb_fire_loop",
                        crate::audio::SoundSource::attached(target),
                    );
                    commands.entity(target).insert(FlameIgnited {
                        remaining: 12.,
                        pulse: 0.,
                        sound,
                    });
                }
            }
        }
        match panzer.phase {
            PanzerPhase::Pursuit
            | PanzerPhase::LiftOff
            | PanzerPhase::Sprint
            | PanzerPhase::TouchDown => {
                if tank_goal.is_some_and(|goal| {
                    goal.xz().distance(pose.translation.xz()) < 48. * 0.0254
                        && (goal.y - pose.translation.y).abs() < 48. * 0.0254
                }) {
                    let facing = tank.unwrap().1.translation - pose.translation;
                    pose.rotation = Quat::from_rotation_y((-facing.x).atan2(-facing.z));
                    panzer.phase = PanzerPhase::FlameSweep;
                } else if !on_tank && visible && distance < 1.9 {
                    panzer.melee = session.random() as usize % 2;
                    panzer.phase = PanzerPhase::Melee;
                } else if !on_tank
                    && visible
                    && (2.54..5.68).contains(&distance)
                    && aim_index(*pose, player_pose.translation, 26. * 0.0254).is_some()
                    && panzer.flame_cooldown == 0.
                {
                    panzer.phase = PanzerPhase::FlameWindup;
                    panzer.flame_cooldown = 5.;
                } else if !on_tank
                    && visible
                    && (7.62..=25.4).contains(&distance)
                    && facing > 45_f32.to_radians().cos()
                    && !underground
                    && !(player.prone && mobility.dive.is_some())
                    && !armor.powerplant_destroyed
                    && panzer.claw_cooldown == 0.
                    && captured.0.is_none()
                {
                    panzer.phase = PanzerPhase::ClawWindup;
                    panzer.claw_intro_seconds = 0.;
                    panzer.claw_cooldown = 7.;
                } else {
                    let feet = pose.translation - Vec3::Y * 1.2;
                    let target = navigation.destination(
                        feet,
                        panzer
                            .jump_origin
                            .map(|i| animations.spawns[i].pose.translation)
                            .or(tank_goal.map(|p| p - Vec3::Y * 1.2))
                            .unwrap_or(player_pose.translation - Vec3::Y * player.height * 0.5),
                        &access,
                    ) + Vec3::Y * 1.2;
                    match panzer.phase {
                        PanzerPhase::Pursuit if distance > 9.144 && !armor.powerplant_destroyed => {
                            panzer.phase = PanzerPhase::LiftOff
                        }
                        PanzerPhase::LiftOff if clip_end >= current_clip.seconds => {
                            panzer.phase = PanzerPhase::Sprint
                        }
                        PanzerPhase::Sprint if distance < 6.096 => {
                            panzer.phase = PanzerPhase::TouchDown
                        }
                        PanzerPhase::TouchDown if clip_end >= current_clip.seconds => {
                            panzer.phase = PanzerPhase::Pursuit
                        }
                        _ => {}
                    }
                    let motion = current_clip
                        .root_motion
                        .as_ref()
                        .expect("Panzer movement source root motion");
                    let displacement = Vec3::from_array(if looping {
                        motion.loop_displacement(clip_start, clip_end)
                    } else {
                        let a = Vec3::from_array(motion.translation_at(clip_start));
                        (Vec3::from_array(motion.translation_at(clip_end)) - a).to_array()
                    });
                    let velocity = (target - pose.translation).with_y(0.).normalize_or_zero()
                        * (displacement.xz().length() / dt.max(f32::EPSILON));
                    let result = crate::player::slide_character(
                        &slide,
                        &query,
                        crate::player::MotionShape {
                            collider,
                            radius: 0.5,
                            step_height: 0.45,
                            grounded: true,
                        },
                        pose.translation,
                        velocity - Vec3::Y * 2.,
                        time.delta(),
                        &crate::environment::actor_solids([entity]),
                    );
                    if path_check {
                        if target.distance_squared(pose.translation) < 0.0001
                            || result.position.distance_squared(pose.translation) < 0.000001
                        {
                            panzer.failed_paths = panzer.failed_paths.saturating_add(1);
                        } else {
                            panzer.failed_paths = 0;
                        }
                    }
                    pose.translation = result.position;
                }
                if direction.length_squared() > 0. {
                    let desired = Quat::from_rotation_y((-direction.x).atan2(-direction.z));
                    pose.rotation = pose
                        .rotation
                        .rotate_towards(desired, 180_f32.to_radians() * dt);
                }
            }
            PanzerPhase::Melee => {
                if current_clip
                    .notifications
                    .iter()
                    .any(|n| n.event == "fire" && n.seconds >= clip_start && n.seconds < clip_end)
                    && distance < 2.2
                    && visible
                {
                    strike(&mut damage, entity, owner, 75, DamageCategory::Melee);
                }
                if clip_end >= current_clip.seconds {
                    panzer.phase = PanzerPhase::Pursuit;
                }
            }
            PanzerPhase::FlameWindup => {
                panzer.phase = if clip_end < current_clip.seconds {
                    PanzerPhase::FlameWindup
                } else {
                    panzer.flame_tick = 0.;
                    PanzerPhase::Flame
                };
            }
            PanzerPhase::Flame => {
                if distance > 50000_f32.sqrt() * 0.0254
                    || !visible
                    || aim_index(*pose, player_pose.translation, 26. * 0.0254).is_none()
                {
                    panzer.firing = false;
                    panzer.phase = PanzerPhase::Pursuit;
                }
            }
            PanzerPhase::FlameSweep => {
                if clip_end >= current_clip.seconds {
                    panzer.firing = false;
                    if on_tank && tank_state.phase != crate::tank::TankPhase::Moving {
                        panzer.animation_elapsed = 0.;
                        continue;
                    } else {
                        panzer.phase = PanzerPhase::Pursuit;
                    }
                }
            }
            PanzerPhase::HeldBurn => {
                if clip_end >= current_clip.seconds {
                    release(&mut panzer, &mut captured);
                }
            }
            PanzerPhase::ClawWindup => {
                if current_clip.notifications.iter().any(|n| {
                    n.event == "muzzleflash" && n.seconds >= clip_start && n.seconds < clip_end
                }) {
                    panzer.claw_intro_seconds = current_clip.seconds;
                    panzer.phase = PanzerPhase::Claw {
                        tip: pose.translation + Vec3::Y * 0.4,
                        direction: delta.normalize_or_zero(),
                        remaining: distance / 30.48,
                    };
                    crate::audio::emit(
                        &mut commands,
                        "zmb_ai_mechz_claw_fire",
                        crate::audio::SoundSource::attached(entity),
                    );
                }
            }
            PanzerPhase::ShieldBreak(remaining) => {
                if remaining <= dt {
                    shield.shield_health = shield.shield_health.saturating_sub(1);
                    if shield.shield_health == 0 {
                        shield.shield_held = false;
                        shield.owned.remove("tomb_shield_zm");
                    }
                    panzer.phase = PanzerPhase::Pursuit;
                } else {
                    panzer.phase = PanzerPhase::ShieldBreak(remaining - dt);
                }
            }
            PanzerPhase::Claw {
                tip,
                direction,
                remaining,
            } => {
                let step = direction * 30.48 * dt;
                let blocker = Dir3::new(step).ok().and_then(|d| {
                    query.cast_shape(
                        &Collider::cuboid(0.762, 1.524, 0.762),
                        tip,
                        Quat::IDENTITY,
                        d,
                        &ShapeCastConfig::from_max_distance(step.length()),
                        &crate::environment::solids([entity]),
                    )
                });
                let contact = blocker.is_some_and(|hit| hit.entity == owner)
                    || blocker.is_none() && (tip + step).distance(player_pose.translation) < 0.65;
                if contact {
                    if shield.shield_held && shield.shield_health > 0 {
                        shield.shield_health = shield
                            .shield_health
                            .saturating_sub(crate::crafting::SHIELD_HEALTH - 1);
                        if shield.shield_health == 0 {
                            shield.shield_held = false;
                            shield.owned.remove("tomb_shield_zm");
                        }
                        panzer.phase = PanzerPhase::ShieldBreak(1.);
                    } else {
                        player.prone = false;
                        player.crouched = false;
                        player.height = game.tuning.standing_height;
                        player.eye_height = game.tuning.eye_height;
                        player.yaw = delta.x.atan2(delta.z);
                        mobility.stance = crate::movement::Stance::Standing;
                        mobility.dive = None;
                        commands
                            .entity(owner)
                            .insert(crate::player::capsule(game.tuning.radius, player.height));
                        captured.0 = Some(entity);
                        panzer.captured = Some(owner);
                        panzer.claw_baseline = armor.explosive_damage;
                        panzer.phase = PanzerPhase::Pulling(8.);
                    }
                } else if blocker.is_some() || remaining <= dt {
                    panzer.phase = PanzerPhase::Pursuit;
                } else {
                    panzer.phase = PanzerPhase::Claw {
                        tip: tip + step,
                        direction,
                        remaining: remaining - dt,
                    };
                }
            }
            PanzerPhase::Pulling(remaining) => {
                if panzer.captured != Some(owner) || captured.0 != Some(entity) || remaining <= dt {
                    release(&mut panzer, &mut captured);
                } else if distance < 1.8 {
                    panzer.flame_tick = 0.;
                    panzer.phase = PanzerPhase::HeldBurn;
                } else {
                    let velocity =
                        (pose.translation - player_pose.translation).normalize_or_zero() * 5.08;
                    let result = slide.move_and_slide(
                        player_collider,
                        player_pose.translation,
                        Quat::IDENTITY,
                        velocity,
                        time.delta(),
                        &MoveAndSlideConfig::default(),
                        &crate::environment::player_solids([owner, entity]),
                        |_| MoveAndSlideHitResponse::Accept,
                    );
                    player_pose.translation = result.position;
                    player.velocity = Vec3::ZERO;
                    panzer.phase = PanzerPhase::Pulling(remaining - dt);
                }
            }
            PanzerPhase::Landing
            | PanzerPhase::Exit { .. }
            | PanzerPhase::Hover { .. }
            | PanzerPhase::Arrive { .. }
            | PanzerPhase::Stunned(_)
            | PanzerPhase::Pain(_)
            | PanzerPhase::Knockdown { .. }
            | PanzerPhase::Dead(_) => {}
        }
        panzer.animation_elapsed = if previous == panzer.animation().0 {
            panzer.animation_elapsed + dt
        } else {
            0.
        };
    }
}
fn deaths(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut deaths: MessageReader<EnemyDeath>,
    mut panzers: Query<(Entity, &mut Panzer, &Transform, &Vitality)>,
    mut players: Query<&mut CapturedByPanzer>,
    mut waves: ResMut<PanzerWaves>,
    mut parts: ResMut<crate::environment::parts::Parts>,
    mut blasts: ResMut<crate::equipment::BlastQueue>,
    session: Res<crate::session::SessionId>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut rng: ResMut<Session>,
    mut destroyed: MessageWriter<PanzerDestroyed>,
    spatial: SpatialQuery,
    rigs: Query<&crate::enemies::ActorRig>,
    globals: Query<&GlobalTransform>,
) {
    for event in deaths.read() {
        let Ok((entity, mut panzer, pose, _)) = panzers.get_mut(event.0.impact.target) else {
            continue;
        };
        waves.killed += 1;
        panzer.death_attacker = Some(event.0.impact.attacker);
        panzer.phase = PanzerPhase::Dead(2.);
        panzer.firing = false;
        panzer.boosting = false;
        panzer.animation_elapsed = 0.;
        if let Some(owner) = panzer.captured.take()
            && let Ok(mut captured) = players.get_mut(owner)
        {
            captured.0 = None;
        }
        commands.entity(entity).insert(ColliderDisabled);
        crate::audio::emit(
            &mut commands,
            "zmb_ai_mechz_vox_death",
            crate::audio::SoundSource::World(pose.translation),
        );
    }
    for (entity, mut panzer, pose, health) in &mut panzers {
        if health.state() != LifeState::ActorDead {
            continue;
        }
        let PanzerPhase::Dead(remaining) = panzer.phase else {
            continue;
        };
        if remaining > time.delta_secs() {
            panzer.phase = PanzerPhase::Dead(remaining - time.delta_secs());
            panzer.animation_elapsed += time.delta_secs();
            continue;
        }
        if panzer.exploded {
            commands.entity(entity).despawn();
            continue;
        }
        let attacker = panzer.death_attacker.unwrap_or(entity);
        let position = pose.translation - Vec3::Y * 1.2;
        let explosion = rigs
            .iter()
            .find_map(|r| r.socket(entity, "tag_powersupply"))
            .and_then(|e| globals.get(e).ok())
            .map_or(pose.translation, GlobalTransform::translation);
        crate::native_fx::emit(
            &mut commands,
            "maps/zombie_tomb/fx_tomb_mech_death",
            Transform::from_translation(explosion),
        );
        crate::audio::emit(
            &mut commands,
            "zmb_ai_mechz_death_explode",
            crate::audio::SoundSource::World(explosion),
        );
        blasts.0.push(crate::equipment::BlastRequest {
            session: *session,
            owner: entity,
            definition: WeaponId("weapon.panzer_death".into()),
            center: explosion,
            radius: 3.2512,
            inner_radius: 0.,
            inner_damage: 100,
            outer_damage: 25,
            direction: None,
            category: Some(DamageCategory::Explosive),
            player_damage: None,
            source: Some(ImpactSource::Enemy(ContentId("enemy.panzer".into()))),
        });
        if !panzer.quest && !waves.fire_part_dropped {
            let from = position + Vec3::Y * 1.016;
            let ground = spatial
                .cast_ray(
                    from,
                    Dir3::NEG_Y,
                    100.,
                    true,
                    &crate::environment::solids([entity]),
                )
                .map_or(position, |hit| from - Vec3::Y * hit.distance);
            if let Some(piece) = crate::crafting::reveal_part(
                &mut commands,
                &mut parts,
                "part.elemental_staff_fire.lower_staff",
                ground + Vec3::Y * 0.8128,
            ) {
                commands.entity(piece).insert(PanzerPartFailsafe {
                    remaining: 120.,
                    check: 0.,
                    movement: None,
                });
            }
            waves.fire_part_dropped = true;
        }
        if panzer.drop_powerup && !panzer.quest && players.contains(attacker) {
            let choices: Vec<_> = game
                .content
                .powerups
                .iter()
                .filter(|p| {
                    matches!(
                        p.id.0.as_str(),
                        "powerup.full_ammo"
                            | "powerup.nuke"
                            | "powerup.insta_kill"
                            | "powerup.double_points"
                            | "powerup.zombie_blood"
                    )
                })
                .collect();
            if !choices.is_empty() {
                let choice = choices[rng.random() as usize % choices.len()];
                crate::powerups::spawn_named(
                    &mut commands,
                    &game,
                    &server,
                    choice.id.0.trim_start_matches("powerup."),
                    position,
                );
            }
        }
        destroyed.write(PanzerDestroyed {
            entity,
            position,
            quest: panzer.quest,
            attacker,
        });
        panzer.exploded = true;
        panzer.phase = PanzerPhase::Dead(3.);
        commands.entity(entity).insert(Visibility::Hidden);
    }
}
fn burning(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut burning: Query<(Entity, &Zombie, &Transform, &Vitality, &mut FlameIgnited)>,
    players: Query<(Entity, &Transform, &Vitality), With<Player>>,
    mut damage: ResMut<DamageQueue>,
    mut blasts: ResMut<crate::equipment::BlastQueue>,
    session: Res<crate::session::SessionId>,
) {
    for (entity, zombie, pose, health, mut burn) in &mut burning {
        burn.remaining -= time.delta_secs();
        if health.state() == LifeState::ActorDead {
            blasts.0.push(crate::equipment::BlastRequest {
                session: *session,
                owner: entity,
                definition: WeaponId("weapon.panzer_ignited_zombie".into()),
                center: pose.translation,
                radius: 3.2512,
                inner_radius: 0.,
                inner_damage: 30,
                outer_damage: 15,
                direction: None,
                category: Some(DamageCategory::Explosive),
                player_damage: None,
                source: Some(ImpactSource::Enemy(zombie.definition.clone())),
            });
            commands.entity(burn.sound).despawn();
            commands.entity(entity).remove::<FlameIgnited>();
            commands.entity(entity).insert(Visibility::Hidden);
            continue;
        }
        if burn.remaining <= 0. {
            commands.entity(burn.sound).despawn();
            commands.entity(entity).remove::<FlameIgnited>();
            continue;
        }
        burn.pulse -= time.delta_secs();
        if burn.pulse <= 0. {
            burn.pulse += 1.;
            for (target, body, vitality) in &players {
                if vitality.state() == LifeState::Standing
                    && body.translation.distance_squared(pose.translation) < 0.635_f32.powi(2)
                {
                    damage.0.push(Impact {
                        point: None,
                        attacker: entity,
                        target,
                        source: ImpactSource::Enemy(zombie.definition.clone()),
                        category: DamageCategory::Fire,
                        region: HitRegion::Torso,
                        amount: 2,
                    });
                }
            }
        }
    }
}

#[derive(Component, Default)]
struct PanzerFeedback {
    flame: Option<Entity>,
    sound: Option<Entity>,
    headlamp: Option<Entity>,
    exhaust: Option<Entity>,
    boosters: Vec<Entity>,
    rocket_sound: Option<Entity>,
    helmet: bool,
    powerplant: bool,
}
fn feedback(
    mut commands: Commands,
    mut panzers: Query<(Entity, &Panzer, &Armor, &mut PanzerFeedback)>,
    rigs: Query<&crate::enemies::ActorRig>,
) {
    for (entity, panzer, armor, mut state) in &mut panzers {
        let socket = |name: &str| rigs.iter().find_map(|r| r.socket(entity, name));
        let dead = matches!(panzer.phase, PanzerPhase::Dead(_));
        let hidden = matches!(panzer.phase, PanzerPhase::Hover { .. });
        if !dead && !hidden {
            if state.headlamp.is_none() {
                if let Some(owner) = socket("tag_headlamp_FX") {
                    state.headlamp = Some(crate::native_fx::attach(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_mech_head_light",
                        owner,
                        Transform::IDENTITY,
                    ));
                }
            }
            if state.exhaust.is_none() {
                if let Some(owner) = socket("tag_back_exhaust_FX") {
                    state.exhaust = Some(crate::native_fx::attach(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_mech_exhaust_smoke",
                        owner,
                        Transform::IDENTITY,
                    ));
                }
            }
        } else {
            for emitter in [state.headlamp.take(), state.exhaust.take()]
                .into_iter()
                .flatten()
            {
                commands.entity(emitter).despawn();
            }
        }
        let flame = panzer.firing && !dead;
        if flame && state.flame.is_none() {
            if let Some(owner) = socket("tag_flamethrower_FX") {
                state.flame = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_mech_wpn_flamethrower",
                    owner,
                    Transform::IDENTITY,
                ));
                crate::audio::emit(
                    &mut commands,
                    "zmb_ai_mechz_flame_start",
                    crate::audio::SoundSource::attached(entity),
                );
                state.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_ai_mechz_flame_loop",
                    crate::audio::SoundSource::attached(entity),
                ));
            }
        } else if !flame {
            if let Some(e) = state.flame.take() {
                commands.entity(e).despawn();
                crate::audio::emit(
                    &mut commands,
                    "zmb_ai_mechz_flame_stop",
                    crate::audio::SoundSource::attached(entity),
                );
            }
            if let Some(e) = state.sound.take() {
                commands.entity(e).despawn();
            }
        }
        let boosting = panzer.boosting && !dead;
        let rocket = boosting && panzer.quest;
        if rocket && state.rocket_sound.is_none() {
            crate::audio::emit(
                &mut commands,
                "zmb_ai_mechz_rocket_start",
                crate::audio::SoundSource::attached(entity),
            );
            let sound = crate::audio::emit_loop(
                &mut commands,
                "zmb_ai_mechz_rocket_loop",
                crate::audio::SoundSource::attached(entity),
            );
            commands.entity(sound).insert(crate::audio::SoundControl {
                volume: 0.75,
                ..default()
            });
            state.rocket_sound = Some(sound);
        } else if !rocket && let Some(sound) = state.rocket_sound.take() {
            if dead {
                commands.entity(sound).try_despawn();
            } else {
                commands.entity(sound).insert(crate::audio::SoundFade {
                    remaining: 1.,
                    duration: 1.,
                    out: true,
                });
                crate::audio::emit(
                    &mut commands,
                    "zmb_ai_mechz_rocket_stop",
                    crate::audio::SoundSource::attached(entity),
                );
            }
        }
        if boosting && !hidden && state.boosters.is_empty() {
            for (tag, name) in [
                (
                    "tag_booster_LE_FX",
                    "maps/zombie_tomb/fx_tomb_mech_jump_booster",
                ),
                (
                    "tag_booster_RI_FX",
                    "maps/zombie_tomb/fx_tomb_mech_jump_booster",
                ),
                (
                    "tag_leg_booster_LE_FX",
                    "maps/zombie_tomb/fx_tomb_mech_jump_booster_sm",
                ),
                (
                    "tag_leg_booster_RI_FX",
                    "maps/zombie_tomb/fx_tomb_mech_jump_booster_sm",
                ),
            ] {
                if let Some(owner) = socket(tag) {
                    state.boosters.push(crate::native_fx::attach(
                        &mut commands,
                        name,
                        owner,
                        Transform::IDENTITY,
                    ));
                }
            }
        } else if !boosting || hidden {
            for e in state.boosters.drain(..) {
                commands.entity(e).despawn();
            }
        }
        if armor.broken && !state.helmet {
            state.helmet = true;
            if let Some(owner) = socket("j_helmet") {
                crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_mech_dmg_armor_face",
                    owner,
                    Transform::IDENTITY,
                );
            }
        }
        if armor.powerplant_destroyed && !state.powerplant {
            state.powerplant = true;
            if let Some(owner) = socket("tag_powersupply") {
                crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_mech_dmg_sparks",
                    owner,
                    Transform::IDENTITY,
                );
            }
        }
    }
}

#[derive(Component)]
struct PanzerPartFailsafe {
    remaining: f32,
    check: f32,
    movement: Option<(Vec3, Vec3, f32)>,
}
fn part_failsafe(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut parts: Query<(Entity, &mut Transform, &mut PanzerPartFailsafe), Without<Player>>,
    players: Query<&Transform, With<Player>>,
    animations: Res<PanzerAnimations>,
) {
    let dt = time.delta_secs();
    for (entity, mut pose, mut failsafe) in &mut parts {
        if let Some((from, to, elapsed)) = failsafe.movement.as_mut() {
            *elapsed += dt;
            pose.translation = from.lerp(*to, (*elapsed / 3.).min(1.));
            if *elapsed >= 3. {
                commands.entity(entity).remove::<PanzerPartFailsafe>();
            }
            continue;
        }
        failsafe.remaining -= dt;
        if failsafe.remaining > 0. {
            continue;
        }
        failsafe.check -= dt;
        if failsafe.check > 0. {
            continue;
        }
        failsafe.check = 1.;
        if players
            .iter()
            .any(|p| p.translation.xz().distance_squared(pose.translation.xz()) < 25.4_f32.powi(2))
        {
            continue;
        }
        if let Some(spawn) = animations.spawns.iter().min_by(|a, b| {
            a.pose
                .translation
                .xz()
                .distance_squared(pose.translation.xz())
                .total_cmp(
                    &b.pose
                        .translation
                        .xz()
                        .distance_squared(pose.translation.xz()),
                )
        }) {
            failsafe.movement = Some((
                pose.translation,
                spawn.pose.translation + Vec3::Y * 0.8128,
                0.,
            ));
        }
    }
}
