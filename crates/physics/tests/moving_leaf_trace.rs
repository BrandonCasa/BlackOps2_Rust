#![forbid(unsafe_code)]
use bo2_assets::{
    brush_collection::{BrushCollectionSnapshot, read_brush_collection},
    collision_hierarchy::{CollisionHierarchy, MAGIC, read_collision_hierarchy},
    terrain_snapshot::TerrainLeaf,
};
use bo2_physics::{
    Vec3,
    moving_brush_trace::{MovingBrushRequest, TraceError, TraceResult},
    moving_leaf_trace::{
        MovingBrushOnly, MovingLeafError, MovingLeafOutcome, MovingTraversalLimits,
    },
};
use serde_json::json;
fn w(b: &mut Vec<u8>, v: u32) {
    b.extend(v.to_le_bytes());
}
fn half(b: &mut Vec<u8>, v: u16) {
    b.extend(v.to_le_bytes());
}
#[derive(Clone, Copy)]
struct Node {
    count: i16,
    contents: u32,
    children: [u16; 2],
    first: u32,
    distance: f32,
    range: f32,
}
fn terminal(first: u32, count: i16, contents: u32) -> Node {
    Node {
        first,
        count,
        contents,
        children: [0, 0],
        distance: 0.,
        range: 0.,
    }
}
fn branch(count: i16, children: [u16; 2]) -> Node {
    Node {
        first: 0,
        count,
        contents: u32::MAX,
        children,
        distance: 0.,
        range: 0.,
    }
}
fn hierarchy(nodes: &[Node], indices: &[u16]) -> CollisionHierarchy {
    let mut b = Vec::new();
    b.extend(MAGIC);
    w(&mut b, 1);
    w(&mut b, 0);
    b.extend((216 + 20 + nodes.len() as u64 * 24 + indices.len() as u64 * 2).to_le_bytes());
    for hash in [0, 0x11, 0x22, 0x33] {
        b.extend(1_u64.to_le_bytes());
        b.extend([hash; 32]);
    }
    for count in [
        1,
        nodes.len() as u32,
        indices.len() as u32,
        0,
        3,
        1,
        indices.len() as u32,
        0,
    ] {
        w(&mut b, count);
    }
    for bits in [f32::MAX.to_bits(), 0, 0, 0] {
        w(&mut b, bits);
    }
    b.extend([3, 0]);
    half(&mut b, 0);
    for n in nodes {
        b.extend([0, 0]);
        half(&mut b, n.count as u16);
        w(&mut b, n.contents);
        w(&mut b, n.distance.to_bits());
        w(&mut b, n.range.to_bits());
        half(&mut b, n.children[0]);
        half(&mut b, n.children[1]);
        w(&mut b, n.first);
    }
    for i in indices {
        half(&mut b, *i);
    }
    read_collision_hierarchy(&mut b.as_slice(), b.len() as u64).unwrap()
}
fn collection_impl(
    order: &[u32],
    contents: [u32; 3],
    reject0: bool,
    overflow0: bool,
    mode: u8,
) -> BrushCollectionSnapshot {
    let records=order.iter().map(|&id|{let lo:f32=if reject0&&(id==0||(id==1&&mode==2)){if mode==0{100.}else{-0.275}}else{-1000.};let hi:f32=if reject0&&(id==0||(id==1&&mode==2)){if mode==0{200.}else{0.}}else{1000.};let sides=if id==0&&overflow0{json!([{"plane_index":0,"cflags":0,"sflags":0,"normal_bits":[f32::MAX.to_bits(),0,0],"dist_bits":0,"type":3,"signbits":0}])}else{json!([])};
        json!({"provider_id":0,"brush":{"schema":34,"brush_index":id,"side_first":0,"vertex_first":0,"side_count":sides.as_array().unwrap().len(),"vertex_count":0,"contents":contents[id as usize],"parallel_bounds_present":false,"parallel_contents_present":false,"parallel_contents":null,"midpoint_bits":null,"halfsize_bits":null,"mins_bits":([lo.to_bits();3]),"maxs_bits":([hi.to_bits();3]),"axial_cflags":([0;6]),"axial_sflags":([0;6]),"sides":sides,"vertices_bits":[]}})
    }).collect::<Vec<_>>();
    let v = json!({"schema":34,"interpretation":"diagnostic_relative_negative_adjacent_zero_empty","source":{"bytes":1,"sha256":"00".repeat(32),"membership_bytes":1,"membership_sha256":"11".repeat(32)},"selection":{"status":"admitted","reached_nodes":3,"keys":order.iter().map(|id|json!({"provider_id":0,"brush_id":id})).collect::<Vec<_>>()},"coverage":{"models":0,"absent_model_providers":0,"box_excluded":true,"triangles":0,"partitions":0,"aabb_trees":0,"static_models":0,"dynents":([0;4])},"providers":[{"id":0,"planes":1,"sides":usize::from(overflow0),"vertices":0,"brushes":3,"optional_bounds_present":false,"optional_contents_present":false}],"record_count":records.len(),"records":records});
    let bytes = serde_json::to_vec(&v).unwrap();
    read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap()
}
fn collection(
    order: &[u32],
    contents: [u32; 3],
    reject: bool,
    overflow: bool,
) -> BrushCollectionSnapshot {
    collection_impl(order, contents, reject, overflow, 0)
}
fn entry_collection(
    order: &[u32],
    contents: [u32; 3],
    reject: bool,
    overflow: bool,
) -> BrushCollectionSnapshot {
    collection_impl(order, contents, reject, overflow, 1)
}
fn tied_collection(
    order: &[u32],
    contents: [u32; 3],
    reject: bool,
    overflow: bool,
) -> BrushCollectionSnapshot {
    collection_impl(order, contents, reject, overflow, 2)
}
fn leaf(root: i32) -> TerrainLeaf {
    TerrainLeaf {
        first_aabb: 0,
        aabb_count: 0,
        brush_contents: u32::MAX,
        terrain_contents: 0,
        mins_bits: [(-1000_f32).to_bits(); 3],
        maxs_bits: [1000_f32.to_bits(); 3],
        brush_root: root,
        cluster: -1,
    }
}
fn initial() -> TraceResult {
    TraceResult {
        normal: Vec3::new(-0., 8., 9.),
        fraction: 0.75,
        surface_flags: 71,
        contents: 72,
        startsolid: false,
        allsolid: false,
        opaque_flag_2c: 255,
    }
}
fn request(start: Vec3, end: Vec3, radius: f32, mask: u32) -> MovingBrushRequest {
    MovingBrushRequest::new(start, end, Vec3::splat(-radius), Vec3::splat(radius), mask).unwrap()
}
fn limits() -> MovingTraversalLimits {
    MovingTraversalLimits {
        max_node_visits: 100,
        max_pending_nodes: 100,
    }
}
#[test]
fn terminals_continue_after_inside_and_preserve_stored_order_unsorted_providers() {
    let h = hierarchy(&[terminal(0, 3, 7)], &[1, 0, 2]);
    let b = collection(&[2, 0, 1], [1, 2, 4], false, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let mut r = initial();
    let step = q
        .trace_leaf(&leaf(0), &request(Vec3::ZERO, Vec3::X, 0., 7), &mut r)
        .unwrap();
    assert_eq!(step.tested_brushes, 3);
    assert_eq!(step.first_overlap.unwrap().brush_id, 1);
    assert_eq!(r.contents, 0);
    assert!(r.allsolid && r.startsolid);
    assert_eq!(r.fraction, 0.);
    assert_eq!(
        r.normal.to_array().map(f32::to_bits),
        initial().normal.to_array().map(f32::to_bits)
    );
    assert_eq!(r.opaque_flag_2c, 255);
}
#[test]
fn one_sided_equality_and_crossing_have_native_order_and_fraction_pruning() {
    let h = hierarchy(
        &[branch(0, [1, 2]), terminal(0, 1, 1), terminal(1, 1, 1)],
        &[0, 1],
    );
    let b = collection(&[2, 1, 0], [1, 1, 1], false, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    for (start, end, want) in [
        (1., 2., 0),
        (-1., -2., 1),
        (0.125, 1., 0),
        (-0.125, -1., 1),
        (-1., 1., 1),
        (1., -1., 0),
        (0., 0., 0),
    ] {
        let mut r = initial();
        let s = q
            .trace_leaf(
                &leaf(0),
                &request(Vec3::new(start, 0., 0.), Vec3::new(end, 1., 0.), 0., 1),
                &mut r,
            )
            .unwrap();
        assert_eq!(s.first_overlap.unwrap().brush_id, want, "{start} -> {end}");
        // Crossing second child prunes only if it is itself a crossing node; terminal keeps running.
        assert_eq!(
            s.tested_brushes,
            if start.abs() < 0.125 || start * end < 0. {
                2
            } else {
                1
            }
        );
    }
}
#[test]
fn negative_adjacency_does_not_stop_after_allsolid_before_one_sided_split() {
    let h = hierarchy(
        &[
            branch(-1, [2, 3]),
            terminal(0, 1, 1),
            terminal(1, 1, 1),
            terminal(2, 1, 1),
        ],
        &[2, 0, 1],
    );
    let b = collection(&[1, 2, 0], [1, 1, 1], false, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let mut r = initial();
    let s = q
        .trace_leaf(&leaf(0), &request(Vec3::X, 2. * Vec3::X, 0., 1), &mut r)
        .unwrap();
    assert_eq!(s.first_overlap.unwrap().brush_id, 2);
    assert_eq!(s.tested_brushes, 2);
    assert_eq!(r.contents, 0);
}
#[test]
fn leaf_clip_strict_entry_limit_and_exit_origin_with_capsule_support() {
    let h = hierarchy(&[terminal(0, 1, 1)], &[0]);
    let b = collection(&[0, 1, 2], [1, 1, 1], false, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    for axis in 0..3 {
        for sign in [-1., 1.] {
            let mut start = Vec3::ZERO;
            let mut end = Vec3::ZERO;
            start[axis] = sign * 1001.;
            end[axis] = sign * 999.;
            let req = request(start, end, 0., 1);
            let mut r = initial();
            r.fraction = 0.5;
            let s = q.trace_leaf(&leaf(0), &req, &mut r).unwrap();
            assert_eq!(s.outcome, MovingLeafOutcome::BoundsRejected);
            r.fraction = 0.5001;
            assert_eq!(
                q.trace_leaf(&leaf(0), &req, &mut r).unwrap().tested_brushes,
                1
            );
            let mut r = initial();
            assert_eq!(
                q.trace_leaf(&leaf(0), &request(start, end, 1., 1), &mut r)
                    .unwrap()
                    .tested_brushes,
                1
            );
            start[axis] = sign * 1000.;
            end[axis] = sign * 1001.;
            let mut r = initial();
            assert_eq!(
                q.trace_leaf(&leaf(0), &request(start, end, 0., 1), &mut r)
                    .unwrap()
                    .outcome,
                MovingLeafOutcome::BoundsRejected
            );
        }
    }
}
#[test]
fn masks_root_zero_and_zero_fraction_gate_before_public_leaf_validation() {
    let h = hierarchy(&[terminal(0, 2, 3)], &[0, 1]);
    let b = collection(&[2, 0, 1], [1, 2, 4], false, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let req = request(Vec3::ZERO, Vec3::X, 0., 2);
    let mut r = initial();
    assert_eq!(
        q.trace_leaf(&leaf(0), &req, &mut r)
            .unwrap()
            .first_overlap
            .unwrap()
            .brush_id,
        1
    );
    let mut l = leaf(-1);
    l.brush_contents = 1;
    l.mins_bits = [f32::NAN.to_bits(); 3];
    let mut r = initial();
    assert_eq!(
        q.trace_leaf(&l, &req, &mut r).unwrap().outcome,
        MovingLeafOutcome::MaskedOut
    );
    l.brush_contents = 2;
    r.fraction = -0.;
    let before = r;
    assert_eq!(
        q.trace_leaf(&l, &req, &mut r).unwrap().outcome,
        MovingLeafOutcome::FractionExhausted
    );
    assert_eq!(r, before);
    let mut inert = branch(0, [0, 0]);
    inert.contents = 0;
    let h = hierarchy(&[inert], &[]);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let mut r = initial();
    assert_eq!(
        q.trace_leaf(&leaf(0), &req, &mut r).unwrap().tested_brushes,
        0
    );
    assert_eq!(r, initial());
}
#[test]
fn seeded_flags_do_not_add_terminal_early_out() {
    let h = hierarchy(&[terminal(0, 2, 1)], &[0, 1]);
    let b = collection(&[2, 0, 1], [1, 1, 1], true, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let mut r = initial();
    r.allsolid = true;
    let s = q
        .trace_leaf(&leaf(0), &request(Vec3::ZERO, Vec3::X, 0., 1), &mut r)
        .unwrap();
    assert_eq!(s.tested_brushes, 2);
    assert_eq!(s.first_overlap.unwrap().brush_id, 1);
}
#[test]
fn late_brush_failure_and_workspace_limits_are_atomic() {
    let h = hierarchy(&[terminal(0, 2, 1)], &[1, 0]);
    let b = collection(&[0, 1, 2], [1, 1, 1], false, true);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let mut r = initial();
    assert_eq!(
        q.trace_leaf(
            &leaf(0),
            &request(Vec3::new(2., 0., 0.), Vec3::new(3., 0., 0.), 0., 1),
            &mut r
        ),
        Err(MovingLeafError::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(r, initial());
    let h = hierarchy(
        &[branch(0, [1, 2]), terminal(0, 1, 1), terminal(1, 1, 1)],
        &[0, 1],
    );
    let b = collection(&[0, 1, 2], [1, 1, 1], false, false);
    let mut q = MovingBrushOnly::new(
        &h,
        &b,
        MovingTraversalLimits {
            max_pending_nodes: 1,
            max_node_visits: 100,
        },
    )
    .unwrap();
    assert_eq!(
        q.trace_leaf(&leaf(0), &request(-Vec3::X, Vec3::X, 0., 1), &mut r),
        Err(MovingLeafError::PendingCapacity)
    );
    assert_eq!(r, initial());
    assert_eq!(
        q.trace_leaf(&leaf(0), &request(Vec3::X, 2. * Vec3::X, 0., 1), &mut r)
            .unwrap()
            .tested_brushes,
        1
    );
    let mut q = MovingBrushOnly::new(
        &h,
        &b,
        MovingTraversalLimits {
            max_pending_nodes: 100,
            max_node_visits: 1,
        },
    )
    .unwrap();
    let mut r = initial();
    assert_eq!(
        q.trace_leaf(&leaf(0), &request(Vec3::X, 2. * Vec3::X, 0., 1), &mut r),
        Err(MovingLeafError::NodeVisitLimit)
    );
    assert_eq!(r, initial());
    let mut l = leaf(99);
    assert_eq!(
        q.trace_leaf(&l, &request(Vec3::ZERO, Vec3::X, 0., 1), &mut r),
        Err(MovingLeafError::InvalidReference)
    );
    l.mins_bits = [f32::NAN.to_bits(); 3];
    assert_eq!(
        q.trace_leaf(&l, &request(Vec3::ZERO, Vec3::X, 0., 1), &mut r),
        Err(MovingLeafError::NonFiniteLeaf)
    );
}

#[test]
fn nonunit_incoming_fraction_scales_segment_w_and_child_only_precedes_pruning() {
    // Initial split far ratio is (2-.125)/4=.46875. Native W=.75*.46875=.3515625,
    // below first brush entry .4. A fabricated W=1 would produce .46875 and prune.
    let mut nested = branch(0, [2, 3]);
    nested.distance = 1.;
    let h = hierarchy(
        &[
            branch(0, [1, 2]),
            nested,
            terminal(0, 1, 1),
            terminal(1, 1, 0),
            terminal(2, 1, 1),
        ],
        &[0, 2, 1],
    );
    let b = entry_collection(&[2, 1, 0], [1, 1, 1], true, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let mut r = initial();
    let req = request(-2. * Vec3::X, 2. * Vec3::X, 0., 1);
    let step = q.trace_leaf(&leaf(0), &req, &mut r).unwrap();
    assert_eq!(step.tested_brushes, 2);
    assert_eq!(step.first_overlap.unwrap().brush_id, 1);
    assert_eq!(r.fraction, 0.);
}

#[test]
fn negative_padding_dead_zone_and_small_delta_use_exact_native_gates() {
    let mut root = branch(0, [1, 2]);
    root.range = 0.25;
    let h = hierarchy(&[root, terminal(0, 1, 1), terminal(1, 1, 1)], &[0, 1]);
    let b = collection(&[0, 1, 2], [1, 1, 1], false, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let mut r = initial();
    assert_eq!(
        q.trace_leaf(&leaf(0), &request(Vec3::ZERO, Vec3::Y, 0., 1), &mut r)
            .unwrap()
            .tested_brushes,
        0
    );
    assert_eq!(r, initial());
    let h = hierarchy(
        &[branch(0, [1, 2]), terminal(0, 1, 1), terminal(1, 1, 1)],
        &[0, 1],
    );
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    for d in [0., f32::from_bits(0x35000000), -f32::from_bits(0x35000000)] {
        let mut r = initial();
        let s = q
            .trace_leaf(
                &leaf(0),
                &request(Vec3::ZERO, Vec3::new(d, 1., 0.), 0., 1),
                &mut r,
            )
            .unwrap();
        assert_eq!(s.first_overlap.unwrap().brush_id, 0);
        assert_eq!(s.tested_brushes, 2);
    }
}
#[test]
fn vertical_capsule_support_and_nonfinite_expansion_are_explicit() {
    let h = hierarchy(&[terminal(0, 1, 1)], &[0]);
    let b = collection(&[0, 1, 2], [1, 1, 1], false, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let req = MovingBrushRequest::new(
        Vec3::new(0., 0., 1002.),
        Vec3::new(1., 0., 1002.),
        Vec3::new(-1., -1., -4.),
        Vec3::new(1., 1., 4.),
        1,
    )
    .unwrap();
    let mut r = initial();
    assert_eq!(
        q.trace_leaf(&leaf(0), &req, &mut r).unwrap().tested_brushes,
        1
    );
    let mut l = leaf(0);
    l.maxs_bits = [f32::MAX.to_bits(); 3];
    let huge = request(Vec3::ZERO, Vec3::X, f32::MAX * 0.75, 1);
    let mut r = initial();
    assert_eq!(
        q.trace_leaf(&l, &huge, &mut r),
        Err(MovingLeafError::ArithmeticOverflow)
    );
    assert_eq!(r, initial());
}

#[test]
fn equal_entry_fractions_preserve_single_brush_writes_and_seeded_flags() {
    let h = hierarchy(&[terminal(0, 2, 1)], &[1, 0]);
    let b = tied_collection(&[2, 0, 1], [1, 1, 2], true, false);
    let mut q = MovingBrushOnly::new(&h, &b, limits()).unwrap();
    let req = request(-2. * Vec3::X, 2. * Vec3::X, 0., 1);
    let mut expected = initial();
    expected.startsolid = true;
    req.trace(
        b.records()
            .iter()
            .find(|r| r.key().brush_id == 1)
            .unwrap()
            .snapshot(),
        &mut expected,
    )
    .unwrap();
    let mut actual = initial();
    actual.startsolid = true;
    let step = q.trace_leaf(&leaf(0), &req, &mut actual).unwrap();
    assert_eq!(step.tested_brushes, 2);
    assert_eq!(step.first_overlap, None);
    assert_eq!(actual, expected);
    assert!((actual.fraction - 0.4).abs() < 1e-7);
    assert!(actual.startsolid);
    assert!(!actual.allsolid);
}
