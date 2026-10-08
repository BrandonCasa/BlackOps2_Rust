# Current handoff

TASK: Continue the sequential source-backed TODO goal with Extend asset preparation with trigger volume conversion. Rendering cleanup is committed at 19a5199ac and placement identities at ec0b04a24.

STATUS: PARTIAL

FINDINGS: Trigger-volume conversion is complete. The map has 212 authored entries in seven classes: 108 info volumes, 66 multiple, 18 use, 14 boxes, four damage, one radius and one look-at. Their 519 convex pieces and one cylinder now load as native-data query geometry. Existing gameplay data is structurally identical after excluding environment.triggers. Only this TODO is newly checked: 476 complete, 737 open out of 1,213 tasks.

EVIDENCE: Retained _zm_unitrigger.gsc::build_trigger_from_unitrigger_stub supplies box/radius dimensions and angles; its debug geometry centers boxes and extends cylinders upward. Read-only clip-submodel extraction supplies native brush planes. Source entity SHA256 3085e6c9c263fac28335f5fee070da6b017f681ccf126ce57b3bc662a4069de9; brush table SHA256 5dd2adbe9e70d0dc8d6a9af00eaa606efc4e9b66df0353c0d0076fe9eb687c5c. Both remain unchanged. E:/Coding/Captures/origins-trigger-volumes-20261007-234307 contains preparation/candidate-review/asset-integrity/compiler-and-cli-gates/launch/exit receipts and window/verification.json. Preserved executable SHA256 6DB1824DEBD620388B3BB224DF524EA162FF3A66E5FD5DC6E3B851DDA060E5BC.

CHANGES: Added --prepare-triggers, retained metadata and disambiguated source identities, shared convex brush solver, complete game.json catalog, matching Rust/importer validation, Bevy map roots and isolated Avian sensors. Mud preserves other environment fields; navigation/door preparation share the solver. Existing actual-game driver observes every imported shape and repeats coverage after restart. Documented the preparation path and approximation limits.

VALIDATION: Locked metadata, release build (3m 02s), format, six Python source parses, scoped diff and complete source/destination runtime closure passed: 4,137 files, 3,327,976,086 bytes. Public CLI regeneration is byte-identical to the validated catalog. Owned game PID 42936 exited 0: 212/212 roots, 520 interior and 212 outside probes with zero errors both initially and after restart, six ordinary kills, movement, Round 2, fifteen rendered views, fresh Round 1, preserved 59 placement IDs and zero streaming errors. Inspected the Round 2 render. Staged asset/LFS integrity passed for all 4,137 files: 4,135 LFS pointers and two ordinary Git metadata files. No unit tests, fixtures, Clippy or push.

UNRESOLVED: 737 gameplay TODOs remain. Convex reconstruction retains 0.01 source-unit tolerance; Avian touching replaces unrecovered native engine tolerances. This catalog prepares geometry/metadata, while authored quest, use, damage and flag callbacks remain separate TODOs. Performance acceptance and sustained manual play are unqualified.

RECOMMENDED NEXT WORK: Implement the next open TODO: Extend asset preparation with brush entity conversion. Reuse the retained cmodel/brush closure and original script_brushmodel source semantics.

SCOPE NOT IMPLEMENTED: No brush-entity milestone, new quest/damage/use callbacks, rendering changes, native numeric/pixel equivalence or release-package refresh.
