# Origins: built-in Bevy rendering and external assets

Status: architecture specification; implementation has not started.
Branch: `codex/basic-bevy-rendering`.
Base commit: `8ee2cc13b66dba85bd43894e2006c72ac94f8491`.

## 1. Intended result

Origins uses Bevy's normal asset loading and rendering, Avian for physics, and the existing game modules for Origins rules. External tools produce ordinary asset files. This repository contains no asset extractor, native binary decoder, material baker, custom renderer, or proprietary runtime pack.

Success means both deletion and preservation: the old infrastructure is gone, and full-map content and implemented gameplay still work. A clean build, model preview, or walkable approximation alone is insufficient.

This specification supersedes previous pack-only output, custom-builder, and renderer-fidelity requirements for this branch. Simplified visuals are accepted. Gameplay behavior, map coverage, source data, and collision semantics are preserved. Full Origins remains the goal; existing unfinished mechanics remain explicitly unfinished.

## 2. Worktree and operating boundaries

All work occurs in `C:/Users/brand/.codex/worktrees/basic-bevy-rendering/BlackOps2_Rust`. The original checkout is not edited, built, reset, cleaned, or used to generate assets. Shared Git metadata changes needed for this worktree and branch are expected.

Every Cargo invocation uses process-local settings:

```powershell
$env:CARGO_TARGET_DIR = Join-Path $checkout 'target/build'
$env:CARGO_HOME = Join-Path $checkout 'target/cargo-home'
```

Keep the installed Rust toolchain, locked surviving crate versions, static linking, profiles, and `rust-lld.exe`. Do not copy previous builds, Cargo caches, packs, or extracted assets. Store external tools, their private environments/caches, exports, logs, and captures under this checkout's ignored `target` directory. Keep assets untracked.

Original game payloads come only from `C:/Program Files (x86)/Steam/steamapps/common/Call of Duty Black Ops II`. Recovered source in `C:/Users/brand/Documents/T6ZMResearch/t6zm-master` may explain game behavior and values. Do not obtain asset payloads from another project's exports.

## 3. Final data flow

```text
Authorized Steam installation
    -> unmodified external exporters/converters
    -> ordinary files under target/assets
    -> Bevy AssetServer and built-in loaders
    -> game-owned scene bindings
    -> Bevy rendering / animation / audio + Avian physics
    -> existing Origins gameplay modules
```

No runtime process invokes an exporter. No runtime process reads fastfiles, IPAKs, native animation binaries, or external-tool work directories. Loading never falls back to the old pack or builder.

External tools remain external dependencies pinned by release or upstream commit. Do not copy their extraction code into this repository, patch them, fork them for missing support, or implement substitutes. A small invocation script may run documented commands and stage files; it may not decode assets, compose geometry, evaluate materials, or synthesize gameplay definitions.

## 4. External tool selection and current evidence

| Responsibility | Selection | Evidence / acceptance boundary |
| --- | --- | --- |
| Primary BO2 extraction | OpenAssetTools Unlinker v0.33.0 | Runs against installed `zm_tomb.ff`; model GLB/glTF and ordinary asset export are supported. Full Origins coverage is not established. |
| Native world and collision gap | `eithan1231/open-fastfile`, commit `088dac3bdee0e6e653871f877ad8ee4326d314a4` | Candidate only. Contains T6 `gfxworld` and `clipmap` dumpers and an existing `--collision-only` GLB export command. Not yet run on Origins. |
| Render-world alternatives | Existing Husky / C2M releases | BO2 rendered geometry and placements are documented. Their rendered mesh is not proof of native collision coverage. |
| Specific missing model, animation, image support | Tom Crowley's tools, Greyhound, or another existing exporter/importer | Use only when the primary route lacks a required capability; prove the exact export and import on representative Origins content. |
| Format conversion | Existing application/import-export plugin | Use its ordinary conversion path. Do not write a converter or introduce baking. |

The OAT probe found both `gfxworld` and `clipmap` named `maps/mp/zm_tomb.d3dbsp`. Its targeted export exited successfully but wrote only `zone_source/zm_tomb.zone`; no world or collision payload was produced. Exit status alone is not an asset-coverage check.

The open-fastfile candidate reads native collision records; its source exposes brush contents, face flags, terrain triangles, submodels, static models, and dynamic entities. Its downstream collision GLB exporter filters/groups content for its own game, and its documentation names unsupported cases. Verify that the output retains everything Origins needs; do not assume the raw dumper's coverage survives into the GLB.

