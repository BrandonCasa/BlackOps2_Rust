# Origins implementation checklist

Complete the current implementation using the existing Rust asset builder, committed in-memory material baker, runtime pack format, Bevy, Avian, and Solari integration.

This checklist includes graphics and rendering work because the user explicitly requested a todo.md for the complete six-phase plan. It is the sole project checklist.

## Execution order and working constraints

- Phase 1 is priority number one: complete every one of the 165 remaining material families before implementing later phases.
- Material-dependent camo, script parameters, animated states, and required geometry inputs belong to phase 1. Do not postpone them to the general animation or remaining-assets phases.
- Defer game runtime testing until material conversion and runtime bindings are complete. Later pack dependencies may still need completion before a game launch is possible.
- Use the current checkout and work completed after 6 p.m. Eastern on October 12, 2026. Ignore older descriptions of superseded implementations.
- Trust and extend the committed baking implementation. Keep its current PBR conversion, layered geometry baking, atlas handling, tangent handling, and runtime material architecture.
- Use only the authorized Steam installation for source assets and the existing C-drive t6zm-master project for reverse engineering, as specified in AGENTS.md.
- Read and transform source assets in memory. Write generated assets and generated runtime data only inside runtime pack files.
- Never commit original assets, converted assets, packs, generated executables, or machine-specific configuration.
- Keep tools/build_assets.rs and its existing Rust modules as the sole extraction and conversion pipeline.
- Preserve one Cargo package, origins.exe as the game executable, safe Rust, static Bevy linking, and the locked dependencies.
- Do not add subagents, read other tasks, add custom crates, patch dependency sources, introduce vendor copies, or create competing builders.
- Keep source gaps explicit. Do not substitute invented gameplay values, generic material fallbacks, empty required catalogs, or discarded behavior.
- Use normal Git history and Cargo.lock. Do not add identity manifests, SHA gates, source snapshots, provenance systems, or external comparison receipts.
- Preserve unrelated working-tree and staged changes. Stage or commit only when authorized.
- Mark tasks complete only when their stated implementation and checks are actually finished. Record concrete remaining limitations beside the affected task.
- Batch at least five concrete todo implementations before each new test run; prefer combined focused checks, as requested by the user.
- Check selected programs, active bindings, coordinate routes and render states for every affected material, then use representative conversions for new operations during development. Reserve exhaustive full-resolution conversion for integration checkpoints and final acceptance.
- Share decoded zones, material metadata and image-package indices within each combined run. Repeat geometry/export checks when their inputs change; keep full metadata checks, representative conversion, exhaustive conversion, runtime connection and visual acceptance distinct.

## Current implementation to build on

The current checkout already contains the Rust pack builder and runtime pack integration, ordinary PBR material conversion, layered world baking, mixed tangent-handedness handling, base-alpha response and reveal baking, native static-model LODs, brush collision bindings, ordinary animation decoding, primitive trigger conversion, crafting bench bindings, and source zone graph/flag activation bindings.

Relevant recent commits: 6f691a14c, 10ffa5a4d, ffc8d1961, a3607cfa1, cfb1b4abb, 8bd71a794, 0a67833bb, aa5665325, 88744c4a5, d07247bdb, and b13e79ab6. These describe the baseline to extend, not work to redo.

The post-cutoff geometry-used material inventory contains 180 exact technique families and 1,034 materials. Starting dispatch covered 15 families. The extensions now dispatch twenty-seven additional families covering 117 geometry-used materials. The first twelve families passed all 72 installed-material conversions; the latest fifteen recipes cover 45 more materials. Combined installed-source preflight passed for those 45 and the previous 38 ordinary materials: exact programs, active bindings, routes, states and 112 image definitions checked for 83 materials in twenty families. Seventeen representative materials converted from 25 native images; nine identical image encodings were shared in memory. Exhaustive conversion of the new families remains pending. The four additive-unlit materials also require source fog-attenuation runtime support before pack closure. Full-pack and runtime acceptance remain pending. Phase 1 below explicitly lists all 165 remaining families and their 664 currently used materials. Counts describe the starting backlog; an allow-list entry alone does not complete a family.

# Phase 1 - Complete all 165 remaining material families

Primary files: tools/asset_builder/material.rs, bake.rs, geometry.rs, texture.rs, scenes.rs, gltf.rs, source.rs, src/rendering/opaque.rs, src/rendering/pbr.rs, and src/assets/prepared.rs.

## 1.1 Establish the exact material work inside the existing pipeline

- [x] Reconcile the existing geometry-used family list against current ordinary_recipe and bake::supported dispatch before changing conversion. Installed-source check confirmed 1,034 materials, 180 families, 15 dispatched families and 165 missing families before extension. Exact checklist-family reconciliation passed for the first twelve additions. The combined current inventory reads all 1,034 used materials and reports 138 missing dispatch families after twenty-seven recipe additions; the combined binding and representative conversion check passed without claiming full-family or runtime acceptance.
- [ ] Keep all 165 listed families in scope, including one-material families and families used by infrequently visible content.
- [ ] Include required material dependencies introduced by exact weapon, attachment, camo, FX, and presentation bindings.
  - [x] Expand full-builder material requirements with nonnull distinct source/replacement pairs reached by the installed Origins upgraded-camo option. The current table selects material set 13 for camo ID 45; 35 native camo definitions supply 68 records, seven zero-pair records, and 112 source/replacement material dependencies. Actual descriptor eligibility and other FX/presentation dependencies remain open.
- [ ] Resolve each required material through the existing zone/patch selection rules.
- [ ] Reuse post-cutoff recovered equations and bindings where they already match the selected programs.
- [ ] Use bounded Ghidra decompilation only for missing selectors, constant writers, material consumers, or source equations.
- [ ] Serialize access to the existing Ghidra project and keep all reverse-engineering work in the authorized workspace.
- [ ] Implement shared operations in the existing modules only where actual selected-program behavior agrees.
- [ ] Keep separate exact family recipes where inputs, transfer order, states, or runtime selectors differ.
- [ ] Preserve precise failures for missing source bindings and unsupported source operations while implementing each family.

## 1.2 Source-faithful channel and render-state conversion

- [ ] Select the actual source technique and passes for each family; support valid unlit and special passes without assuming technique 4 exists.
- [ ] Resolve selected vertex and pixel programs directly from the installed material/technique definitions.
- [ ] Resolve active texture arguments rather than treating every stored material texture as a shader input.
- [ ] Resolve active constant arguments rather than treating every stored material constant as used.
- [ ] Preserve authored texture coordinate sets and auxiliary vertex streams required by selected programs.
- [ ] Preserve sampler wrap, clamp, filtering, and per-input coordinate behavior.
- [ ] Preserve the source order of texture mixing, vertex tint, gamma transfer, reveal operations, and response fitting.
- [ ] Distinguish coverage alpha, gloss alpha, layer masks, emissive masks, and camo masks.
- [ ] Preserve source alpha-test thresholds and the channels participating in the test.
- [ ] Preserve source blend factors, blend operations, premultiplication, depth behavior, culling, and visibility.
- [ ] Preserve source fog participation and fog behavior for emissive, additive, replace, and decal materials.
  - [ ] Bind attenuation-only native unlit additive fog through the existing runtime material path. Five new unlit recipes retain the source mode; four additive materials remain blocked by an explicit pack-closure error instead of adding the shared fog color.
- [ ] Convert authored normal slopes using the committed source reconstruction and tangent-frame handling.
- [ ] Carry authored specular RGB and gloss through the existing PBR conversion where the source uses them.
- [ ] Preserve diffuse-only source behavior without inventing missing authored specular inputs.
- [ ] Preserve source emissive scaling and masks separately from albedo.
- [ ] Preserve authored occlusion inputs and their distinct consumers where selected programs use them.
  - [ ] Connect active non-unit occlusionAmount.x/y/z separately to direct specular, probe specular and diffuse lighting. Current ordinary conversion retains the native binding and values; metadata preflight accepts finite source values without requiring neutral constants. The installed mc/mtl_test_sphere_silver_lores exposes this remaining runtime requirement. The corrected finite-value metadata guard passed in the combined installed-source check; separate runtime lighting consumers remain pending.
- [ ] Preserve special view-, depth-, or state-dependent terms through existing supported runtime paths.
- [ ] Keep known PBR fitting and finite-bake approximations explicit without using them to omit a family.
- [ ] Reject non-finite generated channels and invalid texture dimensions at the existing conversion boundary.
- [ ] Encode source-derived outputs using the current KTX2/compression path and validate their mip layouts.
  - [x] Reuse identical linear/sRGB native-image payloads in memory while retaining both referenced URIs. Implemented for normal/response roles and half-float source images; the combined installed-source check confirmed nine shared encodings with both KTX2 URI layouts valid. This is reuse evidence, not a measured speedup.

## 1.3 Extend the verified layered baker

