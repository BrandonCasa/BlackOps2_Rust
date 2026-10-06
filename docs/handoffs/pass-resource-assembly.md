TASK:
Assemble shader-bank pass arguments into explicit Bevy OriginalMaterial resources.

STATUS:
COMPLETE
Executable assembly and strict missing/conflict errors are implemented. Native
fixture integration remains an outer validation step; runtime values are caller-owned.

FINDINGS:
- Native destinations use byte offsets and sizes; material constants may copy
  only 4/8 bytes. Code constants select float4 rows; original inventory uses
  first-row zero and one/four rows (16/64 bytes).
- Texture and sampler registers are independent. VS/PS uniforms have distinct
  namespaces; textures/samplers shared by stages need consistent assignments.
- Native custom bits one/two select reflection t15/s15 and secondary lightmap
  t13/s13. All 1,404 pass reflections agree with the source enum and captures.
- Twenty-six code constants and six code textures have source enum identities,
  while most runtime values remain unqualified. Explicit values are required.

EVIDENCE:
- assets/shader_bank.rs and full-scene-shader-bank findings.
- Asset lane source enum audit and exhaustive pass reflection correspondence.
- Exact selected pass declares world matrix code213 at VS3 byte0 and view-projection
  code229 at VS0 byte576. Capture-qualified full buffers remain external.

CHANGES:
- rendering/pass_resources.rs: typed caller context, exact hash joins, row/byte
  writes, uniform coverage tracking, dead-input filtering, independent slots,
  qualified world lighting helper and explicit missing/conflict errors.
- Initial native buffer bytes may initialize untouched reflected fields; absent
  seeds require complete argument coverage. Allocated zeros never become admitted
  missing globals. Engine code values and builtins receive no invented defaults.
- Synthetic tests cover material hashes, partial writes, row/matrix offsets,
  stage separation, missing code/builtin/global values and shared slot conflicts.

VALIDATION:
- Five focused pass-resource tests passed.
- Strict rendering all-targets Clippy passed; changed Rust formatted.
- Outer owns selected captured fixture/GPU and workspace gates.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Production engine code/global generation, builtin images, shadow creation,
  camera adaptation and whole-map technique selection are not supplied here.

RECOMMENDED NEXT WORK:
- Run selected captured translated pair through this assembler, then qualify the
  production camera/global resolver and other actual code resources.

SCOPE NOT IMPLEMENTED:
- Renderer replacement, viewer/assets changes, original-game inputs, generated
  proprietary tracked content, nested agents, commits or pushes.
