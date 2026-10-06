# Original full-scene rendering observations

2026-10-05. Status: INVESTIGATING. Original Plutonium T6 Zombies r5354 LAN,
Origins solo, D3D11, RenderDoc 1.46. This replaces the earlier assumption that
original capture was unavailable; it does not establish full-scene fidelity.

## Capture and identity

The existing signed portable RenderDoc launched the existing x86 bootstrapper
with its supported pre-start capture command, local LAN mode and child hooks.
The original menu and map displayed the D3D11 capture overlay. Solo Play followed
by Start Match loaded Origins. A direct map command from the title menu first
encountered a LUI lifecycle error; restarting the owned process and using the
normal menu route succeeded. No global hook or new RenderDoc installation was used.

Original frame 6325 opens and replays successfully: 1,810 draws and 154 distinct
vertex-shader/pixel-shader/input-layout combinations. Its 348,338,830-byte capture
SHA256 is `708ff000dd7593950a1526ada264444a6b5eb16dcd74894b19c32aee702dd36c`.
Captured originals, shader bytecode, buffers, textures and readback stay external
under the task-local inspection output. No capture or original asset is tracked.

## Qualified world inputs

Live static-world draws use the following 36-byte primary stream. Formats are
actual D3D11 input-layout observations, not candidate signed-integer decoders.

| Semantic | Byte offset | Format |
| --- | ---: | --- |
| POSITION0 | 0 | R32G32B32A32_FLOAT |
| COLOR0 | 16 | R8G8B8A8_UNORM |
| TEXCOORD0 | 20 | R16G16_FLOAT |
| NORMAL0 | 24 | R10G10B10A2_UNORM |
| TEXCOORD2, tangent | 28 | R10G10B10A2_UNORM |
| TEXCOORD1, baked-light UV | 32 | R16G16_UNORM |

Packed normal/tangent XYZ reach the vertex shader as unsigned-normalized 10-bit
channels. Apply the shader's piecewise remap and normalization described in the
[opaque contract](materials-0002-opaque-contract.md). Do not reinterpret them as
signed normalized integers. Position W carries the original handedness value.

Source stream offsets and index offsets join these captured draws:

| Frame/event | Source surface | World format | Secondary stream |
| --- | ---: | ---: | --- |
| 6325 / 4462 | 506 | 0 | absent |
| 6325 / 4383 | 70 | 1 | stride 4; TEXCOORD3 half2 at 0 |
| 6325 / 4398 | 206 | 2 | stride 8; TEXCOORD3 half2 at 0, TEXCOORD5 UNORM8x4 at 4 |
| 27970 / 4574 | 386 | 3 | stride 8; TEXCOORD3 half4 at 0 |
| 6325 / 4747 | 4377 | 4 | stride 12; TEXCOORD3 half4 at 0, TEXCOORD5 UNORM8x4 at 8 |

Frame 40264 additionally observes format 5, event 4692: primary stream unchanged;
secondary stride 16, TEXCOORD3 half4 at 0, TEXCOORD5 UNORM8x4 at 8 and TEXCOORD6
UNORM8x4 at 12. Later targeted captures qualify formats 3 and 7 as described below.

## Qualified TECH4 sample

Event 4462 draws six indices starting at 18,987. Its primary stream binding offset
is 1,904,112 bytes. All six recovered corners match source surface 506 exactly,
216 bytes, SHA256 `7fc9fd94aeb982d4860516cc693320b0f75c426f86fa313ad4a2d344d3ff87fc`.
Its vertex and pixel shaders are byte-identical to the previously inspected TECH4
pair, establishing actual dispatch for this sample. This is not proof that every
surface uses TECH4 or that TECH4 alone reproduces final composited appearance.

Observed raster state is back-face culling with clockwise front faces. Material,
normal, specular, lightmap and probe views are linear UNORM; the bound color
target view is R8G8B8A8_UNORM over a typeless four-sample resource. No sRGB
view/target conversion occurs in this draw. Live depth testing is GreaterEqual,
with depth writes enabled and stencil disabled; this corrects the earlier static
state-enum LEQUAL inference. Blending is disabled and all four channels write.
Viewport is (0,0,1920,1080), depth range 0..0.984375; enabled scissor is
(0,0,8192,8192). Depth clip is enabled, depth biases are zero, conservative
rasterization is disabled and the sample mask enables all samples.
Three material samplers wrap U/V, have anisotropy 4 and zero mip bias. Lightmap
and probe samplers clamp to edge and use linear minification, magnification and
mip filtering, zero bias, and unbounded sampler LOD limits.

