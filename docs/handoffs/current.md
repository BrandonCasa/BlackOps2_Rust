TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 249 completed; 970 remain. Full goal active.

FINDINGS:
Retained _zm_playerhealth.gsc supplies ordinary regeneration delay 2,400 ms, critical delay 5,000 ms, critical threshold 20 percent, critical recovery 10 percent of maximum every 50 ms, worthy-hit threshold 10 percent and immunity intervals 0.35/0.5/0.3 seconds. Ordinary wounds refill fully after the delay. Critical status remains latched until full health. Recovery is restricted to standing humans. Exact native script/frame-end scheduling is reconstructed with a 60 Hz clock.

EVIDENCE:
External inspection/solo-content/window-player-health/verification.json passed; process exited zero. A normal enemy delivered two separate 50-point hits, each followed by 50-to-100 recovery at 2.416665 seconds. The repeated attacker completed its attack/recovery cycle before re-engaging. Subsequent normal weapon/knife combat killed six enemies, awarded 740 points and reached Round 2. After the paused survey, two ordinary enemy hits produced one natural terminal/game-over event. Four accepted player damage events produced feedback and no enemy rewards. Restart restored 100 health with zero immunity/critical/feedback state and cleared the damage queue/history. player-hurt.png, player-recovered.png and player-game-over.png were visually inspected. Critical recovery, immunity rejection and repeated terminal rejection were not exercised (zero rejection counters). Combat averaged 23.276 FPS at 1280x720; room 49.411 FPS, with 2,442,374 reused cube faces, 3,748 reused directional cascades and two allocation changes. These are bounded workloads, not sustained performance or original-equivalence proof.

CHANGES:
Prepared player-health profile with Rust/import validation and source-hash receipt. Reusable standing-player recovery component and bounded vitality healing enforce maximum health and terminal/downed exclusion. Typed weapon/enemy impact sources admit enemy hits through the shared damage phase. Weapon combat resolves before queued enemy damage, canceling attacks from killed enemies. Fixed-step order now includes enemy intentions before combat and player life after damage; corpse transitions remain after accepted deaths. Accepted damage resets recovery delay and supplies feedback; player death emits a separate game-over event. Bevy UI supplies reconstructed fading peripheral red bands. The existing actual-window driver gains --verify-health, natural damage/retreat/recovery, natural fatal hits, frame capture and restart evidence. No second executable or tests were added.

VALIDATION:
Runtime-player-health.json closure passed: 1,665 files, 1,428 PNGs, 1,059,900,195 bytes. Locked metadata, release build and format gates passed; affected build/format gates repeated after fixes. Initial compile failed because the current Bevy feature set excludes Pickable; removing the unused picking marker fixed it. Live receipts independently confirm both recovery intervals, bounded healing, repeated attack recovery and one natural death event. Original-game live comparison was not run; timing comparison uses the retained script. No unit tests, fixtures, Clippy, replacement toolchains, delegation or push. Candidate diffs reviewed; all 1,665 runtime hashes match the receipt; staged closure contains 1,664 LFS pointers plus game.json; 20 tracked Markdown files.

UNRESOLVED:
Critical regeneration rates, downed suppression and immunity rejection are implemented but not live-qualified, so their checklist items remain unchecked. Human terminal feedback reports effective zero health without claiming the retained vitality number equals original post-callback health. Original blood overlay/blur and damage/recovery audio remain unfinished. Perk maxima, Quick Revive/downed loadout/bleedout and death-camera behavior are pending. Run/sprint, population caps/admission failure, higher-round survival, animated hit regions, enemy navigation/traversal, hazards, powered systems, equipment and quests remain incomplete. Health evidence is bounded cumulative verification telemetry; per-player recovery state is replaced on restart.

RECOMMENDED NEXT WORK:
Complete animated hit regions and explosive/equipment damage so critical health and immunity behavior can be exercised naturally; then integrate perks and explicit solo self-revival. Continue navigation, zones/doors, special populations and the complete checklist through crafting, quests and sustained solo acceptance.

SCOPE NOT IMPLEMENTED:
The 970 unchecked tasks and complete solo quest/survival acceptance. Current health evidence covers ordinary standing-player recovery and one natural solo terminal route.
