TASK:
Complete all 1,219 todo.md tasks according to the accepted complete solo Origins plan.

STATUS:
PARTIAL - 266 completed; 953 remain. Full goal active.

FINDINGS:
Retained frag_grenade_zm supplies initial/capacity 4, 0.55-second prime, 0.365-second recovery, 3.5-second fuse, 23.368 m/s launch plus 3.048 m/s relative up, 6.5024 m blast radius, 300/75 inner/outer damage and 0.7/0.2 default tangent/normal bounce. Rust fixed-step equipment owns Ready/Priming/Cooking/Recovery; held cooking expires in hand and release preserves remaining fuse. Swept collision radius 4.5 cm, shared gravity, default-surface responses, linear radial falloff and transient flash are explicit reconstructions. Native material-specific bounce and complete effects are pending.

EVIDENCE:
External inspection/solo-content/window-frag-grenades-04/verification.json passed; process exited zero. One released grenade launched with 3.3 seconds remaining, bounced four times, rested and detonated after 3.316664 seconds of flight. Its 191-point impact at 3.164088 m killed an ordinary enemy. Continued knife/Mauser combat completed six Round 1 kills, awarded 670 points and reached Round 2. A held grenade cooked off, consumed a second grenade (counts 4-to-3-to-2), enqueued 275 self damage at 0.72 m and two additional enemy impacts. The accepted self-hit changed 100 health to terminal and emitted one natural death/game-over event. Restart cleared damage queue/history and restored full health. Blast and game-over screenshots were visually inspected. Combat averaged 43.830 FPS; paused room 54.194 FPS at 1280x720, 3,669,590 reused cube faces and two allocation changes. Bounded measurements do not establish original equivalence or sustained performance.

CHANGES:
Prepared throwable tuning and source-hash receipt; Rust/import value validation. Reusable grenade inventory/state machine, held-G input, counts in HUD, original world-model projectiles, fixed-step Avian sweeps/bounce/rest, fuse expiration and radius/obstruction/falloff queries. Projectile impact sources preserve attribution after owner death within the current session; shared damage/life/rewards handle self and enemy effects. Grenade priming cancels unfinished weapon actions and suspends weapon input through recovery. Bevy owns scene/model/flash assets and entity lifetime. Existing actual-window driver gains --verify-grenades, physical escape selection, blast capture, cooked self-death and restart. Eleven qualified tasks checked. The preceding f35ce31 milestone added animated head/torso/arm/leg regions and nearest selection; these remain implemented and qualified.

VALIDATION:
Runtime-frag-grenades.json closure passed: 1,665 files, 1,428 PNGs, 1,059,957,457 bytes. Locked metadata, release build and format gates passed; affected build/format repeated after driver fixes. First two actual runs failed through fatal self damage: retreat into the spawn wall left only 1.49 m clearance. A clear direction selected through real shape queries fixed the driver. The third receipt passed but its game-over capture was not saved before process exit; the driver now waits for the saved capture. Fourth run passed and produced the inspected game-over image. Original-game live comparison was not run. No unit tests, fixtures, Clippy, replacement toolchains, delegation or push. Candidate diffs reviewed; runtime hashes, staged LFS closure and Markdown count checked before commit.

UNRESOLVED:
Explosion obstruction filtering is implemented but zero targets were blocked in the final run, so its task remains unchecked. Grenade collider size, gravity and default bounce are reconstructions. Original blast particles/audio and equipment deployment animations remain unfinished; the checked effects task covers a rendered transient flash. Held grenade drop on external player death, Semtex attachment, Claymores, Monkey Bomb attraction, tactical replacement and source round replenishment remain pending. Native hit surfaces, armor/penetration, critical regeneration, immunity rejection, solo self-revival/perks, navigation/traversal, powered systems, hazards and quests remain incomplete.

RECOMMENDED NEXT WORK:
Continue throwable equipment with native Semtex tuning, shared inventory replacement and surface/actor attachment; exercise obstruction rejection in the actual map. Then implement Claymores/Monkey Bombs and original deployment presentation, followed by solo Quick Revive/perks/downing.

SCOPE NOT IMPLEMENTED:
All 953 unchecked tasks remain in scope. No complete arsenal, armor, special enemy, navigation, revival, perk, powered system or quest completeness is claimed. Do not mark the goal complete.
