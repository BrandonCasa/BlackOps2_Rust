TASK:
Extract reusable accepted gray-map display and persistent ordered host input,
ready for qualified movement command/camera integration.

STATUS:
COMPLETE (assigned preparation scope)

FINDINGS:
- Gray display uses the existing accepted snapshot reader/canonical mesh rotation,
  unchanged indices/source metadata, identity transform, neutral unlit cull-none
  material and optional existing Bevy wireframe overlay.
- Host receipt order is explicit and survives frames with zero fixed updates.
  Bevy keyboard/mouse/focus messages do not share native timestamps; collection
  applies documented focus/capture, keyboard, mouse ordering within each frame.
- Command timing, key bindings, sensitivity and camera parameters require explicit
  assembly configuration and qualified native interfaces.

EVIDENCE:
- Project state, architecture README, rendering-analysis skill, viewer-up-axis
  handoff, rendering adapters/tests and viewer prepare/spawn/interactive sources.
- Installed Bevy 0.19.1 keyboard, mouse, cursor/window and input scheduling APIs.
- Synthetic project-owned snapshot and injected Bevy input messages only.
  No original-game play/capture, live reads or external asset copies.

CHANGES:
- rendering/gray_map: reusable prepare and client display helpers; viewer prepare
  and gray spawn branch reuse them. Texture preview and inspection camera retained.
- boot/movement_input: persistent bounded ordered receipts, held state, focus and
  cursor resets, finite mouse validation, atomic admission failures, explicit
  configured client collector and persistent failure reporting.
- boot/movement_host: explicit consume system and persistent consumed queue;
  no automatic render-frame draining or guessed native fixed rate.
- Tests cover preservation, zero/multiple consumer schedules, short press/release,
  mouse consumption, focus/cursor release, malformed input and capacity atomicity.
- Shared exports/features/manifests are owned and wired by the primary agent.

VALIDATION:
- Locked offline gray-map preparation test passes.
- Locked offline boot client check passes.
- Locked offline headless input tests: 4 passed; client input tests: 5 passed.
- Locked offline client gray-map tests: 2 passed, including exact display policy.
- Warning-denying locked offline all-target Clippy passes for boot, rendering and
  viewer with boot/rendering client features. Two initial style warnings fixed.
- Existing viewer regression suite passes: 15 tests, including camera message
  focus/Escape handling, framing and mesh/source-metadata preservation.
- Owned Rust formatting and git diff whitespace checks pass.
- Primary independently passes real MinimalPlugins fixed-time pacing integration;
  these focused results do not substitute for final locked workspace gates.

FIDELITY STATUS:
UNKNOWN

UNRESOLVED:
- Native command translation and authentic first-person camera await qualified
  contracts; actual window input routing/exit and GPU checks await final assembly.
- Gray spawn helper requires initialized Bevy mesh/material asset resources;
  overlay additionally requires WireframePlugin and its GPU feature admission.
- Collector receipts describe host ordering, not native cross-channel timing.

RECOMMENDED NEXT WORK:
- Primary integrates explicit command timing/translation and authoritative camera
  once contracts land; run pacing equivalence and window lifecycle checks.

SCOPE NOT IMPLEMENTED:
- Numerical motor, collision readers, authentic camera defaults, boot dispatch,
  command replay schema, appearance/damage expansion, assets, commits or pushes.
