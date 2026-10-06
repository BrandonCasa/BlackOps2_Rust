TASK:
GPU pipeline readiness census of every external original shader-bank pass.

STATUS:
COMPLETE
All 56 families and 1,404 pass references have GPU-ready pipelines.

FINDINGS:
- Shader pairs require exact typed uniforms, float/depth and 2D/cube images,
  ordinary/comparison samplers and reflected vertex component/location layouts.
- Fragment outputs independently determine required color-target locations/types;
  valid void/depth-only fragments have no color targets.
- The actual bank has 1,065 distinct vertex/pixel pairs. Every actual fragment
  interface needs one color target; no MRT interface or compilation failure found.
  All 51 vertex and 1,062 pixel shader identities are covered.
- Pipeline compilation is distinct from original scene values or draw fidelity.

EVIDENCE:
- Hash-admitted shader bank, Bevy PipelineCache/RenderDevice implementation,
  Naga fragment interface introspection and prior captured translated pair.

CHANGES:
- Ignored external pipeline_census test uses Bevy renderer initialization and
  pipeline cache with exact synthetic resource binding signatures.
- Duplicate pairs reuse readiness; all pass/family references receive receipts.
  Deadline/compilation errors and per-family progress remain external.
- Pinned existing Naga 29.0.4 dev dependency inspects safe external WGSL interfaces.

VALIDATION:
- Synthetic output/vertex layout and malformed-interface test PASS.
- Actual GPU synthetic valid pipeline and invalid-shader error classification PASS.
- Strict rendering all-targets Clippy PASS; changed Rust formatted.
- Initial GPU run attempted 603 passes before its 180-second budget; it aborted
  without the old version's final receipt, so those attempts are not claimed ready.
- Revised census has a 600-second deadline and incremental receipts.
- Full revised opt-in GPU census PASS: 1,404/1,404 READY, zero failures across all
  56 families, 1,065 distinct pairs, and zero MRT passes. Total 224.68 seconds;
  timed pipeline loop 223.55 seconds. Complete per-pass receipt/log stay external.
- Reproduce with BO2_SHADER_BANK set to the external bank manifest and
  BO2_PIPELINE_CENSUS_RECEIPT set to an external writable TSV, then run
  cargo test -p bo2-rendering --test pipeline_census
  external_original_pipeline_census -- --ignored --nocapture.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- This does not qualify source mesh routing, original states/values, dynamic
  bindings, frame passes, MSAA execution or Bevy camera/view-layout integration.

RECOMMENDED NEXT WORK:
- Qualify actual source mesh routing, dynamic code/global binding and original
  frame execution. Add representative capture comparisons beyond shader readiness.

SCOPE NOT IMPLEMENTED:
- Original-game inputs, replacement renderer, original visual equivalence,
  proprietary tracked artifacts, viewer/assets edits, agents or commits.
