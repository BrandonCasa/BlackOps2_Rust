# Air staff: unlock, insertion, initial charge and upgraded pickup

Recorded 2026-10-05. **INVESTIGATING**, decoded original-data evidence only.
Selected transition: given an unlocked built air staff, does insertion immediately
upgrade/grant it, or do charge completion and a separate pickup intervene?
**Result: insertion removes the base weapon and clears its owner; qualifying
deaths accumulate charger state; completion exposes the upgraded pickup; accepted
pickup separately grants the upgraded weapon.** No original runtime comparison.

## Evidence route and scope

Steam Zombies manifest build **24784239**, English PC T6, from the existing
[catalog](../../0001-local-asset-catalog.md). Retained OAT extraction is reused
from `damage-life-state-01`; its zone listing/dump receipts associate these scripts
with `zm_tomb_patch`. Container identity is 1,288,384 bytes, SHA-256
`770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca`.
That container was checked by preceding tasks; this task freshly hashes the five
compiled dependencies below, rather than claiming another extraction/container read.

| Script identifier, all under `maps/mp/` | Bytes | Fresh SHA-256 |
| --- | ---: | --- |
| `zm_tomb_main_quest.gsc` | 29,539 | `1df7d5af7c24751e2bc5ea5516c5ea634032d4bdb989cf260fdbac132bc9488b` |
| `zm_tomb_craftables.gsc` | 35,917 | `17d503b35576c1dc9fd25b960d96f842607d3a171a0e92a1c21385050e9df552` |
| `zm_tomb_quest_air.gsc` | 6,073 | `50a521e139b8613f518de63b66918f6c5331dc0bdc68bfe8ab9171d44ce4c03e` |
| `zm_tomb_quest_crypt.gsc` | 8,569 | `ef33fe4e6094153228532a5bdb0d102bafb99e1267ee02ae1cd32fe38b26df99` |
| `zm_tomb_utility.gsc` | 33,944 | `8b79970b981a51ac3df0ea6faebac298a9a636bb6ac8d90e17927cc949b18e50` |

Procedure: resolve external roots through ignored local configuration; verify
these retained compiled inputs. In unique external `mechanics-quests-02/decompile`
and `disasm` directories run respectively `gsc-tool -m decomp -g t6 -s pc <script>`
and `gsc-tool -m disasm -g t6 -s pc <script>` for each exact file. Ten invocations
exited0. No installation-wide traversal, new zone dump, runtime launch or input.
GSC Tool **1.4.10.333**, executable SHA-256
`3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec`;
OAT v0.33.0 provenance/tool identity is in [directory evidence](../../0004-origins-asset-directory.md).
Fresh input/exit receipts, output lengths/hashes and generated views stay external.
Main-quest decompile SHA-256:
`041c2fd2aa4343c687497334af2a5e5c0456400cb7f4eede5caa2da49d2401d1`;
disassembly:
`46a93f1f8717e072f048ab814a0a256a1d8ac3bdde9866fc8d5c64550073fcd8`.

Evidence labels below: **O** decoded instruction observation, **I** semantic
inference, **P** future implementation proposal. Manual disassembly/decompile
agreement uses the same decoder, not independent opcode proof. Effective loaded
patches, native helpers, scheduling and multiplayer behavior remain unvalidated.
Read [build/ownership01](staff-build-ownership.md) for the separate earlier build.

## Prerequisite and event dependency

**O:** air quest `main` initializes two completion flags and an availability flag;
after first-puzzle completion it starts the second puzzle. `air_puzzle_2_run`
waits for a smoke-solved notification, checks all recorded smoke solved states,
then sets the second completion flag. That function does **not** set staff upgrade
unlock. Initialization of `air_upgrade_available` alone proves no unlock behavior.

**O:** crypt `run_crypt_gem_pos` selects the base air weapon and
`staff_air_zm_upgrade_unlocked` for the air gem. Its utility
`puzzle_orb_chamber_to_crypt` parses exactly two prerequisite flag names from the
external gem structure's `script_flag`, verifies their existence, waits for the
first before orb departure and the second before crypt descent. Exact names in
the active map structure were not inspected: tying those slots to the two air
puzzle flags is a lead, **not a confirmed binding**. Orb movement/look gates are
dependencies, not newly implemented puzzle or geometry rules.

