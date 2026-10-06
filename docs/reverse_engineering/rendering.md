# Origins rendering

Bevy owns scenes, transforms, visibility, PBR, lights, shadows, cameras, filtering and post-processing. `src/rendering/opaque.rs` extends StandardMaterial with fresh embedded project-owned `opaque.wgsl` and `surface_vertex.wgsl`. The vertex adapter calls Bevy's mesh function and forwards authored layer attributes; shadow/depth passes retain Bevy layouts. Original shaders and captures are research references, never production inputs. No retired renderer, additional pipeline or viewer is present.

## Terrain and surface response

Preparation decodes retained half-float secondary UVs and packed signed normal-basis rotations. Four registered glTF attributes carry them: `_ORIGINS_LAYER_UV`, `_ORIGINS_LAYER3_UV`, `_ORIGINS_ROTATION1`, `_ORIGINS_ROTATION2`. They follow the same expanded corners as normals, tangents, lightmap UVs and RGBA controls. Source world triangles are clockwise relative to their normals; preparation reverses complete triangles to Bevy/glTF CCW. Source culling then preserves floors and walls. Static provider winding stays unchanged.

Readable `surface.layers` records base/blend/multiply/add operations, reveal parameters and control/basis interpretation. Binding hashes establish layer identity rather than texture-table order; sparse specular layers remain valid. Up to four colors and three normal/specular layers cover playable world combinations. Vertex G/B controls and overlay alpha form parameterized reveal masks. Secondary UVs sample matching overlays; packed 2x2 bases orient normal slopes. Blend weights combine normal/gloss/specular response. Multiply/add color overlays preserve the underlying response; a fourth color supplies macro modulation. Unrecovered reveal defaults are strength 0.25/exponent 1; the readable profile scales reveal width by 3.2 to soften hard decal transitions. This is a practical reconstruction.

`tile_macro_bake` retains the shared fourfold diffuse conversion, sampled once by Bevy. Raw macro red/green controls combine only normal and specular/gloss response. Static providers and placement references remain shared; UV/basis defaults attach to the existing mesh rather than copies per instance.

Raw linear normal RG uses slopes `4.015748 * sample - 2.015748`, with DirectX Y and prepared tangent handedness applied once. Raw specular RGB/gloss maps to squared RGB dielectric F0 and `pow(1-gloss, gloss_exponent)` roughness. Missing channels use zero slope, F0 0.04 and roughness 0.8. Metal-like tint remains a dielectric approximation; names do not infer metallicity.

## Special surfaces and water

Preparation reads retained TECH4/TECH2 blend, alpha-test and cull state. Explicit `surface.presentation` supplies alpha/cutoff, culling, emissive response, fog, visibility, collision suitability, wetness suitability and lamp disposition. Runtime blanket double-sided rendering and material-name substring guesses are removed. Legitimate uncullable surfaces remain two-sided; fences/foliage use cutouts. Tree-canopy transparency deliberately becomes a cutout, with reconstructed cutoff 0.5. Add/multiply/premultiplied/blend states use matching Bevy modes, including ONE/INV_SRC_ALPHA and ZERO/SRC_COLOR dispositions. Freshly lit premultiplied surfaces supply alpha-weighted radiance after fog; the decoded source DDS albedo itself is straight color. Separate static phong/emissive bindings supply glow instead of becoming whole-object albedo. Four explicit lamp identities place warm Bevy lights; other glowing objects do not become lamps.

Shadow/editor-default helpers are visually hidden. Nonsolid static water/unlit/effect surfaces are excluded from collision. The giant smoke-pillar atmosphere proxy is explicitly hidden: its native multistage effect is not reconstructed, and its previously solid rectangular planes duplicated fog and covered the sky. Single Bevy distance fog replaces that volume approximation.

Water uses both retained normal maps at different moving UVs, F0 0.02, adjustable tint/opacity/roughness, Bevy transparency and assigned filtered source reflection cubes. Authored water UVs span hundreds of repeats, so a reconstructed 0.02 scale makes the retained normal patterns visible as meter-scale ripples rather than subpixel noise. Real presentation time continues during pause; fixed-step gameplay is unchanged. Only cached water material handles update. Colored transparency reveals underlying geometry as simplified shallow refraction; no screen-space scene copy or depth distortion is used. Static surroundings reflect through moving normals; live actors do not enter those cubes.

Wetness applies only to explicitly suitable opaque lit surfaces. Up-facing orientation and broad world-space patches scale wetness, modestly darken albedo, move F0 toward 0.04 and lower roughness. This approximates damp ground/stone without repainting walls, foliage, glow or water, or blending tile/macro controls into diffuse twice.

## Lighting and atmosphere profile

