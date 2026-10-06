TASK:
Implement a validated external original shader bank and full recovered-family export.

STATUS:
COMPLETE

FINDINGS:
- All1,113 shaders/56families/1,404passes/280material contracts admit through new loader.
- Typed uniform/float2D/cube/depth/comparison resources retain exact source interfaces.
- Native sampler argument low/high bytes are separate texture/sampler registers.
- All material argument hashes join; two builtin image references remain explicit.

EVIDENCE:
- Existing appearance/shader translation boundaries and actual external reflection corpus.
- Native public argument-location union and all material/pass inventories.
- Findings: ../reverse_engineering/full-scene-shader-bank.md.

CHANGES:
- crates/assets/src/shader_bank.rs, module export and synthetic/opt-in tests.
- Distinct external bank manifest/export preserves existing v1/v2 appearance inputs.
- External export includes native constants/states/texture assignments and replaces atomically.

VALIDATION:
- Focused synthetic plus canonical external bank tests pass4/4.
- All assets default tests pass60; four external tests ignored by default.
- Strict all-target assets Clippy and owned Rust formatting pass.
- Actual shader bytecodes were separately translated/Naga-validated, not embedded.
- Workspace/GPU integration remains outer/renderer responsibility.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Engine26constant/6texture code runtime resolver, custom lighting bindings and2builtins.
- Original pass dispatch/resource population and full original scene comparisons.

RECOMMENDED NEXT WORK:
- Integrate selected captured native pair using exact reflection/source buffers.
- Investigate shared engine code-resource bindings and builtin generation before full scene.

SCOPE NOT IMPLEMENTED:
- Renderer/viewer, live game control, generated shader content inGit, commits/merge.
