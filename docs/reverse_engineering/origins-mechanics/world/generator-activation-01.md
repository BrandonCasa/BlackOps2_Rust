# Generator activation and capture boundary

Recorded 2026-10-05. Status: **INVESTIGATING**. This is original-data script
evidence, not an original-game timing, appearance or multiplayer comparison.
The representative question is how one inactive generator admits activation,
advances or loses capture progress, and becomes player controlled.

## Source identity and reproducible procedure

Steam Zombies manifest build `24784239`, English, PC T6. The retained external
OAT extraction receipt identifies `zone/all/zm_tomb_patch.ff` as 1,288,384 bytes,
SHA-256 `770d8a2c3558c9e4c981d8d6db6bd67698111f01e8be32423c66b71719b4c2ca`.
A fresh read-only hash reproduced that digest. The retained directory listing
contains both script identifiers below. Extraction itself was not repeated;
provenance relies on the prior `damage-life-state-01` listing/dump and receipt.

| Selected compiled script identifier | Bytes | SHA-256 |
| --- | ---: | --- |
| `maps/mp/zm_tomb_capture_zones.gsc` | 52,210 | `c32043a75c53eb5cae4779d2495e4b55500dfb5917e2743d2d9228305244857b` |
| `maps/mp/zm_tomb_capture_zones_ffotd.gsc` | 430 | `299479cd9fe8d0176df5070827f27a7885b378f7f930414693cd5c91bef32a89` |

OAT Unlinker v0.33.0, revision
`7d027e8f89118196713e955b0e11f8404149c54d`, executable SHA-256
`9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63`,
created the retained extraction. See [directory evidence](../../0004-origins-asset-directory.md).
Fresh interpretation used the retained GSC Tool `1.4.10.333`, executable SHA-256
`3eff023c3f21cd8f7759fbe6466de37f43b15c4cd6ddfa64b7a44e421b6611ec`.
No compiler implementation or original script text is incorporated into this repository.

1. Resolve external input/output roots from ignored local configuration. Reuse
   the two exact compiled scripts from the external patch extraction; do not dump a zone.
2. Set a unique external working directory `mechanics-world-01`.
3. Invoke `gsc-tool -m decomp -g t6 -s pc <selected-script>` once per script,
   then `gsc-tool -m disasm -g t6 -s pc <capture-controller>`.
   All three commands exited 0. Tool output remains external.
4. Read functions named in the sections below. Cross-check rate initialization,
   branch ordering, division/multiplication, spatial strict comparisons and update
   formula against their corresponding disassembly subroutines. The disassembler
   and decompiler are the same tool, not independent implementations.
5. Reproduce synthetic checks with `python <this-directory>/generator_contract_checks.py`.

External outputs: controller decompile 81,663 bytes, SHA-256
`0079bea49c4f27d2f93351ec382edad51812411d50867d38d6108f1d425bce7f`;
controller disassembly 219,595 bytes, SHA-256
`ad78efc28b5a4fad0e52d2afa6fb646af088711f54111ab5dcbba01dc845e28`;
overlay decompile 340 bytes, SHA-256
`f37766f3d78cb94dff5480bf543c968fce23f7cf8a015ee7c6b0e84e59bc5b6a`.
The overlay's six initialization/capture/recapture hook bodies are empty in this
retained patch version. Dynamic updates, effective load precedence and any runtime
override of initialized rates were not established.

## Adjacent world inventory

These are observed script-family availability and assembly-call relationships,
not resolved mechanics. Identifiers are investigative leads.

| Family | Original-data anchor | Remaining bounded question |
| --- | --- | --- |
| Generators | Capture controller and overlay above; `zm_tomb` calls capture initialization | Representative activation resolved below; recapture scheduling and enemy control remain incomplete |
| Giant robots | Retained `zm_tomb_giant_robot.gsc`; `zm_tomb` calls glow/robot initialization and installs a revive override | Foot-entry admission and transition versus lethal contact; no robot geometry/timing claims |
| Excavation | Retained `zm_tomb_dig.gsc`; map assembly calls shovel initialization | Shovel ownership, dig admission, depletion and replenishment |
| Buildables | Retained `zm_tomb_craftables.gsc` and shared `_zm_craftables.gsc` | Part ownership, assembly admission and reusable inventory boundary |
| Tank traversal | Retained `zm_tomb_tank.gsc`; map assembly initializes tank and consults players on tank | Boarding, paid departure, route, cooldown and exits |
| Teleport/chamber traversal | Retained `zm_tomb_teleporter.gsc`, `zm_tomb_chamber.gsc`; map assembly initializes teleporter and watches chamber activity | Entry preconditions, destination, transition lifecycle and exit |
| Weather/environment | Map assembly calls weather initialization and registers snow/weather client fields | Movement effects and timing have no evidence in this record |

For assembly checks, reused `damage-life-state-03` decompiled `zm_tomb.gsc`,
SHA-256 `1f442456c5705e24f66c7534af4fe9fd49e45d06007ae89ab83e0d636a8a3afc`.
No adjacent script bodies were newly extracted or broadly investigated.

## Observed activation contract

Anchors: `init_capture_zone`, `setup_generator_unitrigger`,
`generator_trigger_prompt_and_visibility`, `generator_unitrigger_think`,
`wait_for_capture_trigger`, `get_generator_capture_start_cost`.

The map discovers generator structures by target name. Each has an opaque zone
identifier, objective/HUD index, spatial origin and related emergence-hole targets.
The controller initializes progress to zero and player ownership false. An
interaction stub is per-player, requires looking at the target, and uses a box-use
trigger with source dimensions 128 and radius 32; exact native trigger geometry
is unresolved. This interaction volume is distinct from the capture-participant area.

