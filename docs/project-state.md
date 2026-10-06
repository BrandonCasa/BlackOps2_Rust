# Project state

Updated: 2026-10-05. Read this brief, then the source/tests and evidence needed for
the next bounded task. Historical snapshots are lookup references.

## Objective and policy

Reimplement the complete Black Ops II Zombies Origins experience in stable Rust
with Bevy, preserving observable original behavior and appearance. Prioritize
correctness, safety, reusable architecture, fidelity and deterministic gameplay
before performance. A scaffold or vertical slice is not the complete goal.

The active full-map movement phase uses three concurrent workers plus the primary,
replacing sequential delegation at the user's instruction. Workers exclusively
own assigned modules/tests/handoffs; primary owns shared exports, manifests,
lockfile, boot dispatch, integration tests, project state and reviewed local commits.
Investigations resolve named implementation questions and release ready code work.
Use licensed files and retained evidence; no original play, capture, process reads
or native invocation during this phase. Appearance and damage remain paused.

Original assets and inspection outputs stay external. Machine paths belong only
in ignored local configuration. Keep safe crates' unsafe prohibition, Bevy 0.19.1,
stable Rust 1.99.0 and minimal default features. Enable client features locally.
Work locally; no remote push is authorized.

## Implemented boundaries

- Eleven Cargo members. [Architecture](architecture/README.md) owns dependency
  direction and directory responsibilities; Bevy owns ECS, schedules and rendering.
- Assets admits bounded geometry snapshots, schema28 brushes, schema34 brush
  collections, complete terrain, global collision hierarchy and static-model
  exchanges. It is not a full BO2 decoder.
- Physics implements diagnostic segment queries plus source-qualified moving and
  stationary single-brush adapters, moving/stationary brush-leaf traversal and raw
  per-instance static-point queries. World/family aggregation and the player
  receiver/contact solver remain unimplemented.
- Rendering preserves the accepted mesh arrays and metadata. Canonical runtime
  mapping is source `(x,y,z)` -> Bevy `(x,z,-y)`; gray display is reusable.
- Viewer retains offscreen/windowed diagnostic inspection and one unlit BC1/UV
  material preview. Its flight camera is diagnostic, not original player movement.
- Boot has the headless lifecycle check and persistent ordered input/host modules.
  There is no full motor, movement replay CLI or interactive Origins controller.
- Gameplay's reviewed life-state boundary handles resolved damage, revival and
  bleedout. Original raw impacts, complete solo/session-loss behavior and final
  actor health remain unresolved. See [damage integration](handoffs/damage-life-state-integration.md).

## Active movement delivery

Completion requires walking, sprinting, jumping, crouching, prone and authentic
camera behavior, the same numerical motor for headless/windowed replay, persistent
input across zero/multiple fixed updates, and resolution or reproducible source
exclusion of every participating full-map collision reference. Unsupported families
cannot be silently skipped. Bevy's fixed rate does not establish native timing.

Reviewed integration currently passes 261 workspace tests (9 external opt-ins
ignored), locked metadata, formatting and warning-denying all-target Clippy.
Hierarchy/static payload opt-ins and all 4,791 stationary brush-only leaf probes
pass separately. Client host tests pass (8), with warning-denying client Clippy.
Earlier brush/terrain opt-ins, input/pacing checks and a synthetic
reconstructed-window GPU smoke pass; sustained physical input/full movement and
original comparisons remain unverified. See [integration01](handoffs/movement-integration-01.md)
and [reviewed integration03](handoffs/movement-integration-03.md).

Current contracts and limits:

- [Native command/update](reverse_engineering/movement-command-update.md): 60-byte
  command shape, ordinary masks/posture hulls, 150 ms backlog and 66 ms substeps.
- [Grounded source](reverse_engineering/movement-grounded-contract.md): friction,
  acceleration, slope and step rules. [Kernel05](handoffs/movement-native-05.md)
  supplies independently qualified timing/SSE building blocks. Integer sprint
  accounting and atomic stop kernels are also implemented. Full motor, x87 policy
  and mode eligibility remain unresolved.
