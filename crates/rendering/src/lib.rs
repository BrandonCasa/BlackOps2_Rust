#![forbid(unsafe_code)]

//! Diagnostic geometry integration with Bevy; original rendering is unestablished.
pub mod captured_world;
pub mod native_camera;

use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
};
use bo2_assets::geometry_snapshot::{GeometrySnapshot, SnapshotSurface};

/// Transfer a validated diagnostic snapshot into CPU-side Bevy mesh storage.
///
/// Positions become `ATTRIBUTE_POSITION` (`Float32x3`) and indices remain `U32`
/// under `TriangleList`. All three owned vectors are moved, without copying,
/// filtering, coordinate conversion or attribute generation. Ordered surfaces
/// and their exact opaque metadata/bounds are returned separately to the caller;
/// they do not define Bevy materials or draw commands.
///
/// `MAIN_WORLD` deliberately retains diagnostic CPU access and does not request
/// render-world extraction. No engine plugin, asset identity, GPU upload or
/// original renderability is established here. Bevy allocates its attribute map
/// bookkeeping; later asset insertion allocates engine storage/handle metadata.
/// This function adds no fallible allocation policy or process-memory guarantee.
pub fn diagnostic_mesh(snapshot: GeometrySnapshot) -> (Mesh, Vec<SnapshotSurface>) {
    let (positions, indices, surfaces) = snapshot.into_parts();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD,
    );
    // The fixed attribute and explicit value variant have identical formats;
    // this new mesh has never been extracted, satisfying Bevy's insertion rules.
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(positions),
    );
    mesh.insert_indices(Indices::U32(indices));
    (mesh, surfaces)
}

pub mod appearance;
pub mod gray_map;

/// Shared original-data to canonical Bevy runtime basis.
pub use bo2_core::coordinates;

/// Adapt validated source geometry to canonical Bevy +Y-up runtime positions.
/// Indices and UV conventions are unchanged. Returned surface bounds remain in
/// source space; use `coordinates::source_bounds_to_bevy` for runtime bounds.
/// The original-data reader and `diagnostic_mesh` retain their raw contracts.
pub fn canonical_mesh(snapshot: GeometrySnapshot) -> (Mesh, Vec<SnapshotSurface>) {
    let (mut mesh, surfaces) = diagnostic_mesh(snapshot);
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for position in positions {
            *position = coordinates::source_to_bevy(*position);
        }
    }
    (mesh, surfaces)
}

pub mod tech4;

pub mod full_scene;

pub mod original_material;
pub mod pass_resources;
pub mod vertex_routes;
