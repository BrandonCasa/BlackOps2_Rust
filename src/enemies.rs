use crate::hit_regions::SkeletalReady as ActorReady;
use crate::{
    GameState,
    animation::{AnimationGraphs, AnimationLayer, AnimationPlayback},
    assets::GameDefinition,
    gameplay::{GameStep, Navigation},
    life::{LifeState, Vitality},
    origins::content::{
        ContentId, EnemyAnimation, EnemyCombat, EnemyCombatClip, EnemyDefinition, EnemyLocomotion,
    },
    player::Player,
};
use avian3d::prelude::*;
use bevy::{
    camera::{
        primitives::Aabb,
        visibility::{DynamicSkinnedMeshBounds, VisibilitySystems},
    },
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
mod eyes;
mod gibs;
mod ragdoll;
pub(crate) use ragdoll::{RagdollBullet, RagdollCollisionHooks, RagdollExplosion};

pub(crate) fn reset(world: &mut World) {
    world.insert_resource(gibs::GibCounter::default());
    if let Some(mut messages) = world.get_resource_mut::<Messages<RagdollBullet>>() {
        messages.clear();
    }
    if let Some(mut messages) = world.get_resource_mut::<Messages<RagdollExplosion>>() {
        messages.clear();
    }
}

#[derive(Component)]
pub struct EnemyGoal {
    pub position: Vec3,
    pub face: Vec3,
    pub radius: f32,
}

pub(crate) fn body_center(enemy: &EnemyDefinition, crawler: bool) -> Vec3 {
    enemy
        .physics
        .map_or(Vec3::Y * if crawler { 0.35 } else { 0.85 }, |physics| {
            physics.center(crawler)
        })
}

pub(crate) fn body_shape(enemy: &EnemyDefinition, crawler: bool) -> Collider {
    enemy.physics.map_or_else(
        || {
            if crawler {
                Collider::capsule(0.3, 0.1)
            } else {
                crate::player::capsule(0.3, 1.7)
            }
        },
        |physics| {
            Collider::cuboid(
                physics.radius * 2.,
                physics.height(crawler),
                physics.radius * 2.,
            )
        },
    )
}

fn native_bounds(
    game: Res<GameDefinition>,
    mut actors: Query<(&Zombie, &Transform, &mut Collider)>,
) {
    for (zombie, pose, mut shape) in &mut actors {
        if let Some(enemy) = game
            .content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
            && enemy.physics.is_some()
        {
            *shape = Collider::compound(vec![(
                Vec3::ZERO,
                pose.rotation.inverse(),
                body_shape(enemy, zombie.crawler),
            )]);
        }
    }
}

#[derive(Component)]
#[require(
    RigidBody::Kinematic,
    CustomPositionIntegration,
    crate::hit_regions::Hitboxes,
    crate::combat_effects::Wounds,
    eyes::ActorEyes
)]
pub struct Zombie {
    pub definition: ContentId,
    pub gait: crate::rounds::Gait,
    pub(crate) tank_chasing: bool,
    pub(crate) tank_animation: Option<String>,
    burning: bool,
    velocity: Vec3,
    navigation: f32,
    destination: Vec3,
    pub action: EnemyAnimation,
    pub elapsed: f32,
    pub stunned: f32,
    crawler: bool,
    pub(crate) known_damage: bool,
    locomotion_seed: u32,
    next_hit: usize,
    animation_sequence: u64,
    combat: Option<(EnemyCombat, usize)>,
    combat_rotation: Quat,
    lure: Option<Entity>,
    last_target: Vec3,
    route_revision: u32,
    stuck: f32,
    traversal: Option<TraversalMotion>,
    traversal_clip: Option<String>,
    landing: f32,
    tear: Option<BoardTear>,
    barrier_action: Option<BarrierAnimation>,
    barrier_post: Option<(ContentId, usize)>,
    barrier_slot: Option<(ContentId, usize)>,
    barrier_wait: f32,
    spawn_anchor: Option<SpawnAnchor>,
    spawn_show: Option<f32>,
    source_entrance: Option<crate::access::SpawnEntranceDefinition>,
    eye_spawn: eyes::EyeSpawn,
}
impl Zombie {
    pub fn new(position: Vec3, definition: ContentId, gait: crate::rounds::Gait) -> Self {
        Self {
            definition,
            gait,
            tank_chasing: false,
            tank_animation: None,
            burning: false,
            velocity: Vec3::ZERO,
            navigation: 0.,
            destination: position,
            action: EnemyAnimation::Idle,
            elapsed: 0.,
            stunned: 0.,
            crawler: false,
            known_damage: false,
            locomotion_seed: position
                .x
                .to_bits()
                .wrapping_add(position.z.to_bits().rotate_left(13)),
            next_hit: 0,
            animation_sequence: 0,
            combat: None,
            combat_rotation: Quat::IDENTITY,
            lure: None,
            last_target: position,
            route_revision: u32::MAX,
            stuck: 0.,
            traversal: None,
            traversal_clip: None,
            landing: 0.,
            tear: None,
            barrier_action: None,
            barrier_post: None,
            barrier_slot: None,
            barrier_wait: 0.,
            spawn_anchor: None,
            spawn_show: None,
            source_entrance: None,
            eye_spawn: eyes::EyeSpawn::Ordinary,
        }
    }
    pub fn completed_emerging(&self) -> bool {
        self.traversal.is_none()
            && self.spawn_anchor.is_none()
            && self.source_entrance.is_none()
            && self.tear.is_none()
            && self.barrier_action.is_none()
            && !matches!(
                self.action,
                EnemyAnimation::Traverse | EnemyAnimation::Landing
            )
    }
    pub fn knockdown(&mut self, seconds: f32) {
        self.stunned = self.stunned.max(seconds);
        self.transition(EnemyAnimation::Idle);
    }
    pub fn with_locomotion_seed(mut self, seed: u32) -> Self {
        self.set_locomotion_seed(seed);
        self
    }
    pub(crate) fn set_locomotion_seed(&mut self, seed: u32) {
        self.locomotion_seed = seed;
    }
    pub(crate) fn locomotion_clip<'a>(&self, enemy: &'a EnemyDefinition) -> Option<&'a str> {
        let state = if self.burning {
            EnemyLocomotion::Burned
        } else if self.tank_chasing {
            EnemyLocomotion::SuperSprint
        } else if self.crawler {
            match self.action {
                EnemyAnimation::Walk => EnemyLocomotion::WalkCrawl,
                EnemyAnimation::Run => EnemyLocomotion::RunCrawl,
                EnemyAnimation::Sprint => EnemyLocomotion::SprintCrawl,
                _ => EnemyLocomotion::for_action(self.action)?,
            }
        } else {
            EnemyLocomotion::for_action(self.action)?
        };
        let clips = enemy.locomotion_states.get(&state)?;
        if clips.is_empty() {
            return None;
        }
        let seed = self
            .locomotion_seed
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        clips[(seed as usize) % clips.len()].0.rsplit('.').next()
    }
    pub fn reserves(&self, position: Vec3) -> bool {
        self.traversal.as_ref().is_some_and(|m| {
            let delta = m.end - position;
            delta.xz().length_squared() < 0.5625 && delta.y.abs() < 1.
        })
    }
    pub fn rising(mut self, profile: &crate::origins::content::EnemyTraversalDefinition) -> Self {
        self.eye_spawn = eyes::EyeSpawn::Ground;
        self.traversal = Some(TraversalMotion {
            start: self.destination,
            end: self.destination,
            alias: "rise".into(),
            rotation: Quat::IDENTITY,
            profile: 0,
        });
        self.traversal_clip = profile.animation.0.rsplit('.').next().map(str::to_owned);
        self.transition(EnemyAnimation::Traverse);
        self
    }
    pub(crate) fn source_spawn(
        mut self,
        spawner: &crate::access::ActorSpawnerDefinition,
        spawn: &crate::navigation::Admission,
        entrance: crate::access::SpawnEntranceDefinition,
        rise: Option<usize>,
    ) -> Self {
        let desired = spawn
            .desired_origin
            .or(spawner.desired_origin.map(Vec3::from_array));
        let center = self.destination - spawn.position;
        self.spawn_anchor = Some(SpawnAnchor {
            start: Vec3::from_array(spawner.position) + center,
            end: self.destination,
            rotation: Quat::from_rotation_y(spawner.yaw),
            target_rotation: desired.map(|target| {
                let direction = target - spawn.position;
                Quat::from_rotation_y((-direction.z).atan2(direction.x))
            }),
            elapsed: 0.,
            rise,
            rise_rotation: Quat::from_rotation_y(spawn.yaw),
        });
        self.source_entrance = Some(entrance);
        if rise.is_some() {
            self.eye_spawn = eyes::EyeSpawn::Ground;
        }
        self
    }
    pub(crate) fn dug_rising(
        mut self,
        profile: &crate::origins::content::EnemyTraversalDefinition,
        rotation: Quat,
    ) -> Self {
        self = self.rising(profile);
        if let Some(motion) = self.traversal.as_mut() {
            motion.rotation = rotation;
            if let Some(last) = profile.motion.last() {
                motion.end = motion.start + rotation * Vec3::new(last[1], last[2], last[3]);
            }
        }
        self.eye_spawn = eyes::EyeSpawn::Dug;
        self
    }
    fn transition(&mut self, action: EnemyAnimation) {
        if self.action != action {
            if EnemyLocomotion::for_action(action).is_some() {
                self.locomotion_seed = self
                    .locomotion_seed
                    .wrapping_mul(1664525)
                    .wrapping_add(1013904223);
            }
            self.action = action;
            self.elapsed = 0.;
            self.next_hit = 0;
            self.animation_sequence = self.animation_sequence.wrapping_add(1);
            self.combat = None;
        }
    }
    fn start_combat(&mut self, enemy: &EnemyDefinition, state: EnemyCombat) {
        let Some(clips) = enemy.combat_states.get(&state).filter(|c| !c.is_empty()) else {
            return;
        };
        let seed = (self.locomotion_seed as u64)
            .wrapping_add(self.animation_sequence.wrapping_mul(1664525));
        self.combat = Some((state, seed as usize % clips.len()));
    }
    pub(crate) fn combat_clip<'a>(
        &self,
        enemy: &'a EnemyDefinition,
    ) -> Option<&'a EnemyCombatClip> {
        let (state, index) = self.combat?;
        enemy.combat_states.get(&state)?.get(index)
    }
    pub(crate) fn crawler(&self) -> bool {
        self.crawler
    }
    pub(crate) fn animation_sequence(&self) -> u64 {
        self.animation_sequence
    }
    pub(crate) fn barrier_vocal(&self) -> Option<&'static str> {
        self.barrier_action
            .as_ref()
            .filter(|action| !action.enter)
            .map(|action| if action.reach { "attack" } else { "taunt" })
    }
    fn barrier_behavior(
        &mut self,
        window: &crate::barriers::BarrierDefinition,
        slot: Option<usize>,
        target: Option<Entity>,
        pose: &Transform,
        session: &mut crate::gameplay::Session,
    ) -> bool {
        if self.crawler {
            return false;
        }
        let reach = slot
            .zip(window.reach_through.as_ref())
            .filter(|(_, action)| target.is_some() && action.frequency >= session.random() % 100);
        let selected = if let Some((slot, action)) = reach {
            action
                .reach_clip(slot, session.random())
                .map(|clip| (true, clip))
        } else {
            None
        };
        let selected = selected.or_else(|| {
            window
                .taunt
                .as_ref()
                .filter(|action| action.frequency >= session.random() % 100)
                .and_then(|action| {
                    action
                        .clips
                        .get(session.random() as usize % action.clips.len())
                        .map(|clip| (false, clip))
                })
        });
        let Some((reach, clip)) = selected else {
            return false;
        };
        self.transition(EnemyAnimation::Tear);
        self.elapsed = 0.;
        self.next_hit = 0;
        self.animation_sequence = self.animation_sequence.wrapping_add(1);
        self.traversal_clip = Some(clip.name.clone());
        self.barrier_action = Some(BarrierAnimation {
            barrier: window.id.clone(),
            reach,
            enter: false,
            clip: clip.clone(),
            origin: pose.translation,
            rotation: pose.rotation,
            target,
            waiting_slot: slot.is_none(),
        });
        true
    }
    pub(crate) fn animation_clip<'a>(
        &self,
        enemy: &EnemyDefinition,
        game: &'a GameDefinition,
    ) -> Option<&'a crate::origins::content::AnimationDefinition> {
        let name = self.animation_name(enemy)?;
        game.content.animations.iter().find(|animation| {
            enemy.presentation.as_ref() == Some(&animation.skeleton)
                && animation.id.0.rsplit('.').next() == Some(name)
        })
    }
    fn animation_name<'a>(&'a self, enemy: &'a EnemyDefinition) -> Option<&'a str> {
        if let Some(name) = self.tank_animation.as_deref() {
            Some(name)
        } else if matches!(self.action, EnemyAnimation::Traverse | EnemyAnimation::Tear) {
            self.traversal_clip.as_deref()
        } else if let Some(clip) = self.combat_clip(enemy) {
            clip.animation.0.rsplit('.').next()
        } else if let Some(name) = self.locomotion_clip(enemy) {
            Some(name)
        } else {
            enemy
                .animation_actions
                .get(&if self.action == EnemyAnimation::Landing {
                    EnemyAnimation::Idle
                } else {
                    self.action
                })
                .and_then(|id| id.0.rsplit('.').next())
        }
    }
}
struct BoardTear {
    barrier: ContentId,
    board: usize,
    slot: usize,
    pulling: bool,
    destroyed: bool,
    crawling: bool,
}
struct SpawnAnchor {
    start: Vec3,
    end: Vec3,
    rotation: Quat,
    target_rotation: Option<Quat>,
    elapsed: f32,
    rise: Option<usize>,
    rise_rotation: Quat,
}
impl SpawnAnchor {
    fn sample(&self) -> Transform {
        Transform::from_translation(
            self.start
                .lerp(self.end, (self.elapsed / 0.05).clamp(0., 1.)),
        )
        .with_rotation(self.target_rotation.map_or(self.rotation, |target| {
            self.rotation
                .slerp(target, ((self.elapsed - 0.05) / 0.05).clamp(0., 1.))
        }))
    }
    fn seconds(&self) -> f32 {
        if self.target_rotation.is_some() {
            0.1
        } else {
            0.05
        }
    }
}
struct BarrierAnimation {
    barrier: ContentId,
    reach: bool,
    enter: bool,
    clip: crate::barriers::BarrierActorClip,
    origin: Vec3,
    rotation: Quat,
    target: Option<Entity>,
    waiting_slot: bool,
}
struct TraversalMotion {
    start: Vec3,
    end: Vec3,
    alias: String,
    rotation: Quat,
    profile: usize,
}
impl TraversalMotion {
    fn position(
        &self,
        profile: &crate::origins::content::EnemyTraversalDefinition,
        elapsed: f32,
    ) -> Vec3 {
        let last = profile.motion.last().copied().unwrap_or([0.; 4]);
        let mut sample = Vec3::new(last[1], last[2], last[3]);
        for pair in profile.motion.windows(2) {
            if elapsed <= pair[1][0] {
                let t = ((elapsed - pair[0][0]) / (pair[1][0] - pair[0][0])).clamp(0., 1.);
                sample = Vec3::new(pair[0][1], pair[0][2], pair[0][3])
                    .lerp(Vec3::new(pair[1][1], pair[1][2], pair[1][3]), t);
                break;
            }
        }
        let finish = self.rotation * Vec3::new(last[1], last[2], last[3]);
        let fraction = (elapsed / profile.seconds).clamp(0., 1.);
        self.start + self.rotation * sample + (self.end - self.start - finish) * fraction
    }
}
#[derive(Resource, Default)]
pub struct EnemyEvidence {
    pub variants: BTreeSet<String>,
    pub animations: BTreeSet<String>,
    pub locomotion_clips: BTreeSet<String>,
    pub attached_heads: u32,
    pub attacks_started: u32,
    pub replans: u32,
    pub target_replans: u32,
    pub blocker_replans: u32,
    pub direct_pursuits: u32,
    pub stuck_recoveries: u32,
    pub separations: u32,
    pub attack_hits: u32,
    pub deaths: u32,
    pub corpses_retired: u32,
    pub corpses_evicted: u32,
    pub corpses_removed: u32,
    pub bounds_updates: u64,
    pub native_bounds_frames: u64,
    pub fallback_bounds_frames: u64,
    pub traversals_started: u32,
    pub traversals_finished: u32,
    pub traversal_blocked_landings: u32,
    pub rising_finished: u32,
    pub adjusted_landings: u32,
}
#[derive(Component)]
pub(crate) struct ActorRig {
    presentation: ContentId,
    replacement: bool,
    primary: bool,
    owner: Entity,
    source: Handle<Gltf>,
    socket: Option<String>,
    socket_entity: Option<Entity>,
    partial_root: Option<String>,
    ready: bool,
    attached: bool,
    players: Vec<Entity>,
    sockets: HashMap<String, Entity>,
    action: Option<EnemyAnimation>,
    clip: Option<String>,
    sequence: Option<u64>,
}
impl ActorRig {
    pub(crate) fn joint(&self, owner: Entity, name: &str) -> Option<Entity> {
        (self.owner == owner && !self.replacement)
            .then(|| self.sockets.get(name).copied())
            .flatten()
    }
    pub(crate) fn socket(&self, owner: Entity, name: &str) -> Option<Entity> {
        (self.owner == owner && self.primary)
            .then(|| self.sockets.get(name).copied())
            .flatten()
    }
}
#[derive(Component)]
struct PartialRoot;
#[derive(Component)]
struct ActorMesh {
    owner: Entity,
}
#[derive(Component)]
struct HasActor;
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct EnemyBounds;
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct EnemyRetirement;
#[derive(Component)]
struct ClientRagdollRequested;
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ActorCorpse {
    pub(crate) slot: u8,
}
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct EnemyAnimationPlayback;
pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        ragdoll::plugin(app);
        gibs::plugin(app);
        app.init_resource::<EnemyEvidence>()
            .add_systems(
                Update,
                eyes::update
                    .after(crate::presentation::cues::advance)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                clone_animation
                    .in_set(GameStep::Zombies)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                lifecycle
                    .in_set(GameStep::EnemyIntent)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                native_bounds
                    .after(GameStep::Zombies)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (corpses, replace_distant)
                    .in_set(GameStep::Zombies)
                    .in_set(EnemyRetirement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    spawn_actors,
                    wounds_visibility.run_if(resource_exists::<GameDefinition>),
                    attach_parts,
                    admit_ready,
                    animate.in_set(EnemyAnimationPlayback),
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (
                    partial_roots,
                    wounded_joints.run_if(resource_exists::<GameDefinition>),
                    follow_sockets,
                )
                    .chain()
                    .in_set(crate::animation::AnimationPoseModifiers)
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                animated_bounds
                    .in_set(EnemyBounds)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(VisibilitySystems::CalculateBounds)
                    .before(VisibilitySystems::CheckVisibility),
            )
            .add_observer(ready);
    }
}
#[derive(Component)]
struct VisualClone {
    elapsed: f32,
}
pub(crate) fn spawn_visual_clone(
    commands: &mut Commands,
    server: &AssetServer,
    game: &GameDefinition,
    definition: &ContentId,
    pose: Transform,
    clip: &str,
) -> Option<Entity> {
    let enemy = game.content.enemies.iter().find(|e| e.id == *definition)?;
    let owner = commands
        .spawn((
            VisualClone { elapsed: 0. },
            crate::gameplay::SessionEntity,
            pose,
            Visibility::Inherited,
        ))
        .id();
    for (id, socket, partial_root) in
        enemy
            .presentation
            .iter()
            .map(|id| (id, None, None))
            .chain(enemy.parts.iter().map(|p| {
                (
                    &p.presentation,
                    Some(p.socket.clone()),
                    p.root_joint.clone(),
                )
            }))
    {
        let Some(scene) = game
            .content
            .presentations
            .iter()
            .find(|p| p.id == *id)
            .and_then(|p| p.scene.as_ref())
        else {
            continue;
        };
        let source: Handle<Gltf> = server.load(scene.split('#').next().unwrap_or(scene).to_owned());
        let body = socket.is_none();
        commands.spawn((
            ChildOf(owner),
            WorldAssetRoot(server.load(scene.to_owned())),
            ActorRig {
                presentation: id.clone(),
                replacement: false,
                primary: body,
                owner,
                source: source.clone(),
                socket,
                socket_entity: None,
                partial_root,
                ready: false,
                attached: body,
                players: Vec::new(),
                sockets: HashMap::new(),
                action: None,
                clip: None,
                sequence: None,
            },
            if body {
                Transform::from_translation(-body_center(enemy, false))
            } else {
                Transform::IDENTITY
            },
            if body {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            crate::animation::NativeAnimation {
                source,
                clip: clip.into(),
                seconds: 0.,
                looping: false,
                restart: 0,
            },
        ));
    }
    Some(owner)
}
fn clone_animation(
    time: Res<Time<Fixed>>,
    mut clones: Query<&mut VisualClone>,
    mut rigs: Query<(&ActorRig, &mut crate::animation::NativeAnimation)>,
) {
    for mut clone in &mut clones {
        clone.elapsed += time.delta_secs();
    }
    for (rig, mut animation) in &mut rigs {
        if let Ok(clone) = clones.get(rig.owner) {
            animation.seconds = clone.elapsed;
        }
    }
}
pub(crate) fn lifecycle(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut navigation: ResMut<Navigation>,
    access: Res<crate::access::ZoneAccess>,
    attractions: Res<crate::lures::Attractions>,
    effects: (
        Res<crate::powerups::PowerupEffects>,
        ResMut<crate::gameplay::Session>,
    ),
    player: Single<(Entity, &Transform, &Vitality, &Player), (With<Player>, Without<Zombie>)>,
    mut enemies: Query<
        (
            Entity,
            &mut Zombie,
            &mut Transform,
            &Collider,
            &Vitality,
            Has<ActorReady>,
            &crate::combat_effects::Wounds,
        ),
        (
            Without<Player>,
            Without<crate::panzer::Panzer>,
            Without<crate::tank::TankMotion>,
        ),
    >,
    slide: MoveAndSlide,
    query: SpatialQuery,
    mut impacts: ResMut<crate::damage::DamageQueue>,
    mut evidence: ResMut<EnemyEvidence>,
    mut barriers: ResMut<crate::barriers::Barriers>,
    goals: Query<(&EnemyGoal, Option<&crate::buildables::ShieldAggro>)>,
) {
    let (effects, mut session) = effects;
    let (player_entity, player_transform, health, player_state) = player.into_inner();
    let dt = time.delta_secs();
    let mut positions: Vec<_> = enemies
        .iter()
        .filter(|(_, _, _, _, v, _, _)| v.state() == LifeState::ActorAlive)
        .map(|(e, _, t, _, _, _, _)| (e, t.translation))
        .collect();
    for (entity, mut zombie, mut transform, shape, vitality, ready, wounds) in &mut enemies {
        let Some(enemy) = definition
            .content
            .enemies
            .iter()
            .find(|e| e.id == zombie.definition)
        else {
            continue;
        };
        if vitality.state() == LifeState::ActorDead {
            barriers.release(entity);
            continue;
        }
        if !ready {
            continue;
        }
        if let Some(anchor) = zombie.spawn_anchor.as_mut() {
            anchor.elapsed += dt;
            *transform = anchor.sample();
            if anchor.elapsed >= anchor.seconds() {
                let rise = anchor.rise;
                let rotation = anchor.rise_rotation;
                zombie.spawn_anchor = None;
                if let Some(index) = rise {
                    let Some(profile) = enemy
                        .traversals
                        .get("rise")
                        .and_then(|profiles| profiles.get(index))
                    else {
                        continue;
                    };
                    zombie.traversal = Some(TraversalMotion {
                        start: transform.translation,
                        end: transform.translation
                            + rotation
                                * Vec3::from_array(
                                    profile.motion.last().unwrap()[1..4].try_into().unwrap(),
                                ),
                        alias: "rise".into(),
                        rotation,
                        profile: index,
                    });
                    zombie.traversal_clip =
                        profile.animation.0.rsplit('.').next().map(str::to_owned);
                    zombie.transition(EnemyAnimation::Traverse);
                    zombie.spawn_show = Some(0.5);
                } else {
                    commands
                        .entity(entity)
                        .insert(Visibility::Inherited)
                        .remove::<ColliderDisabled>();
                }
            }
            zombie.velocity = Vec3::ZERO;
            continue;
        }
        if let Some(seconds) = zombie.spawn_show.as_mut() {
            *seconds -= dt;
            if *seconds <= 0. {
                zombie.spawn_show = None;
                commands.entity(entity).insert(Visibility::Inherited);
            }
        }
        let crawler_shape = if wounds.crawler && !zombie.crawler {
            zombie.crawler = true;
            transform.translation += body_center(enemy, true) - body_center(enemy, false);
            let shape = body_shape(enemy, true);
            commands.entity(entity).insert(shape.clone());
            zombie.transition(EnemyAnimation::Idle);
            Some(shape)
        } else {
            None
        };
        let source_shape = enemy.physics.map(|_| body_shape(enemy, zombie.crawler));
        let shape = source_shape
            .as_ref()
            .or(crawler_shape.as_ref())
            .unwrap_or(shape);
        zombie.burning = wounds.burning > 0.;
        let half_height = body_center(enemy, zombie.crawler).y;
        if zombie.stunned > 0. {
            zombie.stunned = (zombie.stunned - dt).max(0.);
            if zombie.traversal.is_none() && zombie.tear.is_none() {
                zombie.transition(EnemyAnimation::Idle);
            }
            zombie.velocity = Vec3::ZERO;
            continue;
        }
        if zombie.elapsed == 0. && zombie.traversal.as_ref().is_some_and(|m| m.alias == "rise") {
            evidence.traversals_started += 1;
        }
        zombie.elapsed += dt * wounds.movement_scale();
        zombie.barrier_wait -= dt;
        if let Some(id) = zombie
            .barrier_action
            .as_ref()
            .filter(|action| action.waiting_slot && zombie.barrier_wait <= 0.)
            .map(|action| action.barrier.clone())
            && let Some(window) = definition
                .content
                .barriers
                .iter()
                .find(|window| window.id == id)
        {
            zombie.barrier_wait = 0.5;
            if let Some(slot) = barriers.slot(&id, entity, session.random()) {
                zombie.barrier_slot = Some((id, slot));
                zombie.barrier_action.as_mut().unwrap().waiting_slot = false;
                zombie.barrier_wait = 0.;
            } else if zombie.barrier_behavior(window, None, None, &transform, &mut session) {
                continue;
            }
        }
        if let Some(action) = zombie.barrier_action.as_ref() {
            let Some(window) = definition
                .content
                .barriers
                .iter()
                .find(|window| window.id == action.barrier)
            else {
                continue;
            };
            let elapsed = zombie.elapsed.min(action.clip.seconds);
            if let Some(motion) = &action.clip.motion {
                let initial = if action.enter {
                    Vec3::ZERO
                } else {
                    Vec3::from_array(motion.translation_at(0.))
                };
                transform.translation = action.origin
                    + action.rotation
                        * (Vec3::from_array(motion.translation_at(elapsed)) - initial);
                transform.rotation = action.rotation
                    * Quat::from_array(motion.rotation_at(elapsed))
                    * Quat::from_array(motion.rotation_at(0.)).inverse();
            }
            let target = action.target;
            let reach = action.reach;
            let enter = action.enter;
            let origin = action.origin;
            let seconds = action.clip.seconds;
            let hits = action.clip.hits.clone();
            while zombie.next_hit < hits.len() && elapsed >= hits[zombie.next_hit] {
                zombie.next_hit += 1;
                if reach
                    && target == Some(player_entity)
                    && let Some(attack) = &window.reach_through
                {
                    let player = player_transform.translation - Vec3::Y * player_state.height * 0.5;
                    let feet = transform.translation - Vec3::Y * half_height;
                    if attack.hits_through(player, feet, Vec3::from_array(window.repair_origin)) {
                        impacts.0.push(crate::damage::Impact {
                            point: None,
                            attacker: entity,
                            target: player_entity,
                            source: crate::damage::ImpactSource::Enemy(zombie.definition.clone()),
                            category: crate::origins::content::DamageCategory::Melee,
                            region: crate::weapons::HitRegion::Torso,
                            amount: enemy.damage,
                        });
                    }
                }
            }
            if elapsed >= seconds {
                if enter {
                    zombie.barrier_slot = None;
                    zombie.source_entrance = None;
                    commands.entity(entity).remove::<ColliderDisabled>();
                } else {
                    transform.translation = origin;
                }
                zombie.barrier_action = None;
                zombie.traversal_clip = None;
                zombie.transition(EnemyAnimation::Idle);
                zombie.navigation = 0.;
            }
            zombie.velocity = Vec3::ZERO;
            continue;
        }
        if let Some((id, slot)) = zombie.barrier_post.take()
            && let Some(window) = definition
                .content
                .barriers
                .iter()
                .find(|window| window.id == id)
        {
            let player = player_transform.translation - Vec3::Y * player_state.height * 0.5;
            let feet = transform.translation - Vec3::Y * half_height;
            let target = (health.state() == LifeState::Standing
                && effects.zombie_blood <= 0.
                && window.reach_through.as_ref().is_some_and(|attack| {
                    (player - feet).xz().length_squared() <= attack.range.powi(2)
                }))
            .then_some(player_entity);
            if zombie.barrier_behavior(window, Some(slot), target, &transform, &mut session) {
                continue;
            }
        }
        if let Some(tear) = zombie.tear.as_ref() {
            let Some(window) = definition
                .content
                .barriers
                .iter()
                .find(|b| b.id == tear.barrier)
            else {
                continue;
            };
            let board = window.boards[tear.board].attack(tear.slot, tear.crawling);
            if let Some(placement) = definition
                .content
                .placements
                .iter()
                .find(|p| p.id == window.placement)
            {
                let motion = if tear.pulling {
                    &board.tear_motion
                } else {
                    &board.grab_motion
                };
                let sample = Vec3::from_array(motion.translation_at(zombie.elapsed));
                transform.translation = Vec3::from_array(placement.position)
                    + Vec3::Y * half_height
                    + Quat::from_rotation_y(placement.yaw) * sample;
                transform.rotation = Quat::from_rotation_y(placement.yaw)
                    * Quat::from_array(motion.rotation_at(zombie.elapsed))
                    * Quat::from_array(motion.rotation_at(0.)).inverse();
            }
            if !tear.pulling && zombie.elapsed >= board.grab_seconds {
                let crawling = zombie.crawler;
                let next = window.boards[tear.board].attack(tear.slot, crawling);
                barriers.begin_pull(&tear.barrier, tear.board, entity);
                zombie.tear.as_mut().unwrap().pulling = true;
                zombie.tear.as_mut().unwrap().crawling = crawling;
                zombie.elapsed = 0.;
                zombie.traversal_clip = Some(next.tear_clip.clone());
            } else if tear.pulling {
                let post = (tear.barrier.clone(), tear.slot);
                let finished = zombie.elapsed >= board.tear_seconds;
                if !tear.destroyed && zombie.elapsed >= board.destroy_seconds {
                    barriers.destroy(&tear.barrier, tear.board, entity);
                    zombie.tear.as_mut().unwrap().destroyed = true;
                }
                if finished {
                    zombie.barrier_post = Some(post);
                    zombie.tear = None;
                    zombie.traversal_clip = None;
                    zombie.transition(EnemyAnimation::Idle);
                    zombie.navigation = 0.;
                }
            }
            zombie.velocity = Vec3::ZERO;
            continue;
        }
        if let Some(motion) = zombie.traversal.as_ref() {
            let Some(profile) = enemy
                .traversals
                .get(&motion.alias)
                .and_then(|profiles| profiles.get(motion.profile))
            else {
                continue;
            };
            transform.translation = motion.position(profile, zombie.elapsed);
            if let Some(root) = definition
                .content
                .animations
                .iter()
                .find(|clip| clip.id == profile.animation)
                .and_then(|clip| clip.root_motion.as_ref())
            {
                transform.rotation = motion.rotation
                    * Quat::from_array(root.rotation_at(zombie.elapsed.min(profile.seconds)))
                    * Quat::from_array(root.rotation_at(0.)).inverse();
            }
            if zombie.elapsed >= profile.seconds {
                let filter = crate::environment::actor_solids([entity]);
                let occupied = |p: Vec3| {
                    positions.iter().any(|(other, point)| {
                        *other != entity
                            && (point - p).xz().length_squared() < 0.49
                            && (point.y - p.y).abs() < 1.
                    })
                };
                let landing =
                    if motion.alias == "rise" && matches!(zombie.eye_spawn, eyes::EyeSpawn::Dug) {
                        Some(motion.end)
                    } else if !occupied(motion.end)
                        && query
                            .shape_intersections(shape, motion.end, Quat::IDENTITY, &filter)
                            .is_empty()
                    {
                        Some(motion.end)
                    } else if zombie.elapsed > profile.seconds + 0.3 {
                        nearby_landing(
                            &query,
                            shape,
                            motion.end,
                            half_height,
                            &filter,
                            &crate::environment::actor_solids([entity, player_entity]),
                            &navigation,
                            &access,
                            &occupied,
                        )
                    } else {
                        None
                    };
                if let Some(landing) = landing {
                    if landing.distance_squared(motion.end) > 0.01 {
                        evidence.adjusted_landings += 1;
                    }
                    transform.translation = landing;
                    if let Some((_, point)) = positions.iter_mut().find(|(e, _)| *e == entity) {
                        *point = landing;
                    }
                    let rising = motion.alias == "rise"
                        && (enemy.physics.is_some()
                            || matches!(zombie.eye_spawn, eyes::EyeSpawn::Dug));
                    if motion.alias == "rise" {
                        evidence.rising_finished += 1;
                    } else if zombie.source_entrance.as_ref().is_some_and(|entrance| {
                        definition.content.navigation_nodes.iter().any(|node| {
                            node.id == entrance.node
                                && (Vec3::from_array(node.position) + Vec3::Y * half_height)
                                    .distance_squared(motion.start)
                                    < 0.001
                        })
                    }) {
                        zombie.source_entrance = None;
                    }
                    zombie.traversal = None;
                    zombie.traversal_clip = None;
                    if rising {
                        zombie.landing = 0.;
                        let action = match zombie.gait {
                            crate::rounds::Gait::Walk => EnemyAnimation::Walk,
                            crate::rounds::Gait::Run => EnemyAnimation::Run,
                            crate::rounds::Gait::Sprint => EnemyAnimation::Sprint,
                        };
                        zombie.transition(action);
                    } else {
                        zombie.landing = 0.25;
                        zombie.transition(EnemyAnimation::Landing);
                    }
                    zombie.navigation = 0.;
                    commands.entity(entity).remove::<ColliderDisabled>();
                    evidence.traversals_finished += 1;
                } else {
                    evidence.traversal_blocked_landings += 1;
                }
            }
            continue;
        }
        if zombie.landing > 0. {
            zombie.landing = (zombie.landing - dt).max(0.);
            zombie.velocity = Vec3::ZERO;
            continue;
        }
        let entrance = zombie.source_entrance.clone();
        let lure = entrance
            .is_none()
            .then(|| attractions.0.get(&entity))
            .flatten();
        if zombie.lure != lure.map(|l| l.bomb) {
            zombie.lure = lure.map(|l| l.bomb);
            zombie.navigation = 0.;
            zombie.transition(EnemyAnimation::Idle);
        }
        let goal = (lure.is_none() && entrance.is_none())
            .then(|| goals.get(entity).ok().map(|(goal, _)| goal))
            .flatten();
        let ignored = entrance.is_none()
            && lure.is_none()
            && goal.is_none()
            && (health.state() != LifeState::Standing || effects.zombie_blood > 0.);
        if ignored {
            zombie.transition(EnemyAnimation::Idle);
        }
        let target = if let Some(entrance) = &entrance {
            Vec3::from_array(entrance.position) + Vec3::Y * half_height
        } else if ignored {
            transform.translation
        } else {
            lure.map_or_else(
                || goal.map_or(player_transform.translation, |g| g.position),
                |l| l.position,
            )
        };
        let shield_target = goals
            .get(entity)
            .ok()
            .and_then(|(_, shield)| shield.map(|s| s.0));
        let filter =
            crate::environment::solids([entity, player_entity].into_iter().chain(shield_target));
        let visible = |from: Vec3, to: Vec3| {
            let delta = to - from;
            Dir3::new(delta).ok().is_some_and(|d| {
                query
                    .cast_ray(from, d, delta.length(), false, &filter)
                    .is_none()
            })
        };
        let distance = transform.translation.distance(target);
        let stop_check = zombie.combat_clip(enemy).is_some_and(|clip| {
            clip.stops.iter().any(|time| {
                *time > zombie.elapsed - dt * wounds.movement_scale() && *time <= zombie.elapsed
            })
        });
        if zombie.action == EnemyAnimation::Attack
            && (wounds.melee_disabled()
                || ignored
                || lure.is_some()
                || (stop_check
                    && (distance > goal.map_or(64_f32 * 0.0254, |g| g.radius)
                        || !visible(transform.translation, target))))
        {
            zombie.transition(EnemyAnimation::Idle);
        }
        if lure.is_none()
            && entrance.is_none()
            && !wounds.melee_disabled()
            && !ignored
            && zombie.action != EnemyAnimation::Attack
            && distance <= goal.map_or(64_f32 * 0.0254, |g| g.radius)
            && visible(transform.translation, target)
        {
            zombie.transition(EnemyAnimation::Attack);
            let facing = goal.map_or(target, |goal| goal.face) - transform.translation;
            zombie.combat_rotation = Quat::from_rotation_y((-facing.z).atan2(facing.x));
            let state = if shield_target.is_some() {
                if zombie.crawler {
                    EnemyCombat::ShieldMeleeCrawl
                } else {
                    EnemyCombat::ShieldMelee
                }
            } else if goal.is_some() {
                EnemyCombat::GeneratorMelee
            } else if zombie.gait == crate::rounds::Gait::Walk {
                if zombie.crawler {
                    EnemyCombat::WalkMeleeCrawl
                } else {
                    EnemyCombat::WalkMelee
                }
            } else {
                if zombie.crawler {
                    EnemyCombat::RunMeleeCrawl
                } else {
                    EnemyCombat::RunMelee
                }
            };
            zombie.start_combat(enemy, state);
            evidence.attacks_started += 1;
        }
        if zombie.action == EnemyAnimation::Attack {
            let clip = zombie.combat_clip(enemy);
            let hits: &[f32] = if shield_target.is_some() {
                &[]
            } else {
                clip.map_or(enemy.attack_hits.as_slice(), |c| c.hits.as_slice())
            };
            let seconds = clip.map_or(enemy.attack_seconds, |c| c.seconds);
            while zombie.next_hit < hits.len() && zombie.elapsed >= hits[zombie.next_hit] {
                zombie.next_hit += 1;
                if goal.is_none()
                    && !ignored
                    && distance <= 64_f32 * 0.0254
                    && visible(transform.translation, target)
                    && health.state() == LifeState::Standing
                {
                    impacts.0.push(crate::damage::Impact {
                        point: None,
                        attacker: entity,
                        target: player_entity,
                        source: crate::damage::ImpactSource::Enemy(zombie.definition.clone()),
                        category: crate::origins::content::DamageCategory::Melee,
                        region: crate::weapons::HitRegion::Torso,
                        amount: enemy.damage,
                    });
                }
            }
            if zombie.elapsed >= seconds {
                if let Some(shield) = shield_target.filter(|_| distance < 1.8) {
                    impacts.0.push(crate::damage::Impact {
                        point: None,
                        attacker: entity,
                        target: shield,
                        source: crate::damage::ImpactSource::Enemy(zombie.definition.clone()),
                        category: crate::origins::content::DamageCategory::Melee,
                        region: crate::weapons::HitRegion::Torso,
                        amount: 100,
                    });
                }
                zombie.transition(EnemyAnimation::Idle);
            }
        }
        let feet = transform.translation - Vec3::Y * half_height;
        let target_center = if enemy.physics.is_some() {
            if goal.is_some() || entrance.is_some() {
                half_height
            } else {
                player_state.height * 0.5
            }
        } else {
            0.9
        };
        if zombie.route_revision != navigation.revision
            || target.distance_squared(zombie.last_target) > 0.5625
        {
            if zombie.route_revision != u32::MAX && zombie.route_revision != navigation.revision {
                evidence.blocker_replans += 1;
            }
            if target.distance_squared(zombie.last_target) > 0.5625 {
                evidence.target_replans += 1;
            }
            zombie.navigation = 0.;
            zombie.route_revision = navigation.revision;
            zombie.last_target = target;
            evidence.replans += 1;
        }
        zombie.navigation -= dt;
        if zombie.navigation <= 0. {
            zombie.navigation = 0.5;
            zombie.destination = if let Some(entrance) = &entrance {
                navigation.entrance_destination(feet, entrance, &access) + Vec3::Y * half_height
            } else if distance < 4.
                && (transform.translation.y - target.y).abs() < 0.7
                && navigation.direct_allowed(feet, target - Vec3::Y * target_center, &access)
                && visible(transform.translation, target)
            {
                evidence.direct_pursuits += 1;
                target
            } else {
                navigation.destination(feet, target - Vec3::Y * target_center, &access)
                    + Vec3::Y * half_height
            };
        }
        let step = if let Some(entrance) = &entrance {
            navigation.entrance_traversal(feet, entrance, &access)
        } else {
            navigation.traversal(feet, zombie.destination - Vec3::Y * half_height, &access)
        };
        let mut approaching_barrier = false;
        if let Some(step) = step {
            let crate::navigation::TraversalStep {
                start,
                end,
                alias,
                yaw,
            } = step;
            if let Some(window) = definition.content.barriers.iter().find(|b| {
                entrance
                    .as_ref()
                    .is_none_or(|entrance| b.entry == entrance.node)
                    && definition
                        .content
                        .navigation_nodes
                        .iter()
                        .find(|n| n.id == b.entry)
                        .is_some_and(|n| {
                            let d = Vec3::from_array(n.position) - start;
                            d.xz().length_squared() < 0.25 && d.y.abs() < 0.8
                        })
            }) {
                if zombie.barrier_wait > 0. {
                    continue;
                }
                let slot = zombie
                    .barrier_slot
                    .as_ref()
                    .filter(|(id, _)| *id == window.id)
                    .map(|(_, slot)| *slot)
                    .or_else(|| barriers.slot(&window.id, entity, session.random()));
                let Some(slot) = slot else {
                    zombie.velocity = Vec3::ZERO;
                    zombie.barrier_wait = 0.5;
                    if !zombie.barrier_behavior(window, None, None, &transform, &mut session) {
                        zombie.transition(EnemyAnimation::Idle);
                    }
                    continue;
                };
                zombie.barrier_slot = Some((window.id.clone(), slot));
                let Some(attack_spot) = barriers.attack_spot(&window.id, slot) else {
                    continue;
                };
                if (attack_spot - feet).length_squared() > (2. * 0.0254_f32).powi(2) {
                    zombie.destination = attack_spot + Vec3::Y * half_height;
                    approaching_barrier = true;
                }
                if !approaching_barrier {
                    if barriers.obstructed(&window.id) {
                        zombie.velocity = Vec3::ZERO;
                        if zombie.barrier_wait > 0. {
                            continue;
                        }
                        if let Some((board, slot)) =
                            barriers.claim(&window.id, entity, session.random())
                        {
                            zombie.barrier_slot = Some((window.id.clone(), slot));
                            zombie.tear = Some(BoardTear {
                                barrier: window.id.clone(),
                                board,
                                slot,
                                pulling: false,
                                destroyed: false,
                                crawling: zombie.crawler,
                            });
                            zombie.traversal_clip = Some(
                                window.boards[board]
                                    .attack(slot, zombie.crawler)
                                    .grab_clip
                                    .clone(),
                            );
                            zombie.transition(EnemyAnimation::Tear);
                            let delta = end - start;
                            transform.rotation = Quat::from_rotation_y((-delta.z).atan2(delta.x));
                            if let Some(placement) = definition
                                .content
                                .placements
                                .iter()
                                .find(|p| p.id == window.placement)
                            {
                                let sample = Vec3::from_array(
                                    window.boards[board]
                                        .attack(slot, zombie.crawler)
                                        .grab_motion
                                        .translation_at(0.),
                                );
                                transform.translation = Vec3::from_array(placement.position)
                                    + Vec3::Y * half_height
                                    + Quat::from_rotation_y(placement.yaw) * sample;
                                transform.rotation = Quat::from_rotation_y(placement.yaw);
                            }
                        } else {
                            let slot = barriers.slot(&window.id, entity, session.random());
                            if let Some(slot) = slot {
                                zombie.barrier_slot = Some((window.id.clone(), slot));
                            }
                            let player =
                                player_transform.translation - Vec3::Y * player_state.height * 0.5;
                            let feet = transform.translation - Vec3::Y * half_height;
                            let target = (health.state() == LifeState::Standing
                                && effects.zombie_blood <= 0.
                                && window.reach_through.as_ref().is_some_and(|attack| {
                                    (player - feet).xz().length_squared() <= attack.range.powi(2)
                                }))
                            .then_some(player_entity);
                            zombie.barrier_wait = if slot.is_some() { 0.1 } else { 0.5 };
                            if !zombie.barrier_behavior(
                                window,
                                slot,
                                target,
                                &transform,
                                &mut session,
                            ) {
                                zombie.transition(EnemyAnimation::Idle);
                            }
                        }
                        continue;
                    }
                    if window.enter.is_some() {
                        let slot = zombie
                            .barrier_slot
                            .as_ref()
                            .filter(|(id, _)| *id == window.id)
                            .map(|(_, slot)| *slot)
                            .or_else(|| barriers.slot(&window.id, entity, session.random()));
                        let Some(slot) = slot else {
                            zombie.barrier_wait = 0.5;
                            zombie.barrier_behavior(window, None, None, &transform, &mut session);
                            continue;
                        };
                        let Some(clip) =
                            window.entry_clip(zombie.gait, slot, zombie.crawler, session.random())
                        else {
                            continue;
                        };
                        let Some(placement) = definition
                            .content
                            .placements
                            .iter()
                            .find(|placement| placement.id == window.placement)
                        else {
                            continue;
                        };
                        zombie.transition(EnemyAnimation::Traverse);
                        zombie.elapsed = 0.;
                        zombie.traversal_clip = Some(clip.name.clone());
                        zombie.barrier_action = Some(BarrierAnimation {
                            barrier: window.id.clone(),
                            reach: false,
                            enter: true,
                            clip: clip.clone(),
                            origin: Vec3::from_array(placement.position) + Vec3::Y * half_height,
                            rotation: Quat::from_rotation_y(placement.yaw),
                            target: None,
                            waiting_slot: false,
                        });
                        zombie.velocity = Vec3::ZERO;
                        commands.entity(entity).insert(ColliderDisabled);
                        barriers.release(entity);
                        barriers.entries += 1;
                        continue;
                    }
                    barriers.entries += 1;
                }
            }
            if !approaching_barrier {
                let alias = if alias == "zm_mantle_over_40" {
                    if zombie.crawler {
                        "barrier_crawl"
                    } else {
                        match zombie.gait {
                            crate::rounds::Gait::Walk => "barrier_walk",
                            crate::rounds::Gait::Run => "barrier_run",
                            crate::rounds::Gait::Sprint => "barrier_sprint",
                        }
                    }
                    .to_owned()
                } else {
                    alias
                };
                if let Some(profiles) = enemy
                    .traversals
                    .get(&alias)
                    .filter(|profiles| !profiles.is_empty())
                {
                    let index = zombie
                        .locomotion_seed
                        .wrapping_add(zombie.animation_sequence as u32)
                        as usize
                        % profiles.len();
                    let profile = &profiles[index];
                    let rotation = Quat::from_rotation_y(yaw);
                    zombie.traversal = Some(TraversalMotion {
                        start: start + Vec3::Y * half_height,
                        end: end + Vec3::Y * half_height,
                        alias,
                        rotation,
                        profile: index,
                    });
                    zombie.traversal_clip =
                        profile.animation.0.rsplit('.').next().map(str::to_owned);
                    zombie.transition(EnemyAnimation::Traverse);
                    zombie.velocity = Vec3::ZERO;
                    transform.rotation = rotation;
                    commands.entity(entity).insert(ColliderDisabled);
                    evidence.traversals_started += 1;
                    continue;
                }
            }
        }
        let delta = zombie.destination - transform.translation;
        let direction = Vec3::new(delta.x, 0., delta.z).normalize_or_zero();
        let moving = !ignored
            && zombie.action != EnemyAnimation::Attack
            && !(lure.is_some() && distance < 0.4);
        if moving {
            let action = if zombie.tank_chasing {
                EnemyAnimation::Sprint
            } else {
                match zombie.gait {
                    crate::rounds::Gait::Walk => EnemyAnimation::Walk,
                    crate::rounds::Gait::Run => EnemyAnimation::Run,
                    crate::rounds::Gait::Sprint => EnemyAnimation::Sprint,
                }
            };
            if zombie.action != action {
                zombie.transition(action);
                zombie.elapsed = dt * wounds.movement_scale();
            }
        }
        let clip = zombie
            .combat_clip(enemy)
            .map(|clip| &clip.animation)
            .or_else(|| {
                zombie.locomotion_clip(enemy).and_then(|name| {
                    definition
                        .content
                        .animations
                        .iter()
                        .find(|clip| {
                            enemy
                                .presentation
                                .as_ref()
                                .is_some_and(|presentation| clip.skeleton == *presentation)
                                && clip.id.0.rsplit('.').next() == Some(name)
                        })
                        .map(|clip| &clip.id)
                })
            });
        let motion = clip
            .and_then(|id| {
                definition
                    .content
                    .animations
                    .iter()
                    .find(|clip| clip.id == *id)
            })
            .and_then(|clip| clip.root_motion.as_ref());
        let displacement = if moving || zombie.action == EnemyAnimation::Attack {
            motion
                .map(|motion| {
                    let start = (zombie.elapsed - dt * wounds.movement_scale()).max(0.);
                    Vec3::from_array(if zombie.action == EnemyAnimation::Attack {
                        let a = motion.translation_at(start);
                        let b = motion.translation_at(zombie.elapsed);
                        std::array::from_fn(|axis| b[axis] - a[axis])
                    } else {
                        motion.loop_displacement(start, zombie.elapsed)
                    })
                })
                .unwrap_or(Vec3::ZERO)
        } else {
            Vec3::ZERO
        };
        let velocity = if zombie.action == EnemyAnimation::Attack {
            zombie.combat_rotation * displacement / dt
        } else {
            direction
                * if approaching_barrier {
                    displacement.xz().length().min(delta.xz().length()) / dt
                } else {
                    displacement.xz().length() / dt
                }
        };
        zombie.velocity.x = velocity.x;
        zombie.velocity.z = velocity.z;
        if moving && !approaching_barrier {
            let mut separate = Vec3::ZERO;
            for (other, point) in &positions {
                if *other == entity {
                    continue;
                }
                let delta = transform.translation - *point;
                let flat = Vec3::new(delta.x, 0., delta.z);
                let distance = flat.length();
                if distance > 0.001 && distance < 0.85 && delta.y.abs() < 1. {
                    separate += flat / distance * (1. - distance / 0.85);
                }
            }
            if separate.length_squared() > 0.001 {
                let separate = separate.clamp_length_max(0.65);
                zombie.velocity += separate;
                evidence.separations += 1;
            }
        }
        zombie.velocity.y = (zombie.velocity.y - definition.tuning.gravity * dt).max(-30.);
        let facing = if zombie.action == EnemyAnimation::Attack {
            goal.map_or(target, |g| g.face) - transform.translation
        } else {
            direction
        };
        if zombie.action == EnemyAnimation::Attack {
            let rotation = motion
                .map(|motion| {
                    Quat::from_array(motion.rotation_at(zombie.elapsed))
                        * Quat::from_array(motion.rotation_at(0.)).inverse()
                })
                .unwrap_or(Quat::IDENTITY);
            transform.rotation = zombie.combat_rotation * rotation;
        } else if facing.xz().length_squared() > 0.001 {
            transform.rotation = Quat::from_rotation_y((-facing.z).atan2(facing.x));
        }
        let filter = crate::environment::actor_solids([entity]);
        let support = query.cast_ray(feet + Vec3::Y * 0.1, Dir3::NEG_Y, 0.25, false, &filter);
        let grounded = support.is_some_and(|h| h.normal.y > 0.65);
        let before = transform.translation;
        let result = crate::player::slide_character(
            &slide,
            &query,
            crate::player::MotionShape {
                collider: shape,
                radius: enemy.physics.map_or(0.3, |physics| physics.radius),
                step_height: definition.tuning.step_height,
                grounded,
            },
            before,
            zombie.velocity,
            time.delta(),
            &filter,
        );
        transform.translation = result.position;
        zombie.velocity.y = result.projected_velocity.y;
        if moving {
            let speed = transform.translation.xz().distance(before.xz()) / dt;
            zombie.stuck = if speed < 0.05 { zombie.stuck + dt } else { 0. };
            if zombie.stuck > 0.8 {
                zombie.destination = if let Some(entrance) = &entrance {
                    navigation.entrance_destination(feet, entrance, &access)
                } else {
                    navigation.alternate(
                        feet,
                        target - Vec3::Y * target_center,
                        zombie.destination - Vec3::Y * half_height,
                        &access,
                    )
                } + Vec3::Y * half_height;
                zombie.navigation = 0.5;
                zombie.stuck = 0.;
                evidence.stuck_recoveries += 1;
            }
        } else if zombie.action != EnemyAnimation::Attack {
            zombie.transition(EnemyAnimation::Idle);
        }
    }
}
fn nearby_landing(
    query: &SpatialQuery,
    shape: &Collider,
    end: Vec3,
    half_height: f32,
    filter: &SpatialQueryFilter,
    route_filter: &SpatialQueryFilter,
    navigation: &Navigation,
    access: &crate::access::ZoneAccess,
    occupied: &impl Fn(Vec3) -> bool,
) -> Option<Vec3> {
    for radius in [0.75, 1.] {
        for index in 0..16 {
            let angle = index as f32 * std::f32::consts::TAU / 16.;
            let probe = end + Vec3::new(angle.cos() * radius, 0., angle.sin() * radius);
            let Some(floor) =
                query.cast_ray(probe - Vec3::Y * 0.35, Dir3::NEG_Y, 1., false, filter)
            else {
                continue;
            };
            if floor.normal.y < 0.65 {
                continue;
            }
            let base = probe + Vec3::Y * (0.5 - floor.distance + 0.03);
            if (base.y - end.y).abs() > 0.45 {
                continue;
            }
            for lift in 0..=6 {
                let candidate = base + Vec3::Y * (lift as f32 * 0.05);
                if occupied(candidate)
                    || !navigation.direct_allowed(
                        end - Vec3::Y * half_height,
                        candidate - Vec3::Y * half_height,
                        access,
                    )
                    || !query
                        .shape_intersections(shape, candidate, Quat::IDENTITY, filter)
                        .is_empty()
                {
                    continue;
                }
                let delta = candidate - end;
                let Ok(dir) = Dir3::new(delta) else {
                    continue;
                };
                if query
                    .cast_shape(
                        shape,
                        end,
                        Quat::IDENTITY,
                        dir,
                        &ShapeCastConfig {
                            ignore_origin_penetration: true,
                            ..ShapeCastConfig::from_max_distance(delta.length())
                        },
                        route_filter,
                    )
                    .is_none()
                {
                    return Some(candidate);
                }
            }
        }
    }
    None
}
fn corpses(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut enemies: Query<
        (
            Entity,
            &mut Zombie,
            &Vitality,
            &crate::combat_effects::Wounds,
            &mut Transform,
            &mut Visibility,
            Option<&ClientRagdollRequested>,
            Option<&ActorCorpse>,
        ),
        (Without<crate::panzer::Panzer>, Without<Player>),
    >,
    mut evidence: ResMut<EnemyEvidence>,
    players: Query<(&Player, &Transform), Without<Zombie>>,
) {
    let limit = definition.rounds.corpse_limit;
    let mut occupied = vec![None; limit as usize];
    for (entity, zombie, _, wounds, pose, _, _, corpse) in enemies.iter() {
        if let Some(corpse) = corpse
            && let Some(enemy) = definition
                .content
                .enemies
                .iter()
                .find(|enemy| enemy.id == zombie.definition)
            && let Some(slot) = occupied.get_mut(corpse.slot as usize)
        {
            *slot = Some((
                entity,
                pose.translation - body_center(enemy, wounds.crawler),
            ));
        }
    }
    let (eye, forward) =
        players
            .iter()
            .next()
            .map_or((Vec3::ZERO, Vec3::NEG_Z), |(player, pose)| {
                (
                    pose.translation,
                    Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z,
                )
            });
    for (entity, mut zombie, vitality, wounds, mut pose, mut visibility, ragdoll, corpse) in
        &mut enemies
    {
        if corpse.is_some() {
            zombie.elapsed += time.delta_secs();
            continue;
        }
        if vitality.state() != LifeState::ActorDead {
            continue;
        }
        let Some(enemy) = definition
            .content
            .enemies
            .iter()
            .find(|e| e.id == zombie.definition)
        else {
            continue;
        };
        if zombie.action != EnemyAnimation::Death {
            zombie.transition(EnemyAnimation::Death);
            zombie.combat_rotation = pose.rotation;
            zombie.start_combat(
                enemy,
                if wounds.crawler {
                    EnemyCombat::DeathCrawl
                } else if wounds.burned {
                    EnemyCombat::FireDeath
                } else {
                    EnemyCombat::Death
                },
            );
            commands.entity(entity).insert(ColliderDisabled);
            if wounds.shattered || wounds.disintegrated {
                *visibility = Visibility::Hidden;
                if wounds.shattered {
                    crate::native_fx::emit(
                        &mut commands,
                        "weapon/zmb_staff/fx_zmb_staff_ice_exp",
                        Transform::from_translation(pose.translation),
                    );
                    crate::audio::binding(
                        &mut commands,
                        &definition,
                        "weapon.staff_water_upgraded_zm.impact",
                        crate::audio::SoundSource::World(pose.translation),
                    );
                }
            }
            evidence.deaths += 1;
        }
        let previous = zombie.elapsed;
        zombie.elapsed += time.delta_secs();
        let death_seconds = zombie
            .combat_clip(enemy)
            .map_or(enemy.death_seconds, |c| c.seconds);
        let animation = zombie.combat_clip(enemy).and_then(|combat| {
            definition
                .content
                .animations
                .iter()
                .find(|animation| animation.id == combat.animation)
        });
        let duration = zombie
            .combat_clip(enemy)
            .and_then(|clip| clip.duration)
            .or_else(|| {
                animation
                    .and_then(|animation| animation.root_motion.as_ref())
                    .map(|motion| motion.seconds)
            })
            .unwrap_or(death_seconds);
        let ragdoll_start = animation
            .and_then(|animation| {
                animation
                    .notifications
                    .iter()
                    .find(|note| note.event == "start_ragdoll")
                    .map(|note| note.seconds)
            })
            .unwrap_or(duration * 0.35);
        if let Some(motion) = animation.and_then(|animation| animation.root_motion.as_ref()) {
            let start = motion.translation_at(previous.min(death_seconds));
            let end = motion.translation_at(zombie.elapsed.min(death_seconds));
            pose.translation += zombie.combat_rotation
                * Vec3::from_array(std::array::from_fn(|axis| end[axis] - start[axis]));
            pose.rotation = zombie.combat_rotation
                * Quat::from_array(motion.rotation_at(zombie.elapsed.min(death_seconds)))
                * Quat::from_array(motion.rotation_at(0.)).inverse();
        }
        if ragdoll.is_none() && zombie.elapsed >= ragdoll_start {
            commands.entity(entity).insert(ClientRagdollRequested);
        }
        let retirement = (death_seconds / definition.rounds.actor_think_seconds).ceil()
            * definition.rounds.actor_think_seconds;
        if zombie.elapsed >= retirement {
            let slot = corpse_slot(&occupied, eye, forward);
            if let Some((evicted, _)) = occupied[slot] {
                commands.entity(evicted).despawn();
                evidence.corpses_evicted += 1;
            }
            occupied[slot] = Some((
                entity,
                pose.translation - body_center(enemy, wounds.crawler),
            ));
            commands
                .entity(entity)
                .insert(ActorCorpse { slot: slot as u8 });
            evidence.corpses_retired += 1;
        }
    }
}

fn corpse_slot(occupied: &[Option<(Entity, Vec3)>], eye: Vec3, forward: Vec3) -> usize {
    if let Some(free) = occupied.iter().position(Option::is_none) {
        return free;
    }
    let horizontal = forward.with_y(0.);
    let flat = horizontal.length_squared() > 0.;
    let forward = if flat {
        horizontal.normalize()
    } else {
        forward
    };
    let mut behind = None;
    let mut side = None;
    let mut farthest = None;
    for (slot, corpse) in occupied.iter().enumerate() {
        let Some((_, position)) = corpse else {
            continue;
        };
        let mut delta = *position - eye;
        if flat {
            delta.y = 0.;
        }
        let dot = delta.dot(forward);
        let distance = delta.length_squared();
        if dot < 0. && behind.is_none_or(|(_, previous)| distance > previous) {
            behind = Some((slot, distance));
        }
        if dot * dot <= distance * 0.5 && side.is_none_or(|(_, previous)| distance > previous) {
            side = Some((slot, distance));
        }
        if farthest.is_none_or(|(_, previous)| distance > previous) {
            farthest = Some((slot, distance));
        }
    }
    behind.or(side).or(farthest).map_or(0, |(slot, _)| slot)
}
fn spawn_actors(
    mut commands: Commands,
    server: Res<AssetServer>,
    definition: Option<Res<GameDefinition>>,
    enemies: Query<(Entity, &Zombie), Without<HasActor>>,
    mut evidence: ResMut<EnemyEvidence>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (owner, zombie) in &enemies {
        let Some(enemy) = definition
            .content
            .enemies
            .iter()
            .find(|e| e.id == zombie.definition)
        else {
            continue;
        };
        let mut scenes = enemy
            .presentation
            .iter()
            .map(|id| (id, None, None, false))
            .chain(enemy.parts.iter().map(|part| {
                (
                    &part.presentation,
                    Some(part.socket.clone()),
                    part.root_joint.clone(),
                    false,
                )
            }))
            .collect::<Vec<_>>();
        if let Some(gibs) = &enemy.gibs {
            let mut used = BTreeSet::new();
            scenes.extend(
                gibs.presentations()
                    .filter_map(|id| used.insert(id).then_some((id, None, None, true))),
            );
        }
        let mut pending = Vec::new();
        for (id, socket, partial_root, replacement) in scenes {
            let Some(scene) = definition
                .content
                .presentations
                .iter()
                .find(|p| p.id == *id)
                .and_then(|p| p.scene.as_ref())
            else {
                continue;
            };
            let body = socket.is_none();
            let source = server.load(scene.split('#').next().unwrap_or(scene).to_owned());
            pending.push((
                WorldAssetRoot(server.load(scene.to_owned())),
                ActorRig {
                    presentation: id.clone(),
                    replacement,
                    primary: body && !replacement,
                    owner,
                    source,
                    socket,
                    socket_entity: None,
                    partial_root,
                    ready: false,
                    attached: body,
                    players: Vec::new(),
                    sockets: HashMap::new(),
                    action: None,
                    clip: None,
                    sequence: None,
                },
                if body {
                    Transform::from_translation(if zombie.definition.0 == "enemy.panzer" {
                        Vec3::Y * -1.2
                    } else {
                        -body_center(enemy, zombie.crawler)
                    })
                } else {
                    Transform::default()
                },
                if body && !replacement {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
            ));
        }
        commands.queue(move |world: &mut World| {
            if world.get::<Zombie>(owner).is_none() {
                return;
            }
            for bundle in pending {
                world.spawn((ChildOf(owner), bundle));
            }
            world.entity_mut(owner).insert(HasActor);
        });
        evidence.variants.insert(zombie.definition.0.clone());
    }
}
fn ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut rigs: Query<&mut ActorRig>,
    children: Query<&Children>,
    players: Query<(), With<AnimationPlayer>>,
    meshes: Query<(), With<Mesh3d>>,
    names: Query<&Name>,
    definition: Res<GameDefinition>,
    enemies: Query<&Zombie>,
) {
    let Ok(mut rig) = rigs.get_mut(event.entity) else {
        return;
    };
    rig.ready = true;
    rig.attached = rig.socket.is_none();
    rig.socket_entity = None;
    rig.players.clear();
    rig.sockets.clear();
    rig.action = None;
    rig.clip = None;
    rig.sequence = None;
    let owner = rig.owner;
    let body = rig.socket.is_none() && !rig.replacement;
    commands.entity(owner).remove::<ActorReady>();
    if let Ok(zombie) = enemies.get(rig.owner)
        && let Some(enemy) = definition
            .content
            .enemies
            .iter()
            .find(|e| e.id == zombie.definition)
    {
        let bones: BTreeMap<_, _> = children
            .iter_descendants(event.entity)
            .filter_map(|child| {
                names
                    .get(child)
                    .ok()
                    .map(|name| (name.as_str().to_owned(), child))
            })
            .collect();
        let bindings = enemy
            .hit_regions
            .iter()
            .filter(|r| r.presentation == rig.presentation)
            .filter_map(|r| {
                Some(crate::hit_regions::HitBinding {
                    id: r.id.clone(),
                    region: r.region,
                    joint: *bones.get(&r.joint)?,
                    bounds: r.bounds,
                    sphere: r.sphere,
                })
            })
            .collect();
        commands
            .entity(event.entity)
            .insert(crate::hit_regions::HitRig {
                owner: rig.owner,
                bindings,
                active: !rig.replacement,
            });
    }
    for child in children.iter_descendants(event.entity) {
        if (rig.socket.is_none() || !rig.replacement)
            && let Ok(name) = names.get(child)
        {
            rig.sockets.entry(name.to_string()).or_insert(child);
        }
        if players.contains(child) {
            rig.players.push(child);
        }
        if meshes.contains(child) {
            commands
                .entity(child)
                .insert(ActorMesh { owner: rig.owner });
        }
        if rig
            .partial_root
            .as_ref()
            .is_some_and(|root| names.get(child).is_ok_and(|n| n.as_str() == root))
        {
            commands.entity(child).insert(PartialRoot);
        }
    }
    drop(rig);
    if body {
        for mut part in &mut rigs {
            if part.owner == owner && part.socket.is_some() {
                part.attached = false;
                part.socket_entity = None;
            }
        }
    }
}
fn attach_parts(
    mut commands: Commands,
    mut rigs: Query<(Entity, &mut ActorRig)>,
    mut evidence: ResMut<EnemyEvidence>,
) {
    let _span = info_span!("enemy_attachment_lifecycle").entered();
    let attachments: Vec<_> = rigs
        .iter()
        .filter(|(_, rig)| rig.socket.is_some() && rig.ready)
        .filter_map(|(entity, rig)| {
            let socket = rig.socket.as_ref()?;
            let parent = rigs.iter().find_map(|(_, body)| {
                (body.owner == rig.owner && body.primary && body.ready)
                    .then(|| body.sockets.get(socket).copied())
                    .flatten()
            })?;
            Some((entity, parent))
        })
        .collect();
    for (entity, parent) in attachments {
        let Ok((_, mut rig)) = rigs.get_mut(entity) else {
            continue;
        };
        if rig.attached && rig.socket_entity == Some(parent) {
            continue;
        }
        commands.entity(entity).insert(ChildOf(rig.owner));
        rig.socket_entity = Some(parent);
        rig.attached = true;
        evidence.attached_heads += 1;
    }
}
fn admit_ready(
    mut commands: Commands,
    enemies: Query<Entity, (With<Zombie>, Without<ActorReady>)>,
    rigs: Query<&ActorRig>,
    cache: Res<AnimationGraphs>,
) {
    for owner in &enemies {
        let mut parts = rigs.iter().filter(|r| r.owner == owner).peekable();
        if parts.peek().is_some()
            && parts.all(|r| {
                r.ready
                    && r.attached
                    && (r.socket.is_some() && r.players.is_empty()
                        || !r.players.is_empty() && cache.contains_scene(&r.source))
            })
        {
            commands.entity(owner).insert(ActorReady);
        }
    }
}
fn animate(
    mut commands: Commands,
    definition: Option<Res<GameDefinition>>,
    enemies: Query<&Zombie>,
    panzers: Query<&crate::panzer::Panzer>,
    mut cache: ResMut<AnimationGraphs>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut rigs: Query<&mut ActorRig>,
    mut evidence: ResMut<EnemyEvidence>,

) {
    let Some(definition) = definition else {
        return;
    };
    for mut rig in &mut rigs {
        if !rig.ready || !rig.attached || rig.players.is_empty() {
            continue;
        }
        let Ok(zombie) = enemies.get(rig.owner) else {
            continue;
        };
        let Some(enemy) = definition
            .content
            .enemies
            .iter()
            .find(|e| e.id == zombie.definition)
        else {
            continue;
        };
        let Some(bindings) = cache.scene(&rig.source, &gltfs, &mut graphs) else {
            continue;
        };
        let panzer = panzers.get(rig.owner).ok();
        let name = if let Some(panzer) = panzer {
            Some(panzer.animation().0)
        } else {
            zombie.animation_name(enemy)
        };
        let Some(name) = name else {
            continue;
        };
        let Some(&node) = bindings.named.get(name) else {
            if !rig.primary {
                continue;
            }
            panic!("{}", format!(
                "Required actor animation {name} missing from {:?}",
                rig.source.path()
            ));
        };
        let changed = rig.action != Some(zombie.action)
            || rig.clip.as_deref() != Some(name)
            || rig.sequence != Some(zombie.animation_sequence);
        let Some(duration) = gltfs
            .get(&rig.source)
            .and_then(|g| g.named_animations.get(name))
            .and_then(|h| clips.get(h))
            .map(AnimationClip::duration)
            .filter(|d| *d > 0.)
        else {
            continue;
        };
        let looping = panzer.map_or_else(
            || {
                definition
                    .content
                    .animations
                    .iter()
                    .find(|animation| {
                        animation.skeleton == rig.presentation
                            && animation.id.0.rsplit('.').next() == Some(name)
                    })
                    .is_some_and(|animation| animation.looping)
            },
            |p| p.animation().1,
        );
        let elapsed = panzer.map_or(zombie.elapsed, |p| p.animation_elapsed);
        let seek = if looping {
            elapsed.rem_euclid(duration)
        } else {
            elapsed.min(duration)
        };
        let mut playback = AnimationPlayback::new(
            bindings.graph.clone(),
            vec![AnimationLayer::new(node, Some(seek), looping)],
        );
        playback.restart = zombie.animation_sequence;
        for &entity in &rig.players {
            commands.entity(entity).insert(playback.clone());
        }
        if changed {
            evidence.animations.insert(format!("{:?}", zombie.action));
            if EnemyLocomotion::for_action(zombie.action).is_some() {
                evidence.locomotion_clips.insert(name.to_owned());
            }
            rig.action = Some(zombie.action);
            rig.clip = Some(name.to_owned());
            rig.sequence = Some(zombie.animation_sequence);
        }
    }
}

fn partial_roots(mut roots: Query<&mut Transform, With<PartialRoot>>) {
    for mut transform in &mut roots {
        if transform.translation != Vec3::ZERO {
            transform.translation = Vec3::ZERO;
        }
        let rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
    }
}
fn follow_sockets(
    mut poses: ParamSet<(
        Query<(&Transform, Option<&ChildOf>)>,
        Query<(Entity, &ActorRig, &mut Transform)>,
    )>,
) {
    let attachments: Vec<_> = poses
        .p1()
        .iter()
        .filter(|(_, rig, _)| rig.ready && rig.attached)
        .filter_map(|(entity, rig, _)| Some((entity, rig.socket_entity?, rig.owner)))
        .collect();
    let transforms = poses.p0();
    let followed: Vec<_> = attachments
        .into_iter()
        .filter_map(|(entity, socket, owner)| {
            let mut cursor = socket;
            let mut local = Mat4::IDENTITY;
            while cursor != owner {
                let Ok((pose, Some(parent))) = transforms.get(cursor) else {
                    return None;
                };
                local = pose.to_matrix() * local;
                cursor = parent.parent();
            }
            Some((
                entity,
                Transform::from_matrix(local * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2))
                    .with_scale(Vec3::ONE),
            ))
        })
        .collect();
    for (entity, followed) in followed {
        if let Ok((_, _, mut pose)) = poses.p1().get_mut(entity) {
            pose.set_if_neq(followed);
        }
    }
}
fn animated_bounds(
    mut meshes: Query<(
        &ActorMesh,
        &Mesh3d,
        &SkinnedMesh,
        &GlobalTransform,
        Has<DynamicSkinnedMeshBounds>,
        &mut Aabb,
    )>,
    joints: Query<&GlobalTransform>,
    enemies: Query<&Zombie>,
    mesh_assets: Res<Assets<Mesh>>,
    inverse_binds: Res<Assets<SkinnedMeshInverseBindposes>>,
    definition: Option<Res<GameDefinition>>,
    mut evidence: ResMut<EnemyEvidence>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (mesh, geometry, skin, transform, dynamic, mut aabb) in &mut meshes {
        let Ok(zombie) = enemies.get(mesh.owner) else {
            continue;
        };
        let native_ready = dynamic
            && aabb.center.is_finite()
            && aabb.half_extents.is_finite()
            && transform.to_matrix().is_finite()
            && transform.affine().matrix3.determinant().abs() > 1e-8
            && mesh_assets.get(&geometry.0).is_some_and(|geometry| {
                geometry.morph_targets().is_none()
                    && geometry.skinned_mesh_bounds().is_some_and(|bounds| {
                        !bounds.aabbs.is_empty()
                            && inverse_binds
                                .get(&skin.inverse_bindposes)
                                .is_some_and(|binds| {
                                    bounds.iter().all(|(index, _)| {
                                        let index = usize::from(index.0);
                                        binds.get(index).is_some_and(|bind| bind.is_finite())
                                            && skin.joints.get(index).is_some_and(|joint| {
                                                joints
                                                    .get(*joint)
                                                    .is_ok_and(|pose| pose.to_matrix().is_finite())
                                            })
                                    })
                                })
                    })
            });
        if native_ready {
            evidence.native_bounds_frames += 1;
            if aabb.is_changed() {
                evidence.bounds_updates += 1;
            }
            continue;
        }
        evidence.fallback_bounds_frames += 1;
        let margin = definition
            .content
            .enemies
            .iter()
            .find(|e| e.id == zombie.definition)
            .map_or(2.5, |e| e.bounds_margin);
        let inverse = transform.affine().inverse();
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for joint in &skin.joints {
            if let Ok(pose) = joints.get(*joint) {
                let p = inverse.transform_point3(pose.translation());
                min = min.min(p);
                max = max.max(p);
            }
        }
        if min.is_finite() && max.is_finite() {
            let next = Aabb::from_min_max(min - Vec3::splat(margin), max + Vec3::splat(margin));
            if aabb.center != next.center || aabb.half_extents != next.half_extents {
                *aabb = next;
                evidence.bounds_updates += 1;
            }
        }
    }
}

