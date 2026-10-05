# Air staff: collected parts, built object, player ownership

Recorded 2026-10-05. Representative question: **does completing the air staff
build give its builder the staff, or does a separate pickup establish ownership?**
The inspected original-data path separates those transitions. Completion sets
crafted/quest state and exposes the stand model. A later accepted pickup grants
the selected staff weapon and assigns its owner. This is a decoded-script answer,
not a measured original-game comparison. Fidelity: **INVESTIGATING**.

## Inventory and boundary

| Dependency family | Evidence in this task | Separate next question |
| --- | --- | --- |
| Staff acquisition | Air craftable defines three shared structural pieces and a nonshared crystal; pickup and build accounting inspected | Exact robot/chamber/record acquisition conditions and world placements |
| Build to weapon | Shared crafting callback and Origins persistent pickup/owner callback traced below | Live final-build versus pickup ordering and simultaneous use |
| Staff upgrade | Base and upgraded metadata share a charger; pickup resolves charged variant | One upgrade unlock, insertion, charge and retrieval transition |
| Return/reclaim | Stand availability monitor and stale-owner guard inspected as ownership support | Exact disconnect/death/transfer ordering and ammo preservation |
| Main quest | Fourth first-time build sets all-staffs-crafted flag; elemental quest threads are named leads | Consumers and prerequisites of one later quest stage |
| Weapon behavior | Air weapon script present and initial registration inspected only as a lead | Firing, ammo, damage and melee are excluded |

No production inventory, staff or scripted-event abstraction exists in the current
[architecture](../../../architecture/README.md) and [project state](../../../project-state.md).
The proposed shared boundary below is a recommendation, not an approved API.

## Sources, identity and bounded procedure

Licensed Steam Zombies manifest build **24784239**; English installation identity
from [catalog](../../0001-local-asset-catalog.md). Current original-zone identities:

| Zone | Bytes | SHA-256 |
| --- | ---: | --- |
| `zone/all/patch_zm.ff` | 3,001,792 | `f5a27345a61de5fabd735f45f05b127e95ad9a488619bd7fbc96ac14c5ab4d47` |
| `zone/all/zm_tomb_patch.ff` | 1,288,384 | `770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca` |

The same zones were freshly hashed during the immediately preceding
[round task](../rounds/normal-round-accounting.md); this task freshly hashed the
retained compiled scripts below. Existing external `damage-life-state-01` dump
logs associate the scripts with their source zones. This task reuses those
extractions; it does not independently redo extraction or prove runtime load order.

| Compiled script identifier | Source zone | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `maps/mp/zm_tomb_craftables.gsc` | `zm_tomb_patch` | 35,917 | `17d503b35576c1dc9fd25b960d96f842607d3a171a0e92a1c21385050e9df552` |
| `maps/mp/zombies/_zm_craftables.gsc` | `zm_tomb_patch` | 48,246 | `1de03f0c94a0dacc229e8b4fc31b41c0465f349f5c3ba83dc20dc76f075433a1` |
| `maps/mp/zm_tomb_main_quest.gsc` | `zm_tomb_patch` | 29,539 | `1df7d5af7c24751e2bc5ea5516c5ea634032d4bdb989cf260fdbac132bc9488b` |
| `maps/mp/zm_tomb_utility.gsc` | `zm_tomb_patch` | 33,944 | `8b79970b981a51ac3df0ea6faebac298a9a636bb6ac8d90e17927cc949b18e50` |
| `maps/mp/zombies/_zm_weapons.gsc` | `patch_zm` | 46,245 | `e0af5ee97c9e16eae03166d5fd9f3854efaa9cee3d373f82a20dd663fce90870` |
| `maps/mp/zombies/_zm_equipment.gsc` | `patch_zm` | 29,006 | `5066dc230d6eac85146d54a068a0d021d4e84ac823c1a68ba7700237fc090942` |
| `maps/mp/zm_tomb.gsc` | `zm_tomb_patch` | 63,195 | `9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6` |

Air weapon registration lead, outside the representative transition:
`maps/mp/zombies/_zm_weap_staff_air.gsc`, `zm_tomb_patch`, 10,776 bytes,
SHA-256 `b240ce257b598db81d5be42fe616edc62ac2efce7f60fedf871902f723e046c7`.
No weapon damage behavior is concluded here.

1. Resolve external roots from ignored local configuration. Read the retained
   original compiled scripts and prior identity/dump receipts; verify the stated
   lengths/hashes. Keep originals untouched and all generated script views outside Git.
2. Use existing OAT `v0.33.0` extraction provenance, revision
   `7d027e8f89118196713e955b0e11f8404149c54d`, binary SHA-256
   `9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63`
   ([qualified tool identity](../../0004-origins-asset-directory.md)).
3. In external `inspection_output/mechanics-quests-01/decompile`, invoke
   `gsc-tool -m decomp -g t6 -s pc <one retained compiled script>` for the first six
   scripts above plus the air weapon lead. Use retained `zm_tomb` view for its
   limited-equipment list. In the task's `disasm` directory invoke
   `gsc-tool -m disasm -g t6 -s pc <one retained compiled script>` for map
   craftables, shared craftables, main quest and map utility.
