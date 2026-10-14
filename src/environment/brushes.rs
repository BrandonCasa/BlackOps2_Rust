use super::VolumeDefinition;
use crate::player::Player;
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const BRUSH_LAYER: u32 = 16;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriangleShape {
    pub id: String,
    pub contents: u32,
    pub surface_flags: u32,
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<[u32; 3]>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrushDefinition {
    pub id: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub bounds: [[f32; 3]; 2],
    pub source: BTreeMap<String, String>,
    pub volumes: Vec<VolumeDefinition>,
    #[serde(default)]
    pub triangle_shapes: Vec<TriangleShape>,
    pub visual_meshes: usize,
    #[serde(default)]
    pub initial_state: Option<InitialBrushState>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialBrushState {
    pub visible: bool,
    pub solid: bool,
    pub removed: bool,
}
impl BrushDefinition {
    pub fn collider(&self) -> Option<Collider> {
        compound(&self.volumes, &self.triangle_shapes)
    }
    pub fn transform(&self) -> Transform {
        Transform::from_translation(Vec3::from_array(self.position))
            .with_rotation(Quat::from_array(self.rotation))
    }

    fn solid_collider(&self) -> Option<Collider> {
        let volumes = self
            .volumes
            .iter()
            .filter(|volume| volume.contents & 0x10001 != 0)
            .cloned()
            .collect::<Vec<_>>();
        let triangles = self
            .triangle_shapes
            .iter()
            .filter(|shape| shape.contents & 0x10001 != 0)
            .cloned()
            .collect::<Vec<_>>();
        compound(&volumes, &triangles)
    }
}

pub(crate) fn compound(
    volumes: &[VolumeDefinition],
    triangles: &[TriangleShape],
) -> Option<Collider> {
    let mut pieces: Vec<_> = volumes
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
        .collect::<Option<Vec<_>>>()?;
    for shape in triangles {
        for indices in &shape.indices {
            let a = Vec3::from_array(*shape.vertices.get(indices[0] as usize)?);
            let b = Vec3::from_array(*shape.vertices.get(indices[1] as usize)?);
            let c = Vec3::from_array(*shape.vertices.get(indices[2] as usize)?);
            if !a.is_finite()
                || !b.is_finite()
                || !c.is_finite()
                || (b - a).cross(c - a).length_squared() == 0.
            {
                return None;
            }
            pieces.push((Vec3::ZERO, Quat::IDENTITY, Collider::triangle(a, b, c)));
        }
    }
    (!pieces.is_empty()).then(|| Collider::compound(pieces))
}

pub(crate) fn valid_triangle(shape: &TriangleShape) -> bool {
    identity(&shape.id)
        && (3..=65536).contains(&shape.vertices.len())
        && shape
            .vertices
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        && (1..=65536).contains(&shape.indices.len())
        && shape
            .indices
            .iter()
            .flatten()
            .all(|&index| (index as usize) < shape.vertices.len())
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
                && b.bounds[0].iter().zip(b.bounds[1]).all(|(a, z)| *a <= z)
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
                && b.triangle_shapes.len() <= 4096
                && b.triangle_shapes
                    .iter()
                    .all(|shape| valid_triangle(shape) && pieces.insert(&shape.id))
                && ((b.volumes.is_empty() && b.triangle_shapes.is_empty())
                    || b.collider().is_some())
        })
        && pieces.len() <= 32768
}

#[derive(Component)]
pub struct MapBrush {
    pub id: String,
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
    ownership: Option<&super::support::OwnershipDefinition>,
) {
    if source
        .initial_state
        .as_ref()
        .is_some_and(|state| state.removed)
    {
        commands.entity(entity).despawn();
        return;
    }
    commands.entity(entity).insert(MapBrush {
        id: source.id.clone(),
        visual_meshes,
    });
    if let Some(owner) = ownership {
        commands.entity(entity).insert(owner.owner);
        if owner.shootable {
            commands
                .entity(entity)
                .insert(super::support::ShootableEnvironment);
        }
    }
    if let Some(shape) = source.collider() {
        commands.entity(entity).insert((
            shape,
            Sensor,
            CollisionLayers::new(BRUSH_LAYER, LayerMask::NONE),
        ));
    }
    if source
        .initial_state
        .as_ref()
        .is_some_and(|state| state.solid)
        && let Some(shape) = source.solid_collider()
    {
        commands.spawn((
            ChildOf(entity),
            Transform::IDENTITY,
            RigidBody::Static,
            shape,
            CollisionLayers::new(1, LayerMask::ALL),
        ));
    }
    if source
        .initial_state
        .as_ref()
        .is_some_and(|state| !state.visible)
    {
        commands.entity(entity).insert(Visibility::Hidden);
    }
}
