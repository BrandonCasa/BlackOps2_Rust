TASK:
Complete every one of the 1,219 todo.md tasks according to the accepted solo Origins plan.

STATUS:
PARTIAL - 275 completed; 944 remain. Full goal active.

FINDINGS:
Retained _zm_weap_claymore.gsc specifies twelve stored mines per owner, stock two, 1,000-point ownership purchase, two-stock round refill, 96-unit detection radius, 70-degree forward detection, minimum forward projection 20 units, 0.4-second alert and 0.1-second detonation for unsafe over-limit planting. Resolved claymore_zm supplies 0.2-second placement, 0.992-second recovery, 5.08 m blast radius and 200/50 damage. Three authored buy placements and the original t6_wpn_claymore_world model were prepared.

EVIDENCE:
External inspection/solo-content/window-claymores-04/claymores.json and verification.json passed; process exited zero. Normal six-kill Round 1 preceded controlled balances/positions and empty-round intermissions. Purchase rejected insufficient funds, accepted 1,000 from 1,500 leaving 500 and two mines, and rejected duplicate ownership. Twelve overlapping ground placements reached the unchanged native owner limit. A thirteenth consumed stock and detonated as unsafe. Seven native two-stock replenishments occurred. All twelve stored mines triggered against a naturally admitted Round 9 actor; seven attributed impacts were accepted and the actor died once. Primary weapon and all four frags were retained, including competing G gestures during placement/recovery. Original model screenshot inspected; survey and restart passed. Base combat averaged 25.642 FPS and paused room 54.527 FPS at 1280x720; controlled empty rounds do not qualify sustained survival.

CHANGES:
Five Claymore tasks checked. Added reusable owned mine inventory, fixed-step placement/alert/recovery, directional Avian detection and attributed detonation. Digit 4 deploys equipment; HUD shows stock. Purchases retain primary/lethal slots. Mine and grenade hand states exclude competing actions, with explicit grenade-input-before-mine ordering. Grenades and mines share radial damage/obstruction/flash resolution. Native round refill affects owned mines only. Imported original world model through the normal validated pack; no separate executable or tests.

VALIDATION:
Locked metadata, release build and formatting passed. Runtime closure: 1,667 files, 1,428 PNGs, 1,060,024,862 bytes. All runtime hashes match the receipt; staged closure has 1,666 valid LFS pointers and ordinary game.json. Twenty tracked Markdown files retained. Initial formatting/compile errors (leading decimal literals, scene handle inference and history field name) were fixed. First live driver failed to reach its first buy; searching all three buys with real support/clearance/visibility fixed it. A later screenshot missed the nearby model below the camera; aiming at the mine fixed the capture. Final mine run repeated after the hand-state guard. Final same-executable window-frag-after-claymores passed four bounces, two detonations, one cook-off, lethal 275 self damage and restart. Window-semtex-after-claymores-final passed native purchase/holding, geometry and moving-actor attachment, blocked cover blast with health 100, 287 enemy damage and restart. Staged helper initially lacked NumPy/tomllib; the existing Python 3.13 uv environment with declared libraries passed. Logical G/F/4 input was exercised, not physical keyboard hardware. No unit tests, fixtures, Clippy, replacement Rust toolchains, delegation or push. No original-game live comparison.

UNRESOLVED:
Direct ground support (8 cm offset/6 cm clearance), movement-capsule detection, retained-angle dot-cone blast and linear radial falloff are practical reconstructions. Native deployed flight, detailed damageconetrace arithmetic, lasers, first-person deployment, retrieval, shot/chain/EMP reactions, owner-specific buy hiding and zone gating remain pending. Source-specific explosive rewards/zombie overrides and held-grenade death drops remain unqualified. General equipment replenishment stays unchecked until other equipment is implemented. Remaining arsenal, solo revival/perks, traversal, powered systems, special enemies and quests remain incomplete.

RECOMMENDED NEXT WORK:
Implement Monkey Bomb definitions, activation delay, zombie attraction and detonation in this application. Then complete tactical replacement, source equipment replenishment and original deployment animations before continuing solo revival/life systems.

SCOPE NOT IMPLEMENTED:
All 944 unchecked tasks remain required. This milestone is not full equipment, arsenal, session, survival or quest completion. Do not mark the goal complete.
