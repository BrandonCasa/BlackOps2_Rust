# Static XModel collision contract

Status: INVESTIGATING. This qualifies the classic point-segment family and
retained provider availability. It does not qualify the alive player motor's
separate shape backend, stationary player hulls, or original comparison fidelity.
See [player world](player-world-contract.md),
[global ownership](collision-global-ownership.md), and
[motor collision access](movement-collision-access.md).

## Evidence and procedure

Static disk executable identity: 13,090,072 bytes, SHA256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
Preferred image base `0x400000`. RVAs below identify that disk build; equivalence
with another installed executable has not been proved. No original code was
executed, process accessed, or game played/captured.

The owned external `movement-static-model-01` task root contains
`native_qualify.py`, per-entry instruction records/disassembly, and
`latest.private.receipt.json`. The bounded Capstone walker admits all reachable
direct branch targets within 64 KiB, stops on returns/traps, verifies instruction
bytes against PE disk mapping and rejects overlapping instruction boundaries.
Nineteen complete bodies contain 2,222 instructions / 8,384 instruction bytes.
One runtime-entity callback remains indirect; the qualified static family has
no indirect calls. Instruction-stream hashes cover concatenated reachable
instructions ordered by RVA, not a continuous function span.

| Entry RVA | Qualified role | Instruction-stream SHA256 |
| --- | --- | --- |
| `1d73d0` | Classic static point wrapper | `f10d0a7650c56797e14df98b5c31bdeaca41a5840fa358323fce88215710c0b4` |
| `486bf0` | Static sector moving-point traversal | `00904d596b4e9d1af84a20e3f50153cafb046b2a9168b2158644943994b75f28` |
| `29acc0` | Instance transform and result promotion | `c4b366b4c2accda2d3657bd3f0e43cae1d2369dd3b91fcaaeb472d6febe5545c` |
| `00dfd0` | Ordered XModel collision-surface triangle consumer | `348da3e386ac5f482ae7a5caa5c83306597e9d74a53948730d582095efa252d4` |
| `1831e0` | Point-segment versus bounds rejection | `bdce8b10cab90a3644fb01ce5866d6f35f57c445ea2a003957f6823cdee190d0` |
| `275950` | Explicit inverse endpoint difference | `ae08c91bb857b678baa4287e113a7b12e2b9cdd44d90165196372d60ab70e236` |
| `486940` | Static ordinal insertion and sector refinement | `8281e995da1b335ca8039075cd50c9f51366b8e7d51c8d77cff778ffcd1212e4` |
| `486550` | Sorted one-based static linked-list insertion | `4147271cfc87dbc84a31e33a0df706996d74567052d9f7008885af06047e013e` |
| `4865b0` | Sector child construction / instance relocation | `480eac11c3306bac6595853caee95075c657e9a3545c096bb208c53094af3933` |
| `486380` | Sector free-list allocation | `6a803af7e668c97ac7d0197a31ff29b93717612c91bb13871417d0b010c40e5c` |
| `486e50` | Boolean static point-segment traversal | `76b3f24479d9dbf09657d4620ce594e90fd30ef60a9facdb42523eefe1f4f28a` |
| `2db8a0` | Boolean transform wrapper, temporary fraction 1 | `eaef4ad5c65eb439abc2c7092c297aabab570f46c2d9edb2d97b7a0d90837dfa` |

Also qualified complete `2db280`, `2d5a60`, `487610`, `19cd80`, `0ba160`,
`406210`, and `1e4ae0` to establish family boundaries. Their individual hashes
and calls are retained externally. Source declarations / generated loader rules
are OAT revision `7d027e8f89118196713e955b0e11f8404149c54d`, T6_Assets.h,
clipMap_t.txt and XModel.txt. No proprietary bodies or payloads are tracked.

## Dispatch and ownership

The initial leads were misleading family names: `2d5a60 -> 487610 -> 19cd80`
reads runtime entity storage at globals `0x28c0698/0x28c069c`, contents `+118`
and entity flags `+fa`. `487610` can call query callback `+24` with entity ordinal.
`0ba160 -> 406210` consumes the two dynent classes' arrays through clipmap
globals `0x2503988/0x25039b0` plus selector stride 4. Neither is the static pool.

