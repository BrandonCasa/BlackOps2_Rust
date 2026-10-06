# Project state

Updated: 2026-10-05. This is the current working brief; historical detail is linked below.

## Integrated damage and life states

The damage goal is paused at the user's request. Reviewed work through
`7e241cc` is integrated into main: safe Bevy Vitality, ordered resolved-damage and
coop revival/expiry commands, stale-token checks, episode-bound bleedout and
sampled revival clocks, Origins configuration and a runnable headless demo.
See the [boundary](architecture/damage-life-state.md) and
[latest comparison handoff](handoffs/damage-life-state-23.md).

Original player health 100 to 40 and a combat-associated actor sequence from
150 to -11 support scoped resolved arithmetic comparisons. Final actor health
remains unknown; raw impacts, live coop timing and solo/session-loss implementation
remain unresolved. [Source24](reverse_engineering/damage-life-state-24.md) separates
solo without available Quick Revive from last-eligible multiplayer; Task25 was
stopped before edits. No observer runs; the original game is under user control.
Fidelity remains APPROXIMATE / INVESTIGATING. Integrated validation is recorded below.

## Objective and working policy

Reimplement the full Black Ops II Zombies Origins game in stable Rust with Bevy,
using the licensed original as the specification. Prioritize executable progress:
inspectable map geometry, measured movement/collision, then a weapon, a zombie,
damage/death and one round before expanding remaining Origins systems.
The full goal is not complete when the first vertical slice works.

Read this brief, the relevant source/tests, and only the evidence needed for the
next code change. The full-map movement phase uses three concurrent workers plus
the primary integration agent, replacing the previous sequential policy by the
user's instruction on 2026-10-05. Keep exclusive module ownership and route shared
contracts, manifests, exports and boot dispatch through the primary. Keep documentation
to a short handoff and an updated current brief; avoid repeating historical audits.
Investigations should resolve a named implementation question. Do not make a
complete capture framework or perfect understanding of every subsystem a
prerequisite for implementing independently supported behavior. Preserve explicit
uncertainty and do not invent original behavior to achieve a visible milestone.

## Working implementation

- Eleven Cargo members: core, assets, physics, rendering, gameplay, Origins configuration,
  boot, asset inspector, map inspector, geometry viewer and damage demo.
  Stable Rust 1.99.0 / Bevy 0.19.1 are pinned; retain the
  existing toolchain unless an actual implementation requirement needs a change.
- `crates/assets`: bounded fastfile envelope inspection and a safe reader for the
  project-owned geometry snapshot format, external schema28 brush records and
  bounded schema34 source-keyed brush collections and complete terrain exchange v1.
  This is not a complete Rust BO2 decoder.
- `crates/rendering`: raw diagnostic Mesh preserves source arrays; canonical runtime
  Mesh and textured scenes adapt source `(x,y,z)` to Bevy `(x,z,-y)` through
  `bo2_core::coordinates`, retaining indices/UVs and source metadata. Runtime +Y-up
  policy is [documented](adr/0002-bevy-runtime-coordinates.md).
- `crates/physics`: safe owned convex halfspaces and deterministic point-segment
  clipping across all 8,417 selected diagnostic map-leaf brushes, including 3,178
  zero-side axial hulls. Queries preserve coefficients and use exact entry/source-key
  ties; precision diagnostics stay separate. A separate single moving-brush adapter
  now implements the evidenced native scalar-f32 request/contact contract with
  explicit shape, mask and incoming result. World/player traces and runtime output
  comparisons remain unresolved.
- `tools/comparison_tools/geometry_viewer`: renders three offscreen raw-coordinate
  solid/wireframe views and a windowed perspective inspection scene with diagnostic
  keyboard flight/look/reset. Accepted external snapshot: 191,259 positions,
  149,907 triangles, 6,469 surfaces. Geometry and decoded assets remain external.
- `game/boot`: a headless lifecycle smoke check. There is no interactive game,
  player movement, player collision integration, weapon, zombie AI or round yet.
  The integrated damage demo runs the shared resolved life-state boundary, rather
  than an interactive game or complete zombie/player controller.
