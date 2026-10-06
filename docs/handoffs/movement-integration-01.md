TASK:
Review and integrate reusable Bevy map/input preparation and native-evidenced
stationary single-brush collision without claiming the full movement phase complete.

STATUS:
COMPLETE (bounded integration; full-map movement phase remains active)

FINDINGS:
- Host receipts persist across render frames and explicit fixed consumers. Accepted
  map arrays, metadata, gray display and existing viewer behavior are preserved.
- Stationary brush overlap differs from moving clipping; shared request construction
  preserves accepted moving behavior. Neither supplies complete world collision.
- Native baseline configuration and stationary prerequisites are source-qualified;
  command/motor and participating family investigations continue independently.

EVIDENCE:
- Reviewed host01, collision01/02/03 and native01 handoffs, source contracts,
  owned code/tests and shared dependency changes.
- Independently rehashed schemas33/34, six stationary spans and five player methods.
- Bevy fixed-time scheduling uses explicit synthetic pacing in host integration
  tests; it supplies no native command duration or movement-default evidence.

CHANGES:
- Primary wired boot library exports, optional client rendering/window features,
  rendering exports and viewer reuse. Central Bevy version/defaults retained.
- Primary added actual fixed-runner pacing integration across zero/multiple ticks
  and reversed synthetic entity creation order.
- Reviewed stationary request and private shape construction; no proprietary
  fixtures, original assets, unsafe-policy changes or remote pushes.

VALIDATION:
- Locked offline metadata: eleven members; workspace tests: 188 passed, five
  external-input tests ignored. Formatting and warning-denying all-target Clippy pass.
- Primary separately reran moving/stationary retained-input opt-ins: both pass.
- Boot client input/pacing: six tests pass; client all-target strict Clippy passes.
- Existing viewer synthetic window smoke: exit0 at35 updates, GPU meshes/pipelines
  ready, scripted move/look/reset true, 960x720 readback with37437 foreground pixels.
- Window test uses reconstructed synthetic geometry, never original-game capture.
  Physical input, sustained use and full movement window assembly remain unverified.

FIDELITY STATUS:
INVESTIGATING for brush source interpretation; gameplay/full-map UNKNOWN.

UNRESOLVED:
- Numerical motor/ECS, authentic camera, executable Origins baseline, replay/window
  dispatch, full scene manifest and all participating collision families.
- Canonical motor outputs across headless/windowed replay require those interfaces.

RECOMMENDED NEXT WORK:
- Continue command/update qualification, global scene ownership and terrain payload
  admission in the three lanes; implement each qualified numerical increment.

SCOPE NOT IMPLEMENTED:
- Full-map controller, world queries, unsupported family exclusions, original
  comparison/play/capture, appearance/damage expansion or remote publication.
