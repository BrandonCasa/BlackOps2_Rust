TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 166 completed; 1,053 remain. Full goal active.

FINDINGS:
Held aiming can reuse native tag_torso animation deltas without replacing Bevy animation ownership. Idle clips sometimes omit unchanged torso tracks; the ADS clip first pose supplies that baseline. Native horizontal FOV needs conversion to vertical projection. Owned weapon state supplies aim/recoil to both camera and shot queries. Default Mauser ADS and reload return to hip without accumulated offsets in inspected frames.

EVIDENCE:
External inspection/solo-content/window-weapon-handling/verification.json passed movement, combat, Round 2, one room view and restart; ads_reached and recoil_observed are true. ads.png and playing.png were inspected: original Mauser/Dempsey aim pose, suppressed aiming crosshair, hip pose/crosshair restored during reload/intermission. performance.json: room 48.18 FPS, combat 32.23 FPS at 1280x720; all 331 shadowed lanterns retained, 1,996,802 reused cube faces and two allocation changes. Full validated runtime-weapon-handling.json: 1,628 files, 1,409 PNGs, 1,037,973,414 bytes.

CHANGES:
Held ADS uses native enable flag and in/out timing, native torso endpoint translation/quaternion and interpolated world/viewmodel FOV. Separate configurable ADS mouse sensitivity defaults to 0.0012 versus hip 0.002. Crosshair suppression, action/aim sprint restrictions and native weapon movement scales are active. Shared hitscan firing adds deterministic disk spread, standing/crouch selection, movement scaling, ADS interpolation, per-weapon yaw/pitch recoil and recovery. Shot queries and cameras share the same recoil/clamped pitch. Native burst delay and per-shot pellet loops are implemented but family acceptance remains unchecked. Arsenal preparation derives/validates ADS poses; importer avoids a redundant complete source validation while preserving source/destination closure and end-of-import source hashes. Actual-window receipts now record ADS/recoil observations.

VALIDATION:
Locked metadata/release-build/format gates passed; affected build/format gates repeated after corrections. Complete source and imported runtime asset validation passed. An initial FOV conversion put an equipment value below the contract minimum; preparation was corrected and validation repeated successfully. Missing idle torso tracks were detected before live validation and handled using the ADS first frame. Actual game-window run exited zero. Diffs/candidates reviewed before integration. No tests, fixtures, Clippy, replacement toolchains, delegation or push.

UNRESOLVED:
Switch/equip, automatic/burst, shared ammo, segmented reload and shotgun loops still need family live coverage and remaining timings. Magazine insertion currently precedes completion; completion-transfer task remains unchecked. No weapon acquisition yet beyond starting Mauser. ADS pose uses endpoint interpolation rather than native curve timing; horizontal FOV conversion uses reference 4:3, recoil uses a 0.01 source-value reconstruction scale, movement spread doubles at six meters/second, and ADS movement is capped at one. Those are practical approximations. Crouch spread exists; prone remains pending, so full stance-spread task stays unchecked. Projectile definitions still need dedicated projectile delivery. Temporary zombie capsules remain; original enemy animation, anatomical hit volumes, melee windup, effects/audio, character selection, other arsenal presentation and complete quest/survival are pending.

RECOMMENDED NEXT WORK:
Complete normal weapon acquisition and exercise switching/equip/automatic/burst/shotgun behavior across prepared families, then original enemy rigs and transitions. Finish reload completion/segmented timing and melee windup. Continue remaining content, interactions and economy sequentially, preserving the world shadow cache.

SCOPE NOT IMPLEMENTED:
The 1,053 unchecked tasks and final playthrough gates. Current acceptance is bounded Mauser combat/ADS/render/restart, not complete arsenal or sustained quest/survival.
