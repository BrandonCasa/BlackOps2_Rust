use super::EnvironmentDefinition;
use crate::{
    GameState,
    damage::{DamageQueue, Impact, ImpactSource},
    gameplay::GameStep,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Component, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentOwner {
    Boundary,
    Movement,
    Access,
    Ambience,
    Robot,
    Vehicle,
    Quest,
    Crafting,
    Deferred,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnershipDefinition {
    pub id: String,
    pub owner: EnvironmentOwner,
    pub shootable: bool,
}

pub(super) fn valid(definition: &EnvironmentDefinition) -> bool {
    let ids: BTreeSet<_> = definition
        .triggers
        .iter()
        .map(|t| &t.id)
        .chain(definition.brush_entities.iter().map(|b| &b.id))
        .collect();
    let owned: BTreeSet<_> = definition.ownership.iter().map(|o| &o.id).collect();
    owned.len() == definition.ownership.len() && owned == ids
}

#[derive(Component)]
pub struct ShootableEnvironment;

pub type ShootableTargets<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Collider,
        &'static GlobalTransform,
        &'static EnvironmentOwner,
    ),
    With<ShootableEnvironment>,
>;

pub fn cast(
    targets: &ShootableTargets,
    spatial: &SpatialQuery,
    shooter: Entity,
    origin: Vec3,
    direction: Dir3,
    range: f32,
) -> Option<(Entity, f32)> {
    let candidate = targets
        .iter()
        .filter_map(|(e, shape, pose, _)| {
            let transform = pose.compute_transform();
            shape
                .cast_ray(
                    transform.translation,
                    transform.rotation,
                    origin,
                    direction.as_vec3(),
                    range,
                    true,
                )
                .map(|(d, _)| (e, d))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    let blocker = spatial.cast_ray(origin, direction, range, true, &super::solids([shooter]));
    if blocker.is_some_and(|b| b.distance + 0.01 < candidate.1) {
        None
    } else {
        Some(candidate)
    }
}

#[derive(Message, Clone)]
pub struct EnvironmentalHit {
    pub target: Entity,
    pub attacker: Entity,
    pub weapon: crate::origins::WeaponId,
    pub amount: u32,
    pub point: Vec3,
    pub direction: Vec3,
}

#[derive(Clone, Component, Debug)]
pub enum DamageEmitter {
    Robot(String),
    Vehicle(String),
}
impl DamageEmitter {
    pub fn source(&self) -> ImpactSource {
        match self {
            Self::Robot(id) => ImpactSource::Robot(id.clone()),
            Self::Vehicle(id) => ImpactSource::Vehicle(id.clone()),
        }
    }
    pub fn matches(&self, source: &ImpactSource) -> bool {
        matches!((self, source), (Self::Robot(a), ImpactSource::Robot(b)) | (Self::Vehicle(a), ImpactSource::Vehicle(b)) if a == b)
    }
}

#[derive(Component)]
pub struct DamageContact {
    pub(crate) session: crate::session::SessionId,
    pub amount: u32,
}

#[derive(Resource, Default, Serialize)]
pub struct EnvironmentEvidence {
    pub hits: Vec<serde_json::Value>,
    pub contacts: u32,
}

pub struct SupportPlugin;
impl Plugin for SupportPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<EnvironmentalHit>()
            .init_resource::<EnvironmentEvidence>()
            .add_systems(
                FixedUpdate,
                (contacts, observe_hits)
                    .in_set(GameStep::Combat)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn contacts(
    session: Res<crate::session::SessionId>,
    spatial: SpatialQuery,
    emitters: Query<(
        Entity,
        &DamageEmitter,
        &DamageContact,
        &Collider,
        &GlobalTransform,
    )>,
    targets: Query<(), With<crate::life::Vitality>>,
    mut damage: ResMut<DamageQueue>,
    mut evidence: ResMut<EnvironmentEvidence>,
) {
    for (owner, emitter, contact, shape, pose) in &emitters {
        if contact.session != *session {
            continue;
        }
        let pose = pose.compute_transform();
        for target in spatial.shape_intersections(
            shape,
            pose.translation,
            pose.rotation,
            &SpatialQueryFilter::default().with_excluded_entities([owner]),
        ) {
            if targets.contains(target) {
                damage.0.push(Impact {
                    attacker: owner,
                    target,
                    source: emitter.source(),
                    category: crate::origins::DamageCategory::Crush,
                    region: crate::weapons::HitRegion::Torso,
                    amount: contact.amount,
                });
                evidence.contacts += 1;
            }
        }
    }
}

fn observe_hits(
    mut hits: MessageReader<EnvironmentalHit>,
    owners: Query<&EnvironmentOwner>,
    mut evidence: ResMut<EnvironmentEvidence>,
) {
    for hit in hits.read() {
        if evidence.hits.len() >= 128 {
            evidence.hits.remove(0);
        }
        evidence.hits.push(serde_json::json!({"target":hit.target.to_bits(),"attacker":hit.attacker.to_bits(),"weapon":hit.weapon.0,"amount":hit.amount,"point":hit.point.to_array(),"owner":owners.get(hit.target).ok()}));
    }
}
