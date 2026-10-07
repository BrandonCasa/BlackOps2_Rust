TASK:
Merge the five reviewed fixes from 3cf5db8 into main, preserving generator/rendering milestones and the other chat's uncommitted viewmodel work.

STATUS:
COMPLETE for this integration. The larger TODO goal remains PAUSED at the user's explicit request: 415 of 1,219 tasks checked, 804 remaining.

FINDINGS:
Code/tool changes merge automatically onto d26d394. Only this shared handoff required reconciliation. The other chat's unstaged src/presentation.rs and src/verification.rs belong to its pending final pose/framing fix and are excluded from the merge commit.

EVIDENCE:
Main's existing generator and vertex-color/overlay-opacity commits remain ancestors. The fix commit restores traversal/rising explosive and Claymore targeting, cancels pending primary bursts during placement, retains required gameplay profiles during content-only preparation, moves equipment animation validation into the weapon branch and admits finite positive ADS sensitivity. External review evidence remains in C:/Users/brand/Downloads/code-review-findings/fix-validation-01 and findings.md.

CHANGES:
Integrate the five fixes and their existing actual-window driver extensions. Preserve generator schema validation/preparation support and all existing main runtime assets/rendering changes. Reconcile this one current record with the paused project state and review validation limits. Preserve the other chat's two uncommitted files byte-for-byte.

VALIDATION:
Merge diff/source review and whitespace checks only; no builds, tests or game windows were started during integration, honoring the user's stop-testing instruction. Before integration, the fix worktree passed locked metadata, isolated release build, formatting and complete 1,726-file asset integrity; retained rounds/player_health/solo_revive values matched exactly. Traversal -02 passed 66 actual-window checks, including attribution, cover/radius/dead exclusions, disabled-actor Claymore geometry/cone/cover checks and restart. Claymores -02/-03 passed B23R 15-to-14 ammunition, unchanged 14 ammunition/16 shots during deployment and fresh firing afterward. Main's earlier rendering hotfix separately passed normal combat/Round 2/fifteen views/restart with zero rendering errors and retained its 1,734-file runtime closure. These are separate pre-merge qualifications, not a combined validation pass. No unit tests, suites, fixtures or Clippy. Twenty tracked Markdown files.

UNRESOLVED:
Review driver repairs addressed missing cover setup, narrower blast-cone admission and failed retreat clearance. The final corrected Claymore driver compiled, but its last two launches failed before combat with GPU DeviceLost/invalid-buffer errors. GPU usage was 13,399/16,376 MiB while other Origins processes were running; causation was not established. Final corrected Claymore and separate normal combat/Round 2/restart checks for the fix batch remain incomplete because the user stopped further tests. Native pixel equivalence, remaining actor cutout appearance and sustained survival remain unqualified. The other chat's pose-validation window separately passed normal combat/ADS/Round 2/fifteen views/restart using a pre-merge executable; this does not qualify the combined changes.

RECOMMENDED NEXT WORK:
Notify the main chat that integration is finished so it can complete its requested final pose-fix commit and replace this handoff while retaining integration/validation limits. Keep the larger goal paused until the user resumes it. Run combined validation only when the user authorizes it again.

SCOPE NOT IMPLEMENTED:
No additional gameplay TODOs or rendering repairs in this merge. All 804 unchecked tasks remain required. No original source assets were modified, other game processes interrupted or commits pushed.
