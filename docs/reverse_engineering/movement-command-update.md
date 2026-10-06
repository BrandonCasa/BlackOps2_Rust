# Native command, timing and posture qualification

Task movement-native-02, 2026-10-05. Source fidelity INVESTIGATING. This qualifies
the command boundary and its caller associations; it does not qualify a complete
numerical controller or original-game equivalence. No original invocation,
process reads, input, play or capture occurred.

## Evidence and reproduction

Use the disk identity and PE mapping in [movement-native-contract.md](movement-native-contract.md).
All RVAs below refer exclusively to that qualified disk image. Read complete
bounded bodies and their referenced static data, not isolated instruction hits.
External `movement-native-02/qualification-receipt.json` records body hashes,
lengths and independent timing checks. Resolve external roots through ignored
configuration. Raw bytes, disassembly and helper scripts remain external.

| RVA | Qualified responsibility | Retained bytes |
| --- | --- | ---: |
| 034ad0 | Ordinary entity command owner | 2657 |
| 18f1f0 | Separate special command path | 414 |
| 1a6d30 | Movement environment wrapper | 522 |
| 23af10 | Command backlog/substep loop | 121 |
| 544b30 | Single-step dispatcher | 4084 |
| 542ba0 | Posture requests, clearance and bounds | 3418 |
| 1a3920 | Effective posture classifier | 62 |
| 541e10 | Clearance response adapter | 70 |
| 22ef70 | Environment query/refinement wrapper | 259 |
| 025170 | Alternate query producer dispatch | 173 |
| 2ecd50 | Handler-table world query dispatch | 161 |
| 1b2d20 | Alternate result clear initializer | 83 |
| 2c52c0 | Alternate result conversion | 316 |
| 44b290 | Native getstance flag consumer | 114 |

Single-step has 4049 code bytes through its final return at 545b00. The retained
span additionally contains alignment and switch data: five absolute targets at
545b04 and twelve selector bytes at 545b18. Targets resolve inside the body to
5455fc, 54554e, 5455c3, 5453dd and 54540b. Those bytes are data, not instructions.

## Ordinary ownership and command boundary

034ad0 obtains state through entity+154. Eligibility includes client+5428==2;
client+5410 values 3 and 2 dispatch separate paths, and the ordinary path requires
1164d0 to succeed. Do not substitute this chain for those modes. Entity index
ownership is checked against state+1a0 when elapsed command time is below 1.

The ordinary owner prepares a movement context with state pointer at +0, copies
15 dwords (60 bytes) of the incoming command to +4 and the previous command to
+40, selects handler byte +25c=1 and initializes +260=-1. It invokes
1a6d30 -> 23af10 -> 544b30. The wrapper installs a separate environment pointer
at +264 and additional query bookkeeping at +270. Pointer-bearing native layouts
are investigative schemas, not proposed Rust layouts.

| Field | Qualified storage/read | Rust boundary implication |
| --- | --- | --- |
| Command time | Command-relative +0, signed 32-bit integer | Explicit native milliseconds |
| Buttons | Command-relative +4/+8, two 32-bit halves | Preserve 64 bits and previous command |
| Forward/right | Command-relative +24/+25, signed byte reads | Signed 8-bit axes; normalization unqualified |
| Origin | State +28/+2c/+30, f32 | Authoritative vector |
| Velocity | State +34/+38/+3c, f32 | Authoritative vector |
| Command time | State +0, signed 32-bit integer | Persistent accepted time |
| Mode/flags | State +4 / +c | Preserve qualified predicates; not life state |
| Owner index | State +1a0 | Explicit ignored player identity |
| View angles | State +1f8/+1fc/+200, f32 | Authoritative angle state, conversion pending |
| View target/current | State +204 integer / +208 f32 | Separate target and interpolation |

The command is larger than these admitted fields. Angle encoding, other command
members, axis scaling and mode flag policy remain unqualified. The view-angle
owner lead is 1aef70, called from 544b30; it invokes 53fb00/53fa10 and contains
mode-specific view behavior. Do not derive mouse-to-angle conversion from Bevy.

Current buttons live at client+568c/+5690, previous buttons at +5694/+5698,
pressed edges at +569c/+56a0, with pressed bits ORed into +56a4/+56a8.
The ordinary owner also applies conditional masking before updating those words.
This establishes persistent edge ownership, not a complete host key mapping.

## Explicit timing policy

034ad0 clamps incoming time to [server time - 1000, server time + 200] using signed
comparisons. Its local elapsed value is capped at 200 for other owner work. This
is distinct from the motor's policy and requires an explicit server-time input.

