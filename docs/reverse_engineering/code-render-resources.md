# Shared code render resources

## Scope and evidence

This inventory identifies the shared resource resolver inputs used by all 1,404
available passes in the 56 recovered technique families. It does not establish
that every pass is dispatched for every surface. Source assignments come from the
hash-admitted external shader bank, public pinned T6 declarations and original
capture observations. Fidelity remains INVESTIGATING.

The public T6 header SHA256 is
`66f23295ee5e4545d6d7cae2a3f0fc266e9bea8d331c6b35e7fa5a7008a8cc91`;
the inspected bank SHA256 is
`77ccc4d8d7254224dc22a1329fcb6ad0cda386a7226336d79914423e724ceae6`.
The external `code-render-resources/inventory_resources.py` reproduces the census
and emits `resource-identities.private.json` with every stage, destination,
family, technique and pass. No licensed resource contents are tracked here.

## Typed constant identities

Every used CodeConstant has first_row 0. Of 11,545 assignments, 8,268 copy one
float4 row (16 bytes), and 3,277 copy four float4 rows (64 bytes). The latter
are matrices. Destination offsets and sizes are bytes. MaterialConstant writes
of 4 or 8 bytes are a separate contract and must not truncate CodeConstant rows.
The following names are public symbol identities, not proof of generation math.

| Code | Identity | Required generation inputs still to establish | Assignments |
| --- | --- | --- | ---: |
| 0 | light position | selected light; position basis and fourth component | 768 |
| 1 | light diffuse | selected light; color/intensity encoding | 768 |
| 2 | light spot direction | selected light; direction basis/component packing | 768 |
| 3 | light spot factors | cone setup and units | 572 |
| 5 | light falloff A | attenuation parameters | 768 |
| 6 | light falloff B | attenuation parameters | 768 |
| 7, 8, 9 | light spot matrix rows 0, 1, 2 | cookie/light projection and coordinate basis | 576 each |
| 11 | light spot AABB | light bounds and encoding | 192 |
| 12 | light cone control 1 | cone setup | 192 |
| 14 | light spot cookie slide control | cookie animation/configuration | 384 |
| 17 | render target size | current target dimensions; component units/order | 22 |
| 25 | game time | original clock and vector generator | 111 |
| 37 | shadow map switch partition | sun shadow cascade selection | 94 |
| 38 | sun shadow map pixel size | atlas dimensions and component formula | 94 |
| 44 | debug bump map | original debug configuration | 46 |
| 45 | debug performance | original debug configuration | 53 |
| 46 | material color | runtime material color owner | 3 |
| 60 | spot shadow map pixel adjust | spot atlas viewport/texel adjustment | 376 |
| 61 | dynamic-light spot shadow map pixel adjust | dynamic-light atlas adjustment | 517 |
| 169 | generic parameter 0 | per-draw parameter owner | 44 |
| 213 | transpose world matrix | source world/camera-relative transform | 1,403 |
| 229 | transpose view-projection matrix | original camera/projection convention | 1,403 |
| 233 | transpose world-view-projection matrix | source world and camera matrices | 1 |
| 237 | transpose shadow lookup matrix | source light projection and atlas lookup | 470 |

A safe resolver can accept explicitly supplied `code -> float4 rows`, select the
recorded span and copy exactly into the recorded destination. It must reject
missing values, wrong row counts and out-of-range writes. This preserves an
observed snapshot, but does not generate a correct moving scene by itself.
Matrices need source-space camera/world conventions and original depth/viewport
handling; the symbolic transpose suffix does not establish a multiplication order.
Canonical Bevy coordinates `(x,z,-y)` must be adapted at that ownership boundary.

Capture event 8001 demonstrates code25 at native VS buffer2 byte64 and code169 at
PS buffer1 byte368. Code169 is zero at that event only. A second-frame code25
observation supports changing fractional/time-like components, but its first two
components do not equal sin/cos of the observed fourth component. The temporal
formula remains unresolved. Frozen or zero values are not general defaults.

