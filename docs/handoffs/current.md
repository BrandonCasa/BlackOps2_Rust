# Current handoff

TASK:
Finish the accepted neutral-lighting correction by removing systemic sunlight leaks through ceiling, pipe and wall/door blockers; commit the worktree result into local main.

STATUS: COMPLETE

FINDINGS:
The previous blue-gray wash fix is accepted and remains integrated at dea0fd952/2ea7ec645. Actual runtime triangle traces find roof, pipe and wall blockers above the leaking receivers, including a room-facing wall sheet 4.57 cm away. Hardware ray visibility already tests both faces. Sun-only and shadow-mask captures isolate the remaining patches to raster directional shadows. Disabling reuse restores those blockers with the same GPU renderer; changing GPU batching alone does not. Directional reuse retains incorrect occlusion in camera-owned cascades. Pending-caster, full-requeue, pass-acknowledgment and prepared-caster-signature candidates did not remove the leak and were discarded. The final implementation uses Bevy's current-frame cascade passes while retaining lantern reuse.

EVIDENCE:
E:/Coding/Captures/origins-systemic-light-leaks-20261007/: before-sun/, before-shadow/ and blocker-triangle-receipt.json establish the original path and existing blockers. two-sided-shadow/ and two-sided-sun/ retain the leak. cpu-uncached-lit/ and gpu-uncached-lit/ restore occlusion; direct-gpu-lit/ retains it. pending-gate-shadow/, rebuild-shadow/, rebuild-lit/, pass-ack-shadow/ and native-casters-shadow/ are rejected cache candidates. live-sun-shadow/map-8.png removes the wall/header rectangles and pipe-interior patch. final-room-hands/map-8.png retains warm lanterns, readable materials and the accepted neutral fill. Offline traces use actual runtime triangles, omit collision-scene duplicates, and do not evaluate masked texture alpha; they are research receipts, not replacement game geometry or proof of all masked coverage.

CHANGES:
OpaqueSurface specializes native sun/point shadow passes with culling disabled, separately from authored visible and camera-depth culling. Existing alpha discard remains intact. Sun receiver depth bias is 0.002 m and normal bias is 0.25 texel. Camera-owned directional cascades execute each frame instead of suspending their views/phases; unchanged lantern faces retain the established cache. Physical sky, neutral diffuse fill, shadowed fog, GI, scene reflections, exposure and original asset bytes retain the accepted first-stage behavior. No speculative cache guards remain.

VALIDATION:
Final source release builds passed, including the final locked workspace gate (71 seconds). Locked metadata, format and diff checks passed; tracked Markdown remains 20 files. Focused shadow mask passed in 32.49 seconds and lit room/hands passed in 32.19 seconds, both with empty stderr, zero streaming errors/no pending detail reads. The latter reports 5,424 native directional cascade passes, zero directional reuse and 866,291 reused lantern faces. Focused window-tested packaged executable SHA256: 4ad339993073d58ecc07eee5b492ee6e9ff1cda3f95de7c27ffb2d2697be27a0. Complete asset/staged LFS/package closure passed: 4,139 files, 3,301,685,797 bytes, 4,137 LFS pointers/two ordinary Git files, unchanged source hashes and matching executable-relative pack. The final rebuild SHA256 is c579c71b028600a057b53300490848750cdc9033057320fb5ba9623bb8962865; the hash check stopped before launching further gameplay. The user explicitly stopped validation and requested immediate commit to main. Full ordinary gameplay/survey/restart, moving-camera qualification and post-integration window were skipped for this final sunlight change. The focused window evidence belongs to the earlier packaged build of the final source.

Initial candidate compilation failed because Bevy 0.19.1 has no MeshPipelineKey::HDR; native light-projection keys replaced that assumption. The discarded prepared-caster experiment also required correcting a DefaultHasher import and mesh_asset_id method calls. The cpu-uncached-shadow launch failed because material-debug does not accept benchmark overrides; the supported benchmark-view route was used for the comparison. No unit tests, suites, fixtures or Clippy.

UNRESOLVED:
Exhaustive whole-map visual acceptance, sustained/manual survival, exact original equivalence and controlled performance comparison remain unqualified. Directional cache reintroduction requires native camera/visibility/GPU lifecycle qualification; correctness currently takes precedence. Local main receives the isolated sunlight correction on top of 2ea7ec645, with no further validation at the user's request. The broader TODO remains partial at 478 complete/770 remaining: controls/menu behavior is qualified, its performance comparison is partial after failed/interfered cohorts, and weapon/powerup/Mystery Box drafts remain in preserved external worktrees.

RECOMMENDED NEXT WORK:
Stop after committing this correction into local main, as explicitly requested. Any later validation or broader native weapon/powerup/box work requires a new instruction.

SCOPE NOT IMPLEMENTED:
No geometry inflation, texture changes/upscales, exposure increase, resolution reduction, blur, temporal reconstruction, new renderer, baked lighting or gameplay changes. The neutral floor remains an artistic readability approximation. No original modification, replacement toolchain, delegation or push.
