# Current handoff

TASK:
Fix inverted roof rubble normals and PCSS striations on main; integrate the existing task's native FPS overlay, physical 1080p startup and ordinary-permission Windows manifest in one local commit.

STATUS: COMPLETE

FINDINGS:
The reported top-right chunk is mc/zm_tm_trim_concrete_broken, not the nearby tile-macro pillar. Its four shared providers had malformed surface frames: one visible triangle's averaged vertex normal opposed its geometric normal with dot -0.809, and mesh 2811 reached -0.986. The offline repair preserves smoothing/UV splits and reconstructs angle-weighted geometry normals plus UV tangents. All four repaired providers now have zero opposed averaged normals; minimum alignment is 0.780. Only NORMAL/TANGENT byte ranges changed in map.bin. Shape, indices, UVs, textures, placements and collision geometry are preserved. This is an explicit surface-frame reconstruction, not recovered native arithmetic.

PCSS fed continuous light-plane coordinates to screen-pixel IGN, producing correlated stripes. Nearest blocker depths, deterministic center-inclusive admission, hardware-PCF fallback on a search miss, bounded receiver-plane compensation and stable integer hashing remove the reported bands without temporal history or image blur. The final sampled arm shadow retains its shape. Version-two material semantics now authoritatively set unlit/emission state, and raster/ray paths distinguish stored RGB normal Z from reconstructed two-channel Z. Backface orientation is applied once to the complete mapped normal; clearcoat normals are normalized. Prepared tile response order follows core/coating color, independently of macro/coating/core normal bindings.

EVIDENCE:
External root: E:/Coding/Captures/origins-striations-20261007/. roof-hit-receipt.json identifies the material/triangles. roof-frame-repair.json and roof-frame-qualification.json record the four provider repairs and prove unchanged geometry outside frame ranges. geometry, identity, emission, shadow and indirect captures isolate the contributing terms. repaired-lit/map-8.png shows the corrected dark underside of the top-right chunk and arms without the original vertical bands; repaired-normal/map-8.png shows its corrected mapped orientation. The earlier lit/final-lit/accepted-lit candidates were rejected as insufficient; their passing driver receipts did not establish visual acceptance.

Existing task evidence: E:/Coding/Captures/origins-spawn-20261007-1151/. application.manifest confirms asInvoker/uiAccess=false. first-person/map-8.png and spawn-1080p.png show spawn/hands and the native overlay. spawn-30s.mp4 is a thirty-second OBS 1920x1080/60fps encoding; game FPS was approximately 10-16, and stationary gameplay ended in death. Manual shots/melee/held-ADS/movement coverage remains partial.

CHANGES:
Renderer/shadow adapters and diagnostic modes 7-11; qualified material recipe and reusable offline frame preparation; validated map.gltf/map.bin in LFS. Existing task's build.rs, src/lib.rs, src/hud.rs and src/hud/performance.rs are included. F12 toggles the bounded five-second AVG/P95 overlay at four refreshes/second, preserving F10 captions and F11 material diagnostics. Work was sequential here; no subagents. Combined local integration on main includes both tasks; no push.

VALIDATION:
Locked metadata, release build and format check passed; final release build completed in 57.55 seconds. External assembly/import passed the complete 4,139-file closure, 3,301,685,797 bytes. Eight focused actual-window diagnostic/lit drivers passed with empty stderr; final lit streaming had zero errors and no pending detail reads, with live DDGI/reflection updates. Final paused spawn measured 41.38 FPS, P95 38.49 ms over eight sampled seconds; this is not a controlled performance A/B. The moving-camera run also passed with twelve rotation captures, empty stderr, zero streaming errors, 3,113 DDGI/reflection updates and 47.26 FPS over 56 sampled seconds. Final staged LFS/index closure passed: 4,139 files, 4,137 LFS pointers and two ordinary Git files (index-closure.json). The user accepted the final appearance and authorized the combined commit. No unit tests, fixtures, suites or Clippy.

UNRESOLVED:
Sustained gameplay and whole-map visual acceptance, controlled overlay/filter overhead A/B, and actual F12 toggle qualification. Original pixel equivalence is not claimed. Earlier renderer generator/outdoor/combat/RenderDoc and complete upscale acceptance remain deferred.

RECOMMENDED NEXT WORK:
Future whole-map and sustained gameplay validation should use the retained material diagnostics and matched camera/exposure evidence. Qualify the F12 toggle and controlled overhead when extending the performance overlay.

SCOPE NOT IMPLEMENTED:
No exposure/floor rebalance, texture edits/upscales, temporal filtering, shadow-resolution reduction, GPU timing/profiler panel, or new combat video. Originals remain untouched. No push.