#[derive(Default)]
struct DistanceClock {
    player: Option<Entity>,
    remaining: f32,
}

fn replace_distant(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut clock: Local<DistanceClock>,
    mut hits: MessageReader<crate::damage::DamageApplied>,
    mut enemies: Query<
        (
            Entity,
            &mut Zombie,
            &Transform,
            &Vitality,
            &crate::rounds::PopulationKind,
        ),
        Without<crate::panzer::Panzer>,
    >,
    players: Query<(Entity, &Player, &Transform), Without<Zombie>>,
    query: SpatialQuery,
    volumes: Query<&crate::environment::hazards::BoundaryVolume>,
    mut session: ResMut<crate::gameplay::Session>,
    definition: Res<GameDefinition>,
    mut barriers: ResMut<crate::barriers::Barriers>,
    mut evidence: ResMut<crate::rounds::RoundEvidence>,
) {
    for hit in hits.read() {
        if let Ok((_, mut zombie, _, _, _)) = enemies.get_mut(hit.impact.target) {
            zombie.known_damage = matches!(
                hit.impact.source,
                crate::damage::ImpactSource::Weapon(_) | crate::damage::ImpactSource::Projectile(_)
            );
        }
    }
    let Ok((entity, player, body)) = players.single() else {
        return;
    };
    if clock.player != Some(entity) {
        clock.player = Some(entity);
        clock.remaining = 10.;
    }
    clock.remaining -= time.delta_secs();
    if clock.remaining > 0. {
        return;
    }
    clock.remaining += 10.;
    evidence.distance_checks += 1;
    let tunnel = query.shape_intersections(
        &Collider::sphere(0.03), body.translation - Vec3::Y * player.height * 0.5,
        Quat::IDENTITY, &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER),
    ).into_iter().any(|entity| matches!(volumes.get(entity),
        Ok(crate::environment::hazards::BoundaryVolume::Playable(zone))
        if matches!(zone.0.as_str(), "zone.air_stairs" | "zone.bolt_stairs" | "zone.fire_stairs" | "zone.ice_stairs")));
    let fov = crate::rendering::BASE_FOV_DEGREES.to_radians();
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let origin = body.translation - Vec3::Y * player.height * 0.5;
    let mut living = enemies
        .iter()
        .filter(|(_, _, _, health, _)| health.state() == LifeState::ActorAlive)
        .count();
    for (entity, zombie, pose, health, kind) in &mut enemies {
        if health.state() != LifeState::ActorAlive
            || *kind != crate::rounds::PopulationKind::Ordinary
            || zombie.stunned > 0.
            || zombie.traversal.as_ref().is_some_and(|m| m.alias == "rise")
        {
            continue;
        }
        let delta = pose.translation - Vec3::Y * 0.85 - origin;
        let distance = delta.length_squared();
        let radius = 38.608_f32.powi(2);
        if (distance < radius * if tunnel { 2.25 } else { 1. } && delta.y.abs() < 25.4)
            || (distance < radius * 3.
                && forward.dot(delta.normalize_or_zero()) >= (fov * 0.4).cos())
        {
            continue;
        }
        let current = health.current().unwrap_or(0);
        let unknown_damage = !zombie.known_damage && current < health.maximum();
        let replace =
            unknown_damage || living + session.pending as usize > 24 || current >= health.maximum();
        evidence.retired += 1;
        if replace {
            session.pending += 1;
            evidence.requeued += 1;
            if unknown_damage || current < definition.rounds.health(session.round) {
                commands.spawn((
                    crate::rounds::ReplacementHealth {
                        current,
                        sequence: evidence.retired,
                    },
                    crate::gameplay::SessionEntity,
                ));
            }
        }
        living = living.saturating_sub(1);
        barriers.release(entity);
        commands.entity(entity).despawn();
    }
}

