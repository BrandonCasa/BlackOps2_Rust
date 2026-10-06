TASK:
Complete every one of the 1,219 todo.md tasks according to the accepted solo Origins plan.

STATUS:
PARTIAL - 304 completed; 915 remain. Full goal active.

FINDINGS:
Retained _zm.gsc, _zm_laststand.gsc, _zm_perks.gsc and zm_tomb.gsc define a ten-second solo revival, 45-second bleedout, 500-point Quick Revive, one owned life and a three-purchase budget. The machine expires on the third purchase. Origins replaces ordinary downed pistols with Boomhilda. Stronger-pistol selection/owned-pistol ammunition branches remain unfinished. Fixed-step clocks, 75 cm/1 m/s crawl, machine endpoint/AABB and HUD layout/font reconstruct presentation and movement.

EVIDENCE:
External inspection/solo-content/window-solo-revive-04-final/solo-revive.json passed; actual process exited zero. Held-F rejected 499 points, accepted each 500-point purchase once, and naturally admitted enemy attacks caused three downed episodes. Each restored at 10.000019 seconds, preserving Mauser 8/32 after temporary Boomhilda 50/100 and downed firing. Crawl covered 4.79 m; stance/speed/ADS/melee/slot restrictions passed. Third-purchase entity removal passed and its image shows the empty machine location. No charge caused one attributed terminal transition, game-over and clean restart. window-solo-bleedout-01/solo-bleedout.json and bleedout-restart.json passed with process zero: after a real purchase/natural down, controlled automatic-revival delay exposed expiry at 44.999485 seconds, one death and zero downed heals. Final machine, downed HUD and removed-machine images inspected. Travel, budgets and one-enemy admission were controlled; these are life/purchase branch qualifications, not normal full-session acquisition or sustained survival.

CHANGES:
Checked twenty solo life/Quick Revive/ownership tasks. Added an explicit solo self-revive policy and target/episode/attempt tokens, ordered life commands, shared monotonic queue allocation, episode retirement and restart cleanup. Added typed prepared solo rules, bounded ownership/evidence, natural downing notifications, separate temporary inventory, loadout restoration/perk loss/charge consumption, crawl restrictions and ignored downed enemy targets. Added normal paid Quick Revive interaction and original native machine scene with complete runtime dependencies. Third purchase removes its scene/collider. Original localized revival text fades without exposing internal purchase counters. Added opt-in actual-window revive and delayed-bleedout workloads.

VALIDATION:
Locked metadata, release build and formatting passed. Assembled asset integrity passed: 1,676 files, 1,435 PNGs, 1,062,665,707 bytes. Staged closure/index verification passed: 1,675 LFS pointers/payloads and ordinary game.json match the validated hashes. Twenty tracked Markdown files. Initial release build failed on the Bevy plugin tuple size and driver borrow conflicts; fixed by nesting the plugin pair and snapshotting evidence values. Later driver mutation and obsolete TextLayout constructor errors were fixed; final release build passed. Initial window workload panicked on an invalid view index; second run's controlled travel was overwritten by physics state. Corrected bounded travel/Position synchronization; final normal and bleedout cases passed. Nonfatal Vertex_Color specialization/extracted RenderMesh logs remain; no permanent shader failure receipt was emitted and required images rendered. No unit tests, suites, fixtures, Clippy, toolchain installation, delegation, push or original-game live comparison.

UNRESOLVED:
Complete nine-perk catalog/effects, drinking actions and delayed activation, original audio/icons, local power, stronger downed-pistol selection and conserved ammunition, Boomhilda projectile/camo behavior, higher-perk loadout restoration and adversarial late-command injection remain unfinished or unqualified. Only base health/owned Quick Revive and Mauser restoration were exercised. Full arsenal, powered systems, traversal, special enemies, quests and sustained play remain required.

RECOMMENDED NEXT WORK:
Implement perk drinking timing/presentation and activation, then the remaining perk catalog and typed modifiers: Juggernog health, Speed Cola, Stamin-Up, Mule Kick, Double Tap, Deadshot, PhD and Electric Cherry. Preserve solo revival, full ownership/loss and atomic purchase semantics; retain Original/solo source differences and source-defined power requirements.

SCOPE NOT IMPLEMENTED:
All 915 unchecked tasks remain required. The complete Origins game, normal solo main quest and extended survival have not been achieved. Do not mark the goal complete.
