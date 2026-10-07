TASK:
Fix forgotten surface vertex-layout errors and verification error reporting; validate current main's combined assembly/ADS/audio/GPU integration and land before the separate PBR overhaul.

STATUS:
COMPLETE implementation and local main integration. Source commit 082889d was fast-forwarded into clean main from 637e9f2. Acceptance is PARTIAL because the user explicitly stopped further testing and requested immediate integration. No push.

FINDINGS:
The earlier audio source restart logged seven Vertex_Uv_1 material-specialization errors. OpaqueSurface requested optional attributes unconditionally, including during late scene/upload transitions. PipelineCache inspection misses these failures because specialization rejects the descriptor before cache insertion. Required position, normal, primary UV and tangent inputs still reject incompatible layouts.

EVIDENCE:
inspection/handoff-render-fixes-20261007 retains the isolated package, assets.json and serial window logs/captures. The validated combined closure contains 2,898 files, 1,478 PNGs and 1,282,529,758 bytes. Source/package hashes match; index integrity confirms 2,897 LFS pointers and ordinary Git game.json. Runtime assets and metadata are unchanged. Tested executable SHA256: d63a09c8c7269deb4451ad52fea175f68d113ca1ee414c294fd43d306a4b257e.

CHANGES:
Surface specialization requests optional inputs only when present in the uploaded layout. Vertex defaults match CPU preparation: zero secondary UV, white color, primary UV for layers, identity normal rotations and lighting index zero. Bevy intermediate/output structs and fragment locations remain stable; shadow/depth prepasses remain engine-owned. Verification captures bevy_pbr error events in a LogPlugin layer initialized before engine registration, retains the same failure resource during plugin installation, writes target/error receipts and exits unsuccessfully. Pending shader imports remain recoverable.

VALIDATION:
Locked Cargo metadata, isolated workspace release build, formatting, diff whitespace, complete import closure, copied-package hashes and index/LFS gates passed. Six serial actual-window workloads exited successfully with no rendering errors or overlapping processes:

- Normal combat: six kills, Round 2, 15 survey views, water captures and restart.
- Audio sources: 1,549 frames, zero source/volume/backend errors, pause/settings/resume and owner cleanup.
- Wall weapons: M14/MP40/Remington/B23R purchases, firing/reloading/switching and burst cancellation; 62 sampled animation clocks.
- ADS: all 120 scenarios across five weapons at 16:9 and 4:3; projected optical alignment and scenario receipts passed.
- Frag: four bounces, cooking, enemy/self impacts, Round 2 and restart.
- Semtex: surface/actor attachment, moving attached grenade, two-second fuse and unchanged primary inventory.

Representative normal/survey, MP40 attachment, Remington pump and ADS captures were inspected without missing draws. The Monkey run was stopped after 93.4 seconds at the user's request: exit -1, no logged errors, no acceptance claim. Claymore and perk-bottle/solo-revival checks were skipped. The first asset wrapper used a relative path for check_index after successful import; corrected and rechecked source/package hashes and all index/LFS objects. No unit tests, fixtures or Clippy. Every launch waited for the shared game slot. Only this chat's executable was stopped; the runner then exited.

UNRESOLVED:
Monkey acceptance, Claymore and perk-bottle/solo-revival checks remain incomplete under the user's validation stop. No reproducible merge regression appeared in the completed checks. Full catalog manual review remains unqualified; four staff melee world proxies retain their explicit incompatible-crystal exception, and eighteen native auxiliary sound aliases remain unavailable. Exact native evaluator behavior and documented gameplay/visual approximations remain unqualified. Optional-input fallback variants were not observed in the successful logs; no malformed assets or fault fixtures were introduced. Historical combat slowdown is outside this batch.

RECOMMENDED NEXT WORK:
PBR INTEGRATION CONTRACT: The active Plan PBR texture improvements chat in worktree 007b starts at 8e585b9. Merge updated main, including 082889d and this handoff's commit, before its final qualification; preserve uploaded-layout input checks, neutral vertex defaults, stable shader outputs and the shared specialization-error detector. Retain main's catalog assembly, native ADS, audio, GPU preprocessing and launch drivers. Adapt its PBR shaders/materials, regenerate its combined pack and rerun affected acceptance. Do not restore old snapshots or replace this handoff with stale pre-integration results. Resume the skipped checks only when testing is authorized again. No unfinished PBR changes were imported or altered.

SCOPE NOT IMPLEMENTED:
PBR semantics/schema migration, texture cooking/compression/tuning, performance investigation, missing gameplay, original-game equivalence or sustained manual survival. No new assets, dependencies, CLI options or bind-group changes.
