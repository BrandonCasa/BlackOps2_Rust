# Origins damage and lethal routing: first evidence

Recorded 2026-10-05. Question: which shared damage/life-state behavior can be
implemented for an unperked human player and an ordinary Origins zombie?
Result: licensed compiled script inspection establishes scalar configuration,
separate player/actor routes and player last-stand assignments. Native damage
application and live comparison remain unmeasured. Fidelity is INVESTIGATING.

## Identity and access

Steam Zombies app 212910 manifest build and TargetBuildID both read 24784239.
The following fastfiles were opened read-only; hashes agreed before/after access.
Relative identifiers are not local installation paths.

| Input | Bytes | SHA-256 |
| --- | ---: | --- |
| zone/all/common_zm.ff | 10488640 | b0663e765b50a17e3e7a53e2675ca18a1eeca186e0356df74e9b2c615f09ce9e |
| zone/all/patch_zm.ff | 3001792 | f5a27345a61de5fabd735f45f05b127e95ad9a488619bd7fbc96ac14c5ab4d47 |
| zone/all/zm_tomb_patch.ff | 1288384 | 770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca |

Existing OAT v0.33.0 Unlinker SHA-256 remained
9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63.
Its already available public source contains a T6 ScriptParseTree dumper which
writes the compiled buffer, not source text. Three LIST invocations exited 0.
Shared gameplay scripts were found in patch_zm; Origins overrides in zm_tomb_patch.
Two script-only dumps exited 0 with zero reported warnings/errors. They emit
compiled copyrighted buffers externally: 150 server GSC files in patch_zm and
93 in the Origins patch, plus client scripts and tool zone metadata. No dump
was committed or placed in this worktree. Full fastfile loading is required by
OAT even when output selects scripts; this is not a bounded 312-byte read.

