TASK:
Preserve useful original surface data through preparation and the committed Origins runtime pack.

STATUS:
COMPLETE - prepared surface data regenerated, validated, imported and exercised in the actual game window; local integration complete.

FINDINGS:
- All runtime dependencies belong under assets/origins; originals, OAT exports, raw DDS files and inspection receipts remain external. The game uses Bevy shaders and needs no original shader files or machine-local configuration.
- The retained world inventory carries material names and complete texture identities; preview image indexes differ and recovered builtin bindings are appended. Correspondence is checked by semantic/binding identity and image file or recovered definition.
- OAT model glTF omits tangents/specular textures and has unused material slots. A read-only callback now exports those source attributes, material bindings and static-instance lighting.
- Source coverage: 6,469 world surfaces, 149,907 world triangles, 14,480 placements, 494 providers; 31 probes with six faces each. All 2,922 prepared meshes carry source tangents and RGBA colors. The pack has 912 material records, 359 specular-gloss bindings, 505 world/probe PNGs and 730 model PNGs.

EVIDENCE:
- Retained world packed-corner/inventory/lighting exchanges and OAT T6 vector/vertex definitions; Git selected opaque shader response dbf869c establishes original bitangent/channel meanings. No retired implementation restored.
- External preparation/export files and surface-data-import.json under the configured inspection directory. surface-data-window contains the actual game log, screenshots and verification.json.

CHANGES:
- Standard glTF tangents/handedness, untouched RGBA colors and corner UV seams; original world/model material names and readable families. All decoded texture layers remain accessible through compact material.extras.surface channel records alongside normal/roughness conversions.
- Roughness is 1-gloss alpha without the old clamp; raw normal and specular/gloss channels survive independently. One unresolved editor-default texture has an explicitly marked neutral placeholder; identity normal uses its recovered definition.
- Document lighting tables carry both lightmap channels, probe cube faces/positions/SH and source coefficient basis. World meshes and nodes retain assignments, including hidden helpers. Slot 31 means no lightmap.
- Static model meshes remain shared across placements. Instance lighting IDs/probe assignments/per-LOD color accessors and expanded-corner-to-source-vertex mappings survive separately from geometry.
- Importer validates metadata/PNG closure, tangent handedness, vertex colors, cube dimensions, world material/lightmap/probe partitions and static sharing/transforms/lighting correspondence. Existing asset/rendering references and project state updated; no added Markdown files.

VALIDATION:
- External OAT callback compiled and ran successfully; affected external runtime pack regenerated successfully.
- Actual failures/fixes: preview builtin bindings and unused model material slots required identity-based correspondence; missing model identity-normal/editor-default references required recovered normal/marked placeholder handling; lightmap slot 31 required explicit unassigned normalization. A validation begun during assembly observed the previous slot-31 data; final import started after regeneration completed.
- Asset integrity: import/source reconciliation and exact copied hashes passed for 1,238 runtime files (1,235 PNGs), 596,228,344 bytes. Staged closure passed: 1,237 LFS pointers, ordinary Git game.json and local payload hashes; git lfs fsck passed.
- Compiled: locked metadata, release workspace build and formatting gates passed once. Rendered/played: actual release-window driver exited 0; movement=true, round_two=true, restart_round=1, survey_views=8. Spawn screenshot inspected; material appearance remains approximate.
- Bevy emitted unused glTF texture dependency warnings; render/collision scene loading and gameplay completed successfully. Documentation remains at 20 tracked Markdown files. No Rust/dependency/toolchain changes.
- Skipped: unit tests/test suites/fixtures and Clippy (banned), sustained/manual play, native shader equivalence, push.

UNRESOLVED:
PNG outputs retain decoded top-level channels rather than original compressed mip chains. Packed directions/secondary UVs and PBR normal/gloss response are practical reconstructions. Native lightmap decoding, layered shading, probe/SH evaluation and instance baked-color application remain rendering work. The editor-default model texture has one explicit placeholder. Unused glTF raw-texture dependency warnings remain in the game log; they did not prevent scene loading or play.

RECOMMENDED NEXT WORK:
Use the retained material channels and lighting assignments to refine materials inside the same Bevy game, starting with packed lightmap decoding and specular response.

SCOPE NOT IMPLEMENTED:
Native shader/register/constant-buffer/pipeline reproduction, source shader import, additional viewers/executables, retired-code restoration, higher model LOD geometry, exact original appearance, remote push.
