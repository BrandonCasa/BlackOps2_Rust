# Origins economy 02: one Pack-a-Punch payment and return

Recorded 2026-10-05. **INVESTIGATING**. Original compiled-script observations
support a modifier-free first upgrade of `mp40_zm` to `mp40_upgraded_zm`, with
payment, temporary weapon removal, owner retrieval or expiry, and Origins shutdown
deferral. No original gameplay comparison or production implementation occurred.

## Sources, interpretation and procedure

The source candidate is Steam Zombies manifest build `24784239`, English labels.
Reuse the [previous purchase record](purchase-boundary.md) for inherited OAT
extraction provenance, shared score helper, tool identities and decoding limits.
This task did not re-extract fastfiles; it reused retained external compiled
scripts read-only. Paths resolve through ignored local configuration. Container
hashes below were rechecked against the inherited receipt.

| Relative source | Bytes | SHA-256 |
| --- | ---: | --- |
| `zone/all/patch_zm.ff` | 3,001,792 | `f5a27345a61de5fabd735f45f05b127e95ad9a488619bd7fbc96ac14c5ab4d47` |
| `zone/all/zm_tomb_patch.ff` | 1,288,384 | `770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca` |
| `maps/mp/zombies/_zm_perks.gsc` (patch) | 68,541 | `1a0e5f410486d8a34ea297a8c9cef0eaa22ce5ca01ba5a7df828f2850777ebc0` |
| `maps/mp/zombies/_zm_weapons.gsc` (patch) | 46,245 | `e0af5ee97c9e16eae03166d5fd9f3854efaa9cee3d373f82a20dd663fce90870` |
| `maps/mp/zm_tomb.gsc` (tomb patch) | 63,195 | `9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6` |
| `maps/mp/zm_tomb_capture_zones.gsc` (tomb patch) | 52,210 | `c32043a75c53eb5cae4779d2495e4b55500dfb5917e2743d2d9228305244857b` |
| `maps/mp/zombies/_zm_power.gsc` (patch) | 10,453 | `cdf02177d04435ea61524ec0f1c658788e8144b935619bd40c5aec96a15ea789` |

Additional directly hashed dependencies are `_zm_score`,
`_zm_magicbox` and `_zm_utility`; exact hashes/lengths are in the external
`mechanics-economy-02/identity-receipt.json`. Only user-created code, abstract
semantics and permitted identifiers/hashes are tracked. Original script bodies,
disassemblies, models and data stay external.

Reproduce in a unique external directory:

1. Verify retained source identities and OAT `v0.33.0`/GSC Tool `1.4.10.333`
   identities from the previous record. This task inherited extraction success;
   that is distinct from having independently validated OAT's complete decoder.
2. Reuse perk, tomb, capture-zone, score, magicbox and utility views. Run
   `gsc-tool -m decomp -g t6 -s pc <one-compiled-script>` and the corresponding
   `-m disasm` for `_zm_weapons`; disassemble `zm_tomb`; decompile `_zm_power`.
   Four invocations, no directory-wide decoding, no game process/input or launch.
3. Inspect the named functions in the transition table and the opcode anchors
   below; compare the critical decompiled branch/call order against disassembly.
4. Hash source dependencies and tools; retain output/receipt externally. Run the
   user-created checker from repository root:

   ```text
   python docs/reverse_engineering/origins-mechanics/economy/check_synthetic_pap.py
   ```

S = licensed compiled-script observation via decompilation; B = cross-check in
its disassembly using the same tool; I = architectural inference; M = measured
original behavior. **No M evidence**, native execution, timing measurements or
multiplayer comparison exists here. Script overlays/FFOTD, caller-installed
callbacks and built-ins can alter behavior; decoded views are not original source.

## Representative initial conditions

Origins registers and includes the original MP40 and its upgrade identifiers.
The shared registration helper records the upgrade mapping. The representative
is a first upgrade, not attachment reroll: weapon included, current weapon valid,
no action conflict, no moving machine, no custom validation refusal, no sale or
persistent price modifier, and no intervening level intermission. After original
weapon removal, inventory remains below the applicable primary weapon limit at
retrieval. Power setup and all-zone availability have completed before interaction.
These are controlled scenario assumptions, not proof that callbacks/modifiers are
universally absent in Origins.

## Shared transition and map eligibility

