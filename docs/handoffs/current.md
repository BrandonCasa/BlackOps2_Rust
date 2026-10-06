TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 244 completed; 975 remain. Full goal active.

FINDINGS:
Retained _zm.gsc and zombiemode.csv supply health 150, +100 through Round 9, truncated 10 percent thereafter, spawn delay 2.0 decayed by 0.95, minimum 0.08 and between-round time 10. Native intermission_time 15 concerns match-end presentation, not the between-round wait. The script Divide opcode and earlier-engine [VM reconstruction](https://git.alterware.dev/Future/T4SP-Server-Plugin/src/commit/df2440a3baf87be342a611154d38bb8da795016a/src/codsrc/clientscript/cscr_variable.cpp) support an inferred fractional round/5 multiplier rather than the old floor. _zm_utility.gsc chooses walk/run/sprint at random-band thresholds 35/70; the end-of-round callback updates speed before incrementing the round. Recovered native clip deltas yield gait speeds 0.9560/2.7520/3.6086 m/s.

EVIDENCE:
External inspection/solo-content/window-round-profile/verification.json passed and process exited zero. Normal input-driven weapon/knife combat killed six 150-health Round 1 actors and eight 250-health Round 2 actors, reaching Round 3 before survey/restart. Actual admission differences were 2.0 and 1.9 seconds; two intermissions each measured 10.0 seconds. All 14 admissions consumed exactly one pending entry. All six German body/head combinations appeared. This seed selected walkers only; Run/Sprint remain unqualified. No failed admission or cap wait occurred; peak ordinary count was two. Fourteen deaths and 12 natural corpse removals occurred before remaining corpses were cleared on restart. Combat awarded 1,830 points; restart damage state cleared. playing.png was visually inspected. Combat averaged 29.016 FPS at 1280x720; room 60.341 FPS. All 331 shadowed lamps remained, with 4,425,584 reused cube faces, 6,184 reused directional cascades and two texture allocation changes. These bounded workloads are not sustained or original-equivalence proof.

CHANGES:
Reusable typed RoundProfile in the content contract owns solo health, population, pacing, caps and gait selection. Original profile preparation reads retained scripts/table and native locomotion deltas; Rust/import validators reject invalid profile data. Ordinary actors carry a population kind; distinct special/defender counts and aggregate caps are implemented for later admission. Selection/health/placement validation precede pending consumption. Enemy motion and playback use gait selection; original sprint clips are prepared for all 11 body/head rigs. --verify-rounds extends the existing actual-window driver through two combat rounds and records bounded admissions plus calculated profile values through Round 255. High-round values are runtime calculations, not played rounds.

VALIDATION:
Runtime-round-profile.json closure passed: 1,665 files, 1,428 PNGs, 1,059,899,823 bytes. Locked metadata/release-build/format gates passed; affected build/format gates repeated after fixes. Initial release compilation rejected reading gait during a mutable transition call; computing the action first fixed it. The population arithmetic correction rebuilt successfully before final window validation. Receipt intervals, health and pending differences were inspected directly. No tests, fixtures, Clippy, replacement toolchains, delegation or push. Candidate diffs reviewed; all 1,665 runtime hashes match the receipt; the staged closure has 1,664 LFS pointers and game.json; 20 tracked Markdown files.

UNRESOLVED:
Actual run/sprint playback, cap exhaustion and failed-admission preservation have no live coverage and remain unchecked. Higher-round combat, last-enemy speed-up, map-specific callbacks, Easy difficulty and special/defender admission are pending. Fractional division is inferred from earlier-engine VM behavior, not directly qualified BO2 VM arithmetic. Clip-derived speeds use one motion variant per gait rather than every native blend. Headwear, gibs, crusader admission, animated hit regions, traversal/navigation, player life, hazards, powered systems, acquisitions and quests remain incomplete. Round/actor evidence is bounded cumulative verification telemetry; gameplay entities, population snapshots, round clocks and pending budgets reset independently.

RECOMMENDED NEXT WORK:
Complete animated hit regions and player regeneration/life so higher-round combat can qualify gait/cap/admission behavior. Continue ordinary traversal/navigation, zone/door progression, special populations and the complete checklist through crafting, quests and sustained solo acceptance.

SCOPE NOT IMPLEMENTED:
The 975 unchecked tasks and complete solo quest/survival acceptance. Current live evidence covers ordinary German zombies in two Original-profile rounds and the preceding bounded primary-wall arsenal milestones.
