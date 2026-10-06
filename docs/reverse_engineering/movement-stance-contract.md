# Ordinary posture requests, clearance and view-height transitions

Task movement-native-09, 2026-10-05. Fidelity INVESTIGATING. Static source and
synthetic checks qualify these bounded transitions; no original process reads,
invocation, play or capture occurred. This is not a complete posture controller.

## Evidence and boundary

Use [movement-native-contract.md](movement-native-contract.md) for disk identity
and mapping; [movement-command-update.md](movement-command-update.md),
[movement-jump-contract.md](movement-jump-contract.md) and
[movement-sprint-contract.md](movement-sprint-contract.md) retain dependencies.
External `movement-native-09/qualification-receipt.json` records13 body identities,
four height-table identities and synthetic checks. Resolve external roots through
ignored configuration. Source bytes, disassembly and tables remain external.

| RVA | Boundary | Bytes |
| --- | --- | ---: |
| 542ba0 | Posture owner | 3418 |
| 1a3920 | Effective posture classifier | 62 |
| 53eaa0 / 53e9f0 / 037ae0 | Height owner / curve / duration | 1102 / 176 / 45 |
| 53ef40 / 53eef0 | Request inhibition / weapon-specific inhibition | 55 / 80 |
| 158530 | Environment request gate | 184 |
| 18eb40 / 03c790 / 11c830 | Prone admission / geometry / weapon gate | 186 / 3436 / 24 |
| 2ac080 / 23a890 | Transition animation receiver / associated jump-timer write | 863 / 25 |

53e9f0 ends directly before53eaa0;53eef0 ends directly before53ef40. Padding-based
extraction initially included adjacent functions; admitted lengths exclude them.
The03c790 tail branches back into its own body and belongs to that function.

## Distinct state fields

Player flags c low1 represents raw prone and low2 raw crouch. Context hulls are
lo(-15,-15,0),hiXY(15,15), highZ70/50/30 for standing/crouch/prone. Player flags
fc bits8/4 mirror final effective prone/crouch assignment. These are posture
fields, separate from the existing life state. Special modes5/10 and modes7/8
have distinct paths and are excluded from the ordinary baseline.

Target view height204 is signed integer; current208 is f32. Effective classifier
1a3920 reads the **target**, not raw flags or current height: target11 ->1(prone),
40 ->2(crouch), other ->0(standing), except target22. Target22 consults state5b8:
>=3 ->0;0 ->1; other nonzero ->2. Stage22's ordinary reachability and meaning
remain unqualified; retain it separately rather than naming a fourth posture.
Standing target is integer provider31fe6a4, whose value/default is still missing.
Do not confuse hull70 with a recovered standing eye height.

Height transition fields20c(timestamp),210(stage target),214(direction) are
separate from target204. Context228 is reset at entry and set for selected prone
entry/exit target changes. Source owner is called from544b30 after preprocessing,
with local explicit seconds at24 and current command time in context4.

## Request processing and blocked rises

The complete owner has forced/special paths before ordinary requests. They include
active sprint8000 forcing standing, player fc force bits100/200, fc4000, state5b8,
request inhibition53ef40 and environment158530. No absent flag/dvar is invented
as a baseline default. Ordinary request conclusions below require those gates to
permit the request, modes outside5/7/8/10, and no configured toggle branch.

53ef40 inhibits requests for flags c c00, flags18 bit400, weapon state1d0==2c,
non-sentinel state14c, or weapon-specific53eef0. The latter checks states6/7/8/28/
13/14/15, nonzero weapon low ID and baseWeaponDef byte740. The source getter is
qualified; that definition field's active C96 value is not admitted here.
158530 depends on bool providers31e06c4/31e06c8 and float31e06d4 with state4d4;
it may invoke2d10d0 before returning. Its configuration remains a supplied gate.

Current low button bit23 has priority over bit22 for ordinary requests. No prior
button edge is read here. Bit23 plus absence of flags400 invokes18eb40. On success
raw crouch clears and raw prone sets. On failure with owner3ff it skips rejection
events; with an owner it sets flags1000, normally emits eventea parameter3, then
may emit event9/8 according to existing posture unless command bit19 suppresses
that event. It does not silently convert a failed prone request into a generic
standing request.

