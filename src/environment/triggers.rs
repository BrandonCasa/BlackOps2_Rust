use super::VolumeDefinition;
use crate::{assets::GameDefinition, player::Player};
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const TRIGGER_LAYER: u32 = 8;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerDefinition {
    pub id: String,
    pub position: [f32; 3],
    pub source: BTreeMap<String, String>,
    pub shape: TriggerShape,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TriggerShape {
    Convex {
        volumes: Vec<VolumeDefinition>,
    },
    Cylinder {
        radius: f32,
        height: f32,
    },
    Brush {
        rotation: [f32; 4],
        volumes: Vec<VolumeDefinition>,
        triangle_shapes: Vec<super::brushes::TriangleShape>,
    },
}

impl TriggerDefinition {
    pub fn collider(&self) -> Option<Collider> {
        match &self.shape {
            TriggerShape::Brush {
                volumes,
                triangle_shapes,
                ..
            } => super::brushes::compound(volumes, triangle_shapes),
            TriggerShape::Convex { volumes } => {
                let pieces: Option<Vec<_>> = volumes
                    .iter()
                    .map(|v| {
                        Collider::convex_hull(
                            &v.vertices
                                .iter()
                                .copied()
                                .map(Vec3::from_array)
                                .collect::<Vec<_>>(),
                        )
                        .map(|shape| (Vec3::from_array(v.position), Quat::IDENTITY, shape))
                    })
                    .collect();
                pieces.filter(|p| !p.is_empty()).map(Collider::compound)
            }
            TriggerShape::Cylinder { radius, height } => Some(Collider::compound(vec![(
                Vec3::Y * height * 0.5,
                Quat::IDENTITY,
                Collider::cylinder(*radius, *height),
            )])),
        }
    }
    pub fn transform(&self) -> Transform {
        let rotation = match &self.shape {
            TriggerShape::Brush { rotation, .. } => Quat::from_array(*rotation),
            _ => Quat::IDENTITY,
        };
        Transform::from_translation(Vec3::from_array(self.position)).with_rotation(rotation)
    }
    pub fn interior_points(&self) -> Vec<Vec3> {
        let origin = Vec3::from_array(self.position);
        match &self.shape {
            TriggerShape::Brush {
                volumes,
                triangle_shapes,
                ..
            } => {
                let pose = self.transform();
                volumes
                    .iter()
                    .map(|volume| pose.transform_point(Vec3::from_array(volume.position)))
                    .chain(triangle_shapes.iter().flat_map(|shape| {
                        shape.indices.iter().map(move |indices| {
                            pose.transform_point(
                                (Vec3::from_array(shape.vertices[indices[0] as usize])
                                    + Vec3::from_array(shape.vertices[indices[1] as usize])
                                    + Vec3::from_array(shape.vertices[indices[2] as usize]))
                                    / 3.,
                            )
                        })
                    }))
                    .collect()
            }
            TriggerShape::Convex { volumes } => volumes
                .iter()
                .map(|v| origin + Vec3::from_array(v.position))
                .collect(),
            TriggerShape::Cylinder { height, .. } => vec![origin + Vec3::Y * height * 0.5],
        }
    }
    pub fn outside_point(&self) -> Vec3 {
        let max = match &self.shape {
            TriggerShape::Brush {
                volumes,
                triangle_shapes,
                ..
            } => {
                let rotation = self.transform().rotation;
                volumes
                    .iter()
                    .flat_map(|volume| {
                        volume.vertices.iter().map(move |vertex| {
                            rotation
                                * (Vec3::from_array(volume.position) + Vec3::from_array(*vertex))
                        })
                    })
                    .chain(triangle_shapes.iter().flat_map(|shape| {
                        shape
                            .vertices
                            .iter()
                            .map(|vertex| rotation * Vec3::from_array(*vertex))
                    }))
                    .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max)
            }
            TriggerShape::Convex { volumes } => volumes
                .iter()
                .flat_map(|v| {
                    v.vertices
                        .iter()
                        .map(|p| Vec3::from_array(v.position) + Vec3::from_array(*p))
                })
                .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max),
            TriggerShape::Cylinder { radius, height } => Vec3::new(*radius, *height, *radius),
        };
        Vec3::from_array(self.position) + max + Vec3::splat(0.1)
    }
}

