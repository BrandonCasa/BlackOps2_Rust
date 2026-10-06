#![forbid(unsafe_code)]

use std::any::TypeId;

use bevy::{
    MinimalPlugins,
    app::{App, ScheduleRunnerPlugin},
    asset::{
        AssetApp, AssetPlugin, AssetServer, Assets, Handle, RenderAssetUsages,
        io::{AssetSourceBuilder, AssetSourceId, memory::MemoryAssetReader},
    },
    ecs::reflect::AppTypeRegistry,
    mesh::{Indices, Mesh, MeshPlugin, PrimitiveTopology, VertexAttributeValues},
    prelude::PluginGroup,
};
use bo2_assets::geometry_snapshot::{GeometrySnapshot, SnapshotSurface, read_geometry_snapshot};
use bo2_rendering::diagnostic_mesh;

#[derive(Clone, Copy)]
struct Surface {
    first: u64,
    count: u64,
    category: u32,
    declared: u32,
    bounds: [[f32; 3]; 2],
}

// Arbitrary synthetic v1 data only. Parser rejection coverage belongs to assets.
fn snapshot(positions: &[[f32; 3]], indices: &[u32], surfaces: &[Surface]) -> GeometrySnapshot {
    let length = 64 + 12 * positions.len() + 4 * indices.len() + 48 * surfaces.len();
    let mut bytes = Vec::with_capacity(length);
    bytes.extend_from_slice(b"BO2DGS\0\0");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    for value in [positions.len(), indices.len(), surfaces.len(), length] {
        bytes.extend_from_slice(&(value as u64).to_le_bytes());
    }
    bytes.extend_from_slice(&[0; 16]);
    for position in positions {
        for component in position {
            bytes.extend_from_slice(&component.to_bits().to_le_bytes());
        }
    }
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    for surface in surfaces {
        bytes.extend_from_slice(&surface.first.to_le_bytes());
        bytes.extend_from_slice(&surface.count.to_le_bytes());
        bytes.extend_from_slice(&surface.category.to_le_bytes());
        bytes.extend_from_slice(&surface.declared.to_le_bytes());
        for endpoint in surface.bounds {
            for component in endpoint {
                bytes.extend_from_slice(&component.to_bits().to_le_bytes());
            }
        }
    }
    read_geometry_snapshot(&mut bytes.as_slice(), length as u64).unwrap()
}

fn mesh_positions(mesh: &Mesh) -> &Vec<[f32; 3]> {
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(values)) => values,
        _ => panic!("expected exactly Float32x3 diagnostic positions"),
    }
}

fn mesh_indices(mesh: &Mesh) -> &Vec<u32> {
    match mesh.indices() {
        Some(Indices::U32(values)) => values,
        _ => panic!("expected explicit U32 diagnostic indices"),
    }
}

fn assert_mesh(mesh: &Mesh, positions: &[[f32; 3]], indices: &[u32]) {
    assert_eq!(mesh.primitive_topology(), PrimitiveTopology::TriangleList);
    assert_eq!(mesh.asset_usage, RenderAssetUsages::MAIN_WORLD);
    assert_eq!(mesh.attributes().count(), 1);
    assert_eq!(mesh.final_aabb, None);
    assert_eq!(mesh.count_vertices(), positions.len());
    let actual = mesh_positions(mesh);
    assert_eq!(actual.len(), positions.len());
    for (actual, expected) in actual.iter().zip(positions) {
        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
    }
    assert_eq!(mesh_indices(mesh), indices);
}

fn assert_surfaces(actual: &[SnapshotSurface], expected: &[Surface]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(
            actual.index_range(),
            expected.first as usize..(expected.first + expected.count) as usize
        );
        assert_eq!(actual.triangle_count(), expected.count as usize / 3);
        assert_eq!(actual.source_format_category(), expected.category);
        assert_eq!(actual.source_declared_vertex_count(), expected.declared);
        for (actual, expected) in actual.bounds().iter().zip(expected.bounds) {
            assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));
        }
    }
}

fn headless_mesh_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(|| Box::new(MemoryAssetReader::default())),
    );
    app.add_plugins((AssetPlugin::default(), MeshPlugin));
    app.finish();
    app.cleanup();
    app
}

fn example() -> (Vec<[f32; 3]>, Vec<u32>, Vec<Surface>) {
    let positions = vec![
        [-0.0, f32::from_bits(1), f32::from_bits(0x8000_0001)],
        [f32::MAX, -f32::MAX, 0.0],
        [-0.0, f32::from_bits(1), f32::from_bits(0x8000_0001)],
        [7.0, 8.0, 9.0], // Unreferenced, retained; never centered or welded.
    ];
    let bounds = [[-f32::MAX; 3], [f32::MAX; 3]];
    let empty_bounds = [
        [-0.0, -2.0, f32::from_bits(1)],
        [0.0, 3.0, f32::from_bits(1)],
    ];
    let indices = vec![2, 0, 2, 1, 1, 0]; // Ordered repeated/degenerate triangles.
    let surfaces = vec![
        Surface {
            first: 0,
            count: 0,
            category: 8,
            declared: u32::MAX,
            bounds: empty_bounds,
        },
        Surface {
            first: 0,
            count: 3,
            category: 6,
            declared: 0,
            bounds,
        },
        Surface {
            first: 3,
            count: 0,
            category: u32::MAX,
            declared: 0,
            bounds: empty_bounds,
        },
        Surface {
            first: 3,
            count: 3,
            category: 8,
            declared: 1,
            bounds,
        },
        Surface {
            first: 6,
            count: 0,
            category: 6,
            declared: 0,
            bounds: empty_bounds,
        },
    ];
    (positions, indices, surfaces)
}

