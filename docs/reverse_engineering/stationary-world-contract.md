# Native stationary brush and world clearance

Collision lane 02, 2026-10-05. **INVESTIGATING** for source interpretation;
player/full-map fidelity **UNKNOWN**. Static licensed-file inspection only.
This qualifies a separate stationary brush consumer; it does not establish a
complete collision scene or native execution comparison.

## Source identity and reproducible procedure

The licensed disk image SHA256 is
`2aff340f22f0c68bf46511c7bbc6167e4bb84524817f54cae08e8f60e6f9afc3`.
PE image base is `0x400000`; text begins RVA `0x1000` at file offset `0x400`.
Native lane independently established that all fifteen task0035 qualified spans
match this disk image byte-for-byte. This task decodes bounded reachable blocks
from the entries below, retaining raw bytes/disassembly externally. Direct branches
must land on admitted instruction starts. Return blocks and the plane helper's
explicit invalid-signbits trap terminate block decoding. The helper's eight-way
indirect jump is resolved from its own eight direct initializer stores, not a
guessed jump table. All six spans rehash against a separate disk reread; whole
image hashes before/after agree. No native code is executed or process opened.

| Entry RVA | Span bytes | SHA256 | Role qualified from callers/data flow |
| --- | ---: | --- | --- |
| `0x483790` | 255 | `0970bcfd46ffbd7a57d2e4d53298f51b89ce427c1d3711c49e5a31185037807b` | Stationary brush overlap |
| `0x483890` | 234 | `71edec7218193cacfa1dca45c9faec7b745f7dbdf24ce67a494f61a82adfb273` | Stationary node recursion |
| `0x483980` | 303 | `3774db61691c6700b1dcf8c4f56d074ca8e56e1252533fbd076aa00c7e611f00` | Leaf bounds/node consumer |
| `0x483cf0` | 424 | `082fbb22e01c389fde5ef3dfe9b747bd8c02af9d2faf7e6ebc333a943cc430fb` | Stationary world dispatch |
| `0x1c02e0` | 150 | `128a73b352474ca45091ae21032a538385aa3999ddd23aadcc35bb724f114381` | BSP leaf enumeration |
| `0x11520` | 567 | `2ae386720a15dc48ba92d1b53759ba66daad4f5b7e30dfc1a2c4ef61399def2a` | BSP bounds/plane classifier |

## Stationary request and brush arithmetic

The already qualified builder chooses this route when all three centered
endpoints compare equal. It retains source-space center `p`, radius
`r=min(extent.x,extent.z)` and half segment `h=extent.z-r` from
[moving construction](0024-original-brush-trace-consumer.md). The stationary
capsule bounds are `(p.x-r,p.y-r,(p.z-h)-r)` and
`(p.x+r,p.y+r,(p.z+h)+r)`, with each operation separately rounded to f32.
These are not the supplied general AABB: differing Y half extent does not change
capsule radial bounds.

Brush overlap first rejects, in lower X/Y/Z then upper X/Y/Z test order, if
`queryMin[axis] >= brushMax[axis]` or
`brushMin[axis] >= queryMax[axis]`. Axial touching therefore does not overlap.
Stored side planes are then tested in their original order. Their scalar-f32
residual is:

`((ny*py + nx*px) + nz*pz) - (abs(h*nz) + (distance+r))`.

Reject only when this residual is strictly positive. Side-plane equality counts
as inside. Preserve this expression order, coefficients and signed finite inputs;
it differs from the moving consumer's dot operand order. There is no `0.125`
allowance, normalization, vertex read or side-flags read. Zero-side brushes use
only their six axial tests. Both supplied mask and brush contents are opaque u32;
the mask test is performed by the caller before the narrow phase.

On overlap the brush consumer writes `startsolid=true`, `allsolid=true`, positive
zero fraction, brush contents and zero surface flags. It does not write the normal,
plane distance or opaque flag `0x2c`. On rejection it writes nothing. Existing
incoming flags are not reset; native callers terminate when allsolid is set.
The first admitted overlapping brush therefore supplies the contents; no lowest-ID
tie sort is permitted. Safe Rust rejection for nonfinite/invalid arithmetic is a
project boundary, not an observed native invalid-input policy.

