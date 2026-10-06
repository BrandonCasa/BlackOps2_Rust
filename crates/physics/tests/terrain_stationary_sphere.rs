#![forbid(unsafe_code)]
use bo2_assets::terrain_snapshot::{MAGIC, TerrainSnapshot, read_terrain_snapshot};
use bo2_physics::{
    Vec3,
    moving_brush_trace::{TraceError, TraceResult},
    terrain_stationary_sphere::{
        TerrainStationarySphereEffect as Effect, TerrainStationarySphereError as Error,
        TerrainStationarySphereRequest as Request,
    },
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
        normal: Vec3::new(-0., 2., 3.),
        fraction: 0.75,
        startsolid: false,
        allsolid: false,
        contents: 19,
        surface_flags: 20,
        opaque_flag_2c: 21,
    }
}
fn flat() -> TerrainSnapshot {
    fixture(
        &[[0., 0., 0.], [2., 0., 0.], [0., 2., 0.]],
        &[[0, 1, 2], [0, 2, 1]],
    )
}
fn req(center: [f32; 3], radius: f32) -> Request {
    Request::new(Vec3::from_array(center), radius).unwrap()
}
#[test]
fn independent_scalar_regions_preserve_unusual_operand_signs() {
    let s = flat();
    for (center, bits) in [
        ([0.5, 0.5, 1.], 0x3fc00000),
        ([-0.5, -0.5, 1.], 0x40400000),
        ([-3., 0., 0.], 0x41c80000),
        ([0., -3., 0.], 0x41c80000),
        ([-2., -2., 0.], 0x41900000),
        ([-3., 1., 0.], 0x41d00000),
        ([-3., -1., 0.], 0x41d00000),
    ] {
        let mut r = initial();
        let before = r;
        let step = req(center, 0.).trace(&s, 0, &mut r).unwrap();
        assert_eq!(step.distance_squared.to_bits(), bits);
        assert_eq!(step.effect, Effect::NoHit);
        assert_eq!(r, before);
    }
    // A conventional perpendicular projection would return distance1 here.
    let mut r = initial();
    assert_eq!(
        req([0.5, 0.5, 1.], 1.1)
            .trace(&s, 0, &mut r)
            .unwrap()
            .effect,
        Effect::NoHit
    );
}
#[test]
fn independent_oblique_exact_bits() {
    let s = fixture(
        &[
            [1.125, -2.75, 3.5],
            [3.375, 1.125, -0.625],
            [-1.875, 2.25, 4.125],
        ],
        &[[0, 1, 2]],
    );
    let mut r = initial();
    let step = req([-3.625, -5.125, 6.75], 0.)
        .trace(&s, 0, &mut r)
        .unwrap();
    assert_eq!(step.distance_squared.to_bits(), 0x43006e7a);
}
#[test]
fn tangent_vertex_inside_and_qualified_writes() {
    let s = flat();
    for (center, radius, hit) in [
        ([0., 0., 1.], 1., false),
        ([0., 0., 1.], 1.001, true),
        ([0., 0., 0.], 0., false),
        ([0., 0., 0.], 0.1, true),
        ([-3., 0., 0.], 5., false),
        ([-3., 0., 0.], 5.01, true),
        ([-2., -2., 0.], 5., true),
    ] {
        let mut r = initial();
        let before = r;
        let step = req(center, radius).trace(&s, 0, &mut r).unwrap();
        assert_eq!(
            step.effect,
            if hit { Effect::Overlap } else { Effect::NoHit }
        );
        if hit {
            assert!(r.startsolid && r.allsolid);
            assert_eq!(r.fraction.to_bits(), 0);
            assert_eq!(
                r.normal.to_array().map(f32::to_bits),
                before.normal.to_array().map(f32::to_bits)
            );
            assert_eq!(
                (r.contents, r.surface_flags, r.opaque_flag_2c),
                (19, 20, 21)
            );
        } else {
            assert_eq!(r, before);
        }
    }
}
#[test]
fn winding_and_seeded_flags_do_not_gate_raw_kernel() {
    let s = flat();
    for ordinal in [0, 1] {
        let mut r = initial();
        r.startsolid = true;
        r.allsolid = true;
        r.fraction = -0.;
        let before = r;
        let step = req([0., 0., 1.], 1.).trace(&s, ordinal, &mut r).unwrap();
        assert_eq!(step.effect, Effect::NoHit);
        assert_eq!(step.distance_squared, 1.);
        assert_eq!(r.fraction.to_bits(), before.fraction.to_bits());
        assert_eq!(r, before);
    }
}
#[test]
fn degenerate_nonfinite_arithmetic_is_atomic_not_repaired() {
    for vertices in [
        vec![[0., 0., 0.]; 3],
        vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]],
        vec![[0., 0., 0.], [f32::MAX, 0., 0.], [0., 1., 0.]],
    ] {
        let s = fixture(&vertices, &[[0, 1, 2]]);
        let mut r = initial();
        let before = r;
        assert_eq!(
            req([0., 0., 0.], 1.).trace(&s, 0, &mut r),
            Err(Error::Trace(TraceError::NonFiniteArithmetic))
        );
        assert_eq!(r, before);
    }
    // Collinear geometry can take a finite branch after rounded cancellation.
    let s = fixture(&[[0., 0., 0.], [1., 0., 0.], [3., 0., 0.]], &[[0, 1, 2]]);
    let mut r = initial();
    assert_eq!(
        req([3.1, 0., 0.], 2.)
            .trace(&s, 0, &mut r)
            .unwrap()
            .distance_squared
            .to_bits(),
        0x4119c28f
    );
}
#[test]
fn malformed_requests_references_and_late_radius_failure_are_atomic() {
    assert!(matches!(
        Request::new(Vec3::ZERO, -1.),
        Err(Error::InvalidRadius)
    ));
    for (center, radius) in [(Vec3::NAN, 1.), (Vec3::ZERO, f32::INFINITY)] {
        assert!(matches!(
            Request::new(center, radius),
            Err(Error::Trace(TraceError::NonFiniteInput))
        ));
    }
    let s = flat();
    let mut r = initial();
    let before = r;
    assert_eq!(
        req([0., 0., 1.], 1.).trace(&s, 2, &mut r),
        Err(Error::InvalidTriangle)
    );
    assert_eq!(r, before);
    assert_eq!(
        req([0., 0., 1.], f32::MAX).trace(&s, 0, &mut r),
        Err(Error::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(r, before);
    r.fraction = 2.;
    let before = r;
    assert_eq!(
        req([0., 0., 1.], 1.).trace(&s, 0, &mut r),
        Err(Error::Trace(TraceError::InvalidFraction))
    );
    assert_eq!(r, before);
    let mut r = initial();
    r.normal.x = f32::NAN;
    let before = r.normal.x.to_bits();
    assert_eq!(
        req([0., 0., 1.], 1.).trace(&s, 0, &mut r),
        Err(Error::Trace(TraceError::NonFiniteInput))
    );
    assert_eq!(r.normal.x.to_bits(), before);
}
#[test]
fn supplied_signed_zero_radius_and_overflowing_center_delta() {
    let s = flat();
    let mut r = initial();
    assert_eq!(
        req([0., 0., 1.], -0.).trace(&s, 0, &mut r).unwrap().effect,
        Effect::NoHit
    );
    let s = fixture(
        &[[f32::MAX, 0., 0.], [f32::MAX, 1., 0.], [f32::MAX, 0., 1.]],
        &[[0, 1, 2]],
    );
    let mut r = initial();
    let before = r;
    assert_eq!(
        req([-f32::MAX, 0., 0.], 1.).trace(&s, 0, &mut r),
        Err(Error::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(r, before);
}
