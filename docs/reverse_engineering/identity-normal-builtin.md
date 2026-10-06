# Measured identity-normal builtin

The shared engine identity-normal image is generated from a nonexpressive
measured constant: one linear RGBA8 pixel `[128,128,255,128]`, dimensions1x1,
depth1, one mip. The fourth channel is128, not an assumed opaque default.
Original material sampling remains unchanged.

Capture read-only runtime data relocated the actual name/function table after
retail addresses proved inapplicable to the live build. The identified initializer
supplies the packed pixel, dimensions and format28. The common allocation/upload
helpers retain the format and original pixel pointer, then call the D3D context
update without CPU channel conversion. Public DXGI declarations identify28 as
RGBA8 UNORM. Raw executable bytes remain external.

Evidence hashes: initializer SHA256
`3256e0c2ceb20b691b8abd693d51a18ceca19b55938b092d12ed94c12cd433ec`;
common helper SHA256
`8de22ed5c8a6009f049b5c54811835467d9b555d94745f5e758fc06d7f4db95a`.
Final upload helper SHA256 `06ba8dabe5a1490f2c0fa0a31f9964e860a7ce105bf30ed2a01091b789e5fba6`.
External runtime-evidence.private.json and capture files retain the complete chain.

`assets::builtin_images::definition_for_source` admits only source identifier hash
`3343bb032d046705946aeb4c58e273a17c592f51f69a12df0eefe62053b66eca`.
Unknown or altered identities return Unsupported. `validate_external_builtin`
checks exact format, dimensions, mip count, cube rejection and pixel bytes in
addition to an enclosing manifest's hash. This is a measured original builtin,
not a general substitute for missing normal textures.

The external export appends its DDS at image260, preserving the original260
shared image indexes. Its SHA256 is
`93c6b0c9bed9811735b7003f19944015c2d7768b73b62f8b38f4ef7eb852139e`.
The compatible v2 scene and shader bank now each declare261 images. All41
identity-normal material references resolve to that shared image, retaining name
hash, semantic and packed sampler. The shadow builtin's single binding remains
explicitly unresolved. `export_builtin.py` is idempotent; bank export joins the
source identifier hash to the appended image table entry. All generated files
remain external.

Synthetic tests check measured channels, source identity, altered pixels and mip
errors. Actual opt-in scene/bank validation checks all41 joins, sampler equality
and the remaining shadow reference. This proves bounded builtin data admission;
full original rendering and runtime resource generation remain INVESTIGATING.