4. Tool: **GSC Tool 1.4.10.333**, binary SHA-256
   `3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec`.
   Seven decompile and four disassemble invocations exited 0. Retain views and
   `identity-receipt.json` externally; tracked output is abstract findings and
   [handoff](../../../handoffs/mechanics-quests-01.md).

No decoder failure occurred for the selected scripts. Decompiler output is an
interpretation of compiled data; manual opcode cross-checks below use the same
decoder and do not independently validate opcode definitions. Generated views
can misrepresent types, locals, developer blocks and scheduling. No running game,
process injection, capture or input was used; active loaded patches and timing
remain unproved. Both Origins-analysis and weapon-analysis skill boundaries were
used; no production or damage implementation was authorized.

## Representative observations

**O:** decoded original-data observation. **I:** semantic inference. **P:** proposed
implementation. Function identifiers are lookup anchors, not copied source.

| Anchor | Abstract observation |
| --- | --- |
| `zm_tomb_craftables::init_craftables`, `include_craftables`, `staffcraftable_air` | **O:** air craftable registers an all-pieces requirement and full-build callback. It contains crystal, upper, middle and lower pieces. Three structural pieces use shared inventory; crystal is nonshared. Air pickup stand binds craftable `elemental_staff_air` to weapon `staff_air_zm`, persistent mode 1. **I:** recipe completion and a player's weapon inventory are distinct state owners. |
| `_zm_craftables::player_take_piece`; map `count_staff_piece_pickup`, `watch_staff_pickup`, `tomb_staff_update_prompt` | **O:** shared pickup sets shared inventory instead of the player's carried-piece slot. Nonshared pickup uses that slot and can drop its previous item. Three structural pickup watchers each decrement a remaining-part count once. Unbuilt staff prompt requires that count to reach zero and the player's carried piece to match the recipe. **I:** globally collected parts plus the appropriate carried crystal form the normal build precondition; spatial acquisition of those parts is not traced. |
| `_zm_craftables::player_can_craft`, `player_continue_crafting`, `craftable_use_hold_think_internal`, `player_craft` | **O:** validation checks recipe membership/piece crafting status and the map prompt callback. Hold continuation requires continued use input, usable interaction geometry/look state and valid crafting state; it rejects incapacitated/revive/screecher states. Success requires the elapsed threshold and continued eligibility. Failure clears crafting markers without the success path's crafted-piece commit. Success marks carried/shared recipe pieces crafted and consumes shared inventory; consumed carried piece is released. **I:** interrupted hold and completed recipe are different transitions. Exact unitrigger lifetime/input routing remains a dependency. |
| `_zm_craftables::player_finish_craftable`, `craftable_place_think`; map `staff_fullycrafted` and `staff_air_fullycrafted` | **O:** shared completion sets recipe/stub/global crafted state and emits completion events. Map callback exposes the air staff stand model, sets elemental quest client field to state 3, emits quest-progress notification and counts its first model activation once. Fourth first-time activation sets all-staffs-crafted. These functions do not grant the staff weapon or assign its owner. **I:** builder attribution for completion differs from ownership; map quest-progress callback uses a closest-player query, not necessarily the builder. |
| `_zm_craftables::craftable_place_think`, persistent mode 1 | **O:** after build, a new trigger event enters pickup checks: map validation, parent-player match, crafted/valid state, no screecher weapon, and no already-held matching equipment. The installed persistent callback can handle the interaction and skip the generic equipment grant. **I:** the same logical stand supports construction and later acquisition, with separate phases. |
| `zm_tomb_main_quest::player_has_staff`, `can_pickup_staff`, disassembly `loc_3551` through `loc_355A` | **O:** player primary inventory is inspected for a staff; if one exists, pickup requires the current equipped weapon to be a staff. Otherwise pickup is permitted. Rejection starts the swap hint. This answers one concrete repeat/swap eligibility rule without guessing a general weapon quota. |
| `zm_tomb_craftables::tomb_custom_craftable_validation` | **O:** after the crafted state exists and staff pickup eligibility passes, other staff-named primary weapons differing from the selected target weapon are removed before the later unclaimed-target check. **Limit:** a rejected claimed-target interaction cannot be described as side-effect free; that earlier removal order is real script structure. Runtime consequences/concurrent races are not measured. |
| Map `is_unclaimed_staff_weapon`; `_zm_equipment::is_limited_equipment`; `zm_tomb` limited-equipment initialization | **O:** Origins lists the base air staff among limited equipment. Claim check for a limited staff iterates current players and rejects an existing matching staff/upgrade according to the shared weapon-family predicate. **I:** admission examines actual inventory, not only a stored owner pointer. Variant/attachment predicates and game-mode loading still require comparison. |
| Map `tomb_check_crafted_weapon_persistence`, disassembly `loc_593D` through `loc_5AA5` | **O:** unclaimed staff resolves its current metadata, calls shared weapon grant, restores saved stock/clip when both exist, records pickup, hides the stand model, starts availability/return monitors and calls owner assignment. Claimed staff clears the hint without this grant path. The callback returns handled for staff in both cases, bypassing the generic grant. **I:** repeat rejection must not accidentally fall through and duplicate the weapon. Weapon-grant failure/rollback semantics are not resolved. |
| Map `get_staff_info_from_weapon_name`, `set_player_staff`, `clear_player_staff` | **O:** metadata lookup can select upgraded info when charger is charged and caller requests current variant. Owner assignment uses base lookup, stores player reference and element enum, writes per-player presentation state and updates accessories. Air element is 2. Owner-clear with a supplied player differing from current owner returns early. **I:** base staff identity, current weapon variant, claim eligibility and presentation fields are related but not one interchangeable boolean. |
| Map `track_staff_weapon_respawn`, `track_crafted_staff_trigger` | **O:** monitor clears enabled flag; while owner remains alive it treats inserted/quest-used staff as held or examines actual base/upgraded primary weapons and snapshots ammo. After the monitored state ends, it checks every player's inventory; if no base/upgraded staff remains, stand model is exposed and enabled flag is restored. It then invokes guarded owner clear. **Limit:** no complete death/disconnect/transfer system is established here. |

