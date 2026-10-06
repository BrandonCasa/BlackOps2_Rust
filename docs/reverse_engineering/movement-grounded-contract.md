# Grounded numerical contract: qualified kernels and solver boundary

Task movement-native-03, 2026-10-05. PARTIAL, INVESTIGATING. Source is the static
disk identity admitted in [movement-native-contract.md](movement-native-contract.md).
Command timing/hulls/masks remain governed by
[movement-command-update.md](movement-command-update.md). No original invocation,
play, capture or process reads occurred. No complete motor is approved here.

## Reproduction and bounded source inventory

Resolve ignored movement source locators, map RVAs through PE sections, decode
x86 complete bodies, then inspect referenced data and direct/indirect callees.
External `movement-native-03/qualification-receipt.json` retains hashes/lengths
and independent checks; raw bytes/disassembly remain external. Thirty bounded
spans were inspected, including leads subsequently excluded from ground ownership.
The linear decoder's labels do not establish function semantics.

| RVA | Admitted role | Bytes |
| --- | --- | ---: |
| 544200 | Walk numerical coordinator | 907 |
| 53d9b0 | Shared friction | 602 |
| 53dc10 | Shared acceleration | 424 |
| 14c9b0 | Speed-preserving ground projection | 229 |
| 542850 | Ground query/classification coordinator | 833 |
| 542440 | Initial inside-contact recovery | 660 |
| 5426e0 | No-ground proximity/event handling | 366 |
| 0246c0 | First solve, raised solve and step selection | 2188 |
| 282630 | Numerical solver request preparation | 125 |
| 12ae20 | Initial solve / inside correction / secondary solve | 115 |
| 545d90 | Raised solve adapter | 120 |
| 2530d0 | Contact-constrained numerical solve | 3244 |
| 199760 | Inside correction | 2497 |
| 0df430 | Contact constraint solve | 2670 |
| 1caa80 | Bound solver contact notification | 87 |
| 07bf60 | Ground/step-down contact selection | 519 |
| 12bfa0 | Contact walkability predicate | 456 |

The 14c9b0 retained span is 229 bytes and 0246c0 is 2188 bytes; reproduce their
receipt rather than inferring sizes from upstream instruction addresses. Earlier
candidate 541ef0 is sprint-state processing, and 541670 is speed/animation
bookkeeping; neither is the ground classifier. They were excluded from scope.

## Ground ownership and response fields

Ordinary single-step dispatch runs 542850 before selecting walk544200 when local
walking at +2c is nonzero, otherwise air543ec0. Ground-plane contact at local+30
is distinct; near-ground is local+34. The selected 64-byte ground result resides
at local+40, with normal XYZ at +40/+44/+48 and surface flags at +54.
State+a4 is the persistent ground owner, with sentinel 3ff for no ground.

542850 normally queries start=(origin.x,origin.y,origin.z+0.25) and
end=(origin.x,origin.y,origin.z-0.25). A separate state+fc predicate uses
start=origin/end=origin-1Z. It passes existing hull, mask, ignored owner and the
environment at context+264 into 07bf60. It retains the whole result and interprets
fraction, inside bytes, walkability byte+2c and special-result byte+30 separately.
If a contact exists but byte+2c is zero, plane/near-ground become1 while walking
is0 and persistent ground owner is cleared. With byte+2c nonzero it sets all three
local fields1 and records the owner. An inside retry has a distinct branch copying
result byte+30 into walking. Initial inside recovery542440 must remain explicit.

07bf60 prepares contacts via27b1e0/00ff70 and prefers suitable contacts within
0.5 vertical units; it calls12bfa0 for walkability and2c52c0 for output conversion.
12bfa0 requires contact byte+38, accepts the normal-Z threshold directly, and
otherwise consults the owning collision object, optional transform and method
at its table+30. This is not a universal slope-normal test. The normal-Z threshold
and receiver implementation remain unqualified, so no guessed maximum slope is
admitted. Contact ordering, transformed ownership and inside cases need the
collision producer contract described below.

## Qualified ordinary friction and acceleration kernels

Here `F` denotes rounding each SSE scalar operation to f32; use the documented
operation order, no fused multiply-add. Native x87 sequences must preserve their
store boundaries separately rather than assuming all intermediate arithmetic
is f32 or replacing it by unrestricted Rust f64.

