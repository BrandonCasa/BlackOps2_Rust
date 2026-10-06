TASK:
Complete every todo.md task per PLAN (4).md; this milestone implements native enemy negotiation, rising, landing recovery and synchronized animation/capsule motion.

STATUS:
PARTIAL. 380 of 1,219 tasks complete; 839 remain. Four traversal tasks newly ticked. The complete goal remains active.

FINDINGS:
Native traverse/zm_shared.gsc disables gravity/collision during negotiation and resumes movement after clip notetracks. Twenty-three map aliases plus rise resolve to 23 additional clips per existing rig; two mantle aliases use an explicit reconstructed traverse_v1 selection. Native root deltas provide motion/duration. Ground_dugup carries emergence in its main-root pose, so final-bind alignment preserves below-ground emergence and a standing endpoint. Player occupancy could hold a completed traversal indefinitely; bounded supported clearance resolves it after a short wait.

EVIDENCE:
External inspection/solo-content/traversal-recovery retains 23 decompiled native scripts plus shared traversal logic. Traversal-preparation.json records aliases, duration/root samples and reconstruction limits. Window-traversal-04/traversal.json passed all route/door/cache/restart cases, native jump_up_127 from (-23.978,3.041,-11.379) to (-22.962,5.735,-11.379), nine traversal starts/finishes, two rising completions, two occupied-landing checks and observed landing hold followed by Walk. Native climb PNGs inspected. Window-traversal-normal-03/verification.json passed six Round 1 kills, Round 2, fifteen views and restart; one adjusted landing reduced blocked checks from the earlier 855 to 18. Five completed traversals/one completed rise were recorded; interrupted/fresh-session starts are not claimed complete. Final processes terminated.

CHANGES:
Twenty-nine original clips on each of eleven existing enemy rigs; 3,432 total pack bindings. Original meshes/skeletons/materials/scenes compared unchanged. Typed validated enemy traversal profiles; admitted negotiation links participate in cached routes. Fixed-step root fitting, explicit traversal/recovery state and shared Bevy clip seeking. Native noclip/nogravity lifecycle; landing clearance, bounded nearby supported/accessible alternatives and active endpoint reservations. Rising spawns retain native identities and play the original dug-up clip. Stun freezes active traversal timing. Offline CLI exposes navigation/traversal preparation; source originals untouched.

VALIDATION:
Locked metadata, release workspace build and formatting checks passed. Asset assembly and staged closure passed: 1,708 runtime files, 1,453 PNGs, 1,096,326,765 bytes, 1,707 LFS pointers and ordinary Git game.json. Twenty tracked Markdown files. No unit tests/suites/fixtures/Clippy. Initial formatting rejected shorthand float literals; corrected. Compiler coverage borrows/expanded population query patterns corrected. Import caught source changes during preparation; rerun against frozen assets passed. Traversal -01/-02 passed climb/rise; -03/-04 additionally observed recovery before actor retirement. Earlier normal runs exposed occupied endpoints; reservation alone did not reduce waits, while actual supported-clearance fallback did. Existing actor cutout appearance remains a visible presentation limit, not original visual equivalence evidence.

UNRESOLVED:
Endpoint/root fitting, mantle selection, 250 ms recovery and 0.75/1 m clearance search reconstruct behavior. All 23 aliases and every enemy variant were prepared but not individually manually played. Original traversal audio/dust, crawler negotiation selection, unassigned entrance zones and barrier prerequisites remain unfinished. 206 graph nodes remain unassigned; rejected geometry stays disabled. Stuck recovery, failed-admission/cap branches and sustained full-map survival remain unticked or unqualified.

RECOMMENDED NEXT WORK:
Implement native barriers, repair economy and zombie barrier seeking/damage/entry, including outside entrance-zone association. Then generator capture/defenders and the remaining todo tasks; retain complete quest/survival acceptance.

SCOPE NOT IMPLEMENTED:
Remaining barriers/generators, special enemies/crawlers, robots/tank, crafting/staffs/quests, remaining arsenal/audio/HUD/settings, full solo ending and sustained survival. All 839 unchecked tasks remain required.
