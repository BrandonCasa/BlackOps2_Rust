# Full-scene material adapter

## Current executable boundary

Rendering owns two deliberately separate APIs:

- `full_scene::diagnostic_texture_scene` prepares all admitted geometry, selects
  the first semantic-2 color binding, shares images by source image/sampler, and
  groups canonical corners by material. It retains UV seams and surface accounting.
  Multiple color layers and missing colors are reported explicitly. The complete
  external scene contains 6,469 surfaces; 70 water surfaces have no color binding.
  This preview does not implement their original shader, layering or lighting.
- `tech4` implements the mathematical response in
  [the selected opaque contract](materials-0002-opaque-contract.md) as a Bevy custom
  Material, with project-owned WGSL. It consumes explicit float mesh attributes,
  five linear image inputs, probe SH, HDR and fog globals. It adds no renderer.

The legacy v1 `MaterialScene` remains available. All rendered positions/directions
use the [canonical Bevy boundary](../adr/0002-bevy-runtime-coordinates.md).
SH polynomials, cube directions and lightmap directions are evaluated in source
basis through the inverse adapter; their coefficients are not silently reinterpreted
as Bevy-axis values.

## Observed native qualification

Capture lane reports original frame6325 event4462 uses the selected TECH4 bytecode
and joins exactly to source surface506, material99, format0, lightmap0, probe2.
The captured stride36 input layout proves POSITION float4, COLOR UNORM8x4,
base UV half2, normal/tangent R10G10B10A2_UNORM, lightmap UV R16G16_UNORM.
The shader remaps direction channels below0.5 to2c and otherwise to2c-2, then
normalizes each direction separately. Original POSITION.w supplies handedness.
Source probe2 SH matches captured object SH. The assets lane preserves the raw
corner records and exposes exact normalized input channels; rendering supplies
the shader-specific direction remap.

Captured views are linear UNORM and the intermediate target is RGBA8 UNORM.
Material samplers use wrapped UV with anisotropy4; lightmap/probe samplers use
linear filtering and clamp. OAT sampler enums qualify mip disabled0, nearest1,
linear2 and packed U/V/W clamp bits. WGPU sampler bounds may use the physical
mip interval where native bounds extend beyond existing levels.

`prepare_captured_tech4_surface` admits only surface506, the exact captured base
opaque technique-set identifier and format0. It requires explicit native HDR,
fog and lightmap/probe samplers, source packed attributes and static lighting
references. `Tech4Material::new_original` installs captured clockwise-front/back
culling and runtime fog. Other source surfaces are not admitted by that helper.
This scoped qualification does not prove all materials sharing a technique name.

The native fog vertex output uses camera-relative source position P. Its height
profile is `h=c.w*P.z+c.x`; the integrated profile is exp(h) below zero and h+1
otherwise. Near-zero `c.w*P.z` uses the clamped second-constant limit. The profile
scales distance attenuation before exp2/clamp. View/sun alignment selects fog
color; fog alpha controls transmittance. The fragment blends before HDR scaling
and square root. Runtime values remain external rather than becoming defaults.

Bevy cameras supply canonical world-relative positions; the inverse rotation
returns source vectors for the measured shading equations. Static world meshes
use identity model transform. Arbitrary original model transforms are unqualified.

## Shader-family extent

The assets lane exported56 representative technique sets. A bounded read/hash
inventory finds1,404 VS and1,404 PS files across available technique slots,
51 distinct VS bytecode identities and1,062 distinct PS identities. TECH4 is
present in50 representatives, with19 VS and50 PS identities. Stripping disassembly
comments/blank lines leaves the same19/50 instruction-form counts. Therefore the
selected base response must not stand in for the other49 TECH4 pixel programs.

TECH4 contains arithmetic/dot/normalize, sample/explicit-LOD, conditional selects,
comparisons, bit operations, sin/cos, fraction and discard operations. An external
shader translation route should qualify input/output signatures, resource and
constant bindings, state and instruction semantics; shader identity counts alone
are not an instruction-support test. Originals and any derivative shader artifacts
stay external. A translator would supply Bevy Shader/Material assets, not an engine.

