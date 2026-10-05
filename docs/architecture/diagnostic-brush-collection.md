# Diagnostic brush collection

Schema34 is a project-owned exchange for a selected set of ordinary source-keyed
brush records. It is not a BO2 decoder or an original world/player trace. The
assets crate owns bounded admission; physics owns a pure ordered collection of
existing `ConvexBrush` values; the read-only map inspector assembles these through
`--brush-collection`. Bevy retains engine/ECS/runtime ownership. No map dependency,
new engine, duplicate brush parser, spatial index or runtime entity registry is added.

The private schema carries declared source and retained-membership byte/hash
identities; the named selection interpretation, reached-node count and exact keys;
provider pool counts/presence; unsupported coverage counts; and ordered geometry
records. Each record retains its provider/brush key, side/vertex pool spans, every
float32 word, opaque u32 flags/contents, required nullable parallel metadata and
side/vertex order. Hashes alone are declarations. `check_identity` compares caller
identities; the standalone inspector cannot authenticate originals. The separate
native observation/checker binds the accepted artifact to retained membership and
source identities; this provenance check is distinct from Rust internal admission.

Admission shares schema28's private validated construction and bounded read helper.
Schema28 still requires 1-64 sides; schema34 explicitly permits 0-64. Limits are
32MiB JSON, 65,535 records, 16 providers, 256 vertices per brush and one million
aggregate side/vertex references. Recursion, interruption and same-handle length
checks remain bounded. Counts, keys, full selection coverage, finite words, positive
bounds, nonzero normals, presence and checked pool references are required. Shared
provider/global plane, side and vertex references must have identical raw records;
temporary consistency maps are bounded by charged aggregate references. Errors
retain neither paths, raw words nor underlying parser messages. Unknown fields,
missing nullable keys and unsupported versions are rejected.

All admitted hulls are constructed or assembly fails visibly. Raw sides followed
by six axial endpoint constraints use the existing `n dot x - d <= 0` convention,
without normalization, epsilon, snapping or transforms. A zero-side record is an
explicit diagnostic six-axial hull; its stored vertices do not establish complete
corners or an original shape. Allocation-free point-segment clipping visits every
hull. Minimum entering parameter wins; exact equal-entry ties choose the lowest
`(provider_id, brush_id)` regardless of record order. Start-inside/stationary hits,
closed tangencies, misses and arithmetic errors retain the single-brush contract.
A result's interval belongs to its selected hull, not the union of all hulls.

The inexpensive source-vertex residual scan checks all retained vertices against
raw side/axial planes and reports strict/old-tolerant totals, affected brushes and
maximum residual, including a zero-side subgroup. Reconstruction and numerical
float32 witnesses are separate representative diagnostics; their thresholds never
enter queries. Zero-side witnesses are explicitly unavailable. A point outside a
representative hull can hit another collection hull; collective probes must not be
interpreted as local-hull assertions or original static world ownership.

The accepted task0034 artifact includes all 8,417 diagnostic map-leaf candidates,
including 3,178 zero-side records. The separate box association is excluded; all
265 model providers are absent. Terrain, triangles/partitions, AABB trees, static
XModels, dynents, masks, player hulls and submodel/runtime transforms remain
unsupported. Data interpretation is INVESTIGATING; original gameplay is UNKNOWN.
See [handoff0034](../handoffs/0034-rust-brush-collection.md) for actual evidence and
validation. Larger residuals and missing corner agreement remain unresolved, not
silently accepted fidelity.
