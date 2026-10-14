# Origins completion and final fidelity backlog

[todo.md](todo.md) owns the first playable solo Origins milestone. This file owns the remaining full-map gameplay, complete Origins progression, final asset/render fidelity, release work, and nice-to-haves. The two files replace the former single six-phase checklist at the user's request.

Nothing here blocks the initial milestone unless it is an actual dependency of enabled initial content. Promote such a dependency into todo.md explicitly. Reuse completed initial work; tasks below cover the remaining full-game scope rather than requiring the initial implementation to be repeated.

## Delivery order and scope

1. Finish todo.md and demonstrate the current game opening with a newly generated compatible pack and a working early-round solo survival loop.
2. Expand to full-map survival: doors/debris, connected navigation and spawns, wall weapons/Mystery Box, perks, powerups, generators, and special enemies.
3. Complete Origins progression: digging and rewards, parts/buildables, gramophone and portals, staffs/upgrades, robot and tank systems, quests, side rewards, and finale.
4. Complete remaining animation, audio, UI/localization, FX, material/render fidelity, cinematics, and release acceptance where they are not prerequisites of the earlier deliveries.

Full Origins gameplay is deferred work, not an optional replacement for original mechanics. Presentation polish, complete cosmetic coverage, cinematics, and performance comparisons are later work and nice-to-haves for the initial playable delivery. Keep required gameplay timings, prices, probabilities, and balance source-derived throughout.

The family/material totals below describe the prior broad loaded-catalog inventory. First distinguish actual full-Origins dependencies from unused shared/test content. Retain the detailed backlog until usage is resolved; a family name alone cannot prove that content belongs to another map. Complete every family genuinely required by the final Origins scope.

Continue through the sole Rust builder and existing ECS/runtime paths under AGENTS.md. Historical material-first ordering, the ban on early game testing, and the old three-subagent instruction no longer apply. There is no new authorization for subagents. Batch focused material checks where practical, share decoded inputs within a run, and keep compilation, pack closure, gameplay, visuals, and performance as separate evidence.

## Existing implementation and reported checkpoints

The current checkout already contains the Rust pack builder and runtime pack integration, ordinary PBR material conversion, layered world baking, mixed tangent-handedness handling, base-alpha response and reveal baking, native static-model LODs, brush collision bindings, ordinary animation decoding, primitive trigger conversion, crafting bench bindings, and source zone graph/flag activation bindings.

Relevant recent commits: 6f691a14c, 10ffa5a4d, ffc8d1961, a3607cfa1, cfb1b4abb, 8bd71a794, 0a67833bb, aa5665325, 88744c4a5, d07247bdb, and b13e79ab6. These describe the baseline to extend, not work to redo.

The post-cutoff geometry-used material inventory contains 180 exact technique families and 1,034 materials. Starting dispatch covered 15 families. The extensions now dispatch one hundred seven additional families covering 461 geometry-used materials. The first twelve families passed all 72 installed-material conversions; the latest fifteen recipes cover 45 more materials. Combined installed-source preflight passed for those 45 and the previous 38 ordinary materials: exact programs, active bindings, routes, states and 112 image definitions checked for 83 materials in twenty families. Seventeen representative materials converted from 25 native images; nine identical image encodings were shared in memory. Exhaustive conversion of the new families remains pending. A further twenty recipes cover fifty-nine materials: exact selected lit/sun programs, active bindings, routes, states and image definitions passed metadata checks for all fifty-eight layered materials in nineteen families and one ordinary glass material. Twenty representative source triangles were baked/exported from twenty layered materials, and the ordinary glass material converted using the existing PBR fit, sharing fifty-seven decoded native images in one combined run. The representative layered materials' 11,823 source vertices and 3,941 source triangles passed auxiliary-input checks, including sixteen-byte two-normal and UV3 layouts. CPU/GPU agreement passed for thirty-three recipe modes and eight samplers, with maximum absolute channel error 0.00002885. Full-family conversion remains pending. The four additive-unlit materials still require attenuation-only fog, and the newly converted opaque layer-selection and glass materials have explicit framebuffer-composition runtime gaps that continue to fail pack closure. Five additional ordinary recipes cover ten materials. Their source program/binding/state checks passed for all ten, in the combined 93-material preflight; five new representatives and one explicit camo state converted alongside the previous representatives. Raw camo inputs remain packed and the five families retain explicit runtime gates for native model color transforms, per-owner camo state, tread time inputs and distinct indirect-lighting terms. Finite-resolution camo filtering and the existing PBR fit remain approximations. Six additional recipes cover 101 materials. All 101 passed selected-program, binding, route, constant and state checks within the combined 194-material preflight; 319 active image definitions resolve. Thirty-three representative material/state conversions passed from 68 native images in one combined run, reusing 33 identical linear/sRGB encodings. The new recipes include both authored wood detail scales, the sun-only specular multiplier edge and two explicit blue-biased camo states. Source slope-addition and camo mask/gloss checks passed; native occlusion lighting, masked framebuffer alpha and catwalk sqrt-coverage composition retain runtime gates. Full-pack and runtime acceptance remain pending. The material backlog below explicitly lists all 165 remaining families and their 664 currently used materials. Counts describe the starting backlog; an allow-list entry alone does not complete a family.

## 1. Complete remaining map access and crafting lifecycle

Primary files: tools/asset_builder/game.rs, src/access.rs, src/player.rs, src/rounds.rs, src/barriers.rs, src/crafting.rs, src/environment.rs, and existing gameplay messages.

### 1.1 Zone graph, flag producers, doors, and barriers

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

### 1.2 Crafting benches, unitriggers, and gramophone

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

## 2. Complete animation coverage beyond the initial playable content

Primary files: tools/asset_builder/animation_decode.rs, animation.rs, gltf.rs, scenes.rs, source.rs, src/animation.rs, src/presentation.rs, and existing actor/weapon animation consumers.

### 2.1 Finish native stream decoding

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

### 2.2 Source flags, looping, discontinuities, and pose composition

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

### 2.3 Export and bind animations to packed scenes

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

### 2.4 Integrate actual gameplay playback

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
- [ ] Leave actual playback acceptance pending for the current executable in the final acceptance section.

## 3. Complete remaining Origins game data and assets

Primary files: tools/asset_builder/game.rs, source.rs, scenes.rs, sound.rs, convert.rs, native.rs, src/origins/content.rs, src/assets.rs, and existing gameplay consumers.

Initial spawn, movement, ordinary-enemy, round, solo-revive, hit-region, and starting-barrier tasks are owned by todo.md. Complete the remaining full-map definitions, assets, placements, and runtime pack bindings here. Weapon/equipment tasks below cover the arsenal beyond the initial pistol, knife, and grenades; reuse the initial converters. Material-required inputs are tracked in section 4. Preserve original mechanics and balance.

### 3.1 Complete game.json assembly

- [ ] Convert required spawn_points.
- [ ] Convert required waypoints and navigation bindings.
- [ ] Populate source-derived survey_views used by existing validation/capture tools.
- [ ] Complete environment.digging and environment.parts.
- [ ] Complete access.doors, access.volumes, and access.spawns.
- [ ] Complete audio aliases, bindings, timelines, emitters, surface mappings, and character selection.
- [ ] Complete required tuning values from original source/content.
- [ ] Preserve the already converted player health, round, solo-revive, localization, trigger, crafting, and zone values.
- [ ] Replace unresolved generated fields only with recovered source-derived data.
- [ ] Remove duplicate obsolete embedded-data callers when the packed replacements are complete.
- [ ] Retain explicit errors for missing required source data.
- [ ] Ensure game.json deserializes into the actual runtime schema and satisfies existing content validation.

### 3.2 Weapons, attachments, equipment, and presentations

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
- [ ] Bind material/camo selections completed in section 4 to the correct gameplay weapon and attachment state.
- [ ] Convert weapon names, localization references, HUD definitions, reticles, and low-ammo behavior.
- [ ] Resolve every weapon, attachment, pickup, animation, sound, material, and FX dependency.

### 3.3 Enemies, rounds, movement, navigation, and barriers