53d9b0 computes speed with order y*y + x*x + z*z and sqrt; the speed calculation
uses z=0 when walking but subsequently scales all three authoritative velocity
components. Speed<1 clears all three components and returns. For the ordinary
standing ground branch (special posture-state+5b8=0, mode not5, flags excluding
20000/40000100/80/4000/200000, surface flags excluding2), control=max(speed,
configured stop speed). The ground friction coefficient is **5.5**, returned by
the complete263de0 body from constant RVA7da530 (bits40b00000).
The native x87 multiply sequence is control*5.5*seconds, stored to f32 drop.
Then newSpeed=max(F(speed-drop),0), ratio=F(newSpeed/speed), and each component
is independently multiplied by ratio. Excluded predicates select additional
friction/modifier branches and cannot be silently treated as ordinary ground.

53dc10's ordinary flag-bit3-clear branch computes projected speed as
F(F(F(direction.x*velocity.x)+F(velocity.y*direction.y))+
F(velocity.z*direction.z)); available=F(wishSpeed-projected). It returns unchanged
if available<=0. amount=min(F(F(seconds*acceleration)*max(stopSpeed,wishSpeed)),
available), followed by velocity[i]=F(F(direction[i]*amount)+velocity[i]).
The flag-bit3 branch instead accelerates toward a target velocity; that separate
mode is excluded from this grounded increment.

Walk chooses acceleration **9** standing, **12** crouching, **19** prone under its
ordinary predicate. Special surface/flags select **2**, and flag80 multiplies by
**0.25**. These are qualified code constants, not active sampled profiles. The
standing value9 is available for a future ordinary baseline; posture modes remain
for their own bounded implementation. No generic engine acceleration was used.

## Walk ordering and remaining scalar inputs

544200 first runs mode-dependent helpers0add80/01fac0, then2b1cd0/273a50 may
redirect to air. Their ordinary eligibility predicates remain to qualify. On
the admitted walking branch it applies friction, copies the command, computes
command scale540f60, and applies speed curve04dfb0. State+e4 is decreased by
truncate(seconds*-1000) and clamped to0. This damage/slowdown timer participates
in speed even when no new damage command is being applied.

The forward/right basis vectors are flattened and independently normalized with
SSE f32 sqrt/division. Direction is the normalized horizontal linear combination
of signed axes; wish speed is its original length times command scale. It projects
the desired direction onto the ground with14c9b0, chooses acceleration, accelerates,
possibly applies an additional current/field contribution05c5e0, runs53ddc0,
applies gravity for special surface/flags, projects authoritative velocity onto
ground, executes0246c0 with applyGravity=0, then updates movement orientation53e1b0.

Command scale540f60 uses integer base speed state+94, axis norm and denominator127;
backward and strafe parameters are distinct. It also reads movement scale state
+50c and weapon/stance/mode predicates. This is not simply axis/127*constant speed.
04dfb0 returns1 for timer0, otherwise uses configured slowdown parameters with
x87 arithmetic. Base speed, stop speed, backward/strafe scale, gravity and timer
parameters require qualified profile/state inputs. Parameter reads use298b90,
which obtains a float or integer from a dvar type and current value. No current
dvar object or registration default was sampled.

Static reference searches recovered stop-speed pointer VA31fc4b0 (read by both
friction and acceleration), backward VA31fe73c, strafe VA31fe83c, slowdown
VA31fe778/31fe67c and special friction VA31fe880. The bounded disk-reference scan
found consumers but no direct stores/registration/default association for those
slots. Their defaults are unresolved; recognizable strings do not bind values
to those consumers. A finite next investigation should qualify the registration
or initializer ownership for exactly these slots and state+94/+50c assignments.

## Ground projection differs from an overbounce clip

14c9b0 takes input velocity, ground normal and output. If abs(normal.z)<0.001
or horizontal speed squared is zero, it copies the vector. Otherwise it computes
candidateZ=-(normal.x*vx + normal.y*vy)/normal.z and
k=sqrt((vx*vx+vy*vy+vz*vz)/(vx*vx+vy*vy+candidateZ*candidateZ)).
It writes (vx*k,vy*k,candidateZ*k) when k<1, candidateZ>0 or old vz>0.
Otherwise it returns without writing; aliasing/output initialization at each
caller therefore matters. This preserves length while following an ascending
slope. It is not the contact solver's0df430 plane-constraint operation.

## Solver request, step selection and exact next contract

