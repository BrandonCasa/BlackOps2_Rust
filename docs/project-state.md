# Project state

Updated: 2026-10-05. This is the current working brief; historical detail is linked below.

## Active isolated damage/life-state goal

This worktree owns shared player/ordinary-zombie damage and life states, Origins
configuration, a runnable headless demonstration, deterministic tests and original
comparisons. See [ownership and acceptance](architecture/damage-life-state-goal.md).
Collision/movement and static-world appearance continue in separate worktrees.
The user explicitly permits one active delegated worker per worktree; review and
sequence workers locally. Reviewed damage, revive-script and runtime restoration
evidence is linked from the [implemented boundary](architecture/damage-life-state.md).
Shared safe Bevy components now process resolved damage and ordinary coop revival
begin/cancel/completion in explicit fixed-step request order. Checked episode and
attempt tokens reject stale completions; Origins supplies qualified no-bonus
restoration of current/max health 100. Component replacement requires caller input
retirement. See [implementation handoff](handoffs/damage-life-state-05.md).

Validation: locked metadata has 10 members; all 102 workspace tests,
formatting and warning-denying Clippy pass. The demo verifies 21 exact ordered
outcomes and exits 0. Outer repeated 22 focused tests and the demo after range correction.
Scoped fidelity is APPROXIMATE: active native dispatch, final actor health,
raw interaction/timing and full live agreement remain unresolved. The first
[original comparison attempt](fidelity/damage-life-state-comparison.md) is PARTIAL;
unexplained input changes prevented controlled outcome attribution. The user then
granted session control. Solo follow-up observed game-over/results; the latest
[numeric comparison](fidelity/damage-life-state-comparison.md) captured entity and
client mirror health 100 to 40, agreeing with the Rust resolved-60 case. Exact
contact count/timing and return-to-100 cause remain unqualified. Bleedout comparison
needs multiple players.
Reviewed [bleedout evidence](reverse_engineering/damage-life-state-06.md) finds a
source-default 45-second countdown, post-countdown deferral during active revival
and ordinary spectator disposition. Final spectator health remains unqualified.
The reviewed [terminal implementation](handoffs/damage-life-state-07.md) adds
episode-bound expiry, active-revive deferral and explicit HumanTerminal state while
preserving qualified numeric health. Timing admission is external; the source
network-frame interval is explicitly collapsed. A bounded external read-only
[health observer](handoffs/damage-life-state-08.md) qualifies a paused solo baseline
of current/entity-max/client-max 100, with 15 synthetic checks. Its first live run
accepted 966 unchanged baseline samples and refused 34 unstable samples; it did
not establish damage sensitivity. Online mod-author notes corroborate baseline
health 100 and melee damage 60, separately from live proof. The subsequent
[diagnostic trial](reverse_engineering/damage-life-state-09.md) accepted 972 unchanged
100/100/100 samples and refused 28 solely for changing local-state pointers;
irrelevant flag changes do not explain that run. The subsequent
[native writer trace](reverse_engineering/damage-life-state-10.md) confirms signed
subtraction at that exact field and an integer-only positive damage argument;
active VM dispatch and execution remain unqualified. The reviewed
[numeric correction](handoffs/damage-life-state-11.md) now admits positive signed
32-bit damage/health/restoration values and rejects oversized inputs atomically.
Three bounded [dispatch leads](reverse_engineering/damage-life-state-12.md) found
no active VM slot; that survey is closed without proving execution or overrides.
The [mirror trial](reverse_engineering/damage-life-state-13.md) confirms live field
sensitivity: 43 accepted samples at 40/40, with maxima 100/100, out of 957 accepted
and 43 refused records. Next repeat an isolated-hit comparison and qualify one
ordinary actor terminal transition, then qualify timing admission in multiplayer;
the [actor native trace](reverse_engineering/damage-life-state-14.md) now supports
nonlethal subtraction and shows negative health retained/floored at -999 before
the death handler. The bounded [handler follow-up](reverse_engineering/damage-life-state-15.md)
qualifies an active AI/entity backlink and selected handler, which clears the
repeat-impact gate. Ordinary archetype and callback/cleanup health effects remain
unqualified. Keep ActorDead health 0 explicitly approximate; next prefer a controlled
ordinary-actor final-health comparison over another broad callback survey.
The [identity check](reverse_engineering/damage-life-state-16.md) qualifies native
classname/targetname/noteworthy token fields, but not their selected-entity decoding
or special-spawn exclusions. Fresh round one with no objective activation supports
controlled inference only. Use those explicit limits in the next finite actor trial.
The [selected-actor trial](fidelity/damage-life-state-comparison.md) accepted 318
unchanged 150/150/gate1 samples, then stopped on a generic identity refusal; native
profile requalification passed. Solo game over preceded an established melee contact.
This gives no actor damage/death comparison. Reset earlier and improve refusal
diagnostics for a controlled repeat; do not infer terminal health from the refusal.
The [diagnostic repeat](reverse_engineering/damage-life-state-18.md) passed 15
synthetic checks and a fresh five-sample baseline. Its live run accepted 319 unchanged
150/150/gate1 samples before refusing changed active-byte flags with the admitted
active bit still set. Aiming input changed the view and fired one shot; no hit or
actor death was proven. The original is left at the fresh match's pause menu.
Solo supplies no bleedout proof. Launch the
original through the desktop Plutonium launcher; Origins uses the robot-head
planet icon. These increments do not complete original comparisons or the goal.
## Objective and working policy