After arrival, crypt gem processing waits for damage whose weapon identity equals
the **base air staff**, then checks disc clearance. Failed clearance waits for
`crypt_disc_rotation` before checking again. Successful clearance starts the
ascent/return movement sequence; only after final chamber arrival/model replacement
does it set the upgrade-unlocked flag. This orders damage, clearance, movement
completion and unlock; solving an air smoke puzzle is not interchangeable with
that final state. Disassembly: `run_crypt_gem_pos` damage comparison at
`loc_10D9` through `loc_1108`, clearance loop `loc_1139`, final `flag_set` call;
utility `puzzle_orb_chamber_to_crypt` contains the two ordered `flag_wait` calls.

Main-quest initialization starts one insertion watcher and one charge checker per
base staff; both wait for the same weapon-specific unlock flag. It also registers
`zombie_spawn_func` on configured zombie spawners; that assigns an actor-killed
override. Registration is source-backed; actual loaded callback precedence is
not a live dispatch measurement. Shared flag-helper wake/latching internals and
restart behavior were not decoded here.

## Selected state transitions

| Stage / anchors | Observed contract and limit |
| --- | --- |
| Identity/init: `main_quest_init` | **O:** base `staff_air_zm` and `staff_air_upgraded_zm` share one charger object; air element enum2. Charger count starts0, inserted0 and charged0. Weapon variant, shared charger and player ownership are distinct. |
| Admission setup: `staff_upgrade_watch`, `place_staff_in_charger` | **O:** after unlock, mark charger-ready, replace an existing charge interaction, create radius-use interaction at charger origin Z-30 with radius argument120 and look required. Native trigger geometry/validity remains a helper dependency. Hint depends on exact weapon possession or inserted-and-charged state; hint text is not the full callback gate. |
| Insert: `waittill_staff_inserted` | **O:** a defined notifying player must possess this exact base weapon; it is removed before inserted state is set. Then clear player/base owner, disable interaction, move model with requested0.05 script seconds, wait for movement completion, mark charger presentation and full0. The function returns after one accepted insertion. A defined player without the weapon loops without this insertion. |
| Undefined notification counterexample | **O:** local weapon-available defaults true; undefined player skips possession/removal and still enters insertion. Opcode `loc_489A` through `loc_48E1` confirms the branch. Reachability via real input is unknown; do not call this callback an unconditional player-validity guard or silently fabricate a normal input producer. |
| Count: `zombie_killed_override` | **O:** portal/souls branch and successful footprint challenge consume the event first. Remaining branch requires player attacker or the named punch weapon. It selects the nearest inserted, not-full, uncharged charger by horizontal squared distance <=9,000,000; no vertical bound here. At equal distance the later array candidate replaces the earlier. One selected charger count increments before starting soul presentation. Helper side effects and competing callbacks remain gaps. |
| Soul notification: `zombie_soul_to_charger` | **O:** presentation client field set, requested1.5 script-second wait, then charger `soul_received`. Initial upgrade checker reads accumulated count, not this arrival notification. Death credit and notification arrival are separate events. |
| Charge: `staff_charger_check` | **O:** after unlock, poll count>=20; alternative cheat>=2 requires inserted state. Count branch does not independently recheck inserted. On admission, wait0.5 script seconds, set charged/full, set quest indication4, start upgraded pickup task, increment match staffs-charged count once, and break. Four completions set all-upgraded flag. Poll delay is1 script second, not measured latency. No normal count clamp20 or second state check after0.5 wait appears. |
| Expose: `spawn_upgraded_staff_triggers` | **O:** reuse base charge interaction for upgraded model, turn interaction on, move upgraded model to base location and await completion, then wait0.6 script seconds before calling pickup waiter. Early trigger notification retention/thread timing remains unresolved. Completion is neither automatic grant nor measured ready time. |
| Pickup: `watch_for_player_pickup_staff` | **O:** each trigger notifies `retrieved` **before** `can_pickup_staff`. Staff already carried with a gun equipped fails; loop listens again without target grant. Accepted use may remove current weapon under inventory-limit/staff conditions, grants/switches upgraded weapon, fills clip using definition helper, records upgraded model owner, emits staff-equip, clears inserted, sets full and updates base owner/element through `set_player_staff`. No grant-success return/rollback consumed here. |

Relevant main-quest opcode anchors: insertion `loc_48E1` through `loc_4986`;
death filter/selection `loc_49FE` through `loc_4B0A`, count-before-thread
`loc_4B22`; checker `loc_4BB5` through `loc_4C86` confirms20, cheat grouping,
0.5 wait and loop exit; pickup `loc_35A5` through `loc_36D4` confirms notify
before eligibility and later grant/state updates. Craftable helper
`get_staff_info_from_weapon_name` defaults to base-info lookup; owner set/clear
therefore uses base identity even for upgraded weapon input. Fresh helper reading
agrees with the preceding ownership record.