- [Jump](reverse_engineering/movement-jump-contract.md),
  [sprint](reverse_engineering/movement-sprint-contract.md) and
  [stance](reverse_engineering/movement-stance-contract.md) contracts distinguish
  qualified transitions from missing providers. Effective posture uses target
  height; blocked clearance can retain native events and transition side effects.
- [Baseline parameters](reverse_engineering/movement-baseline-parameters.md): script
  movement scale1, C96 ordinary movement/sprint modifiers1, sprint4000ms/cooldown0.
  Stop speed, base speed, gravity and other parameter registrations/defaults are
  unresolved; no guessed executable profile is admitted.
- [Model ownership](reverse_engineering/collision-global-ownership.md): all265
  cmodel dispositions;583 model brush references already resolve in schema34.
  Serialized null providers do not imply missing geometry. Runtime scene/addon
  binding, model poses and activation remain unresolved.
- [Terrain admission](reverse_engineering/terrain-payload-admission.md): complete
  arrays/refs and all map/model leaves, including214 root-zero terrain leaves.
  Numerical capsule/edge/vertex queries, stamp lifecycle and packed-edge semantics
  remain to implement/qualify.
- [Static XModel contract](reverse_engineering/static-xmodel-collision-contract.md):
  all 10,837 instances resolve 420 shared providers. Complete bounded payload
  admission and raw per-instance classic point queries are implemented. Sector
  setup and player-hull binding remain pending. Contents-zero exclusions apply
  only to the proven route.
- [Collision access](reverse_engineering/movement-collision-access.md) and
  [effective producer gap](reverse_engineering/movement-collision-receiver-producer.md):
  motor uses receiver shapes/candidate contacts beyond classic traces. Effective
  callback, receiver initialization and candidate world/pose bindings are missing.
- [Floating-point policy](reverse_engineering/movement-floating-point-policy.md)
  establishes a startup-thread 53-bit x87 precision request; movement-thread
  rounding/MXCSR binding remains unknown. [Camera](reverse_engineering/first-person-camera-contract.md)
  qualifies angles/basis and eye-effect ownership; runtime parameters and final
  view/projection publication remain unresolved.
- [Coverage ledger](reverse_engineering/player-world-contract.md) tracks hierarchy,
  terrain, static XModels, transformed models, dynents and separate box scope.

Ready lanes: terrain point numerical queries, movement-thread floating-point
binding and baseline startup overrides proceed in parallel. Keep one
numerical motor owner across all locomotion modes. Shared
PlayerCollisionWorld/scene and MovementState/Command/Profile/Step/Error contracts
must reflect qualified source needs; do not substitute a generic trace-only motor.

Overall gameplay/visual fidelity remains UNKNOWN. Source/data interpretation is
INVESTIGATING. Synthetic tests/native-flow reconstructions do not justify MATCHED
or VERIFIED without original comparison evidence.

## Subsequent work and references

After movement: one weapon, one zombie controller, existing damage integration and
an ordinary round, then economy, generators, inventory/buildables, robots, staffs,
quests, audio/HUD and cooperative behavior toward the complete game.
[Origins mechanics research](reverse_engineering/origins-mechanics/README.md) is
merged research, not production implementation or comparison proof.

Appearance is paused after the selected opaque-material contract/one-texture
increment; see [material handoff](handoffs/materials-0002-opaque-contract.md).
RenderDoc/process-protection preparation is parked. Previous Goal/monitor work is
paused; this phase adds no automation or original-game observer.

Baseline gates: locked workspace metadata/tests, cargo fmt --all -- --check and
locked all-target Clippy with -D warnings. Validate headless/client separately.
Use existing Cargo installation if it is absent from PATH; do not reinstall it.
[Root README](../README.md) retains boot/viewer commands.

[Historical project state](project-state-history-2026-10-05.md) and
[historical index](history-index-2026-10-05.md) retain earlier milestone details.
Task handoffs/evidence carry detailed receipts; this brief does not duplicate them.
