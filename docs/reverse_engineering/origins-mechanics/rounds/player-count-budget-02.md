# Normal-round player-count budget

Recorded 2026-10-05. Question: **which configured operands, player-count branch,
round thresholds and conversion order determine the normal-round pending budget?**
The original scripts and numeric configuration establish the operation sequence
below. Exact VM division/conversion/float behavior and runtime patch precedence
remain unresolved; the numerical tables are explicitly conditional models, not
claimed original counts. Fidelity: **INVESTIGATING**.

## Source identity and reproduction

Steam Zombies manifest build **24784239**, English installation label from
[catalog](../../0001-local-asset-catalog.md). The previously accepted
[round lifecycle](normal-round-accounting.md) and [handoff01](../../../handoffs/mechanics-rounds-01.md)
remain the completion/accounting baseline. Current architecture has no production
round implementation; reusable simulation belongs below Origins assembly.

New original input: `zone/all/common_zm.ff`, 10,488,640 bytes, SHA-256
`b0663e765b50a17e3e7a53e2675ca18a1eeca186e0356df74e9b2c615f09ce9e`.
Freshly hashed retained compiled scripts:

| Identifier | Source zone | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| `maps/mp/zombies/_zm.gsc` | `patch_zm` | 105,301 | `ae9fa4952fbd43353a91eb75236d403ac0454163460670dacb88d00456391ddf` |
| `maps/mp/zombies/_zm_utility.gsc` | `patch_zm` | 74,393 | `485e791715a5660414238878ee1fa77facfe1b2612ff367d0b830b77a516e1b1` |
| `maps/mp/_utility.gsc` | `patch_zm` | 37,646 | `a077687f785996e28a4992608d9f4b1ad723789ffcf7d85a99b78354af2e4b1d` |
| `maps/mp/zombies/_zm_spawner.gsc` | `patch_zm` | 54,818 | `41ad5d02a62f5fc7a8b2c01a6b86dc1b4f184f3f029cced46b2e21bcda7f4488` |
| `maps/mp/zm_tomb.gsc` | `zm_tomb_patch` | 63,195 | `9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6` |
| `maps/mp/zm_tomb_gamemodes.gsc` | `zm_tomb_patch` | 546 | `9b3d2c310e8548161a7c0c70e607e807f581b9db0bced78e91ba28dc4301ca90` |
| `maps/mp/zm_tomb_classic.gsc` | `zm_tomb_patch` | 1,090 | `8e33b7d93197d1274c3186b1e73f8735055570e23c9e180226ba57a428124ef6` |

Retained script provenance is external `damage-life-state-01` dump logs/receipt;
zone hashes for these two script sources are recorded in the first round finding.
This task does not repeat their extraction. It does read the common zone directly
for the required numeric table.

Procedure:

1. Resolve roots from ignored local configuration and create external
   `inspection_output/mechanics-rounds-02`. Hash source/script identities above.
2. Existing common-zone listing identifies two stringtables. Run qualified OAT
   `v0.33.0` with `--no-color --include-assets stringtable -o <external table-dump>
   <common_zm.ff>`; no dependency preload, game process or other asset types.
   It emits two tables (2,943 and 905 bytes), plus a 56,010-byte zone asset-directory
   metadata file. All remain external; read only the two relevant numeric table rows.
   Log reports finished with 0 warnings/0 errors. The individual OAT process exit
   code was not separately retained by the shell sequence.
3. Decoded `mp/zombiemode.csv` SHA-256:
   `7f13accfa1cb87acc3c279c13bc36b06aca81228a2af63181a2bd6d8de3657be`.
   Its relevant operands are 24 and 6 across the four numeric columns. Do not copy
   the raw table or zone directory into tracked fixtures.
4. In external decompile/disasm working directories run
   `gsc-tool -m decomp -g t6 -s pc <one compiled script>` for `_zm`, `_zm_utility`,
   `_utility`, `zm_tomb_gamemodes`, `zm_tomb_classic`; disassemble `_zm` and
   `_zm_utility` with mode `disasm`. All seven conversions reported completed;
   reuse prior map/spawner views only for narrow capacity/replacement checks.