Use the candidate only for the missing export capabilities. Do not adopt its browser engine, gameplay, renderer, material baking, lighting pipeline, or unrelated conventions. It is recent and experimental, not a proven Origins exporter. A blocked external command is reported with its exact input and failure; it is not repaired with local extraction code.

Sources:

- https://openassettools.dev/
- https://github.com/Laupetin/OpenAssetTools/blob/main/docs/SupportedAssetTypes.md
- https://tom-crowley.co.uk/downloads/
- https://github.com/eithan1231/open-fastfile
- https://github.com/eithan1231/open-fastfile/blob/088dac3bdee0e6e653871f877ad8ee4326d314a4/scripts/cod/t6/dumpers/clipmap.py
- https://github.com/sheilan102/C2M
- https://github.com/Scobalula/Husky

## 5. Runtime asset contract

### Files and launch

Use `target/assets` as the isolated runtime asset root. Pass it explicitly:

```text
origins.exe --asset-root <absolute-checkout-path>/target/assets [existing gameplay/scenario options]
```

`--asset-root` is required, accepts an absolute existing directory, and works together with existing scenario options. A missing root or required file produces a clear load error. There is no working-directory lookup, pack discovery, or search of other checkouts.

Use Bevy's default filesystem asset source configured to this root. Do not implement an `AssetReader`, virtual filesystem, archive, asset proxy, cache manager, or custom image/mesh/animation loader.

| Content | Runtime representation |
| --- | --- |
| World and props | GLB/glTF scenes and meshes, preserving exported transforms and node names |
| Actors and weapons | GLB/glTF skins, skeletons, meshes, and animation clips |
| Collision | Externally exported collision meshes plus explicit contents, surface, and ownership data |
| Images | PNG by default; existing directly supported formats may remain without an extra conversion pass |
| Audio | WAV, OGG, or FLAC consumed by Bevy |
| Gameplay definitions | Existing typed `GameDefinition` and domain data with ordinary file references |
| Scene bindings | Explicit source-ID to file/node/clip references in game-owned data |

Keep relative asset references inside the declared root. Use glTF scene/animation labels and stable names. Do not infer gameplay identity from array position or exporter traversal order.

### Gameplay data is independent of extraction

Retain the useful domain types and validation in `origins` and its gameplay consumers. Remove pack and prepared-art dependencies from them. Keep `game.json` as the ordinary game-definition entry file; it is data, not a second export manifest or new versioned framework.

Preserve source-derived tuning, weapon definitions, spawn rules, hit regions, collision flags, animation events, round data, and interaction/quest identities. External exports supply those fields where supported. Existing source-backed game data and recovered source may supply game-owned definitions through explicit data entries. Do not port the builder's parsers/generators into the runtime or a preparation script. Do not fabricate missing values.

Use the existing definition's content records for file, node, clip, and material references. Extend only fields required by actual consumers. Delete `PreparedArt`, `prepared/runtime.json`, source-zone ranking at runtime, native material contracts, and duplicate registries. Resolve source patch precedence during external extraction and staging, with one explicit final binding per identity. Conflicts are errors, not runtime search heuristics.

Full-map asset coverage and gameplay completion are separate. Preserve existing delivery-scope distinctions; never silently downgrade to the first playable area or advertise unfinished full-game systems as complete.

## 6. Module responsibilities

Retain the current gameplay module organization. This is not a wholesale project reorganization.

| Owner | Final responsibility |
| --- | --- |
| App composition (`lib`) | Configure DefaultPlugins, asset root, window, Avian, game states, and game plugins. No custom asset source or render registration. |
| `assets` | Read typed game data, request Bevy handles, observe dependency failures, bind loaded scenes to game identities. |
| `rendering` | One small setup plugin for world/viewmodel cameras, ordinary lights, fog, and stock renderer settings. |
| `session` | Ordered session initialization, entity ownership, teardown, restart, and load-to-play handoff. |
| `presentation`, weapons, actors | Select assets and animation clips; attach equipment; update game-specific visibility and transforms. |
| Animation | Bevy AnimationPlayer/AnimationGraph playback; retain only necessary game event, root-motion, and state-selection integration. |
| Combat/environment effects | Spawn and retire ordinary Bevy entities on gameplay events; preserve cue timing and ownership. |
| HUD/frontend/audio | Existing game behavior using Bevy UI, text, images, and audio. |
| Physics/navigation/gameplay | Preserve movement, collision policy, combat, interactions, pathfinding, progression, and rules. |
| Verification | Gameplay scenarios, process ownership, screenshots, load errors, and direct asset/scene observations. |

