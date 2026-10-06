TASK:
Recover retained collision inputs and qualify native world dispatch, filtering,
ownership and family entry points for a shared trace contract and coverage ledger.

STATUS:
COMPLETE

FINDINGS:
- Retained schema33/schema34 and fifteen native windows recovered and rehashed.
- Moving leaf dispatch separately processes terrain; 214 root-zero terrain leaves
  contain 253 AABB references and cannot be skipped through brush-root selection.
- All 265 model providers remain absent in retained metadata, not excluded from
  native collision. Native terminals resolve against global brush storage.
- Proposed contract and exhaustive family/model dispositions are in
  [player-world-contract](../reverse_engineering/player-world-contract.md).

EVIDENCE:
- Project state, architecture, handoffs0034-0037, physics/assets source/tests,
  retained schemas, complete native spans/disassembly, T6 declarations/loader rules.
- Licensed local files only. No playing, capturing, original invocation or process reads.
- Counts: 8,417 brushes, 67,196 triangles, 18,968 partitions, 51,129 AABB trees,
  10,837 static models, dynent counts [275, 0, 19, 0].

CHANGES:
- Added contract/coverage findings and this handoff only; shared API proposal
  awaits primary review. External task receipt contains permitted count/hash checks.
- Original payloads/disassembly/helpers/paths remain external; no Rust changes.

VALIDATION:
- Schema33/schema34 exact identities, fifteen qualified native span lengths/hashes,
  all 265 provider dispositions and 214 terrain-leaf/253 AABB references pass.
- Retained synthetic f32 versus rational reconstruction: 24 cases pass. New receipt
  written to owned external task directory; old receipts unchanged.
- Documentation relative links and diff whitespace checked by worker.
- Cargo gates skipped for documentation-only task; no native comparison claim.

FIDELITY STATUS:
INVESTIGATING for native contract; full-map/player behavior UNKNOWN.

UNRESOLVED:
- Stationary bodies, native traversal/order, family bodies/readers, model provider
  ownership/transforms, dynent current poses/activation and ordinary player inputs.
- Plutonium-associated retained image equivalence to unmodified Steam remains unproved.

RECOMMENDED NEXT WORK:
- Qualify stationary world/brush and native global-pool ownership as one bounded
  task, then independently qualify terrain and static XModel families.

SCOPE NOT IMPLEMENTED:
- Collision/movement Rust, ECS, scene manifest, family payload readers, original
  gameplay/capture, appearance/damage, shared exports, manifests, state or commits.
