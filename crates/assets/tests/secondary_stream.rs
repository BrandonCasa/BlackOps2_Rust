use bo2_assets::{
    appearance::AppearanceError, full_scene::SceneImage, geometry_snapshot::read_geometry_snapshot,
    secondary_stream::*,
};
use sha2::{Digest, Sha256};
fn geometry(format: u32) -> Vec<u8> {
    let mut b = vec![0u8; 64];
    b[..8].copy_from_slice(b"BO2DGS\0\0");
    b[8..12].copy_from_slice(&1u32.to_le_bytes());
    for (o, v) in [(16, 3u64), (24, 6), (32, 1), (40, 172)] {
        b[o..o + 8].copy_from_slice(&v.to_le_bytes());
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
    b.extend(format.to_le_bytes());
    b.extend(0u32.to_le_bytes());
    for v in [0f32, 0., 0., 1., 1., 0.] {
        b.extend(v.to_le_bytes());
    }
    b
}
fn manifest(format: u32) -> SecondaryManifest {
    let image = || SceneImage {
        file: "x".into(),
        sha256: "0".repeat(64),
    };
    SecondaryManifest {
        version: 1,
        geometry_sha256: "0".repeat(64),
        stream: image(),
        corner_local_indices: image(),
        surfaces: vec![SecondarySurface {
            first_corner: 0,
            corners: 6,
            source_format: format,
            byte_offset: 0,
        }],
    }
}
#[test]
fn exact_layouts_distinguish_seams_color_and_formats() {
    let mut b = vec![0u8; 16];
    b[..2].copy_from_slice(&0x3c00u16.to_le_bytes());
    b[2..4].copy_from_slice(&0xc000u16.to_le_bytes());
    b[8..12].copy_from_slice(&[0, 128, 255, 64]);
    b[12..16].copy_from_slice(&[255, 0, 64, 128]);
    let five = decode_secondary_record(5, &b).unwrap();
    let three = decode_secondary_record(3, &b[..8]).unwrap();
    assert_eq!(three.texcoord3, five.texcoord3);
    assert!(three.color5.is_none() && three.color6.is_none() && three.texcoord4.is_none());
    assert_eq!(
        five.texcoord3,
        Some(SourceTexcoord::Float4([1., -2., 0., 0.]))
    );
    assert_eq!(five.color6.unwrap()[0], 1.);
    assert!(five.texcoord4.is_none());
    b[8..12].fill(0);
    let seven = decode_secondary_record(7, &b).unwrap();
    assert_eq!(seven.texcoord4, Some([0., 0.]));
    assert!(seven.color6.is_none());
    assert_eq!(seven.color5.unwrap()[0], 1.);
    let bytes = geometry(1);
    let g = read_geometry_snapshot(&mut &bytes[..], bytes.len() as u64).unwrap();
    let stream = [0u8, 0, 0, 0, 0, 60, 0, 0, 0, 0, 0, 60, 0, 64, 0, 0];
    let locals = [0u8, 0, 1, 0, 2, 0, 3, 0, 2, 0, 1, 0];
    let result = decode_secondary_corners(&manifest(1), &stream, &locals, &g).unwrap();
    assert_ne!(result[0], result[3]);
    assert_eq!(result[1], result[5]);
}
#[test]
fn rejects_unknown_truncated_nonfinite_and_overflow() {
    assert_eq!(
        decode_secondary_record(6, &[]),
        Err(AppearanceError::Unsupported)
    );
    assert_eq!(
        decode_secondary_record(2, &[0; 7]),
        Err(AppearanceError::Attributes)
    );
    assert_eq!(
        decode_secondary_record(1, &[0, 124, 0, 0]),
        Err(AppearanceError::Attributes)
    );
    let bytes = geometry(2);
    let g = read_geometry_snapshot(&mut &bytes[..], bytes.len() as u64).unwrap();
    let mut m = manifest(2);
    assert_eq!(
        decode_secondary_corners(&m, &[0; 8], &[0; 11], &g),
        Err(AppearanceError::Attributes)
    );
    m.surfaces[0].byte_offset = u64::MAX;
    assert_eq!(
        decode_secondary_corners(&m, &[0; 8], &[0; 12], &g),
        Err(AppearanceError::Limit)
    );
    m.surfaces[0].byte_offset = 8;
    assert_eq!(
        decode_secondary_corners(&m, &[0; 8], &[0; 12], &g),
        Err(AppearanceError::Attributes)
    );
    m.surfaces[0].first_corner = 1;
    assert_eq!(
        decode_secondary_corners(&m, &[0; 8], &[0; 12], &g),
        Err(AppearanceError::Attributes)
    );
}
#[test]
fn loader_enforces_snapshot_and_dependency_identities() {
    let root = std::env::temp_dir().join(format!("bo2-secondary-test-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let bytes = geometry(1);
    let g = read_geometry_snapshot(&mut &bytes[..], bytes.len() as u64).unwrap();
    let hash = |b: &[u8]| format!("{:x}", Sha256::digest(b));
    let mut m = manifest(1);
    m.geometry_sha256 = hash(&bytes);
    m.stream = SceneImage {
        file: "stream".into(),
        sha256: hash(&[0; 4]),
    };
    m.corner_local_indices = SceneImage {
        file: "locals".into(),
        sha256: hash(&[0; 12]),
    };
    std::fs::write(root.join("stream"), [0; 4]).unwrap();
    std::fs::write(root.join("locals"), [0; 12]).unwrap();
    let p = root.join("manifest");
    std::fs::write(&p, serde_json::to_vec(&m).unwrap()).unwrap();
    assert_eq!(
        load_secondary_corners(&p, &g, &m.geometry_sha256)
            .unwrap()
            .len(),
        6
    );
    assert_eq!(
        load_secondary_corners(&p, &g, &"1".repeat(64)),
        Err(AppearanceError::Identity)
    );
    std::fs::write(root.join("stream"), [1; 4]).unwrap();
    assert_eq!(
        load_secondary_corners(&p, &g, &m.geometry_sha256),
        Err(AppearanceError::Identity)
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
#[ignore = "requires licensed external scene export"]
fn actual_scene_accounts_all_corners_without_admitting_unknown_layout() {
    let p = std::path::PathBuf::from(std::env::var("BO2_SECONDARY_MANIFEST").unwrap());
    let bytes = std::fs::read(p.parent().unwrap().join("geometry.bo2dgs")).unwrap();
    let g = read_geometry_snapshot(&mut &bytes[..], bytes.len() as u64).unwrap();
    let hash = format!("{:x}", Sha256::digest(&bytes));
    assert_eq!(g.surfaces().len(), 6469);
    assert_eq!(g.indices().len(), 449721);
    let manifest: SecondaryManifest = serde_json::from_slice(&std::fs::read(&p).unwrap()).unwrap();
    let stream = std::fs::read(p.parent().unwrap().join(&manifest.stream.file)).unwrap();
    let locals = std::fs::read(
        p.parent()
            .unwrap()
            .join(&manifest.corner_local_indices.file),
    )
    .unwrap();
    assert_eq!(locals.len(), 449721 * 2);
    let mut supported = 0;
    for (s, gs) in manifest.surfaces.iter().zip(g.surfaces()) {
        assert_eq!(s.first_corner, gs.index_range().start);
        assert_eq!(s.corners, gs.index_range().len());
        assert_eq!(s.source_format, gs.source_format_category());
        let stride = secondary_stride(s.source_format).unwrap();
        for corner in gs.index_range() {
            let local = u16::from_le_bytes([locals[corner * 2], locals[corner * 2 + 1]]) as usize;
            let start = s.byte_offset as usize + local * stride;
            let bytes = if stride == 0 {
                &[][..]
            } else {
                &stream[start..start + stride]
            };
            decode_secondary_record(s.source_format, bytes).unwrap();
            supported += 1;
        }
    }
    assert_eq!(supported, 449721);
    assert_eq!(load_secondary_corners(&p, &g, &hash).unwrap().len(), 449721);
}
