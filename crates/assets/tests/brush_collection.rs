#![forbid(unsafe_code)]
use bo2_assets::{
    brush_collection::{
        CollectionError, CollectionIdentity, Identity, MAX_BYTES, read_brush_collection,
    },
    brush_snapshot::BrushError,
};
use serde_json::{Value, json};

fn brush(id: u32, zero: bool) -> Value {
    let b = |x: f32| x.to_bits();
    json!({"schema":34,"brush_index":id,"side_first":0,"vertex_first":0,"side_count":usize::from(!zero),"vertex_count":2,"contents":u32::MAX,
    "parallel_bounds_present":false,"parallel_contents_present":false,"parallel_contents":null,"midpoint_bits":null,"halfsize_bits":null,
    "mins_bits":[0,0,0],"maxs_bits":[b(1.0),b(2.0),b(3.0)],"axial_cflags":([u32::MAX;6]),"axial_sflags":([2147483648u32;6]),
    "sides":if zero {json!([])} else {json!([{"plane_index":0,"cflags":u32::MAX,"sflags":2147483648u32,"normal_bits":[b(1.0),0,0],"dist_bits":b(1.0),"type":0,"signbits":0}])},
    "vertices_bits":[[0,0,0],[b(1.0),b(2.0),b(3.0)]]})
}
fn fixture() -> Value {
    json!({"schema":34,"interpretation":"diagnostic_relative_negative_adjacent_zero_empty",
    "source":{"bytes":10,"sha256":"01".repeat(32),"membership_bytes":20,"membership_sha256":"02".repeat(32)},
    "selection":{"status":"admitted","reached_nodes":5,"keys":[{"provider_id":3,"brush_id":1},{"provider_id":3,"brush_id":0}]},
    "coverage":{"models":2,"absent_model_providers":2,"box_excluded":true,"triangles":7,"partitions":8,"aabb_trees":9,"static_models":10,"dynents":[1,2,3,4]},
    "providers":[{"id":3,"planes":1,"sides":1,"vertices":2,"brushes":2,"optional_bounds_present":false,"optional_contents_present":false}],
    "record_count":2,"records":[{"provider_id":3,"brush":brush(1,true)},{"provider_id":3,"brush":brush(0,false)}]})
}
fn read(
    v: &Value,
) -> Result<bo2_assets::brush_collection::BrushCollectionSnapshot, CollectionError> {
    let bytes = serde_json::to_vec(v).unwrap();
    read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64)
}
#[test]
fn schema34_preserves_selection_order_zero_sides_words_and_identity_declarations() {
    let d = read(&fixture()).unwrap();
    assert_eq!(d.records().len(), 2);
    assert_eq!(d.records()[0].key().brush_id, 1);
    assert!(d.records()[0].snapshot().sides().is_empty());
    assert_eq!(d.records()[0].snapshot().contents(), u32::MAX);
    assert_eq!(
        d.records()[0].snapshot().axial_sflags(),
        &[2147483648u32; 6]
    );
    assert_eq!(d.selected_keys()[0], d.records()[0].key());
    let expected = CollectionIdentity {
        source: Identity {
            bytes: 10,
            sha256: [1; 32],
        },
        membership: Identity {
            bytes: 20,
            sha256: [2; 32],
        },
    };
    d.check_identity(expected).unwrap();
    let mut bad = expected;
    bad.source.bytes += 1;
    assert_eq!(
        d.check_identity(bad),
        Err(CollectionError::IdentityMismatch)
    );
    assert_eq!(d.coverage().triangles, 7);
    assert_eq!(d.providers()[0].id, 3);
}
#[test]
fn rejects_missing_duplicate_changed_keys_and_coverage() {
    let original = fixture();
    let cases: Vec<(Value, CollectionError)> = vec![
        {
            let mut v = original.clone();
            v["records"].as_array_mut().unwrap().pop();
            (v, CollectionError::SelectionMismatch)
        },
        {
            let mut v = original.clone();
            v["records"][1] = v["records"][0].clone();
            (v, CollectionError::SelectionMismatch)
        },
        {
            let mut v = original.clone();
            v["selection"]["keys"][1] = v["selection"]["keys"][0].clone();
            (v, CollectionError::DuplicateKey)
        },
        {
            let mut v = original.clone();
            v["selection"]["keys"][0]["brush_id"] = json!(9);
            (v, CollectionError::SelectionMismatch)
        },
        {
            let mut v = original.clone();
            v["records"][0]["provider_id"] = json!(4);
            v["selection"]["keys"][0]["provider_id"] = json!(4);
            (v, CollectionError::InvalidReference)
        },
        {
            let mut v = original.clone();
            v["coverage"]["box_excluded"] = json!(false);
            (v, CollectionError::InvalidCoverage)
        },
        {
            let mut v = original.clone();
            v["coverage"]["absent_model_providers"] = json!(3);
            (v, CollectionError::InvalidCoverage)
        },
        {
            let mut v = original.clone();
            v["providers"][0]["optional_bounds_present"] = json!(true);
            (v, CollectionError::InvalidCoverage)
        },
        {
            let mut v = original.clone();
            let duplicate = v["providers"][0].clone();
            v["providers"].as_array_mut().unwrap().push(duplicate);
            (v, CollectionError::DuplicateKey)
        },
    ];
    for (v, error) in cases {
        assert_eq!(read(&v).unwrap_err(), error);
    }
}
#[test]
fn rejects_references_nullable_fields_nonfinite_opaque_types_and_limits() {
    let cases: Vec<(&[&str], Value, CollectionError)> = vec![
        (&["schema"], json!(33), CollectionError::UnsupportedSchema),
        (
            &["interpretation"],
            json!("unproven"),
            CollectionError::UnsupportedInterpretation,
        ),
        (
            &["source", "sha256"],
            json!("x".repeat(64)),
            CollectionError::InvalidIdentity,
        ),
        (
            &["record_count"],
            json!(65536),
            CollectionError::LimitExceeded,
        ),
        (
            &["providers", "0", "vertices"],
            json!(1_000_001),
            CollectionError::LimitExceeded,
        ),
        (
            &["records", "1", "brush", "side_first"],
            json!(1),
            CollectionError::InvalidReference,
        ),
        (
            &["records", "1", "brush", "vertex_first"],
            json!(u32::MAX),
            CollectionError::Brush(BrushError::ReferenceOverflow),
        ),
        (
            &["records", "1", "brush", "sides", "0", "plane_index"],
            json!(1),
            CollectionError::InvalidReference,
        ),
        (
            &["records", "1", "brush", "sides", "0", "normal_bits"],
            json!([0, 0, 0]),
            CollectionError::Brush(BrushError::ZeroNormal),
        ),
        (
            &["records", "0", "brush", "mins_bits"],
            json!([f32::INFINITY.to_bits(), 0, 0]),
            CollectionError::Brush(BrushError::NonFinite),
        ),
        (
            &["records", "0", "brush", "contents"],
            json!(-1),
            CollectionError::Brush(BrushError::InvalidJson),
        ),
        (
            &["records", "0", "brush", "schema"],
            json!(28),
            CollectionError::Brush(BrushError::UnsupportedSchema),
        ),
    ];
    for (path, value, error) in cases {
        let mut v = fixture();
        let mut target = &mut v;
        for key in path {
            target = if let Ok(i) = key.parse::<usize>() {
                &mut target[i]
            } else {
                &mut target[*key]
            };
        }
        *target = value;
        assert_eq!(read(&v).unwrap_err(), error);
    }
    let mut v = fixture();
    v["records"][0]["brush"]
        .as_object_mut()
        .unwrap()
        .remove("parallel_contents");
    assert_eq!(
        read(&v).unwrap_err(),
        CollectionError::Brush(BrushError::InvalidJson)
    );
}
#[test]
fn malformed_length_oversize_and_redacted_failures() {
    let b = serde_json::to_vec(&fixture()).unwrap();
    assert_eq!(
        read_brush_collection(&mut &b[..b.len() - 1], b.len() as u64).unwrap_err(),
        CollectionError::Brush(BrushError::LengthMismatch)
    );
    assert_eq!(
        read_brush_collection(&mut b.as_slice(), MAX_BYTES as u64 + 1).unwrap_err(),
        CollectionError::Brush(BrushError::LimitExceeded)
    );
    for mut b in [
        b"{PRIVATE".as_slice(),
        b"{\"schema\":34,\"schema\":34}",
        b"null",
    ] {
        let len = b.len() as u64;
        let e = read_brush_collection(&mut b, len).unwrap_err();
        assert_eq!(e, CollectionError::Brush(BrushError::InvalidJson));
        assert!(!format!("{e:?} {e}").contains("PRIVATE"));
    }
}
#[test]
fn aggregate_admission_rejects_bounded_repeated_source_vertex_spans() {
    let mut v = fixture();
    let count = 3907u32;
    let mut records = Vec::new();
    let mut keys = Vec::new();
    for id in 0..count {
        let mut b = brush(id, true);
        b["vertex_count"] = json!(256);
        b["vertices_bits"] = json!(vec![[0u32; 3]; 256]);
        records.push(json!({"provider_id":3,"brush":b}));
        keys.push(json!({"provider_id":3,"brush_id":id}));
    }
    v["record_count"] = json!(count);
    v["records"] = json!(records);
    v["selection"]["keys"] = json!(keys);
    v["providers"][0]["brushes"] = json!(count);
    v["providers"][0]["vertices"] = json!(256);
    assert_eq!(read(&v).unwrap_err(), CollectionError::LimitExceeded);
}

