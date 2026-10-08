# Current handoff

TASK: Fix viewmodel texture streaming and self-shadowing, bound streaming to 75% of detected GPU VRAM, and finish selected complete 4x material sets using existing tools.

STATUS: PARTIAL

FINDINGS: Foreground demand, stable-image GPU binding invalidation, fair producer scheduling, native shadow admission, posed bounds, screen coverage and replacement/retirement accounting are implemented. The native extended-material bindless lookup requires descending range descriptors in the pinned Bevy release. All transferred texture preparation work is retained.

EVIDENCE: External evidence lives in E:/Coding/Captures/origins-viewmodel-streaming-20261007/. Final-source executable D638BF806C40128AC398409C95E6D376E644697BE8196D8D42300E0FADE1A6AA passed five streaming cases, eight native shadow admissions, and ordinary combat/survey/restart on the prior partial pack. Detected VRAM is 17,171,480,576 bytes; the cap is 12,878,610,432 bytes. The selected four-set cook is still running.

CHANGES: All renderer/presentation/verification changes and transferred offline tools are ready for main. prepare_current_pbr.py reuses DeepBump, semantic mips, BCn cache and importer for gloves, wrist device, Mauser and bullet sets, including both Mauser identities. Selected response maps are reauthored, not preserved. Remaining Origins textures are one broad todo.md item with the measured approximately nine-hour current-map inference estimate plus encoding/import.

VALIDATION: Locked metadata, release build and format checks pass. The release hash matches the qualified executable. Changed Python syntax passes. The first selected cook failed because soundfile was missing from the existing environment; that dependency was installed with existing uv. Its retry rejected BC7 response mip 11 at RMS 0.041007 against 0.025; the final cook explicitly uses lossless response maps. Other selected roles retain BC7/BC5/BC4. Final selected-pack closure/import/window checks remain pending. No unit tests, fixtures, Clippy or push.

UNRESOLVED: The running selected-material cook must finish, pass closure, enter assets/origins and receive a focused native-window check and asset commit. Do not restart bulk upscaling.

RECOMMENDED NEXT WORK: Import and commit the four complete sets directly to main immediately after the running cook completes. Preserve every change from both main and the viewmodel worktree during integration.

SCOPE NOT IMPLEMENTED: Remaining full-pack material/density migration, future Origins assets, original pixel equivalence, causal performance claims and full-pack pressure qualification are deferred.
