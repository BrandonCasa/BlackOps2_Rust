# Selected opaque shader contract

Scope: surface508 and the reusable mathematical response of its TECH4 lit shader.
This is sufficient to implement/test that response with explicit synthetic float
attributes and globals. It does **not** admit an original shaded scene yet.

## Observed data and ownership

The selected material has three textures: BC1 color, BC5 normal and BC3 specular /
gloss, each128x512 with10mips and sampler20(repeatUV,4x anisotropy,linear mips).
Material sampler slots are0/1/2 respectively. No emissive texture is present.
Secondary lightmap0 is RGBA8,1024x3072,12mips; primary lightmap is null.
Probe23 is a BC3 cube,128x128,8mips. All five reachable image dependencies resolve.
The selected ComWorld primary light0 has type0 and zero parameters. The world's
sun has a separate index; no additional directional light should be added merely
because that sun exists: TECH4 has baked illumination and probe reflection already.

Selected TECH4 has one pass and state2: opaque blending, alpha test disabled,
depth write enabled, LEQUAL depth test, cull enum2 and no polygon offset.
Original front-face convention remains unconfirmed. Its only material constant
is an occlusion-amount vector; it is not bound by this pass's recorded arguments.
The shaders/declaration are reusable T6 subassets owned by the admitted technique
set, not independent XAsset pools. Public loader and dumper confirm DXBC lengths
are bytes. An external replacement printer emits only this material's shaders,
arguments/states, six36-byte corner records and reachable images. Originals and
DXBC/disassembly/data remain external.

## Shader-proven response

Use componentwise products/squares, `sat(x)=clamp(x,0,1)`, epsilon=1e-6.
Define geometric normal N, tangent T, original handedness h, vertex color C,
base UV u, lightmap UV w, and camera-relative surface position P. The vertex
stage separately normalizes transformed N and T; it produces
`B=-sign(h)*cross(N,T)`. It passes base and lightmap UVs separately.
No orthogonalization or synthesized tangent is observed.

- Let sampled normal channels be a,b. Set x=4.015748a-2.015748,
  y=4.015748b-2.015748, and mapped normal M=normalize(N+xT+yB).
  This differs from ordinary2*sample-1 normal mapping and from reconstructing z.
- Let sampled specular RGB be S and gloss alpha g. With view vector
  V=-normalize(P), define e=2^(-9.28*sat(dot(M,V))) and
  f=1.041667g*min(e,0.475g)+0.018229g-0.015625.
  Reflection modulation is `sat(S²*(0.75+0.25g-f)+f)`.
- Sample the probe in reflect(normalize(P),M) at explicit LOD4*(1-g).
  Decode its radiance as RGB/(alpha+epsilon).
- Sample the secondary lightmap at (w.x,w.y/3), (w.x,(w.y+1)/3),
  (w.x,(w.y+2)/3). Decode first and second RGB/(alpha+epsilon) as
  ambient A and directional D. Decode third RGB*2-1 as direction L.
  Mapped diffuse irradiance is A+D*sat(dot(L,M)); geometric irradiance
  uses N in place of M. Do not add another approximation of this baked light.
- Vertex-stage probe SH vectors Q0,Q1,Q2 define a scalar
  k=dot(Q2,(Nz*Nx,Ny*Nz,Nx*Ny,Nx²-Ny²))
  +dot(Q1,(Nx,Ny,Nz,1))+Q0.w*Nz².
  The vertex output is inverse(max(Q0.rgb*k,0.1)); interpolate that output.
  Probe reflection is multiplied by geometric lightmap irradiance and this inverse.
- Multiply sampled base RGB by C.rgb, then square the product. Multiply by mapped
  diffuse irradiance and add the modulated probe term. There is no material alpha
  test/emissive operation in this pass; output alpha is1.
- The source vertex stage supplies a position-dependent fog color/transmittance.
  Blend shaded RGB toward that fog color, multiply by runtime HDR scale and take
  componentwise square root. This is an observed intermediate shader output;
  original texture-view/render-target transfer and subsequent postprocessing are
  unresolved. Do not silently substitute Bevy sRGB/PBR conventions.

TECH5 adds direct sun response, and TECH6 adds shadow-dependent sampling. Their
availability does not establish their dispatch for this surface. Its primary light0
provides no forward light, but serialized surface draw records are all zero
(runtime-allocated), so they cannot prove which technique runs in the original frame.
The lit-range and sorted-index direction likewise require consumer/runtime evidence.

## Attribute and runtime limits

The actual VS input signature consumes float position+handedness,color,baseUV,
normal,tangent and secondUV. Routing confirms their semantic roles. Six source
baseUVs retain the previously established values. The packed normal/tangent
words support signed10-bit candidate decoding; public helper directions are finite,
unit length after normalization, orthogonal, and produce the observed handedness.
However the VS consumes unsigned float components, remaps each by2*x below0.5
or2*x-2 above it, then normalizes. Runtime input format/conversion is not serialized
in the loaded declaration. The helper and a R10G10B10A2_UNORM hypothesis differ;
no pure Rust original decoder is approved by this evidence. Byte32 secondUV may
be normalized16-bit components, but this is an inference, not an admitted encoding.

A bounded read of the licensed executable found semantic-name records, but no
complete static D3D11 input-layout descriptor table. This attempt does not establish
those formats. The smallest next investigation is to observe the input-element
formats/offsets supplied for this one shader's input-layout creation, or locate
its concrete descriptor-construction routine; no full frame-capture framework
is needed. Keep exact packed decoding unsupported meanwhile.

Runtime HDR/fog parameters, camera-relative world transform, SH binding, custom
lightmap/probe samplers, shader-pass dispatch and target transfer are required
explicit inputs. Static sun exposure alone does not prove the HDR constant.
Implement shader math on synthetic float inputs now; original admission and
controlled comparison remain INVESTIGATING. No renderer changes or new bundle
schema were made in this investigation.