## Ordered node, leaf and world behavior

Stationary node recursion tests node contents against mask first. Positive signed
counts visit terminal u16 indices in stored order. Each index addresses the global
96-byte brush array; matching brush contents precede the overlap test. Stop at
first allsolid. Negative signed counts process the adjacent 20-byte node first,
then continue split processing. For split distance `d`: if query minimum on the
axis is `>d`, follow child zero only; otherwise process child zero recursively when
query maximum is `>=d`, then child one. Child offsets are relative node strides.
Split equality can visit both children. Range/cycle/finite checks are required in
the safe adapter, independent of native trust in its allocation.

The leaf consumer applies the same strict bounds overlap convention before its
root-node traversal. Its leaf and node roots address global storage, including
root zero; it does not adopt diagnostic schema33's root-zero-empty assumption.
The node contents gate still controls whether a root is relevant.

Unfiltered stationary world builds broadphase bounds `(p-extent)-1.0` and
`(p+extent)+1.0` using original builder half extents and scalar f32. The constant
at RVA `0x7fdd24` is word `0x3f800000` (1.0). Starting BSP node zero, it collects
up to 1,024 u16 leaf identifiers in depth-first child-zero then child-one order.
Negative BSP child `n` maps to leaf `-1-n`. Leaf collector overflow sets an internal
overflow flag and retains the first 1,024; the world caller does not check that
flag. A safe full-scene adapter should return explicit overflow rather than claim
complete coverage from a truncated list.

For each collected leaf, allsolid early-outs the loop. Brush contents matching mask
invoke the leaf brush consumer first; successful overlap skips that leaf's terrain.
Otherwise terrain contents matching mask invoke RVA `0x0e9e20`. Its body remains
unqualified. Root-zero terrain remains independent of brush selection.
When the supplied filter has a nonzero list count, world dispatch instead iterates
its ordered 8-byte entries: one discriminator branch invokes RVA `0x004050`, the
other masks an explicitly supplied brush and invokes stationary overlap. The
`0x004050` body/list ownership is unqualified; this route must remain explicit.

The BSP plane classifier uses x87 arithmetic, unlike the scalar-SSE narrow phase.
Its initializer supplies all eight signbit cases. Result bit one tests maximum
support `>=distance`; result bit two tests minimum support `<distance`.
Classification one follows child zero, two child one, otherwise both in order.
Original x87 control/precision mode is not established by disk code alone;
near-plane numerical equivalence remains unresolved. Do not silently substitute
a default f32/f64 dot product and call it native parity.

## Ownership and implementable increment

Both stationary leaf/node and moving node consumers use global node/brush arrays;
neither reads the selected cmodel's nullable serialized ClipInfo reference.
This defeats null-provider-as-exclusion but does not independently establish
the disk clipmap-to-runtime global binding. The exact loader/runtime assignment
and initialization of those globals remains the next ownership question.
Likewise a cmodel leaf is not a current transformed entity instance or dynent pose.

The implementable isolated increment is a stationary brush request on the existing
admitted payloads with explicit shape/mask/incoming result, preserving the overlap
rules and atomic failures. Native node traversal is source-qualified structurally,
but scene admission still needs global-provider binding and complete references.
Full stationary world acceptance additionally needs the filtered family and terrain
consumers, BSP precision policy and overflow handling; none may be skipped.

## Validation limits

Six function spans (1,933 bytes), direct branch boundaries, eight-way initialization
and repeated disk identity checks pass. Independent dyadic rational versus scalar
f32 tests pass 15 stationary cases (six axial tangencies, side equality/outside,
capsule support, asymmetric centering and zero-side cases), two mask cases, one
ordered contact-write case and four node ordering cases. These test reconstructions,
not native execution. No Rust changed; workspace gates are not rerun.
