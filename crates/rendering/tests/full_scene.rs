use bevy::{
    image::CompressedImageFormats,
    mesh::{Mesh, VertexAttributeValues},
};
use bo2_assets::{
    appearance::AppearanceError, full_scene::*, geometry_snapshot::read_geometry_snapshot,
};
use bo2_rendering::full_scene::diagnostic_texture_scene;

fn input() -> FullSceneInput {
    let mut bytes = vec![0; 64];
    bytes[..8].copy_from_slice(b"BO2DGS\0\0");
    bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
    for (offset, value) in [(16, 3u64), (24, 9), (32, 3), (40, 280)] {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
    for p in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
        for c in p {
            bytes.extend(c.to_le_bytes());
        }
    }
    for i in [0u32, 1, 2, 0, 2, 1, 0, 1, 2] {
        bytes.extend(i.to_le_bytes());
    }
    for start in [0u64, 3, 6] {
        bytes.extend(start.to_le_bytes());
        bytes.extend(3u64.to_le_bytes());
        bytes.extend([0u8; 8]);
        for b in [0.0f32, 0.0, 0.0, 1.0, 1.0, 0.0] {
            bytes.extend(b.to_le_bytes());
        }
    }
    let geometry =
        read_geometry_snapshot(&mut std::io::Cursor::new(&bytes), bytes.len() as u64).unwrap();
    let mut dds = vec![0u8; 136];
    dds[..4].copy_from_slice(b"DDS ");
    for (o, v) in [
        (4, 124u32),
        (8, 0x81007),
        (12, 4),
        (16, 4),
        (20, 8),
        (28, 1),
        (76, 32),
        (80, 4),
        (108, 0x1000),
    ] {
        dds[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    dds[84..88].copy_from_slice(b"DXT1");
    let metadata = inspect_dds(&dds).unwrap();
    let material = |count| SceneMaterial {
        source_technique_set: "synthetic".into(),
        contract: "diagnostic-unlit-color".into(),
        bindings: (0..count)
            .map(|_| SceneBinding {
                semantic: 2,
                name_hash: 0,
                image: 0,
                sampler: 20,
            })
            .collect(),
    };
    FullSceneInput {
        geometry,
        corner_attributes: None,
        lightmaps: vec![],
        probes: vec![],
        corner_uv: vec![[0.25, 0.75]; 9],
        images: vec![SceneImageInput { dds, metadata }],
        materials: vec![material(2), material(1), material(0)],
        surfaces: (0..3)
            .map(|material| SceneSurface {
                material,
                source_vertex_format: 0,
                lightmap: None,
                probe: None,
            })
            .collect(),
    }
}
#[test]
fn original_mesh_keeps_native_inputs_and_canonical_engine_bounds_separate() {
    use bo2_rendering::original_material::{SourceAttribute, original_world_mesh};
    let mut source = input();
    source.corner_attributes = Some(vec![
        SourceCornerAttributes {
            position_w: 0.5,
            color: [0.25; 4],
            normal_unorm: [0.5, 0.25, 0.75, 1.0],
            tangent_unorm: [0.0, 0.5, 1.0, 0.0],
            lightmap_uv: [0.125, 0.875],
        };
        9
    ]);
    let mesh = original_world_mesh(&source, 0).unwrap();
    assert!(
        matches!(mesh.attribute(Mesh::ATTRIBUTE_POSITION), Some(VertexAttributeValues::Float32x3(p)) if p[2] == [0.0,0.0,-1.0])
    );
    assert!(
        matches!(mesh.attribute(SourceAttribute::Position4.attribute()), Some(VertexAttributeValues::Float32x4(p)) if p[2] == [0.0,1.0,0.0,0.5])
    );
    assert!(
        matches!(mesh.attribute(SourceAttribute::Normal4.attribute()), Some(VertexAttributeValues::Float32x4(n)) if n[0] == [0.5,0.25,0.75,1.0])
    );
    assert_eq!(source.geometry.positions()[2], [0.0, 1.0, 0.0]);
    source.corner_attributes.as_mut().unwrap()[0].normal_unorm[0] = f32::NAN;
    assert!(original_world_mesh(&source, 0).is_err());
}
#[test]
fn shared_texture_geometry_layers_and_missing_color_are_accounted_explicitly() {
    let scene = diagnostic_texture_scene(input(), CompressedImageFormats::BC).unwrap();
    assert_eq!(scene.images.len(), 1);
    assert_eq!(scene.meshes.len(), 3);
    assert_eq!(scene.source_surface_count, 3);
    assert_eq!(scene.multi_color_surfaces, 1);
    assert_eq!(scene.missing_color_surfaces, 1);
    assert_eq!(scene.materials[0].image, scene.materials[1].image);
    assert_eq!(scene.materials[0].selected_binding, Some(0));
    assert_eq!(scene.materials[2].image, None);
    for (index, part) in scene.meshes.iter().enumerate() {
        assert_eq!(part.source_surfaces, vec![index]);
        assert!(
            matches!(part.mesh.attribute(Mesh::ATTRIBUTE_POSITION),Some(VertexAttributeValues::Float32x3(p)) if p.iter().any(|p| p==&[0.0,0.0,-1.0]))
        );
        assert!(
            matches!(part.mesh.attribute(Mesh::ATTRIBUTE_UV_0),Some(VertexAttributeValues::Float32x2(uv)) if uv==&vec![[0.25,0.75];3])
        );
    }
}
#[test]
fn original_contract_and_bad_references_do_not_silently_become_preview_materials() {
    let mut original = input();
    original.materials[0].contract = "original-tech4".into();
    assert!(matches!(
        diagnostic_texture_scene(original, CompressedImageFormats::BC),
        Err(AppearanceError::Unsupported)
    ));
    let mut bad = input();
    bad.materials[0].bindings[0].image = 999;
    assert!(matches!(
        diagnostic_texture_scene(bad, CompressedImageFormats::BC),
        Err(AppearanceError::Reference)
    ));
}
