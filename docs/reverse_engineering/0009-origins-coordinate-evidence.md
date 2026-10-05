# Origins coordinate-boundary evidence

Recorded: 2026-10-05. Task 0015 asks which world-coordinate conversion is
justified for the accepted Origins XYZ/index candidate before graphical
placement in Bevy. **Result: no original-to-Bevy conversion is established.**
The investigation is **PARTIAL**: destination conventions and representational
ambiguity are established, while original units, gravity/up, angle semantics,
basis orientation and triangle front state remain unresolved. Data understanding
is **INVESTIGATING**; overall, gameplay and visual fidelity remain **UNKNOWN**.
This does not narrow or complete the full Origins goal.

## Inputs and evidence classes

The only new licensed-data read is the already accepted external
`0013-origins-diagnostic-geometry-snapshot/observations-v2/production1/diagnostic-v1.bo2dgs`:
4,404,568 bytes, SHA-256
`ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f`.
[Findings 0008](0008-origins-diagnostic-geometry-snapshot.md) ties its complete
raw XYZ/reference preservation to the unchanged `zone/all/zm_tomb.ff`, Steam
Zombies build `24784239`, English, selected `maps/mp/zm_tomb.d3dbsp` family.
That original container is 78,753,344 bytes, SHA-256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`;
it is neither reopened nor decoded by this task.

Read [directory findings 0004](0004-origins-asset-directory.md),
[surface findings 0005](0005-gfxworld-surface-boundary.md),
[stream findings 0006](0006-gfxworld-vertex-stream-boundary.md),
[prefix findings 0007](0007-gfxworld-position-prefix.md),
[handoff 0011](../handoffs/0011-gfxworld-position-prefix.md),
[0012](../handoffs/0012-diagnostic-geometry-snapshot-reader.md),
[0013](../handoffs/0013-origins-diagnostic-geometry-snapshot.md),
[0014](../handoffs/0014-bevy-diagnostic-mesh-adapter.md),
the [snapshot contract](../architecture/diagnostic-geometry-snapshot.md) and
[Mesh adapter boundary](../architecture/diagnostic-mesh-adapter.md).
Finite triples, complete accounting and own-surface containment support the
numeric candidate, without establishing its physical or rendering conventions.

Evidence classes in this document are deliberately distinct:

- **Direct:** repeatable measurements of the accepted snapshot, or literal
  behavior of an inspected public tool's source.
- **Source-supported:** the pinned decoder's declarations/loading rules and the
  pinned destination engine's documented implementation.
- **Hypothesis:** transferring an export policy, familiar COD convention, label
  or plausible shape to the licensed original's runtime semantics.
- **Application policy:** a later choice of Bevy world units, world basis,
  transforms and rendering configuration. No such conversion is selected here.

## Pinned source tracing

Bevy `0.19.1` cached crate VCS records identify immutable commit
`b56fc29d3016e641754765244b5ba3f9cc504671`. The three inspected cached source
files match bytes fetched from that commit. These are **destination facts**:

| Pinned primary source | Supported fact and limit |
| --- | --- |
| [Transform basis](https://github.com/bevyengine/bevy/blob/b56fc29d3016e641754765244b5ba3f9cc504671/crates/bevy_transform/src/components/transform.rs#L274-L332) and [look-to construction](https://github.com/bevyengine/bevy/blob/b56fc29d3016e641754765244b5ba3f9cc504671/crates/bevy_transform/src/components/transform.rs#L467-L483) | Local right/up/back follow rotated +X/+Y/+Z; forward is rotated -Z. Identity orientation therefore has +Y local up and -Z local forward. A caller can supply a different up direction; this does not require one original-world permutation. |
| [Perspective projection](https://github.com/bevyengine/bevy/blob/b56fc29d3016e641754765244b5ba3f9cc504671/crates/bevy_camera/src/projection.rs#L336-L341) and [orthographic projection](https://github.com/bevyengine/bevy/blob/b56fc29d3016e641754765244b5ba3f9cc504671/crates/bevy_camera/src/projection.rs#L635-L646) | The defaults use right-handed projection construction, with reversed depth. This is a destination view/projection convention, not original-engine handedness or a physical-unit calibration. |
| [Mesh winding documentation](https://github.com/bevyengine/bevy/blob/b56fc29d3016e641754765244b5ba3f9cc504671/crates/bevy_mesh/src/mesh.rs#L211-L219) | The documented default StandardMaterial back culling considers counterclockwise viewed triangles front-facing. Mesh storage alone installs no culling state. The current application has no active renderer/PBR/camera plugin. |

OAT `v0.33.0`, commit `7d027e8f89118196713e955b0e11f8404149c54d`,
remains external tooling, not original engine source. Inspection found:

| Pinned primary source | Supported fact and limit |
| --- | --- |
| [MapEnts declaration](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1152-L1158) and [loading rule](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/MapEnts.txt#L4-L7) | The text pointer has a signed declared character count. The preserved generated T6 loader, lines 106–109, allocates/loads that declared count; it supplies no angle/unit interpretation. No string body or unbounded string traversal is performed. |
| [World sun declaration](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L1168-L1177) and [primary light declaration](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L3439-L3468), with [light loading count](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/ZoneCode/Game/T6/XAssets/ComWorld.txt#L4-L10) | Named angle/direction/origin fields and explicit array loading exist. No inspected original consumer links their components to gravity, angular units/order, a camera basis or original rasterizer state. Names do not establish those semantics. |

Accordingly, a new map-entity/light label or distribution probe would not
discriminate the competing conventions. None is performed. No native source
mutation/build, original decode, executable access, game launch or runtime
measurement occurs.

C2M commit `9a3accfb75c9a2f9780446c2b7bef040570357cf` is the primary
source for its author's exporter behavior, and a third-party hypothesis about
the original engine. The inspected
[world path](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L439-L512)
constructs WavefrontOBJ, appends decoded positions in source reference order,
assigns its Face array in order `(0, 2, 1)`, and calls the `.obj` save path.
It excludes repeated-index triangles. Its
[position unpacker](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L597-L610)
multiplies all three components by `2.54` without an axis permutation there.
This world path performs no FBX basis conversion. The downstream OBJ writer is
not inspected here, so the Face assignment is not a new proof of final file or
original GPU front state. Neither the multiplier nor the Face assignment proves
the licensed build's units, handedness or culling. No exporter implementation is
copied into tracked code.

Seven bounded immutable public-source fetches total 423,254 bytes. The OAT
checkout's CRLF files match its pinned Git blobs after line-ending normalization
only; their existing byte identities remain unchanged. Bevy/C2M fetched bytes
match the inspected copies exactly. External `source-identities.json` records
both local and public hashes, source URLs and the generated-loader identity.
The cached Bevy directories lack `.cargo-checksum.json`; no additional package
checksum verification is claimed. Preparation attempts that assumed that file
and exact OAT line endings were excluded before any source fetch succeeded.

## Admission, measurement and preservation

External `0015-origins-coordinate-evidence/predeclared-policy.md` precedes the
observations. Resolve only the ignored configured output root. Require a regular
file, the accepted exact length and SHA from one read-only opened handle before
mapping. Header version/flags/reserved bytes, checked u64 section arithmetic,
exact length, platform size and every count/visit policy precede body traversal.
Check all positions for finiteness, every reference for range validity, each
ordered surface partition/endpoints, and every reference against its own bounds.
Counts never substitute for byte-span admission.

| Logical policy | Bound |
| --- | ---: |
| File/read-only mapping | 64 MiB |
| Position / index / surface records | 2,000,000 / 10,000,000 / 100,000 |
| Complete position / index validation reads | 2,000,000 / 10,000,000 |
| Triangle-reference position reads | 10,000,000 |
| Transformed reference visits across nine cases | `9*I <= 30,000,000` |
| Transformed lower/upper scalar comparisons across referenced positions | `54*I <= 96,000,000` |
| Hash block / aggregate output record | 65,536 / 8,192 bytes |
| Invocation / combined pipe capture | 120 seconds / 256 KiB |

Each transformed axis interval test has two scalar comparisons. The comparison
policy covers those transformed lower/upper tests; header, finiteness, endpoint
and untransformed own-bounds checks are separate admitted passes. Both products
are admitted before mapping, independently of the count ceilings. Retained
working data consists of fixed header/triangle/endpoint scratch, nine cases,
fixed ten-bin category counters and other counters. A read-only mmap avoids a
second full geometry copy. These are logical mapping/visit/output policies,
not a hard process-memory bound; Python/OS overhead is not capped.

The first snapshot attempt is **excluded**: `observation1` exits 1 in 3.125 seconds,
with empty stdout and 73 stderr bytes, after its final rehash failed. Exact
pre-correction helper/driver/policy bytes, hashes and result/logs are retained.
A same-length arbitrary synthetic file reproduces the buffered-file/mmap/rehash
mismatch twice: Windows mmap resets the underlying file position, and the
buffered follow-up read produces a different full hash. The corrected path uses
unbuffered read-only FileIO, matches initial/final full hashes twice from each same
handle, and passes forced-exception mapping/handle cleanup. Both context managers
also close on analysis errors. Independent worker and parent rehashes confirm
the source's complete SHA/length/write-time/device/file-ID identity still equals
the accepted task 0014 record; the excluded result is never relabeled successful.

The final frozen helper passes 24 arbitrary synthetic parser/numeric cases,
including valid/empty/unreferenced data, truncation/trailer, unsupported headers,
nonfinite positions, index/partition/bounds/containment failures, negative checked
arithmetic, overflow, count and separate visit/comparison policies. Synthetic
cross-product tests cover all 48 signed permutations, split 24 positive and 24
negative determinants, and both face orders (96 cases). Six supervisor cases
cover success, child nonzero, capture cap, deadline, pipe-reader exception and
coordinator exception, with terminal child cleanup and no live reader threads.
Final preflight is `preflight-v3.json`; `mmap-regression.json` records the handle
regression. Four final helper/driver/regression/policy hashes are frozen before
the authoritative runs in `frozen-analysis-identities.json`.

## Direct repeated aggregate observations

The two authoritative runs, `production-v2-1` and `production-v2-2`, exit 0 in
3.031 and 3.000 seconds. Each emits 695 stdout bytes, empty stderr, and only the
allowed stdout/stderr logs plus aggregate `identity.json` in its fresh directory.
Neither limit is exhausted. Their aggregate records and complete input identities
agree exactly. No coordinate, endpoint, individual index, name list or derivative
geometry is emitted by the helper.

| Complete measure | Both observations |
| --- | ---: |
| Positions / references / triangles / surfaces | 191,259 / 449,721 / 149,907 / 6,469 |
| Zero declared-count surfaces / observed categories | 6,236 / 7 |
| Complete position scan / own-surface reference visits | 191,259 / 449,721 |
| Transformed reference visits across nine cases | 4,047,489 |
| Transformed scalar-comparison upper bound | 24,284,934 |
| Outside-own-bounds visits, each of nine cases | 0 |
| Repeated-index / binary64 zero-cross triangles | 0 / 0 |

Category counts remain those accepted in findings 0008; absent categories 6/8
are not newly supported. Cross products use promoted binary32 positions and
binary64 arithmetic; the zero/sign counts are diagnostics, not exact-arithmetic
degeneracy classification, original normals, visible-side or front-state proof.

| Raw component of diagnostic triangle cross product | Negative / zero / positive triangles |
| --- | ---: |
| X | 57,020 / 36,811 / 56,076 |
| Y | 60,537 / 34,959 / 54,411 |
| Z | 82,608 / 38,933 / 28,366 |

Mixed signs do not select a vertical direction or source winding: different world
surfaces can face different directions, and no original viewing/culling state is
present. Geometric cross products are not imported vertex normals.

## Why these observations cannot select a mapping

The nine actual diagnostics combine three abstract mappings
`(x,y,z)`, `(x,z,-y)`, `(x,z,y)` with each positive multiplier
`1`, `2.54`, `0.0254`. These are competing mathematical examples; no original
meaning is assigned to their axes or units. All nine preserve exact inclusive
own-bounds containment after signed endpoint exchange in binary64. No transformed
binary32 file, rounding repair, rendering or selected application policy exists.

For any signed permutation matrix `P` and positive scale `s`, transforming both
positions and their endpoints preserves component-wise inclusion, exchanging
lower/upper roles on reflected components. This is an algebraic inference; the
nine observed cases and arbitrary synthetic cases test it. The 48 permutations
are only a finite ambiguity subset, not every possible world basis. Bounds,
finiteness, distances up to scale, ordered reference preservation and an attractive
scene cannot identify the original choice.

For a triangle's diagnostic cross vector `n`, a signed permutation gives
`n' = s^2 * det(P) * P * n`. Swapping two face references changes its sign.
Thus an axis reflection and an index reversal are coupled decisions. Cross signs
alone cannot reveal the original visible side. No determinant rule here is
permission to reverse indices automatically.

