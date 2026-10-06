# Terrain trace family: static source contract

Movement terrain task 01, 2026-10-05. Source interpretation is **INVESTIGATING**;
player/full-map fidelity remains **UNKNOWN**. This investigation reads licensed
disk files and retained external metadata only. No game/process access, playback,
capture, extraction run or Rust collision implementation occurs.

## Source and reproducible qualification

The disk executable identity is SHA256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`,
13,090,072 bytes, preferred image base `0x400000`. Its admitted retained-code
association is documented in [native movement](movement-native-contract.md),
not proof of unmodified Steam equivalence. Resolve private locations through
ignored local configuration and the external task receipt.

External `movement-terrain-01/native_qualify.py` maps each instruction through PE
sections, follows direct reachable branches until returns, rejects overlapping
instruction boundaries and bounds each entry to 64 KiB/12,000 instructions.
Calls are followed separately within this family. Raw instructions/disassembly
remain external. All seventeen bodies have no unresolved indirect jumps or
out-of-bound branch tails. Separate disk reread checks every instruction byte;
before/after whole-image hashes agree. Hashes below cover concatenated reachable
instruction bytes in RVA order, excluding unreachable alignment bytes; they are
not contiguous-function span hashes.

| Entry RVA | Instruction bytes | Qualified role |
| --- | ---: | --- |
| `0x05b690` | 90 | Moving AABB material gate/wrapper |
| `0x0e9e20` | 742 | Stationary leaf terrain loop/material gate |
| `0x482ff0` | 1,020 | Moving AABB hierarchy traversal |
| `0x483400` | 608 | Stationary AABB hierarchy traversal |
| `0x482f10` | 213 | Moving partition/triangle dispatch |
| `0x482e40` | 205 | Stationary triangle dispatch/result writes |
| `0x1d1230` | 588 | Moving point/triangle consumer |
| `0x481940` | 1,480 | Moving nonpoint triangle/edge/vertex dispatch |
| `0x481310` | 40 | Packed edge-bit lookup |
| `0x481340` | 1,012 | Moving edge contact classification/result writes |
| `0x481740` | 498 | Moving vertex contact/result writes |
| `0x4821a0` | 314 | Stationary point/triangle squared distance |
| `0x481f10` | 649 | Closest point on triangle |
| `0x482680` | 1,984 | Stationary vertical segment/triangle overlap |
| `0x4822e0` | 917 | Segment distance helper |
| `0x07f8d0` | 136 | Normalization helper |
| `0x74af80` | 15 | SIMD XYZ-mask initializer |

Total: 10,511 qualified instruction bytes. Full per-entry SHA256 and branch/call
receipts are in external `qualification-receipt.json` and
`all-functions.private.receipt.json`. The two entry hashes are
`b5a72a8ea70640a778dfec58ba05dcc2435b76cd450553afdd9b4fa5934b987a`
and `964de322c49941d2c060727f5ce31c210aac0266df14c54a98754f68b90f1c0c`.

## Storage and eligibility

OAT revision `7d027e8f89118196713e955b0e11f8404149c54d` T6 declarations,
clipMap loader rules, the task0034 external producer and native consumers agree
on this separate collision-data chain; rendered triangles are unrelated providers:

| Provider | Native storage/relationship | Required reader admission |
| --- | --- | --- |
| Collision materials | Ordered 12-byte records; AABB u16 ordinal; flags at +4, contents at +8 | Preserve opaque u32 flags/contents; every referenced ordinal resolves. No material-name strings needed. |
| AABB tree | Ordered 32-byte records; float3 origin/halfsize, u16 material/child count, i32 union reference | Finite nonnegative halfsize; checked child span or partition reference, no cycles; retain every declared record. |
| Partitions | Ordered 16-byte records; u8 triangle count and i32 first triangle at +4 | Checked contiguous triangle span; preserve remaining declared fields as metadata until separately consumed. |
| Triangles | Ordered six-byte u16 triplets | Every vertex reference resolves; original order retained, no welding/retriangulation. |
| Collision vertices | Ordered float3, 12-byte stride | Finite source-space bit-preserved values. |
| Packed edges | One bit per triangle edge; loader bytes `4*ceil(3*triangleCount/32)` | Checked packed length and native-addressed bit range; preserve bits including padding. |
| Leaf owners | Ordered first-AABB/count, independent terrain contents | Every participating leaf span resolves, including root-zero brush leaves. |

Native global pools are shared with clipMap storage: vertices at global VA
`0x25038f0`, triangles `0x25038f8`, packed edges `0x25038fc`, partitions
`0x2503904`, AABBs `0x250390c`, materials `0x2503894`.
These are source identity anchors, never persisted host pointers. Their loader-to-
runtime ownership assignment remains the collision ownership task's responsibility.

Moving leaf admission first checks leaf terrain contents, then each AABB wrapper
checks its material contents against the request mask. Stationary leaf uses the
same separate material gate. AABB descendants inherit the admitted outer material;
the hierarchy bodies do not recheck child material contents. Every participation
decision must retain those two mask levels rather than use render material flags.

## Traversal, duplicate suppression and broadphase

Leaf AABB references are visited ascending. Moving hierarchy uses a local LIFO
stack: it tests children in descending storage order and pushes accepted children,
so processing is ascending depth first. The initial node is also broadphase-tested.
Stationary hierarchy recurses over ascending child storage order and stops a child
loop when result allsolid is set. Moving traversal has no equivalent allsolid
early-out in its tree body. Terminal partitions visit triangles ascending.

Both terminal routes require request+`0xd0` to point at a u16 partition stamp array,
compared against request+`0xcc`. Null storage skips a partition in the native body;
an equal stamp suppresses duplicate work. Otherwise the stamp is written before
triangle dispatch. This is a query-workspace prerequisite, never a source-backed
terrain exclusion. A safe implementation needs explicit fresh initialization,
complete partition coverage and wrap/reset policy; the initializer/generation
lifecycle is not established here. Shared partitions are consumed once in the
original encounter order, even when reached from multiple AABBs/material owners.

Broadphase tests the query midpoint/half-travel/extent against AABB center/halfsize
on three axial and, unless request+`0xbd` bypasses them, three cross-product axes.
Each rejects only for strict greater-than separation; equality proceeds. Preserve
scalar-f32 operation order. Request construction/field meanings come from the
[existing builder](0024-original-brush-trace-consumer.md), not a generic sweep.
The moving stack has a bounded local backing region without an observed capacity
guard; a Rust reader/query must reject excessive pending nodes explicitly.

The XYZ SIMD mask initially looked unavailable because its storage is zero-fill
BSS. Complete initializer `0x74af80` copies disk constant RVA `0x8789f0` with words
`[0xffffffff,0xffffffff,0xffffffff,0]`; its address occurs in a non-executable
initializer slot RVA `0x76a878`. Thus the masking preserves XYZ bits and clears
the fourth loaded lane. This is source initialization evidence, not a live read.

## Numerical consumers and result writes

The point subset is selected by request+`0xbc`; it uses the existing centered start
and displacement. Its triangle normal follows stored winding, and rejects motion
with normal-dot-displacement `>=0`. Plane and edge predicates are inclusive at
triangle edges/vertices. Admission requires strictly earlier fraction than the
incoming contact; equality preserves the earlier contact. Starts behind the face
reject; an on-plane inward point can write signed zero fraction. It writes a
normalized normal, fraction and zero edge flag at result+`0x2c`; it does not set
startsolid/allsolid. There is no brush-style 0.125 entry allowance in this subset.

The nonpoint swept consumer normalizes the face, chooses the vertical capsule
endpoint according to normal Z sign, uses radius and half segment, then dispatches
face, edge and vertex contact paths. Native face/edge/vertex tolerances use
**0.125 raw units** (RVA `0x84d19c`, word `0x3e000000`). Vertex discriminant
rejection additionally uses f32 **0.001** (RVA `0x823aa0`, word `0x3a83126f`).
One correction factor is `(3-normalLengthSquared)*0.5`, not a generic exact
normalization substitute. Face/edge/vertex paths can modify normal, fraction,
allsolid and edge flag independently; they do not uniformly set startsolid.
An implementation must preserve each write path and incoming fields.

The packed-bit helper receives a triangle **partition-local ordinal** and edge
ordinal and addresses bit `3*ordinal+edge`; the caller initializes its ordinal to
zero and increments it, without adding partition.firstTriangle. Observed edge
ordinals are 2, 1 and 0 in the corresponding feature branches. Edge outcomes write
the bit or conjunction of feature bits to result+`0x2c`; some early returns write
only that flag, without a new fraction. The native data flow is explicit; the
conventional global-triangle interpretation of the loader's packed buffer does
not resolve this local-addressing peculiarity. Preserve both observations and
investigate the producer/meaning before claiming walkability fidelity.

Moving partition dispatch writes its partition ordinal at result+`0x34` only after
a strict fraction reduction. The outer moving AABB wrapper likewise writes
material surface flags/contents only after a strict fraction reduction. An
edge-flag-only update therefore must not overwrite contact material/partition.
This additional contact identity field requires shared-contract review.

Stationary triangle dispatch chooses sphere distance for zero half segment and
vertical segment/triangle overlap otherwise. Sphere overlap uses strict
`radiusSquared > closestDistanceSquared`; exact tangency is clear. The nonzero
segment and segment-distance helpers deliberately mix separately rounded f32
vector operations with promoted f64 products, comparisons/division and narrowed
f32 outputs. All-f32 or all-f64 generic distance code would change the contract.
On overlap dispatch writes startsolid, allsolid and positive-zero fraction; it
preserves normal and edge flag. Stationary leaf writes material flags/contents
when result startsolid is set after an admitted material's broadphase/consumer
route. This checks the current flag, rather than proving a newly generated
contact; preexisting incoming flags must remain explicit.

## Payload availability and implementable prerequisites

Rehashed schema33 (`6b999dba22db62c98a22c469eaad9d4ee7441b7ee10c058405b74f39221e67b3`)
retains leaf spans/counts; schema34
(`3c171b4f18ce65eeec0cd8f2d1a1f2c19bc15e192597e9a9cc52c317e2d51fb6`)
retains brush geometry. Neither contains terrain vertices, triangles, packed
edges, partition records, AABB records or collision materials. All map-leaf AABB
ranges are within declared count 51,129. The 214 root-zero terrain leaves reference
253 distinct AABBs, rechecked here; they remain **UNRESOLVED_MISSING_PAYLOAD**,
never excluded. Declared totals remain 67,196 triangles and 18,968 partitions.

Task0034's external OAT ContentPrinter receives a decoded `clipMap_t` and already
reads the terrain header counts while emitting only brush payloads. Its retained
producer/build helper and generated clipMap loader support a subsequent bounded
external terrain producer from the same identity-bound fastfile: 78,753,344 bytes,
SHA256 `663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
That producer has not been built/run by this task. No claim of terrain payload
recovery or complete provider admission is made.

An external versioned terrain payload should bind source/membership identities
and every ordered array above, encode floats as exact words, omit names/pointers,
and validate complete reference graphs, packed sizes and output caps before
publication. Scene-owner admission remains separate. After reviewing this contract,
the point-triangle numerical subset, ordered traversal and reader can be separate
increments. Complete capsule numerical transcription/testing, stamp initialization,
packed-edge interpretation and provider binding remain required before player
terrain integration; the structural findings alone do not authorize a generic
triangle/capsule implementation.

## Validation and limits

Seventeen complete reachable bodies/10,511 instruction bytes, branch boundaries,
separate disk rechecks, mask initializer, constants and retained identity/range
checks pass. Independent scalar-f32 point reconstruction agrees with exact
rational geometric predicates on ten interior/backface/edge/vertex/outside/
parallel/earlier-tie/on-plane/behind cases; five flat-face sphere radius/tangency
cases pass. These fifteen cases cover narrow subsets, not capsule edge/vertex
arithmetic or native execution. External receipts retain actual results.
No Rust changes: workspace Cargo gates are not rerun for these two documents.
Original comparisons, complete world/family coverage, live stamp lifecycle and
unmodified Steam equivalence remain unverified.
