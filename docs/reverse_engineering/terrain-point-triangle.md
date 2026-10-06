# Moving terrain point triangle consumer

Fidelity: INVESTIGATING. This is the scalar numerical subset selected by native point dispatch, not a complete terrain or player-world query.

## Qualified source

The disk identity and storage evidence are inherited from [terrain contract](terrain-trace-contract.md) and [payload admission](terrain-payload-admission.md). File-only bounded CFG decoding requalified these complete bodies; extracted bytes also match the retained terrain01 instruction-byte receipts exactly:

| RVA | Bytes | SHA256 |
|---|---:|---|
| 1d1230 | 588 | 05ea4c69ace825670f1fa63772318e5bbfe1cf2e4e180bb925db263f12e407a7 |
| 07f8d0 | 136 | de3c68a3a33303fb95e12053c6e6ff5ed3bb9d47ec8e8d616f4bd13d9e0d4ab5 |
| 74af80 | 15 | ebbd56eb61b795108abb972005dcc8a1d2467a7506a80c6b9df59a0ce548e7f5 |

Initializer74af80 copies the mask constant `[ffffffff,ffffffff,ffffffff,00000000]` at RVA8789f0 to SIMD BSS storage; its initializer pointer is present at RVA76a878. Rechecked normalization constant VA d1f3ec is f32 one. These initialization facts do not authorize inserting SIMD masking into the point consumer: 1d1230 and its normal-writing helper use scalar SSE operations only. The helper returns length through x87, but this caller immediately discards that return. No x87 numerical result enters the retained point output.

External terrain03 qualification-receipt.json records three spans, hashes, exact retained comparisons, mask/slot/one bytes and no native execution. Independent-cases.json retains only synthetic input/output measurements. Original binary, decoder outputs and local paths remain external.

## Observed arithmetic

Triangle ordinal resolves the ordered u16 triplet and native global collision vertex pool. Let vertices be A,B,C. Construct E1=A-B and E2=A-C through separate f32 subtraction, normal=E2 cross E1 through separate product/subtract operations. Compute D=displacement dot normal with X then Y then Z addition. D>=0 rejects, so degenerate triangles and zero displacement reject before normalization.

Offset=A-start. Plane value P uses Y product plus X product, then Z product. P>0 rejects starts behind the oriented face. IncomingFraction*D>=P rejects; equality keeps the earlier contact. Side=displacement cross Offset. U=Side dot E1 and V=Side dot E2 each sum Y then X then Z. Reject U>0, D>U, 0>V, or D>(U-V). The inequalities admit face edges and vertices exactly; no brush entry allowance or static-model tolerance is inserted.

On admission helper07f8d0 computes rounded Xsquare+Ysquare, then Zsquare, then scalar f32 sqrt. It uses length as denominator when negative length compares below zero, otherwise one. Each normal component multiplies by separately rounded reciprocal1/denominator. A nonzero normal whose square sum underflows to zero therefore remains tiny, rather than being normalized with a generic vector method or rejected as degenerate. Final fraction=P/D is scalar f32 and may be negative zero for an on-plane inward start.

Only result normal XYZ, fraction, and edge flag byte2c=0 are written. Inherited startsolid/allsolid, contents and surface flags remain unchanged. Native writes flag2c before normalization; the safe wrapper computes into local state and commits everything together only on success.

## Rust boundary and checks

`TerrainPointRequest::new(start, displacement)` requires explicit finite vectors; its second argument is a **displacement**, not an endpoint. Conversion from source hull/request builders remains a separate assembling responsibility. `trace(snapshot, triangle_ordinal, incoming)` resolves immutable admitted terrain identities and performs this per-triangle subset. It returns Hit with the requested triangle ordinal or NoHit. Missing triangle/vertex and nonfinite inputs/arithmetic return typed errors atomically. Vertex-reference failure is defended at query time even though private snapshot admission already ensures complete triplets. Identity/scene ownership and actual payload authentication remain caller responsibilities.

Finite arithmetic checks reject invalid inputs safely rather than reproducing native NaN propagation. They do not silently clamp or replace source output. On every successful no-hit, incoming state is unchanged. Entry fraction ties retain earlier state. No masks, triangle stamps, materials, partition outputs, bounds tests or aggregation are inferred here.

Two independent nonaxis synthetic fixtures were evaluated with IEEE754 f32 pack/unpack after each primitive and scalar square-root rounding. Exact output normal/fraction words are preserved in Rust tests, alongside analytic flat-face/edge/vertex/backface/parallel/on-plane cases, winding, explicit displacement misuse, duplicate ties, degenerate vertices, underflow guard and early/late atomic failures. These establish reconstruction within their evidence limits, not native execution or MATCHED/VERIFIED fidelity.

## Remaining coverage

Terrain AABB/leaf/partition aggregation, duplicate stamps and lifecycle, material masks/owner writes, capsule face/edge/vertex consumers, stationary terrain, packed-edge interpretation and complete scene binding remain unresolved assembly requirements. This per-triangle point subset does not permit excluding participating terrain from player queries or claiming full-map collision.
