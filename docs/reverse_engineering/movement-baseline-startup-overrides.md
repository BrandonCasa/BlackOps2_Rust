# Unperked Origins baseline startup ledger

Movement-profile01 completes a bounded startup/override investigation, with six
native parameter values still unresolved. Fidelity is **INVESTIGATING**. This
ledger is not an executable baseline profile. It supplements
[native04](movement-baseline-parameters.md),
[Origins callback qualification](movement-native-contract.md),
[FP10](movement-floating-point-policy.md) and
[camera02](first-person-camera-contract.md), without replacing their limits.

## Source identities and reproducible reads

Native disk identity remains 13,090,072 bytes, SHA-256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
PE image base is `400000`; translate references through its section mapper.
The five retained compiled buffers below came from the previously admitted
Steam24784239 patch inputs. Their container identities and extraction procedure
are in [damage01](damage-life-state-01.md). This task reads existing buffers,
not a new broad extraction or original runtime. Distinct native/script build
identities are not treated as original comparison evidence.

| Compiled script identifier | Bytes | SHA-256 |
| --- | ---: | --- |
| `maps/mp/zm_tomb.gsc` | 63,195 | `9f8eefae125123362121849d0a4f88bae5d8a9add1352f3b3e52fd4883ae7de6` |
| `maps/mp/zombies/_zm.gsc` | 105,301 | `ae9fa4952fbd43353a91eb75236d403ac0454163460670dacb88d00456391ddf` |
| `maps/mp/zombies/_load.gsc` | 10,904 | `1c5746609a8e9145852ac66a4ad2e0eef0d56c31e304a1ef557ce69232d6ff19` |
| `maps/mp/zombies/_zm_ffotd.gsc` | 4,592 | `28482275f9abdbc75da538b22ca5c452c2bb44af92c8d72b5b3537b2e66b2e7d` |
| `maps/mp/zm_tomb_ffotd.gsc` | 6,914 | `2441b38f7b7b31d697c3f7f0813c24642f2e8b60734f8043af6da89ef536882b` |

Owned external `movement-profile-01/scripts.py` hashes each original before/after,
checks pinned gsc-tool1.4.10.333 SHA-256
`3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec`,
and invokes fresh decompilation and disassembly in its owned output directory.
Limits are five files, 1 MiB per compiled input, 4 MiB cumulative input,
30 seconds per invocation, 64 KiB captured tool diagnostic output, and
8 MiB per generated file. Actual total compiled input is 190,906 bytes;
all ten invocations succeed. Output size checks occur after production and
captured-output checks after subprocess completion; these are not hard OS
memory bounds. Originals and prior schemas/helpers are unchanged.
The script/output identity receipt SHA-256 is
`88fffaab202833de271620215e4313e83a77bd7afe7795a5814cc6ba6e7d9f15`.

`search.py` rehashes native disk before/after, searches six exact slot-address
encodings with at most128 matches each, and saves local instruction candidates
externally. A candidate beginning inside another instruction is not qualified
code or a writer. This search does not cover encoded/computed pointers or
runtime-populated tables. Slot report SHA-256 is
`f3defe6688b7892c9699ab2902dafbc124c486ed03ccc6e104a829df6a911e8b`.
The source-consumer assignments in the table below rely on prior complete
bodies, not these fresh candidate windows.

## Qualified Origins startup path and override limits

Fresh compiled call sequences corroborate Origins `zm_tomb::main` calling
`_load::main`, then shared `_zm::init`. Shared initialization calls
`init_dvars`, and starts `_zm_ffotd::main_start`; Origins starts its own
`zm_tomb_ffotd::main_start`. Disassembly locators are respectively lines616
in `zm_tomb.gscasm`,316/115 in `_zm.gscasm`, and169 in `zm_tomb.gscasm`.
These are reproducible decoded-output locators, not original VM execution.

All **37** decoded direct `setdvar` calls in the five selected files were
inspected with their literal argument windows: four in `_load`, ten in `_zm`,
23 in `zm_tomb`, none in either FFOTD file. Shared `init_dvars` concerns debug,
bots, cheat, movable chest, revive radius, bleedout and explosive cleanup inputs;
it does not supply any six requested movement parameter values. None of the
six candidate names `g_speed`, `bg_gravity`, `stopspeed`,
`player_backSpeedScale`, `player_strafeSpeedScale`,
`player_standingViewHeight` occurs in these five disassemblies. Names remain
search leads: native slot-to-name registration is unresolved, so this absence
cannot prove absence of an override under another name or hash.