- [ ] Convert special enemy definitions and source behavior parameters.
- [ ] Convert source gait selection, speed bands, and traversal selection.
- [ ] Convert navigation_nodes and navigation_links with source connectivity and traversal flags.
- [ ] Preserve source collision and navigation access changes caused by doors and debris.
- [ ] Convert exact ordinary and special enemy spawn placements and source zone eligibility.
- [ ] Resolve every enemy, traversal, navigation, barrier, and presentation dependency.

### 3.4 Perks, powerups, world systems, and progression

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

### 3.5 Models, world geometry, and scene integration

- [ ] Complete exact model selection for every gameplay presentation.
- [ ] Preserve source model LOD geometry and authored distance selection.
- [ ] Preserve source skeletons, skin weights, bind transforms, and required sockets.
- [ ] Preserve source static-model transforms and placements.
- [ ] Preserve native world brush ownership and rendered brush placements.
- [ ] Preserve static collision, dynamic brush collision, terrain shapes, contents, and surface flags.
- [ ] Preserve the committed source-derived atmosphere bindings in the packed Origins world.
- [ ] Resolve all model/world buffers, textures, collision scenes, animation clips, and presentation URIs.
- [ ] Keep generated scene and binary geometry data inside the final runtime packs.

### 3.6 Native FX and remaining camo asset data

- [ ] Implement required native FX decoding/conversion in existing Rust asset-builder modules.
- [ ] Resolve required effect definitions from actual gameplay/presentation references.
- [ ] Convert source effect element selection, lifetime, delays, looping, and spawn timing.
- [ ] Convert source effect spatial offsets, orientation, attachment, velocity, and size behavior.
- [ ] Convert source effect color/alpha curves and material/image selections.
- [ ] Convert required effect model, sound, light, and child-effect references.
- [ ] Bind required source effect activation and stop/reset events to existing ECS consumers.
- [ ] Reuse material conversion completed in section 4 for all FX materials.
- [ ] Populate prepared/effects.nativefx with actual converted runtime definitions.
- [ ] Populate prepared/camo.json with actual required native camo definitions and selections.
- [ ] Preserve material/pattern selection and override behavior implemented in section 4.
- [ ] Resolve all nested FX and camo dependencies inside packs.

### 3.7 Audio, localization, UI, fonts, and cinematics

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

### 3.8 Required catalogs and runtime assembly gate

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

## 4. Complete remaining required material families and render fidelity

Current converter checkpoint adds forty recipes covering 134 more geometry-used materials, bringing implemented dispatch to 112 additions and 53 remaining families. The latest combined cycle passed 31 focused checks: 25 runtime and six builder checks. Installed preflight checked 315 ordinary/unlit materials in 70 families and 67 special materials in 15 families; 502 ordinary/unlit image definitions resolve. The shared run checked 102 ordinary material/state representatives, 25 special representatives and exact packed flag bases in 36 primitives across 28 native models. The prepared subset passed dependency checks for 1,078 pack entries. Live flicker, eye luminance, actor callbacks, source freeze values and unit-rigid skinned flag ordering have controlled coverage. Remaining source lighting, material states and non-unit skinned placement still fail closure. Full conversion, the complete pack and actual-game acceptance remain pending.

Primary files: tools/asset_builder/material.rs, bake.rs, geometry.rs, texture.rs, scenes.rs, gltf.rs, source.rs, src/rendering/opaque.rs, src/rendering/pbr.rs, and src/assets/prepared.rs.

### 4.1 Establish the exact material work inside the existing pipeline

- [x] Reconcile the existing geometry-used family list against current ordinary_recipe and bake::supported dispatch before changing conversion. Installed inventory checks confirmed 1,034 materials in 180 families, initially 15 dispatched and 165 missing. Current installed-source check passed at 53 dispatch gaps after 112 additions. All-family metadata and representative checks passed for the latest 40 recipes; this does not establish exhaustive conversion or runtime acceptance.
- [ ] Reconcile all 165 listed starting-backlog families with actual full-Origins usage; complete required families, including infrequently visible ones, and remove a shared/test dependency only after proving it is unused.
- [ ] Include required material dependencies introduced by exact weapon, attachment, camo, FX, and presentation bindings.
  - [x] Expand full-builder material requirements with nonnull distinct source/replacement pairs reached by the installed Origins upgraded-camo option. The current table selects material set 13 for camo ID 45; 35 native camo definitions supply 68 records, seven zero-pair records, and 112 source/replacement material dependencies. Actual descriptor eligibility and other FX/presentation dependencies remain open.
- [ ] Resolve each required material through the existing zone/patch selection rules.
- [ ] Reuse post-cutoff recovered equations and bindings where they already match the selected programs.
- [ ] Use bounded Ghidra decompilation only for missing selectors, constant writers, material consumers, or source equations.
- [ ] Serialize access to the existing Ghidra project and keep all reverse-engineering work in the authorized workspace.
- [ ] Implement shared operations in the existing modules only where actual selected-program behavior agrees.
- [ ] Keep separate exact family recipes where inputs, transfer order, states, or runtime selectors differ.
- [ ] Preserve precise failures for missing source bindings and unsupported source operations while implementing each family.

### 4.2 Source-faithful channel and render-state conversion

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

### 4.3 Extend the verified layered baker

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

### 4.4 Material selectors, animation, and Solari runtime bindings

- [x] Serialize native camo definitions, the option table, weapon/attachment selectors, knife selection and optic-switch control into the existing runtime pack. Version-3 library implementation and controlled round-trip checks passed for 130 weapons, 56 attachments, 35 installed camo definitions plus the native named knife-template copy. Installed-source check found no explicit camo_knife definition. Recovered WaitForAssetTypeOrFind, DB_CreateNamedDefaultAssetEntry and the type-31 default-name table prove a named copy of installed defaultcamo; this source lookup passed combined installed validation.
- [ ] Prepare disabled and Origins camo45 input variants with native material scope, image preservation, zero weaponParam2 defaults and record-driven weaponParam6 values in the shared conversion run. Exact bound inputs are the variant keys; 214 states serialized in the representative run, including one prepared disabled attachment baseline and 213 explicit pending conversions. Unresolved variants continue to fail final pack closure. Implementation and controlled checks passed. Full-pack and actual-game acceptance remain pending.
- [x] Replace the obsolete static weapon-name override table with the packed native library and the existing shared surface validation. Implementation and controlled checks passed. Full-pack and actual-game acceptance remain pending.
- [ ] Rebuild normal-hand viewmodel camo descriptors from the active weapon, recovered attachment triple, held model index and source optic zoom threshold. Uses prepared source model identities; missing/ambiguous source fields, dual-hand and alternate-role delivery remain explicit gaps. Implementation and controlled checks passed. Full-pack and actual-game acceptance remain pending.
  - [ ] Accept matching held-model source slots only when their complete material plans agree, without inventing an animation index. Implementation and rejection/equivalence regressions added; validation remains pending.
- [ ] Apply prepared camo variants by native model index, group, scope and effective inputs while retaining per-object material handles and resetting source inputs on selection, owner and session changes. Controlled per-object rebinding/default/session-reset checks passed. Knife/world submission, complete live owner selection and shared native allocation/registration failure boundaries remain pending.

