# Current handoff

TASK:
Fix the reported blue-gray surface wash and thin-wall light leaking in an isolated worktree; integrate the completed fix into local main.

STATUS: PARTIAL

FINDINGS:
Bevy's finite-depth atmospheric aerial LUT assumes unobstructed planetary transport; it added sky-colored haze over enclosed geometry alongside the existing shadowed fog volume. Constant volumetric ambient scatter and cool shadow grading reinforced that wash. GI normalized a five-percent blocked-probe floor into visible illumination, mixed coarse probes into fine interiors and biased receivers up to twenty centimeters across thin walls. Reflection remainder could bypass scene visibility through the global sky map. These are separate transport/compositing paths, rather than damaged albedo textures.

The replacement neutral 80-unit diffuse fill preserves material color and native BRDF/AO, independently of sun visibility or probe coverage. It excludes ambient specular; the current traced scene cube supplies reflections. This is a deliberate artistic readability approximation, not recovered native arithmetic or Unreal/Lumen equivalence.

EVIDENCE:
External root: E:/Coding/Captures/origins-gray-leaks-20261007/. before-quick/quick-revive-machine.png reproduces the reported blue-gray machine and bright wall seam. before-room/ and before-indirect/ isolate the baseline room. The first after-quick/ candidate retained the veil and was rejected; only its owned process was stopped. quick-reflection/ used a stale executable after a file-in-use copy failure and failed launch parsing; quick-reflection-02/ and quick-indirect/ passed the corrected diagnostic launch. Their black reflection/indirect views isolated the remaining wash to the atmosphere path.

neutral-quick/map-17.png shows visible painted metal, white panel/blood, wood grain and warm lantern light without the former right-wall strip. integrated-gameplay/ qualifies the combined controls/renderer executable: fifteen survey views, ADS/melee/actor frames, Round 2 and restart. Captures were visually inspected across the machine, spawn, interiors, church, Crazy Place and water. Deep enclosed areas remain deliberately dark; authored blue emission and genuine outdoor sky illumination remain colored.

CHANGES:
Finite scene rays use Bevy's shadowed height fog alone, retaining the physical sky and sky transport input. Fog/shadow tint is neutral and constant volume ambient scatter is zero. Native ambient shading provides diffuse-only readability fill. GI separates initialized support from visible confidence, assigns the finest supported cascade its interior and blends coarse fields at the boundary. Receiver bias is capped at two centimeters; nearer visibility moments are admitted immediately. Lantern shadow offsets are reduced for thin geometry. Native reflection remainder uses the scene cube, retaining GGX filtering and local parallax correction. The existing actual-game gallery adds the source-placement Quick Revive viewpoint 17; material debug accepts all existing views through 17.

Work proceeded sequentially without agents. The isolated codex/fix-gray-light-leaks branch was advanced onto main's controls commit 27296f493; both launch/gallery integrations merged cleanly. Runtime assets and originals are unchanged.

VALIDATION:
Complete assembly/import and staged LFS closure passed: 4,139 files / 3,301,685,797 bytes, including 4,137 LFS payloads and two ordinary Git files (asset-closure.json). Initial worktree map.gltf was an unresolved LFS pointer; local LFS checkout repaired it before assembly. Locked metadata, full release build and format check passed on the combined source; release compilation took 60 seconds. The neutral Quick Revive driver passed with empty stderr, zero streaming errors and no pending detail reads. Combined ordinary gameplay passed in 154.59 seconds: movement, combat, Round 2, fifteen views and clean restart, empty stderr, zero streaming errors/no pending detail reads and 5,168 DDGI/reflection updates. This run did not qualify player damage/healing. The rotating-camera driver passed in 84.09 seconds with twelve rotation captures and clean restart; inspected views retain local shadows/material detail without the reported indoor veil. Post-merge qualification remains pending. No unit tests, suites, fixtures or Clippy.

UNRESOLVED:
Sustained/manual survival, exhaustive whole-map visual acceptance, exact original equivalence and controlled renderer performance comparison remain unqualified. The broader TODO goal remains partial: 478 complete / 770 remaining. Main's 26 controls/menu items passed their own 33-case driver, but their release comparison remains partial after failed/interfered cohorts. The 58 native weapon/powerup/Mystery Box tasks remain unchecked in the preserved external worktrees; their source corrections and incomplete drafts remain available in Git/history and E:/Coding/Worktrees/.

RECOMMENDED NEXT WORK:
Finish local-main integration and post-merge qualification for this fix, then continue the native weapon prerequisites and powerup/box milestone sequentially. Retry the guarded controls release comparison with compiler/window resources free before claiming performance acceptance.

SCOPE NOT IMPLEMENTED:
No texture edits/upscales, exposure increase, resolution reduction, image blur, temporal reconstruction, new renderer engine, baking/path tracing or new gameplay systems. The neutral floor is artistic fill rather than additional traced multi-bounce energy. No push, replacement toolchain, delegation or original-input modification.
