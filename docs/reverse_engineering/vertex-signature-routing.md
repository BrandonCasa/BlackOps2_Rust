# Original vertex signatures and stream routing

All 51 unique recovered vertex shader identities now carry their original DXBC
ISGN input signatures, with source semantic name/index, input register, component
mask, used mask, scalar type and system-value classification. The producer checks
each original DXBC hash before parsing bounded signature entries. It emits a
hash-bound external sibling, not tracked source bytecode or shader text.

The sibling contains274input records: POSITION51, COLOR42, NORMAL35 and
TEXCOORD146. All observed inputs are float32 and non-system-valued. POSITION is
xyz for3shaders and xyzw for48; secondary TEXCOORD3/4/5/6 are xyzw shader inputs.
Builtin vertex/instance IDs have distinct typed system-value categories; a source
stream route is not substituted for a builtin. Unsupported semantics/types and
incorrect builtin declarations fail explicitly.

## Admission and API

Shader bank version2 requires the external signature descriptor. Version1 remains
loadable; absent semantic metadata yields Unsupported from the semantic lookup,
allowing existing numeric diagnostic fixtures to retain their original contract.
The signature file is capped at1MiB and SHA256-bound; the actual48,433-byte file
hash is `fcc98b1c08a2aa6f9289007a44c45683e55cf113841459be12404649f5ac3af4`.
Every VS must appear exactly once. Semantic/register uniqueness, known names and
index limits, masks, scalar/system types and translated WGSL location/component
counts are validated. Every generic input must also have a pass destination route.

`ShaderBankInput::signature_for_shader(index)` exposes qualified metadata.
`assets::vertex_signature::decode_stream_route` decodes original numeric enum
identities; `source_for_input` joins through semantic name/index, preserving the
source meaning. Missing or ambiguous routes are errors. Numeric register position
alone never selects a source attribute.

The external `export_signatures.py` reads original signature chunks and emits the
sibling. The main bank exporter invokes it after bank export, so regenerated banks
retain version2 metadata. Original DXBC/disassemblies and generated reports remain
external. Synthetic metadata fixtures do not contain original shader content.

## Source and destination identities

The public pinned T6 source IDs are0position,1color,2texcoord0,3normal,4tangent,
5texcoord1,6texcoord2,7texcoord3,8normal-transform0,9normal-transform1 and10blend
weight. Destination IDs are0POSITION,1NORMAL,2/3COLOR0/1,4DEPTH,5..18TEXCOORD0..13
and19BLENDWEIGHT. The original routing record stores source first, destination
second. Abstract enum definitions and paired source captures establish these
identities; names alone do not reinterpret the shader's mathematics.

All1,404recovered passes use ten distinct pairs:

| Source ID | Source identity | Destination semantic/index |
| ---: | --- | --- |
| 0 | position | POSITION0 |
| 1 | color | COLOR0 |
| 2 | texcoord0 | TEXCOORD0 |
| 3 | normal | NORMAL0 |
| 4 | tangent | TEXCOORD2 |
| 5 | texcoord1 | TEXCOORD1 |
| 6 | texcoord2 | TEXCOORD3 |
| 7 | texcoord3 | TEXCOORD4 |
| 8 | normal-transform0 | TEXCOORD5 |
| 9 | normal-transform1 | TEXCOORD6 |

Captured world layouts0/1/2/3/4/5/7 qualify the physical field encodings recorded
in [secondary inputs](secondary-world-inputs.md), including separate format5 and7
layouts. A shader's float4 input can consume an original HALF2 field: original IA
fills absent z with0 and w with1. This is specified native input conversion,
not a missing-attribute fallback; see [Direct3D11.3 section19.1.3.3](https://microsoft.github.io/DirectX-Specs/d3d/archive/D3D11_3_FunctionalSpec.htm#19.1.3.3%20Defaults%20for%20Missing%20Components).
Physical format widths and source routing remain separate from shader component
masks. The rendering lane owns that bridge and must reject missing required data.

Synthetic tests cover missing/version1 metadata, wrong/duplicate signatures,
registers, masks, scalar/system types, unknown semantics, missing/ambiguous routes,
invalid enum values and changed metadata hashes. Actual validation covers all51
signatures,274inputs and1,404pass routes. Rendering fidelity remains INVESTIGATING.
