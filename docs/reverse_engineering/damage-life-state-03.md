# Ordinary coop revive completion

Recorded 2026-10-05. Question: how does an ordinary Origins human transition
from downed to standing after teammate revive? Script routing/maximum restoration
are resolved; revived **current health is not established** because native
`reviveplayer` owns that effect. Evidence is INVESTIGATING, not live fidelity.
Existing damage implementation remains APPROXIMATE.

## Sources and reproduction

Reuse the licensed compiled buffers retained by [task01](damage-life-state-01.md),
Steam Zombies app212910 build24784239. Rehashing original zone/all/patch_zm.ff and
zone/all/zm_tomb_patch.ff agrees with task01 SHA-256
f5a27345a61de5fabd735f45f05b127e95ad9a488619bd7fbc96ac14c5ab4d47 and
770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca.
No original was written or newly dumped. Five selected compiled buffers were
re-decompiled with the same official gsc-tool1.4.10.333, tool SHA-256
3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec.

| Buffer source | Selected relative script | SHA-256 |
| --- | --- | --- |
| patch_zm | maps/mp/zombies/_zm_laststand.gsc | 7baaa3c9ba431c06cba0db8f6ea0cbc5c9284b560051338c21125287f37d3077 |
| patch_zm | maps/mp/zombies/_zm_playerhealth.gsc | 521cafab0838d0b4463bb668fa135c6b24f8bb03bb89c58eaab43aba086a2d85 |
| patch_zm | maps/mp/zombies/_zm_perks.gsc | 1a0e5f410486d8a34ea297a8c9cef0eaa22ce5ca01ba5a7df828f2850777ebc0 |
| zm_tomb_patch | maps/mp/zm_tomb.gsc | 9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6 |
| zm_tomb_patch | maps/mp/zm_tomb_giant_robot.gsc | 9f815424d2de4aa21cc94ecd1629490d55598b5e633c4759c9c3f92b55768a31 |

From an external task03 directory, run `gsc-tool -m decomp -g t6 -s pc
<task01-compiled-script>` for each; disassemble _zm_laststand with `-m disasm`.
All five decompilations and the disassembly succeed. Keep copyrighted outputs
external under configured inspection root `damage-life-state-03/`; its
`identity-receipt.json` records selected inputs and output hashes/lengths.
No tracked inspector, executable, copyrighted text or machine path added.
Relevant shared _zm player-revive monitor was also read from task01 output;
it records/audio-notifies revival without restoring current health.

This qualifies Steam script identities only. The outer agent separately reports
Plutonium runtime r5354; this worker did not observe that UI or verify its script
load/patch behavior. A future Plutonium comparison must state runtime identity
and test conditions rather than assuming equivalence to Steam source.

## Abstract observed contract

Admission here means standard coop humans, same team, target currently downed,
ordinary linked revive trigger, no perks/persistent upgrades/powerup weapon,
no auto/remote/staff revive, no underwater override, robot-stomp, grief reset or
session end. Source functions below are locators, not reproduced script bodies.

| Source function | Observation |
| --- | --- |
| _zm_laststand player_is_in_laststand | Ordinary human downed status is determined by presence of revive trigger, not just the script laststand flag. |
| revive_trigger_spawn / think | Downed target owns trigger and thinker. Thinker polls every0.1 seconds and tries players in get_players order; self-revive is excluded. Held use plus can_revive starts an attempt; ordinary temporary revive tool must not already be current. No engine iteration-order guarantee recovered. |
| can_revive / is_facing | Same team, non-zombie reviver, no powerup weapon, touching trigger, facing planar dot>0.9, passing sight and bullet traces. Map/game-module policy may reject. These are interaction admission, not health components. |
| Origins initialization / giant robot override | Origins installs robot can-revive policy: a stomped target is rejected, otherwise allowed by that override. Ordinary non-stomped target uses shared checks. Origins revival watcher reacts to attempt-ending notification; that notification alone is not proof of success. |
| revive_do_revive | Ordinary base revivetime3 seconds; excluded quick-revive/persistent-revive modifiers shorten it. Timer starts0 and each loop waits0.05 seconds then adds0.05. Loop eligibility is sampled before waiting; after wait, reviver downed and auto-revive are checked before timer threshold. Thus strict continuously-held input at every instant and exact60 fixed steps are not proven. |
| revive_do_revive | Target beingrevived becomes1; native startrevive invoked. A new attempt begins from0, so observed failed attempts do not carry accumulated progress into restart. Failed attempt calls native stoprevive; beingrevived cleared0 and failure returned. |
| revive_trigger_think | Successful attempt dispatches target revive_success. Script laststand flag is cleared after the attempt even on failure; ordinary trigger still defines downed status. Clearing that flag must not alone set Standing in Rust. |
| revive_success | Player-revived notification precedes native reviveplayer and max-health reboot; later trigger deleted/undefined, weapons enabled and ignoreme cleared0. Notification is an intra-script signal, not an atomic already-restored state receipt. |
| _zm_perks health_reboot | Chooses max100 under admitted no-persistent-bonus condition. Called with current-health clamping disabled. No explicit current assignment in this completion path; setmaxhealth is native too. |
| _zm_playerhealth | Existing regeneration routine watches current/max health and damage. No special completed-revive current-health assignment located. Its native setnormalhealth route does not resolve native reviveplayer restoration. |

