# Classic transformed cmodel query adapter

Movement-submodel01 qualifies a reusable classic transformed-query boundary,
not a full collision scene or the alternate player receiver. Fidelity remains
**INVESTIGATING**. It reuses [global model ownership](collision-global-ownership.md),
[moving leaves](moving-leaf-traversal.md), [stationary dispatch](stationary-world-contract.md),
[terrain](terrain-trace-contract.md) and [player coverage](player-world-contract.md).
There is no native invocation, game/process access, playback or capture.

## Reproduction and complete-body scope

Licensed disk identity is 13,090,072 bytes, SHA-256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
Image base is `400000`; PE mapping admits the relevant text starting
RVA/raw `1000/400`. Owned external `movement-submodel-01/native_qualify.py`
hashes before/after, walks reachable direct branches under64-KiB displacement
and12,000-instruction bounds, checks ordered instructions against exact disk
bytes and excludes overlaps. Calls require their own qualification. Raw
instructions, discovery windows and helpers remain external.

| Entry RVA | Instruction bytes | Ordered instruction SHA-256 | Qualified role |
| --- | ---: | --- | --- |
| `1a9780` | 80 | `2673f0100e2b6179f2d5e21717fc18a718209c8ac394febaddc63b5a47ea2c62` | Initializes a new result, then builder |
| `2d8770` | 1,741 | `a7cd8754734d8e996c88a1864a0c1be89b818d46692774af2753eed8dd13e9f9` | Model selection, centered request and stationary/moving dispatch |
| `0e7cc0` | 511 | `6fb1a4406577ca76a0eb2f9feb51fd3a11b57437538400daa5db23719eab4c9e` | World/request wrapper; not transformed cmodel adapter |
| `21a410` | 547 | `62dd48849d430c2bd2e20711dfe131b74e70c9a5ef74d48ea25e41ff82683481` | Supplied origin/angles adapter |
| `27c640` | 891 | `c696cfa4a29f52d3dc7f56ad86b8fdb41d7d8a7e09814fcfe7288be8827ef468` | Supplied origin/matrix adapter and normal promotion |
| `0ac740` | 381 | `3a62b7fa2c3fdf94639951b54d60df5bff7db5e15ef70f32c936fc4b97ecc27b` | Angles to ordered model-axis rows |
| `19cd80` | 1,388 | `bedbf2f3e775346798c2a1de8959fdd7c9fc8e805313967aff40b43e3191de61` | Classic entity candidate filter/pose/selector caller |
| `1c5df0` | 123 | `6453c1812b3bb5ef1dc8f91f77e2b2984adbf89bc17eb5c180fe6610b472cc57` | Entity selector resolver; alternate tail unresolved |

Aggregate8 bodies/1,368 instructions/5,662 instruction bytes; final receipt
SHA-256 `84e2284bc28e86fc7bab77cb61bc81926eb9aeb14c4279850c1dd9d85ba88cc7`.
`1c5df0` alternate tail `1756d0` remains unqualified; its simple cmodel branch
returns before that dependency. Bounded exact relative-call discovery locates
16 calls to `1a9780`, three to `2d8770`,12 to `21a410`, three to `27c640`.
Partial caller windows and padding-based entry leads are discovery only;
selected entries are subsequently decoded through complete reachable returns.
All12 angle-adapter callers are not thereby admitted scene producers.

## Distinct initialization and transformed boundaries

`1a9780` clears64 bytes of result, sets fraction1 and native word`+26=fe`, then
calls `2d8770`. `0e7cc0` reaches it with caller-supplied request/filter and assigns
world contact metadata afterward. Neither applies an origin/angle transform.
They must not reset an incoming closest result when implementing a raw
per-instance transformed query.

`21a410` arguments, in order, are:
`result, start, end, mins, maxs, selector, mask, origin, angles`.
The three finite angle components compare individually with zero. All-zero
angles take the translation branch; otherwise `0ac740` constructs a matrix and
calls `27c640` with the same arguments except its last argument is that matrix.
Neither transformed adapter clears the incoming result.

