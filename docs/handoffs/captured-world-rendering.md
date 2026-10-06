TASK:
Prepare and dispatch the admitted 694 captured main-color world draws in native order through Bevy.

STATUS:
COMPLETE for admitted dispatch implementation and scoped GPU ordering.

FINDINGS:
- Native opaque and blended draws must share one ordered phase. Stock opaque
  material sorting cannot retain interleaved native dispatch order.
- Bevy Transparent3d supports retained material draws; a late PhaseSort system
  orders replay items by receipt ordinal and selects a stock DrawMaterial tuple
  preceded by recorded viewport/scissor/blend-constant commands.
- Separate canonical mesh positions support Bevy bounds. Exact source attributes
  and physical native uniforms retain source object matrices and shader arithmetic.

EVIDENCE:
- Assets captured_frame admission, native replay-state census in
  captured-world-frame.md, Bevy 0.19.1 Transparent3d/DrawMaterial/TrackedRenderPass
  facilities, world-vertex-routing and translated-material adapters.

CHANGES:
- Rendering captured_world exposes prepare_captured_world, captured_mesh,
  native_raster, native_sampler and native_dispatch.
- Builder accepts shader-bank-ordered handles and texture handles keyed by
  payload SHA-256 plus native typed view format. Missing resources/states fail.
- Batched source slices remain one mesh/draw with ordered per-corner inputs.
- OriginalMaterialDescriptor optional replay state forces ordered transparent
  admission while preserving exact native depth/blend/write state and depth bias.
  Replay pipelines retain four samples, RGBA8 UNORM and Depth32Float.
- CapturedWorldPlugin adds native dynamic-state commands before stock DrawMaterial
  and preserves ordinal order after stock sorting. CapturedDraw requires
  NoAutomaticBatching and NoFrustumCulling; replay cameras need NoIndirectDrawing.
- Viewport max depth is applied by the GPU viewport, without rewriting source z.
  Exporter-proven x/y/width/height scissors are clipped to attachment bounds.
  Samplers retain native wrap/clamp,
  filter, anisotropy and comparison state; unsupported bias/address/filter fails.
- Unreflected native bindings are excluded before GPU resource lookup by stage
  visibility; required shader bindings never receive a fallback.
- Feature-aware image validation permits native linearly sampled R32_FLOAT only
  when the rendering device enabled FLOAT32_FILTERABLE, and uses typed view format.

VALIDATION:
- Three synthetic CPU tests PASS: native state/sampler mapping, viewport/scissor,
  unsupported features, ordered batch slices, source/canonical positions, draw
  ordinal/event sequence and missing resources.
- Actual external CPU assembly PASS: 694 draws, 152 texture/view identities, all
  native states and slice meshes admitted. This test deliberately uses strong
  synthetic image handles; exact typed image upload/readiness is a separate gate.
- Reproduce opt-in test with BO2_SHADER_BANK, BO2_SHADER_BANK_SHA256,
  BO2_FULL_SCENE, BO2_SECONDARY_CORNERS and BO2_CAPTURED_FRAME external inputs:
  cargo test -p bo2-rendering --test captured_world external_all_694
  -- --ignored --nocapture.
- Rendering crate tests PASS: 37 normal tests; three external opt-in tests remain
  ignored by default. The actual 694-draw CPU test was separately run and passed.
- Subsequent affected tests PASS: three captured-world CPU cases including offset
  scissor, unused binding and required missing image; five OriginalMaterial tests,
  including enabled/disabled float32 filtering (38 normal tests total).
- Rendering fmt check and strict all-targets Clippy PASS after these corrections.
- Parent synthetic GPU ordering PASS: reversed entity spawn order, two native
  opaque draws, retained four-sample color/depth, viewport maximum 0.984375,
  two occluded samples and later blue draw winning. All four output pixels agree
  with expected [50,0,100,255] within 1/255. This qualifies ordered execution and
  initial attachment interaction, not original scene equivalence.
- Parent owns actual 694-draw GPU integration and workspace gates.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Caller initializes native four-sample color/depth, prepares typed image views
  and depth atlas uploads, and loads external shader handles.
- Replay camera/attachments should contain only replay draws; excluded original
  draws and dynamic producers remain outside this bounded dispatch.
- Full original-frame equivalence is not established by CPU assembly or ordering.

RECOMMENDED NEXT WORK:
- Validate ordered execution with retained initial samples and matched captured
  intervals, explicitly accounting for excluded interleaved draws and frozen inputs.

SCOPE NOT IMPLEMENTED:
- Viewer/assets edits, original-game control, producers, excluded 1,815 world and
  1,522 non-world receipts, gameplay, agents or commits.