The decoded _zm_laststand opcode stream corroborates3,0.05, completion call order
and trigger/ignore changes. This is two modes of the same tool, not original VM
execution or independent decompiler proof.

## Cancellation and competition

The target's thinker terminates on target disconnect, zombification, stop-trigger,
death and level end-game notifications. Explicit cleanup watches target disconnect
and restores reviver equipment; another watcher balances active-attempt count.
Reviver becoming downed breaks the timed attempt before threshold success.
Releasing use or failing can_revive stops the loop at its next eligibility sample;
the final0.05-second interval can reach success without another eligibility check.
Do not infer an exact ordering for simultaneous release/completion/death from
these scripts. Native thread/notification/frame ordering remains unqualified.

Bleedout terminates when player-revived notification arrives. When its countdown
expires, it waits while trigger beingrevived==1, then notifies bled-out, waits a
network frame and executes terminal cleanup. Consequently an in-progress ordinary
revive can defer bleedout; a simple unconditional deadline with death priority
would contradict this source. No bleedout duration/value is selected here.
Failed attempt after expiry permits that pending route to continue once attempt
flag clears. Target death/trigger teardown must invalidate any delayed completion;
exact same-frame race remains a comparison question.

## Recommended integration and missing health evidence

Keep health restoration in a validated `Vitality` transition, separate from
interaction geometry/tool/HUD. A resolved completion needs target Bevy identity,
reviver identity, current downed-episode/attempt token, ordered completion command
and **qualified restored current/max values**. A stale completion from an older
downing episode must not revive a newly downed entity. Entity generation alone
does not distinguish two episodes on the same living player.

For a first completed-revive boundary, caller can resolve eligibility/attempt
timing and provide qualified restored health; do not embed3 seconds as a guessed
simulation tick count or set health100 silently. Alternatively explicitly mark
full-health restoration an approximation with its comparison gate. Future raw
interaction attempts need begin/cancel/complete and elapsed simulation time,
accepted eligibility samples, and pending terminal disposition. Producer ordering
must be explicit relative to damage/termination in Bevy FixedUpdate; choose any
unmeasured tie rule as implementation policy with a comparison gap, not original
fact. This task does not authorize a universal timer/interaction framework.

The concrete remaining access question is native reviveplayer/current health at
completion, not missing compiled scripts. Locate/qualify that native implementation
or measure read-only current/max values before/after a completed ordinary revive.
HUD/movement proof establishes standing but cannot distinguish immediate full
health from low health followed by regen. Damage subtraction from task02 remains
inferred; using Rust tests cannot qualify original restored health.

## Original comparison procedure (not executed)

Use the explicitly required desktop Plutonium launcher, record r5354 or actual
version, loaded map/mode and script overrides. Select Origins robot-head icon
with arrows. Ordinary same-team coop requires two human players and no admitted
exclusions; a solo lobby cannot establish teammate revive.

1. Down target with controlled ordinary attacks; record that target stays downed.
   Hold teammate revive without interruption. Video-record onset, completion,
   standing control and revive count across at least3 fresh attempts. Compare
   observed duration to source3-second accumulator with sampling/network delay
   distinguished; do not claim exact timer precision from HUD alone.
2. Release use well before completion (for example near1 second), then restart.
   Verify target remains downed and restart requires a fresh attempt. Separately
   leave touch/facing eligibility or down the reviver before completion. Compare
   cancellation outcomes; avoid expiry until the basic transition is established.
3. To resolve current health, use a separately qualified read-only observer of
   target health/max before, immediately after native completion and over regen.
   If only hit counts/video are available, distinguish hypotheses conservatively
   and retain exact numeric restoration UNKNOWN. Do not copy full memory dumps.
4. Later isolate expiry while beingrevived and cancellation-after-expiry. This is
   a separate adjacent bleedout comparison, not prerequisite for ordinary early
   completion or permission to implement guessed death timing.

No UI actions/live comparison were performed by this task. Git/content review,
script/output identities and document whitespace checks pass. Unchanged Cargo
code was not retested for these documentation-only deliverables.
