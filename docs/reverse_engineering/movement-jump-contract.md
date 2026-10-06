# Ordinary jump eligibility and launch boundary

Task movement-native-06, 2026-10-05. Fidelity INVESTIGATING. This describes
source-qualified transitions, not a complete jumping controller or an original
comparison. No original process access, invocation, play or capture occurred.

## Reproduction and ownership

Use the identity and PE mapping in [movement-native-contract.md](movement-native-contract.md).
External `movement-native-06/qualification-receipt.json` binds retained complete
bodies and synthetic checks to RVAs. Resolve the external root through ignored
configuration. Disassembly, executable bytes and helper scripts remain external.
The shared command and grounding contracts remain
[movement-command-update.md](movement-command-update.md) and
[movement-grounded-contract.md](movement-grounded-contract.md).

| RVA | Boundary | Bytes |
| --- | --- | ---: |
| 2b1cd0 | Jump eligibility and launch owner | 243 |
| 53b610 | Launch writes and repeated-jump modifier | 281 |
| 53b8c0 / 20da70 | Jump event / surface parameter | 57 / 46 |
| 287740 / 0fd290 / 104c10 | Ordinary ground owner permission | 63 / 29 / 33 |
| 0d5e60 | Clear jump flag and launch height | 23 |
| 0af620 | Jump slowdown factor and expiration reset | 144 |
| 53e5a0 | Landing consequence owner | 1092 |

The last 0af620 body ends before adjacent function 0af6b0, without padding.
An automatic padding-delimited span alone is not a function boundary. The
exploratory 53b900 body is unrelated and is not admitted as a timer updater.

## Eligibility and command edges

Ordinary walk 544200 calls 2b1cd0 before friction. A true return selects air
543ec0 immediately. Conditions are evaluated in this order:

1. Reject state flags `0x80000`.
2. Subtract last jump time (state c0) from current command time (context 4),
   compare signed result against 500, and reject below 500 milliseconds.
3. Reject flags `0x400`, then low flag `4`, then signed movement mode >=10.
4. Reject nonzero effective posture (1a3920) when ground owner a4 !=3ff.
5. Require current command low button word bit21 (context 8).
6. If previous command low word bit21 (context 44) is set, clear bit21 in the
   current word and return false. Earlier rejections do not perform this clear.
7. Reject ground surface flags (local 54) bit1 (`2`).
8. If ground owner !=3ff, require its environment permission callback nonzero.

The signed subtraction machine overflow domain remains unqualified; a checked
Rust boundary should reject overflow rather than claim native wrapping policy.
The edge refers to the previous native subcommand copied by 23af10, not previous
render input. Release then press, held press, and rejection have distinct effects.
No extra ground-normal threshold or guessed grounded-only predicate is admitted.

For ordinary handler byte1 the callback table entry at VA101e7b4 is VA687740
(RVA287740). It addresses owner storage VA21c13c0, stride31c. Reject when owner
154 is nonzero; reject when owner word d8 is6 and byte70 bit0 is set; reject when
owner158 is non-null and its referenced object c equals1. It otherwise permits.
Owner bounds, lifetime and live binding remain supplied evidence requirements;
these reads must not become unchecked Rust indexing. Other handler rows are not
qualified by this task. Owner sentinel3ff bypasses the callback entirely.

## Launch state and numerical order

Owner supplies height literal39.0 (VA c506fc, bits421c0000). Launch53b610 first
multiplies height by2.0 (bits40000000), converts signed gravity state8c to f32,
then multiplies converted gravity by that product. In the first-launch path
(jump flag4000 clear), sqrtss of this f32 produces velocityZ; there is no added
old velocityZ and no guessed gravity default. Conversion and sqrt require an
admitted floating-point rounding domain; moderate synthetic values avoid integer
conversion ambiguity, but do not establish engine startup FP policy.

