use super::{ActorRig, ClientRagdollRequested, Zombie};
use crate::{GameState, assets::GameDefinition, origins::content::RagdollJointKind};
use avian3d::prelude::*;
use bevy::{ecs::system::SystemParam, prelude::*};
use std::collections::HashMap;
use std::time::Duration;

const CLIENT_PHYSICS_LAYER: u32 = 128;
pub(super) fn client_physics_filter() -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask(CLIENT_PHYSICS_LAYER)
}
#[derive(Resource, Default)]
struct ClientPhysicsClock {
    previous: Option<i32>,
    delta: Duration,
}
impl ClientPhysicsClock {
    fn advance(&mut self, milliseconds: Option<i32>, active: bool, paused: bool) -> Duration {
        self.delta = Duration::ZERO;
        if !active {
            self.previous = None;
        } else if paused {
            self.previous = milliseconds;
        } else if let Some(milliseconds) = milliseconds {
            let elapsed = milliseconds - self.previous.unwrap_or(0);
            self.previous = Some(milliseconds);
            if elapsed != 0 {
                self.delta = Duration::from_millis(elapsed.clamp(1, 100) as u64);
            }
        }
        self.delta
    }
}
#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) struct ClientPhysicsEffects;
pub(super) fn client_scene(state: Res<State<GameState>>) -> bool {
    matches!(
        **state,
        GameState::Playing | GameState::GameOver | GameState::Ending
    )
}

fn client_clock(
    source: Res<crate::session::SessionClock>,
    time: Res<Time<Virtual>>,
    state: Res<State<GameState>>,
    mut clock: ResMut<ClientPhysicsClock>,
    mut physics: ResMut<Time<Physics>>,
    mut substeps: ResMut<Time<Substeps>>,
    count: Res<SubstepCount>,
) {
    let active = matches!(
        **state,
        GameState::StartingSession
            | GameState::Playing
            | GameState::Paused
            | GameState::GameOver
            | GameState::Ending
    );
    let delta = clock.advance(
        Some(source.milliseconds(&time)),
        active,
        **state == GameState::Paused,
    );
    advance_clocks(&mut physics, &mut substeps, count.0, delta);
}

fn advance_clocks(
    physics: &mut Time<Physics>,
    substeps: &mut Time<Substeps>,
    count: u32,
    delta: Duration,
) {
    physics.pause();
    physics.advance_by(delta);
    substeps.advance_by(delta.div_f64(count as f64));
}

#[derive(Component)]
struct RagdollEntity(Entity);
#[derive(Component)]
struct RagdollCollider {
    owner: Entity,
    index: usize,
    pairs: u16,
    self_only: bool,
}
#[derive(Message, Clone, Copy)]
pub(crate) struct RagdollExplosion {
    pub(crate) center: Vec3,
    pub(crate) inner_radius: f32,
    pub(crate) radius: f32,
    pub(crate) direction: Option<(Vec3, f32)>,
    pub(crate) magnitude: f32,
}
impl RagdollExplosion {
    pub(super) fn impulse(&self, point: Vec3, scale: f32) -> Vec3 {
        let delta = point - self.center;
        let distance = delta.length();
        if distance >= self.radius || self.radius <= 0. || !self.radius.is_finite() {
            return Vec3::ZERO;
        }
        if let Some((forward, cosine)) = self.direction
            && distance > 0.001
            && forward.dot(delta / distance) < cosine
        {
            return Vec3::ZERO;
        }
        let attenuation = if distance > self.inner_radius && self.inner_radius < self.radius {
            (self.radius - distance) / (self.radius - self.inner_radius)
        } else {
            1.
        };
        let direction = self
            .direction
            .map_or_else(|| delta.normalize_or_zero(), |(forward, _)| forward);
        (direction + Vec3::Y * 0.8).normalize_or_zero()
            * (scale * self.magnitude * attenuation * 0.0254)
    }
}

pub(super) fn blast_visible(
    query: &SpatialQuery,
    actors: &Query<(), With<crate::life::Vitality>>,
    center: Vec3,
    point: Vec3,
) -> bool {
    let delta = point - center;
    Dir3::new(delta).map_or(true, |direction| {
        query
            .cast_ray_predicate(
                center,
                direction,
                delta.length(),
                true,
                &crate::environment::solids([]),
                &|entity| !actors.contains(entity),
            )
            .is_none()
    })
}

