TASK:
Qualify and implement native1d1230 moving terrain point/triangle numerical consumer.

STATUS:
COMPLETE

FINDINGS:
- Point588 bytes, normalizer136 bytes, initializer15 bytes requalified from licensed disk and exactly equal to retained terrain01 instruction-byte receipts.
- Scalar-f32 face/edge predicates admit edges/vertices, reject backfaces and fraction ties. Stored winding controls normal. No brush tolerance is inserted.
- Normalization uses separately rounded squares/sum/sqrt and reciprocal with zero-length denominator1. Underflow and on-plane signed-zero behavior are retained.
- Point writes only normal/fraction/flag2c; inherited solids/material fields survive.

EVIDENCE:
- terrain01/02 contracts, reader, immutable triangle/vertex identities, existing brush/static numerical consumers/tests, project state and current Git status.
- Licensed static source spans/hashes, mask initializer/slot and normalization constant; external terrain03 qualification receipt. No game execution/process access.
- Two independent synthetic Python scalar-f32 exact-bit fixture receipts reproduced; eight Rust tests exercise analytical/source-flow cases.
- Detailed evidence and completion limits: ../reverse_engineering/terrain-point-triangle.md.

CHANGES:
- crates/physics/src/terrain_point_trace.rs: explicit start/displacement request, per-triangle lookup, typed reference/input/arithmetic failures, atomic TraceResult writes.
- crates/physics/tests/terrain_point_trace.rs: eight synthetic tests for interior/edges/vertices/winding/on-plane/backface/parallel/zero-displacement/behind/ties, nonaxis exact-bit fixtures, degenerate geometry, underflow guard, displacement misuse and early/late failure atomicity.
- Numerical evidence doc and this handoff. Primary owns module export; no shared fields/getters changed.

VALIDATION:
- cargo test --locked -p bo2-physics --test terrain_point_trace:8 passed.
- cargo test --locked -p bo2-physics: passed, existing tests included; external opt-ins remain ignored.
- cargo clippy --locked -p bo2-physics --all-targets -- -D warnings: passed.
- Owned Rust files rustfmt edition2024 check: passed.
- Three spans739 bytes retained-equal; mask constant/initializer slot/normalization1 confirmed.
- Independent Python two-case output reproduction: passed. No native execution or original comparison.
- Initial compile attempt encountered pending primary export and malformed leading-dot test literals; literals fixed before final gates. No retained terrain numerical probe performed.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Scene identity/provider binding and actual payload authentication remain caller duties.
- Complete terrain capsule/stationary and aggregate consumers/stamp lifecycle remain required for player-world participation.

RECOMMENDED NEXT WORK:
- Review/integrate this named subset; separately qualify ordered terrain wrappers and stamp lifecycle, then complete capsule/edge/vertex numerics without generic substitutes.

SCOPE NOT IMPLEMENTED:
- AABB/leaf aggregation; partition/material/mask/stamp writes; capsule/edge/vertex/stationary consumers; world/receiver/motor; original invocation/process reads/play/capture; full-map fidelity claim.