If flag4000 is already set and signed state timer1c <=1800, boolean provider
31fc410 controls a divisor. False gives1. True with timer>=1700 gives2.5
(bits40200000); otherwise use ordered f32 operations
`((f32(timer)*1.5)*f32_bits(3a1a33cd))+1`. The squared impulse is divided by this
modifier before sqrt. Provider registration/default and timer progression are
not qualified, so this repeated-launch branch is not an executable baseline.

After the impulse calculation, launch writes in source order:

- local ground plane30, near34 and walking2c become0;
- ground owner a4 becomes3ff;
- last jump time c0 becomes current command time;
- launch height c4 receives originZ30 (x87 load/store, no arithmetic);
- velocityZ3c receives sqrt result;
- flags clear180 and set4000; timer1c and sprint latch4b8 become0;
- state588 receives its old value plus float provider31fc40c through x87,
  capped above255.0 (bits437f0000).

State588 is a player jump accumulator, distinct from weapon definition field588.
Provider association, active value, x87 control/rounding and exceptional-value
semantics remain unresolved. A full atomic launch must account for that write;
implementing just the impulse must be named a numerical kernel, not full jumping.

Owner next emits jump event88 when its parameter is nonzero. Ordinary parameter
20da70 is0 for surface flag2000, otherwise surface bits20..25, replaced by20
when state5b8!=0. Flag8 uses parameter15 and then special branch53b730, which is
outside this ordinary path. The owner finally calls131f30 with transition3 and
local9c; event delivery and its receiver are not numerically reconstructed here.

## Air and landing dependencies

Successful walk-to-air dispatch reaches the previously retained air543ec0 and
step0246c0 collision solver; this task adds no substitute gravity integrator.
Ground coordinator542850 selects landing owner53e5a0 when a qualified contact
is walkable and previous owner was3ff, before assigning the new owner. It does
not unconditionally clear flag4000 on ordinary landing. Nonwalkable contact and
inside recovery can instead call0d5e60, which clears4000 and writes c4=0.
Slowdown factor0af620 also clears4000/c4 when timer1c>1800; it otherwise reads
31fc410 and uses the same1/2.5/interpolated factors. Timer advancement still needs
its actual owner; the neighboring53b900 is not that owner.

Landing53e5a0 reconstructs impact using local prior originZ8c, prior velocityZ98,
current originZ, signed gravity, and ordered SSE roots/products; then calls
threshold providers2209e0/15b3c0, adjustment2dcfa0, gravity provider31fc4d8,
perk/environment consumers and x87 conversion66e3a0. Depending on consequences
it emits landing events, multiplies all velocity components and may set flag80
and timer1c. Treating all landings as merely zeroing vertical speed would discard
observed behavior. Full landing equations/thresholds, provider defaults and timer
owner are a separate finite qualification task, not admitted by the launch checks.

## Candidate executable increment and validation

A narrow checked `first_jump_impulse(gravity: i32)` can expose the source literal
and exact separate f32 multiplication/sqrt boundary for explicitly admitted
conversion domains. A pure eligibility evaluator can expose command mutation
(clear-held), rejection or launch authorization with supplied posture, surface
and owner permission, preserving short-circuit order and checked timing. Neither
is a full motor or an executable Origins profile. Do not infer active gravity,
provider values, unit defaults, ground ownership or original timing from Bevy.

Independent external reconstruction passed15 eligibility cases: 499/500/501ms,
button absent/held, early rejection retaining a held bit, forbidden flags/mode,
posture plus owner sentinel, forbidden surface and callback rejection/bypass.
Five moderate synthetic gravity inputs yielded impulse bits:

| Synthetic gravity | Squared impulse bits | VelocityZ bits |
| ---: | --- | --- |
| 0 | 00000000 | 00000000 |
| 1 | 429c0000 | 410d4ee4 |
| 100 | 45f3c000 | 42b0a29e |
| 800 | 4773c000 | 4379ccc8 |
| 1000 | 47985800 | 438ba474 |

These are independent source-flow fixtures, not original comparison evidence or
proof of the full landing/air pipeline. No Rust changes were made in this task.