Official [gsc-tool 1.4.10 release](https://github.com/xensik/gsc-tool/releases/tag/1.4.10)
Windows x64 archive length 6622894 and SHA-256
ef7d7c86e2356f3e4d2b75b53d28dabb4df8b2a804cf3f6be179273f0fcada20
matched GitHub release API metadata before execution. Archive contained one
gsc-tool.exe, length 19151872, SHA-256
3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec;
version reports 1.4.10.333. Tool/source remain external. No public decompiled
game scripts or online gameplay claims were used as original evidence.

Reproduce below the configured external inspection root in distinct
`damage-life-state-01/` (resolve roots only from ignored local configuration):

1. Hash the three inputs and pinned executable; read Steam manifest build.
2. Invoke `Unlinker --list --no-color <zone>` once per input, logs external.
3. Invoke `Unlinker --no-color --skip-obj --include-assets script
   --output-folder <external-zone-directory> <zone>` for the two patch inputs.
4. In the task directory invoke `gsc-tool -m decomp -g t6 -s pc <compiled-script>`
   separately for the listed script identities. Use `-m disasm` for corroboration.
5. Keep decompiled source/disassembly external; record abstract findings only.
   Rehash originals and tool. External `identity-receipt.json` identifies outputs;
   raw listings/dump logs, selected compiled/decompiled files and disassembly
   remain alongside it. No game process was running when checked; none launched.

## Observed script contract

These are observations of decoder output, not execution of the original VM.
Function identifiers below are precise locators in external decompiled output;
no original script body is reproduced.

| Source / function | Abstract observation and limits |
| --- | --- |
| Origins zm_tomb, main initialization | Calls shared Zombies initialization, then replaces actor damage with an Origins wrapper and registers a player damage callback. Thus shared routes are applicable but have map policy above them. Runtime patch order is not independently observed. |
| Shared _zm, callback initialization | Separate player damage, actor damage, player last-stand and actor-killed callbacks. Damage context carries amount, attacker/inflictor, cause, weapon, location and direction; a health scalar alone is insufficient to choose every policy. |
| _zm_perks, perk_set_max_health_if_jugg | Health reboot chooses max-health 100 before optional persistent-upgrade bonus. Normal unperked, no persistent bonus is the admitted condition; max-health assignment alone does not prove current-health spawn initialization. |
| _zm, zombie variable initialization | Shared health-start 150, linear increase 100 and multiplier 0.1 are configuration. Only 150 is needed for a fresh fallback spawn; do not assert every round-1 actor health until round initialization is inspected. |
| _zm_spawner, zombie spawn initialization | Sets zombie flag and allows death. Uses level zombie-health when defined; respawn may restore saved current health. Otherwise max/current health both use health-start. Ordinary melee amount assigned 60. |
| _zm, player_damage_override | Zero damage or an already last-standing player returns zero. Shield/water/attacker/custom callbacks can reject/replace damage. Ordinary zombie attacker replaces incoming amount with its melee amount; missing melee field falls back to 50. Same attacker is temporarily ignored (default removal delay 0.4 seconds); this is separate from health overlay timing. |
| _zm, player_damage_override | After modifiers/exclusions, amount strictly less than current health returns damage through the nonlethal branch. Equality enters lethal handling. Coop with another eligible player returns damage toward native application; sole/last eligible player involves last-stand, fake death and game-end policy. A generic universal player-death transition would erase this distinction. |
| _zm, callback_playerdamage | Friendly-player policy, override, prevent-damage check and native finishplayerdamage are separate stages. Original native subtraction/clamping, VM number conversion and callback timing are not recovered here. |
| _zm_laststand, playerlaststand | Guard prevents repeated entry. Sets current health to 1, last-stand and ignore flags. This is a downed player with positive health, not a zero-health dead actor. Revive trigger and bleedout are adjacent, not implemented or fully investigated. |
| _zm, actor_damage_override / wrapper | Actor custom policy returns integer final damage. Unless a special cannot-die flag blocks a lethal amount, wrapper calls native finishactordamage. This is not player last-stand routing. Native application remains opaque. |
| Origins zm_tomb, actor wrapper | Ordinary path delegates to shared actor wrapper. Zombie-blood-only, capture zombies, tank position and special weapon/cause cases add policy/effects. Exclude those for the first ordinary-zombie contract. |
| Origins zm_tomb, player callback | Staff/turret/tank weapon families reject player damage; otherwise return supplied amount. Ordinary zombie melee is not one of these exclusions. |

Compiled source identifiers inspected: patch_zm maps/mp/zombies/{_zm,
_zm_spawner,_zm_laststand,_zm_playerhealth,_zm_perks}.gsc,
maps/mp/gametypes_zm/{_globallogic_player,_globallogic_actor}.gsc, and Origins
maps/mp/{zm_tomb,gametypes_zm/zstandard}.gsc. The extra global callbacks establish
the generic/native boundary; Zombies-selected overrides determine this scope.

Selected compiled-buffer SHA-256:

| Script | SHA-256 |
| --- | --- |
| _zm | ae9fa4952fbd43353a91eb75236d403ac0454163460670dacb88d00456391ddf |
| _zm_spawner | 41ad5d02a62f5fc7a8b2c01a6b86dc1b4f184f3f029cced46b2e21bcda7f4488 |
| _zm_laststand | 7baaa3c9ba431c06cba0db8f6ea0cbc5c9284b560051338c21125287f37d3077 |
| _zm_playerhealth | 521cafab0838d0b4463bb668fa135c6b24f8bb03bb89c58eaab43aba086a2d85 |
| _zm_perks | 1a0e5f410486d8a34ea297a8c9cef0eaa22ce5ca01ba5a7df828f2850777ebc0 |
| zm_tomb | 9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6 |

## Recommended reusable boundary and supported subset

Shared gameplay should own validated health values and ordered resolved-damage
application; Zombies policy owns human last-stand versus actor death. Origins
assembles/configures that policy. Compose Bevy components and systems; do not
create player/zombie class inheritance or an entity registry. Preserve attacker
identity and damage cause before admitting raw impacts. Keep terminal actor death
distinct from positive-health player downing and map/session game-end decisions.

An explicitly scoped first implementation can accept already resolved positive
integer damage and demonstrate the script-derived player lethal classification,
downed health 1 and ordinary actor terminal route. Health subtraction to zero
for actors is a source-grounded inference about native finishactordamage, not a
proven original implementation. Label that gap; do not imply full impact fidelity.
Use this evidence to start the scoped foundation while qualifying the native
boundary; its absence does not require another broad asset/tool survey.
Positive integer input is an implementation admission restriction for players,
not proof that every original player path converts to integer before native
application. The actor override explicitly converts its ordinary final result
to integer; original player VM/native conversion remains unresolved. The actor
cannot-die guard uses strict damage-less-than-health: equality is observably
blocked when that flag is true, and admitted to the native function when false.
Equality actually producing actor death is still native/runtime inference.
Do not hardcode every player melee impact as 60: same-attacker filtering, shield,
regen/invulnerability and timing are outside the resolved-damage boundary.

## Comparison procedure and uncertainty

No live game comparison was executed. Proposed controlled tests: licensed build,
Origins standard coop with one second player still alive; no perks/persistent
bonus/shield, no powerups or special zombies. Record round and video timing.
Separate two ordinary-zombie attacks by more than the 0.4-second same-attacker
window but less than regen delay, then observe downing and revived state. Repeat
three fresh trials; include sole/last-player control to distinguish game end.
These downed/game-over outcomes are observable without process injection. A
second noninvasive case compares ordinary melee received by a standing player
and an already downed player while the teammate remains alive; control time to
avoid confusing scheduled bleedout with incoming-damage routing. A third uses
an ordinary fresh zombie and verifies death followed by no further living attack
behavior; it proves a terminal outcome but not the scalar damage amount.
Health boundary tests at h-1/h/h+1 require a separately qualified read-only live
health observer or controlled original test harness; hit counts alone do not prove
numeric equality, native subtraction or exact amount. For actor death, use a
qualified health observer and resolved-damage trials, controlling hit location,
weapon/perks/powerups and round. Do not treat a bullet count as resolved damage.

## Validation

All nine selected server scripts decompiled successfully. Four shared scripts
also disassembled successfully; the strict health comparison and last-stand
health/flag assignments agree with their opcode streams. This is corroboration
through two modes of the same tool, not independent original-VM verification.
A four-byte user-created truncated GSC was rejected with exit 1 and out-of-bounds
diagnostic, leaving no decompiled fixture output. No inspector was added.
Original fastfile hashes agree before/after; pinned executable identities agree.
No hard decoder memory bound or runtime patch-order measurement was performed.
Documentation-only deliverables do not require rerunning unchanged Cargo gates.
No native executable disassembly, original VM execution, live health read or
gameplay comparison was performed. Full fidelity remains INVESTIGATING.
