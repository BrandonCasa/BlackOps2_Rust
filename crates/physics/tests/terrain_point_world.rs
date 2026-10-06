#![forbid(unsafe_code)]
use bo2_assets::terrain_snapshot::{MAGIC, TerrainSnapshot, read_terrain_snapshot};
use bo2_physics::{
    Vec3,
    moving_brush_trace::{TraceError, TraceResult},
    terrain_point_world::{
        TerrainLeafSource as Leaf, TerrainPointLimits, TerrainPointWorld, TerrainPointWorldError,
        TerrainPointWorldRequest,
    },
};
#[derive(Clone, Copy)]
struct Node {
    origin: [f32; 3],
    half: [f32; 3],
    material: u16,
    children: u16,
    reference: i32,
}
fn node(partition: i32, material: u16) -> Node {
    Node {
        origin: [0.; 3],
        half: [4.; 3],
        material,
        children: 0,
        reference: partition,
    }
}
fn w(b: &mut Vec<u8>, v: u32) {
    b.extend(v.to_le_bytes());
}
fn h(b: &mut Vec<u8>, v: u16) {
    b.extend(v.to_le_bytes());
}
fn v(b: &mut Vec<u8>, x: [f32; 3]) {
    for f in x {
        w(b, f.to_bits());
    }
}
fn put_leaf(b: &mut Vec<u8>, first: u16, count: u16, contents: u32) {
    h(b, first);
    h(b, count);
    w(b, 0);
    w(b, contents);
    v(b, [-4.; 3]);
    v(b, [4.; 3]);
    w(b, 0);
    h(b, u16::MAX);
    h(b, 0);
}
fn snapshot(
    nodes: &[Node],
    leaves: &[(u16, u16, u32)],
    model_leaf: (u16, u16, u32),
    bad_triangle: bool,
) -> TerrainSnapshot {
    let triangles = 3;
    let vertices = 9;
    let packed = 4;
    let size = 144
        + 3 * 8
        + nodes.len() * 32
        + 3 * 16
        + triangles * 6
        + vertices * 12
        + packed
        + leaves.len() * 44
        + 80;
    let mut b = Vec::new();
    b.extend(MAGIC);
    w(&mut b, 1);
    w(&mut b, 0);
    b.extend((size as u64).to_le_bytes());
    for _ in 0..2 {
        b.extend(0_u64.to_le_bytes());
        b.extend([0; 32]);
    }
    for count in [
        3,
        nodes.len(),
        3,
        triangles,
        vertices,
        packed,
        leaves.len(),
        1,
    ] {
        w(&mut b, count as u32);
    }
    b.extend([0; 8]);
    for (flags, contents) in [(11, 1), (22, 2), (33, 1)] {
        w(&mut b, flags);
        w(&mut b, contents);
    }
    for n in nodes {
        v(&mut b, n.origin);
        h(&mut b, n.material);
        h(&mut b, n.children);
        v(&mut b, n.half);
        w(&mut b, n.reference as u32);
    }
    for i in 0..3 {
        b.extend([1, 0, 0, 0]);
        w(&mut b, i);
        w(&mut b, 0);
        w(&mut b, 0);
    }
    for i in 0..3u16 {
        for id in [i * 3, i * 3 + 1, i * 3 + 2] {
            h(&mut b, id);
        }
    }
    for z in [0., 1., 0.] {
        for xyz in if bad_triangle && z == 1. {
            [[0., 0., z], [0., 1e10, z], [1e10, 0., z]]
        } else {
            [[-2., -2., z], [0., 2., z], [2., -2., z]]
        } {
            v(&mut b, xyz);
        }
    }
    b.extend([0; 4]);
    for &(f, c, m) in leaves {
        put_leaf(&mut b, f, c, m);
    }
    w(&mut b, 0);
    w(&mut b, 0);
    v(&mut b, [-4.; 3]);
    v(&mut b, [4.; 3]);
    w(&mut b, 4_f32.to_bits());
    put_leaf(&mut b, model_leaf.0, model_leaf.1, model_leaf.2);
    assert_eq!(b.len(), size);
    read_terrain_snapshot(&mut b.as_slice(), b.len() as u64).unwrap()
}
fn limits() -> TerrainPointLimits {
    TerrainPointLimits {
        max_leaves: 10,
        max_aabb_tests: 100,
        max_pending_nodes: 100,
        max_triangles: 100,
    }
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
fn request(mask: u32) -> TerrainPointWorldRequest {
    TerrainPointWorldRequest::new(Vec3::new(0., 0., 2.), Vec3::new(0., 0., -2.), mask).unwrap()
}
#[test]
fn ascending_depth_first_processing_inherits_outer_material_and_promotes_closest_partition() {
    let mut root = node(1, 0);
    root.children = 2;
    let s = snapshot(
        &[root, node(0, 1), node(1, 1)],
        &[(0, 1, 1)],
        (0, 1, 1),
        false,
    );
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    let mut r = initial();
    let step = world
        .trace_leaves(&[Leaf::Map(0)], &request(1), &mut ws, &mut r)
        .unwrap();
    assert_eq!(
        (
            step.tested_aabbs,
            step.admitted_partitions,
            step.tested_triangles
        ),
        (3, 2, 2)
    );
    let contact = step.contact.unwrap();
    assert_eq!(
        (
            contact.material_ordinal,
            contact.partition_ordinal,
            contact.triangle_ordinal
        ),
        (0, 1, 1)
    );
    assert_eq!(r.fraction, 0.25);
    assert_eq!((r.contents, r.surface_flags), (1, 11));
    assert!(r.allsolid && r.startsolid);
}
#[test]
fn shared_partitions_across_map_model_leaves_keep_first_outer_material_owner() {
    let s = snapshot(
        &[node(0, 0), node(0, 2)],
        &[(0, 1, 1), (1, 1, 1)],
        (1, 1, 1),
        false,
    );
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    for selectors in [
        [Leaf::Map(0), Leaf::Model(0), Leaf::Map(1)],
        [Leaf::Model(0), Leaf::Map(0), Leaf::Map(1)],
    ] {
        let mut r = initial();
        let step = world
            .trace_leaves(&selectors, &request(1), &mut ws, &mut r)
            .unwrap();
        assert_eq!((step.admitted_partitions, step.tested_triangles), (1, 1));
        assert_eq!(
            r.surface_flags,
            if selectors[0] == Leaf::Map(0) { 11 } else { 33 }
        );
        assert_eq!(
            step.contact.unwrap().material_ordinal,
            if selectors[0] == Leaf::Map(0) { 0 } else { 2 }
        );
    }
}
#[test]
fn leaf_and_outer_material_masks_gate_before_bounds_and_stamps() {
    let s = snapshot(
        &[node(0, 1), node(0, 0)],
        &[(0, 1, 1), (1, 1, 2)],
        (1, 1, 0),
        false,
    );
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    let mut r = initial();
    let step = world
        .trace_leaves(
            &[Leaf::Map(0), Leaf::Map(1), Leaf::Model(0)],
            &request(1),
            &mut ws,
            &mut r,
        )
        .unwrap();
    assert_eq!(
        (
            step.tested_aabbs,
            step.admitted_partitions,
            step.tested_triangles
        ),
        (0, 0, 0)
    );
    assert_eq!(r, initial());
    assert_eq!(ws.generation(), 1);
}
#[test]
fn exact_sat_equalities_and_cross_axes_reject_without_generic_slabs() {
    // Zero box at origin: segment ending exactly at origin reaches point predicates.
    let mut n = node(0, 0);
    n.half = [0.; 3];
    let s = snapshot(&[n], &[(0, 1, 1)], (0, 1, 1), false);
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    for (start, end, tests) in [
        (Vec3::new(0., 0., 2.), Vec3::ZERO, 1),
        (Vec3::new(-2., -2., 2.), Vec3::new(2., 2., -2.), 1),
        (Vec3::new(-2., -2., 2.), Vec3::new(2., 1., -2.), 0),
    ] {
        let req = TerrainPointWorldRequest::new(start, end, 1).unwrap();
        let mut r = initial();
        let step = world
            .trace_leaves(&[Leaf::Map(0)], &req, &mut ws, &mut r)
            .unwrap();
        assert_eq!(step.tested_triangles, tests);
    }
}
#[test]
fn stored_triangle_and_root_ties_keep_earlier_material_partition_contact() {
    let s = snapshot(&[node(0, 0), node(2, 2)], &[(0, 2, 1)], (0, 2, 1), false);
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    let mut r = initial();
    let step = world
        .trace_leaves(&[Leaf::Map(0)], &request(1), &mut ws, &mut r)
        .unwrap();
    assert_eq!(step.tested_triangles, 2);
    assert_eq!(step.contact.unwrap().partition_ordinal, 0);
    assert_eq!(r.surface_flags, 11);
    let mut r = initial();
    r.fraction = 0.5;
    let before = r;
    let step = world
        .trace_leaves(&[Leaf::Map(0)], &request(1), &mut ws, &mut r)
        .unwrap();
    assert_eq!(step.contact, None);
    assert_eq!(r, before);
}
#[test]
fn late_error_burns_generation_but_preserves_result_and_retry_uses_fresh_stamps() {
    let s = snapshot(
        &[node(0, 0), node(1, 0)],
        &[(0, 1, 1), (1, 1, 1)],
        (0, 1, 1),
        true,
    );
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    let mut r = initial();
    assert!(matches!(
        world.trace_leaves(&[Leaf::Map(0), Leaf::Map(1)], &request(1), &mut ws, &mut r),
        Err(TerrainPointWorldError::Point(_))
    ));
    assert_eq!(r, initial());
    assert_eq!(ws.generation(), 1);
    assert!(
        world
            .trace_leaves(&[Leaf::Map(0), Leaf::Map(1)], &request(1), &mut ws, &mut r)
            .is_err()
    );
    assert_eq!(r, initial());
    assert_eq!(ws.generation(), 2);
    let step = world
        .trace_leaves(&[Leaf::Map(0)], &request(1), &mut ws, &mut r)
        .unwrap();
    assert_eq!(step.tested_triangles, 1);
    assert_eq!(r.fraction, 0.5);
    assert_eq!(ws.generation(), 3);
}
#[test]
fn pending_triangle_leaf_and_bounds_caps_fail_atomically_and_reset_scratch() {
    let mut root = node(1, 0);
    root.children = 2;
    let s = snapshot(
        &[root, node(0, 0), node(1, 0)],
        &[(0, 1, 1), (1, 1, 1)],
        (1, 1, 1),
        false,
    );
    for (lim, want) in [
        (
            TerrainPointLimits {
                max_pending_nodes: 1,
                ..limits()
            },
            TerrainPointWorldError::PendingCapacity,
        ),
        (
            TerrainPointLimits {
                max_triangles: 1,
                ..limits()
            },
            TerrainPointWorldError::TriangleLimit,
        ),
        (
            TerrainPointLimits {
                max_aabb_tests: 1,
                ..limits()
            },
            TerrainPointWorldError::AabbTestLimit,
        ),
        (
            TerrainPointLimits {
                max_leaves: 0,
                ..limits()
            },
            TerrainPointWorldError::LeafLimit,
        ),
    ] {
        let world = TerrainPointWorld::new(&s, lim).unwrap();
        let mut ws = world.new_workspace().unwrap();
        let mut r = initial();
        assert_eq!(
            world.trace_leaves(&[Leaf::Map(0)], &request(1), &mut ws, &mut r),
            Err(want)
        );
        assert_eq!(r, initial());
        assert_eq!(ws.generation(), 1);
        if lim.max_leaves > 0 {
            let step = world
                .trace_leaves(&[Leaf::Map(1)], &request(1), &mut ws, &mut r)
                .unwrap();
            assert_eq!(step.tested_triangles, 1);
        }
    }
}
#[test]
fn snapshot_limit_binding_prevalidation_and_generation_exhaustion_are_explicit() {
    let s = snapshot(&[node(0, 0)], &[(0, 1, 1)], (0, 1, 1), false);
    let other = snapshot(&[node(0, 0)], &[(0, 1, 1)], (0, 1, 1), false);
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    let mut r = initial();
    for w in [
        TerrainPointWorld::new(&other, limits()).unwrap(),
        TerrainPointWorld::new(
            &s,
            TerrainPointLimits {
                max_triangles: 1,
                ..limits()
            },
        )
        .unwrap(),
    ] {
        assert_eq!(
            w.trace_leaves(&[], &request(1), &mut ws, &mut r),
            Err(TerrainPointWorldError::WorkspaceMismatch)
        );
        assert_eq!(ws.generation(), 0);
    }
    r.fraction = 2.;
    assert_eq!(
        world.trace_leaves(&[], &request(1), &mut ws, &mut r),
        Err(TerrainPointWorldError::Trace(TraceError::InvalidFraction))
    );
    assert_eq!(ws.generation(), 0);
    r = initial();
    for generation in 1..=u16::MAX {
        assert_eq!(
            world
                .trace_leaves(&[], &request(1), &mut ws, &mut r)
                .unwrap()
                .generation,
            generation
        );
    }
    assert_eq!(
        world.trace_leaves(&[], &request(1), &mut ws, &mut r),
        Err(TerrainPointWorldError::UnresolvedGenerationWrap)
    );
    assert_eq!(ws.generation(), u16::MAX);
    assert_eq!(r, initial());
}
#[test]
fn selector_and_request_failures_are_typed_and_zero_fraction_skips_dispatch() {
    let s = snapshot(&[node(0, 0)], &[(0, 1, 1)], (0, 1, 1), false);
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    let mut r = initial();
    assert_eq!(
        world.trace_leaves(&[Leaf::Model(99)], &request(1), &mut ws, &mut r),
        Err(TerrainPointWorldError::InvalidReference)
    );
    assert_eq!(r, initial());
    assert_eq!(ws.generation(), 1);
    r.fraction = -0.;
    let before = r;
    assert_eq!(
        world
            .trace_leaves(&[Leaf::Map(99)], &request(1), &mut ws, &mut r)
            .unwrap()
            .tested_aabbs,
        0
    );
    assert_eq!(r, before);
    assert!(matches!(
        TerrainPointWorldRequest::new(Vec3::splat(f32::MAX), Vec3::splat(-f32::MAX), 1),
        Err(TerrainPointWorldError::Trace(
            TraceError::NonFiniteArithmetic
        ))
    ));
    assert!(matches!(
        TerrainPointWorldRequest::new(Vec3::ZERO, Vec3::ZERO, 1),
        Err(TerrainPointWorldError::Trace(
            TraceError::UnsupportedStationary
        ))
    ));
}

#[test]
fn nested_depth_first_and_child_equal_ties_retain_first_partition() {
    let mut root = node(1, 0);
    root.children = 2;
    let mut nested = node(3, 1);
    nested.children = 1;
    let s = snapshot(
        &[root, nested, node(0, 1), node(2, 1)],
        &[(0, 1, 1)],
        (0, 1, 1),
        false,
    );
    let world = TerrainPointWorld::new(&s, limits()).unwrap();
    let mut ws = world.new_workspace().unwrap();
    let mut r = initial();
    let step = world
        .trace_leaves(&[Leaf::Map(0)], &request(1), &mut ws, &mut r)
        .unwrap();
    assert_eq!(step.contact.unwrap().partition_ordinal, 2);
    assert_eq!(step.contact.unwrap().triangle_ordinal, 2);
    assert_eq!(step.tested_triangles, 2);
    assert_eq!(r.surface_flags, 11);
}
#[test]
fn independent_f32_sat_cancellation_witness_differs_from_f64_for_root_and_child() {
    // Scalar f32 source Z-cross left=8, right=10, admits. Exact promoted
    // products yield left~=11.9209, right~=10, rejecting a generic f64 test.
    let mut narrow = node(0, 0);
    narrow.origin = [1., f32::from_bits(0x3f800001), 0.];
    narrow.half = [0., 1e-7, 4.];
    let source_left = (1e8_f32 * (-narrow.origin[1]) - 1e8_f32 * (-narrow.origin[0])).abs();
    let source_right = 1e8_f32 * narrow.half[1];
    assert_eq!(source_left.to_bits(), 8_f32.to_bits());
    assert_eq!(source_right.to_bits(), 10_f32.to_bits());
    assert!(1e8_f64 * (f64::from(narrow.origin[1]) - 1.) > 1e8_f64 * f64::from(narrow.half[1]));
    let req = TerrainPointWorldRequest::new(Vec3::new(-1e8, -1e8, 2.), Vec3::new(1e8, 1e8, -2.), 1)
        .unwrap();
    let mut root = node(1, 0);
    root.children = 1;
    root.half = [1e8, 1e8, 4.];
    for nodes in [vec![narrow], vec![root, narrow]] {
        let s = snapshot(&nodes, &[(0, 1, 1)], (0, 1, 1), false);
        let world = TerrainPointWorld::new(&s, limits()).unwrap();
        let mut ws = world.new_workspace().unwrap();
        let mut r = initial();
        let step = world
            .trace_leaves(&[Leaf::Map(0)], &req, &mut ws, &mut r)
            .unwrap();
        assert_eq!(step.tested_triangles, 1);
        assert_eq!(step.tested_aabbs, nodes.len());
    }
}

#[test]
fn zero_fraction_stops_next_outer_root_but_tree_keeps_distinct_partitions() {
    let req = TerrainPointWorldRequest::new(Vec3::ZERO, -Vec3::Z, 1).unwrap();
    let mut root = node(1, 0);
    root.children = 2;
    for (nodes, want) in [
        (vec![node(0, 0), node(1, 0)], 1),
        (vec![root, node(0, 0), node(1, 0)], 2),
    ] {
        let root_count = if nodes.len() == 2 { 2 } else { 1 };
        let s = snapshot(&nodes, &[(0, root_count, 1)], (0, root_count, 1), false);
        let world = TerrainPointWorld::new(&s, limits()).unwrap();
        let mut ws = world.new_workspace().unwrap();
        let mut r = initial();
        let step = world
            .trace_leaves(&[Leaf::Map(0)], &req, &mut ws, &mut r)
            .unwrap();
        assert_eq!(step.admitted_partitions, want);
        assert_eq!(step.tested_triangles, want);
        assert_eq!(r.fraction.to_bits(), (-0_f32).to_bits());
        assert_eq!(step.contact.unwrap().partition_ordinal, 0);
    }
}
#[test]
#[ignore = "licensed external payload; point-family Rust reconstruction only, no original comparison"]
fn retained_all_map_model_leaf_point_probes_repeat_with_fresh_generations() {
    let path = std::env::var("BO2_TERRAIN_SNAPSHOT").expect("external terrain snapshot required");
    let bytes = std::fs::read(path).unwrap();
    let s = read_terrain_snapshot(&mut bytes.as_slice(), bytes.len() as u64).unwrap();
    let world = TerrainPointWorld::new(
        &s,
        TerrainPointLimits {
            max_leaves: 1,
            max_aabb_tests: 1_000_000,
            max_pending_nodes: 65_536,
            max_triangles: 100_000,
        },
    )
    .unwrap();
    let mut ws = world.new_workspace().unwrap();
    let mut observations = Vec::new();
    let mut hits = 0;
    let selectors = (0..s.map_leaves().len())
        .map(|i| Leaf::Map(i as u32))
        .chain((0..s.models().len()).map(|i| Leaf::Model(i as u32)))
        .collect::<Vec<_>>();
    for pass in 0..2 {
        for (i, &source) in selectors.iter().enumerate() {
            let leaf = match source {
                Leaf::Map(i) => &s.map_leaves()[i as usize],
                Leaf::Model(i) => &s.models()[i as usize].leaf,
            };
            let low = Vec3::from_array(leaf.mins_bits.map(f32::from_bits));
            let high = Vec3::from_array(leaf.maxs_bits.map(f32::from_bits));
            let center = (low + high) * 0.5;
            // Explicit numerical probe endpoints, not a player hull/loadout/timing default.
            let req = TerrainPointWorldRequest::new(
                center + 32. * Vec3::Z,
                center - 32. * Vec3::Z,
                u32::MAX,
            )
            .unwrap();
            let mut result = initial();
            let step = world
                .trace_leaves(&[source], &req, &mut ws, &mut result)
                .unwrap();
            let observed = (
                result,
                step.contact,
                step.tested_aabbs,
                step.tested_triangles,
                step.admitted_partitions,
            );
            if pass == 0 {
                hits += usize::from(step.contact.is_some());
                observations.push(observed);
            } else {
                assert_eq!(observed, observations[i]);
            }
        }
    }
    assert!(hits > 0);
    println!(
        "Rust point reconstruction: {} map/model leaves, {hits} contacts, two equal passes",
        selectors.len()
    );
}
