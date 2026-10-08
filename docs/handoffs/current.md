# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings04-07 resolved; 46 remain. Material qualification replaced each shared texture registry entry at every use, discarding earlier alpha thresholds. The collector now retains and sorts the union of all declared material/extra-channel thresholds. Initial cooking also preserves shared entries, appends each masked usage, and normalizes thresholds before compression/cache identity. Fresh surface-channel metadata stays separate from registry-only cutoff state. Current pack has one threshold per masked map and no reproduced old cutoff loss; this is a conditional code correction. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-07-20261008/asset-review.json records actual 275-scene, 1,358-map, 8,163-channel native catalog: 843 shared maps, 45 masked maps, zero multiple-threshold maps and zero old losses in current order. Corrected registry agrees with every retained native surface coverage declaration and keeps the same cutoff union in reversed scene order; every source descriptor hash remains unchanged. Earlier full pack migration/importer and hardlink-source qualification remain in static-review-05-20261008 and static-review-06-20261008.

CHANGES: tools/prepare_materials.py separates coverage collection from coating preparation, preserves shared registry entries through initial texture cooking, appends masked usages and normalizes cutoffs before compression keys. Extra-channel thresholds merge without mutating caller lists. Updated fix progress, project state and this current handoff. Concurrent dirty Rust actor renderer/shadow files remain excluded.

VALIDATION: Actual native material registry integrity and Python AST/scoped diff checks pass. No fixtures, synthetic asset data, tests or Clippy. Current assets do not reproduce the conditional reported loss; no such reproduction is claimed. GPU encoder/recompression, full material recook and emitted-pack import were not run. Canonical runtime assets/Rust/Cargo dependencies remain unchanged; compiler/format/game-window gates were not repeated. Finding05 full importer validation covers the unchanged runtime closure; finding06 qualifies atomic candidate entry/source identity separately. Nothing pushed.

UNRESOLVED: Findings01-03 and08-50 remain. Concurrent actor renderer files remain dirty. 734 gameplay TODOs, native locomotion RNG/per-clip speed fitting, future crawler/stumpy/super-sprint consumers and sustained performance/manual survival remain open or unqualified. Future recooking must perform normal GPU semantic qualification with the complete cutoff registry.

RECOMMENDED NEXT WORK: Fix independent preparation finding08 (retain zone.unassigned during door regeneration when navigation still references it), then continue remaining review findings. Return to renderer01-03 after concurrent renderer work commits. Finish all fifty findings before Generate missing combat clips for recovered rigs.

SCOPE NOT IMPLEMENTED: No other review fixes, concurrent renderer changes, runtime assets/import, GPU texture recook, new gameplay TODO, benchmark or package refresh.