fn stable_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() < 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub(super) fn valid(triggers: &[TriggerDefinition]) -> bool {
    let mut ids = BTreeSet::new();
    let mut pieces = BTreeSet::new();
    triggers.len() <= 4096
        && triggers.iter().all(|t| {
            stable_id(&t.id)
                && ids.insert(&t.id)
                && t.position.iter().all(|v| v.is_finite())
                && (1..=64).contains(&t.source.len())
                && t.source.iter().all(|(k, v)| {
                    !k.is_empty()
                        && k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                        && v.len() <= 4096
                })
                && t.source.get("classname").is_some_and(|class| {
                    matches!(
                        class.as_str(),
                        "trigger_multiple"
                            | "trigger_use"
                            | "trigger_damage"
                            | "trigger_box"
                            | "trigger_radius"
                            | "trigger_lookat"
                            | "info_volume"
                    ) && match &t.shape {
                        TriggerShape::Brush {
                            rotation,
                            volumes,
                            triangle_shapes,
                        } => {
                            class != "trigger_radius"
                                && rotation.iter().all(|value| value.is_finite())
                                && (Quat::from_array(*rotation).length_squared() - 1.).abs()
                                    < 0.00001
                                && (!volumes.is_empty() || !triangle_shapes.is_empty())
                                && volumes.len() <= 4096
                                && triangle_shapes.len() <= 4096
                                && volumes.iter().all(|volume| {
                                    stable_id(&volume.id)
                                        && pieces.insert(&volume.id)
                                        && (4..=128).contains(&volume.vertices.len())
                                        && volume
                                            .position
                                            .iter()
                                            .chain(volume.vertices.iter().flatten())
                                            .all(|value| value.is_finite())
                                })
                                && triangle_shapes.iter().all(|shape| {
                                    super::brushes::valid_triangle(shape)
                                        && pieces.insert(&shape.id)
                                })
                        }
                        TriggerShape::Convex { volumes } => {
                            class != "trigger_radius"
                                && (1..=4096).contains(&volumes.len())
                                && volumes.iter().all(|v| {
                                    stable_id(&v.id)
                                        && pieces.insert(&v.id)
                                        && v.position.iter().all(|p| p.is_finite())
                                        && (4..=128).contains(&v.vertices.len())
                                        && v.vertices.iter().flatten().all(|p| p.is_finite())
                                })
                        }
                        TriggerShape::Cylinder { radius, height } => {
                            class == "trigger_radius"
                                && radius.is_finite()
                                && *radius > 0.
                                && height.is_finite()
                                && *height > 0.
                        }
                    }
                })
                && t.collider().is_some()
        })
        && pieces.len() <= 32768
}

#[derive(Component)]
pub struct MapTrigger(pub String);

pub(super) fn spawn(
    mut commands: Commands,
    game: Res<GameDefinition>,
    existing: Query<(), With<MapTrigger>>,
) {
    if !existing.is_empty() {
        return;
    }
    for trigger in &game.environment.triggers {
        if game
            .environment
            .crafting_benches
            .iter()
            .any(|bench| bench.source_trigger == trigger.id && bench.delete_source)
        {
            continue;
        }
        if let Some(shape) = trigger.collider() {
            let mut entity = commands.spawn((
                MapTrigger(trigger.id.clone()),
                crate::gameplay::SessionEntity,
                shape,
                Sensor,
                CollisionLayers::new(TRIGGER_LAYER, LayerMask::NONE),
                trigger.transform(),
                Name::new(trigger.id.clone()),
            ));
            if let Some(owner) = game
                .environment
                .ownership
                .iter()
                .find(|o| o.id == trigger.id)
            {
                entity.insert(owner.owner);
                if owner.shootable {
                    entity.insert(super::support::ShootableEnvironment);
                }
            }
            if let Some(boundary) = super::hazards::boundary(trigger) {
                entity.insert(boundary);
            }
        }
    }
}

pub type TriggerSites<'w, 's> =
    Query<'w, 's, (Entity, &'static MapTrigger, &'static Transform), Without<Player>>;
