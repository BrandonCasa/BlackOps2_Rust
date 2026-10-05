# Ordinary coop bleedout expiry and terminal disposition

Recorded 2026-10-05. Investigation only; fidelity **INVESTIGATING**. The existing
implementation remains APPROXIMATE. No multiplayer execution/comparison occurred.
User reports bleedout requires more than one player; this is an admission constraint,
not a measured solo or multiplayer result. Admit ordinary Origins coop with another
eligible teammate, no perks, persistent bonuses, special revival devices, underwater,
robot stomp, out-of-bounds kill, suicide, reset, intermission or session termination.

## Source identities and reproduction

Licensed Steam app212910 build24784239 compiled sources are distinct from observed
Plutonium r5354 native code. Loaded runtime script equivalence is unverified.
Reuse [task01](damage-life-state-01.md) external buffers and ignored local asset
configuration. No new extraction or original write was needed.

| Selected compiled buffer | SHA-256 |
| --- | --- |
| patch_zm:maps/mp/zombies/_zm.gsc | ae9fa4952fbd43353a91eb75236d403ac0454163460670dacb88d00456391ddf |
| patch_zm:maps/mp/zombies/_zm_laststand.gsc | 7baaa3c9ba431c06cba0db8f6ea0cbc5c9284b560051338c21125287f37d3077 |

Both were freshly decompiled and disassembled with gsc-tool1.4.10.333 (tool hash
3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec).
From external task06 directory run `gsc-tool -m decomp -g t6 -s pc <buffer>` and
`gsc-tool -m disasm -g t6 -s pc <buffer>` for each. Inspect named functions below;
opcode output corroborates literals, field changes, waits and call/notify order.
This is two tool modes over the same input, not independent VM execution.
Reused Origins `zm_tomb`/standard-mode evidence from task01/task03 provides map
context; no bleedout-duration/getup/zombify override was found in those selected
decoded files. This bounded negative search does not prove absence in all scripts.

External `damage-life-state-06/identity-receipt.json` records two compiled lengths,
input/output hashes and unchanged checks. Original patch_zm.ff, zm_tomb_patch.ff
and t6zm.exe rehashes match task01/task04 and remain unchanged. Their SHA-256s are
respectively f5a27345a61de5fabd735f45f05b127e95ad9a488619bd7fbc96ac14c5ab4d47,
770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca and
f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e.
Scripts, decoded instructions, inspection helpers and machine paths remain external.

## Direct script observations

| Locator | Abstract observation |
| --- | --- |
| _zm init / init_dvars | Initialization calls dvar setup, which sets player_lastStandBleedoutTime to45. This is a shared Zombies source default, not a runtime dvar measurement or immutable universal constant. |
| _zm_laststand init / playerlaststand | Getup mode defaults disabled. Ordinary downing assigns current health1, laststand/ignore flags and creates revive trigger before starting bleedout thread. Delay is sampled once through getdvarfloat; later dvar changes are not reread by this thread. |
| laststand_bleedout | Terminates on player_revived, player_suicide, zombified or disconnect notification. Special no-trigger/zombie paths bypass countdown and are excluded. Ordinary countdown stores delay, subtracts1 before each wait1second; visual midpoint uses integer half-delay. For unchanged delay45 there are45 one-second waits before expiry admission. Displayed counter reaches0 before the final one-second wait finishes. VM scheduling/network drift is unmeasured. |
| laststand_bleedout | Countdown runs while revival is active. Only after countdown ends does it poll every0.1seconds while trigger exists and beingrevived equals1. No pause, extension of countdown or fresh45-second restart appears here. |
| revive_do_revive / checkforbleedout | Attempt clears beingrevived after success/failure; failure calls stoprevive and starts a statistics-only watcher. It does not reset bleedout_time. Restart resets the revive-progress accumulator, not the bleedout countdown. Pending expiry can continue after cancellation on its next poll. |
| laststand_bleedout / bleed_out | After deferral ends: target bled_out notification, wait_network_frame, trigger deletion/undefined and client laststand mode clearing, death bookkeeping/equipment removal, level bleed_out notification, native undolaststand, then mode callback thread. No direct health0 assignment occurs in this script path. |
| _zm init_levelvars / init_function_overrides / zombify_player | Shared Zombies mode sets is_zombie_level, installs zombify_player callback and defaults zombify_player configuration0. Under that admitted ordinary value the callback threads spawnspectator rather than playable-zombie conversion. Function/flag names alone would incorrectly suggest zombie conversion. |
| _zm spawnspectator | Outside excluded intermission/exit routes: sends spawned/end_respawn, marks is_zombie and sends zombified, clears any revive trigger, sets sessionstate spectator and spectatorclient-1, assigns maxhealth from current health, sets spectator permissions, calls native spawn and finally notifies spawned_spectator. is_zombie here is also used for spectator classification; it is not proof of an AI/zombie actor. |
| player_is_in_laststand / can_revive | Ordinary downed predicate is trigger existence. Trigger removal makes target no longer revivable. Standing/alive is not established merely by a cleared laststand flag or positive numeric health. |