23af10 returns for final command time below the state's time. Otherwise, if the
gap exceeds 150ms it advances state time to final time minus 150 before executing
movement. It resets context counters +80/+104, returns if time is equal, and runs
substeps of min(remaining,66) milliseconds. Each substep rewrites current command
time, executes 544b30, then copies all 60 current command bytes into the previous
command. Thus a 150ms gap executes 66,66,18; a 151ms gap discards 1ms then executes
the same three durations. Other command fields persist across these substeps.

544b30 independently clamps its elapsed duration to [1,200], writes accepted
command time to state+0, and computes f32 seconds by multiplying the converted
integer duration by bits 3a83126f (0.001f32). The regular loop admits only 1..66.
Integer wrap boundaries are not qualified as a permitted request domain; reject
unqualified ranges in a future safe boundary instead of inventing wrap behavior.
Bevy fixed frequency does not establish any of these policies.

## Hulls, masks and clearance

Ordinary lower bounds are (-15,-15,0), upper XY (15,15). Effective posture upper
Z is 70 standing, 50 crouching, 30 prone. 544b30 initializes bounds at context
+188/+194; state+c bit0 selects 30, otherwise bit1 selects 50. 542ba0 updates the
same bounds after view-target processing and effective classifier 1a3920.
The native getstance consumer independently distinguishes state+c bit0 and bit1.
These are hull extents, not a qualification that every consumer uses an AABB.

542ba0 consumes low command button bits 23 and 22 in prone/crouch requests, with
additional mode/toggle predicates. Target view heights are 11 prone and 40 crouch;
the standing target reads a dvar, whose value is not yet admitted. 53eaa0 handles
interpolation. Special mode 5/10 bounds and view behavior are excluded from the
ordinary baseline. Prone has orientation/validity helpers 03c790 and 18eb40;
ordinary hull values alone do not establish full prone collision behavior.

Ordinary preparation sets context+7c to 02818011 when state mode <10, otherwise
00800011. Subsequent deathlike conditions can remove bits 02008000. Do not infer
collision meanings from bit names. Rise-clearance requests remove bits 02008000
using fdff7fff, yielding 00810011 from the ordinary mask. Start and end are the
same authoritative origin. The owner identity is state+1a0. Blocked crouch rise
retains crouch; prone rise tries standing, then temporarily tests a 50-high hull
for a crouch fallback and restores the temporary bound before final assignment.

The handler table at image VA101e770 has 36-byte rows. 2ecd50 reads the row selected
by +25c; ordinary row1 points to RVA2fa5e0, agreeing with the retained task0037
filter producer and downstream 0e7cc0 -> 1a9780 -> 2d8770 world route. This is a
static initialized table, not a guessed runtime callback.

Rise-clearance specifically calls 541e10 -> 22ef70 -> 025170 ->
27b1e0/00ff70. This is a separate producer route. 22ef70 additionally invokes
103b20 when fraction at result+10 is >0. Its equivalence to the retained world
brush/terrain route is unresolved. The wrapper environment mask 03818813 is
distinct from the movement mask and is not a replacement player query mask.

541e10 returns true exactly when result byte+2b equals zero; it does not inspect
the fraction or byte+2a. 1b2d20 initializes fraction(+10)=1 and both bytes zero.
2c52c0 copies fraction from intermediate+30, normal XYZ from intermediate+0/4/8,
sets byte+2a=0, and sets byte+2b=(intermediate f32+34 < 0). This is the observed
response predicate; naming it classic startsolid/allsolid or converting existing
brush result booleans without qualifying the producer would be an inference.

## Ready boundary and remaining implementation prerequisites

MovementCommand can admit integer time, 64-bit buttons and signed byte axes now,
while retaining an explicitly unresolved angle representation. MovementState
needs accepted time, origin, velocity, posture, previous command, qualified mode
predicates, view target/current and owner identity. MovementProfile timing fields
must preserve the distinct server clamp, backlog and substep policies. The
Origins baseline modifiers from task01 remain independently qualified.

MovementStep should report explicit discarded backlog/substep durations;
MovementError should cover timing/unsupported mode and collision-provider
failures. These are proposed safe-boundary decisions for primary integration,
not recovered native error behavior. Failed requests must commit no state.

Next bounded numerical roots are ordinary walk 544200 and air 543ec0, reached
from the single-step local ground flag; both reference friction 53d9b0,
acceleration 53dc10 and step/slide 0246c0. Ground classification, sprint/jump
conditions, slope/landing rules and all numerical constants remain to qualify.
The clearance intermediate producer and refinement need collision ownership
qualification before accepting a PlayerCollisionWorld implementation as native
clearance access. Full camera interpolation/angle and prone orientation remain
separate prerequisites. No motor, ECS facade or executable profile landed here.
