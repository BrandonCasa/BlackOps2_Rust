#![forbid(unsafe_code)]
use bo2_assets::brush_collection::{BrushCollectionSnapshot, read_brush_collection};
use bo2_physics::{
    Vec3,
    moving_brush_trace::{ContactPlane, MovingBrushRequest, TraceEffect, TraceError, TraceResult},
};
use serde_json::{Value, json};

fn fixture(sides: &[(Vec3, f32)]) -> Value {
    let b = f32::to_bits;
    let sides: Vec<_> = sides.iter().enumerate().map(|(i,(n,d))|json!({"plane_index":i,"normal_bits":n.to_array().map(b),"dist_bits":b(*d),"cflags":101+i,"sflags":201+i,"type":3,"signbits":0})).collect();
    json!({"schema":34,"interpretation":"diagnostic_relative_negative_adjacent_zero_empty","source":{"bytes":1,"sha256":"00".repeat(32),"membership_bytes":1,"membership_sha256":"11".repeat(32)},"selection":{"status":"admitted","reached_nodes":1,"keys":[{"provider_id":0,"brush_id":0}]},"coverage":{"models":0,"absent_model_providers":0,"box_excluded":true,"triangles":0,"partitions":0,"aabb_trees":0,"static_models":0,"dynents":[0,0,0,0]},"providers":[{"id":0,"planes":sides.len(),"sides":sides.len(),"vertices":0,"brushes":1,"optional_bounds_present":false,"optional_contents_present":false}],"record_count":1,"records":[{"provider_id":0,"brush":{"schema":34,"brush_index":0,"side_first":0,"vertex_first":0,"side_count":sides.len(),"vertex_count":0,"contents":2147483649_u32,"parallel_bounds_present":false,"parallel_contents_present":false,"parallel_contents":null,"midpoint_bits":null,"halfsize_bits":null,"mins_bits":[b(-2.),b(-2.),b(-2.)],"maxs_bits":[b(2.),b(2.),b(2.)],"axial_cflags":[11,12,13,14,15,16],"axial_sflags":[21,22,23,24,25,26],"sides":sides,"vertices_bits":[]}}]})
}
fn admit(value: &Value) -> BrushCollectionSnapshot {
    let bytes = serde_json::to_vec(value).unwrap();
    read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap()
}
fn initial() -> TraceResult {
    TraceResult {
        normal: Vec3::new(7., 8., 9.),
        fraction: 1.,
        surface_flags: 42,
        contents: 43,
        startsolid: false,
        allsolid: false,
        opaque_flag_2c: 255,
    }
}
fn point(start: Vec3, end: Vec3) -> MovingBrushRequest {
    MovingBrushRequest::new(start, end, Vec3::ZERO, Vec3::ZERO, u32::MAX).unwrap()
}
fn run(sides: &[(Vec3, f32)], start: Vec3, end: Vec3) -> (TraceEffect, TraceResult) {
    let c = admit(&fixture(sides));
    let mut result = initial();
    let effect = point(start, end)
        .trace(c.records()[0].snapshot(), &mut result)
        .unwrap();
    (effect, result)
}
#[test]
fn zero_side_entry_allowance_and_raw_axial_output() {
    let (effect, result) = run(&[], Vec3::new(-4., 0., 0.), Vec3::new(4., 0., 0.));
    assert_eq!(
        effect,
        TraceEffect::Entry(ContactPlane::Axial {
            upper: false,
            axis: 0
        })
    );
    assert_eq!(result.fraction, 0.234375); // geometric diagnostic entry would be 0.25.
    assert_eq!(
        result.normal.to_array().map(f32::to_bits),
        [(-1_f32).to_bits(), 0, 0]
    );
    assert_eq!(result.surface_flags, 21);
    assert_eq!(result.contents, 0x80000001);
    assert!(!result.startsolid && !result.allsolid);
    assert_eq!(result.opaque_flag_2c, 0);
}
#[test]
fn near_outside_can_contact_at_zero_without_startsolid_and_exact_margin_rejects() {
    let (_, r) = run(&[], Vec3::new(-2.0625, 0., 0.), Vec3::ZERO);
    assert_eq!(r.fraction, 0.);
    assert!(!r.startsolid);
    let (effect, r) = run(&[], Vec3::new(-3., 0., 0.), Vec3::new(-2.125, 0., 0.));
    assert_eq!(effect, TraceEffect::Rejected);
    assert_eq!(r, initial());
    assert!(matches!(
        run(&[], Vec3::new(-3., 0., 0.), Vec3::new(-2.0625, 0., 0.)).0,
        TraceEffect::Entry(_)
    ));
    assert_eq!(
        run(&[], Vec3::new(-2.0625, 0., 0.), Vec3::new(-3., 0., 0.)).0,
        TraceEffect::Rejected
    );
}
#[test]
fn start_inside_exit_retains_fraction_normal_and_opaque_with_upper_z_overwrite() {
    let (_, r) = run(&[], Vec3::ZERO, Vec3::new(4., 0., 0.));
    assert_eq!(r.fraction, 1.);
    assert!(r.startsolid);
    assert!(!r.allsolid);
    assert_eq!(r.normal, initial().normal);
    assert_eq!(r.opaque_flag_2c, 255);
    assert_eq!(r.surface_flags, 26);
    assert_eq!(r.contents, 16);
    // Y exits at .5; later side enters at .484375. A symmetric exit allowance
    // would exit at .46875 and incorrectly reject the admitted side contact.
    let (effect, r) = run(
        &[(Vec3::new(-1., 0., 0.), 0.)],
        Vec3::new(-4., 0., 0.),
        Vec3::new(4., 4., 0.),
    );
    assert_eq!(effect, TraceEffect::Entry(ContactPlane::Side { index: 0 }));
    assert_eq!(r.fraction, 0.484375);
}
#[test]
fn moving_allinside_sets_allsolid_even_at_incoming_fraction_zero() {
    let c = admit(&fixture(&[]));
    for fraction in [0., 1.] {
        let mut r = initial();
        r.fraction = fraction;
        assert_eq!(
            point(Vec3::ZERO, Vec3::X)
                .trace(c.records()[0].snapshot(), &mut r)
                .unwrap(),
            TraceEffect::Inside
        );
        assert!(r.startsolid && r.allsolid);
        assert_eq!(r.fraction, 0.);
        assert_eq!(r.contents, 16);
    }
}
#[test]
fn strict_fraction_limit_and_mask_rejections_preserve_every_field() {
    let c = admit(&fixture(&[]));
    for limit in [0., 0.25, 0.46875] {
        let mut r = initial();
        r.fraction = limit;
        let before = r;
        assert_eq!(
            point(Vec3::new(-4., 0., 0.), Vec3::ZERO)
                .trace(c.records()[0].snapshot(), &mut r)
                .unwrap(),
            TraceEffect::Rejected
        );
        assert_eq!(r, before);
    }
    for mask in [0, 2] {
        let q = MovingBrushRequest::new(
            Vec3::new(-4., 0., 0.),
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::ZERO,
            mask,
        )
        .unwrap();
        let mut r = initial();
        assert_eq!(
            q.trace(c.records()[0].snapshot(), &mut r).unwrap(),
            TraceEffect::MaskedOut
        );
        assert_eq!(r, initial());
    }
    let q = MovingBrushRequest::new(
        Vec3::new(-4., 0., 0.),
        Vec3::ZERO,
        Vec3::ZERO,
        Vec3::ZERO,
        0x80000000,
    )
    .unwrap();
    assert!(matches!(
        q.trace(c.records()[0].snapshot(), &mut initial()).unwrap(),
        TraceEffect::Entry(_)
    ));
}
#[test]
fn entry_preserves_inherited_solid_flags() {
    let c = admit(&fixture(&[]));
    let mut r = initial();
    r.startsolid = true;
    r.allsolid = true;
    point(Vec3::new(-4., 0., 0.), Vec3::ZERO)
        .trace(c.records()[0].snapshot(), &mut r)
        .unwrap();
    assert!(r.startsolid && r.allsolid);
}

