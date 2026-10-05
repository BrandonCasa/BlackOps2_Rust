TASK:
Implement all retained diagnostic map-leaf candidate geometry as a safe Rust
source-keyed point-segment query collection and read-only inspector.

STATUS:
COMPLETE

FINDINGS:
- All 8,417 selected ordinary records are admitted and queryable, including 3,178
  explicitly diagnostic zero-side axial hulls; no geometry subset was dropped.
- Recovered 17,108 side records and 68,642 vertices. Exact brush7 raw preservation
  retains 8 strict/8 old-tolerant violations and max vertex-set error
  0.0005449587606563 raw units; its witness proves numerical storage consistency only.
- Full retained-vertex scans agree between Rust and independent Python: 30,706
  strict violations across 4,186 brushes; 23,732 old-tolerant across 3,842; maximum
  positive residual 0.09950868785381317. Zero-side subgroup: 32 strict/old violations
  across eight brushes, maximum 0.00048828125. These remain unresolved discrepancies.
- Representative zero-side source has 12 vertices against eight axial corners,
  vertex-set max error 4.0, and explicitly unavailable coefficient witness.
- Collective endpoint probes can begin inside other hulls; a representative's
  outside point is not necessarily a collection miss. Runtime static ownership
  and source shape agreement remain unproved.

EVIDENCE:
- Inspected current project state/architecture, handoffs0028/0029/0033, existing
  safe Rust brush reader/queries/tests, qualified native headers/span helpers,
  task0028 emitter, task0033 membership adapter/checker and retained records.
- Only one fresh configured original read: 78,753,344 bytes, SHA256
  663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5.
- Accepted external schema34: 9,596,663 bytes, SHA256
  3c171b4f18ce65eeec0cd8f2d1a1f2c19bc15e192597e9a9cc52c317e2d51fb6.
  Independent checker binds all selected keys/reached count/provider metadata/
  unsupported coverage to retained schema33 and every brush7 field/bit to schema28.
- External task0034 receipts retain native source.diff, compile/fixture/checker
  results, original-result, collection-summary, source-residual-summary and final
  Rust gate logs. Originals, geometry bodies, native/script sources and paths
  remain external; no tracked original fixture or copied proprietary script.

CHANGES:
- Shared crate-private brush admission/read helper preserves legacy schema28;
  schema34 reader retains source identities, exact keys/order/coverage and all words.
  Required nullable metadata, bounds/normals/counts/references/aggregate limits and
  repeated global plane/side/vertex consistency have redacted typed failures.
- Physics owns an ordered collection of existing ConvexBrush values; construction
  is all-or-error. Allocation-free nearest clipping visits every hull, with exact
  equal-entry ties resolved by lowest provider/brush key regardless of record order.
- --brush-collection reports bounded coverage, collective probes, full residual
  scan and separate representative precision. Hashes are declarations until checked
  against independent caller identities; standalone inspection cannot authenticate.
- [Focused boundary note](../architecture/diagnostic-brush-collection.md).
  Outer owns current state/index integration; pending task0026 files untouched.

VALIDATION:
- Native adapter compile, production relink and fixture link pass with warnings as
  errors. 25 actual cases include high-bit/-1 opaque-word preservation, spans,
  finite geometry, closure rejection and exact/exceeded 32MiB append-time capacity;
  13 independent checker malformed/tamper rejections pass. Python syntax passes.
- Single native read: exit0 in 1.656s, 921 captured bytes, 32MiB body/2MiB stdout
  cap and 120s deadline; no error/limit, child reaped and capture threads joined.
  Same-handle input identity before/after agrees; retained prior identities preserved.
- cargo metadata --locked --offline --no-deps --format-version 1: exit0, eight members.
- cargo test --workspace --locked --offline: exit0, 114 passed, three external tests
  ignored by default. Three opt-in external assets/physics tests separately pass,
  including all 8,417 raw/owned records, signed-zero-sensitive coefficient bits,
  candidate linkage, zero/positive probes, brush7 precision and repeated queries.
- cargo fmt --all -- --check and cargo clippy --workspace --all-targets --locked
  --offline -- -D warnings: exit0. Original collection inspector: exit0.
- Synthetic coverage includes malformed/length/byte/aggregate/count/null/type
  admission, identity mismatch, duplicate/missing keys, conflicting shared pool
  words, analytic zero-side box, separated/overlapping/tied/reversed queries,
  start-inside/stationary/tangent/miss/nonfinite/finite-overflow and residual summaries.
- No original-game trace comparison, GPU integration or performance measurement.
- Outer review: both collection opt-in tests and retained collection/legacy-brush
  inspector executions pass. Scope, source/tests, gate receipts and preservation
  closure reviewed; current state and architecture index updated for integration.

FIDELITY STATUS:
INVESTIGATING for data/math interpretation; original gameplay remains UNKNOWN.

UNRESOLVED:
- Original producer/runtime rounding and the larger residuals/source-corner gaps;
  full membership interpretation, absent model ownership/transforms and original
  runtime static selection, masks/player hulls/step-slide behavior.

RECOMMENDED NEXT WORK:
- Investigate a bounded highest-residual or zero-side discrepancy and the actual
  runtime trace/ownership contract before applying this collection to player motion.

SCOPE NOT IMPLEMENTED:
- Player movement/collision, viewer/appearance, damage/mechanics, other collision
  families, T6 fidelity claims, new engine/decoder/capture setup, original writes,
  proprietary tracked content, commits or pushes.
