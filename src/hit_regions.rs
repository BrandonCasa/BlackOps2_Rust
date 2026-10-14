use crate::{
    life::{LifeState, Vitality},
    player::Player,
    weapons::HitRegion,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::BTreeSet;

#[derive(Component)]
pub(crate) struct SkeletalReady;
#[derive(Component)]
pub(crate) struct HitRig {
    pub owner: Entity,
    pub bindings: Vec<HitBinding>,
}
pub(crate) struct HitBinding {
    pub id: String,
    pub region: HitRegion,
    pub joint: Entity,
    pub bounds: [[f32; 3]; 2],
    pub sphere: [f32; 4],
}
struct NativeVolume {
    bone: bevy::math::Affine3A,
    bounds: [Vec3; 2],
    sphere: [f32; 4],
}
pub struct HitVolume {
    pub id: String,
    pub region: HitRegion,
    pub start: Vec3,
    pub end: Vec3,
    shape: Collider,
    native: Option<NativeVolume>,
}
#[derive(Component, Default)]
pub struct Hitboxes {
    pub disabled: Vec<HitRegion>,
    pub volumes: Vec<HitVolume>,
}
#[derive(Clone, Copy)]
pub struct RegionHit {
    pub region: HitRegion,
    pub distance: f32,
    pub point: Vec3,
}
#[derive(Clone, Copy)]
pub struct ActorHit {
    pub target: Entity,
    pub hit: RegionHit,
}
impl Hitboxes {
    pub fn humanoid(height: f32, radius: f32) -> Self {
        let volumes = [
            (HitRegion::Head, height * 0.35, height * 0.4, radius * 0.6),
            (HitRegion::Torso, -height * 0.12, height * 0.2, radius),
            (HitRegion::Leg, -height * 0.4, -height * 0.2, radius * 0.65),
            (HitRegion::Arm, height * 0.1, height * 0.25, radius * 0.5),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (region, a, b, radius))| {
            let x = if region == HitRegion::Arm {
                radius * 2.
            } else {
                0.
            };
            let start = Vec3::new(x, a, 0.);
            let end = Vec3::new(x, b, 0.);
            HitVolume {
                id: format!("body.{index}"),
                region,
                start,
                end,
                shape: Collider::capsule_endpoints(radius, start, end),
                native: None,
            }
        })
        .collect();
        Self {
            volumes,
            disabled: Vec::new(),
        }
    }
    pub fn panzer() -> Self {
        let mut boxes = Self::humanoid(2.4, 0.5);
        boxes.volumes.retain(|v| v.region != HitRegion::Arm);
        for (region, x) in [(HitRegion::LeftArm, 0.6), (HitRegion::RightArm, -0.6)] {
            let start = Vec3::new(x, 0.25, 0.);
            let end = Vec3::new(x, 0.6, 0.);
            boxes.volumes.push(HitVolume {
                id: format!("panzer.{region:?}"),
                region,
                start,
                end,
                shape: Collider::capsule_endpoints(0.2, start, end),
                native: None,
            });
        }
        let start = Vec3::new(0., 0.4, 0.4);
        let end = Vec3::new(0., 0.65, 0.4);
        boxes.volumes.push(HitVolume {
            id: "panzer.powerplant".into(),
            region: HitRegion::Powerplant,
            start,
            end,
            shape: Collider::capsule_endpoints(0.15, start, end),
            native: None,
        });
        boxes
    }
    pub fn nearest_region(&self, transform: &Transform, point: Vec3) -> HitRegion {
        let local = transform.compute_affine().inverse().transform_point3(point);
        self.volumes
            .iter()
            .filter(|v| !self.disabled_region(v.region))
            .min_by(|a, b| {
                ((a.start + a.end) * 0.5)
                    .distance_squared(local)
                    .total_cmp(&((b.start + b.end) * 0.5).distance_squared(local))
            })
            .map_or(HitRegion::None, |v| v.region)
    }
    fn disabled_region(&self, region: HitRegion) -> bool {
        self.disabled.iter().any(|disabled| {
            *disabled == region
                || (*disabled == HitRegion::Head && region.head())
                || (*disabled == HitRegion::Leg && region.leg())
                || (*disabled == HitRegion::LeftArm && region.left_arm())
                || (*disabled == HitRegion::RightArm && region.right_arm())
        })
    }
    fn occupied_distance(
        &self,
        pose: &Transform,
        origin: Vec3,
        direction: Dir3,
        range: f32,
    ) -> f32 {
        let mut intervals: Vec<_> = self
            .volumes
            .iter()
            .filter(|v| !self.disabled_region(v.region))
            .filter_map(|v| {
                let (start, _) = v.shape.cast_ray(
                    pose.translation,
                    pose.rotation,
                    origin,
                    *direction,
                    range,
                    true,
                )?;
                let end = if v.shape.contains_point(
                    pose.translation,
                    pose.rotation,
                    origin + *direction * range,
                ) {
                    range
                } else {
                    let (reverse, _) = v.shape.cast_ray(
                        pose.translation,
                        pose.rotation,
                        origin + *direction * range,
                        -*direction,
                        range,
                        true,
                    )?;
                    range - reverse
                };
                (end > start).then_some((start, end))
            })
            .collect();
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        if self.volumes.iter().any(|volume| volume.native.is_some()) {
            return intervals.first().map_or(0., |first| {
                intervals
                    .iter()
                    .map(|interval| interval.1)
                    .fold(first.1, f32::max)
                    - first.0
            });
        }
        let mut total = 0.;
        let mut previous: Option<(f32, f32)> = None;
        for (start, end) in intervals {
            if let Some((a, b)) = previous {
                if start <= b {
                    previous = Some((a, b.max(end)));
                } else {
                    total += b - a;
                    previous = Some((start, end));
                }
            } else {
                previous = Some((start, end));
            }
        }
        total + previous.map_or(0., |(a, b)| b - a)
    }
    pub fn cast(
        &self,
        transform: &Transform,
        origin: Vec3,
        direction: Dir3,
        range: f32,
    ) -> Option<RegionHit> {
        self.volumes
            .iter()
            .filter(|volume| !self.disabled_region(volume.region))
            .filter_map(|volume| {
                let distance = if let Some(native) = &volume.native {
                    native.cast(transform, origin, direction.as_vec3(), range)
                } else {
                    volume
                        .shape
                        .cast_ray(
                            transform.translation,
                            transform.rotation,
                            origin,
                            direction.as_vec3(),
                            range,
                            true,
                        )
                        .map(|(distance, _)| distance)
                }?;
                Some(RegionHit {
                    region: volume.region,
                    distance,
                    point: origin + direction.as_vec3() * distance,
                })
            })
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }
    pub fn aim_points(
        &self,
        transform: &Transform,
        region: HitRegion,
    ) -> impl Iterator<Item = Vec3> + '_ {
        let affine = transform.compute_affine();
        self.volumes
            .iter()
            .filter(move |v| v.region == region)
            .map(move |v| affine.transform_point3((v.start + v.end) * 0.5))
    }
}
impl NativeVolume {
    fn cast(&self, actor: &Transform, origin: Vec3, direction: Vec3, range: f32) -> Option<f32> {
        let inverse = (actor.compute_affine() * self.bone).inverse();
        let start = inverse.transform_point3(origin);
        let delta = inverse.transform_vector3(direction) * range;
        let center = Vec3::new(self.sphere[0], self.sphere[1], self.sphere[2]);
        let squared = delta.length_squared();
        if squared == 0. {
            return None;
        }
        let closest = start + delta * ((center - start).dot(delta) / squared).clamp(0., 1.);
        if closest.distance_squared(center) >= self.sphere[3] {
            return None;
        }
        let end = start + delta;
        let inside = |point: Vec3| {
            (0..3).all(|axis| {
                point[axis] >= self.bounds[0][axis] && point[axis] <= self.bounds[1][axis]
            })
        };
        if inside(start) {
            let inverse = actor.compute_affine().inverse();
            let local_origin = inverse.transform_point3(origin);
            let local_direction = inverse.transform_vector3(direction);
            return (inside(end)
                && local_origin.x * local_direction.x + local_origin.z * local_direction.z < 0.)
                .then_some(0.);
        }
        let mut enter: f32 = 0.;
        let mut exit: f32 = 1.;
        for axis in 0..3 {
            if delta[axis] == 0. {
                if start[axis] < self.bounds[0][axis] || start[axis] > self.bounds[1][axis] {
                    return None;
                }
            } else {
                let a = (self.bounds[0][axis] - start[axis]) / delta[axis];
                let b = (self.bounds[1][axis] - start[axis]) / delta[axis];
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
                if exit <= enter {
                    return None;
                }
            }
        }
        (enter < 1.).then_some(enter * range)
    }
}
pub type RayTargets<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Vitality,
        &'static Transform,
        &'static Hitboxes,
    ),
    Without<Player>,