## Validation and remaining limits

Synthetic CPU tests cover directional normal response, probe modulation/SH,
fog/HDR order, qualified UNORM remap, native fog limits and canonical corner/UV
contracts. First-color preview tests cover shared-image reuse, all-surface
accounting, missing/multiple colors and unsupported contracts/references.
Outer synthetic GPU proof compiled the Material and read back512x512 at24 updates;
center RGBA161/117/84/255 matches the analytic matte case within2 byte values.
That first GPU case does not independently qualify every nonzero reflection/fog
branch or original fidelity.

The outer native source506 test compares the exact joined original draw. Fully
covered interior pixels match within one byte value in the initial comparison;
the original four-sample MSAA target's sample-zero edges differ from a single
center-sample render. These results qualify only that isolated draw, with the
MSAA/sample comparison still being completed. Capture verifies GreaterEqual
depth comparison: original clip depth is already reversed. CapturedClip preserves
that depth and applies an explicitly supplied viewport depth range; it does not
apply another reversal. Full-frame equivalence remains unproven.

## External translated material boundary

The assets lane translated all 1,113 distinct bytecode identities through vkd3d
and strict Naga validation into external WGSL. The renderer's OriginalMaterial
uses Bevy Material asset preparation, pipeline specialization and render scheduling.
Each shader pair has its own explicit group-three layout; the direct preparation
path avoids a fixed union layout with incompatible cube/depth resources. Shader
handles, entry points, stage visibility, uniform extents, texture dimensions and
sample classes, sampler types, and raster/blend/depth state form the pipeline key.
No original shader or translated shader text enters the repository.

Native shader vertex inputs are separate explicitly named source-space attributes.
Canonical Bevy POSITION remains available for bounds/visibility. Qualified routing
must match shader reflection component counts and locations; input register numbers
alone never establish semantics. Uniform buffers remain explicit bytes with checked
extent, and image admission verifies dimension/sample class before GPU preparation.
Missing resources have no fallback. MSAA remains controlled by Bevy's pipeline key.

This bridge does not supply inferred technique selection, missing code textures,
source constant updates, model transforms or frame passes. Production camera/global
updates need a source-to-canonical engine boundary, while a captured comparison
may explicitly supply frozen native buffers. Compilation of a shader bank is not
whole-scene appearance or gameplay fidelity proof. The handoff records build gates.

The pass_resources assembler now joins native material hashes and writes declared
constant bytes into stage-separated buffers. Code arrays are explicitly supplied
float4 rows; partial material/literal destinations retain their exact byte widths.
Unwritten reflected bytes require a caller-supplied native buffer, rather than an
assumed zero global. Identical shared resources may merge; conflicting writes or
texture/sampler assignments fail. Dead inputs absent from the selected pair's
reflection do not request unrelated resources. The caller validates final image
classes and material device limits through OriginalMaterial admission.

Source enum plus exhaustive reflection audit qualifies implicit reflection probe
at t15/s15 (flag bit one) and secondary lightmap at t13/s13 (bit two). Resources
remain caller supplied. Unknown flags fail. Material type remains explicit caller
metadata; this assembler neither selects vertex layouts nor generates engine code
values. See the [bounded handoff](../handoffs/pass-resource-assembly.md).

An opt-in pipeline census now establishes GPU readiness for all 56 families and
1,404 pass references (1,065 distinct shader pairs). Exact reflected synthetic
uniform/image/sampler interfaces and vertex layouts are supplied through Bevy's
PipelineCache and RenderDevice. All pairs compile; every fragment interface has
one color target and no MRT requirement. The synthetic valid/invalid shader checks
also pass. The census uses synthetic constants and target/raster state and does
not execute original scene shading, source vertex routing, MSAA or the additional
Bevy view layouts. These remain independent admission/comparison requirements.
See the [census handoff](../handoffs/shader-pipeline-census.md).
Source/capture details and asset receipts belong to their owned lane documents;
raw shaders, assets, runtime constants and installation paths remain external.
