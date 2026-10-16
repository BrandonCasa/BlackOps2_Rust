//! Living actors temporarily hand their existing skeleton to the normal ragdoll solver.
use super::{Cheats, Gun, tools};
use crate::{GameState, enemies::Zombie};
use avian3d::prelude::*;
use bevy::{ecs::system::SystemState, prelude::*};

#[derive(Component)]
pub(crate) struct LivingRagdoll {
    pub(crate) profile: Option<crate::origins::content::EnemyRagdoll>,
    released: Option<f32>,
    original_position: Vec3,
    root_offset: Option<Vec3>,
    animations: Vec<(Entity, AnimationPlayer)>,
}

pub(crate) fn profile<'a>(
    game: &'a crate::assets::GameDefinition,
    zombie: &Zombie,
    living: Option<&'a LivingRagdoll>,
) -> Option<&'a crate::origins::content::EnemyRagdoll> {
    living.and_then(|actor| actor.profile.as_ref()).or_else(|| {
        game.content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
            .and_then(|enemy| enemy.ragdoll.as_ref())
    })
}

fn sandbox_profile(
    world: &mut World,
    owner: Entity,
) -> Option<crate::origins::content::EnemyRagdoll> {
    // The Panzer has no authored death ragdoll. Use the existing humanoid joint
    // limits, fitted to this actor's actual skeleton at the moment of the grab.
    let game = world.resource::<crate::assets::GameDefinition>();
    let mut profile = game
        .content
        .enemies
        .iter()
        .find(|enemy| enemy.id.0 == "enemy.zombie")?
        .ragdoll
        .clone()?;
    let mut rigs = world.query::<&crate::enemies::ActorRig>();
    for bone in &mut profile.bones {
        let start = rigs
            .iter(world)
            .find_map(|rig| rig.socket(owner, &bone.joint))?;
        let end = rigs
            .iter(world)
            .find_map(|rig| rig.socket(owner, &bone.end_joint))?;
        let start = world.get::<GlobalTransform>(start)?;
        let end = world.get::<GlobalTransform>(end)?;
        let length = start.translation().distance(end.translation()) * 0.5;
        let scale = (length / bone.half_length).clamp(0.5, 3.);
        bone.half_length = length;
        bone.radius *= scale;
        bone.mass *= scale * scale * scale;
        bone.base = (start.to_matrix()
            * Mat4::from_scale_rotation_translation(
                Vec3::splat(1. / crate::assets::SOURCE_UNIT_METERS),
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                Vec3::ZERO,
            ))
        .to_cols_array();
    }
    Some(profile)
}

