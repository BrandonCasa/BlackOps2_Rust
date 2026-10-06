TASK:
Complete every one of the 1,219 todo.md tasks according to the accepted solo Origins plan.

STATUS:
PARTIAL - 281 completed; 938 remain. Full goal active.

FINDINGS:
Retained _zm.gsc award_grenades_for_survivors runs at round start. With native capacity four, zero lethal charges becomes two, one becomes three, and two or more becomes four. It does not award tactical ammunition or primary magazines/reserves. _zm_weap_claymore.gsc separately resets owned mine stock to two at between_round_over. The prepared source receipt records the survivor script hash eecd7660136b90c904ec25ae6dd4d01a1b96feb0e5aa1a497fe003d6e2e4f0ce. Explicit transition events avoid losing awards when the displayed round remains capped at 255.

EVIDENCE:
External inspection/solo-content/window-equipment-refills-final/equipment-refills.json and verification.json passed; process exited zero. Normal six-kill Round 1 preceded controlled inventory branches and accelerated empty intermissions. All starting counts zero through four were exercised separately for Frags and Semtex and produced 2/3/4/4/4. Unowned tactical/mine slots stayed absent. Owned Monkey stock remained one; owned Claymore stock reset from zero to two on six transitions. Every branch preserved primary magazine/reserve values. An actual gameplay transition at capped Round 255 awarded once. Twelve total lethal awards included the normal Round 2 transition. Actual restart restored four Frags, empty tactical and no mine ownership. Actual room capture inspected. Window-claymores-after-refills then passed twelve stored placements, unsafe thirteenth detonation, seven refills, natural Round 9 triggers, preserved primary/lethal inventory, survey and restart; process exited zero.

CHANGES:
Checked the source-defined equipment round replenishment task. Added prepared round_award tuning (two for Frags/Semtex, zero for Monkey), with Rust/offline contract validation. Gameplay emits a typed RoundStarted message only after intermission completion, including repeated capped rounds. Fixed-step equipment/mine consumers run afterward; primary inventory receives no round award. Existing opt-in game-window verification covers branch counts, capped-round delivery and fresh-session inventory. This turn also committed the five Monkey/tactical tasks in 8223a9c; their detailed retired handoff remains in Git.

VALIDATION:
Locked metadata, release build and formatting passed. After extending restart qualification, release build and formatting passed again. Assembled source/destination validation and staged index checks passed: 1,667 runtime files, 1,428 PNGs, 1,060,060,789 bytes; all hashes match, 1,666 LFS pointers/payloads plus ordinary game.json. Twenty tracked Markdown files retained. First refill run passed all eleven branches; final run repeated with explicit fresh-session inventory qualification and passed. No implementation or live failures in this refill milestone. Actual-window drivers use controlled counts/positions and logical inputs; physical keyboard hardware and sustained survival were not qualified. No unit tests, suites, fixtures, Clippy, replacement Rust toolchains, delegation, push or original-game live comparison.

UNRESOLVED:
Equipment deployment animations, tactical replacement/normal Monkey acquisition and source-specific explosive rewards/overrides remain pending. G-Strike and Max Ammo replenishment remain separate unchecked tasks. Downed/spectator/life-state variants are not qualified by this standing solo survivor run. Previous grenade, Claymore and Monkey reconstructions/limitations remain recorded in project state and source. Complete arsenal, solo self-revival/perks, traversal, powered systems, special enemies and quests remain unfinished.

RECOMMENDED NEXT WORK:
Implement original equipment deployment animations. Native view models/throw clips already exist, but holdFireAnim prime clips were omitted from the shared ACTIONS preparation mapping. Add the recovered prime clips to equipment and arm bindings, drive presentation from authoritative equipment state, and exercise actual held/release/plant gestures without changing primary ownership. Continue source tactical replacement/acquisition and solo life systems afterward.

SCOPE NOT IMPLEMENTED:
All 938 unchecked tasks remain required. Controlled branch qualification is not sustained survival, complete equipment presentation or full game completion. Do not mark the goal complete.
