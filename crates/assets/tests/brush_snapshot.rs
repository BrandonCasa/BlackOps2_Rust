#![forbid(unsafe_code)]
use bo2_assets::brush_snapshot::{BrushError, BrushSnapshot, MAX_BYTES, read_brush_snapshot};
use serde_json::{Value, json};
use std::io::{self, Read};

// Arbitrary user-created schema record; no original values or derived fixtures.
fn fixture() -> Value {
    let bits = |x: f32| x.to_bits();
    json!({"schema":28,"brush_index":13,"side_first":2,"vertex_first":5,
        "side_count":1,"vertex_count":1,"contents":123,
        "parallel_bounds_present":false,"parallel_contents_present":false,
        "parallel_contents":null,"midpoint_bits":null,"halfsize_bits":null,
        "mins_bits":[bits(-2.0),bits(-1.0),bits(-3.0)],
        "maxs_bits":[bits(2.0),bits(3.0),bits(4.0)],
        "axial_cflags":[1,2,3,4,5,6],"axial_sflags":[6,5,4,3,2,1],
        "sides":[{"plane_index":19,"cflags":321,"sflags":456,
        "normal_bits":[bits(1.0),bits(-0.0),bits(0.0)],"dist_bits":bits(2.0),"type":9,"signbits":241}],
        "vertices_bits":[[bits(-0.0),bits(1.0),bits(2.0)]]})
}
fn read(v: &Value) -> Result<BrushSnapshot, BrushError> {
    let bytes = serde_json::to_vec(v).unwrap();
    read_brush_snapshot(&mut bytes.as_slice(), bytes.len() as u64)
}
#[test]
fn preserves_bits_metadata_order_and_explicit_absence() {
    let snapshot = read(&fixture()).unwrap();
    assert_eq!(snapshot.schema(), 28);
    assert_eq!(
        (
            snapshot.brush_index(),
            snapshot.side_first(),
            snapshot.vertex_first()
        ),
        (13, 2, 5)
    );
    assert_eq!(snapshot.contents(), 123);
    assert_eq!(snapshot.parallel_contents(), None);
    assert_eq!(snapshot.midpoint_bits(), None);
    assert_eq!(snapshot.halfsize_bits(), None);
    assert_eq!(snapshot.axial_cflags(), &[1, 2, 3, 4, 5, 6]);
    assert_eq!(snapshot.axial_sflags(), &[6, 5, 4, 3, 2, 1]);
    let s = &snapshot.sides()[0];
    assert_eq!(
        (s.plane_index, s.cflags, s.sflags, s.plane_type, s.signbits),
        (19, 321, 456, 9, 241)
    );
    assert_eq!(s.normal_bits, [1_f32.to_bits(), (-0_f32).to_bits(), 0]);
    assert_eq!(s.dist_bits, 2_f32.to_bits());
    assert_eq!(
        snapshot.vertices_bits()[0],
        [(-0_f32).to_bits(), 1_f32.to_bits(), 2_f32.to_bits()]
    );
    assert!(!format!("{snapshot:?}").contains("321"));
}
#[test]
fn nullable_fields_are_required_and_presence_must_match() {
    for key in [
        "parallel_contents",
        "midpoint_bits",
        "halfsize_bits",
        "parallel_contents_present",
        "parallel_bounds_present",
    ] {
        let mut v = fixture();
        v.as_object_mut().unwrap().remove(key);
        assert_eq!(read(&v).unwrap_err(), BrushError::InvalidJson);
    }
    for key in ["parallel_contents_present", "parallel_bounds_present"] {
        let mut v = fixture();
        v[key] = json!(true);
        assert_eq!(read(&v).unwrap_err(), BrushError::PresenceMismatch);
    }
    let mut v = fixture();
    v["parallel_bounds_present"] = json!(true);
    v["parallel_contents_present"] = json!(true);
    v["midpoint_bits"] = json!([0, 0, 0]);
    v["halfsize_bits"] = json!(([1_f32.to_bits(); 3]));
    v["parallel_contents"] = json!(999);
    let s = read(&v).unwrap();
    assert_eq!(s.parallel_contents(), Some(999)); // Opaque, not silently joined to contents.
    assert_eq!(s.midpoint_bits(), Some([0; 3]));
    assert_eq!(s.halfsize_bits(), Some([1_f32.to_bits(); 3]));
    for key in ["midpoint_bits", "halfsize_bits", "parallel_contents"] {
        let mut absent = v.clone();
        absent[key] = Value::Null;
        assert_eq!(read(&absent).unwrap_err(), BrushError::PresenceMismatch);
    }
    let mut negative = v.clone();
    negative["halfsize_bits"][0] = json!((-1_f32).to_bits());
    assert_eq!(read(&negative).unwrap_err(), BrushError::InvalidBounds);
    let mut only_contents = fixture();
    only_contents["parallel_contents_present"] = json!(true);
    only_contents["parallel_contents"] = json!(0);
    assert!(read(&only_contents).is_ok());
}
#[test]
fn rejects_schema_counts_types_references_bounds_and_nonfinite_bits() {
    let cases = [
        ("schema", json!(27), BrushError::UnsupportedSchema),
        ("side_count", json!(0), BrushError::LimitExceeded),
        ("side_count", json!(65), BrushError::LimitExceeded),
        ("vertex_count", json!(257), BrushError::LimitExceeded),
        ("side_count", json!(2), BrushError::CountMismatch),
        ("vertex_count", json!(0), BrushError::CountMismatch),
        ("side_first", json!(u32::MAX), BrushError::ReferenceOverflow),
        (
            "vertex_first",
            json!(u32::MAX),
            BrushError::ReferenceOverflow,
        ),
        ("contents", json!(-1), BrushError::InvalidJson),
        ("mins_bits", json!([0, 0]), BrushError::InvalidJson),
        (
            "maxs_bits",
            json!(([(-4_f32).to_bits(); 3])),
            BrushError::InvalidBounds,
        ),
        (
            "mins_bits",
            json!([f32::INFINITY.to_bits(), 0, 0]),
            BrushError::NonFinite,
        ),
        (
            "parallel_bounds_present",
            json!("PRIVATE"),
            BrushError::InvalidJson,
        ),
    ];
    for (key, value, expected) in cases {
        let mut v = fixture();
        v[key] = value;
        assert_eq!(read(&v).unwrap_err(), expected, "{key}");
    }
    for bits in [f32::NAN.to_bits(), f32::NEG_INFINITY.to_bits()] {
        let mut v = fixture();
        v["sides"][0]["dist_bits"] = json!(bits);
        assert_eq!(read(&v).unwrap_err(), BrushError::NonFinite);
        v = fixture();
        v["vertices_bits"][0][1] = json!(bits);
        assert_eq!(read(&v).unwrap_err(), BrushError::NonFinite);
        v = fixture();
        v["sides"][0]["normal_bits"][1] = json!(bits);
        assert_eq!(read(&v).unwrap_err(), BrushError::NonFinite);
    }
    let mut v = fixture();
    v["sides"][0]["normal_bits"] = json!([0, 0, 0]);
    assert_eq!(read(&v).unwrap_err(), BrushError::ZeroNormal);
    let mut v = fixture();
    v["PRIVATE_UNKNOWN"] = json!("PRIVATE");
    assert_eq!(read(&v).unwrap_err(), BrushError::InvalidJson);
    let mut largest = fixture();
    largest["sides"] = json!(vec![largest["sides"][0].clone(); 64]);
    largest["vertices_bits"] = json!(vec![largest["vertices_bits"][0].clone(); 256]);
    largest["side_count"] = json!(64);
    largest["vertex_count"] = json!(256);
    let admitted = read(&largest).unwrap();
    assert_eq!(admitted.sides().len(), 64);
    assert_eq!(admitted.vertices_bits().len(), 256);
    largest["vertices_bits"] = json!([]);
    largest["vertex_count"] = json!(0);
    assert!(read(&largest).unwrap().vertices_bits().is_empty());
}
#[test]
fn byte_length_json_and_io_failures_are_bounded_redacted() {
    let bytes = serde_json::to_vec(&fixture()).unwrap();
    for cut in [0, 1, bytes.len() - 1] {
        assert_eq!(
            read_brush_snapshot(&mut &bytes[..cut], bytes.len() as u64).unwrap_err(),
            BrushError::LengthMismatch
        );
    }
    assert_eq!(
        read_brush_snapshot(&mut bytes.as_slice(), bytes.len() as u64 - 1).unwrap_err(),
        BrushError::LengthMismatch
    );
    for malformed in [
        b"{PRIVATE".as_slice(),
        b"null",
        b"{}",
        b"{\"schema\":28,\"schema\":28}",
    ] {
        let mut input = malformed;
        assert_eq!(
            read_brush_snapshot(&mut input, malformed.len() as u64).unwrap_err(),
            BrushError::InvalidJson
        );
    }
    assert_eq!(
        read_brush_snapshot(&mut bytes.as_slice(), MAX_BYTES as u64 + 1).unwrap_err(),
        BrushError::LimitExceeded
    );
    let oversized = vec![b' '; MAX_BYTES + 1];
    assert_eq!(
        read_brush_snapshot(&mut oversized.as_slice(), MAX_BYTES as u64).unwrap_err(),
        BrushError::LimitExceeded
    );
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "PRIVATE_PATH",
            ))
        }
    }
    let e = read_brush_snapshot(&mut Broken, 10).unwrap_err();
    assert_eq!(e, BrushError::Io(io::ErrorKind::PermissionDenied));
    assert!(!format!("{e:?} {e}").contains("PRIVATE"));
    struct Interrupted;
    impl Read for Interrupted {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::Interrupted.into())
        }
    }
    assert_eq!(
        read_brush_snapshot(&mut Interrupted, 0).unwrap_err(),
        BrushError::InterruptedReadLimit
    );
    struct InvalidCount;
    impl Read for InvalidCount {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            Ok(b.len() + 1)
        }
    }
    assert_eq!(
        read_brush_snapshot(&mut InvalidCount, 10).unwrap_err(),
        BrushError::InvalidReadCount
    );
}