fn wounds_visibility(
    game: Res<GameDefinition>,
    mut rigs: Query<(
        &mut ActorRig,
        &mut Visibility,
        &mut Transform,
        Option<&mut crate::hit_regions::HitRig>,
    )>,
    mut actors: Query<(
        &crate::combat_effects::Wounds,
        Option<&mut crate::hit_regions::Hitboxes>,
        Option<&crate::combat_effects::Armor>,
        Option<&Zombie>,
    )>,
) {
    for (wounds, hitboxes, _, zombie) in &mut actors {
        let Some(mut hitboxes) = hitboxes else {
            continue;
        };
        if zombie.is_some_and(|zombie| {
            game.content
                .enemies
                .iter()
                .any(|enemy| enemy.id == zombie.definition && enemy.gibs.is_some())
        }) {
            continue;
        }
        for (disabled, region) in [
            (wounds.headless, crate::weapons::HitRegion::Head),
            (wounds.crawler, crate::weapons::HitRegion::Leg),
            (wounds.left_arm, crate::weapons::HitRegion::LeftArm),
            (wounds.right_arm, crate::weapons::HitRegion::RightArm),
        ] {
            if disabled && !hitboxes.disabled.contains(&region) {
                hitboxes.disabled.push(region);
            }
        }
    }
    for (mut rig, mut visibility, mut pose, hit_rig) in &mut rigs {
        if let Ok((wounds, _, armor, zombie)) = actors.get(rig.owner) {
            let enemy = zombie.and_then(|zombie| {
                game.content
                    .enemies
                    .iter()
                    .find(|enemy| enemy.id == zombie.definition)
            });
            if let Some(enemy) = enemy
                && let Some(gibs) = &enemy.gibs
            {
                let models = wounds.body_gib.and_then(|kind| gibs.models.get(&kind));
                rig.primary = if let Some(models) = models {
                    rig.presentation == models.body
                } else {
                    enemy.presentation.as_ref() == Some(&rig.presentation)
                };
                let visible = if rig.replacement {
                    rig.presentation == gibs.beheaded && wounds.headless
                        || models.is_some_and(|models| {
                            rig.presentation == models.body || rig.presentation == models.legs
                        })
                } else if rig.socket.is_none() {
                    models.is_none()
                } else {
                    let hat = gibs
                        .hat
                        .as_ref()
                        .is_some_and(|hat| hat.presentation == rig.presentation);
                    !(wounds.headless && !hat || wounds.hatless && hat)
                };
                *visibility = if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if let Some(mut hit_rig) = hit_rig {
                    hit_rig.active = visible;
                }
            } else {
                if rig.ready && rig.attached {
                    *visibility = Visibility::Inherited;
                }
                if armor.is_some_and(|a| {
                    (a.broken && rig.socket.as_deref() == Some("j_helmet"))
                        || (a.powerplant_cover == 0
                            && rig.socket.as_deref() == Some("tag_powersupply"))
                }) {
                    *visibility = Visibility::Hidden;
                }
                if wounds.headless && rig.presentation.0.to_lowercase().contains("head") {
                    *visibility = Visibility::Hidden;
                }
                if wounds.hatless && rig.presentation.0.to_lowercase().contains("hat") {
                    *visibility = Visibility::Hidden;
                }
                if let Some(mut hit_rig) = hit_rig {
                    hit_rig.active = *visibility != Visibility::Hidden;
                }
            }
            if wounds.crawler && rig.socket.is_none() {
                pose.translation.y = zombie
                    .and_then(|zombie| {
                        game.content
                            .enemies
                            .iter()
                            .find(|enemy| enemy.id == zombie.definition)
                    })
                    .map_or(-0.35, |enemy| -body_center(enemy, true).y);
            }
        }
    }
}
fn wounded_joints(
    game: Res<GameDefinition>,
    rigs: Query<&ActorRig>,
    actors: Query<(&crate::combat_effects::Wounds, Option<&Zombie>)>,
    mut joints: Query<&mut Transform>,
) {
    for rig in &rigs {
        let Ok((wounds, zombie)) = actors.get(rig.owner) else {
            continue;
        };
        if zombie.is_some_and(|zombie| {
            game.content
                .enemies
                .iter()
                .any(|enemy| enemy.id == zombie.definition && enemy.gibs.is_some())
        }) {
            continue;
        }
        for (lost, name) in [
            (wounds.left_arm, "j_shoulder_le"),
            (wounds.right_arm, "j_shoulder_ri"),
            (wounds.crawler, "j_hip_le"),
            (wounds.crawler, "j_hip_ri"),
        ] {
            if lost
                && let Some(entity) = rig.sockets.get(name)
                && let Ok(mut pose) = joints.get_mut(*entity)
            {
                pose.scale = Vec3::ZERO;
            }
        }
    }
}

