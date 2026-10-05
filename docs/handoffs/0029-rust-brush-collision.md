TASK:
Implement a bounded safe Rust schema28 brush reader and owned convex point-segment
query boundary, preserving the measured original discrepancy and checking the
float32 storage hypothesis on the retained record.

STATUS:
COMPLETE

FINDINGS:
- Runnable Rust now consumes the retained four-side/sixteen-vertex brush and clips
  caller-supplied point segments under an explicitly diagnostic halfspace convention.
- Original source still has 8 strict and 8 unchanged old-tolerant violations; the
  vertex-set error remains 0.0005449587606563 raw units. No epsilon was relaxed.
- A joint numerical float32-cell witness was constructed: every altered coefficient
  rounds to its original word and reconstructed vertices match source words.
  Witness vertex error is 9.094947017729282e-13; maximum directional half-ULP cell
  usage is 0.882629398257 for coefficients and approximately 3.725e-9 for vertices.
  This is storage consistency under declared solver thresholds, not exact
  feasibility or proof of original producer/runtime rounding.

EVIDENCE:
- Inspected project state, architecture, assets reader/tests, inspector/tests,
  selected Bevy 0.19.1 math/raycast facilities and
  [findings0019](../reverse_engineering/0019-origins-collision-brush-recovery.md).
- Read only the retained external schema28 record (1,656 bytes; SHA256
  `0b3cab3d285b299712879cff25924100a93f6495f7f179f7ec4b1be8d0e7c17f`)
  and its existing analyzer. No decoder run, new original read or raw repository fixture.
- External identity still matches. Opt-in Rust test independently compared every
  raw word, opaque field, ordered side/vertex and nullable presence to the JSON source.
- Worker and outer independently ran retained `--brush`: midpoint stationary hit;
  x/y/z endpoint-derived crossings each enter 0.25/exit 0.75; outside stationary misses.

CHANGES:
- `crates/assets/src/brush_snapshot.rs` and synthetic reader tests: 64 KiB input,
  1-64 sides/0-256 vertices, private wire deserialization, finite/positive geometry,
  checked counts/references, required nullable fields and redacted typed errors.
- New safe `bo2-physics` member: Bevy DVec3, bounded owned planes, allocation-free
  closed point-segment intervals, typed arithmetic failures and separate precision
  diagnostics with retained old thresholds and adjacent-f32 rounding cells.
- Inspector adds explicit `--brush`; legacy snapshot CLI/tests remain. Emits counts,
  precision and named query outcomes only, without source coordinates/bits/paths.
- Minimal manifests/lockfile changes; cached serde 1.0.229/serde_json 1.0.151 selected.
- [Focused boundary note](../architecture/diagnostic-brush-collision.md).
  Outer owns state/index integration. Existing pending task0026 files untouched.

VALIDATION:
- `cargo metadata --locked --offline --no-deps --format-version 1`: exit 0, eight members.
- `cargo test --workspace --locked --offline`: exit 0, 92 passed, one external-input
  test ignored by default. That opt-in test was separately run on the retained
  external record with `BO2_BRUSH_SNAPSHOT` and `-- --ignored`: exit 0, one passed.
- `cargo fmt --all -- --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: exit 0.
- Box/wedge exact fractions, start-inside, tangent/parallel/opposing/stationary,
  repeat/order, nonfinite/arithmetic failure, malformed/truncated/oversized/type/
  count/null/missing input, limits, roundoff witness and semantic mismatch covered.
- `cargo run -p bo2-map-inspector --locked --offline -- --brush <external-record>`:
  exit 0, reported results above; original retained identity unchanged.
- `git diff --check`: exit 0. No source lint suppression. Cargo sometimes reports
  incremental-cache hard-link fallback; compilation/check exits still succeed.
- No GPU, original gameplay comparison or performance measurement required/run.

FIDELITY STATUS:
INVESTIGATING for geometric data interpretation; original movement/collision,
overall gameplay and appearance remain UNKNOWN.

UNRESOLVED:
- Original producer rounding, runtime brush consumer, contact epsilon and masks.
- Original units/basis, player shape, movement timing/speeds, step/slide behavior
  and full-map collision. The witness uses f64 numerical reconstruction thresholds;
  a failed restricted witness is not a disproof of every storage-cell hypothesis.

RECOMMENDED NEXT WORK:
- Use this executable geometry boundary for the next bounded measured collision/
  movement increment; establish player/runtime semantics before authentic movement.

SCOPE NOT IMPLEMENTED:
- Player/controller, generic physics/ECS/scheduler, full map, decoder/capture,
  renderer/viewer/boot/damage edits, original assets/scripts, commits or pushes.