#[test]
fn parallel_outside_and_late_geometry_rejection_preserve_result() {
    assert_eq!(
        run(&[], Vec3::new(-3.0, 0.0, 0.0), Vec3::new(-3.0, 1.0, 0.0)),
        (TraceEffect::Rejected, initial())
    );
    assert_eq!(
        run(
            &[(Vec3::new(0.0, 1.0, 0.0), -1.0)],
            Vec3::new(-4.0, 0.0, 0.0),
            Vec3::ZERO
        ),
        (TraceEffect::Rejected, initial())
    );
}
#[test]
fn ordered_sides_keep_first_tie_raw_signed_zero_normal_and_surface_flags() {
    let normal = Vec3::new(-1., -0., 0.);
    let (effect, r) = run(
        &[(normal, 0.), (Vec3::new(0., -1., 0.), 0.)],
        Vec3::new(-1., -1., 0.),
        Vec3::new(1., 1., 0.),
    );
    assert_eq!(effect, TraceEffect::Entry(ContactPlane::Side { index: 0 }));
    assert_eq!(r.fraction, 0.4375);
    assert_eq!(
        r.normal.to_array().map(f32::to_bits),
        normal.to_array().map(f32::to_bits)
    );
    assert_eq!(r.surface_flags, 201);
    let (_, scaled) = run(
        &[(Vec3::new(-2., 0., 0.), 0.)],
        Vec3::new(-1., 0., 0.),
        Vec3::X,
    );
    assert_eq!(scaled.fraction, 0.46875);
    assert_eq!(scaled.normal.x, -2.);
}
#[test]
fn axes_precede_sides_and_first_axial_tie_wins() {
    let (effect, r) = run(
        &[(Vec3::new(-1., 0., 0.), 2.)],
        Vec3::new(-4., -4., 0.),
        Vec3::ZERO,
    );
    assert_eq!(
        effect,
        TraceEffect::Entry(ContactPlane::Axial {
            upper: false,
            axis: 0
        })
    );
    assert_eq!(r.surface_flags, 21);
}
#[test]
fn explicit_capsule_support_is_not_a_box_and_y_extent_does_not_change_it() {
    let c = admit(&fixture(&[(Vec3::new(-0.5, 0., -0.5), 0.)]));
    for y in [1., 8.] {
        let q = MovingBrushRequest::new(
            Vec3::new(-3., 0., -3.),
            Vec3::new(1., 0., 1.),
            Vec3::new(-1., -y, -2.),
            Vec3::new(1., y, 2.),
            u32::MAX,
        )
        .unwrap();
        let mut r = initial();
        assert_eq!(
            q.trace(c.records()[0].snapshot(), &mut r).unwrap(),
            TraceEffect::Entry(ContactPlane::Side { index: 0 })
        );
        assert_eq!(r.fraction, 0.34375);
    }
    let q = MovingBrushRequest::new(
        Vec3::new(-4., 0., -2.),
        Vec3::new(0., 0., -2.),
        Vec3::new(-1., -1., 0.),
        Vec3::new(1., 1., 4.),
        u32::MAX,
    )
    .unwrap();
    assert_eq!(q.centered_endpoints(), [Vec3::new(-4., 0., 0.), Vec3::ZERO]);
    let c = admit(&fixture(&[]));
    let mut r = initial();
    q.trace(c.records()[0].snapshot(), &mut r).unwrap();
    assert_eq!(r.fraction, 0.21875);
}
#[test]
fn input_admission_stationary_and_constructor_overflow_are_typed() {
    assert_eq!(
        MovingBrushRequest::new(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, 1).unwrap_err(),
        TraceError::UnsupportedStationary
    );
    // Distinct caller endpoints become equal after float32 centering.
    assert_eq!(
        MovingBrushRequest::new(Vec3::ZERO, Vec3::X, Vec3::splat(1e20), Vec3::splat(1e20), 1)
            .unwrap_err(),
        TraceError::UnsupportedStationary
    );
    assert_eq!(
        MovingBrushRequest::new(Vec3::ZERO, Vec3::X, Vec3::ONE, Vec3::ZERO, 1).unwrap_err(),
        TraceError::InvalidExtents
    );
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            MovingBrushRequest::new(Vec3::splat(value), Vec3::X, Vec3::ZERO, Vec3::ZERO, 1)
                .unwrap_err(),
            TraceError::NonFiniteInput
        );
    }
    assert_eq!(
        MovingBrushRequest::new(
            Vec3::splat(-f32::MAX),
            Vec3::splat(f32::MAX),
            Vec3::ZERO,
            Vec3::ZERO,
            1
        )
        .unwrap_err(),
        TraceError::NonFiniteArithmetic
    );
    assert_eq!(
        MovingBrushRequest::new(
            Vec3::ZERO,
            Vec3::X,
            Vec3::splat(f32::MAX),
            Vec3::splat(f32::MAX),
            1
        )
        .unwrap_err(),
        TraceError::NonFiniteArithmetic
    );
    assert_eq!(
        MovingBrushRequest::new(
            Vec3::ZERO,
            Vec3::new(f32::from_bits(1), 0., 0.),
            Vec3::ZERO,
            Vec3::ZERO,
            1
        )
        .unwrap_err(),
        TraceError::NonFiniteArithmetic
    );
}
#[test]
fn late_side_arithmetic_failure_is_transactional_and_invalid_results_reject() {
    let c = admit(&fixture(&[(Vec3::splat(f32::MAX), 0.)]));
    let mut r = initial();
    assert_eq!(
        point(Vec3::new(-4., 1.5, 1.5), Vec3::new(0., 1.5, 1.5))
            .trace(c.records()[0].snapshot(), &mut r),
        Err(TraceError::NonFiniteArithmetic)
    );
    assert_eq!(r, initial());
    let c = admit(&fixture(&[]));
    for fraction in [-0.1, 1.1] {
        let mut r = initial();
        r.fraction = fraction;
        let before = r;
        assert_eq!(
            point(Vec3::ZERO, Vec3::X).trace(c.records()[0].snapshot(), &mut r),
            Err(TraceError::InvalidFraction)
        );
        assert_eq!(r, before);
    }
    let mut r = initial();
    r.normal.x = f32::INFINITY;
    assert_eq!(
        point(Vec3::ZERO, Vec3::X).trace(c.records()[0].snapshot(), &mut r),
        Err(TraceError::NonFiniteInput)
    );
    assert_eq!(r.normal.x, f32::INFINITY);
}

