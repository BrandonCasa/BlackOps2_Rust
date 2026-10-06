TASK:
Identify shared CodeConstant/CodeTexture resolver dependencies for recovered passes.

STATUS:
PARTIAL

FINDINGS:
- All 26 constant and 6 texture symbolic IDs identified across 1,404 passes; no unnamed code remains.
- Exact row spans, byte destinations and independent texture/sampler index packing established.
- Custom probe/lightmap flag mapping corroborated across every pass and original event4462.
- Runtime generators remain dependencies; direct source6087 DXBC comparison confirms existing physical CB remap correct.

EVIDENCE:
- Public pinned T6 declarations, external shader-bank and complete argument census.
- External inventory_resources.py/resource-identities.private.json reproduce identities and assignments.
- Capture worker original events4462,4633,8001 and second-frame time observation.
- Runtime table relocation plus initializer/upload proof qualifies identity-normal pixels; retail addresses are excluded.
- See ../reverse_engineering/code-render-resources.md for hashes, counts and typed mapping.

CHANGES:
- Added reverse-engineering evidence and this handoff; external read-only inventory/inspection scripts.
- No production Rust or viewer/rendering changes.

VALIDATION:
- Reproduced 11,545 constant arguments: 8,268 one-row/16-byte and 3,277 four-row/64-byte; all first_row0.
- Exhaustive custom flags/reflection census: 792 both,310 lightmap only,302 neither.
- No Rust checks required for this investigation-only increment; no gameplay/rendering completion claim.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- All symbol names known, but complete camera/light/time/debug/generic/shadow generator formulas and owner assignment unproved.
- Five of six CodeTexture runtime producers unresolved; captured sun atlas is a snapshot, not generation proof.
- Relocated runtime identity initializer now proves packed [128,128,255,128] and format28; common/upload helpers prove direct one-pixel RGBA8 data. Shadow builtin generation remains unresolved.
- RenderDoc CB declaration ordinal must not be used as physical register; source6087 DXBC/mapped bank check confirms current remap is correct.

RECOMMENDED NEXT WORK:
- Checked resource assembler with explicit supplied rows/images and physical CB register mapping; reject missing assignments.
- Runtime code-array writer/texture producer observation and moving-camera/time comparisons.

SCOPE NOT IMPLEMENTED:
- Runtime resource generation, renderer integration, secondary corner streams, shader changes, original-game control, commits and merging.
