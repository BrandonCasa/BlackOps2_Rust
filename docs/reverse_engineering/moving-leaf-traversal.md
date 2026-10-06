# Moving leaf/global-node brush traversal

Fidelity: INVESTIGATING. This is a static source reconstruction and synthetic Rust validation, not an original comparison or complete player collision world.

## Evidence and reproduction

Use the licensed disk identity admitted in [stationary world contract](stationary-world-contract.md) and [global ownership](collision-global-ownership.md). File-only CFG disassembly follows direct branches, admits no instruction outside the bounded function span, and hashes bytes at the disk section mapping. The external task09 receipt records:

| RVA | Bytes | SHA256 |
|---|---:|---|
| 484250 | 892 | 59fe8980f1848dae875256cb92fa1099b22a1c78437fc48434f2b44b9be5f0d2 |
| 4845d0 | 318 | 7b2d620150e1e822ae12d79f1d1456d0e3157fc402a1b4f291c285e975cb79a9 |
| 484710 | 140 | 60f7eac85bd75e817da6b25b7df99a0457aa3c1e1d1cf7b0ee3d77cb07f04479 |
| 1831e0 | 206 | e5811bd4445f358d025bd42bc17cfd79684d0c73358d635c6a8a0f933ad6a272 |

The first three agree with the retained task0035 qualified bodies. The newly completed helper has no callees. Data-section mapping confirms VA be5924=-1 (bf800000), bfdd24=1 (3f800000), c4d19c=.125 (3e000000), be7360=2^-21 (35000000). Sign negation uses the sign-bit mask; delta magnitude clears that bit. External decoder/receipts remain outside Git. No licensed program was executed or inspected through a running process.

## Observed flow

Outer 484710 exits when incoming fraction equals zero, then gates leaf brush contents by request mask. It calls 4845d0 before any terrain consumer. That brush helper expands leaf bounds by stored axial support, using individual f32 subtract/add operations. Helper 1831e0 performs lower then upper passes with signs -1 then +1; axes X, Y, Z. It maintains entry=0 and exit=incoming fraction. Both endpoints outside a halfspace reject. Crossing entry >= exit rejects; exit <= entry rejects. Equality at an origin exiting a bound therefore rejects. All intermediate multiplication and subtraction orders are retained, rather than replacing the helper with a generic segment-AABB routine.

The traversal receives spatial start from request+80 and spatial end from request+90. Its start W is zero. Its end W is the *incoming result fraction*, not one: 4846d6 loads result+10 and 4846dc writes frame+30 (end vector frame+24, after five argument pushes). Spatial end is not shortened. Clipped four-vectors control traversal; terminal consumers still receive the original full request.

Every entered node first gates its contents by mask. Positive count visits u16 brush references in stored order, performs each brush mask gate, then invokes the accepted single-brush consumer. It does not stop on allsolid or zero fraction. Negative count first calls the adjacent node with the same segment, then resumes its split without an allsolid check.

For split axis, compute a=start-distance, b=end-distance, padding=(support+.125)-range. Select max using a-b >=0; select min using b-a >=0, preserving subtraction and signed-zero behavior. When min>=padding, return if -padding>=max, otherwise visit child0. Otherwise -padding>=max visits child1. These one-child/dead-zone branches precede the crossing-only startW>=result.fraction prune.

Crossing delta=b-a. If abs(delta)>2^-21, orient a by negating it when delta>=0, compute reciprocal=1/abs(delta), first-end ratio=(oriented a+padding)*reciprocal, second-start ratio=(oriented a-padding)*reciprocal; first child is1 for delta>=0, otherwise0. Small/equal delta uses first child0, ratios1 and0. Cap first-end ratio at1 through the source subtraction predicate; cap second-start at0 only. Interpolate all four components with separate subtraction, multiplication, addition. Visit first child recursively, then tail through the other child with the second start. Relative child offsets remain source order.

## Safe Rust boundary

`MovingBrushOnly` is explicitly a brush-only leaf subset. Construction checks hierarchy/collection identity declarations, side-plane bindings and required geometry references, builds a keyed provider lookup that permits unsorted collection records, and reserves bounded reusable scratch. The caller independently authenticates payload bytes and establishes scene/leaf ownership; declarations are not authentication.

Public leaf records receive finite/ordered bounds validation only after zero-fraction and leaf-mask gates. Missing participating roots/providers and invalid arithmetic are errors. Every error preserves all authoritative incoming fields; workspace resets on every request. Limits are explicit safety capacities, not authentic timing or map defaults. Scratch capacity may reject before native recursion would perform work. No query allocation is performed. Finite checks are deliberate safe Rust rejection of invalid input, not recovered native error behavior.

The implementation reuses the existing moving brush consumer, including inside-contact contents sourced from axial flags and existing entry/tie writes. `first_overlap` is an adapter diagnostic SourceKey, not a recovered native owner field. Production lookup construction is kept local to the two bounded consumers pending primary-owned extraction; no shared API was edited by the worker.

Synthetic fixtures independently derive strict leaf crossings, one-child equality, negative adjacency, terminal continuation, a negative-padding dead zone, small-delta threshold, capsule support, entry ties, seeded flags and atomic late errors. The non-unit fraction fixture derives second-start W=.75*((2-.125)/4)=.3515625; a prior entry .4 permits a nested crossing, whereas fabricated endW=1 would prune it at .46875.

## Limits

Terrain, complete BSP enumeration/x87 classification, filtering004050, transformed/shared submodels, dynents, runtime entities, special box selection and alternate movement receiver are excluded from this named subset. Their participating scene references must resolve before assembling a full world. The effective alternate receiver factory gap from [receiver producer](movement-collision-receiver-producer.md) remains explicit. There is no new movement hull/mask default, native invocation, full-map motor claim, or MATCHED/VERIFIED claim.