/// Opt-in same-handle external record check. The path is provided only through
/// the environment, never embedded in code/fixtures or printed by this test.
#[test]
#[ignore = "requires explicit BO2_BRUSH_SNAPSHOT external input"]
fn external_record_preserves_every_word_and_opaque_field() {
    let path = std::env::var_os("BO2_BRUSH_SNAPSHOT").expect("external input variable is required");
    let mut file = std::fs::File::open(path).expect("external input open failed");
    let metadata = file.metadata().expect("external input metadata failed");
    assert!(metadata.is_file() && metadata.len() <= MAX_BYTES as u64);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("external read failed");
    assert_eq!(bytes.len() as u64, metadata.len());
    let snapshot = read_brush_snapshot(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let retained = json!({"schema":snapshot.schema(),"brush_index":snapshot.brush_index(),"side_first":snapshot.side_first(),"vertex_first":snapshot.vertex_first(),
        "side_count":snapshot.sides().len(),"vertex_count":snapshot.vertices_bits().len(),"contents":snapshot.contents(),
        "parallel_bounds_present":snapshot.midpoint_bits().is_some(),"parallel_contents_present":snapshot.parallel_contents().is_some(),
        "parallel_contents":snapshot.parallel_contents(),"mins_bits":snapshot.mins_bits(),"maxs_bits":snapshot.maxs_bits(),
        "midpoint_bits":snapshot.midpoint_bits(),"halfsize_bits":snapshot.halfsize_bits(),"axial_cflags":snapshot.axial_cflags(),"axial_sflags":snapshot.axial_sflags(),
        "sides":snapshot.sides().iter().map(|s|json!({"plane_index":s.plane_index,"cflags":s.cflags,"sflags":s.sflags,
            "normal_bits":s.normal_bits,"dist_bits":s.dist_bits,"type":s.plane_type,"signbits":s.signbits})).collect::<Vec<_>>(),
        "vertices_bits":snapshot.vertices_bits()});
    // A failure must not print proprietary values through assert_eq! Debug.
    assert!(
        v == retained,
        "external record word/metadata preservation failed"
    );
}
