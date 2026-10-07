TASK:
Fix gun/hand displacement after melee and stabilize shared first-person framing; make the requested final local commit and stop.

STATUS:
COMPLETE for this bounded pose fix. The larger TODO goal remains PAUSED: 415 of 1,219 tasks checked, 804 remaining.

FINDINGS:
Native melee changes joints that sparse idle/fire clips omit, including the Mauser torso. Stopping the clip leaves those channels in the lunge pose. Transform change detection also cannot distinguish which channels animation overwrote before ADS adjustment. Separately, the driver held ADS throughout combat, and both world and overlay lenses zoomed, enlarging the hands.

EVIDENCE:
External inspection/solo-content/window-viewmodel-placement-02/playing.png reproduces the stuck melee pose. Placement-03 and window-viewmodel-normal-05 images show active melee, centered ADS and restored lower-right Mauser idle. Normal-05/verification.json passed six kills (four knife kills), ADS, movement, Round 2, fifteen views and fresh-session restart with empty stdout/stderr. Supplied original Mauser hip-fire and current-game screenshots guided shared framing; this is practical calibration, not pixel equivalence.

CHANGES:
Capture original named rig-node transforms when Bevy creates each first-person scene. Restore them before animation sampling while playing, then apply ADS once after sampling. Pause retains the sampled action pose and removes/reapplies only the ADS modifier. Weapon grips, sockets, native clips and model offsets remain authored. Use a shared 65-degree overlay lens and 2.5 cm camera-space vertical calibration; the world lens retains weapon ADS zoom. The existing driver releases ADS after reaching full aim rather than holding it throughout combat. Assets are unchanged.

VALIDATION:
Locked metadata, release build, formatting, diff review and runtime closure checks passed before integration: 1,734 files, 1,733 LFS pointers and ordinary Git game.json. Normal-05 was an isolated actual-window pass of the complete pose fix. Placement-03 exercised original M14, MP40, Remington and B23R equip/fire/reload presentation, but failed the B23R burst-switch check in phase 5; complete arsenal acceptance is not claimed. Earlier placement-01 failed with GPU DeviceLost while another game ran. Normal-04 rendered all fifteen views and restarted but exceeded the 180-second deadline; normal-05 passed the repeat. No unit tests, suites, fixtures or Clippy. Twenty tracked Markdown files. One final local pose-fix commit; no push.

UNRESOLVED:
Merge 78a5ff5 integrated the separately completed five-finding fix batch while preserving these pose changes. The final integrated source passes compiler gates, but normal-05 used the pre-merge executable: no combined game-window qualification is claimed. The review batch previously passed compiler/integrity and focused traversal/Claymore checks; its final corrected Claymore and separate normal workloads remain incomplete after GPU failures and the user's stop-testing request. Concurrent GPU use was observed, but causation of GPU/timing failures was not established. Native pixel equivalence, all-weapon/manual coverage, actor cutout appearance and sustained survival remain unqualified. Dempsey arms differ from the supplied Richtofen reference.

RECOMMENDED NEXT WORK:
Remain paused until the user resumes the larger goal. Further combined game-window qualification requires renewed authorization after the review chat's stop-testing instruction.

SCOPE NOT IMPLEMENTED:
No additional TODOs, burst gameplay changes, original asset edits or unrelated rendering repairs in this pose commit. All 804 unchecked tasks remain required when resumed. Other game processes were not interrupted.