- [ ] Add the remaining exact layered recipes to the current recipe dispatch.
- [ ] Extend existing input evaluation for the remaining two-, three-, and higher-layer combinations actually referenced by Origins.
- [ ] Preserve all authored blend weights from vertex channels and source textures.
- [ ] Preserve each reveal curve's selected constants, operation order, and legal source domain.
- [ ] Preserve base-alpha gloss independently from surface coverage.
- [ ] Preserve layer specular/gloss contributions and their source mixing order.
- [ ] Extend multiply and add layer operations using recovered selected-pass equations.
- [ ] Keep material-local layer operations distinct from receiver-dependent framebuffer decal behavior.
- [ ] Carry required UV1/UV2 and auxiliary data into each bake.
- [ ] Reuse the committed periodic bake path where its source conditions hold.
- [ ] Reuse the committed atlas path for geometry-dependent inputs and variable vertex data.
- [ ] Preserve source texel density and source sampler behavior while extending recipes.
- [ ] Preserve mixed native tangent handedness and the committed destination tangent-frame conversion.
- [ ] Preserve geometry, material assignments, skinning data, and source triangle counts when partitioning baked surfaces.
- [ ] Implement source-preserving subdivision only where an actual source chart exceeds the current atlas page limit.
- [ ] Preserve chart padding, normal handling, response handling, and mip generation across new outputs.
- [ ] Include required static-model/model geometry in the current baking flow when its materials depend on geometry inputs.
- [ ] Keep baked material dependencies reachable through the final scene and pack entries.

## 1.4 Material selectors, animation, and Solari runtime bindings

- [ ] Recover exact weapon and attachment material selections for every required camo-dependent family.
- [x] Read native camo pattern and material-set data directly through the existing Rust pipeline. Reads all 35 installed definitions, both base images, every pattern tuple/image pair, ordered material sets/pairs/flags, all eight record constants, and the native 128-slot option table. Positive-count null arrays fail explicitly; null individual pairs retain the native rejection behavior. Generated runtime serialization and consumers remain separate outstanding work.
- [ ] Apply source surface masks and material scope to texture and constant overrides. Native input planner implements low-16-bit model-index eligibility, destination-material scope, source semantic 2/5/8 to destination 2/13/14 preservation and command order. Focused checks pass, including indices 16..31, scope mismatch and retained destination samplers. Live descriptor construction and runtime application remain pending.
- [ ] Preserve source pattern-mode image replacement order and success conditions. Native planner preserves first/second swap order, null/equal rejection, the shared 1,024-command limit, partial first-swap success and the requirement that both swaps succeed before the parameter write. Focused checks pass; the runtime selector and submission consumer remain pending.
- [ ] Preserve material-mode replacement lists and the selected replacement materials. Native planner preserves set/record/pair order, zero-pair rejection and newest matching material choice without recursive replacement chaining. Controlled installed checks resolve 105 model-index binding plans across the 35 camo definitions. This is source-input validation, not proof of active gameplay descriptors or completed variant bakes.
- [ ] Preserve the active weaponParam2 values and their disabled/default state. Planner reads exact pattern offset/scale, writes [x,y,scale,1] only after both image swaps succeed, and preserves the supplied initial constants otherwise. Checks cover disabled zero input, partial swap failure, capacity exhaustion, option masking and the mapped-distance boundary. Runtime per-object registration, refresh and reset remain pending.
- [ ] Preserve the active weaponParam6 values for material-set paths that consume them. Planner delivers record XYZ and W+10 only when flag 2 is set, scoped to the replacement material. Consumer checks preserve head-first traversal, including older matching constant writes overwriting newer ones as in the recovered native consumer. All eight record constants remain available; selected shader use and live runtime delivery remain pending.
- [ ] Resolve required model, weapon, attachment, and hand ownership for material state selection.
- [ ] Reset material overrides on weapon replacement, unequip, owner destruction, and session reset.
- [ ] Convert source time controls, offsets, and UV animation inputs into the existing UvAnimation path where applicable.
- [ ] Convert source scalar lookup/curve controls into the existing ScalarAnimation path where applicable.
- [ ] Use existing AnimatedMaterial state/frame support where it represents the recovered source behavior.
- [ ] Derive required animation timing, loop boundaries, and state selection from source inputs.
- [ ] Preserve continuous source controls through existing supported runtime updates where finite state selection is insufficient.
- [ ] Wire script-vector producers to NativeArtInputs for materials that actually consume those vectors.
- [ ] Complete charged staff material inputs and their normal/charged state selection.
- [ ] Complete frozen stone and ice material inputs and their source progression.
- [ ] Complete source-driven emissive, flicker, portal, scrolling, and electrical state consumers.
- [ ] Preserve character damage/reveal and ember/emissive state inputs for the families that use them.
- [ ] Preserve required flag/cloth material geometry motion through the existing supported geometry and Solari integration.
- [ ] Keep scope, reticle, lens, and viewmodel material ownership and visibility tied to actual weapon state.
- [ ] Complete source-derived water, glass, cornea, and transparent material bindings through the current rendering architecture.
- [ ] Ensure changed material handles and textures reach the existing Solari scene/material update path.
- [ ] Ensure shared material reuse does not leak one object's animated state into another object.
- [ ] Ensure material animation waits for required packed texture dependencies before applying a frame.
- [ ] Keep source lighting and view-dependent response out of fixed albedo/emission bakes.
- [ ] Preserve the current source-derived atmosphere and color response while adding material coverage.

## 1.5 Exact family backlog

Each family remains incomplete until its selected-program behavior is implemented, all listed source materials convert, the emitted pack dependencies resolve, and its intended existing runtime/Solari binding is connected. The second task under each family identifies a focused source behavior to resolve; the family name is a work locator, not proof of that behavior.

### 1.5.1 Lit model and world families

46 families; 246 currently used materials. Extend material.rs and existing runtime material consumers.

- [ ] Convert family `mc_lit_sm_r0c0n0s0o0_3z86zq2z` for all 92 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve active occlusion/detail inputs and any scoped camo texture/constant overrides, then bind the source-selected material state.
- [ ] Convert family `mlv_lit_sm_r0c0n0x0_619829wj` for all 23 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `wpc_lit_sm_t0c0_2w3887e6` for all 15 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves geometric normals, diffuse-only response, source texture-alpha × vertex-alpha coverage and the native 128/255 cutoff. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `wpc_lit_sm_b0c0n0s0v0_efz5w56f` for all 10 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_b0c0s0_805w75j2` for all 8 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_r0c0s0_986ezzjq` for all 8 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves geometric normals, authored specular RGB/gloss, source gamma transfer and existing metallic/GGX fitting. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `mlv_lit_sm_r0c0_j60qqj0e` for all 8 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mc_lit_sm_t0c0_90wz6fe2` for all 7 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves geometric normals, diffuse-only response, source texture-alpha × vertex-alpha coverage and the native 128/255 cutoff. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `wpc_lit_sm_r0c0n0_80fe30wz` for all 6 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves authored normal slopes, diffuse-only response with no invented specular map, source gamma transfer and ordinary world bindings. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve the actual selected color, geometric/mapped normal, lighting, coverage, and specular inputs without assuming another family's recipe.
- [ ] Convert family `mlv_lit_sm_r0c0n0s0_wqqj839w` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mc_lit_sm_b0c0_fw0jf955` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_r0c0n0s0_ze50j7jz` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `mc_lit_sm_r0c0n0s0o0_909j993q` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve active occlusion/detail inputs and any scoped camo texture/constant overrides, then bind the source-selected material state.
- [ ] Convert family `mc_lit_sm_b0c0n0v0_3559e8jj` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve material-specific coverage, normal/response inputs, and required source geometry motion for the flag path.
- [ ] Convert family `mc_lit_sm_r0c0n0s0_7j414q0z` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `mlv_lit_sm_r0c0n0_3qf192ew` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `wpc_lit_sm_b0c0n0_19f3588f` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_b0c0n0s0_68f25z0u` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_flag_b0c0s0_6406ez95` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve material-specific coverage, normal/response inputs, and required source geometry motion for the flag path.
- [ ] Convert family `mc_lit_sm_r0c0d0n0_33ffej1u` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected detail-map operations, coordinates, and interaction with base normal/specular/coverage inputs.
- [ ] Convert family `mc_lit_sm_t0c0n0_9qf6e4qj` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `mlv_lit_sm_t0c0n0s0_9jfj95q5` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `wpc_lit_sm_b0c0_6j30z67q` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `wpc_lit_sm_b0c0n0v0_ew347fe3` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `wpc_lit_sm_t0c0n0_9ew0jz24` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves authored normal slopes, diffuse-only response, source texture-alpha × vertex-alpha coverage and the native 128/255 cutoff. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `wpc_lit_sm_t0c0n0s0_weef63f3` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `mc_lit_sm_b0c0n0_67fqj7jf` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_b0c0n0s0_21fz2e61` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_b0c0n0s0o0_jf00fe4q` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve active occlusion/detail inputs and any scoped camo texture/constant overrides, then bind the source-selected material state.
- [ ] Convert family `mc_lit_sm_b0c0s0_q94f27qw` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_r0c0_5137qq99` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the actual selected color, geometric/mapped normal, lighting, coverage, and specular inputs without assuming another family's recipe.
- [ ] Convert family `mc_lit_sm_r0c0d0n0s0_qwq4q449` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected detail-map operations, coordinates, and interaction with base normal/specular/coverage inputs.
- [ ] Convert family `mc_lit_sm_r0c0n0s0_4w3wqfe5` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `mc_lit_sm_r0c0n0x0_53144qq1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the source alpha response and selected specular/gloss consumers independently from coverage.
- [ ] Convert family `mc_lit_sm_t0c0d0n0s0_68eww765` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected detail-map operations, coordinates, and interaction with base normal/specular/coverage inputs.
- [ ] Convert family `mc_lit_sm_t0c0n0s0_f933q84u` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `mlv_lit_sm_b0c0_50077370` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_b0c0n0_322zqw73` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_b0c0s0_41731e52` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_r0c0_475q06w3` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_r0c0d0n0_w65788j0` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_r0c0n0x0_8f47740f` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `wpc_lit_sm_b0c0s0_3f3q946z` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `wpc_lit_sm_r0c0s0_qfqz7850` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `wpc_lit_sm_r0c0x0_5j63fj97` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the source alpha response and selected specular/gloss consumers independently from coverage.