5. Keep views/logs and `identity-receipt.json` external. Track only this abstract
   record, [synthetic checks](budget_model_checks.py) and [handoff](../../../handoffs/mechanics-rounds-02.md).

Tool identities: OAT release revision
`7d027e8f89118196713e955b0e11f8404149c54d`, executable SHA-256
`9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63`
([qualification](../../0004-origins-asset-directory.md)); GSC Tool **1.4.10.333**,
SHA-256 `3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec`.
No decode failure observed. Generated disassembly and decompilation share a decoder;
agreement does not independently prove correct VM decoding. Original runtime was
not launched, injected into or controlled.

## Configured formula and instruction anchors

**O:** decoded original observation. **I:** inference. **H:** explicitly assumed
numerical model. **P:** proposed implementation boundary.

- **O, `_zm::init_levelvars`:** reads initial round from game-type setting;
  installs maximum-base and per-player values through `set_zombie_var` with integer
  conversion and column 2. **O, `_zm_utility::set_zombie_var`, disassembly
  `loc_92AF` through `loc_92FB`:** nonempty table lookup overrides fallback before
  assigning the shared variable. Observed original table values confirm **A=24,
  C=6** for that selected column; these are not merely fallback literals.
- **O, `zm_tomb`/`zm_tomb_gamemodes::init`/`zm_tomb_classic::main`:** default mode
  is `zclassic`, location `tomb`; registration selects classic map callbacks and
  standard game module. No override of budget/total-set callbacks appears in the
  inspected classic/map bodies. Metadata search over the retained map/patch script
  subsets found no additional named callback/operand writer. This is a bounded
  search, **not proof** against base-zone scripts, later patches or engine changes.
- **O, `_utility::get_players`:** wrapper returns native `getplayers()` unchanged.
  Budget samples its cardinality when spawning begins. Native roster eligibility
  (spectators/disconnected/downed players) remains unresolved; p is returned-array
  size, not an invented alive-player count. The ordinary spawn loop does not
  recalculate this formula for each admission.
- **O, `_zm::round_spawning`, disassembly `loc_DF0A` through `loc_DFA0`:** divide
  round by 5, clamp that multiplier to at least 1, and from round 10 multiply by
  the product of round and floating 0.15. Solo uses floating 0.5 times C; otherwise
  use `(p-1)` times C. Multiply by the multiplier, then apply VM `int`, then add A.
  **The division is a generic divide opcode with integer-looking operands, not
  independent proof of integer truncation.**
- **O, `default_max_zombie_func`, `loc_DD9B` through `loc_DE3F`:** below rounds
  2/3/4/5/6, respectively, apply early factors 0.25/0.3/0.5/0.7/0.9 to the entire
  intermediate maximum and convert with VM `int`. From round 6 return that
  intermediate maximum unchanged. Developer-block dvar override can return a
  configured count directly; normal model excludes it, but active block behavior
  must be verified before an original comparison.
- **O, round spawning `loc_DFB5` through `loc_DFEF`:** absent a custom maximum
  callback, install the default above. A defined kill-counter HUD with positive
  existing pending budget suppresses initialization; optional total-set callback
  runs afterward. Formula below assumes both exceptions absent.
- **O, `round_think`, cap immediately before `loc_F09D`:** round number is capped
  at 255 after increment. No pending-budget ceiling appears in the inspected
  default callback. **I:** concurrent AI capacity is separate; Origins' ordinary
  24 / classic split-screen 20 assignments do not clamp the full-round budget.

Abstract operation notation, preserving VM-dependent steps:

1. `m = max(1, VM_divide(r, 5))`; if `r >= 10`, multiply m by `r * 0.15`.
2. `k = 0.5` for p=1; otherwise `k = p-1`.
3. `v = A + VM_int(k * C * m)`.
4. `B = VM_int(v * early_factor(r))` for r=1..5; otherwise `B = v`.

Operands/counters are dimensionless spawn units. Formula-source is established;
exact returned budget additionally requires VM numeric semantics, selected
roster/configuration, callback state and active script load precedence. No evidence
permits silently substituting Rust integer division, f64, binary32 or floor/round.

## Conditional numerical models and checks