## Interruption, repeat and limits

No selected initial insertion/charge/pickup function specifies a cancel-to-return
or refund/rollback transition. Removal occurs before a movement wait; interruption
during that wait must not be given invented inventory restoration. These functions
have no local disconnect `endon`; wider teardown/helper effects remain unknown.
The initial checker exits after one completion, and successful insertion/pickup
exits its waiter; rejected pickup can emit repeated `retrieved` notifications.
Exactly-once original multiplayer behavior is not proved by those local exits.

Upgraded reload/reinsertion and owner-respawn monitors are adjacent follow-up
dependencies, not the initial20-charge contract. The retained death selector
excludes already-charged chargers, while reload code waits for `soul_received`;
effective post-upgrade soul routing requires its own consumer/patch investigation.
Do not extrapolate initial charge into a complete ammo recharge implementation.
Ammo capacities, stock grant semantics, damage/firing and all-staff quest consumers
remain outside scope. Coordinates are raw source units; 3,000 is the horizontal
radius corresponding to this squared-distance guard, not calibrated meters.

## Minimal boundary proposal and discriminating cases

**P:** keep map-scoped unlock, charger identity/count/inserted/charged/full and
completion-count state in Origins components/resources; keep actual weapon-family
inventory and player ownership in reviewed shared inventory. Base and upgraded
identities map to the same charger without a second entity registry. Persistent
state here means surviving skipped consumers within one simulation episode, not
proved storage across death, disconnect or restart.

Typed death credit, insertion request, pickup request, movement completion and
presentation notification should be separate. Use reviewed Bevy system ordering
and durable pending inputs/timers/state; Bevy messages can announce ordered
outcomes, but `retrieved` must not become an authoritative acquired receipt.
Capture original array tie order and explicit callback consumption inputs for
replay; query iteration must not decide which charger receives credit. Do not
create a universal quest VM/event bus or substitute arrival notifications for the
count update. See [review](../../../handoffs/mechanics-boundary-review.md).

Proposed synthetic cases and original comparisons, **not executed**:

- Locked versus unlocked: no charger insertion task before the confirmed unlock;
  two upstream air puzzle flags alone do not stand in for crypt completion.
- Defined player lacks base weapon: no insertion/removal. Defined holder inserts:
  base inventory/owner clear, inserted true, charge initially incomplete.
- Undefined insertion notification: decoded branch accepts; test synthetic only
  until a legitimate original producer is identified.
- Credit19 then20: charge incomplete before threshold; at20 completion awaits poll
  and delay, while no1.5-second arrival notification is required by initial checker.
- Inserted uncharged at boundary distance, different heights, and tied chargers:
  distinguish 2D <= guard and later-array tie selection from nearest3D rules.
- Footprint/portal consumed event: no selected initial staff credit. Nonplayer
  attacker/punch exception and native helper effects need caller-controlled cases.
- Charged pickup attempt while another staff is carried but a gun is equipped:
  `retrieved` can fire without grant; later eligible attempt grants one variant.
- Charge completion without pickup: quest/charger state changes but no player
  owner is inferred. Repeated successful pickup and duplicate completion need
  concurrent/restart context before claiming global idempotence.
- Disconnect/interruption during insertion movement or before upgraded pickup:
  observe actual inventory/model/owner behavior; no synthetic automatic rollback.

Original comparisons must coordinate with the runtime owner. Record effective
build/patch/mode, cheat setting, player/life states, actual inventory/current weapon,
air charger and competing charger states, event consumer outcomes, source positions,
credit/count, movement completion, input edges and monotonic time/frame resolution.
Compare unlock-only, insertion-only, 19/20 qualifying deaths, count-versus-soul
arrival, completion-without-pickup, rejected/accepted pickup and selected interruption.
User Escape pause and proposed console `timescale 0.5` remain unverified leads;
verify actual pause/scale, exclude pauses, restore and verify scale1 for baseline.
No duration, concurrency outcome or fidelity match is claimed before those runs.

## Validation and next dependencies

Fresh five-input hashes, ten successful interpretation invocations, external
output identities and manual named opcode checks support this record. No
production code or executed model checker; Cargo gates intentionally skipped for
documentation. Document links/privacy checked in [handoff02](../../../handoffs/mechanics-quests-02.md).
Next bounded work: bind air gem's actual prerequisite struct flags; resolve one
effective death-credit callback/helper route; then measure initial charge/pickup.
Full puzzle solutions, world part acquisition, recharge, staff death transfer and
main quest completion remain UNKNOWN outside these scoped source observations.