#[derive(Component)]
struct DeathBlasts(Vec<RagdollExplosion>);
#[derive(Message)]
pub(crate) struct RagdollBullet {
    pub(crate) weapon: crate::origins::WeaponId,
    pub(crate) origin: Vec3,
    pub(crate) end: Vec3,
}
#[derive(SystemParam)]
pub(crate) struct RagdollCollisionHooks<'w, 's> {
    parts: Query<'w, 's, &'static RagdollCollider>,
    actors: Query<'w, 's, (), Or<(With<crate::player::Player>, With<Zombie>)>>,
}
impl CollisionHooks for RagdollCollisionHooks<'_, '_> {
    fn filter_pairs(&self, a: Entity, b: Entity, _: &mut Commands) -> bool {
        match (self.parts.get(a).ok(), self.parts.get(b).ok()) {
            (Some(a), Some(b)) if a.owner == b.owner => {
                a.self_only && b.self_only && a.pairs & (1 << b.index) != 0
            }
            (Some(a), Some(b)) => !a.self_only && !b.self_only,
            (Some(part), None) => !part.self_only && !self.actors.contains(b),
            (None, Some(part)) => !part.self_only && !self.actors.contains(a),
            (None, None) => true,
        }
    }
}
pub(super) fn dynamic_piece(owner: Entity) -> impl Bundle {
    (
        RagdollCollider {
            owner,
            index: 0,
            pairs: 0,
            self_only: false,
        },
        CollisionLayers::new(
            CLIENT_PHYSICS_LAYER,
            crate::assets::DYNENT_WORLD_LAYER | CLIENT_PHYSICS_LAYER,
        ),
        ActiveCollisionHooks::FILTER_PAIRS,
    )
}
#[derive(Component)]
struct PendingRagdoll {
    poses: Vec<Option<Transform>>,
}
#[derive(Component)]
struct RejectedRagdoll;
#[derive(Component)]
pub(super) struct PhysicalRagdoll {
    bodies: Vec<Option<Entity>>,
    seconds: f32,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ClientPhysicsClock>()
        .add_systems(
            OnEnter(GameState::StartingSession),
            |mut clock: ResMut<ClientPhysicsClock>| *clock = default(),
        )
        .add_message::<RagdollExplosion>()
        .add_message::<RagdollBullet>()
        .add_systems(PostUpdate, client_clock.in_set(PhysicsSystems::First))
        .add_systems(
            PostUpdate,
            begin
                .after(bevy::transform::TransformSystems::Propagate)
                .run_if(client_scene),
        )
        .add_systems(
            PostUpdate,
            apply_pose
                .in_set(crate::animation::AnimationPoseModifiers)
                .after(bevy::app::AnimationSystems)
                .after(PhysicsSystems::Writeback)
                .after(ClientPhysicsEffects)
                .after(super::wounded_joints)
                .before(super::follow_sockets)
                .before(bevy::transform::TransformSystems::Propagate)
                .run_if(client_scene),
        )
        .add_systems(
            PostUpdate,
            (bullets, explosions)
                .chain()
                .in_set(ClientPhysicsEffects)
                .after(PhysicsSystems::Writeback)
                .before(bevy::transform::TransformSystems::Propagate)
                .run_if(client_scene),
        )
        .add_systems(
            PhysicsSchedule,
            joint_friction
                .after(PhysicsStepSystems::NarrowPhase)
                .before(PhysicsStepSystems::Solver)
                .run_if(client_scene),
        )
        .add_systems(
            PostUpdate,
            tick.after(apply_pose)
                .before(bevy::transform::TransformSystems::Propagate)
                .run_if(client_scene),
        )
        .add_systems(FixedUpdate, cleanup);
}

fn bullets(
    mut commands: Commands,
    mut events: MessageReader<RagdollBullet>,
    game: Res<GameDefinition>,
    effects: Res<crate::native_fx::Catalog>,
    actors: Query<(
        &Zombie,
        &PhysicalRagdoll,
        &Transform,
        &crate::hit_regions::Hitboxes,
    )>,
    mut bodies: Query<Forces>,
) {
    for event in events.read() {
        let delta = event.end - event.origin;
        let Ok(direction) = Dir3::new(delta) else {
            continue;
        };
        let range = delta.length();
        let Some((zombie, ragdoll, part, hit)) = actors
            .iter()
            .filter_map(|(zombie, ragdoll, pose, boxes)| {
                boxes
                    .cast_part(pose, event.origin, direction, range)
                    .map(|(part, hit)| (zombie, ragdoll, part, hit))
            })
            .min_by(|a, b| a.3.distance.total_cmp(&b.3.distance))
        else {
            continue;
        };
        if let Some(effect) = game
            .content
            .weapons
            .iter()
            .find(|weapon| weapon.id == event.weapon)
            .and_then(|weapon| {
                effects.impact(
                    weapon.mechanics.impact_type,
                    7,
                    2 | u32::from(hit.region.head()),
                )
            })
        {
            crate::native_fx::emit(
                &mut commands,
                effect,
                effects.explosion_pose(hit.point, -*direction),
            );
        }
        let Some(profile) = game
            .content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
            .and_then(|enemy| enemy.ragdoll.as_ref())
        else {
            continue;
        };
        let part = part.rsplit('.').next().unwrap_or(part);
        let Some(index) = profile.bones.iter().position(|bone| bone.end_joint == part) else {
            continue;
        };
        let Some(entity) = ragdoll.bodies[index] else {
            continue;
        };
        if let Ok(mut body) = bodies.get_mut(entity) {
            body.apply_force_at_point(direction.as_vec3() * (0.8 * 3. * 0.0254), hit.point);
        }
    }
}

