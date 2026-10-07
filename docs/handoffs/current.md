TASK:
Implement Bevy GPU mesh preprocessing and integrate the reviewed local commit into main; record the shared validation-slot rule in AGENTS.md.

STATUS:
COMPLETE: GPU preprocessing is implemented and integrated into local main at 5e7c6af by fast-forward from 08ef6f1. No intervening main changes or conflicts existed. Compiler/asset gates, serial CPU/GPU room checks and the longer-budget combat/restart workload passed. No push.

FINDINGS:
The application disabled both PBR GPU instance building and GPU phase batching. Enable compute expansion of compact mesh inputs by default, retaining direct batching and Bevy visibility. Device capability limits preserve CPU fallback. Dirty shadow-phase revival must follow Bevy's device/view mode. GPU indirect/frustum culling remains an explicit benchmark comparison; no temporal occlusion culling is enabled. Focus-dependent Winit reactive updates distorted early throughput comparisons; final benchmarks use continuous updates independently of VSync. Earlier 1080p CPU figures are historical.

EVIDENCE:
External gpu-preprocessing-20261006 contains assets.json, final-compute-room/final-cpu-room receipts/captures, final-cpu-gpu-image.json and final-game/failure.json, plus final-combat game/performance/audio receipts and captures. Final executable SHA256: E0DC6B00494D142AEE6846822E7563CF36F85C294877340E3E2AD3600CF7397E. Both final room captures are 1280x720 and identical across all 921,600 pixels. GPU receipts identify gpu_preprocessing and nonzero Bevy early_mesh_preprocessing compute timing; CPU receipts identify cpu. The serial continuous-update pair measured GPU 32.4 FPS and CPU 24.5 FPS, with substantial frame-time variation; this is not a general speedup or original-equivalence claim.

CHANGES:
Enable Bevy GPU instance building; add typed preprocessing selection and shared active-mode reporting. Keep restored shadow phases consistent with engine batching. Add mutually exclusive --cpu-preprocessing/--gpu-culling benchmark options, combinable with --uncached-shadows. Use continuous benchmark window updates. Update rendering/project state and require one game-validation slot across all chats/worktrees in AGENTS.md. No runtime pack, visual calibration, gameplay, shader, resolution or antialiasing changes.

VALIDATION:
Locked metadata, release build and formatting gates passed; release builds were repeated after preprocessing/window-policy refinements, with no compiler errors. Complete asset closure passed: 2,597 files, 1,140,678,549 bytes, 1,461 PNGs. Initial unresolved LFS pointers failed asset validation; local git lfs checkout restored them and the affected gate then passed. Final serial CPU/GPU room windows exited zero and passed scene/restart receipts, with pixel-identical captures and no pipeline errors. Native --verify completed six kills, Round 2, animated attack/death, ADS and thirteen survey views, then failed the 180-second overall budget at 181 seconds while paused before the final water views/restart. No permanent pipeline error was reported. The sole-window longer-budget --benchmark-combat run then exited zero: six kills, Round 2, movement, ADS, seven animation states, six attached heads, four retired corpses, one room view and clean restart. It recorded 13,581 animated bounds updates, 89,052 rendered cube faces and 1,354,770 reused faces, with no logged warning/error or pipeline failure. Its 66.7-second combat sample averaged 6.8 FPS with substantial stalls; this is a measured limitation, without a current CPU-combat comparator. Early overlapping measurements are excluded; one overlapping compute run was stopped with exit -1. Subsequent launches check the shared slot and wait when occupied. Main integration preserved the validated source and assets exactly; no additional game window or repeated compiler/asset gate was needed for the fast-forward. No unit tests, fixtures, Clippy, replacement toolchain or push.

UNRESOLVED:
Native all-view --verify did not complete within its deadline; the final two water views remain unqualified in this run. The separate longer-budget combat/restart check passed. Combat performance is low (6.8 FPS measured) and needs paired CPU/GPU profiling; historical combat figures are not a current comparator. Unsupported GPU hardware fallback, packaged launch, sustained manual play, GPU-culling performance and a stable repeated performance distribution are unqualified. Existing material/lighting/gameplay approximations remain unchanged. Main was clean at the same base 08ef6f1 when reviewed.

RECOMMENDED NEXT WORK:
Profile low combat frame times with a current CPU comparison and finish the native all-view workload without changing gameplay. Obtain repeatable isolated throughput measurements before claiming a broad performance gain. Continue gameplay work through the shared application.

SCOPE NOT IMPLEMENTED:
GPU clustered lighting, temporal occlusion culling, custom GPU rendering infrastructure, deferred rendering, new assets/gameplay, full original equivalence.
