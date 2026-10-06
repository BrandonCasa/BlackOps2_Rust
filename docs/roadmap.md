# Complete Origins systems roadmap

All work extends the single game application. Implement useful behavior first, compare where it resolves a real design question, and label practical differences. Never create another viewer or demo executable.

| Area | Foundation / next implementation |
| --- | --- |
| Map / rendering | Complete world and static assembly; refine layered materials, water, lighting and special effects |
| Player | Capsule walk/sprint/jump/crouch/stairs; refine movement feel, prone, camera effects and interactions |
| Character presentation | Original player/zombie models, rigs, first-person arms, locomotion and combat animations |
| Weapons | Pistol/reload/melee foundation; definitions, recoil, spread, penetration, attachments and complete arsenal |
| Damage / death | Retained life states; hit regions, regeneration, downing, revival and cooperative terminal handling |
| Enemies | Temporary ordinary zombie pursuit/attack; full navigation, barriers, climb/rise, special archetypes and corpses |
| Rounds / spawning | Separate pending/living budgets and normal rounds; spawn weighting, special populations, replacement and scaling |
| Doors / barriers | Map triggers, purchases, barrier repair, zone access and activation |
| Economy | Hit/kill points; purchases, discounts and transaction rules |
| Perks | Definitions, effects, purchase limits, loss and persistence |
| Drops | Probability, pickup effects, duration and presentation |
| Generators | Power, capture/contestation, enemy interactions and rewards |
| Robots | Paths, timing, collision, entry, interior and evacuation |
| Staffs | Parts, crafting, upgrades, elemental attacks and quests |
| Pack-a-Punch | Activation, purchase, weapon replacement and upgrades |
| Mystery Box | Placement, purchase, selection, take/timeout and relocation |
| Buildables | Parts, inventory, benches, construction and placed behavior |
| Quests / Easter egg | Typed transitions, map interactions, rewards and complete main quest |
| Audio | Ambient map sounds, spatial emitters, weapons, enemies, characters and music |
| HUD / menus | Round/health/ammunition/points foundation; prompts, perks, equipment, scores and menus |
| Cooperative play | Player identity, networking, synchronization, revival, scaling and session lifecycle |

The first milestone is a playable spawn-room session and a renderer that remains complete when the player moves. It is not a claim that the whole original game has been rewritten.
