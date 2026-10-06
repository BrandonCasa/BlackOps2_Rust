TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 239 completed; 980 remain. Full goal active.

FINDINGS:
Recovered animation channels with no samples must preserve bind transforms. Enemy translation channels are normalized around neutral bind offsets, retaining displacement while the collider owns travel. German bodies use separate head rigs attached at j_spine4. Native attack fire notifications occur at 0.833333 and 1.3 seconds within a 1.8-second action. Ordinary death currently reconstructs motion from the recovered fire-staff collapse clip.

EVIDENCE:
External inspection/solo-content/window-zombie-bind-offsets/verification.json and wall-weapons.json passed; process exited zero. Normal input-driven combat recorded six deaths, six expired corpses, six attached heads, one attack/hit and Idle/Walk/Attack/Death playback. Five distinct body/head combinations appeared in the final run; the earlier lifecycle run exercised all six combinations. Final zombie-attack.png and zombie-death.png were visually inspected and show recognizable original bodies and attached heads. Round 2 and restart passed. Combat averaged 24.950 FPS at 1280x720; settled room 57.836 FPS. The cold combat maximum was 404.2 ms. Shadow reuse recorded 5,925,515 cube faces and 9,748 directional cascades, with two texture allocation changes. These bounded workloads do not establish sustained performance or original equivalence.

CHANGES:
Reusable enemy module owns fixed-step locomotion, native-notify attacks, death cancellation and corpse expiry. Ordinary spawning selects six German body/head combinations; eight crusader combinations are prepared for later admission. Bevy owns multipart scenes, shared animation graphs, meshes/materials and playback synchronized to gameplay clocks. Per-joint conservative animated bounds precede affected-light shadow invalidation. Source bind offsets and absent channels are preserved; head roots inherit torso socket transforms. Preparation/import/content validation checks enemy tuning, animation references and attachment joints. Actual-window evidence captures attack/death frames and corpse expiry.

VALIDATION:
Complete runtime-zombie-bind-offsets.json closure passed: 1,665 files, 1,428 PNGs, 1,058,957,880 bytes. Locked metadata, release build and format gates passed; affected checks repeated after visual fixes. Initial compile failed on a private system signature; a named ordering set fixed it. Earlier window runs exposed dark materials, premature animation selection and collapsed bone translations; final actor fill/readiness/bind-offset fixes passed visual inspection. No tests, fixtures, Clippy, toolchain replacement, delegation or push. Candidate diffs reviewed; all 1,665 runtime hashes match the receipt and the staged closure contains 1,664 LFS pointers plus game.json; 20 tracked Markdown files.

UNRESOLVED:
Headwear, gibs, crusader zone admission, crawlers, rises/climbs, full attack recovery live coverage, animated hit regions, Panzer and generator enemies remain pending. Original navigation still uses the retained heuristic rather than the required zone/link graph. Enemy lighting and normalized animation offsets are practical reconstructions. Corpse duration is tuned, not asserted native parity. Player life, hazards, regeneration, power-up rewards and the remaining arsenal/world/quest systems remain incomplete. Enemy evidence is cumulative verification telemetry; gameplay entities and clocks reset on restart.

RECOMMENDED NEXT WORK:
Complete ordinary enemy traversal/navigation and source population/scaling, then animated hit regions and player life. Continue acquisition, zones/doors, power and the complete checklist sequentially through crafting, quests and sustained solo acceptance.

SCOPE NOT IMPLEMENTED:
The 980 unchecked tasks and complete solo quest/survival acceptance. Current evidence covers ordinary animated German zombies and bounded primary-wall arsenal/action handling.