#[cfg(test)]
mod native_motion_tests {
    use super::*;

    #[test]
    fn source_corpse_slots_prefer_free_then_behind_then_outside_view_then_distance() {
        let entity = Entity::PLACEHOLDER;
        let row = |point| Some((entity, point));
        let eye = Vec3::ZERO;
        let forward = Vec3::NEG_Z;
        assert_eq!(corpse_slot(&[row(Vec3::Z), None, None], eye, forward), 1);
        assert_eq!(
            corpse_slot(&[row(Vec3::Z), row(Vec3::NEG_Z * 100.)], eye, forward),
            0
        );
        assert_eq!(
            corpse_slot(&[row(Vec3::X * 2.), row(Vec3::NEG_Z * 100.)], eye, forward),
            0
        );
        assert_eq!(
            corpse_slot(&[row(Vec3::NEG_Z), row(Vec3::NEG_Z * 2.)], eye, forward),
            1
        );
        assert_eq!(
            corpse_slot(&[row(Vec3::Z), row(Vec3::Z + Vec3::Y * 100.)], eye, forward),
            0
        );
    }

    #[test]
    fn source_locomotion_preserves_pauses_and_crosses_loop_seams() {
        let motion = crate::origins::content::AnimationRootMotion {
            seconds: 1.,
            translation: vec![
                [0., 0., 0., 0.],
                [0.2, 0.2, 0., 0.],
                [0.3, 0.2, 0., 0.],
                [1., 1., 0., 0.],
            ],
            rotation: vec![],
        };
        assert_eq!(motion.loop_displacement(0.21, 0.29), [0.; 3]);
        let displacement = motion.loop_displacement(0.95, 1.05);
        assert!((displacement[0] - (0.05 * 0.8 / 0.7 + 0.05)).abs() < 0.00001);
        assert_eq!(motion.loop_displacement(0., 2.), [2., 0., 0.]);
    }

