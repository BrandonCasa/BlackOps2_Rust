use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, Mesh, VertexAttributeValues},
};
use bo2_assets::geometry_snapshot::read_geometry_snapshot;
use bo2_rendering::gray_map::prepare_gray_map;

#[test]
fn gray_preparation_preserves_allocations_indices_and_source_bounds() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"BO2DGS\0\0");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    for value in [3_u64, 3, 1, 160] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[0; 16]);
    for position in [[1_f32, 2., 3.], [2., 2., 3.], [1., 4., 3.]] {
        for value in position {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for value in [2_u32, 0, 1] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [0_u64, 3] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [6_u32, 3] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [1_f32, 2., 3., 2., 4., 3.] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let snapshot = read_geometry_snapshot(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
    let positions = snapshot.positions().as_ptr();
    let indices = snapshot.indices().as_ptr();
    let surfaces = snapshot.surfaces().as_ptr();
    let gray = prepare_gray_map(snapshot);
    assert_eq!(
        gray.mesh.asset_usage,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    );
    let Some(VertexAttributeValues::Float32x3(values)) =
        gray.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!()
    };
    assert_eq!(values.as_ptr(), positions);
    assert_eq!(values, &[[1., 3., -2.], [2., 3., -2.], [1., 3., -4.]]);
    let Some(Indices::U32(values)) = gray.mesh.indices() else {
        panic!()
    };
    assert_eq!(values.as_ptr(), indices);
    assert_eq!(values, &[2, 0, 1]);
    assert_eq!(gray.surfaces.as_ptr(), surfaces);
    assert_eq!(gray.surfaces[0].bounds(), &[[1., 2., 3.], [2., 4., 3.]]);
}

#[cfg(feature = "client")]
#[test]
fn shared_gray_display_retains_existing_unlit_overlay_policy() {
    use bevy::{
        pbr::wireframe::{Wireframe, WireframeColor},
        prelude::*,
    };
    let mut world = World::new();
    world.insert_resource(Assets::<Mesh>::default());
    world.insert_resource(Assets::<StandardMaterial>::default());
    let mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handles = bo2_rendering::gray_map::spawn_gray_map(&mut world, mesh, true);
    let entity = world.entity(handles.entity);
    assert_eq!(*entity.get::<Transform>().unwrap(), Transform::IDENTITY);
    assert!(entity.contains::<Wireframe>());
    assert_eq!(
        entity.get::<WireframeColor>().unwrap().color,
        Color::srgb(0.12, 0.12, 0.12)
    );
    assert_eq!(entity.get::<Mesh3d>().unwrap().0, handles.mesh);
    let material_handle = &entity.get::<MeshMaterial3d<StandardMaterial>>().unwrap().0;
    let material = world
        .resource::<Assets<StandardMaterial>>()
        .get(material_handle)
        .unwrap();
    assert!(material.unlit);
    assert_eq!(material.cull_mode, None);
    assert_eq!(material.base_color, Color::srgb(0.72, 0.72, 0.72));
}