fn explosions(
    mut commands: Commands,
    mut events: MessageReader<RagdollExplosion>,
    actors: Query<(Entity, &PhysicalRagdoll)>,
    dying: Query<
        (
            Entity,
            &crate::life::Vitality,
            &Transform,
            Option<&DeathBlasts>,
        ),
        (
            With<Zombie>,
            Without<PhysicalRagdoll>,
            Without<RejectedRagdoll>,
        ),
    >,
    targets: Query<(), With<crate::life::Vitality>>,
    query: SpatialQuery,
    mut random: ResMut<super::gibs::GibCounter>,
    mut bodies: Query<(Forces, &ComputedCenterOfMass)>,
) {
    let mut pending = HashMap::<Entity, Vec<RagdollExplosion>>::new();
    for event in events.read() {
        if event.radius <= 0. || !event.radius.is_finite() {
            continue;
        }
        // Damage has resolved in FixedUpdate, but an explosion victim may still
        // be playing its death animation. Seed its first physical pose with the
        // blast instead of losing the event before the bodies exist.
        for (owner, vitality, pose, previous) in &dying {
            if vitality.state() == crate::life::LifeState::ActorDead
                && event.impulse(pose.translation, 18.) != Vec3::ZERO
                && blast_visible(&query, &targets, event.center, pose.translation)
            {
                pending
                    .entry(owner)
                    .or_insert_with(|| previous.map_or_else(Vec::new, |blasts| blasts.0.clone()))
                    .push(*event);
            }
        }
        for (_, ragdoll) in &actors {
            let count = ragdoll.bodies.iter().flatten().count();
            for entity in ragdoll.bodies.iter().flatten() {
                let Ok((mut body, center)) = bodies.get_mut(*entity) else {
                    continue;
                };
                let center = body.position() + body.rotation() * center.0;
                let impulse = event.impulse(center, 18. / count as f32);
                if impulse == Vec3::ZERO || !blast_visible(&query, &targets, event.center, center) {
                    continue;
                }
                let jitter =
                    Vec3::new(random.random(), random.random(), random.random()) * 2. - Vec3::ONE;
                body.apply_linear_impulse_at_point(impulse, center + jitter * 0.0254);
            }
        }
    }
    for (owner, blasts) in pending {
        commands
            .entity(owner)
            .insert((ClientRagdollRequested, DeathBlasts(blasts)));
    }
}

fn capsule(radius: f32, half_length: f32, root: bool, scale: f32) -> (Collider, Vec3) {
    let center = if root { Vec3::Y } else { Vec3::X } * half_length;
    (
        Collider::compound(vec![(
            center,
            Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
            Collider::capsule(radius * scale, half_length * 2.),
        )]),
        center,
    )
}

fn initial_pose(
    profile: &crate::origins::content::EnemyRagdoll,
    snapshot: &[Option<Transform>],
    trace: impl Fn(Vec3, Vec3) -> (bool, f32),
) -> Option<Vec<Option<Transform>>> {
    let root = snapshot.first().copied().flatten()?.translation;
    let mut placed = snapshot.to_vec();
    for (index, bone) in profile.bones.iter().enumerate() {
        let Some(pose) = snapshot[index] else {
            continue;
        };
        let rotation = pose.rotation
            * if bone.mirror {
                Quat::from_rotation_y(std::f32::consts::PI)
            } else {
                Quat::IDENTITY
            };
        let length = bone.half_length * 2. - if index == 3 { 4. * 0.0254 } else { 0. };
        if index == 0 {
            let end = pose.translation + rotation * Vec3::X * length;
            let forward = trace(pose.translation, end);
            if forward.0 {
                if trace(end, pose.translation).0 {
                    return None;
                }
            } else if forward.1 != 1. && trace(end, pose.translation).1 != 1. {
                return None;
            }
        } else {
            let end = pose.translation + rotation * Vec3::X * (length * 0.5);
            let (start_solid, fraction) = trace(root, end);
            if start_solid {
                return None;
            }
            if fraction < 1. {
                placed[index].as_mut()?.translation = root + (end - root) * (fraction * 0.8);
            }
        }
    }
    // An unobstructed skeleton already is the physical starting pose. Pulling
    // every bone into a 10-inch sphere collapses it before the first solve.
    // Only the obstruction correction above may move a sampled bone.
    Some(placed)
}

pub(crate) fn terrain_fraction(
    shape: &Collider,
    pose: Transform,
    start: Vec3,
    end: Vec3,
) -> Option<f32> {
    let delta = end - start;
    let length = delta.length();
    let direction = delta.try_normalize()?;
    let inverse = pose.rotation.inverse();
    let ray =
        avian3d::parry::query::Ray::new(inverse * (start - pose.translation), inverse * direction);
    shape
        .shape_scaled()
        .as_trimesh()?
        .cast_local_ray_with_culling(
            &ray,
            length,
            avian3d::parry::query::RayCullingMode::IgnoreFrontfaces,
        )
        .map(|hit| hit.time_of_impact / length)
}

fn bone_pose(pose: &GlobalTransform) -> Option<Transform> {
    // Severed limbs and their descendants are hidden with zero scale. They
    // have no physical pose: decomposing that matrix produces a NaN rotation.
    if pose.affine().matrix3.determinant() == 0. {
        return None;
    }
    Some(Transform::from_matrix(
        pose.to_matrix()
            * Mat4::from_scale_rotation_translation(
                Vec3::splat(1. / crate::assets::SOURCE_UNIT_METERS),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                Vec3::ZERO,
            ),
    ))
}

