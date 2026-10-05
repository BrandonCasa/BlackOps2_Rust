# Normal-round accounting boundary

Recorded 2026-10-05. Question: **does an exhausted spawn budget end a normal
round, or must already spawned enemies also be exhausted?** The inspected original
bytecode requires both, subject to named holds and overrides. This answers the
shared script predicate and spawn-budget consumption question; active Origins
callback dispatch and live behavior are not established. Fidelity: **INVESTIGATING**.

## Inventory and scope

| Family | Current evidence | Next bounded question |
| --- | --- | --- |
| Normal lifecycle | Shared start/spawn/wait/end sequence and separate pending/enemy counts inspected below | Compare final pending spawn and final counted enemy against original round change |
| Population production | Shared budget callback, capacity gates, successful-spawn debit | Resolve complete player-count scaling and truncation semantics separately |
| Capacity | Origins sets 24 AI ordinarily; 20 in its classic split-screen optimization branch | Confirm relevant mode, actor/corpse cap, and actual active cap |
| Enemy membership | Shared utility filter and Origins capture-zombie exception inspected; dispatch discrepancy unresolved | Identify active callback consumer/load or patch replacement |
| Replacement accounting | Origins timeout handler can credit pending budget before removing a stuck enemy | Establish replacement enable flag and all ledger consumers |
| Round scaling | Health/speed/spawn-delay calculations are adjacent leads | Measure one selected scaling rule after mode/config binding |
| Special populations | Shared dog/ghost holds, map-specific capture enemies and mech scripts are leads | Separate special-round/population investigation; do not assume Origins uses generic dog rounds |
| Interruptions | Restart/end/intermission event cancellation is present | Define restart, host migration and game-ending ordering separately |

No prior production round abstraction exists; [architecture responsibilities](../../../architecture/README.md)
reserve reusable simulation below Origins assembly. [Project state](../../../project-state.md)
records that no round is implemented.

## Identity and original-data route

Steam Zombies manifest build **24784239**, English local installation identity
inherited from [catalog](../../0001-local-asset-catalog.md). Current input/script
hashes were checked on 2026-10-05. Build identity is a manifest label, not proof of
the exact script set loaded by a running session.

| Read-only original or retained extracted bytecode | Bytes | SHA-256 |
| --- | ---: | --- |
| `zone/all/patch_zm.ff` | 3,001,792 | `f5a27345a61de5fabd735f45f05b127e95ad9a488619bd7fbc96ac14c5ab4d47` |
| `zone/all/zm_tomb_patch.ff` | 1,288,384 | `770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca` |
| `maps/mp/zombies/_zm.gsc`, retained `patch_zm` extraction | 105,301 | `ae9fa4952fbd43353a91eb75236d403ac0454163460670dacb88d00456391ddf` |
| `maps/mp/zombies/_zm_utility.gsc`, retained `patch_zm` extraction | 74,393 | `485e791715a5660414238878ee1fa77facfe1b2612ff367d0b830b77a516e1b1` |
| `maps/mp/zm_tomb.gsc`, retained `zm_tomb_patch` extraction | 63,195 | `9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6` |

The extraction provenance is the existing external `damage-life-state-01`
identity receipt and its OAT dump logs. They associate these retained scripts
with the two zones; this task did not repeat extraction or independently decode
fastfile script payloads. OAT `v0.33.0`, revision
`7d027e8f89118196713e955b0e11f8404149c54d`, binary SHA-256
`9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63`,
is the previously qualified decoder ([tool identity](../../0004-origins-asset-directory.md)).

Read/emit reproduction:

1. Resolve roots from ignored `config/local_assets.toml`; use the existing
   external extraction and receipt. Hash the two zones and three scripts above,
   preserving originals. Verify the byte lengths and hashes before interpretation.
2. Use retained **GSC Tool 1.4.10.333**, binary SHA-256
   `3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec`.
   In task-specific external working directories run
   `gsc-tool -m decomp -g t6 -s pc <one retained compiled script>` and
   `gsc-tool -m disasm -g t6 -s pc <one retained compiled script>`.
   No server/client runtime, injection, or game input is involved.
3. Freshly decompile `_zm`, `_zm_utility`, and `zm_tomb`; freshly disassemble the
   utility and map script. Reuse existing `_zm` disassembly for instruction
   cross-checks. Read only the named function families below; do not bulk copy
   scripts or emit their contents into tracked documents.
4. Retain generated proprietary views and `identity-receipt.json` only in external
   `inspection_output/mechanics-rounds-01`. Tracked output is this abstract
   interpretation and the [handoff](../../../handoffs/mechanics-rounds-01.md).

All five new decompile/disassemble invocations exited 0. Fresh `_zm` and
`zm_tomb` decompiled hashes equal the existing receipt, respectively
`eecd7660136b90c904ec25ae6dd4d01a1b96feb0e5aa1a497fe003d6e2e4f0ce`
and `1f442456c5705e24f66c7534af4fe9fd49e45d06007ae89ab83e0d636a8a3afc`.
Fresh utility decompiled hash:
`46b944a9798510df7bf4d7232ae7936e5670d2a155c482d7cd2c234425af878c`.
No decompiler failure was observed for these files. Decompilation can misrepresent
control flow, names, numeric types or developer blocks; disassembly shares the
same decoder and is not independent validation of opcode decoding. Nothing here
proves the game loaded this exact zone ordering or applied no later patch.

## Observed transitions and counters

Evidence level **O** means decoded original-data instruction observation; **I**
means inferred semantic meaning; **P** means a proposed implementation choice.
No live timing or gameplay measurements were made.

