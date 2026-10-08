# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings01,04-11 and13 resolved; 40 remain. Recooking kept qualified:true from its old manifest while changing catalog and prepared assets. Qualification is now atomically withdrawn before those writes; all three existing mutating preparation entrypoints share that production operation. No new repository Python scripts. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-13-20261008/qualification.json records the actual complete qualified manifest: 6,196 models, 32,519 texture records; SHA256 06959e2a6d2c7d6cd66d8aa575f18bc811ec8c94af1c427c5be03afcf34dd606. A byte-identical native reference and hardlinked candidate prove atomic replacement preserves originals and every other manifest field, changes qualified to false, repeats identically and leaves no temporary files. Prior native game evidence remains at static-review-01-20261008: 323 premultiplied instances/zero mismatches, six kills, Round 2, 17 views/restart, zero streaming/audio errors.

CHANGES: tools/prepare_bo2_library.py adds revoke_qualification with a closed sibling temporary and os.replace; prepare calls it before catalog/cooker/asset writes, qualify_existing and retune_unbound reuse it. Updated review13 progress, project state and this handoff. Workers02/03/12 own separate worktrees; each must qualify and merge its one bug commit into local main after coordination. Other actor renderer commit85479ad75 is preserved.

VALIDATION: Actual complete native manifest revocation, other-field equality, original/reference SHA preservation, hardlink detachment, repeated operation and temporary cleanup pass. AST inspection confirms revocation precedes mutation (line70 before first mutation89); changed Python parses and scoped diff check passes. Initial cross-drive hardlink attempt failed WinError17; corrected to a byte-identical same-drive native reference. No synthetic inputs/interruption injection/tests/fixtures/Clippy. No complete library recook or GPU qualification; runtime assets/Rust unchanged, so no additional compiler/format/game checks or runtime import. Nothing pushed.

UNRESOLVED: Findings02-03,12 and14-50 remain. Worker02 holds the compiler/game slot. Worker03 found the report's Ok(None) premise contradicts locked Bevy API and is documenting that while replacing duplicate unchecked reads; no original panic reproduced. Other native locomotion/gameplay TODO and sustained survival/performance boundaries remain unchanged.

RECOMMENDED NEXT WORK: Qualify/integrate workers02/03/12 one commit each, coordinating shared compiler/game and main-merge slots; continue remaining findings. After all fifty closures, end subagent use and resume Generate missing combat clips for recovered rigs sequentially.

SCOPE NOT IMPLEMENTED: Other review fixes, full library recook, runtime asset import, new gameplay TODO, sustained benchmark and package refresh.
