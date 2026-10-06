TASK:
Implement ordered moving point terrain leaf/AABB/partition aggregation in the qualified fresh workspace domain.

STATUS:
COMPLETE

FINDINGS:
- Exact point builder and scalar-f32 SAT equations feed admitted initial/child AABB tests; descending child tests/LIFO produce ascending depth-first traversal.
- Map/model selectors reuse one common leaf consumer. Outer material inheritance, masks, root order, triangle/partition order and cross-leaf stamp dedup are preserved.
- Native484710 checks fraction0 before every outer root; tree482ff0 continues internal partitions after zero. Paired regression cases distinguish them.
- Fresh exclusive workspace generations1..65535 only; wrap returns explicit error. Failure burns reserved generation/stamps, clears scratch and preserves incoming state; retry uses next generation deterministically.

EVIDENCE:
- terrain01/03/04, builder0024, current immutable terrain reader/tests and complete482ff0/482f10/005b690/484710 source reread.
- Source/implementation details: ../reverse_engineering/terrain-point-aggregation.md.
- Twelve synthetic source-flow tests, independent f32 versus f64 cancellation witness and retained identity-checked Rust point probes; no original execution/process/play/capture.

CHANGES:
- crates/physics/src/terrain_point_world.rs: explicit request/limits/workspace, ordered map/model leaf selectors, point aggregation and separately returned material/partition/diagnostic triangle contact.
- crates/physics/tests/terrain_point_world.rs:12 synthetic tests plus explicit retained opt-in.
- Numerical implementation evidence and this handoff. Primary owns export; shared TraceResult fields unchanged.

VALIDATION:
- cargo test --locked -p bo2-physics --test terrain_point_world:12 passed,1 retained opt-in ignored normally.
- Explicit retained opt-in: passed,5056 map/model leaves,63 first-pass contacts,two equal authoritative/diagnostic passes excluding generation.
- Retained payload SHA256 e5ecb51a076898aeae66cfb6fd38ef79218768c29d3089279303e39801315a67 independently checked before/after; external terrain05/rust-reconstruction-receipt.json records measurements.
- cargo test --locked -p bo2-physics: passed with existing accepted consumers (including concurrent transformed family); external opt-ins normally ignored.
- cargo clippy --locked -p bo2-physics --all-targets -- -D warnings: passed after fixing a source-witness test-style lint.
- Owned Rust formatting check: passed. Initial cancellation expectation used16 instead of actual8; independently recalculated8/right10 and corrected the fixture, then all focused tests passed.
- Original comparison absent; broad workspace/client integration gates belong primary.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Runtime scene/provider byte authentication/ownership and qualified leaf selection remain outer assembly duties.
- Capsule/stationary terrain and reset/reload scheduling beyond the fresh domain remain explicit participating-family prerequisites.

RECOMMENDED NEXT WORK:
- Review/integrate point aggregate; separately complete capsule/stationary numerical paths and generation/reset links before player terrain/world assembly.

SCOPE NOT IMPLEMENTED:
- Capsule/stationary terrain; chosen reset policy; BSP enumeration; full world/receiver/motor; transformed activation/poses; hull/mask/timing defaults; original process/native execution/play/capture; full-map player fidelity.
