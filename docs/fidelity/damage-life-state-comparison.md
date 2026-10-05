# Damage and life-state comparison

## Ordinary actor terminal outcome: initial attempt

2026-10-05, observed Plutonium T6 Zombies r5354 LAN, Origins solo, fresh round 1.
Comparison target: a living ordinary zombie reaches terminal death after player
attack and does not resume living attack behavior. Rust reference is the reviewed
actor route at ca51fa5, with synthetic resolved damage and terminal-repeat tests.
Weapon damage, initial actor health and animation are not qualified by this case.

The existing launcher-created Origins lobby was reused after the appearance chat
released UI ownership. Computer-use selected its returned game window, activated
it, started the match with Return and observed the spawn scene: score 500, ammo
8/32. Escape exposed the pause menu; keyboard navigation worked. Mouse clicks
did not produce the requested menu/tab selection, so combat bindings were not
established from those clicks. No setting change was intentionally made.

After returning to play, a candidate V input produced a melee-like view/weapon
animation. A later observation, with no intervening attack/movement input issued
by this chat, showed a changed camera, score 960 and ammo 0/16. The cause of these
changes is unresolved. Another input source or retained input is a hypothesis;
neither is established. Ordinary zombies were visible, but no controlled
living-to-dead transition was observed and attributed.

Input stopped; the user and collision chat were asked about session control.
There was no console command, timescale change, injection or original-file write
by this attempt. Pause state and uncontrolled wall time prevent timing claims.
Runtime script overrides, perks/persistent bonuses and external input conditions
were not independently qualified. Screenshots remain in external tool history;
no original pixels are tracked here.

Result: **PARTIAL / INVESTIGATING**. This is access/input qualification and an
uncontrolled attempt, not a successful original/Rust comparison. Numerical health,
damage, death timing, player downing and coop revival remain unmeasured. Resume a
controlled trial only after input/session ownership is resolved; verify attack
binding and baseline speed before collecting repeatable outcomes.

## Session control and solo follow-up

The user subsequently granted this chat control and specified that ordinary
bleedout requires more than one player; solo cannot supply that comparison.
Two fresh Origins solo starts were observed in the same r5354 LAN window,
each lobby showing one player and maximum one. Spawn observations showed round 1,
score 500 and ammunition 8/32, with no purchase or combat input by this chat.
The first later observation showed a result scoreboard with zero kills and one
down. The second showed GAME OVER / survived one round, followed by the same
zero-kill, one-down result. Attack contact and the exact lethal instant were not
captured. These are solo terminal-outcome observations, not a numerical damage
comparison, a controlled actor death or evidence for cooperative bleedout.

During the second trial, console commands requested timescale 0.5 using both
the user-suggested prefixed form and the direct form. Subsequent console
completion displayed current/default timescale 1; a successful slowdown was
not established. The direct timescale 1 command was submitted and completion
again displayed current/default 1. No wall-clock/game-clock conversion or timing
claim is admitted. Console closed; the session was left at the Origins lobby.
No scripts were injected, process memory written or original files modified.

Session ownership is now resolved. Cooperative comparison still requires a
multiplayer setup with an eligible second player. Source/native investigation
continues independently; it does not substitute for live cooperative observation.

## Online damage-value cross-check

