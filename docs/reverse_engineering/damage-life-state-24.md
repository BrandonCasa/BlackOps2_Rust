# Ordinary solo and last-eligible lethal disposition

## Provenance and reproduction

Read-only investigation of Steam build 24784239 server scripts, using Task01's
existing compiled buffers and gsc-tool decompiled/disassembled outputs. Recheck
their lengths/SHA-256 against Task01's identity receipt; Task24's external receipt
records 10 matching artifacts. No extraction, runtime read or game input occurred.
These scripts are not proof of loaded Plutonium r5354 equivalence.

Relevant locators: `_zm::player_damage_override`, `player_laststand`,
`wait_and_revive`, `end_game`, `intermission`; `_zm_laststand::playerlaststand`,
`player_is_in_laststand`, `revive_trigger_think`; `zm_tomb::zm_player_fake_death`.
Task01 records full hashes. Selected compiled SHA-256:

- `_zm`: ae9fa4952fbd43353a91eb75236d403ac0454163460670dacb88d00456391ddf
- `_zm_laststand`: 7baaa3c9ba431c06cba0db8f6ea0cbc5c9284b560051338c21125287f37d3077
- `zm_tomb`: 9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6

## Direct source observations

After earlier raw-impact/perk exclusions, damage strictly below current health
returns the resolved amount. Equality enters lethal policy. The player list is
counted with self, zombie players, trigger-defined downed players and spectators
excluded from remaining candidates. Any remaining candidate, or a game-module
veto of ending, returns ordinary damage. This is not literal player count or the
full facing/touch/team predicate used for an actual revive interaction.

For one listed player with the solo flag, Quick Revive requires both positive
lives and the perk. That admitted recovery route returns damage and subsequently
starts the separate automatic-revive path. No-perk or exhausted-lives solo instead
sets **player** intermission before threading shared `playerlaststand`. The latter
synchronously calls the installed `_zm::player_laststand` callback before assigning
health 1, script laststand/ignore flags and creating a revive trigger. That callback
sees player intermission, waits 0.5 s and then waits for a level notification named
`forever`; those later assignments are consequently not reached while this wait
persists. Disassembly corroborates the synchronous callback and wait ordering.
Do not represent this solo path as an ordinary completed Downed transition.

Last-eligible multiplayer instead threads laststand without that early player
intermission assignment. It can reach health 1, flags, trigger and bleedout setup.
Both fatal routes then invoke fake-damage presentation and the Origins fake-death
thread, followed by `pre_end_game`, a network-frame wait, `end_game`, and return 0.
These are separate threads: their complete cross-thread execution order is not
established by textual ordering. Origins fake death marks ignore/invulnerability,
removes weapons and freezes controls/falls down; it contains no health assignment.
No-end-game and custom-module policies remain excluded.

The session-end worker clears client laststand presentation, sets **level**
intermission and eventually calls `intermission`. That function directly assigns
each player's current-health field 100, before threading custom presentation;
disassembly confirms a field store, not setmaxhealth or the revive health reboot.
No maximum assignment accompanies it. Revive-trigger thinking terminates on
end_game, disabling its running interaction/completion flow. This is not proof
that every trigger or bleedout worker is synchronously removed: the countdown
does not itself declare an end_game termination condition.

## Boundary and evidence limits

Recommend caller-resolved session admission distinguishing ordinary coop down,
solo without available Quick Revive, and last-eligible multiplayer session loss.
Session ownership should close revival/timing admission and emit an explicit loss
disposition; preserve player numeric health separately from terminal eligibility.
Do not fold the later intermission current 100 into revival, guess terminal zero,
or silently assign death priority. Solo and multiplayer loss cannot share an
unqualified health-1 rule. Source supports this split before implementation;
native fake-damage effects and thread/network-frame timing remain unresolved.

Next comparison: separately observe unperked solo and multiplayer last-eligible
loss, recording health, trigger/client state and session phase. User-reported
solo bleedout exclusion is not multiplayer measurement. A narrow native candidate
is registered `fakedamagefrom` if immediate numeric effects must be resolved.
Existing Task13 health returning to 100 is consistent with this script assignment,
but unaligned samples do not prove that causal path. Fidelity: INVESTIGATING.
