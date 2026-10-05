# Single moving brush trace

Task0036 implements the selected native contract from
[findings0024](../reverse_engineering/0024-original-brush-trace-consumer.md).
`bo2-physics::moving_brush_trace` borrows the existing admitted `BrushSnapshot`;
there is no second asset reader, engine, registry or world traversal. Bevy supplies
Vec3 and retains lifecycle/ECS/scheduling ownership. Legacy f64 geometry and
precision diagnostics are unchanged.

`MovingBrushRequest::new` requires explicit endpoints, extent endpoints and opaque
mask. It computes the original scalar-f32 center, extents, radius (ordered X/Z
comparison, X on equality), vertical half segment and per-axis reciprocals.
Finite ordered extents and finite arithmetic are a safe admitted project subset;
original invalid-input behavior is unproved. Equal centered endpoints return
`UnsupportedStationary`, including distinct inputs merged by f32 centering.
Zero extents retain the point subset; there are no player defaults.

`trace` receives an explicit incoming `TraceResult` with finite normal and fraction
in [0,1]. It filters brush contents against mask, clips lower XYZ/upper XYZ/stored
sides in order, and returns `MaskedOut`, `Rejected`, `Entry(ContactPlane)` or
`Inside`. Side support evaluates distance+r before abs(h*nZ); ratio-improvement
tests precede side division; axial ratios use ordered reciprocal multiplications.
Entry alone uses 0.125. Strict ties keep the earlier plane. Source normals are
copied without normalization, including signed-zero words; axial contacts use
the observed temporary normal's positive-zero components and signed axis unit.

All updates are transactional: typed arithmetic errors and rejected/filtered
brushes preserve incoming fields. Entry writes fraction/normal/surface flags/
contents and clears opaque byte 2c, retaining inherited solid flags. Inside sets
startsolid, sets allsolid/fraction 0 only if the end stays inside, and performs the
observed upper-Z surface/content overwrite. It retains incoming normal/opaque
byte and otherwise incoming fraction/allsolid. Plane distance is absent because
this consumer does not write it. Caller-owned aggregation order is not a claim
about original world traversal or source-key ties.

The collection inspector adds three crossing requests per positive/zero
representative, each against that single source brush, with explicitly diagnostic
zero extents/all-bits mask/fraction 1. It emits status/fraction/solid flags/contact
presence and plane kind, without coordinates, source flags/words/paths or indices.
Existing collective probes and precision summaries remain visibly diagnostic.

Synthetic tests establish admitted math/result/error behavior. The retained
external collection test preserves all 8,417 records/words and residual totals;
six representative crossing results agree exactly with the independent f32
reconstruction for float words, plane identity and metadata. This is model
agreement, not native execution/output comparison. Contract interpretation is
**INVESTIGATING**, original gameplay **UNKNOWN**. Player hulls/masks, movement,
stationary/world queries, submodels/transforms and other families remain excluded.
