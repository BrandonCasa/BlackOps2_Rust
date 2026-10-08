# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings04-10 resolved; 43 remain. Library material cache identity covered only prepare_materials.py/material_textures.py. Native binding/physical-response and presentation helpers could change while cached output bypassed new interpretation. The signature now includes all four local material dependencies by filename/content SHA256. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-10-20261008/asset-review.json records actual dependency fingerprints, old ade660ac signature, corrected 8fcd7551 signature and stable repeated production output. Actual external cache contains 20,780 records with unchanged SHA256 d84eff596a2884ad6776fd495e834fea1637071c2fb88936c650231e163895e1. The recorded historical material_semantics revision has identical bytes; no differing historical semantic implementation or changed-binding output reproduction is claimed.

CHANGES: tools/prepare_bo2_library.py adds material_converter_hash for named fingerprints of prepare_materials.py, material_textures.py, material_semantics.py and prepare_origins.py, then uses it in existing material cache keys. Updated fix progress, project state and this handoff. Concurrent dirty Rust renderer/shadow files are preserved and excluded.

VALIDATION: Actual four-source signature equals independently computed content identity and repeats stably; all source dependency/cache bytes remain unchanged. Changed Python AST and scoped diff checks pass. No fixtures, synthetic inputs, tests or Clippy. Full library GPU recook, import and changed-binding rendering were not run. Canonical runtime assets/Rust/Cargo dependencies unchanged; compiler/format/game-window gates not repeated. Existing full runtime qualification remains applicable. Nothing pushed.

UNRESOLVED: Findings01-03 and11-50 remain. Existing prepared library will require ordinary recook/semantic qualification when material recipes are next regenerated; no stale-cache output is reused under the new identity. Concurrent renderer files remain dirty. 734 gameplay TODOs, native locomotion RNG/per-clip speed fitting, crawler/stumpy/super-sprint consumers and sustained performance/manual survival remain open or unqualified.

RECOMMENDED NEXT WORK: Fix11 by checking copied full DDS bytes against the initially validated full-chain hash, then continue remaining findings. Return to renderer01-03 after concurrent renderer work commits. Complete all fifty findings before Generate missing combat clips for recovered rigs.

SCOPE NOT IMPLEMENTED: No other review fixes, renderer changes, source material arithmetic changes, full library recook/import, new gameplay TODO, benchmark or package refresh.