>;
pub fn cast(
    targets: &RayTargets,
    world: &SpatialQuery,
    shooter: Entity,
    origin: Vec3,
    direction: Dir3,
    range: f32,
) -> Option<ActorHit> {
    let hit = targets
        .iter()
        .filter(|(_, health, _, _)| health.state() == LifeState::ActorAlive)
        .filter_map(|(target, _, transform, hitboxes)| {
            hitboxes
                .cast(transform, origin, direction, range)
                .map(|hit| ActorHit { target, hit })
        })
        .min_by(|a, b| a.hit.distance.total_cmp(&b.hit.distance))?;
    let blocker = world.cast_ray_predicate(
        origin,
        direction,
        range,
        true,
        &crate::environment::solids([shooter]),
        &|entity| !targets.contains(entity),
    );
    if blocker.is_some_and(|b| b.distance <= hit.hit.distance) {
        None
    } else {
        Some(hit)
    }
}
pub fn melee(
    targets: &RayTargets,
    world: &SpatialQuery,
    shooter: Entity,
    eye: Vec3,
    rotation: Quat,
    range: f32,
    [width, height]: [f32; 2],
) -> Option<ActorHit> {
    let forward = rotation * Vec3::NEG_Z;
    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let origin = eye - forward * (10. * 0.0254);
    for [x, y] in [
        [0., 0.],
        [0.5, 0.],
        [-0.5, 0.],
        [0., 0.5],
        [0., -0.5],
        [1., 1.],
        [-1., 1.],
        [1., -1.],
        [-1., -1.],
    ] {
        let delta = eye + forward * range + right * (x * width) + up * (y * height) - origin;
        let Ok(direction) = Dir3::new(delta) else {
            continue;
        };
        if let Some(mut hit) = cast(targets, world, shooter, origin, direction, delta.length()) {
            hit.hit.distance = hit.hit.point.distance(eye);
            return Some(hit);
        }
    }
    None
}
pub type PenetrationSurfaces<'w, 's> = Query<
    'w,
    's,
    (
        &'static Collider,
        &'static GlobalTransform,
        &'static crate::movement::physics::SurfaceFlags,
    ),