fn begin(
    mut commands: Commands,
    game: Res<GameDefinition>,
    gravity: Res<Gravity>,
    clock: Res<ClientPhysicsClock>,
    actors: Query<
        (
            Entity,
            &Zombie,
            Option<&PendingRagdoll>,
            Option<&DeathBlasts>,
        ),
        (
            With<ClientRagdollRequested>,
            Without<PhysicalRagdoll>,
            Without<RejectedRagdoll>,
        ),
    >,
    rigs: Query<(&ActorRig, &Visibility)>,
    poses: Query<&GlobalTransform>,
    brushes: Res<crate::assets::WorldBrushes>,
    world: Query<
        (&Collider, &GlobalTransform, &crate::assets::WorldContents),
        (With<crate::assets::WorldTerrain>, Without<ColliderDisabled>),
    >,
) {
    for (owner, zombie, pending, blasts) in &actors {
        let Some(enemy) = game
            .content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
        else {
            continue;
        };
        let Some(profile) = enemy.ragdoll.as_ref() else {
            continue;
        };
        let Some(rig) = rigs
            .iter()
            .find(|(rig, visibility)| {
                rig.owner == owner && rig.primary && rig.ready && **visibility != Visibility::Hidden
            })
            .map(|(rig, _)| rig)
        else {
            continue;
        };
        let fragments = std::iter::once(rig)
            .chain(
                rigs.iter()
                    .filter(|(fragment, visibility)| {
                        fragment.owner == owner
                            && !fragment.primary
                            && fragment.ready
                            && **visibility != Visibility::Hidden
                    })
                    .map(|(fragment, _)| fragment),
            )
            .collect::<Vec<_>>();
        let snapshot = profile
            .bones
            .iter()
            .map(|bone| {
                fragments
                    .iter()
                    .find_map(|fragment| fragment.sockets.get(&bone.end_joint))?;
                let joint = fragments
                    .iter()
                    .find_map(|fragment| fragment.sockets.get(&bone.joint))?;
                poses.get(*joint).ok().and_then(bone_pose)
            })
            .collect::<Vec<_>>();
        if snapshot.iter().take(3).any(Option::is_none) {
            continue;
        }
        let Some(pending) = pending else {
            commands
                .entity(owner)
                .insert(PendingRagdoll { poses: snapshot });
            continue;
        };
        let elapsed = clock.delta.as_secs_f32();
        if elapsed <= 0. {
            continue;
        }
        let trace = |start: Vec3, end: Vec3| {
            let (start_solid, brushes) = brushes.trace(start, end, 0x0280ec93);
            let fraction = world
                .iter()
                .filter(|(_, _, contents)| contents.0 & 0x0280ec93 != 0)
                .filter_map(|(shape, pose, _)| {
                    let pose = pose.compute_transform();
                    terrain_fraction(shape, pose, start, end)
                })
                .fold(brushes, f32::min);
            (start_solid, fraction)
        };
        let Some(placed) = initial_pose(profile, &snapshot, trace) else {
            commands
                .entity(owner)
                .remove::<(PendingRagdoll, DeathBlasts)>()
                .insert(RejectedRagdoll);
            continue;
        };
        let mut bodies = vec![None; profile.bones.len()];
        let count = placed.iter().flatten().count();
        for (index, bone) in profile.bones.iter().enumerate() {
            let Some(pose) = placed[index] else {
                continue;
            };
            let half_length = bone.half_length;
            let mirror = if bone.mirror {
                Quat::from_rotation_y(std::f32::consts::PI)
            } else {
                Quat::IDENTITY
            };
            let rotation = pose.rotation * mirror;
            let sample = snapshot[index].unwrap();
            let old = pending.poses[index].unwrap_or(sample);
            let mut velocity = (sample.translation - old.translation) / elapsed;
            velocity = velocity.clamp_length_max(600. * 0.0254);
            velocity.y = velocity.y.min(50. * 0.0254);
            if let Some(blasts) = blasts {
                let center = pose.translation
                    + rotation * if index == 0 { Vec3::Y } else { Vec3::X } * half_length;
                for blast in &blasts.0 {
                    velocity += blast.impulse(center, 18. / count as f32) / bone.mass;
                }
            }
            let mut delta_rotation = pose.rotation * old.rotation.inverse();
            if delta_rotation.w < 0. {
                delta_rotation = -delta_rotation;
            }
            let angular_velocity = delta_rotation.to_scaled_axis() / elapsed;
            let pairs = profile.self_pairs.iter().fold(0u16, |mask, pair| {
                if pair[0] == index {
                    mask | (1 << pair[1])
                } else if pair[1] == index {
                    mask | (1 << pair[0])
                } else {
                    mask
                }
            });
            let (shape, center) = capsule(bone.radius, half_length, index == 0, 1.);
            let size = Vec3::new(
                (half_length + bone.radius) * 2.,
                bone.radius * 2.,
                bone.radius * 2.,
            );
            let inertia = bone.mass / 12.
                * Vec3::new(
                    size.y * size.y + size.z * size.z,
                    size.x * size.x + size.z * size.z,
                    size.x * size.x + size.y * size.y,
                );
            let entity = commands
                .spawn((
                    RagdollEntity(owner),
                    crate::gameplay::SessionEntity,
                    RigidBody::Dynamic,
                    shape,
                    Transform::from_translation(pose.translation).with_rotation(rotation),
                    (
                        Mass(bone.mass),
                        AngularInertia::new(inertia),
                        CenterOfMass(center),
                        ColliderDensity(0.),
                        // The source values describe the whole bone, including
                        // its massless self-collision shape.
                        NoAutoMass,
                        NoAutoAngularInertia,
                        NoAutoCenterOfMass,
                    ),
                    (LinearVelocity(velocity), AngularVelocity(angular_velocity)),
                    (
                        GravityScale(800. * 0.0254 / gravity.0.length()),
                        Friction::new(bone.friction),
                        Restitution::ZERO,
                    ),
                    CollisionLayers::new(
                        CLIENT_PHYSICS_LAYER,
                        crate::assets::RAGDOLL_WORLD_LAYER | CLIENT_PHYSICS_LAYER,
                    ),
                    RagdollCollider {
                        owner,
                        index,
                        pairs,
                        self_only: false,
                    },
                    ActiveCollisionHooks::FILTER_PAIRS,
                ))
                .id();
            let (self_shape, _) = capsule(bone.radius, half_length, index == 0, 1.2);
            commands.spawn((
                ChildOf(entity),
                self_shape,
                Transform::IDENTITY,
                ColliderDensity(0.),
                CollisionLayers::new(
                    CLIENT_PHYSICS_LAYER,
                    crate::assets::RAGDOLL_WORLD_LAYER | CLIENT_PHYSICS_LAYER,
                ),
                Friction::new(bone.friction),
                Restitution::ZERO,
                RagdollCollider {
                    owner,
                    index,
                    pairs,
                    self_only: true,
                },
                ActiveCollisionHooks::FILTER_PAIRS,
            ));
            bodies[index] = Some(entity);
        }
        for joint in &profile.joints {
            let index = joint.bone;
            let Some(parent) = profile.bones[index].parent else {
                continue;
            };
            let (Some(child), Some(parent_body)) = (bodies[index], bodies[parent]) else {
                continue;
            };
            let base = |index: usize| {
                let bone = &profile.bones[index];
                let mut pose = Transform::from_matrix(Mat4::from_cols_array(&bone.base));
                if bone.mirror {
                    pose.rotation *= Quat::from_rotation_y(std::f32::consts::PI);
                }
                pose
            };
            let parent_frame = base(parent);
            let child_frame = base(index);
            let anchor = parent_frame.rotation.inverse()
                * (child_frame.translation - parent_frame.translation);
            let first = &joint.limits[0];
            let axis = Vec3::from_array(first.axis);
            let parent_basis = parent_frame.rotation.inverse() * child_frame.rotation;
            match joint.kind {
                RagdollJointKind::Hinge => {
                    commands.spawn((
                        RagdollEntity(owner),
                        crate::gameplay::SessionEntity,
                        RevoluteJoint::new(parent_body, child)
                            .with_local_anchor1(anchor)
                            .with_local_anchor2(Vec3::ZERO)
                            .with_hinge_axis(axis)
                            .with_local_basis1(parent_basis)
                            .with_angle_limits(first.minimum, first.maximum),
                    ));
                }
                RagdollJointKind::Swivel => {
                    commands.spawn((
                        RagdollEntity(owner),
                        crate::gameplay::SessionEntity,
                        SphericalJoint::new(parent_body, child)
                            .with_local_anchor1(anchor)
                            .with_local_anchor2(Vec3::ZERO)
                            .with_twist_axis(axis)
                            .with_local_basis1(parent_basis)
                            .with_twist_limits(first.minimum.max(-1.2), first.maximum.min(1.2)),
                    ));
                    for limit in joint.limits.iter().skip(1) {
                        commands.spawn((
                            RagdollEntity(owner),
                            crate::gameplay::SessionEntity,
                            SphericalJoint::new(parent_body, child)
                                .with_local_anchor1(anchor)
                                .with_local_anchor2(Vec3::ZERO)
                                .with_twist_axis(axis)
                                .with_local_basis1(parent_basis)
                                .with_swing_limits(0., (limit.maximum - limit.minimum) * 0.5),
                        ));
                    }
                }
            }
        }
        commands
            .entity(owner)
            .remove::<(PendingRagdoll, DeathBlasts)>()
            .insert(PhysicalRagdoll {
                bodies,
                seconds: clock.delta.as_secs_f32(),
            });
    }
}

