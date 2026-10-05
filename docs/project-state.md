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

- Eight Cargo members: core, assets, physics, rendering, boot, asset inspector, map inspector,
  and the geometry viewer. Stable Rust 1.99.0 / Bevy 0.19.1 are pinned; retain the
  existing toolchain unless an actual implementation requirement needs a change.
- `crates/assets`: bounded fastfile envelope inspection and a safe reader for the
  project-owned geometry snapshot format, external schema28 brush records and
  bounded schema34 source-keyed brush collections.
  This is not a complete Rust BO2 decoder.
- `crates/rendering`: preserves snapshot positions/indices in a Bevy Mesh.
- `crates/physics`: safe owned convex halfspaces and deterministic point-segment
  clipping across all 8,417 selected diagnostic map-leaf brushes, including 3,178
  zero-side axial hulls. Queries preserve coefficients and use exact entry/source-key
  ties; precision diagnostics stay separate. Original runtime ownership, player
  traces and zero-side source shape remain unresolved.
- `tools/comparison_tools/geometry_viewer`: renders three offscreen raw-coordinate
  solid/wireframe views and a windowed perspective inspection scene with diagnostic
  keyboard flight/look/reset. Accepted external snapshot: 191,259 positions,
  149,907 triangles, 6,469 surfaces. Geometry and decoded assets remain external.
- `game/boot`: a headless lifecycle smoke check. There is no interactive game,
  player movement, player collision integration, weapon, zombie or round yet.
- The integrated viewer adds --scene with a bounded external manifest bound
  to snapshot SHA256. One original surface renders recovered corner UVs and a
  native BC1 texture with all ten mip levels and its source sampler. This is an
  explicitly unlit, linear diagnostic preview, not a complete original material.
- Original orientation, units, camera, full materials and lighting remain unresolved.
  Overall/gameplay/visual fidelity is UNKNOWN; data access is INVESTIGATING.

## Immediate implementation direction

The reviewed first texture increment and selected opaque material contract are
integrated. All five selected image dependencies resolve; shader equations are
recovered, but packed normal/tangent/lightmap-UV formats, runtime technique
selection, global bindings and transfer conventions remain unproved. Original
shaded-scene admission is unsupported. Remaining 6,468 surfaces and complete
original shading/lighting are unsupported; gray diagnostics remain available.
See [texture increment](handoffs/materials-0001-textured-surface.md) and
[material contract](handoffs/materials-0002-opaque-contract.md). The appearance
worker is stopped; this appearance goal pauses after integration at the user's
request. Further appearance implementation requires resuming that work.
An original spawn observation was captured externally through Plutonium LAN;
measured camera conditions and equivalent reconstructed comparisons remain pending.

1. Use the existing interactive inspection scene when useful; its free-camera
   axes, speed and projection are diagnostic, not original movement. Reuse the
   accepted external snapshot instead of restarting asset extraction.
2. Use the integrated `--brush-collection` queries and retained task0034 geometry
   to resolve a bounded source/convention discrepancy before player integration.
   All 8,417 task0033 candidates are admitted without omissions; the separate box
   is excluded. Traversal remains diagnostic and all 265 model providers are absent,
   so original static ownership/transforms remain unproved. Full source scans find
   30,706 strict violations across 4,186 brushes, with maximum positive residual
   0.09950868785381317 raw units. A zero-side representative has 12 source vertices
   versus eight axial corners and vertex-set error 4.0. Geometry is preserved;
   no snapping, contact epsilon or silent exclusion hides those discrepancies.
   Original producer/runtime rounding and zero-side source shape remain unresolved.
3. Measure the player hull, collision masks and step/slide rules before integrating
   movement. Point-segment queries do not establish original player traces.
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

Source-only research from `codex/origins-mechanics-research` is merged into main
at `8af13e8`; see
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
- The latest reviewed collision increments are linked in the handoffs below.
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

Task0034 integrates the bounded collection reader, owned hull sequence and inspector.
All 114 workspace tests, locked metadata, formatting and strict Clippy pass; three
opt-in external tests separately pass. Outer review reruns both collection external
tests and the retained collection/legacy-brush inspectors successfully. Native
qualification passes 25 actual cases and 13 independent rejection cases; the single
original read retains all 17,108 sides and 68,642 vertices. Child/threads are cleaned and prior
identities preserved. Original player collision remains UNKNOWN; no game trace,
GPU integration or performance comparison is claimed.
Task0033 passes 39 actual native cases and 16 independent checker rejection cases.
Its single original read retains a 2,634,532-byte metadata graph; replay agrees on
8,417 candidate IDs, 14,551 reached nodes and 40,004 charged work units. Box overlap
is absent under the diagnostic traversal. Input identities agree and child/threads
are cleaned. Tasks0031/0032 resolve the rejected tuple and separate box association;
task0030's 23 native/nine checker cases remain synthetic evidence.
These documentation-only results add no Rust implementation; unchanged workspace
gates were not repeated.
Task0029 adds brush admission and executable segment queries: all 92 workspace
tests, locked metadata, formatting and Clippy pass. The opt-in external record
test separately passes every raw word/metadata check; outer original inspector
execution also passes. Original player/gameplay fidelity remains unverified.
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
- [Rust brush collision boundary](handoffs/0029-rust-brush-collision.md)
- [Unresolved world-brush membership](handoffs/0030-origins-world-brush-membership.md)
- [One-past leaf-brush identifier](handoffs/0031-origins-leaf-brush-index-namespace.md)
- [Separate box-brush association](handoffs/0032-origins-box-brush-membership.md)
- [Diagnostic map-leaf membership](handoffs/0033-origins-map-leaf-brush-membership.md)
- [Rust brush collection](handoffs/0034-rust-brush-collection.md)
- [Pending collision result](handoffs/0026-origins-collision-brush-boundary.md)
- [Collision source/loader evidence](reverse_engineering/0018-origins-collision-brush-boundary.md)
- [Fidelity status](fidelity/README.md)
- [Historical project state](project-state-history-2026-10-05.md) and
  [historical document index](history-index-2026-10-05.md): reference only, not a startup reading list.

Keep real asset/tool/output paths only in ignored local configuration. Original
assets, scripts, binaries and generated proprietary data stay outside Git. Work
locally and do not push without the user's instruction.