There is a real notification-to-cleanup interval. The bleedout thread still has
player_revived endon during the network-frame wait; the trigger thinker does not
itself endon bled_out. A revive completion in that interval could cancel pending
cleanup according to notification scheduling. Actual VM same-frame ordering is
unqualified. Do not assert unconditional expiry priority or atomic spectator entry
at countdown0. The native/engine `isalive` predicate after spectator spawn was not
measured and must not be replaced with a simple health>0 interpretation.

## Narrow native qualification

Revalidated existing process42012 creation2026-10-05T21:06:08.9703421Z, executable
identity from [task04](damage-life-state-04.md), mapped PE base0x400000,
timestamp1379794238 and header SHA-256
d203ed499e46f35abf308fd88c4069357a86b1d654d4a6b87b236cf2e152b161.
Parent separately qualifies its UI as r5354; this worker sends no UI input.
OpenProcess rights0x1010 allow query/read only. Rediscover exact undolaststand
name and24-byte registration in bounded .rdata/first4MiB .data scans, validate
argument bounds and function within mapped .text, then query executable image page.
No Steam RVA assumption, injection, write, console or builtin execution.

Observed registration: nameRVA0x7e76b0, entryRVA0x868ba8, methodID0x7e3,
arguments0/0, functionRVA0x424880. Exact145-byte function through return has SHA-256
9ec6ba9eeeafe7bc606af800d30c50fcca7c1e54d6ac19dc4796b6530b0cd28b;
repeat bytes/entry agree, PAGE_EXECUTE_READ0x20/MEM_IMAGE0x1000000.
It clears several client/internal fields and has **no direct store to the health
fields qualified in task04**. Its valid-player path calls a small client-structure
locator twice; that locator computes a client-derived address after another
unnamed call. A bounded128-byte helper span is retained externally, not a full
call-graph qualification. Internal fields/unnamed helper effects remain unnamed.
Thus the native observation does not establish numerical health after the whole
spectator transition. Later native spawn is also unqualified. Neither source nor
this inspection supports hardcoding terminal current0/max100.

External native-undo.py reads/emits only the bounded spans/registration metadata
described above. native-receipt.json and process-identity.json record identities.
Synthetic entry checks cover valid/truncated/wrong-name-pointer/out-of-range and
inverted argument bounds. Helper span is an exploratory locator, not admitted
complete native-effect proof. No new dependencies; reuse task04 cached Capstone.

## Minimal implementation recommendation

Add one resolved ordinary-coop `Downed -> HumanTerminal` transition to existing
Vitality/LifeCommandQueue, qualified as no-longer-revivable awaiting spectator
assembly. Keep numeric health independent: preserve the last qualified current/max
unless a later measured/native-qualified spectator health rule supplies another
value. A positive stored health must not make terminal state damageable/revivable.
Represent spectator/session disposition above this shared component; do not add
an AI zombie, respawn model, custom registry or duplicate event queue.

Expiry input minimally binds Entity generation and current DownedEpisode, plus
strictly ordered requestID and externally admitted coop expiry condition. Active
revive defers terminal application; cancellation leaves the episode and elapsed
deadline intact. A producer retains pending expiry while deferred, retries after
cancel and retires it on successful revival/terminal/disconnect/component retirement.
It must not relabel a stale expiry as a fresh episode. Completed terminal transition
invalidates active/held revival tokens and rejects repeats normally without mutation.
Same-Entity component replacement keeps the existing caller input-retirement contract.

Separate timer/deferral admission from terminal mutation for the next increment.
Origins assembly can later supply source-default 45 seconds, with externally sampled
delay and comparison gap; do not turn source waits into a guessed tick count. If
collapsing the network-frame interval into one resolved command, document that as
bounded approximation. Preserve accepted queue order for completion/cancel/expiry;
there is no evidence for hidden death priority. Test stale episode, cancel-after-
deadline, deferred active attempt, complete-before-expiry versus expiry-before-
complete, terminal repeats and positive-health terminal state.

## Comparison needed and validation

Not executed: launch through desktop Plutonium, record runtime/loaded scripts and
ordinary same-team multiplayer with more than one player and another standing
teammate. Keep all admitted exclusions absent; solo provides no bleedout proof.
Across at least3 fresh downs video-time onset to loss of revival/spectator entry:
(1) no attempt, (2) begin near42seconds and complete after nominal expiry,
(3) begin before expiry then release after45seconds, (4) interrupted early attempt
then no restart. Distinguish countdown,0.1second sampling and network-frame cleanup.
Do not demand exact45seconds from HUD/video or infer health from hit counts.
A separately qualified read-only observer is required to sample current/max,
sessionstate and native alive predicate around cleanup/spawn, before selecting
terminal numeric health. No runtime mod/cheat substitutes for ordinary evidence.

Fresh 2 decompilations / 2 disassemblies succeed; source preservation and receipt
hashes pass. Native name/entry/executable span/repeat checks and synthetic malformed
entry checks pass. No code changes, so Cargo gates were not rerun. Permitted-content
and git diff whitespace review pass. Unresolved: loaded-runtime source equivalence,
coop timing/race comparisons, unnamed native effects and spectator-spawn health.
