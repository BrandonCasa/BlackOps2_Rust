# Workspace boundaries

## Current implementation

Local Cargo dependency edges are `bo2-boot -> bo2-core`,
`bo2-asset-inspector -> bo2-assets`, `bo2-map-inspector -> bo2-assets, bo2-physics`,
`bo2-physics -> bo2-assets` and
`bo2-rendering -> bo2-assets` and `bo2-geometry-viewer -> bo2-assets, bo2-rendering`.
Core, boot, physics, rendering and the viewer depend on the same workspace-pinned Bevy
0.19.1 release. All eight local crates forbid unsafe code.
Core exposes the project name and `headless_app(ScheduleRunnerPlugin)`, a small
composition function returning a Bevy `App` with `MinimalPlugins`. Bevy owns
the ECS world, entity identity, schedules, task pools, frame counter, time and
application lifecycle. The caller chooses the runner policy and adds plugins.

Boot owns argument handling and a development-only `SmokeCheckPlugin`. Its
synthetic resource/component validate startup, deferred entity spawning, repeated
updates and an `AppExit` message from the final schedule. The smoke check's three
updates and 1 ms runner pause have no original-game or fixed-step meaning. No
gameplay components, simulation rate or BO2 entity schema is approved by this task.

Assets contains a std-only reusable fastfile format adapter supported by
[findings 0002](../reverse_engineering/0002-fastfile-envelope.md). Its
`inspect_pc_signed_envelope` API accepts a reader at offset zero and a length from
the same opened file. It stages 12 then 312 bytes in one fixed buffer and retains
only metadata, including at most 31 allocated name bytes. Unsupported variants,
truncation, malformed authentication markers, observed length inconsistency and
I/O failures are distinct typed errors containing no paths or raw invalid fields.
The name never becomes a file reference. Signature bytes are discarded; the
remainder is not read. Larger length claims and concurrent source mutation remain
unverified outside the envelope. Metadata success never establishes authentication,
payload validity or loadability.

The thin inspector opens one explicit regular file read-only and obtains metadata
from that same handle. Disk-stem comparison is diagnostic and emits no source
path or disk stem. No TOML discovery, recursive inventory, Bevy asset loader,
asset server, engine or entity abstraction is introduced. The format/tooling
boundary has no Bevy dependency because it uses no engine facilities; Bevy keeps
all existing engine ownership.

Assets also contains the [diagnostic geometry snapshot v1 reader](diagnostic-geometry-snapshot.md).
This is a project-owned exchange contract for finite XYZ positions, explicit u32
triangle references and ordered surface ranges with opaque source categories/counts
and separate endpoints. Checked sizes/lengths/policies precede body reads and
fallible allocations. Each surface validates only its own references against its
exact endpoints; values and order are preserved. The thin `bo2-map-inspector`
reads one explicit regular file and emits bounded counts/categories/status only.
It has no original-file decoder, exporter, Bevy loader or renderer. Synthetic
contract success establishes neither original-format support nor fidelity;
task 0012 is reviewed and accepted. Bevy retains asset/mesh/runtime ownership.

Assets also admits bounded external schema28 brush records through a private
deserialization boundary. Physics preserves their halfspaces with Bevy math and
adds allocation-free point-segment clipping; precision diagnostics remain separate
from queries. The inspector exposes this through `--brush`. This is diagnostic
geometry, with no player hull, collision masks or original trace fidelity yet.
See the [convex-brush boundary](diagnostic-brush-collision.md).

Schema34 adds a bounded source-keyed collection using the same admitted brush
data and ConvexBrush abstraction. All selected records are retained, including
explicitly diagnostic zero-side axial hulls. Allocation-free segment queries visit
every hull and resolve equal entry parameters by source key. The inspector adds
`--brush-collection`; retained-vertex residuals and unsupported collision families
remain explicit. Hashes are caller-checkable declarations, not authentication.
See the [collection boundary](diagnostic-brush-collection.md).

The [single moving-brush query](moving-brush-trace.md) borrows the same admitted
record and implements the recovered scalar-f32 capsule-like support/contact rules.
Caller shape, mask, incoming result and aggregation order remain explicit; stationary
and full world/player queries are outside this boundary. Existing f64 diagnostic
queries and precision analysis remain separate from that native-evidenced adapter.