Reimplement the full Black Ops II Zombies Origins game in stable Rust with Bevy,
using the licensed original as the specification. Prioritize executable progress:
inspectable map geometry, measured movement/collision, then a weapon, a zombie,
damage/death and one round before expanding remaining Origins systems.
The full goal is not complete when the first vertical slice works.

Read this brief, the relevant source/tests, and only the evidence needed for the
next code change. Keep one delegated worker active at a time. Keep documentation
to a short handoff and an updated current brief; avoid repeating historical audits.
Investigations should resolve a named implementation question. Do not make a
complete capture framework or perfect understanding of every subsystem a
prerequisite for implementing independently supported behavior. Preserve explicit
uncertainty and do not invent original behavior to achieve a visible milestone.

## Working implementation

- Ten Cargo members: core, assets, rendering, gameplay, Origins configuration,
  boot, asset inspector, map inspector, geometry viewer and damage demo.
  Stable Rust 1.99.0 / Bevy 0.19.1 are pinned; retain the
  existing toolchain unless an actual implementation requirement needs a change.
- `crates/assets`: bounded fastfile envelope inspection and a safe reader for the
  project-owned geometry snapshot format. This is not a complete Rust BO2 decoder.
- `crates/rendering`: preserves snapshot positions/indices in a Bevy Mesh.
- `tools/comparison_tools/geometry_viewer`: renders three offscreen raw-coordinate
  solid/wireframe views and a windowed perspective inspection scene with diagnostic
  keyboard flight/look/reset. Accepted external snapshot: 191,259 positions,
  149,907 triangles, 6,469 surfaces. Geometry and decoded assets remain external.
- `game/boot`: a headless lifecycle smoke check. There is no interactive game,
  player movement, collision implementation, weapon, zombie AI or round yet.
  The separate damage demo implements a resolved life-state boundary, not an
  interactive game or complete zombie/player controller.
- Original orientation, units, camera, materials and lighting remain unresolved.
  Overall/gameplay/visual fidelity is UNKNOWN; data access is INVESTIGATING.

## Immediate implementation direction

1. Use the existing interactive inspection scene when useful; its free-camera
   axes, speed and projection are diagnostic, not original movement. Reuse the
   accepted external snapshot instead of restarting asset extraction.
2. Resolve the collision reader's concrete failure toward
   a standing movement slice: task0026 rejects at embedded ClipInfo admission,
   before selecting any brush. Check the existing separate `pInfo` loader lead and
   identify the failing count/null/span predicate. Do not repeat broad surveys.
3. Implement a reusable Rust collision/movement boundary once the required
   original evidence is available, with focused behavioral/error-path tests.
   Camera-capture difficulties must not indefinitely block independent gameplay work.
4. Continue through one evidenced weapon, zombie, damage/death and round, then
   compare and expand toward the full Origins scope. Keep visible progress and
   behavior comparisons distinct from scaffolding and synthetic correctness.

RenderDoc/process-protection work is parked. The static camera fallback established
no usable camera locator. Reopen either only when a specific gameplay/rendering
measurement needs it and there is a concrete new route; historical tool-preparation
work does not itself advance the implemented game.

## Restart and pending work

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
- [Pending collision result](handoffs/0026-origins-collision-brush-boundary.md)
- [Collision source/loader evidence](reverse_engineering/0018-origins-collision-brush-boundary.md)
- [Fidelity status](fidelity/README.md)
- [Historical project state](project-state-history-2026-10-05.md) and
  [historical document index](history-index-2026-10-05.md): reference only, not a startup reading list.

Keep real asset/tool/output paths only in ignored local configuration. Original
assets, scripts, binaries and generated proprietary data stay outside Git. Work
locally and do not push without the user's instruction.
