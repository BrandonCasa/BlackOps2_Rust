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
