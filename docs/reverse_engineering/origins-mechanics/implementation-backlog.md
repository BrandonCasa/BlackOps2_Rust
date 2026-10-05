# Dependency-ordered mechanics implementation backlog

Recorded 2026-10-05. These are bounded next tasks, not production implementation
or an obligation to finish all research before independently supported gameplay.
Use [coverage](README.md) to distinguish established source semantics from gaps,
and [the boundary proposal](../../architecture/origins-mechanics-proposal.md)
for ownership. Every implementation receives outer review and applicable locked
metadata/test/fmt/Clippy gates; synthetic passes do not promote fidelity.

## Comparison preconditions

Original inputs remain read-only/external; use ignored local configuration and a
unique external output directory per task. Record source hashes, effective runtime
build/mode, player count, initial progression, seed/random decisions and units.
Steam script build 24784239 is not established as equivalent to Plutonium r5354.
Coordinate the shared session before input. Launch through Plutonium launcher if
needed; use the green robot-head planet icon for Origins, never direct t6zm launch.

Escape is the user-provided pause lead. The proposed console `\timescale 0.5` is
an unverified slow-motion lead. Verify actual pause/scale effect before use;
record wall time and game time separately. Restore and verify normal speed for
baseline timing, and exclude paused intervals explicitly. Do not infer script
wait duration or simulation frequency from a paused/slowed observation.

## First implementation chain

1. **M01: bind effective script overrides and round enemy query.** Evidence:
   [round accounting](rounds/normal-round-accounting.md), especially the custom
   Origins callback that the inspected utility does not consume. Read only the
   exact consumer/overlay family; output one bounded record under `rounds/`.
   No code until the effective membership rule is admitted. Test a synthetic
   ignored ordinary enemy and ignored capture enemy through the selected rule.
   Compare original final-enemy behavior with identified runtime/load order.
   Accept when dispatch is evidenced or explicitly exclude the ambiguous actor
   class from a named first implementation scope. Depends on no other backlog task.

2. **M02: resolve VM arithmetic and active roster for the admitted budget.** Follow
   [configured budget evidence](rounds/player-count-budget-02.md); the table operands
   and operation order are already inspected. Resolve float/integer/division/int
   behavior and effective roster/callback state, not another generic formula audit.
   Deliver one `rounds/` record, not health/speed/AI work. Synthetic
   table covers roster1-4, first/threshold rounds and truncation boundaries; compare
   original ordinary admissions versus replacement credits/round totals for solo
   round9 or two-player round6, which distinguish the conditional models. Accept only
   source-supported formula with explicit active-config and runtime limits.
   Independent of M01, but both precede complete round configuration.

3. **M03: admit timing and replay input boundary.** Inspect actual shared
   life-command integration and Bevy fixed clock before adding reusable input
   data under `crates/gameplay` or common assembly. Measure one named original
   event sequence with pause/scale recorded; separate script waits from native
   ticks. Prototype tests run identical ordered requests through zero/one/multiple
   fixed updates per render frame and compare canonical state/receipts. Accept
   persistent requests/results, explicit ordering/time configuration and no custom
   scheduler. No guessed original tick rate. Depends on neighboring damage review.

4. **M04: implement one normal-round accounting core.** Files: a minimal module
   in reusable `crates/zombies` only when warranted, common plugin assembly and
   synthetic tests; Origins supplies admitted configuration. Follow
   [round accounting](rounds/normal-round-accounting.md). Tests cover pending0/live1,
   pending1/live0, both0, hold/override, failed spawn/no debit, admitted spawn/one
   debit, restart retiring old work and deferred membership visibility. Compare
   selected original final-spawn/final-kill sequence separately. Accept no premature
   completion and deterministic replay with M01/M02/M03 limits visible. Do not
   implement all spawning/navigation/special rounds in this task.

5. **M05: implement one modifier-supplied score debit and perk admission.** Evidence:
   [perk purchaser](economy/purchase-boundary.md). Files: reusable point/ownership
   module, a small shared Zombies purchaser and Origins local-zone adapter. Inputs
   supply effective price/cap until active modifier semantics are separately bound.
   Tests use the tracked16-case oracle plus checked arithmetic, admission refusal,
   cancellation, stale episode and serialized repeat input. Compare2499/2500/2501,
   owned/paused/cap and another-zone power cases in the original. Accept observed
   reason priority, debit separate from grant and no invented refund. The shared
   helper's intermission no-debit return with continued caller staging must remain
   an explicit ordering case; do not repair it with a universal atomic transaction
   without evidence. Depends on
   M03 and reviewed life eligibility; do not claim original event atomicity.

6. **M06: bind generator participant and score-helper semantics.** Evidence:
   [generator capture](world/generator-activation-01.md). Resolve actual player-validity
   helper mapping, roster denominator, mode-selected rate and net reward/refund
   transformations in one source/consumer task. Files: one world record and proposed
   configuration only. Synthetic tests distinguish valid roster from capture subset,
   refund departure and double-points argument/net result. Original comparison
   captures initiation, progress and score deltas. Accept explicit helper boundary
   with no duplicate life state. Depends on owning damage foundation and M05 score
   boundary; source inspection can proceed earlier.

