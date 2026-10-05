# Excavation: shovel admission, site consumption and round replenishment

Recorded 2026-10-05. **INVESTIGATING**, source evidence only. Representative
question: after one player acquires a shovel, what admits a normal mound dig,
what is consumed, and what can make that mound available again? No game launch,
input, process injection, live measurement or production implementation occurred.

## Source and procedure

Steam Zombies manifest build `24784239`, English, PC T6. Reused the retained
external OAT v0.33.0 extraction from `damage-life-state-01`; no zone extraction
was repeated. Its listings contain the selected script identifiers. The prior
zone identities are:

| Zone identifier | Bytes | SHA-256 |
| --- | ---: | --- |
| `zone/all/zm_tomb_patch.ff` | 1,288,384 | `770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca` |
| `zone/all/patch_zm.ff` | 3,001,792 | `f5a27345a61de5fabd735f45f05b127e95ad9a488619bd7fbc96ac14c5ab4d47` |

The first zone was rehashed during the [preceding generator task](generator-activation-01.md).
This task freshly hashed these retained compiled script inputs:

| Script identifier | Source zone | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `maps/mp/zm_tomb_dig.gsc` | Origins patch | 21,549 | `f8e2dcba88df7be7b518a5b50a14d4208ba4c7f57777af60cde9501573261f9d` |
| `maps/mp/zm_tomb_utility.gsc` | Origins patch | 33,944 | `8b79970b981a51ac3df0ea6faebac298a9a636bb6ac8d90e17927cc949b18e50` |
| `maps/mp/zombies/_zm_unitrigger.gsc` | Zombies patch | 11,003 | `753b0c4c0f30b4bdd5c54504f824587ed240cfa4e074e3fbc0ba2a6f811c20d3` |

Use ignored local configuration for external roots and the retained exact script
files. In the unique external working directory `mechanics-world-02`, run
`gsc-tool -m decomp -g t6 -s pc <script>` for these three files, and
`gsc-tool -m disasm -g t6 -s pc <script>` for the dig/map-utility scripts.
All five commands exited 0 using retained GSC Tool `1.4.10.333`, executable
SHA-256 `3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec`.
OAT tool identity/provenance is recorded in the preceding task and
[asset-directory evidence](../../0004-origins-asset-directory.md).

Inspect the symbols below; cross-check dig admission/consumption and replenishment
comparison order in dig disassembly, and trigger forwarding/deletion in utility
disassembly. These interpretations share one tool and are not independent decoder
proof. Raw outputs remain external. Fresh output identities:

| Output | Bytes | SHA-256 |
| --- | ---: | --- |
| Dig decompile | 33,510 | `892a5251e5f59754e1a8b864b6ebca96e9f873debcdf2684187d25bca03dd099` |
| Dig disassembly | 91,318 | `07261bb8895a951d66844fc8530f0be59ecc4e4566fdcb92d3a225ff7eb65baf` |
| Map-utility decompile | 50,048 | `805bfb4d558dfa0fc2646be4b3135a2b4e34b15192fe00b671a05e7cc2efdcc2` |
| Map-utility disassembly | 134,375 | `9fb2e74b7617dab261612d0ac464ae97ffe4d4ff63c12d1eaaf09dbccafa9152` |
| Shared-unitrigger decompile | 22,502 | `28949e546468c20090567d9efb805844b6033d2e8e7d63cd2acb6f239dee074c` |

## Observations: personal shovel, shared site

Anchors: dig `init_shovel`, `init_shovel_player`, `shovel_unitrigger_think`,
`shovel_prompt_update`, `dig_disconnect_watch`.

Map shovel locations group by their opaque noteworthy identifier. One location
is selected per group by an indexed random call, then a model and look-required
box-use interaction are created. This does not establish the actual number or
coordinates of map shovels: map entity structures were not inspected here.

