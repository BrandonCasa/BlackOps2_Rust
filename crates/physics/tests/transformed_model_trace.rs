#![forbid(unsafe_code)]
use bo2_physics::{
    Vec3,
    moving_brush_trace::{TraceError, TraceResult},
    transformed_model_trace::{ModelTransform, TransformedModelError, TransformedModelRequest},
};
const IDENTITY: [[f32; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
const YAW: [[f32; 3]; 3] = [[0., 1., 0.], [-1., 0., 0.], [0., 0., 1.]];
fn seed(fraction: f32) -> TraceResult {
    TraceResult {
        normal: Vec3::new(2., 3., 4.),
        fraction,
        surface_flags: 7,
        contents: 9,
        startsolid: true,
        allsolid: false,
        opaque_flag_2c: 23,
    }
}
fn request(transform: ModelTransform) -> TransformedModelRequest {
    TransformedModelRequest::new(
        Vec3::new(10., 20., 30.),
        Vec3::new(11., 21., 31.),
        Vec3::new(-1., -2., 0.),
        Vec3::new(1., 2., 6.),
        264,
        0xfedcba98,
        transform,
    )
    .unwrap()
}
fn bits(v: Vec3) -> [u32; 3] {
    v.to_array().map(f32::to_bits)
}

#[test]
fn asymmetric_hull_translation_preserves_query_and_contact_identity() {
    let q = request(ModelTransform::Translation {
        origin: Vec3::new(4., 5., 6.),
    });
    let mut out = seed(1.);
    let contact = q
        .trace_with(&mut out, |local, staged| {
            assert_eq!(local.start, Vec3::new(6., 15., 27.));
            assert_eq!(local.end, Vec3::new(7., 16., 28.));
            assert_eq!(local.mins, Vec3::new(-1., -2., -3.));
            assert_eq!(local.maxs, Vec3::new(1., 2., 3.));
            assert_eq!(local.selector, 264);
            assert_eq!(local.mask, 0xfedcba98);
            staged.fraction = 0.25;
            staged.normal = Vec3::X;
            Ok::<_, &str>((264_u32, 71_u32))
        })
        .unwrap();
    assert_eq!(contact, (264, 71));
    assert_eq!(out.normal, Vec3::X);
    assert_eq!(out.opaque_flag_2c, 23);
}

#[test]
fn matrix_rows_rotate_endpoints_without_rotating_hulls_and_transpose_normal() {
    let q = request(ModelTransform::Matrix {
        origin: Vec3::new(4., 5., 6.),
        rows: YAW,
    });
    let mut out = seed(1.);
    q.trace_with(&mut out, |local, staged| {
        assert_eq!(local.start, Vec3::new(15., -6., 27.));
        assert_eq!(local.end, Vec3::new(16., -7., 28.));
        assert_eq!(local.mins, Vec3::new(-1., -2., -3.));
        staged.fraction = 0.5;
        staged.normal = Vec3::X;
        staged.opaque_flag_2c = 91;
        Ok::<_, &str>(())
    })
    .unwrap();
    assert_eq!(out.normal, Vec3::Y);
    assert_eq!(out.opaque_flag_2c, 91);
}

#[test]
fn independent_nonaxis_f32_source_fixture() {
    // Expected words: independent external Python equations with all input words
    // rounded to f32 before separate products/additions; no native invocation.
    let q = TransformedModelRequest::new(
        Vec3::new(0.81, -0.37, 0.69),
        Vec3::new(0.81, -0.37, 0.69),
        Vec3::new(-0.31, -0.17, -0.42),
        Vec3::new(0.23, 0.51, 0.82),
        41,
        3,
        ModelTransform::Matrix {
            origin: Vec3::new(0.27, -0.13, 0.09),
            rows: [[0.7, 0.2, -0.1], [-0.3, 0.8, 0.15], [0.1, -0.25, 0.9]],
        },
    )
    .unwrap();
    let mut out = seed(1.);
    q.trace_with(&mut out, |local, staged| {
        assert_eq!(bits(local.start), [1048777326, 3182436551, 1061788056]);
        assert_eq!(bits(local.end), bits(local.start));
        assert_eq!(bits(local.mins), [3196730737, 3199079546, 3206461522]);
        assert_eq!(bits(local.maxs), [1049247089, 1051595899, 1058977874]);
        staged.fraction = 0.;
        staged.normal = Vec3::new(0.37, -0.81, 0.19);
        Ok::<_, &str>(())
    })
    .unwrap();
    assert_eq!(bits(out.normal), [1057316930, 3206486688, 1011666096]);
}

#[test]
fn addition_precedes_origin_subtraction_in_source_order() {
    let q = TransformedModelRequest::new(
        Vec3::splat(16777216.),
        Vec3::splat(16777216.),
        Vec3::ONE,
        Vec3::ONE,
        1,
        1,
        ModelTransform::Translation {
            origin: Vec3::splat(16777216.),
        },
    )
    .unwrap();
    q.trace_with(&mut seed(1.), |local, _| {
        assert_eq!(bits(local.start), [0; 3]);
        assert_eq!(local.mins, Vec3::ZERO);
        assert_eq!(local.maxs, Vec3::ZERO);
        Ok::<_, &str>(())
    })
    .unwrap();
}

#[test]
fn nonunit_matrix_and_inside_contact_retain_native_normal_behavior() {
    let q = request(ModelTransform::Matrix {
        origin: Vec3::ZERO,
        rows: [[2., 0., 0.], [0., 3., 0.], [0., 0., 4.]],
    });
    let mut out = seed(1.);
    q.trace_with(&mut out, |_, staged| {
        staged.fraction = 0.;
        staged.allsolid = true;
        // Stationary inside consumer need not write a normal.
        Ok::<_, &str>(())
    })
    .unwrap();
    assert_eq!(out.normal, Vec3::new(4., 9., 16.));
    assert!(out.allsolid && out.startsolid);
    assert_eq!(out.opaque_flag_2c, 23);
}

#[test]
fn ties_and_zero_fraction_do_not_promote_but_publish_callback_writes() {
    for fraction in [0., 0.5, 1.] {
        let mut out = seed(fraction);
        request(ModelTransform::Matrix {
            origin: Vec3::ZERO,
            rows: YAW,
        })
        .trace_with(&mut out, |_, staged| {
            staged.normal = Vec3::X;
            staged.contents = 44;
            Ok::<_, &str>(())
        })
        .unwrap();
        assert_eq!(out.normal, Vec3::X);
        assert_eq!(out.contents, 44);
    }
}

#[test]
fn translation_and_identity_matrix_have_distinct_signed_zero_behavior() {
    for (pose, wanted) in [
        (
            ModelTransform::Translation { origin: Vec3::ZERO },
            [0x80000000, 0, 0],
        ),
        (
            ModelTransform::Matrix {
                origin: Vec3::ZERO,
                rows: IDENTITY,
            },
            [0; 3],
        ),
    ] {
        let mut out = seed(1.);
        request(pose)
            .trace_with(&mut out, |_, staged| {
                staged.fraction = 0.5;
                staged.normal = Vec3::new(-0.0, 0.0, 0.0);
                Ok::<_, &str>(())
            })
            .unwrap();
        assert_eq!(bits(out.normal), wanted);
    }
}

#[test]
fn input_and_early_arithmetic_failures_never_call_query() {
    for (start, lo, hi, pose, wanted) in [
        (
            Vec3::NAN,
            Vec3::ZERO,
            Vec3::ONE,
            ModelTransform::Translation { origin: Vec3::ZERO },
            TraceError::NonFiniteInput,
        ),
        (
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::ZERO,
            ModelTransform::Translation { origin: Vec3::ZERO },
            TraceError::InvalidExtents,
        ),
        (
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::ONE,
            ModelTransform::Matrix {
                origin: Vec3::ZERO,
                rows: [[f32::NAN; 3]; 3],
            },
            TraceError::NonFiniteInput,
        ),
    ] {
        assert_eq!(
            TransformedModelRequest::new(start, Vec3::ZERO, lo, hi, 0, 0, pose).unwrap_err(),
            wanted
        );
    }
    for q in [
        TransformedModelRequest::new(
            Vec3::splat(f32::MAX),
            Vec3::ZERO,
            Vec3::ONE,
            Vec3::ONE,
            0,
            0,
            ModelTransform::Translation {
                origin: Vec3::splat(-f32::MAX),
            },
        )
        .unwrap(),
        request(ModelTransform::Matrix {
            origin: Vec3::ZERO,
            rows: [[f32::MAX; 3]; 3],
        }),
    ] {
        let mut out = seed(1.);
        let before = out;
        assert_eq!(
            q.trace_with(&mut out, |_, _| -> Result<(), &str> {
                panic!("invalid arithmetic reached query")
            }),
            Err(TransformedModelError::Trace(
                TraceError::NonFiniteArithmetic
            ))
        );
        assert_eq!(out, before);
    }
}

#[test]
fn callback_errors_invalid_results_and_late_promotion_are_atomic() {
    let q = request(ModelTransform::Matrix {
        origin: Vec3::ZERO,
        rows: [[2., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    });
    let mut out = seed(1.);
    let before = out;
    assert_eq!(
        q.trace_with(&mut out, |_, staged| {
            staged.fraction = 0.;
            staged.contents = 19;
            Err::<(), _>("missing-provider")
        }),
        Err(TransformedModelError::Query("missing-provider"))
    );
    assert_eq!(out, before);
    assert_eq!(
        q.trace_with(&mut out, |_, staged| {
            staged.fraction = 0.5;
            staged.normal = Vec3::new(f32::MAX, 0., 0.);
            staged.surface_flags = 21;
            Ok::<_, &str>(())
        }),
        Err(TransformedModelError::Trace(
            TraceError::NonFiniteArithmetic
        ))
    );
    assert_eq!(out, before);
    assert_eq!(
        q.trace_with(&mut out, |_, staged| {
            staged.normal = Vec3::NAN;
            staged.contents = 21;
            Ok::<_, &str>(())
        }),
        Err(TransformedModelError::Trace(TraceError::NonFiniteInput))
    );
    assert_eq!(out, before);
    for fraction in [f32::NAN, -1., 2.] {
        assert!(
            q.trace_with(&mut out, |_, staged| {
                staged.fraction = fraction;
                Ok::<_, &str>(())
            })
            .is_err()
        );
        assert_eq!(out, before);
    }
    for fraction in [f32::NAN, -1., 2.] {
        let mut bad = seed(fraction);
        assert!(
            q.trace_with(&mut bad, |_, _| -> Result<(), &str> {
                panic!("invalid incoming reached query")
            })
            .is_err()
        );
        assert_eq!(bad.fraction.to_bits(), fraction.to_bits());
    }
}