| Phase | Original function anchors | Established rule and limit |
| --- | --- | --- |
| Map availability | capture zones `get_captured_zone_count`, `update_captured_zone_count`, `pack_a_punch_think/enable/disable` (S/B critical disable) | Count player-controlled zones; exactly 6 sets all-zones-captured. That flag enables the machine trigger; loss enters deferred disable. Not the local-zone rule used by Juggernog. |
| Weapon definition | tomb weapon registration/inclusion; weapons `add_zombie_weapon`, `is_weapon_or_base_included`, `can_upgrade_weapon`, `get_upgrade_weapon`, `will_upgrade_weapon_as_attachment` (S/B mapping) | MP40 upgrade is explicitly configured. A defined upgrade mapping admits an unupgraded included weapon; first upgrade is not attachment mode. Native attachment/option/ammo behavior remains unmeasured. |
| Admission | perks `vending_weapon_upgrade` and `player_use_can_pack_now` (S) | Custom validation, action/player eligibility, moving-machine refusal, switching recheck after 0.1 script seconds, and weapon inclusion precede affordability. Trigger visibility separately hides other players while an owner is reserved. Native use admission/order is not proved. |
| Price | `vending_weapon_upgrade_cost` (S/B) | Normal first-upgrade price is 5000 points; attachment price 2000 and sale branches exist but are outside the representative. Effective price can be adjusted by a persistent helper. Score below effective price refuses before in-use/debit. |
| Reservation/payment | `vending_weapon_upgrade`, score `minus_to_player_score` (S/B) | Set machine owner and global in-use flag, start disconnect/blackout watchers, debit effective price with modifier bypass, disable interaction, then start knuckle handling and upgrade presentation. No purchased weapon grant yet. The score helper can no-op on level intermission and has no consumed success return; callbacks/thread scheduling forbid an unconditional atomic-payment claim. |
| Temporary removal | `do_knuckle_crack`, `upgrade_knuckle_crack_begin/end` (S/B removal) | Increase busy/drinking state, temporarily constrain movement and remove the current ordinary weapon. Give/switch a temporary knuckle item; after its sequence restore access to another primary/fallback, not the original MP40. Concurrent callbacks could change the current weapon before this thread begins; the representative holds it stable. |
| Processing/ready | `third_person_weapon_upgrade`, `pap_weapon_move_in/out`, purchaser loop (S) | Create transient original/upgrade models; after processing record upgrade name and expose retrieval to buyer. Default script waits include 0.5, 0.35 and 3 seconds, but custom movement callbacks and native scheduling prevent asserting total processing time. Presentation models are not inventory ownership. |
| Retrieval | `wait_for_player_to_take` (S/B notification-before-grant) | Default buyer-only retrieval; an optional anyone-grab setting changes this. Require valid player and current item, no drinking/hacker/blocked equipment/revive-tool/none. Failed retrieval remains waiting. Accepted retrieval emits machine and player taken notifications **before** inventory grant. Remove fallback fists, then below primary limit give upgraded weapon/options, start ammo and switch. At capacity delegate to weapon-give replacement logic; that branch is not the representative. |
| Timeout | perks init, `wait_for_timeout` (S/B parameter/default) | Default timeout 15 script seconds only if not already configured. Timer starts after processing exposes retrieval, not at payment. Taken/disconnected notifications terminate the timeout waiter. On expiry emit timeout, clear associated weapon-toggle acquisition when applicable, and record not-grabbed statistics. Inspected timeout path does not refund or restore original inventory. |
| Terminal cleanup | purchaser loop; `wait_for_disconnect`, destruction watchers (S/B flag clear) | On taken/timeout/disconnected signal, delete transient models, restore normal hint/visibility, clear owner and in-use flag, then listen for another purchase. Physical trigger state is separately managed. No refund occurs in these scoped terminal bodies; global refund policy is unestablished. |

## Shutdown is not one generic cancellation

Origins' `pack_a_punch_disable` first sets all-generator HUD state to 0, waits for
the in-use flag to clear, then switches the trigger off. Therefore generator loss
does not directly interrupt a paid machine episode in this inspected map path.
It neither emits the shared `Pack_A_Punch_off` event nor clears global power in
that body. Availability, HUD feedback, in-use reservation and a destructive
power event must remain distinguishable (I).

