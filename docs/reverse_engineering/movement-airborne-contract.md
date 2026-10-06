# Ordinary airborne numerical boundary

Task movement-native-14, 2026-10-05. Fidelity INVESTIGATING. The admitted airborne
coordinator and scalar subsets are source-backed; the complete contact solve,
worker FP policy and baseline inputs remain unresolved. No original process,
instruction invocation, play or capture occurred. No Rust changed.

## Source and reproduction

Use [native identity](movement-native-contract.md), disk SHA256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`, image base400000.
PE mapping determines raw offsets. Resolve external roots through ignored
configuration. External `movement-native-14/qualification-receipt.json` contains
nine complete body identities, exact constant bits and15 synthetic f32 fixtures.
Candidate/padding spans were excluded; raw bytes/disassembly remain external.
Reviewed current timing, acceleration, sprint, jump eligibility and posture timing
kernels, [grounded03](movement-grounded-contract.md),
[jump06](movement-jump-contract.md), [FP11](movement-thread-fp-binding.md), source
contracts and current concurrent diff. Retained solver bodies are reused; contact
producer investigation remains with the collision lane.

| RVA | Admitted responsibility | Bytes |
| --- | --- | ---: |
| 543ec0 | Airborne coordinator | 826 |
| 53e0f0 | Separate airborne command scale | 185 |
| 2edf70 | Indexed gravity multiplier source | 20 |
| 1ea8b0 | Flag200000 vertical limiter | 118 |
| 05c5e0 | Optional spatial contribution lookup | 336 |
| 53d9b0 / 53ddc0 | Shared friction / surface-dependent velocity postprocess | 602 / 802 |
| 282630 / 2530d0 | Shared solver request / contact solve | 125 / 3244 |

## Call order and inputs

Single-step544b30 selects543ec0 when local walking+2c is zero after ground
classification. A successful ordinary jump in544200 also redirects to543ec0,
with launch clearing walking/plane/near-ground. This path is distinct from special
movement dispatch modes and does not add a guessed grounded-normal predicate.

543ec0 executes in this order:

1. Flag c200000 invokes1ea8b0's special vertical limiter.
2. It may replace signed gravity state8c through the supplied gravity provider
   and indexed multiplier described below.
3. Run shared friction53d9b0.
4. Read original signed i8 axes, copy60-byte current command, compute the separate
   air scale53e0f0, flatten/normalize forward/right basis and construct horizontal wish.
5. Call53dc10 with coefficient1.0 and wish speed. Flag-bit3-clear acceleration
   is the existing checked scalar kernel; bit3-set branch remains excluded.
6. Query05c5e0; if a contribution exists, accelerate again with its supplied
   vector/magnitude and coefficient2.0. No normalization is inserted here.
7. Run53ddc0 surface/flag-dependent postprocess.
8. If local plane+30 is nonzero, perform the exact plane correction below.
9. Execute0246c0 with applyGravity=1, then53e1b0 orientation bookkeeping.

Existing native command timing remains explicit: accepted subcommands <=66ms,
single-step clamp1..200ms and local seconds conversion from timing05. Air does
not select a Bevy fixed rate. Existing context hull, mask, ignored owner and
query environment are reused; no air-specific mask replacement occurs here.

## Gravity selection and ordinary friction

When state18 bit20 is set, source reads provider VA31fc4d8, multiplies through
x87 by literal0.3 (bits3e99999a), then66e3a0 converts and writes state8c.
Otherwise2edf70 receives owner state1a0: unsigned index<62 loads the f32 at
VA101db00 +4*index; indices>=62 return x87 1.0. If the returned finite modifier
is >=1, the coordinator leaves current gravity unchanged. If <1, it calls the
multiplier again, stores to f32, reads31fc4d8, multiplies through x87 and converts
through66e3a0. The second call/read is preserved, not cached by assumption.
The static62-entry table contains0/1 and hashes to
`6f0fa03c1a7116489451450ee7fe24b7e4e69879e2e9b1972b4598879a929f6f`;
its initialized active ownership/writes are not qualified. Disk zeros are not
an admitted runtime gravity profile. Provider defaults and FP/conversion policy
remain supplied dependencies.

For local walking0, flags20000 clear, state5b8==0, surface bit2 clear and mode!=5,
shared friction computes full speed using y*y + x*x + z*z in scalar SSE order.
Speed<1 clears all velocity components. For finite ordinary speed>=1 its drop
remains0, then it subtracts0, divides by speed and multiplies all components.
It does not apply the grounded5.5 coefficient merely because friction is called.
Special state/flags/surface/mode branches retain grounded03's dependencies.

Flag200000's1ea8b0 consults31e06c8/265360/31e06d8 and launch fields c0/c4;
vertical height/timing conditions can clear or restrict velocityZ. This is an
explicit excluded special path, not a baseline limiter.53ddc0 is a no-op when
surface bit2 is clear and flag40000000 is clear. Its other branches use additional
providers and current contact metadata; they cannot be silently skipped for
participating inputs.

## Separate airborne wish and scalar order

53e0f0 reads only signed command axes24/25 for ordinary air. Let a/b be their
exact i8 values, m=max(abs(a),abs(b)) and F denote each scalar f32 rounding.
Squared axis integers a*a+b*b fit i32. Source computes n=sqrt(F(a*a+b*b)); if
m==0 return positive0 before reading speed/flags. Otherwise:

`scale = F(F(f32(state94)*f32(m)) / F(n*127.0))`.

A reduced branch multiplies by literal0.4 (bits3ecccccd) when flags40 are set,
state90 is unequal to0, or flags200000 are set. The finite ordinary path uses
state90==0 and those flags clear. Mode5 further multiplies a supplied31fe6d8
provider through x87, outside this ordinary contract. There is no read of
movement scale50c, C96 movement modifier or grounded540f60's backward/strafe
providers on this scaler path. Original owners still establish state94.

The coordinator sets forward.z/right.z to positive0 and separately normalizes
both basis vectors. Forward length is sqrt(F(F(x*x)+F(y*y))); right length adds
F(z*z) after its x/y sum. Normalization uses divisor1 only for zero length and
otherwise length, then multiplies each component by F(1/divisor).

Wish x is F(F(right.x*f32(b))+F(forward.x*f32(a))); wish y follows the same right-
then-forward order. Its length is sqrt(F(F(y*y)+F(x*x))). Direction normalizes
x/y by F(1/length), using1 for zero length; direction.z is F(reciprocal*0).
Wish speed is F(length*scale). The first coefficient is literal1.0 (bits3f800000),
not a guessed air constant. Ordinary53dc10 still reads its supplied stop speed;
reuse acceleration05's strict selection/product/add order, not wish-only scaling.
Signed axis-128 remains-128; no normalization to-127 is admitted.

05c5e0 iterates a signed scene count VA2503220 and shape/model lookup structures.
AABB boundary equality rejects; interior plane equality permits. It returns the
first accepted record's three-vector and scalar from VA2504450 +16*index.
Its active records, traversal ownership and fallback59a740 receiver are unresolved.
A supplied false result can bypass the second acceleration; missing scene binding
must be an error in a full adapter, not an assumed absent contribution.

## Plane correction and gravity predictor

For local plane+30 nonzero, n is result normal+40/+44/+48 and v authoritative
velocity. Compute `dot = F(F(F(n.y*v.y)+F(v.x*n.x))+F(v.z*n.z))`, then
`adjusted = F(dot-F(abs(dot)*f32_bits(3a83126f)))` and negate the sign bit.
Each output is `F(F(n[i]*(-adjusted))+v[i])`, in x/y/z write order. This applies
for either sign of dot; no inferred incoming-only predicate, ground speed
preservation or generic overbounce replacement is used. Negative and positive
vertical10 both yield residual bits3c23d800 on a supplied unit-up plane.

Air supplies applyGravity=1 to0246c0 ->282630, so the request's ground-plane
constraint byte is false even when local plane was present. Request carries
origin/velocity refs, signed gravity, explicit seconds, current hull/mask/ignored
owner and qualified query environment. Existing step/candidate selection and
inside correction remain required; air does not substitute a simple position add.

At253154..253161 solver computes F(f32(gravity)*seconds), then
`endVz = F(oldVz-product)`. Before generating contacts,253176..2531e0 forms its
first query endpoint with `halfSeconds=F(seconds*0.5)` (bits3f000000):

`endpoint[i] = F(origin[i] + F(F(oldVelocity[i]+endVelocity[i])*halfSeconds))`.

End X/Y equal old X/Y in this prefix. This is a qualified query predictor, **not
an admitted final origin/velocity output**. The contact generator27b1e0 and later
constraint/inside/step branches may replace the candidate. No synthetic empty
receiver/no-contact path has been qualified. Output flags18/19 still have the
separate [access](movement-collision-access.md) meanings. Final no-contact result,
active receiver and constraints remain precise solver prerequisites.

## Candidate increment and validation limits

A future narrow air command-scale/wish kernel and supplied-plane correction can
reuse the established SSE domain. It must expose raw axes/state94/reduction
predicate and supplied basis/normal; actual profile/state and FP policy remain
external. A gravity predictor may expose this named prefix, not full flight.
Complete air update additionally requires optional contribution records,
postprocess eligibility, solver/contact/inside/step behavior and orientation.

Independent nearest-even f32 reconstruction passed7 scale,4 plane correction
and4 predictor fixtures, including axis0/127/-128, diagonal axes, reduced scale,
supplied negative/zero speed/gravity and66ms timing. These reconstruct operation
order within their explicit arithmetic model; they do not resolve native thread
rounding, denormals, x87 or exceptional domains. Nine complete spans were hashed
and decoded. Owned document whitespace/path/diff checks passed. No Rust/build,
original comparison, full airborne integration or executable baseline is claimed.
