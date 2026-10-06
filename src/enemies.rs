//! Ordinary enemy lifecycle and Bevy-owned original multipart actor playback.
use crate::hit_regions::SkeletalReady as ActorReady;
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Navigation},
    life::{LifeState, Vitality},
    origins::content::{ContentId, EnemyAnimation},
    player::Player,
};
use avian3d::prelude::*;
use bevy::{
    camera::{primitives::Aabb, visibility::VisibilitySystems},
    mesh::skinning::SkinnedMesh,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    next_hit: usize,
    lure: Option<Entity>,
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
            next_hit: 0,
            lure: None,
        }
    }
    fn transition(&mut self, action: EnemyAnimation) {
        if self.action != action {
            self.action = action;
            self.elapsed = 0.;
            self.next_hit = 0;
        }
    }
}
#[derive(Resource, Default)]
pub struct EnemyEvidence {
    pub variants: BTreeSet<String>,
    pub animations: BTreeSet<String>,
    pub attached_heads: u32,
    pub attacks_started: u32,
    pub attack_hits: u32,
    pub deaths: u32,
    pub corpses_removed: u32,
    pub bounds_updates: u64,
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
    action: Option<EnemyAnimation>,
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
struct ActorMaterials(HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>);
#[derive(Resource, Default)]
struct Graphs(
    HashMap<AssetId<Gltf>, (Handle<AnimationGraph>, BTreeMap<String, AnimationNodeIndex>)>,
);

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct EnemyBounds;
pub struct EnemyPlugin;
impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Graphs>()
            .init_resource::<EnemyEvidence>()
            .init_resource::<ActorMaterials>()
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
                    animate,
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
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    navigation: Res<Navigation>,
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
) {
    let Ok((player_entity, player_transform, health)) = players.single() else {
        return;
    };
    let dt = time.delta_secs();
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
            continue;
        }
        if ready.is_none() {
            continue;
        }
        zombie.elapsed += dt;
        let lure = attractions.0.get(&entity);
        if zombie.lure != lure.map(|l| l.bomb) {
            zombie.lure = lure.map(|l| l.bomb);
            zombie.navigation = 0.;
            zombie.transition(EnemyAnimation::Idle);
        }
        let target = lure.map_or(player_transform.translation, |l| l.position);
        let filter = SpatialQueryFilter::from_excluded_entities([entity, player_entity]);
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
            && zombie.action != EnemyAnimation::Attack
            && distance < 1.65
            && visible(transform.translation, target)
        {
            zombie.transition(EnemyAnimation::Attack);
            evidence.attacks_started += 1;
        }
        if zombie.action == EnemyAnimation::Attack {
            while zombie.next_hit < enemy.attack_hits.len()
                && zombie.elapsed >= enemy.attack_hits[zombie.next_hit]
            {
                zombie.next_hit += 1;
                if distance < 1.5
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
        zombie.navigation -= dt;
        if zombie.navigation <= 0. {
            zombie.navigation = 0.5;
            zombie.destination = if visible(transform.translation, target) {
                target
            } else {
                navigation.destination(feet, target - Vec3::Y * 0.9) + Vec3::Y * 0.85
            };
        }
        let delta = zombie.destination - transform.translation;
        let direction = Vec3::new(delta.x, 0., delta.z).normalize_or_zero();
        let moving = zombie.action != EnemyAnimation::Attack && !(lure.is_some() && distance < 0.4);
        zombie.velocity.x = direction.x * zombie.speed * f32::from(moving);
        zombie.velocity.z = direction.z * zombie.speed * f32::from(moving);
        zombie.velocity.y = (zombie.velocity.y - definition.tuning.gravity * dt).max(-30.);
        if direction.length_squared() > 0.001 {
            transform.rotation = Quat::from_rotation_y((-direction.z).atan2(direction.x));
        }
        let filter = SpatialQueryFilter::from_excluded_entities([entity]);
        let support = query.cast_ray(feet + Vec3::Y * 0.1, Dir3::NEG_Y, 0.25, false, &filter);
        let grounded = support.is_some_and(|h| h.normal.y > 0.65) && zombie.velocity.y <= 0.;
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
                        action: None,
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
    children: Query<&Children>,
    names: Query<&Name>,
    mut evidence: ResMut<EnemyEvidence>,
) {
    let sockets: Vec<_> = rigs
        .iter()
        .filter(|(_, r)| r.socket.is_none() && r.ready)
        .flat_map(|(root, r)| {
            children
                .iter_descendants(root)
                .filter_map(|e| names.get(e).ok().map(|n| (r.owner, n.to_string(), e)))
                .collect::<Vec<_>>()
        })
        .collect();
    for (entity, mut rig) in &mut rigs {
        if rig.attached || !rig.ready {
            continue;
        }
        let Some(socket) = rig.socket.as_ref() else {
            continue;
        };
        if let Some((_, _, parent)) = sockets
            .iter()
            .find(|(owner, n, _)| *owner == rig.owner && n == socket)
        {
            commands.entity(*parent).add_child(entity);
            commands.entity(entity).insert((
                Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                Visibility::Inherited,
            ));
            rig.attached = true;
            evidence.attached_heads += 1;
        }
    }
}
fn light_materials(
    mut commands: Commands,
    mut cache: ResMut<ActorMaterials>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    meshes: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        (With<ActorMesh>, Without<ActorLit>),
    >,
) {
    for (entity, material) in &meshes {
        let handle = if let Some(handle) = cache.0.get(&material.0.id()) {
            handle.clone()
        } else {
            let Some(source) = materials.get(&material.0) else {
                continue;
            };
            let source = crate::rendering::with_actor_fill(source);
            let handle = materials.add(source);
            cache.0.insert(material.0.id(), handle.clone());
            handle
        };
        commands
            .entity(entity)
            .insert((MeshMaterial3d(handle), ActorLit));
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
        let Some(name) = enemy
            .animation_actions
            .get(&zombie.action)
            .and_then(|id| id.0.rsplit('.').next())
        else {
            continue;
        };
        let Some(&node) = nodes.get(name) else {
            continue;
        };
        for &entity in &rig.players {
            let Ok(mut player) = players.get_mut(entity) else {
                continue;
            };
            if rig.action != Some(zombie.action) {
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
            }
            if let Some(active) = player.animation_mut(node) {
                active.seek_to(zombie.elapsed);
            }
            if **state == GameState::Playing {
                player.resume_all();
            } else {
                player.pause_all();
            }
        }
        rig.action = Some(zombie.action);
    }
}
fn partial_roots(mut roots: Query<&mut Transform, With<PartialRoot>>) {
    for mut transform in &mut roots {
        transform.translation = Vec3::ZERO;
        transform.rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    }
}
fn animated_bounds(
    mut meshes: Query<(&ActorMesh, &SkinnedMesh, &GlobalTransform, &mut Aabb)>,
    joints: Query<&GlobalTransform>,
    enemies: Query<&Zombie>,
    definition: Option<Res<GameDefinition>>,
    mut evidence: ResMut<EnemyEvidence>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (mesh, skin, transform, mut aabb) in &mut meshes {
        let Ok(zombie) = enemies.get(mesh.owner) else {
            continue;
        };
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