- The integrated viewer adds --scene with a bounded external manifest bound
  to snapshot SHA256. One original surface renders recovered corner UVs and a
  native BC1 texture with all ten mip levels and its source sampler. This is an
  explicitly unlit, linear diagnostic preview, not a complete original material.
- Runtime coordinates now follow Bevy +Y up; original basis calibration, units,
  camera, full materials and lighting remain unresolved.
  Overall/gameplay/visual fidelity is UNKNOWN; data access is INVESTIGATING.

## Immediate implementation direction

The full-map movement phase is active on `codex/origins-full-map-movement`.
Three concurrent lanes qualify native movement, world collision coverage, and
the reusable Bevy map/input host. Investigation uses licensed files and retained
evidence only; no original-game play, capture or process reads. The primary owns
shared interfaces and integration. Appearance and damage remain paused. Completion
requires all five locomotion modes and every participating full-map collision
reference resolved or reproducibly excluded; the existing brush collection alone
does not meet that condition. Native command timing and posture hulls are now
source-qualified; numerical motor rules and a complete executable profile remain pending.

Reviewed host preparation, stationary single-brush and terrain reader integration pass194 workspace
tests, locked metadata, formatting and strict all-target Clippy; both retained brush
opt-ins and client input/pacing checks pass separately. Synthetic reconstructed
window smoke passes with GPU readback. The external terrain opt-in separately
admits all declared terrain arrays and map/model leaf references; numerical terrain
queries remain unimplemented. See [integration01](handoffs/movement-integration-01.md)
and [terrain integration02](handoffs/movement-integration-02.md).
The command owner, explicit150ms backlog/66ms substeps and posture hulls are in
[native02](handoffs/movement-native-02.md). All265 declared model dispositions and
583 existing model brush references are in [ownership04](handoffs/movement-collision-04.md);
serialized null providers are not missing geometry. Runtime scene binding, addon
association, movement solver access and complete collision families remain under
qualification; there is no executable movement motor or full-map coverage yet.
The [grounded contract](reverse_engineering/movement-grounded-contract.md) requires
the receiver/contact solver, beyond classic world traces. [Collision access](reverse_engineering/movement-collision-access.md)
records its provider/pose gaps. [Baseline parameters](reverse_engineering/movement-baseline-parameters.md)
qualify C96 movement/sprint modifiers and script movement scale; parameter defaults
and startup floating-point policy remain unresolved. Timing/SSE kernels are next.

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
2. Qualify the separate stationary wrapper and standing-player request boundary,
   then assemble evidence-backed collision for player movement. Task0036 implements
   the finite-f32 single moving-brush contract on existing admitted records. The authentic
   native consumer clips lower XYZ, upper XYZ, then sides; zero-side records retain
   the axial constraints and stored vertices are not consulted. It uses capsule-like
   support, an entry-only 0.125 raw-unit allowance, distinct startsolid/allsolid/contact
   writes and ordered plane ties. Preserve the unusual upper-Z result overwrite.
   The existing f64 queries remain diagnostic; their arithmetic and tie rules are
   not the native contract. Equal centered endpoints take an uninvestigated wrapper
   route and are rejected by the moving-only adapter. See
   [original contract](reverse_engineering/0024-original-brush-trace-consumer.md).
   Six retained positive/zero point crossings agree exactly with the independent
   scalar reconstruction, not native execution. Player dimensions/masks are explicit
   unknowns; no general box support or world aggregation is claimed.
   [Task0037](handoffs/0037-original-standing-player-trace-request.md) qualifies
   three bounds/mask forwarding functions into that builder, but no ordinary
   standing-player caller or defaults. Resolve a player movement owner/callback
   and its actual bounds writes before integrating player collision.
   All 8,417 candidates remain admitted, but static ownership/transforms and all
   265 absent model providers remain unresolved. Source residuals remain visible
   (30,706 strict violations, maximum 0.09950868785381317 raw units); the original
   contact allowance does not repair them or prove producer rounding.
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
The current full-map movement phase also uses the user's four-agent instruction.

- Previous Goal chat is paused; its worker finished and reports no owned operation
  running. Its hourly auto-resume monitor is also paused.
