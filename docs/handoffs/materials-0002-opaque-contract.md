TASK:
Recover surface508's opaque shader response, attributes, states and reachable
appearance/lighting inputs for the next reusable custom-material increment.

STATUS:
PARTIAL
Shader equations and reachable dependencies are ready for implementation with
explicit synthetic float inputs. Original shaded-scene admission remains unsupported.

FINDINGS:
- Three material textures, secondary three-panel lightmap and reflection cube resolve.
- TECH4's actual normal mapping, specular/gloss, probe normalization, baked lighting,
  vertex-color transfer, fog/HDR/output equations are derived from selected DXBC.
- Original handedness requires B=-sign(h)*cross(N,T), not an assumed Bevy convention.
- Shader availability is distinct from original frame dispatch. Source runtime
  draw records are zero; TECH4 cannot yet be called the proven selected runtime pass.

EVIDENCE:
- Current brief, accepted first appearance handoff, relevant source/tests/skills.
- Pinned public T6 declarations/loader/dumper and actual selected native data/DXBC.
- Findings: ../reverse_engineering/materials-0002-opaque-contract.md.
- External native producer emits56DXBCs across28available techniques, selected
  material/argument/state metadata,216corner bytes,5DDSs and explicit adapter contract.
- Selected primary light0 is type0 with zero parameters; primary lightmap is null.
- Bounded original executable descriptor search found semantic records, no complete
  static input-layout table. No app/UI/runtime input was used.

CHANGES:
- One findings note and this concise handoff. No Rust/renderer/schema changes:
  exact runtime packed-input conversion is still unproved.
- Separate external native printer/build/data tooling; shared OAT remains untouched.

VALIDATION:
- Standalone native compile/link pass with existing public libraries and D3DCompiler.
- Actual selected read exits0 with0warnings/0errors; all5reachable DDSs resolve.
- Native pool/table/stream/image/shader bounds and finite guards; parsed JSON output.
- Six candidate decoded directions are finite and orthogonal after normalization;
  these checks do not qualify runtime input formats.
- Original zone hash remains663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5.
- No original shader/data/image is tracked. No Rust gates required for docs-only diff;
  outer reviews documentation and external observations before integration.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Original input formats for packed normal/tangent and byte32 lightmap UV.
- Original technique dispatch, sampler/texture-view/target transfer conventions.
- Runtime fog/HDR/camera-relative transforms and SH/global binding values.

RECOMMENDED NEXT WORK:
- Implement/test reusable TECH4 mathematical response through Bevy custom Material
  with explicit synthetic float attributes/globals and GPU cases. Keep original
  --scene admission unsupported until unresolved encodings/bindings are established.
- Observe one shader's input-layout descriptor construction to settle packing;
  don't make this a prerequisite for independent shader-math implementation.

SCOPE NOT IMPLEMENTED:
- Original shaded scene, complete materials/lighting, custom renderer integration,
  all-family interpretation, collision/gameplay/camera tools, UI, commits or pushes.
