# Origins economy 01: one Juggernog purchase boundary

Recorded 2026-10-05. Question: which conditions admit a normal Juggernog machine
purchase, when are points debited, and when is ownership acquired? **Result:
original compiled-script evidence establishes an ordered, staged transition.**
Gameplay fidelity remains **INVESTIGATING**; no original game comparison occurred.
This is research, not a gameplay implementation or a complete economy model.

## Evidence and reproduction

Use the Steam Zombies manifest build `24784239` candidate already recorded in
[the directory evidence](../../0004-origins-asset-directory.md). This task reused
external retained OAT script outputs from `damage-life-state-01`; it did not
launch the game, inspect its process, or extract another container. Actual local
roots resolve only through ignored `config/local_assets.toml`.

| Source identifier | Bytes | SHA-256 |
| --- | ---: | --- |
| `zone/all/patch_zm.ff` | 3,001,792 | `f5a27345a61de5fabd735f45f05b127e95ad9a488619bd7fbc96ac14c5ab4d47` |
| `zone/all/zm_tomb_patch.ff` | 1,288,384 | `770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca` |
| `maps/mp/zombies/_zm_perks.gsc` in patch | 68,541 | `1a0e5f410486d8a34ea297a8c9cef0eaa22ce5ca01ba5a7df828f2850777ebc0` |
| `maps/mp/zm_tomb_capture_zones.gsc` in tomb patch | 52,210 | `c32043a75c53eb5cae4779d2495e4b55500dfb5917e2743d2d9228305244857b` |
| `maps/mp/zombies/_zm_score.gsc` in patch | 7,170 | `74e45076f04c32de9f323177ae1fb6eb74c4b4b4aeac36737b7bdec4aac86d99` |
| `maps/mp/zombies/_zm_magicbox.gsc` in patch | 25,917 | `f095b54e151612312e177049ad60c22f766bca93a55f32044f9959568c998b2e` |
| `maps/mp/zombies/_zm_utility.gsc` in patch | 74,393 | `485e791715a5660414238878ee1fa77facfe1b2612ff367d0b830b77a516e1b1` |

The container identities above came from the retained extraction receipt and were
rehashed in this task. All five compiled script hashes were checked directly.
OAT `v0.33.0` binary hash remains
`9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63`.
The reused GSC Tool reports `1.4.10.333`, binary hash
`3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec`.
Those identities pin the interpretation tools; they do not prove perfect decoding.

Reproduction, outside the repository:

1. Verify container/script identities against the retained extraction receipt.
   Use read-only originals and an empty task-specific output directory.
2. Reuse `_zm_perks` and `zm_tomb` decompiled views and `_zm_perks` disassembly.
   Invoke `gsc-tool -m decomp -g t6 -s pc <one-compiled-script>` individually for
   capture zones, score, magicbox and utility, with external task directory as
   working directory. Never supply an installation directory for recursive input.
3. Invoke the analogous `-m disasm` commands individually for capture zones and
   score. Inspect only named functions below. Raw/decompiled/disassembled content
   stays external under `mechanics-economy-01`; no original text is tracked here.
4. Compare branches/calls/constants in disassembly with the decompiled reading:
   callback before affordability and cap; debit before pending/perk animation;
   score subtraction, score mirror and notification; generator validation return;
   acquisition helper's 0.5-second notification-or-timeout parameter.
5. Rehash compiled dependencies/tools; run the synthetic case checker and check
   tracked links/privacy. The external `identity-receipt.json` and
   `synthetic-results.json` retain this task's exact artifact identities/results.
   The user-created checker and fixture are also tracked for independent reruns.

**Evidence levels:** S means a semantic observation in licensed compiled scripts,
interpreted by the pinned tool. B means the stated observation was additionally
checked in its disassembly (same decoder, not independent verification). I means
an architectural inference. M would mean measured original gameplay; this task
has **zero M samples**, no animation duration measurement, and no multiplayer run.
Native built-ins, fastfile overlay/load order, FFOTD overrides and scheduling are
not fully resolved. An anomalous unrelated upgrade case string in the decompiled
bottle selector is another reason not to treat that view as original source code.

## Initial inventory and representative scope

The base map directory alone lists scripts, not semantic rules. Retained patches
provide the shared perk purchaser plus Origins capture-zone registration and
validation. `zm_tomb` enables Juggernog and installs custom presentation callbacks;
the shared purchaser owns admission, debit and bottle staging. There is no
existing production economy/inventory abstraction to duplicate in this workspace.

The representative is `specialty_armorvest` purchased at `vending_jugg`, registered
to `generator_nml_right`. The captured-zone registration provides a concrete
Origins anchor; it is not an inferred generator association from a model name.
The selected scenario is a normal, non-cheat player with an eligible current
weapon, no active purchase/action conflict, fewer than the applicable perk cap,
and no active persistent cost modifier. The modifier-free condition is a scenario
precondition, **not** a proven universal Origins setting.

