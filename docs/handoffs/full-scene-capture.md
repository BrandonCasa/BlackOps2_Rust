TASK:
Qualify original full-scene rendering contracts through actual original capture.

STATUS:
PARTIAL

FINDINGS:
- Supported pre-start RenderDoc route captures/replays actual Origins D3D11.
- Exact primary packed-input formats and all seven serialized world formats,
  TECH4 dispatch, linear views/target, samplers and frame globals are observed.
- Source surface506 matches captured six-corner draw byte-for-byte.
- Runtime probe SH matches the source probe2 table.
- Shadowed TECH6 source2546 corners and shaders match the original draw.
- Frame6325 shader/source joins:367draws,189surfaces,56materials; no unmatched pairs.
- Frame119013:1020draws,744surfaces,146materials,40techsets; no unmatched pairs.
- Rare format7 secondary layout qualified from its only source surface3844.
- Rare format3 lit layout qualified from source386 in frame27970event4574.
- Water code textures18/9 and render-target-size uniform observed live.
- Three-frame union joins1189surfaces,180materials,43of56techsets.

EVIDENCE:
- ../reverse_engineering/full-scene-render-contracts.md
- ../fidelity/full-scene-original-observations.md
- External frame6325:1810draws,154shader/layout groups, exact fixture4462.
- External frame40264 adds the format5 secondary layout.
- External frames119013 and27970 add rareformats7/3 respectively.
- External shadow4633 includes six runtime textures, constants and four samples
  before/after; dynamic atlas is4096x8192R32_FLOAT with GREATER comparison.

CHANGES:
- Three concise evidence/handoff documents; no production code changes.
- Small external C++ replay extractor and local console-input helpers.
- All original outputs remain external; no commits or new global hooks.

VALIDATION:
- External native extractor compiles and links with existing SDK/libraries.
- Original capture open/replay PASS for frames6325 and40264.
- Source506 equality PASS:216bytes; original changed-pixel mask15154pixels.
- Source2546 equality PASS:216bytes; original TECH6 VS/PS equality PASS.
- God Mode ON observed in the live HUD; later teleport session-loss unresolved.
- User ON flags preserved in final live session; no protection toggles issued
  before successful format3 capture. Foreground/window input guard added.
- Capture139573 was incomplete and original process exited; replay rejects it.
  Relaunching the same supported route successfully produced frame27970.
- Cargo gates not applicable to this documentation-only lane.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Remaining-family dispatch/global contracts beyond40observed techsets.
- Dynamic shadow generation and final postprocessing.
- Source6369..6468 placement metadata absent from current asset export; local
  brushmesh/entity transforms require original runtime/source-model proof.
- Equivalent Bevy numerical/visual comparison; camera traversal session loss.

RECOMMENDED NEXT WORK:
- Compare source506 against its exact original intermediate-target mask.
- Join remaining captured shaders to native family contracts; continue bounded
  original observations for remaining uniform/resource producers.
- Capture layouts are complete; next original work should target the13remaining
  families, dynamic object placement and postprocessing dependencies.

SCOPE NOT IMPLEMENTED:
- Shader/material Rust implementation, gameplay, proprietary asset integration,
  full-scene fidelity claim, commits, merge or push.