- [ ] Recover exact weapon and attachment material selections for every required camo-dependent family. The merged runtime work adds shared viewmodel descriptor construction, native attachment/combined-model selection and a Rust reader for all source weapon/attachment selection fields, now called by the sole builder. Combined checks for 130 weapon and 56 attachment selector inputs and six controlled combined-attachment plans have been added. Native library packing and controlled per-object application/default/session-reset checks passed. The representative pack contains one prepared baseline and 213 explicit pending variants; live normal-hand selector completeness, dual-hand, alternate-role, knife/world paths and actual-game acceptance remain pending.
- [x] Read native camo pattern and material-set data directly through the existing Rust pipeline. Reads all 35 installed definitions, both base images, every pattern tuple/image pair, ordered material sets/pairs/flags, all eight record constants, and the native 128-slot option table. Positive-count null arrays fail explicitly; null individual pairs retain the native rejection behavior. Generated runtime serialization and consumers remain separate outstanding work.
- [ ] Apply source surface masks and material scope to texture and constant overrides. Native input planner implements low-16-bit model-index eligibility, destination-material scope, source semantic 2/5/8 to destination 2/13/14 preservation and command order. Focused checks pass, including indices 16..31, scope mismatch and retained destination samplers. Live descriptor construction and runtime application remain pending.
- [ ] Preserve source pattern-mode image replacement order and success conditions. Native planner preserves first/second swap order, null/equal rejection, the shared 1,024-command limit, partial first-swap success and the requirement that both swaps succeed before the parameter write. Focused checks pass; the runtime selector and submission consumer remain pending.
- [ ] Preserve material-mode replacement lists and the selected replacement materials. Native planner preserves set/record/pair order, zero-pair rejection and newest matching material choice without recursive replacement chaining. Controlled installed checks resolve 105 model-index binding plans across the 35 camo definitions. This is source-input validation, not proof of active gameplay descriptors or completed variant bakes.
- [ ] Preserve the active weaponParam2 values and their disabled/default state. Planner reads exact pattern offset/scale, writes [x,y,scale,1] only after both image swaps succeed, and preserves the supplied initial constants otherwise. Checks cover disabled zero input, partial swap failure, capacity exhaustion, option masking and the mapped-distance boundary. Runtime per-object registration, refresh and reset remain pending.
- [ ] Preserve the active weaponParam6 values for material-set paths that consume them. Planner delivers record XYZ and W+10 only when flag 2 is set, scoped to the replacement material. Consumer checks preserve head-first traversal, including older matching constant writes overwriting newer ones as in the recovered native consumer. All eight record constants remain available; selected shader use and live runtime delivery remain pending.
- [ ] Resolve required model, weapon, attachment, and hand ownership for material state selection.
- [ ] Reset material overrides on weapon replacement, unequip, owner destruction, and session reset. The merged runtime work restores saved material/name/extras on replacement and loading, resets preparation and animation markers, and refreshes animation ownership. Actual-game lifecycle validation and descriptor-driven submissions remain pending.
- [ ] Convert source time controls, offsets, and UV animation inputs into the existing UvAnimation path where applicable.
  - [ ] Consume explicit native render-game milliseconds and shader-start offsets for the tread's fractional-second time component, and compose authored static UV transforms before source scrolling. Implementation and controlled checks passed with live native server/client clock delivery, pause/restart/shutdown handling and source-proven owner offsets; actual-game acceptance remains pending.
- [ ] Convert source scalar lookup/curve controls into the existing ScalarAnimation path where applicable.
- [ ] Use existing AnimatedMaterial state/frame support where it represents the recovered source behavior.
- [ ] Derive required animation timing, loop boundaries, and state selection from source inputs.
- [ ] Preserve continuous source controls through existing supported runtime updates where finite state selection is insufficient.
- [ ] Wire script-vector producers to NativeArtInputs for materials that actually consume those vectors.
  - [x] Deliver actual actor ScriptVector2 eye overrides with initial unmapped state, source ordinary/ground/dug spawn boundaries, Panzer fly-in enablement, head-gib/death disablement and actor reset. Controlled lifecycle checks passed; remote snapshot acknowledgement and actual-game acceptance remain pending.
  - [x] Map the actual ice stones to ScriptVector3 and write the iterative source float progression to all four components. Controlled step, terminal-value and mask-isolation checks passed; native client wait rounding and actual-game timing remain pending.
- [ ] Complete charged staff material inputs and their normal/charged state selection.
- [ ] Complete frozen stone and ice material inputs and their source progression.
- [ ] Complete source-driven emissive, flicker, portal, scrolling, and electrical state consumers.
- [ ] Preserve character damage/reveal and ember/emissive state inputs for the families that use them.
- [ ] Preserve required flag/cloth material geometry motion through the existing supported geometry and Solari integration.
  - [x] Preserve the original packed normal/tangent words through triangle remapping and upstream Uint32x2 glTF loading. Bit-exact checks passed for 36 installed primitives across 28 models; 92 source static instances retain the rigid scene path.
  - [x] Implement source CPU skinning, native-axis basis repacking, flag deformation and mesh-local restoration for unit-rigid placements. Controlled paused-pose, raster/Solari update, skin reset and double-skin rejection checks passed; non-unit placement and actual-game acceptance remain pending.
  - [ ] Apply rigid per-object source flag displacement and normal derivatives, preserve pinned vertices, force fragment vertex alpha, update bounds and refresh Solari geometry in the same frame. Implementation and controlled reset/geometry checks passed with the live native clock; skinned ordering, nonuniform/reflected placements and actual-game acceptance remain pending.
- [ ] Keep scope, reticle, lens, and viewmodel material ownership and visibility tied to actual weapon state.
- [ ] Complete source-derived water, glass, cornea, and transparent material bindings through the current rendering architecture.
- [ ] Ensure changed material handles and textures reach the existing Solari scene/material update path. The merged runtime work refreshes changed material extras, invalidates changed mesh proxies, updates UV-transformed ray meshes after material animation and retains source meshes needed by live animation. Compilation passed with the current unoptimized dev profile after integration into main. Actual-game/Solari validation remains pending.
- [ ] Ensure shared material reuse does not leak one object's animated state into another object. The merged runtime work allocates animated material handles per object and retains/restores source bindings. Descriptor-driven camo changes and actual-game validation remain pending.
- [ ] Ensure material animation waits for required packed texture dependencies before applying a frame. The merged runtime work checks frame dependencies and reports failed loads before binding animation frames. Actual-game validation remains pending.
- [ ] Keep source lighting and view-dependent response out of fixed albedo/emission bakes.
- [ ] Preserve the current source-derived atmosphere and color response while adding material coverage.

### 4.5 Exact family backlog

For each family retained by the Origins usage review, completion requires its selected-program behavior, conversion of all required source materials, resolved pack dependencies, and its intended existing runtime/Solari binding. The listed counts retain the starting inventory; proven unused shared/test content is excluded from final requirements. The second task under each family identifies a focused source behavior to resolve; the family name is a work locator, not proof of that behavior.

#### 4.5.1 Lit model and world families

46 families; 246 currently used materials. Extend material.rs and existing runtime material consumers.

- [ ] Convert family `mc_lit_sm_r0c0n0s0o0_3z86zq2z` for all 92 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact selected lit/sun programs and five active bindings. Explicit weaponParam2 states use centered camo UVs, blue occlusion as the camo color bias, base-alpha masking and encoded color/specular mixing before the existing PBR fit. Retains all occlusion channels separately: red indirect diffuse/direct specular, green reflection, with sun-only authored colorDetailScale.z on direct specular. Live per-model overrides and distinct lighting consumers remain behind pending_runtime. Combined metadata checks passed for 194 ordinary materials in thirty-one families and 319 active image definitions. Thirty-three representative material/state conversions passed from 68 native images, including all six new families, both detail-scale wood variants, the authored half-strength sun-specular edge and two explicit occlusion/camo states. Exhaustive conversion, full-pack and actual-game acceptance remain pending.
  - [x] Resolve active occlusion/detail inputs and any scoped camo texture/constant overrides, then bind the source-selected material state.
