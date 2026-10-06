# Ordinary sprint transition and speed boundary

Task movement-native-07, 2026-10-05. Fidelity INVESTIGATING. Static licensed
source qualification and independent synthetic checks establish the following
bounded behavior; they do not establish original comparison or full movement.
No original process access, invocation, play or capture occurred.

## Evidence and prerequisites

Use the disk identity and PE mapping in [movement-native-contract.md](movement-native-contract.md).
External `movement-native-07/qualification-receipt.json` records13 admitted body
identities. Raw bytes, disassembly and helper scripts remain external, resolved
through ignored configuration. Read together with
[movement-command-update.md](movement-command-update.md),
[movement-grounded-contract.md](movement-grounded-contract.md),
[movement-baseline-parameters.md](movement-baseline-parameters.md) and
[movement-jump-contract.md](movement-jump-contract.md).

| RVA | Boundary | Bytes |
| --- | --- | ---: |
| 541ef0 | Sprint transition owner | 534 |
| 540c10 / 540dd0 | Stop helper / continuation interruption | 57 / 398 |
| 1f5f30 | Start motion interruption | 438 |
| 541e60 / 207200 | Two posture clearance consumers | 137 / 96 |
| 068770 / 08c440 | Available budget / maximum duration | 181 / 145 |
| 3306c0 / 297610 | Minimum-start seconds / exceptional rule | 3 / 3 |
| 540f60 / 280120 | Command speed scale / sprint factor | 682 / 7 |
| 540c50 | Posture speed scale | 371 |

207200 ends at207260 before an unrelated adjacent function, without padding.
Its exploratory filename `sprint-weapon` is not evidence of a weapon consumer:
it is a standing-hull clearance check. Names were investigative leads only.

## State and source order

Dispatcher544b30 calls541ef0 after command preprocessing0e9c40; normal paths
classify ground before this transition and consume speed/movement afterward.
Sprint fields are separate from life state and posture:

| Player state field | Observed role |
| --- | --- |
| flags c bit8000 | Active sprint |
| 4b8 | Restart latch |
| 4bc | Exhaustion/cooldown marker |
| 4c0 | Start command time; zero has special budget meaning |
| 4c4 | Stop command time, possibly delayed |
| 4c8 | Remaining budget stored when starting |
| 4cc / 4d0 | Configured duration / cooldown milliseconds |

Ordinary unperked C96 adapters admit duration4000ms and cooldown0ms, from player
Origins configuration and weapon duration factor1. Flag fc4000 or perk25 changes
duration08c440; these branches are outside the baseline. Duration converts the
configured integer to f32, multiplies weapon590, truncates and caps above16383.
No runtime weapon ID/name population or player initial field values are invented.

Owner first clears a nonzero restart latch when current low button bit30 is
absent and player flags800 is absent. It then permits active/start processing
only for signed modes0 or1; other modes stop an active sprint. Weapon state1d0
in inclusive38..3a also stops immediately. Nonpositive maximum duration stops.
Stopping clears exhaustion4bc, records current time in4c4, clears active8000,
and sets latch4b8=1 if current bit30 is set. An inactive stop is a no-op.

When active, ordinary unperked source first checks elapsed `now-start >= stored`.
The exceptional helper297610 is a complete false-return stub in this image;
perk3e can bypass exhaustion. At equality, ordinary exhaustion invokes stop and
then sets4bc=1. Otherwise continuation predicate540dd0 can stop; when it stops
with current jump bit21 set, stop time4c4 additionally increases by800ms.
This future stop time affects recharge directly. Sprint continuation does not
require bit30 to remain held; releasing it alone is not a stop condition.

When inactive, an exhaustion marker gates restart until signed `now-stop >=
cooldown`. Then require current bit30, player flags40000 absent and latch4b8=0.
Require start motion predicate1f5f30 false, posture541e60 true and standing
clearance207200 true. Available budget068770 must be strictly greater than
minimum seconds3306c0 times1000. That complete helper returns x87 literal1,
so the admitted integer boundary is **remaining >1000ms**, not >=1000.
A successful start writes4c8=remaining,4c0=now and sets active8000. It does not
clear an old4bc marker or write4c4. No old button edge is read for start: latch
and current input govern it.

All signed arithmetic is native i32 instructions. The overflow domain remains
unqualified; a checked Rust boundary should reject overflow atomically, including
intermediate sums, rather than claim wrapping is original timing policy.

## Debit and recharge equations

068770 first gets maximum duration D. Unperked exceptional-rule false selects
ordinary equations. Let S=start4c0,E=stop4c4,B=stored4c8,N=now,C=cooldown4d0:

- S==0: return D, regardless of other fields.
- S>E: R=B-N+S (ordered subtract then add); this debits running time.
- S<=E with exhaustion marker: R=(-E)+(-E)-C+B+S+N.
- S<=E without exhaustion: R=B-(E+E)+S+N.
- Clamp negative R to0, then cap R above D.

The equality branch matters. The equations do not clamp elapsed recharge to
zero separately. A jump-delayed stop time can reduce the returned budget before
that time. The start-zero sentinel must survive source reconstruction; replacing
it with an inferred active flag changes observable budget results.

## Eligibility and interrupt dependencies

Both start-motion1f5f30 and continuation540dd0 reject signed forward command
axis <= integer provider31fc4a0. Its registration/type/range/default and active
Origins value remain unresolved. Both reject state5b8>=2 and nonzero or unordered
state90 (comparison against source0). For the ordinary unperked branch:

