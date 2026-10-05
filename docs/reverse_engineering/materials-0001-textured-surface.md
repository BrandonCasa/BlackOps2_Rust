# First world-surface color preview

Scope: one static Origins world surface, not a complete original material.
The pinned public OAT T6 unpacker reads the packed UV at byte20 of its admitted
36-byte stream0 records using the custom half helper. For surface508/category0,
its six original triangle corners yield finite UVs spanning -0.5029297..0.9995117
and -0.0625..0.953125. No UV flip, coordinate transform, welding or normal/tangent
interpretation is applied. The geometry identity remains
`ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f`.

A non-reference material/image pool join resolves one color texture for the selected
non-layered material. The decoded external DDS is BC1_UNORM,128x512,10mips;
sampler byte20 specifies repeatUV,4x anisotropic filtering and linear mip filtering
in the pinned T6 declarations. Original sampler fidelity is not yet compared.
The selected decoder read finishes without warnings/errors. Earlier candidate
layered surfaces were excluded; their missing textures do not count as resolved.

Appearance manifest v1 uses geometry SHA256, one source surface ordinal, per-corner
f32 little-endian UVs, relative texture/attribute references, source sampler and an
explicit `diagnostic-unlit-color` / `linear-diagnostic` declaration. Bounds and
identity/path/encoding checks precede Bevy image/mesh integration. The original
snapshot v1 reader is unchanged. Every original triangle corner becomes a distinct
preview vertex; source indices are retained, preserving triangle order and UV seams.
LegacyDDS input is bounded to2D,4096x4096 and13mips; unknown extensions are rejected.

Actual offscreen GPU checks produce the selected textured surface in rawX/oblique;
rawZ is edge-on. A separate user-created quadrant DDS/plane displays red/green
above blue/white in rawX using explicit cornerUVs, demonstrating the adapter's UV
ordering. This qualifies the chosen diagnostic convention; it does not prove the
original engine's UV/gamma convention. Captures and licensed data remain external.

Original transfer function, normal/tangent conventions, alpha/depth/cull equations,
specular/emissive behavior and lighting remain INVESTIGATING. Unlit rendering,
disabled culling, raw coordinates and diagnostic cameras are intentional preview
choices, not a claim of original appearance. No controlled original comparison was
performed. Remaining6468surfaces and full material/lighting coverage are unsupported.