The shader reflects scene constants at register b0 and object/light constants at
b3. Captured binding-list indices are reflection indices, not register numbers.

| Scene b0 field | Byte offset |
| --- | ---: |
| HDR control | 320 |
| Fog color | 416 |
| Fog constants | 432 |
| Secondary fog constants | 448 |
| Sun fog direction | 464 |
| Sun fog color | 480 |
| Sun fog interpolation | 496 |
| Projection matrix | 512 |
| View-projection matrix | 576 |

The observed HDR multiplier is 0.03125. The object b3 world matrix at offset 0
is identity plus translation (-2768.875, -5363.5908203125, 294.6428527832031),
giving source camera origin (2768.875, 5363.5908203125, -294.6428527832031).
Reflection SH vectors at offsets 128, 144 and 160 match source reflection probe 2
coefficients. These vectors must be selected per source probe, not reused globally.
Exact fog vectors and matrices are retained externally for controlled replay;
they are frame conditions, not universal Origins defaults.

## Additional rare layout

Frame 119013 event 7354 matches the only format-7 surface, ordinal 3844, by
primary byte offset 6661024 and index offset 319521. The secondary stream binds
at byte offset 479160 with stride 16: TEXCOORD3 is half4 at offset 0,
TEXCOORD4 is half2 at offset 8, and TEXCOORD5 is UNORM8x4 at offset 12.
Format 5 also has stride 16, but contains half4 at 0 and two UNORM8x4 values
at offsets 8 and 12. Equal strides therefore do not imply equal attribute layouts.
Fresh frame 27970 event 4574 matches source format-3 surface 386, primary byte
offset 1433680 and index offset 14742, with vertex offset zero. Its lit draw binds
an 8-byte secondary stream at byte offset 325464: TEXCOORD3 half4 at offset 0,
with no additional color value. All seven serialized world formats are therefore
represented in actual original draw bindings. This establishes their layouts,
not pixel fidelity for every layered material.

## Shadowed family and dispatch joins

Frame 6325 event 4633 matches source surface 2546: six primary corners agree
byte-for-byte, format 0, lightmap 0 and reflection probe 14. The captured shader
pair equals technique 6 pass 0 of the same native techset as surface 506.
Its texture registers are color/normal/specular at t0/t1/t2, dynamic shadow atlas
at t9, secondary lightmap at t13 and reflection cube at t15, with matching sampler
registers. The fixture filenames use reflection indices 3/4/5 for these resources;
those indices must not be interpreted as shader registers. The shadow atlas is
4096 by 8192, one sample, R32_FLOAT over typeless storage. Its sampler clamps U/V,
uses linear filtering and GREATER comparison. This is an actual frame resource,
not a static placeholder. The fixture retains all four target samples before
and after the draw, original corners, shaders, constants and bound textures.

Matching source primary offsets and index offsets, then native shader hashes,
joins 367 draws to 189 surfaces and 56 materials in this frame, with zero unmatched
shader pairs. Shared shader pairs can alias several technique indices; the join
retains all candidates rather than inventing a unique dispatch. These material
counts do not establish complete coverage of all 56 source techsets. Frame 119013
extends the joins to 1020 draws, 744 surfaces and 146 materials across 40 techsets,
again with no unmatched shader pair.

Event 8001 maps to source surface 6087 and demonstrates an animated transparent
family. Code constant 25 occupies vertex b2 offset 64 and constant 169 occupies
pixel b1 offset 368. The first captured vector is (0.8852267, -0.4651598,
0.32700157, 29.327002); the latter is zero. A single sample does not establish
the CPU time generator or generic-parameter producer. The draw blends source
alpha with inverse source alpha and disables depth writes.

Frame 119013 event 17278 demonstrates the water family's runtime scene inputs.
Code texture 18 binds t2 as a 1920 by 1080 R32_FLOAT image, one mip and one sample;
its s2 sampler clamps and uses point filtering. Code texture 9 binds t3 as a
1920 by 1080 R8G8B8A8_UNORM image, also one mip and one sample, sampled using linear
clamped s5. These are captured resolved depth and post-sun color dependencies.
Code constant 17 occupies pixel b0 offset 1520 and contains (1920,1080,1/1920,
1/1080). Code constant 25 occupies vertex b2 offset 0 for this shader.

## Limits

The original frame supplies exact runtime bindings and a source-matched original
sample. Every technique's dispatch, dynamic light/shadow
generation, remaining built-in image semantics and final postprocessing remain
separate questions. GPU replay is original evidence; full-scene Bevy comparison
is still pending.
