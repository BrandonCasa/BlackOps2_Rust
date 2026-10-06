TASK:
Prove efficient original DXBC translation and Bevy-compatible shader ingestion.

STATUS:
COMPLETE

FINDINGS:
- Official vkd3d1.12 translates all1,113 unique original stage shaders without errors.
- Naga29.0.4 strictly parses/validates/emits WGSL for every shader.
- Stage-specific group3 descriptor remapping independently revalidates all1,113.
- Shader reflection supports typed image/sampler/uniform bindings and input locations.

EVIDENCE:
- Official release compiler source and installed Bevy0.19.1 shader/pipeline-cache source.
- External full-scene-shader-translation source identities, compiler logs, SPIR-V,
  generated WGSL/reflection and full-corpus validation receipts.
- Detailed procedure/limits: ../reverse_engineering/full-scene-shader-translation.md.

CHANGES:
- Findings and this handoff only; no production rendering/assets mutation.
- Safe external Naga checker/remapper plus external native compiler and batch scripts.

VALIDATION:
- 1,113/1,113 native conversions, strict Naga parse/validation/WGSL writes.
- 1,113/1,113 descriptor-remapped Naga validations/WGSL writes.
- No unsupported shader opcode family in recovered corpus.
- GPU pipeline/render comparisons remain separate next-phase work.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Complete runtime resource/pass/global bridge and original visual comparison.
- Captured viewport depth-range integration and secondary attribute completion.

RECOMMENDED NEXT WORK:
- Integrate a validated external shader bank and generic Bevy custom material interface.
- Render captured native shader pair with exact resources, then expand observed families.

SCOPE NOT IMPLEMENTED:
- GPU renderer, production scene/shader-bank schema, original fidelity claim, commits/merge.
