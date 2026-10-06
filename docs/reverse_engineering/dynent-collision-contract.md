# Classic dynent collision dispatch and ownership

Status: **INVESTIGATING**. This qualifies one classic point-family dispatch, not
the player receiver, active scene, complete dynent geometry or hull collision.
Static disk inspection and an owned read-only OAT printer supplied the evidence;
no original instructions, game or process were executed.

## Reproduction and identities

The configured native executable has SHA-256
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`,
image base `0x400000`; code RVAs map through PE sections (`.text` RVA `0x1000`,
raw offset `0x400`). External task `movement-dynent-01/native_qualify.py` follows
reachable instruction boundaries through returns, records each admitted byte,
compares it to the same disk image and rejects unresolved decoding. Its aggregate
receipt SHA-256 is
`c7c30de8b5b93818625c6250612cc746a21ab83140b48b64924342aad52e8299`.

| Entry RVA | Instructions | Bytes | Role established from consumers |
| --- | ---: | ---: | --- |
| `0ba160` | 57 | 202 | Two-class dynent query wrapper |
| `406210` | 193 | 677 | Sector/link traversal, mask and family dispatch |
| `0b3080` | 5 | 20 | Server-record address from class and ordinal |
| `130ae0` | 3 | 8 | Definition contents |
| `180940` | 100 | 381 | Quaternion/local XModel point query and promotion |
| `14ba60` | 43 | 128 | Quaternion/transformed cmodel point query |
| `2f7c20` | 6 | 25 | Sector-bank record address |
| `1c5840` | 63 | 274 | Quaternion-to-matrix arithmetic |
| `1ef4b0` | 37 | 160 | Subtract-origin then row transform |
| `007fe0` | 10 | 33 | Definition-pointer-to-ordinal conversion |

All ten bodies total 517 instructions/1,908 bytes, with no unresolved instruction
or indirect call. Existing [static triangle](static-xmodel-collision-contract.md)
and [transformed model](transformed-submodel-trace-contract.md) evidence qualifies
the called `00dfd0` and `27c640`; they are not new dynent geometry implementations.
Disk constants used here are `2.0` at VA `bfc42c`, `1.0` at `bfdd24`, zero at
`d1ffa8`, and a three-word zero hull at `c786a4`.

Retained OAT revision `7d027e8` declarations in `T6_Assets.h` and `clipMap_t.txt`
separately describe definition, runtime-virtual pose/client/server/collision
arrays. An owned copy of the accepted printer/build/supervisor performs only
`--list --no-color` decode of the configured Origins fastfile, validates array
memory bounds and finite seven-word definition placements, and publishes bounded
pointer-free metadata. It does not extract asset names, geometry or scripts.
Input: 78,753,344 bytes, SHA-256
`663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5`.
Published metadata: 12,515 bytes, SHA-256
`6c6ab9bc7610e1afe6a656e335981a35f474304c0f085c0f1e9f2bc3ecba4266`.
Source hash before/after is identical. Decoder/library originals and accepted
schemas were not edited; all code, logs and licensed metadata remain external.

## Query dispatch, ordering and filtering

`2db280` dispatches this family through `0ba160`; the latter returns when incoming
fraction is nonpositive, clamps fraction greater than one to one, then invokes
`406210(class=1,node=1)` followed by `class=0,node=1` if fraction remains nonzero.
Its four-component traversal endpoints contain the original request XYZ, with
start W zero and end W the incoming fraction. XYZ is not shortened to that fraction.
No hull expansion is present in this sector walk.

For class `c`, `2f7c20` obtains a 20-byte node at
`VA17ff7ac + (c+2)*0x501c + node*20` (inline absolute bank via LEA,
not a dereferenced pointer). Node zero terminates. Node contents at
`+0c` must intersect request mask `+34`; then head `u16 +10` is traversed in
stored order. Head/next are **one-based**, zero terminates, and each nonzero value
is decremented and zero-extended before indexing. Collision records use
`[VA25039b0+c*4]+ordinal*32`, next `u16 +2`; definitions use
`[VA2503988+c*4]+ordinal*84`. `0b3080` computes the server address
`[VA25039a0+c*4]+ordinal*8`; this computes an address, not a contents read.
Definition contents `+4c` must also intersect the mask before family dispatch.
No definition type/flags, client flags or independent enable bit is tested in
these bodies. Activation is therefore supplied through the linked sector state,
contents and current definitions; construction/mutation of that state is missing.
Fraction zero after a candidate suppresses remaining entries and classes.

After local entries, node split `f32 +0`, axis `u16 +4`, and child ordinals
`u16 +8/+a` govern traversal. Let `a=start[axis]-split`, `b=end[axis]-split`.
For `a*b>=0`, `min(a,b)>=0` chooses child `+8`, otherwise `+a` (including an
endpoint exactly on the split). For a crossing, start W greater than or equal to
current result fraction prunes the branch. `a/(a-b)` interpolates all four
components as `start + ratio*(end-start)`, with separate scalar stores.
The first child is `+8` when `b<0`, `+a` otherwise; recursion receives start to
intersection. A nonzero fraction continues intersection to end in the other
child. Native reads no declared count here, no index-range guard, cycle check or
count bit mask. Safe admission must obtain valid activation/index bounds rather
than inherit these unchecked reads.

## Geometry and pose consumers

Class zero selects definition XModel `+20`. It instead selects destroyed XModel
`+24` only when that pointer is nonnull, definition health `+44` is positive and
server health `+4` is nonpositive. Server health is dereferenced only for this
selection gate. The actual supplied placement is **definition `+4`**, consisting
of quaternion XYZW then origin XYZ; neither runtime pose array `VA2503990` nor
client array `VA2503998` is read in the admitted dispatch bodies. This is a proven
query input, not a claim that a serialized initial placement is the active pose.
The owner that updates/rebinds definitions and sectors remains unqualified.

`180940` returns without querying when request field `+38` is zero. Its broader
owner/meaning must be preserved as a supplied gate, not guessed from a name.
It builds a matrix through `1c5840`, inserts origin in the last three words, and
transforms the original request start/end through `1ef4b0`. For each row `r`,
after component subtraction `d=world-origin`, the source adds
`(d.y*r.y + d.x*r.x) + d.z*r.z`. No hull is supplied to `00dfd0`; its existing
LOD/surface/mask/triangle ordering and allowances apply.

For quaternion `(x,y,z,w)`, define `xx=x*(2*x)`, `yy=y*(2*y)`, `zz=z*(2*z)`,
`xy=y*(2*x)`, `xz=z*(2*x)`, `yz=z*(2*y)`, `wx=w*(2*x)`, `wy=w*(2*y)`,
`wz=w*(2*z)`, retaining each native f32 operation. The matrix rows are:

```text
[1-(zz+yy), wz+xy, xz-wy]
[xy-wz, 1-(zz+xx), yz+wx]
[wy+xz, yz-wx, 1-(yy+xx)]
```

There is no quaternion normalization or scale/orthogonality validation. A later
safe adapter must retain source arithmetic or declare a rejected input boundary.

**Distinct result gate:** `180940` promotes only when `00dfd0` returns exactly
zero, not any nonnegative bone index. A positive nonzero bone or negative return
can leave triangle-consumer local writes without dynent owner/normal promotion.
On zero, the transpose normal uses `(nx*A0j + ny*A1j) + nz*A2j` for each component;
length sums `(ny*ny + nx*nx) + nz*nz`, then square root. A nonpositive length uses
denominator one, otherwise division normalizes. Result type `+1c=2`, ordinal
`u16 +20` comes from `007fe0(definition,class=0)`. `00dfd0` hit writes clear
startsolid/allsolid bytes `2b/2a`; this adapter does not recompute `2c`.
Do not substitute the existing static adapter's broader promotion gate.

Class one calls `14ba60` with the same supplied definition placement. It builds
the quaternion matrix and calls `27c640` with original endpoints, zero mins/maxs,
unsigned definition brush selector `+28`, request mask and supplied origin/matrix.
The existing transformed-model centering/normal rules apply. Only a strictly
smaller fraction publishes dynent result type `3` and class-one definition ordinal;
equal fraction retains owner identity. No separate stationary or capsule family
entry was established here. Zero endpoints can reach the supplied cmodel consumer;
that does not establish full dynent stationary/hull query support.

## Retained records and count discrepancy

OAT declares original count `u16=19` and four `u16` counts `[275,0,19,0]`.
Its definition/pose/client arrays 0/1 use counts 0/1; server arrays 0/1 use counts
2/3; four collision arrays use the four counts. These are **decoder declarations**,
not four proven native query classes. Native sector bank `class+2` is likewise
not evidence that count-list 2 is an independently traversed definition family.

The bounded decoder reads all 275 declared class-zero definition records:

| Retained records | Observed associations | Disposition within this snapshot |
| --- | --- | --- |
| Ordinals 0–18 | Type 1, definition contents 1041, one shared nonnull resolved XModel | Provider candidates; active sector/index/pose binding missing |
| Ordinals 19–274 | Type 0, contents zero, null base and destroyed XModel | Zero-content snapshot records; no runtime exclusion claim |
| Class-one definitions | Declared count zero | No retained definitions, not an exclusion of all runtime class-one activation |
| Server/collision counts 2/3 | 19/0 | Runtime allocation declaration; native initialization/mapping unresolved |

All definitions have zero brush/physics-brush selectors, no destroyed-model
reference and no positive definition health. Runtime pose/client/server pointer
presence is `[present,present,present]` for array zero and absent for array one;
pointer presence is not valid active contents. The sole resolved shared provider
has contents 17, signed collision LOD 2, two classic surfaces, 377 triangle records
and one collmap. These geometry records are available through the decoder but were
not extracted/admitted in this task. Collmap geometry is not implemented by the
classic triangle consumer and remains a participating-provider question.

The difference `275 = 19 + 256` is measured, not an admitted encoding rule.
The admitted native route never reads those counts or masks an ordinal with 255.
It directly indexes one-based sector links; a bound tied to the smaller server
allocation must come from activation/initialization evidence. No claim is made
that all 275 are native population, that the zero suffix can never mutate, or that
the retained runtime-virtual arrays contain a valid active scene.

## Independent checks and required next inputs

External `source_checks.py` performs 27 synthetic equation/gate checks: identity,
half-turn and nonunit quaternion matrices, translated endpoints, destroyed-model
gates, masks, class-zero suppression after fraction zero, exact bone-zero
promotion and split-child choice. A separately f32-rounded non-axis fixture gives
local endpoint words `[1058168408,3201817050,1055215216]` for quaternion
`(.17,-.23,.31,.87)`, world `(.81,-.37,.69)`, origin `(.27,-.13,.09)`.
These are independent source-flow checks, not native execution or comparisons.

Required before active scene admission: native initialization/rebinding and count
semantics; valid sector heads/children/contents and collision next links; current
definition placement mutation/activation ownership; conditional server health
ownership and range; provider identity/admission for the two classic surfaces and
collmap; request `+38` and player receiver binding; dynent hull/capsule and additional
query dispatch. Missing inputs must yield explicit provider/pose/activation/index
errors, not zero placement, empty sectors or skipped family defaults.

See [global ownership](collision-global-ownership.md),
[player world contract](player-world-contract.md),
[terrain contract](terrain-trace-contract.md) and
[handoff](../handoffs/movement-dynent-01.md).
