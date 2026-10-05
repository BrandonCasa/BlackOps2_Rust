# Resolved damage, coop revival and terminal disposition

This boundary follows [damage evidence](../reverse_engineering/damage-life-state-01.md),
[ordinary revive source](../reverse_engineering/damage-life-state-03.md) and
[qualified native restoration](../reverse_engineering/damage-life-state-04.md) and
[ordinary coop bleedout](../reverse_engineering/damage-life-state-06.md).
Fidelity remains APPROXIMATE: native damage subtraction/clamping is inferred;
revive100/100 composes Steam build24784239 scripts with inspected Plutonium native
behavior. Loaded runtime-script agreement and live original comparisons remain
unverified. Positive integer player damage is an implementation admission restriction.

## Ownership and state

`bo2-gameplay` owns validated health, `Vitality`, typed lifecycle commands/results
and one Bevy fixed-step system. `bo2-origins` supplies observed unperked maximum100,
downed1, fallback actor150, and ordinary no-bonus coop restoration100/100. Gameplay
has no Origins dependency. The demo assembles both with existing core; no new
engine foundation, registry, timer framework or dependency is added.

Private Vitality fields keep current/max, human/ordinary-actor route, authoritative
life state, episode/attempt counters and active attempt coherent. Shared numeric
validation and ordered processing prevent duplicated player/zombie health systems.
A universal death route would contradict positive-health downing and native revive.
Actor death is terminal health0; human Downed and HumanTerminal remain positive.
Numeric health is not an alive/revival predicate. HumanTerminal preserves the last
qualified current/max because native spectator-spawn health remains unknown. Ordinary actor
cannot-die flags/special enemies and solo/last-player lethal routing are excluded.

Damage less than current health subtracts; equality/greater downs an admitted coop
human or kills an ordinary actor. Unsupported solo/last-player lethal damage rejects
atomically. Downed/dead repeat damage is ignored. Downing increments checked u64
episode counter. The Origins initial current health remains caller-selected: max100
or fallback150 does not establish actual spawn/round state.

## Resolved revive admission

Begin requires a human Downed target, its current episode reference and an externally
admitted Standing human reviver Entity. The fixed system also checks current reviver
existence/Standing state. Raw same-team/use/touch/facing/sight, eligibility modifiers
and timing are caller admission; no automatic3-second progress is implemented.
Self-revive, active-attempt replacement and nonhuman/standing targets reject.
An active attempt must be cancelled before a new begin is admitted.

Begin increments checked u64 attempt counter and returns a private token binding
target Entity generation, reviver, episode and attempt. Cancel requires that active
token and leaves authoritative Downed/health unchanged. This follows source trigger
semantics despite a script laststand flag being cleared after failed attempts.
Completion requires matching token plus a still-Standing human reviver and validated
positive current<=maximum restoration. Map assembly supplies100/100 for the admitted
ordinary no-bonus path; shared gameplay does not assume full health. Restoration
also preserves the configured future downed-health invariant. Completion sets
Standing, clears active attempt, then later ordered damage can down a new episode.

Cancelled/replaced-attempt completion returns StaleAttempt; a prior downed episode
returns StaleEpisode. Standing target returns NotDowned, actor NotHuman, wrong-token
target WrongTarget, missing/no-component/despawned target MissingTarget. Missing,
despawned, downed or actor reviver returns InvalidReviver. Completion failure leaves
active attempt and target unchanged; caller may explicitly cancel for cleanup.
Exhausted episode/attempt counters return CounterExhausted without mutation/wrapping.
Terminal human state rejects begin/cancel/complete with NotDowned; raw bleedout
timing and session-end behavior are outside this boundary.

## Resolved coop expiry admission

`downed_expiry(target)` binds the caller's Entity generation and current DownedEpisode
in a private DownedExpiry. Assembly must obtain it from that target's Vitality and
retire it with the component's other outstanding inputs before replacement.
It records identity, not a timer/deadline. `ExpireDowned` enters the same queue as
damage/revival; direct `expire_downed` has the same validated transition contract.
Caller admits ordinary multiplayer coop expiry timing. Current solo/last-player
policy rejects UnsupportedPlayerPolicy without mutation, leaving session disposition
to a separate boundary. No 45-second clock, guessed tick count or automatic retry exists.

