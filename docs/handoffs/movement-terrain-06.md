TASK:
Qualify and implement native stationary sphere versus one terrain triangle (4821a0 ->481f10, sphere writes482e40).

STATUS:
COMPLETE

FINDINGS:
- Complete admitted disk and retained instruction bodies agree for all three spans (1,168 bytes). Constants and caller stack/vertex mapping independently checked.
- Preserves scalar f32 operand/dot/region/equality order, including observed center-minus-A signs and negative weight numerators. This differs from conventional closest-point geometry; no geometric repair is introduced.
- Strict radiusSquared > nativeDistanceSquared admits overlap; tangency is clear. Overlap writes only startsolid/allsolid=true and positive-zero fraction. All other authoritative fields remain unchanged.
- Finite executed arithmetic is the safe domain. Nonfinite degenerate paths explicitly fail atomically; rounded collinear geometry can also follow a finite source branch.

EVIDENCE:
- Inspected project state/architecture, Git diff, terrain01/03/04/05 contracts/handoffs, immutable snapshot and TraceResult/point consumer/tests.
- Static licensed image identity, full body hashes, constant values, exact abstract equations and limits: docs/reverse_engineering/terrain-stationary-sphere.md.
- External task06 qualification receipt, read-only static decoder and independent scalar Python fixtures. No original process access or instruction/native execution.
- Seven synthetic axis fixtures and one oblique exact-bit fixture reconstructed before implementation. Raw signs explicitly distinguish the conventional geometric projection.

CHANGES:
- crates/physics/src/terrain_stationary_sphere.rs: supplied finite center/radius, admitted triangle access, typed effect/diagnostic/errors, transactional contact writes.
- crates/physics/tests/terrain_stationary_sphere.rs: seven meaningful tests covering independent regions/bits, tangent/inside/edge/vertex, winding, seeded fields, signed zero, invalid inputs/references/fraction, finite/failed degenerate paths and late overflow atomicity.
- docs/reverse_engineering/terrain-stationary-sphere.md and this handoff. Primary owns module export; no other shared changes.

VALIDATION:
- cargo test --locked -p bo2-physics --test terrain_stationary_sphere: 7 passed.
- cargo test --locked -p bo2-physics: all crate/unit/integration/doc tests passed; retained-input opt-ins remain ignored.
- cargo clippy --locked -p bo2-physics --all-targets -- -D warnings: passed.
- rustfmt --edition2024 --check on both owned Rust files: passed.
- During iteration, a zero-length-edge fixture incorrectly expected a finite return; the actual nonfinite division was retained, and a separately derived collinear cancellation fixture now checks a finite branch. No numerical implementation correction was needed. A formatting check then required splitting a chained diagnostic assertion; corrected and passed.
- No original comparison. Primary runs outer workspace/client integration gates.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Effective original runtime equivalence of the unusual static scalar signs/operand behavior remains unverified. No MATCHED/VERIFIED claim.
- Native nonfinite fall-through is deliberately outside the explicit safe finite domain; no fallback clear/overlap is chosen.

RECOMMENDED NEXT WORK:
- Qualify/implement the separate nonzero vertical-segment stationary triangle consumer before stationary player terrain aggregation.

SCOPE NOT IMPLEMENTED:
- Vertical capsule, stationary aggregate, stamps/reset, materials/masks, full world/scene, special poses, alternate receiver, numerical motor or original comparisons.
