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