| Coordinate decision | Established evidence | What rejects competing hypotheses | Current boundary decision |
| --- | --- | --- | --- |
| Numeric XYZ/reference storage | Complete accepted source/snapshot/Mesh preservation and repeated own-bounds checks | A contradicted source correspondence or incomplete accounting | Preserve accepted raw data and ordering in the diagnostic adapter. This is storage, not graphical placement. |
| World units / positive scale | Exporter multiplies by 2.54; nine diagnostic scales/bases retain containment | An independent dimensional reference in the original, or an explicitly sourced original unit definition | No original physical unit or conversion factor established. Bevy world-unit size would be application policy. A screenshot with camera/distances scaled together cannot calibrate it. |
| Up axis and sign | Destination identity local up is +Y; snapshot has no gravity/view basis | Original gravity/level-view direction registered to this raw geometry | No raw axis is established as up. Names, extents and cross-sign frequencies do not settle it. |
| Basis orientation / handedness | Destination projection is right-handed; both determinant signs survive diagnostic checks | Original ordered camera right/up/forward basis and asymmetric geometric correspondences | No original basis parity established; do not equate a renderer API or mirrored appearance with world handedness. |
| Angle units/order/sign | Public declarations contain named vectors; original consumers and angle samples are absent | Numeric original angles paired with controlled single-axis camera rotations, including sign and nonzero multi-axis cases | No degrees/radians, component order, rotation composition or angle-to-vector mapping established. |
| Triangle front/winding | Stored order preserved; exporter assigns 0,2,1; destination documented default front is CCW | Known original one-sided draw/front/cull state plus visible-side observations of the same nondegenerate triangle | No reversal or front-face policy selected. Two-sided or visually obscured surfaces cannot settle this. |