Bit22 requests crouch. From raw prone it first tests a temporary50-high hull and
restores highZ afterward. On success clear prone/set crouch; blocked clearance
keeps those raw posture bits and normally emits event10 parameter2. From standing
it sets crouch without this rising clearance, unless a force predicate forbids it.

With neither bit23 nor bit22, raw prone first tests standing70. If allowed, clear
both raw posture bits. If blocked, try temporary50 and restore temporary highZ;
if allowed, clear prone/set crouch. If both blocked retain raw posture and normally
emit event10 parameter1. Raw crouch tests70; allowed clears crouch, blocked keeps
it and normally emits event9 parameter1. Command bit19 suppresses the rejection
event. Other transition events invoke131f30 with IDs a/d at qualified branches;
receiver delivery is not reconstructed by these request checks.

Both rise checks use541e10 ->22ef70's **alternate** query route at the same origin,
mask `movementMask & fdff7fff`, owner1a0, lower/upper hull refs, and accept exactly
when result byte2b==0. Fraction/normal are not substituted for that predicate.
No ordinary moving brush result is coerced into this clearance result.

Native clearance rejection is a valid outcome with the above side effects, not
a safe Rust error. Query/provider/arithmetic errors must leave authoritative Rust
state unchanged, while qualified native rejection events/writes must be retained.
Only raw posture and restored temporary hull are guaranteed unchanged at the
blocked request phase; later interpolation, effective assignment and pose work
can still write state. A whole-function no-op on blocked clearance is incorrect.

Flags3800000 plus environment0f6960(4) select request remapping: event choice8/9/a,
clearing a source-built button mask through022d00 and setting bit22/23. Its active
configuration is not admitted as the baseline. Special flags200000 and failed
environment gate can force prone request bits/target11 with command mutation;
these paths must remain explicit exclusions rather than disappear into defaults.

## Target staging and final hull assignment

When transition timestamp20c is nonzero, ordinary target-selection block is
skipped; existing stage runs. Otherwise raw prone with old target equal to supplied
standing target first chooses40; subsequent raw prone chooses11. Selected entry
sets context228=1, invokes2ac080 and23a890. Raw non-prone with target11 first
chooses40 and invokes2ac080; raw crouch chooses40; raw standing chooses supplied
standing target. Thus stand/prone changes stage through crouch height.
23a890, if player timer1c==0, sets flags4000 and timer1800: this is a directly
associated write, not a qualification of timer advancement or jump landing.
2ac080 depends on thread-local animation storage and provider31fc46c; complete
source retained, receiver/storage/default binding is unresolved.

After height owner53eaa0, classifier1a3920 determines effective stance. It writes
highZ30 and flags c prone/fc8, or highZ50 and flags c crouch/fc4, or highZ70 and
clears both mirrors. Raw requested posture can therefore differ from final
posture during staging. Initial hull writes precede rise checks; final effective
hull assignment follows interpolation. Neither hull nor camera target may be
updated independently from this source order in a full adapter.

## Height owner and pure curve boundary

If target0 or current height is finite zero,53eaa0 initializes current to converted
target (mode5 has its separate zero path). If current equals target and timestamp
is0, it returns. Boolean provider31fe740, when enabled during lowering, writes
player accumulator588=255; its association/default is unresolved.
Nonstandard targets outside11/40/supplied standing use explicit local seconds
multiplied by source180.0, add/subtract current, clamp at target and clear timestamp.
This branch does not establish full camera projection/bob/view angles.

For ordinary stages, duration037ae0 is600ms when stage11; stage40 is400ms for
direction0 and200ms for nonzero direction; other stages200ms. Existing stage
percentage is signed `((now-timestamp)*100)/duration`, integer truncation toward
zero, clamped0..100. Overflow/zero timestamp domains remain explicit. At100 the
owner clears timestamp, assigns current height to stage, and changes mode9 to0.

