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
  per-instance static-point queries, raw stationary terrain spheres, ordered terrain point aggregation and a
  supplied-transform model adapter. Full world aggregation and the player
  receiver/contact solver remain unimplemented.
- Rendering preserves the accepted mesh arrays and metadata. Canonical runtime
  mapping is source `(x,y,z)` -> Bevy `(x,z,-y)`; gray display is reusable.
- Viewer retains diagnostic inspection and native shaded `--native-fly` preview.
  Flight is diagnostic, not original player movement. See the scoped rendering
  integration below.
- Boot has the headless lifecycle check and persistent ordered input/host modules.
  There is no full motor, movement replay CLI or interactive Origins controller.
- Gameplay's reviewed life-state boundary handles resolved damage, revival and
  bleedout. Original raw impacts, complete solo/session-loss behavior and final
  actor health remain unresolved. See [damage integration](handoffs/damage-life-state-integration.md).

## Accepted shaded flight preview

The user's accepted current shader preview is now integrated with a free camera.
`--native-fly` accepts external full-scene, captured-frame, shader-bank and secondary
stream manifests. It admits **6,296/6,469 surfaces** in 718 grouped draws; 173 lacking
captured family/state/vertex contracts are explicitly omitted. All 1,113 translated
shader handles are retained. Shared images, source material constants, secondary
lightmaps/probes and evidenced native states use the existing Bevy material bridge.

WASD flies, Q/E moves down/up, arrows look, Shift speeds up, Control slows down,
R resets exactly, Escape closes. Bevy coordinates remain `(x,z,-y)`. Interactive
attachments clear instead of retaining original weapon/silhouette pixels. GPU
initial/moved-and-rotated/reset views pass, including pixel-exact reset. Workspace
metadata, tests, formatting and strict Clippy pass in the rendering worktree.

Runtime lighting/fog/screen-space resources stay frozen; additional passes,
post-processing and arbitrary-view transparency ordering are unresolved. This is
an accepted inspection preview, not complete scene/game fidelity. Status remains
INVESTIGATING. See [flight integration](handoffs/native-full-map-flight.md),
[camera contract](handoffs/native-world-camera.md) and
[captured dispatch](handoffs/captured-world-integration.md).

## Active movement delivery

Completion requires walking, sprinting, jumping, crouching, prone and authentic
camera behavior, the same numerical motor for headless/windowed replay, persistent
input across zero/multiple fixed updates, and resolution or reproducible source
exclusion of every participating full-map collision reference. Unsupported families
cannot be silently skipped. Bevy's fixed rate does not establish native timing.

The movement checkpoint passed 312 workspace tests (10 external opt-ins
ignored), locked metadata, formatting and warning-denying all-target Clippy.
Hierarchy/static payload opt-ins and all 4,791 stationary brush-only leaf probes
pass separately. Client host tests pass (8), with warning-denying client Clippy.
Earlier brush/terrain opt-ins, input/pacing checks and a synthetic
reconstructed-window GPU smoke pass; sustained physical input/full movement and
original comparisons remain unverified. See [integration01](handoffs/movement-integration-01.md)
and [stable checkpoint integration06](handoffs/movement-integration-06.md).

Combined local main integration passes352 workspace tests (17 external/GPU
opt-ins ignored), locked metadata, formatting and warning-denying all-target
Clippy. Separate client host tests pass8 with client Clippy. Both movement and
the existing material work are preserved; no full player replay or new original
comparison is implied. See [main integration](handoffs/main-movement-integration.md).

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
  Ordered jump eligibility/button mutation and integer posture timing kernels are
  executable; launch, full stance transitions and clearance assembly remain pending.
- [Baseline parameters](reverse_engineering/movement-baseline-parameters.md): script
  movement scale1, C96 ordinary movement/sprint modifiers1, sprint4000ms/cooldown0.
  Stop speed, base speed, gravity and other parameter registrations/defaults are
  unresolved; [startup override inspection](reverse_engineering/movement-baseline-startup-overrides.md)
  narrows the inspected script route but supplies no bound defaults. No guessed
  executable profile is admitted.
- [Model ownership](reverse_engineering/collision-global-ownership.md): all265
  cmodel dispositions;583 model brush references already resolve in schema34.
  Serialized null providers do not imply missing geometry. Runtime scene/addon
  binding, model poses and activation remain unresolved.