### 1.5.2 Remaining merged layered families

26 families; 92 currently used materials. Extend bake.rs and the existing geometry-aware bake outputs.

- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2` for all 14 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2` for all 11 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1` for all 10 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; all ten installed materials passed conversion/export checks across 84 surfaces and 441 triangles. Preserves layer-alpha × vertex-green encoded-color mixing before gamma transfer, base normal, and base-alpha gloss. Only the three selected texture arguments are applied; stored identity layer normals are inactive. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement this layer combination's active color, normal, specular/gloss, coordinate, and vertex-weight inputs.
- [ ] Convert family `lit_sm_r0c0n0x0_m1c1` for all 10 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; all ten installed materials passed conversion/export checks across 116 surfaces and 1,002 triangles. Preserves multiply-before-gamma color, vertex-green weight, half2 UVs, authored wrap/clamp, base normal, and base-alpha gloss modulated by encoded tint luminance. Periodic baking is used only where UV/frame/sampler conditions permit it; other surfaces use the existing atlas path. Combined multiply-family checks passed for 15 materials, 168 surfaces, 1,371 triangles and 84 in-memory pack entries. Native environment/direct-lobe/GGX fitting and raster tangent-frame approximations remain explicit; full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the authored multiply layer, including its vertex/mask weight and interaction with earlier color/response operations.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1v1` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; all five installed materials passed conversion/export checks across 159 surfaces, 2,052 triangles and 45 in-memory pack entries. Preserves the selected reveal curve including authored zero exponents, UV/auxiliary normal-frame inputs, reveal-weighted color/normal mixing, and unchanged base-alpha gloss. Native view-dependent response fitting and raster tangent-frame differences remain approximations; full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the selected reveal curve and its effect on color, normal, specular/gloss, and base-alpha response.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0_m1c1` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with the recovered multiply-before-gamma equation, base normal, vertex-green weight, auxiliary half2 UVs, and source wrap/clamp samplers; installed conversion/export checks passed for all three materials across 46 surfaces and 343 triangles, with packed KTX2 mip layouts validated. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the authored multiply layer, including its vertex/mask weight and interaction with earlier color/response operations.
- [ ] Convert family `lit_sm_r0c0n0s0_b1c1n1s1v1` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the selected reveal curve and its effect on color, normal, specular/gloss, and base-alpha response.
- [ ] Convert family `lit_sm_r0c0n0x0_a1c1` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; all three installed materials passed conversion/export checks across 62 surfaces, 314 triangles and 28 in-memory pack entries. Preserves source RGB × layer-alpha × vertex-green addition before gamma transfer, unchanged base normal/gloss, and auxiliary half2 UVs. Above-one color values use linear RGBA16F KTX2 with linear mips; existing PBR texture loading uses the matching linear color-space binding. Oversized charts are rotated into bounded pages where feasible without changing source triangles, UV metrics, or texel density; charts that cannot fit still fail explicitly. Sparse large charts incur substantial atlas/pack memory cost. Full-pack and actual-game acceptance remain pending.
  - [ ] Resolve and implement the authored add-layer contribution and its coverage, tint, and response ordering.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement this layer combination's active color, normal, specular/gloss, coordinate, and vertex-weight inputs.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the authored multiply layer, including its vertex/mask weight and interaction with earlier color/response operations.
- [ ] Convert family `lit_sm_r0c0n0_m1c1_m2c2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented and both installed materials passed conversion/export checks across six surfaces and 26 triangles. Green/blue weights, separate half2 UV pairs, multiply-before-gamma order, unchanged base normal, and authored samplers are preserved. The combined multiply-family check passed for five materials, 52 surfaces, 369 triangles and 22 in-memory pack entries. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the authored multiply layer, including its vertex/mask weight and interaction with earlier color/response operations.
- [ ] Convert family `lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1_b2c2n2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1_a2c2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the authored add-layer contribution and its coverage, tint, and response ordering.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1_b2c2n2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the authored add-layer contribution and its coverage, tint, and response ordering.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0s0_b1c1n1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement this layer combination's active color, normal, specular/gloss, coordinate, and vertex-weight inputs.
- [ ] Convert family `lit_sm_r0c0n0x0_a1c1_m2c2` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the combined multiply/add layer order and each layer's authored coordinates and masks.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1_b2c2` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; the installed material passed conversion/export checks across one surface and four triangles. Preserves ordered green/blue alpha-weighted color blends, separate UV pairs, base normal/gloss, opaque coverage, and selected-pass samplers. Combined color-blend checks passed for eleven materials, 85 surfaces, 445 triangles and 75 in-memory pack entries. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1_b2c2n2s2` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement the combined multiply/add layer order and each layer's authored coordinates and masks.
- [ ] Convert family `lit_sm_r0c0n0x0_t1c1n1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected pass's coverage and blend behavior and implement the additional layer without losing its authored surface response.
- [ ] Convert family `lit_sm_r0c0s0_b1c1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve and implement this layer combination's active color, normal, specular/gloss, coordinate, and vertex-weight inputs.

### 1.5.3 Unlit and decal families

31 families; 92 currently used materials. Extend material.rs, bake.rs where needed, and existing runtime material consumers.

- [ ] Convert family `mc_unlit_replace_3j970129` for all 12 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Technique-2 converter implemented from the selected installed unlit programs, active scaleRGB binding and exact native states. Preserves gamma-2 surface color, authored HDR intensity and the distinct opaque/add/blend alpha roles. Blend coverage premultiplies color before transfer; additive RGB ignores source alpha because its native RGB/alpha factors do not consume it. Focused HDR/coverage, vertex-alpha export and pending-runtime closure checks passed. Installed-source preflight passed for all 83 materials in twenty families, including all twenty unlit materials, and 17 representative conversions passed across the fifteen new families. Exhaustive conversion, polygon-offset behavior, shared linear-HDR composition and actual-game acceptance remain open.
  - [x] Resolve the unlit output, source intensity, vertex transfer, opacity, blend operation, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_normal_script_q0zez5ju` for all 7 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the multi-layer normal/color equations and script-vector controls, then bind their source producers.
- [ ] Convert family `wpc_sw4_3d_unlit_1layer_8691j77q` for all 7 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_model_unlit_cheap_zombie_eyes_jq3e7eqw` for all 6 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the eye emission, opacity, and actor-state selection required by the selected unlit program.
- [ ] Convert family `mlv_sw4_3d_unlit_3layer_normal_script_766fz50u` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the multi-layer normal/color equations and script-vector controls, then bind their source producers.
- [ ] Convert family `wpc_unlitdecalblend_add_j26wq580` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `wpc_unlitdecalblend_multiply_35079164` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `mc_sw4_3d_unlit_4layer_nofog_eee768jw` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_scroll_jq69qew7` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source UV motion and timing and bind them to the existing UV/material animation consumer.
- [ ] Convert family `mlv_sw4_3d_unlit_3layer_nofog_falloff_zfeather_z0zw1090` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve layer composition, view/depth falloff, feathering, fog behavior, and required live inputs.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the layer-composition and falloff equations and preserve their source live inputs and fog behavior.
- [ ] Convert family `mc_sw4_3d_viewmodel_unlit_j299w86e` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve viewmodel-specific opacity, emissive/unlit output, fog/depth behavior, and weapon-state ownership.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_falloff_5z943qe4` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the layer-composition and falloff equations and preserve their source live inputs and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_falloff_zfeather_wq4jwzz6` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve layer composition, view/depth falloff, feathering, fog behavior, and required live inputs.
- [ ] Convert family `mc_sw4_3d_viewmodel_unlit_8216051u` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve viewmodel-specific opacity, emissive/unlit output, fog/depth behavior, and weapon-state ownership.
- [ ] Convert family `mc_sw4_3d_viewmodel_unlit_alpha_85z6j5w5` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve viewmodel-specific opacity, emissive/unlit output, fog/depth behavior, and weapon-state ownership.
- [ ] Convert family `mc_unlit_add_ww82zw88` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Technique-2 converter implemented from the selected installed unlit programs, active scaleRGB binding and exact native states. Preserves gamma-2 surface color, authored HDR intensity and the distinct opaque/add/blend alpha roles. Blend coverage premultiplies color before transfer; additive RGB ignores source alpha because its native RGB/alpha factors do not consume it. Focused HDR/coverage, vertex-alpha export and pending-runtime closure checks passed. Installed-source preflight passed for all 83 materials in twenty families, including all twenty unlit materials, and 17 representative conversions passed across the fifteen new families. Exhaustive conversion, polygon-offset behavior, shared linear-HDR composition and actual-game acceptance remain open. The source attenuation-only fog operation remains a precise required-content failure until its runtime consumer exists.
  - [x] Resolve the unlit output, source intensity, vertex transfer, opacity, blend operation, and fog behavior.