The prompt hides for contested or player-controlled generators. The trigger
callback rejects an invalid player, a player currently reviving someone, a player
different from its assigned trigger owner, insufficient points, and a globally
active capture event. The script's `is_player_valid` helper semantics must be
resolved through the shared player/life-state investigation; downed/dead membership
must not be guessed here. Prompt hiding is not a separately proven server-side
ownership guard: the callback does not repeat the contested/player-owned checks.

An accepted notification claims the global capture lock and charges the initiating
player `200 * current roster size` points. Capture-zombie monitoring and zone
activation start. The source enforces one ordinary admission globally, not one
independent charge per nearby participant. Queued notification and same-frame
thread races need a runtime comparison before a final event-ordering contract.

## Progress, interruption and completion

Anchors: `get_players_in_capture_zone`, `get_update_rate`,
`init_capture_progress`, `get_progress_rate`, `capture_progress_think`,
`handle_generator_capture`, `players_capture_zone`, `reward_players_in_capture_zone`,
`refund_generator_cost_if_player_captured_it`.

Capture membership requires player validity, horizontal squared distance **strictly
less than 48,400**, and player Z **strictly greater than generator Z minus 20**.
The radius is therefore 220 raw source coordinate units; there is no upper-Z
bound in this predicate. These units are not established as meters. The roster
denominator uses the full `get_players()` array, not only the valid nearby subset.

The controller updates progress before each nominal 0.1-second script wait.
Rate defaults initialize only when absent. Progress is clamped to [0,100].
The following are source-derived increments per update, not measured durations:

| Condition, in source branch priority | Increment |
| --- | ---: |
| Recapture event and this zone targeted, live matching attackers exist | -0.25 |
| Same, no attackers and not previously attacked | 0 |
| Same, no attackers but previously attacked | +1 |
| Target flag remains set outside recapture event | +1 |
| Ordinary capture, participant exists, forever-solo mode | 100/12 * 0.1 |
| Ordinary capture, participant exists, other mode | (participants/roster) * 1 |
| Ordinary capture, no participants | -0.5 |

Thus uninterrupted full occupancy has a nominal source budget of 12 seconds in
forever-solo or 10 seconds otherwise; one of four players yields 40 seconds by
formula. Update-before-wait, float precision, scheduler timing, mode selection,
network frames and dynamic rate overrides prevent treating these as measured
wall-clock completion times. The participant ratio's effective VM numeric behavior
still merits a multiplayer comparison; the disassembly confirms divide then multiply.

An ordinary empty-area interruption decays existing progress. Reentry before
zero can resume. At zero the contest clears and zombie-controlled state is set;
a new ordinary activation resets progress and requires a new admission/charge.
At 100 the contest ends only when the attacked-by-recapture flag is false.
At 100 while attacked, the contest remains active; the defense path is deliberately
not identical to ordinary occupancy capture. At zero with zero delta, the loop's
changed-progress branch is skipped. Ordinary empty activation instead has a
negative raw delta: the change test occurs before clamping, so it reaches the zero
endpoint and fails even when the clamped value remains zero. This ordering needs
explicit regression coverage.

Successful completion requests a success sound, stops the loop sound, waits a
network frame, rewards valid participants currently inside with the 100-point
bonus category and challenge/stat notifications if the zone was not already
owned, then sets player ownership. Actual awarded points depend on the shared
score function, not proven here. Ownership enables linked perk machines, random
perk machines and Mystery Box barriers, updates count/HUD fields, and emits a
zone-captured notification. Six owned zones set the all-captured flag; the full
Pack-a-Punch contract is outside scope.

After the contest and a one-second source wait, the initiating player is refunded
the stored activation cost only if still defined, the zone is owned, and that
player is currently within the valid participant predicate. The source halves
the refund argument when double-points scalar equals two; whether the shared
score helper restores its net amount requires separate evidence. This does not
prove an unconditional refund or a net final score. The global capture lock clears
when no contested zones remain. Owned-zone trigger waiting resumes only after
ownership is lost, which allows later reactivation but does not establish recapture
wave scheduling.

## Minimal proposed boundary and unresolved comparisons

Inference/proposal: use distinct ownership, contested progress, global admission,
roster membership, participant eligibility and presentation state. A single
`Powered(bool)` loses observable intermediate behavior. Reusable layers can own
participant filtering and a bounded progress transition; Origins supplies rate,
cost, linked services and defender conditions. Bevy retains entities, scheduling
and event delivery. No general scripting engine, custom ECS, new production API
or architecture approval is implied. Animation state is separate: startup and
power-up depend on original animation duration; shutdown waits its original end
animation. Exact animation lengths and audiovisual output were not measured.

The accompanying synthetic checks exercise the mathematical proposal and strict
boundaries only. They cannot validate engine helpers, original VM arithmetic,
load precedence, presentation, enemy combat or fidelity.

Original comparison procedure, pending runtime-owner permission: fix this build,
mode, round, rates and generator identity; record roster and life states plus
activation/debit time, participant positions, progress and ownership. Compare
solo full occupancy, one/four occupancy, boundary distance/Z, partial departure
then reentry, full decay then recharge, duplicate activation, initially empty
activation, initiator departure before refund, and a defender attack at 100.
Repeat each timing case at least three times and record frame/time resolution,
seed/attack variation and actual score deltas. Capture availability alone proves
none of these cases. No original runtime was launched or controlled for this task.
