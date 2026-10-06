TASK:
Implement the qualified ordered stationary leaf/global brush-node traversal with
existing immutable hierarchy, terrain-owned leaf records and single-brush consumer.

STATUS:
COMPLETE

FINDINGS:
- Exact strict leaf bounds, split/equality behavior, adjacent-first negative nodes,
  stored terminal order, masks and seeded-allsolid continuation points implemented.
- Explicit keyed lookup handles unsorted schema34 records. First-overlap source key
  preserves native first contact; no entity-owner binding is invented.
- Fallible bounded construction and persistent iterative workspace; failure commits
  no authoritative result, and scratch resets after success/error.

EVIDENCE:
- Re-read complete admitted 483890 / 483980 / 483cf0 bodies and hierarchy07 evidence.
- Supporting implementation notes in ../reverse_engineering/stationary-world-contract.md.
- Identity-checked external hierarchy, terrain and schema34; Rust reconstruction
  receipt retained externally under configured inspection/movement-collision-08.

CHANGES:
- stationary_leaf_trace.rs, its integration tests and brief contract notes.
- Primary owns added export and pub(crate) exact request bounds/mask getters.

VALIDATION:
- Eight synthetic tests passed: branches/equality, adjacency, ties/order, masks/root0,
  strict six-axis tangency, seeded flags, limits/scratch reuse, invalid public leaves,
  missing providers and arithmetic atomicity.
- Rust-only retained opt-in: 4,791 point probes, 1,844 contacts; repeated outputs equal.
- Locked physics suite and warning-denying all-target physics Clippy passed.
- Formatting and diff checks passed after primary normalized shared exports.
  Full integration gates belong to primary.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Full world assembly, BSP x87 classifier/enumeration, terrain and filtered consumers,
  transforms/activation/dynamic providers, special-box and alternate receiver access.
- Original comparison evidence; no native run or process read occurred.

RECOMMENDED NEXT WORK:
- Implement separately qualified moving ordered leaf traversal or resolve the
  numerical BSP/world enumerator before assembling brush-only world queries.

SCOPE NOT IMPLEMENTED:
- Full stationary world/PlayerCollisionWorld, movement, guessed masks/hulls, terrain,
  transformed models, dynents, filtered004050 or original comparison.
