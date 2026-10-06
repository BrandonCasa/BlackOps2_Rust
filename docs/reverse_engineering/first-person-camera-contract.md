# Basic first-person camera boundary

Movement-host02 qualifies an angle/basis and supplied-eye adapter boundary from
licensed disk instructions. It does **not** establish the complete rendered
first-person camera, mouse translation, projection, or an Origins startup pose.
Fidelity remains **INVESTIGATING**. No original executable was run, attached to,
played, or captured.

## Reproducible source qualification

The admitted PE32 executable is 13,090,072 bytes, SHA-256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
This is the same licensed disk identity used by the movement and collision lanes;
it is not equated with the older Steam identity investigated in
[active-view task0025](0017-original-active-view-consumer.md).
Preferred image base is `0x400000`; the relevant text RVA/raw start is
`0x1000/0x400`. Translate RVAs through the PE section mapper, not arbitrary
pointer dereferences. The owned external `movement-host-02/native_qualify.py`
checks this hash before decoding and after its read-only session, walks each
reachable direct branch with a 64-KiB displacement/12,000-instruction cap,
checks nonoverlapping instruction bytes against disk, and retains ordered bytes,
disassembly, unresolved tails and calls externally. Calls are separate bodies,
not implicitly qualified by qualifying their caller.

The final external receipt admits 19 bodies/1,213 instructions/4,469 instruction
bytes, SHA-256 `a7295b45005ceff1e3fa6885be9b3167e97d747f6f61b24250d5c5402917714e`.
It includes angle owner `1aef70`, `53fb00`, `53fa10`, script height `424d30`,
the two distinct geteye table targets `4490e0/3c8140`, getplayerangles target
`421f50`, vector-to-angles `09e500`, eye dispatch `17aa00/2a9990`, angle dispatch
`0c2130/0f6b10/054d10`, basis `053dc0`, and eye consumers
`3016b0/21dc50/24f8a0/1058f0/04c400`.

| Body RVA | Instruction bytes | Ordered instruction SHA-256 |
| --- | ---: | --- |
| `53fb00` | 370 | `a55bf69439d18695264e13065309ed1072a995a8005e1691cf3aa7f6f890e70b` |
| `053dc0` | 431 | `7dc3e2ebd187ef41e2213c7c7d8f3a8a58af3334f61e31edb8a93224db23cd9e` |
| `0f6b10` | 18 | `e1f573911fa34440c2fbb1d009eb8838e0f5e75632467232d149dc938221d682` |
| `3016b0` | 367 | `1c8c09d438541eef2228a83fa4e32c61f2808dbec93cc8e99cd4ed45efae8699` |
| `424d30` | 112 | `db3d41108b2a44ded51ee6f56dec9057dff8973124036e3d6bce552ec3d988e8` |

`17aa00` has unresolved alternate tails `20c000/139070`; `2a9990` has an
unqualified error tail `2013a0`; `0f6b10` tails to the separately qualified
`053dc0`. Alternate eye callees, runtime table callbacks and external math/dvar
providers remain distinct unresolved dependencies. Consequently this receipt
does not qualify all camera modes.

## Authoritative angles and command translation

[The command owner](movement-command-update.md) calls `1aef70` from ordinary
`544b30`. Its basic branch calls `53fb00` with player state and command pointers.
The latter loops over **two** components: state floats `+1f8/+1fc`, using signed
command integers `+0c/+10`, state delta floats `+98/+9c`, and scalar conversion
`360/65536` (bits `3bb40000`, constant RVA `837198`). It does not update the
third state float `+200` in this loop. Commands are signed 32-bit fields in this
consumer, not a newly assumed u16 memory layout. The raw host mouse queue does
not already contain these command values.

For each component it converts the integer to f32, multiplies by the conversion,
adds its delta, then centers the angle using scalar SSE operations. The centering
uses bits `3b360b61` (rounded 1/360, RVA `83e854`), `3f000000` (0.5),
`4b000000` (2^23), a sign mask, and `3f800000` (1). The source constructs a
floor of turns plus 0.5 with add/subtract of signed 2^23 and a correction when
rounding exceeds the input; subtracting those integral turns and multiplying
by 360 produces the centered result. Ordinary finite command cases map +180
to -180 and ±360 to zero. This is a source instruction sequence, not permission
to replace it with Rust `%`, a differently rounded remainder, or an unbounded
floating-point-domain claim.

Component zero is pitch: upper limit is a runtime float accessor `298b90`
through global provider RVA `2dfc48c`; lower limit is the negative of the value
through provider RVA `2dfc4e0`. If exceeded, it changes state delta pitch to
`bound - converted_command_pitch` and uses the bound. Both components are
centered again before writing state. **Provider values/defaults and registration
bindings are unresolved; no ±85/±89 or other guessed pitch limits are admitted.**
`1aef70` subsequently has mode/flag gates and handlers which may alter angles;
`53fa10` includes a runtime table callback. The basic conversion alone does not
claim final angles for every special mode.

## Source angle-to-basis boundary

`0f6b10` adds `1f8` to its state argument and tails to `053dc0`.
`0c2130` uses this route when `054d10` rejects its alternate attachment branch.
`053dc0` reads angle components in order **pitch, yaw, roll**, multiplying each
by degree-to-radian bits `3c8efa35` at RVA `84e4e0` and using x87 `fsincos`.
Let `sp/cp`, `sy/cy`, and `sr/cr` be their sine/cosine pairs. Its three optional
output vectors are:

```text
forward = [cp*cy, cp*sy, -sp]
right   = [sr*(-sp)*cy + cr*sy,
           cr*(-cy) - sr*sp*sy,
           -sr*cp]
up      = [cr*sp*cy + sr*sy,
           cr*sp*sy - sr*cy,
           cr*cp]
```