282630 constructs a request with source origin and velocity references, integer
gravity, explicit applyGravity integer, lo/hi references, mask, ignored owner,
seconds and ground-plane boolean. The latter is true only when applyGravity=0,
local ground-plane is present and flag-bit3 is clear. A movement-context reference
backs notifications. The request's table at RVA7d75ac resolves its first callback
to1caa80; that callback records touched-owner/event metadata. No opaque callback
target was guessed from a runtime address.

12ae20 calls2530d0, receives an inside/contact flag, attempts199760 with two
different correction selectors, and on failure marks the secondary-solve flag
and calls2530d0 again. Output carries origin XYZ, velocity XYZ and two flag bytes
at +18/+19. 0246c0 commits that first output provisionally before step comparison.
2530d0 builds contacts through27b1e0/00ff70, binds notification through the
qualified request callback, and calls0df430 for constraint resolution. Thus a
trace-only AABB interface is insufficient for this native transition.

0246c0 selects ordinary step height **18**, prone **10**. It retains initial and
first-solve origin/velocity, may test a raised endpoint using22ef70, computes
allowed rise=(height+1)*fraction-1, and rejects rise<1. The raised branch resets
origin/velocity to the saved initial state at allowed rise and solves via545d90.
It queries downward with07bf60 and compares both candidates' displacement
projected along initial horizontal velocity, using an additive0.001 bias. It
can restore the first-solve candidate. Additional ground-plane branches query9
units downward. Inside, owner-class and movement-mode branches affect selection;
the whole selection is not yet reconstructed as an executable algorithm.

The finite next collision/numerical prerequisite is the **contact-generation and
constraint contract**:27b1e0/00ff70 input/list schema, ordered contact normal,
fraction, penetration, body transform/activation/ownership,12bfa0 walkability,
199760 correction,0df430 active-set selection and2530d0 loop termination/output.
These exact bodies are externally retained. This task does not broaden that to
asset loading or every collision-family algorithm. Missing family/provider/pose
must remain an error rather than fabricated empty contacts.

Proposed reusable input fields, for primary review: GroundContact { normal,
surface_flags, owner, plane_contact, walking, near_ground, walkable }, plus
MoveSolveRequest { origin,velocity,gravity,seconds,lo,hi,mask,ignored_owner,
apply_gravity,ground_plane } and MoveSolveResult { origin,velocity,had_contact,
inside_correction_attempted,secondary_solve }. They are safe semantic proposals,
not final approved APIs. The motor state also needs base speed, movement scale,
slowdown timer and qualified mode predicates; profile contains stop speed,
backward/strafe scales and the evidenced coefficients. The motor must own ordered
numerical calls and transactional state commit; collision owns qualified access.

Independent checks passed: three acceleration cases (start, cap, already above
wish speed), two friction cases and an analytic ascending slope case mapping
(100,0,0) on normal(-0.6,0,0.8) to(80,0,60). Stop speed50/wish speed100/10ms were
synthetic inputs, not admitted defaults. No wall/stair/inside solver checks or
original comparisons were performed. Grounded motor implementation remains
blocked on the named solver, ordinary eligibility and parameter associations.

## Implemented independent increment: movement-native-05

`bo2-gameplay::movement::timing` now exposes validated, allocation-free command
plans and explicit single-step durations. The regular loop preserves backward
requests as ignored, the150ms discard and66ms maximum chunks; the independent
single-step clamp remains1..200ms with exact f32 seconds conversion. Signed
arithmetic outside the qualified non-wrapping domain is rejected by safe-boundary
policy. Server-owner time clamping and previous-command payload copies remain
caller work; there is no Bevy fixed-rate inference.

`movement::acceleration` implements only53dc10's ordinary flag-bit3-clear SSE
branch. It accepts supplied velocity/direction/coefficient values and the typed
duration, with no direction normalization or profile defaults. It preserves
strict stop>wish selection (else wish), uncapped>available replacement (else
uncapped), and increment+velocity operand order, including signed zero. Finite
input/intermediate/output guards and nonnegative coefficient restrictions are
safe API policy, not claims about native registration ranges or native errors.
Errors return no authoritative velocity; callers receive a fresh result to commit.

Twelve focused locked tests pass, including independent output-bit references,
a fusion discriminator, signed-zero selection, saturation, overflow/error paths,
explicit timing boundaries and1001 iterator conservation cases. The same12 tests
pass under standalone optimized compilation of the unchanged modules/tests.
Package all-target warning-denying Clippy, owned-file formatting and diff checks
pass. This increment does not implement x87 friction, slope projection, mode
predicates, collision, a full motor, an executable Origins profile or ECS queues.