`0ac740` takes pitch/yaw/roll degrees, multiplies by f32 bits`3c8efa35`, then uses
x87 `fsincos`. Its rows are source **forward, left, up** (left is negative
camera right in [camera02](first-person-camera-contract.md)). At zero angles
this is the identity matrix; positive yaw90 gives rows approximately
`[0,1,0],[-1,0,0],[0,0,1]`. The public angle path has no scale argument.
The raw matrix path does not validate, normalize or remove matrix scale, and
its normal promotion would retain any such scale. This is not evidence that
all matrix callers supply scaled or orthonormal poses. Angular x87 bit precision
remains subject to [FP10](movement-floating-point-policy.md).

## Endpoint, hull and normal operand order

For each axis, both adapters compute f32 hull center `c=(maxs+mins)*0.5`,
centered bounds `mins-c`, `maxs-c`, then endpoint `q=(world+c)-origin`.
The subtraction occurs **after** the addition. An asymmetric supplied hull
therefore changes endpoints before transformation. Zero-angle dispatch passes
these centered bounds/endpoints straight to the builder.

Matrix dispatch applies **row dots**, not the static-XModel endpoint column
transform: local component0 is `(qx*A00 + qy*A01) + qz*A02`; component1 is
`(qy*A11 + qx*A10) + qz*A12`; component2 is
`(qx*A20 + qy*A21) + qz*A22`. Products/additions are separate scalar-f32
instructions. Both start and end follow those orders. **Hull bounds remain
the same centered axis-aligned bounds; they are not rotated or scaled.**
The builder then constructs the existing capsule/support request from them.
Do not replace this behavior with a rotated AABB or oriented capsule.

`27c640` saves incoming fraction before querying. Only `old_fraction > new_fraction`
promotes the three-component result normal. Ties/misses leave the normal untouched.
For each world component j its sequence is
`(local_ny*A1j + local_nx*A0j) + local_nz*A2j`: matrix transpose, Y product plus
X product, then Z product. All original normal components are retained in scratch
before writes. There is **no normalization**, plane-distance translation, or
recomputation of result`+2c` after promotion. Other builder result writes remain
as produced. In particular, stationary inside contacts can reduce fraction to0
without writing a new normal; the adapter still follows its fraction predicate.
An implementation must reproduce that distinction rather than invent a normal.

## Builder selectors and stationary/moving dispatch

`2d8770` first resolves selector with an unsigned compare against native global
model count. In-range selectors index global model array with stride76;
out-of-range selectors use thread-context helper`19a740(3)` and its `+10` model
pointer. That fallback is not permission to index retained payload out of bounds
or silently select ordinal0. Its required thread model/provider remains separate.

After centering and request construction, equality of all three centered endpoints
selects stationary behavior. Selector0 uses world BSP/leaf dispatch rather than
its cmodel leaf. Ordinary nonzero selectors use the selected model’s embedded
leaf at`+20`, with global brush/terrain ownership established in ownership04.
Stationary ordinary-model dispatch checks incoming allsolid byte`+2a`, then leaf
brush contents against mask; successful brush overlap stops before terrain.
Otherwise terrain contents gate calls stationary `0e9e20`. Moving ordinary-model
dispatch calls combined leaf entry`484710`, retaining its native brush/terrain
ordering. These family entry points reuse their qualified contracts; capsule
terrain completeness is not established by this adapter investigation.

Reserved selectors`3fff/4000` have separate stationary entries
`483ab0/4847a0` and moving entries`484e40/484900`. The `3fff` path includes a
contents gate through special query storage; it is not an ordinary cmodel leaf.
The `4000` path also differs. Neither is admitted as a retained model ordinal,
excluded by a map count alone, or implemented here.

Both transformed wrappers create their own local filter storage, with the two
identity words initialized to`3ff` and remaining fields zero. The supplied
mask remains explicit. Ordinary cmodel leaf dispatch does not consume the world
entity exclusion filter, while selector0 enters the world path with that local
filter. Builder postprocessing can update native flag`+2c` based on local normal
and preserved startsolid/opaque state before any matrix normal promotion.
No new result-field interpretation is invented.

## Runtime cmodel pose and participation owner

The classic callback `19cd80` derives an entity pointer from its candidate index
and native entity pool base/stride globals. It first gates entity contents`+118`
by query mask, evaluates supplied exclusion identities/flags and owner`+14c`,
then has separate model/hitbox alternatives controlled by query options and
entity flags. The qualified ordinary fallback bounds gate uses current absolute
entity bounds`+11c/+128`, not just serialized cmodel bounds.

