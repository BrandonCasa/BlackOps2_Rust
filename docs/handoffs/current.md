# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings04/05 resolved; 48 remain. Migration preserved scene/game image references but omitted the texture-streaming index, leaving all 1,356 full DDS chains eligible for deletion. The corrected path validates that index before any mutation and retains both bootstrap and full dependencies. Previous finding04 retains nonlighting node metadata and all 67 brush roots. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-05-20261008/asset-review.json contains actual source/candidate hash closure, original-pruner dependency analysis and full importer result. All 1,356 original full chains were absent from the old retained set. All 2,713 streaming-index/bootstrap/full dependency hashes survive corrected migration. External candidate files have independent file identities before migration, preventing the still-unfixed hardlink path from touching source. Finding04 evidence remains E:/Coding/Captures/static-review-04-20261008.

CHANGES: tools/realtime_assets.py imports the existing streaming validator and seeds the retained set with its validated closure. Script metadata declares NumPy/Pillow for the validator's standard DDS parser. Updated fix progress, project state and this current handoff. Concurrent dirty Rust actor rendering/shadow work remains outside this commit.

VALIDATION: Actual complete 4,137-file runtime pack copied externally without hardlinks; no synthetic inputs or fixtures. Migration removed zero images/bytes. Complete importer semantic closure and exact file inventory pass; index, full chains and bootstraps preserve their qualified hashes. All 4,137 canonical source hashes still match the locomotion import receipt. Python AST parse, uv standalone CLI/help and scoped diff checks pass. The documentation updater initially rejected mixed report encoding before writes, then stopped at mixed state line endings after its report-only update; byte-preserving ASCII edits corrected both without altering unrelated text. No runtime assets, Rust or Cargo dependencies changed; compiler/format/window gates were not repeated. Prior locomotion qualification includes locked gates, 210 focused clip/variant cases and ordinary combat/Round2/survey/restart. No tests, Clippy or push.

UNRESOLVED: Findings01-03 and06-50 remain. Finding06 still writes candidate glTF/buffers in place, so only an independent copy was migrated for finding05. Concurrent actor renderer edits remain dirty. 734 gameplay TODOs, native locomotion RNG/per-clip speed fitting, future crawler/stumpy/super-sprint consumers and sustained performance/manual survival remain open or unqualified.

RECOMMENDED NEXT WORK: Fix06 using atomic file replacement and qualify migration of an actual externally hardlinked candidate while proving its source bytes remain unchanged. Return to renderer findings01-03 after concurrent renderer work commits. Finish all fifty findings before Generate missing combat clips for recovered rigs.

SCOPE NOT IMPLEMENTED: No hardlink-write correction, other review fixes, concurrent rendering changes, runtime import, new gameplay TODO, performance benchmark or package refresh.