fn apply_pose(
    game: Res<GameDefinition>,
    actors: Query<(&Zombie, &PhysicalRagdoll)>,
    rigs: Query<&ActorRig>,
    body_poses: Query<(&Position, &Rotation)>,
    mut joints: ParamSet<(Query<(&Transform, Option<&ChildOf>)>, Query<&mut Transform>)>,
) {
    // Imported joints use CoD inches; physical bone poses use Y-up metres.
    let source_frame = Mat4::from_scale_rotation_translation(
        Vec3::splat(crate::assets::SOURCE_UNIT_METERS),
        Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        Vec3::ZERO,
    );
    let mut writes = Vec::new();
    for rig in &rigs {
        if rig.socket.is_some() || !rig.ready {
            continue;
        }
        let Ok((zombie, ragdoll)) = actors.get(rig.owner) else {
            continue;
        };
        let Some(profile) = game
            .content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
            .and_then(|enemy| enemy.ragdoll.as_ref())
        else {
            continue;
        };
        let mut world_poses = profile
            .bones
            .iter()
            .zip(&ragdoll.bodies)
            .filter_map(|(bone, body)| {
                let entity = *rig.sockets.get(&bone.joint)?;
                let (position, rotation) = body_poses.get((*body)?).ok()?;
                let mirror = if bone.mirror {
                    Quat::from_rotation_y(std::f32::consts::PI)
                } else {
                    Quat::IDENTITY
                };
                Some((
                    entity,
                    Mat4::from_rotation_translation(rotation.0 * mirror, position.0),
                ))
            })
            .collect::<HashMap<_, _>>();
        let poses = joints.p0();
        let animated_world = |entity| {
            let mut world = Mat4::IDENTITY;
            let mut cursor = entity;
            loop {
                let (pose, parent) = poses.get(cursor).ok()?;
                world = pose.to_matrix() * world;
                let Some(parent) = parent else {
                    return Some(world);
                };
                cursor = parent.parent();
            }
        };
        for bone in &profile.baselerp {
            let Some(&entity) = rig.sockets.get(&bone.joint) else {
                continue;
            };
            let Some(&parent) = rig.sockets.get(&profile.bones[bone.bone].joint) else {
                continue;
            };
            let (Some(parent_world), Some(animated_parent), Some(animated_bone)) = (
                world_poses.get(&parent),
                animated_world(parent),
                animated_world(entity),
            ) else {
                continue;
            };
            // Keep severed end joints collapsed instead of decomposing their
            // singular world transform and writing NaNs into the skeleton.
            if animated_bone.determinant() == 0. {
                continue;
            }
            let current = Transform::from_matrix(
                source_frame * animated_parent.inverse() * animated_bone * source_frame.inverse(),
            );
            let base = Transform::from_matrix(Mat4::from_cols_array(&bone.base));
            let factor = if current.translation.length() <= 400. * 0.0254 {
                (ragdoll.seconds / bone.seconds).clamp(0., 1.)
            } else {
                1.
            };
            let quaternion = Vec4::from_array(current.rotation.to_array())
                .lerp(Vec4::from_array(base.rotation.to_array()), factor)
                .normalize_or_zero();
            let rotation = if quaternion == Vec4::ZERO {
                Quat::IDENTITY
            } else {
                Quat::from_array(quaternion.to_array())
            };
            let relative = Mat4::from_rotation_translation(
                rotation,
                current.translation.lerp(base.translation, factor),
            );
            world_poses.insert(entity, *parent_world * relative);
        }
        for (&entity, &world) in &world_poses {
            let Ok((pose, Some(parent))) = poses.get(entity) else {
                continue;
            };
            if pose.scale.cmpeq(Vec3::ZERO).any() {
                continue;
            }
            let mut cursor = parent.parent();
            let mut ancestor = Mat4::IDENTITY;
            loop {
                if let Some(world) = world_poses.get(&cursor) {
                    ancestor = *world * source_frame * ancestor;
                    break;
                }
                let Ok((pose, parent)) = poses.get(cursor) else {
                    break;
                };
                ancestor = pose.to_matrix() * ancestor;
                let Some(parent) = parent else {
                    break;
                };
                cursor = parent.parent();
            }
            if ancestor.determinant() == 0. {
                continue;
            }
            writes.push((
                entity,
                Transform::from_matrix(ancestor.inverse() * world * source_frame),
            ));
        }
    }
    for (entity, pose) in writes {
        if let Ok(mut joint) = joints.p1().get_mut(entity) {
            *joint = pose;
        }
    }
}