An active valid revive attempt returns `Deferred(expiry)` without mutation. Producer
may retain that exact input and retry after cancellation using a new globally
increasing request ID. Cancellation does not reset/rebind expiry or episode.
Completion makes old expiry StaleEpisode immediately, including before redown;
redown also rejects the older episode. The producer retires expiry after successful
revival, terminal transition, disconnect or component retirement.

Without an active attempt, admitted expiry sets HumanTerminal and preserves numeric
current/max. Spectator/session assembly is above shared gameplay and remains absent.
Terminal human damage is Ignored; the same terminal episode's expiry is Ignored;
revival operations reject NotDowned. Wrong expiry target returns WrongTarget,
old episode returns StaleEpisode, actor returns NotHuman, absent/despawned target
returns MissingTarget. All rejection/deferral paths preserve the component.
Expiry uses existing counters without increment/wrapping or resetting them.

Source sends bled_out before a network-frame wait and cleanup. This increment
collapses that interval into caller-resolved terminal admission: explicit bounded
APPROXIMATE behavior, not a measured VM race policy. Queue order governs completion,
cancel and expiry with no hidden death priority. Expiry before an active completion
defers and completion can succeed; completion before expiry makes it stale;
cancel then expiry terminates and a delayed completion rejects.

Counters/tokens apply to a **continuous Vitality component lifetime**. Removing or
replacing Vitality on the same living Entity requires assembly to retire pending
commands, held tokens/episode references and old result receipts first. Bevy Entity
generation does not change on component replacement, so it cannot prevent those
counter-reset aliases. `retire_pending_target` returns pending commands for explicit
retirement while preserving other targets and request-ID history; held inputs remain
caller-owned. This is an admission boundary, not automatic lifecycle interception.

## Fixed ordering and demonstration

LifeCommandQueue replaces the damage-only queue. All Damage/Begin/Cancel/Complete/Expire
commands share strictly increasing caller IDs across fixed steps and execute in
that accepted order on FixedUpdate, via direct entity lookup. Duplicate/reordered
IDs reject before enqueue. u64::MAX is a valid final request ID; there is no wrapping
counter. ApplyLifeCommands lets producers/consumers define before/after ordering;
multiple producers must specify mutual order. No query iteration order, hidden
revive/death priority or original simultaneous-event policy is assumed.

Bevy0.19.1 owns ECS/entities/fixed scheduling. No original simulation rate is selected.
Identical input order at simulation boundaries gives identical results despite
application/render pacing. Queue/results persist until caller consumption; no
capacity/drop policy is selected. Current results must be consumed/retired before
component replacement. No parallel queue or general replay framework is added.

Run `cargo run -p bo2-damage-life-state-demo --locked`. It explicitly selects initial
health/teammate admission, then verifies21 exact ordered outcomes: nonlethal/down,
begin/cancel, stale completion, restart, revived100/100, subsequent damage/redown,
stale episode rejection, actor nonlethal/death/repeat, stale expiry, active expiry
deferral, cancellation/terminal expiry, rejected completion and terminal damage/expiry
repeats. It asserts intermediate restored Standing health and final human terminal
health1/max100 plus actor death before exit0. No original assets are read.

Tests preserve damage cases and exercise success, cancel/restart, invalid/stale
identities, missing/despawned/nonhuman targets/revivers, invalid restoration,
u32/u64 boundaries, atomic counter exhaustion, component-input retirement and
repeated mixed lifecycle replay with0/1/3 fixed steps per update. Expiry tests cover
active deferral, cancel/retry, order permutations, stale/wrong/missing identity,
positive terminal health and counter/numeric maxima. These are synthetic correctness
and engine integration. Raw revival interaction,3-second sampling, bleedout timing,
spectator/respawn, regen, perks, solo/session-end and coop comparison remain next tasks.
