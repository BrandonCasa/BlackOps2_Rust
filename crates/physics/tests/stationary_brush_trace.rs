#![forbid(unsafe_code)]
use bo2_assets::brush_collection::{BrushCollectionSnapshot, read_brush_collection};
use bo2_physics::{
    Vec3,
    moving_brush_trace::{TraceEffect, TraceError, TraceResult},
    stationary_brush_trace::StationaryBrushRequest,
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
        normal: Vec3::new(-0.0, 8., 9.),
        fraction: 1.,
        surface_flags: 42,
        contents: 43,
        startsolid: false,
        allsolid: false,
        opaque_flag_2c: 255,
    }
}

fn point(origin: Vec3) -> StationaryBrushRequest {
    StationaryBrushRequest::new(origin, Vec3::ZERO, Vec3::ZERO, u32::MAX).unwrap()
}

#[test]
fn zero_side_overlap_writes_only_qualified_stationary_contact_fields() {
    let collection = admit(&fixture(&[]));
    let mut result = initial();
    result.fraction = -0.0;
    assert_eq!(
        point(Vec3::ZERO)
            .trace(collection.records()[0].snapshot(), &mut result)
            .unwrap(),
        TraceEffect::Inside
    );
    assert_eq!(result.fraction.to_bits(), 0);
    assert!(result.startsolid && result.allsolid);
    assert_eq!(result.contents, 0x80000001);
    assert_eq!(result.surface_flags, 0);
    assert_eq!(
        result.normal.to_array().map(f32::to_bits),
        initial().normal.to_array().map(f32::to_bits)
    );
    assert_eq!(result.opaque_flag_2c, 255);
}

#[test]
fn all_six_axial_tangencies_and_outside_contacts_reject_without_moving_allowance() {
    let collection = admit(&fixture(&[]));
    for axis in 0..3 {
        for sign in [-1., 1.] {
            for distance in [2., 2.0625, 3.] {
                let mut origin = Vec3::ZERO;
                origin[axis] = sign * distance;
                let mut result = initial();
                assert_eq!(
                    point(origin)
                        .trace(collection.records()[0].snapshot(), &mut result)
                        .unwrap(),
                    TraceEffect::Rejected
                );
                assert_eq!(result, initial());
            }
            let mut origin = Vec3::ZERO;
            origin[axis] = sign * 1.9375;
            assert_eq!(
                point(origin)
                    .trace(collection.records()[0].snapshot(), &mut initial())
                    .unwrap(),
                TraceEffect::Inside
            );
        }
    }
}

#[test]
fn capsule_axial_tangency_rejects_and_nonzero_overlap_admits() {
    let collection = admit(&fixture(&[]));
    for x in [3., 2.9375] {
        let query =
            StationaryBrushRequest::new(Vec3::new(x, 0., 0.), Vec3::splat(-1.), Vec3::ONE, 1)
                .unwrap();
        let mut result = initial();
        let effect = query
            .trace(collection.records()[0].snapshot(), &mut result)
            .unwrap();
        assert_eq!(
            effect,
            if x == 3. {
                TraceEffect::Rejected
            } else {
                TraceEffect::Inside
            }
        );
        if x == 3. {
            assert_eq!(result, initial());
        }
    }
}

#[test]
fn side_boundary_is_inclusive_and_late_side_rejection_is_atomic() {
    let collection = admit(&fixture(&[(Vec3::X, 1.)]));
    for (x, expected) in [(1., TraceEffect::Inside), (1.0625, TraceEffect::Rejected)] {
        let mut result = initial();
        assert_eq!(
            point(Vec3::new(x, 0., 0.))
                .trace(collection.records()[0].snapshot(), &mut result)
                .unwrap(),
            expected
        );
        if expected == TraceEffect::Rejected {
            assert_eq!(result, initial());
        }
    }
    let collection = admit(&fixture(&[(Vec3::X, 1.), (Vec3::Y, -1.)]));
    let mut result = initial();
    assert_eq!(
        point(Vec3::ZERO)
            .trace(collection.records()[0].snapshot(), &mut result)
            .unwrap(),
        TraceEffect::Rejected
    );
    assert_eq!(result, initial());
}

#[test]
fn ordered_scalar_side_sum_keeps_cancellation_before_z_and_raw_scale() {
    // Native ((Y+X)+Z) is 1; reassociating X+(Y+Z) would become 0.
    let collection = admit(&fixture(&[(Vec3::new(1e20, -1e20, 1.), 0.)]));
    assert_eq!(
        point(Vec3::ONE)
            .trace(collection.records()[0].snapshot(), &mut initial())
            .unwrap(),
        TraceEffect::Rejected
    );
    let collection = admit(&fixture(&[(Vec3::new(2., 0., 0.), 1.)]));
    assert_eq!(
        point(Vec3::new(0.5, 0., 0.))
            .trace(collection.records()[0].snapshot(), &mut initial())
            .unwrap(),
        TraceEffect::Inside
    );
    assert_eq!(
        point(Vec3::new(0.5625, 0., 0.))
            .trace(collection.records()[0].snapshot(), &mut initial())
            .unwrap(),
        TraceEffect::Rejected
    );
}

#[test]
fn capsule_support_ignores_y_half_extent_and_centers_asymmetric_bounds() {
    let collection = admit(&fixture(&[(Vec3::Z, 0.)]));
    for y in [1., 8.] {
        let query = StationaryBrushRequest::new(
            Vec3::new(0., 0., 3.),
            Vec3::new(-1., -y, -3.),
            Vec3::new(1., y, 3.),
            1,
        )
        .unwrap();
        assert_eq!(
            query
                .trace(collection.records()[0].snapshot(), &mut initial())
                .unwrap(),
            TraceEffect::Inside
        );
    }
    let query = StationaryBrushRequest::new(
        Vec3::new(-1., 0., -2.),
        Vec3::new(0., -1., 0.),
        Vec3::new(2., 1., 4.),
        1,
    )
    .unwrap();
    assert_eq!(query.centered_origin(), Vec3::ZERO);
    let collection = admit(&fixture(&[]));
    assert_eq!(
        query
            .trace(collection.records()[0].snapshot(), &mut initial())
            .unwrap(),
        TraceEffect::Inside
    );
}

