TASK:
Connect the admitted captured world dispatch to the scene viewer through Bevy.

STATUS:
PARTIAL for complete original scene appearance; admitted dispatch executes.

FINDINGS:
- The new captured-world mode loads and retains all 1,113 admitted shader programs.
  It dispatches 694 native draws spanning 1,248 surfaces and 37 families, using
  shared typed image views, native uniforms, samplers, states and receipt order.
- Four initial color and depth samples are required. A Bevy Core3d system between
  the opaque and transparent passes copies each sample through a fragment shader
  with sample-index addressing. CPU depth uploads remain prohibited.
- The main target is RGBA8 UNORM. Explicit sRGB compositing with an sRGB final
  readback target preserves encoded bytes through Bevy's final output conversion;
  a linear final target would decode them a second time.
- Ordinary R32_FLOAT scene-depth inputs require feature-aware image validation.
  Comparison depth atlases remain separate GPU-produced Depth32Float views.

EVIDENCE:
- Captured-world-frame and captured-world-rendering handoffs, native shader bank,
  Bevy 0.19.1 Core3d schedules, stock material draw functions and sample attachments.
- External render receipt: native-world-render-01/render.txt. Pixels and original
  payloads remain external. Retained frame/capture identifiers are in the loader handoff.

CHANGES:
- Viewer captured_world and frame_initialization adapters; CLI accepts external
  scene, frame, shader-bank and secondary manifests plus a new external output.
- All shader handles are retained; shared images are content/view deduplicated.
- Native pipeline and texture errors stop rendering; no diagnostic fallback.
- Existing diagnostic and isolated comparison modes remain available.

VALIDATION:
- Synthetic GPU PASS: four individual color/depth samples, two occluded samples,
  native opaque draws spawned in reverse receipt order, max depth 0.984375.
  Later blue draw wins; all four pixels match [50,0,100,255] within 1/255.
- Actual admitted world GPU PASS: 694/4,031 draws, 1,248 surfaces, all shader
  handles retained, successful readback at 23 updates, 1,212,338 pixels changed
  from resolved initial color. No uncaptured GPU validation error occurred.
- Workspace metadata, tests, formatting and strict all-targets Clippy PASS.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Excluded interleaved geometry leaves visible black silhouettes. Initial target
  pixels include original rendered content, including the weapon; this is not a
  reconstructed weapon, entity system or complete original frame.
- 326 source-joined depth draws, 1,489 unjoined world-bank draws and 1,522 other
  shader-bank draws remain explicitly excluded. Runtime producers, postprocessing,
  out-of-view families, movable camera resources and sustained traversal remain open.

RECOMMENDED NEXT WORK:
- Establish runtime vertex contracts for the unjoined main-color world-bank draws,
  then integrate them and compare the same original interval.

SCOPE NOT IMPLEMENTED:
- Complete 6,469-surface shader dispatch, live producer systems, full-frame matching,
  gameplay, merge of this incomplete rendering requirement or remote push.
