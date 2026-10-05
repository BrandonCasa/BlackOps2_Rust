#![forbid(unsafe_code)]
use bevy::{
    asset::RenderAssetUsages,
    camera::CameraProjection,
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
};
use bo2_geometry_viewer::{
    DiagnosticError, IMAGE_BYTES, foreground_pixels, prepare, synthetic_snapshot, views,
};

#[test]
fn chosen_camera_frusta_contain_every_asymmetric_fixture_position() {
    let snapshot = synthetic_snapshot().unwrap();
    for mut view in views(snapshot.positions()).unwrap() {
        view.projection.update(512.0, 512.0);
        let clip_from_world =
            view.projection.get_clip_from_view() * view.transform.to_matrix().inverse();
        for point in snapshot.positions() {
            let clip = clip_from_world.project_point3(Vec3::from_array(*point));
            assert!(clip.is_finite());
            assert!(clip.x.abs() < 0.81 && clip.y.abs() < 0.81);
            assert!(clip.z > 0.0 && clip.z < 1.0);
        }
    }
}
#[test]
fn empty_extreme_degenerate_and_poorly_resolved_camera_inputs_reject() {
    assert!(matches!(views(&[]), Err(DiagnosticError::EmptyGeometry)));
    for input in [
        vec![[f32::MAX, 0.0, 0.0]],
        vec![[0.0; 3]; 3],
        vec![[0.0; 3], [1e-8; 3]],
        vec![[1_000_000.0, 0.0, 0.0], [1_000_000.0 + 0.125, 0.0, 0.0]],
        vec![[f32::NAN, 0.0, 0.0]],
    ] {
        assert!(matches!(
            views(&input),
            Err(DiagnosticError::UnframeableGeometry)
        ));
    }
}
#[test]
fn render_opt_in_preserves_exact_arrays_metadata_and_allocations() {
    let snapshot = synthetic_snapshot().unwrap();
    let expected_positions = snapshot.positions().to_vec();
    let expected_indices = snapshot.indices().to_vec();
    let position_pointer = snapshot.positions().as_ptr();
    let index_pointer = snapshot.indices().as_ptr();
    let surface_pointer = snapshot.surfaces().as_ptr();
    let geometry = prepare(snapshot).unwrap();
    assert_eq!(
        geometry.mesh.asset_usage,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    );
    assert_eq!(geometry.mesh.attributes().count(), 1);
    let Some(VertexAttributeValues::Float32x3(positions)) =
        geometry.mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("position-only");
    };
    assert_eq!(positions.as_ptr(), position_pointer);
    assert_eq!(
        positions
            .iter()
            .map(|v| v.map(f32::to_bits))
            .collect::<Vec<_>>(),
        expected_positions
            .iter()
            .map(|v| v.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    let Some(Indices::U32(indices)) = geometry.mesh.indices() else {
        panic!("u32 indices");
    };
    assert_eq!(indices.as_ptr(), index_pointer);
    assert_eq!(indices, &expected_indices);
    assert_eq!(geometry.surfaces.as_ptr(), surface_pointer);
    assert_eq!(geometry.surfaces[0].index_range(), 0..12);
    assert_eq!(geometry.surfaces[0].source_format_category(), u32::MAX);
    assert_eq!(geometry.surfaces[0].source_declared_vertex_count(), 0);
    assert_eq!(
        geometry.surfaces[0].bounds(),
        &[[-2.0, -1.0, 0.0], [3.0, 2.0, 4.0]]
    );
    assert_eq!(geometry.mesh.final_aabb, None);
}
#[test]
fn coverage_rejects_missing_rows_and_background_only_readback() {
    assert_eq!(foreground_pixels(&[]), Err(DiagnosticError::ReadbackLayout));
    let mut pixels = [0, 0, 0, 255].repeat(IMAGE_BYTES / 4);
    assert_eq!(
        foreground_pixels(&pixels),
        Err(DiagnosticError::BackgroundOnly)
    );
    let offset = (32 * 512 + 32) * 4;
    pixels[offset..offset + 4].copy_from_slice(&[128, 128, 128, 255]);
    assert_eq!(foreground_pixels(&pixels), Ok(1));
}