- [ ] Convert family `mc_unlit_blend_588ewzff` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Technique-2 converter implemented from the selected installed unlit programs, active scaleRGB binding and exact native states. Preserves gamma-2 surface color, authored HDR intensity and the distinct opaque/add/blend alpha roles. Blend coverage premultiplies color before transfer; additive RGB ignores source alpha because its native RGB/alpha factors do not consume it. Focused HDR/coverage, vertex-alpha export and pending-runtime closure checks passed. Installed-source preflight passed for all 83 materials in twenty families, including all twenty unlit materials, and 17 representative conversions passed across the fifteen new families. Exhaustive conversion, polygon-offset behavior, shared linear-HDR composition and actual-game acceptance remain open.
  - [x] Resolve the unlit output, source intensity, vertex transfer, opacity, blend operation, and fog behavior.
- [ ] Convert family `wpc_unlit_add_8ez3wzw3` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Technique-2 converter implemented from the selected installed unlit programs, active scaleRGB binding and exact native states. Preserves gamma-2 surface color, authored HDR intensity and the distinct opaque/add/blend alpha roles. Blend coverage premultiplies color before transfer; additive RGB ignores source alpha because its native RGB/alpha factors do not consume it. Focused HDR/coverage, vertex-alpha export and pending-runtime closure checks passed. Installed-source preflight passed for all 83 materials in twenty families, including all twenty unlit materials, and 17 representative conversions passed across the fifteen new families. Exhaustive conversion, polygon-offset behavior, shared linear-HDR composition and actual-game acceptance remain open. The source attenuation-only fog operation remains a precise required-content failure until its runtime consumer exists.
  - [x] Resolve the unlit output, source intensity, vertex transfer, opacity, blend operation, and fog behavior.