Generic `_zm_power` initializes powered-item callbacks and follows a global power
flag. Its PaP off callback emits `Pack_A_Punch_off`, terminates shared worker
threads/restarts the purchaser and notifies target death; the blackout watcher
can destroy the transient weapon after 1.5 script seconds. These are concrete
shared paths, but **not proof that generator loss dispatches them in Origins**.
Other local-power, map/FFOTD or caller paths were not exhaustively resolved.

In particular, accepted retrieval emits taken before grant. The parent loop may
clear in-use before inventory mutation finishes, and Origins' waiting shutdown
can then proceed. Exact interleaving and native success/failure are unmeasured.
Do not interpret deferred shutdown as a guarantee that inventory return completed
atomically. Loss then recapture while disable waits is another unmeasured race:
the map loop can finish its outstanding disable and next re-enable. Concurrent
episode admission, timeout-at-pickup tie and pre-ready disconnect cleanup also
need comparison; synthetic serialization is not native scheduling evidence.

## Opcode anchors and review

No original assembly text is reproduced here. External function sections support
these bounded checks:

- `_zm_perks::vending_weapon_upgrade_cost`: unsigned-short literal 5000 assigned
  to cost; 2000 assigned to attachment cost.
- `vending_weapon_upgrade`: owner field assignment and in-use flag-set call
  precede score-debit call (argument 1), temporary interaction disabling, upgrade
  work and ready wait; terminal flag-clear follows model cleanup.
- `upgrade_knuckle_crack_begin`: current-weapon check and weapon-removal call
  precede temporary weapon grant/switch.
- `wait_for_player_to_take`: taken notifications precede fallback removal,
  inventory-limit decision and upgraded weapon grant/start-ammo calls.
- `wait_for_timeout`: waits using configured timeout field; taken/disconnect
  termination guards and timeout notification precede toggle helper/statistics.
- capture `pack_a_punch_disable`: in-use-open wait precedes native trigger-off
  call; all-generator client state is cleared before the wait.
- tomb registration/inclusion: paired MP40/upgrade identifiers feed registration
  and are separately included. Weapons mapping/admission helpers also reviewed.

## Synthetic validation and comparison plan

The [checker](check_synthetic_pap.py) and [fixture](synthetic-pap-cases.json) are
user-created abstract tests. They exercise 4999/5000/5001 points, ready-before-timeout
boundaries, wrong/invalid retriever refusal, serial replay with one debit/return,
timeout/disconnect disposal, deferred shutdown and notification-before-grant.
The model makes removal and grant explicit supplied events; it does not infer
animation completion, native time or refunds. A synthetic take then cleanup then
grant schedule demonstrates why in-use clear alone cannot prove acquired ownership.
All 15 cases pass (exit 0, `status: PASS`, `case_count: 15`) with the command
above. Actual command result is also recorded in the handoff; compiler gates do not test this
research-only model. Simultaneous ties, pre-ready disconnect and generic destructive
power events are excluded rather than silently assigned behavior.

Unexecuted original comparison procedure:

1. Record build/mode, player count, round, six generator states, exact current
   weapon/options/ammo, inventory slots, price modifiers and machine state. Verify
   normal simulation speed 1 and unpaused play; Escape-pause and console timescale 0.5
   are testing leads, not verified controls or baseline conditions.
2. Compare score 4999/5000/5001 with MP40 first upgrade. Capture balance, original
   removal, temporary item, ready model, upgraded return and ammo. Use separate
   wall/game timestamps, frame resolution and repeat count; source wait sums are
   not a substitute for these measurements.
3. Try non-owner and blocked-action retrieval, then buyer retrieval. Compare
   transaction/grant count under repeated use without assuming native serialization.
4. Do not retrieve: time ready-to-expiry and observe original/upgrade inventory,
   balance and next machine availability. Separately inspect a disconnect case
   without intruding on another chat's controlled session.
5. Lose a generator while processing and while ready. Observe early HUD change,
   retrieval/timeout, trigger shutdown, inventory grant and any destructive event;
   regain power during the pending shutdown in a distinct controlled case.

Reusable candidate: shared point debit, machine owner/reservation, staged weapon
replacement, pending return and terminal result; Origins supplies the six-zone
availability and deferred disable rule. Use existing Bevy scheduling/messages
and shared player-state boundary. Do not create a script/event engine, assume a
universal perk/weapon transaction, or couple reusable inventory to Origins.

Next bounded work: original comparison of normal MP40 return versus ready expiry,
then shutdown interleaving. Remaining price modes, ammo options, at-capacity
replacement, native validity and dispatcher paths are explicit backlog items.
