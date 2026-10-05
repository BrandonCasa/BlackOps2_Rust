#![forbid(unsafe_code)]
use bo2_assets::brush_snapshot::{BrushSnapshot, read_brush_snapshot};
use bo2_physics::{
    DVec3,
    brush::{Convention, ConvexBrush, QueryError, RoundingWitness, SegmentHit, rounding_cell},
};
use serde_json::{Value, json};

fn fixture(wedge: bool, scale: f32, maximum: f32, distance: f32) -> Value {
    let b = |x: f32| x.to_bits();
    let mut vertices = Vec::new();
    for x in [0.0, maximum] {
        for y in [0.0, 1.0] {
            for z in [0.0, 1.0] {
                if !wedge || x + y <= 1.0 {
                    vertices.push([b(x), b(y), b(z)]);
                }
            }
        }
    }
    json!({"schema":28,"brush_index":1,"side_first":0,"vertex_first":0,
        "side_count":1,"vertex_count":vertices.len(),"contents":0,
        "parallel_bounds_present":false,"parallel_contents_present":false,
        "parallel_contents":null,"midpoint_bits":null,"halfsize_bits":null,
        "mins_bits":[0,0,0],"maxs_bits":[b(maximum),b(1.0),b(1.0)],
        "axial_cflags":[0,0,0,0,0,0],"axial_sflags":[0,0,0,0,0,0],
        "sides":[{"plane_index":0,"cflags":0,"sflags":0,"normal_bits":[b(scale),b(if wedge {scale} else {0.0}),0],"dist_bits":b(distance),"type":0,"signbits":0}],
        "vertices_bits":vertices})
}
fn snapshot(v: &Value) -> BrushSnapshot {
    let b = serde_json::to_vec(v).unwrap();
    read_brush_snapshot(&mut b.as_slice(), b.len() as u64).unwrap()
}
fn hull(v: &Value) -> ConvexBrush {
    ConvexBrush::from_snapshot(&snapshot(v), Convention::DiagnosticMinusDistanceAtMostZero).unwrap()
}
fn point(x: f64, y: f64, z: f64) -> DVec3 {
    DVec3::new(x, y, z)
}
#[test]
fn box_crossing_start_inside_exit_reverse_and_stationary() {
    let h = hull(&fixture(false, 1.0, 1.0, 1.0));
    let a = point(-1.0, 0.5, 0.5);
    let b = point(2.0, 0.5, 0.5);
    let expected = Some(SegmentHit {
        entering: 1.0 / 3.0,
        exiting: 2.0 / 3.0,
        start_inside: false,
    });
    assert_eq!(h.clip_segment(a, b).unwrap(), expected);
    assert_eq!(h.clip_segment(b, a).unwrap(), expected);
    for _ in 0..100 {
        assert_eq!(h.clip_segment(a, b).unwrap(), expected);
    }
    assert_eq!(
        h.clip_segment(point(0.5, 0.5, 0.5), point(1.5, 0.5, 0.5))
            .unwrap(),
        Some(SegmentHit {
            entering: 0.0,
            exiting: 0.5,
            start_inside: true
        })
    );
    assert_eq!(
        h.clip_segment(point(0.5, 0.5, 0.5), point(0.5, 0.5, 0.5))
            .unwrap(),
        Some(SegmentHit {
            entering: 0.0,
            exiting: 1.0,
            start_inside: true
        })
    );
    assert_eq!(h.clip_segment(a, a).unwrap(), None);
    assert!(h.contains(DVec3::ZERO).unwrap());
    assert!(!h.contains(a).unwrap());
}
#[test]
fn wedge_tangent_parallel_opposing_and_boundary_cases() {
    let h = hull(&fixture(true, 1.0, 1.0, 1.0));
    assert_eq!(
        h.clip_segment(point(-1.0, 0.25, 0.5), point(2.0, 0.25, 0.5))
            .unwrap(),
        Some(SegmentHit {
            entering: 1.0 / 3.0,
            exiting: 7.0 / 12.0,
            start_inside: false
        })
    );
    assert_eq!(
        h.clip_segment(point(0.5, -0.5, 0.5), point(1.5, 0.5, 0.5))
            .unwrap(),
        Some(SegmentHit {
            entering: 0.5,
            exiting: 0.5,
            start_inside: false
        })
    );
    assert_eq!(
        h.clip_segment(point(0.25, 0.75, 0.0), point(0.75, 0.25, 1.0))
            .unwrap(),
        Some(SegmentHit {
            entering: 0.0,
            exiting: 1.0,
            start_inside: true
        })
    );
    assert_eq!(
        h.clip_segment(point(0.5, 0.6, 0.0), point(0.5, 0.6, 1.0))
            .unwrap(),
        None
    );
    assert_eq!(
        h.clip_segment(point(-1.0, 2.0, 0.5), point(2.0, 2.0, 0.5))
            .unwrap(),
        None
    );
    assert_eq!(
        h.clip_segment(point(-1.0, 0.0, 0.5), point(0.0, 0.0, 0.5))
            .unwrap()
            .unwrap()
            .entering,
        1.0
    );
}
#[test]
fn rejects_nonfinite_points_and_arithmetic_overflow_without_panics() {
    let h = hull(&fixture(false, 1.0, 1.0, 1.0));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            h.contains(point(bad, 0.0, 0.0)),
            Err(QueryError::NonFiniteInput)
        );
        assert_eq!(
            h.clip_segment(DVec3::ZERO, point(0.0, bad, 0.0)),
            Err(QueryError::NonFiniteInput)
        );
    }
    assert_eq!(
        h.clip_segment(point(-f64::MAX, 0.5, 0.5), point(f64::MAX, 0.5, 0.5)),
        Err(QueryError::NonFiniteArithmetic)
    );
}
#[test]
fn plane_order_is_preserved_queries_repeat_and_empty_intersection_is_no_hit() {
    let mut v = fixture(false, 1.0, 1.0, 1.0);
    let mut opposing = v["sides"][0].clone();
    opposing["normal_bits"] = json!([(-1_f32).to_bits(), 0, 0]);
    opposing["dist_bits"] = json!((-2_f32).to_bits());
    v["sides"].as_array_mut().unwrap().push(opposing);
    v["side_count"] = json!(2);
    let h = hull(&v);
    assert_eq!(h.planes()[0].normal, DVec3::X);
    assert_eq!(h.planes()[1].normal, -DVec3::X);
    assert_eq!(
        h.clip_segment(point(-1.0, 0.5, 0.5), point(3.0, 0.5, 0.5))
            .unwrap(),
        None
    );
    let mut reordered = v.clone();
    reordered["sides"].as_array_mut().unwrap().reverse();
    assert_eq!(
        hull(&reordered)
            .clip_segment(point(-1.0, 0.5, 0.5), point(3.0, 0.5, 0.5))
            .unwrap(),
        None
    );
}
#[test]
fn precision_witness_accounts_for_roundoff_but_not_clear_semantic_disagreement() {
    let v = fixture(false, 0.1, 7.0, 0.7);
    let s = snapshot(&v);
    let h = hull(&v);
    let d = h.precision_diagnostics(&s).unwrap();
    assert!(d.strict_violations > 0);
    assert!(d.old_tolerant_violations > 0);
    assert!(d.vertex_set_max_error.unwrap() > 0.0);
    assert_eq!(
        d.rounding_witness,
        RoundingWitness::NumericallyConstructed,
        "{d:?}"
    );
    let bad = fixture(false, 0.1, 7.0, 0.6);
    let d = hull(&bad).precision_diagnostics(&snapshot(&bad)).unwrap();
    assert_eq!(d.rounding_witness, RoundingWitness::NotFound);
    assert_eq!(
        h.precision_diagnostics(&snapshot(&bad)),
        Err(QueryError::SnapshotMismatch)
    );
    let mut no_vertices = v.clone();
    no_vertices["vertices_bits"] = json!([]);
    no_vertices["vertex_count"] = json!(0);
    assert_eq!(
        hull(&no_vertices)
            .precision_diagnostics(&snapshot(&no_vertices))
            .unwrap()
            .rounding_witness,
        RoundingWitness::Unavailable
    );
}
#[test]
fn rounding_cells_follow_asymmetric_ulp_spacing_and_finite_extremes() {
    assert_eq!(
        rounding_cell(1_f32.to_bits()).unwrap(),
        [1.0 - 2_f64.powi(-25), 1.0 + 2_f64.powi(-24)]
    );
    assert_eq!(
        rounding_cell((-1_f32).to_bits()).unwrap(),
        [-1.0 - 2_f64.powi(-24), -1.0 + 2_f64.powi(-25)]
    );
    assert_eq!(
        rounding_cell(0).unwrap(),
        [-2_f64.powi(-150), 2_f64.powi(-150)]
    );
    assert!(
        rounding_cell(f32::MAX.to_bits())
            .unwrap()
            .iter()
            .all(|x| x.is_finite())
    );
    assert_eq!(
        rounding_cell(f32::NAN.to_bits()),
        Err(QueryError::NonFiniteInput)
    );
}