#[test]
fn filtered_results_stay_unchanged_and_high_bit_mask_participates() {
    let collection = admit(&fixture(&[]));
    for (mask, expected) in [
        (0, TraceEffect::MaskedOut),
        (2, TraceEffect::MaskedOut),
        (0x80000000, TraceEffect::Inside),
    ] {
        let query = StationaryBrushRequest::new(Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, mask).unwrap();
        let mut result = initial();
        assert_eq!(
            query
                .trace(collection.records()[0].snapshot(), &mut result)
                .unwrap(),
            expected
        );
        if expected == TraceEffect::MaskedOut {
            assert_eq!(result, initial());
        }
    }
}

#[test]
fn invalid_request_inputs_and_constructor_overflows_are_typed() {
    assert_eq!(
        StationaryBrushRequest::new(Vec3::ZERO, Vec3::ONE, Vec3::ZERO, 1).unwrap_err(),
        TraceError::InvalidExtents
    );
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for field in 0..3 {
            let mut inputs = [Vec3::ZERO; 3];
            inputs[field].x = value;
            assert_eq!(
                StationaryBrushRequest::new(inputs[0], inputs[1], inputs[2], 1).unwrap_err(),
                TraceError::NonFiniteInput
            );
        }
    }
    assert_eq!(
        StationaryBrushRequest::new(Vec3::ZERO, Vec3::splat(f32::MAX), Vec3::splat(f32::MAX), 1)
            .unwrap_err(),
        TraceError::NonFiniteArithmetic
    );
    assert_eq!(
        StationaryBrushRequest::new(Vec3::splat(f32::MAX), Vec3::ZERO, Vec3::splat(f32::MAX), 1)
            .unwrap_err(),
        TraceError::NonFiniteArithmetic
    );
    assert_eq!(
        StationaryBrushRequest::new(
            Vec3::splat(f32::MAX),
            Vec3::splat(-f32::MAX),
            Vec3::splat(f32::MAX),
            1
        )
        .unwrap_err(),
        TraceError::NonFiniteArithmetic
    );
}

#[test]
fn late_arithmetic_failure_and_invalid_results_leave_all_fields_unchanged() {
    let collection = admit(&fixture(&[(Vec3::X, 1.), (Vec3::splat(f32::MAX), 0.)]));
    let mut result = initial();
    assert_eq!(
        point(Vec3::ONE).trace(collection.records()[0].snapshot(), &mut result),
        Err(TraceError::NonFiniteArithmetic)
    );
    assert_eq!(result, initial());
    let collection = admit(&fixture(&[]));
    for fraction in [-0.1, 1.1] {
        let mut result = initial();
        result.fraction = fraction;
        let before = result;
        assert_eq!(
            point(Vec3::ZERO).trace(collection.records()[0].snapshot(), &mut result),
            Err(TraceError::InvalidFraction)
        );
        assert_eq!(result, before);
    }
    for value in [f32::NAN, f32::INFINITY] {
        let mut result = initial();
        result.fraction = value;
        let before = result.fraction.to_bits();
        assert_eq!(
            point(Vec3::ZERO).trace(collection.records()[0].snapshot(), &mut result),
            Err(TraceError::NonFiniteInput)
        );
        assert_eq!(result.fraction.to_bits(), before);
    }
    let mut result = initial();
    result.normal.x = f32::INFINITY;
    let before = result;
    assert_eq!(
        point(Vec3::ZERO).trace(collection.records()[0].snapshot(), &mut result),
        Err(TraceError::NonFiniteInput)
    );
    assert_eq!(result, before);
}

#[test]
#[ignore = "requires retained external brush collection in BO2_BRUSH_COLLECTION"]
fn external_zero_and_positive_side_brushes_keep_deterministic_contact_and_payload_words() {
    let mut file = std::fs::File::open(
        std::env::var_os("BO2_BRUSH_COLLECTION").expect("external input required"),
    )
    .unwrap();
    let len = file.metadata().unwrap().len();
    let collection = read_brush_collection(&mut file, len).unwrap();
    assert_eq!(collection.records().len(), 8417);
    for zero_side in [true, false] {
        let record = collection
            .records()
            .iter()
            .find(|r| r.snapshot().sides().is_empty() == zero_side)
            .unwrap();
        let brush = record.snapshot();
        let lo = brush.mins_bits().map(f32::from_bits);
        let hi = brush.maxs_bits().map(f32::from_bits);
        let origin = Vec3::from_array(std::array::from_fn(|i| (lo[i] + hi[i]) * 0.5));
        let before = (
            brush.mins_bits(),
            brush.maxs_bits(),
            brush.sides().to_vec(),
            brush.vertices_bits().to_vec(),
        );
        let query = point(origin);
        let mut first = initial();
        let effect = query.trace(brush, &mut first).unwrap();
        if zero_side {
            assert_eq!(effect, TraceEffect::Inside);
        }
        for _ in 0..10 {
            let mut again = initial();
            assert_eq!(query.trace(brush, &mut again).unwrap(), effect);
            assert_eq!(again, first);
        }
        assert!(
            before
                == (
                    brush.mins_bits(),
                    brush.maxs_bits(),
                    brush.sides().to_vec(),
                    brush.vertices_bits().to_vec()
                )
        );
    }
}
