TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 255 completed; 964 remain. Full goal active.

FINDINGS:
Ordinary zombie hit detection now follows original animated joints rather than movement-capsule height. Each prepared variant has 16 anatomical capsules: head, chest, abdomen/pelvis, paired upper arms/forearms/hands and thighs/shins/feet. Head bindings use the attached head rig. Capsule radii reconstruct anatomical coverage; native collision surfaces were not recovered. The latest rendered Bevy pose is captured in actor-local coordinates and combined with current fixed-step actor movement, a bounded previous-pose approximation.

EVIDENCE:
External inspection/solo-content/window-hit-regions/verification.json passed; origins.exe exited zero. Accepted Mauser impacts covered Leg (12 damage), Arm (17), Torso (20), UpperTorso (25) and Head (70). The run bound 96 volumes across six admitted German variants and recorded 10,048 pose updates. One knife kill plus five headshot kills and 13 surviving bullet impacts awarded 760 points; Round 2 and restart passed with cleared damage history/queue. knife-melee.png and playing.png were visually inspected. Bounded combat averaged 44.058 FPS; paused room 57.433 FPS at 1280x720, 3,700,522 reused cube faces and two texture allocation changes. These are workload measurements, not original equivalence or sustained-performance proof.

CHANGES:
Reusable skeletal HitRig/Hitboxes module captures Bevy joint transforms and queries Avian capsule geometry without adding movement colliders. Shared combat selects the nearest live region across actors and retains world obstruction queries. Typed HitRegion deserialization, catalog validation, offline bone-closure checks and tunable preparation cover all 14 prepared German/crusader variants. The existing actual-window driver gains --verify-hit-regions, aiming through the shared geometric path and checking accepted damage receipts for all five regions. Six supported checklist tasks are checked. No tests or separate executable added.

VALIDATION:
Runtime-hit-regions.json passed: 1,665 files, 1,428 PNGs, 1,059,956,983 bytes. Locked metadata, release build and format gates passed. Initial compilation rejected DamageCategory equality in the verification driver; pattern matching fixed it and affected build/format gates passed again. Actual-window verification passed on its first run, including all five regions, normal attack/melee, six kills, Round 2 and restart. Original-game live comparison was not run. No unit tests, fixtures, Clippy, replacement toolchains, delegation or push. Candidate diffs reviewed; staged asset closure/hashes and Markdown count checked before commit.

UNRESOLVED:
Native hit-volume radii, exact within-frame pose arithmetic, armored volumes/absorption, penetration, limb severing and special enemies remain unfinished. Crusader assets and bindings are prepared but their zone admission is not implemented. Critical regeneration, immunity rejection, downed restrictions, self-revival/perks and death camera remain pending or unqualified. Navigation/traversal, powered systems, equipment, hazards and quests remain incomplete. Most of the checklist remains outstanding; do not mark the goal complete.

RECOMMENDED NEXT WORK:
Continue complete combat/life scope with equipment explosions and armor integration, then solo Quick Revive/perks/downed behavior. Keep source-defined parameters and normal game-window acceptance in the shared application; do not substitute verification branches for gameplay.

SCOPE NOT IMPLEMENTED:
All 964 unchecked tasks remain in scope. No armor, penetration, special enemy, equipment, revival, perk or quest completeness is claimed by this milestone.
