# Application ownership

One Cargo package, `origins`, produces one executable. `src/lib.rs` composes plugins; `main.rs` runs that application. There are no viewer, inspector or demo executables.

| Module | Responsibility |
| --- | --- |
| assets | External configuration, pack validation, Bevy glTF loading and collision admission |
| rendering | Bevy camera, standard lighting, lightmaps, tone mapping, bloom and fog |
| input | Physical input, persistent short edges, focus clearing, pause and restart |
| player | Capsule stance, mouse look, grounding, gravity and Avian move-and-slide |
| life | Retained reusable health, death, revival and bleedout semantics |
| gameplay | Session, pistol/melee, population, pursuit, attacks and rounds |
| origins | Map content contract and one tunable gameplay profile |
| hud | Bevy UI and temporary weapon presentation |
| verification | Opt-in driver and screenshots of the real application |
| tools | Offline preparation only; all generated content is external |

Bevy owns ECS, scheduling, scenes, asset loading, materials, cameras, visibility, transforms and UI. Avian owns spatial queries and physics representations. Gameplay runs at 60 Hz in ordered movement, combat, zombie and round systems. Variable updates collect input and present the current simulation.

Every session entity is marked for restart. Map and engine entities persist. Entering Playing from pause retains the session; restarting removes session entities and rebuilds Round 1. Loading failures become visible UI states.

The [coordinate decision](../adr/0002-bevy-runtime-coordinates.md) applies once during preparation. Runtime code uses meters and Y-up. Captures are references for visual understanding and never define scene membership or cameras.

Future reusable systems belong in focused modules, with map-specific parameters in Origins configuration. Add a dependency or rendering extension only for a concrete need Bevy/Avian does not already supply.
