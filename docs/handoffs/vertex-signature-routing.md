TASK:
Expose validated original vertex semantics and typed stream routing for the source mesh bridge.

STATUS:
COMPLETE

FINDINGS:
- All 51 unique VS signatures and 274 input records qualified from original DXBC ISGN and matched translated locations/components.
- All 1,404 pass routes cover required generic input semantics; ten observed source/destination pairs.
- Version2 hash-bound metadata required; version1 still loads and reports Unsupported for absent semantic lookup.
- Authorized preceding secondary continuation qualifies format3 stride8/half4; all 6,469 surfaces and 449,721 corners now load.

EVIDENCE:
- Original source hash/signature chunks, public pinned T6 stream enums, captured seven source layouts.
- External export_signatures.py/signature-accounting.private.json and main exporter hook.
- Secondary format3 actual lit frame27970/event4574/source386; complete export bounds census.
- See ../reverse_engineering/vertex-signature-routing.md and ../reverse_engineering/secondary-world-inputs.md.

CHANGES:
- assets vertex_signature module: typed semantic/system/component metadata and route helpers.
- Shader bank v2 sibling loader, full signature/route admission and signature lookup; v1 compatibility retained.
- Synthetic/actual shader-bank tests; small secondary format3 decoder/test completion and updated secondary handoff.
- No renderer/viewer changes or original content tracked.

VALIDATION:
- Shader-bank synthetic plus actual full bank 5 PASS; secondary synthetic plus actual all corners 4 PASS.
- Default assets suite PASS; assets strict all-target Clippy and owned formatting PASS.
- No workspace/GPU fidelity completion claim; outer agent owns those gates.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Renderer source-input expansion/routing, runtime code resources and possible brushmodel placement remain separate integration tasks.

RECOMMENDED NEXT WORK:
- Consume semantic metadata in the renderer and validate actual mesh input pipelines; retain missing-resource errors.
- Investigate brushmodel/entity placement separately when source surfaces are local-space.

SCOPE NOT IMPLEMENTED:
- Renderer/viewer edits, runtime generation, game input, new agents, commits or merging.