fn joint_friction(
    game: Res<GameDefinition>,
    clock: Res<ClientPhysicsClock>,
    actors: Query<(&Zombie, &PhysicalRagdoll)>,
    mut bodies: Query<(Forces, &ComputedAngularInertia, Has<Sleeping>)>,
) {
    for (zombie, ragdoll) in &actors {
        let Some(profile) = game
            .content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
            .and_then(|enemy| enemy.ragdoll.as_ref())
        else {
            continue;
        };
        let factor = 1. + (ragdoll.seconds / 3.).clamp(0., 1.);
        for joint in &profile.joints {
            let Some(parent) = profile.bones[joint.bone].parent else {
                continue;
            };
            let (Some(parent), Some(child)) = (ragdoll.bodies[parent], ragdoll.bodies[joint.bone])
            else {
                continue;
            };
            let friction = match joint.kind {
                RagdollJointKind::Hinge => joint.limits[0].friction,
                RagdollJointKind::Swivel => joint
                    .limits
                    .iter()
                    .take(2)
                    .map(|limit| limit.friction)
                    .fold(0., f32::max),
            } * factor
                * 0.04
                * 0.0254f32.powi(2);
            let Ok(
                [
                    (mut parent, parent_inertia, parent_asleep),
                    (mut child, child_inertia, child_asleep),
                ],
            ) = bodies.get_many_mut([parent, child])
            else {
                continue;
            };
            if parent_asleep && child_asleep {
                continue;
            }
            let relative = child.angular_velocity() - parent.angular_velocity();
            let speed = relative.length();
            if speed <= 0.0001 {
                continue;
            }
            let axis = relative / speed;
            let parent_rotation = parent.rotation();
            let child_rotation = child.rotation();
            let inverse = parent_rotation
                * (parent_inertia.inverse_tensor() * (parent_rotation.inverse() * axis))
                + child_rotation
                    * (child_inertia.inverse_tensor() * (child_rotation.inverse() * axis));
            let effective = axis.dot(inverse);
            if effective <= 0. {
                continue;
            }
            let impulse = axis * (speed / effective).min(friction * clock.delta.as_secs_f32());
            parent.non_waking().apply_angular_impulse(impulse);
            child.non_waking().apply_angular_impulse(-impulse);
        }
    }
}

