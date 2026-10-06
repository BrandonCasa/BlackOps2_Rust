TASK:
Resolve the remaining shadow material image reference with qualified original dependency provenance.

STATUS:
COMPLETE

FINDINGS:
- Reference is a real static shared-zone image in code_post_gfx_zm, not a runtime sun-shadow atlas or generated builtin.
- Source material 264/seven surfaces 6319..6325 TECH2/3 PS t0/s0 require ordinary float2D image, semantic 2/sampler 20.
- Original decoded image256x256 BC1 UNORM,nine mips; appended image 261 preserves prior image 0..260.
- Scene/bank 262 images,all 1,080 material image bindings resolved,zero UnresolvedBuiltin entries.

EVIDENCE:
- Native bounded image pool scanner and pinned converter; shared source container/DDS hashes.
- Source material arguments, bank reflection and source surface inventory.
- Existing captured surface6324 depth-only dispatch distinguishes current view from available TECH2/3.
- See ../reverse_engineering/shadow-material-image.md; external shadow-material producer/logs/export.

CHANGES:
- assets builtin_images external definition/hash-bound loader; tests/shadow_material.rs.
- Compatible opt-in actual tests updated to accept expanded shared image table.
- External shared DDS/table/bank joins; original image data remains external.

VALIDATION:
- Synthetic plus actual shared-image tests 2 PASS; prior identity-normal tests 2 PASS with expanded table.
- Source sampler/semantic/name hash and both technique destinations checked; unknown/missing/changed dependency errors checked.
- Native shared-zone inspection zero warnings/errors; assets strict Clippy and formatting PASS.
- No original shader appearance completion or workspace/GPU gate claim.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Runtime code texture/constant generation, original pass dispatch/fidelity and format3 secondary input proof remain separate.

RECOMMENDED NEXT WORK:
- Reload expanded scene/bank for renderer integration; retain runtime code-resource errors.
- Complete format3 input proof/decoder in a separate bounded followup.

SCOPE NOT IMPLEMENTED:
- Renderer/viewer edits, runtime shadow atlas producer, game control, new agents, commits or merging.