The project-owned [check script](budget_model_checks.py) uses **exact rational
arithmetic**, positive conversion by truncation, and two division candidates:
**H-I** floors r/5 before the clamp; **H-R** retains the rational quotient.
These are hypotheses used to choose discriminating comparisons. They are not
original runtime emulation, and neither establishes floating rounding near an
integer boundary. Columns list p=1,2,3,4 in that order.

| Round | H-I budgets | H-R budgets |
| ---: | --- | --- |
| 1 | 6, 7, 9, 10 | same |
| 2 | 8, 9, 10, 12 | same |
| 3 | 13, 15, 18, 21 | same |
| 4 | 18, 21, 25, 29 | same |
| 5 | 24, 27, 32, 37 | same |
| 6 | 27, 30, 36, 42 | 27, 31, 38, 45 |
| 9 | 27, 30, 36, 42 | 29, 34, 45, 56 |
| 10 | 33, 42, 60, 78 | same |
| 11 | 33, 43, 63, 83 | 34, 45, 67, 89 |
| 15 | 44, 64, 105, 145 | same |
| 254 | 5739, 11454, 22884, 34314 | 5830, 11636, 23249, 34862 |
| 255 | 5876, 11728, 23433, 35137 | same |

`python docs/reverse_engineering/origins-mechanics/rounds/budget_model_checks.py`:
**12 tests passed**. Table-driven early/threshold/counterexample cases, conversion
order counterexamples, monotonic selected-domain checks, capacity separation,
snapshot behavior, explicit model-domain errors and deterministic replacement
replay are exercised. Synthetic model input rejection is the fixture's selected
domain, **not a claimed original validation rule**. Cargo gates not run:
documentation plus standalone explanatory checks only.

## One extra-population boundary: replacement credit

**O:** shared initialization zeros `zombie_total_subtract`. Shared timeout and
Origins' installed timeout handler can increment pending budget and this separate
counter before removing an eligible timed-out enemy. Replacement enablement is
set true in shared initialization; Origins replaces the timeout callback.
**O:** `_zm_spawner` recycle branch credits both counters when an undamaged enemy
is marked for recycle. This is accounting-only evidence; damage mechanics are
excluded. Conditions/exclusions and actual runtime replacement remain unmeasured.

The named inspected script families show initialization/increments, but no read
of this counter in the ordinary budget formula. **I:** subtracting it from B would
invent a rule. **P:** keep initial budget, pending work, replacement credit and
admitted/counted entities distinct until all consumers are traced. A synthetic
initial budget 1 completes after one ordinary admission/removal; adding a
replacement-credit/removal and re-admission completes after **two** admissions
with the same initial budget and replacement ledger 1. Therefore counting every
spawned actor as a new original budget unit is not a valid comparison method.

Dog/mixed/ghost, capture/quest enemies and mech scheduling remain separate unknown
population policies. Generic dog code does not prove an Origins special round.
The first task's custom enemy-filter dispatch gap also remains unresolved. This
task adds no special-round algorithm or zombie controller.

## Proposed implementation and original comparison

**P:** configuration should supply A/C and an explicit evidenced budget policy;
simulation samples the relevant roster once, owns pending work, and admits entities
under separate capacity. Keep numeric conversion/order documented and testable;
do not promote either numerical model to production until VM/count evidence
selects it. Origins assembly supplies mode-specific configuration and hooks.

For original comparison, coordinate runtime ownership first; control build,
zclassic/tomb mode, 1..4 returned players, start-round setting, callback/dev/HUD
state and replacement/special enemies. Count ordinary admissions, replacements
and final counted removals separately. Round 6 with two players and solo round 9
are small discriminating cases; rounds 1..5 and 9/10/11 cover conversion and
threshold ordering. Avoid an impractical round-255 live run; validate its cap and
numeric boundaries with later VM/source evidence or supported controlled setup.
Record wall time and game time separately. Escape pause and `timescale 0.5` are
user-provided unverified leads; verify effects and restore/verify scale 1 for
baseline timing. No such controls were exercised here.

Unresolved: native roster membership, exact divide/int/float behavior, active
patch/zone precedence and runtime callbacks, full replacement-ledger consumers,
special population policies, and all original count comparisons. Source-data
finding is complete within scope; numerical/gameplay fidelity is not established.