The actual classic static branch is `2db280`, when options `+18` is nonzero,
calling `1d73d0 -> 486bf0`. It runs after the world result and before runtime
entities/dynents/refinement, unless fraction zero short-circuits the wrapper.
The static pool pointer is global `0x25038d8`, clipmap base `0x2503880 + 88`;
count at `0x25038d4`, base `+84`. The 84-byte instance agrees with the source:
writable next-link u16 at 0; XModel pointer `+4`; contents `+8`; origin `+c`;
inverse-scaled axes `+18..+38`; absolute bounds `+3c/+48`.

Sector storage is a runtime 24-byte array at `0x25046cc`. Static aggregate mask
is `+0`; first static one-based ordinal is u16 `+a`; split coordinate `+c`,
axis u16 `+10`, child ordinals `+14/+16`. Traversal begins at sector 1. It
rejects aggregate masks before visiting the list, follows the instance's
one-based next link, rejects bounds using the incoming authoritative fraction,
and calls the instance consumer. A zero instance-contents gate computes an
extra mask-bit-1 rejection; actual positive instance masks are aggregated by
the constructor, while each collision surface applies its own contents mask.
Do not replace these distinct gates with an invented single provider mask.

Splits visit the start-side region first, interpolate XYZ and time fraction
at the split in f32, then visit the other side; later regions reject when their
start time is at least the current hit fraction. Within each sector list,
`486550` inserts ascending original instance ordinal. `486940` iterates source
ordinals ascending and skips instances with contents zero. `4865b0` may relocate
whole-bounds instances to children, preserving sorted lists and aggregate masks.
Initial world bounds, root/free-list construction and invocation ownership
remain to be qualified before claiming native full-scene ordering. All retained
serialized next links are zero: they are loader inputs, not ready runtime lists.

## Transform and triangle numerical boundary

For each world endpoint, subtract instance origin, then compute local component
`j = sum_i(delta[i] * invScaledAxis[i][j])`. These are matrix columns, with f32
multiplications/additions in the admitted instruction order. Do not substitute
Euler angles, a guessed uniform scale, or the display mesh transform.

`00dfd0` rejects XModel signed `collLod` at `+c6` when negative, then visits
`collSurfs` at `+98`, count `+9c`, in stored order. Each 44-byte surface contains
triangle pointer/count `+0/+4`, local bounds `+8/+14`, bone index `+20`, contents
`+24`, flags `+28`. Contents must intersect the query mask. The surface bounds
must admit the segment before its ordered 48-byte triangle records are read.
Triangles contain plane, s-vector and t-vector, each four f32 words.
This consumer reads neither render surfaces nor PhysGeomList collmaps.

For finite endpoints, let `a = dot(start, plane.xyz) - plane.w` and similarly
`b` for end. It requires `a > 0` and `b < 0`: backfaces, endpoints exactly on
the plane, and stationary points do not hit. Candidate fraction is
`max(0, (a - 0.125)/(a-b))`. It must be strictly less than incoming fraction.
Barycentric coordinates use the actual plane intersection `a/(a-b)`, not the
offset fraction. Let `u = dot(intersection,s.xyz)-s.w`, similarly `v` from t:
admit `u >= -0.001`, `u <= 1.001`, `v >= -0.001`, `u+v <= 1.001`, using the
exact f32 words `ba83126f` and `3f8020c5`. Plane offset 0.125 is `3e000000`.
Equal-fraction ties retain the first ordered hit.

The consumer writes fraction `+10`, flags `+14`, contents `+18`, local normal
`+0..+8`, and u16 `+2a = 0`, returning the hit surface's bone index. Negative
return denotes no hit. `29acc0` promotes a successful nonnegative return:
static owner pointer `+30`, owner-kind `+1c = 1`, owner ordinal `+20 = 0x3fe`;
normal is transformed by the inverse-scaled matrix rows and renormalized with
f32 sqrt/divide (zero length denominator replaced by 1). It does not populate
a per-instance ordinal in this classic result.