Four identity-bound static curves are selected:

| Table VA | Stage / direction | Curve base argument | Other endpoint |
| --- | --- | --- | --- |
| 101e648 | 40 / nonzero | 40 | Supplied standing target |
| 101e690 | Other ordinary /0 | 40 | Supplied standing target |
| 101e6d8 | 11 | 11 | 40 |
| 101e730 | 40 /0 | 11 | 40 |

Curve53e9f0 receives percentage in ECX, other endpoint in EAX, table in EDI, base
on stack. **Percentage0 returns converted base immediately**, bypassing table[0].
Independent bounded stack provenance for53ec04..53ec16 confirms push stage11,
EAX=40,ECX=percentage,EDI=101e6d8; callee PUSH EBX then [ESP+8] reads11.
Lowering tables' zero row does not override this early return. This source edge
is preserved without inventing smooth motion or claiming observed original output.

Otherwise exact table knot supplies coefficient, or adjacent integer percentages
supply ordered f32 interpolation: convert differences, divide, subtract upper/
lower coefficient, multiply, add lower coefficient. Height uses separate
`coefficient*f32(other-base) + f32(base)`. The source curves are not generic easing:
prone rise has an initial undershoot, and lowering is not monotone everywhere.
At ordinary owner percentage100 the endpoint shortcut bypasses curve evaluation.

Starting a stage records now; target11 chooses direction1 and stage40 when current
>40, otherwise11. Target40 chooses stage40 with direction(current>40). Standing
chooses direction0 and stage40 when current<40, otherwise standing. An in-flight
target reversal changes direction, remaps the stage to its adjacent endpoint,
uses remaining percentage100-old, and rewinds timestamp through exact ordered
f32 duration*(f32(remaining)*bits3c23d70a), truncate, then subtract from now.
When remaining==100, clear timestamp and assign remapped stage directly. These
source rules must not be replaced by a generic lerp/restart. Full reversal owner
state and provider policy are not yet an executable controller.

## Prone geometry and pose gap

18eb40 first invokes weapon byte694 gate11c830; nonzero rejects even existing
prone. Otherwise existing raw prone returns true. New prone requires owner!=3ff,
state5b8<1 and03c790 geometry. Its arguments are player/owner/origin, radius15,
height30,yaw1fc, output floats4d8/4dc, fixed flags0/1/1, handler byte, selector0,
span50.03c790 selector0 chooses mask00810011, obtains moving query callback from
handler table101e770 (ordinary row1 ->2fa5e0), initially traces origin upward10
with the supplied box and rejects result byte2a when nonzero. It then performs
rearward/body/ground probes and writes output orientation values; acceptance is
not determined solely by a normal-dot threshold.

Complete3436-byte body retained; remaining orientation/math/provider dependencies
include053dc0,07f8d0,182b60,2c5be0,01da20,170650,630d90 and integer provider2bb2f20,
plus qualified collision callback access. After new effective prone, owner542ba0
can move origin using additional up/down traces and fraction interpolation, copy
yaw to464 and derive/clamp468/46c through2a9ac0/225e90, with source limits +/-45.
This geometry/orientation chain is the precise finite next prone task. Its outputs
and x87 math are not approximated here, and missing pose cannot be silently skipped.

## Validation and executable candidate

Independent external checks passed5 durations,12 timing boundaries,8 classifier
cases (including separate22) and7 request-phase rise outcomes with blocked-event
parameters. Twenty f32 curve fixtures cover zero, knots/interior points and
undershoot. Synthetic standing endpoint60 is deliberately supplied and is **not**
a recovered default. The stack-provenance check passed separately.

No Rust changed. Owned `git diff --check` passed. Candidate next kernels are
supplied target-based classifier, stage duration/checked percentage and exact
source curve evaluation with admitted table/operand/rounding domain. Full posture
still needs standing target and lowering provider, forced/toggle/environment and
weapon state binding, animation receiver, alternate clearance and prone pose.
Full camera, movement motor, other collision families, FP startup and parameter
registration remain separate tasks; original comparison is absent.
