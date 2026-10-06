#![forbid(unsafe_code)]
use bo2_assets::terrain_snapshot::{MAGIC, TerrainSnapshot, read_terrain_snapshot};
use bo2_physics::{
    Vec3,
    moving_brush_trace::{TraceError, TraceResult},
    terrain_point_trace::{TerrainPointEffect, TerrainPointError, TerrainPointRequest},
};
fn w(b: &mut Vec<u8>, v: u32) {
    b.extend(v.to_le_bytes());
}
fn fixture(vertices: &[[f32; 3]], triangles: &[[u16; 3]]) -> TerrainSnapshot {
    let packed = 4 * (3 * triangles.len()).div_ceil(32);
    let mut b = Vec::new();
    b.extend(MAGIC);
    w(&mut b, 1);
    w(&mut b, 0);
    b.extend(((144 + 6 * triangles.len() + 12 * vertices.len() + packed) as u64).to_le_bytes());
    for _ in 0..2 {
        b.extend(0_u64.to_le_bytes());
        b.extend([0; 32]);
    }
    for n in [0, 0, 0, triangles.len(), vertices.len(), packed, 0, 0] {
        w(&mut b, n as u32);
    }
    b.extend([0; 8]);
    for t in triangles {
        for v in t {
            b.extend(v.to_le_bytes());
        }
    }
    for v in vertices {
        for x in v {
            w(&mut b, x.to_bits());
        }
    }
    b.resize(b.len() + packed, 0);
    read_terrain_snapshot(&mut b.as_slice(), b.len() as u64).unwrap()
}
fn initial() -> TraceResult {
    TraceResult {
        normal: Vec3::new(-0., 8., 9.),
        fraction: 1.,
        surface_flags: 71,
        contents: 72,
        startsolid: true,
        allsolid: true,
        opaque_flag_2c: 255,
    }
}
fn flat() -> TerrainSnapshot {
    fixture(
        &[[-2., -2., 0.], [0., 2., 0.], [2., -2., 0.]],
        &[[0, 1, 2], [0, 2, 1], [0, 1, 2]],
    )
}
fn req(start: [f32; 3], delta: [f32; 3]) -> TerrainPointRequest {
    TerrainPointRequest::new(Vec3::from_array(start), Vec3::from_array(delta)).unwrap()
}
#[test]
fn face_edge_vertex_and_on_plane_keep_only_qualified_writes() {
    let s = flat();
    for start in [[0., 0., 2.], [0., -2., 2.], [-2., -2., 2.], [0., 0., 0.]] {
        let mut r = initial();
        assert_eq!(
            req(start, [0., 0., -4.]).trace(&s, 0, &mut r).unwrap(),
            TerrainPointEffect::Hit {
                triangle_ordinal: 0
            }
        );
        assert_eq!(
            r.normal.to_array().map(f32::to_bits),
            [0, 0, 1_f32.to_bits()]
        );
        assert_eq!(
            r.fraction.to_bits(),
            if start[2] == 0. {
                (-0_f32).to_bits()
            } else {
                0.5_f32.to_bits()
            }
        );
        assert_eq!(
            (
                r.contents,
                r.surface_flags,
                r.startsolid,
                r.allsolid,
                r.opaque_flag_2c
            ),
            (72, 71, true, true, 0)
        );
    }
}
#[test]
fn backface_parallel_outside_behind_and_fraction_ties_are_atomic() {
    let s = flat();
    for (start, delta, index, fraction) in [
        ([0., 0., -2.], [0., 0., 4.], 0, 1.),
        ([0., 0., 2.], [1., 0., 0.], 0, 1.),
        ([3., 0., 2.], [0., 0., -4.], 0, 1.),
        ([0., 0., -1.], [0., 0., -1.], 0, 1.),
        ([0., 0., 2.], [0., 0., -4.], 1, 1.),
        ([0., 0., 2.], [0., 0., -4.], 0, 0.5),
        ([0., 0., 2.], [0., 0., -4.], 0, 0.25),
    ] {
        let mut r = initial();
        r.fraction = fraction;
        let before = r;
        assert_eq!(
            req(start, delta).trace(&s, index, &mut r).unwrap(),
            TerrainPointEffect::NoHit
        );
        assert_eq!(r, before);
    }
    let query = req([0., 0., 2.], [0., 0., -4.]);
    let mut r = initial();
    query.trace(&s, 0, &mut r).unwrap();
    let first = r;
    assert_eq!(
        query.trace(&s, 2, &mut r).unwrap(),
        TerrainPointEffect::NoHit
    );
    assert_eq!(r, first);
}
#[test]
fn explicit_displacement_is_not_an_endpoint() {
    let s = flat();
    let mut r = initial();
    req([0., 0., 2.], [0., 0., -4.])
        .trace(&s, 0, &mut r)
        .unwrap();
    assert_eq!(r.fraction, 0.5);
    let mut r = initial();
    assert_eq!(
        req([0., 0., 2.], [0., 0., -2.])
            .trace(&s, 0, &mut r)
            .unwrap(),
        TerrainPointEffect::NoHit
    );
}
#[test]
fn independent_nonaxis_scalar_f32_receipts_have_exact_bits() {
    // Independent Python IEEE754 pack/unpack after each primitive, then scalar sqrt.
    for (vertices, start, delta, normal, fraction) in [
        (
            [[-2., -2., 0.], [0., 2., 1.], [2., -2., 2.]],
            [0., 0., 4.],
            [0.25, 0.125, -8.],
            [3202677038, 0, 1063581998],
            1052576721,
        ),
        (
            [[-1.75, 0.125, 0.5], [0.25, 3., 2.], [2.5, -0.75, 1.25]],
            [0., 1., 4.],
            [0.125, -0.5, -8.],
            [3194601011, 3198435447, 1063998083],
            1051975334,
        ),
    ] {
        let s = fixture(&vertices, &[[0, 1, 2]]);
        let mut r = initial();
        assert_eq!(
            req(start, delta).trace(&s, 0, &mut r).unwrap(),
            TerrainPointEffect::Hit {
                triangle_ordinal: 0
            }
        );
        assert_eq!(r.normal.to_array().map(f32::to_bits), normal);
        assert_eq!(r.fraction.to_bits(), fraction);
    }
}
#[test]
fn degenerate_and_repeated_vertices_reject_without_normalization() {
    for vertices in [
        [[0., 0., 0.]; 3],
        [[0., 0., 0.], [1., 1., 1.], [2., 2., 2.]],
        [[0., 0., 0.], [0., 0., 0.], [0., 1., 0.]],
    ] {
        let s = fixture(&vertices, &[[0, 1, 2]]);
        let mut r = initial();
        assert_eq!(
            req([0., 0., 2.], [0., 0., -4.])
                .trace(&s, 0, &mut r)
                .unwrap(),
            TerrainPointEffect::NoHit
        );
        assert_eq!(r, initial());
    }
}
#[test]
fn zero_length_from_underflow_uses_source_denominator_guard() {
    let s = fixture(
        &[[0., 0., 0.], [0., 1e-15, 0.], [1e-15, 0., 0.]],
        &[[0, 1, 2]],
    );
    let mut r = initial();
    assert_eq!(
        req([0., 0., 1.], [0., 0., -1e30])
            .trace(&s, 0, &mut r)
            .unwrap(),
        TerrainPointEffect::Hit {
            triangle_ordinal: 0
        }
    );
    assert_eq!(r.normal[2].to_bits(), (1e-15_f32 * 1e-15_f32).to_bits());
    assert!(r.fraction > 0.);
}
#[test]
fn reference_input_and_late_normalization_errors_preserve_every_field() {
    let s = flat();
    let query = req([0., 0., 2.], [0., 0., -4.]);
    let mut r = initial();
    assert_eq!(
        query.trace(&s, 99, &mut r),
        Err(TerrainPointError::InvalidTriangle)
    );
    assert_eq!(r, initial());
    assert_eq!(
        TerrainPointRequest::new(Vec3::splat(f32::NAN), Vec3::ZERO).unwrap_err(),
        TerrainPointError::Trace(TraceError::NonFiniteInput)
    );
    for fraction in [f32::NAN, 2.] {
        let mut r = initial();
        r.fraction = fraction;
        let before = r;
        assert!(query.trace(&s, 0, &mut r).is_err());
        assert_eq!(r.normal, before.normal);
        assert_eq!(r.fraction.to_bits(), before.fraction.to_bits());
        assert_eq!(r.opaque_flag_2c, 255);
    }
    // Face arithmetic finite and admitted; normalization squares overflow after
    // all face/edge predicates, unlike an early cross-product failure.
    let s = fixture(
        &[[0., 0., 0.], [0., 1e10, 0.], [1e10, 0., 0.]],
        &[[0, 1, 2]],
    );
    let mut r = initial();
    assert_eq!(
        req([0., 0., 1e-10], [0., 0., -2e-10]).trace(&s, 0, &mut r),
        Err(TerrainPointError::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(r, initial());
    let s = fixture(
        &[[f32::MAX, 0., 0.], [-f32::MAX, 0., 0.], [0., 1., 0.]],
        &[[0, 1, 2]],
    );
    let mut r = initial();
    assert_eq!(
        query.trace(&s, 0, &mut r),
        Err(TerrainPointError::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(r, initial());
}

#[test]
fn reversed_winding_admits_reverse_motion_and_zero_displacement_rejects() {
    let s = flat();
    let mut r = initial();
    assert_eq!(
        req([0., 0., -2.], [0., 0., 4.])
            .trace(&s, 1, &mut r)
            .unwrap(),
        TerrainPointEffect::Hit {
            triangle_ordinal: 1
        }
    );
    assert_eq!(r.normal[2].to_bits(), (-1_f32).to_bits());
    assert_eq!(r.fraction, 0.5);
    let mut r = initial();
    assert_eq!(
        req([0., 0., 0.], [0.; 3]).trace(&s, 0, &mut r).unwrap(),
        TerrainPointEffect::NoHit
    );
    assert_eq!(r, initial());
    let mut r = initial();
    r.normal[0] = f32::NAN;
    let before = r;
    assert_eq!(
        req([0., 0., 2.], [0., 0., -4.]).trace(&s, 0, &mut r),
        Err(TerrainPointError::Trace(TraceError::NonFiniteInput))
    );
    assert_eq!(
        r.normal.to_array().map(f32::to_bits),
        before.normal.to_array().map(f32::to_bits)
    );
    assert_eq!(r.fraction, before.fraction);
    assert_eq!(r.opaque_flag_2c, 255);
}
