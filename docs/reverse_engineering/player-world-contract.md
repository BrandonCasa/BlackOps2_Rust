# Player collision world: contract and coverage qualification

Movement collision lane 01, 2026-10-05, with reviewed model-ledger refinement from
[ownership lane04](collision-global-ownership.md). Retained-file inspection only; no original
process reads, execution, input or capture. Contract interpretation is
**INVESTIGATING**; player/full-map fidelity remains **UNKNOWN**.

## Available evidence

Recovered external task0033 membership, task0034 geometry and task0035/0037
native windows through ignored local configuration and task-local external roots.
Schema33 is 2,634,532 bytes, SHA256
`6b999dba22db62c98a22c469eaad9d4ee7441b7ee10c058405b74f39221e67b3`;
schema34 is 9,596,663 bytes, SHA256
`3c171b4f18ce65eeec0cd8f2d1a1f2c19bc15e192597e9a9cc52c317e2d51fb6`.
Both hashes agree in this task. All fifteen task0035 qualified native spans agree
with their sealed lengths/hashes (8,515 bytes). These are the retained
Plutonium-r5354-associated image, not proof of unmodified Steam equivalence.
Original source identity and narrow-phase arithmetic remain documented in
[findings0024](0024-original-brush-trace-consumer.md).

Locally retained OAT revision `7d027e8f89118196713e955b0e11f8404149c54d`
T6 declarations and generated loader rules were inspected. They establish storage
references, not native behavior. `clipMap_t` separates embedded/global ClipInfo,
map BSP/leaves, collision vertices/triangles/partitions/AABBs, static models,
cmodels and four dynent collision lists. `cmodel_t` owns a leaf and nullable
reusable ClipInfo reference. Static models retain an XModel, contents, origin,
inverse scaled axes and world bounds. Dynent definitions retain model/brush-model
references and initial placement; runtime pose/client/server/collision arrays are
declared runtime storage. Definition placement alone does not prove current pose
or activation.

## Direct native-flow observations and entry points

- Qualified builder RVA `0x2d8770` selects a 76-byte cmodel by supplied selector
  when within the model count; its selected leaf is at model offset `0x20`.
  This is not an ownership interpretation for every selector or a proof that
  null serialized model providers exclude collision.
- Equal centered endpoints dispatch separately. World stationary entry is
  RVA `0x483cf0`; model stationary brush/terrain entries are `0x483980` and
  `0x0e9e20`. These calls are observed in the complete builder; their target
  bodies are not qualified by task0035. Moving selected-model leaf entry is
  `0x484710`; world moving dispatch selects `0x485130` or filtered `0x485460`.
  Special selector routes `0x3fff`/`0x4000` are distinct and unimplemented.
- Complete moving leaf consumer `0x484710` gates brush contents and terrain
  contents separately against the supplied mask. Brush leaf processing may
  terminate the leaf when its result has reached zero fraction; otherwise the
  terrain loop visits `firstCollAabbIndex + ordinal` in ascending order, using
  32-byte AABB records and calling RVA `0x05b690`. That target body remains
  unqualified. Root-zero terrain leaves cannot be discarded as empty world.
- Complete brush-node recursion `0x484250` tests node contents against mask,
  dispatches positive signed terminal counts in stored u16 order, and resolves
  each terminal against a global 96-byte brush pool before the contents test and
  `0x483ea0` narrow phase. Negative counts recurse to the adjacent 20-byte node
  before split processing. Native split/ordering calculations require their own
  complete transcription; diagnostic schema33 traversal does not substitute.
- Existing single moving brush logic uses scalar f32, explicit endpoints/bounds,
  mask and incoming result. Its first-plane ties are order sensitive. Existing
  diagnostic collection source-key ties are not native world aggregation.
- Complete point-world wrapper `0x2db280` performs additional filtered/world
  family calls after the builder, including `0x2d5a60` and `0x0ba160`.
  Their bodies/ownership are unresolved. A brush-only implementation of the
  builder therefore cannot establish full player world collision.

These entry points are reproducible leads derived from admitted function calls;
they are not approved inferred symbol names for unexplored bodies.

## Full-map coverage ledger

All counts below were rechecked from the identity-bound external schemas.

