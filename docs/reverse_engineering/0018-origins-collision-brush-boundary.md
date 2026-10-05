# Origins collision brush boundary

Historical Task0026 result, reviewed for documentation integration on 2026-10-05.
The investigation remains **PARTIAL**; accepting this record does not accept a
collision primitive. Later [Task0028](0019-origins-collision-brush-recovery.md)
identifies the failed predicate as `bounds.nonnull`, records absent parallel
contents and recovers one brush. The recommendations below describe Task0026's
closing state and are superseded by that work and the implemented
[moving-brush boundary](../architecture/moving-brush-trace.md).

Task0026 is **PARTIAL**. The second and final admitted original decoder attempt
reaches one selected clipmap's embedded ClipInfo admission, then rejects at stage3.
No brush is selected or emitted. Its plane equation, halfspace orientation,
vertices, endpoint bounds and parallel bounds therefore remain unmeasured.
This result supplies a specific access gate toward one inspectable collision
primitive; it does not support a Rust collision-data adapter yet.

Data/access understanding remains **INVESTIGATING**. Overall, gameplay and visual
fidelity remain **UNKNOWN**. The complete stable Rust with Bevy Origins objective
remains active. See the [handoff](../handoffs/0026-origins-collision-brush-boundary.md).

## Sources and interpretation boundary

The only original input is the already catalogued `zone/all/zm_tomb.ff`,
78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
The selected family identifier is `maps/mp/zm_tomb.d3dbsp`. Historic catalogue
context is Steam build24784239, English; this task does not refresh that context.
Configured roots are read only from ignored local configuration. All native
source, binaries, logs, helper code and possible original records remain external
under the task-relative area `0026-origins-collision-brush-boundary`.

