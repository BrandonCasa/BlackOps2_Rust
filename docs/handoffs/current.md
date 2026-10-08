# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings04/05/06 resolved; 47 remain. In-place migration writes could alter immutable source files through candidate hardlinks. Each rewritten geometry/glTF entry now uses a unique sibling temporary file, closes the complete write, then atomically replaces the candidate entry; finally removes surviving temporary files. JSON output uses UTF-8/LF. Brush metadata and validated streaming dependency preservation remain intact. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-06-20261008/asset-review.json records an actual 4,137-file fully hardlinked candidate copied from the finding05 full-qualified external pack. CLI migration exit0 detaches all 275 glTF documents plus the map buffer; 3,861 untouched assets remain shared. Every source SHA256 is unchanged, all scene documents are equal, and all binary bytes are equal. UTF-8/LF changes JSON byte hashes only. Candidate inventory remains the exact qualified closure with no temporary files. Finding05 independent migration/full importer evidence remains E:/Coding/Captures/static-review-05-20261008; finding04 brush evidence remains static-review-04-20261008.

CHANGES: tools/realtime_assets.py adds one shared atomic byte writer for compact_geometry and scene-document output. Updated the report fix progress, current project state and this handoff. Concurrent dirty Rust actor rendering/shadow edits remain excluded.

VALIDATION: Actual standalone CLI migration, file identity checks, immutable-source hash verification, complete candidate inventory and native scene/binary identity pass; no synthetic inputs or fixtures. Source pack was already fully validated under finding05; semantic closure is reused through equality of every JSON scene, every binary/other dependency, unchanged game/index and complete inventory. The streaming validator also rechecks full DDS headers/ranges/hashes before migration. The full importer is not repeated for atomic entry replacement, and no runtime import occurs. Changed Python AST and scoped diff checks pass. Canonical runtime/Rust/Cargo dependencies unchanged; new compiler/format/window checks skipped. Finding05 documentation updater initially stopped on mixed encoding/line endings and was corrected with byte-preserving edits; its first commit command had misplaced options and was corrected before the commit succeeded. No tests, Clippy or push.

UNRESOLVED: Findings01-03 and07-50 remain. Concurrent actor renderer files remain dirty. 734 gameplay TODOs, native locomotion RNG/per-clip speed fitting, future crawler/stumpy/super-sprint consumers and sustained performance/manual survival remain open or unqualified. Atomic replacement protects individual source-linked entries, not a transaction/rollback of the whole candidate pack.

RECOMMENDED NEXT WORK: Fix independent preparation finding07 (preserve the union of alpha cutoffs across shared material textures), then continue remaining review findings. Return to renderer01-03 once concurrent renderer work commits. Finish all fifty findings before Generate missing combat clips for recovered rigs.

SCOPE NOT IMPLEMENTED: No other review fixes, concurrent renderer changes, runtime import, new gameplay TODO, whole-pack transaction, benchmark or package refresh.
