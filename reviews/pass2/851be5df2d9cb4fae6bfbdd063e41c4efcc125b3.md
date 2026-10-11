# Pass 2 — 851be5df2d9cb4fae6bfbdd063e41c4efcc125b3

Original: **Draft Crazy Place barrier scheduling and native mesh collision**. Corresponding pass 1 commit: `514f3225b813c7321cef12f7b4045f9aa8543aea`.

## Scope and evidence

Reviewed all changes to `Cargo.toml`, `src/{chamber,navigation,lib}.rs` and `todo.md`, scene readiness, collider construction, navigation admission and quest-stage producers. Locations are at this original commit. Future status ends at frozen HEAD `5f4673a49c2a9c04437ff5c433468971a97ab15b`; dirty work was excluded.

`GSC/` means research-root-relative `source/server-tomb/maps/mp/`. `research:` and `inspection:` refer to their respective evidence roots. Read research README, identity/provenance, verification/query receipts and unresolved information. Packed SHA-256: `f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e`; unpacked: `ebf8b363455c22ed25a323a3ba0a45318cc0dd09769d4ec0ba3fbbac7d01251c`. Existing C, bindings, references/instructions and read-only SQLite were used. Runtime pack `assets/origins/game.json` has frozen Git blob `29747327f454e9ad66212b355f3914bedcb03e65`. The committed divider GLTF was read only after matching its LFS SHA-256 `a58e75997ab14dfbd35243dcc1ea587037bfc86d41030e0ed2555b044cbe9b9d`; no other checkout's assets were substituted.

## Pass 1 disposition

All three prior findings are **confirmed** as P1-1, P2-2 and P3-3; none is repaired through frozen HEAD. Pass 1's downstream missing `Finale -> Complete` producer is **resolved later** by `7df83438a3215c3864370b84830c26873b4753ac`; that does not fix the findings. Startup/polling differences remain disclosed approximations rather than demonstrated exploits.

## Findings

### P1-1 — Collider construction is consumed before scene meshes arrive

Trigger: normal setup of a fresh divider. Draft: `src/chamber.rs:33-36` attaches `WorldAssetRoot` and `ColliderConstructorHierarchy` to a root without descendants. `Cargo.toml:35` selects Avian mesh support but omits its `bevy_scene` feature. Pinned Avian revision `34c9b008d2f40638a953d9218db48d54f661365d`, `src/collision/collider/backend.rs:238-244,329-354,440-442`, runs construction in Update, waits for scene readiness only with that feature, and removes the request after inspecting current descendants. Its `crates/avian3d/Cargo.toml:45-46` does not enable the scene feature through collider-from-mesh. Pinned Bevy 0.20.0 `bevy_app/src/main_schedule.rs:224-230` and `bevy_world_serialization/src/lib.rs:64-68` spawn world instances later in SpawnScene. `assets::scene_ready` services MapRoot, not these dividers. Thus the request is gone when meshes arrive.

Source: `GSC/zm_tomb_chamber.gsc:164-177` moves the wall itself and changes its paths; original placements are `inspection:game-prepare/entities/maps/mp/zm_tomb.d3dbsp.ents:482-523`. Identity-matched `assets/origins/library/models/zone/all/zm_tomb/f9817b0fba952a9788ff05e2/p6_zm_tm_elm_divider_lod0.gltf` retains the relevant mesh descendants. Impact: moving visual walls have no generated solid collision and do not provide their arena division. Correction: enable the pinned Avian readiness integration or add the constructor from the actual world-instance-ready observer, then gate wall readiness on collider construction. This uses ordinary ECS integration. Confidence: high from exact features/schedules; runtime traversal was not performed. Current at frozen HEAD.

### P2-2 — Transient actor occupancy becomes persistent wall connectivity

Trigger: a player/actor occupies a node or crosses a link during refresh. Draft: `src/navigation.rs:96-111,256-257`, called at `src/chamber.rs:62-64`, uses `solids([])` and inside-solid rays without excluding actors, then caches obstruction in `chamber_blocked`. The player capsule (`src/player.rs:110`) can contain the node-ray origin. Initial admission (`src/navigation.rs:495-513`) already excludes actors and temporary blockers. Pinned Avian `src/spatial_query/system_param.rs:88` confirms inside-origin solid-ray behavior.

Source: `GSC/zm_tomb_chamber.gsc:164-177` changes connectivity on the wall entity. Native bindings `research:evidence/native-bindings.jsonl:1618-1619` identify disconnectpaths `00627cc0` and connectpaths `00627d40`; their C exports call `00822320`/`008221b0` on the resolved entity. `research:exports/functions/0082/00822320.c:38-99` scopes link testing to it; `008221b0.c:16-50` releases entity-owned links/refcounts. This supports wall-owned connectivity rather than a snapshot of all solids. Impact: open routes stay rejected after the player moves, until a later wall refresh, potentially longer when the chamber becomes empty. Correction: filter to moved divider colliders, or consistently exclude actors/temporary blockers and derive connectivity from stable wall geometry/state. Confidence: high; independent of P1-1 because player colliders alone populate the cache. Current at frozen HEAD.

### P3-3 — Landing has no impact cue

Trigger: a wall completes descent. Draft: `src/chamber.rs:54-60` emits a selection sound and updates elevation without a landing transition. Source: `GSC/zm_tomb_chamber.gsc:80-86,171-177` rumbles chamber players for .1 seconds at movedone and emits nine sequenced dust tags; `GSC/zm_tomb_fx.gsc:45` resolves `maps/zombie_tomb/fx_tomb_chamber_walls_impact`. The matching committed GLTF retains `tag_fx_dust_01` through `_09`. Native moveto `0063d1b0` (`research:exports/functions/0063/0063d1b0.c:31-38`) creates a timed trajectory via `0063c5f0`, corroborating the movement-completion boundary. Impact: the layout-settled impact cue is absent. Correction: emit one movement-completed message per descent and run the recovered dust/rumble sequence through presentation systems. Confidence: high; current at frozen HEAD.

## Coverage and unresolved limitations

Confirmed placements/yaws, 1,000-unit lift, round-10..30 interval clamp, nonrepeat selection and permanent crystal latch. Already-raised startup walls, continuous occupancy polling and a 1.1-second connectivity delay differ from source startup lift/one-second poll/movedone transition; ordinary-play consequences were not established. No static-divider duplicate is asserted for this pack. Finale now raises the active wall, but source hiding of raised visuals remains unfinished presentation. Native receipts disclose inferred/custom-return prototypes, unresolved indirect flow and four assembly fallbacks; these were not silently filled in. Child collider transforms under actual moving world instances and rendered landing cues remain unexercised. No tests, build, game, media processing or analysis pipeline was run; static checks do not establish runtime acceptance.