#[derive(Resource)]
pub(super) struct PhysicsGun {
    pub held: Option<Entity>,
    pub distance: f32,
    body: Option<Entity>,
    hit: Vec3,
}
impl Default for PhysicsGun {
    fn default() -> Self {
        Self {
            held: None,
            distance: 4.,
            body: None,
            hit: Vec3::ZERO,
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        drive
            .after(PhysicsSystems::Writeback)
            .before(crate::enemies::ragdoll::apply_pose)
            .run_if(in_state(GameState::Playing))
            .run_if(active),
    );
}
fn active(actors: Query<(), With<LivingRagdoll>>) -> bool {
    !actors.is_empty()
}

pub(super) fn grab(world: &mut World) {
    if world.resource::<PhysicsGun>().held.is_some() {
        return;
    }
    let Some((player, origin, direction, _)) = tools::aim(world) else {
        return;
    };
    let mut query = SystemState::<(crate::hit_regions::RayTargets, SpatialQuery)>::new(world);
    let (actors, spatial) = query
        .get(world)
        .expect("game physics and hit regions are installed");
    let Some(hit) = crate::hit_regions::cast(&actors, &spatial, player, origin, direction, 30.)
    else {
        world.resource_mut::<Cheats>().notice = "Aim at a living enemy within 30 metres".into();
        return;
    };
    let owner = hit.target;
    if world.get::<Zombie>(owner).is_none()
        || world
            .get::<crate::hit_regions::SkeletalReady>(owner)
            .is_none()
    {
        world.resource_mut::<Cheats>().notice = "This enemy's native ragdoll is not ready".into();
        return;
    }
    if let Some(mut living) = world.get_mut::<LivingRagdoll>(owner) {
        living.released = None;
    } else {
        let authored = profile(
            world.resource::<crate::assets::GameDefinition>(),
            world.get::<Zombie>(owner).unwrap(),
            None,
        )
        .is_some();
        let profile = if authored {
            None
        } else {
            let Some(profile) = sandbox_profile(world, owner) else {
                world.resource_mut::<Cheats>().notice = "This enemy's skeleton is not ready".into();
                return;
            };
            Some(profile)
        };
        let mut rigs = world.query::<(Entity, &crate::enemies::ActorRig, &mut AnimationPlayer)>();
        let mut animations = Vec::new();
        for (entity, rig, mut player) in rigs.iter_mut(world) {
            if rig.owner() == owner {
                animations.push((entity, player.clone()));
                player.pause_all();
            }
        }
        let position = world
            .get::<Transform>(owner)
            .expect("skeletal actor pose")
            .translation;
        world.entity_mut(owner).insert((
            LivingRagdoll {
                profile,
                released: None,
                original_position: position,
                root_offset: None,
                animations,
            },
            ColliderDisabled,
            crate::enemies::ClientRagdollRequested,
        ));
        world
            .resource_mut::<crate::barriers::Barriers>()
            .release(owner);
        // A Panzer cannot keep grappling the player while its body is being carried.
        if let Some(mut panzer) = world.get_mut::<crate::panzer::Panzer>(owner) {
            panzer.captured = None;
            panzer.phase = crate::panzer::PanzerPhase::Pursuit;
        }
        let mut captured = world.query::<&mut crate::panzer::CapturedByPanzer>();
        for mut capture in captured.iter_mut(world) {
            if capture.0 == Some(owner) {
                capture.0 = None;
            }
        }
    }
    *world.resource_mut::<PhysicsGun>() = PhysicsGun {
        held: Some(owner),
        distance: hit.hit.distance.clamp(1.5, 30.),
        body: None,
        hit: hit.hit.point,
    };
}

pub(super) fn release(world: &mut World) {
    let owner = world.resource_mut::<PhysicsGun>().held.take();
    if let Some(owner) = owner
        && let Some(mut actor) = world.get_mut::<LivingRagdoll>(owner)
    {
        actor.released = Some(0.);
    }
    world.resource_mut::<PhysicsGun>().body = None;
}

fn drive(
    world: &mut World,
    mut spatial: Local<SystemState<SpatialQuery>>,
    mut actors: Local<QueryState<Entity, With<LivingRagdoll>>>,
    mut owners: Local<Vec<Entity>>,
) {
    if world.resource::<Cheats>().gun != Gun::Physics {
        release(world);
    }
    let aim = tools::aim(world);
    let held = world.resource::<PhysicsGun>().held;
    let dt = world.resource::<Time<Virtual>>().delta_secs();
    owners.clear();
    owners.extend(actors.iter(world));
    for &owner in owners.iter() {
        let pelvis = world
            .get::<crate::enemies::ragdoll::PhysicalRagdoll>(owner)
            .and_then(|ragdoll| ragdoll.bodies().next());
        let Some(pelvis) = pelvis else {
            // Loading/hidden rigs cannot leave an actor indefinitely suspended.
            let waiting = {
                let mut living = world.get_mut::<LivingRagdoll>(owner).unwrap();
                let elapsed = living.released.get_or_insert(0.);
                *elapsed += dt;
                *elapsed
            };
            if waiting > 2. {
                restore(world, owner);
            }
            continue;
        };
        let Some(pelvis_position) = world.get::<Position>(pelvis).map(|pose| pose.0) else {
            continue;
        };
        let offset = {
            let mut living = world.get_mut::<LivingRagdoll>(owner).unwrap();
            let initial = living.original_position;
            *living.root_offset.get_or_insert(initial - pelvis_position)
        };
        set_position(world, owner, pelvis_position + offset);
        if held == Some(owner) {
            let Some((player, origin, direction, _)) = aim else {
                release(world);
                continue;
            };
            let (distance, hit, selected) = {
                let gun = world.resource::<PhysicsGun>();
                (gun.distance, gun.hit, gun.body)
            };
            let body = selected.unwrap_or_else(|| {
                world
                    .get::<crate::enemies::ragdoll::PhysicalRagdoll>(owner)
                    .unwrap()
                    .bodies()
                    .filter_map(|entity| {
                        world
                            .get::<Position>(entity)
                            .map(|p| (entity, p.0.distance_squared(hit)))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map_or(pelvis, |pair| pair.0)
            });
            world.resource_mut::<PhysicsGun>().body = Some(body);
            let mut target = origin + *direction * distance;
            // Keep the grab point in front of walls instead of pulling through map geometry.
            let query = spatial.get(world).expect("game physics is installed");
            if let Some(hit) = query.cast_ray_predicate(
                origin,
                direction,
                distance,
                true,
                &crate::environment::solids([player, owner]),
                &|entity| world.get::<RigidBody>(entity) != Some(&RigidBody::Dynamic),
            ) {
                target = origin + *direction * (hit.distance - 0.35).max(0.5);
            }
            if let Some(position) = world.get::<Position>(body).map(|p| p.0) {
                let velocity = ((target - position) * 14.).clamp_length_max(25.);
                world
                    .entity_mut(body)
                    .insert(LinearVelocity(velocity))
                    .remove::<Sleeping>();
            }
        } else {
            let elapsed = {
                let mut living = world.get_mut::<LivingRagdoll>(owner).unwrap();
                let elapsed = living.released.get_or_insert(0.);
                *elapsed += dt;
                *elapsed
            };
            let query = spatial.get(world).expect("game physics is installed");
            let floor = query
                .cast_ray_predicate(
                    pelvis_position + Vec3::Y * 0.5,
                    Dir3::NEG_Y,
                    1.3,
                    true,
                    &crate::environment::actor_solids([owner]),
                    &|entity| world.get::<RigidBody>(entity) != Some(&RigidBody::Dynamic),
                )
                .filter(|hit| hit.normal.y > 0.5)
                .map(|hit| pelvis_position + Vec3::Y * (0.5 - hit.distance));
            if elapsed >= 0.15
                && let Some(feet) = floor
            {
                let center = world
                    .get::<Zombie>(owner)
                    .and_then(|zombie| {
                        world
                            .resource::<crate::assets::GameDefinition>()
                            .content
                            .enemies
                            .iter()
                            .find(|enemy| enemy.id == zombie.definition)
                            .map(|enemy| crate::enemies::body_center(enemy, zombie.crawler()))
                    })
                    .unwrap_or(Vec3::Y * 0.85);
                let center = if world.get::<crate::panzer::Panzer>(owner).is_some() {
                    Vec3::Y * 1.2
                } else {
                    center
                };
                let game = world.resource::<crate::assets::GameDefinition>();
                let supported = world.get::<Collider>(owner).and_then(|shape| {
                    crate::enemies::resolve_floor_overlap(
                        &query,
                        shape,
                        feet + center + Vec3::Y * 0.04,
                        &crate::environment::actor_solids([owner]),
                        &game.tuning,
                    )
                });
                if let Some(position) = supported {
                    set_position(world, owner, position);
                    restore(world, owner);
                }
            } else if elapsed > 20. {
                // An actor dropped outside the map returns to its last valid place.
                let original = world.get::<LivingRagdoll>(owner).unwrap().original_position;
                set_position(world, owner, original);
                restore(world, owner);
            }
        }
    }
    if held.is_some_and(|owner| world.get_entity(owner).is_err()) {
        *world.resource_mut::<PhysicsGun>() = default();
    }
}

fn set_position(world: &mut World, owner: Entity, position: Vec3) {
    if let Some(mut pose) = world.get_mut::<Transform>(owner) {
        pose.translation = position;
    }
    if let Some(mut pose) = world.get_mut::<Position>(owner) {
        pose.0 = position;
    }
}

pub(super) fn restore(world: &mut World, owner: Entity) {
    let Some(living) = world
        .get_entity_mut(owner)
        .ok()
        .and_then(|mut actor| actor.take::<LivingRagdoll>())
    else {
        return;
    };
    crate::enemies::ragdoll::remove_living(world, owner);
    for (entity, animation) in living.animations {
        if let Ok(mut rig) = world.get_entity_mut(entity) {
            rig.insert(animation);
        }
    }
    world.entity_mut(owner).remove::<(
        ColliderDisabled,
        crate::tank::TankMotion,
        crate::quest_recovery::PilotLanding,
    )>();
    if let Some(mut zombie) = world.get_mut::<Zombie>(owner) {
        zombie.resume_after_physics();
    }
    if world.resource::<PhysicsGun>().held == Some(owner) {
        *world.resource_mut::<PhysicsGun>() = default();
    }
}

pub(super) fn restore_all(world: &mut World) {
    let mut actors = world.query_filtered::<Entity, With<LivingRagdoll>>();
    let owners: Vec<_> = actors.iter(world).collect();
    for owner in owners {
        restore(world, owner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_keeps_the_living_actor_suspended_until_recovery_restores_animation_and_ai() {
        let mut world = World::new();
        world.init_resource::<PhysicsGun>();
        let node = AnimationNodeIndex::new(0);
        let mut animation = AnimationPlayer::default();
        animation.play(node).repeat();
        let saved = animation.clone();
        animation.pause_all();
        let rig = world.spawn(animation).id();
        let mut zombie = Zombie::new(
            Vec3::ZERO,
            crate::origins::ContentId("enemy.test".into()),
            crate::rounds::Gait::Run,
        );
        zombie.action = crate::origins::content::EnemyAnimation::Run;
        zombie.elapsed = 1.;
        zombie.stunned = 5.;
        let actor = world
            .spawn((
                zombie,
                ColliderDisabled,
                LivingRagdoll {
                    profile: None,
                    released: None,
                    original_position: Vec3::ZERO,
                    root_offset: None,
                    animations: vec![(rig, saved)],
                },
            ))
            .id();
        world.resource_mut::<PhysicsGun>().held = Some(actor);
        release(&mut world);
        assert!(
            world
                .get::<LivingRagdoll>(actor)
                .unwrap()
                .released
                .is_some()
        );
        assert!(
            world
                .get::<AnimationPlayer>(rig)
                .unwrap()
                .animation(node)
                .unwrap()
                .is_paused()
        );
        restore(&mut world, actor);
        assert!(world.get::<LivingRagdoll>(actor).is_none());
        assert!(world.get::<ColliderDisabled>(actor).is_none());
        assert!(
            !world
                .get::<AnimationPlayer>(rig)
                .unwrap()
                .animation(node)
                .unwrap()
                .is_paused()
        );
        let zombie = world.get::<Zombie>(actor).unwrap();
        assert_eq!(zombie.action, crate::origins::content::EnemyAnimation::Idle);
        assert_eq!(zombie.elapsed, 0.);
        assert_eq!(zombie.stunned, 0.);
        assert!(world.resource::<PhysicsGun>().held.is_none());
    }
}
