TASK:
Implement the qualified stationary single-brush request on existing admitted
BrushSnapshot payloads and share exact request construction with moving traces.

STATUS:
COMPLETE

FINDINGS:
- Stationary overlap is distinct from moving clipping: axial tangency rejects,
  side-plane equality admits, no moving entry allowance, zero-side axial support.
- Scalar f32 operation order and contact writes follow
  [qualified stationary evidence](../reverse_engineering/stationary-world-contract.md).
- Explicit masks, asymmetric centering and inherited results remain caller-owned;
  errors/rejections leave all authoritative result fields unchanged.

EVIDENCE:
- Reviewed current Git state/diff, prior contract/handoff02, existing moving source
  and tests, admitted brush schemas and validation skill.
- Source contract references six qualified native spans; no new original reads.
- Existing external schema34 used only through explicit ignored-test environment
  input. No original geometry or machine paths added to tracked files.

CHANGES:
- Added stationary_brush_trace.rs and its synthetic/opt-in external integration tests.
- Added crate-private trace_shape.rs for centered endpoints and capsule support;
  moving constructor reuses it without changing clipping or tests. Moving equal
  centered endpoints still reject before support construction, preserving precedence.
- Public request new(origin, lo, hi, mask), trace(brush, incoming) and centered_origin
  reuse TraceError/TraceEffect/TraceResult. Primary owns module exports.
- All existing forbid(unsafe_code) policies retained; no dependencies introduced.

VALIDATION:
- cargo test -p bo2-physics --locked --test moving_brush_trace
  --test stationary_brush_trace --lib: PASS, 23 tests; two external tests ignored.
- Same moving/stationary targets with -- --ignored and retained collection input:
  PASS, both opt-ins. Representative zero/positive-side contacts repeat identically;
  source arrays remain unchanged. Moving six independent reconstructions/residual
  preservation still pass.
- cargo clippy -p bo2-physics --all-targets --locked -- -D warnings: PASS.
- Rustfmt check on all four owned Rust files and git diff --check: PASS.
- First compile exposed missing Debug on BrushSide in a test assertion; comparison
  changed to ordinary equality and checks reran successfully.
- New tests cover six axial tangencies, near/outside contacts, inclusive side edges,
  sequential scalar cancellation, raw plane scale, capsule/y extent, asymmetric
  bounds, masks/high bit, signed zero, nonfinite/overflow and transactional failures.
- Full locked workspace gates remain primary integration responsibility; no claim
  of original execution, window behavior, world coverage or fidelity comparison.

FIDELITY STATUS:
INVESTIGATING for admitted numerical contract; player/full-map UNKNOWN.

UNRESOLVED:
- Native world/provider binding, terrain/static XModel/transformed/dynent families,
  complete scene manifest, BSP precision and collision ordering integration.
- Ordinary player caller association and original collision output comparisons.

RECOMMENDED NEXT WORK:
- Qualify runtime global ownership and native world traversal separately; reuse
  stationary/moving single-brush consumers after scene ownership is admitted.

SCOPE NOT IMPLEMENTED:
- World aggregation/traversal, scene/asset-family readers, guessed hulls, motor/ECS,
  original process reads/gameplay/capture, appearance/damage, commits or pushes.
