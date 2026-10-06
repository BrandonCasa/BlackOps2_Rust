TASK:
Implement the qualified ordered moving leaf/global brush-node subset using admitted hierarchy and the accepted moving single-brush consumer.

STATUS:
COMPLETE

FINDINGS:
- Completed native 484250/4845d0/484710 and previously unqualified strict bounds helper1831e0; four static spans1556 bytes. All three retained traversal spans match the admitted disk exactly.
- Native end spatial coordinates remain full request endpoints; end W is incoming fraction. A derived nested split fixture distinguishes this from W=1.
- Negative adjacency resumes split after contact; terminal lists continue after allsolid/fraction0. Crossing-only fraction pruning follows one-child/dead-zone branches. Source order, masks, equality, support, padding and small-delta behavior are preserved.

EVIDENCE:
- Current moving/stationary consumers, hierarchy and terrain leaf readers, focused tests, task0035 complete spans, global ownership04 and stationary08.
- Licensed file-only disassembly/hash receipts and data-section constant qualification. External task09 static-spans.json and qualification-receipt.json; primary has private locator.
- Detailed arithmetic, span hashes, procedure and safety limits: ../reverse_engineering/moving-leaf-traversal.md.
- No original process access, native invocation, playing or capture.

CHANGES:
- crates/physics/src/moving_leaf_trace.rs: MovingBrushOnly, explicit traversal limits, keyed unsorted provider lookup, bounded persistent scratch, atomic authoritative result updates and typed failures.
- crates/physics/tests/moving_leaf_trace.rs:11 synthetic tests covering traversal order/equality/crossings, terminal/adjacent continuation, masks/root0, strict bounds, capsule support, small delta, negative padding, non-unit fraction, entry ties, seeded flags, caps and late atomic failures.
- docs/reverse_engineering/moving-leaf-traversal.md and this handoff.
- Primary separately owns export and exact stored request getters. No other modules edited by this worker.

VALIDATION:
- cargo test --locked -p bo2-physics --test moving_leaf_trace:11 passed.
- cargo test --locked -p bo2-physics: passed; existing accepted moving/stationary tests passed; external opt-ins remain ignored in this invocation.
- cargo clippy --locked -p bo2-physics --all-targets -- -D warnings: passed.
- rustfmt edition2024 check on both owned Rust files: passed.
- Four admitted disk spans rehashed; three retained bodies byte-equal; four constant bit patterns confirmed.
- Initial two fixture expectations incorrectly used brush contents for inside contact; corrected to the existing source-backed axial flag writes, then all focused tests passed.
- No actual retained moving leaf replay or original comparison performed; broader integration gates belong to primary.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Full scene identity/leaf ownership authentication and all participating family providers remain assembler responsibilities.
- Effective alternate movement receiver factory, complete BSP enumeration, terrain and remaining world families are outside this subset.

RECOMMENDED NEXT WORK:
- Review/integrate the moving subset and then assemble only qualified family dispatch with explicit missing providers; continue alternate receiver effective producer resolution.

SCOPE NOT IMPLEMENTED:
- Complete PlayerCollisionWorld/motor; terrain; BSP/x87 world enumeration; filtered004050; transformed submodels; dynents/runtime entities; special box selector; alternate receiver; guessed hulls/masks/timing; original fidelity comparison.
