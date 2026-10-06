TASK:
Review and revise the external PLAN (5).md lighting replacement for completion blockers, pinned Bevy conflicts, performance risks and archival before/after validation.

STATUS:
COMPLETE - plan review and in-place revision only. Lighting implementation/performance improvement remain unimplemented. The full Origins goal remains PARTIAL; the latest committed gameplay milestone records 322 completed tasks and 897 remaining.

FINDINGS:
The current renderer already has baked lighting and cached shadows, so rebaking alone is not an FPS improvement. Stock Bevy lightmaps do not implement directional coefficient banks; shared shadow arrays do not implement per-light/per-cascade resolution tiers or separate static/moving depth. Per-source HDR direct pages and permanent dual shadow sets threaten memory/bandwidth. The revised architecture uses directional indirect coefficients, compact static visibility and Bevy analytic direct light, with fixed-resolution shadow residency proved before full-map baking. Both the main and layer-1 viewmodel cameras need replacement lighting coverage.

EVIDENCE:
Inspected project state, renderer/material/shadow/probe/residency code, actual-window drivers, timing/capture predicates, importer, packaging, LFS policy and installed Bevy 0.19.1 sources. Current map inventory: 18,189 primitive placements, 14,480 static model instances, 331 lamp placements, approximately 10,026,887 placement-expanded visual triangles. Matrix-scale-corrected opaque/cutout triangle area is approximately 563,796 square metres, including distant/unseen/card area. Applying 64 texels/metre and three RGBA16F maps uniformly would require approximately 68.8 GiB with full mips before padding; this is illustrative, not the proposed density allocation. Headless Blender 5.2.2 LTS enumerated the RTX 4080 through OptiX; no bake ran. External plan SHA256: 3D4E01FB667865BE4EE7B6DADCB2B356B36524BE0E3FCBB46132E3DE01868ECB.

CHANGES:
Replaced C:/Users/brand/Downloads/PLAN (5).md in place. Added an issue/resolution inventory, budgets, mixed-shadow integration requirements, material/UV/format/cache/publication rules, camera/probe coverage, source removal audit and sequential milestones. Defined a verified external before screenshot/timing archive before runtime deletion, frozen driver/capture state, real-time/native-release repetitions, same-size pairs/crops/sequences, populated/moving-light/traversal workloads and gameplay acceptance. There is one final lighting implementation with no old/new selector or alternate executable. No game source/assets were changed by this review.

VALIDATION:
Plan content checks passed for balanced fences, archive-before-deletion, single renderer, 15 survey views, raw combat samples, capture completion, directional/mask/shadow contracts, viewmodels, shader context, early cutover, required gate text and explicit unmeasured status. No trailing whitespace; final file hash verified. Read-only geometry inventory completed with the Python standard library. Some public documentation fetches failed; pinned local engine source supplied the concrete contracts. Host Python lacked NumPy; no package/toolchain was installed. A large shell write was blocked and a later patch-context error was corrected using file patches. Cargo gates, full asset validation, game-window runs, bakes and FPS comparisons were skipped because this task changes a plan and handoff only. No unit tests/fixtures, Clippy, delegation or push. Twenty tracked Markdown files.

UNRESOLVED:
Directional bake production/export, final density/overlap allocation, sparse mixed-shadow integration, full-map bake duration/RAM/VRAM, streaming, probe coverage, final quality and gameplay FPS benefit are unmeasured. The checkout changed during review: the latest completed gameplay commit is a9197ae, with further gameplay edits present. Preserve those edits and freeze an identified baseline only when implementation starts. Prior Double Tap/Deadshot qualifications and remaining gameplay limitations belong to project state and Git history; this review did not revalidate them.

RECOMMENDED NEXT WORK:
Follow the revised plan: inventory/offline feasibility, complete and freeze the existing driver, capture and verify the full before archive, delete the old runtime lighting, prove the new room/shadow/camera integration, then complete the map and measured after acceptance. Continue the separate incomplete Origins gameplay roadmap without treating this plan as implementation evidence.

SCOPE NOT IMPLEMENTED:
No renderer replacement, lighting asset production/deletion, streaming, baseline/after capture, measured optimization or complete Origins experience. The full game goal is not complete.