Rendering now contains the consuming [diagnostic Mesh adapter](diagnostic-mesh-adapter.md).
It moves validated position/index vectors into Bevy Float32x3/U32 TriangleList
storage with MAIN_WORLD usage and returns every ordered surface record unchanged.
No coordinate mapping, attribute generation or draw/material semantics is selected.
Its synthetic tests and one complete accepted external snapshot check exercise
actual headless Bevy Assets<Mesh> ownership; no loader or graphical output is added.
Assets stays std-only. This bounded task is reviewed and accepted.

The [offscreen raw-geometry diagnostic](diagnostic-geometry-render.md) now composes
that reader/adapter with Bevy rendering, fixed image targets and screenshot
readback. Its explicit synthetic/single-snapshot modes preserve raw arrays,
surface metadata and identity mesh Transform; chosen orthographic cameras,
neutral unlit solid and built-in one-pixel wireframe are diagnostic display only.
Outer review accepts the synthetic projection/internal-edge qualification and
two retained runs of one accepted snapshot identity on Vulkan. Original calibration
and fidelity remain unresolved. No loader, production-game integration or new
engine subsystem is introduced.

Bevy is the selected engine, ECS, rendering, and application foundation. The
[decision record](../adr/0001-bevy-foundation.md) records version evidence and the
minimal feature choice. Rendering locally enables asset/mesh features; the viewer
locally enables PBR/render/camera/core-pipeline facilities and PNG encoding.
Task0027 adds a windowed diagnostic viewer mode using the same Mesh/display setup,
Bevy WinitPlugin, InputPlugin and a tool-local inspection camera component/plugin.
Perspective, raw +Y up and flight controls are explicit diagnostic choices.
The viewer locally enables window/keyboard/x11 features; legacy offscreen mode
disables WinitPlugin. Scripted input and window readback checks establish engine
integration, without original movement or rendering fidelity. See
[handoff0027](../handoffs/0027-interactive-geometry-inspection.md).

Central defaults remain disabled. Boot assembles no
renderer/window/asset plugin. Headless adapter validation assembles Bevy AssetPlugin
and MeshPlugin with an in-memory source, with no graphical runtime. These diagnostic
boundaries establish no original-game engine architecture. No custom ECS or renderer
is approved.

The [capture protection controls](capture-protection-controls.md) are an external
fixture-only protocol, reviewed and accepted as PARTIAL. Held file sharing, owned DACLs,
pre-creation TEMP routing, suspended identity admission and exact-pair job cleanup
are measured. Fresh exact-job inventory accounts for the observed2/4 distinct
members and qualifies retained-job teardown. Directory-name enforcement, general
quota behavior and historical last-close inventory remain unresolved.
No production wrapper, SDK execution or Rust/Bevy ownership change is introduced.

The [ordinary SDK startup qualifier](ordinary-sdk-startup-qualification.md)
prepares one concrete external x86 phase protocol, compiled native candidates,
synthetic checks and an uninvoked Windows controller. Its PARTIAL preparation
separates exact file guarding from same-context standard-child path inference,
registration log evidence, actual module observations and retained-job teardown.
No SDK runtime, new engine subsystem or Rust unsafe exception is admitted.
Task0024's required bounded source repairs are reviewed and accepted, with tests of
the actual controller/adapter through injected operations and compile-only native
validation. Source preparation is distinct from unmeasured Windows/SDK runtime;
the hard outer supervisor and direct standard-child observations remain absent.
See its repair section and [handoff0024](../handoffs/0024-ordinary-sdk-qualifier-repair.md).

## Planned responsibilities

Directories other than core, assets, physics, rendering, boot, asset_inspector, map_inspector
and comparison_tools/geometry_viewer are reserved
locations, not Cargo members or finalized APIs. Add a crate/module only when a
bounded task supplies evidence
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
| `tools/map_inspector` | Current explicit-input diagnostic geometry snapshot counts inspector; further geometry/entity inspection requires separate evidence. |
| `tools/comparison_tools` | Reproducible diagnostics/comparisons; geometry_viewer currently renders raw diagnostic geometry offscreen with unresolved original calibration. |
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
