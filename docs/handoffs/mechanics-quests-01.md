TASK:
Establish one original-data air-staff build/inventory-to-weapon ownership transition.

STATUS:
COMPLETE

FINDINGS:
- Construction completion and staff ownership are separate: built/quest state exposes the staff; a later accepted pickup grants weapon and records owner.
- Three structural parts are shared inventory; crystal is a carried recipe item. Interrupted hold does not commit successful construction.
- Staff pickup uses a handled map callback, distinct variant/claim/base-owner state, and repeat/swap eligibility. Validation removes other staffs before later target claim checking; do not assume rejected swaps are side-effect free.

EVIDENCE:
- Read project state, architecture, Origins/weapon skills, retained dump receipts and named craftable/main-quest/equipment/weapon helpers.
- Fresh hashes for eight retained original compiled scripts; seven decompile/four disassemble invocations exit 0, outputs external only.
- Source/build/hash/procedure, symbol/opcode anchors and abstract transitions: [staff-build-ownership](../reverse_engineering/origins-mechanics/quests/staff-build-ownership.md).
- External identity/views retained in inspection_output/mechanics-quests-01. No runtime control, capture, injection or live measurements.

CHANGES:
- Added one findings record and this handoff only; no production, shared index/state/architecture or other worker edits.
- Proposed shared recipe/carried-item/constructed-object/weapon-family/owner separation; no API approved.

VALIDATION:
- Fresh compiled-script identity checks and bounded decoder exit receipts; manually inspected grant/handled-return/owner, eligibility, hold and construction opcodes.
- Checked relative links, tracked-content privacy and whitespace.
- Cargo gates not run for documentation-only change; synthetic cases and original comparison procedures proposed, not executed.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Runtime loaded patches, unitrigger/input scheduling, simultaneous claim/swap semantics and weapon-grant rollback remain unmeasured.
- Full world acquisition, upgrade puzzles/charge, disconnect/death transfer and ammo capacities remain outside scope.
- Decoded default hold uses raw gettime units; engine unit mapping/actual latency unvalidated.

RECOMMENDED NEXT WORK:
- Investigate one staff upgrade unlock-to-charge/retrieval transition as a separate task.
- Coordinate original build-only versus separate pickup comparison after shared runtime owner permits access.

SCOPE NOT IMPLEMENTED:
- Rust/Bevy gameplay, staff damage/firing, main quest completion, full upgrades/acquisition, visual/collision work, runtime launch/input, nested delegation, commits/push.
