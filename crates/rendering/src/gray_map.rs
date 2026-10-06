//! Reusable accepted gray geometry display; all colors remain diagnostic policy.
use bevy::{asset::RenderAssetUsages, mesh::Mesh};
use bo2_assets::geometry_snapshot::{GeometrySnapshot, SnapshotSurface};

pub struct GrayMap {
    pub mesh: Mesh,
    /// Original source-space metadata, preserved in order.
    pub surfaces: Vec<SnapshotSurface>,
}

pub fn prepare_gray_map(snapshot: GeometrySnapshot) -> GrayMap {
    let (mut mesh, surfaces) = crate::canonical_mesh(snapshot);
    mesh.asset_usage |= RenderAssetUsages::RENDER_WORLD;
    GrayMap { mesh, surfaces }
}

#[cfg(feature = "client")]
pub struct GrayMapHandles {
    pub entity: bevy::prelude::Entity,
    pub mesh: bevy::prelude::Handle<Mesh>,
}

/// Requires initialized Bevy mesh/material assets and, for overlay, WireframePlugin.
#[cfg(feature = "client")]
pub fn spawn_gray_map(
    world: &mut bevy::prelude::World,
    mesh: Mesh,
    overlay: bool,
) -> GrayMapHandles {
    use bevy::{
        pbr::wireframe::{Wireframe, WireframeColor},
        prelude::*,
    };
    let handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
    let material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.72, 0.72),
            unlit: true,
            cull_mode: None,
            ..default()
        });
    let mut entity = world.spawn((
        Mesh3d(handle.clone()),
        MeshMaterial3d(material),
        Transform::IDENTITY,
    ));
    if overlay {
        entity.insert((
            Wireframe,
            WireframeColor {
                color: Color::srgb(0.12, 0.12, 0.12),
            },
        ));
    }
    GrayMapHandles {
        entity: entity.id(),
        mesh: handle,
    }
}
