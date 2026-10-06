# External translated shader bank

`bo2_assets::shader_bank` safely admits a distinct version-one external shader bank
without changing appearance versions one/two. It retains original stage identities,
hash-bound external WGSL, entry points, input-register locations/component counts,
explicit uniform/float-texture/depth-texture/comparison-sampler interfaces and
original family/pass routing/argument declarations. Shader computation stays external.

The actual external export admits all 1,113 unique translated shaders, 56 technique
families, 1,404 available passes and 280 material contracts. Material contracts retain
all source constants, raw state words, 36 technique-state entries and native texture
hash/semantic/sampler assignments. Image indices match the 260-image shared scene
input; the two unbacked engine references remain explicit unresolved builtin
identities rather than invented image data.

## Argument and resource contract

Native sampler argument location is a union: low byte is texture register, high
byte is sampler register. Thus a packed value of 256 means texture register 0 and
sampler register 1. Uniform destinations retain byte offset/size and source constant
buffer register; observed writes include 4, 8, 16 and 64 bytes. Arguments distinguish
material constants, material textures, engine constant codes, engine texture codes
and literals. Every material hash argument for available source state/pass entries
joins its native material constant/texture table: zero missing hash joins.

All shader resources use material group 3. VS constant buffers occupy bindings
0..15; PS constant buffers 16..31; textures 32..63; samplers 64..95. Reflection
retains exact uniform spans and float/depth image types. Runtime binding must verify
matching source resource kinds and stage-specific buffer ownership.

The selected captured TECH4 pair has bank VS index86 and PS index587. Their minimum
uniform spans are VS0=640, VS3=176 and PS16=336 bytes. Larger actual source buffers
are permitted. Images bind at32/33/34/45 (float2D) and47 (floatCube); samplers
64/65/66/77/79 are ordinary samplers. Original input registers are position4,
color4, baseUV2, normal3, tangent3 and lightmapUV2 at locations0..5.

## Safety and evidence limits

The manifest is capped at32MiB; shaders at1MiB each and64MiB aggregate. Counts,
references, unique identities/locations/bindings, shader stages, typed descriptor
namespaces, destination units, finite literal values and material state tables are
validated before shader reads. Files use the existing canonical relative dependency
boundary and SHA256 admission. Export replaces the external manifest atomically.
Synthetic fixtures test identities, malformed type/namespace/reference combinations,
limits and path traversal. Actual full-bank admission is an explicit opt-in test.

This is metadata/hash admission, not a standalone WGSL semantic validator; Bevy/Naga
must validate shader stages and bindings during actual pipeline construction. The
external producer independently Naga-validates the recovered corpus as recorded in
[translation findings](full-scene-shader-translation.md). A parser pass does not prove
resource values, original pass dispatch, visual response or full rendering fidelity.

Twenty-six engine constant codes, six engine texture codes and custom lighting/
probe bindings still need an evidence-backed runtime resolver. Available shaders
include dynamic shadow/depth and post-sun inputs; decoded zone textures alone cannot
populate these. Missing runtime assignments must produce explicit unsupported-data
errors. Fidelity remains INVESTIGATING.