- Start rejects flags8 early; command bits21,27,29,31,26,17,16; flags1001c;
  weapon states13,14,15,16..1d,1e..22; flags400000; and finally flags200000.
- Start command bit7 requires baseWeaponDef64==3. The same definition value3
  additionally rejects weapon states6,7,8,28 (plus13..15). The definition value
  is not labeled a weapon class without further evidence.
- If start reaches flags4000 with timer1c==0, it returns permitted immediately
  before the later weapon-state checks; this branch must preserve source order.
- Continuation rejects flags10018; command bits21,27,29,31,26,17,16,22;
  command bit23 unless helper11c830 admits the weapon; command bit7 unless
  definition64==3; weapon states13,14,15,16..1d,2a,2b,1e..22; flags400000;
  and finally flags40000.

Perk2a plus environment0f6960(4) select a distinct bit7/29/31 branch in both
predicates; it is excluded from the unperked baseline. Sparse numeric state/flag
identifiers retain raw names until their meanings are independently qualified.
Weapon-state production and optional helper11c830 remain separate dependencies;
ordinary command bit23 clear does not invoke that helper.

Posture541e60 returns true directly if player low flag1 is absent. Otherwise it
temporarily sets the existing hull highZ to **50**, traces stationary at origin
using the current bounds and mask `movementMask & fdff7fff`, then restores
highZ and accepts iff alternate trace result inside-byte2b is zero. Consumer
207200 returns true directly if low flags3 are absent; otherwise checks the
source hull lo(-15,-15,0),hi(15,15,70) with the same mask and inside-byte.
Both use22ef70's alternate route, whose receiver remains a collision dependency.
No classic stationary-brush equivalence or slope threshold is inferred.

Jump53b610 resets latch4b8=0 after launch; that adjacent write is already qualified
in jump06. Landing/timer advancement and stance transition ownership are excluded.
Only two admitted direct stop-helper call sites occur, in541ef0; mode, weapon and
duration stops also inline its writes. Raw byte call-candidate search was checked
against the bounded admitted owner, not treated as an independent call graph.

## Numerical sprint speed consumer

Command scale540f60 computes axis norm from signed8 axes, qualified base speed94,
backward/strafe parameter weighting and denominator127. It applies additional
state modifiers before the sprint branch. Active8000 only applies a sprint factor
when view target204 equals integer provider31fe6a4. That provider remains
unqualified. Complete helper280120 loads source literal**1.5** (bits3fc00000)
through x87, multiplies the current stored f32 scale, then stores f32. This is not
an always-on replacement for source gating, nor a qualified x87 Rust kernel.

Next posture helper540c50 scales (and handles animated posture interpolation),
state5b8 adds an SSE factor, excluded perk2a may apply another provider, ordinary
C96 movement factor1 applies, and player movement50c factor1 is final. Full
scale retains backward/strafe, view-target, base speed and x87 control gaps from
native04. No executable default run speed is admitted from factor1.5 alone.

## Validation and candidate increment

Independent external source-flow reconstruction passed16 budget cases spanning
initial sentinel, running debit, 1000/1001 restart boundary, exact exhaustion,
normal recharge, exhausted recharge, equality and zero cooldown. Two strict
minimum checks and one jump-delayed stop equation check also passed. These are
synthetic checks, not original gameplay comparison. Owned documentation passes
`git diff --check`; no Rust changes or workspace build claims were made here.

A finite next executable increment is a checked integer budget/stop transition
with explicit source fields and supplied duration/cooldown, including zero-start
sentinel and signed intermediate overflow errors. A supplied eligibility facade
may follow after threshold, weapon-state and alternate clearance bindings land.
Full sprint additionally needs active profile parameters, source FP policy and
speed/posture integration. Startup parameter overrides and FP initialization are
separate source tasks; this investigation does not broaden into them.

## Bounded integer implementation (movement-native-08)

`bo2-gameplay::movement::sprint_budget` now implements available budget068770
and ordinary stop540c10 plus the qualified caller's optional800ms jump delay.
`SprintBudgetState` supplies active/latch/exhaustion semantic fields and explicit
start/stop/stored milliseconds. `SprintBudgetParameters` supplies signed maximum
duration/cooldown. No weapon duration conversion or profile defaults are supplied.
The strict restart minimum is exposed separately; it does not grant sprint start.

Every native-ordered signed intermediate uses checked arithmetic; TimeOverflow
is a safe Rust domain error, not an assertion about original overflow behavior.
The zero-start sentinel bypasses arithmetic. Active/latch fields do not influence
the budget timestamp branch. Negative remaining is clamped before the supplied
upper cap, including the raw negative maximum case. The caller's nonpositive
duration stop gate remains outside this kernel.

Stop returns a new state, preserving start/stored and an existing latch, clearing
active/exhaustion and setting latch only when supplied sprint input is pressed.
Inactive stop is a no-op even when a supplied optional delay would overflow.
Active delayed stop computes checked now+800 before returning any writes; failure
leaves authoritative state unchanged. Exhaustion-marker writes after depletion
and latch clearing on release remain owner transitions outside this stop helper.

Ten focused tests pass: 16 independent source fixtures, strict minimum and
cooldown boundaries, zero/equality/timestamp branches, raw signed caps, source
intermediate overflow, latch/no-op behavior, delayed recharge and atomic failure.
A 9,600-case bounded grid also agrees with independent i64 elapsed-time accounting.
Focused locked tests, gameplay all-target warning-denying Clippy, owned rustfmt
and owned diff checks pass. These do not establish full sprint or original
comparison; the unresolved transition/profile/FP/collision dependencies above
remain unchanged.