For entity byte`+f9 !=0`, `1c5df0` returns signed16 model selector`+dc`, and
`19cd80` passes entity current origin`+134/+138/+13c` and angles`+140/+144/+148`
to `21a410`. For `+f9 ==0`, it supplies zero angles by explicit source choice and
the selector resolver can reach runtime model geometry or a separately prepared
box through `2894e0`. This explicit non-cmodel zero-angle branch does not prove
zero pose for brush submodels. Entity type6 is excluded only in the non-cmodel
fallback gate; it is not a blanket cmodel disposition.

This particular callback supplies zero mins/maxs, so its admitted transformed
request is a **point segment**. It is not proof that all twelve callers or the
player’s non-point request use that hull. A strictly closer hit is promoted by
the caller to native entity contact identity and entity contents; a scene adapter
should preserve a typed instance identity instead of exposing native pointers.

Still missing: active candidate enumeration/link ownership, pool stride binding
to retained scene instances, current pose producers and updates, entity`+f9/+dc`
population, activation/link/exclusion state, and any addon replacement model
namespace. Static serialized model declarations contain geometry ownership,
not these active runtime poses. Every participating instance needs an explicitly
bound selector, finite current pose, activation/exclusion state and scene revision.
Missing pose/provider/activation must be an explicit later admission error;
there is no zero-pose/default-active inference for all265 models.

## Validation and bounded implementation prerequisite

Independent external Python source equations pass **12** synthetic cases:
asymmetric recenter/translation, yaw row transform with unchanged hull bounds,
transpose normal directions/inverse relation, non-axis row-versus-column
discrimination, retained nonunit normal under scale, strict hit/tie/zero-fraction
promotion gates, and a rounding case distinguishing `(world+c)-origin` from
`(world-origin)+c`. Explicit f32 stores separate operands; no native code is run.
Helper SHA-256 `d07d9407041044bfc111fdcd0d654be3e8228adc9ca33eeb2ca832417a3207e6`.
These checks do not establish x87 trig bit identity, actual scene pose or original
comparison. Document links/whitespace checks pass; no Rust changed.

The implementable boundary after review is a supplied-pose ordinary cmodel
adapter with explicit selector/mask/hulls/incoming result and existing family
queries. Full player collision still requires world traversal/filtering, complete
terrain hull consumers, native player receiver binding and a qualified scene
instance/pose/activation provider. No dynent work or scene defaults are added.

## Supplied-query Rust increment (submodel02)

[`TransformedModelRequest`](../../crates/physics/src/transformed_model_trace.rs)
now implements the reviewed translation and supplied-matrix arithmetic, with
explicit opaque selector/mask and no angle-to-matrix defaults. `trace_with`
passes a `LocalModelRequest` to one caller-supplied family query, preserves its
typed contact result/error, stages all writes and commits only after callback
result validation and successful normal promotion. Missing provider/query
errors propagate; no registry, scene provider or selector aggregation is added.
Atomicity covers authoritative `TraceResult`; callback-owned workspace effects
follow that consumer’s own policy. Stationary endpoints remain valid inputs.

Finite-input/ordered-bounds checks, finite intermediate arithmetic, callback
result validation and rollback are deliberate safe adapter policies, not native
error behavior. Finite matrix scale is retained without an invented orthonormal
restriction. Translation and identity-matrix branches remain explicit, including
different signed-zero behavior. Strict fraction improvement promotes the normal
without renormalization or recomputing `opaque_flag_2c`; ties still publish any
callback writes exactly as supplied.

Nine focused Rust tests pass, including an independent non-axis f32 bit fixture,
asymmetric hull centering, add-before-subtract rounding, row/transpose direction,
retained nonunit/inside normal, stationary endpoints, ties/zero fraction and
atomic early/query/result/late-promotion failures. Owned external submodel02
`reference.py` rounds every input to f32 before its independent source equations;
SHA-256 `db20525344b4f29b63ecd3077622845c87b09baedb831d6535d91ad9cb784b71`.
Locked focused tests, physics all-target warning-denying Clippy and owned-file
format checks pass. This is a supplied-pose/query adapter, not a resolved
`PlayerCollisionWorld`, full-map motor or original comparison.