The raw hold default is **3000 elapsed `gettime()` units**, with 0.05 script-second
polling and an initial 0.01 script-second wait. Availability monitor requests 0.5
script-second polling. These are decoded thresholds/requested waits, not measured
latency. Engine `gettime()` unit mapping and scheduler precision were not validated
in this task. Counts, element enum and quest-state values are dimensionless.
Stock/clip fields represent saved ammunition counters; actual capacities are not
concluded from the initialization. The decompiled main-quest initialization uses
an earlier upgraded-loop local for base ammo queries, another reason not to invent
base capacities or silently correct the original from this view.

## Candidate shared model and discriminating cases

**P:** a reusable inventory boundary should distinguish shared recipe parts,
player-carried recipe item, constructed object availability, actual weapon-family
inventory and owner/presentation state. A held interaction emits completion only
after its continued eligibility succeeds; map code supplies recipe, variant and
quest-state policy. Bevy resources/components/systems can express these facts;
there is no evidence requiring a separate custom scripting engine or event bus.
Preserve the original operation order, including swap validation before claim
checking, until a measured discrepancy justifies a correction.

Proposed synthetic cases and later original comparisons:

| Controlled case | Expected under inspected path | Unverified comparison |
| --- | --- | --- |
| Parts collected, missing/wrong carried crystal | Build prompt/eligibility rejects | Original part/crystal combinations and actual stand geometry |
| Correct prerequisites, release use before threshold | No completed-recipe transition; crafting markers cleared | Repeated interruption timing and resource retention |
| Final build succeeds | Crafted/global quest state and visible staff; no ownership grant yet | Capture final hold release and subsequent distinct pickup input |
| Unclaimed built base staff, eligible player | Weapon grant followed by base owner/element state; generic grant skipped | Solo pickup and full ordinary inventory replacement |
| Player holds a staff but equips a gun | Swap pickup gate rejects and hints | Staff-in-hand versus gun-in-hand comparison |
| Another player owns target staff | No target grant; callback still handled | Multiplayer repeat/concurrent input, including prior swap removal |
| Repeat full-build callback | First-time build count protected by model activation state | Restart/rebuild conditions and duplicate event side effects |
| Stale old-owner cleanup after transfer | Guard prevents clearing new owner's staff record | Controlled ownership handoff after monitor wake |
| Staff absent from all inventories after release | Model/availability can return; saved ammo used on later grant | Disconnect/quest insertion boundaries, delay and ammo snapshots |

These cases are proposed, not executed tests. A coordinated original comparison
should record build/mode, players, uncharged air variant, recipe/stand availability,
current weapon, inventory before/after, ownership presentation, input edges and
monotonic timestamps. Repeat build-only and separate-pickup cases; use synthetic
fixtures for concurrency first. Do not control the shared original session without
its owner's coordination.

For a future coordinated timing session, the user's Escape-pause and console
`timescale 0.5` suggestions are unverified leads. Observe whether pause and scale
actually take effect, record their state, and restore/verify scale 1 before any
baseline timing claim. This task exercised neither control.

## Validation and remaining limits

Fresh retained-script hashes and eleven successful bounded decoder invocations
anchor this task beyond public advice. Manually cross-checked map grant/handled
return/owner and pickup eligibility plus shared crafted-state/hold/persistent
callback dispatch in disassembly. No copied scripts or raw assets enter tracked
files. Relative links/privacy checked in the handoff. Cargo gates were not run:
this task changes documentation only. No live comparison, concurrency experiment,
full upgrade/acquisition proof or production implementation exists. Scoped status
remains **INVESTIGATING**; overall gameplay fidelity remains **UNKNOWN**.