## Ordered transition

| Phase | Original anchor and evidence | Abstract observable rule |
| --- | --- | --- |
| Setup | `_zm_perks::vending_trigger_think`, `turn_jugger_on` (S) | Nominal Juggernog price is 2500 points. Trigger requires looking at it. Initial purchaser waits for its perk power-on notification. Models, hints and trigger `power_on` are separate from purchase validation. |
| Player admission | `vending_trigger_think`, `_zm_magicbox::can_buy_weapon` (S) | Reject action conflicts: unavailable player state/intermission, revive interaction, positive drinking state, hacker action, placeable mine/blocking equipment, no current weapon, grenade throw or weapon switch. Some busy rejections wait 0.1 script seconds before listening again; this is not a measured input debounce guarantee. |
| Existing ownership | `vending_trigger_think`, `has_perk_paused` (S/B ordered branch) | Active **or paused** ownership rejects a duplicate purchase in the normal non-cheat path. This precedes zone validation and affordability. |
| Map eligibility | `setup_capture_zones`, `register_elements_powered_by_zone_capture_generators`, `check_perk_machine_valid` (S/B validator) | Origins installs a callback. Juggernog requires its assigned zone's `player_controlled` flag. The callback's exception list contains `specialty_additionalprimaryweapon`, not Juggernog. It checks zone control directly rather than relying solely on a hint, lock bit or global power flag. |
| Price | `vending_trigger_think` (S/B) | Start from nominal price; a persistent-upgrade helper may produce the effective price. Compare current score against that effective price. Exact equality passes affordability; 2499 fails a modifier-free 2500 purchase. |
| Capacity | `vending_trigger_think`, `_zm_utility::get_player_perk_purchase_limit` (S/B caller) | Reject when `num_perks >= applicable limit`. Shared default is 4; the utility allows a map callback override. Do not assume every perk acquisition source obeys machine purchase capacity. |
| Payment | `vending_trigger_think`, `_zm_score::minus_to_player_score` (S/B) | Call debit with the effective price and bypass the helper's additional modifier transformation, then record pending purchased perk and start post-purchase processing. Debit subtracts, mirrors the score into the player's persistent score field and emits a spent-points notification. No second affordability check or clamp exists in that helper. It returns without debit during level intermission. |
| Pending acquisition | `vending_trigger_post_think`, `perk_give_bottle_begin` (S/B caller order) | Payment does not itself set the perk or increment perk count. Begin bottle handling, positive drinking state and temporary weapon/action handling. Await sequence event; only the successful completion branch starts the acquisition waiter. |
| Acquisition | `wait_give_perk`, `give_perk` (S/B waiter) | Wait for a burp notification or 0.5-second script timeout, then set perk and increment count. Acquisition updates owned/history/active lists, client indication, statistics, perk-specific handling, and emits acquisition/bought events. This number is a waiter bound, **not** total drink duration. |
| Repeat | purchaser loop and ownership checks (S) | Once drinking state is established, another interaction is busy; once acquired, another purchase is duplicate-owned. Same-frame event ordering before drinking begins has not been measured. Do not claim native atomicity or exactly-once processing. |

Failed pre-payment admission does not invoke debit or grant in the inspected path.
The first failing branch determines feedback: existing ownership denial precedes
power denial; power denial precedes insufficient points; affordability precedes
capacity. Costs are integer script values in points; no real currency is involved.
Disconnect/end-game/abort cancels the staged threads. No refund occurs inside the
inspected purchaser/post-purchase functions, but a global refund policy is **not**
established. Damage/downing/revival outcomes belong to the separate life-state
investigation and are deliberately not specified here.

Admission checks player intermission, while the score helper checks level
intermission and has no success/failure return consumed by the caller. Map or
price callbacks could also change state or yield before debit. Whether either
race is reachable in the original remains unmeasured; the synthetic checker
assumes unchanged callback inputs and level intermission false throughout. It
does not assert an atomic transaction or that every admitted event necessarily
debits under all runtime conditions.

## Power and persistence boundary

Origins zone gain/loss toggles machine lock/hints and emits on/off notifications.
The ordinary Juggernog on/off loop changes machine state; its inspected off branch
does not call perk removal. Purchased ownership, purchase eligibility and machine
presentation therefore cannot be one Boolean (I). This is not proof of complete
ownership persistence after arbitrary power loss: other scripts/FFOTD/runtime
paths have not all been traced.

The later [Pack-a-Punch investigation](pack-a-punch-02.md) also inspected the
generic `_zm_power::perk_power_off` callback: it directly invokes shared perk
pause before emitting the machine off notification. Its dispatch on a particular
Origins generator-loss path remains unresolved. The Juggernog loop's lack of a
pause call must therefore not be generalized to the complete power dispatcher.

