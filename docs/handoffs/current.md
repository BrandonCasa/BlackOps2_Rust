TASK:
Complete every one of the 1,219 todo.md tasks according to the accepted solo Origins plan.

STATUS:
PARTIAL - 282 completed; 937 remain. Full goal active.

FINDINGS:
Recovered weapon holdFireAnim fields supply native m67 pin-pull, Monkey winding and Claymore preparation clips omitted by the prior preparation mapping. Four equipment models plus four character hands gained sixteen clip bindings, bringing the catalog to 3,179. Held equipment follows separate authoritative inventory state; primary ownership need not change. Native held/late throw poses can move below or behind the camera. The shared 14 cm upward/15 cm rearward view offset and fitting clip time to gesture durations reconstruct framing/timing; pixel equivalence is unqualified.

EVIDENCE:
External inspection/solo-content/window-equipment-animations-frag-warmed, window-equipment-animations-semtex-diagnostic, window-equipment-animations-claymore-01 and window-equipment-animations-monkeys-level-ground passed gameplay and restart receipts; each process exited zero. Inspected actual captures show Frag/Semtex pin pulling, Monkey winding/throwing and Claymore preparation/planting. Recorded native bone movement and all relevant phase playback. Frag release has only two sampled frames across its short throws; the actual throw image is visible but smooth sustained release playback is unqualified. Equipment physics, stock and prior natural-enemy coverage passed. window-primary-after-equipment-animations passed M14/MP40/Remington/B23R firing, reload, shell interruption, purchases, switching, survey and restart; MP40 capture inspected.

CHANGES:
Checked equipment deployment animations. Added prime mappings and native clips, validated view offsets and required deployment mappings. Bevy caches only owned primary/lethal/tactical/mine rigs, shares animation graphs, attaches each model to its matching hand socket and selects visibility from current action. Primary ADS/knife/switching remain driven by their existing state. Equipment seeks native clips using fixed-step gesture progress; pause freezes playback. Existing opt-in actual-window runs now capture native phases and bounded pose/visibility evidence. Monkey input holds long enough to qualify the held phase; the driver selects level open ground for its intended lethal branch. The Frag driver waits two seconds before its first input to qualify loaded presentation.

VALIDATION:
Locked metadata, release build and formatting passed. Final asset integrity and staged index checks passed: 1,667 files, 1,428 PNGs, 1,061,202,409 bytes; 1,666 verified LFS pointers/payloads plus ordinary game.json. Twenty tracked Markdown files. Initial unfitted/cold captures did not show equipment adequately; framing and warmed capture corrected this. Two Monkey repeat runs failed lethal damage after covered landings; level-ground placement selection then passed with a natural actor and unchanged obstruction rules. An incorrect validator flag was corrected; a formatting failure was fixed. No unit tests, suites, fixtures, Clippy, replacement toolchains, delegation, push or original-game live comparison.

UNRESOLVED:
Cold-start readiness, sustained fluid presentation and full original visual equivalence remain unqualified. Equipment audio, normal Monkey acquisition, G-Strike/tactical replacement qualification, material-specific bounce, held-death drops, Claymore lasers/retrieval/chain/EMP and other source branches remain pending. Existing explosion visuals and world trajectories retain their recorded approximations. Complete arsenal, solo life/perks, traversal, powered systems, special enemies and quests remain unfinished.

RECOMMENDED NEXT WORK:
Continue the checklist through solo life and source-defined perk/revival behavior. Tactical receive already supports a different tactical definition, but the retained second type is beacon_zm/G-Strike; qualify replacement alongside its real challenge acquisition and strike behavior instead of introducing a fake variant. Continue weapon/equipment audio with their source aliases.

SCOPE NOT IMPLEMENTED:
All 937 unchecked tasks remain required. Controlled positions/logical inputs and isolated equipment runs are not physical input qualification, sustained survival, the complete quest or full game completion. Do not mark the goal complete.