| Original-data anchor | Observation and interpretation |
| --- | --- |
| `_zm::round_start`, `_zm::round_think` | **O:** absent custom callbacks, round start selects shared spawning/wait/thinking functions. The loop waits for at least one spawn location, starts the spawning worker, emits start-of-round, invokes the wait function synchronously, then emits end-of-round. After end hooks and variable updates it increments round number, caps it at 255, invokes between-round behavior and loops. **I:** lifecycle owner is the shared round orchestrator; map callbacks customize stages. |
| `_zm::round_spawning`, disassembly `loc_E015` gate and `loc_E393` through `loc_E3AC` debit | **O:** pending `level.zombie_total` is initialized through the maximum-budget callback except the active kill-counter-HUD preservation branch. Spawn loop waits while counted enemies reach AI capacity **or pending budget is nonpositive**; actor capacity is a second gate. A defined result from ordinary `spawn_zombie` decrements pending budget exactly once in the inspected branch; an undefined result bypasses that debit. **I:** this counter is remaining spawn budget, not active enemy count or whole-round kills remaining. |
| `_zm_utility::get_current_zombie_count`, `get_round_enemy_array` | **O:** count is the length of a freshly queried enemy array; it is not a read of the separately initialized `current_zombie_count` field. Query selects all AI species on the zombie team and omits entities whose ignore-count field is defined and true. **I:** entity membership/removal owns the counted population; do not blindly maintain a second integer on death events. |
| `_zm::round_wait`, disassembly `loc_F5A3`, `loc_F5DC` through `loc_F644` | **O:** initial wait is 1 script second. In the non-dog branch, ghost callback activity holds the round; otherwise any positive counted population, positive pending budget or intermission holds it. With no hold, return allows round end. Explicit end-round-wait flag can return despite a hold. Ordinary loop sleeps 1.0 script second between checks. Restart event cancels this wait. **I:** empty pending budget alone cannot complete an ordinary round. Script waits are requested durations, not measured wall-clock precision. |
| `_zm::round_spawning` event termination | **O:** end-of-round, restart and intermission events cancel the spawn worker. **I:** round orchestration must retire work for the previous round before allowing new-round budget ownership. Thread scheduling within a network frame is not resolved here. |
| `zm_tomb` initialization and `zm_tomb_get_round_enemy_array` | **O:** Origins installs a custom enemy-array callback and defines a filter that retains capture-tagged enemies even when their ignore-count field is true. Ordinary AI cap is 24; classic split-screen optimization branch assigns 20. **Important limit:** the inspected `patch_zm` utility never reads the custom callback. Therefore the map filter's actual use is **unresolved**, not an observed active Origins count rule. |
| `zm_tomb::tomb_round_spawn_failsafe` | **O:** selected timeout branches increment both pending budget and a subtraction ledger before damaging the removed enemy, conditioned on replacement flags and exclusions. **I:** despawn/replacement can preserve remaining round work rather than count as ordinary completion. The replacement flag's active value and ledger consumers are outside this task. |

Counts are dimensionless entity/spawn units. AI and actor limits are concurrent
capacity, not a full-round budget. The normal predicate is structurally:
retain the round while pending work or counted enemies or a named hold exists;
completion is a separate transition after that predicate clears or an explicit
override applies. This paraphrase is not copied original source.

## Minimal proposed shared boundary

**P:** reusable Zombies simulation should own round identity/phase, pending spawn
budget, capacity configuration, and explicit hold/override inputs. Bevy entities
own enemy membership; a query or one reviewed membership abstraction derives the
count. A successful spawn commits a budget debit together with entity admission;
a failed spawn leaves pending work. A replacement operation records its credit
and removal distinctly. Origins supplies policy/configuration for enemy membership
and callbacks above the shared layer. These are candidate responsibilities, not
approved Rust APIs or permission to implement undocumented full behavior.

Do not freeze an Origins capture-exception policy until active callback dispatch
is proved. Do not select fixed-step duration from these script polling intervals,
assume float/integer arithmetic for population scaling, or use generic dog logic
as proof of an Origins special-round mechanism.

## Synthetic cases and original comparison plan

| Case | Expected under inspected normal predicate | Missing original comparison |
| --- | --- | --- |
| Pending 0, counted 1, no hold | Continue round | Final active enemy survives after all spawns are consumed |
| Pending 1, counted 0, no hold | Continue round | Spawn gap/failed spawn cannot finish the round |
| Pending 0, counted 0, no hold | Completion on a wait check | Repeated final-kill to end-signal timing under controlled solo round |
| Both zero, intermission or ghost hold active | Continue round | Define which holds Origins actually activates |
| Positive work, explicit end-wait override | Wait returns | Identify legitimate override owner and its conditions |
| Undefined ordinary spawn result | No debit | Reproduce failed admission using synthetic spawn outcome |
| Successful spawn at available capacity | One debit; admitted entity blocks completion | Controlled admitted entity and membership timing |
| Ignored ordinary enemy vs capture-tagged ignored enemy | Shared filter omits former; map candidate retains latter | Resolve callback dispatch before claiming Origins result |
| Replacement credit followed by removal | Pending work remains positive | Confirm active timeout replacement configuration separately |

Synthetic cases are proposed, **not executed game tests**. For live comparison,
record original build/mode, player count, round number, available spawn locations,
hold/override state, final admission/removal and round HUD/audio transitions with
monotonic timestamps; repeat ordinary final-enemy cases. Runtime access must first
be coordinated with its owner. No elapsed-time claim is established by script
constants alone.

## Validation and limits

Fresh source hashes, successful bounded decoding, matching regenerated views, and
manual instruction cross-checks establish a representative original-data route.
The callback discrepancy prevents claiming complete Origins binding. No production
code changed; Cargo gates were not run for this documentation-only task. Checked
relative document links and path/content privacy are reported in the handoff.
Overall gameplay fidelity remains **UNKNOWN**; this boundary is **INVESTIGATING**.