| Family/reference set | Declared scope | Disposition and missing qualification |
| --- | ---: | --- |
| Map BSP and leaves | 4,525 nodes; 4,791 leaves | Source metadata retained; native traversal arithmetic/order and owner admission pending. |
| Ordinary brushes | 8,417; 17,108 sides; 68,642 vertices | Payload reader and moving narrow phase available; runtime traversal/membership and stationary consumer pending. |
| Declared cmodels | Every ordinal 0 through 264 (265 total) | All serialized provider pointers are null; this is not missing native geometry. Reviewed global-node closure resolves all 583 model brush IDs in schema34. Dispositions: world selector 0, empty leaf 5, 223 brush-only, 13 brush/terrain, 27 terrain-only. Terrain exchange v1 now admits all declared terrain arrays and embedded model leaves; numerical queries and runtime scene binding/transforms remain pending. See ownership lane04 and [terrain admission](terrain-payload-admission.md). |
| Root-zero terrain leaves | 214 leaves; 253 AABB references | Every leaf has positive AABB count and nonzero terrain contents. Participating candidates require terrain dispatch/contents qualification, never brush-root exclusion. |
| Terrain | 67,196 triangles; 18,968 partitions; 51,129 AABB records | Bodies, indexed traversal, edge/walkability rules, contents/material resolution and stationary/moving consumers pending. Render triangles are not a collision replacement. |
| Static XModels | 10,837 instances | Model collision providers, contents eligibility, transforms and native consumer order pending. |
| Dynent lists | `[275, 0, 19, 0]` | Definition/model references, runtime pose/activation and collision-list association pending; zero list counts do not exclude the nonzero lists. |
| Separate box | Explicit box owner; diagnostic map candidate does not reach it | Source-backed exclusion from that diagnostic candidate only; special native selectors remain distinct queries. |

The current per-model ledger is in ownership lane04. Ordinal5 has a source-backed
empty-leaf exclusion for its own model query only. No participating family has a
whole-family exclusion proof.
Schema34 stores counts but lacks a family/ownership/transform/activation scene
manifest. Geometry-reader success cannot satisfy scene completeness.

## Proposed shared boundary for primary review

Physics owns `PlayerCollisionWorld`; assets owns external family admission and a
separate versioned scene manifest. Keep existing geometry schemas unchanged.
The manifest binds independently checked payload identities to explicit ordered
owners, traversal references, model sharing, source-space transforms and activation
revisions. Null provider/pose is an error unless a named source-backed exclusion
applies. Validate the complete scene and every referenced family before publication;
failed scene/activation updates preserve the previous usable world.

The numerical movement boundary needs explicit source-space start/end, lower/upper
extent endpoints, contents mask, exclusion/filter context, incoming result and
scene revision. Provide distinct stationary clearance and swept query operations;
do not convert clearance into an epsilon sweep. Return authoritative fraction,
normal, contents, surface flags, startsolid/allsolid, source family/owner/contact
identity and any additionally qualified native result fields. Preserve original
order and scalar precision; do not silently resolve ties by ECS entity order.
Queries returning missing-provider, missing-pose, unresolved-family, invalid-reference
or arithmetic errors leave the caller's authoritative state unchanged.

Names/error categories above are implementation proposals, not native field or ABI
claims. Primary must finalize shared signatures after the numerical motor's trace
needs and family result writes stabilize. No default player hull/mask or scene
timing is supplied by this task.

## Validation and next tasks

Read-only identity/count checks pass for both schemas, all fifteen native spans,
all 265 absent-provider dispositions, all 214 root-zero terrain leaves and their
253 AABB references. The retained synthetic f32/rational narrow-phase reconstruction
passes 24 cases, with its new receipt written externally without modifying old
receipts. It validates transcription, not original execution. No Rust changed;
workspace gates are deferred to implementation integration.

Next separate bounded tasks: qualify stationary world/brush and native global-pool
ownership; transcribe native world/node traversal; recover and qualify terrain
reader/consumers; static XModel reader/consumer; transformed cmodel adapter; dynent
pose/activation adapter reusing established model/brush consumers. A lane blocked
on missing bodies or runtime poses must report the exact gap and release its slot.
No synthetic family can satisfy a missing full-map participating reference.
