# Workspace boundaries

## Current implementation

The local Cargo dependency edge is `bo2-boot -> bo2-core`; both also depend on the
same workspace-pinned Bevy 0.19.1 release. Both local crates forbid unsafe code.
Core exposes the project name and `headless_app(ScheduleRunnerPlugin)`, a small
composition function returning a Bevy `App` with `MinimalPlugins`. Bevy owns
the ECS world, entity identity, schedules, task pools, frame counter, time and
application lifecycle. The caller chooses the runner policy and adds plugins.

Boot owns argument handling and a development-only `SmokeCheckPlugin`. Its
synthetic resource/component validate startup, deferred entity spawning, repeated
updates and an `AppExit` message from the final schedule. The smoke check's three
updates and 1 ms runner pause have no original-game or fixed-step meaning. No
gameplay components, simulation rate or BO2 entity schema is approved by this task.

Bevy is the selected engine, ECS, rendering, and application foundation. The
[decision record](../adr/0001-bevy-foundation.md) records version evidence and the
minimal feature choice. No renderer/window/asset plugin is assembled. Some
upstream support types may be compiled by Bevy's feature dependencies; this does
not initialize a graphical runtime. There are no original-game architecture
findings yet. No custom ECS or renderer is approved.

## Planned responsibilities

Directories other than core and boot are reserved locations, not Cargo members
or finalized APIs. Add a crate/module only when a bounded task supplies evidence
and justifies its dependency boundary; merge or refine these directories when
that avoids duplication of Bevy functionality.

| Directory | Responsibility |
| --- | --- |
| `crates/core` | Shared minimal Bevy headless composition and justified lower-level definitions; no map or development-smoke behavior. |
| `crates/math` | Evidence-required math utilities that Bevy does not already provide. |
| `crates/assets` | Safe external asset access, format adapters, and metadata; no embedded originals. |
| `crates/rendering` | Bevy rendering integration and evidence-backed visual configuration, not a separate renderer. |
| `crates/audio` | Bevy audio integration and observed emitter/playback semantics. |
| `crates/physics` | Evidence-backed simulation/collision integration and fixed-step behavior. |
| `crates/animation` | Animation data adapters and observed playback/state semantics using engine facilities. |
| `crates/input` | Input mapping and simulation-facing inputs using Bevy input facilities. |
| `crates/gameplay` | Shared gameplay components, resources, and systems; no Origins coupling. |
| `crates/weapons` | Evidence-backed reusable weapon definitions and simulation. |
| `crates/ai` | Shared AI/controller/navigation behavior that engine facilities do not supply. |
| `crates/zombies` | Reusable Zombies mode simulation systems and state. |
| `crates/scripting` | Safe evidence-backed event/quest/script semantics; no copied proprietary scripts. |
| `crates/ui` | Bevy UI integration for observed HUD/menu behavior. |
| `crates/diagnostics` | Permitted diagnostics, measurements, and profiling instrumentation. |
| `game/zombies/common` | Assembly and shared Zombies configuration above reusable system crates. |
| `game/zombies/origins` | Origins-specific configuration, mechanics, and assembly based on evidence. |
| `game/boot` | Application entrypoint and final assembly; currently a bounded headless lifecycle smoke plugin. |
| `tools/asset_inspector` | Read-only bounded asset inspectors with permitted output. |
| `tools/map_inspector` | Read-only geometry/entity metadata inspection. |
| `tools/comparison_tools` | Reproducible behavior/visual/performance comparisons. |
| `tests` | Cross-layer tests and explicitly user-created or synthetic fixtures. |
| `config` | Tracked placeholder templates and ignored local configuration. |
| `docs/architecture` | Evidence-backed architectural conclusions and boundaries. |
| `docs/reverse_engineering` | Observations, abstract schemas, methods, and open questions. |
| `docs/fidelity` | Measurements, comparisons, discrepancies, and system status. |
| `docs/adr` | Important reviewed architecture decisions. |
| `docs/handoffs` | Sequential delegated-task records. |
| `.agents/skills` | Bounded investigation/implementation/validation skills. |

## Dependency direction

Application assembly may depend on Origins and common Zombies assembly. Origins
may depend on reusable Zombies/gameplay and engine integration. Reusable Zombies
systems may depend on shared gameplay/engine facilities. Lower layers must not
import Origins or application assembly. Tooling may use format and diagnostics
helpers without taking a dependency on map gameplay. Test placement follows the
boundary being exercised.

Use Bevy components/systems/resources/plugins to preserve those ownership
boundaries. Avoid duplicate definitions split between `crates/zombies` and
`game/zombies/common`: simulation belongs in reusable crates, assembly in the
game layer. Check the actual Cargo graph for cycles as each member is introduced.
This is a dependency policy, not a claim that the original architecture is known.