`config/origins-visual.json` is embedded at build time: edit and rebuild to tune. It loads no local paths or captures. Light/surface/fog RGB values are sRGB; the generated HDR sky palette is linear radiance scaled by `sky_nits`.

| Controls | Meaning |
| --- | --- |
| ambient/sun color, brightness/illuminance and rotation | Cool actors/unbaked surfaces; baked diffuse excludes ambient/sun |
| indoor/outdoor EV100, adaptation speed | Cover-ray exposure; higher EV darkens the view |
| baked/vertex/probe/reflection radiance, instance probe fill | Separate atlas/vertex and SH calibration; subtle fill for missing local lighting |
| reveal softness, transparent opacity scale, post saturation | Tunable blending, glass coverage and restrained color grade |
| comparison FOV degrees | Opt-in capture projection; ordinary player camera is unchanged |
| normal/tile strength, gloss exponent, F0 scale | Continuous retained-channel response |
| bloom, fog RGB/density | AgX bloom and single exponential distance fog |
| lantern RGB/lumens/range, lamp/other emissive nits | Warm pools and glow; nearby lamps cast shadows |
| water tint/opacity/roughness/normals/UV scale/velocities | Transparent animated water |
| wetness/darkening/roughness | Damp up-facing solid surfaces |
| sky palette/nits/sun direction | Procedural storm cube supplied to Bevy Skybox |

Ambient brightness falls from 180 to 35, sunlight from 2600 to 1600 lux, lanterns from 6000 to 2600 lumens, bloom from 0.18 to 0.14 and fog density from 0.009 to 0.006. Indoor/outdoor EV100 8.25/8.7 balances warm rooms and cool outdoor rubble. Bevy scales skybox radiance by camera exposure once. Continuous procedural storm clouds replace the plain clear background through standard Bevy Skybox, approximating the supplied reference sky.

Packed world atlases decode linear RGB/alpha ambient and directional radiance plus source-basis direction. Irradiance is `A + D * clamp(dot(L, mapped_normal), 0, 1)`, with band-local insets. Static instances interpolate retained LOD0 vertex colors or use assigned source-basis SH. Preparation already retains original LOD vertex identity for expanded provider corners. The vertex adapter samples a small per-instance RGBA16F lighting texture through that identity; providers and placements remain shared. Alpha remains lighting metadata, never surface opacity. SH uses 22 radiance units versus atlas 90 and vertex 140, with no artificial minimum; vertex lighting gets a modest 0.035 probe fill to retain readable colored faces in deep shade. These independent gains are reconstruction controls. A zero Bevy Lightmap marker excludes duplicate ambient/sun diffuse; the extension supplies baked radiance once. Emission follows camera exposure (Bevy emissive exposure weight 1). Lamp emission is warm at 4,000 nits; other emissive regions use 150 nits so equipment gauges do not compete with the lanterns. Point lights illuminate baked surfaces; actors/unbaked meshes use ordinary profile lighting. Unsupported special families retain their standard fallback.

Assigned source cubes decode to unclipped linear RGBA16F. Bevy owns GGX filtering/mips. WGSL rotates reflection directions to source Z-up, selects roughness mips and uses Bevy's split-sum response. Reflection enters scene-linear lighting once and preserves transparent alpha before fog/post-processing. Albedo is sRGB; normal/specular/control/lightmap/probe data are linear. No native intermediate square-root transfer or extra albedo squaring is reproduced. AgX, display transfer, exposure and bloom remain Bevy-owned. Runtime RGBA8 material images receive box-filtered mip levels and eightfold anisotropic sampling; color mips filter in linear light, while data/alpha remains linear. Packed lightmap atlases stay exempt, and Bevy continues to filter reflection cubes. These are reconstructed levels, not recovered original DDS mips.

## Fidelity and evidence

The render root stays hidden until material/lightmap assembly completes. The actual-window driver traverses combat/movement/round/restart, sweeps eight map positions/yaws, adds five room directions and repeats a stationary water view. Comparison captures hide player overlays and record actual camera position/yaw/pitch/FOV/aspect in the receipt. Compiler gates cannot validate WGSL; game logs and inspected captures are required. Asset checks establish integrity, never original equivalence. Current outcomes/failures belong in the single current handoff.

Comparison targets supplied room colors, warm lamps, cool rubble, vegetation, distant haze and overcast sky. Remaining approximations include reveal constants, native radiance/lobe transfer, SH/basis interpretation, shallow water refraction, static reflections, procedural sky, canopy cutoff and absent source mip chains. Original volumetric smoke/cloud animation and temporary actor/weapon replacement are not implemented. Lantern bloom approximates halos; distance fog does not reproduce the reference's localized drifting smoke pockets. The world-material milestone does not replace the temporary pistol, hands or diagnostic HUD.
