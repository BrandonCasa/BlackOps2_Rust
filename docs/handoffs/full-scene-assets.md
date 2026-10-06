TASK:
Inventory/export all admitted scene appearance dependencies and add safe version-two input.

STATUS:
PARTIAL

FINDINGS:
- Every 6,469 surface, 280 material and 56 technique set is inventoried.
- All 260 backed images resolve; two engine builtin references remain unresolved.
- Captured main input packing admits all 449,721 corners without guessed vector decoding.
- One lightmap and 31 probe records retain exact source slots and float coefficients.

EVIDENCE:
- Current project brief, appearance001/002 handoffs, native public decoder and source pools.
- External full-scene-assets producers, inventory, 56 contracts, DDS/main/secondary streams.
- Original D3D11 capture packing and source-probe comparison; see full-scene-capture findings.
- Detailed permitted counts: ../reverse_engineering/full-scene-assets.md.

CHANGES:
- crates/assets/src/full_scene.rs and module export; synthetic/opt-in tests.
- Version-two tables and hash-bound inputs preserve version-one compatibility.
- Full original shaded admission explicitly unsupported; diagnostic contract is named.

VALIDATION:
- Native producers compile/link and actual read exits0 with0warnings/errors.
- All56 technique contract JSON files parse.
- Focused synthetic DDS/manifest tests and full external260DDS/6,469surface/449,721corner admission pass.
- All assets tests pass (57 default passing, three external tests ignored).
- Full-scene synthetic and external tests pass4/4; assets strict all-target Clippy passes.
- Owned Rust formatting check passes; full workspace remains outer responsibility.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Two builtin references, remaining secondary-layout categories, full rendering/global contracts.
- Original shaded scene and original comparison acceptance remain incomplete.

RECOMMENDED NEXT WORK:
- Integrate qualified rendering families with actual material state/lighting inputs.
- Observe remaining secondary categories and prove dynamic/global transfer conventions.

SCOPE NOT IMPLEMENTED:
- Renderer, viewer, game interaction, original fidelity claim, commits, main merge or push.
