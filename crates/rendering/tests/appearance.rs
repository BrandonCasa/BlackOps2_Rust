use bevy::image::{ImageAddressMode, ImageFilterMode};
use bo2_rendering::appearance::source_sampler;
#[test]
fn observed_sampler_keeps_repeat_and_mip_anisotropy() {
    let s = source_sampler(20).unwrap();
    assert_eq!(s.address_mode_u, ImageAddressMode::Repeat);
    assert_eq!(s.address_mode_v, ImageAddressMode::Repeat);
    assert_eq!(s.anisotropy_clamp, 4);
    assert_eq!(s.mipmap_filter, ImageFilterMode::Linear);
    let clamp = source_sampler(20 | 32 | 64).unwrap();
    assert_eq!(clamp.address_mode_u, ImageAddressMode::ClampToEdge);
    assert_eq!(clamp.address_mode_v, ImageAddressMode::ClampToEdge);
    assert!(source_sampler(5).is_err());
}
#[test]
fn repeated_source_vertices_keep_corner_seams_and_linear_dds() {
    use bevy::{
        image::CompressedImageFormats,
        mesh::{Indices, Mesh, VertexAttributeValues},
    };
    use bo2_assets::{appearance::MaterialSceneInput, geometry_snapshot::read_geometry_snapshot};
    use bo2_rendering::appearance::material_scene;
    let mut b = vec![0u8; 64];
    b[..8].copy_from_slice(b"BO2DGS\0\0");
    b[8..12].copy_from_slice(&1u32.to_le_bytes());
    for (o, v) in [(16, 3u64), (24, 6), (32, 1), (40, 172)] {
        b[o..o + 8].copy_from_slice(&v.to_le_bytes())
    }
    for p in [[0f32, 0., 0.], [1., 0., 0.], [0., 1., 0.]] {
        for v in p {
            b.extend(v.to_le_bytes());
        }
    }
    for i in [0u32, 1, 2, 0, 2, 1] {
        b.extend(i.to_le_bytes());
    }
    b.extend(0u64.to_le_bytes());
    b.extend(6u64.to_le_bytes());
    b.extend([0u8; 8]);
    for v in [0f32, 0., 0., 1., 1., 0.] {
        b.extend(v.to_le_bytes());
    }
    let geometry = read_geometry_snapshot(&mut std::io::Cursor::new(&b), b.len() as u64).unwrap();
    // User-created one-block BC1 DDS with black color endpoints.
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
    let uv = vec![[0., 0.], [1., 0.], [0., 1.], [4., 5.], [1., 1.], [2., 2.]];
    let scene = material_scene(
        MaterialSceneInput {
            geometry,
            surface: 0,
            corner_uv: uv.clone(),
            texture_dds: dds,
            sampler: 20,
        },
        CompressedImageFormats::BC,
    )
    .unwrap();
    assert_eq!(scene.source_indices, vec![0, 1, 2, 0, 2, 1]);
    assert!(
        matches!(scene.mesh.attribute(Mesh::ATTRIBUTE_UV_0),Some(VertexAttributeValues::Float32x2(v)) if v==&uv)
    );
    assert!(
        matches!(scene.mesh.attribute(Mesh::ATTRIBUTE_POSITION),Some(VertexAttributeValues::Float32x3(v)) if v[0]==v[3])
    );
    assert!(matches!(scene.mesh.indices(),Some(Indices::U32(v)) if v==&vec![0,1,2,3,4,5]));
    assert!(!scene.image.texture_descriptor.format.is_srgb());
    assert_eq!(
        scene.image.texture_descriptor.format.block_dimensions(),
        (4, 4)
    );
}