At the user's request, searched BO2/Origins health and damage on 2026-10-05.
The [BO2-Reimagined author's change notes](https://github.com/Jbleezy/BO2-Reimagined)
describe changing ordinary player health from 100 to 150 and zombie damage to
players from 60 to 50. These baseline claims corroborate the selected local
script evidence for no-bonus health 100 and actor melee field 60. The mod's changed
values are not Origins configuration, and its notes are not a live comparison.

Community results commonly propose first-round zombie health 150, normal knife
damage 150, health growth of 100 per round through round 9, then a 1.1 multiplier.
These are investigative leads only: search results mix BO1, BO2, BO3 and mods,
and some player-damage claims conflict at 50 versus 60. Licensed selected spawn,
weapon and numeric conversion evidence is required before adopting these additional
constants. No HUD numeric damage display is assumed. Task08 investigates a bounded
read-only hidden-health observer instead of inferring damage from score or hit count.

## Numeric observer trial 01

The qualified external [observer](../reverse_engineering/damage-life-state-08.md)
ran for 1,000 samples during a fresh r5354 solo round-1 match, starting paused.
The outer agent resumed through Escape and issued no purchase, combat, console or
timescale input. The finite process exited 0 after 55.743646 seconds: 966 accepted
samples all reported current/entity-max/client-max 100/100/100; 34 records refused
unstable snapshots between approximately 21.145 and 55.354 seconds. Later UI
observation showed the result scoreboard with zero kills and one down. The hit
and terminal instant were not captured or synchronized with numeric reads.

Result: **PARTIAL / INVESTIGATING**. No 100-to-40 damage transition was admitted.
Constant accepted baseline values do not prove damage sensitivity or contradict
the source-backed value 60. The original refusal records lack changed-field
detail; that information cannot be recovered. The external observer now reports
changed field names without raw values or weakened checks. Full flag-word
comparison may reject irrelevant flag changes; another controlled fresh-state
trial must resolve that possibility. Post-trial invalid local-state pointers were
safely refused without health. No original files or process memory were written.

## Numeric comparison: entity and client mirror trial

The next [qualified finite trial](../reverse_engineering/damage-life-state-13.md)
used fresh r5354 LAN Origins solo, Nikolai, round 1, score 500 and ammo 8/32.
The outer agent resumed from the paused baseline and issued no further gameplay
input. UI snapshots showed ordinary zombies climbing the right spawn-room window,
then GAME OVER / survived one round with the player prostrate, nearby zombies and
score 470. Exact attack contacts and lethal timing were not captured or aligned.

Of 1,000 records, 957 were accepted: 914 showed entity current/client mirror
100/100 and 43 showed 40/40. Both maxima remained 100. The other 43 records refused
changing local-state pointers and emitted no health. Samples 943 to 944 changed
100 to 40; 944 through 986 retained 40; 987 returned to 100. Return-to-100 cause is
unqualified and is not admitted as regeneration, revival or terminal health.

Outer review independently counted these groups and reran the current Rust demo:
request 0 applies source-backed resolved damage 60 at health 100 and asserts
`Hurt { before: 100, after: 40 }`; all 21 demo outcomes pass, exit 0. The original
numeric decrease of 60 agrees with that nonlethal Rust case. This is one scoped
numeric comparison, with live damage sensitivity now demonstrated in both fields.
It does not prove a single isolated melee contact, active builtin dispatch,
full raw-damage policy or lethal/down/revive/bleedout agreement. Fidelity remains
**APPROXIMATE / INVESTIGATING**; repeat an isolated-hit comparison before claiming
MATCHED. Original data and numeric receipts remain external.

## Actor comparison preparation follow-up

A subsequent fresh solo match was paused at spawn to inspect attack bindings.
Keyboard navigation reached Options/Controls/Look, but Combat-tab coordinate clicks,
Right, Tab and Ctrl+Tab did not expose combat bindings. No binding was intentionally
changed; no combat input or kill was admitted. Escape returned to the visible pause
menu. This supplies no actor death comparison. Qualify attack input before resuming
that controlled case; native actor findings are separately recorded in
[task14](../reverse_engineering/damage-life-state-14.md).

## Selected-actor trial

The [task17 sampler](../reverse_engineering/damage-life-state-17.md) first qualified
five paused entity25 samples at current/max 150/150, gate 1. The parent then started
one finite trial, resumed with Escape and issued one V input from the saved melee
binding. The resume screenshot showed Nikolai score 500, facing the spawn workbench;
the subsequent screenshot showed GAME OVER / survived one round, score 470. No melee
animation, contact, actor kill or visual association with entity25 was established.

Trial samples 0 through 317 all accepted 150/150, gate 1. Sample 318 refused
`selected_identity_changed` without health and ended recording. Final native profile
qualification passed; total reads 41,828 bytes. The shell session exited 1; the
sampler's refusal path specifies exit 2. The refusal records no changed identity
field names, so its particular cause cannot be recovered. Do not infer actor death,
pool reuse or benign flag changes from that generic reason. No selected-actor damage
sensitivity or terminal-health agreement was demonstrated. Original receipts remain
external. Next reset the scenario earlier and expose refusal field names before a
controlled repeat; preserve exact identity checks until evidence supports changing them.

The diagnostic repeat used fresh solo Richtofen, score 500, ammo 8/32, no purchases
or objective activation, paused on the first observed playable frame. Five baseline
samples again qualified 150/150, gate 1. Parent started session30051, resumed, saw a
zombie behind the central spawn window, then attempted an aiming drag. The drag API
does not support the supplied mouse-button option; the action sharply changed the
view and ammo became 7/32. Parent immediately returned to the pause menu. No hit,
melee contact or actor kill was established.

Samples 0 through 318 accepted unchanged 150/150, gate 1; sample 319 ended with
`selected_identity_changed:active`, without health. All other pinned identity
comparisons had passed for that sample. Final profile qualification passed, total
reads 41,918 bytes; shell session exit 1. This identifies full active-byte variation
as this repeat's refusal, not the historical trial's cause or an actor-death proof.
The admitted active bit remained set, because the snapshot's active-bit check had
passed. Other flag semantics remain unqualified; no admission check was relaxed.
Task18 external trial SHA-256 is
3e956d93c9d90bcff6ff27c240abaf259dea37e299e13fa1da787bb4306c5294.
Neither trial supplies selected-actor damage sensitivity or final-health evidence.

Task21 qualifies native active-bit admission and preserves other exact identity
checks. Parent fresh solo Nikolai500/ammo8/32 baseline accepts five150/150/gate1
samples. Finite session21299 exits0 with959 accepted unchanged samples,58s cap,
final native/process qualification and99428bytes read. No refusal occurs. Two
keyboard shots (ammo8->7->6), one stance input, visible central-window zombie and
later GAMEOVER1round/470 establish no selected-actor hit or death. Score stayed500
on shots; scoreboard kills0/downs1. Thus health sensitivity/final-health comparison
remain incomplete. See [Task21 receipts and limits](../reverse_engineering/damage-life-state-21.md).
Keyboard attack input works without mouse rotation; temporary binding restored.

A separate fresh Nikolai solo repeat (Task21 trial02) used only two W movement
inputs, pause/resume and no attack input. Paused baseline five150/150/gate1 samples
qualified. Finite session17308 ended terminal shell exit1, sampler identity-refusal
path2; final qualification passed,81096bytes. Samples0..740 accepted150/150/gate1;
741 refused unstable current/gate without health;742..754 accepted-999/150/gate0;
755 refused selected_team_changed and stopped. Other pinned identities remained
admitted for the signed-health samples. Original subsequently showed GAMEOVER,
one round/470. No actor hit, ordinary combat death or exact event-time alignment
was established. Session cleanup is a plausible cause, not a traced route.
This demonstrates signed terminal observation and conservative refusal; it does
not supply ordinary-death final health. SHA-256:
f7ec14a435a417de6e3fe0ee31d0f18b99b244f68587bba09bd134bd7ba92f87.
Passive captures appeared unchanged until a subsequent input revealed game-over;
refresh foreground before future observations and do not infer precise timing
from these screenshots. Original now at solo lobby; bindings unchanged, no jobs remain.
