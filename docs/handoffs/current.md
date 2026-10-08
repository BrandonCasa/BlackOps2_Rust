# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings01,04-11,13 and28 resolved; 39 remain. Retuning treated the loose native launcher bitmap as DDS and failed even though it was already explicitly cooked as color. It now preserves that prepared bitmap variant by skipping DDS/IWI reinterpretation before decoding. No new repository Python scripts. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-28-20261008/qualification.json records actual loose/codlogo.bmp: 640x60 RGB, source SHA256 b319cdf76a79b16250d9e89df87bfcf036d25403aa999c1bae52525996eb4ff5; old decoder fails invalid DDS header. Its qualified sRGB color DDS variant retains all ten mips with declared dimensions. Full qualified library manifest SHA256 06959e2a6d2c7d6cd66d8aa575f18bc811ec8c94af1c427c5be03afcf34dd606 remains unchanged. Prior review01 native game and review13 atomic revocation receipts remain authoritative for their separate scopes.

CHANGES: tools/prepare_bo2_library.py adds an explicit BMP guard before DDS header/IWI interpretation in the existing retune loop. Updated review28 status/progress, project state and this handoff. Workers02/03/12 retain separate worktrees and must qualify/merge their single bug commits into local main after coordination. Other actor renderer commit85479ad75 is preserved.

VALIDATION: Actual bitmap source SHA, Pillow decode, existing qualified DDS variant dimensions/complete mips and old decoder failure confirmed. Python parse and AST branch inspection show BMP skip precedes decoder; scoped diff check passes. No full mutating retune or GPU library recook; original/source manifest/catalog/runtime assets untouched. No synthetic inputs/tests/fixtures/Clippy. Rust unchanged, so no additional compiler/game gates, runtime import, benchmark or package refresh. Nothing pushed.

UNRESOLVED: Findings02-03,12,14-27 and29-50 remain. Worker02 holds compiler/game slot;03 waits and documents the disproved Ok(None) review premise alongside direct fallible reads. Worker12 checks all native source LOD hashes. Other gameplay TODO and sustained survival/performance boundaries remain unchanged.

RECOMMENDED NEXT WORK: Qualify/integrate workers02/03/12 one commit each while coordinating shared compiler/game/main-merge slots; continue remaining findings. After all fifty closures, end subagent use and resume Generate missing combat clips for recovered rigs sequentially.

SCOPE NOT IMPLEMENTED: Other review fixes, complete mutating retune/library recook, runtime import, new gameplay TODO, sustained benchmark and package refresh.
