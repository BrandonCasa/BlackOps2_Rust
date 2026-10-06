TASK:
Complete every todo.md task per PLAN (4).md; this milestone implements native wood barriers, repairs, reward limits and zombie tearing/entry.

STATUS:
PARTIAL. 392 of 1,219 tasks complete; 827 remain. Twelve barrier tasks newly ticked. The complete goal remains active.

FINDINGS:
Twelve basic wood windows share a prefab GUID but have unique target names and exact native mantle links. Each has six original board models/repair/pull clips. Source NPC grab/pull clips destroy boards at destroy_piece notes. Native reward counters increment before the strict cap check: Round 1 awards 40 points, with further repairs still allowed. Original spawn projections landed inside the newly intact boards; bounded exterior clearance preserves their native identities/routes and permits admission.

EVIDENCE:
External inspection/solo-content/barriers-preparation.json retains source models, clip timings and reconstruction limits. Window-barriers-03/barriers.json passed twelve-window/72-board assembly, native tearing/entry, observed partial hold (0.4167 seconds) and release, six replacements/40 points, full-window stop, retired-owner cleanup, repairs beyond cap, next-round reward reset, 36 spatial audio sinks and fresh-session reset. Torn/rebuilt PNGs inspected. Window-barriers-normal-03/verification.json passed six Round 1 kills, Round 2, fifteen views and restart. Before restart it recorded 24 native board destructions, six entries and 24 audio sinks; spawn-admission.json recorded three actors tearing at native windows with zero early admission failures. Final processes terminated.

CHANGES:
Typed barrier definitions/action and individual session plank states. Twelve placements, shared six original skinned board rigs, validated presentation/animation/audio references and exterior-node association. Fixed-step repair/tear clocks, single middle-slot ownership, retired-owner release, synchronized Bevy poses and collision state. Held-use repair cancels partial progress on release/target loss/busy/downed state and stops at full windows. Completion rewards enforce the native round counter. Native source node projection now checks supported exterior offsets of 0.6/0.75/1 m against actual geometry before admission. Original wood mesh animation stays on the dynamic actor material path. Eleven enemy rigs expanded to 41 clips with original meshes/skeletons/materials/scenes compared unchanged; 3,576 total pack bindings. Source originals untouched.

VALIDATION:
Locked metadata, release workspace build and formatting checks passed. Asset assembly and staged closure passed: 1,726 runtime files, 1,457 PNGs, 1,107,756,185 bytes, 1,725 LFS pointers and ordinary Git game.json. Twenty tracked Markdown files. No unit tests/suites/fixtures/Clippy. Initial driver field/borrow errors corrected. Window -01/-02 exposed repair approach checks while the retired NPC still occupied Avian's landing; delayed approach passed in -03. Normal -01 timed out with zero actors and six pending spawns; normal -02 was stopped after its snapshot confirmed twelve projections inside intact barriers. Exterior clearance fixed this, and normal -03 passed. Existing Vertex_Color pipeline errors/actor cutout appearance predate this milestone; native board meshes render on the dynamic path, and those broader presentation issues remain unresolved.

UNRESOLVED:
Window blocker dimensions, middle-only attack slot, exterior offsets, one-second repair gesture/completion rewards, target-height offset and wood sounds reconstruct native behavior. Left/right slots, upgraded plank behavior, native wood aliases and exact acoustics remain unfinished. Three ingress nodes inherit linked interior membership; 205 other nodes remain unassigned/closed, with rejected geometry disabled. All twelve windows assemble, but only the starting repair window and naturally selected starting entries were exercised. Full-map manual traversal/survival and remaining enemy presentation are unqualified.

RECOMMENDED NEXT WORK:
Implement generator capture, shutdown/recapture, local power and native defender waves, then continue every remaining checklist task. Preserve the full legitimate solo quest and sustained survival acceptance.

SCOPE NOT IMPLEMENTED:
Generators, special enemies/crawlers, robots/tank, crafting/staffs/quests, remaining arsenal/audio/HUD/settings, full solo ending and sustained survival. All 827 unchecked tasks remain required.
