TASK:
Recover one inspectable Origins collision brush and identify task0026's embedded
ClipInfo admission failure through the existing separate pInfo loader lead.

STATUS:
COMPLETE

FINDINGS:
- One-brush access is complete; geometric compatibility and gameplay are unresolved.
- Exact old root failure is bounds.nonnull at 8,417 brushes; parallel contents are
  also null. Core arrays admit 12,105 planes, 17,108 sides, 68,642 vertices and 8,417 brushes.
- pInfo is absent in both observations, not an alternative populated provider.
- Schema28 preserves missing parallel arrays explicitly. Second attempt recovers
  first eligible brush 7 with 4 sides/16 vertices; no candidate contradiction skipped.
- Minus-distance/at-most-zero alone reconstructs a 16-vertex full-rank solid with
  exact endpoint extrema, but 8 tolerant stored-vertex violations and 0.0005449587606563
  bidirectional vertex disagreement mean zero fully compatible hypotheses.

EVIDENCE:
- [Findings0019](../reverse_engineering/0019-origins-collision-brush-recovery.md)
  describes pinned declarations/rules/generated loader, read/emit boundaries and limits.
- Only `zone/all/zm_tomb.ff`, 78,753,344 bytes,
  SHA256 `663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
- External schema28 record, 1,656 bytes,
  SHA256 `0b3cab3d285b299712879cff25924100a93f6495f7f179f7ec4b1be8d0e7c17f`.
- Attempt 1 exits 123/stage 3 in 1.563s; attempt 2 exits 0/stages 1-9 in 3.734s.
  Both meet 120s/2MiB/64KiB limits; four before/after identities agree.

CHANGES:
- Added findings0019 and this handoff; outer agent owns state/index integration.
- External tagged native adapter, exact fixtures and nullable schema28 analyzer;
  initial candidate preserved before the reviewed second revision.
- No Rust, Cargo, gameplay, architecture, config, old handoff or commit changes.

VALIDATION:
- 23 actual initial adapter cases pass; 6 optional-array adapter cases pass.
- 8 analyzer consistency cases and 3 missing-nullable cases reject; synthetic
  asymmetric wedge still discriminates the original four-hypothesis procedure.
- Native compile/link/fixture exit 0 with warnings-as-errors. Original record
  admission succeeds; full original geometric compatibility does not.
- Both owned child/threads cleaned; original identity preserved. Targeted retained
  task0026 source/tool preservation checked. Workspace gates skipped: no Rust change.

FIDELITY STATUS:
INVESTIGATING for data/access; overall/gameplay/visual UNKNOWN.

UNRESOLVED:
- Original stored-vertex/plane precision and contact epsilon; unavailable parallel
  bounds/contents cross-check and runtime provider/consumer semantics.
- Decoder allocation extents remain trusted; decoder independently name-opens input.
- Player dimensions, units/basis, masks, movement/timing, step/slide and live comparison.

RECOMMENDED NEXT WORK:
- Resolve float32 storage/solver precision on the retained brush before the bounded
  safe owned Rust boundary; no further decoder read is needed for that question.
  Do not silently adopt a relaxed diagnostic tolerance as a gameplay rule.

SCOPE NOT IMPLEMENTED:
- Safe Rust collision/gameplay, movement, full map collision, capture framework,
  original runtime/executable changes, materials/scripts, broad export, new engine,
  unsafe Rust/FFI, extra original reads, commit or push. Both allowed attempts consumed.