- [ ] Convert family `mlv_lit_sm_r0c0n0x0_619829wj` for all 23 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert exact vertex-lit color/normal and base-alpha gloss inputs while preserving separate vertex-derived grid and reflection spherical-harmonic multipliers and source probe response. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `wpc_lit_sm_t0c0_2w3887e6` for all 15 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves geometric normals, diffuse-only response, source texture-alpha × vertex-alpha coverage and the native 128/255 cutoff. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `wpc_lit_sm_b0c0n0s0v0_efz5w56f` for all 10 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement exact specular alpha-reveal color/normal/specular inputs and preserve the authored reveal curve, source vertex-alpha domain and sqrt fragment coverage. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_b0c0s0_805w75j2` for all 8 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_r0c0s0_986ezzjq` for all 8 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves geometric normals, authored specular RGB/gloss, source gamma transfer and existing metallic/GGX fitting. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `mlv_lit_sm_r0c0_j60qqj0e` for all 8 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert exact diffuse-only vertex-lit color input without inventing a specular texture, retaining its geometric-normal and separate grid-lighting correction inputs. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mc_lit_sm_t0c0_90wz6fe2` for all 7 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves geometric normals, diffuse-only response, source texture-alpha × vertex-alpha coverage and the native 128/255 cutoff. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `wpc_lit_sm_r0c0n0_80fe30wz` for all 6 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves authored normal slopes, diffuse-only response with no invented specular map, source gamma transfer and ordinary world bindings. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve the actual selected color, geometric/mapped normal, lighting, coverage, and specular inputs without assuming another family's recipe.
- [ ] Convert family `mlv_lit_sm_r0c0n0s0_wqqj839w` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert exact vertex-lit color/normal/authored specular/gloss channels while retaining separate grid/probe correction and secondary-alpha shadow fallback. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mc_lit_sm_b0c0_fw0jf955` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_r0c0n0s0_ze50j7jz` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with selected lit/sun programs, authored color/normal/specular/gloss inputs and source states. Retains the native linear-lighting color matrix rows and an explicit pending_runtime gate until that parameter binding is connected. Combined checks passed for all 93 ordinary materials in twenty-five families and 137 active image definitions; 23 representative material/state conversions passed, including one explicit attachment camo state. Exhaustive conversion, full-pack and actual-game acceptance remain pending for this increment.
  - [x] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `mc_lit_sm_r0c0n0s0o0_909j993q` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact selected lit/sun programs, five active bindings, weaponParam2 and sun-only colorDetailScale binding. Uses blue-biased camo mixing and retains distinct occlusion channels plus the native linear-lighting color matrix rows. Live owner state, lighting consumers and color-matrix binding remain behind pending_runtime. Combined metadata checks passed for 194 ordinary materials in thirty-one families and 319 active image definitions. Thirty-three representative material/state conversions passed from 68 native images, including all six new families, both detail-scale wood variants, the authored half-strength sun-specular edge and two explicit occlusion/camo states. Exhaustive conversion, full-pack and actual-game acceptance remain pending.
  - [x] Resolve active occlusion/detail inputs and any scoped camo texture/constant overrides, then bind the source-selected material state.
- [ ] Convert family `mc_lit_sm_b0c0n0v0_3559e8jj` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert reviewed model reveal albedo/normal channels and preserve the raw alpha input, authored reveal parameters, vertex-alpha domain and sqrt coverage; geometry-dependent reveal and framebuffer composition remain pending. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime composition acceptance remains pending.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement exact masked flag color/normal/detail/specular inputs and retain source flag displacement, cosine normal derivatives, time binding and forced fragment vertex alpha. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve material-specific coverage, normal/response inputs, and required source geometry motion for the flag path.
- [ ] Convert family `mc_lit_sm_r0c0n0s0_7j414q0z` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with selected lit/sun programs, four active images and weaponParam2 binding. Explicit bound camo states use the native centered UV transform, base-alpha mask and encoded color/specular mixing before gamma transfer and the existing PBR fit. Raw color/detail images and original specular/gloss are retained in the pack. No initial camo state is invented; live per-model overrides and parameter updates remain behind pending_runtime. Combined checks passed for all 93 ordinary materials in twenty-five families and 137 active image definitions; 23 representative material/state conversions passed, including one explicit attachment camo state. Exhaustive conversion, full-pack and actual-game acceptance remain pending for this increment.
  - [x] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `mlv_lit_sm_r0c0n0_3qf192ew` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert exact normal-mapped diffuse-only vertex-lit channels and retain their secondary-vertex lighting route without baking lighting into albedo. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `wpc_lit_sm_b0c0n0_19f3588f` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_b0c0n0s0_68f25z0u` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert reviewed translucent character albedo, normal and specular/gloss channels; retain coverage-before-affine-matrix order, coverage fog and sqrt fragment alpha as explicit runtime gaps. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime composition acceptance remains pending.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_flag_b0c0s0_6406ez95` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement exact translucent flag color/specular inputs and required geometric tangent route, retaining source motion and framebuffer composition. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve material-specific coverage, normal/response inputs, and required source geometry motion for the flag path.
- [ ] Convert family `mc_lit_sm_r0c0d0n0_33ffej1u` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact selected lit/sun programs and detailScale binding. Samples tiled detail normals, adds scaled decoded slopes to the base slopes, then normalizes once; retains diffuse-only response and the raw detail-normal input. Finite base-normal resolution, major-axis mip selection and filtering remain approximations. Combined metadata checks passed for 194 ordinary materials in thirty-one families and 319 active image definitions. Thirty-three representative material/state conversions passed from 68 native images, including all six new families, both detail-scale wood variants, the authored half-strength sun-specular edge and two explicit occlusion/camo states. Exhaustive conversion, full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected detail-map operations, coordinates, and interaction with base normal/specular/coverage inputs.
- [ ] Convert family `mc_lit_sm_t0c0n0_9qf6e4qj` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `mlv_lit_sm_t0c0n0s0_9jfj95q5` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert exact masked vertex-lit color/normal/specular/gloss channels, retaining native cutoff, post-cutoff framebuffer coverage and separate vertex lighting inputs. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `wpc_lit_sm_b0c0_6j30z67q` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `wpc_lit_sm_b0c0n0v0_ew347fe3` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert reviewed world reveal albedo/normal channels with their exact lightmap UV route and preserve raw alpha plus zero/nonzero exponent parameters; geometry-dependent reveal and framebuffer composition remain pending. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime composition acceptance remains pending.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `wpc_lit_sm_t0c0n0_9ew0jz24` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact lit vertex/pixel and sun program checks. Preserves authored normal slopes, diffuse-only response, source texture-alpha × vertex-alpha coverage and the native 128/255 cutoff. The five-family batch passed for all 38 installed materials, including 24 masked materials, eight normal-map and eight response-map bindings, 72 world/model LOD scenes, 35,863 triangles and 316 in-memory pack entries. Source RGB is constant within every used triangle in this batch, so existing vertex gamma transfer is equivalent for these inputs. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `wpc_lit_sm_t0c0n0s0_weef63f3` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `mc_lit_sm_b0c0n0_67fqj7jf` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_b0c0n0s0_21fz2e61` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Uses exact installed lit/sun programs, three active texture bindings, native vertex routes, occlusion constant binding and blend/depth states with the existing PBR fit; native specular/gloss remain packed. Native sqrt coverage and coverage-independent specular before gamma-2 premultiplied composition still need runtime support; the explicit pending_runtime field prevents pack closure. Combined metadata checks passed for all fifty-nine materials in twenty families; twenty representative layered triangles baked/exported and the ordinary glass material converted. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_b0c0n0s0o0_jf00fe4q` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact selected lit/sun programs, four active images and native blend/depth states. Keeps red direct-specular, green reflection/direct-diffuse and blue indirect-diffuse occlusion separate from static albedo. Native gamma-2 premultiplication and sqrt coverage remain behind pending_runtime. Combined metadata checks passed for 194 ordinary materials in thirty-one families and 319 active image definitions. Thirty-three representative material/state conversions passed from 68 native images, including all six new families, both detail-scale wood variants, the authored half-strength sun-specular edge and two explicit occlusion/camo states. Exhaustive conversion, full-pack and actual-game acceptance remain pending.
  - [x] Resolve active occlusion/detail inputs and any scoped camo texture/constant overrides, then bind the source-selected material state.
- [ ] Convert family `mc_lit_sm_b0c0s0_q94f27qw` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert reviewed monkey-fur albedo and authored specular/gloss without activating its unused identity-normal binding; retain native translucent matrix and framebuffer composition gaps. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime composition acceptance remains pending.
  - [ ] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `mc_lit_sm_r0c0_5137qq99` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with selected lit/sun programs, geometric normals and diffuse-only response. The stored identity normal map is inactive in the selected passes. Preserves opaque coverage and records source vertex-alpha attenuation of indirect diffuse independently from sun lighting. Runtime lighting support remains behind pending_runtime. Combined checks passed for all 93 ordinary materials in twenty-five families and 137 active image definitions; 23 representative material/state conversions passed, including one explicit attachment camo state. Exhaustive conversion, full-pack and actual-game acceptance remain pending for this increment.
  - [x] Resolve the actual selected color, geometric/mapped normal, lighting, coverage, and specular inputs without assuming another family's recipe.
- [ ] Convert family `mc_lit_sm_r0c0d0n0s0_qwq4q449` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact selected lit/sun programs, swapped detail/specular slots and detailScale binding. Uses scaled slope addition before normalization with the existing authored specular/gloss PBR fit and retains the raw detail-normal input. Finite base-normal resolution, major-axis mip selection and filtering remain approximations. Combined metadata checks passed for 194 ordinary materials in thirty-one families and 319 active image definitions. Thirty-three representative material/state conversions passed from 68 native images, including all six new families, both detail-scale wood variants, the authored half-strength sun-specular edge and two explicit occlusion/camo states. Exhaustive conversion, full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected detail-map operations, coordinates, and interaction with base normal/specular/coverage inputs.
- [ ] Convert family `mc_lit_sm_r0c0n0s0_4w3wqfe5` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with selected lit/sun vertex and pixel programs and authored color/normal/specular/gloss inputs. Retains the exact V minus gameTime.z animation binding for every texture channel; native time-component generation and live animation remain behind pending_runtime. Combined checks passed for all 93 ordinary materials in twenty-five families and 137 active image definitions; 23 representative material/state conversions passed, including one explicit attachment camo state. Exhaustive conversion, full-pack and actual-game acceptance remain pending for this increment.
  - [x] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `mc_lit_sm_r0c0n0x0_53144qq1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with selected lit/sun programs, mapped normal and base-alpha gloss. Preserves opaque coverage and records distinct source vertex-alpha attenuation for indirect diffuse/specular and gloss-dependent ambient response. The shared direct-light PBR fit remains an approximation; indirect-light consumers remain behind pending_runtime. Combined checks passed for all 93 ordinary materials in twenty-five families and 137 active image definitions; 23 representative material/state conversions passed, including one explicit attachment camo state. Exhaustive conversion, full-pack and actual-game acceptance remain pending for this increment.
  - [x] Resolve the source alpha response and selected specular/gloss consumers independently from coverage.