7. **M07: implement one ordinary generator capture.** Files: Origins configuration,
   generator component/plugin and focused tests, reusing point/life/interaction
   boundary. Tests cover global admission lock, strict radius/Z edges, full/partial
   participation, leave/reentry, raw-negative-to-clamped-zero termination, reward
   and conditional refund. Compare repeated solo captures at verified normal speed
   and label source-derived versus measured duration. Accept one named ordinary
   capture and linked eligibility update; recapture waves/robot/quest work excluded.
   Depends on M03/M05/M06. The 10 synthetic math checks are a proposal oracle,
   not original float precision evidence.

## Next independent branches

8. **M08: Pack-a-Punch staged weapon return.** Follow the bounded
   [PaP record](economy/pack-a-punch-02.md) and
   [perk dependency inventory](economy/purchase-boundary.md).
   Before implementation admit actual eligible weapon identity, effective price,
   removal/return and in-use shutdown stages. Files: shared weapon inventory/upgrade
   episode, Origins all-zone availability adapter and synthetic tests. Tests cover
   one admitted purchase, duplicate use, power loss during in-use, timeout/abort and
   return ownership/ammo. Compare each selected phase in the original. Accept scoped
   source-supported behavior; Box draw and weapon damage/firing remain excluded.
   Depends on M03/M05 and reviewed weapon ownership; generator simulation is not a
   prerequisite to testing supplied all-zone availability.

9. **M09: one excavation ownership/depletion transition.** Follow
   [excavation](world/excavation-02.md); bind shovel ownership, site admission, shared depletion and
   reward selector separately. Files: Origins dig-site/configuration and a small
   inventory adapter only when recurring ownership supports reuse. Tests include
   shovel absent/present, repeat depleted site, failed admission/no decrement,
   source count boundary and recorded random choice/replay. Compare a selected
   normal dig and later replenishment under recorded weather/round. Accept shovel
   persistence and site mutation without inventing reward probabilities.
   Depends on M03; reward branch can be stubbed only as an explicit unresolved
   output for an inspector/prototype, not silently approximated in gameplay.

10. **M10: one staff build versus claim transition.** Follow
    [base staff](quests/staff-build-ownership.md). Files: typed Origins recipe/team-build state and shared weapon
    ownership episode after review. Tests distinguish team recipe completion from
    player grant, carried versus shared parts, same-staff owned denial, exchange
    order and returned ammo. Compare one recorded build and claim/repeat sequence.
    Accept explicit ownership scopes and source-supported order; no copied scripts
    or universal craftable abstraction from one recipe. Depends on M03 and reviewed
    weapon inventory. Damage/firing, upgrade puzzles and main quest excluded.

11. **M11: one initial staff-charge threshold transition.** Follow
    [upgrade/event evidence](quests/staff-upgrade-02.md). Supply the admitted unlock
    and qualifying credit outcomes as inputs; active death callback and exact gem
    prerequisite binding require their own small investigations before full map
    assembly. Files: Origins charger/progression state and focused tests; shared
    message helper only if independent examples justify it. Tests cover locked
    state, credit19/20, count preceding delayed soul arrival, repeated notifications,
    array tie order and completion without player grant. Compare original initial
    charging with initial flags/weapon variant recorded. Accept explicit count,
    completion and presentation boundaries; no full quest or generic script engine.
    Depends on M03 and M10 only where actual insertion inventory is included.

## Explicit research backlog beyond the representatives

| Family | Next small investigation | Dependency / exclusion |
| --- | --- | --- |
| Spawn eligibility | One zone/barrier admission and failed spawn retry | M01/M02, no navigation rewrite |
| Capture attackers / special populations | One admission/counting interaction with ordinary round completion | Generator controller + M01; do not assume generic dog rounds |
| Box | Paid draw -> selected weapon -> claim/expiry episode and local-zone lock | Score/inventory; randomized table/distribution needs separate evidence |
| Perk persistence / Wunderfizz | One generator-loss pause/resume or free/random grant | Coordinate damage owner; life-state persistence remains excluded here |
| Robot | One cycle/foot-entry eligibility boundary | Coordinate damage owner on lethal contact; no collision/visual changes |
| Buildable | One shield part ownership -> team assembly -> player claim | Counterexample before extracting shared craftable recipe API |
| Tank | Paid departure -> stop/cooldown eligibility | Traversal input/physics owners; no route approximation |
| Chamber / teleporter | One admitted entry -> destination/exit state | Movement owner; explicit event and loading boundary |
| Staff prerequisite binding | One air gem's actual two external struct flags -> ordered crypt waits | Upgrade02 establishes consumer order but not the flag names; no puzzle lore substitution |
| Staff credit dispatch | One effective actor-death callback/helper consumer -> initial charger credit | Coordinate damage owner; no damage implementation or post-upgrade recharge assumption |
| Main quest | One cross-staff prerequisite and reset/cancellation transition | Do not combine whole quest into one task |
| Power/presentation | Compare one authoritative eligibility change with HUD/animation | Appearance owner; rendering is not admission authority |

All backlog entries require the project's concise required-format handoff. Source
inventory or a compiled prototype alone does not satisfy original comparison.