Native constant-buffer destination indexes must be joined to actual shader
registers, rather than variable declaration ordinals: at event8001 reflected VS
CB0 occupies native b2; reflected PS CB2 occupies native b1. Capture evidence
retains these aliases. A direct check of source6087 TECH4 DXBC declarations and
translated modules proves the current mapping correct: VS declares physical
CB2/CB0/CB3 and maps2/0/3; PS declares CB2/CB0/CB1 and maps18/16/17. Thus code25
writes binding2 and code169 binding17. RenderDoc CB labels here are declaration
array ordinals. No remap change is justified by this evidence.

## Texture identities and custom lighting

| Code | Identity | Assignments | Proven runtime contract / remaining input |
| --- | --- | ---: | --- |
| 6 | sun shadow map | 94 | event4633: 4096x8192 R32_FLOAT view over R32_TYPELESS, one mip/sample; clamp UV, linear comparison GREATER; atlas generation remains unresolved |
| 7 | spot shadow map | 705 | selected spot atlas generation, format and comparison state unresolved |
| 9 | resolved post-sun | 22 | producer pass, current render target and resolve timing unresolved |
| 15 | light attenuation | 384 | generated lookup/owner and exact contents unresolved |
| 16 | dynamic-light attenuation | 517 | generated lookup/owner and exact contents unresolved |
| 18 | float Z | 22 | depth producer, encoding and read timing unresolved |

MaterialArgumentLocation packs texture index in its low byte and sampler index
in its high byte. They are independent namespaces: value256 means t0/s1, not
texture256. Reflection supplies float/depth dimension and comparison requirements;
a source shadow image cannot be admitted as an ordinary color sampler silently.
For the representative TECH6 contract, code6 maps t9/s9; code237 is VS buffer0
byte768/64, code37 PS buffer0 byte1552/16 and code38 PS buffer0 byte1584/16.

Public custom sampler indexes0 and1 identify reflection probe and secondary
lightmap. Across the entire census, 792 passes have flags3 and PS t15 cube plus
t13 2D; 310 have flags2 and t13 only; 302 have flags0 and neither. Event4462
confirms those bindings as original probe and secondary lightmap. These resources
are selected per surface from exported probe/lightmap indexes, not an arbitrary
scene-global image. One captured SH vector exactly matches source probe2; all31
source probe SH vectors are exported, while complete dynamic assignment proof
remains separate.

## Two engine builtin image references

The native material tables retain two unresolved engine image references:
identity normal and shadow. They cannot be erased from original-material admission.
The retail executable hash is
`f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e`.
Its intact data section contains an identity-normal name/function table entry.
However, the corresponding on-disk code does not yield a usable initializer.
The live original-game module is a different mapped build: read-only checks of
the retail table/string addresses returned different data. Its unrelated function
bytes are excluded from builtin proof. Capture subsequently relocated the actual runtime name/function table. Its
identity initializer supplies packed little-endian bytes `[128,128,255,128]`,
two size-like arguments1 and format value28. Public DXGI declarations identify
28 as RGBA8 UNORM. The common helper stores dimensions1/1/depth1, retains
format28 and passes the original pixel pointer to the upload helper, which calls
the D3D context update directly without a CPU channel conversion. This qualifies
the builtin one-pixel linear RGBA8 input; sampler comes from its material binding. The earlier unrelated retail-address bytes remain excluded.
The shadow material builtin must also be distinguished from code6's live shadow
atlas; matching a name alone is insufficient.

## Next executable boundary

Implement the explicit checked resource assembler independently of generation,
using external qualified values and per-shader physical CB register mapping.
Investigate runtime code-array writers and texture producers next, prioritizing
camera matrices, moving time, shadow atlas and post-sun/depth dependencies.
Actual missing assignments must remain errors. No shader fallback or speculative
constant generator follows from this inventory.

The next secondary-stream export task can reuse the external 489,788-byte
`secondary-stream.bin` and surface records (offset1, firstVertex, vertex count,
format). The producer reads local corner indexes but does not serialize them;
add hash-bound local corner indexes before joining secondary values at
`offset1 + local_index * qualified_stride`. Formats1/2/4 are capture-qualified;
format5 has later capture evidence; formats3/7 still need complete qualification.
This adjacent task is not implemented here.
