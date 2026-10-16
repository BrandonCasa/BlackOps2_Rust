use crate::{
    GameState,
    assets::GameDefinition,
    damage::{DamageQueue, Impact},
    environment::support::{
        DamageEmitter, EnvironmentOwner, EnvironmentalHit, ShootableEnvironment,
    },
    gameplay::{GameStep, Session, SessionEntity},
    input::GameInput,
    interactions::InteractionFocus,
    life::{LifeState, Vitality},
    player::Player,
    quests::{MainQuest, QuestStage},
};
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    world_serialization::{WorldInstanceReady, WorldInstanceSpawner},
};
mod source;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HatchState {
    Closed,
    Open,
    Entered,
    Closing(f32),
}
#[derive(Clone, Copy, Debug)]
pub struct RobotPass {
    pub delay: f32,
    pub step: usize,
    pub clock: f32,
    pub eligible: Option<usize>,
    pub hatch: HatchState,
    age: f32,
    ready_at: Option<f32>,
    can_open: bool,
    hatch_age: f32,
    intro: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Occupancy {
    pub player: Entity,
    pub robot: usize,
    pub remaining: f32,
    pub exit: Vec3,
    tank_bunker: bool,
    yaw: f32,
    pitch: f32,
}
#[derive(Resource)]
pub struct Robots {
    pub passes: [Option<RobotPass>; 3],
    pub occupancy: Option<Occupancy>,
    pub entries: u32,
    pub evacuations: u32,
    pub stomps: u32,
    pub triple_round: u32,
    pub triple_active: bool,
    foot_activation_delay: f32,
    next: f32,
    last: Option<usize>,
    latched: bool,
}
impl Default for Robots {
    fn default() -> Self {
        Self {
            passes: [None; 3],
            occupancy: None,
            entries: 0,
            evacuations: 0,
            stomps: 0,
            triple_round: 0,
            triple_active: false,
            // the player spawns inside the foot and gets killed somehow, look into this later
            foot_activation_delay: 30.,
            next: 5.,
            last: None,
            latched: false,
        }
    }
}
#[derive(Message, Clone, Copy)]
pub struct RobotStomp(pub Vec3);
#[derive(Message)]
struct FootEvent {
    robot: usize,
    side: usize,
    action: FootAction,
}
#[derive(Clone, Copy)]
enum FootAction {
    Kill,
    Impact,
    OpeningImpact,
    Sound(&'static str),
}

fn foot_action(event: &str) -> Option<(usize, FootAction)> {
    Some(match event {
        "kill_zombies_leftfoot_1" => (0, FootAction::Kill),
        "kill_zombies_rightfoot_1" => (1, FootAction::Kill),
        "footstep_left_large" => (0, FootAction::Impact),
        "footstep_right_large" => (1, FootAction::Impact),
        "soundfootstart_left" => (0, FootAction::Sound("zmb_robot_leg_move_left")),
        "soundfootstart_right" => (1, FootAction::Sound("zmb_robot_leg_move_right")),
        "soundfootwarning_left" => (0, FootAction::Sound("zmb_robot_foot_alarm")),
        "soundfootwarning_right" => (1, FootAction::Sound("zmb_robot_foot_alarm")),
        "soundfootdown_left" => (0, FootAction::Sound("zmb_robot_leg_whoosh")),
        "soundfootdown_right" => (1, FootAction::Sound("zmb_robot_leg_whoosh")),
        "soundfootalarm_left" => (0, FootAction::Sound("zmb_robot_pre_stomp_a")),
        "soundfootalarm_right" => (1, FootAction::Sound("zmb_robot_pre_stomp_a")),
        _ => return None,
    })
}
#[derive(Component)]
#[require(crate::animation::SceneClips)]
struct Foot {
    robot: usize,
    side: usize,
    kill: Collider,
    active: bool,
    finish: f32,
    poll: f32,
    entered: bool,
    light: Entity,
    fx: Option<Entity>,
    phase: HatchState,
    clips: [usize; 3],
    impact: Entity,
}
#[derive(Component)]
struct FootTarget(Entity);
#[derive(Component)]
struct FootSocket(Entity);
// Additional membership lets character movement recognize the existing solid foot clips.
pub(crate) const FOOT_COLLISION_LAYER: u32 = 1024;
#[derive(Component)]
struct SafeSpot;
#[derive(Component)]
pub(crate) struct Evacuation(usize);
#[derive(Component)]
#[require(crate::animation::SceneClips)]
struct RobotBody {
    index: usize,
    feet: [Option<Entity>; 2],
    clips: [usize; 4],
    handles: [Handle<AnimationClip>; 4],
    warnings: [WarningFoot; 2],
}
struct WarningFoot {
    path: Vec<source::Footprint>,
    cursor: usize,
    effects: Vec<Entity>,
    phase: Option<WarningPhase>,
}
enum WarningPhase {
    Landed(f32),
    Dark(f32),
}
impl WarningFoot {
    fn stop(&mut self, commands: &mut Commands) {
        for effect in self.effects.drain(..) {
            crate::native_fx::stop(commands, effect);
        }
    }
    fn show(&mut self, commands: &mut Commands) {
        if let Some(footprint) = self.path.get(self.cursor) {
            self.effects.extend(footprint.lights.iter().map(|pose| {
                crate::native_fx::emit(
                    commands,
                    "maps/zombie_tomb/fx_tomb_foot_warning_light_red",
                    *pose,
                )
            }));
        }
    }
}
#[derive(Component)]
struct LandingReservation {
    point: Vec3,
    remaining: f32,
}
#[derive(Component)]
struct RobotShake {
    remaining: f32,
    robot: usize,
    origin: Vec3,
    opening: bool,
}
#[derive(Component)]
pub(crate) struct StompReaction {
    remaining: f32,
    pub fade: f32,
    pub black: bool,
}
#[derive(Clone, Copy)]
enum TransitPhase {
    Enter,
    Tube,
    Lift,
    FallingWait,
    Fall,
    Landing,
}
#[derive(Component)]
pub(crate) struct RobotTransit {
    phase: TransitPhase,
    remaining: f32,
    from: Vec3,
    occupancy: Occupancy,
    started: bool,
    steam: Option<(Vec3, f32)>,
    effects: Vec<Entity>,
    pub fade: f32,
    pub black: bool,
}
pub struct RobotPlugin;
pub(crate) fn reset_session(world: &mut World) {
    world.insert_resource(Robots::default());
    world.resource_mut::<Messages<FootEvent>>().clear();
}
impl Plugin for RobotPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Robots>()
            .add_message::<RobotStomp>()
            .add_message::<FootEvent>()
            .add_observer(ready)
            .add_systems(
                PostUpdate,
                follow_sockets
                    .in_set(crate::animation::AnimationPoseModifiers)
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                OnEnter(GameState::StartingSession),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (tick, foot_events, foot_presentation, shake)
                    .chain()
                    .in_set(GameStep::Combat)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                reactions
                    .before(GameStep::Movement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                transit
                    .in_set(GameStep::Movement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (purge, evacuate)
                    .chain()
                    .in_set(GameStep::Interactions)
                    .after(crate::portals::interact)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn pass(delay: f32, eligible: bool) -> RobotPass {
    RobotPass {
        delay,
        step: 0,
        clock: 0.,
        eligible: eligible.then_some(2),
        hatch: HatchState::Closed,
        age: 0.,
        ready_at: None,
        can_open: true,
        hatch_age: 1.,
        intro: false,
    }
}
fn setup(
    mut commands: Commands,
    mut robots: ResMut<Robots>,
    mut parts: ResMut<crate::environment::parts::Parts>,
    mut access: ResMut<crate::access::ZoneAccess>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    options: Res<crate::assets::LaunchOptions>,
) {
    *robots = default();
    let mut initial = pass(0., false);
    initial.intro = true;
    robots.passes[1] = Some(initial);
    let bytes = std::fs::read(
        options.asset_root.join(
            game.entity_metadata
                .as_ref()
                .expect("Robot source entity metadata"),
        ),
    )
    .expect("Read robot source entity metadata");
    let paths = source::warning_paths(&bytes);
    let mut paths = paths.into_iter();
    for i in 0..3 {
        let warnings = paths.next().unwrap().map(|path| WarningFoot {
            path,
            cursor: 0,
            effects: Vec::new(),
            phase: None,
        });
        let skeleton = format!("actor.veh_t6_dlc_zm_robot_{i}");
        let clips = std::array::from_fn(|phase| {
            let name = if phase == 3 {
                "ai_zombie_giant_robot_bunker_intro".to_owned()
            } else {
                walk_clip(i, phase)
            };
            game.content
                .animations
                .iter()
                .position(|a| {
                    a.skeleton.0 == skeleton && a.id.0.rsplit('.').next() == Some(name.as_str())
                })
                .unwrap_or_else(|| panic!("Required robot {i} source clip {name}"))
        });
        let handles = clips.map(|index| server.load(game.content.animations[index].path.clone()));
        if let Some(scene) = crate::assets::native_scene(&game, &format!("veh_t6_dlc_zm_robot_{i}"))
        {
            commands.spawn((
                SessionEntity,
                RobotBody {
                    index: i,
                    feet: [None; 2],
                    clips,
                    handles,
                    warnings,
                },
                WorldAssetRoot(
                    server
                        .load_builder()
                        .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                            settings.load_animations = false;
                        })
                        .load(scene.clone()),
                ),
                Transform::from_translation(Vec3::from_array(source::ALIGNMENT[i])),
                Visibility::Hidden,
                RigidBody::Kinematic,
            ));
        }
        for tube in source::TUBES[i] {
            commands.spawn((
                SessionEntity,
                Evacuation(i),
                Transform::from_translation(Vec3::from_array(*tube) + Vec3::Y * 1.016),
            ));
        }
    }
    commands.spawn((
        SessionEntity,
        SafeSpot,
        Collider::cuboid(80. * 0.0254, 150. * 0.0254, 64. * 0.0254),
        Sensor,
        CollisionLayers::new(crate::environment::triggers::TRIGGER_LAYER, LayerMask::NONE),
        Transform::from_translation(Vec3::new(-493., 389., 198.) * 0.0254),
    ));
    access.activated.extend(
        game.content
            .zones
            .iter()
            .filter(|z| z.id.0 == "zone.robot_head")
            .map(|z| z.id.clone()),
    );
    access.refresh(&game.content);
    for (id, point) in [
        ("upper_staff", [-171.628562, 4.8895, 170.53687]),
        ("middle_staff", [-157.954218, 4.918964, 170.675554]),
        ("lower_staff", [-144.915128, 4.91617, 170.575986]),
    ] {
        crate::crafting::reveal_part(
            &mut commands,
            &mut parts,
            &format!("part.elemental_staff_air.{id}"),
            Vec3::from_array(point),
        );
    }
}
fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut bodies: Query<&mut RobotBody>,
    feet: Query<&Foot>,
    tags: Query<&Name>,
    surfaces: Query<&Mesh3d>,
    meshes: Res<Assets<Mesh>>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    spawner: Res<WorldInstanceSpawner>,
) {
    if let Ok(mut body) = bodies.get_mut(event.entity) {
        for side in 0..2 {
            let tag = if side == 0 {
                "tag_attach_hatch_le"
            } else {
                "tag_attach_hatch_ri"
            };
            let Some(socket) = spawner.iter_instance_entities(event.instance_id).find(|e| {
                tags.get(*e)
                    .is_ok_and(|name| name.as_str().eq_ignore_ascii_case(tag))
            }) else {
                panic!("Robot {} required socket {tag} missing", body.index);
            };
            let impact_tag = if side == 0 {
                "tag_hatch_fx_le"
            } else {
                "tag_hatch_fx_ri"
            };
            let impact = spawner
                .iter_instance_entities(event.instance_id)
                .find(|e| {
                    tags.get(*e)
                        .is_ok_and(|name| name.as_str().eq_ignore_ascii_case(impact_tag))
                })
                .unwrap_or_else(|| {
                    panic!("Robot {} required socket {impact_tag} missing", body.index)
                });
            if let Some(root) = body.feet[side] {
                commands.entity(root).insert(FootSocket(socket));
                continue;
            }
            let Some(scene) = crate::assets::native_scene(&game, "veh_t6_dlc_zm_robot_foot_hatch")
            else {
                continue;
            };
            let side_name = if side == 0 { "left" } else { "right" };
            let Some(kill) = game
                .environment
                .triggers
                .iter()
                .find(|t| {
                    t.source.get("targetname").is_some_and(|n| {
                        n == &format!("trig_stomp_kill_{side_name}_{}", body.index)
                    })
                })
                .and_then(|t| t.collider())
            else {
                continue;
            };
            let root = commands.spawn_empty().id();
            body.feet[side] = Some(root);
            let light = commands
                .spawn((
                    SessionEntity,
                    Transform::IDENTITY,
                    Visibility::Hidden,
                    ChildOf(root),
                ))
                .id();
            if let Some(scene) =
                crate::assets::native_scene(&game, "veh_t6_dlc_zm_robot_foot_hatch_lights")
            {
                commands.entity(light).insert(WorldAssetRoot(
                    server
                        .load_builder()
                        .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                            settings.load_animations = false;
                        })
                        .load(scene.clone()),
                ));
            }
            commands.entity(root).insert((
                SessionEntity,
                Foot {
                    robot: body.index,
                    side,
                    kill,
                    active: false,
                    finish: 0.,
                    poll: 0.,
                    entered: false,
                    light,
                    fx: None,
                    phase: HatchState::Closed,
                    clips: [
                        "ai_zombie_giant_robot_hatch_open",
                        "ai_zombie_giant_robot_hatch_open_idle",
                        "ai_zombie_giant_robot_hatch_close",
                    ]
                    .map(|name| {
                        game.content
                            .animations
                            .iter()
                            .position(|a| {
                                a.skeleton.0 == "actor.veh_t6_dlc_zm_robot_foot_hatch"
                                    && a.id.0.rsplit('.').next() == Some(name)
                            })
                            .unwrap_or_else(|| panic!("Required robot hatch clip {name}"))
                    }),
                    impact,
                },
                WorldAssetRoot(
                    server
                        .load_builder()
                        .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                            settings.load_animations = false;
                        })
                        .load(scene.clone()),
                ),
                FootSocket(socket),
                Transform::IDENTITY,
                Visibility::Inherited,
                ChildOf(event.entity),
                DamageEmitter::Robot(format!("robot.{}", body.index)),
                ColliderDisabled,
            ));
            if let Some(shape) = game
                .environment
                .brush_entities
                .iter()
                .find(|b| {
                    b.source
                        .get("targetname")
                        .is_some_and(|n| n == &format!("clip_foot_{side_name}_{}", body.index))
                })
                .and_then(|b| b.collider())
            {
                commands.entity(root).insert((
                    shape,
                    CollisionLayers::new(
                        1u32 | crate::assets::DYNENT_WORLD_LAYER | FOOT_COLLISION_LAYER,
                        LayerMask::ALL,
                    ),
                ));
            }
        }
    } else if feet.contains(event.entity) {
        for mesh in spawner.iter_instance_entities(event.instance_id) {
            if let Ok(handle) = surfaces.get(mesh)
                && let Some(shape) = meshes.get(&handle.0).and_then(Collider::trimesh_from_mesh)
            {
                commands.entity(mesh).insert((
                    shape,
                    Sensor,
                    FootTarget(event.entity),
                    EnvironmentOwner::Robot,
                    CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE),
                ));
            }
        }
    }
}
fn follow_sockets(
    mut poses: ParamSet<(
        Query<(&Transform, Option<&ChildOf>)>,
        Query<(Entity, &FootSocket, &ChildOf, &mut Transform)>,
    )>,
) {
    let attachments: Vec<_> = poses
        .p1()
        .iter()
        .map(|(entity, socket, parent, _)| (entity, socket.0, parent.parent()))
        .collect();
    let transforms = poses.p0();
    let followed: Vec<_> = attachments
        .into_iter()
        .filter_map(|(entity, socket, parent)| {
            let mut cursor = socket;
            let mut local = Mat4::IDENTITY;
            while cursor != parent {
                let Ok((pose, Some(ancestor))) = transforms.get(cursor) else {
                    return None;
                };
                local = pose.to_matrix() * local;
                cursor = ancestor.parent();
            }
            Some((entity, Transform::from_matrix(local).with_scale(Vec3::ONE)))
        })
        .collect();
    for (entity, followed) in followed {
        if let Ok((_, _, _, mut pose)) = poses.p1().get_mut(entity) {
            pose.set_if_neq(followed);
        }
    }
}
fn walk_clip(robot: usize, step: usize) -> String {
    format!(
        "ai_zombie_giant_robot_walk_{}{}",
        ["nml", "trenches", "village"][robot],
        ["_intro", "", "_outtro"][step]
    )
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut robots: ResMut<Robots>,
    mut session: ResMut<Session>,
    access: Res<crate::access::ZoneAccess>,
    quest: Res<MainQuest>,
    game: Res<GameDefinition>,
    mut assets: crate::animation::ClipAssets,
    mut animated: Query<(
        &mut crate::animation::SceneClips,
        &mut AnimationPlayer,
        &mut AnimationGraphHandle,
    )>,
    mut bodies: Query<(Entity, &mut RobotBody, &mut Transform)>,
    feet: Query<&Foot>,
    targets: Query<&FootTarget>,
    mut hits: MessageReader<EnvironmentalHit>,
    mut events: MessageWriter<FootEvent>,
    globals: Query<&GlobalTransform>,
) {
    let dt = time.delta_secs();
    robots.foot_activation_delay = (robots.foot_activation_delay - dt).max(0.);
    for hit in hits.read() {
        if hit.amount == 0 {
            continue;
        }
        let target = targets.get(hit.target).map_or(hit.target, |t| t.0);
        let Ok(foot) = feet.get(target) else {
            continue;
        };
        let Some(p) = robots.passes[foot.robot].as_mut() else {
            continue;
        };
        if p.delay <= 0.
            && p.eligible == Some(foot.side)
            && p.can_open
            && p.ready_at.is_some_and(|t| p.age >= t)
            && p.hatch == HatchState::Closed
        {
            p.hatch = HatchState::Open;
            p.hatch_age = 0.;
            p.can_open = false;
        }
    }
    if robots.passes.iter().all(Option::is_none) {
        robots.next -= dt;
        if robots.next <= 0. {
            robots.triple_active = false;
            let triple = session.round % 4 == 0 && robots.triple_round != session.round
                || quest.enabled && quest.stage == QuestStage::BreakSeal;
            if triple {
                robots.triple_active = true;
                let eligible = session.random() as usize % 3;
                robots.passes[2] = Some(pass(10., eligible == 2));
                robots.passes[0] = Some(pass(15., eligible == 0));
                robots.passes[1] = Some(pass(20., eligible == 1));
                robots.triple_round = session.round;
                robots.last = None;
            } else {
                let nml = access
                    .open
                    .iter()
                    .any(|id| id.0.contains("activate_zone_nml"));
                let index = if !nml {
                    session.random() as usize % 2
                } else {
                    loop {
                        let candidate = session.random() as usize % 3;
                        if robots.last != Some(candidate) {
                            break candidate;
                        }
                    }
                };
                robots.passes[index] = Some(pass(10., true));
                robots.last = Some(index);
            }
            robots.next = 5.;
        }
    }
    for (robot_entity, mut body, mut pose) in &mut bodies {
        let Some(mut p) = robots.passes[body.index] else {
            commands.entity(robot_entity).insert(Visibility::Hidden);
            for warning in &mut body.warnings {
                warning.stop(&mut commands);
                warning.cursor = 0;
                warning.phase = None;
            }
            continue;
        };
        if p.delay > 0. {
            p.delay = (p.delay - dt).max(0.);
            if p.delay == 0. && p.eligible == Some(2) {
                p.eligible = Some(session.random() as usize % 2);
            }
            robots.passes[body.index] = Some(p);
            continue;
        }
        if p.eligible == Some(2) {
            p.eligible = Some(session.random() as usize % 2);
        }
        let phase = if p.intro { 3 } else { p.step };
        let animation = &game.content.animations[body.clips[phase]];
        let end = animation.seconds;
        if !assets.clips.contains(&body.handles[phase]) || body.feet.iter().any(Option::is_none) {
            continue;
        }
        let Ok((mut clips, mut player, mut graph)) = animated.get_mut(robot_entity) else {
            continue;
        };
        if !clips.sample(
            &animation.path,
            (p.clock + dt).min(end),
            false,
            &mut player,
            &mut graph,
            &mut assets,
        ) {
            continue;
        }
        let previous = p.clock;
        if !p.intro && p.step == 0 && previous == 0. {
            for warning in &mut body.warnings {
                warning.show(&mut commands);
            }
        }
        for side in 0..2 {
            let point = body.feet[side]
                .and_then(|e| feet.get(e).ok())
                .and_then(|foot| globals.get(foot.impact).ok())
                .map(GlobalTransform::translation);
            let warning = &mut body.warnings[side];
            if let Some(phase) = warning.phase.take() {
                warning.phase = match phase {
                    WarningPhase::Landed(t) if t > dt => Some(WarningPhase::Landed(t - dt)),
                    WarningPhase::Landed(_) => {
                        if warning.path.get(warning.cursor).is_some_and(|mark| {
                            point.is_some_and(|p| {
                                p.distance_squared(mark.point) < (300_f32 * 0.0254).powi(2)
                            })
                        }) {
                            warning.stop(&mut commands);
                            warning.cursor += 1;
                            Some(WarningPhase::Dark(0.05))
                        } else {
                            None
                        }
                    }
                    WarningPhase::Dark(t) if t > dt => Some(WarningPhase::Dark(t - dt)),
                    WarningPhase::Dark(_) => {
                        warning.show(&mut commands);
                        None
                    }
                };
            }
        }
        p.clock += dt;
        p.age += dt;
        p.hatch_age += dt;
        if let HatchState::Closing(t) = p.hatch {
            p.hatch = if t <= dt {
                p.hatch_age = 0.;
                HatchState::Closed
            } else {
                HatchState::Closing(t - dt)
            };
        }
        commands.entity(robot_entity).insert(Visibility::Inherited);
        let motion = animation
            .root_motion
            .as_ref()
            .expect("Required robot scripted root motion");
        pose.translation = Vec3::from_array(source::ALIGNMENT[body.index])
            + Vec3::from_array(motion.translation_at(p.clock));
        pose.rotation = Quat::from_array(motion.rotation_at(p.clock));
        if p.intro && previous < 0.5 && p.clock >= 0.5 {
            events.write(FootEvent {
                robot: body.index,
                side: 0,
                action: FootAction::OpeningImpact,
            });
        }
        for note in animation
            .notifications
            .iter()
            .filter(|n| n.seconds >= previous && n.seconds < p.clock)
        {
            if note.event == "kill_zombies_leftfoot_1" && p.ready_at.is_none() {
                p.ready_at = Some(p.age + 1.);
            }
            if let Some((side, action)) = foot_action(&note.event) {
                if !p.intro && matches!(action, FootAction::Impact) {
                    body.warnings[side].phase = Some(WarningPhase::Landed(1.));
                }
                events.write(FootEvent {
                    robot: body.index,
                    side,
                    action,
                });
            }
        }
        if p.clock >= end {
            p.step += 1;
            p.clock = 0.;
            if p.intro || p.step >= 3 {
                if p.intro {
                    robots.next = 0.;
                }
                robots.passes[body.index] = None;
                commands.entity(robot_entity).insert(Visibility::Hidden);
                continue;
            }
        }
        robots.passes[body.index] = Some(p);
    }
}
fn foot_events(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut robots: ResMut<Robots>,
    spatial: SpatialQuery,
    mut events: MessageReader<FootEvent>,
    mut feet: Query<(Entity, &mut Foot, &GlobalTransform)>,
    safe: Query<(&Collider, &GlobalTransform), With<SafeSpot>>,
    mut players: Query<
        (
            Entity,
            &mut Player,
            &mut Transform,
            &Vitality,
            &mut crate::movement::Mobility,
            &crate::environment::digging::Digger,
            Has<StompReaction>,
        ),
        Without<Foot>,
    >,
    enemies: Query<
        (Entity, &Transform, &Vitality, Has<crate::panzer::Panzer>),
        (With<crate::enemies::Zombie>, Without<Player>, Without<Foot>),
    >,
    mut damage: ResMut<DamageQueue>,
    mut stomps: MessageWriter<RobotStomp>,
    mut rumble: MessageWriter<crate::actions::Rumble>,
    globals: Query<&GlobalTransform>,
    zone: Res<crate::access::CurrentZone>,
) {
    let dt = time.delta_secs();
    for event in events.read() {
        let Some((entity, mut foot, foot_pose)) = feet
            .iter_mut()
            .find(|(_, f, _)| f.robot == event.robot && f.side == event.side)
        else {
            continue;
        };
        let impact = globals.get(foot.impact).expect("Robot foot impact socket");
        let point = impact.translation();
        if matches!(event.action, FootAction::Kill) {
            foot.active = true;
            foot.finish = 0.;
            foot.poll = 0.;
        } else if matches!(event.action, FootAction::Impact | FootAction::OpeningImpact) {
            foot.finish = 0.5;
            robots.stomps += 1;
            stomps.write(RobotStomp(point));
            crate::native_fx::emit(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_robot_dust",
                impact.compute_transform(),
            );
            crate::audio::emit(
                &mut commands,
                "zmb_robot_foot_impact",
                crate::audio::SoundSource::World(point),
            );
            let opening = matches!(event.action, FootAction::OpeningImpact);
            commands.spawn((
                SessionEntity,
                RobotShake {
                    robot: event.robot,
                    origin: foot_pose.translation() + Vec3::Y * 72. * 0.0254,
                    remaining: if opening { 0. } else { 0.2 },
                    opening,
                },
            ));
        } else {
            if let FootAction::Sound(alias) = event.action {
                crate::audio::emit(
                    &mut commands,
                    alias,
                    crate::audio::SoundSource::attached(entity),
                );
            }
        }
    }
    for (entity, mut foot, pose) in &mut feet {
        if robots.passes[foot.robot].is_none() {
            foot.active = false;
            continue;
        }
        if foot.finish > 0. {
            foot.finish = (foot.finish - dt).max(0.);
            if foot.finish == 0. {
                foot.active = false;
                if foot.entered {
                    foot.entered = false;
                    if let Some(p) = robots.passes[foot.robot].as_mut() {
                        p.hatch = HatchState::Closing(5.);
                    }
                }
            }
        }
        if !foot.active || robots.foot_activation_delay > 0. {
            continue;
        }
        foot.poll -= dt;
        if foot.poll > 0. {
            continue;
        }
        foot.poll += 0.05;
        let pose = pose.compute_transform();
        let center = pose.translation + Vec3::Y * 1.8288;
        let touching = spatial.shape_intersections(
            &foot.kill,
            center,
            pose.rotation,
            &SpatialQueryFilter::default(),
        );
        for (owner, mut player, mut body, health, mut mobility, digger, stomped) in &mut players {
            if !matches!(health.state(), LifeState::Standing | LifeState::Downed)
                || robots.occupancy.is_some()
                || stomped
                || !touching.contains(&owner)
            {
                continue;
            }
            if safe.iter().any(|(shape, pose)| {
                let pose = pose.compute_transform();
                spatial
                    .shape_intersections(
                        shape,
                        pose.translation,
                        pose.rotation,
                        &SpatialQueryFilter::default(),
                    )
                    .contains(&owner)
            }) {
                continue;
            }
            let p = robots.passes[foot.robot].as_mut().unwrap();
            if matches!(
                p.hatch,
                HatchState::Open | HatchState::Entered | HatchState::Closing(_)
            ) && p.eligible == Some(foot.side)
                && health.state() == LifeState::Standing
            {
                let occupancy = Occupancy {
                    player: owner,
                    robot: foot.robot,
                    remaining: f32::INFINITY,
                    exit: body.translation - Vec3::Y * player.height * 0.5,
                    tank_bunker: foot.robot == 1
                        && zone.0.as_ref().is_some_and(|z| {
                            matches!(z.0.as_str(), "zone.bunker_4d" | "zone.bunker_4c")
                        }),
                    yaw: player.yaw,
                    pitch: player.pitch,
                };
                p.hatch = HatchState::Entered;
                foot.entered = true;
                commands.entity(owner).insert((
                    RobotTransit {
                        phase: TransitPhase::Enter,
                        remaining: 4.,
                        from: body.translation,
                        occupancy,
                        started: true,
                        steam: None,
                        effects: Vec::new(),
                        fade: 1.,
                        black: false,
                    },
                    crate::actions::HandReservation::held(crate::actions::HandAction::Portal),
                    ColliderDisabled,
                ));
                crate::portals::relocate(
                    &mut commands,
                    owner,
                    &mut player,
                    &mut body,
                    &mut mobility,
                    Vec3::new(-71.5264, -24.1808, 60.96),
                );
                crate::audio::emit(
                    &mut commands,
                    "zmb_bot_elevator_ride_up",
                    crate::audio::SoundSource::attached(owner),
                );
            } else {
                commands.entity(owner).insert((
                    StompReaction {
                        remaining: 5.,
                        fade: 1.,
                        black: true,
                    },
                    crate::actions::HandReservation::timed(crate::actions::HandAction::Portal, 5.),
                ));
                player.velocity = Vec3::ZERO;
                if digger.helmet {
                    prone(
                        &mut commands,
                        owner,
                        &mut player,
                        &mut body,
                        &mut mobility,
                        &game,
                    );
                    rumble.write(crate::actions::Rumble {
                        origin: body.translation,
                        radius: 1.,
                        seconds: 7.,
                        strength: 2.,
                    });
                }
                crate::audio::emit(
                    &mut commands,
                    "zmb_zombie_arc",
                    crate::audio::SoundSource::attached(owner),
                );
                damage.0.push(Impact {
                    point: None,
                    attacker: entity,
                    target: owner,
                    source: crate::damage::ImpactSource::Robot(format!("robot.{}", foot.robot)),
                    category: crate::origins::DamageCategory::Crush,
                    region: crate::weapons::HitRegion::Torso,
                    amount: 999999,
                });
            }
        }
        for (target, _, health, panzer) in &enemies {
            if health.state() == LifeState::ActorAlive && touching.contains(&target) {
                if panzer {
                    commands
                        .entity(target)
                        .insert(crate::panzer::KnockdownRequest(
                            crate::panzer::KnockdownKind::Robot,
                        ));
                    continue;
                }
                damage.0.push(Impact {
                    point: None,
                    attacker: entity,
                    target,
                    source: crate::damage::ImpactSource::Robot(format!("robot.{}", foot.robot)),
                    category: crate::origins::DamageCategory::Crush,
                    region: crate::weapons::HitRegion::Torso,
                    amount: health.maximum(),
                });
            }
        }
    }
}
fn shake(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    robots: Res<Robots>,
    mut shakes: Query<(Entity, &mut RobotShake)>,
    players: Query<(Entity, &Transform, &Vitality, Has<RobotTransit>), With<Player>>,
    mut rumble: MessageWriter<crate::actions::Rumble>,
) {
    for (entity, mut shake) in &mut shakes {
        shake.remaining -= time.delta_secs();
        if shake.remaining > 0. {
            continue;
        }
        for (player, pose, health, transit) in &players {
            if health.state() != LifeState::Standing || transit {
                continue;
            }
            let band = if shake.opening {
                Some(3)
            } else if let Some(occupancy) = robots.occupancy.filter(|o| o.player == player) {
                (occupancy.robot == shake.robot).then_some(2)
            } else {
                let distance = pose.translation.distance(shake.origin) / 0.0254;
                if distance < 1500. {
                    Some(3)
                } else if distance < 3000. {
                    Some(2)
                } else if distance < 6000. {
                    Some(1)
                } else {
                    None
                }
            };
            if let Some(band) = band {
                rumble.write(crate::actions::Rumble {
                    origin: pose.translation,
                    radius: 100. * 0.0254,
                    seconds: if band == 1 { 1. } else { 1.5 },
                    strength: [0., 0.1, 0.3, 0.6][band],
                });
            }
        }
        commands.entity(entity).despawn();
    }
}
fn foot_presentation(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    robots: Res<Robots>,
    game: Res<GameDefinition>,
    mut assets: crate::animation::ClipAssets,
    mut animated: Query<(
        &mut crate::animation::SceneClips,
        &mut AnimationPlayer,
        &mut AnimationGraphHandle,
    )>,
    mut feet: Query<(Entity, &mut Foot, &GlobalTransform)>,
    targets: Query<(Entity, &FootTarget)>,
    mut lights: Query<&mut Visibility, Without<Foot>>,
) {
    for (entity, mut foot, pose) in &mut feet {
        let pass = robots.passes[foot.robot];
        let walking = pass.is_some_and(|p| p.delay <= 0.);
        let selected = pass.is_some_and(|p| p.delay <= 0. && p.eligible == Some(foot.side));
        let phase = pass.map_or(HatchState::Closed, |p| {
            if selected {
                p.hatch
            } else {
                HatchState::Closed
            }
        });
        let open = !matches!(phase, HatchState::Closed);
        if walking && robots.foot_activation_delay == 0. {
            commands.entity(entity).remove::<ColliderDisabled>();
        } else {
            commands.entity(entity).insert(ColliderDisabled);
        }
        for (target, owner) in &targets {
            if owner.0 != entity {
                continue;
            }
            if selected && !open {
                commands.entity(target).insert(ShootableEnvironment);
            } else {
                commands.entity(target).remove::<ShootableEnvironment>();
            }
        }
        if let Ok(mut visibility) = lights.get_mut(foot.light) {
            *visibility = if selected && pass.is_some_and(|p| p.can_open || open) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if open && foot.fx.is_none() {
            foot.fx = Some(crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_mech_death",
                entity,
                Transform::from_translation(pose.rotation().inverse() * Vec3::Y * -1.4224),
            ));
            crate::audio::emit(
                &mut commands,
                "zmb_zombieblood_3rd_plane_explode",
                crate::audio::SoundSource::attached(entity),
            );
        } else if !open && let Some(fx) = foot.fx.take() {
            commands.entity(fx).try_despawn();
        }
        let age = pass.map_or(1., |p| p.hatch_age);
        let opening = &game.content.animations[foot.clips[0]];
        let clip = if open && age < opening.seconds {
            0
        } else if open {
            1
        } else {
            2
        };
        let animation = &game.content.animations[foot.clips[clip]];
        {
            if let Ok((mut clips, mut player, mut graph)) = animated.get_mut(entity) {
                clips.sample(
                    &animation.path,
                    if clip == 1 {
                        age - opening.seconds
                    } else {
                        age.min(animation.seconds)
                    },
                    clip == 1,
                    &mut player,
                    &mut graph,
                    &mut assets,
                );
            }
        }
        foot.phase = phase;
        let _ = time;
    }
}
fn prone(
    commands: &mut Commands,
    owner: Entity,
    player: &mut Player,
    pose: &mut Transform,
    mobility: &mut crate::movement::Mobility,
    game: &GameDefinition,
) {
    let height = game.tuning.mobility.prone_height;
    pose.translation.y += (height - player.height) * 0.5;
    player.height = height;
    player.eye_height = game.tuning.mobility.prone_eye;
    player.prone = true;
    player.crouched = true;
    mobility.stance = crate::movement::Stance::Prone;
    commands
        .entity(owner)
        .insert(crate::player::capsule(game.tuning.radius, height));
}
fn reactions(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut reactions: Query<(Entity, &Vitality, &mut StompReaction)>,
) {
    for (entity, health, mut reaction) in &mut reactions {
        reaction.remaining = (reaction.remaining - time.delta_secs()).max(0.);
        reaction.fade = (reaction.remaining / 0.2).clamp(0., 1.);
        if reaction.remaining == 0. || health.state() == LifeState::HumanTerminal {
            commands.entity(entity).remove::<StompReaction>();
        }
    }
}
fn start_eject(
    commands: &mut Commands,
    owner: Entity,
    pose: Vec3,
    occupancy: Occupancy,
    manual: bool,
) {
    commands.entity(owner).insert((
        RobotTransit {
            phase: if manual {
                TransitPhase::Tube
            } else {
                TransitPhase::Lift
            },
            remaining: if manual { 3. } else { 1. },
            from: pose,
            occupancy,
            started: false,
            steam: None,
            effects: Vec::new(),
            fade: 0.,
            black: false,
        },
        crate::actions::HandReservation::held(crate::actions::HandAction::Portal),
        ColliderDisabled,
    ));
    crate::audio::emit(
        commands,
        "zmb_bot_timeout_alarm",
        crate::audio::SoundSource::attached(owner),
    );
    if !manual {
        crate::audio::emit(
            commands,
            "zmb_giantrobot_exit",
            crate::audio::SoundSource::attached(owner),
        );
    }
}
fn purge(
    mut commands: Commands,
    mut robots: ResMut<Robots>,
    mut events: MessageReader<crate::audio::RobotPurgeReady>,
    players: Query<&Transform, With<Player>>,
    busy: Query<(), With<RobotTransit>>,
) {
    for event in events.read() {
        if let Some(occupancy) = robots.occupancy
            && occupancy.player == event.player
            && occupancy.robot == event.robot
            && !busy.contains(event.player)
            && let Ok(pose) = players.get(event.player)
        {
            start_eject(
                &mut commands,
                event.player,
                pose.translation,
                occupancy,
                false,
            );
            robots.occupancy = None;
        }
    }
}
fn select_landing(
    spatial: &SpatialQuery,
    owner: Entity,
    player: &Player,
    occupancy: &Occupancy,
    reservations: &Query<&LandingReservation>,
    game: &GameDefinition,
) -> Option<Vec3> {
    let shape = crate::player::capsule(game.tuning.radius, player.height);
    let filter = crate::environment::player_solids([owner]);
    let clear = |feet: Vec3| {
        !reservations
            .iter()
            .any(|r| r.point.distance_squared(feet) < 1.5_f32.powi(2))
            && spatial
                .shape_intersections(
                    &shape,
                    feet + Vec3::Y * (player.height * 0.5 + 0.025),
                    Quat::IDENTITY,
                    &filter,
                )
                .is_empty()
    };
    if occupancy.tank_bunker {
        return source::TANK_LANDINGS
            .iter()
            .copied()
            .map(Vec3::from_array)
            .find(|p| clear(*p));
    }
    let footprint = source::LANDINGS
        .iter()
        .copied()
        .map(Vec3::from_array)
        .min_by(|a, b| {
            a.distance_squared(occupancy.exit)
                .total_cmp(&b.distance_squared(occupancy.exit))
        })?;
    for (x, z) in [
        (0., 0.),
        (1., -1.),
        (1., 0.),
        (1., 1.),
        (0., 1.),
        (-1., 1.),
        (-1., 0.),
        (-1., -1.),
        (0., -1.),
    ] {
        let candidate = footprint + Vec3::new(x, 0., z) * 1.27;
        let start = candidate + Vec3::Y * (2.54 + player.height * 0.5);
        let Some(hit) = spatial.cast_shape(
            &shape,
            start,
            Quat::IDENTITY,
            Dir3::NEG_Y,
            &ShapeCastConfig {
                ignore_origin_penetration: true,
                ..ShapeCastConfig::from_max_distance(2.8)
            },
            &filter,
        ) else {
            continue;
        };
        let feet = start - Vec3::Y * (hit.distance + player.height * 0.5);
        if hit.normal1.y > 0.5 && clear(feet) {
            return Some(feet);
        }
    }
    None
}
fn transit(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    spatial: SpatialQuery,
    mut robots: ResMut<Robots>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Vitality,
        &mut crate::movement::Mobility,
        &mut RobotTransit,
    )>,
    mut reservations: ParamSet<(
        Query<&LandingReservation>,
        Query<(Entity, &mut LandingReservation)>,
    )>,
    mut rumble: MessageWriter<crate::actions::Rumble>,
) {
    let dt = time.delta_secs();
    for (entity, mut reservation) in &mut reservations.p1() {
        reservation.remaining -= dt;
        if reservation.remaining <= 0. {
            commands.entity(entity).despawn();
        }
    }
    for (owner, mut player, mut pose, health, mut mobility, mut travel) in &mut players {
        if health.state() == LifeState::HumanTerminal {
            for effect in travel.effects.drain(..) {
                commands.entity(effect).try_despawn();
            }
            commands.entity(owner).remove::<(
                RobotTransit,
                crate::actions::HandReservation,
                ColliderDisabled,
            )>();
            continue;
        }
        if !travel.started {
            travel.started = true;
            let height = game.tuning.standing_height;
            player.height = height;
            player.eye_height = game.tuning.eye_height;
            player.prone = false;
            player.crouched = false;
            mobility.stance = crate::movement::Stance::Standing;
            commands
                .entity(owner)
                .insert(crate::player::capsule(game.tuning.radius, height));
            let tube = source::TUBES[travel.occupancy.robot]
                .iter()
                .copied()
                .map(Vec3::from_array)
                .min_by(|a, b| {
                    a.distance_squared(pose.translation)
                        .total_cmp(&b.distance_squared(pose.translation))
                })
                .unwrap();
            crate::portals::relocate(
                &mut commands,
                owner,
                &mut player,
                &mut pose,
                &mut mobility,
                tube,
            );
            player.yaw = std::f32::consts::PI;
            player.pitch = 0.;
            travel.from = pose.translation;
            if matches!(travel.phase, TransitPhase::Tube) {
                travel.steam = Some((tube, 0.));
                travel.effects.push(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_bot_timeout_steam",
                    crate::audio::SoundSource::World(tube),
                ));
            }
        }
        if let Some((point, mut poll)) = travel.steam {
            poll -= dt;
            if poll <= 0. {
                poll += 0.1;
                travel.effects.push(crate::native_fx::emit(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_robot_eject_steam",
                    Transform::from_translation(point)
                        .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
                ));
            }
            travel.steam = Some((point, poll));
        }
        player.velocity = Vec3::ZERO;
        travel.remaining = (travel.remaining - dt).max(0.);
        let elapsed = match travel.phase {
            TransitPhase::Enter => 4.,
            TransitPhase::Tube => 3.,
            TransitPhase::Lift | TransitPhase::FallingWait => 1.,
            TransitPhase::Fall => 3.,
            TransitPhase::Landing => 0.,
        } - travel.remaining;
        match travel.phase {
            TransitPhase::Enter => {
                travel.fade = if travel.remaining > 0.2 {
                    1.
                } else {
                    travel.remaining / 0.2
                };
                if travel.remaining == 0. {
                    crate::portals::relocate(
                        &mut commands,
                        owner,
                        &mut player,
                        &mut pose,
                        &mut mobility,
                        Vec3::from_array(source::INTERIORS[travel.occupancy.robot]),
                    );
                    player.yaw = 179.6_f32.to_radians();
                    player.pitch = 0.;
                    robots.occupancy = Some(travel.occupancy);
                    robots.entries += 1;
                    commands.entity(owner).remove::<(
                        RobotTransit,
                        crate::actions::HandReservation,
                        ColliderDisabled,
                    )>();
                }
            }
            TransitPhase::Tube => {
                if travel.remaining == 0. {
                    travel.phase = TransitPhase::Lift;
                    travel.remaining = 1.;
                    travel.from = pose.translation;
                    crate::audio::emit(
                        &mut commands,
                        "zmb_giantrobot_exit",
                        crate::audio::SoundSource::attached(owner),
                    );
                }
            }
            TransitPhase::Lift => {
                pose.translation = travel.from + Vec3::Y * 50.8 * (elapsed / 2.5);
                travel.fade = elapsed.clamp(0., 1.);
                if travel.remaining == 0. {
                    travel.phase = TransitPhase::FallingWait;
                    travel.remaining = 1.;
                    pose.translation =
                        travel.occupancy.exit + Vec3::Y * (76.2 + player.height * 0.5);
                    player.pitch = std::f32::consts::FRAC_PI_2;
                }
            }
            TransitPhase::FallingWait => {
                travel.fade = travel.remaining;
                if travel.remaining == 0. {
                    travel.phase = TransitPhase::Fall;
                    travel.remaining = 3.;
                    travel.from = pose.translation;
                    crate::audio::emit(
                        &mut commands,
                        "zmb_giantrobot_fall",
                        crate::audio::SoundSource::attached(owner),
                    );
                }
            }
            TransitPhase::Fall => {
                pose.translation = travel.from.lerp(
                    travel.occupancy.exit + Vec3::Y * player.height * 0.5,
                    elapsed / 3.,
                );
                travel.fade = ((elapsed - 2.75) / 0.25).clamp(0., 1.);
                travel.black = elapsed > 2.75;
                if travel.remaining == 0. {
                    crate::native_fx::emit(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_beacon_exp",
                        Transform::from_translation(travel.occupancy.exit),
                    );
                    travel.phase = TransitPhase::Landing;
                }
            }
            TransitPhase::Landing => {
                if let Some(point) = select_landing(
                    &spatial,
                    owner,
                    &player,
                    &travel.occupancy,
                    &reservations.p0(),
                    &game,
                ) {
                    commands.spawn((
                        SessionEntity,
                        LandingReservation {
                            point,
                            remaining: 1.,
                        },
                    ));
                    crate::portals::relocate(
                        &mut commands,
                        owner,
                        &mut player,
                        &mut pose,
                        &mut mobility,
                        point,
                    );
                    player.yaw = travel.occupancy.yaw;
                    player.pitch = travel.occupancy.pitch;
                    prone(
                        &mut commands,
                        owner,
                        &mut player,
                        &mut pose,
                        &mut mobility,
                        &game,
                    );
                    rumble.write(crate::actions::Rumble {
                        origin: point,
                        radius: 12.7,
                        seconds: 2.5,
                        strength: 2.,
                    });
                    for effect in travel.effects.drain(..) {
                        commands.entity(effect).try_despawn();
                    }
                    commands.entity(owner).remove::<(
                        RobotTransit,
                        crate::actions::HandReservation,
                        ColliderDisabled,
                    )>();
                    robots.evacuations += 1;
                }
            }
        }
    }
}
pub(crate) fn evacuate(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut robots: ResMut<Robots>,
    mut focus: ResMut<InteractionFocus>,
    exits: Query<(Entity, &Evacuation, &Transform), Without<Player>>,
    players: Query<
        (
            Entity,
            &Transform,
            &Vitality,
            &crate::weapons::WeaponInventory,
            Has<crate::perks::PerkDrinking>,
            Has<crate::actions::HandReservation>,
        ),
        With<Player>,
    >,
) {
    if !input.use_held {
        robots.latched = false;
    }
    let Some(occupancy) = robots.occupancy else {
        return;
    };
    if focus.target.is_some() {
        return;
    }
    let Ok((owner, pose, health, inventory, drinking, reserved)) = players.get(occupancy.player)
    else {
        robots.occupancy = None;
        return;
    };
    if health.state() == LifeState::HumanTerminal {
        robots.occupancy = None;
        return;
    }
    if health.state() != LifeState::Standing {
        return;
    }
    let Some((entity, _, _)) = exits.iter().find(|(_, exit, t)| {
        exit.0 == occupancy.robot
            && t.translation.distance_squared(pose.translation) < 3_f32.powi(2)
    }) else {
        return;
    };
    focus.target = Some(entity);
    focus.progress = 0.;
    focus.hold_seconds = 0.;
    focus.prompt = format!(
        "Press {} to evacuate",
        settings.label(crate::input::Action::Interact)
    );
    if input.use_held
        && !robots.latched
        && !drinking
        && !reserved
        && inventory.action == crate::weapons::WeaponAction::Ready
    {
        robots.latched = true;
        start_eject(&mut commands, owner, pose.translation, occupancy, true);
        robots.occupancy = None;
        focus.accepted += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn robot_system_queries_and_session_reset_are_valid() {
        let mut world = World::new();
        IntoSystem::into_system(tick).initialize(&mut world);
        IntoSystem::into_system(foot_events).initialize(&mut world);
        IntoSystem::into_system(foot_presentation).initialize(&mut world);
        world.insert_resource(Robots::default());
        world.init_resource::<Messages<FootEvent>>();
        world.resource_mut::<Robots>().passes[1] = Some(pass(0., false));
        reset_session(&mut world);
        assert!(
            world
                .resource::<Robots>()
                .passes
                .iter()
                .all(Option::is_none)
        );
    }

    #[test]
    #[ignore = "requires exported installed Origins assets"]
    fn exported_warning_markers_form_all_six_footprint_paths() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets/entities.json");
        let paths = source::warning_paths(&std::fs::read(path).unwrap());
        for route in paths {
            assert!(route.iter().flatten().any(|step| !step.lights.is_empty()));
            for side in route {
                assert!(side.len() > 1);
                assert!(side.iter().all(|step| step.point.is_finite()));
            }
        }
    }
}
