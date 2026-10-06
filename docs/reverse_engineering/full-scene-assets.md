# Full-scene appearance inputs

The licensed Origins scene is fully inventoried: 6,469 surfaces, 149,907 triangles,
449,721 ordered corners, 280 materials, 56 unique technique sets and 262 image
references including lighting. Original geometry identity is
`ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f`.

## Reproduction and boundary

External task-local native producers reuse the pinned public T6 decoder and
accepted geometry admission. They open the licensed zone read-only, join native
material/image pools and open installed zone image-container indexes. Resolving
DLC containers, which the earlier selected-surface search omitted, recovers all
260 backed images. Two remaining references are engine builtins without zone-pool
backing: an identity normal input and a shadow input. Their interpretation remains
explicitly unresolved; no invented replacement enters original scene admission.

Outputs remain external: native inventory, per-corner UV and original 36-byte
records, full secondary stream, decoded DDS, 56 representative technique contracts,
original bytecode/disassembly and world lighting. Material inventory retains every
material's constants, state words and technique-state entries. Representative
contracts retain pass routing/arguments; available passes do not prove dispatch.
The producers leave shared decoder sources and originals unchanged.

## Observed coverage

260 DDS images total 85,170,624 bytes: 28 BC1, 9 BC2, 149 BC3, 72 BC5 and
2 RGBA8. There are 31 cube probes and one lightmap record. All declared mip
payloads, cube faces and identities pass the safe Rust admission. BC5 uses DX10
DDS headers. Source sampler bytes are 20, 1, 10, 244 and 74; public enum semantics
identify filtering, disabled/nearest/linear mip selection and U/V/W clamp bits.

World vertex categories account for every surface: 0=4,154; 1=313; 2=1,679;
3=8; 4=105; 5=209; 7=1. Original capture qualifies the common 36-byte main
stream: XYZ/W float4, RGBA8 UNORM color, half2 base UV, packed 10/10/10/2
UNORM normal/tangent and UNORM16x2 lightmap UV. The reader preserves normalized
input channels; shader remapping is a separate rendering responsibility. Each
corner's source XYZ must equal its indexed admitted geometry position.

The captured runtime probe coefficients agree with source probe slot 2; the
external producer now retains round-trippable f32 coefficients for all 31 probes.
This qualifies a source-to-runtime binding for that observed draw, not all global
lighting/fog values. Secondary stream layouts are captured for categories 1, 2
and 4; remaining categories 3, 5 and 7 require further observations.

Name-inferred family grouping, not shader-equivalence proof: 197 lit materials
(5,899 surfaces), 56 layered lit (326), 22 unlit (100), one water (70), two
shadowcaster (62), one default (1), one burning (11). Color bindings vary from
one to four layers. Water has no color-semantic binding. First-color preview is
an explicit diagnostic choice and cannot establish original layered behavior.

## Runtime exchange and remaining limits

`bo2_assets::full_scene` adds version two shared image/material/surface tables,
geometry/UV/optional-main-stream SHA256 binding, original lighting slots and
source probe/lightmap records. It bounds files and aggregate DDS memory, validates
complete surface/UV accounting and rejects malformed DDS or unsupported contracts.
Version one remains unchanged. Only `diagnostic-unlit-color` is admitted;
full original material admission remains unsupported until rendering contracts,
secondary attributes, dynamic globals, builtin dispatch and complete family
implementation are established. Fidelity is INVESTIGATING.
