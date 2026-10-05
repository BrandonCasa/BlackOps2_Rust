use bo2_assets::appearance::*;
#[test]
fn uv_corner_order_and_seams_are_not_collapsed() {
    let bytes: [u8; 24] = [
        0f32.to_le_bytes(),
        1f32.to_le_bytes(),
        2f32.to_le_bytes(),
        3f32.to_le_bytes(),
        4f32.to_le_bytes(),
        5f32.to_le_bytes(),
    ]
    .concat()
    .try_into()
    .unwrap();
    assert_eq!(
        decode_corner_uv(&bytes, 3).unwrap(),
        vec![[0., 1.], [2., 3.], [4., 5.]]
    );
    assert_eq!(
        decode_corner_uv(&bytes, 2).unwrap_err(),
        AppearanceError::Attributes
    );
    let bad = [f32::NAN.to_le_bytes(), 0f32.to_le_bytes()].concat();
    assert_eq!(
        decode_corner_uv(&bad, 1).unwrap_err(),
        AppearanceError::Attributes
    );
}
#[test]
fn diagnostic_semantics_are_explicit_and_unsupported_is_rejected() {
    let mut m = AppearanceManifest {
        version: 1,
        geometry: "geometry.bo2dgs".into(),
        geometry_sha256: "0".repeat(64),
        surface: 0,
        corner_uv: "uv.bin".into(),
        uv_encoding: "f32-le-corner-original-order".into(),
        material: ColorPreview {
            texture: "texture.dds".into(),
            color_space: "linear-diagnostic".into(),
            sampler: 20,
            family: "diagnostic-unlit-color".into(),
        },
    };
    assert!(validate_manifest(&m).is_ok());
    m.material.color_space = "unknown-original-gamma".into();
    assert_eq!(validate_manifest(&m), Err(AppearanceError::Unsupported));
    m.material.color_space = "linear-diagnostic".into();
    m.material.sampler = 5;
    assert_eq!(validate_manifest(&m), Err(AppearanceError::Unsupported));
}
#[test]
fn references_never_escape_the_bundle() {
    for path in [
        "../texture.dds",
        "/texture.dds",
        "C:/texture.dds",
        "a\\b.dds",
        "a/../../x",
        "",
    ] {
        assert_eq!(
            relative_dependency(std::path::Path::new("."), path),
            Err(AppearanceError::Reference)
        );
    }
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
fn bundle_admits_geometry_binding_and_rejects_missing_identity_and_versions() {
    use sha2::{Digest, Sha256};
    use std::fs;
    let root = std::env::temp_dir().join(format!("bo2-appearance-fixture-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let geometry = synthetic_geometry();
    assert_eq!(geometry.len(), 172);
    fs::write(root.join("geometry.bo2dgs"), &geometry).unwrap();
    fs::write(root.join("uv.bin"), vec![0u8; 48]).unwrap();
    let mut dds = vec![0u8; 128];
    dds[..4].copy_from_slice(b"DDS ");
    for (o, v) in [(4, 124u32), (76, 32), (12, 4), (16, 4)] {
        dds[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fs::write(root.join("texture.dds"), dds).unwrap();
    let mut m = AppearanceManifest {
        version: 1,
        geometry: "geometry.bo2dgs".into(),
        geometry_sha256: format!("{:x}", Sha256::digest(&geometry)),
        surface: 0,
        corner_uv: "uv.bin".into(),
        uv_encoding: "f32-le-corner-original-order".into(),
        material: ColorPreview {
            texture: "texture.dds".into(),
            color_space: "linear-diagnostic".into(),
            sampler: 20,
            family: "diagnostic-unlit-color".into(),
        },
    };
    let path = root.join("scene.json");
    let save = |m: &AppearanceManifest| fs::write(&path, serde_json::to_vec(m).unwrap()).unwrap();
    save(&m);
    let admitted = load_material_scene(&path).unwrap();
    assert_eq!(admitted.corner_uv.len(), 6);
    assert_eq!(admitted.geometry.indices(), &[0, 1, 2, 0, 2, 1]);
    m.geometry_sha256 = "0".repeat(64);
    save(&m);
    assert!(matches!(
        load_material_scene(&path),
        Err(AppearanceError::Identity)
    ));
    m.geometry_sha256 = format!("{:x}", Sha256::digest(&geometry));
    m.version = 2;
    save(&m);
    assert!(matches!(
        load_material_scene(&path),
        Err(AppearanceError::Unsupported)
    ));
    m.version = 1;
    m.material.texture = "missing.dds".into();
    save(&m);
    assert!(matches!(
        load_material_scene(&path),
        Err(AppearanceError::Io(std::io::ErrorKind::NotFound))
    ));
    fs::write(&path, b"{bad}").unwrap();
    assert!(matches!(
        load_material_scene(&path),
        Err(AppearanceError::Malformed)
    ));
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn malformed_oversized_and_non_2d_dds_reject_before_engine_decode() {
    assert_eq!(validate_dds(b"DDS "), Err(AppearanceError::Malformed));
    let mut b = vec![0u8; 128];
    b[..4].copy_from_slice(b"DDS ");
    for (o, v) in [(4, 124u32), (76, 32), (12, 4), (16, 4)] {
        b[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    assert!(validate_dds(&b).is_ok());
    b[16..20].copy_from_slice(&8192u32.to_le_bytes());
    assert_eq!(validate_dds(&b), Err(AppearanceError::Limit));
    b[16..20].copy_from_slice(&4u32.to_le_bytes());
    b[112..116].copy_from_slice(&0x200u32.to_le_bytes());
    assert_eq!(validate_dds(&b), Err(AppearanceError::Unsupported));
}