Connection initialization gives each player independent normal/upgraded shovel,
helmet, dig-count and losing-streak fields. Pickup checks the triggering player
matches the per-player trigger owner and that this player has no shovel. It sets
that player's shovel flag, sets the indexed shovel client field to 1, starts a
disconnect watcher using this pickup's origin/angles, deletes the world model and
unregisters its interaction. An already-equipped player instead receives a
different hint and does not take it. The callback has no explicit score charge.

On disconnect the watcher clears that player's shovel/helmet client fields and
respawns a normal shovel model and interaction at the recorded pickup origin,
not the disconnect position. Ordinary digs do not clear shovel ownership or
decrement a shovel durability counter in this script. Death/downed-state
persistence and reconnection identity require the shared life-state/runtime
boundary; no claim is made about those transitions. The upgraded state is a
per-player field/client-field value 2, rather than a separate weapon slot.

## Observations: admission and consumption

Anchors: dig `dig_spots_init`, `dig_spot_spawn`, `dig_spot_trigger_visibility`,
`waittill_dug`; map utility `tomb_spawn_trigger_radius`, `tomb_unitrigger_think`,
`tomb_unitrigger_delete`; shared unitrigger `register_unitrigger_internal`,
`assess_and_apply_visibility`, `main`, `build_trigger_from_unitrigger_stub`.
Native geometry and eligibility are not fully established.

After round-logic startup, the script sets current site count zero and configured
maximum 15, obtains map dig structures, spawns initial-tagged spots, and marks
other spots dug/unavailable. Every spawn increments current count **before**
creating a mound 40 raw source Z units below its origin and requesting a nominal
three-second move to its origin. Only after the move-done event is a radius-use
interaction created at origin Z plus 20, with radius argument 100 and look-at
required. These are source-space quantities and animation requests, not calibrated
meters or measured wall-clock timing. Count includes emerging, not yet usable mounds.

The map helper wraps the shared unitrigger with a trigger-forwarding callback.
The shared implementation creates a native radius-use trigger, defaults height
to 64, and uses a broader script candidate test based on (radius + 15) squared.
Native use-radius/contact/look tests remain unmeasured. It is incorrect to
substitute a simple distance <=100 admission test solely from the radius argument.
The shared monitor tests `is_player_valid` when assigning an interaction unless
the ignore-validity override is set. The dig stub does not set that override.
The exact validity helper and existing-trigger behavior remain gaps; do not infer
downed/dead eligibility. Map forwarding adds no independent validity predicate.

The dig hint distinguishes equipped/unequipped players but returns visible in
both cases. `waittill_dug` waits for a forwarded trigger and accepts only a defined,
truthy personal shovel flag. Without that flag it waits again; no site is consumed.
An accepted event immediately marks the shared site dug and decrements global
active-site count, before any quest/reward choice. It requests dig audio, effects,
player shake and statistics. It does not deduct score or consume the shovel.

Next, a staff-piece hook takes priority. Otherwise reward selection consults
personal dig/loss/upgraded/helmet state and random calls. The non-upgraded dig
counter increments after reward dispatch, including a staff-piece outcome; at
30 it sets upgraded ownership and starts a separate special-dig watcher. The
upgraded helmet early-return path still follows shared-site consumption but
does not pass through that counter block. No reward table, expected payout or
probability is asserted here. Script comparisons alone do not prove RNG range,
uniformity, seed or correlation. In particular the source compares prize roll
strictly greater than its threshold, so a threshold named 50 is not by itself
proof of an exact 50% outcome.

Returning to `dig_spot_spawn` removes the interaction and mound. There is one
waiting coroutine per spawned mound. No second accepted dig is handled by that
coroutine after return. Simultaneous-use ordering, shared trigger lifecycle and
unregistration races need original comparison before claiming exactly-once
network behavior. An implementation proposal should make depletion idempotent
while retaining this unresolved ordering as a falsifier.

## Observations: replenishment of a consumed site