The shared post-purchase path also checks trigger power after completion and may
invoke perk pause after a one-second script delay. Paused ownership remains in a
separate disabled-perk record and blocks another purchase. A purchase that crosses
zone loss needs original comparison: order between return-weapon completion,
grant waiter, power updates and this pause is unresolved. Shared pause helpers
exist, but this task does not claim every Origins generator loss invokes them.
Across round changes, matches, disconnects or player life-state transitions,
ownership persistence is unestablished here. The score mirror is an observed
field update, not proof of disk/account persistence.

## Neighbor dependencies and research backlog

| Neighbor | Established dependency only | Still requires bounded research |
| --- | --- | --- |
| Generator capture | Juggernog is registered to one zone; purchase callback tests current player control. Zone gain/loss updates registered machine/Box lock and feedback. | Capture cost, progress, cancellation, player count, recapture timing and rewards; initial map entity generator numbering; FFOTD patch effects. |
| Mystery Box | Each Origins Box is registered to a zone. Gain/loss changes `is_locked` and barrier/rune state. Shared `can_buy_weapon` is also used by perk purchase. | Exact purchase/debit/draw/acquisition/refund chain, cost modifiers, selection distribution, movement and concurrency. Registration/lock alone does not establish a usable draw. |
| Pack-a-Punch | Origins replaces standard turn-on assembly. `all_zones_captured` enables its trigger; loss waits for `pack_machine_in_use` to clear before disabling. Shared perk script has a separate upgrade purchaser. | Which in-flight phases survive power loss, upgraded ownership/return timeout, effective cost, refund, eligible weapon forms and original comparison. Never reduce shutdown to immediate cancellation. |
| Persistent upgrades | Price-adjustment and map cap callbacks are supported at shared purchaser boundaries. | Whether enabled in this Origins build/mode, factor/rounding, and callback installation or later override. Do not substitute the ordinary double-points power-up for a persistent upgrade. |
| Other perk acquisition | Machine purchaser distinguishes ownership from pending bottle state; random/free-perk entry points exist. | Wunderfizz/random/free rewards, special caps, duplicate prevention and free grants. No machine-price rule is generalized to them. |

Recommended next bounded economy task: trace one modifier-free Pack-a-Punch
payment-to-return transition and the explicit shutdown deferral, with compiled
anchors and one original comparison plan. Generator activation is a separate
dependency task, not an implicit prerequisite to researching that transition.

## Proposed reusable data and tests

Proposal (I), not an approved API: shared gameplay/Zombies owns a point balance,
owned/paused perk state, pending acquisition state, effective price, purchase
capacity and staged interaction events. Origins supplies machine-to-zone binding
and an eligibility result. Keep debit admission distinct from eventual grant.
Preserve reason order for feedback. Use Bevy resources/components and ordered
systems/messages; do not create an event engine or a map-dependent lower layer.
Transaction IDs or atomic admission may be useful implementation choices, but
must not be presented as original mechanisms or used to erase observed races.

[Synthetic cases](synthetic-purchase-cases.json) use invented player states and
expected abstract outcomes. The [checker](check_synthetic_purchase.py) tests affordability at -1/0/+1,
capacity equality, active and paused duplicate refusal, decision priority,
generator independence from global power, debit conservation/score mirror,
pending-before-grant, replay with one debit/grant, and cancellation without an
automatic refund in this scoped model. All are **model checks**, not tests of
production Rust, the decoder, or original runtime. All 16 cases passed on
2026-10-05 with the tracked checker, using this exact command from repository root:

```text
python docs/reverse_engineering/origins-mechanics/economy/check_synthetic_purchase.py
```

The command prints `status: PASS` and `case_count: 16`; assertion failure exits
nonzero. Unknown simultaneous-event
ordering is excluded rather than silently simulated. See the receipt for actual
results; there are no new Rust files, so workspace compiler gates do not validate
this documentation investigation.

Future original comparison cases, all unexecuted:

1. Control build/mode, player count, round, owned perks, active modifiers and
   weapon/action state. With Juggernog's zone controlled and fewer than cap,
   compare balances 2499, 2500 and 2501 before/after use, bottle finish and HUD grant.
2. Repeat with its zone uncontrolled, and with another zone controlled. Observe
   denial, hint, balance and ownership; this discriminates local-zone from global
   power eligibility. Repeat after recapture and reacquisition.
3. At applicable capacity, and while already owning Juggernog, observe unchanged
   balance and denial. Combine ownership/unpowered and insufficient/cap states to
   distinguish reason order where feedback permits.
4. Repeat interaction during bottle handling and immediately after completion.
   Record input timestamps, balance changes and grant count; never infer native
   event serialization from a synthetic model.
5. In a separately authorized comparison, lose power during the purchase sequence
   and after acquisition; record owned/active/paused HUD state and eligibility.
   Distinguish acquisition from later life-state persistence research.

Record script-time units separately from measured wall/frame time. No run count,
timing resolution or fidelity match can be reported until these cases execute.
