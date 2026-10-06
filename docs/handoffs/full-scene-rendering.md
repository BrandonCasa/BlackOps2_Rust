TASK:
Implement Bevy material response and an external translated-shader bridge,
with exact captured TECH4 admission and explicit whole-map texture diagnostics.

STATUS:
PARTIAL
The reusable bridge and selected original shader pair render on the GPU.
Whole-map original pass dispatch and runtime globals remain outstanding.

FINDINGS:
- 56 technique families contain 51 distinct vertex and 1,062 pixel programs.
  One handwritten response cannot represent them. Assets successfully translated
  all 1,113 identities through strict Naga into external WGSL.
- Exact source506/event4462 qualifies source attributes, buffers, textures,
  samplers and raster state. Original depth is already reversed (GreaterEqual).
  Original MSAA sample-zero output requires a coverage-aware comparison.

EVIDENCE:
- Project state/architecture, rendering-analysis and material0002 evidence.
- Asset/capture lane reports: exact 216-byte source corner equality, sourceprobe2
  SH match, native uniforms/samplers, hash-admitted translated shader bank.
- Outer GPU receipts for synthetic, native handwritten and translated pair.
  Original/generated shader text and runtime buffers stay external.

CHANGES:
- tech4.rs/wgsl: Bevy Material, explicit response/fog/SH/textures, qualified native
  surface preparation and capture-only clip/viewport-depth adapter.
- original_material.rs: per-shader-pair Bevy group-three bindgroup layout,
  explicit uniforms/images/samplers, reflection descriptor merge, source input
  routing, raster/blend/depth state, device/image checks and uniform updates.
  Bevy retains rendering/scheduling/MSAA ownership; no alternate renderer.
- original_world_mesh retains source shader attributes separately from canonical
  Bevy POSITION used for visibility/bounds. No raw asset coordinate mutation.
- full_scene.rs: explicit first-color diagnostic, shared images, grouped canonical
  corners and missing/multiple-color surface accounting; v1 scene retained.
- Qualified sampler modes, synthetic regression tests and adapter evidence.

VALIDATION:
- Focused rendering test suites: 25 tests passed across actual focused runs,
  including malformed descriptors/resources and canonical/source attribute split.
- Strict rendering all-targets Clippy PASS; changed Rust files rustfmt formatted.
- Outer synthetic GPU PASS: 512x512, center161/117/84/255 within2 analytic bytes.
- Outer exact native source506 interiors matched within1 byte; full comparison
  remained limited by four-sample original versus sample-zero/resolve edges.
- Outer original translated pair GPU pipeline/render PASS at24 updates, using
  hash-selected external VS/PS and exact native buffers/typed resources. Initial
  full comparison mean0.02854/max12, honestly pending MSAA coverage correction.
- Workspace gates and final coverage comparison remain outer integration work.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Full scene pass selection, code resource binding, dynamic camera/global source
  buffer adaptation, shadows/effects/postprocessing and native MSAA comparison.
- All-family GPU acceptance; shader translation validation alone is insufficient.

RECOMMENDED NEXT WORK:
- Finish captured translated pair coverage-aware comparison.
- Qualify shared pass/code binding resolver and engine camera/global adapter,
  then admit actual whole-map families with zero silent resource fallback.

SCOPE NOT IMPLEMENTED:
- Whole-frame fidelity, gameplay, missing builtin substitutions, proprietary
  tracked output, alternate renderer, nested agents, commits or pushes.
