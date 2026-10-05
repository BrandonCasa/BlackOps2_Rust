# Project state

Updated: 2026-10-05. This is the current working brief; historical detail is linked below.

## Objective and working policy

Reimplement the full Black Ops II Zombies Origins game in stable Rust with Bevy,
using the licensed original as the specification. Prioritize executable progress:
inspectable map geometry, measured movement/collision, then a weapon, a zombie,
damage/death and one round before expanding remaining Origins systems.
The full goal is not complete when the first vertical slice works.

Read this brief, the relevant source/tests, and only the evidence needed for the
next code change. Keep one delegated worker active per worktree (user clarification
on 2026-10-05); independent worktrees may run concurrently. Keep documentation
to a short handoff and an updated current brief; avoid repeating historical audits.
Investigations should resolve a named implementation question. Do not make a
complete capture framework or perfect understanding of every subsystem a
prerequisite for implementing independently supported behavior. Preserve explicit
uncertainty and do not invent original behavior to achieve a visible milestone.

## Working implementation

- Seven Cargo members: core, assets, rendering, boot, asset inspector, map inspector,
  and the geometry viewer. Stable Rust 1.99.0 / Bevy 0.19.1 are pinned; retain the
  existing toolchain unless an actual implementation requirement needs a change.
- `crates/assets`: bounded fastfile envelope inspection and a safe reader for the
  project-owned geometry snapshot format. This is not a complete Rust BO2 decoder.
- `crates/rendering`: preserves snapshot positions/indices in a Bevy Mesh.
- External collision evidence now contains one bounded Origins brush (four sides,
  sixteen vertices). Raw data remains external; there is no Rust collision adapter.
- `tools/comparison_tools/geometry_viewer`: renders three offscreen raw-coordinate
  solid/wireframe views and a windowed perspective inspection scene with diagnostic
  keyboard flight/look/reset. Accepted external snapshot: 191,259 positions,
  149,907 triangles, 6,469 surfaces. Geometry and decoded assets remain external.
- `game/boot`: a headless lifecycle smoke check. There is no interactive game,
  player movement, collision implementation, weapon, zombie or round yet.
- Original orientation, units, camera, materials and lighting remain unresolved.
  Overall/gameplay/visual fidelity is UNKNOWN; data access is INVESTIGATING.

## Immediate implementation direction

1. Use the existing interactive inspection scene when useful; its free-camera
   axes, speed and projection are diagnostic, not original movement. Reuse the
   accepted external snapshot instead of restarting asset extraction.
2. Use the retained task0028 brush to resolve its float32 storage/solver discrepancy
   and build the safe Rust collision boundary. Header admission is resolved:
   core arrays are present; parallel bounds/contents and `pInfo` are absent.
   One plane convention reconstructs exact bounds, but vertex disagreement reaches
   0.0005449587606563 raw units. No full geometric compatibility is established.
3. Implement a reusable Rust collision/movement boundary using that evidence,
   with focused behavioral/error-path tests and explicit remaining uncertainty.
   Camera-capture difficulties must not indefinitely block independent gameplay work.
4. Continue through one evidenced weapon, zombie, damage/death and round, then
   compare and expand toward the full Origins scope. Keep visible progress and
   behavior comparisons distinct from scaffolding and synthetic correctness.

RenderDoc/process-protection work is parked. The static camera fallback established
no usable camera locator. Reopen either only when a specific gameplay/rendering
measurement needs it and there is a concrete new route; historical tool-preparation
work does not itself advance the implemented game.

## Restart and pending work

### Origins mechanics research

Dedicated source-only research is on `codex/origins-mechanics-research`; see
[coverage and gaps](reverse_engineering/origins-mechanics/README.md),
[proposed shared boundaries](architecture/origins-mechanics-proposal.md) and
[dependency-ordered tasks](reverse_engineering/origins-mechanics/implementation-backlog.md).
Original compiled-script interpretation supports representative round accounting,
perk purchase, generator capture and inventory/progression transitions. These
documents propose future gameplay work; they add no production Rust or fidelity
comparison. Research fidelity is INVESTIGATING, gameplay remains UNKNOWN.
The other chats own collision/movement, appearance and damage/downing/revival.
Shared original-game input requires runtime-owner coordination.
This research uses the user's specific four-worker exception; ordinary
implementation keeps the repository's sequential-delegation policy.

- Previous Goal chat is paused; its worker finished and reports no owned operation
  running. Its hourly auto-resume monitor is also paused.
- Last accepted implementation/evidence commit before this consolidation: `32f8e9f`.
- Task0026 is PARTIAL and was not accepted by outer review. Its two original reads
  produced no collision brush. Preserve its pending files and external evidence;
  do not treat synthetic passes as a successful original-data read.
- Pending task0026 files: findings0018, handoff0026, and the reverse-engineering
  index. The old project-state and docs-index edits are preserved in the historical
  snapshots below. The old chat reports punctuation encoding defects in three
  candidate documents; repair locally when reviewing them, without another audit cycle.
- One task-local synthetic executable was overwritten during the old investigation;
  original identity/test receipts survive, but exact old bytes do not. This does
  not establish asset corruption or require redoing unrelated accepted work.

## Validation and essential references

Task0027 adds executable inspection controls: all 80 workspace tests, locked
metadata, formatting and Clippy pass. Outer bounded window checks on synthetic
and accepted external geometry each pass at35 updates with scripted input,
prepared GPU mesh/pipelines and foreground960x720 readback. Legacy synthetic
offscreen rendering also passes on Vulkan. Physical keyboard routing, sustained
use and original comparisons remain unverified. Run applicable gates:
`cargo metadata --locked
--no-deps --format-version 1`, `cargo test --workspace --locked`, `cargo fmt --all
-- --check`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`.
If Cargo is absent from the shell PATH, use the existing user Cargo installation.
Use `cargo run -p bo2-boot -- --smoke` for the headless boot check; see the root
README for the existing snapshot viewer commands. Do not reinstall working tools.

- [Architecture and crate responsibilities](architecture/README.md)
- [Snapshot contract](architecture/diagnostic-geometry-snapshot.md)
- [Accepted renderer milestone](handoffs/0020-bevy-offscreen-geometry-diagnostic.md)
- [Interactive inspection milestone](handoffs/0027-interactive-geometry-inspection.md)
- [One recovered collision brush](handoffs/0028-origins-collision-brush-recovery.md)
- [Pending collision result](handoffs/0026-origins-collision-brush-boundary.md)
- [Collision source/loader evidence](reverse_engineering/0018-origins-collision-brush-boundary.md)
- [Fidelity status](fidelity/README.md)
- [Historical project state](project-state-history-2026-10-05.md) and
  [historical document index](history-index-2026-10-05.md): reference only, not a startup reading list.

Keep real asset/tool/output paths only in ignored local configuration. Original
assets, scripts, binaries and generated proprietary data stay outside Git. Work
locally and do not push without the user's instruction.