- [ ] Convert family `mc_lit_sm_t0c0d0n0s0_68eww765` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with exact selected lit/sun programs, detailScale and specular bindings, slope addition before normalization, source coverage and the native 128/255 cutoff. The native post-cutoff framebuffer alpha remains behind pending_runtime. Finite normal-bake resolution/filtering remains an approximation. Combined metadata checks passed for 194 ordinary materials in thirty-one families and 319 active image definitions. Thirty-three representative material/state conversions passed from 68 native images, including all six new families, both detail-scale wood variants, the authored half-strength sun-specular edge and two explicit occlusion/camo states. Exhaustive conversion, full-pack and actual-game acceptance remain pending.
  - [x] Resolve the selected detail-map operations, coordinates, and interaction with base normal/specular/coverage inputs.
- [ ] Convert family `mc_lit_sm_t0c0n0s0_f933q84u` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert reviewed Ray Gun masked albedo, normal and authored specular/gloss channels; retain the post-lighting model color matrix and post-cutoff framebuffer alpha as explicit runtime gaps. Implementation, all-family source metadata checks and representative channel conversions passed; geometry/runtime composition acceptance remains pending.
  - [ ] Resolve the selected pass's alpha-test/coverage equations, native threshold, normal inputs, and response inputs.
- [ ] Convert family `mlv_lit_sm_b0c0_50077370` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert vertex-lit diffuse-only translucent color with exact source routes, retaining sqrt coverage and separate vertex lighting consumers. Implementation, all-family source metadata checks and representative channel conversions passed; runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_b0c0n0_322zqw73` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert vertex-lit normal-mapped translucent color with exact source routes, retaining sqrt coverage and separate vertex lighting consumers. Implementation, all-family source metadata checks and representative channel conversions passed; runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_b0c0s0_41731e52` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert vertex-lit translucent color and authored specular/gloss without an active normal map, retaining sqrt coverage and separate grid/probe terms. Implementation, all-family source metadata checks and representative channel conversions passed; runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_r0c0_475q06w3` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Convert vertex-lit cheap sandbag color while retaining primary-alpha attenuation of indirect diffuse, opaque coverage and unattenuated sun lighting. Implementation, all-family source metadata checks and representative channel conversions passed; runtime lighting consumers remain pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_r0c0d0n0_w65788j0` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement vertex-lit detail-normal slope addition with authored scale and separate source secondary-lighting corrections. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `mlv_lit_sm_r0c0n0x0_8f47740f` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement vertex-lit sandbag base-alpha gloss and preserve primary-alpha indirect lighting attenuation and source probe response. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the native vertex-lighting inputs and keep their lighting contribution separate from baked albedo.
- [ ] Convert family `wpc_lit_sm_b0c0s0_3f3q946z` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the selected blend state, premultiplication/coverage equations, fog participation, and authored response inputs.
- [ ] Convert family `wpc_lit_sm_r0c0s0_qfqz7850` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve active specular/gloss bindings and preserve the source transfer order in the existing PBR output.
- [ ] Convert family `wpc_lit_sm_r0c0x0_5j63fj97` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Exact installed lit/sun programs, active bindings, vertex routes and source states recovered; ordinary recipe implemented using the existing PBR fit. Combined metadata checks passed for all 83 materials in twenty families, with 17 representative conversions across the fifteen new families. Full-family conversion remains pending. Exhaustive conversion, source polygon-offset behavior where enabled, gamma-2 premultiplied composition, vertex-tint interpolation and actual-game/Solari acceptance remain open.
  - [x] Resolve the source alpha response and selected specular/gloss consumers independently from coverage.

#### 4.5.2 Remaining merged layered families

26 families; 92 currently used materials. Extend bake.rs and the existing geometry-aware bake outputs.

- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2` for all 14 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Preserves the first alpha/green color-normal blend and base-color-alpha gloss, then applies the second authored reveal curve to UV2 color, normal slopes and encoded specular/gloss. Combined metadata checks passed for all fifty-nine materials in twenty families; twenty representative layered triangles baked/exported and the ordinary glass material converted. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2` for all 11 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Uses independent authored first/second reveal curves for ordered encoded color, rotated normal slopes, and encoded specular/gloss blends; gamma transfer and the shared PBR fit follow both layers. Selected installed lit/sun programs, active texture/constant slots and vertex routes were recovered. The combined compiled check passed twenty-one tests: metadata checks covered all fifty-eight layered materials in nineteen families and one ordinary glass material, twenty representative layered source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for thirty-three recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1` for all 10 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; all ten installed materials passed conversion/export checks across 84 surfaces and 441 triangles. Preserves layer-alpha × vertex-green encoded-color mixing before gamma transfer, base normal, and base-alpha gloss. Only the three selected texture arguments are applied; stored identity layer normals are inactive. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement this layer combination's active color, normal, specular/gloss, coordinate, and vertex-weight inputs.
- [ ] Convert family `lit_sm_r0c0n0x0_m1c1` for all 10 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; all ten installed materials passed conversion/export checks across 116 surfaces and 1,002 triangles. Preserves multiply-before-gamma color, vertex-green weight, half2 UVs, authored wrap/clamp, base normal, and base-alpha gloss modulated by encoded tint luminance. Periodic baking is used only where UV/frame/sampler conditions permit it; other surfaces use the existing atlas path. Combined multiply-family checks passed for 15 materials, 168 surfaces, 1,371 triangles and 84 in-memory pack entries. Native environment/direct-lobe/GGX fitting and raster tangent-frame approximations remain explicit; full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the authored multiply layer, including its vertex/mask weight and interaction with earlier color/response operations.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1v1` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; all five installed materials passed conversion/export checks across 159 surfaces, 2,052 triangles and 45 in-memory pack entries. Preserves the selected reveal curve including authored zero exponents, UV/auxiliary normal-frame inputs, reveal-weighted color/normal mixing, and unchanged base-alpha gloss. Native view-dependent response fitting and raster tangent-frame differences remain approximations; full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the selected reveal curve and its effect on color, normal, specular/gloss, and base-alpha response.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Keeps the first alpha/green color blend separate from the second reveal-weighted color, normal and specular/gloss layer, including its single authored auxiliary normal transform. Selected installed lit/sun programs, active texture/constant slots and vertex routes were recovered. The combined compiled check passed twenty-one tests: metadata checks covered all fifty-eight layered materials in nineteen families and one ordinary glass material, twenty representative layered source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for thirty-three recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0_m1c1` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented with the recovered multiply-before-gamma equation, base normal, vertex-green weight, auxiliary half2 UVs, and source wrap/clamp samplers; installed conversion/export checks passed for all three materials across 46 surfaces and 343 triangles, with packed KTX2 mip layouts validated. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the authored multiply layer, including its vertex/mask weight and interaction with earlier color/response operations.
- [ ] Convert family `lit_sm_r0c0n0s0_b1c1n1s1v1` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Uses the authored reveal curve for color, normal and encoded base/layer specular/gloss mixing before the shared PBR fit. Combined metadata checks passed for all twenty materials in ten families, with eleven representative installed-source triangle bakes/exports and validated KTX2/native-specular bindings. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the selected reveal curve and its effect on color, normal, specular/gloss, and base-alpha response.
- [ ] Convert family `lit_sm_r0c0n0x0_a1c1` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; all three installed materials passed conversion/export checks across 62 surfaces, 314 triangles and 28 in-memory pack entries. Preserves source RGB × layer-alpha × vertex-green addition before gamma transfer, unchanged base normal/gloss, and auxiliary half2 UVs. Above-one color values use linear RGBA16F KTX2 with linear mips; existing PBR texture loading uses the matching linear color-space binding. Oversized charts are rotated into bounded pages where feasible without changing source triangles, UV metrics, or texel density; charts that cannot fit still fail explicitly. Sparse large charts incur substantial atlas/pack memory cost. Full-pack and actual-game acceptance remain pending.
  - [ ] Resolve and implement the authored add-layer contribution and its coverage, tint, and response ordering.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Mixes encoded 0.2 base specular and base-color-alpha gloss with authored layer specular/gloss using layer alpha times vertex green. Combined metadata checks passed for all twenty materials in ten families, with eleven representative installed-source triangle bakes/exports and validated KTX2/native-specular bindings. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement this layer combination's active color, normal, specular/gloss, coordinate, and vertex-weight inputs.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Keeps reveal layer specular and second UV2 multiply decal inputs separate; multiplies mixed gloss by encoded decal-tint luminance dot(tint, [0.3,0.6,0.1]) without tinting specular RGB. Combined metadata checks passed for all twenty materials in ten families, with eleven representative installed-source triangle bakes/exports and validated KTX2/native-specular bindings. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the authored multiply layer, including its vertex/mask weight and interaction with earlier color/response operations.
- [ ] Convert family `lit_sm_r0c0n0_m1c1_m2c2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented and both installed materials passed conversion/export checks across six surfaces and 26 triangles. Green/blue weights, separate half2 UV pairs, multiply-before-gamma order, unchanged base normal, and authored samplers are preserved. The combined multiply-family check passed for five materials, 52 surfaces, 369 triangles and 22 in-memory pack entries. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the authored multiply layer, including its vertex/mask weight and interaction with earlier color/response operations.
- [ ] Convert family `lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Retains authored base specular/gloss through the first alpha/green color-normal blend, then mixes the second specular/gloss with its separate reveal weight and UV2. Selected installed lit/sun programs, active texture/constant slots and vertex routes were recovered. The combined compiled check passed twenty-one tests: metadata checks covered all fifty-eight layered materials in nineteen families and one ordinary glass material, twenty representative layered source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for thirty-three recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Preserves nine active source maps on their base/UV1/UV2 coordinates; mixes base, first, and second encoded specular/gloss in source order with two authored reveal curves. Selected installed lit/sun programs, active texture/constant slots and vertex routes were recovered. The combined compiled check passed twenty-one tests: metadata checks covered all fifty-eight layered materials in nineteen families and one ordinary glass material, twenty representative layered source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for thirty-three recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1_b2c2n2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Blends both encoded colors in order, while only the second layer changes normal slopes using its authored rotation and alpha times vertex blue; base gloss is retained. Selected lit/sun programs and active bindings were recovered from the installed source; stored inactive scaleRGB constants are not applied. The combined compiled check passed nineteen tests: metadata checks covered all twenty materials in ten families, eleven representative source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for twenty-four recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1_a2c2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Blends the first color/normal layer before the independent UV2 alpha/blue additive color; base gloss and the blended normal remain unchanged by the add. Selected lit/sun programs and active bindings were recovered from the installed source; stored inactive scaleRGB constants are not applied. The combined compiled check passed nineteen tests: metadata checks covered all twenty materials in ten families, eleven representative source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for twenty-four recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the authored add-layer contribution and its coverage, tint, and response ordering.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1_b2c2n2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Uses separate authored rotations from sixteen-byte auxiliary vertices for sequential first/second normal blends; ordered color blends share their alpha/green and alpha/blue weights and retain base gloss. Selected lit/sun programs and active bindings were recovered from the installed source; stored inactive scaleRGB constants are not applied. The combined compiled check passed nineteen tests: metadata checks covered all twenty materials in ten families, eleven representative source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for twenty-four recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Keeps first-layer reveal color/normal/specular/gloss mixing before the UV2 alpha/blue add; additive color does not modulate the mixed native specular/gloss. Selected lit/sun programs and active bindings were recovered from the installed source; stored inactive scaleRGB constants are not applied. The combined compiled check passed nineteen tests: metadata checks covered all twenty materials in ten families, eleven representative source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for twenty-four recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the authored add-layer contribution and its coverage, tint, and response ordering.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Applies both authored reveal curves to color and normal slopes while retaining the first blended native specular/gloss unchanged through the second layer; supports the installed zero second-reveal exponent. Selected installed lit/sun programs, active texture/constant slots and vertex routes were recovered. The combined compiled check passed twenty-one tests: metadata checks covered all fifty-eight layered materials in nineteen families and one ordinary glass material, twenty representative layered source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for thirty-three recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0s0_b1c1n1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Preserves authored base specular/gloss on UV0 while blending encoded color and transformed layer normal slopes using layer alpha times vertex green. Combined metadata checks passed for all twenty materials in ten families, with eleven representative installed-source triangle bakes/exports and validated KTX2/native-specular bindings. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement this layer combination's active color, normal, specular/gloss, coordinate, and vertex-weight inputs.
- [ ] Convert family `lit_sm_r0c0n0x0_a1c1_m2c2` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Adds layer RGB times layer alpha and vertex green, then multiplies by the separate UV2 blue-weighted tint before gamma transfer; base gloss is modulated only by that second tint luminance. Selected lit/sun programs and active bindings were recovered from the installed source; stored inactive scaleRGB constants are not applied. The combined compiled check passed nineteen tests: metadata checks covered all twenty materials in ten families, eleven representative source triangles baked/exported with valid KTX2/native-specular bindings, and CPU/GPU agreement passed for twenty-four recipe modes and eight samplers. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the combined multiply/add layer order and each layer's authored coordinates and masks.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1_b2c2` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented; the installed material passed conversion/export checks across one surface and four triangles. Preserves ordered green/blue alpha-weighted color blends, separate UV pairs, base normal/gloss, opaque coverage, and selected-pass samplers. Combined color-blend checks passed for eleven materials, 85 surfaces, 445 triangles and 75 in-memory pack entries. Full-pack and actual-game acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1_b2c2n2s2` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Uses ordered alpha/green and alpha/blue color blends; only the second layer blends rotated normal slopes and encoded specular/gloss. The second weight is the direct alpha/blue product, without a reveal curve. Combined metadata checks passed for all fifty-nine materials in twenty families; twenty representative layered triangles baked/exported and the ordinary glass material converted. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the additional authored layer's UVs, weights, normal/response contributions, and operation order in the committed baker.
- [ ] Convert family `lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Applies reveal color/normal/specular first, alpha/blue additive color second, and UV3 multiply tint weighted by vertex alpha third; only the third tint luminance modulates mixed gloss. Stored inactive scaleRGB2 is not applied. Combined metadata checks passed for all fifty-nine materials in twenty families; twenty representative layered triangles baked/exported and the ordinary glass material converted. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement the combined multiply/add layer order and each layer's authored coordinates and masks.
- [ ] Convert family `lit_sm_r0c0n0x0_t1c1n1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Uses the source inclusive alpha-times-green threshold of 0.5 to select color and transformed normal, preserving base gloss and the selected layer alpha in the packed color. Native opaque framebuffer alpha still needs a runtime consumer; the explicit pending_runtime field prevents pack closure. Combined metadata checks passed for all fifty-nine materials in twenty families; twenty representative layered triangles baked/exported and the ordinary glass material converted. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve the selected pass's coverage and blend behavior and implement the additional layer without losing its authored surface response.
- [ ] Convert family `lit_sm_r0c0s0_b1c1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer. Converter implemented. Uses the source geometric normal, four-byte auxiliary UV1, and authored base specular/gloss; inactive stored identity normal maps do not affect the recipe. Combined metadata checks passed for all twenty materials in ten families, with eleven representative installed-source triangle bakes/exports and validated KTX2/native-specular bindings. Exhaustive conversion, full-pack and actual-game/Solari acceptance remain pending.
  - [x] Resolve and implement this layer combination's active color, normal, specular/gloss, coordinate, and vertex-weight inputs.

