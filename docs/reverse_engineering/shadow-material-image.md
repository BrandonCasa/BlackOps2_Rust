# Shared shadow material image dependency

The remaining source image reference hash
`df04ff1564886176b0257baa1e44483e9add8571251fd8f12dea9e3e97014de1`
belongs to material 264 and its seven source surfaces 6319..6325, all format0.
Its texture name hash2695565377, semantic 2 and packed sampler 20 feed PS t0/s0
in TECH2/3. Shader reflection requires an ordinary float2D texture and sampler.
Sampler20 preserves linear filtering, linear mip selection and wrap addressing.
These passes use bank VS1097/PS187; their source hashes are
`fc7e650933e980028e141f29046278628c66eb690f471c45b031e24a14a19916`
and `2a2510a4df2ae2c98a7c421ed3cb96fc617e5201747577dc36c158235e7c78c7`.

The map's comma-prefixed image was a reference to another loaded zone, not proof
of an engine-generated builtin. A bounded native read-only image-pool inspection
found the real nonreference image in `code_post_gfx_zm`; common, code-pre, patch,
map-patch and UI zones yielded no matching image. Native decoding exports an
actual 256x256 BC1 UNORM image, nine complete mips and one 2D layer. Container hash:
`dc44f775074e010e37f3fbdf60fc21af1de2582634432cd6057d6eecd0099942`.
Decoded DDS hash:
`96409c359760f7c2b7a03b2720931c4bb4272e0e1cdc888d16a1bc01585fa3f4`.
The native scanner uses the previously pinned decoder and shared read-only ipak
metadata. All decoded bytes, pool metadata, producer source and logs stay external
in `shadow-material`; inspection returned zero warnings/errors.

This dependency is distinct from CodeTexture6's live 4096x8192 R32 shadow atlas.
No runtime producer or frozen captured atlas is substituted. An existing original
capture observes these surfaces in a depth-only technique; it does not establish
that TECH2/3 execute at that viewpoint. Source material/pass assignments establish
the image dependency independently of view-dependent dispatch.

`assets::builtin_images::external_definition_for_source` supplies the qualified
external identity, container hash and DDS contract. `load_external_builtin`
requires that exact source/DDS identity, safely resolves a relative external
path, bounds reads and verifies dimensions, format, mip count and layer type.
There are no tracked image pixels and no fallback image generation.

External `export_shadow.py` appends shared image 261, preserving image 0..260.
The scene and shader bank now contain 262 images; all 1,080 material texture bindings
resolve shared images, including 41 identity-normal references and this one source
shadow reference. Zero material UnresolvedBuiltin entries remain. Export keeps
semantic/name hash/sampler unchanged and updates the bank atomically. Runtime
CodeTexture/CodeConstant generation and format3 secondary inputs remain separate.

Synthetic tests cover exact contract, unknown identity, changed DDS identity and
missing files. Opt-in actual scene/bank tests verify exact bytes, material hash,
sampler/semantic and TECH2/3t0/s0 assignments. Full rendering remains INVESTIGATING.