#[test]
#[ignore = "requires BO2_BRUSH_COLLECTION retained external input"]
fn external_preserved_words_residuals_and_representative_reconstruction() {
    use bo2_physics::{brush::Convention, brush_collection::BrushCollection};
    let mut file = std::fs::File::open(
        std::env::var_os("BO2_BRUSH_COLLECTION").expect("external input required"),
    )
    .unwrap();
    let len = file.metadata().unwrap().len();
    let c = read_brush_collection(&mut file, len).unwrap();
    assert_eq!(c.records().len(), 8417);
    let diagnostics =
        BrushCollection::from_snapshot(&c, Convention::DiagnosticMinusDistanceAtMostZero).unwrap();
    let before = diagnostics.source_vertex_diagnostics(&c).unwrap();
    assert_eq!(before.all.strict_violations, 30706);
    assert_eq!(before.all.strict_affected_brushes, 4186);
    assert_eq!(before.all.max_positive_residual, 0.09950868785381317);
    let raw: Vec<_> = c
        .records()
        .iter()
        .map(|r| {
            let s = r.snapshot();
            (
                s.mins_bits(),
                s.maxs_bits(),
                s.contents(),
                *s.axial_cflags(),
                *s.axial_sflags(),
                s.sides().to_vec(),
                s.vertices_bits().to_vec(),
            )
        })
        .collect();
    let positive = c
        .records()
        .iter()
        .position(|r| !r.snapshot().sides().is_empty())
        .unwrap();
    let zero = c
        .records()
        .iter()
        .position(|r| r.snapshot().sides().is_empty() && r.snapshot().vertices_bits().len() != 8)
        .unwrap();
    for index in [positive, zero] {
        let s = c.records()[index].snapshot();
        let lo = s.mins_bits().map(f32::from_bits);
        let hi = s.maxs_bits().map(f32::from_bits);
        let center = std::array::from_fn::<_, 3, _>(|a| (lo[a] + hi[a]) * 0.5);
        for axis in 0..3 {
            let mut start = center;
            let mut end = center;
            let span = hi[axis] - lo[axis];
            start[axis] -= span;
            end[axis] += span;
            let q = point(Vec3::from_array(start), Vec3::from_array(end));
            let mut result = initial();
            let effect = q.trace(s, &mut result).unwrap();
            for _ in 0..10 {
                let mut again = initial();
                assert_eq!(q.trace(s, &mut again).unwrap(), effect);
                assert_eq!(again, result);
            }
            println!(
                "MOVING_REPORT:{}",
                json!({"brush_id":s.brush_index(),"axis":axis,"start":start,"end":end,"effect":format!("{effect:?}"),"fraction_bits":result.fraction.to_bits(),"normal_bits":result.normal.to_array().map(f32::to_bits),"surface_flags":result.surface_flags,"contents":result.contents,"startsolid":result.startsolid,"allsolid":result.allsolid,"opaque":result.opaque_flag_2c})
            );
        }
    }
    for (r, before) in c.records().iter().zip(raw) {
        let s = r.snapshot();
        assert!(
            before
                == (
                    s.mins_bits(),
                    s.maxs_bits(),
                    s.contents(),
                    *s.axial_cflags(),
                    *s.axial_sflags(),
                    s.sides().to_vec(),
                    s.vertices_bits().to_vec()
                )
        );
    }
    assert_eq!(
        diagnostics
            .source_vertex_diagnostics(&c)
            .unwrap()
            .all
            .strict_violations,
        before.all.strict_violations
    );
}
