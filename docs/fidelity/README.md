# Fidelity status

Overall status: **UNKNOWN**. External Origins geometry has been inspected, and
safe Rust readers, a Bevy Mesh adapter and an offscreen diagnostic renderer are
qualified. Original orientation, units, camera, materials and appearance remain
unresolved. Shared resolved damage and admitted coop life-state systems are
implemented; scoped live health comparisons are recorded below. Compilation and
synthetic tool qualification alone are not fidelity proof.

Data/access understanding is **INVESTIGATING**; [project state](../project-state.md)
records accepted evidence and its limits.

| System | Status | Comparison evidence |
| --- | --- | --- |
| Geometry, visuals, lighting, and animation | UNKNOWN | No original comparison; [diagnostic tooling only](../architecture/diagnostic-geometry-render.md) |
| Movement, input, physics, and collision | UNKNOWN | None |
| Weapons and projectiles | UNKNOWN | No weapon or projectile comparison |
| Resolved damage and life states | APPROXIMATE / INVESTIGATING | [Scoped original health comparisons](damage-life-state-comparison.md); raw impact admission, solo/session policy and live coop timing remain unresolved |
| Zombies, AI, navigation, spawning, and rounds | UNKNOWN | None |
| Characters, audio, HUD, and UI | UNKNOWN | None |
| Perks, drops, and power-ups | UNKNOWN | None |
| Generators, robots, and staffs | UNKNOWN | None |
| Pack-a-Punch, Mystery Box, and buildables | UNKNOWN | None |
| Quests, Easter egg progression, and scripted events | UNKNOWN | None |
| Remaining Origins-specific mechanics | UNKNOWN | None |

Use `UNKNOWN` for uninvestigated systems, `INVESTIGATING` during evidence
collection, and `APPROXIMATE` for an explicitly documented implementation with
known fidelity limits. `MATCHED` requires documented comparisons against the
original for stated cases. `VERIFIED` requires reproducible validation of the
declared scope and tolerances. Neither status implies untested cases are correct.

For a comparison, record source/build identifiers, conditions, reproducible
steps, units and tolerances, observations, discrepancies, and linked regression
coverage. Preserve uncertainty and keep raw proprietary captures/reports outside
the repository. Never invent behavior that can reasonably be measured.
