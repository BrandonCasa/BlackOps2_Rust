#![forbid(unsafe_code)]
use bo2_assets::collision_hierarchy::{
    CollisionHierarchy, HEADER_BYTES, HierarchyError, MAGIC, MAX_BYTES, read_collision_hierarchy,
};
use std::io::{self, Read};
fn w(b: &mut Vec<u8>, v: u32) {
    b.extend(v.to_le_bytes());
}
fn half(b: &mut Vec<u8>, v: u16) {
    b.extend(v.to_le_bytes());
}
fn node(b: &mut Vec<u8>, flags: u8, count: i16, contents: u32, children: [u16; 2], first: u32) {
    b.extend([0, flags]);
    half(b, count as u16);
    w(b, contents);
    w(b, 0);
    w(b, 0);
    half(b, children[0]);
    half(b, children[1]);
    w(b, first);
}
fn fixture() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend(MAGIC);
    w(&mut b, 1);
    w(&mut b, 0);
    b.extend(380_u64.to_le_bytes());
    for n in 1..=4 {
        b.extend((n as u64).to_le_bytes());
        b.extend([n; 32]);
    }
    for n in [1, 5, 4, 2, 3, 2, 2, 0] {
        w(&mut b, n);
    }
    assert_eq!(b.len(), HEADER_BYTES);
    for f in [-0_f32, 0., 1., 2.] {
        w(&mut b, f.to_bits());
    }
    b.extend([4, 0]);
    half(&mut b, 0);
    node(&mut b, 0, 0, 0, [0, 0], 0);
    node(&mut b, 0, -1, 0x8000_0001, [1, 2], 0);
    node(&mut b, 0, 2, 1, [0, 0], 0);
    node(&mut b, 1, 1, 0x8000_0000, [0, 0], 2);
    node(&mut b, 3, 1, 0, [0, 0], 3);
    for i in [2, 0, 1, 3] {
        half(&mut b, i);
    }
    for children in [[1, -1], [-2, -1]] {
        w(&mut b, 0);
        for c in children {
            half(&mut b, c as u16);
        }
    }
    assert_eq!(b.len(), 380);
    b
}
fn read(mut b: &[u8]) -> Result<CollisionHierarchy, HierarchyError> {
    let length = b.len() as u64;
    read_collision_hierarchy(&mut b, length)
}
fn change(b: &[u8], at: usize, v: u32) -> Vec<u8> {
    let mut r = b.to_vec();
    r[at..at + 4].copy_from_slice(&v.to_le_bytes());
    r
}
#[test]
fn preserves_order_sharing_source_words_and_gated_special_metadata() {
    let h = read(&fixture()).unwrap();
    assert_eq!(h.planes()[0].normal_bits[0], (-0_f32).to_bits());
    assert_eq!(h.planes()[0].kind, 4);
    assert_eq!(h.brush_nodes()[0].child_offsets, [0, 0]);
    assert_eq!(h.brush_nodes()[1].count, -1);
    assert_eq!(h.brush_nodes()[1].contents, 0x8000_0001);
    assert_eq!(h.brush_indices(), [2, 0, 1, 3]);
    assert_eq!(h.original_pool_count(), 2);
    assert!(h.brush_nodes()[3].external_terminal);
    assert!(h.brush_nodes()[4].inert_box_terminal);
    assert_eq!(h.bsp_nodes()[0].children, [1, -1]);
    assert_eq!(h.bsp_nodes()[1].children, [-2, -1]);
    assert_eq!(h.declared_brush_count(), 3);
    assert_eq!(h.declared_leaf_count(), 2);
    h.check_identity(h.identity()).unwrap();
    let mut expected = h.identity();
    expected.terrain_payload.sha256[0] ^= 1;
    assert_eq!(
        h.check_identity(expected),
        Err(HierarchyError::IdentityMismatch)
    );
}
#[test]
fn rejects_all_truncated_prefixes_and_length_disagreement() {
    let b = fixture();
    for n in 0..b.len() {
        assert_eq!(
            read_collision_hierarchy(&mut &b[..n], b.len() as u64).unwrap_err(),
            HierarchyError::Truncated
        );
    }
    assert_eq!(
        read_collision_hierarchy(&mut b.as_slice(), 381).unwrap_err(),
        HierarchyError::LengthMismatch
    );
    let mut trailing = b;
    trailing.push(0);
    assert_eq!(read(&trailing).unwrap_err(), HierarchyError::LengthMismatch);
}
#[test]
fn rejects_counts_before_body_allocation_and_format_errors() {
    let b = fixture();
    assert_eq!(
        read_collision_hierarchy(&mut b.as_slice(), MAX_BYTES + 1).unwrap_err(),
        HierarchyError::LimitExceeded
    );
    for (at, value, error) in [
        (8, 2, HierarchyError::UnsupportedVersion),
        (12, 1, HierarchyError::NonzeroReserved),
        (212, 1, HierarchyError::NonzeroReserved),
        (188, 1_000_001, HierarchyError::LimitExceeded),
        (192, 4_000_001, HierarchyError::LimitExceeded),
        (196, 32769, HierarchyError::LimitExceeded),
        (208, 5, HierarchyError::InvalidReference),
    ] {
        assert_eq!(read(&change(&b, at, value)).unwrap_err(), error);
    }
    let mut wrong = b;
    wrong[0] ^= 1;
    assert_eq!(read(&wrong).unwrap_err(), HierarchyError::UnsupportedMagic);
}
#[test]
fn rejects_malformed_refs_nonfinite_axis_and_storage_flags() {
    let b = fixture();
    let node0 = 236;
    let node1 = node0 + 24;
    let node2 = node1 + 24;
    let node3 = node2 + 24;
    assert_eq!(
        read(&change(&b, 216, f32::NAN.to_bits())).unwrap_err(),
        HierarchyError::NonFinite
    );
    assert_eq!(
        read(&change(&b, node1 + 8, f32::INFINITY.to_bits())).unwrap_err(),
        HierarchyError::NonFinite
    );
    assert_eq!(
        read(&change(&b, node2 + 20, 4)).unwrap_err(),
        HierarchyError::InvalidReference
    );
    assert_eq!(
        read(&change(&b, 364, 1)).unwrap_err(),
        HierarchyError::InvalidReference
    );
    let mut c = b.clone();
    c[node0] = 3;
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidAxis);
    c = b.clone();
    c[233] = 8;
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidPlaneMetadata);
    c = b.clone();
    c[node3 + 1] = 0;
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidReference);
    c = b.clone();
    c[node0 + 1] = 1;
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidFlags);
    c = b.clone();
    c[362..364].copy_from_slice(&4_u16.to_le_bytes());
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidReference);
    c = b;
    c[372..374].copy_from_slice(&(-3_i16).to_le_bytes());
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidReference);
}
#[test]
fn contents_gate_keeps_inert_cycles_but_participating_and_bsp_cycles_reject() {
    let b = fixture();
    assert_eq!(
        read(&change(&b, 240, 1)).unwrap_err(),
        HierarchyError::Cycle
    );
    let mut c = b.clone();
    c[368..370].copy_from_slice(&0_i16.to_le_bytes());
    assert_eq!(read(&c).unwrap_err(), HierarchyError::Cycle);
    // Shared BSP leaf/node paths remain ordered and legal.
    c = b;
    c[370..372].copy_from_slice(&1_i16.to_le_bytes());
    read(&c).unwrap();
}
#[test]
fn negative_last_node_requires_adjacent_record_and_terminals_cannot_share_box_sentinel() {
    let mut b = fixture();
    b[333] = 0;
    b[334..336].copy_from_slice(&(-1_i16).to_le_bytes());
    b[352..356].copy_from_slice(&0_u32.to_le_bytes());
    assert_eq!(read(&b).unwrap_err(), HierarchyError::InvalidReference);
    b = fixture();
    b[328..332].copy_from_slice(&3_u32.to_le_bytes());
    assert_eq!(read(&b).unwrap_err(), HierarchyError::InvalidReference);
}
#[test]
fn terrain_binding_checks_shared_identity_counts_and_every_root_without_copying_leaves() {
    use bo2_assets::terrain_snapshot::{MAGIC as TERRAIN_MAGIC, read_terrain_snapshot};
    let hierarchy = read(&fixture()).unwrap();
    let terrain = |roots: &[i32], different_identity: bool| {
        let mut b = Vec::new();
        b.extend(TERRAIN_MAGIC);
        w(&mut b, 1);
        w(&mut b, 0);
        b.extend((144 + roots.len() as u64 * 44).to_le_bytes());
        b.extend(1_u64.to_le_bytes());
        b.extend([if different_identity { 9 } else { 1 }; 32]);
        b.extend(2_u64.to_le_bytes());
        b.extend([2; 32]);
        for c in [0, 0, 0, 0, 0, 0, roots.len() as u32, 0] {
            w(&mut b, c);
        }
        b.extend([0; 8]);
        for root in roots {
            b.extend([0; 36]);
            w(&mut b, *root as u32);
            b.extend([0; 4]);
        }
        read_terrain_snapshot(&mut b.as_slice(), b.len() as u64).unwrap()
    };
    hierarchy
        .validate_terrain(&terrain(&[0, 3], false))
        .unwrap();
    assert_eq!(
        hierarchy.validate_terrain(&terrain(&[0, 5], false)),
        Err(HierarchyError::InvalidReference)
    );
    assert_eq!(
        hierarchy.validate_terrain(&terrain(&[0, -1], false)),
        Err(HierarchyError::InvalidReference)
    );
    assert_eq!(
        hierarchy.validate_terrain(&terrain(&[0], false)),
        Err(HierarchyError::InvalidReference)
    );
    assert_eq!(
        hierarchy.validate_terrain(&terrain(&[0, 3], true)),
        Err(HierarchyError::IdentityMismatch)
    );
}
#[test]
fn brush_binding_accepts_unsorted_source_records_and_reports_missing_or_mismatched_geometry() {
    use bo2_assets::brush_collection::read_brush_collection;
    use serde_json::json;
    let h = read(&fixture()).unwrap();
    for (ids, error) in [
        (&[2_u32, 0, 1][..], None),
        (&[2, 0][..], Some(HierarchyError::MissingBrush)),
    ] {
        let records=ids.iter().map(|id|json!({"provider_id":0,"brush":{
            "schema":34,"brush_index":id,"side_first":0,"vertex_first":0,"side_count":1,"vertex_count":0,"contents":1,
            "parallel_bounds_present":false,"parallel_contents_present":false,"parallel_contents":null,"midpoint_bits":null,"halfsize_bits":null,
            "mins_bits":[0,0,0],"maxs_bits":[1065353216,1065353216,1065353216],"axial_cflags":([0;6]),"axial_sflags":([0;6]),"sides":[{"plane_index":0,"cflags":0,"sflags":0,"normal_bits":[2147483648_u32,0,1065353216],"dist_bits":1073741824,"type":4,"signbits":0}],"vertices_bits":[]
        }})).collect::<Vec<_>>();
        let v = json!({"schema":34,"interpretation":"diagnostic_relative_negative_adjacent_zero_empty",
            "source":{"bytes":1,"sha256":"01".repeat(32),"membership_bytes":2,"membership_sha256":"02".repeat(32)},
            "selection":{"status":"admitted","reached_nodes":4,"keys":ids.iter().map(|id|json!({"provider_id":0,"brush_id":id})).collect::<Vec<_>>()},
            "coverage":{"models":0,"absent_model_providers":0,"box_excluded":true,"triangles":0,"partitions":0,"aabb_trees":0,"static_models":0,"dynents":([0;4])},
            "providers":[{"id":0,"planes":1,"sides":1,"vertices":0,"brushes":3,"optional_bounds_present":false,"optional_contents_present":false}],"record_count":records.len(),"records":records});
        let bytes = serde_json::to_vec(&v).unwrap();
        let brushes = read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
        assert_eq!(
            h.validate_brush_collection(&brushes),
            error.map_or(Ok(()), Err)
        );
        let mut conflict = v.clone();
        for record in conflict["records"].as_array_mut().unwrap() {
            record["brush"]["sides"][0]["dist_bits"] = json!(1065353216);
        }
        let bytes = serde_json::to_vec(&conflict).unwrap();
        let brushes = read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
        assert_eq!(
            h.validate_brush_collection(&brushes),
            Err(HierarchyError::ConflictingPlane)
        );
        let mut v = v;
        v["source"]["bytes"] = json!(99);
        let bytes = serde_json::to_vec(&v).unwrap();
        let brushes = read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
        assert_eq!(
            h.validate_brush_collection(&brushes),
            Err(HierarchyError::IdentityMismatch)
        );
    }
}
#[test]
fn box_marker_requires_exact_inert_last_terminal_and_sentinel() {
    let b = fixture();
    let boxed = 332;
    assert_eq!(
        read(&change(&b, boxed + 4, 1)).unwrap_err(),
        HierarchyError::InvalidBoxMetadata
    );
    let mut c = b.clone();
    c[362..364].copy_from_slice(&0_u16.to_le_bytes());
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidBoxMetadata);
    c = b.clone();
    c[309] = 3;
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidBoxMetadata);
    c = b;
    c[333] = 1;
    assert_eq!(read(&c).unwrap_err(), HierarchyError::InvalidReference);
}
struct One<'a>(&'a [u8]);
impl Read for One<'_> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        if self.0.is_empty() {
            return Ok(0);
        }
        b[0] = self.0[0];
        self.0 = &self.0[1..];
        Ok(1)
    }
}
struct Hostile(u8);
impl Read for Hostile {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        match self.0 {
            0 => Ok(b.len() + 1),
            1 => Err(io::ErrorKind::Interrupted.into()),
            _ => Err(io::ErrorKind::PermissionDenied.into()),
        }
    }
}
#[test]
fn handles_short_reads_and_rejects_hostile_reader_behavior() {
    let b = fixture();
    read_collision_hierarchy(&mut One(&b), b.len() as u64).unwrap();
    for (mode, error) in [
        (0, HierarchyError::InvalidReadCount),
        (1, HierarchyError::InterruptedReadLimit),
        (2, HierarchyError::Io(io::ErrorKind::PermissionDenied)),
    ] {
        assert_eq!(
            read_collision_hierarchy(&mut Hostile(mode), b.len() as u64).unwrap_err(),
            error
        );
    }
}
#[test]
#[ignore = "requires external licensed hierarchy/terrain/schema34 payloads and independent identities"]
fn retained_full_hierarchy_admission() {
    use bo2_assets::{
        brush_collection::read_brush_collection, terrain_snapshot::read_terrain_snapshot,
    };
    use std::{env, fs, io::BufReader};
    let hbytes =
        fs::read(env::var_os("BO2_HIERARCHY_SNAPSHOT").expect("explicit external hierarchy"))
            .unwrap();
    let actual = sha256(&hbytes);
    assert_eq!(
        actual,
        env::var("BO2_HIERARCHY_SHA256").expect("independent hierarchy hash")
    );
    let h = read(&hbytes).unwrap();
    assert_eq!(
        (
            h.planes().len(),
            h.brush_nodes().len(),
            h.brush_indices().len(),
            h.bsp_nodes().len()
        ),
        (12105, 14968, 26037, 4525)
    );
    assert_eq!(h.original_pool_count(), 25453);
    let tbytes = fs::read(env::var_os("BO2_TERRAIN_SNAPSHOT").unwrap()).unwrap();
    assert_eq!(sha256(&tbytes), hex(h.identity().terrain_payload.sha256));
    assert_eq!(tbytes.len() as u64, h.identity().terrain_payload.bytes);
    let t = read_terrain_snapshot(&mut tbytes.as_slice(), tbytes.len() as u64).unwrap();
    h.validate_terrain(&t).unwrap();
    let bbytes = fs::read(env::var_os("BO2_BRUSH_COLLECTION").unwrap()).unwrap();
    assert_eq!(sha256(&bbytes), hex(h.identity().brush_payload.sha256));
    assert_eq!(bbytes.len() as u64, h.identity().brush_payload.bytes);
    let b =
        read_brush_collection(&mut BufReader::new(bbytes.as_slice()), bbytes.len() as u64).unwrap();
    h.validate_brush_collection(&b).unwrap();
}
fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    hex(Sha256::digest(bytes).into())
}