    #[test]
    fn native_actor_bounds_preserve_feet_and_axis_alignment_under_visual_rotation() {
        let physics = crate::origins::content::EnemyPhysics {
            radius: 15. * 0.0254,
            height: 72. * 0.0254,
            crawler_height: 24. * 0.0254,
        };
        for crawler in [false, true] {
            let center = physics.center(crawler);
            for yaw in [0., 0.37, 1.2, std::f32::consts::PI] {
                let rotation = Quat::from_rotation_y(yaw);
                let shape = Collider::compound(vec![(
                    Vec3::ZERO,
                    rotation.inverse(),
                    Collider::cuboid(
                        physics.radius * 2.,
                        physics.height(crawler),
                        physics.radius * 2.,
                    ),
                )]);
                for point in [
                    Vec3::new(physics.radius - 0.0001, 0.0001, physics.radius - 0.0001),
                    Vec3::new(
                        -physics.radius + 0.0001,
                        physics.height(crawler) - 0.0001,
                        -physics.radius + 0.0001,
                    ),
                ] {
                    assert!(shape.contains_point(center, rotation, point));
                }
                for point in [
                    Vec3::new(physics.radius + 0.0001, center.y, 0.),
                    Vec3::new(0., -0.0001, 0.),
                    Vec3::new(0., physics.height(crawler) + 0.0001, 0.),
                    Vec3::new(0., center.y, physics.radius + 0.0001),
                ] {
                    assert!(!shape.contains_point(center, rotation, point));
                }
            }
        }
    }
}