- [ ] Convert family `wpc_unlit_replace_4688792e` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Technique-2 converter implemented from the selected installed unlit programs, active scaleRGB binding and exact native states. Preserves gamma-2 surface color, authored HDR intensity and the distinct opaque/add/blend alpha roles. Blend coverage premultiplies color before transfer; additive RGB ignores source alpha because its native RGB/alpha factors do not consume it. Focused HDR/coverage, vertex-alpha export and pending-runtime closure checks passed. Installed-source preflight passed for all 83 materials in twenty families, including all twenty unlit materials, and 17 representative conversions passed across the fifteen new families. Exhaustive conversion, polygon-offset behavior, shared linear-HDR composition and actual-game acceptance remain open.
  - [x] Resolve the unlit output, source intensity, vertex transfer, opacity, blend operation, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_1layer_nofog_91f00f80` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_falloff_5z909874` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the layer-composition and falloff equations and preserve their source live inputs and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_qw71w785` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_4layer_jw0z1973` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_viewmodel_unlit_60466j88` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve viewmodel-specific opacity, emissive/unlit output, fog/depth behavior, and weapon-state ownership.
- [ ] Convert family `mc_unlit_multiply_97923qz1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `mc_unlitdecalblend_add_fzw1z66f` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `mc_unlitdecalblend_multiply_34w7q3jw` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `mlv_sw4_3d_unlit_3layer_nofog_falloff_fjz6ze0u` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the layer-composition and falloff equations and preserve their source live inputs and fog behavior.
- [ ] Convert family `mlv_sw4_3d_unlit_4layer_nofog_2j0ezz67` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `wpc_sw4_3d_unlit_3layer_normal_script_q1z1f6w1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the multi-layer normal/color equations and script-vector controls, then bind their source producers.

### 1.5.4 Special material families

62 families; 234 currently used materials. Extend the existing material/bake modules and their runtime consumers.

- [ ] Convert family `mc_sw4_3d_phong_emissive_alcatraz_90892wq5` for all 19 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_viewmodel_transparent_z9z0z75f` for all 18 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native transparency, normal/specular response, view dependence, and scope/viewmodel ownership through the current Solari material path.
- [ ] Convert family `mc_sw4_3d_wpn_zmb_staff_3layer_94jz2ff5` for all 17 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve all staff layers, source camo/material replacements, and charge/state parameters, then bind the active staff owner.
- [ ] Convert family `mc_sw4_3d_phong_emissive_q0751958` for all 15 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_char_cloth_4z8fq5wu` for all 14 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_char_skin_j92387z3` for all 14 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_char_cloth_reveal2_9z738w66` for all 12 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_char_cloth_reveal2_727ej010` for all 9 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_cod7_tile_blend_spec_8w40z341` for all 9 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve authored tile coordinates, blend weights, and active normal/specular contributions using the committed bake machinery.
- [ ] Convert family `mc_sw4_3d_optic_lens_4w47028j` for all 8 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native transparency, normal/specular response, view dependence, and scope/viewmodel ownership through the current Solari material path.
- [ ] Convert family `mc_sw4_3d_light_sphere_46j58w7u` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected effect's geometry/material controls, source animation/state, opacity, emission, and depth behavior.
- [ ] Convert family `mc_sw4_3d_scrolling_falloff_zfeather_69456263` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve scrolling coordinates, falloff/distortion inputs, source timing, coverage, and depth-dependent terms.
- [ ] Convert family `mlv_sw4_3d_cod7_tile_blend_spec_w8wezqe6` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve authored tile coordinates, blend weights, and active normal/specular contributions using the committed bake machinery.
- [ ] Convert family `mc_sw4_3d_char_hair_287j114q` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_char_skin_emberglow_7q60f126` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_cod7_tile_blend_q2q2zej6` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve authored tile coordinates, blend weights, and active normal/specular contributions using the committed bake machinery.
- [ ] Convert family `mc_sw4_3d_portal_e96z3z92` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve portal layers, source animation/state inputs, opacity, and view/depth response.
- [ ] Convert family `mc_sw4_3d_reticle_dynamic_9820jqfj` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source-selected image/color/state inputs and exact world/viewmodel visibility, placement, and blend behavior.
- [ ] Convert family `mc_sw4_3d_scrolling_falloff_2ff9qe6w` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve scrolling coordinates, falloff/distortion inputs, source timing, coverage, and depth-dependent terms.
- [ ] Convert family `mc_sw4_3d_char_cloth_emissive_5ef7226z` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_phong_simple_emissive_flicker_9j5f25q3` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_zm_ice_script_static_877zz4j0` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve script-driven ice progression, opacity, emission, normals, and any blizzard state inputs.
- [ ] Convert family `mc_sw4_3d_zmb_sphere_tendril_j0598qze` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected effect's geometry/material controls, source animation/state, opacity, emission, and depth behavior.
- [ ] Convert family `mc_sw4_3d_char_cloth_reveal2_emissive_5644z720` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_clan_tag_fq631zzu` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source-selected image/color/state inputs and exact world/viewmodel visibility, placement, and blend behavior.
- [ ] Convert family `mc_sw4_3d_phong_emissive_fritz_76q5wqjw` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_phong_emissive_script_multi_72q2fj2u` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_player_icon_jef08712` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source-selected image/color/state inputs and exact world/viewmodel visibility, placement, and blend behavior.
- [ ] Convert family `mc_sw4_3d_scrolling_alpha2_distort_qwj8139f` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve scrolling coordinates, falloff/distortion inputs, source timing, coverage, and depth-dependent terms.
- [ ] Convert family `mc_sw4_3d_zm_ice_script_static_blizzard_45q67691` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve script-driven ice progression, opacity, emission, normals, and any blizzard state inputs.
- [ ] Convert family `mlv_sw4_3d_cod7_tile_blend_26zq1w22` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve authored tile coordinates, blend weights, and active normal/specular contributions using the committed bake machinery.
- [ ] Convert family `wpc_shadowcaster_wj6w5j60` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve this material's visibility and shadow-only role and bind it to the existing geometry/Solari path.
- [ ] Convert family `mc_objective_z4q1qe94` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source-selected image/color/state inputs and exact world/viewmodel visibility, placement, and blend behavior.
- [ ] Convert family `mc_projecteddecal_27zq5445` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve projected receiver selection, coordinates, blend behavior, and source activation through the existing conversion path.
- [ ] Convert family `mc_skycubemaphdr_1f117w7u` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native sky inputs, source HDR/color response, and atmosphere/sky ownership without adding scene lighting to albedo.
- [ ] Convert family `mc_sw4_3d_char_cloth_reveal2_emissive_f88qewf4` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_char_eye_cornea_2eww29wu` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native transparency, normal/specular response, view dependence, and scope/viewmodel ownership through the current Solari material path.
- [ ] Convert family `mc_sw4_3d_char_skin_3e77jj31` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_char_skin_emissive_189j7w84` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_emissive_tomb_qww536q2` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_glass_emissive_jf185j5e` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native transparency, normal/specular response, view dependence, and scope/viewmodel ownership through the current Solari material path.
- [ ] Convert family `mc_sw4_3d_phong_emissive_alcatraz_8q551153` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_phong_emissive_fritz_61728967` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_phong_emissive_script_multi_j05zz52w` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_reticle_dynamic_2colors_5f888f0w` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source-selected image/color/state inputs and exact world/viewmodel visibility, placement, and blend behavior.
- [ ] Convert family `mc_sw4_3d_reticle_static_35866wj0` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source-selected image/color/state inputs and exact world/viewmodel visibility, placement, and blend behavior.
- [ ] Convert family `mc_sw4_3d_reticle_ui3d_q49ew30e` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source-selected image/color/state inputs and exact world/viewmodel visibility, placement, and blend behavior.
- [ ] Convert family `mc_sw4_3d_scrolling_alpha2_distort_z58wq263` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve scrolling coordinates, falloff/distortion inputs, source timing, coverage, and depth-dependent terms.
- [ ] Convert family `mc_sw4_3d_sonar_attachment_lens2_78904485` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native transparency, normal/specular response, view dependence, and scope/viewmodel ownership through the current Solari material path.
- [ ] Convert family `mc_sw4_3d_water_on_surface_reveal_77320z68` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve water coordinates, normal/response composition, coverage, reveal, and source live controls through the existing material path.
- [ ] Convert family `mc_sw4_3d_zm_elec_disc_w323fq8j` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected effect's geometry/material controls, source animation/state, opacity, emission, and depth behavior.
- [ ] Convert family `mc_sw4_3d_zm_sphere_elec_535e0f02` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected effect's geometry/material controls, source animation/state, opacity, emission, and depth behavior.
- [ ] Convert family `mc_sw4_3d_zm_sphere_elec_9j108eq6` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected effect's geometry/material controls, source animation/state, opacity, emission, and depth behavior.
- [ ] Convert family `mc_sw4_3d_zmb_sky_pillar_on_6821fw2w` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected effect's geometry/material controls, source animation/state, opacity, emission, and depth behavior.
- [ ] Convert family `mc_sw4_3d_zmb_smoke_pillar_cheap_w6w7e738` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected effect's geometry/material controls, source animation/state, opacity, emission, and depth behavior.
- [ ] Convert family `mc_treecanopy_3z378q8f` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve canopy coverage, source vertex inputs, lighting/normal response, and required geometry motion.
- [ ] Convert family `mc_treecanopy_sm_68zw237j` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve canopy coverage, source vertex inputs, lighting/normal response, and required geometry motion.
- [ ] Convert family `mc_treecanopy_sm_j9ff996z` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve canopy coverage, source vertex inputs, lighting/normal response, and required geometry motion.
- [ ] Convert family `mlv_sw4_3d_phong_emissive_alcatraz_w1q1w09e` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `wpc_cod7water_we876z2f` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve water coordinates, normal/response composition, coverage, reveal, and source live controls through the existing material path.
- [ ] Convert family `wpc_default` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve this actual source material's selected program and state; preserve its real role without using it as a fallback for other families.
- [ ] Convert family `wpc_sw4_3d_burning_embers_nuketown_74j6971w` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.

## 1.6 Material pack integration and completion gate

- [ ] Route the existing material command through actual family conversion for every applicable family.
- [ ] Route the existing model command through actual converted materials rather than retaining a raw-native-channel bypass.
- [ ] Keep material/model checks inside the sole Rust builder and existing pack format.
- [ ] Ensure selected material packs contain all generated KTX2 channels and binary scene dependencies they reference.
- [ ] Ensure converted world scenes use baked material/mesh assignments for every new recipe.
- [ ] Ensure converted model LODs use the correct source-selected material assignments.
- [ ] Preserve native material distinctions across base, upgraded, camouflaged, damaged, and animated variants.
- [ ] Verify the existing conversion dispatch reports zero unimplemented required families.
- [ ] Verify actual conversion reports zero unresolved required materials.
- [ ] Verify all 1,034 starting geometry-used materials are converted or correctly represented by their source-defined non-visible role.
- [ ] Verify required material variants introduced by exact presentation/camo/FX dependencies also convert.
- [ ] Verify generated KTX2 contents and mip ranges through the existing texture validator.
- [ ] Verify all material texture, scene, frame, and metadata links resolve inside their packs.
- [ ] Verify source triangle counts and required geometry/skin bindings survive material baking.
- [ ] Run the existing focused mathematical and installed-source checks affected by converter changes.
- [ ] Review changed conversion code and remove replaced obsolete callers and bypass paths.
- [ ] Complete every exact-family checkbox above before starting later implementation phases.
- [ ] Record any non-material full-pack blocker separately without calling the final pack complete.
- [ ] Leave actual game runtime and Solari visual acceptance pending until the material phase is finished and a complete pack can load.

# Phase 2 - Finish brush trigger *164 and remaining trigger conversion

Primary files: tools/asset_builder/collision.rs, entities.rs, game.rs, scenes.rs, src/environment/brushes.rs, src/environment/triggers.rs, src/environment/support.rs, and existing interaction consumers.

## 2.1 Brush trigger *164

- [ ] Resolve the exact installed entity and collision submodel associated with brush trigger *164.
- [ ] Recover its source classname, script fields, contents, spawn flags, and activation consumer.
- [ ] Preserve its source origin, orientation, local geometry, and world transform.
- [ ] Preserve its native convex volumes and triangle shapes where present.
- [ ] Represent its actual contact/use/damage behavior using existing Avian geometry and ECS trigger components.
- [ ] Bind its source owner and source lifetime instead of leaving an unresolved brush classification.
- [ ] Connect the trigger to the exact gameplay consumer that produces its intended outcome.
- [ ] Include its required geometry and runtime definition inside the pack.
- [ ] Verify its conversion no longer produces an unresolved geometry, classification, ownership, or dependency failure.

## 2.2 Remaining native trigger geometry

- [ ] Complete required brush-backed trigger_multiple conversion.
- [ ] Complete required brush-backed trigger_use conversion.
- [ ] Complete required brush-backed trigger_damage conversion.
- [ ] Complete required trigger_lookat conversion.
- [ ] Complete remaining brush-backed trigger_box and info_volume conversion.
- [ ] Complete required non-brush trigger kinds currently reported by game::read.
- [ ] Preserve native radius/cylinder geometry without replacing it with a box approximation.
- [ ] Preserve rotated box geometry and the committed convex representation.
- [ ] Carry required terrain/triangle trigger geometry through the existing collision output.
- [ ] Preserve source contents and surface flags on triangle and convex geometry.
- [ ] Preserve body/brush ownership so dynamic brushes are not duplicated in static collision.
- [ ] Resolve source-selected mud volumes and their movement consumer.
- [ ] Resolve source player boundary volumes and their zone/boundary consumer.
- [ ] Preserve overlap results for multi-volume triggers without double-applying a single source activation.
- [ ] Preserve exact scene references for rendered brush geometry.
- [ ] Reject genuinely missing or ambiguous source geometry with the specific entity/submodel named in the error.

## 2.3 Contact, ownership, and script-created lifetime

- [ ] Implement source contact category and contents-mask gates for each required trigger consumer.
- [ ] Preserve native numeric client-mode predicates where they participate in eligibility.
- [ ] Preserve the trigger as recipient and the contacting entity as activator.
- [ ] Preserve native repeat behavior without adding an unsupported enter-only or alive-only filter.
- [ ] Preserve source wait, single-use, and deletion behavior only where the source actually applies it.
- [ ] Preserve queued-contact validity across entity deletion and recreation.
- [ ] Implement required deferred event ordering through existing ECS scheduling and messages.
- [ ] Ensure a deleted single-use trigger cannot activate again through stale queued contacts.
- [ ] Complete script-created trigger spawn, enable, disable, replacement, and destruction as Rust ECS behavior.
- [ ] Complete trigger owner cleanup when the parent gameplay object is removed.
- [ ] Bind shootable trigger consumers to the correct damage category and source predicate.
- [ ] Bind use/look consumers to the correct player ownership and input predicate.
- [ ] Reset trigger state and queued activation state between sessions.
- [ ] Remove resolved trigger gap entries only after their runtime consumers are implemented.
- [ ] Run focused source/geometry checks for changed trigger conversion.
- [ ] Defer game behavior assertions to the final runtime phase.

# Phase 3 - Complete native mode, zone, access, and crafting lifecycle

Primary files: tools/asset_builder/game.rs, src/access.rs, src/player.rs, src/rounds.rs, src/barriers.rs, src/crafting.rs, src/environment.rs, and existing gameplay messages.

## 3.1 Native client mode and zone contact

- [ ] Recover the source transitions relevant to TriggerClientMode for the supported player states.
- [ ] Update TriggerClientMode from actual player/session lifecycle transitions.
- [ ] Preserve exact source mode exclusions used by zone trigger activation.
- [ ] Connect converted player volumes to zone membership.
- [ ] Preserve source handling for overlapping player volumes.
- [ ] Preserve entry, exit, death/downed, revival, and respawn behavior where the source distinguishes them.
- [ ] Deliver the required source zone events through existing ECS messages.
- [ ] Preserve source single-use activation flag updates and sensor deletion.
- [ ] Clear zone contact/membership state when the player or session is removed.
- [ ] Complete the existing source activation bindings without replacing the committed zone graph.

## 3.2 Zone graph, flag producers, doors, and barriers

- [ ] Complete every required source flag producer referenced by converted zone connections.
- [ ] Preserve initial zone availability and always-on connections.
- [ ] Preserve the source relationship between flag changes and connection availability.
- [ ] Preserve multiple source flags that affect the same connection.
- [ ] Convert access.doors with exact connection bindings and source purchase/removal behavior.
- [ ] Convert door/debris visual props, collision blocks, timing, and sound bindings.
- [ ] Preserve transactions and progression when opening a connection unlocks more than one source path.
- [ ] Convert access.volumes using the native zone and trigger geometry already exported.
- [ ] Convert access.spawns with source positions, zones, and rising/traversal selection.
- [ ] Connect opened zones to the source spawn enablement rules.
- [ ] Disable spawns whose source access predicates are unsatisfied.
- [ ] Connect source barrier activation to the relevant zone/connection lifecycle.
- [ ] Preserve barrier board state, repair interaction, and zombie traversal ownership.
- [ ] Connect navigation access changes to door/debris state.
- [ ] Preserve required generator, robot, tank, and quest flag producers used by access.
- [ ] Reset flags, doors, connections, barriers, and active spawns between sessions.
- [ ] Remove the unconditional pending native zone conversion entry only after the underlying behaviors are complete.

## 3.3 Crafting benches, unitriggers, and gramophone

- [ ] Preserve the recently committed exact crafting bench bindings.
- [ ] Recover and implement native per-player unitrigger candidate selection.
- [ ] Preserve source look/shape/use predicates and eligibility.
- [ ] Preserve per-player prompt, ownership, and interaction persistence.
- [ ] Preserve candidate replacement and cleanup when players or crafting objects leave scope.
- [ ] Bind required source inventory/part predicates to bench activation.
- [ ] Preserve crafting completion timing and source state changes.
- [ ] Preserve bench model/state transitions through existing presentation bindings.
- [ ] Complete gramophone recipe/purchase/placement consumers.
- [ ] Preserve gramophone pickup, placement, ownership, sound, and cleanup lifecycle.
- [ ] Connect completed crafting outcomes to their source quest/access consumers.
- [ ] Remove resolved crafting and gramophone gap entries after the full consumer path is implemented.

# Phase 4 - Complete native animation conversion and playback

Primary files: tools/asset_builder/animation_decode.rs, animation.rs, gltf.rs, scenes.rs, source.rs, src/animation.rs, src/presentation.rs, and existing actor/weapon animation consumers.

## 4.1 Finish native stream decoding

- [ ] Preserve the current ordinary rotation and translation stream decoder.
- [ ] Preserve wide-index handling for clips with 256 or more frames.
- [ ] Preserve source frame rate, frame count, key indices, and stream bounds.
- [ ] Decode separate native root-translation delta tracks.
- [ ] Decode separate native root-rotation delta tracks.
- [ ] Preserve source delta interpolation and coordinate conversion.
- [ ] Resolve the near-zero quaternion fallback cases currently rejected by rotation_samples.
- [ ] Preserve the committed unequal-norm component-linear quaternion interpolation.
- [ ] Preserve source bone-name matching and explicitly selected destination skeletons.
- [ ] Preserve constant/default local tracks and source bind/rest transforms.
- [ ] Preserve missing-channel semantics from the selected source pose path.
- [ ] Decode and retain all required source notifies and their timing.

## 4.2 Source flags, looping, discontinuities, and pose composition

- [ ] Interpret bLoop using the selected native animation consumer.
- [ ] Interpret bDelta and bDelta3D using the selected source pose/movement path.
- [ ] Interpret bLeftHandGripIK where required by the equipped weapon/attachment.
- [ ] Preserve authored loop entry time and loop boundary behavior.
- [ ] Implement source-faithful jumpcut interval sampling.
- [ ] Preserve source animation start/restart behavior.
- [ ] Select standalone, delta, additive, and blended playback only where the source requires each path.
- [ ] Preserve the order of animation-tree contributions used by required gameplay presentations.
- [ ] Preserve root motion ownership and avoid applying the same movement through both animation and locomotion.
- [ ] Preserve source attachment/socket transforms and parent/child pose composition.
- [ ] Preserve left/right weapon and hand pose ownership for dual-wield presentations.
- [ ] Preserve source animation notify ordering across loop, restart, and interruption.
- [ ] Remove unsupported-flag and jumpcut export errors only when their selected behavior is implemented.

## 4.3 Export and bind animations to packed scenes

- [ ] Select exact animation sets for each required model/presentation.
- [ ] Export required clips into the existing packed model scenes.
- [ ] Replace empty model animation bindings with actual exported clip mappings.
- [ ] Bind gameplay animation IDs to exact exported skeletons and clip indices.
- [ ] Preserve model LOD, skin, and attachment relationships needed by animated presentations.
- [ ] Preserve skeleton-local naming and case-insensitive matching already used by the converter.
- [ ] Keep irrelevant source bones out of a selected destination skeleton without dropping required channels.
- [ ] Export animation data and source-derived runtime control data inside packs only.
- [ ] Resolve all animation, skeleton, model, and presentation links before runtime assembly.
- [ ] Ensure an absent required clip fails conversion explicitly.

## 4.4 Integrate actual gameplay playback

- [ ] Bind weapon raise/lower, idle, fire, reload, melee, sprint, and ADS-related actions to the correct native clips.
- [ ] Bind shell reload start/loop/end and interruption behavior to source animation timing.
- [ ] Bind dual-wield actions independently to the correct hand/weapon owners.
- [ ] Bind staff charge, attack, upgrade, and state-specific animation actions where present.
- [ ] Bind zombie locomotion, attacks, hit reactions, death, rise, and traversal animations.
- [ ] Bind special enemy animations and their gameplay transitions.
- [ ] Bind required player movement/stance/interaction animations.
- [ ] Bind robot, tank, generator, crafting, and environmental animations used by Origins.
- [ ] Connect source sound/FX/gameplay notifies to the existing animation event consumers.
- [ ] Preserve animation interruption and state replacement across downed, revive, weapon replacement, and session reset.
- [ ] Run focused decoder/interpolation/export checks affected by changes.
- [ ] Leave actual playback acceptance pending for the current executable in phase 6.

# Phase 5 - Complete remaining game data and assets

Primary files: tools/asset_builder/game.rs, source.rs, scenes.rs, sound.rs, convert.rs, native.rs, src/origins/content.rs, src/assets.rs, and existing gameplay consumers.

Material-required inputs already belong to phase 1. This phase completes the remaining gameplay definitions, assets, placements, and runtime pack bindings without changing original mechanics or balance.

## 5.1 Complete game.json assembly

- [ ] Convert the exact player spawn position and initial yaw.
- [ ] Convert required spawn_points.
- [ ] Convert required waypoints and navigation bindings.
- [ ] Populate source-derived survey_views used by existing validation/capture tools.
- [ ] Complete rounds.gait_speeds.
- [ ] Complete solo_revive.downed_height.
- [ ] Complete solo_revive.downed_speed.
- [ ] Complete environment.digging and environment.parts.
- [ ] Complete access.doors, access.volumes, and access.spawns.
- [ ] Complete audio aliases, bindings, timelines, emitters, surface mappings, and character selection.
- [ ] Complete required tuning values from original source/content.
- [ ] Preserve the already converted player health, round, solo-revive, localization, trigger, crafting, and zone values.
- [ ] Replace unresolved generated fields only with recovered source-derived data.
- [ ] Remove duplicate obsolete embedded-data callers when the packed replacements are complete.
- [ ] Retain explicit errors for missing required source data.
- [ ] Ensure game.json deserializes into the actual runtime schema and satisfies existing content validation.

## 5.2 Weapons, attachments, equipment, and presentations

- [ ] Convert required base and upgraded weapon definitions.
- [ ] Convert damage, range falloff, hit multipliers, penetration, pellet count, and projectile delivery.
- [ ] Convert magazine, reserve, starting ammunition, shared ammunition groups, and refill rules.
- [ ] Convert fire patterns, intervals, burst behavior, recoil, recovery, and ADS-specific values.
- [ ] Convert reload durations, per-shell behavior, interruption rules, and animation action bindings.
- [ ] Convert melee timing, damage, and required presentation bindings.
- [ ] Convert attachment selection, alternate modes, hide tags, offsets, rotations, and socket ownership.
- [ ] Convert ADS FOV, zoom fractions, sight definitions, and scoped presentation selection.
- [ ] Convert native viewmodel, worldmodel, pickup, and upgraded model bindings.
- [ ] Convert dual-weapon relationships and independent hand presentations.
- [ ] Convert grenade and Semtex fuse, throw, collision, damage, and inventory definitions.
- [ ] Convert claymore placement, trigger, damage, inventory, and cleanup definitions.
- [ ] Convert monkey/lure activation, duration, targeting, and inventory definitions.
- [ ] Convert staff basic, charged, upgraded, melee, and special-action definitions.
- [ ] Bind material/camo selections completed in phase 1 to the correct gameplay weapon and attachment state.
- [ ] Convert weapon names, localization references, HUD definitions, reticles, and low-ammo behavior.
- [ ] Resolve every weapon, attachment, pickup, animation, sound, material, and FX dependency.

## 5.3 Enemies, rounds, movement, navigation, and barriers

- [ ] Convert ordinary enemy definitions, model/animation selection, health, damage, and movement data.
- [ ] Convert special enemy definitions and source behavior parameters.
- [ ] Convert source gait selection, speed bands, and traversal selection.
- [ ] Convert navigation_nodes and navigation_links with source connectivity and traversal flags.
- [ ] Preserve source collision and navigation access changes caused by doors and debris.
- [ ] Convert exact ordinary and special enemy spawn placements and source zone eligibility.
- [ ] Preserve source round population, actor caps, spawn timing, inter-round timing, and health progression.
- [ ] Preserve source solo-revive timing, limits, downed weapon selection, and recovery behavior.
- [ ] Convert source player movement, stance, mud, sprint, dive, and downed tuning required by runtime.
- [ ] Convert hit-region definitions and source damage-category behavior.
- [ ] Convert barriers, board presentations, repair interactions, and attack/traversal bindings.
- [ ] Resolve every enemy, traversal, navigation, barrier, and presentation dependency.

## 5.4 Perks, powerups, world systems, and progression

- [ ] Convert perk definitions, prices, limits, machine placements, and source activation effects.
- [ ] Convert perk purchase, loss, revival, and replacement state dependencies.
- [ ] Convert powerup definitions, source eligibility, selection, placement, duration, and pickup behavior.
- [ ] Preserve source powerup effects and interactions with ammunition, rounds, points, and enemies.
- [ ] Convert generator definitions, placements, activation costs/timing, capture, defense, and power dependencies.
- [ ] Convert generator raid enemy/route/animation and recovery dependencies.
- [ ] Convert robot_routes with source movement, foot hazards, interior access, and required presentations.
- [ ] Convert tank_routes with source movement, stops, purchase/access rules, passenger behavior, and hazards.
- [ ] Convert weather definitions, source selectors, transitions, sound, and gameplay dependencies.
- [ ] Convert digging definitions, placements, selection probabilities, rewards, cooldowns, and progression.
- [ ] Convert part definitions, source-selected spawn placements, pickup conditions, and inventory ownership.
- [ ] Convert recipes, crafting prerequisites, completion outcomes, and bench associations.
- [ ] Convert staff_puzzles with source element/state predicates and required world bindings.
- [ ] Convert quests and quest_transitions with exact prerequisites, messages, progression, and reset behavior.
- [ ] Convert required tablets, rituals, portals, soul collection, and upgrade dependencies into existing content/quest consumers.
- [ ] Convert required buildable, shield, drone, gramophone, and access dependencies.
- [ ] Convert definitions, placements, and interactions with exact cross-catalog IDs.
- [ ] Preserve original prices, timings, balance, probabilities, and progression conditions.
- [ ] Resolve all required quest, item, zone, generator, route, sound, animation, and FX references.

## 5.5 Models, world geometry, and scene integration

- [ ] Complete exact model selection for every gameplay presentation.
- [ ] Preserve source model LOD geometry and authored distance selection.
- [ ] Preserve source skeletons, skin weights, bind transforms, and required sockets.
- [ ] Preserve source static-model transforms and placements.
- [ ] Preserve native world brush ownership and rendered brush placements.
- [ ] Preserve static collision, dynamic brush collision, terrain shapes, contents, and surface flags.
- [ ] Preserve the committed source-derived atmosphere bindings in the packed Origins world.
- [ ] Resolve all model/world buffers, textures, collision scenes, animation clips, and presentation URIs.
- [ ] Keep generated scene and binary geometry data inside the final runtime packs.

## 5.6 Native FX and remaining camo asset data

- [ ] Implement required native FX decoding/conversion in existing Rust asset-builder modules.
- [ ] Resolve required effect definitions from actual gameplay/presentation references.
- [ ] Convert source effect element selection, lifetime, delays, looping, and spawn timing.
- [ ] Convert source effect spatial offsets, orientation, attachment, velocity, and size behavior.
- [ ] Convert source effect color/alpha curves and material/image selections.
- [ ] Convert required effect model, sound, light, and child-effect references.
- [ ] Bind required source effect activation and stop/reset events to existing ECS consumers.
- [ ] Reuse material conversion completed in phase 1 for all FX materials.
- [ ] Populate prepared/effects.nativefx with actual converted runtime definitions.
- [ ] Populate prepared/camo.json with actual required native camo definitions and selections.
- [ ] Preserve material/pattern selection and override behavior implemented in phase 1.
- [ ] Resolve all nested FX and camo dependencies inside packs.

## 5.7 Audio, localization, UI, fonts, and cinematics

- [ ] Complete required sound payload conversion through the existing sound-bank reader.
- [ ] Preserve the recently retained source sound selection fields.
- [ ] Preserve source alias variants, selection weights/predicates, pitch, gain, distance, limits, and priorities where required.
- [ ] Preserve source channel routing, spatial behavior, secondary aliases, and reverb/wet-distance data.
- [ ] Bind animation sound notifies and gameplay audio messages to exact source aliases.
- [ ] Convert music, ambient emitters, weather audio, dialogue, quest lines, and subtitle dependencies.
- [ ] Resolve all required localized string references through the converted localization catalog.
- [ ] Convert exact HUD/menu/UI image bindings and their source alpha/color behavior.
- [ ] Convert source UI atlas coordinates where native assets use atlases.
- [ ] Convert required font data, glyph images, metrics, and localization coverage.
- [ ] Preserve source HUD placement, sizing, state selection, interaction prompts, and weapon/ammo presentation.
- [ ] Preserve required staff, perk, powerup, part, quest, and inventory indicators.
- [ ] Bind native UI/font entries to content.ui_assets and prepared/runtime.json.
- [ ] Convert required cinematic selection, media/timeline data, sound, and presentation bindings.
- [ ] Resolve required cinematic playback and completion/reset events in the existing game flow.
- [ ] Keep all generated audio, font, UI, and cinematic data inside pack files.
- [ ] Fail explicitly on missing required source payloads rather than borrowing another installation or cache.

## 5.8 Required catalogs and runtime assembly gate

- [ ] Populate content.barriers with actual converted definitions.
- [ ] Populate content.weapons with actual converted definitions.
- [ ] Populate content.enemies with actual converted definitions.
- [ ] Populate content.perks with actual converted definitions.
- [ ] Populate content.powerups with actual converted definitions.
- [ ] Preserve and complete content.zones and content.zone_connections.
- [ ] Populate content.definitions, content.placements, and content.interactions.
- [ ] Populate content.navigation_nodes and content.navigation_links.
- [ ] Populate content.generators, content.robot_routes, and content.tank_routes.
- [ ] Populate content.weather and content.digging.
- [ ] Populate content.recipes and content.staff_puzzles.
- [ ] Populate content.quests and content.quest_transitions.
- [ ] Populate content.presentations, content.sounds, and content.animations.
- [ ] Populate content.ui_assets and content.cinematics.
- [ ] Retain converted localized_strings and resolve every required localization reference.
- [ ] Assemble actual presentation-to-model and animation-to-skeleton bindings in prepared/runtime.json.
- [ ] Assemble actual UI-to-image/font bindings in prepared/runtime.json.
- [ ] Resolve required prepared/effects.nativefx and prepared/camo.json entries.
- [ ] Ensure every source conversion gap reported by game::write is resolved by actual implementation.
- [ ] Ensure every source conversion gap reported by source::runtime is resolved by actual implementation.
- [ ] Assemble complete game.json and prepared/runtime.json inside origins.pack.
- [ ] Run existing schema/content validation and pack dependency validation on the assembled data.

# Phase 6 - Final pack, gameplay, rendering, and release validation

Primary files/tools: tools/build_assets.rs, src/pack.rs, src/assets.rs, src/verification/remote.rs, tools/test_support.py, existing gameplay scenario scripts, and existing rendering validation tools.

Compilation, pack integrity, actual gameplay, visual inspection, and measured performance are separate completion gates.

## 6.1 Build and inspect the complete runtime pack

- [ ] Build the asset builder only when its changed compiled inputs require rebuilding.
- [ ] Generate origins.pack directly from the authorized Steam installation using the sole Rust builder.
- [ ] Confirm a complete pack is written only after all required conversion and dependency checks pass.
- [ ] Run the existing build_assets check command on the generated complete pack.
- [ ] Confirm required game.json and prepared/runtime.json entries load.
- [ ] Confirm required model, world, collision, animation, image, sound, UI/font, FX, camo, and cinematic entries load.
- [ ] Confirm every referenced scene buffer, texture, frame, model, animation, and runtime-data dependency resolves.
- [ ] Confirm packed texture dimensions, faces, formats, level ranges, and mip payloads validate.
- [ ] Confirm no required runtime dependency resolves to a loose generated file or extraction cache.
- [ ] Confirm the source installation and existing reference material remain preserved.
- [ ] Confirm the final pack and generated assets remain local and untracked.

## 6.2 Build the current game

- [ ] Build the changed game using cargo build --features dev-tools --locked with static Bevy linking.
- [ ] Address compiler failures caused by the completed implementation.
- [ ] Run focused existing checks required by changed gameplay/pack integration.
- [ ] Reuse the current target/debug/origins.exe for repeated scenarios until compiled inputs change.
- [ ] Ensure the executable under validation contains the changed code, embedded data, dependencies, features, and build configuration.
- [ ] Place/use the complete compatible origins.pack through the existing loader path.
- [ ] Confirm missing/corrupt/incompatible pack failures identify the actual failing dependency.

## 6.3 Runtime scenario discipline

- [ ] Check running game processes across all checkouts/worktrees immediately before every launch.
- [ ] Check overlapping GPU capture/profiling activity immediately before every launch.
- [ ] Wait and recheck while another game or capture owns the slot; continue independent work where possible.
- [ ] Launch only the instance owned by the current run.
- [ ] Enable the existing development-only loopback BRP interface explicitly for scenarios.
- [ ] Discover supported BRP methods and reflected schemas from the locked Bevy source and active interface.
- [ ] Use bounded startup, readiness, state, and outcome waits in existing Python/PowerShell scenario tools.
- [ ] Drive gameplay through normal inputs and messages.
- [ ] Treat direct ECS edits as setup rather than evidence of a gameplay outcome.
- [ ] Use a reusable scheduled-input hook for fixed-tick timing checks.
- [ ] Release held inputs and reset state between cases.
- [ ] Call client.shutdown(report) in finally before writing each scenario's final report.
- [ ] Close only the game instance owned by that run.
- [ ] Keep external sequencing, waits, assertions, and reports out of new Rust validation infrastructure.

## 6.4 Actual gameplay acceptance

- [ ] Validate player spawn, movement, stance, sprint, dive, mud, and boundary behavior.
- [ ] Validate brush trigger *164 through its actual gameplay consumer.
- [ ] Validate radius, rotated box, brush, use, damage, look, and required script-created trigger behavior.
- [ ] Validate native client-mode eligibility, trigger activation, repeat behavior, and deletion/reset.
- [ ] Validate initial zones, zone transitions, source flag producers, doors, debris, and connections.
- [ ] Validate spawn enablement, navigation access, barriers, and traversal after access changes.
- [ ] Validate round population, spawn timing, health progression, inter-round behavior, and source caps.
- [ ] Validate ordinary and special enemy locomotion, attack, hit, death, and traversal outcomes.
- [ ] Validate player health, damage immunity, downed behavior, solo revive, and bleedout behavior.
- [ ] Validate weapon purchase, pickup, replacement, upgrading, and inventory state.
- [ ] Validate fire patterns, ammunition consumption, reload, shell reload, melee, ADS, alternate modes, and attachments.
- [ ] Validate dual-wield input, ammunition, animation, and hand ownership.
- [ ] Validate grenade, Semtex, claymore, monkey/lure, and refill behavior.
- [ ] Validate perks and powerups through actual purchase/pickup and their gameplay effects.
- [ ] Validate generator activation, defense, loss, and recovery.
- [ ] Validate robot/tank movement, interaction, access, passenger behavior, and hazards.
- [ ] Validate digging, source reward selection, part pickup, and inventory ownership.
- [ ] Validate crafting bench unitrigger selection, per-player persistence, and crafting completion.
- [ ] Validate gramophone purchase/pickup/placement and portal/access effects.
- [ ] Validate staff acquisition, puzzles, charging, upgrading, and required special actions.
- [ ] Validate required quest transitions, rituals, tablets, soul collection, and finale dependencies.
- [ ] Validate gameplay-triggered animations and source-timed sound/FX notifies.
- [ ] Validate pause/restart/session cleanup for animation, triggers, material state, audio, and gameplay objects.
- [ ] Validate sound selection, spatial playback, music, dialogue, and subtitles.
- [ ] Validate HUD, prompts, inventory, ammo, perk, powerup, staff, and quest state through actual gameplay outcomes.
- [ ] Report scenario passes and failures separately from compile and pack-check results.

## 6.5 Solari material and render acceptance

- [ ] Launch the current executable with the complete newly generated pack after the material completion gate.
- [ ] Confirm the actual scene uses the existing intended Solari/PBR path.
- [ ] Confirm each of the 165 completed families reaches its intended visible or source-defined non-visible role.
- [ ] Inspect representative world/model surfaces for every distinct implemented material operation.
- [ ] Inspect layered transitions, reveal curves, decals, vertex tint, base-alpha gloss, and specular/roughness response.
- [ ] Inspect atlas boundaries, mip behavior, source texel density, normal orientation, and mixed tangent-handedness cases.
- [ ] Inspect all required source-selected camo and upgraded weapon material variants.
- [ ] Inspect texture/constant override scope across weapon, attachment, hand, and model surfaces.
- [ ] Inspect scrolling, scalar-driven, script-driven, and animated material states over time.
- [ ] Inspect staff charge, frozen stone, ice, electrical, portal, flicker, and emissive state transitions.
- [ ] Inspect character skin, cloth, hair, damage/reveal, eye, and ember/emissive families.
- [ ] Inspect flags/canopy/required moving geometry with their intended materials and Solari scene updates.
- [ ] Inspect optics, reticles, scopes, viewmodel transparency, and correct owner/state visibility.
- [ ] Inspect glass, water, cornea, and other transparent families in representative light and view conditions.
- [ ] Inspect additive, multiply, projected, receiver-dependent, unlit, and fog-sensitive material behavior.
- [ ] Inspect source sky/HDR response, fog, weather, atmosphere, lighting, and color grading.
- [ ] Inspect dynamic actors, weapons, attachments, and animated material updates in the Solari scene.
- [ ] Inspect source shadow-only and intentionally non-visible material roles.
- [ ] Inspect actor/weapon/environment animation playback, root motion, loops, jumpcuts, grip/attachment poses, and interruption behavior.
- [ ] Inspect native LOD transitions for placement, material, and skinning continuity.
- [ ] Inspect FX, particles, lights, audio synchronization, UI/font rendering, and cinematic presentation.
- [ ] Capture bounded actual-window examples using existing validation/capture tools.
- [ ] Fix observed failures in the existing builder/runtime path and repeat only affected checks.
- [ ] Keep every visually unverified or approximated source behavior explicitly incomplete.
- [ ] Report visual acceptance separately from family dispatch, compiler success, and pack dependency closure.

## 6.6 Release performance and local distribution

- [ ] Build a release executable without dev-tools after relevant compiled inputs are final.
- [ ] Confirm release builds do not expose BRP.
- [ ] Use release builds for any performance comparison.
- [ ] Use comparable scenes, camera paths, settings, resolution, and system load for performance measurements.
- [ ] Check for competing game instances, GPU captures, and other material system load before measurements.
- [ ] Report measured frame time/FPS, settings, hardware, system load, and concrete limitations through existing procedures.
- [ ] Do not infer a performance improvement from compilation, source review, or successful pack conversion.
- [ ] Keep the distributable code/builder separate from locally generated proprietary runtime pack contents.
- [ ] Verify the provided builder can generate the required pack from the user's authorized local installation.
- [ ] Confirm no original/converted assets, packs, generated builds, secrets, or machine-specific paths enter the requested code commit.
- [ ] Review only the requested implementation scope before any authorized local commits.
- [ ] Push or publish only when explicitly requested.

## Final completion gates

- [ ] All 165 remaining material families have actual converters, packed outputs, and intended runtime/Solari bindings.
- [ ] All material-dependent selectors, live inputs, animations, and source states are integrated.
- [ ] Brush trigger *164 and required trigger geometry/ownership/lifetime gaps are complete.
- [ ] Native mode, zone, access, spawn, barrier, crafting, and gramophone lifecycle gaps are complete.
- [ ] Required native animation data, flags, pose composition, export bindings, and gameplay playback are complete.
- [ ] Required gameplay catalogs, source values, world/model data, FX, camo, audio, UI/fonts, and cinematics are complete.
- [ ] The complete generated runtime pack loads and all required dependencies resolve.
- [ ] The current game executable passes the required actual gameplay scenarios.
- [ ] Actual-window inspection validates the completed material/render behavior.
- [ ] Any claimed performance change has release-build measurements.
- [ ] The final report distinguishes implemented work, checks actually run, visual proof, measured performance, approximations, and unresolved gaps.