>;
pub fn penetrating(
    targets: &RayTargets,
    surfaces: &PenetrationSurfaces,
    world: &SpatialQuery,
    shooter: Entity,
    origin: Vec3,
    direction: Dir3,
    range: f32,
    depths: Option<&[f32; 32]>,
) -> Vec<(ActorHit, f32)> {
    let mut actors: Vec<_> = targets
        .iter()
        .filter(|(_, health, _, _)| health.state() == LifeState::ActorAlive)
        .filter_map(|(target, _, pose, boxes)| {
            boxes
                .cast(pose, origin, direction, range)
                .map(|hit| ActorHit { target, hit })
        })
        .collect();
    actors.sort_by(|a, b| a.hit.distance.total_cmp(&b.hit.distance));
    let mut result = Vec::new();
    let mut distance = 0.;
    let mut penetrations = 0;
    let mut energy = 1.;
    for actor in actors {
        while distance < actor.hit.distance {
            let Some(blocker) = world.cast_ray_predicate(
                origin + *direction * distance,
                direction,
                actor.hit.distance - distance,
                true,
                &crate::environment::solids([shooter]),
                &|entity| !targets.contains(entity),
            ) else {
                break;
            };
            let Some(depths) = depths else {
                return result;
            };
            if penetrations >= 5 {
                return result;
            }
            let Ok((shape, pose, flags)) = surfaces.get(blocker.entity) else {
                return result;
            };
            let surface = ((flags.0 >> 20) & 0x3f) as usize;
            let depth = if surface == 20 {
                100. * 0.0254
            } else {
                depths.get(surface).copied().unwrap_or(0.)
            };
            if depth <= 0. {
                return result;
            }
            let incidence = -blocker.normal.dot(*direction);
            if incidence < 0.125 {
                return result;
            }
            let entry = distance + blocker.distance;
            let step = 0.135 * 0.0254 / incidence;
            let pose = pose.compute_transform();
            let Some((exit, _)) = shape.cast_ray(
                pose.translation,
                pose.rotation,
                origin + *direction * (entry + step),
                *direction,
                range - entry - step,
                false,
            ) else {
                return result;
            };
            let thickness = exit + step;
            energy -= thickness.max(0.0254) / depth;
            if energy <= 0. {
                return result;
            }
            penetrations += 1;
            distance = entry + thickness + 0.01 * 0.0254;
        }
        if actor.hit.distance < distance {
            continue;
        }
        result.push((actor, energy));
        if depths.is_none() || penetrations >= 5 {
            break;
        }
        let Ok((_, _, pose, boxes)) = targets.get(actor.target) else {
            break;
        };
        energy -= boxes
            .occupied_distance(pose, origin, direction, range)
            .max(0.0254)
            / (100. * 0.0254);
        penetrations += 1;
        if energy <= 0. {
            break;
        }
    }
    result
}
#[derive(Resource, Default)]
pub struct HitEvidence {
    pub pose_updates: u64,
    pub bound_volumes: u64,
    pub regions: BTreeSet<String>,
    pub hits: Vec<serde_json::Value>,
}
pub struct HitRegionsPlugin;
impl Plugin for HitRegionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HitEvidence>().add_systems(
            PostUpdate,
            capture.after(bevy::transform::TransformSystems::Propagate),
        );
    }
}
fn capture(
    rigs: Query<&HitRig>,
    joints: Query<&GlobalTransform>,
    mut actors: Query<(&GlobalTransform, &mut Hitboxes), With<SkeletalReady>>,
    mut evidence: ResMut<HitEvidence>,
) {
    for rig in &rigs {
        let Ok((owner, mut hitboxes)) = actors.get_mut(rig.owner) else {
            continue;
        };
        let inverse = owner.affine().inverse();
        for binding in &rig.bindings {
            let Ok(joint) = joints.get(binding.joint) else {
                continue;
            };
            let bone = inverse * joint.affine();
            let bounds = binding.bounds.map(Vec3::from_array);
            let center = (bounds[0] + bounds[1]) * 0.5;
            let start = bone.transform_point3(center);
            if !start.is_finite() {
                continue;
            }
            let (scale, rotation, _) = bone.to_scale_rotation_translation();
            let extent = (bounds[1] - bounds[0]) * scale.abs();
            let shape = Collider::compound(vec![(
                start,
                rotation,
                Collider::cuboid(extent.x, extent.y, extent.z),
            )]);
            let native = Some(NativeVolume {
                bone,
                bounds,
                sphere: binding.sphere,
            });
            if let Some(volume) = hitboxes.volumes.iter_mut().find(|v| v.id == binding.id) {
                volume.start = start;
                volume.end = start;
                volume.shape = shape;
                volume.native = native;
            } else {
                hitboxes.volumes.push(HitVolume {
                    id: binding.id.clone(),
                    region: binding.region,
                    start,
                    end: start,
                    shape,
                    native,
                });
                evidence.bound_volumes += 1;
            }
            evidence.pose_updates += 1;
            evidence.regions.insert(format!("{:?}", binding.region));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::SystemState;

    #[test]
    fn native_bone_bounds_follow_rotation_and_reject_sphere_only_hits() {
        let mut volume = NativeVolume {
            bone: bevy::math::Affine3A::IDENTITY,
            bounds: [Vec3::new(-0.2, -0.1, -0.05), Vec3::new(0.2, 0.1, 0.05)],
            sphere: [0., 0., 0., 0.0525],
        };
        let actor = Transform::from_xyz(0., 0., -1.);
        assert!(
            (volume
                .cast(&actor, Vec3::new(0.15, 0., 0.), Vec3::NEG_Z, 2.)
                .unwrap()
                - 0.95)
                .abs()
                < 0.00001
        );
        volume.bone = bevy::math::Affine3A::from_rotation_y(std::f32::consts::FRAC_PI_2);
        assert!(
            volume
                .cast(&actor, Vec3::new(0.15, 0., 0.), Vec3::NEG_Z, 2.)
                .is_none()
        );
        assert!((volume.cast(&actor, Vec3::ZERO, Vec3::NEG_Z, 2.).unwrap() - 0.8).abs() < 0.00001);
        assert!(
            volume
                .cast(&actor, Vec3::new(0., 0.15, 0.), Vec3::NEG_Z, 2.)
                .is_none()
        );
    }

    #[test]
    fn native_actor_penetration_uses_hundred_inch_depth_and_five_passages() {
        let mut world = World::new();
        world.init_resource::<avian3d::collider_tree::ColliderTrees>();
        let shooter = world.spawn_empty().id();
        for index in 1..=7 {
            world.spawn((Vitality::ordinary_actor(150,150).unwrap(),
                Transform::from_xyz(0.,0.,-(index as f32)),Hitboxes {
                    disabled:Vec::new(),volumes:vec![HitVolume {id:format!("actor.{index}"),
                        region:HitRegion::UpperTorso,start:Vec3::ZERO,end:Vec3::ZERO,
                        shape:Collider::cuboid(0.2,0.2,0.0254),native:None}]
                }));
        }
        let mut state = SystemState::<(RayTargets,PenetrationSurfaces,SpatialQuery)>::new(&mut world);
        let (targets,surfaces,query) = state.get(&world).unwrap();
        assert_eq!(penetrating(&targets,&surfaces,&query,shooter,Vec3::ZERO,Dir3::NEG_Z,10.,None).len(),1);
        let depths = [0.;32];
        let hits = penetrating(&targets,&surfaces,&query,shooter,Vec3::ZERO,Dir3::NEG_Z,10.,Some(&depths));
        assert_eq!(hits.len(),6);
        for (index,(_,energy)) in hits.into_iter().enumerate() {
            assert!((energy-(1.-index as f32*0.01)).abs()<0.00001);
        }
    }

    #[test]
    fn melee_fan_reaches_side_target_and_respects_source_range() {
        let mut world = World::new();
        world.init_resource::<avian3d::collider_tree::ColliderTrees>();
        let shooter = world.spawn_empty().id();
        let target = world
            .spawn((
                Vitality::ordinary_actor(150, 150).unwrap(),
                Transform::from_xyz(0.1, 0., -1.),
                Hitboxes {
                    disabled: Vec::new(),
                    volumes: vec![HitVolume {
                        id: "side".into(),
                        region: HitRegion::Torso,
                        start: Vec3::ZERO,
                        end: Vec3::ZERO,
                        shape: Collider::sphere(0.02),
                        native: None,
                    }],
                },
            ))
            .id();
        let mut state = SystemState::<(RayTargets, SpatialQuery)>::new(&mut world);
        {
            let (targets, query) = state.get(&world).unwrap();
            assert!(cast(&targets, &query, shooter, Vec3::ZERO, Dir3::NEG_Z, 1.397).is_none());
            let hit = melee(
                &targets,
                &query,
                shooter,
                Vec3::ZERO,
                Quat::IDENTITY,
                55. * 0.0254,
                [10. * 0.0254; 2],
            )
            .unwrap();
            assert_eq!(hit.target, target);
            assert!(hit.hit.distance < 55. * 0.0254);
        }
        world.get_mut::<Transform>(target).unwrap().translation.z = -1.5;
        let (targets, query) = state.get(&world).unwrap();
        assert!(
            melee(
                &targets,
                &query,
                shooter,
                Vec3::ZERO,
                Quat::IDENTITY,
                55. * 0.0254,
                [10. * 0.0254; 2],
            )
            .is_none()
        );
    }
}
