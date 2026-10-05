# Original moving brush-trace contract

Task0035, 2026-10-05. **INVESTIGATING** for the loaded native implementation;
gameplay fidelity remains **UNKNOWN**. One authentic named-registration/direct-call
chain reaches a complete brush consumer and request builder. This enables a
bounded Rust moving-brush increment, not player movement or a world trace.

## Source and bounded procedure

The existing read-only process profile requalifies process42012, creation
FILETIME134357079689703421, executable SHA-256
`6c405bec03a89b8d954d8ae5edea1bed89b4147f53e4dbe68ced6ef5a815a96b`,
and image-header SHA-256
`d203ed499e46f35abf308fd88c4069357a86b1d654d4a6b87b236cf2e152b161`.
The owner identifies Plutonium r5354. Pause/map/player state is not assumed;
Steam script build24784239 equivalence remains unproved. Reads were strictly
code/identity/registration/constants: no inputs, gameplay sampling or original writes.

External task0035 `native_trace.py` reuses the damage observer's process/header/
native profile and executable-image admission. It finds exact `bullettrace`
names/qualified24-byte registrations in two prior spans totaling6,081,039 bytes;
buffers are discarded. Page-contained reads are at most4KiB,24MiB per invocation.
Code is read twice and terminal exact-span reads agree. Image allocationRVA0,
committed image pages/protection0x20 and before/after identity/profile checks pass.
Handles close in `finally`; all raw bytes/instructions/receipts remain external.

Both registrations have ID0x80/nameRVA0x84cc9c. EntryRVA0x943000, args4/6 points
to RVA0x45aac0 (VM instance0, server-side route); entryRVA0x93b308, args4/5 points
to RVA0x3caf00 (instance1, alternate route). Names, records and code remain
unchanged at terminal qualification. Registration plus a direct-call graph is
not proof of active VM dispatch or that a gameplay query executed this chain.

The layout cross-check uses the existing
[OpenAssetTools T6 declarations](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h),
revision7d027e8f89118196713e955b0e11f8404149c54d. The retained header is197,407
bytes, SHA-256`66f23295ee5e4545d6d7cae2a3f0fc266e9bea8d331c6b35e7fa5a7008a8cc91`.
It supports record names/layout only; the native code supplies this contract.
No analogous engine implementation is treated as T6 proof.

## Authentic chain

The measured native chain is builtin0x45aac0 -> request wrapper0x17af30 -> point
world trace0x2db280 -> initialization wrapper0x1a9780 -> builder0x2d8770 -> BSP
traversal0x485130 -> leaf0x484710 -> leaf-bound test0x4845d0 -> brush-node
traversal0x484250 -> **moving narrow phase0x483ea0**. The builder also directly
calls reciprocal helper0x275950. These RVAs are evidence, not project APIs.

Fifteen complete functions total8,515 bytes; direct jumps land on instruction
starts within their function. Initial windows total67,840 bytes including overlap/
padding, below24 functions/1MiB. Exact byte tables/hashes/calls stay external.
The four other inspected bodies distinguish alternate builtin/client and filtered
boolean-test routes; they do not define the selected contact contract.
Narrow-phase SHA-256 is
`827023c4abacf9ad4e10a663972c62df8e6aaa35878b0e78bb68544061f4daa1`;
builder SHA-256 is
`a7cd8754734d8e996c88a1864a0c1be89b818d46692774af2753eed8dd13e9f9`.

## Request construction and branch boundary

Use supplied start/end and extent endpoints `lo`, `hi`, in unchanged source
coordinates. The builder computes each component in scalar float32 order:
`center=(hi+lo)*0.5`, `extent=hi-center`, then adds center separately to start/end.
It derives `r=min(extent.x,extent.z)`, `h=extent.z-r`, and axial support
`(r,r,r+h)`. The observed side support is capsule-like: `r+abs(h*n.z)`.
It is **not** a general AABB support or a radius multiplied by normal length.
The narrow phase does not select an AABB alternative when extent.y differs.
Do not generalize this contract to arbitrary shapes or other trace families.

For the selected world route, model selector0 and moving endpoints reach BSP
traversal when the supplied request/filter record has no listed filter entries;
listed entries take the separately observed0x485460 path. Endpoints equal in all
three components take a different stationary consumer. The moving builder's
query+0xbc discriminator is set from whether the extent sum is nonzero; it does
not select a box-support branch in0x483ea0. Zero extents make the point subset.
Stationary synthetic calls to0x483ea0 are helper control-flow checks, **not** the
stationary wrapper's contract; that downstream consumer is outside this scope.

Relative to the builder's query record, the relevant fields are centered
start+0x80, end+0x90, reciprocal+0xa0, axial support+0x70, original half extents
+0x40, mask+0xb8, r+0xc0 and h+0xc4. These are evidence labels, not a proposed
Rust memory layout. Ordinary nonnegative finite extents and finite arithmetic are
the recommended safe project subset; original invalid-input policy is unproved.

The named point builtin supplies mask0x0280e033, or0x00806033 when its third
argument is zero. Optional admitted arguments clear bit0x20 or0x10. These are
**script point-trace masks**, not standing-player masks. The immediate brush
caller selects a96-byte brush by leaf index and proceeds only when
`brush.contents & query.mask != 0`; native narrow phase itself does not repeat it.

## Stored brush shape, precision and clipping

