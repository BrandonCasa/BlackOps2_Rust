# Native world vertex routing

The hash-admitted native ISGN semantics identify inputs independently of the
translated numeric register locations. Material stream routing selects a source
stream for each semantic; registers alone cannot select attributes. The 51 native
vertex signatures contain 274 float inputs. Their locations and component counts
agree with translated reflection for all 1,404 shader-bank passes.

The public T6 stream enums distinguish position, color, base UV, packed normal,
packed tangent, lightmap UV, two extra UV streams and two normal-transform streams.
Captured world declarations qualify the following secondary records. Primary
36-byte records remain unchanged across these formats.

| Source format | Secondary stride | Source fields |
|---|---:|---|
| 0 | 0 | none |
| 1 | 4 | extra UV0 HALF2 |
| 2 | 8 | extra UV0 HALF2, normal transform0 RGBA8 UNORM |
| 3 | 8 | extra UV0 HALF4 |
| 4 | 12 | extra UV0 HALF4, normal transform0 RGBA8 UNORM |
| 5 | 16 | extra UV0 HALF4, normal transform0/1 RGBA8 UNORM |
| 7 | 16 | extra UV0 HALF4, extra UV1 HALF2, normal transform0 RGBA8 UNORM |

Format 3 was qualified by frame 27970/event 4574, format 5 by the earlier
captured declaration, and format 7 by frame 119013/event 7354. Native raw bytes,
input signatures and capture metadata remain external; the assets secondary
decoder owns hash/offset/corner admission.

HALF2 formats feeding a four-component shader input expand to `(x,y,0,1)`.
This is the native format-conversion behavior specified by [Direct3D 11.3,
section 19.1.3.3](https://microsoft.github.io/DirectX-Specs/d3d/archive/D3D11_3_FunctionalSpec.htm).
Absent stream data is an error, and receives no replacement values.

`world_vertex_route` joins a selected real pass to its hash-bound source signature
and routing, then validates the surface format and translated input declaration.
`routed_world_mesh` preserves exact per-corner source inputs and UV seams. Bevy's
POSITION attribute remains canonical Y-up for bounds; separate source position
attributes serve the original vertex shader's arithmetic. Normal/tangent inputs
remain normalized packed channels until the original shader decodes them.

Actual CPU admission covered 6,469 surfaces and 177,318 pass uses whose material
state entry is enabled. All routes passed; 73 unique vertex-shader/source-format
mesh combinations were built successfully from admitted primary and secondary
corners. All 51 vertex identities and 1,404 pass definitions have at least one
qualified source format. This proves input admission and construction, not all
original draws, uniforms, camera transforms or visual equivalence.
