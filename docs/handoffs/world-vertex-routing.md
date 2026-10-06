TASK:
Evidence-backed native world vertex-input routing for all admitted source formats.

STATUS:
COMPLETE

FINDINGS:
- Semantics and material stream routing determine source attributes; numeric
  shader locations alone do not. All seven actual formats are capture-qualified.
- Native HALF2-to-four-component input expansion is x,y,0,1; missing streams fail.

EVIDENCE:
- Assets vertex_signature/secondary_stream APIs, native ISGN and T6 stream enums,
  source format captures; docs/reverse_engineering/world-vertex-routing.md.

CHANGES:
- Rendering vertex_routes module exposes world_vertex_route and routed_world_mesh.
- SourceAttribute adds separate source XYZ and exact secondary UV/transform fields.
- Canonical Bevy positions/bounds and unmodified original shader inputs coexist.

VALIDATION:
- Three synthetic tests PASS: semantic/register joins, conflicts, missing streams,
  component validation, canonical/source positions and per-corner UV seams.
- Opt-in real input census PASS: 51 VS identities/1,404 pass definitions;
  6,469 surfaces/177,318 enabled material pass uses, zero unsupported routes;
  73 vertex-shader/format combinations built from admitted source corners.
- Reproduce using external BO2_SHADER_BANK, BO2_FULL_SCENE and
  BO2_SECONDARY_CORNERS paths, cargo test -p bo2-rendering --test vertex_routes
  external_all_native_shader_routes_have_qualified_source_formats -- --ignored --nocapture.
- Rendering crate tests PASS: 34 normal tests; two external opt-in tests ignored
  by default. The new external route census was separately executed and passed.
- Rendering fmt check and all-targets strict Clippy PASS. Parent owns workspace gates.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- CPU input admission does not prove all scene draw execution or original appearance.

RECOMMENDED NEXT WORK:
- Use routes and meshes when assembling actual material passes; qualify dynamic
  globals, camera transforms, render states and frame execution independently.

SCOPE NOT IMPLEMENTED:
- Viewer/assets/game edits, original inputs, new agents, commits or full visual claim.