The complete consumer reads brush bounds, contents, side count/pointer and flags.
It does not read stored vertices/count. Bounds are real clipping constraints,
not only a broad phase: lower X,Y,Z first, upper X,Y,Z second, then stored sides
in original order. A zero side count skips only the side loop, leaving six axial
constraints. Thus this native consumer uses an axial zero-side collision shape;
extra source vertices do not change that consumer. Whether all retained8417
source records are active static world brushes remains a separate question.

Each axial residual is evaluated as `(centeredEndpoint-bound)*sign-supportAxis`,
with sign-1 for lower and+1 for upper bounds. Side normals/distances are preserved.
For sides the observed float32 order is `expandedDistance=distance+r`, then
`expandedDistance+=abs(h*normal.z)`, and residual
`((normal.x*x+normal.y*y)+normal.z*z)-expandedDistance`. There is no normalization,
snapping, vertex fitting or coefficient repair. Multiplications/additions/subtractions
and quotients are separate scalar single-precision operations; no fused multiply-add
or f64 dot product is present in this routine.

The directly referenced scalar constant at RVA0x84d19c is float32 **0.125**,
word0x3e000000, unchanged on repeated reads. This is a raw-coordinate allowance,
not a newly established physical unit. Entry starts at0; exit starts at the
caller's current result fraction. For each plane, let `a,b` be start/end residuals:

- If `a>0`, reject when `b>=min(0.125,a)`. An entering candidate uses
  `(a-0.125)/(a-b)`. Axes actually multiply `(a-0.125)*reciprocalAxis*sign`,
  in that order; reciprocalAxis is float32 `1/(startAxis-endAxis)` or0 for equality.
- For sides, compare `a-0.125 > (a-b)*currentEntry` **before division**. Only a
  strict improvement divides and updates entry. Axes compare their multiplied
  candidate strictly against entry. A first candidate can select the contact
  plane without improving initial entry0. Reject entry greater than or equal to exit.
- If `a<=0` and `b>0`, exit uses the **unshifted** ratio `a/(a-b)`. Side code
  compares `a > (a-b)*currentExit` before division/update. Axes multiply
  `a*reciprocalAxis*sign`. Reject when entry is greater than or equal to exit.
- Exact entry ties retain the earlier selected plane. The first outside plane
  remains selected when its adjusted entry is negative and entry stays0. This is
  not the current diagnostic closed-interval/tangent or source-key tie policy.

The allowance is entry-only; it must not become a uniform inside threshold,
symmetric interval expansion or source-geometry correction. A start just outside
can report fraction0/contact while its startsolid flag remains false. A moving
trace starting inside and ending outside can set startsolid without reducing the
incoming fraction. Cross-brush tie/order behavior depends on traversal; do not
substitute the diagnostic collection's lowest-source-key tie rule.

## Result writes and implementation consequence

Observed result fields are normal+0, fraction+0x10, surface flags+0x14, contents
+0x18, allsolid byte+0x2a, startsolid byte+0x2b and an additional byte+0x2c.
The names allsolid/startsolid are semantic interpretations supported by the complete
control flow, not an imported trace structure declaration.

On an admitted entry, the consumer writes fraction, the original selected plane's
normal (or signed axial unit normal), selected surface flags, brush contents and
clears+0x2c. It does not write plane distance. After all constraints admit the
query, no selected entering plane means startsolid is set. If the end remains
inside every constraint, allsolid is set
and fraction becomes0; otherwise fraction is retained. That inside branch writes
the upper-Z axial surface/content flags into the result, following its initial
brush-contents write. Preserve this surprising observed overwrite; do not silently
substitute contents or invent a contact normal when none is selected. Rejected
brushes leave the result unchanged. Existing result flags are not generally cleared
by the consumer; wrapper initialization and aggregation remain distinct.

The next safe implementation is a separate finite float32 **single moving brush**
query in reusable physics, fed by the admitted raw brush snapshot and explicit
centered capsule parameters or explicit extent endpoints/mask/current fraction.
Reject equal centered endpoints as an unsupported wrapper path for this increment.
Retain raw fields/order and reproduce these comparison/result distinctions with
synthetic tests; keep existing f64 geometry/precision diagnostics available.
It should expose contact plane identity/normal, fraction, startsolid/allsolid,
surface/content result semantics and typed arithmetic/admission failure. Bevy
retains lifecycle/ECS/scheduling. No new engine or duplicate asset decoder is needed.

Player dimensions/mask, movement/step/slide, terrain/static XModels/submodels,
runtime ownership/transforms, complete traversal aggregation and actual execution
comparison are excluded. The previous source residuals still exist; this contact
constant neither repairs stored geometry nor proves producer rounding/fidelity.

## Validation and retained limits

External `abstract_checks.py` compares a scalar-f32 flow transcription with an
independent rational interval derivation on24 synthetic cases. Both agree on
zero/positive sides, masks/high-bit contents, expanded/asymmetric bounds, side
support, nonunit coefficients, ordered ties, parallel rejection, initial fraction
limits, entry-only margin, near-outside fraction0, start-inside exit and direct
helper stationary/boundary cases. Representative zero-side through-box entry is
0.234375 and unshifted exit0.75; symmetric exit epsilon would differ. This checks
the transcription and mathematical distinctions, **not original runtime outputs**.

Complete function decode, terminal byte/profile/registration and helper syntax/
predicate checks pass. Import-shadow and padding-boundary failures rejected safely;
corrected runs pass. No framework/tool installation or production Rust edit.
Unchanged Cargo gates, live callback/output comparison and performance tests were
not rerun. Prior original evidence remains external and preserved. See
[handoff0035](../handoffs/0035-original-brush-trace-consumer.md) for closure.
