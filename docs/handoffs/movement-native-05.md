TASK:
Implement independently qualified native command timing and ordinary scalar
SSE-f32 acceleration as reusable numerical motor building blocks.

STATUS:
COMPLETE

FINDINGS:
- Added typed command plans, allocation-free substep iteration and explicit
  single-step durations; no Bevy timing assumptions or profile defaults.
- Added ordinary acceleration with supplied coefficients, exact scalar operand
  order, equality/signed-zero branch choices and atomic checked output.
- Full movement motor and all unresolved behavior remain explicit exclusions.

EVIDENCE:
- Reviewed gameplay source/tests and native02/03/04 contracts plus concurrent diff.
- Native roots23af10/544b30 and ordinary53dc10 branch support this increment.
- External movement-native-05/reference.py independently reconstructs expected
  f32 bits with per-operation stores; fixtures are synthetic, not original data.

CHANGES:
- Added crates/gameplay/src/movement/{mod.rs,timing.rs,acceleration.rs}.
- Added crates/gameplay/tests/movement_kernels.rs and grounded implementation notes.
- Primary added the existing gameplay lib export; no worker manifest/lockfile edits.
- Safe rejection domains are implementation choices: nonfinite arithmetic,
  nonnegative supplied coefficients and unqualified signed timing overflow.

VALIDATION:
- cargo test --locked -p bo2-gameplay --test movement_kernels:12 passed.
- Identical modules/tests under standalone rustc --test -O:12 passed.
- cargo clippy --locked -p bo2-gameplay --all-targets -- -D warnings:passed.
- Owned-file rustfmt --check and git diff --check:passed.
- Initial bare cargo/rustfmt commands were unavailable on PATH; checks succeeded
  using the installed pinned toolchain shims. No toolchain/dependency edits.
- Workspace, headless/client and integration gates remain primary-owned; no
  original comparison, native invocation or gameplay fidelity claim.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Full controller, mode eligibility, x87 policy, profile defaults, collision
  backend/access, stance/camera transitions and original comparison evidence.

RECOMMENDED NEXT WORK:
- Qualify bounded jump/sprint transitions while collision investigation proceeds;
  retain one numerical motor owner across subsequent locomotion increments.

SCOPE NOT IMPLEMENTED:
- step_movement/full numerical motor, x87 friction/projection, gameplay defaults,
  locomotion-mode branches, collision families, ECS facade and host assembly.