#### 4.5.3 Unlit and decal families

31 families; 92 currently used materials. Extend material.rs, bake.rs where needed, and existing runtime material consumers.

- [ ] Convert family `mc_unlit_replace_3j970129` for all 12 currently used materials and connect its packed outputs to the existing runtime/Solari consumer. Technique-2 converter implemented from the selected installed unlit programs, active scaleRGB binding and exact native states. Preserves gamma-2 surface color, authored HDR intensity and the distinct opaque/add/blend alpha roles. Blend coverage premultiplies color before transfer; additive RGB ignores source alpha because its native RGB/alpha factors do not consume it. Focused HDR/coverage, vertex-alpha export and pending-runtime closure checks passed. Installed-source preflight passed for all 83 materials in twenty families, including all twenty unlit materials, and 17 representative conversions passed across the fifteen new families. Exhaustive conversion, polygon-offset behavior, shared linear-HDR composition and actual-game acceptance remain open.
  - [x] Resolve the unlit output, source intensity, vertex transfer, opacity, blend operation, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_normal_script_q0zez5ju` for all 7 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the multi-layer normal/color equations and script-vector controls, then bind their source producers.
- [ ] Convert family `wpc_sw4_3d_unlit_1layer_8691j77q` for all 7 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement actual selected one-layer gameplay UV/HDR/alpha terms, retaining source time/script/parallax and framebuffer requirements. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_model_unlit_cheap_zombie_eyes_jq3e7eqw` for all 6 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the eye emission, opacity, and actor-state selection required by the selected unlit program.
- [ ] Convert family `mlv_sw4_3d_unlit_3layer_normal_script_766fz50u` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the multi-layer normal/color equations and script-vector controls, then bind their source producers.
- [ ] Convert family `wpc_unlitdecalblend_add_j26wq580` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement source world decal-add encoded color and weighted coverage inputs while retaining native framebuffer composition. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `wpc_unlitdecalblend_multiply_35079164` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement source world decal-multiply texture terms with authored sampler addressing while retaining fog/vertex and framebuffer composition. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `mc_sw4_3d_unlit_4layer_nofog_eee768jw` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_scroll_jq69qew7` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source UV motion and timing and bind them to the existing UV/material animation consumer.
- [ ] Convert family `mlv_sw4_3d_unlit_3layer_nofog_falloff_zfeather_z0zw1090` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve layer composition, view/depth falloff, feathering, fog behavior, and required live inputs.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement exact three-to-two-to-one layer mixing with explicit script169 inputs, preserving authored alpha overshoot and view-dependent falloff. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the layer-composition and falloff equations and preserve their source live inputs and fog behavior.
- [ ] Convert family `mc_sw4_3d_viewmodel_unlit_j299w86e` for all 3 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Read and convert actual technique-3 viewmodel emission and native screen blend, retaining explicit source composition gaps. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve viewmodel-specific opacity, emissive/unlit output, fog/depth behavior, and weapon-state ownership.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_falloff_5z943qe4` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement the opaque three-layer branch without activating its unused layer-three alpha constant. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the layer-composition and falloff equations and preserve their source live inputs and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_falloff_zfeather_wq4jwzz6` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement the actual gameplay-pass layer equations with explicit script controls and preserve native depth feather inputs. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve layer composition, view/depth falloff, feathering, fog behavior, and required live inputs.
- [ ] Convert family `mc_sw4_3d_viewmodel_unlit_8216051u` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Read and convert actual technique-3 premultiplied viewmodel emission and coverage with authored emissive scaling and culling. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
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
  - [x] Implement selected no-fog one-layer gameplay UV/HDR/alpha terms without substituting a Radiant preview pass. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_falloff_5z909874` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement source blended coverage independently from layer color weights and retain sqrt coverage composition. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the layer-composition and falloff equations and preserve their source live inputs and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_3layer_nofog_qw71w785` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_unlit_4layer_jw0z1973` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `mc_sw4_3d_viewmodel_unlit_60466j88` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Read and convert actual technique-3 additive viewmodel emission while retaining source fog attenuation and framebuffer requirements. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve viewmodel-specific opacity, emissive/unlit output, fog/depth behavior, and weapon-state ownership.
- [ ] Convert family `mc_unlit_multiply_97923qz1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement native plain-unlit multiply texture terms without applying its inactive RGB-scale constant; preserve neutral-white fog and destination composition gaps. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `mc_unlitdecalblend_add_fzw1z66f` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement source unlit decal-add luminance coverage and premultiplied color terms, retaining fog-disabled vertex behavior and destination alpha composition. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `mc_unlitdecalblend_multiply_34w7q3jw` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement source unlit decal-multiply neutral-white texture terms, retaining fog/vertex modulation and destination alpha composition. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the actual receiver-dependent blend operation and integrate its source-derived receiver/material behavior through the existing conversion path.
- [ ] Convert family `mlv_sw4_3d_unlit_3layer_nofog_falloff_fjz6ze0u` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement vertex-lit three-layer inputs while retaining separate source lighting and view falloff requirements. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve the layer-composition and falloff equations and preserve their source live inputs and fog behavior.
- [ ] Convert family `mlv_sw4_3d_unlit_4layer_nofog_2j0ezz67` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve each selected layer's coordinates, tint/mask operations, emission scale, and fog behavior.
- [ ] Convert family `wpc_sw4_3d_unlit_3layer_normal_script_q1z1f6w1` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the multi-layer normal/color equations and script-vector controls, then bind their source producers.

#### 4.5.4 Special material families

62 families; 234 currently used materials. Extend the existing material/bake modules and their runtime consumers.

- [ ] Convert family `mc_sw4_3d_phong_emissive_alcatraz_90892wq5` for all 19 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement exact static alcatraz emission and HDR specular/gloss retention with separate source reflection/direct-light amounts. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_viewmodel_transparent_z9z0z75f` for all 18 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native transparency, normal/specular response, view dependence, and scope/viewmodel ownership through the current Solari material path.
- [ ] Convert family `mc_sw4_3d_wpn_zmb_staff_3layer_94jz2ff5` for all 17 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve all staff layers, source camo/material replacements, and charge/state parameters, then bind the active staff owner.
- [ ] Convert family `mc_sw4_3d_phong_emissive_q0751958` for all 15 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement exact static emissive surface equations, including authored specular/gloss pushes and emission push fourth-power behavior. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
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
  - [x] Implement exact static macro/micro tile color, normal slopes and specular/gloss equations with source-derived repeat periods. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve authored tile coordinates, blend weights, and active normal/specular contributions using the committed bake machinery.