Game-specific systems remain necessary to implement a game. The deletion requirement covers custom rendering infrastructure and facilities that Bevy already supplies. It does not authorize deleting Origins movement, simulation, animation events, physics policy, or gameplay code because those are ECS systems.

## 7. Built-in rendering only

Use Bevy's standard mesh rendering, culling, transforms, visibility, lighting, shadows, materials, skinning, sprites, and UI. Keep ordinary Vulkan raster rendering initially, with normal Bevy device features; remove Solari's required GPU features and hardware eligibility checks. Do not add another backend abstraction.

Use imported ordinary PBR materials for lit 3D objects. Use unlit materials for suitable 3D cues and Bevy sprites/UI for their existing roles. Do not force HUD or every effect through StandardMaterial. Preserve directly supported alpha blending/masking, sidedness, color, normal, metallic/roughness, occlusion, and emissive channels. External format conversion may expose channels; this repository does not recreate native material evaluation.

Use built-in camera ordering and render layers for first-person weapon visibility. Retain source FOV, ADS alignment, attachment transforms, and viewport behavior required for gameplay. Use normal Bevy lighting and basic fog; no custom sky, atmosphere, reflection, shadow, exposure, or grading system. Start from Bevy defaults for render settings unless a concrete gameplay visibility requirement needs configuration.

Unsupported layered, refractive, procedural, or view-dependent source effects become static source-texture appearances or simple ordinary material/entity animation. Preserve event timing, readable hazards, muzzle flashes, impacts, pickups, and interaction cues. Document each material class that changes visibly. No baking or shader emulation is an acceptable fallback.

Per-instance material changes must not mutate a shared material used by unrelated entities. Create an instance material only for effects that actually need independent state; use Bevy handle ownership for cleanup.

## 8. Collision, animation, and scene preservation

Native collision is a separate asset from the visible map. Do not generate it from the visible world and claim equivalence. Preserve solid, player/AI, projectile, trigger, material/surface, and movable-brush semantics required by the existing game.

Use Avian colliders from exported collision meshes and existing game collision layers. Bind movable collision to its owning doors, barriers, vehicles, or other entities. Static/dynamic ownership must not be lost or duplicated. Keep native collision policy in gameplay/physics code; do not rebuild an extraction stage there.

Retain the source meaning needed by existing brush traces and contacts until an equivalent existing Avian query is demonstrated. Do not silently replace volume tests, start-inside behavior, projectile masks, or movement clipping with a visual triangle mesh. If an external exporter drops required flags or brush data, the export route fails acceptance.

Use exported glTF skinning and Bevy clip playback. Preserve clip names, durations, skeleton bindings, animation notifies, weapon-action timing, and source-driven root motion. Do not implement native animation decoding, a second interpolation engine, or a generic retargeter. Verify character and weapon skeletons with the actual assets.

Normalize units and axis conventions through established exporter/importer settings. A single scene-root transform may express a consistent unit/axis conversion; all collision, placements, clips, and gameplay coordinates must agree. Do not scatter corrective transforms throughout gameplay.

## 9. Loading and session lifecycle

Remove `PreparingShaders` and every shader/material/pipeline readiness resource. Replace that state with `StartingSession` solely because current session setup is asynchronous and ordered. It owns gameplay/scene initialization, never GPU readiness.

```text
Loading -> MainMenu -> StartingSession -> Playing
Playing <-> Paused
Playing -> GameOver / Ending
Restart -> teardown -> StartingSession -> Playing
Required asset or scene error -> LoadError
```

Preserve the current Initialize -> World -> Player ordering. StartingSession completes when required recursive asset dependencies are loaded, scene instances exist, and required colliders, skeletons, and game bindings are attached. It must not wait on frames rendered, shader compilation counters, native material admission, or arbitrary warmup timers.

Readiness covers required content for the session and any subsequently spawned actor/weapon. Spawn only when that object's dependencies and bindings are ready; missing required content is an error. Do not mark readiness by merely counting requests or allocating handles.

Session entities retain ownership tags. Restart removes the owned hierarchy, scene instances, transient materials/effects, event queues, and gameplay state through the existing teardown path. Persistent asset handles may be reused. Repeated restarts must not accumulate cameras, lights, actors, audio, or materials.

## 10. Deletion inventory

Delete after direct consumers have migrated:

