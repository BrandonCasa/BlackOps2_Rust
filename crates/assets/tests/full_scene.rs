use bo2_assets::{appearance::AppearanceError, full_scene::*};
use sha2::{Digest, Sha256};
fn dds(cube: bool, dx10: bool) -> Vec<u8> {
    let mut b = vec![
        0u8;
        if dx10 {
            164
        } else if cube {
            176
        } else {
            136
        }
    ];
    b[..4].copy_from_slice(b"DDS ");
    for (o, v) in [(4, 124u32), (76, 32), (12, 4), (16, 4), (28, 1)] {
        b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    if cube {
        b[112..116].copy_from_slice(&0xfe00u32.to_le_bytes());
    }
    b[84..88].copy_from_slice(if dx10 { b"DX10" } else { b"DXT1" });
    if dx10 {
        for (o, v) in [(128, 83u32), (132, 3), (140, 1)] {
            b[o..o + 4].copy_from_slice(&v.to_le_bytes());
        }
    }
    b
}
#[test]
fn complete_mips_cube_faces_and_dx10_are_bounded() {
    assert!(inspect_dds(&dds(false, false)).is_ok());
    assert!(inspect_dds(&dds(true, false)).unwrap().cube);
    assert_eq!(
        inspect_dds(&dds(false, true)).unwrap().format,
        DdsFormat::Bc5
    );
    let mut b = dds(true, false);
    b.pop();
    assert_eq!(inspect_dds(&b), Err(AppearanceError::Malformed));
    let mut b = dds(true, false);
    b[16..20].copy_from_slice(&8u32.to_le_bytes());
    assert_eq!(inspect_dds(&b), Err(AppearanceError::Malformed));
    let mut b = dds(false, false);
    b[28..32].copy_from_slice(&4u32.to_le_bytes());
    assert_eq!(inspect_dds(&b), Err(AppearanceError::Malformed));
    let mut b = dds(false, true);
    b[140..144].copy_from_slice(&2u32.to_le_bytes());
    assert_eq!(inspect_dds(&b), Err(AppearanceError::Unsupported));
}
fn synthetic_geometry() -> Vec<u8> {
    let mut b = vec![0u8; 64];
    b[..8].copy_from_slice(b"BO2DGS\0\0");
    b[8..12].copy_from_slice(&1u32.to_le_bytes());
    for (offset, value) in [(16, 3u64), (24, 6), (32, 1), (40, 172)] {
        b[offset..offset + 8].copy_from_slice(&value.to_le_bytes())
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
    b
}

#[test]
fn full_accounting_identity_and_explicit_contracts() {
    let root = std::env::temp_dir().join(format!("bo2-full-scene-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let geometry = synthetic_geometry();
    let image = dds(false, false);
    let uv = vec![0u8; 48];
    for (name, bytes) in [("geometry", &geometry), ("image", &image), ("uv", &uv)] {
        std::fs::write(root.join(name), bytes).unwrap();
    }
    let hash = |b: &[u8]| format!("{:x}", Sha256::digest(b));
    let mut m = FullSceneManifest {
        version: 2,
        geometry: "geometry".into(),
        geometry_sha256: hash(&geometry),
        corner_uv: "uv".into(),
        corner_uv_sha256: hash(&uv),
        uv_encoding: "f32-le-corner-original-order".into(),
        corner_vertices: None,
        images: vec![SceneImage {
            file: "image".into(),
            sha256: hash(&image),
        }],
        materials: vec![SceneMaterial {
            source_technique_set: "synthetic".into(),
            contract: "diagnostic-unlit-color".into(),
            bindings: vec![
                SceneBinding {
                    semantic: 2,
                    name_hash: 0,
                    image: 0,
                    sampler: 20,
                },
                SceneBinding {
                    semantic: 5,
                    name_hash: 1,
                    image: 0,
                    sampler: 1,
                },
            ],
        }],
        lightmaps: vec![],
        probes: vec![],
        surfaces: vec![SceneSurface {
            material: 0,
            source_vertex_format: 0,
            lightmap: None,
            probe: None,
        }],
    };
    let path = root.join("scene.json");
    let save =
        |m: &FullSceneManifest| std::fs::write(&path, serde_json::to_vec(m).unwrap()).unwrap();
    save(&m);
    let admitted = load_full_scene(&path).unwrap();
    assert_eq!(admitted.images.len(), 1);
    assert_eq!(admitted.materials[0].bindings.len(), 2);
    assert_eq!(admitted.corner_uv.len(), 6);
    m.materials[0].contract = "original-tech4".into();
    save(&m);
    assert!(matches!(
        load_full_scene(&path),
        Err(AppearanceError::Unsupported)
    ));
    m.materials[0].contract = "diagnostic-unlit-color".into();
    m.surfaces.clear();
    save(&m);
    assert!(matches!(
        load_full_scene(&path),
        Err(AppearanceError::Attributes)
    ));
    m.surfaces.push(SceneSurface {
        material: 0,
        source_vertex_format: 0,
        lightmap: None,
        probe: None,
    });
    m.corner_uv_sha256 = "0".repeat(64);
    save(&m);
    assert!(matches!(
        load_full_scene(&path),
        Err(AppearanceError::Identity)
    ));
    m.corner_uv_sha256 = hash(&uv);
    m.materials[0].bindings[0].image = 1;
    save(&m);
    assert!(matches!(
        load_full_scene(&path),
        Err(AppearanceError::Reference)
    ));
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[ignore = "requires external BO2_FULL_SCENE; never embeds original data"]
fn external_complete_scene_payload_admission() {
    let path = std::env::var("BO2_FULL_SCENE").unwrap();
    let scene = load_full_scene(std::path::Path::new(&path)).unwrap();
    assert_eq!(scene.surfaces.len(), 6469);
    assert_eq!(scene.images.len(), 260);
    assert_eq!(scene.materials.len(), 280);
    assert_eq!(scene.corner_uv.len(), 449721);
    assert_eq!(scene.corner_attributes.as_ref().unwrap().len(), 449721);
}

#[test]
fn source_input_channels_preserve_unorm_and_reject_geometry_mismatch() {
    let g = synthetic_geometry();
    let geometry = bo2_assets::geometry_snapshot::read_geometry_snapshot(
        &mut std::io::Cursor::new(&g),
        g.len() as u64,
    )
    .unwrap();
    let mut records = Vec::new();
    for index in geometry.indices() {
        let mut b = [0u8; 36];
        for (i, f) in geometry.positions()[*index as usize].iter().enumerate() {
            b[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
        b[12..16].copy_from_slice(&(-1f32).to_le_bytes());
        b[16..20].copy_from_slice(&[0, 127, 255, 255]);
        b[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
        b[28..32].copy_from_slice(&1023u32.to_le_bytes());
        b[32..34].copy_from_slice(&65535u16.to_le_bytes());
        records.extend(b);
    }
    let attrs = decode_world_corner_attributes(&records, &geometry).unwrap();
    assert_eq!(attrs[0].normal_unorm, [1.; 4]);
    assert_eq!(attrs[0].tangent_unorm, [1., 0., 0., 0.]);
    assert_eq!(attrs[0].lightmap_uv, [1., 0.]);
    assert_eq!(attrs[0].position_w, -1.);
    assert_eq!(attrs[0].color, [0., 127. / 255., 1., 1.]);
    records[0..4].copy_from_slice(&1f32.to_le_bytes());
    assert_eq!(
        decode_world_corner_attributes(&records, &geometry),
        Err(AppearanceError::Attributes)
    );
}
