//! Ordinary enemy lifecycle and Bevy-owned original multipart actor playback.
use crate::hit_regions::SkeletalReady as ActorReady;
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Navigation},
    life::{LifeState, Vitality},
    origins::content::{ContentId, EnemyAnimation, EnemyDefinition, EnemyLocomotion},
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

/// A positional objective uses the same navigation and attack lifecycle as pursuit.
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
    crate::hit_regions::Hitboxes
)]
pub struct Zombie {
    pub definition: ContentId,
    pub gait: crate::rounds::Gait,
    pub speed: f32,
    velocity: Vec3,
    navigation: f32,
    destination: Vec3,
    pub action: EnemyAnimation,
    pub elapsed: f32,
    pub stunned: f32,
    locomotion_seed: u32,
    next_hit: usize,
    lure: Option<Entity>,
    last_target: Vec3,
    route_revision: u32,
    stuck: f32,
    traversal: Option<TraversalMotion>,
    traversal_clip: Option<String>,
    landing: f32,
    tear: Option<BoardTear>,
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
            velocity: Vec3::ZERO,
            navigation: 0.,
            destination: position,
            action: EnemyAnimation::Idle,
            elapsed: 0.,
            stunned: 0.,
            locomotion_seed: position
                .x
                .to_bits()
                .wrapping_add(position.z.to_bits().rotate_left(13)),
            next_hit: 0,
            lure: None,
            last_target: position,
            route_revision: u32::MAX,
            stuck: 0.,
            traversal: None,
            traversal_clip: None,
            landing: 0.,
            tear: None,
        }
    }
    pub fn with_locomotion_seed(mut self, seed: u32) -> Self {
        self.set_locomotion_seed(seed);
        self
    }
    pub(crate) fn set_locomotion_seed(&mut self, seed: u32) {
        self.locomotion_seed = seed;
    }
    pub(crate) fn locomotion_clip<'a>(&self, enemy: &'a EnemyDefinition) -> Option<&'a str> {
        let state = EnemyLocomotion::for_action(self.action)?;
        let clips = enemy.locomotion_states.get(&state)?;
        if clips.is_empty() {
            return None;
        }
        // BO2's zm_run preserves ASD substates. Keep this actor's choice across
        // stops, attacks and repeated movement updates, shared by body and head.
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
    fn transition(&mut self, action: EnemyAnimation) {
        if self.action != action {
            self.action = action;
            self.elapsed = 0.;
            self.next_hit = 0;
        }
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
struct ActorRig {
    presentation: ContentId,
    owner: Entity,
    source: Handle<Gltf>,
    socket: Option<String>,
    partial_root: Option<String>,
    ready: bool,
    attached: bool,
    players: Vec<Entity>,
    sockets: HashMap<String, Entity>,
    action: Option<EnemyAnimation>,
    clip: Option<String>,
}
#[derive(Component)]
struct PartialRoot;
#[derive(Component)]
struct ActorMesh {
    owner: Entity,
}
#[derive(Component)]
struct HasActor;
#[derive(Component)]
struct ActorLit;
#[derive(Resource, Default)]
struct Graphs(
    HashMap<AssetId<Gltf>, (Handle<AnimationGraph>, BTreeMap<String, AnimationNodeIndex>)>,
);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct EnemyBounds;
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct EnemyAnimationPlayback;
pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Graphs>()
            .init_resource::<EnemyEvidence>()
            .add_systems(
                FixedUpdate,
                lifecycle
                    .in_set(GameStep::EnemyIntent)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                corpses
                    .in_set(GameStep::Zombies)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    spawn_actors,
                    graphs,
                    attach_parts,
                    light_materials,
                    admit_ready,
                    animate.in_set(EnemyAnimationPlayback),
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                partial_roots
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
pub(crate) fn lifecycle(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut navigation: ResMut<Navigation>,
    access: Res<crate::access::ZoneAccess>,
    attractions: Res<crate::lures::Attractions>,
    players: Query<(Entity, &Transform, &Vitality), (With<Player>, Without<Zombie>)>,
    mut enemies: Query<
        (
            Entity,
            &mut Zombie,
            &mut Transform,
            &Collider,
            &Vitality,
            Option<&ActorReady>,
        ),
        Without<Player>,
    >,
    slide: MoveAndSlide,
    query: SpatialQuery,
    mut impacts: ResMut<crate::damage::DamageQueue>,
    mut evidence: ResMut<EnemyEvidence>,
    mut barriers: ResMut<crate::barriers::Barriers>,
    server: Res<AssetServer>,
    goals: Query<&EnemyGoal>,
) {
    let Ok((player_entity, player_transform, health)) = players.single() else {
        return;
    };
    let dt = time.delta_secs();
    let mut positions: Vec<_> = enemies
        .iter()
        .filter(|(_, _, _, _, v, _)| v.state() == LifeState::ActorAlive)
        .map(|(e, _, t, _, _, _)| (e, t.translation))
        .collect();
    for (entity, mut zombie, mut transform, shape, vitality, ready) in &mut enemies {
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
        if ready.is_none() {
            continue;
        }
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
        zombie.elapsed += dt;
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
                // Native aligned clips include the authoring origin's floor offset.
                transform.translation = Vec3::from_array(placement.position)
                    + Vec3::Y * 0.85
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
        let goal = lure.is_none().then(|| goals.get(entity).ok()).flatten();
        let ignored = lure.is_none() && goal.is_none() && health.state() != LifeState::Standing;
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
        let filter = crate::environment::solids([entity, player_entity]);
        let visible = |from: Vec3, to: Vec3| {
            let delta = to - from;
            Dir3::new(delta).ok().is_some_and(|d| {
                query
                    .cast_ray(from, d, delta.length(), false, &filter)
                    .is_none()
            })
        };
        let distance = transform.translation.distance(target);
        if lure.is_none()
            && !ignored
            && zombie.action != EnemyAnimation::Attack
            && distance < goal.map_or(1.65, |g| g.radius)
            && visible(transform.translation, target)
        {
            zombie.transition(EnemyAnimation::Attack);
            evidence.attacks_started += 1;
        }
        if zombie.action == EnemyAnimation::Attack {
            if let Some(goal) = goal {
                let facing = goal.face - transform.translation;
                transform.rotation = Quat::from_rotation_y((-facing.z).atan2(facing.x));
            }
            while zombie.next_hit < enemy.attack_hits.len()
                && zombie.elapsed >= enemy.attack_hits[zombie.next_hit]
            {
                zombie.next_hit += 1;
                if goal.is_none()
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
            if zombie.elapsed >= enemy.attack_seconds {
                zombie.transition(EnemyAnimation::Idle);
            }
        }
        let feet = transform.translation - Vec3::Y * 0.85;
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
                navigation.destination(feet, target - Vec3::Y * 0.9, &access) + Vec3::Y * 0.85
            };
        }
        if let Some(step) = navigation.traversal(feet, zombie.destination - Vec3::Y * 0.85, &access)
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
                        // Source aligned tear clips and board delta curves share this origin.
                        if let Some(placement) = definition
                            .content
                            .placements
                            .iter()
                            .find(|p| p.id == window.placement)
                        {
                            let sample = window.boards[board].grab_motion[0];
                            transform.translation = Vec3::from_array(placement.position)
                                + Vec3::Y * 0.85
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
                // zm_shared::dotraverse faces the authored negotiation start angle.
                let rotation = Quat::from_rotation_y(yaw);
                zombie.traversal = Some(TraversalMotion {
                    start: start + Vec3::Y * 0.85,
                    end: end + Vec3::Y * 0.85,
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
        zombie.velocity.x = direction.x * zombie.speed * f32::from(moving);
        zombie.velocity.z = direction.z * zombie.speed * f32::from(moving);
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
        if direction.length_squared() > 0.001 {
            transform.rotation = Quat::from_rotation_y((-direction.z).atan2(direction.x));
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
                    zombie.destination - Vec3::Y * 0.85,
                    &access,
                ) + Vec3::Y * 0.85;
                zombie.navigation = 0.5;
                zombie.stuck = 0.;
                evidence.stuck_recoveries += 1;
            }
            let action = if speed < 0.05 {
                EnemyAnimation::Idle
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
                        end - Vec3::Y * 0.85,
                        candidate - Vec3::Y * 0.85,
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
    mut enemies: Query<(Entity, &mut Zombie, &Vitality)>,
    mut evidence: ResMut<EnemyEvidence>,
) {
    for (entity, mut zombie, vitality) in &mut enemies {
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
            commands.entity(entity).insert(ColliderDisabled);
            evidence.deaths += 1;
        }
        zombie.elapsed += time.delta_secs();
        if zombie.elapsed >= enemy.death_seconds + enemy.corpse_seconds {
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
            let root = commands
                .spawn((
                    WorldAssetRoot(server.load(scene.to_owned())),
                    ActorRig {
                        presentation: id.clone(),
                        owner,
                        source,
                        socket,
                        partial_root,
                        ready: false,
                        attached: body,
                        players: Vec::new(),
                        sockets: HashMap::new(),
                        action: None,
                        clip: None,
                    },
                    if body {
                        Transform::from_xyz(0., -0.85, 0.)
                    } else {
                        Transform::default()
                    },
                    if body {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                ))
                .id();
            commands.entity(owner).add_child(root);
        }
        commands.entity(owner).insert(HasActor);
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
            // Preserve the first matching native descendant, just as the
            // attachment lookup did, but discover it once when the scene is ready.
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
}
fn graphs(
    mut cache: ResMut<Graphs>,
    rigs: Query<&ActorRig>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for rig in &rigs {
        if cache.0.contains_key(&rig.source.id()) {
            continue;
        }
        let Some(gltf) = gltfs.get(&rig.source) else {
            continue;
        };
        let mut clips: Vec<_> = gltf.named_animations.iter().collect();
        clips.sort_by(|a, b| a.0.cmp(b.0));
        let (graph, nodes) = AnimationGraph::from_clips(clips.iter().map(|(_, h)| (*h).clone()));
        let named = clips
            .into_iter()
            .zip(nodes)
            .map(|((n, _), node)| (n.to_string(), node))
            .collect();
        cache.0.insert(rig.source.id(), (graphs.add(graph), named));
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
        commands.entity(parent).add_child(entity);
        commands.entity(entity).insert((
            Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            Visibility::Inherited,
        ));
        rig.attached = true;
        evidence.attached_heads += 1;
    }
}
fn light_materials(
    mut commands: Commands,
    meshes: Query<
        Entity,
        (
            With<ActorMesh>,
            Without<ActorLit>,
            With<crate::rendering::PbrReady>,
        ),
    >,
) {
    for entity in &meshes {
        commands.entity(entity).insert(ActorLit);
    }
}
fn admit_ready(
    mut commands: Commands,
    enemies: Query<Entity, (With<Zombie>, Without<ActorReady>)>,
    rigs: Query<&ActorRig>,
    cache: Res<Graphs>,
) {
    for owner in &enemies {
        let parts: Vec<_> = rigs.iter().filter(|r| r.owner == owner).collect();
        if !parts.is_empty()
            && parts.iter().all(|r| {
                r.ready
                    && r.attached
                    && !r.players.is_empty()
                    && cache.0.contains_key(&r.source.id())
            })
        {
            commands.entity(owner).insert(ActorReady);
        }
    }
}
fn animate(
    mut commands: Commands,
    state: Res<State<GameState>>,
    definition: Option<Res<GameDefinition>>,
    enemies: Query<&Zombie>,
    cache: Res<Graphs>,
    mut rigs: Query<&mut ActorRig>,
    mut players: Query<&mut AnimationPlayer>,
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
        let Some((graph, nodes)) = cache.0.get(&rig.source.id()) else {
            continue;
        };
        let name = if matches!(
            zombie.action,
            EnemyAnimation::Traverse | EnemyAnimation::Tear
        ) {
            zombie.traversal_clip.as_deref()
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
        let Some(&node) = nodes.get(name) else {
            continue;
        };
        let changed = rig.action != Some(zombie.action) || rig.clip.as_deref() != Some(name);
        for &entity in &rig.players {
            let Ok(mut player) = players.get_mut(entity) else {
                continue;
            };
            if changed {
                commands
                    .entity(entity)
                    .insert(AnimationGraphHandle(graph.clone()));
                player.stop_all();
                let active = player.play(node);
                if matches!(
                    zombie.action,
                    EnemyAnimation::Idle
                        | EnemyAnimation::Walk
                        | EnemyAnimation::Run
                        | EnemyAnimation::Sprint
                ) {
                    active.repeat();
                }
                evidence.animations.insert(format!("{:?}", zombie.action));
                if EnemyLocomotion::for_action(zombie.action).is_some() {
                    evidence.locomotion_clips.insert(name.to_owned());
                }
            }
            if player
                .animation(node)
                .is_some_and(|active| active.seek_time() != zombie.elapsed)
                && let Some(active) = player.animation_mut(node)
            {
                active.seek_to(zombie.elapsed);
            }
            if **state == GameState::Playing && zombie.stunned <= 0. {
                if player
                    .playing_animations()
                    .any(|(_, active)| active.is_paused())
                {
                    player.resume_all();
                }
            } else if !player.all_paused() {
                player.pause_all();
            }
        }
        if changed {
            rig.action = Some(zombie.action);
            rig.clip = Some(name.to_owned());
        }
    }
}
/// Read the live multipart Bevy animation players for opt-in window validation.
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
            // Attachment rigs intentionally do not build the torso's socket
            // lookup. Observe their actual named native descendant directly.
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
        // Bevy's CalculateBounds has already transformed each influencing
        // joint's vertex envelope. Do not overwrite it with an envelope of
        // every joint in the multipart rig plus an unrelated limb margin.
        // Missing data or unsupported morph deformation keeps the old fallback.
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