#[test]
#[ignore = "requires BO2_BRUSH_COLLECTION and BO2_BRUSH_MEMBERSHIP external inputs"]
fn external_all_records_preserve_words_and_bind_retained_selection() {
    use sha2::{Digest, Sha256};
    let path =
        std::env::var_os("BO2_BRUSH_COLLECTION").expect("external collection input required");
    let bytes = std::fs::read(path).expect("external collection read failed");
    let d = read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let path =
        std::env::var_os("BO2_BRUSH_MEMBERSHIP").expect("external membership input required");
    let mbytes = std::fs::read(path).expect("external membership read failed");
    let m: Value = serde_json::from_slice(&mbytes).unwrap();
    let mut expected = d.declared_identity();
    expected.membership = Identity {
        bytes: mbytes.len() as u64,
        sha256: Sha256::digest(&mbytes).into(),
    };
    d.check_identity(expected).unwrap();
    assert_eq!(d.records().len(), 8417);
    assert_eq!(
        d.records()
            .iter()
            .filter(|r| r.snapshot().sides().is_empty())
            .count(),
        3178
    );
    let ids = m["map_leaf_candidate"]["indices"].as_array().unwrap();
    assert_eq!(ids.len(), d.selected_keys().len());
    for (key, id) in d.selected_keys().iter().zip(ids) {
        assert_eq!(key.provider_id, 0);
        assert_eq!(Some(u64::from(key.brush_id)), id.as_u64());
    }
    for (record, raw) in d.records().iter().zip(v["records"].as_array().unwrap()) {
        let s = record.snapshot();
        let retained = json!({"schema":s.schema(),"brush_index":s.brush_index(),"side_first":s.side_first(),"vertex_first":s.vertex_first(),"side_count":s.sides().len(),"vertex_count":s.vertices_bits().len(),"contents":s.contents(),"parallel_bounds_present":s.midpoint_bits().is_some(),"parallel_contents_present":s.parallel_contents().is_some(),"parallel_contents":s.parallel_contents(),"mins_bits":s.mins_bits(),"maxs_bits":s.maxs_bits(),"midpoint_bits":s.midpoint_bits(),"halfsize_bits":s.halfsize_bits(),"axial_cflags":s.axial_cflags(),"axial_sflags":s.axial_sflags(),"sides":s.sides().iter().map(|s|json!({"plane_index":s.plane_index,"cflags":s.cflags,"sflags":s.sflags,"normal_bits":s.normal_bits,"dist_bits":s.dist_bits,"type":s.plane_type,"signbits":s.signbits})).collect::<Vec<_>>(),"vertices_bits":s.vertices_bits()});
        assert!(
            retained == raw["brush"],
            "external word or metadata mismatch"
        );
        let meta = &m["providers"][record.key().provider_id as usize]["brush_metadata"]
            [record.key().brush_id as usize];
        assert_eq!(meta[0].as_u64(), Some(s.sides().len() as u64));
        assert_eq!(meta[1].as_u64(), Some(s.vertices_bits().len() as u64));
        assert_eq!(meta[2].as_u64(), Some(u64::from(s.contents())));
    }
}

#[test]
fn shared_pool_words_must_agree_without_conflating_side_flags_with_planes() {
    let mut v = fixture();
    v["records"][0]["brush"] = brush(1, false);
    read(&v).unwrap();
    let mut vertex = v.clone();
    vertex["records"][0]["brush"]["vertices_bits"][0] = json!([1_f32.to_bits(), 0, 0]);
    assert_eq!(
        read(&vertex).unwrap_err(),
        CollectionError::ConflictingReference
    );
    let mut side = v.clone();
    side["records"][0]["brush"]["sides"][0]["cflags"] = json!(7);
    assert_eq!(
        read(&side).unwrap_err(),
        CollectionError::ConflictingReference
    );
    v["providers"][0]["sides"] = json!(2);
    v["records"][0]["brush"]["side_first"] = json!(1);
    v["records"][0]["brush"]["sides"][0]["cflags"] = json!(7);
    read(&v).unwrap();
    v["records"][0]["brush"]["sides"][0]["dist_bits"] = json!(2_f32.to_bits());
    assert_eq!(read(&v).unwrap_err(), CollectionError::ConflictingReference);
}
