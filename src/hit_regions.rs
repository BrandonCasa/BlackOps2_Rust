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
    pub start: Entity,
    pub end: Entity,
    pub radius: f32,
}
pub struct HitVolume {
    pub id: String,
    pub region: HitRegion,
    pub start: Vec3,
    pub end: Vec3,
    shape: Collider,
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
        let volumes = [(HitRegion::Head, height * 0.35, height * 0.4, radius * 0.6),
            (HitRegion::Torso, -height * 0.12, height * 0.2, radius),
            (HitRegion::Leg, -height * 0.4, -height * 0.2, radius * 0.65),
            (HitRegion::Arm, height * 0.1, height * 0.25, radius * 0.5)]
            .into_iter().enumerate().map(|(index, (region, a, b, radius))| {
                let x = if region == HitRegion::Arm { radius * 2. } else { 0. };
                let start = Vec3::new(x, a, 0.); let end = Vec3::new(x, b, 0.);
                HitVolume { id: format!("body.{index}"), region, start, end, shape: Collider::capsule_endpoints(radius, start, end) }
            }).collect();
        Self { volumes, disabled: Vec::new() }
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
            .filter(|volume| !self.disabled.contains(&volume.region))
            .filter_map(|volume| {
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
                    .map(|(distance, _)| RegionHit {
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
pub type PenetrationSurfaces<'w, 's> = Query<'w, 's, (&'static Collider, &'static GlobalTransform, Option<&'static crate::audio::SurfaceSound>)>;
pub fn penetrating(
    targets: &RayTargets, surfaces: &PenetrationSurfaces, world: &SpatialQuery,
    shooter: Entity, origin: Vec3, direction: Dir3, range: f32, strength: f32,
) -> Vec<(ActorHit, f32)> {
    let mut actors: Vec<_> = targets.iter()
        .filter(|(_, health, _, _)| health.state() == LifeState::ActorAlive)
        .filter_map(|(target, _, pose, boxes)| boxes.cast(pose, origin, direction, range).map(|hit| ActorHit { target, hit }))
        .collect();
    actors.sort_by(|a,b| a.hit.distance.total_cmp(&b.hit.distance));
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut crossed = std::collections::HashSet::new();
    let mut distance = 0.;
    let mut penetrations = 0;
    let mut energy = strength.max(0.);
    let initial = energy.max(0.001);
    for actor in actors {
        if !seen.insert(actor.target) { continue; }
        while distance < actor.hit.distance {
            let blocker = world.cast_ray_predicate(origin + *direction * distance, direction,
                actor.hit.distance - distance, true, &crate::environment::solids([shooter]),
                &|entity| !targets.contains(entity) && !crossed.contains(&entity));
            let Some(blocker) = blocker else { break; };
            if energy <= 0. || penetrations >= 5 { return result; }
            let Ok((shape, pose, surface)) = surfaces.get(blocker.entity) else { return result; };
            let resistance = match surface.map(|s| s.0.as_str()).unwrap_or("") {
                "glass" => 0.25, "wood" => 0.6, "plaster" | "drywall" => 1.,
                "metal" => 3., "concrete" | "stone" | "brick" => 4., _ => return result,
            };
            let entry = distance + blocker.distance + 0.002;
            let pose = pose.compute_transform();
            let Some((thickness, _)) = shape.cast_ray(pose.translation, pose.rotation,
                origin + *direction * entry, direction.as_vec3(), range - entry, false) else { return result; };
            energy -= thickness.max(0.0254) * resistance;
            if energy <= 0. { return result; }
            crossed.insert(blocker.entity);
            penetrations += 1;
            distance = entry + thickness + 0.002;
        }
        result.push((actor, if strength <= 0. { 1. } else { (energy / initial).clamp(0., 1.) }));
        if strength <= 0. { break; }
        if penetrations >= 5 { break; }
        let Ok((_, _, pose, boxes)) = targets.get(actor.target) else { break; };
        let thickness = boxes.volumes.iter().filter(|v| !boxes.disabled.contains(&v.region))
            .filter_map(|volume| volume.shape.cast_ray(pose.translation, pose.rotation,
                actor.hit.point + *direction * 0.002, direction.as_vec3(), range - actor.hit.distance, false))
            .map(|(distance, _)| distance).min_by(f32::total_cmp).unwrap_or(0.0254);
        energy -= initial * thickness.max(0.0254) / 2.54;
        penetrations += 1;
        if energy <= 0. { break; }
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
            let (Ok(start), Ok(end)) = (joints.get(binding.start), joints.get(binding.end)) else {
                continue;
            };
            let start = inverse.transform_point3(start.translation());
            let end = inverse.transform_point3(end.translation());
            if !start.is_finite() || !end.is_finite() {
                continue;
            }
            if let Some(volume) = hitboxes.volumes.iter_mut().find(|v| v.id == binding.id) {
                if volume.start == start && volume.end == end {
                    continue;
                }
                volume.start = start;
                volume.end = end;
                volume.shape = Collider::capsule_endpoints(binding.radius, start, end);
            } else {
                hitboxes.volumes.push(HitVolume {
                    id: binding.id.clone(),
                    region: binding.region,
                    start,
                    end,
                    shape: Collider::capsule_endpoints(binding.radius, start, end),
                });
                evidence.bound_volumes += 1;
            }
            evidence.pose_updates += 1;
            evidence.regions.insert(format!("{:?}", binding.region));
        }
    }
}
