TASK:
Adopt canonical Bevy runtime coordinates and adapt rendered map/textured scene
positions so the inspection world uses +Y up consistently with Bevy cameras.

STATUS:
COMPLETE
Code, executable and outer GPU smoke/relaunch validation complete.

FINDINGS:
- The previous viewer interpreted unchanged source XYZ positions with a Y-up
  camera, producing the user's vertical-world report. Runtime policy now uses
  Bevy +Y up and camera-local -Z forward; source Z-up data rotates at the boundary.
- Proper rotation `(x,y,z) -> (x,z,-y)` preserves winding, lengths and raw scale.
  This is explicit project policy requested by the user, not a native basis proof.
- Raw readers, source metadata and native-evidenced physics remain source-space.
  Future runtime consumers must use the shared adapter and inverse explicitly.

EVIDENCE:
- Inspected project-state, architecture README, handoff0027, rendering-analysis
  skill, coordinate evidence0009, viewer camera/prepare/scene and rendering tests.
- Evidence0009 establishes Bevy's basis and allows a different application policy,
  while leaving original physical units, angles and complete basis unproved.
- Synthetic basis, asymmetric bounds, triangle cross-product, corner-UV seam,
  canonical Mesh and camera-frustum cases. No original game/data read or input.

CHANGES:
- Core coordinates module: source/Bevy position-vector conversions and bounds.
- Rendering adds core dependency and canonical_mesh; raw diagnostic_mesh remains.
  Textured corner positions use the same conversion once; indices/UVs unchanged.
- Viewer computes offscreen and interactive framing from canonical positions;
  identity mesh Transform and existing Bevy Y-up camera/look/movement/reset remain.
- Synthetic tests cover exact inverse including signed zero, right-handed rotation,
  length/dot/cross preservation, reflected bounds, retained allocations/source
  metadata/indices, material UV seams and canonical orthographic/perspective framing.
- ADR0002, architecture README, usage/help/title and project-state updated.
  Legacy raw-* offscreen filenames now identify canonical-space images.

VALIDATION:
- Locked/offline metadata passes: eleven members.
- cargo test --workspace --locked --offline: 171 passed, four pre-existing
  external-input collision tests ignored; none failed.
- cargo fmt --all -- --check passes.
- cargo clippy --workspace --all-targets --locked --offline -- -D warnings passes.
- cargo build -p bo2-geometry-viewer --locked --offline passes.
- git diff --check passes. Outer synthetic and full-map interactive-smoke both
  exit0 at35 updates: GPU mesh/pipelines ready, move/look/reset true,960x720
  readbacks with37437 and32652 foreground pixels respectively.
- Full-map and recovered-texture windows relaunched and responsive with Bevy
  Y-up titles; no stderr. No original-game comparison or sustained-use proof.

FIDELITY STATUS:
UNKNOWN

UNRESOLVED:
- Original camera/angle/unit calibration and complete original visual fidelity.
- Runtime physics/gameplay adapters are future work; raw native contracts retained.

RECOMMENDED NEXT WORK:
- Compare equivalent reconstructed and original views when calibration is available.
- Reuse the shared boundary for future engine-facing positions/vectors/bounds.

SCOPE NOT IMPLEMENTED:
- Full materials/textures, gameplay, collision migration, original-game inputs,
  extraction, asset mutation, profiling, commits, pushes or nested delegation.
