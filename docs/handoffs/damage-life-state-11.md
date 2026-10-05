TASK:
Constrain resolved damage and lifecycle health admission to positive signed32 domain.

STATUS:
COMPLETE

FINDINGS:
- All current/max/downed health, restoration and damage inputs admit1..=i32::MAX;
  u32 API/storage retained without truncation. Typed range errors reject oversized
  values before mutation; prior zero/relative errors remain for in-range values.
- ActorDead alone retains internal current0. Actor shared domain is a conservative
  project restriction; actor arithmetic remains inferred. Player ordinary positive
  subtraction is statically qualified, not demonstrated active dispatch/execution.

EVIDENCE:
- Reviewed native10, handoff10, current state/architecture and gameplay/config tests;
  [native findings](../reverse_engineering/damage-life-state-10.md).
- No original-game data/process/UI access in this implementation task.

CHANGES:
- gameplay/src/lib.rs and tests/damage.rs, revive.rs, terminal.rs: validated range,
  admitted maxima, rejected i32MAX+1/u32MAX, atomic failed restoration coverage.
- Architecture numeric-domain/evidence status; this handoff. No dependency changes.
- Demo final diagnostic and architecture README now distinguish statically qualified
  player subtraction, inferred actor arithmetic and unqualified dispatch/comparison.

VALIDATION:
- `cargo test -p bo2-gameplay -p bo2-origins --locked`:22 tests pass, exit0.
- `cargo metadata --locked --no-deps --format-version 1`:exit0, unchanged10 members.
- `cargo test --workspace --locked`:102 tests pass, exit0.
- `cargo fmt --all -- --check`:exit0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`:exit0.
- `cargo run -p bo2-damage-life-state-demo --locked`:21 asserted outcomes, exit0.
- After summary corrections: fmt check, demo and focused demo-package all-target
  warning-denying Clippy pass, exit0; all21 outcomes preserved.
- Diff/content review and `git diff --check`:pass; only assigned source/tests/docs,
  no proprietary output, local asset paths or unsafe/dependency additions.

FIDELITY STATUS:
APPROXIMATE

UNRESOLVED:
- Active VM dispatch/overrides, upstream conversion, actor native arithmetic,
  and scoped original damage/coop comparisons. Synthetic gates are not live proof.

RECOMMENDED NEXT WORK:
- Qualify active native player dispatch and obtain a controlled numeric comparison.

SCOPE NOT IMPLEMENTED:
- Lethal-policy/timer changes, solo/session behavior, raw impacts, UI/native tooling,
  original sampling, commits/pushes or adjacent systems.
