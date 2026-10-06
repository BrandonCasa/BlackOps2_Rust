TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 210 completed; 1,009 remain. Full goal active.

FINDINGS:
Native shell reloads have distinct start, insertion-loop and finish phases. Remington inserts its first shell during the start phase; Python's start insertion lies beyond its start duration and is therefore omitted. Magazine ammunition now transfers only when its action completes. Weapon actions retain an interrupted shell reload's requested fire/melee/switch through the finish phase. Burst cancellation and melee windup share the same owned action state.

EVIDENCE:
External inspection/solo-content/window-weapon-actions-final/wall-weapons.json passed M14/MP40/B23R completion-only magazine transfers, Remington start/loop/end and interrupted-fire/resumed reload, and cancellation of a B23R burst after one shot by switching. Native wall purchases/refills, single-charge held use, firing patterns and direct/cyclic switching remain covered. verification.json passed normal movement/combat, Round 2, a room survey and restart. performance.json: room 58.26 FPS and combat 36.90 FPS at 1280x720, 6,063,971 reused cube faces, 10,938 reused directional cascades, two allocation changes. Prior window-weapon-actions-edges passed shell interruption; its B23R shell flag incorrectly carried the preceding family's observation and was corrected in the final receipt.

CHANGES:
Native reloadStartTime/reloadStartAddTime/reloadEndTime preparation and numeric validation. Typed shell phases, once-only insertion, native start/end clip signals, queued interruption actions, completion-only magazine transfer, delayed once-only melee strikes and melee priority over unfinished firearm bursts. Explicit replacement can cancel reload state after validating ownership; normal wall purchases still reject busy weapons. Cancellation retires queued actions. The actual-window driver retains action edges until fixed-step consumption, exercises multiple shotgun shells, interruption/resumption, transfer timing and burst cancellation.

VALIDATION:
Complete runtime-weapon-actions.json source/destination closure passed: 1,628 files, 1,409 PNGs, 1,038,001,323 bytes. Locked metadata/release-build/format gates passed; affected build/format checks repeated after fixes. First window-weapon-actions run failed because its driver discarded a fire edge before a fixed tick; retention corrected this. Final actual-window receipts passed and process exited zero. A final duration-bound correction covers melee windup longer than recovery; no current native definition has that combination, so existing window behavior is unchanged and the affected compiler/format gate was repeated. No tests, fixtures, Clippy, replacement toolchains, delegation or push. Integration candidates/diffs reviewed; staged asset hashes/LFS closure checked.

UNRESOLVED:
Replacement cancellation API needs natural acquisition live coverage. Melee animation/damage attribution and original enemy rigs remain pending. Native insertion values beyond an insertion-loop duration are clamped to its completion; Python-specific original arithmetic is not proven. Offhand wall equipment, upgraded/shared-variant refill matching, zones/doors/power, non-weapon interactions and source audio/effects remain unfinished. Original first-person knife animation requires separate presentation work: Dempsey has tag_knife_attach and a compatible viewmodel_M4m203_knife_melee_1 clip, but the current primary-weapon melee action may fall back to idle. Viewmodel lighting/FOV, endpoint ADS interpolation, 4:3 FOV conversion, 0.01 recoil scale, movement spread, capsule hit regions and 90-nit textured wall fill remain practical approximations. Full quest/survival/8,000 Hz hardware/final playthrough gates remain pending.

RECOMMENDED NEXT WORK:
Bind original enemy meshes and their locomotion/attack/death transitions, and complete first-person knife presentation with actual melee damage attribution. Extend natural acquisition through Mystery Box/Pack-a-Punch, upgraded/shared wall refills and equipment. Then implement source-defined zones/doors, powered systems and remaining complete gameplay sequentially.

SCOPE NOT IMPLEMENTED:
The 1,009 unchecked tasks and complete solo quest/survival acceptance. Current acceptance is normal Mauser combat and bounded primary-wall arsenal/action handling.
