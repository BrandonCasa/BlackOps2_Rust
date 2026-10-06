# Ordered moving point terrain aggregation

Fidelity: INVESTIGATING. This implements only point-family terrain over explicit ordered map/model leaf selections; it does not provide BSP enumeration or a player hull world.

## Source contract

Reuses [terrain numerical point](terrain-point-triangle.md), [terrain traversal](terrain-trace-contract.md), [workspace/builder](terrain-query-workspace.md), and admitted immutable terrain arrays. Complete482ff0/482f10 and005b690 were reread before coding; complete484710 resolves the outer zero-fraction behavior. Source spans/hashes remain in terrain01/04 receipts; this task adds no guessed source consumer.

Request endpoints are explicitly **already centered** source coordinates. Point aggregation computes displacement E-S, midpoint(E+S)*.5, signed half travel displacement*.5 and bit-cleared absolute half travel. It retains checked travel-square/length and swept-volume arithmetic from the finite builder domain. Point extent/radius/halfsegment are zero by the named subset; the source discriminator is point and its volume comparison derives cross-axis bypass false. This is not a hull default or a substituted capsule branch.

For an AABB, D=midpoint-origin and H=halfsize+point extent, computed through separate f32 operations. Axial rejection is abs(Di)>absoluteHalfTraveli+Hi. Three ordered cross-axis rejections use:

- abs(halfY*DZ-halfZ*DY) > absoluteHalfZ*HY+absoluteHalfY*HZ
- abs(halfZ*DX-halfX*DZ) > absoluteHalfZ*HX+absoluteHalfX*HZ
- abs(halfX*DY-halfY*DX) > absoluteHalfY*HX+absoluteHalfX*HY

All products/subtractions/additions retain scalar-f32 order, not fused/f64 or a segment slab replacement. Initial node and each child are tested. Equality proceeds. The SIMD XYZ mask only clears the unrelated fourth loaded lane; reader data already contains exactly XYZ.

Leaf terrain contents gates before outer AABB material contents. Children inherit the admitted outer material; their own material does not create another gate. Leaf root AABBs process ascending. Each internal node tests children in descending storage order and pushes accepted children into a bounded LIFO, producing ascending depth-first processing. Terminal partitions visit ordered triangles and share query stamps across all roots and leaf selectors.

**Zero fraction:**484710 checks at leaf entry and, separately, at484763/484773/484781 before each next outer terrain AABB wrapper call. It therefore stops the outer root loop after an on-plane zero hit. Complete482ff0 has no such check inside an already admitted tree; later terminal partitions still stamp/dispatch. Paired synthetic fixtures preserve this distinction.

Partition stamps update before triangle dispatch, including no-hit partitions. Partition identity promotes only when its entire consumer reduces fraction; outer material flags/contents promote only when its wrapper reduces fraction. Shared TraceResult remains unchanged structurally. The returned material/partition identity mirrors those promotion conditions; returned triangle is a diagnostic of the last strict triangle fraction reduction within the promoted partition, not a recovered native field. It is not an opaque edge-only or normal-only promotion substitute.

## API, workspace and failures

`TerrainPointWorldRequest::new(centered_start,centered_end,mask)` requires finite distinct endpoints and finite builder arithmetic; equal endpoints return UnsupportedStationary. The numerical point request inside it receives displacement, not the end coordinate.

`TerrainPointWorld::trace_leaves` accepts ordered `TerrainLeafSource::Map` and `::Model` ordinals resolved through the same immutable pool/leaf consumer. Model0 does not select BSP/world behavior here; transformations, runtime activation and pose selection remain outer assembly responsibilities. The caller chooses qualified leaf order, not entity-creation order.

A fresh workspace has generation0, all-zero partition stamps and preallocated bounded scratch. Its safe lifetime binds the exact snapshot and limits; different snapshot or limits reject without exposing host pointers. Mutable borrowing prevents simultaneous use of the same workspace. Generation1..65535 is the admitted fresh domain. The next request returns UnresolvedGenerationWrap before any generation/stamp/result mutation; no reset policy is selected.

Prevalidation is exactly workspace binding and incoming normal/fraction validity. Those errors reserve no generation. After prevalidation, generation reserves once for the whole selection list. Leaf/reference/cap/arithmetic failures **burn** that generation and any stamp writes already made, clear scratch, and leave every incoming authoritative result field unchanged. Retrying reserves the next distinct generation, so partial stamps cannot suppress the retry. This is explicit safe-adapter invalid-input behavior; native error semantics were not established. Allocation occurs only during workspace creation. Public limits bound selectors, AABB tests, pending nodes and triangles; immutable reader admission handles graph cycles and structural references first.

The snapshot's declared identity is not byte authentication or runtime ownership. The assembling caller authenticates payloads independently and binds the scene.

## Validation

Twelve synthetic tests cover strict masks, SAT equalities/cross axes, nested depth-first/tie ordering, inherited materials, cross-map/model shared partitions, seeded flags, zero-fraction root/tree distinction, caps/reference/error atomicity, workspace mismatch, retries and all65535 allowed generations plus exhaustion. A derived scalar-f32 SAT cancellation witness yields left8/right10 and admits, while promoted f64 gives left about11.9209/right10 and rejects; both initial and child paths are tested. This guards against generic higher-precision substitution.

Locked focused/full physics tests, warning-denying all-target physics Clippy and owned formatting checks pass. The retained opt-in runs two identical authoritative/diagnostic passes over all4791 map leaves and265 model leaves (5056 selectors), using explicit center+/-32Z numerical probes and all-bit mask. First pass reports63 promoted contacts; second outputs agree exactly excluding generation. Terrain payload SHA256 `e5ecb51a076898aeae66cfb6fd38ef79218768c29d3089279303e39801315a67` is independently checked before/after. External terrain05 receipt records these measurements. These are Rust reconstruction probes, not original comparisons, player defaults or full triangle-contact coverage.

## Remaining scope

Capsule/stationary terrain, reset/reload/scheduling beyond the fresh domain, packed-edge semantics, BSP selection, complete family dispatch, alternate movement receiver/solver, transformed activation/poses, motor and original comparisons remain outside this implementation. Participating terrain still cannot be excluded from player-world assembly merely because this point subset passes.
