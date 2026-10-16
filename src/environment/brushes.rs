use super::VolumeDefinition;
use crate::player::Player;
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub const BRUSH_LAYER: u32 = 16;

#[derive(Clone, Deserialize)]
pub struct TriangleShape {
    pub id: String,
    pub contents: u32,
    pub surface_flags: u32,
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<[u32; 3]>,
}

#[derive(Clone, Deserialize)]
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

    fn solid_colliders(&self) -> Vec<(u32, Collider)> {
        let contents: BTreeSet<_> = self
            .volumes
            .iter()
            .map(|volume| volume.contents)
            .chain(self.triangle_shapes.iter().map(|shape| shape.contents))
            .filter(|contents| contents & 0x30001 != 0)
            .collect();
        contents
            .into_iter()
            .filter_map(|contents| {
                let volumes = self
                    .volumes
                    .iter()
                    .filter(|volume| volume.contents == contents)
                    .cloned()
                    .collect::<Vec<_>>();
                let triangles = self
                    .triangle_shapes
                    .iter()
                    .filter(|shape| shape.contents == contents)
                    .cloned()
                    .collect::<Vec<_>>();
                compound(&volumes, &triangles).map(|collider| (contents, collider))
            })
            .collect()
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
    {
        for (contents, shape) in source.solid_colliders() {
            commands.spawn((
                ChildOf(entity),
                Transform::IDENTITY,
                RigidBody::Static,
                shape,
                crate::assets::WorldContents(contents),
                crate::assets::world_collision_layers(contents),
            ));
        }
    }
    if source
        .initial_state
        .as_ref()
        .is_some_and(|state| !state.visible)
    {
        commands.entity(entity).insert(Visibility::Hidden);
    }
}
