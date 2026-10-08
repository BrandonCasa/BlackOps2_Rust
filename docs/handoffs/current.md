# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings01 and 04-11 resolved; 41 remain. Static surfaces replayed authored premultiplied coverage after pbr::prepare had selected Blend for prepared straight DDS albedo. The later system now preserves that canonical coverage decision. Native conversion observed 323 authored premultiplied instances and zero mismatches. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-01-20261008/qualification.json, window-normal/verification.json, audio.json, screenshots, logs and exit-receipt.json. Frozen executable SHA256 025AE9F50FC89A52D3028B6314C852B3EE552C7FE160B45C74F416256687E0BE; own PID 53876 exited 0 after 185.905 seconds. Six kills, Round 2, 17 views and restart passed; actual playing screenshot inspected. No logged early material/pipeline errors. unchanged-assets.json confirms all 4,137 files/3,358,926,284 bytes retain the prior fully qualified closure.

CHANGES: src/rendering.rs removes duplicate alpha selection after PbrReady. src/rendering/opaque.rs counts premultiplied source instances and alpha mismatches in existing diagnostics. src/verification.rs rejects mismatches. Updated finding01 status, project state and this handoff. Other actor texture/shadow commit 85479ad75 remains untouched. User authorizes own-worktree subagents only for review bugs; each must qualify, create one detailed bug commit and merge it into local main after coordination.

VALIDATION: Locked metadata/release build pass. Initial format check failed one wrapped line; corrected format check passes. Actual normal game, coverage observation, combat, Round 2, camera survey and restart pass; streaming and audio error counters are zero. Asset hashes equal the prior complete importer qualification, so no redundant importer run. No tests, fixtures or Clippy. No before/after brightness measurement, native pixel equivalence, sustained performance/survival, benchmark or package refresh. Nothing pushed.

UNRESOLVED: Findings 02-03 and12-50 remain. Workers 02/03/12 use isolated worktrees;02 holds the shared compiler/game slot and no worker may merge while primary changes are pending. Native locomotion RNG/per-clip speed fitting, crawler/stumpy/super-sprint consumers and 734 gameplay TODOs remain open or unqualified. Older preassembly source pack still needs ordinary schema/material preparation before full import.

RECOMMENDED NEXT WORK: Qualify and integrate findings 02/03/12 one commit each, coordinating shared compiler/game and main-merge slots; continue the remaining findings. After all fifty fixes, end subagent use and resume Generate missing combat clips for recovered rigs sequentially.

SCOPE NOT IMPLEMENTED: Other review fixes, new gameplay TODO, runtime import, library recook, sustained benchmark and package refresh.