Retained OpenAssetTools v0.33.0 source at commit
`7d027e8f89118196713e955b0e11f8404149c54d` supplies format hypotheses and decoder
access, not original runtime collision semantics. The inspected
[T6 declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1017)
and [clipmap loading rules](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/clipMap_t.txt#L40)
support the following abstract schema. These are third-party declarations,
not measured original field values.

| Relationship | Source interpretation | Unresolved original evidence |
| --- | --- | --- |
| Clipmap root | Embedded ClipInfo and separate pInfo pointer | Which provider is populated/consumed; ownership and equivalence |
| ClipInfo | Counted plane, side, vertex and brush arrays; parallel brush bounds and contents | Actual counts, pointer presence and complete allocation extents |
| Brush | Endpoint triples, opaque contents, side/vertex counts and pointers; six axial flag pairs | Actual finite endpoints and array joins |
| Side | One plane pointer and opaque flag fields | Plane-pool join and original consumer meaning |
| Plane | Normal triple, distance scalar, type/sign bits | Equation sign, orientation, normalization and semantics |
| Parallel Bounds | Midpoint and half-size triples, indexed by brush | Agreement with brush endpoints or another interpretation |

Declaration anchors are ClipInfo1021-1040, clipMap1064-1106,
plane3191-3198, side3207-3212, brush3240-3251 and Bounds3253-3257.
Generated native x86 structure tests report ClipInfo72, plane20, side12,
vertex12, brush96 and Bounds24 bytes. The brush/bounds array aliases request
128-byte allocation alignment; generated element loading advances by native
element size, not a guessed 128-byte brush stride.

Retained generated `clipmap_t_t6_load_db.cpp` shows counted shared arrays at
226-360, one reusable side plane pointer at373-393, and one reusable brush
side/vertex pointer at442-477. Public rules at64-76 do not give those brush
pointers per-brush loading counts. The proposed inspection consequently requires
joining each brush span to its independently counted ClipInfo array before
following a loader-created single pointer. Root info/pInfo loading is an existing
lead for the next admission investigation. It proves no runtime provider choice.

## Predeclared read and emit policy

First-attempt policyv3 was frozen before the original run. Second-attempt policyv4
adds fixed stage markers and controlled standard-exception rejection, reviewed
and explicitly admitted by the outer agent. Its fourteen-member fixed tuple is
`revision-v3/second-predecode-fixed-identities.json`, SHA256
`a1a51b51c4362f781a9a42f220b298eb9362ffcdd0c32379fbb62b5fceb07763`.
All fourteen byte/hash/mtime identities were checked immediately before opening
the original for the second session. No policy, source or tolerance changed after
that admission.

1. Run the existing signed, single-zone normal LIST route with no preload,
   signature bypass, wholesale dump or other original input. The native decoder
   loads the zone; its allocations can include other families. The observation
   adapter follows only the admitted selected clipmap family.
2. Select the sole matching, non-reference clipmap pool entry. Use embedded
   ClipInfo; pInfo equality is bookkeeping only, with no pInfo dereference.
3. Before array reads, reject negative plane count; require checked pointer/count
   byte spans for planes, sides and vertices capped at1,000,000 each, brushes
   capped at65,535, and parallel bounds/contents indexed by the brush count.
   Address arithmetic checks do not independently prove decoder allocation
   extents. Allocation validity remains decoder-trusted.
4. Consider ascending indices among at most32 metadata candidates. Select the
   first with1-64 sides,0-256 vertices, finite strictly positive asymmetric
   endpoint extents, and side/vertex spans joined to shared counted arrays.
   A zero-vertex/null span uses first=0 as absence bookkeeping, not an observed
   vertex index. Do not skip a selected contradiction to search another brush.
5. Admit every selected side's plane pointer into the counted plane pool before
   reading any plane body. Only then read the one brush, its sides/planes/vertices,
   parallel bounds/contents and opaque flags. Preserve original float32 bits and
   order externally; do not follow material names or other bodies.
6. Exclusively create one external record capped at65,536 bytes and emit at most
   8,192 adapter-summary bytes plus fixed markers. Accept only complete exit0,
   expected framing and a checked completed record. A failed/incomplete output
   file is never evidence merely because it exists.

Each observation uses the previously qualified own-process streaming supervisor:
120-second observed deadline,2,097,152 combined retained stdout/stderr bytes,
65,536-byte record cap, owned-child reaping and capture-thread cleanup. These
are logical supervision limits, not a hard native memory or process-tree bound.
The decoder name-opens its input independently of the held hashing handle.
Preservation is conditional on that name resolving the checked file during decode;
no held-handle decoder binding or transient-name proof is claimed.

## Geometric diagnostic and synthetic discrimination

The external analyzer preserves decoded float32 inputs and calculates in float64.
It compares four candidate nonaxial halfspaces: normal dot point minus distance
at most/at least zero, and normal dot point plus distance at most/at least zero.
Each uses six axial constraints derived from brush endpoints. Parallel midpoint
plus/minus half-size bounds are compared independently and never substituted.

The cumulative four-hypothesis budget is60,000 plane triples; unique candidate
vertices are capped at1,000. Although metadata selection admits64 sides, this
combined triple budget rejects more than39 sides before reconstruction. That is
an explicit policy limitation, not permission to change the selected brush.
Normalized determinant cutoff is1e-10. Diagnostic residual tolerance is1e-8
times the maximum of one, absolute distance and the sum of absolute coordinate
products; point deduplication/set tolerance is1e-7 times the maximum of one and
largest endpoint span. Exact residual/containment results remain separate.
Near-singular triples, finite arithmetic, affine rank, reconstructed extrema,
source-vertex containment and bidirectional vertex-set agreement are checked.
These tolerances are mathematical diagnostic choices, not physical/contact units.
Missing, interior, extra or incomplete stored vertices would limit the set-agreement
hypothesis rather than establish a complete original extreme-vertex set.

Twenty-five native arithmetic/span/finite-bit checks, five supervisor cases and
32 analyzer cases pass. Malformed counts, null/spanning pointers, overflow,
nonfinite planes, degeneracy and a discriminating asymmetric synthetic wedge
are included. An earlier wedge test failed because containment/extrema alone
admitted a redundant opposing-distance candidate; that failed helper/test receipt
is retained. Bidirectional vertex-set comparison fixed the discriminator before
any original observation, without changing tolerances after original data.

Seven cases additionally exercise the actual corrected production adapter
translation unit with actual OAT T6/pool/Zone definitions and own synthetic data:
valid bounded record, wrong scope, negative count, null side array, out-of-pool
plane, preexisting invalid output and no eligible candidate. All pass through
the same supervisor. Controlled rejections terminate at stages1,3,5 or8 as
expected. The valid wedge reaches stage9 and selects only the minus-distance,
at-most-zero geometric candidate. The outer agent independently reran these
seven cases before admitting the second original attempt. Synthetic success
does not establish original geometry or player collision.

## Actual original observations

| Attempt | Exit / duration | Captured bytes | Last stage / accepted record |
| --- | --- | --- | --- |
| First | 3221226505 (0xc0000409) /4.750s | 0 | No markers; no record |
| Second | 123 /1.109s | 401 (338 stdout,63 stderr) | 3; rejected, no record |

Neither run hits the deadline, capture cap or artifact cap. Owned child and capture
threads are cleaned; reported I/O/supervisor failures are absent. Observed peak
working sets are392,077,312 and391,581,696 bytes respectively; sampling is not a
hard bound or performance/fidelity comparison. The first failure has no attributable
native cause. Its exit alone proves neither signature acceptance nor a completed
decode or root admission.

The second run emits stages1,2,3 and BRUSH_REJECTED:3. Stage3 is a pre-read
admission marker after matching pool selection, before successful root-array
admission. The rejected predicate is **unknown** among the negative plane-count
check and six checked array count/presence/address-span checks. No individual
count/pointer value or predicate identifier was emitted. Stage4 was not reached;
no candidate metadata, selected side/plane, vertex or bounds read is established.
The result does not show no eligible brush, corrupt original data, parallel-bounds
contradiction, pInfo equivalence or original player-mask semantics.

Four same-handle full identity passes use64KiB blocks with128MiB/pass ceilings:
admission and preservation for each session. The initial two-pass allowance was
explicitly revised by the outer agent to four for the one reviewed diagnostic
repair. All four retained byte/SHA/mtime/device/file identities agree. Two decoder
attempts and four full hashes are consumed; there is no retry or fifth hash.
The decoder remains a separate name-open consumer as described above.

## Tool identities, preservation and evidence limits

Fresh copies/builds preserve the accepted warm tool. First production decoder
is4,767,232 bytes, SHA256
`5c9b46a93fa08596def8e61ecb151309c44990b12d561203a6c29a7267dcbd18`;
corrected production decoder is4,768,768 bytes, SHA256
`db9ef36dc8dd7609343942913ada239022aa19473d793bccfa307c2f35131342`.
Fresh MSBuild receipts exit0 in205.360s and201.703s. The corrected source/header,
observer, supervisor, analyzer, actual fixture/test/results and policy identities
are individually bound by the fourteen-member second tuple. Both native source/
header/binary versions and referenced public/generated loader files are retained
in the selected terminal closure. Remaining fresh dependency/build scratch is
explicitly excluded; its immutability is not claimed.

Selected reproducibility identities (filenames are external relative identifiers):

| Record | Bytes | SHA256 |
| --- | --- | --- |
| Final policy (`revision-v3/predeclared-policy-v4.md`) | 4997 | `f04a640fc7e700e2d2a8af4748b3cb87234eeb0df822722fd2afb7a8b2e32783` |
| Original observer (`revision-v3/observe-v2.py`) | 4907 | `cd749300ede8a9b7aedf29cee23085ee06b8ae95090992e3412871489472a149` |
| Streaming supervisor (`revision-v3/supervisor_v4.py`) | 4816 | `6b122c1b93e07e5f29218ec813f840f236236809e5361cd517ea76a62db89575` |
| Geometric analyzer (`revision-v3/analyzer.py`) | 7688 | `4f8727beb95a46038267334b9134f4cc72acd062de65fb61c2de0e703ef1ed12` |
| Corrected actual adapter (`source-v2/src/Unlinking/ContentLister/ContentPrinter.cpp`) | 6116 | `0fa7f5a31520ee87bf84a01e04b2cdaf48ed8f51716fc04ffd08fb91ae570273` |
| Public T6 declarations (`src/Common/Game/T6/T6_Assets.h`) | 197407 | `66f23295ee5e4545d6d7cae2a3f0fc266e9bea8d331c6b35e7fa5a7008a8cc91` |
| Generated root/brush loader (`build/src/ZoneCode/Game/T6/XAssets/clipmap_t/clipmap_t_t6_load_db.cpp`) | 29086 | `53a5ae90fc0c3c55cb1751728abe05576773e4bfead9002d5c91a5a28bf7426b` |

Task0025's worker43 terminal records,35 evidence records and parent34 records
are preserved with byte/hash/mtime identities. Directly reused accepted warm
records remain unchanged. The accepted warm executable's SHA matches its retained
accepted identity, but its initial mtime was accidentally omitted from this task's
first baseline. That omission does not permit an initial-mtime preservation claim.

The first predecode freeze is **14/15 intact**: a copied v2 build wrapper
overwrote the frozen arithmetic-selftest executable. Its original version is
irrecoverable. The discrepancy and offending wrapper are retained; no restoration
or timestamp fabrication occurred. First source,25-check receipt and production
decoder provenance remain separately reviewable. Later concrete wrappers preflight
every output/log/object/executable against frozen paths and new revision-area
containment using explicit rejecting predicates. Five focused path cases pass.
The second fourteen-member tuple remains intact.

Other preserved failures include the wrong-case public loader-rule lookup,
environment-wrapper quoting, wrong fresh binary output assumption, initial
synthetic wedge discriminator and actual-fixture compilation without the x86
macro. These are preparation failures, not additional original attempts. Path-bearing lookup/freeze and closure path-alias tracebacks reached tool output;
their privacy limitation
is recorded externally and no private path is copied into tracked documents.
Policyv1/v2 historical bytes/hash/mtime were not frozen before replacement;
retained reconstructions are labeled accordingly. Policyv3's actual pre-original
identity and second policyv4 tuple remain distinct from those reconstructions.

## Boundary and next measurement at task close

No collision abstraction or Rust/Bevy API is approved from this task. A later
minimal safe owned primitive would require admitted original finite planes,
explicit halfspaces, independent bounds checks and validated bounded index joins,
with opaque metadata preserved. None of those one-primitive observations exists
yet. Existing [snapshot](../architecture/diagnostic-geometry-snapshot.md),
[Mesh](../architecture/diagnostic-mesh-adapter.md) and
[render](../architecture/diagnostic-geometry-render.md) boundaries remain unchanged.

The next bounded investigation should discriminate the **single stage3 root
admission gate**, using unique predicate tags and scalar/count/null/checked-arithmetic measurements
only, with no brush dereferences. Compare embedded ClipInfo with the already
inspected loader's separate pInfo relationship. Do not guess
provider ownership or proceed to player implementation. The milestone remains
one inspectable original collision primitive supporting a later standing movement
slice, independent of camera access. Units, world/gravity basis, capsule dimensions,
speed, simulation rate, masks, contact epsilon, step/slide and live behavior all
remain unresolved. No original-game comparison, Rust change, capture, gameplay,
complete map import or calibrated scene occurs. Unchanged74-test Cargo/GPU/inspector
gates are intentionally skipped for this documentation-only task.
