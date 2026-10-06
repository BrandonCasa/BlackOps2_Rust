TASK:
Complete every one of the 1,219 todo.md tasks according to the accepted solo Origins plan.

STATUS:
PARTIAL - 280 completed; 939 remain. Full goal active.

FINDINGS:
Retained cymbal_monkey_zm defines three tactical charges, 1.9-second prime, 0.7-second recovery, eight-second fuse, 20.32 m/s launch and 2.54 m radius with 5,000/450 damage. Native _zm_weap_cymbal_monkey.gsc activates after stationary placement, resets detonation time, plays o_monkey_bomb and registers attraction within 1,536 source units, up to 96 attractors, with four positions spaced 45 units. No native Origins cap override was found. Native developer grant supports the isolated live qualification; normal acquisition remains Mystery Box work.

EVIDENCE:
External inspection/solo-content/window-monkeys-03/monkeys.json and verification.json passed; process exited zero. A naturally admitted Round 2 actor followed the first activated bomb: distance decreased 2.8592 m over three seconds. After expiry, normal pursuit resumed and closed 1.3961 m. Two throws retained the primary and four frags, consumed stock from three to one, and rejected full tactical inventory. Two stationary activations reset the fuse; detonations occurred eight seconds afterward. Second explosion accepted 3,009 lethal damage against the actual actor. Native Bevy animation playback recorded 297 player updates; original toy presentation was inspected. Survey and actual new-session restart cleared tactical stock, projectiles and attraction state. Final window-frag-after-monkeys, window-semtex-after-monkeys and window-claymores-after-monkeys receipts passed with process exits zero.

CHANGES:
Five tasks checked: tactical input and Monkey definitions, activation delay, attraction and detonation. Added shared tactical inventory and T input with mutual exclusion across primary/lethal/mine actions. Prepared native throwable slot and lure tuning, grounded model/clip, animation graph reuse, actor-fill material reuse and supported attraction positions. Enemy intent cancels player attacks while lured and returns to pursuit afterward. Existing shared trajectory, attributed blast, damage/life/rewards and restart cleanup handle Monkey projectiles. HUD shows provisional tactical name/count. Extended existing opt-in actual-window driver; no test suite or separate executable.

VALIDATION:
Locked metadata, release build and formatting passed. Runtime closure: 1,667 files, 1,428 PNGs, 1,060,060,702 bytes; all hashes match the final receipt. Staged closure verified 1,666 valid LFS pointers plus ordinary game.json; twenty tracked Markdown files retained. First live run failed its post-expiry Euclidean pursuit metric because the actor navigated around cover; driver now chooses a supported clear route. Second run qualified behavior but checked restart before the new Playing/Round 1 session; state gate repaired and final run passed. Compile failures from JSON macro recursion, observer parameter ordering and material borrow overlap were repaired. Model was initially too dark; existing actor-fill calibration made it readable. Frag regression covered throwing, four bounces, full cooking, two detonations, self damage and restart. Semtex regression covered purchase, held fuse, geometry/actor attachment, cover obstruction and restart. Claymore regression covered twelve stored mines, over-limit detonation, native refill, natural Round 9 trigger and restart. Logical T/G/F/4 inputs were exercised, not physical keyboard hardware. No unit tests, fixtures, Clippy, replacement Rust toolchains, delegation or push. No original-game live comparison.

UNRESOLVED:
Four support-tested ring positions, nearest available attraction and generic non-special eligibility reconstruct native POI/path behavior. Native zone checks, EMP/dud/stolen/clone branches, crowd/special/defender attraction, source-specific explosive rewards/overrides, song/glow and first-person deployment remain pending or unqualified. Animation-player updates are not direct per-bone motion measurements. Normal Monkey acquisition is not implemented. General tactical replacement and source equipment round replenishment remain unchecked. Earlier Claymore and grenade approximations remain documented in project state and source.

RECOMMENDED NEXT WORK:
Complete source-defined equipment round replenishment, then original equipment deployment animations and remaining tactical replacement through source acquisition. Continue the remaining solo life, arsenal, traversal, powered systems, special enemies and quest tasks in checklist order.

SCOPE NOT IMPLEMENTED:
All 939 unchecked tasks remain required. This milestone is not full equipment, arsenal, session, survival or quest completion. Do not mark the goal complete.
