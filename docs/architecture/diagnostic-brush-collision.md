# Diagnostic convex-brush boundary

Task0029 consumes the retained schema28 exchange from
[findings0019](../reverse_engineering/0019-origins-collision-brush-recovery.md).
`bo2-assets` admits one bounded record, preserving raw float32 words and opaque
metadata/order. It uses cached serde/serde_json rather than a custom JSON parser.
The private deserialization type prevents bypassing admission. Limits are 64 KiB,
1-64 sides and 0-256 vertices; finite positions/planes, nonzero normals, positive
endpoints, count lengths and explicit nullable presence are checked. Errors retain
neither paths nor source values/messages. This is a project exchange reader,
not a BO2 disk decoder or Bevy loader.

`bo2-physics -> bo2-assets, Bevy` owns at most 70 halfspaces and uses Bevy's
`DVec3`. The selected 0.19.1 `RayCast3d` exposes f32 AABB/sphere intersection;
it does not provide arbitrary coefficient-preserving convex-halfspace segment
clipping. This small adapter adds that missing operation without an ECS, scheduler,
physics engine, coordinate mapping or map dependency. Callers explicitly select
`DiagnosticMinusDistanceAtMostZero`: original ordered sides plus six axial
constraints derived from the original endpoint values. Coefficients are neither
normalized nor rounded again. Point-segment queries allocate nothing, use closed
parameter intervals, include boundary/tangent/stationary cases, and reject
nonfinite inputs/arithmetic. A hull can be empty; such queries return no hit.
Determinism is repeatability for the same platform, inputs and source order,
not a cross-platform bit-identity claim.

Precision diagnostics are separate from queries. They retain task0028's 1e-8
scaled residual, 1e-7 extent-scaled deduplication and 1e-10 determinant thresholds
for diagnostic vertex reconstruction. Source violations and bidirectional
L-infinity vertex discrepancy remain visible. No threshold becomes a contact
epsilon. A restricted rounding witness uniformly scales each side normal and
places its distance at the stored vertices' support value. Each adjusted scalar
must round to its original float32 word inside cells derived from adjacent
representable values. Reconstructed witness vertices must match source words
bijectively; exact real zeros choose the source signed-zero representative without
numerical snapping. Reports quantify the used directional half-ULP cell fraction.

`NumericallyConstructed` means joint storage consistency under the declared f64
solver procedure, not an exact rational feasibility proof or evidence of the
original producer/runtime rounding. `NotFound` means this restricted witness
failed, not a disproof of every independent-cell hypothesis. Empty source vertices
leave the witness unavailable. A diagnostic request with a different source hull
is rejected. Work/allocation are bounded to 256 source and 1,000 reconstructed
vertices and at most 54,740 plane triples per reconstruction.

`bo2-map-inspector --brush <record>` reads one explicit regular external file
and emits counts, precision summaries and five endpoint-derived point-segment
probes. It emits no coordinates, float words, paths, flags or materials and writes
no file. Legacy snapshot inspection remains available. The midpoint probe may
miss a valid asymmetric hull; it is diagnostic rather than a gameplay assertion.
Input data remains external; tests contain only independent synthetic geometry.

Data interpretation remains **INVESTIGATING**; original movement/collision
fidelity remains **UNKNOWN**. No original units, orientation, masks, player shape,
contact rule, step/slide, timing, runtime consumer or full-map collision is defined.
The original strict discrepancy remains even when storage consistency succeeds.
