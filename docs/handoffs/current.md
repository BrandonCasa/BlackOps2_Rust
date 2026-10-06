TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 219 completed; 1,000 remain. Full goal active.

FINDINGS:
The original knife mesh has no animation player; its parent hand joint supplies animation. Native Dempsey hands contain the compatible viewmodel_M4m203_knife_melee_1 track and tag_knife_attach. Its authored blade crosses the view early and then moves outside the camera; the original first capture was too late. Source _zm_score.gsc death awards replace damage awards. zombiemode.csv provides surviving hit 10, solo kill base 50, head bonus 50, torso bonus 10 and melee bonus 80.

EVIDENCE:
External inspection/solo-content/window-melee-attribution-final/verification.json and wall-weapons.json passed. Normal input-driven Round 1 combat produced six unique attributed deaths: four knife kills at 130, two headshot kills at 100, and six surviving bullet hits at 10. Total awards 780, balance 1,280. Every impact records attacker/target/weapon/category/region/damage. Round 2, four-family wall handling, room survey and restart passed; damage queue/history cleared on restart. knife-melee.png was visually inspected and shows the original sleeve/hand and blade in the authored swing. Process exited zero. Combat averaged 26.89 FPS; room result in performance.json, with 4,898,636 reused cube faces, 8,320 reused directional cascades and two allocation changes. These workloads do not establish sustained performance or original equivalence.

CHANGES:
Separate reusable damage module with typed impact queue, accepted damage messages, one attributed EnemyDeath message per vitality transition and message-driven kill rewards. Explicit damage/reward fixed-step stages precede enemy attacks and corpse cleanup. Bounded 128-entry combat history, attribution counters and restart reset. Native CSV combat award preparation and importer/Rust validation. Original knife scene attaches despite having no animation player, stays visible only during melee, and hands select the knife track rather than idle fallback. Animation speed follows native knife action duration; hit/recovery timing now use knife definition. Actual-window driver retains melee edges, exercises normal knife combat and exports attribution/reward/reset evidence.

VALIDATION:
Complete runtime-melee-attribution.json source/destination closure passed: 1,628 files, 1,409 PNGs, 1,038,001,673 bytes. Locked metadata/release-build/format gates passed; affected compiler/format gates repeated after changes. First import rejected the new point tuning; its numeric array validation was added. First release build exceeded Bevy's system-parameter count; combat read resources were grouped. First visual capture caught the knife after its authored crossing; earlier final capture verified it. External receipt checks prove every award value, aggregate balance and unique deaths. No tests, fixtures, Clippy, toolchain replacement, delegation or push. Candidates/diffs and staged asset hashes/LFS closure reviewed before integration.

UNRESOLVED:
Enemy capsules and approximate anatomical regions remain; original zombie/Panzer meshes, locomotion, attack/death playback and hit transforms are next. Point rewards still need explosive/burn/other special dispatch and Double Points. Terminal repeat rejection exists but was not exercised in this run (zero rejected terminal impacts), so its checklist item remains unchecked. Damage integration currently covers player weapon/knife impacts on ordinary zombies; enemy/player hazards, armor and life progression need their own routes. Melee audio/effects, upgraded fists and special melee equipment remain pending. Knife animation duration fitting, hand/viewmodel fill, ADS endpoint interpolation/FOV and recoil remain practical reconstructions. Replacement cancellation still needs natural acquisition coverage; wall equipment, upgraded/shared refills, zones/doors/power and all remaining systems remain unfinished.

RECOMMENDED NEXT WORK:
Bind original enemy meshes and locomotion/attack/death transitions with animated hit regions and conservative shadow bounds. Extend source-defined zones/doors and acquisition through Mystery Box/Pack-a-Punch, upgraded/shared wall refills and equipment. Continue the complete checklist sequentially through powered systems, traversal, crafting and quests.

SCOPE NOT IMPLEMENTED:
The 1,000 unchecked tasks and complete solo quest/survival acceptance. Current evidence covers normal Mauser/knife combat plus bounded primary-wall arsenal/action handling.