Anchor: `dig_spots_respawn`; disassembly confirms end-of-round wait, integer-2
script wait, weather branch priority and less-than-or-equal count comparison.

Replenishment waits for an `end_of_round` notification and a nominal two-second
script wait, then calls the shared array-randomization helper on all site structures.
Ordinary quota starts at 3; snow >0 takes precedence and sets it to 0, otherwise
rain >0 makes it 5. If snow equals zero, it adds an indexed random call with
current roster size as the bound. Native random-call bounds and array permutation
algorithm were not resolved; supply observed draw/permutation explicitly in a
synthetic model instead of asserting a uniform range or fixed replenishment rate.

In that order, a candidate must be defined/truthy dug, quota must remain, and
current count must be **<=15**. The script clears dug, starts its spawn thread,
waits a network frame, then increments the replenished-this-pass counter.
It does not filter ordinary candidates by player's access or map-zone occupancy
in this function. A serial model where the spawn's immediate count increment
precedes the next candidate admits one site at count 15 and then reaches 16.
Both <= and the spawn increment are directly observed; the exact thread/network
interleaving and any effective runtime overrides remain unmeasured. Do not silently
replace <= with < or claim that 15 is an observed hard runtime cap.

Snow suppresses ordinary replenishment but has a separate staff-dependent branch:
for each remaining ice-staff piece, it finds sites whose zone contains that
piece's zone substring, counts active mounds, and may randomly respawn one depleted
candidate when active count is below 2 and the same <=15 guard permits it. Thus
"no mound replenishes during snow" is contradicted by source structure. The
staff-piece lifecycle, zone membership and branch multiplicity are adjacent gaps;
the representative synthetic model excludes this quest-specific exception.

A particular consumed mound is not guaranteed to return next round: candidate
order, quota, active count, weather and staff-zone eligibility govern selection.
There is no independent per-mound countdown in the ordinary replenishment function.

## Proposed boundary, falsifiers and validation limits

Inference/proposal: personal persistent capability is separate from world pickup
availability and shared excavation-site lifecycle. Reusable inventory can express
normal/upgraded capability without equipping a fabricated weapon. A site admits
an eligible use, records one depletion, emits a reward request, and later admits
a respawn request. Origins owns weather/round quota and staff exceptions. Bevy
owns entities, messages and scheduling; shared randomness should have explicit
draw/permutation inputs for replay. No production API or new generic subsystem
is approved by this research task.

Meaningful falsifiers: two players receiving a shovel from one world pickup;
digging consuming shovel ownership; a no-shovel event depleting the site; reward
failure restoring a depleted site; respawn occurring independently of round events;
a strict runtime cap of 15 despite the observed <= guard; different pickup
return location on disconnect; or a trigger accepting an otherwise invalid player.
These require precise original observations and may expose helper or scheduler
behavior absent from this source-only record.

Run `python <this-directory>/excavation_contract_checks.py` for synthetic cases
with caller-provided eligibility, candidate order and random quota. Tests isolate
consumption/repeat behavior, emerging versus ready count, count-at-limit,
weather priority, refill quota and disconnect location. They do not reproduce
native triggers, original RNG, original scheduler or reward helpers.

Pending original comparison procedure: record this build, round, mode, roster,
player/life state, weather and mound/pickup identity. Compare no-shovel use,
successful use then repeat, two players using one mound/pickup, pickup origin
versus disconnect origin, depleted site across a round event, quota under dry/rain,
snow with/without remaining staff pieces, and count at 14/15/16. Observe count at
spawn versus trigger readiness and actual event-to-ready times. Repeat quota cases
with recorded draws or enough samples to distinguish order/range hypotheses;
record frame/time resolution and avoid interpreting a small sample as RNG proof.
No live comparison was executed. Overall excavation fidelity remains INVESTIGATING.

Adjacent robot contact, buildable assembly, tank traversal, weather movement,
staff-part selection, reward spawning and shovel death persistence remain backlog.