Origins personality/team character callbacks again show scalar1 for movement
scale,4 for sprint duration,0 for cooldown, with native associations retained
in native01/native04. Their method calls and literal operands are freshly
corroborated at `zm_tomb.gscasm:2425-2435` and2706-2716. This establishes
configuration for those callbacks, not engine base speed or a persistent
invariant through all later states. Shared `_zm` has a distinct later revival
path setting movement scale0.3; it is not a standing unperked startup default.
C96 selection and its movement/sprint modifiers1 remain the native04 admitted
loadout evidence; no weapon table ID is guessed here.

The retained `common_zm` asset listing was also inspected for rawfile/config
leads; it does not furnish a bound config/default initializer for these slots.
No new config extraction is justified by that listing. Native command-line,
session dvars, external configuration, other imported scripts and dynamic
callbacks are outside this finite call/literal inspection. Thus “no assignment
found here” is not “the complete startup never overrides it.”

## Six-parameter admission ledger

Slots are preferred native VAs for continuity with the existing contracts;
values, registration types/ranges and effective baseline must remain absent
until a registration/override association is proven.

| Parameter | Qualified numerical consumer | Value/type/range and override admission | Exact remaining mapping |
| --- | --- | --- | --- |
| Base speed, slot `2338668` -> state `94` | `034ad0` calls integer accessor `2909f0`; separate perk branch can modify it | Value, registered type, range unknown. Accessor’s conversion branches are not registered type evidence. Fresh search3 address matches | Registration result or table entry bound to this slot, its default/type/range; Origins/session override order; exclude qualified perk branch for baseline |
| Gravity, slot `31fc4d8` -> state `8c` | `16cc30` float accessor `298b90` then integer conversion; `034ad0` also supports owner configuration | Value, registered type, range unknown. Explicit owner flag/value override is qualified; absence at baseline not yet established. Fresh search13 matches | Slot registration plus default/type/range; producer of owner stride`c70` flag`c60`/value`c64`, and baseline inactive predicate |
| Stop speed, slot `31fc4b0` | `53d9b0/53dc10` use `298b90` | Value/type/range unknown; no bound startup script override. Fresh search3 matches | Registration return/table binding, then default/type/range and any effective override |
| Backward scale, slot `31fe73c` | `540f60` uses `298b90` | Value/type/range unknown; no bound startup override. Fresh search2 matches | Registration binding and values; match the exact ordinary backward consumer branch rather than a recognizable name |
| Strafe scale, slot `31fe83c` | `540f60` uses `298b90` | Value/type/range unknown; no bound startup override. Fresh search2 matches | Registration binding and values; preserve distinct sprint/strafe consumers and C96 ordinary modifier |
| Standing eye target, slot `31fe6a4` | `53eaa0` stance owner uses integer accessor for standing target; target `204` differs from current `208` | Value/type/range unknown. Constants11/40 qualify prone/crouch targets, not standing. Fresh search34 matches | Registration binding/default/type/range, stance target producers and Origins/session override chain |

Fresh absolute-reference candidates consistent with documented consumer loads
do not reveal a direct initializer store for the six slots. Unaligned alternate
decodes sometimes resemble writes; they are explicitly excluded. This negative
result is limited to the address encoding, not proof that initialization is absent
or that an absent DLL owns it. Native04's encoded startup dispatcher `162b70`
remains an unbound lead. FP10's reachable precision hook qualifies a different
startup concern and does not authenticate dvar registration/defaults.

## Outcome and next discriminating evidence

This finite retained script route cannot fill any of the six missing defaults.
The exact gap is **native registration/result-to-global-slot association**, plus
the applicable startup/session/owner overrides. Further literal script searches
without that association cannot resolve it. A next bounded source task should
qualify one initializer/table writer through an established startup call path,
with the dvar registration argument convention and actual destination binding,
then use that parameter identity to follow the Origins override chain.

The camera contract also retains pitch limits, bob parameters and projection
ownership as separate missing inputs; standing eye target alone would not
complete its eye calculation. The existing numerical kernels can continue with
explicit supplied synthetic parameters. No guessed 190/800/100, axis scale,
standing height or external configuration is admitted as a baseline value.

Adjacent source observation, reported to the primary: Origins FFOTD startup
registers deferred script-model collision creation. These runtime scene additions
are distinct from serialized static instances and are not covered by a static
pool disposition alone. No poses/assets or new collision code are implemented
in this parameter task.

Validation: original/tool identity checks and fresh ten decoder invocations pass;
decoded call/operand inspection corroborates the previously admitted callback
scalars; all37 selected direct `setdvar` call windows and finite literal searches
are retained externally. Document links and whitespace checks pass. No Rust
changed, original instructions executed, process read, live comparison or
executable profile acceptance occurred.