- `tools/build_assets.rs` and the entire `tools/asset_builder/` tree: native loaders, fastfile/IPAK readers, animation decoding, texture conversion, material baking, scene generation, and pack production.
- `src/pack.rs`, `src/assets/reader.rs`, `src/assets/prepared.rs`, pack discovery, pack schemas, pack checks, and prepared-art binding contracts.
- Custom rendering submodules, WGSL/WESL and extensions, Solari, ray-tracing proxies, custom LOD/preprocessing/residency systems, native material preparation, custom lighting/shadows/sky/atmosphere/grading/postprocessing, and custom GPU readback machinery.
- Custom FX renderer extensions, including superflare rendering. Preserve cue semantics through ordinary presentation entities.
- Graphics-only remote endpoints, pipeline readiness and renderer-inspection scenarios, flags, scripts, tests, and stale renderer/performance assertions.
- The asset-builder Cargo binary/feature and exclusively unused dependencies. Remove Solari/DLSS features, direct GPU dependencies if no legitimate consumer remains, and DLSS SDK/runtime copying from `build.rs`; retain Windows manifest behavior.

Do not keep wrappers, aliases, disabled plugins, legacy feature flags, fallback loaders, abandoned tests, or a dormant alternate path. Do not retain custom GPU systems solely because their removal would require updating a verification caller.

Keep narrow gameplay/scene diagnostics, screenshots, and useful existing scenarios. Their receipts must describe observed behavior rather than hard-code a renderer identity. Preserve profiling support only where it still serves ordinary application diagnostics.

## 11. Implementation sequence and gates

1. Prove external export of native world/collision and their required semantics on `zm_tomb`, then placements, one actor, one animated first-person weapon, textures, audio, and required game definitions. Check patch/companion dependencies. Tool capability is established by usable output, not a README or zero exit code.
2. Record the working tool revisions, commands, settings, and outputs in this specification. Pin external tools under ignored `target/tools`; do not vendor their code.
3. Migrate filesystem asset loading, direct bindings, and session startup together. Preserve the game definition and error behavior. Do not use the old pack to conceal missing exports.
4. Replace graphics with stock Bevy and move necessary game presentation changes to their owners. Delete old renderer callers as they are migrated.
5. Delete builder/pack infrastructure, unused dependencies, and graphics-only tooling. No dead compatibility layer remains at completion.
6. Build and run focused verification, then full-map and gameplay acceptance. Commit coherent changes locally on the isolated branch. Do not merge, cherry-pick to the original checkout, or push.

If an external capability cannot preserve required content, stop the affected migration and report the precise gap. Continue only independently useful authorized work. Do not create an exporter, patch a tool, reduce the map, invent data, or represent the infrastructure deletion as complete.

## 12. Acceptance evidence

- **Architecture:** no custom renderer, native decoder, bake pipeline, pack reader/writer, prepared-material framework, or replacement asset builder remains. New code is limited to game-specific setup/bindings and existing domain behavior.
- **Build:** `cargo build --features dev-tools --locked` succeeds with the isolated Cargo environment. Surviving lockfile versions, profiles, static linking, and linker settings are unchanged.
- **Assets:** fresh external exports cover full world geometry, authored placements, native collision including dynamic ownership and masks, skins/clips, textures, audio, and required gameplay data. No previous pack or copied dump is used.
- **Loading:** correct absolute asset root works from a different working directory; missing roots, images, clips, or required scene nodes give named errors. No shader warmup state remains.
- **Visual:** representative full-map views show world/props/actors, first-person weapons, animation, HUD, transparency, and readable effects. Record visual approximations without treating them as missing assets.
- **Gameplay:** normal inputs and existing scenarios exercise movement, collision boundaries, firing, damage, interactions, death, and repeated restart. Include relevant bullet/player/AI collision differences and moving barriers/doors.
- **Ownership:** inspect all checkout game/capture processes before launch; bind tools to the isolated process and port. Scenario-owned processes shut down in `finally`; other sessions are untouched.
- **Honesty:** compiler, export, visual, and gameplay results are separate. No FPS improvement claim without a measurement. Unfinished gameplay remains visible as unfinished.

## 13. Current implementation status

The isolated worktree and branch exist. OAT v0.33.0 is downloaded and its world/collision limitation has been reproduced. Husky/C2M source was inspected. The unmodified open-fastfile candidate and its isolated Python dependencies are present under `target/tools`; its Origins collision export has not been executed. No game code has been changed, no infrastructure has been deleted, and no game build or runtime acceptance has run.

The next action after this specification is reviewed is to test that candidate's existing collision-only export, without enabling its baking or modifying its implementation.