#[test]
fn preserves_all_position_bits_references_and_ordered_opaque_surfaces() {
    let (positions, indices, expected) = example();
    let (mesh, surfaces) = diagnostic_mesh(snapshot(&positions, &indices, &expected));
    assert_mesh(&mesh, &positions, &indices);
    assert_surfaces(&surfaces, &expected);
}

#[test]
fn transfers_the_three_existing_vector_allocations() {
    let (positions, indices, expected) = example();
    let input = snapshot(&positions, &indices, &expected);
    let positions_pointer = input.positions().as_ptr();
    let indices_pointer = input.indices().as_ptr();
    let surfaces_pointer = input.surfaces().as_ptr();
    let (mesh, surfaces) = diagnostic_mesh(input);
    assert_eq!(mesh_positions(&mesh).as_ptr(), positions_pointer);
    assert_eq!(mesh_indices(&mesh).as_ptr(), indices_pointer);
    assert_eq!(surfaces.as_ptr(), surfaces_pointer);
}

#[test]
fn retains_empty_arrays_unreferenced_positions_and_empty_metadata() {
    let empty = Surface {
        first: 0,
        count: 0,
        category: u32::MAX,
        declared: 0,
        bounds: [[-0.0; 3], [f32::from_bits(1); 3]],
    };
    for (positions, expected) in [
        (vec![], vec![]),
        (vec![], vec![empty]),
        (vec![[f32::MAX, -0.0, -f32::MAX]], vec![]),
        (vec![[f32::MAX, -0.0, -f32::MAX]], vec![empty, empty]),
    ] {
        let (mesh, surfaces) = diagnostic_mesh(snapshot(&positions, &[], &expected));
        assert_mesh(&mesh, &positions, &[]);
        assert_surfaces(&surfaces, &expected);
    }
}

#[test]
fn never_narrows_wide_indices() {
    let positions = vec![[0.0; 3]; 65_537];
    let indices = [65_536, 0, 65_536];
    let expected = [Surface {
        first: 0,
        count: 3,
        category: 6,
        declared: 0,
        bounds: [[0.0; 3]; 2],
    }];
    let (mesh, surfaces) = diagnostic_mesh(snapshot(&positions, &indices, &expected));
    assert_mesh(&mesh, &positions, &indices);
    assert_surfaces(&surfaces, &expected);
}

#[test]
fn bevy_app_registers_owns_returns_and_removes_the_complete_mesh() {
    let (positions, indices, expected) = example();
    let (mesh, surfaces) = diagnostic_mesh(snapshot(&positions, &indices, &expected));
    let positions_pointer = mesh_positions(&mesh).as_ptr();
    let indices_pointer = mesh_indices(&mesh).as_ptr();
    let mut app = headless_mesh_app();
    assert!(app.is_plugin_added::<AssetPlugin>());
    assert!(app.is_plugin_added::<MeshPlugin>());
    assert!(app.world().contains_resource::<AssetServer>());
    let registry = app.world().resource::<AppTypeRegistry>().read();
    assert!(registry.get(TypeId::of::<Mesh>()).is_some());
    assert!(registry.get(TypeId::of::<Handle<Mesh>>()).is_some());
    drop(registry);
    let handle: Handle<Mesh> = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    app.update();
    let stored = app.world().resource::<Assets<Mesh>>().get(&handle).unwrap();
    assert_mesh(stored, &positions, &indices);
    assert_eq!(mesh_positions(stored).as_ptr(), positions_pointer);
    assert_eq!(mesh_indices(stored).as_ptr(), indices_pointer);
    assert_surfaces(&surfaces, &expected);
    let removed = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .remove(&handle)
        .unwrap();
    assert_mesh(&removed, &positions, &indices);
    assert_eq!(mesh_positions(&removed).as_ptr(), positions_pointer);
    assert_eq!(mesh_indices(&removed).as_ptr(), indices_pointer);
    assert!(
        app.world()
            .resource::<Assets<Mesh>>()
            .get(&handle)
            .is_none()
    );
    app.update();
    assert_surfaces(&surfaces, &expected);
}

#[test]
fn engine_strong_handle_lifecycle_releases_cpu_asset_storage() {
    let (mesh, surfaces) = diagnostic_mesh(snapshot(&[], &[], &[]));
    let mut app = headless_mesh_app();
    let handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
    let id = handle.id();
    app.update();
    assert!(app.world().resource::<Assets<Mesh>>().contains(id));
    drop(handle);
    app.update();
    assert!(!app.world().resource::<Assets<Mesh>>().contains(id));
    assert!(surfaces.is_empty());
}

#[test]
fn canonical_mesh_rotates_positions_once_and_retains_source_metadata_and_winding() {
    let (positions, indices, expected) = example();
    let converted = positions
        .iter()
        .copied()
        .map(bo2_rendering::coordinates::source_to_bevy)
        .collect::<Vec<_>>();
    let input = snapshot(&positions, &indices, &expected);
    let position_pointer = input.positions().as_ptr();
    let (mesh, surfaces) = bo2_rendering::canonical_mesh(input);
    assert_mesh(&mesh, &converted, &indices);
    assert_eq!(mesh_positions(&mesh).as_ptr(), position_pointer);
    assert_surfaces(&surfaces, &expected);
}