Neither admitted static triangle consumer writes inside/allsolid flags, nor
does it provide capsule, swept AABB, or stationary overlap behavior. The boolean
variant `486e50 -> 2db8a0` uses the same point triangles and returns early on a
hit with a temporary fraction initialized to 1. Calling it with equal endpoints
does not establish stationary hull clearance. Source proofs for the motor's
shape/candidate-to-XModel binding remain in the collision lane.

## Exhaustive retained provider availability

A copied, source-only OAT adapter in owned external `movement-static-model-01`
reads only the already decoded map static instance array and its referenced
XModel collision metadata. It preserves source/helper originals, emits no asset
names or host pointers, and never executes original instructions. The fastfile
is 78,753,344 bytes, SHA256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
Its hash and the prior terrain producer/decoder hashes were checked unchanged
before/after the run. The original accepted schemas 33/34 and terrain payload
were not changed.

The bounded metadata probe checks known instance count, finite origin/axes/
bounds and ordered bounds, nonnull XModel references, exact pointer association
to decoded pool assets, reference-only status, bounded surface/triangle/collmap
arrays and finite collision triangle words. Caps: 100,000 zone assets, 65,535
instances, 65,536 cumulative surfaces, 1,000,000 cumulative triangles/geoms,
8 MiB final metadata, 2 MiB diagnostic streams, 120-second supervised deadline.
The output schema is investigation metadata, not an admitted Rust payload.

| Retained disposition | Count |
| --- | ---: |
| Original static instances checked | 10,837 |
| Distinct first-use ordered XModel providers | 420 |
| Decoded nonreference providers | 420 |
| Missing / reference-only providers | 0 / 0 |
| Collision surfaces / collision triangles checked | 510 / 78,296 |
| Providers with no surfaces or triangles | 0 |
| CollLod 0 / 1 / 2 / 3 providers | 53 / 189 / 136 / 42 |
| Nonzero instance contents, at least one matching surface | 10,627 |
| Zero instance contents, skipped by classic constructor | 210 |
| Nonzero serialized next links | 0 |
| Collmap records with decoded geomList | 2 |

Instance contents histogram: 1: 10,140; 16: 150; 17: 285; 0: 210; 3: 52.
The two collmaps each expose one geom metadata record, contents 536,871,936 and
1,024 respectively. Their geometry consumers have not been qualified here.
The 210 zero-content exclusions apply only to this proven classic constructor;
they do not waive an unresolved participating motor backend family.

External metadata is 3,209,708 bytes, SHA256
`d322619cd3b97284006f0673e1b3fc817cfaf6a41528b34a1e5752213ee8271b`.
It preserves every instance ordinal, provider association, exact pose/bounds
words, contents and serialized next; providers retain collLod, content and all
surface/triangle counts and collmap presence. Triangle geometry was validated
in decoded storage but not emitted by this bounded metadata task.

## Implementation prerequisites and validation limits

A bounded assets reader can be based on all ordered instances and 420 shared
providers, preserving exact origin/inverse-axis/bounds words, signed collLod,
ordered surface bounds/bone/contents/flags and complete plane/s/t records. Bind
the scene's provider IDs and original ordinals through explicit source identity,
not host pointers or asset names. Runtime sector construction/order requires a
separate scene contract; serialized next zero is not a missing-geometry error
or blanket exclusion. Resolve motor backend association and hull/overlap
semantics before exposing this point consumer as `PlayerCollisionWorld`.

External `synthetic.py` passed 16 independent f32 point/transform cases: front/
back crossing, stationary/on-plane endpoints, surface masks, incoming-fraction
ties, ordered ties, barycentric miss/tolerance edges, zero clamping, scaled
rotation, normal renormalization, and shared provider with distinct origins.
These exercise a separately written mathematical flow and expected synthetic
results; they do not run native code or prove original trace equivalence.
No Rust family implementation or original comparison was performed.
