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

#[derive(Component)]
pub struct EnemyGoal {
    pub position: Vec3,
    pub face: Vec3,
    pub radius: f32,
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
    pub speed: f32,
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
    lure: Option<Entity>,
    last_target: Vec3,
    route_revision: u32,
    stuck: f32,
    traversal: Option<TraversalMotion>,
    traversal_clip: Option<String>,
    landing: f32,
    tear: Option<BoardTear>,
    eye_spawn: eyes::EyeSpawn,
}
impl Zombie {
    pub fn new(
        position: Vec3,
        definition: ContentId,
        gait: crate::rounds::Gait,
        speed: f32,
    ) -> Self {
        Self {
            definition,
            gait,
            speed,
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
            lure: None,
            last_target: position,
            route_revision: u32::MAX,
            stuck: 0.,
            traversal: None,
            traversal_clip: None,
            landing: 0.,
            tear: None,
            eye_spawn: eyes::EyeSpawn::Ordinary,
        }
    }
    pub fn completed_emerging(&self) -> bool {
        self.traversal.is_none()
            && self.tear.is_none()
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
        });
        self.traversal_clip = profile.animation.0.rsplit('.').next().map(str::to_owned);
        self.transition(EnemyAnimation::Traverse);
        self
    }
    pub(crate) fn dug_rising(
        mut self,
        profile: &crate::origins::content::EnemyTraversalDefinition,
    ) -> Self {
        self = self.rising(profile);
        self.eye_spawn = eyes::EyeSpawn::Dug;
        self
    }
    fn transition(&mut self, action: EnemyAnimation) {
        if self.action != action {
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
}
struct BoardTear {
    barrier: ContentId,
    board: usize,
    pulling: bool,
    destroyed: bool,
}
struct TraversalMotion {
    start: Vec3,
    end: Vec3,
    alias: String,
    rotation: Quat,
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
    pub(crate) fn socket(&self, owner: Entity, name: &str) -> Option<Entity> {
        (self.owner == owner && self.socket.is_none())
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
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct EnemyAnimationPlayback;
pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyEvidence>()
            .add_systems(
                Update,
                eyes::update
                    .after(crate::rendering::opaque::advance_native_time)
                    .before(crate::rendering::SurfacePreparation)
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
                (corpses, replace_distant)
                    .in_set(GameStep::Zombies)
                    .in_set(EnemyRetirement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    spawn_actors,
                    attach_parts,
                    wounds_visibility,
                    admit_ready,
                    animate.in_set(EnemyAnimationPlayback),
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (partial_roots, wounded_joints, follow_sockets)
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
                Transform::from_xyz(0., -0.85, 0.)
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
    effects: Res<crate::powerups::PowerupEffects>,
    player: Single<(Entity, &Transform, &Vitality), (With<Player>, Without<Zombie>)>,
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
    server: Res<AssetServer>,
    goals: Query<(&EnemyGoal, Option<&crate::buildables::ShieldAggro>)>,
) {
    let (player_entity, player_transform, health) = player.into_inner();
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
        let crawler_shape = if wounds.crawler && !zombie.crawler {
            zombie.crawler = true;
            transform.translation.y -= 0.5;
            let shape = Collider::capsule(0.3, 0.1);
            commands.entity(entity).insert(shape.clone());
            zombie.transition(EnemyAnimation::Idle);
            Some(shape)
        } else {
            None
        };
        let shape = crawler_shape.as_ref().unwrap_or(shape);
        zombie.burning = wounds.burning > 0.;
        let half_height = if zombie.crawler { 0.35 } else { 0.85 };
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
        if let Some(tear) = zombie.tear.as_ref() {
            let Some(window) = definition
                .content
                .barriers
                .iter()
                .find(|b| b.id == tear.barrier)
            else {
                continue;
            };
            let board = &window.boards[tear.board];
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
                let mut sample = motion.last().copied().unwrap_or([0.; 4]);
                for pair in motion.windows(2) {
                    if zombie.elapsed <= pair[1][0] {
                        let t = ((zombie.elapsed - pair[0][0]) / (pair[1][0] - pair[0][0]))
                            .clamp(0., 1.);
                        for axis in 1..4 {
                            sample[axis] = pair[0][axis] + (pair[1][axis] - pair[0][axis]) * t;
                        }
                        break;
                    }
                }
                transform.translation = Vec3::from_array(placement.position)
                    + Vec3::Y * half_height
                    + Quat::from_rotation_y(placement.yaw)
                        * Vec3::new(sample[1], sample[2], sample[3]);
            }
            if !tear.pulling && zombie.elapsed >= board.grab_seconds {
                barriers.begin_pull(&tear.barrier, tear.board, entity);
                zombie.tear.as_mut().unwrap().pulling = true;
                zombie.elapsed = 0.;
                zombie.traversal_clip = Some(board.tear_clip.clone());
            } else if tear.pulling {
                if !tear.destroyed && zombie.elapsed >= board.destroy_seconds {
                    if barriers.destroy(&tear.barrier, tear.board, entity) {
                        crate::barriers::sound(
                            &mut commands,
                            &server,
                            &definition,
                            &window.break_sound,
                            transform.translation,
                        );
                        barriers.sounds += 1;
                    }
                    zombie.tear.as_mut().unwrap().destroyed = true;
                }
                if zombie.elapsed >= board.tear_seconds {
                    barriers.release(entity);
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
            let Some(profile) = enemy.traversals.get(&motion.alias) else {
                continue;
            };
            transform.translation = motion.position(profile, zombie.elapsed);
            if zombie.elapsed >= profile.seconds {
                let filter = crate::environment::solids([entity]);
                let occupied = |p: Vec3| {
                    positions.iter().any(|(other, point)| {
                        *other != entity
                            && (point - p).xz().length_squared() < 0.49
                            && (point.y - p.y).abs() < 1.
                    })
                };
                let landing = if !occupied(motion.end)
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
                        &crate::environment::solids([entity, player_entity]),
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
                    if motion.alias == "rise" {
                        evidence.rising_finished += 1;
                    }
                    zombie.traversal = None;
                    zombie.traversal_clip = None;
                    zombie.landing = 0.25;
                    zombie.transition(EnemyAnimation::Landing);
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
        let lure = attractions.0.get(&entity);
        if zombie.lure != lure.map(|l| l.bomb) {
            zombie.lure = lure.map(|l| l.bomb);
            zombie.navigation = 0.;
            zombie.transition(EnemyAnimation::Idle);
        }
        let goal = lure
            .is_none()
            .then(|| goals.get(entity).ok().map(|(goal, _)| goal))
            .flatten();
        let ignored = lure.is_none()
            && goal.is_none()
            && (health.state() != LifeState::Standing || effects.zombie_blood > 0.);
        if ignored {
            zombie.transition(EnemyAnimation::Idle);
        }
        let target = if ignored {
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
        if zombie.action == EnemyAnimation::Attack
            && (wounds.melee_disabled()
                || ignored
                || lure.is_some()
                || distance > goal.map_or(1.65, |g| g.radius) + 0.3
                || !visible(transform.translation, target))
        {
            zombie.transition(EnemyAnimation::Idle);
        }
        if lure.is_none()
            && !wounds.melee_disabled()
            && !ignored
            && zombie.action != EnemyAnimation::Attack
            && distance < goal.map_or(1.65, |g| g.radius)
            && visible(transform.translation, target)
        {
            zombie.transition(EnemyAnimation::Attack);
            let state = if shield_target.is_some() {
                if zombie.crawler {
                    EnemyCombat::ShieldMeleeCrawl
                } else {
                    EnemyCombat::ShieldMelee
                }
            } else if goal.is_some() {
                EnemyCombat::GeneratorMelee
            } else if zombie.gait == crate::rounds::Gait::Walk {
                EnemyCombat::WalkMelee
            } else {
                EnemyCombat::RunMelee
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
                    && distance < 1.5
                    && visible(transform.translation, target)
                    && health.state() == LifeState::Standing
                {
                    impacts.0.push(crate::damage::Impact {
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
            zombie.destination = if distance < 4.
                && (transform.translation.y - target.y).abs() < 0.7
                && navigation.direct_allowed(feet, target - Vec3::Y * 0.9, &access)
                && visible(transform.translation, target)
            {
                evidence.direct_pursuits += 1;
                target
            } else {
                navigation.destination(feet, target - Vec3::Y * 0.9, &access)
                    + Vec3::Y * half_height
            };
        }
        if let Some(step) =
            navigation.traversal(feet, zombie.destination - Vec3::Y * half_height, &access)
        {
            let crate::navigation::TraversalStep {
                start,
                end,
                alias,
                yaw,
            } = step;
            if let Some(window) = definition.content.barriers.iter().find(|b| {
                definition
                    .content
                    .navigation_nodes
                    .iter()
                    .find(|n| n.id == b.entry)
                    .is_some_and(|n| {
                        let d = Vec3::from_array(n.position) - start;
                        d.xz().length_squared() < 0.25 && d.y.abs() < 0.8
                    })
            }) {
                if barriers.obstructed(&window.id) {
                    zombie.velocity = Vec3::ZERO;
                    if let Some(board) = barriers.claim(&window.id, entity) {
                        zombie.tear = Some(BoardTear {
                            barrier: window.id.clone(),
                            board,
                            pulling: false,
                            destroyed: false,
                        });
                        zombie.traversal_clip = Some(window.boards[board].grab_clip.clone());
                        zombie.transition(EnemyAnimation::Tear);
                        let delta = end - start;
                        transform.rotation = Quat::from_rotation_y((-delta.z).atan2(delta.x));
                        if let Some(placement) = definition
                            .content
                            .placements
                            .iter()
                            .find(|p| p.id == window.placement)
                        {
                            let sample = window.boards[board].grab_motion[0];
                            transform.translation = Vec3::from_array(placement.position)
                                + Vec3::Y * half_height
                                + Quat::from_rotation_y(placement.yaw)
                                    * Vec3::new(sample[1], sample[2], sample[3]);
                            transform.rotation = Quat::from_rotation_y(placement.yaw);
                        }
                    } else {
                        zombie.transition(EnemyAnimation::Idle);
                    }
                    continue;
                }
                barriers.entries += 1;
            }
            if let Some(profile) = enemy.traversals.get(&alias) {
                let rotation = Quat::from_rotation_y(yaw);
                zombie.traversal = Some(TraversalMotion {
                    start: start + Vec3::Y * half_height,
                    end: end + Vec3::Y * half_height,
                    alias,
                    rotation,
                });
                zombie.traversal_clip = profile.animation.0.rsplit('.').next().map(str::to_owned);
                zombie.transition(EnemyAnimation::Traverse);
                zombie.velocity = Vec3::ZERO;
                transform.rotation = rotation;
                commands.entity(entity).insert(ColliderDisabled);
                evidence.traversals_started += 1;
                continue;
            }
        }
        let delta = zombie.destination - transform.translation;
        let direction = Vec3::new(delta.x, 0., delta.z).normalize_or_zero();
        let moving = !ignored
            && zombie.action != EnemyAnimation::Attack
            && !(lure.is_some() && distance < 0.4);
        let speed = if zombie.tank_chasing {
            enemy
                .traversals
                .get("tank_super_sprint")
                .and_then(|p| {
                    p.motion
                        .last()
                        .map(|m| Vec3::new(m[1], m[2], m[3]).length() / p.seconds)
                })
                .unwrap_or(zombie.speed)
        } else {
            zombie.speed
        };
        zombie.velocity.x = direction.x
            * speed
            * if wounds.crawler { 0.2 } else { 1. }
            * wounds.movement_scale()
            * f32::from(moving);
        zombie.velocity.z = direction.z
            * speed
            * if wounds.crawler { 0.2 } else { 1. }
            * wounds.movement_scale()
            * f32::from(moving);
        if moving {
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
        if facing.xz().length_squared() > 0.001 {
            transform.rotation = Quat::from_rotation_y((-facing.z).atan2(facing.x));
        }
        let filter = crate::environment::solids([entity]);
        let support = query.cast_ray(feet + Vec3::Y * 0.1, Dir3::NEG_Y, 0.25, false, &filter);
        let grounded = support.is_some_and(|h| h.normal.y > 0.65);
        let before = transform.translation;
        let result = crate::player::slide_character(
            &slide,
            &query,
            crate::player::MotionShape {
                collider: shape,
                radius: 0.3,
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
                zombie.destination = navigation.alternate(
                    feet,
                    target - Vec3::Y * 0.9,
                    zombie.destination - Vec3::Y * half_height,
                    &access,
                ) + Vec3::Y * half_height;
                zombie.navigation = 0.5;
                zombie.stuck = 0.;
                evidence.stuck_recoveries += 1;
            }
            let action = if speed < 0.05 {
                EnemyAnimation::Idle
            } else if zombie.tank_chasing {
                EnemyAnimation::Sprint
            } else {
                match zombie.gait {
                    crate::rounds::Gait::Walk => EnemyAnimation::Walk,
                    crate::rounds::Gait::Run => EnemyAnimation::Run,
                    crate::rounds::Gait::Sprint => EnemyAnimation::Sprint,
                }
            };
            zombie.transition(action);
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
            &Transform,
            &mut Visibility,
        ),
        Without<crate::panzer::Panzer>,
    >,
    mut evidence: ResMut<EnemyEvidence>,
) {
    for (entity, mut zombie, vitality, wounds, pose, mut visibility) in &mut enemies {
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
            zombie.start_combat(
                enemy,
                if wounds.burned && !wounds.crawler {
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
        zombie.elapsed += time.delta_secs();
        let death_seconds = zombie
            .combat_clip(enemy)
            .map_or(enemy.death_seconds, |c| c.seconds);
        if zombie.elapsed >= death_seconds + enemy.corpse_seconds {
            commands.entity(entity).despawn();
            evidence.corpses_removed += 1;
        }
    }
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
        let scenes =
            enemy
                .presentation
                .iter()
                .map(|id| (id, None, None))
                .chain(enemy.parts.iter().map(|part| {
                    (
                        &part.presentation,
                        Some(part.socket.clone()),
                        part.root_joint.clone(),
                    )
                }));
        let mut pending = Vec::new();
        for (id, socket, partial_root) in scenes {
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
                    Transform::from_xyz(
                        0.,
                        if zombie.definition.0 == "enemy.panzer" {
                            -1.2
                        } else {
                            -0.85
                        },
                        0.,
                    )
                } else {
                    Transform::default()
                },
                if body {
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
    let body = rig.socket.is_none();
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
                    region: if r.region == crate::weapons::HitRegion::Arm {
                        if r.id.ends_with("_le") {
                            crate::weapons::HitRegion::LeftArm
                        } else {
                            crate::weapons::HitRegion::RightArm
                        }
                    } else {
                        r.region
                    },
                    start: *bones.get(&r.start)?,
                    end: *bones.get(&r.end)?,
                    radius: r.radius,
                })
            })
            .collect();
        commands
            .entity(event.entity)
            .insert(crate::hit_regions::HitRig {
                owner: rig.owner,
                bindings,
            });
    }
    for child in children.iter_descendants(event.entity) {
        if rig.socket.is_none()
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
        .filter(|(_, rig)| !rig.attached && rig.ready)
        .filter_map(|(entity, rig)| {
            let socket = rig.socket.as_ref()?;
            let parent = rigs.iter().find_map(|(_, body)| {
                (body.owner == rig.owner && body.socket.is_none() && body.ready)
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
        commands
            .entity(entity)
            .insert((ChildOf(rig.owner), Visibility::Inherited));
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
        } else if zombie.tank_animation.is_some() {
            zombie.tank_animation.as_deref()
        } else if matches!(
            zombie.action,
            EnemyAnimation::Traverse | EnemyAnimation::Tear
        ) {
            zombie.traversal_clip.as_deref()
        } else if let Some(clip) = zombie.combat_clip(enemy) {
            clip.animation.0.rsplit('.').next()
        } else if let Some(name) = zombie.locomotion_clip(enemy) {
            Some(name)
        } else {
            enemy
                .animation_actions
                .get(&if zombie.action == EnemyAnimation::Landing {
                    EnemyAnimation::Idle
                } else {
                    zombie.action
                })
                .and_then(|id| id.0.rsplit('.').next())
        };
        let Some(name) = name else {
            continue;
        };
        let Some(&node) = bindings.named.get(name) else {
            continue;
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
                matches!(
                    zombie.action,
                    EnemyAnimation::Idle
                        | EnemyAnimation::Walk
                        | EnemyAnimation::Run
                        | EnemyAnimation::Sprint
                )
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
pub(crate) fn locomotion_observation(world: &mut World, owner: Entity) -> Vec<serde_json::Value> {
    let rigs: Vec<_> = world
        .query::<(Entity, &ActorRig)>()
        .iter(world)
        .filter(|(_, r)| r.owner == owner)
        .map(|(entity, r)| {
            (
                entity,
                r.presentation.0.clone(),
                r.ready && r.attached,
                r.clip.clone(),
                r.players.clone(),
                r.sockets
                    .get("j_head")
                    .copied()
                    .or_else(|| r.sockets.get("j_spineupper").copied()),
            )
        })
        .collect();
    rigs.into_iter()
        .map(|(entity, rig, ready, clip, players, joint)| {
            let mut joint = joint;
            if joint.is_none() {
                let mut descendants = vec![entity];
                while let Some(child) = descendants.pop() {
                    if world
                        .get::<Name>(child)
                        .is_some_and(|n| n.as_str() == "j_head")
                    {
                        joint = Some(child);
                        break;
                    }
                    if let Some(children) = world.get::<Children>(child) {
                        descendants.extend(children.iter());
                    }
                }
            }
            let clocks: Vec<_> = players
                .iter()
                .filter_map(|e| world.get::<AnimationPlayer>(*e))
                .flat_map(|p| p.playing_animations().map(|(_, a)| a.seek_time()))
                .collect();
            let pose = joint
                .and_then(|e| world.get::<Transform>(e))
                .map(|t| t.rotation.to_array());
            serde_json::json!({"rig":rig,"ready":ready,"clip":clip,"clocks":clocks,"pose":pose})
        })
        .collect()
}
#[cfg(debug_assertions)]
pub(crate) fn animation_observation(world: &mut World) -> Vec<serde_json::Value> {
    let actors: Vec<_> = world.query::<(Entity, &Zombie, &Transform)>().iter(world).map(|(entity,zombie,transform)| {
        let game = world.resource::<GameDefinition>();
        let clip = game.content.enemies.iter().find(|e|e.id == zombie.definition).and_then(|e|zombie.combat_clip(e));
        (entity, serde_json::json!({"entity":entity.to_bits(),"action":format!("{:?}",zombie.action),
            "position":transform.translation.to_array(),"forward":(transform.rotation * Vec3::X).to_array(),
            "gait":format!("{:?}",zombie.gait),"elapsed":zombie.elapsed,"sequence":zombie.animation_sequence,
            "combat_state":zombie.combat.map(|(s,_)|format!("{s:?}")),"combat_clip":clip.map(|c|&c.animation.0),
            "seconds":clip.map(|c|c.seconds),"hits":clip.map(|c|&c.hits)}))
    }).collect();
    actors
        .into_iter()
        .map(|(entity, mut observation)| {
            observation["rigs"] = serde_json::json!(locomotion_observation(world, entity));
            observation
        })
        .collect()
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
                Transform::from_matrix(local * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)),
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
    mut rigs: Query<(&ActorRig, &mut Visibility, &mut Transform)>,
    mut actors: Query<(
        &crate::combat_effects::Wounds,
        Option<&mut crate::hit_regions::Hitboxes>,
        Option<&crate::combat_effects::Armor>,
    )>,
) {
    for (wounds, hitboxes, _) in &mut actors {
        let Some(mut hitboxes) = hitboxes else {
            continue;
        };
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
    for (rig, mut visibility, mut pose) in &mut rigs {
        if let Ok((wounds, _, armor)) = actors.get(rig.owner) {
            if armor.is_some_and(|a| {
                (a.broken && rig.socket.as_deref() == Some("j_helmet"))
                    || (a.powerplant_cover == 0 && rig.socket.as_deref() == Some("tag_powersupply"))
            }) {
                *visibility = Visibility::Hidden;
            }
            if wounds.headless && rig.presentation.0.to_lowercase().contains("head") {
                *visibility = Visibility::Hidden;
            }
            if wounds.crawler && rig.socket.is_none() {
                pose.translation.y = -0.35;
            }
        }
    }
}
fn wounded_joints(
    rigs: Query<&ActorRig>,
    actors: Query<&crate::combat_effects::Wounds>,
    mut joints: Query<&mut Transform>,
) {
    for rig in &rigs {
        let Ok(wounds) = actors.get(rig.owner) else {
            continue;
        };
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
