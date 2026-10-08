# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings04-08 resolved; 45 remain. Native door adjacency lacks navigation's fallback zone.unassigned. Replacing the zone catalog with adjacency endpoints loses that required reference. Regeneration now retains the existing fallback record and its state, avoiding duplicate IDs if native adjacency already owns it. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-08-20261008/asset-review.json records recovered zm_tomb.gsc SHA256 1f442456c5705e24f66c7534af4fe9fd49e45d06007ae89ab83e0d636a8a3afc and actual game.json SHA256 5dc4707738ded0fac8b9a76dccb86f5bb4cec6bd9a0955a4df5c27d7a7351a17. Native script supplies 160 edges/98 zones. Current pack has 99 zones, 2,776 navigation nodes and 205 fallback users. Old catalog fails: missing or wrong-kind zone reference in node.native.17: zone.unassigned. Corrected full content contract passes, catalog/content are identical to qualified source, and repeated regeneration is identical.

CHANGES: tools/prepare_doors.py shares the native zone-ID mapping and builds door_zones from authored endpoints plus the existing navigation fallback. Receipt zone count includes the retained zone. Updated report progress, project state and this handoff. Concurrent dirty Rust actor-renderer/shadow files are preserved and excluded.

VALIDATION: Actual recovered script and canonical content supply all inputs; no fixtures, synthetic data, tests or Clippy. Reproduced old metadata failure; corrected complete content contract, unchanged-content comparison, idempotence, source hashes, Python AST and scoped diff checks pass. Full door/model/audio regeneration is not run because those outputs are outside this metadata correction. Runtime assets/Rust/Cargo dependencies unchanged; compiler/format/game-window gates not repeated. Prior full importer qualification continues to cover unchanged assets. Nothing pushed.

UNRESOLVED: Findings01-03 and09-50 remain. Concurrent renderer files remain dirty. 734 gameplay TODOs, native locomotion RNG/per-clip speed fitting, crawler/stumpy/super-sprint consumers and sustained performance/manual survival remain open or unqualified.

RECOMMENDED NEXT WORK: Fix independent preparation finding09 (preserve Monkey Bomb world assembly ownership during equipment regeneration), then continue remaining review findings. Return to renderer01-03 after concurrent renderer work commits. Complete all fifty findings before Generate missing combat clips for recovered rigs.

SCOPE NOT IMPLEMENTED: No other review fixes, concurrent renderer changes, door model/audio recook, runtime import, new gameplay TODO, benchmark or package refresh.
