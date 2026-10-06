#![forbid(unsafe_code)]
use bo2_assets::{
    brush_collection::{BrushCollectionSnapshot, read_brush_collection},
    collision_hierarchy::{CollisionHierarchy, MAGIC, read_collision_hierarchy},
    terrain_snapshot::TerrainLeaf,
};
use bo2_physics::{
    Vec3,
    moving_brush_trace::{TraceError, TraceResult},
    stationary_brush_trace::StationaryBrushRequest,
    stationary_leaf_trace::{
        StationaryBrushOnly, StationaryLeafError, StationaryLeafOutcome, StationaryTraversalLimits,
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
}
fn terminal(first: u32, count: i16, contents: u32) -> Node {
    Node {
        first,
        count,
        contents,
        children: [0, 0],
        distance: 0.,
    }
}
fn branch(count: i16, children: [u16; 2]) -> Node {
    Node {
        first: 0,
        count,
        contents: u32::MAX,
        children,
        distance: 0.,
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
        w(&mut b, 0);
        half(&mut b, n.children[0]);
        half(&mut b, n.children[1]);
        w(&mut b, n.first);
    }
    for i in indices {
        half(&mut b, *i);
    }
    read_collision_hierarchy(&mut b.as_slice(), b.len() as u64).unwrap()
}
fn collection(
    order: &[u32],
    contents: [u32; 3],
    reject0: bool,
    overflow0: bool,
) -> BrushCollectionSnapshot {
    let records=order.iter().map(|&id|{let lo:f32=if id==0&&reject0{100.}else{-1000.};let hi:f32=if id==0&&reject0{200.}else{1000.};let sides=if id==0&&overflow0{json!([{"plane_index":0,"cflags":0,"sflags":0,"normal_bits":[f32::MAX.to_bits(),0,0],"dist_bits":0,"type":3,"signbits":0}])}else{json!([])};
        json!({"provider_id":0,"brush":{"schema":34,"brush_index":id,"side_first":0,"vertex_first":0,"side_count":sides.as_array().unwrap().len(),"vertex_count":0,"contents":contents[id as usize],"parallel_bounds_present":false,"parallel_contents_present":false,"parallel_contents":null,"midpoint_bits":null,"halfsize_bits":null,"mins_bits":([lo.to_bits();3]),"maxs_bits":([hi.to_bits();3]),"axial_cflags":([0;6]),"axial_sflags":([0;6]),"sides":sides,"vertices_bits":[]}})
    }).collect::<Vec<_>>();
    let v = json!({"schema":34,"interpretation":"diagnostic_relative_negative_adjacent_zero_empty","source":{"bytes":1,"sha256":"00".repeat(32),"membership_bytes":1,"membership_sha256":"11".repeat(32)},"selection":{"status":"admitted","reached_nodes":3,"keys":order.iter().map(|id|json!({"provider_id":0,"brush_id":id})).collect::<Vec<_>>()},"coverage":{"models":0,"absent_model_providers":0,"box_excluded":true,"triangles":0,"partitions":0,"aabb_trees":0,"static_models":0,"dynents":([0;4])},"providers":[{"id":0,"planes":1,"sides":usize::from(overflow0),"vertices":0,"brushes":3,"optional_bounds_present":false,"optional_contents_present":false}],"record_count":records.len(),"records":records});
    let bytes = serde_json::to_vec(&v).unwrap();
    read_brush_collection(&mut bytes.as_slice(), bytes.len() as u64).unwrap()
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
fn request(x: f32, radius: f32, mask: u32) -> StationaryBrushRequest {
    StationaryBrushRequest::new(
        Vec3::new(x, 0., 0.),
        Vec3::splat(-radius),
        Vec3::splat(radius),
        mask,
    )
    .unwrap()
}
fn limits() -> StationaryTraversalLimits {
    StationaryTraversalLimits {
        max_node_visits: 100,
        max_pending_nodes: 100,
    }
}
fn owner(step: bo2_physics::stationary_leaf_trace::StationaryLeafStep) -> Option<u32> {
    step.first_overlap.map(|k| k.brush_id)
}
#[test]
fn terminal_source_order_wins_with_unsorted_provider_records_and_allsolid_stops() {
    let h = hierarchy(&[terminal(0, 3, u32::MAX)], &[1, 0, 2]);
    let b = collection(&[2, 0, 1], [1, 2, 4], false, false);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let mut result = initial();
    let step = q
        .trace_leaf(&leaf(0), &request(0., 0., u32::MAX), &mut result)
        .unwrap();
    assert_eq!(owner(step), Some(1));
    assert_eq!(step.tested_brushes, 1);
    assert!(result.allsolid && result.startsolid);
    assert_eq!(result.contents, 2);
    assert_eq!(result.fraction.to_bits(), 0);
    assert_eq!(
        result.normal.to_array().map(f32::to_bits),
        initial().normal.to_array().map(f32::to_bits)
    );
    assert_eq!(result.opaque_flag_2c, 255);
}
#[test]
fn split_comparisons_visit_child0_for_lower_greater_and_both_on_either_equality() {
    let h = hierarchy(
        &[
            branch(0, [1, 2]),
            terminal(0, 1, u32::MAX),
            terminal(1, 1, u32::MAX),
        ],
        &[0, 1],
    );
    let b = collection(&[1, 2, 0], [1, 1, 1], false, false);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    for (x, r, want) in [
        (1., 0., 0),
        (-1., 0., 1),
        (0., 0., 0),
        (-1., 1., 0),
        (1., 1., 0),
    ] {
        assert_eq!(
            owner(
                q.trace_leaf(&leaf(0), &request(x, r, 1), &mut initial())
                    .unwrap()
            ),
            Some(want)
        );
    }
    let b = collection(&[2, 1, 0], [1, 1, 1], true, false);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let step = q
        .trace_leaf(&leaf(0), &request(0., 0., 1), &mut initial())
        .unwrap();
    assert_eq!(owner(step), Some(1));
    assert_eq!(step.tested_brushes, 2);
}
#[test]
fn negative_count_processes_adjacent_before_split_children() {
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
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let step = q
        .trace_leaf(&leaf(0), &request(0., 0., 1), &mut initial())
        .unwrap();
    assert_eq!(owner(step), Some(2));
    assert_eq!((step.visited_nodes, step.tested_brushes), (2, 1));
    let b = collection(&[1, 2, 0], [1, 1, 0], false, false);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    assert_eq!(
        owner(
            q.trace_leaf(&leaf(0), &request(0., 0., 1), &mut initial())
                .unwrap()
        ),
        Some(0)
    );
}
#[test]
fn masks_gate_leaf_node_and_brush_and_real_root0_is_not_an_empty_sentinel() {
    let h = hierarchy(&[terminal(0, 2, 3)], &[0, 1]);
    let b = collection(&[2, 0, 1], [1, 2, 4], false, false);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let step = q
        .trace_leaf(&leaf(0), &request(0., 0., 2), &mut initial())
        .unwrap();
    assert_eq!(owner(step), Some(1));
    assert_eq!(step.tested_brushes, 1);
    let mut l = leaf(-1);
    l.brush_contents = 1;
    let mut result = initial();
    let step = q.trace_leaf(&l, &request(0., 0., 2), &mut result).unwrap();
    assert_eq!(step.outcome, StationaryLeafOutcome::MaskedOut);
    assert_eq!(result, initial());
    let mut inert = branch(0, [0, 0]);
    inert.contents = 0;
    let h = hierarchy(&[inert], &[]);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let mut result = initial();
    let step = q
        .trace_leaf(&leaf(0), &request(0., 0., 1), &mut result)
        .unwrap();
    assert_eq!((step.visited_nodes, step.tested_brushes), (1, 0));
    assert_eq!(result, initial());
}
#[test]
fn leaf_all_six_tangencies_reject_strictly_before_node_dispatch() {
    let h = hierarchy(&[terminal(0, 1, 1)], &[0]);
    let b = collection(&[0, 1, 2], [1, 1, 1], false, false);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    for axis in 0..3 {
        for sign in [-1., 1.] {
            let mut p = Vec3::ZERO;
            p[axis] = sign * 1000.;
            let r = StationaryBrushRequest::new(p, Vec3::ZERO, Vec3::ZERO, 1).unwrap();
            let mut result = initial();
            let step = q.trace_leaf(&leaf(0), &r, &mut result).unwrap();
            assert_eq!(step.outcome, StationaryLeafOutcome::BoundsRejected);
            assert_eq!(step.visited_nodes, 0);
            assert_eq!(result, initial());
        }
    }
}
#[test]
fn seeded_allsolid_obeys_continuations_without_invented_leaf_entry_earlyout() {
    let h = hierarchy(&[terminal(0, 2, 1)], &[0, 1]);
    let b = collection(&[2, 0, 1], [1, 1, 1], true, false);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let mut result = initial();
    result.allsolid = true;
    let before = result;
    let step = q
        .trace_leaf(&leaf(0), &request(0., 0., 1), &mut result)
        .unwrap();
    assert_eq!((step.tested_brushes, owner(step)), (1, None));
    assert_eq!(result, before);
    let mut ns = [branch(0, [1, 2]), terminal(0, 1, 2), terminal(1, 1, 1)];
    let h = hierarchy(&ns, &[0, 1]);
    let b = collection(&[0, 1, 2], [1, 1, 1], false, false);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let step = q
        .trace_leaf(&leaf(0), &request(0., 0., 1), &mut result)
        .unwrap();
    assert_eq!(step.tested_brushes, 0);
    assert_eq!(step.visited_nodes, 2);
    ns[0].count = -1;
    let h = hierarchy(&ns, &[0, 1]);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let step = q
        .trace_leaf(&leaf(0), &request(0., 0., 1), &mut result)
        .unwrap();
    assert_eq!(step.tested_brushes, 0);
    assert_eq!(step.visited_nodes, 2);
}
#[test]
fn query_validation_and_limits_are_atomic_and_workspace_recovers_after_errors() {
    let h = hierarchy(
        &[branch(0, [1, 2]), terminal(0, 1, 1), terminal(1, 1, 1)],
        &[0, 1],
    );
    let b = collection(&[0, 1, 2], [1, 1, 1], false, false);
    let mut q = StationaryBrushOnly::new(
        &h,
        &b,
        StationaryTraversalLimits {
            max_pending_nodes: 1,
            max_node_visits: 100,
        },
    )
    .unwrap();
    let mut result = initial();
    assert_eq!(
        q.trace_leaf(&leaf(0), &request(0., 0., 1), &mut result),
        Err(StationaryLeafError::PendingCapacity)
    );
    assert_eq!(result, initial());
    assert_eq!(
        owner(
            q.trace_leaf(&leaf(0), &request(1., 0., 1), &mut result)
                .unwrap()
        ),
        Some(0)
    );
    let mut q = StationaryBrushOnly::new(
        &h,
        &b,
        StationaryTraversalLimits {
            max_pending_nodes: 100,
            max_node_visits: 1,
        },
    )
    .unwrap();
    let mut result = initial();
    assert_eq!(
        q.trace_leaf(&leaf(0), &request(0., 0., 1), &mut result),
        Err(StationaryLeafError::NodeVisitLimit)
    );
    assert_eq!(result, initial());
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    for (l, error) in [
        (leaf(-1), StationaryLeafError::InvalidReference),
        (leaf(4), StationaryLeafError::InvalidReference),
        (
            {
                let mut l = leaf(0);
                l.mins_bits[0] = f32::NAN.to_bits();
                l
            },
            StationaryLeafError::NonFiniteLeaf,
        ),
        (
            {
                let mut l = leaf(0);
                l.mins_bits[0] = 2000_f32.to_bits();
                l
            },
            StationaryLeafError::InvalidLeafBounds,
        ),
    ] {
        let mut result = initial();
        assert_eq!(
            q.trace_leaf(&l, &request(0., 0., 1), &mut result),
            Err(error)
        );
        assert_eq!(result, initial());
    }
}
#[test]
fn arithmetic_failures_and_missing_construction_provider_are_explicit() {
    let h = hierarchy(&[terminal(0, 1, 1)], &[0]);
    let b = collection(&[0, 1, 2], [1, 1, 1], false, true);
    let mut q = StationaryBrushOnly::new(&h, &b, limits()).unwrap();
    let mut result = initial();
    assert_eq!(
        q.trace_leaf(&leaf(0), &request(100., 0., 1), &mut result),
        Err(StationaryLeafError::Trace(TraceError::NonFiniteArithmetic))
    );
    assert_eq!(result, initial());
    let b = collection(&[1, 2], [1, 1, 1], false, false);
    assert!(matches!(
        StationaryBrushOnly::new(&h, &b, limits()),
        Err(StationaryLeafError::Hierarchy(
            bo2_assets::collision_hierarchy::HierarchyError::MissingBrush
        ))
    ));
}
#[test]
#[ignore = "external identity-checked licensed payloads; Rust brush-only reconstruction, no original comparison"]
fn retained_stationary_leaf_brush_subset_replays_deterministically() {
    use bo2_assets::terrain_snapshot::read_terrain_snapshot;
    use std::{env, fs};
    let hb = fs::read(env::var_os("BO2_HIERARCHY_SNAPSHOT").expect("explicit external hierarchy"))
        .unwrap();
    assert_eq!(hb.len(), 689822);
    let h = read_collision_hierarchy(&mut hb.as_slice(), hb.len() as u64).unwrap();
    let bb =
        fs::read(env::var_os("BO2_BRUSH_COLLECTION").expect("explicit external schema34")).unwrap();
    assert_eq!(bb.len() as u64, h.identity().brush_payload.bytes);
    let b = read_brush_collection(&mut bb.as_slice(), bb.len() as u64).unwrap();
    let tb =
        fs::read(env::var_os("BO2_TERRAIN_SNAPSHOT").expect("explicit external terrain")).unwrap();
    assert_eq!(tb.len() as u64, h.identity().terrain_payload.bytes);
    let t = read_terrain_snapshot(&mut tb.as_slice(), tb.len() as u64).unwrap();
    h.validate_terrain(&t).unwrap();
    let mut q = StationaryBrushOnly::new(
        &h,
        &b,
        StationaryTraversalLimits {
            max_node_visits: 4_000_000,
            max_pending_nodes: h.brush_nodes().len() + 1,
        },
    )
    .unwrap();
    let mut hits = 0;
    // Explicit zero-radius probes at source leaf centers, not a player hull/profile.
    for leaf in t.map_leaves() {
        let lo = Vec3::from_array(leaf.mins_bits.map(f32::from_bits));
        let hi = Vec3::from_array(leaf.maxs_bits.map(f32::from_bits));
        let r =
            StationaryBrushRequest::new((lo + hi) * 0.5, Vec3::ZERO, Vec3::ZERO, u32::MAX).unwrap();
        let mut a = initial();
        let sa = q.trace_leaf(leaf, &r, &mut a).unwrap();
        let mut b = initial();
        let sb = q.trace_leaf(leaf, &r, &mut b).unwrap();
        assert_eq!(sa, sb);
        assert_eq!(a, b);
        hits += usize::from(sa.first_overlap.is_some());
    }
    assert!(hits > 0);
    println!(
        "Rust reconstructed {} stationary brush-only leaf probes, {} contacts, repeated exactly",
        t.map_leaves().len(),
        hits
    );
}
