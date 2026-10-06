TASK:
Complete all 1,219 todo.md tasks according to the accepted solo Origins plan.

STATUS:
PARTIAL - 307 completed; 912 remain. Full goal active.

FINDINGS:
Native Quick Revive bottle firstRaiseTime is 2.8 seconds; lowering is 0.9 seconds, and wait_give_perk delays activation by 0.5 seconds. The retained lower clip is byte-identical to the drink clip; holding its completed pose avoids repeating drinking. Phase boundaries quantize to fixed steps. Primary return completes at about 4.283 seconds.

EVIDENCE:
External inspection/solo-content/window-perk-drinking-02-final/solo-revive.json passed with process zero: three paid purchases activated at 3.299998 seconds, each drink returned at 4.283334 seconds, followed by three natural downings/ten-second restorations with unchanged Mauser 8/32. Incompatible primary/equipment actions were requested during drinking. The third activation removed the machine; subsequent fatal damage and restart passed. perk-drinking-abort.json passed: controlled resolved damage cancelled a fourth drink at 1.033333 seconds before activation, leaving zero perks, charges and purchase allowance consumed, with paid points still spent. perk-drinking-restart.json passed. Native bottle-opening, completed lower pose and Mauser return images inspected. Travel, budgets and interruption damage were controlled; normal acquisition and sustained sessions remain unqualified.

CHANGES:
Checked three drinking timing/presentation/activation tasks. Added typed prepared bottle timings, authoritative fixed-step phases, delayed ownership/charge activation and death cancellation. Existing native bottle/arms assets animate through shared Bevy rigs without replacing primary ownership. Movement and primary/equipment actions respect drinking. Added bounded evidence and actual-window interruption/restart coverage.

VALIDATION:
Locked metadata, release build, formatting and diff whitespace checks passed. Assembled asset integrity passed: 1,676 files, 1,435 PNGs, 1,062,665,891 bytes. Staged closure/index verification passed: 1,675 LFS pointers/payloads and ordinary game.json match validated hashes. Twenty tracked Markdown files. Initial compile failed on a verification query tuple mismatch; corrected, final release build passed. Final actual-window process exited zero; required images rendered. Existing nonfatal Vertex_Color/extracted RenderMesh logs remain. Previous controlled 45-second bleedout case was not rerun in this milestone. No unit tests, suites, fixtures, Clippy, delegation, toolchain installation, push or original-game live comparison.

UNRESOLVED:
Remaining eight perks/catalog/effects, original audio/icons, power requirements, stronger downed-pistol priority/ammunition and native Boomhilda projectile/camo behavior remain unfinished. Lower-pose holding and movement restriction choices reconstruct missing native presentation/state details. Full arsenal, powered systems, traversal, special enemies, quests and sustained play remain required.

RECOMMENDED NEXT WORK:
Implement remaining perk catalog and typed modifiers, beginning with Juggernog health; preserve delayed activation, limits, loss, recovery and solo revival. Then Speed Cola, Stamin-Up, Mule Kick, Double Tap, Deadshot, PhD and Electric Cherry with source-defined power requirements.

SCOPE NOT IMPLEMENTED:
All 912 unchecked tasks remain required. Complete Origins, normal solo quest and sustained survival have not been achieved. Do not mark the goal complete.
