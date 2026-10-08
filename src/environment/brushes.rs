//! Authored brush roots retain native data independently of their gameplay consumers.
use super::VolumeDefinition;
use crate::player::Player;
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const BRUSH_LAYER: u32 = 16;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrushDefinition {
    pub id: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub bounds: [[f32; 3]; 2],
    pub source: BTreeMap<String, String>,
    pub volumes: Vec<VolumeDefinition>,
    pub visual_meshes: usize,
}
impl BrushDefinition {
    pub fn collider(&self) -> Option<Collider> {
        let pieces: Option<Vec<_>> = self
            .volumes
            .iter()
            .map(|v| {
                Collider::convex_hull(v.vertices.iter().copied().map(Vec3::from_array).collect())
                    .map(|shape| (Vec3::from_array(v.position), Quat::IDENTITY, shape))
            })
            .collect();
        pieces.filter(|p| !p.is_empty()).map(Collider::compound)
    }
    pub fn transform(&self) -> Transform {
        Transform::from_translation(Vec3::from_array(self.position))
            .with_rotation(Quat::from_array(self.rotation))
    }
}

fn identity(id: &str) -> bool {
    !id.is_empty()
        && id.len() < 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub(super) fn valid(brushes: &[BrushDefinition]) -> bool {
    let mut ids = BTreeSet::new();
    let mut pieces = BTreeSet::new();
    brushes.len() <= 4096
        && brushes.iter().all(|b| {
            identity(&b.id)
                && ids.insert(&b.id)
                && b.position
                    .iter()
                    .chain(b.rotation.iter())
                    .chain(b.bounds.iter().flatten())
                    .all(|v| v.is_finite())
                && (Quat::from_array(b.rotation).length_squared() - 1.).abs() < 0.00001
                && b.bounds[0].iter().zip(b.bounds[1]).all(|(a, z)| *a < z)
                && b.visual_meshes <= 4096
                && (1..=64).contains(&b.source.len())
                && b.source.iter().all(|(k, v)| {
                    !k.is_empty()
                        && k.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                        && v.len() <= 4096
                })
                && b.source
                    .get("classname")
                    .is_some_and(|c| matches!(c.as_str(), "script_brushmodel" | "heli_height_lock"))
                && b.source.get("model").is_some_and(|m| {
                    m.strip_prefix('*')
                        .is_some_and(|n| n.parse::<u32>().is_ok_and(|n| n > 0))
                })
                && b.volumes.len() <= 4096
                && b.volumes.iter().all(|v| {
                    identity(&v.id)
                        && pieces.insert(&v.id)
                        && v.position
                            .iter()
                            .chain(v.vertices.iter().flatten())
                            .all(|p| p.is_finite())
                        && (4..=128).contains(&v.vertices.len())
                })
                && (b.volumes.is_empty() || b.collider().is_some())
        })
        && pieces.len() <= 32768
}

#[derive(Component)]
pub struct MapBrush {
    pub id: String,
    /// Counted from the loaded Bevy hierarchy when its scene becomes ready.
    pub visual_meshes: usize,
}
pub type BrushSites<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MapBrush,
        &'static GlobalTransform,
        Option<&'static Collider>,
    ),
    Without<Player>,
>;

pub(crate) fn bind(
    commands: &mut Commands,
    entity: Entity,
    source: &BrushDefinition,
    visual_meshes: usize,
) {
    commands.entity(entity).insert(MapBrush {
        id: source.id.clone(),
        visual_meshes,
    });
    if let Some(shape) = source.collider() {
        // Preparation retains geometry; solid/hidden/moving rules belong to its source-script consumer.
        // Existing doors keep their session-owned blockers rather than acquiring a second permanent one.
        commands.entity(entity).insert((
            shape,
            Sensor,
            CollisionLayers::new(BRUSH_LAYER, LayerMask::NONE),
        ));
    }
}
