TASK:
Add windowed diagnostic inspection and a controllable free camera to the existing
geometry viewer, reusing its admitted raw snapshot Mesh and display policy.

STATUS:
COMPLETE
Outer review accepts the assigned code, synthetic/ECS tests, workspace gates,
and bounded actual window/legacy offscreen runtime checks.

FINDINGS:
- Raw geometry, surface records and identity mesh Transform are retained. Both
  modes share the existing neutral unlit solid and Bevy WireframePlugin setup.
- Tool-local perspective flight uses raw +Y up, bounds-radius/second speed,
  arrow-key yaw/pitch, capped dt and bounded finite position. These are diagnostic
  choices, without original movement, collision, units or orientation claims.
- W/S forward/back, A/D strafe, E/Q rise/fall, Shift 5x, Ctrl 0.1x, R reframe,
  Escape/window close exit. Reset fits the bounds sphere to the window aspect.
- Explicit interactive modes consume one synthetic or external snapshot and
  write no files. Legacy offscreen mode disables WinitPlugin explicitly.

EVIDENCE:
- Relevant viewer reader/prepare/render/tests, current state, architecture member
  boundaries, accepted handoff0020, and rendering-analysis skill were inspected.
- Cached pinned Bevy0.19.1 DefaultPlugins, InputPlugin/keyboard messages,
  WindowPlugin/close handling, projection, render/pipeline and Screenshot APIs;
  downloaded pinned WinitPlugin initialization and WinitSettings update modes.
- Worker accessed no original game, SDK, external snapshot or source asset.
  Outer read the previously accepted snapshot with unchanged size4,404,568 and
  SHA256 ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f.
  Window/input APIs compile against the pinned release without API repairs.

CHANGES:
- Viewer manifest locally enables bevy_winit, keyboard and x11 (minimal Linux
  backend availability), with legitimate Cargo.lock dependency additions; central
  release/toolchain/features remain unchanged. No existing locked package identity
  was removed or changed.
- src/camera.rs implements the tool-local Bevy component/plugin. src/interactive.rs
  assembles the window and finite probe. main.rs shares Mesh/display setup and
  admits interactive CLI modes; lib.rs exports the controller and typed failure.
- tests/camera.rs adds semantic and actual Bevy keyboard/ECS tests; CLI argument
  tests and README viewer usage are updated. Pending task0026 files are preserved.

VALIDATION:
- cargo metadata --locked --no-deps --format-version1 passes (seven members).
- cargo test -p bo2-geometry-viewer --locked passes13tests: five new camera tests,
  four main/CLI tests and four retained geometry tests. Actual KeyboardInput
  messages drive InputPlugin and the ECS camera through translation, orientation,
  reset, focus-loss release and Escape exit. Framing covers portrait/landscape;
  semantic checks cover opposing/diagonal keys, speed, pitch, bounded dt/position.
- cargo test --workspace --locked passes all80tests; cargo fmt --all -- --check,
  cargo clippy --workspace --all-targets --locked -- -D warnings and git diff
  --check all pass. No lint weakening, Rust unsafe or test skips.
- Initial cargo check failed resolving uncached bevy_winit: sandbox DNS failure.
  Worker escalation was interrupted while pending; outer's approved cargo fetch
  then succeeded. Focused compilation/tests and all workspace gates passed after
  that download. No worker GUI was launched.
- Outer launched bounded synthetic and accepted-snapshot interactive smoke checks:
  both exit0 at35 updates, move/look/reset=true, prepared GPU mesh/pipelines,
  960x720 window readback with41,270/33,473 foreground pixels respectively.
  No image files are emitted. Separate legacy synthetic offscreen run exits0
  at20 updates on Vulkan (vendor4318/device9988), retaining4positions/12indices/
  1surface and foreground25,433/20,401/29,937. Stderr is empty in all three runs.
  Each owned process has a120s supervisor and exits normally; logs and synthetic
  offscreen images stay in external temporary directories. No live original-game
  comparison or physical keyboard input was performed.
- Bounded runtime command: cargo run -p bo2-geometry-viewer -- --synthetic
  --interactive-smoke. Optionally use --snapshot with one admitted external input.
  --frames accepts 64..600 (smoke default180), plus a between-update60s deadline;
  supervise independently at120s because engine calls may block.
- Expected smoke signal: exit0 and `Interactive smoke: PASS` reporting changed
  camera translation/orientation, exact reset, prepared indexed GPU Mesh/pipelines
  and960x720 foreground window readback. It sends KeyboardInput through the same
  InputPlugin/controller; no physical-keyboard or original comparison is claimed.

FIDELITY STATUS:
UNKNOWN

UNRESOLVED:
- Original axes/units/movement/material/camera calibration; physical keyboard
  routing, resize behavior, sustained use and other platform/backend behavior.

RECOMMENDED NEXT WORK:
- Use the inspection tool while resolving the concrete collision-reader failure.

SCOPE NOT IMPLEMENTED:
- Gameplay, collision, original movement, materials/assets decoding, production
  boot changes, capture/process-control infrastructure, proprietary output,
  original comparisons, profiling, commits, pushing or nested delegation.
