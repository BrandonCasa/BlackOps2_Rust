#![forbid(unsafe_code)]
use bo2_assets::static_model_snapshot::{MAGIC, StaticModelSnapshot, read_static_model_snapshot};
use bo2_physics::{
    Vec3,
    moving_brush_trace::{TraceError, TraceResult},
    static_point_trace::{
        StaticPointContact, StaticPointEffect, StaticPointError, StaticPointRequest,
    },
};
#[derive(Clone)]
struct Instance {
    origin: [f32; 3],
    axes: [[f32; 3]; 3],
    contents: u32,
}
impl Default for Instance {
    fn default() -> Self {
        Self {
            origin: [0.; 3],
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            contents: 0,
        }
    }
}
#[derive(Clone)]
struct Surface {
    contents: u32,
    flags: u32,
    bone: i32,
    bounds: [[f32; 3]; 2],
    triangles: Vec<[[f32; 4]; 3]>,
}
impl Default for Surface {
    fn default() -> Self {
        Self {
            contents: 1,
            flags: 31,
            bone: 7,
            bounds: [[-4.; 3], [4.; 3]],
            triangles: vec![[[0., 0., 1., 0.], [1., 0., 0., 0.], [0., 1., 0., 0.]]],
        }
    }
}
fn word(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn half(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn floats<const N: usize>(b: &mut Vec<u8>, v: [f32; N]) {
    for x in v {
        word(b, x.to_bits());
    }
}
fn fixture(instances: &[Instance], surfaces: &[Surface], lod: i16) -> StaticModelSnapshot {
    let triangles = surfaces.iter().map(|s| s.triangles.len()).sum::<usize>();
    let length = 144 + instances.len() * 88 + 56 + surfaces.len() * 48 + triangles * 48;
    let mut b = Vec::new();
    b.extend_from_slice(&MAGIC);
    word(&mut b, 1);
    word(&mut b, 0);
    b.extend_from_slice(&(length as u64).to_le_bytes());
    b.extend_from_slice(&[0; 80]);
    for n in [instances.len(), 1, surfaces.len(), triangles, 0] {
        word(&mut b, n as u32);
    }
    b.extend_from_slice(&[0; 20]);
    for (ordinal, i) in instances.iter().enumerate() {
        word(&mut b, ordinal as u32);
        word(&mut b, 0);
        half(&mut b, 0);
        half(&mut b, 0);
        word(&mut b, i.contents);
        floats(&mut b, i.origin);
        for a in i.axes {
            floats(&mut b, a);
        }
        // Deliberately unrelated absolute instance bounds: this raw consumer
        // excludes the native sector wrapper's separate bounds gate.
        floats(&mut b, [100.; 3]);
        floats(&mut b, [101.; 3]);
    }
    word(&mut b, 0);
    word(&mut b, 0);
    half(&mut b, lod as u16);
    half(&mut b, 0);
    for n in [0, surfaces.len(), 0, 0] {
        word(&mut b, n as u32);
    }
    floats(&mut b, [-4.; 3]);
    floats(&mut b, [4.; 3]);
    word(&mut b, 1_f32.to_bits());
    let mut first = 0;
    for (ordinal, s) in surfaces.iter().enumerate() {
        word(&mut b, ordinal as u32);
        word(&mut b, first);
        word(&mut b, s.triangles.len() as u32);
        floats(&mut b, s.bounds[0]);
        floats(&mut b, s.bounds[1]);
        word(&mut b, s.bone as u32);
        word(&mut b, s.contents);
        word(&mut b, s.flags);
        first += s.triangles.len() as u32;
    }
    for s in surfaces {
        for triangle in &s.triangles {
            for v in triangle {
                floats(&mut b, *v);
            }
        }
    }
    assert_eq!(b.len(), length);
    read_static_model_snapshot(&mut b.as_slice(), length as u64).unwrap()
}
fn initial() -> TraceResult {
    TraceResult {
        normal: Vec3::new(7., 8., 9.),
        fraction: 1.,
        surface_flags: 77,
        contents: 88,
        startsolid: true,
        allsolid: true,
        opaque_flag_2c: 123,
    }
}
fn request(start: [f32; 3], end: [f32; 3], mask: u32) -> StaticPointRequest {
    StaticPointRequest::new(Vec3::from_array(start), Vec3::from_array(end), mask).unwrap()
}
fn contact(surface: u32, triangle: u32, bone: i32) -> StaticPointEffect {
    StaticPointEffect::Hit(StaticPointContact {
        instance_ordinal: 0,
        provider_ordinal: 0,
        surface_ordinal: surface,
        triangle_ordinal: triangle,
        bone_index: bone,
    })
}
fn base() -> StaticModelSnapshot {
    fixture(&[Instance::default()], &[Surface::default()], 0)
}
#[test]
fn independent_front_crossing_fixture_promotes_hit_and_clears_inside_flags() {
    // Independent static01 mathematical fixture: (1 - 1/8)/(1 - -1)=7/16.
    let snapshot = base();
    let mut result = initial();
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        contact(0, 0, 7)
    );
    assert_eq!(result.fraction.to_bits(), 0x3ee0_0000);
    assert_eq!(result.normal, Vec3::Z);
    assert_eq!(
        (
            result.contents,
            result.surface_flags,
            result.startsolid,
            result.allsolid,
            result.opaque_flag_2c
        ),
        (1, 31, false, false, 123)
    );
    // Both source instance/provider contents are zero, and absolute instance
    // bounds exclude the endpoints. They are not raw-consumer filters.
    assert_eq!(snapshot.instances()[0].contents, 0);
    assert_eq!(snapshot.providers()[0].contents, 0);
}
#[test]
fn no_hit_backface_stationary_on_plane_masks_lod_bounds_and_ties_are_atomic() {
    let snapshot = base();
    for (start, end, mask) in [
        ([0.25, 0.25, -1.], [0.25, 0.25, 1.], 1),
        ([0.25, 0.25, 1.], [0.25, 0.25, 1.], 1),
        ([0.25, 0.25, 0.], [0.25, 0.25, 0.], 1),
        ([0.25, 0.25, 0.], [0.25, 0.25, -1.], 1),
        ([0.25, 0.25, 1.], [0.25, 0.25, 0.], 1),
        ([0.25, 0.25, 1.], [0.25, 0.25, -1.], 2),
    ] {
        let mut result = initial();
        assert_eq!(
            request(start, end, mask)
                .trace(&snapshot, 0, &mut result)
                .unwrap(),
            StaticPointEffect::NoHit
        );
        assert_eq!(result, initial());
    }
    let mut result = initial();
    result.fraction = 0.4375;
    let before = result;
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        StaticPointEffect::NoHit
    );
    assert_eq!(result, before);
    let mut result = initial();
    let negative = fixture(&[Instance::default()], &[Surface::default()], -1);
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&negative, 0, &mut result)
            .unwrap(),
        StaticPointEffect::NoHit
    );
    assert_eq!(result, initial());
    let bounds = Surface {
        bounds: [[0.5, -4., -4.], [1., 4., 4.]],
        ..Surface::default()
    };
    let snapshot = fixture(&[Instance::default()], &[bounds], 0);
    let mut result = initial();
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        StaticPointEffect::NoHit
    );
    assert_eq!(result, initial());
}
#[test]
fn geometric_bounds_entry_uses_incoming_fraction_without_triangle_allowance() {
    let surface = Surface {
        bounds: [[-4., -4., -0.25], [4., 4., 0.]],
        ..Surface::default()
    };
    let snapshot = fixture(&[Instance::default()], &[surface], 0);
    let mut result = initial();
    result.fraction = 0.5;
    let before = result;
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        StaticPointEffect::NoHit
    );
    assert_eq!(result, before); // Geometric bounds entry 0.5 equals incoming limit.
    result.fraction = f32::from_bits(0.5_f32.to_bits() + 1);
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        contact(0, 0, 7)
    );
    assert_eq!(result.fraction, 0.4375);
}
#[test]
fn barycentric_tolerances_use_exact_plane_crossing_and_source_words() {
    let lower = f32::from_bits(0xba83_126f);
    let upper = f32::from_bits(0x3f80_20c5);
    for (x, y, hit) in [
        (lower, 0.25, true),
        (f32::from_bits(lower.to_bits() + 1), 0.25, false),
        (upper, 0., true),
        (f32::from_bits(upper.to_bits() + 1), 0., false),
        (0.25, lower, true),
        (0.25, f32::from_bits(lower.to_bits() + 1), false),
        (0.75, 0.75, false),
    ] {
        let mut result = initial();
        let effect = request([x, y, 1.], [x, y, -1.], 1)
            .trace(&base(), 0, &mut result)
            .unwrap();
        assert_eq!(matches!(effect, StaticPointEffect::Hit(_)), hit);
        if !hit {
            assert_eq!(result, initial());
        }
    }
    // Plane intersection at x=0 is admitted; using the offset fraction would
    // wrongly evaluate x=-0.125 and reject the triangle.
    let mut result = initial();
    assert_eq!(
        request([-1., 0.25, 1.], [1., 0.25, -1.], 1)
            .trace(&base(), 0, &mut result)
            .unwrap(),
        contact(0, 0, 7)
    );
}
#[test]
fn ordered_triangle_surface_ties_and_masks_keep_original_contacts() {
    let mut first = Surface::default();
    first.triangles.push(first.triangles[0]);
    let second = Surface {
        bone: 21,
        flags: 99,
        ..first.clone()
    };
    let snapshot = fixture(&[Instance::default()], &[first.clone(), second], 0);
    let mut result = initial();
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        contact(0, 0, 7)
    );
    assert_eq!(result.surface_flags, 31);
    first.triangles[1][0][3] = 0.5;
    let snapshot = fixture(&[Instance::default()], &[first], 0);
    let mut result = initial();
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        contact(0, 1, 7)
    );
    assert_eq!(result.fraction, 0.1875);
    let masked = Surface {
        contents: 2,
        ..Surface::default()
    };
    let second = Surface {
        flags: 99,
        bone: 21,
        ..Surface::default()
    };
    let snapshot = fixture(&[Instance::default()], &[masked, second], 0);
    let mut result = initial();
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        contact(1, 0, 21)
    );
    assert_eq!(result.surface_flags, 99);
}
#[test]
fn shared_provider_distinct_origins_column_transform_row_normal_and_scale() {
    let rotated = Instance {
        origin: [10., 20., 30.],
        axes: [[0., 0., 0.5], [0., 0.5, 0.], [-0.5, 0., 0.]],
        contents: 0,
    };
    let translated = Instance {
        origin: [0., 0., 2.],
        ..Instance::default()
    };
    let snapshot = fixture(
        &[Instance::default(), rotated, translated],
        &[Surface::default()],
        0,
    );
    assert_eq!(snapshot.providers().len(), 1);
    let mut a = initial();
    request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
        .trace(&snapshot, 0, &mut a)
        .unwrap();
    let mut b = initial();
    let effect = request([12., 20.5, 29.5], [8., 20.5, 29.5], 1)
        .trace(&snapshot, 1, &mut b)
        .unwrap();
    assert_eq!(
        effect,
        StaticPointEffect::Hit(StaticPointContact {
            instance_ordinal: 1,
            provider_ordinal: 0,
            surface_ordinal: 0,
            triangle_ordinal: 0,
            bone_index: 7
        })
    );
    assert_eq!(a.fraction.to_bits(), b.fraction.to_bits());
    assert_eq!(a.normal, Vec3::Z);
    assert_eq!(b.normal, Vec3::X);
    let mut c = initial();
    assert_eq!(
        request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1)
            .trace(&snapshot, 2, &mut c)
            .unwrap(),
        StaticPointEffect::NoHit
    );
    assert_eq!(c, initial());
}
#[test]
fn independent_nonaxis_f32_fixtures_distinguish_source_operand_order() {
    // Independently evaluated in external static03 reference.py with explicit
    // struct-packed f32 stores. Reassociation changes the first fixture's Z
    // normal by one ULP; the second's fraction and Y normal by one ULP.
    let instance = Instance {
        origin: [1_038_174_126, 3_180_289_065, 1_022_739_087].map(f32::from_bits),
        axes: [
            [1_060_320_051, 1_045_220_557, 3_184_315_597],
            [1_050_253_722, 1_063_675_494, 1_041_865_114],
            [3_196_059_648, 1_036_831_949, 1_066_192_077],
        ]
        .map(|r| r.map(f32::from_bits)),
        contents: 0,
    };
    let start = [1_063_675_494, 3_206_125_978, 1_060_320_051].map(f32::from_bits);
    let end = [3_197_737_370, 1_066_192_077, 3_192_704_205].map(f32::from_bits);
    for (normal, fraction, world_normal) in [
        (
            [1_058_849_456, 3_209_874_959, 1_055_514_691],
            1_052_404_057,
            [1_052_428_712, 3_209_768_812, 1_055_163_553],
        ),
        (
            [1_015_237_232, 3_219_762_911, 1_079_879_257],
            1_057_667_198,
            [3_191_469_358, 3_197_406_676, 1_064_339_780],
        ),
    ] {
        let n = normal.map(f32::from_bits);
        let mut surface = Surface::default();
        surface.triangles[0][0] = [n[0], n[1], n[2], 0.];
        let snapshot = fixture(std::slice::from_ref(&instance), &[surface], 0);
        let mut result = initial();
        assert_eq!(
            request(start, end, 1)
                .trace(&snapshot, 0, &mut result)
                .unwrap(),
            contact(0, 0, 7)
        );
        assert_eq!(result.fraction.to_bits(), fraction);
        assert_eq!(result.normal.to_array().map(f32::to_bits), world_normal);
    }
}
#[test]
fn normal_length_underflow_uses_native_zero_length_denominator_guard() {
    let scale = 1e-30_f32;
    let instance = Instance {
        axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., scale]],
        ..Instance::default()
    };
    let snapshot = fixture(&[instance], &[Surface::default()], 0);
    let mut result = initial();
    assert_eq!(
        request([0.25, 0.25, 1e30], [0.25, 0.25, -1e30], 1)
            .trace(&snapshot, 0, &mut result)
            .unwrap(),
        contact(0, 0, 7)
    );
    // Squared length rounds to zero; native substitutes divisor 1, retaining
    // the finite tiny normal rather than forcing a unit vector or an epsilon.
    assert_eq!(
        result.normal.to_array().map(f32::to_bits),
        [0, 0, scale.to_bits()]
    );
}
#[test]
fn fraction_zero_hit_clears_flags_but_zero_incoming_limit_preserves_them() {
    let mut result = initial();
    let req = request([0.25, 0.25, 0.0625], [0.25, 0.25, -1.], 1);
    assert_eq!(
        req.trace(&base(), 0, &mut result).unwrap(),
        contact(0, 0, 7)
    );
    assert_eq!(result.fraction, 0.);
    assert!(!result.startsolid && !result.allsolid);
    assert_eq!(result.opaque_flag_2c, 123);
    let mut result = initial();
    result.fraction = 0.;
    let before = result;
    assert_eq!(
        req.trace(&base(), 0, &mut result).unwrap(),
        StaticPointEffect::NoHit
    );
    assert_eq!(result, before);
}
#[test]
fn reference_input_negative_bone_and_late_arithmetic_errors_are_atomic() {
    let req = request([0.25, 0.25, 1.], [0.25, 0.25, -1.], 1);
    let mut result = initial();
    assert_eq!(
        req.trace(&base(), u32::MAX, &mut result),
        Err(StaticPointError::InvalidReference)
    );
    assert_eq!(result, initial());
    assert_eq!(
        StaticPointRequest::new(Vec3::NAN, Vec3::ZERO, 1).unwrap_err(),
        StaticPointError::Trace(TraceError::NonFiniteInput)
    );
    let mut result = initial();
    result.fraction = 2.;
    let before = result;
    assert_eq!(
        req.trace(&base(), 0, &mut result),
        Err(StaticPointError::Trace(TraceError::InvalidFraction))
    );
    assert_eq!(result, before);
    let negative = fixture(
        &[Instance::default()],
        &[Surface {
            bone: -1,
            ..Surface::default()
        }],
        0,
    );
    let mut result = initial();
    assert_eq!(
        req.trace(&negative, 0, &mut result),
        Err(StaticPointError::NegativeBoneContact)
    );
    assert_eq!(result, initial());
    let mut bad = Surface::default();
    bad.triangles.push([
        [f32::MAX, f32::MAX, f32::MAX, 0.],
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
    ]);
    let snapshot = fixture(&[Instance::default()], &[bad], 0);
    let mut result = initial();
    assert_eq!(
        req.trace(&snapshot, 0, &mut result),
        Err(StaticPointError::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(result, initial());
    let huge = Instance {
        axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., f32::MAX]],
        ..Instance::default()
    };
    let snapshot = fixture(&[huge], &[Surface::default()], 0);
    let mut result = initial();
    assert_eq!(
        request([0.25, 0.25, 1e-38], [0.25, 0.25, -1e-38], 1).trace(&snapshot, 0, &mut result),
        Err(StaticPointError::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(result, initial());
    let offset = Instance {
        origin: [-f32::MAX, 0., 0.],
        ..Instance::default()
    };
    let snapshot = fixture(&[offset], &[Surface::default()], 0);
    let mut result = initial();
    assert_eq!(
        request([f32::MAX, 0., 1.], [f32::MAX, 0., -1.], 1).trace(&snapshot, 0, &mut result),
        Err(StaticPointError::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(result, initial());
    let mut result = initial();
    let subnormal = f32::from_bits(1);
    assert_eq!(
        request([0.25, 0.25, subnormal], [0.25, 0.25, 0.], 1).trace(&base(), 0, &mut result),
        Err(StaticPointError::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(result, initial());
}