fn tick(clock: Res<ClientPhysicsClock>, mut actors: Query<&mut PhysicalRagdoll>) {
    for mut ragdoll in &mut actors {
        ragdoll.seconds += clock.delta.as_secs_f32();
    }
}
fn cleanup(
    mut commands: Commands,
    owned: Query<(Entity, &RagdollEntity)>,
    actors: Query<(), With<Zombie>>,
) {
    for (entity, owner) in &owned {
        if !actors.contains(owner.0) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ragdoll_snapshots_omit_collapsed_limbs_and_preserve_source_units() {
        for scale in [Vec3::ZERO, Vec3::new(1., 0., 1.)] {
            let collapsed = GlobalTransform::from(Transform::from_scale(scale));
            assert!(bone_pose(&collapsed).is_none());
        }
        let translation = Vec3::new(2., 3., 4.);
        let source = GlobalTransform::from(
            Transform::from_translation(translation)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(crate::assets::SOURCE_UNIT_METERS)),
        );
        let pose = bone_pose(&source).unwrap();
        assert_eq!(pose.translation, translation);
        assert!(pose.scale.abs_diff_eq(Vec3::ONE, 1e-5));
        assert!(pose.rotation.abs_diff_eq(Quat::IDENTITY, 1e-5));
    }

    #[test]
    fn native_initial_ragdoll_terrain_trace_culls_native_backfaces() {
        let shape = Collider::trimesh(
            vec![
                Vec3::new(-1., 0., -1.),
                Vec3::new(1., 0., -1.),
                Vec3::new(0., 0., 1.),
            ],
            vec![[0, 1, 2]],
        );
        assert_eq!(
            terrain_fraction(&shape, Transform::IDENTITY, Vec3::Y, Vec3::NEG_Y),
            Some(0.5)
        );
        assert_eq!(
            terrain_fraction(&shape, Transform::IDENTITY, Vec3::NEG_Y, Vec3::Y),
            None
        );
    }

    fn placement_profile(half_lengths: &[f32]) -> crate::origins::content::EnemyRagdoll {
        crate::origins::content::EnemyRagdoll {
            bones: half_lengths
                .iter()
                .map(|half_length| crate::origins::content::RagdollBone {
                    joint: "start".into(),
                    end_joint: "end".into(),
                    radius: 0.01,
                    half_length: *half_length,
                    base: Mat4::IDENTITY.to_cols_array(),
                    mass: 1.,
                    friction: 0.,
                    parent: None,
                    mirror: false,
                })
                .collect(),
            joints: Vec::new(),
            baselerp: Vec::new(),
            self_pairs: Vec::new(),
        }
    }
    fn wall_trace(walls: &[(Collider, Vec3)], start: Vec3, end: Vec3) -> (bool, f32) {
        let delta = end - start;
        let distance = delta.length();
        let direction = delta / distance;
        let solid = walls
            .iter()
            .any(|(shape, position)| shape.contains_point(*position, Quat::IDENTITY, start));
        let fraction = walls
            .iter()
            .filter_map(|(shape, position)| {
                shape
                    .cast_ray(*position, Quat::IDENTITY, start, direction, distance, false)
                    .map(|hit| hit.0 / distance)
            })
            .fold(1., f32::min);
        (solid, fraction)
    }
    #[test]
    fn native_initial_ragdoll_root_rejects_two_embedded_ends_and_accepts_one_clear_orientation() {
        let profile = placement_profile(&[1.]);
        let snapshot = [Some(Transform::IDENTITY)];
        let mut walls = vec![(Collider::cuboid(0.2, 1., 1.), Vec3::ZERO)];
        assert!(
            initial_pose(&profile, &snapshot, |start, end| wall_trace(
                &walls, start, end
            ))
            .is_some()
        );
        walls.push((Collider::cuboid(0.2, 1., 1.), Vec3::X * 2.));
        assert!(
            initial_pose(&profile, &snapshot, |start, end| wall_trace(
                &walls, start, end
            ))
            .is_none()
        );
        walls.clear();
        walls.push((Collider::cuboid(0.2, 1., 1.), Vec3::X));
        assert!(
            initial_pose(&profile, &snapshot, |start, end| wall_trace(
                &walls, start, end
            ))
            .is_none()
        );
    }
    #[test]
    fn native_initial_ragdoll_obstructed_limb_pulls_back_before_the_world_hit() {
        let profile = placement_profile(&[0.1, 0.1]);
        let snapshot = [
            Some(Transform::IDENTITY),
            Some(Transform::from_translation(Vec3::X * 2.)),
        ];
        let walls = [(Collider::cuboid(0.2, 1., 1.), Vec3::X)];
        let placed = initial_pose(&profile, &snapshot, |start, end| {
            wall_trace(&walls, start, end)
        })
        .unwrap();
        assert!((placed[1].unwrap().translation.x - 0.72).abs() < 0.00001);
        assert_eq!(snapshot[1].unwrap().translation.x, 2.);
    }

    #[test]
    fn unobstructed_ragdoll_preserves_every_sampled_bone_pose() {
        let profile = placement_profile(&[0.3, 0.2, 0.1, 0.1]);
        let snapshot = [
            Some(Transform::from_xyz(4., 1.5, -3.).with_rotation(Quat::from_rotation_y(0.7))),
            Some(Transform::from_xyz(4.6, 1.8, -3.2).with_rotation(Quat::from_rotation_x(-0.8))),
            None,
            Some(Transform::from_xyz(3.8, 0.4, -2.9).with_rotation(Quat::from_rotation_z(1.1))),
        ];
        let placed = initial_pose(&profile, &snapshot, |_, _| (false, 1.)).unwrap();
        assert_eq!(placed.as_slice(), snapshot.as_slice());
    }

    #[test]
    fn native_client_physics_clock_clamps_frames_and_resets_pause_and_scene_boundaries() {
        let mut clock = ClientPhysicsClock::default();
        assert_eq!(clock.advance(None, true, false), Duration::ZERO);
        assert_eq!(
            clock.advance(Some(50), true, false),
            Duration::from_millis(50)
        );
        assert_eq!(clock.advance(Some(50), true, false), Duration::ZERO);
        assert_eq!(
            clock.advance(Some(51), true, false),
            Duration::from_millis(1)
        );
        assert_eq!(
            clock.advance(Some(451), true, false),
            Duration::from_millis(100)
        );
        assert_eq!(
            clock.advance(Some(449), true, false),
            Duration::from_millis(1)
        );
        assert_eq!(clock.advance(Some(500), true, true), Duration::ZERO);
        assert_eq!(
            clock.advance(Some(516), true, false),
            Duration::from_millis(16)
        );
        assert_eq!(clock.advance(None, false, false), Duration::ZERO);
        assert_eq!(
            clock.advance(Some(50), true, false),
            Duration::from_millis(50)
        );
    }

    #[derive(Resource, Default)]
    struct ForceOrder {
        queued: u32,
        applied: Vec<(Duration, Duration, u32)>,
    }
    fn integrate_probe(
        time: Res<Time>,
        substeps: Res<Time<Substeps>>,
        mut order: ResMut<ForceOrder>,
    ) {
        let queued = std::mem::take(&mut order.queued);
        order.applied.push((time.delta(), substeps.delta(), queued));
    }
    fn queue_probe(mut order: ResMut<ForceOrder>) {
        order.queued += 1;
    }

    #[test]
    fn native_client_physics_schedule_integrates_prior_frame_forces_and_preserves_game_time() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            avian3d::schedule::PhysicsSchedulePlugin::new(PostUpdate),
        ))
        .init_resource::<ForceOrder>()
        .add_systems(
            PhysicsSchedule,
            integrate_probe.in_set(PhysicsStepSystems::Solver),
        )
        .add_systems(PostUpdate, queue_probe.after(PhysicsSystems::Writeback));
        let gameplay_delta = Duration::from_millis(777);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(gameplay_delta);
        let count = app.world().resource::<SubstepCount>().0;
        for milliseconds in [16, 25, 0, 100] {
            app.world_mut()
                .resource_scope(|world, mut physics: Mut<Time<Physics>>| {
                    let mut substeps = world.resource_mut::<Time<Substeps>>();
                    advance_clocks(
                        &mut physics,
                        &mut substeps,
                        count,
                        Duration::from_millis(milliseconds),
                    );
                });
            app.world_mut().run_schedule(PostUpdate);
            assert_eq!(app.world().resource::<Time>().delta(), gameplay_delta);
            assert_eq!(
                app.world().resource::<Time<Physics>>().delta(),
                Duration::ZERO
            );
        }
        let order = app.world().resource::<ForceOrder>();
        assert_eq!(order.applied.len(), 3);
        for ((delta, substep, forces), (milliseconds, expected)) in
            order.applied.iter().zip([(16, 0), (25, 1), (100, 2)])
        {
            assert_eq!(*delta, Duration::from_millis(milliseconds));
            assert_eq!(*substep, delta.div_f64(count as f64));
            assert_eq!(*forces, expected);
        }
    }
}
