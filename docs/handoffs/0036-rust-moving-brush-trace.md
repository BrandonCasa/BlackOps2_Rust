TASK:
Implement the evidenced finite-f32 single moving brush request/consumer and
exercise the same routine through representative collection inspection.

STATUS:
COMPLETE

FINDINGS:
- Six axial constraints and ordered sides share entry-only 0.125 allowance;
  zero-side records remain executable axial brushes. Capsule-like support and
  explicit caller parameters replace no existing diagnostic geometry query.
- Ordered radius selection chooses X on equality, including signed zeros. Raw
  side normals retain their words; strict ties and scalar operation order follow
  [native findings0024](../reverse_engineering/0024-original-brush-trace-consumer.md).
- Inside/result overwrites differ from entry; moving entirely inside still sets
  solid flags at incoming fraction 0. No global empty-interval rejection is added.

EVIDENCE:
- Read project state/rules, architecture/diagnostic boundaries, handoff0035,
  admitted assets/physics/inspector code and validation skill. Original build and
  consumer identity remain those sealed in findings0024; no new original reads.
- Retained schema34 collection: 9,596,663 bytes, SHA-256
  3c171b4f18ce65eeec0cd8f2d1a1f2c19bc15e192597e9a9cc52c317e2d51fb6.
  Reused the retained task0035 independent f32 model without changing it.
- External task0036 retains helper sources, captured opt-in/inspector results,
  independent-model and gate receipts. Proprietary coordinates/words remain
  external; tracked fixtures are synthetic.

CHANGES:
- Added safe reusable moving_brush_trace module/tests and lib exports only;
  borrowed BrushSnapshot, Bevy Vec3 and existing schema admission. No dependency,
  crate, decoder, unsafe exception or engine owner added.
- Request construction rejects equal centered endpoints and invalid/nonfinite
  inputs/arithmetic with typed errors. Result commits transactionally; rejection
  and mask filtering preserve incoming fields. Caller owns aggregation order.
- Inspector adds three diagnostic point crossings per positive/zero representative
  with zero extents, all-bits mask and fraction 1; bounded reports omit original
  coordinates/words/flags/paths/indices. Legacy precision/probes remain unchanged.
- Added [architecture boundary](../architecture/moving-brush-trace.md) and this
  handoff. Pending task0026 files untouched; outer owns integration/brief/index.

VALIDATION:
- Focused physics/CLI checks pass. Thirteen added regular tests cover zero/positive
  sides, capsule/asymmetric/unequal-Y support, raw nonunit/signed-zero normals,
  strict ties/limits, entry-only allowance, near-contact fraction 0, inside exit,
  moving all-inside at fraction 0, masks/inherited flags and transactional errors.
- Final gates: cargo metadata --locked --offline --no-deps --format-version 1
  (8 members); cargo test --workspace --locked --offline (127 passed, 4 ignored);
  cargo fmt --all -- --check; cargo clippy --workspace --all-targets --locked
  --offline -- -D warnings. All exit 0; no lockfile/dependency change.
- Explicit new retained-input opt-in passes: all 8,417 source raw geometry/flag
  records preserved, no drops; strict source residuals remain 30,706 violations
  in 4,186 brushes, maximum 0.09950868785381317. Six endpoint-derived crossings
  repeat deterministically and agree exactly with the independent f32 model on
  fraction/normal words, plane identity and result metadata.
- Retained collection inspector exits 0 with six bounded entry reports. Native
  execution comparison, gameplay, performance and three older opt-ins were not
  rerun. Initial fixture literal/bit assertion and receipt case-label errors were
  corrected without weakening checks; production gates are clean.
- Helper syntax, candidate/whitespace/link checks pass. All owned subprocesses
  completed; no native handles/jobs created, no original writes, no cleanup of
  retained evidence, no commits/pushes.
- Outer review independently reruns the new retained opt-in, its six exact
  reconstruction comparisons and the collection inspector successfully. Source,
  tests, scope and gate receipts reviewed; current brief/README/index updated.

FIDELITY STATUS:
INVESTIGATING for native contract; original gameplay remains UNKNOWN.

UNRESOLVED:
- Original runtime result comparison and active source ownership; stationary
  wrapper, player hull/mask, transforms/submodels, world traversal and tie order.
- Finite-input rejection is a safe project subset, not original invalid-input
  proof. Source geometry residuals remain; no repair or normalization is inferred.

RECOMMENDED NEXT WORK:
- Review/integrate this single-brush increment. Qualify original output comparison
  or the separate stationary wrapper in a bounded investigation before movement
  or world-query integration; do not infer player dimensions from these probes.

SCOPE NOT IMPLEMENTED:
- Player/world movement, stationary/world collision, traversal/acceleration,
  shape families, player defaults, geometry repair, native invocation or original
  gameplay changes, appearance/damage/mechanics and unrelated pending work.