- [ ] Convert family `mc_sw4_3d_optic_lens_4w47028j` for all 8 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native transparency, normal/specular response, view dependence, and scope/viewmodel ownership through the current Solari material path.
- [ ] Convert family `mc_sw4_3d_light_sphere_46j58w7u` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve the selected effect's geometry/material controls, source animation/state, opacity, emission, and depth behavior.
- [ ] Convert family `mc_sw4_3d_scrolling_falloff_zfeather_69456263` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve scrolling coordinates, falloff/distortion inputs, source timing, coverage, and depth-dependent terms.
- [ ] Convert family `mlv_sw4_3d_cod7_tile_blend_spec_w8wezqe6` for all 5 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement vertex-lit tile surface equations with exact repeat periods while keeping secondary lighting and AO/probe consumers separate. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve authored tile coordinates, blend weights, and active normal/specular contributions using the committed bake machinery.
- [ ] Convert family `mc_sw4_3d_char_hair_287j114q` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_char_skin_emberglow_7q60f126` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve character-specific normal/response inputs and any damage/reveal, skin, hair, or emissive state controls used by the selected pass.
- [ ] Convert family `mc_sw4_3d_cod7_tile_blend_q2q2zej6` for all 4 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement diffuse-only tile color and normal slope equations with exact source repeat periods and no invented specular texture. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
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
  - [x] Connect source script170/time25 flicker sampling and authored min/max emission to the live native clock, challenge/soul-box callbacks and normal-view shovel inputs. All three source materials passed metadata, CPU/GPU factorization and prepared dependency checks; actual-game acceptance remains pending.
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
  - [x] Implement explicit script169/time25 Fritz warp, base-color glow blending and nonperiodic UV sparkle with the native cutoff and composition gates. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_phong_emissive_script_multi_72q2fj2u` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement the inclusive script169.z emission threshold with authored normal/response inputs and explicit effective states. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_player_icon_jef08712` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source-selected image/color/state inputs and exact world/viewmodel visibility, placement, and blend behavior.
- [ ] Convert family `mc_sw4_3d_scrolling_alpha2_distort_qwj8139f` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve scrolling coordinates, falloff/distortion inputs, source timing, coverage, and depth-dependent terms.
- [ ] Convert family `mc_sw4_3d_zm_ice_script_static_blizzard_45q67691` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve script-driven ice progression, opacity, emission, normals, and any blizzard state inputs.
- [ ] Convert family `mlv_sw4_3d_cod7_tile_blend_26zq1w22` for all 2 currently used materials and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement diffuse-only vertex-lit tile color/normal equations and preserve separate secondary lighting and AO inputs. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
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
  - [x] Implement script172 RGBA modulation before source glow RGB-times-alpha and HDR transfer, retaining missing producer states. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_glass_emissive_jf185j5e` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve native transparency, normal/specular response, view dependence, and scope/viewmodel ownership through the current Solari material path.
- [ ] Convert family `mc_sw4_3d_phong_emissive_alcatraz_8q551153` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement translucent emissive surface terms while retaining native sqrt coverage and gamma2 composition. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_phong_emissive_fritz_61728967` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement the exact masked Fritz state, preserving warped diffuse/specular inputs, source UV bounds and framebuffer alpha requirements. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `mc_sw4_3d_phong_emissive_script_multi_j05zz52w` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [x] Implement the masked script-controlled emission threshold and preserve the native cutoff and framebuffer composition. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
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
  - [x] Implement explicit script172/time25 water surface evaluation and source leading-edge emission without inventing an initial state; retain dynamic reflection, matrix and runtime producer gaps. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
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
  - [x] Implement vertex-lit emissive surface terms while unconditionally retaining secondary inverse-SH and shadow-fallback lighting requirements. All-family source metadata and representative conversion checks passed; final runtime acceptance remains pending.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.
- [ ] Convert family `wpc_cod7water_we876z2f` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve water coordinates, normal/response composition, coverage, reveal, and source live controls through the existing material path.
- [ ] Convert family `wpc_default` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve this actual source material's selected program and state; preserve its real role without using it as a fallback for other families.
- [ ] Convert family `wpc_sw4_3d_burning_embers_nuketown_74j6971w` for all 1 currently used material and connect its packed outputs to the existing runtime/Solari consumer.
  - [ ] Resolve source emission masks/scales, flicker or script controls, normal/response inputs, and fog participation.

### 4.6 Material pack integration and completion gate

- [x] Reject unresolved camo input states during build_assets check even when their referenced entries exist. Controlled installed-source checks verify camo-state rejection, material-state rejection and valid-subset dependency closure; no full game pack is claimed.
- [ ] Route the existing material command through actual family conversion for every applicable family. Converted ordinary/special/layered selection now uses the existing shared baker and rejects unresolved states; combined validation remains pending.
- [ ] Route the existing model command through actual converted materials rather than retaining a raw-native-channel bypass. The bypass is removed; all native LODs use converted materials and preserved source skeleton bindings. Combined validation remains pending.
- [ ] Keep material/model checks inside the sole Rust builder and existing pack format.
- [ ] Ensure selected material packs contain all generated KTX2 channels and binary scene dependencies they reference.
- [ ] Ensure converted world scenes use baked material/mesh assignments for every new recipe.
- [ ] Ensure converted model LODs use the correct source-selected material assignments.
- [ ] Preserve native material distinctions across base, upgraded, camouflaged, damaged, and animated variants.
- [ ] Verify the existing conversion dispatch reports zero unimplemented required families.
- [ ] Verify actual conversion reports zero unresolved required materials.
- [ ] Resolve usage of the 1,034 starting geometry-used materials; convert every actual Origins dependency or preserve its source-defined non-visible role, and exclude shared/test content only after proving non-use.
- [ ] Verify required material variants introduced by exact presentation/camo/FX dependencies also convert.
- [ ] Verify generated KTX2 contents and mip ranges through the existing texture validator.
- [ ] Verify all material texture, scene, frame, and metadata links resolve inside their packs.
- [ ] Verify source triangle counts and required geometry/skin bindings survive material baking.
- [ ] Run the existing focused mathematical and installed-source checks affected by converter changes.
- [ ] Review changed conversion code and remove replaced obsolete callers and bypass paths.
- [ ] Complete every exact-family task retained as a full-Origins dependency before claiming final material fidelity; initial gameplay and other implementation proceed under todo.md.
- [ ] Record any non-material full-pack blocker separately without calling the final pack complete.
- [ ] Complete final Solari visual acceptance with the finished required material set and full pack; run initial gameplay acceptance earlier under todo.md.

## 5. Full Origins pack, gameplay, rendering, and release validation

Primary files/tools: tools/build_assets.rs, src/pack.rs, src/assets.rs, src/verification/remote.rs, tools/test_support.py, existing gameplay scenario scripts, and existing rendering validation tools.

These checks apply to the full final scope after the initial acceptance in todo.md. Completed trigger implementation is retained there; final coverage of its consumers remains below. Compilation, pack integrity, actual gameplay, visual inspection, and measured performance are separate completion gates.

### 5.1 Build and inspect the complete runtime pack

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

### 5.2 Build the current game

- [ ] Build the changed game using cargo build --features dev-tools --locked with static Bevy linking.
- [ ] Address compiler failures caused by the completed implementation.
- [ ] Run focused existing checks required by changed gameplay/pack integration.
- [ ] Reuse the current target/debug/origins.exe for repeated scenarios until compiled inputs change.
- [ ] Ensure the executable under validation contains the changed code, embedded data, dependencies, features, and build configuration.
- [ ] Place/use the complete compatible origins.pack through the existing loader path.
- [ ] Confirm missing/corrupt/incompatible pack failures identify the actual failing dependency.

### 5.3 Runtime scenario discipline

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

### 5.4 Actual gameplay acceptance

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

### 5.5 Solari material and render acceptance

- [ ] Launch the current executable with the complete newly generated pack after the material completion gate.
- [ ] Confirm the actual scene uses the existing intended Solari/PBR path.
- [ ] Confirm each required completed family reaches its intended visible or source-defined non-visible role.
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

### 5.6 Release performance and local distribution

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

### Final completion gates

- [ ] All material families retained as actual full-Origins dependencies have converters, packed outputs, and intended runtime/Solari bindings; exclusions from the 165-family starting backlog have proven non-use.
- [ ] All material-dependent selectors, live inputs, animations, and source states are integrated.
- [ ] Native mode, zone, access, spawn, barrier, crafting, and gramophone lifecycle gaps are complete.
- [ ] Required native animation data, flags, pose composition, export bindings, and gameplay playback are complete.
- [ ] Required gameplay catalogs, source values, world/model data, FX, camo, audio, UI/fonts, and cinematics are complete.
- [ ] The complete generated runtime pack loads and all required dependencies resolve.
- [ ] The current game executable passes the required actual gameplay scenarios.
- [ ] Actual-window inspection validates the completed material/render behavior.
- [ ] Any claimed performance change has release-build measurements.
- [ ] The final report distinguishes implemented work, checks actually run, visual proof, measured performance, approximations, and unresolved gaps.
