# Original shader translation route

The official vkd3d shader compiler 1.12 converts every currently recovered original
DXBC shader to SPIR-V. The pinned Bevy-compatible Naga 29.0.4 then strictly parses,
validates and emits WGSL. All 1,113 unique inputs pass: 51 vertex shaders and 1,062
pixel shaders across the 56 recovered technique sets. No handwritten arithmetic
translation is needed for this recovered corpus.

## Procedure and sources

The external shader corpus deduplicates all 2,808 recovered stage bytecodes by
SHA256 and retains the source-to-contract mapping. Every original and generated
shader remains external. The official [Wine release archive](https://dl.winehq.org/vkd3d/source/vkd3d-1.12.tar.xz)
is pinned by SHA256
`1ad1fa0e2c6c7b160fcd11856e263476960e0ab762e13102d5e73a177d791d8b`.
The compiler was built in an isolated external WSL directory with locally
extracted compiler/header dependencies; no system-wide package installation or
repository dependency change was needed. Command: `vkd3d-compiler -x dxbc-tpf
-b spirv-binary <external-input> -o <external-output>`.

A separate safe Rust executable uses Naga exactly 29.0.4 with `spv-in` and
`wgsl-out`. SPIR-V parsing uses strict capabilities and disables automatic
coordinate-space adjustment. Validation enables all validation flags. Original
translation and descriptor-remapped translation independently pass the complete
1,113-input batch. Generated WGSL totals 28,845,667 bytes after remapping.

## Bevy interface

Installed Bevy 0.19.1 source confirms `Shader::from_spirv` and the pipeline-cache
SPIR-V path behind `shader_format_spirv`. Its direct SPIR-V validation setting has
special restrictions. WGSL emission provides the normal safe Bevy shader path,
while retaining the native shader computation externally.

Public vkd3d interface mappings support explicit descriptor register-to-binding
selection. The proved external Naga IR remapper uses source register names and
revalidates after changing only descriptors. All native resources use group 3:
VS constant-buffer register N maps to binding N, PS constant-buffer N to 16+N,
texture N to 32+N, sampler N to 64+N. Stage-specific buffers avoid incorrect aliasing
of native VS/PS buffers with identical register numbers. Source register numbers
and descriptor classes are retained in external reflection metadata.

Across translated modules: 6,346 float 2D texture declarations, 774 float cubes,
798 depth 2D textures, 7,120 ordinary samplers, 798 comparison samplers and 2,083
uniform buffers. These are repeated declarations, not unique scene resources.
The maximum is 14 textures and 31 total resources per module. Reflection records
uniform spans, texture dimension/sample class, shader entry stage/name and input
register locations/types. Depth/comparison resources require corresponding typed
bindings rather than an RGBA substitute.

## Limits

Translation and IR validation do not establish GPU pipeline linkage, correct
resource population, dynamic globals, original pass dispatch, source/runtime
coordinate adaptation, texture-view interpretation, or visual fidelity. Later
original capture confirms reversed depth; preserve the native vertex depth
computation and adapt captured viewport depth range explicitly at integration.
Native shader code and generated WGSL are never repository fixtures or tracked
assets. Fidelity remains INVESTIGATING.