These equations describe the orientation; exact implementation must preserve
the qualified scalar multiplication/addition sequence and separately address
x87 trigonometric precision. At zero angles forward is source +X, right -Y,
up +Z. Positive yaw turns forward toward +Y; positive pitch turns toward -Z;
positive roll rotates right toward -Z. Roll is a real basis input even though
the basic command updater only writes pitch/yaw. `421f50` reconstructs a
script angle return from a forward vector using `09e500`, whose roll output is
zero; that script return is not evidence that authoritative roll is always zero.

## Eye origin is a supplied authoritative result

`424d30` returns state **current interpolated height `+208`**. Stance target
`+204` and collision hull heights are separate concepts; see
[stance timing](movement-stance-contract.md). The geteye registration candidate
at RVA `944d40` points to `4490e0`; its receiver path reaches `17aa00`.
The other table target `3c8140` reads a separate client record and is not treated
as the same player-state path or proof of active rendered-view ownership.

In `17aa00`, the normal state branch reaches `2a9990` through entity `+154`.
`2a9990` dispatches special modes/attachments before its normal `3016b0` branch.
That branch copies state origin `+28/+2c/+30` and adds current `+208` to Z.
It then performs additional operations, in this order:

1. `21dc50` derives a phase from the low byte of state `+8`: multiply by rounded
   1/255, then rounded pi, double, add rounded 2*pi (constant bits respectively
   `3b808081/40490fdb/40c90fdb`).
2. Choose a speed argument: ordinary branch horizontal velocity magnitude from
   `+34/+38`; flag `+0c & 8` uses a time gate and vertical `+3c` instead.
3. `24f8a0` contributes vertical bob. `1058f0` contributes displacement along
   the **right** vector from the full state angles, not forward. Both select
   runtime vector parameters by target height, flags, state `+5b8`, and blend
   scalar `+1e0`, then use cap provider RVA `2dfe8a4`. Their bodies are decoded,
   but accessor `2c24c0` and runtime parameter values remain unqualified.
   Their final wave forms are respectively
   `(sin(4*phase + pi/2)*0.2 + sin(2*phase))*0.75*amplitude`
   and `sin(phase)*amplitude`; these use x87 arithmetic/sine.
4. Unless state `+fc & 4000`, `04c400` applies a lean-dependent offset using
   yaw `+1fc`, scalar `+90`, and explicit constants 16/20. The scalar’s producer
   and its intended valid domain are not established here. Zero scalar skips
   the offset; nonzero scalar builds angles `[0,yaw,(2-abs(scalar))*scalar*16]`
   and displaces along their right vector by `(2-abs(scalar))*scalar*20`.
5. Clamp output Z to at least state origin Z + **8** (bits `41000000`,
   RVA `81eee8`).

An unqualified bob parameter, attachment, mode, time source, or lean source is
an explicit missing input, not justification for silently returning the base eye.
The actual rendered view may apply additional prediction, recoil, damage,
camera effects or interpolation; no render publication chain is qualified here.

## Reusable Bevy adapter contract

The bounded adapter can receive a finite **final source eye position**, finite
authoritative pitch/yaw/roll in degrees, and a separately qualified projection.
It derives source basis, then applies the existing
[`source_to_bevy`](../../crates/core/src/coordinates.rs) rotation
`R([x,y,z]) = [x,z,-y]`, preserving current unit scale. Its Bevy world translation
is `R(eye)` and rotation matrix columns are
`[R(right), R(up), -R(forward)]`. This accounts for Bevy local -Z forward and
+Y up and retains native roll. It does not derive orientation from the previous
camera transform or derive simulation timing from render frames.

Local Bevy **0.19.1** `bevy_transform/src/components/transform.rs:466-488`
confirms `look_to` builds columns right/up/back with back = negative direction;
its malformed-direction fallbacks should be avoided by explicit finite/nonzero
input validation. `bevy_camera/src/projection.rs:284-287` defines perspective
`fov` as **vertical radians**. Bevy’s default projection is not a native value.
These are inspected installed dependency sources, not guesses about the game.

Only a `cg_fov` string/reference lead was located in the admitted disk:
string RVA `7ef590`, preferred-address reference RVA `208c70`. A decode begun
at guessed `208c50` starts in the middle of an instruction and is explicitly
**rejected as function qualification**; its retained output is not consumer
evidence. No native horizontal/vertical FOV convention, aspect transform,
near/far planes, dynamic FOV ownership, registration default, or active viewport
binding is admitted. A projection must remain a required unavailable input
until the relevant registration/consumer chain is qualified.

## Checks, limits, next prerequisite

The external independent `checks.py` passes **20** synthetic source-equation
cases: centered wrap boundaries, signed command conversion/delta, cardinal
pitch/yaw/roll signs, a non-axis orthogonal basis, and current-height versus
target-height base eye. Helper SHA-256 is
`37070d78c212df22331b5c84dda64c82d11782264bd999348103af25929b76e7`.
It uses explicit f32 stores for wrapping/conversion and Python math for basis
relations. Python sine/cosine does not reproduce native x87 instructions;
these checks establish source-equation consistency, not bit identity, original
comparison, camera rendering fidelity, or executable Rust integration.

The usable boundary is eye/basis conversion with explicit supplied inputs. To
assemble the basic camera, qualify pitch-provider registration/values, bob
vector accessor and baseline values, bob/lean state producers and time source,
render-view publication, raw mouse-to-command scaling/encoding, projection
ownership and Origins startup position/angles. Preserve their absence as typed
missing prerequisites. No Rust camera or command defaults were added.