- The latest reviewed collision increments are linked in the handoffs below.
- Task0026 is now reviewed and retained as historical PARTIAL evidence. Its two
  original reads produced no brush; Task0028 later names the failed bounds.nonnull
  predicate and recovers one. Punctuation is repaired; historical project-state
  and docs-index proposals remain in the snapshots below. Accepting the historical
  record does not establish original geometry or gameplay fidelity.
- Task0037 is reviewed as PARTIAL: bounds/mask forwarding is qualified, ordinary
  standing-player inputs remain unknown. All collision workers are stopped;
  this documentation integration performs no original-game reads or inputs.
- One task-local synthetic executable was overwritten during the old investigation;
  original identity/test receipts survive, but exact old bytes do not. This does
  not establish asset corruption or require redoing unrelated accepted work.

## Validation and essential references

Canonical Bevy coordinate integration passes locked/offline metadata (11 members),
all 171 workspace tests, formatting and strict all-target Clippy; four existing
external collision tests remain ignored. Synthetic and full-map window smoke
checks pass at35 updates with GPU readback and scripted move/look/reset. Map and
texture windows are relaunched and responsive; original visual comparison remains
unverified. See [coordinate handoff](handoffs/viewer-up-axis.md).

Collision documentation integration passes local link, heading, content and
whitespace checks. Outer review verifies Task0026's fourteen retained fixed
identities and Task0037's24 windows, three complete functions and66 worker-closure
identities before review edits; nine forwarding checks rerun successfully without
a process observer.
No production code changed, so the unchanged workspace gates below are not rerun.

Damage integration through `7e241cc` passes locked metadata (eleven members),
all 167 workspace tests, formatting and warning-denying all-target Clippy.
Four external collision tests remain ignored by default. The damage demo passes
21 ordered outcomes plus elapsed bleedout and sampled revival scenarios in main.
Existing collision/rendering implementation and pending collision documents are
preserved. See [integration handoff](handoffs/damage-life-state-integration.md).

Task0036 passes locked/offline metadata (eight members), all 127 workspace tests,
formatting and strict all-target Clippy. Four external tests are ignored by default;
the new retained-input opt-in separately passes. Outer review repeats that opt-in,
its six exact independent-model comparisons and the retained collection inspector.
All 8,417 source records/words and residual totals remain unchanged. No new original
reads or unsafe code; worker execution is stopped. Native outputs, player movement,
world/stationary traces and performance comparisons remain unverified.
Task0035 recovers the native moving-brush/request contract through a named builtin
and direct call chain. Fifteen complete functions (8,515 bytes), registration and
constant identities requalify with read-only handles. Twenty-four synthetic f32
versus rational cases and eight registration checks pass. Outer review independently
checks retained span hashes and reruns all 24 analytic cases. No native invocation,
runtime output comparison or player default is established; unchanged Cargo gates
were not rerun. Worker execution is stopped and handles are closed.
Retained task0034 geometry contains all 17,108 sides and 68,642 vertices; its
producer/checker and task0033 candidate replay were reviewed. The diagnostic graph
reaches 14,551 nodes and excludes the separately identified box. Full source and
test receipts are linked in their handoffs; those results do not prove runtime
static ownership or the unsupported collision families.
The accepted task0027 window checks pass at35 updates on synthetic and external
geometry with scripted input, GPU mesh/pipeline readiness and foreground960x720
readback. Legacy offscreen rendering passes on Vulkan. Physical keyboard routing,
sustained use and original comparisons remain unverified. Run applicable gates:
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
- [Original moving-brush contract](handoffs/0035-original-brush-trace-consumer.md)
- [Rust moving-brush query](handoffs/0036-rust-moving-brush-trace.md)
- [Standing-player request investigation](handoffs/0037-original-standing-player-trace-request.md)
- [Historical partial collision result](handoffs/0026-origins-collision-brush-boundary.md)
- [Collision source/loader evidence](reverse_engineering/0018-origins-collision-brush-boundary.md)
- [Fidelity status](fidelity/README.md)
- [Historical project state](project-state-history-2026-10-05.md) and
  [historical document index](history-index-2026-10-05.md): reference only, not a startup reading list.

Keep real asset/tool/output paths only in ignored local configuration. Original
assets, scripts, binaries and generated proprietary data stay outside Git. Work
locally and do not push without the user's instruction.