## One highest-value next measurement

Assign a separately scoped **original camera/visible-side coordinate calibration
for one identifiable asymmetric static world patch**, on the same licensed build.
It needs original runtime/view access that is excluded from task 0015. Keep raw
poses, captures, triangle correspondence and any draw-state records external.

Record the original numeric camera origin and ordered right/up/forward view basis,
its angle values, projection parameters and a gravity/level reference registered
to the admitted raw patch. Repeat the baseline, a controlled positive and negative
single-axis rotation, and a nonzero combined rotation. Identify at least three
noncollinear patch correspondences whose horizontal orientation distinguishes a
mirror from a rotation. Observe both sides of one nondegenerate triangle while
establishing the actual original front/cull/one-sided state; appearance alone is
insufficient. Report residuals, repeat variability, build/settings/state and
ambiguous or unavailable values, without guessing. This one calibration targets
the coupled basis/angle/front boundary, not movement or material implementation.

To settle absolute physical scale, that calibration additionally requires an
independent original dimensional/unit reference. If none is available, leave
physical scale unresolved and explicitly distinguish a later chosen Bevy unit
policy. Merely registering two scaled cameras/scenes cannot supply that anchor.
Successful view registration must not be reported as proof of physical meters.

No transform/API/renderer is implemented. Final preservation checks cover 230
source/helper/tracked/config records, the 13 accepted pristine source identities,
OAT pinned HEAD, nine individually clean submodules, the exact existing task9
cpp/header/binary/Git patch, official/task10/task11/task13/task14 artifacts and
the frozen analysis helpers. Native restoration is unnecessary because no native
file is changed. Five-document content/link/heading/whitespace validation and
`git diff --check` are recorded in [handoff 0015](../handoffs/0015-origins-coordinate-evidence.md).
Unchanged Rust/manifest/lock/test/unsafe/engine boundaries retain task 0014's
accepted 67-test workspace validation; Cargo, smoke and inspector gates are
intentionally not repeated for this documentation-only investigation.

Outer review accepts this bounded investigation with its PARTIAL result after
independently passing the frozen 24 parser/numeric, 48 determinant, 96 face-order
and six supervisor cases. Parent synthetic mmap tests reproduce both buffered
failures, pass both raw-handle hash cases and forced-exception cleanup. Two fresh
parent observations exit 0 in 3.000/2.969 seconds with 695 stdout bytes and empty
stderr, matching both authoritative worker aggregates and complete input identity.
All 230 preservation records, 84 separately recorded non-owned tracked/config
identities, four frozen helper identities and 31 artifact byte/SHA identities pass.
Pinned source blobs/cached Bevy files, all five candidates, 75 relative links,
12 source anchors, ten handoff headings and content/whitespace checks pass.
Parent records remain external in `0015-coordinate-parent-review/`. Acceptance
establishes the investigation and its limits, not an original coordinate mapping
or fidelity comparison.
