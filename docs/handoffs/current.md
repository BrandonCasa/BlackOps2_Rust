# Current handoff

TASK: Fix all fifty findings in docs/static-review.txt with one local commit per fix, then resume the TODO list. Locomotion completed at 23d9214d4.

STATUS: PARTIAL

FINDINGS: Findings04-09 resolved; 44 remain. Equipment regeneration could replace the Monkey Bomb lure loop with a bare-model-owned clip while pickup retained its assembled world rig. Regeneration now resolves the native loop on that assembled owner, checks presentation membership and matching scene file, and preserves rig/clip data. Invalid assembled bindings fail explicitly. Initial native conversion retains its previous path and explicitly binds pickup to the native owner. TODO counts remain 479 complete/734 open of 1,213.

EVIDENCE: E:/Coding/Captures/static-review-09-20261008/asset-review.json records reproduced lure animation does not belong to its world assembly, corrected complete content-contract acceptance, identical assembled content and zero regenerated assembled files. Owner actor.weapon_71330c8557acffbcd2fa6407 retains its native o_monkey_bomb loop. Recovered xanim SHA256 df63725e90258f6c32468c0589bddb4809828209d43aa3f32dac683979c69ff1 supplies 2.5 seconds/13 identical notifications. Actual preassembly source conversion emits eight external files with matching native owner/loop, notifications, animation name and buffer lengths. Source game/clip/assembled scene hashes remain unchanged.

CHANGES: tools/prepare_equipment.py extracts prepare_monkey_world, preserving already assembled world ownership and native loop while retaining initial native model preparation. Updated report progress, current state and this handoff. Concurrent dirty Rust renderer/shadow work remains excluded.

VALIDATION: Actual canonical assembled content reproduces old binding failure and passes the complete corrected contract; repeated resolution is identical and emits no asset files. Initial native conversion runs on the actual preassembly external pack; relevant owner/loop/notification/buffer checks pass. Its first complete content check failed on pre-existing invalid weapon mechanics fields in that older pack; that failure is preserved in initial-full-contract-error.txt, and no whole legacy-pack acceptance is claimed. Current content and source hashes, changed Python AST and scoped diff checks pass. No synthetic inputs, fixtures, tests or Clippy. Full equipment prime/Claymore regeneration and final native material qualification/import were not run. Canonical runtime assets/Rust/Cargo dependencies unchanged; compiler/format/game-window gates not repeated. Nothing pushed.

UNRESOLVED: Findings01-03 and10-50 remain. The older external preassembly pack needs ordinary schema/material preparation before full import. Concurrent renderer files remain dirty. 734 gameplay TODOs, native locomotion RNG/per-clip speed fitting, crawler/stumpy/super-sprint consumers and sustained performance/manual survival remain open or unqualified.

RECOMMENDED NEXT WORK: Fix independent preparation finding10 (include semantic converter dependencies in library cache identity), then continue remaining findings. Return to renderer01-03 after concurrent renderer work commits. Complete all fifty findings before Generate missing combat clips for recovered rigs.

SCOPE NOT IMPLEMENTED: No other review fixes, renderer changes, legacy-pack schema migration, full equipment recook/import, new gameplay TODO, benchmark or package refresh.
