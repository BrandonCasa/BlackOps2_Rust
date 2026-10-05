#![forbid(unsafe_code)]
use bo2_assets::brush_collection::read_brush_collection;
use bo2_physics::{
    DVec3,
    brush::{Convention, QueryError, RoundingWitness},
    brush_collection::BrushCollection,
};
use serde_json::{Value, json};
fn fixture(boxes: &[(u32, f32, f32, bool)]) -> Value {
    let b = |x: f32| x.to_bits();
    let records:Vec<_>=boxes.iter().enumerate().map(|(i,&(id,lo,hi,zero))|json!({"provider_id":2,"brush":{"schema":34,"brush_index":id,"side_first":i,"vertex_first":0,"side_count":usize::from(!zero),"vertex_count":0,"contents":0,"parallel_bounds_present":false,"parallel_contents_present":false,"parallel_contents":null,"midpoint_bits":null,"halfsize_bits":null,"mins_bits":[b(lo),0,0],"maxs_bits":[b(hi),b(1.0),b(1.0)],"axial_cflags":[0,0,0,0,0,0],"axial_sflags":[0,0,0,0,0,0],"sides":if zero{json!([])}else{json!([{"plane_index":i,"normal_bits":[b(1.0),0,0],"dist_bits":b(hi),"cflags":0,"sflags":0,"type":0,"signbits":0}])},"vertices_bits":[]}})).collect();
    json!({"schema":34,"interpretation":"diagnostic_relative_negative_adjacent_zero_empty","source":{"bytes":1,"sha256":"00".repeat(32),"membership_bytes":1,"membership_sha256":"11".repeat(32)},"selection":{"status":"admitted","reached_nodes":1,"keys":boxes.iter().map(|(id,_,_,_)|json!({"provider_id":2,"brush_id":id})).collect::<Vec<_>>()},"coverage":{"models":0,"absent_model_providers":0,"box_excluded":true,"triangles":0,"partitions":0,"aabb_trees":0,"static_models":0,"dynents":[0,0,0,0]},"providers":[{"id":2,"planes":boxes.len(),"sides":boxes.len(),"vertices":0,"brushes":boxes.iter().map(|(id,_,_,_)|id+1).max().unwrap_or(0),"optional_bounds_present":false,"optional_contents_present":false}],"record_count":records.len(),"records":records})
}
fn collection(v: &Value) -> BrushCollection {
    let bytes = serde_json::to_vec(v).unwrap();
    let s = read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
    BrushCollection::from_snapshot(&s, Convention::DiagnosticMinusDistanceAtMostZero).unwrap()
}
fn p(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}
#[test]
fn separated_hulls_choose_mathematical_nearest_entry_in_both_directions() {
    let v = fixture(&[(9, 3., 4., false), (5, 0., 1., true)]);
    let c = collection(&v);
    let a = p(-1., 0.5, 0.5);
    let b = p(5., 0.5, 0.5);
    let hit = c.clip_segment(a, b).unwrap().unwrap();
    assert_eq!(hit.key.brush_id, 5);
    assert_eq!(hit.interval.entering, 1. / 6.);
    assert_eq!(hit.interval.exiting, 1. / 3.);
    let hit = c.clip_segment(b, a).unwrap().unwrap();
    assert_eq!(hit.key.brush_id, 9);
    assert_eq!(hit.interval.entering, 1. / 6.);
    assert_eq!(hit.interval.exiting, 1. / 3.);
    assert_eq!(c.brushes()[0].key().brush_id, 9);
    assert_eq!(c.brushes()[1].hull().planes().len(), 6);
}
#[test]
fn overlapping_equal_entry_ties_are_lowest_key_independent_of_record_order() {
    let mut v = fixture(&[(3, 0., 1., false), (1, 0., 2., true)]);
    let c = collection(&v);
    let a = p(-1., 0.5, 0.5);
    let b = p(3., 0.5, 0.5);
    let expected = c.clip_segment(a, b).unwrap();
    let hit = expected.unwrap();
    assert_eq!(hit.key.brush_id, 1);
    assert_eq!(hit.interval.entering, 0.25);
    assert_eq!(hit.interval.exiting, 0.75);
    for _ in 0..100 {
        assert_eq!(c.clip_segment(a, b).unwrap(), expected);
    }
    v["records"].as_array_mut().unwrap().reverse();
    assert_eq!(collection(&v).clip_segment(a, b).unwrap(), expected);
    let inside = c
        .clip_segment(p(0.5, 0.5, 0.5), p(0.5, 0.5, 0.5))
        .unwrap()
        .unwrap();
    assert_eq!(inside.key.brush_id, 1);
    assert!(inside.interval.start_inside);
    assert_eq!(inside.interval.entering, 0.);
    assert_eq!(inside.interval.exiting, 1.);
}
#[test]
fn zero_side_axial_box_has_analytic_inside_stationary_tangent_and_miss_cases() {
    let c = collection(&fixture(&[(0, 0., 1., true)]));
    let hit = c
        .clip_segment(p(-1., 0.5, 0.5), p(2., 0.5, 0.5))
        .unwrap()
        .unwrap();
    assert_eq!(hit.interval.entering, 1. / 3.);
    assert_eq!(hit.interval.exiting, 2. / 3.);
    let tangent = c
        .clip_segment(p(-1., 1., 0.5), p(1., -1., 0.5))
        .unwrap()
        .unwrap();
    assert_eq!(tangent.interval.entering, 0.5);
    assert_eq!(tangent.interval.exiting, 0.5);
    assert!(!tangent.interval.start_inside);
    assert!(
        c.clip_segment(p(0., 0., 0.), p(0., 0., 0.))
            .unwrap()
            .unwrap()
            .interval
            .start_inside
    );
    assert_eq!(
        c.clip_segment(p(-1., 0.5, 0.5), p(-1., 0.5, 0.5)).unwrap(),
        None
    );
    assert_eq!(
        c.clip_segment(p(-1., 2., 0.5), p(2., 2., 0.5)).unwrap(),
        None
    );
    let exit = c
        .clip_segment(p(0.5, 0.5, 0.5), p(1.5, 0.5, 0.5))
        .unwrap()
        .unwrap();
    assert_eq!(exit.interval.entering, 0.);
    assert_eq!(exit.interval.exiting, 0.5);
    assert!(exit.interval.start_inside);
}
#[test]
fn nonfinite_and_finite_arithmetic_errors_are_visible_including_empty_collection() {
    let c = collection(&fixture(&[(0, 0., 1., true)]));
    let empty = collection(&fixture(&[]));
    assert_eq!(empty.clip_segment(DVec3::ZERO, DVec3::ZERO).unwrap(), None);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for collection in [&c, &empty] {
            assert_eq!(
                collection.clip_segment(p(bad, 0., 0.), DVec3::ZERO),
                Err(QueryError::NonFiniteInput)
            );
        }
    }
    assert_eq!(
        c.clip_segment(p(-f64::MAX, 0.5, 0.5), p(f64::MAX, 0.5, 0.5)),
        Err(QueryError::NonFiniteArithmetic)
    );
    // The first hull hits, but a later source plane overflows: no early successful
    // result is returned that would hide another admitted record's query failure.
    let mut v = fixture(&[(0, 0., 1., true), (1, 0., 1., false)]);
    v["records"][1]["brush"]["sides"][0]["normal_bits"] = json!([f32::MAX.to_bits(), 0, 0]);
    assert_eq!(
        collection(&v).clip_segment(p(0.5, 0.5, 0.5), p(f64::MAX, 0.5, 0.5)),
        Err(QueryError::NonFiniteArithmetic)
    );
}
#[test]
fn zero_side_incomplete_vertices_keep_discrepancy_and_witness_unavailable() {
    let mut v = fixture(&[(0, 0., 1., true)]);
    v["providers"][0]["vertices"] = json!(2);
    v["records"][0]["brush"]["vertex_count"] = json!(2);
    v["records"][0]["brush"]["vertices_bits"] = json!([
        [0, 0, 0],
        [1_f32.to_bits(), 1_f32.to_bits(), 1_f32.to_bits()]
    ]);
    let bytes = serde_json::to_vec(&v).unwrap();
    let s = read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
    let c =
        BrushCollection::from_snapshot(&s, Convention::DiagnosticMinusDistanceAtMostZero).unwrap();
    let d = c.brushes()[0]
        .hull()
        .precision_diagnostics(s.records()[0].snapshot())
        .unwrap();
    assert_eq!(d.reconstructed_vertices, 8);
    assert_eq!(d.vertex_set_max_error, Some(1.0));
    assert_eq!(d.rounding_witness, RoundingWitness::Unavailable);
}
#[test]
#[ignore = "requires BO2_BRUSH_COLLECTION external input"]
fn external_all_8417_hulls_queryable_with_positive_zero_precision_and_repeat_probes() {
    let path = std::env::var_os("BO2_BRUSH_COLLECTION").expect("external input required");
    let mut f = std::fs::File::open(path).expect("external open failed");
    let len = f.metadata().unwrap().len();
    let s = read_brush_collection(&mut f, len).unwrap();
    let c =
        BrushCollection::from_snapshot(&s, Convention::DiagnosticMinusDistanceAtMostZero).unwrap();
    assert_eq!(c.brushes().len(), 8417);
    for (record, owned) in s.records().iter().zip(c.brushes()) {
        assert_eq!(owned.key(), record.key());
        assert_eq!(
            owned.hull().planes().len(),
            record.snapshot().sides().len() + 6
        );
        for (side, plane) in record.snapshot().sides().iter().zip(owned.hull().planes()) {
            assert!(
                plane.normal.to_array().map(f64::to_bits)
                    == side
                        .normal_bits
                        .map(|b| f64::from(f32::from_bits(b)).to_bits())
            );
            assert_eq!(
                plane.distance.to_bits(),
                f64::from(f32::from_bits(side.dist_bits)).to_bits()
            );
        }
        let side_count = record.snapshot().sides().len();
        for axis in 0..3 {
            assert_eq!(
                owned.hull().planes()[side_count + axis * 2]
                    .distance
                    .to_bits(),
                f64::from(f32::from_bits(record.snapshot().maxs_bits()[axis])).to_bits()
            );
            assert_eq!(
                owned.hull().planes()[side_count + axis * 2 + 1]
                    .distance
                    .to_bits(),
                (-f64::from(f32::from_bits(record.snapshot().mins_bits()[axis]))).to_bits()
            );
        }
        let [lo, hi] = owned.hull().endpoints();
        let center = (lo + hi) / 2.;
        let hit = owned.hull().clip_segment(center, center).unwrap();
        assert_eq!(hit.is_some(), owned.hull().contains(center).unwrap());
    }
    let positive = s
        .records()
        .iter()
        .position(|r| r.key().provider_id == 0 && r.key().brush_id == 7)
        .unwrap();
    let d = c.brushes()[positive]
        .hull()
        .precision_diagnostics(s.records()[positive].snapshot())
        .unwrap();
    assert_eq!(d.strict_violations, 8);
    assert_eq!(d.old_tolerant_violations, 8);
    assert!((d.vertex_set_max_error.unwrap() - 0.0005449587606563).abs() < 1e-12);
    assert_eq!(d.rounding_witness, RoundingWitness::NumericallyConstructed);
    let zero = s
        .records()
        .iter()
        .position(|r| r.snapshot().sides().is_empty() && r.snapshot().vertices_bits().len() != 8)
        .unwrap();
    let d = c.brushes()[zero]
        .hull()
        .precision_diagnostics(s.records()[zero].snapshot())
        .unwrap();
    assert_eq!(d.rounding_witness, RoundingWitness::Unavailable);
    assert!(d.vertex_set_max_error.unwrap() > 0.0);
    for index in [positive, zero] {
        let [lo, hi] = c.brushes()[index].hull().endpoints();
        let center = (lo + hi) / 2.;
        let span = hi - lo;
        let local = c.brushes()[index]
            .hull()
            .clip_segment(center - DVec3::X * span.x, center + DVec3::X * span.x)
            .unwrap();
        assert!(local.is_some());
        for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
            let start = center - axis * span;
            let end = center + axis * span;
            let first = c.clip_segment(start, end).unwrap();
            assert!(first.is_some());
            for _ in 0..10 {
                assert_eq!(c.clip_segment(start, end).unwrap(), first);
            }
        }
    }
}
#[test]
fn linear_source_residual_summary_keeps_all_and_zero_side_disagreements() {
    let mut v = fixture(&[(0, 0., 1., true), (1, 0., 1., false)]);
    v["providers"][0]["vertices"] = json!(1);
    for brush in v["records"].as_array_mut().unwrap() {
        brush["brush"]["vertex_count"] = json!(1);
        brush["brush"]["vertices_bits"] = json!([[2_f32.to_bits(), 0, 0]]);
    }
    let bytes = serde_json::to_vec(&v).unwrap();
    let s = read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
    let c = collection(&v);
    let d = c.source_vertex_diagnostics(&s).unwrap();
    assert_eq!(d.all.brushes, 2);
    assert_eq!(d.all.source_vertices, 2);
    assert_eq!(d.all.strict_violations, 3);
    assert_eq!(d.all.old_tolerant_violations, 3);
    assert_eq!(d.all.strict_affected_brushes, 2);
    assert_eq!(d.all.old_tolerant_affected_brushes, 2);
    assert_eq!(d.all.max_positive_residual, 1.0);
    assert_eq!(d.zero_side.brushes, 1);
    assert_eq!(d.zero_side.strict_violations, 1);
    assert_eq!(d.zero_side.max_positive_residual, 1.0);
    v["records"].as_array_mut().unwrap().reverse();
    let bytes = serde_json::to_vec(&v).unwrap();
    let reordered = read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
    assert_eq!(
        c.source_vertex_diagnostics(&reordered),
        Err(QueryError::SnapshotMismatch)
    );
}
