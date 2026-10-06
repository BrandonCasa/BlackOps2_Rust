//! Skeletal pose snapshots and Avian geometric hit queries, independent of movement colliders.
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
    pub fn cast(
        &self,
        transform: &Transform,
        origin: Vec3,
        direction: Dir3,
        range: f32,
    ) -> Option<RegionHit> {
        self.volumes
            .iter()
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
        &SpatialQueryFilter::from_excluded_entities([shooter]),
        &|entity| !targets.contains(entity),
    );
    if blocker.is_some_and(|b| b.distance <= hit.hit.distance) {
        None
    } else {
        Some(hit)
    }
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
