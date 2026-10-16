use super::{ActorRig, Zombie};
use crate::{
    GameState, assets::GameDefinition, combat_effects::Wounds, gameplay::SessionEntity,
    origins::content::EnemyGibPiece, player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(super) struct GibCounter {
    player: Option<Entity>,
    count: u8,
    elapsed: f32,
    random: u64,
}
impl GibCounter {
    pub(super) fn random(&mut self) -> f32 {
        self.random = self
            .random
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        (self.random >> 32) as u32 as f32 / u32::MAX as f32
    }
}
#[derive(Component, Default)]
struct ClientGibbed;
#[derive(Component)]
struct PendingGib {
    owner: Entity,
    piece: EnemyGibPiece,
    initial: Transform,
    seconds: f64,
    hat: bool,
}
#[derive(Component)]
struct BloodSpurt {
    owner: Entity,
    seconds: f64,
}
#[derive(Component)]
struct GibBody {
    slot: usize,
    bullet_force_scale: f32,
    explosive_force_scale: f32,
}
#[derive(Component)]
struct LaunchForce {
    point: Vec3,
    force: Vec3,
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<GibCounter>()
        .add_systems(
            Update,
            (events, launch, head_spurt)
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            PostUpdate,
            (bullets, explosions, forces)
                .chain()
                .after(super::ragdoll::ClientPhysicsEffects)
                .before(bevy::transform::TransformSystems::Propagate)
                .run_if(super::ragdoll::client_scene),
        );
}

fn socket_pose(
    owner: Entity,
    joint: &str,
    rigs: &Query<(&ActorRig, &Visibility)>,
    poses: &Query<&GlobalTransform>,
) -> Option<Transform> {
    let active = |(rig, visibility): &(&ActorRig, &Visibility)| {
        rig.owner == owner && rig.ready && **visibility != Visibility::Hidden
    };
    let entity = rigs
        .iter()
        .filter(active)
        .find_map(|(rig, _)| rig.socket(owner, joint))
        .or_else(|| {
            rigs.iter()
                .filter(active)
                .find_map(|(rig, _)| rig.sockets.get(joint).copied())
        })?;
    poses
        .get(entity)
        .ok()
        .map(GlobalTransform::compute_transform)
}

fn events(
    mut commands: Commands,
    game: Res<GameDefinition>,
    time: Res<Time>,
    session: Res<crate::gameplay::Session>,
    players: Query<Entity, With<Player>>,
    mut counter: ResMut<GibCounter>,
    mut actors: Query<(Entity, &Zombie, &mut Wounds, Has<ClientGibbed>)>,
    rigs: Query<(&ActorRig, &Visibility)>,
    poses: Query<&GlobalTransform>,
) {
    let player = players.iter().next();
    if player != counter.player {
        counter.player = player;
        counter.count = 0;
        counter.elapsed = 0.;
        counter.random = session.seed;
    } else {
        counter.elapsed += time.delta_secs();
    }
    if counter.elapsed >= 0.5 {
        counter.elapsed = counter.elapsed.rem_euclid(0.5);
        counter.count = 0;
    }
    for (owner, zombie, mut wounds, mut gibbed) in &mut actors {
        let events = std::mem::take(&mut wounds.gib_events);
        let Some(gibs) = game
            .content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
            .and_then(|enemy| enemy.gibs.as_ref())
        else {
            continue;
        };
        for event in events {
            if counter.count > 3 {
                continue;
            }
            counter.count += 1;
            let event = if gibbed { event & (1 << 5) } else { event };
            gibbed = true;
            commands.entity(owner).insert(ClientGibbed);
            for (index, piece) in gibs.pieces.iter().enumerate() {
                if event & (1 << (index + 1)) == 0 {
                    continue;
                }
                if let Some(initial) = socket_pose(owner, &piece.joint, &rigs, &poses) {
                    commands.spawn((
                        SessionEntity,
                        PendingGib {
                            owner,
                            piece: piece.clone(),
                            initial,
                            seconds: time.elapsed_secs_f64(),
                            hat: false,
                        },
                    ));
                }
            }
            if event & (1 << 7) != 0
                && let Some(piece) = &gibs.hat
                && let Some(initial) = socket_pose(owner, &piece.joint, &rigs, &poses)
            {
                commands.spawn((
                    SessionEntity,
                    PendingGib {
                        owner,
                        piece: piece.clone(),
                        initial,
                        seconds: time.elapsed_secs_f64(),
                        hat: true,
                    },
                ));
            }
            if event & (1 << 5) != 0
                && let Some(pose) = socket_pose(owner, "j_neck", &rigs, &poses)
            {
                crate::native_fx::emit(&mut commands, "impacts/fx_flesh_hit", pose);
                crate::native_fx::emit(&mut commands, "misc/fx_zombie_bloodsplat", pose);
                crate::audio::emit(
                    &mut commands,
                    "zombie_head_gib",
                    crate::audio::SoundSource::World(pose.translation),
                );
                commands.spawn((
                    SessionEntity,
                    BloodSpurt {
                        owner,
                        seconds: time.elapsed_secs_f64(),
                    },
                ));
            }
        }
    }
}

fn launch(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<GameDefinition>,
    gravity: Res<Gravity>,
    server: Res<AssetServer>,
    mut counter: ResMut<GibCounter>,
    pending: Query<(Entity, &PendingGib)>,
    rigs: Query<(&ActorRig, &Visibility)>,
    poses: Query<&GlobalTransform>,
    bodies: Query<(Entity, &GlobalTransform, &GibBody)>,
    cameras: Query<&GlobalTransform, With<crate::rendering::GameCamera>>,
    retired: Query<(), With<super::ActorCorpse>>,
) {
    let mut occupied = bodies
        .iter()
        .map(|(entity, pose, body)| (entity, pose.translation(), body.slot))
        .collect::<Vec<_>>();
    occupied.sort_by_key(|body| body.2);
    let Ok(camera) = cameras.single() else {
        return;
    };
    let player = camera.translation();
    for (entity, pending) in &pending {
        if !pending.hat && time.elapsed_secs_f64() - pending.seconds < 0.016 {
            continue;
        }
        let pose = if pending.hat {
            pending.initial
        } else {
            (!retired.contains(pending.owner))
                .then(|| socket_pose(pending.owner, &pending.piece.joint, &rigs, &poses))
                .flatten()
                .unwrap_or_else(|| {
                    let mut pose = pending.initial;
                    pose.translation += pose.rotation * Vec3::X * (10. * 0.0254);
                    pose
                })
        };
        let physics = &pending.piece.physics;
        let shapes = physics
            .shapes
            .iter()
            .filter_map(|points| {
                Collider::convex_hull(
                    &points
                        .iter()
                        .copied()
                        .map(Vec3::from_array)
                        .collect::<Vec<_>>(),
                )
                .map(|shape| (Vec3::ZERO, Quat::IDENTITY, shape))
            })
            .collect::<Vec<_>>();
        if shapes.len() != physics.shapes.len() {
            continue;
        }
        let Some(scene) = game
            .content
            .presentations
            .iter()
            .find(|p| p.id == pending.piece.presentation)
            .and_then(|p| p.scene.as_ref())
        else {
            continue;
        };
        let pose = Transform::from_translation(pose.translation).with_rotation(if pending.hat {
            Quat::from_rotation_x(1f32.to_radians())
        } else {
            pose.rotation
        });
        if occupied.len() > 50 {
            let mut farthest = None;
            let mut distance = 0.;
            for (index, (_, position, _)) in occupied.iter().enumerate() {
                let candidate = position.distance_squared(player);
                if candidate > distance {
                    distance = candidate;
                    farthest = Some(index);
                }
            }
            if let Some(index) = farthest {
                let (evicted, _, _) = occupied.remove(index);
                commands.entity(evicted).despawn();
            } else if occupied.len() == 256 {
                commands.entity(entity).despawn();
                continue;
            }
        }
        let slot = (0..256)
            .find(|slot| !occupied.iter().any(|body| body.2 == *slot))
            .expect("native spawned dynamic entity slot is available");
        let mut force = if pending.hat {
            Vec3::Y * (1.4 + counter.random() * 0.3)
        } else {
            (pose.translation - pending.initial.translation).normalize_or_zero()
                * (0.6 + counter.random() * 0.4)
                + Vec3::Y * (0.4 + counter.random() * 0.3)
        };
        let magnitude = force.length();
        force = (force.normalize_or_zero() + Vec3::Y * 0.5).normalize_or_zero()
            * (magnitude * physics.bullet_force_scale * 0.0254);
        let center = Vec3::from_array(physics.center);
        let mut low = Vec3::splat(f32::INFINITY);
        let mut high = Vec3::splat(f32::NEG_INFINITY);
        for point in physics.shapes.iter().flatten() {
            low = low.min(Vec3::from_array(*point));
            high = high.max(Vec3::from_array(*point));
        }
        let size = high - low;
        let inertia = physics.mass / 12.
            * Vec3::new(
                size.y * size.y + size.z * size.z,
                size.x * size.x + size.z * size.z,
                size.x * size.x + size.y * size.y,
            );
        let body = commands
            .spawn((
                SessionEntity,
                GibBody {
                    slot,
                    bullet_force_scale: physics.bullet_force_scale,
                    explosive_force_scale: physics.explosive_force_scale,
                },
                RigidBody::Dynamic,
                Collider::compound(shapes),
                pose,
                (
                    Mass(physics.mass),
                    AngularInertia::new(inertia),
                    CenterOfMass(center),
                    ColliderDensity(0.),
                    NoAutoMass,
                    NoAutoAngularInertia,
                    NoAutoCenterOfMass,
                ),
                (
                    Friction::new(physics.friction),
                    Restitution::new(physics.bounce),
                    GravityScale(800. * 0.0254 * physics.gravity_scale / gravity.0.length()),
                ),
                LaunchForce {
                    point: pending.initial.translation * 4.
                        - (pose.translation + pose.rotation * center) * 3.,
                    force,
                },
            ))
            .id();
        commands
            .entity(body)
            .insert(super::ragdoll::dynamic_piece(body));
        commands.spawn((
            ChildOf(body),
            WorldAssetRoot(server.load(scene.to_owned())),
            Transform::IDENTITY,
            Visibility::Inherited,
        ));
        occupied.push((body, pose.translation, slot));
        occupied.sort_by_key(|body| body.2);
        if !pending.hat {
            crate::native_fx::attach(
                &mut commands,
                "trail/fx_trail_blood_streak",
                body,
                Transform::IDENTITY,
            );
            crate::native_fx::emit(&mut commands, "weapon/bullet/fx_flesh_gib_fatal_01", pose);
            crate::audio::emit(
                &mut commands,
                "zmb_death_gibs",
                crate::audio::SoundSource::World(pose.translation),
            );
        }
        commands.entity(entity).despawn();
    }
}
fn bullets(
    mut commands: Commands,
    game: Res<GameDefinition>,
    effects: Res<crate::native_fx::Catalog>,
    mut events: MessageReader<super::RagdollBullet>,
    query: SpatialQuery,
    mut bodies: Query<(Forces, &GibBody, &ComputedCenterOfMass)>,
) {
    for event in events.read() {
        let delta = event.end - event.origin;
        let Ok(direction) = Dir3::new(delta) else {
            continue;
        };
        let Some(hit) = query.cast_ray_predicate(
            event.origin,
            direction,
            delta.length(),
            true,
            &super::ragdoll::client_physics_filter(),
            &|entity| bodies.contains(entity),
        ) else {
            continue;
        };
        let Ok((mut body, piece, center)) = bodies.get_mut(hit.entity) else {
            continue;
        };
        let point = event.origin + direction.as_vec3() * hit.distance;
        if let Some(effect) = game
            .content
            .weapons
            .iter()
            .find(|weapon| weapon.id == event.weapon)
            .and_then(|weapon| effects.impact(weapon.mechanics.impact_type, 7, 0))
        {
            crate::native_fx::emit(
                &mut commands,
                effect,
                effects.explosion_pose(point, hit.normal),
            );
        }
        let center = body.position() + body.rotation() * center.0;
        let force = (direction.as_vec3() + Vec3::Y * 0.5).normalize_or_zero()
            * (0.8 * piece.bullet_force_scale * 0.0254);
        body.apply_force_at_point(force, point * 4. - center * 3.);
    }
}
fn explosions(
    mut events: MessageReader<super::RagdollExplosion>,
    mut random: ResMut<GibCounter>,
    query: SpatialQuery,
    actors: Query<(), With<crate::life::Vitality>>,
    mut bodies: Query<(Forces, &GibBody, &ComputedCenterOfMass)>,
) {
    for event in events.read() {
        for (mut body, piece, center) in &mut bodies {
            let center = body.position() + body.rotation() * center.0;
            let impulse = event.impulse(center, 12. * piece.explosive_force_scale);
            if impulse == Vec3::ZERO
                || !super::ragdoll::blast_visible(&query, &actors, event.center, center)
            {
                continue;
            }
            let jitter = (Vec3::new(random.random(), random.random(), random.random()) * 2.
                - Vec3::ONE)
                * (4. * 0.0254);
            body.apply_linear_impulse_at_point(impulse, center + jitter);
        }
    }
}
fn forces(mut commands: Commands, mut bodies: Query<(Entity, &LaunchForce, Forces)>) {
    for (entity, launch, mut body) in &mut bodies {
        body.apply_force_at_point(launch.force, launch.point);
        commands.entity(entity).remove::<LaunchForce>();
    }
}
fn head_spurt(
    mut commands: Commands,
    time: Res<Time>,
    pending: Query<(Entity, &BloodSpurt)>,
    rigs: Query<(&ActorRig, &Visibility)>,
    retired: Query<(), With<super::ActorCorpse>>,
) {
    for (entity, spurt) in &pending {
        if time.elapsed_secs_f64() - spurt.seconds < 0.3 {
            continue;
        }
        if !retired.contains(spurt.owner)
            && let Some(joint) = rigs
                .iter()
                .filter(|(rig, visibility)| rig.ready && **visibility != Visibility::Hidden)
                .find_map(|(rig, _)| rig.socket(spurt.owner, "j_neck"))
        {
            crate::native_fx::attach(
                &mut commands,
                "misc/fx_zombie_bloodspurt",
                joint,
                Transform::IDENTITY,
            );
        }
        commands.entity(entity).despawn();
    }
}