- [Terrain admission](reverse_engineering/terrain-payload-admission.md): complete
  arrays/refs and all map/model leaves, including214 root-zero terrain leaves.
  Ordered point aggregation reuses map/model leaves and a fresh exclusive stamp
  workspace. Generation wrap errors explicitly. The raw stationary sphere kernel
  preserves admitted static arithmetic, including its unusual signs; effective
  original runtime equivalence is unverified. Capsule/edge/vertex/aggregation
  queries, reset scheduling and packed-edge semantics remain to implement/qualify.
- [Static XModel contract](reverse_engineering/static-xmodel-collision-contract.md):
  all 10,837 instances resolve 420 shared providers. Complete bounded payload
  admission and raw per-instance classic point queries are implemented. Sector
  setup and player-hull binding remain pending. Contents-zero exclusions apply
  only to the proven route.
- [Transformed models](reverse_engineering/transformed-submodel-trace-contract.md)
  reuse supplied consumers with source-ordered centering/rotation and atomic errors.
  Runtime poses, activation and special selectors remain pending.
- [Dynent dispatch](reverse_engineering/dynent-collision-contract.md) distinguishes
  native class-zero/one sector traversal from decoder counts `[275,0,19,0]` and
  original count19. [Initialization](reverse_engineering/dynent-activation-initialization.md)
  reserves256 client-only slots; initial server indices0..18 are source-backed
  conditional on startup/providers. Current activation/poses remain unresolved.
  The zero-content suffix is not a permanent runtime exclusion.
- [Collision access](reverse_engineering/movement-collision-access.md) and
  [effective producer gap](reverse_engineering/movement-collision-receiver-producer.md):
  motor uses receiver shapes/candidate contacts beyond classic traces. Effective
  callback, receiver initialization and candidate world/pose bindings are missing.
- [Floating-point policy](reverse_engineering/movement-floating-point-policy.md)
  establishes a startup-thread 53-bit x87 precision request; movement-thread
  rounding/MXCSR binding remains unknown. [Camera](reverse_engineering/first-person-camera-contract.md)
  qualifies angles/basis and eye-effect ownership; runtime parameters and final
  view/projection publication remain unresolved.
- [Encoded prelude record](reverse_engineering/encoded-prelude-record-qualification.md)
  reproduces one static checksum/range/digest association. Returned scalar storage
  is now bound to [shared checksum-table initialization](reverse_engineering/encoded-prelude-storage-binding.md).
  A separate [direct polynomial initializer](reverse_engineering/receiver-polynomial-initialization.md)
  supplies edb88320 conditionally; the effective receiver callback stays unresolved.
  No FP/default/provider semantics follow from digest equality.
- [Coverage ledger](reverse_engineering/player-world-contract.md) tracks hierarchy,
  terrain, static XModels, transformed models, dynents and separate box scope.

Stable checkpoint requested by the user: reviewed work is committed locally;
workers have stopped at bounded handoffs and no subsequent task was launched.
Ready follow-ups are stationary terrain capsule qualification/implementation,
effective receiver/prelude effects and reusable XModel/dynent consumers. Keep one
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

Main also retains the separately integrated full-scene material work: bounded
dependency/shader-bank loaders, native Bevy material/pass-resource adapters,
semantic vertex routing and GPU diagnostic viewers. All6,469 surfaces,262 shared
images and1,080 texture bindings resolve in that scope; shader census covers1,404
references/1,065 pairs. Captured opaque response covers14,826 fully covered pixels
with maximum error1/255; it does not establish complete original scene fidelity.
Runtime globals, final post-processing, shadow fixture readiness and placement of
100 potential submodel surfaces remain unresolved. See
[materials integration](handoffs/main-materials-integration.md) for actual source,
capture and validation limits. Original capture authorization belongs to that
separate appearance task; this movement phase adds no original-game observer.
Previous Goal/monitor work remains paused.

Baseline gates: locked workspace metadata/tests, cargo fmt --all -- --check and
locked all-target Clippy with -D warnings. Validate headless/client separately.
Use existing Cargo installation if it is absent from PATH; do not reinstall it.
[Root README](../README.md) retains boot/viewer commands.

[Historical project state](project-state-history-2026-10-05.md) and
[historical index](history-index-2026-10-05.md) retain earlier milestone details.
Task handoffs/evidence carry detailed receipts; this brief does not duplicate them.
