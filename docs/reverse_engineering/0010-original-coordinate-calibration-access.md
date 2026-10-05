# Original coordinate-calibration access

Recorded: 2026-10-05. Task 0016 investigates access prerequisites for the single
same-build camera/visible-side calibration proposed in
[findings 0009](0009-origins-coordinate-evidence.md). **Result: the inspected
corpus establishes no usable original calibration route.** The investigation is
**PARTIAL**. It identifies a public asset-memory transport, its unsupported build
assumptions, available metadata leads and the missing original camera consumer.
Data understanding remains **INVESTIGATING**; overall, gameplay and visual
fidelity remain **UNKNOWN**. The full Rust with Bevy Origins goal is unchanged.

## Target identity and admission

The accepted [catalog](0001-local-asset-catalog.md) records Steam Zombies app
`212910`, manifest build `24784239`, English. Its exact executable candidate is
relative `t6zm.exe`: 12,543,488 bytes, SHA-256
`f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e`.
Catalogued resource metadata reports file version `1.0`, product version `6` and
the BO2 product name. These are previously recorded observations, not a new
executable verification or proof of runtime/API compatibility. No original
executable is opened, rehashed, copied, decoded or executed in this task.

The admitted Origins container and diagnostic snapshot identities remain those
in [findings 0008](0008-origins-diagnostic-geometry-snapshot.md). Neither is opened
here. No further coordinate statistics, fastfile decode or original body read is
performed. Installed manifest/resource labels cannot validate third-party fixed
addresses, pointer layouts or original coordinate conventions.

Read the goal, project rules, four assigned skills, findings 0004–0009, handoff
0015, architecture README/ADR0001 and the actual core/boot/snapshot/Mesh APIs and
relevant tests. The current adapter still stores unchanged numeric diagnostic
data. Bevy owns ECS, scheduling, assets and Mesh; the boot remains headless.
No telemetry model, camera API, transform or renderer boundary is approved.

## Pinned source access trace

C2M is pinned to `9a3accfb75c9a2f9780446c2b7bef040570357cf`.
The accepted `BlackOps2.cs` copy is 37,357 bytes, SHA-256
`99ce620fddf5ac6cef5594270b142b5e268970ee3bff68a9de2043c09c0e5954`;
it matches task 0015's pinned public copy. Directly referenced caller/project and
reader/helper files were fetched only from that immutable commit. This is the
tool author's implementation evidence, not original-game consumer evidence.

| Source anchor | Directly supported behavior | Access limit |
| --- | --- | --- |
| [Caller game table](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/HuskyUtil.cs#L38-L60) | The Zombies entry is named `t6zm` and contains two fixed asset-table addresses plus the BO2 export callback. | No executable digest, version, Steam build, timestamp, signature or schema revision is attached to that entry. Addresses are unvalidated hypotheses for the installed target. |
| [Caller selection](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/HuskyUtil.cs#L80-L107) | It enumerates processes, accepts the first dictionary name match, constructs `ProcessReader` and forwards the fixed addresses unchanged. | This caller supplies no BO2 module rebasing or build validation. This task does not run its broad enumeration or exporter. |
| [Project reference](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Husky.csproj#L89-L94) and [reader compilation](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/PhilLibX/PhilLibX.csproj#L57-L64) | The caller's reader is the included PhilLibX project, whose compiled files include `ProcessReader`, `MemoryUtil` and `NativeMethods`. | Provenance is established without building or executing the tool. The Husky configurations select x64; that does not measure the original executable's architecture. |
| [Reader construction](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/PhilLibX/IO/ProcessReader.cs#L34-L70) and [module helpers](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/PhilLibX/IO/ProcessReader.cs#L200-L215) | Construction requests VM-read access. Helpers can query the main module base and size. | The selected caller/BO2 path never calls those module helpers. The reader contains no handle-success check or close/dispose implementation. Their existence is not an admitted original access route. |
| [Byte reads](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/PhilLibX/IO/MemoryUtil.cs#L60-L67) and [native declaration](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/PhilLibX/NativeMethods.cs#L51-L59) | The helper allocates the requested byte array, calls `ReadProcessMemory` and returns the array. | It ignores both the success result and transferred-byte count and casts the handle to a 32-bit integer. No span, allocation, deadline or partial-read admission is supplied by this function. |
| [String/structure reads](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/PhilLibX/IO/MemoryUtil.cs#L165-L198) | Strings are read in chunks until a zero byte; structures are marshalled from a requested structure-sized read. | There is no total string limit or mapped-region validation. A successful-looking result after a failed/partial read is not trusted evidence. |
| [BO2 asset validation and acquisition](https://github.com/sheilan102/C2M/blob/9a3accfb75c9a2f9780446c2b7bef040570357cf/src/Husky/Husky/Games/BlackOps2.cs#L372-L430) | A default asset-name pointer chain is tested before reading the world and map-entity assets, names, vertex buffer, indices and surfaces. | This is an asset-content sentinel, not executable/schema compatibility validation. Signed 32-bit asset pointers, fixed pool slots and unchecked source counts are assumptions. No camera, active view, runtime angle, gravity, projection or rasterizer query occurs in the selected BO2/caller path. |

The official [ReadProcessMemory contract](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-readprocessmemory)
requires read access and an accessible requested span and reports success and the
transferred count. [OpenProcess](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openprocess)
can fail and returns a handle that must be closed. Those are supported Windows
transport contracts, without game-specific addresses or semantics. Merely having
those APIs does not establish safe, correct or usable telemetry for this target.
No memory API is invoked here and no reader defects are repaired.

## Available public schema leads

OAT remains external at `v0.33.0`, HEAD
`7d027e8f89118196713e955b0e11f8404149c54d`. The accepted public T6 header was
inspected read-only; no warm source, decoder, binary or submodule is changed.

| Public source lead | What it supplies | What it does not supply |
| --- | --- | --- |
| [Material state relationship](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L687-L704), [cull labels](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L2910-L2915) and [packed state declaration](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L2957-L3007) | A material-to-state-table relation and a named two-bit cull field with none/back/front labels. | No sampled material state, selected original pass, original state-to-rasterizer consumer or actual front orientation for a triangle. Asset cull labels alone cannot select winding. |
| [Camera vector constant labels](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L6062-L6064) and [matrix constant labels](https://github.com/Laupetin/OpenAssetTools/blob/7d027e8f89118196713e955b0e11f8404149c54d/src/Common/Game/T6/T6_Assets.h#L6117-L6135) | Named look/up/side and view/projection-related shader constant categories. | No runtime buffer address, binding/packing, matrix row/column convention, origin/angle record, handedness, physical units or original consumer. These are investigative leads, not telemetry offsets. |
| Map-entity/light vectors from [findings 0009](0009-origins-coordinate-evidence.md) | Counted text access and named static angle/direction/origin declarations. | No registered original camera/gravity/unit anchor. This task reads no entity text, light data or shader body. |

The inspected corpus evidences no game-native position command, console API,
demo schema or exported telemetry interface for this build. This is a scoped
absence of evidence, not a claim that the original has no such facility.
No original camera or render-state consumer has been inspected.

## Official graphics-capture alternative

Official RenderDoc release `v1.46` resolves through its annotated tag to immutable
commit `e4bd23b671d3d5a747ff5221dbe08a63eb6ca200`. Public documentation and
replay declarations provide a stronger **conditional** discriminator than
unvalidated asset addresses. They establish tool capabilities, without confirming
capture/replay compatibility with this installed executable or its active API.
No RenderDoc binary is downloaded or installed; no application is launched,
attached, injected, captured or replayed.

| Pinned official source | Potential same-draw evidence | Remaining prerequisite |
| --- | --- | --- |
| [Pipeline viewer](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/docs/window/pipeline_state.rst#L1-L28), [input assembly](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/api/replay/d3d11_pipestate.h#L215-L252) and [vertex binding](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/api/replay/d3d11_pipestate.h#L150-L173) | Per-event input layout, buffer bindings, byte offset/stride and index binding can relate a selected draw to the accepted raw patch. | Capture must contain that actual static-world draw and its dependencies. Match vertex/index topology and account for base/offset, transforms, instancing, pass and frame selection; a nearby-looking mesh is insufficient. |
| [Constant-buffer example](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/docs/python_api/examples/pipe_state.py#L39-L67), [reflection example](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/docs/python_api/examples/shader_refl.py#L17-L59) and [buffer details](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/docs/getting_started/quick_start.rst#L159-L165) | Bound shader/constant blocks, reflection and constant contents may identify the draw's view/projection transformation. | Original shader usage/binding/packing must identify the constants' meaning. Anonymous floats, combined matrices or stripped names do not directly supply original camera origin, ordered basis, angles or units. No original shader body is inspected here. |
| [Rasterizer state](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/api/replay/d3d11_pipestate.h#L371-L403) | D3D11 state includes cull mode and an explicit counterclockwise-front flag for that event. | The game must actually use the supported API, capture/replay must work, and that state must be paired with the exact original triangle/draw/viewport. Default field initializers are not observed original state. |
| [Mesh-view defaults](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/docs/window/mesh_viewer.rst#L32-L38) | The tool documents its own axis configuration and guessed unprojection. | Exclude the viewer's default Y-up/handedness and estimated projection/FOV from original evidence. Inspect captured bindings/state and demonstrated original transforms instead. |

The [official support statement](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/README.md#L7-L9)
lists D3D11 support but targets users' own programs and excludes official-channel
support/discussion for capturing third-party commercial games. This is a support
limitation, not evidence of this game's technical incompatibility or a revocation
of the user's licensed-original inspection authorization.

Capture acquisition is instrumentation, **not a passive read-only memory
observation**. The [launch/injection documentation](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/docs/window/capture_attach.rst#L13-L55)
requires a launch path or hooking before API initialization; injecting an already
initialized game is not the documented route. The
[quick start](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/docs/getting_started/quick_start.rst#L11-L26)
also makes process/tool bitness relevant. Exact installed-game launch/Steam
compatibility, graphics API, bitness, hook success, replay hardware/driver support
and capture completeness remain unmeasured. No global/child-process hooks or
environment/system changes are proposed by this task. A later capture and all
contained proprietary resources must stay external; read-only analysis of that
capture requires its own admitted bounded read/output contract.

## Compatibility and calibration-signal matrix

| Requirement | Installed target evidence | Public source/access assumption | Usable read-only observation now |
| --- | --- | --- | --- |
| Exact build/executable | Accepted manifest and catalogued executable size/digest/resource versions. | C2M selects a name with fixed addresses; OAT supports asset decoding, not runtime compatibility. | Existing identity records can be reviewed. No bridge from that executable digest to C2M addresses or a camera schema is established. |
| Loaded Origins/world registration | Accepted external geometry bridge preserves the selected world; no current game observation. | C2M's world/entity pool slots assume a compatible loaded process. | No original process memory or current loaded-map identity measured. The snapshot remains geometry evidence only. |
| Numeric camera origin and ordered right/up/forward basis | None. | Public camera labels lack a locator/schema; a valid capture may expose the selected shader's transformation constants. | Unavailable now. Need a captured, interpreted original transformation or one same-executable active-view record, with explicit packing and consumer evidence. |
| Runtime angles and rotation relation | None. | Exported entity/static-model angles are not an active-camera interface. | Unavailable. Need paired original angles and basis during later controlled signed and combined rotations; units/order/sign/composition remain open. |
| Projection and viewport | None. | A compatible RenderDoc event can expose bound constants and rasterizer viewports; public matrix labels alone cannot interpret them. | Unavailable now. Need the original draw's projection/viewport correspondence and original consumer semantics. Viewer estimates cannot fill the gap. |
| Gravity/level up and sign | None registered to raw geometry. | Asset fields named gravity or direction do not establish player/world gravity. | Unavailable. Need an original gravity/level reference tied to the same patch and camera. |
| One-sided front/cull state and triangle correspondence | Raw references preserved; no original front state. | A compatible D3D11 capture exposes event cull/front state; OAT labels and exporter Face choice remain third-party evidence. | Unavailable now. Need one exact original triangle/draw/state correspondence; both-side live observations still belong to the full calibration. |
| Independent physical units | None. | C2M's scale is an exporter policy. | Unavailable. Need an original unit definition or independent dimensional reference; view registration alone cannot establish meters. |
| Native game visibility/capture | CUA session documentation disables native computer APIs; only browser surfaces are exposed. | Official RenderDoc describes a conditional instrumented frame route, with this game's compatibility unobserved. A browser snapshot cannot observe the game. | Unavailable now. A separately admitted capture access pilot may establish selected-draw inputs/constants/state; a screenshot alone supplies neither numeric pose nor cull truth. |

## One permitted availability observation

After the source identifies `t6zm`, one exact-name
`System.Diagnostics.Process.GetProcessesByName` existence check returns **zero
matches**. It exits 0 in 0.297 seconds with 126 combined capture bytes, empty
stderr and a terminal child. The helper accesses no per-process PID, module,
path, window, memory or input properties; it retains no unrelated process records.
The PowerShell helper contains wrapper disposal in `finally`; zero matches
exercise an empty disposal loop. Only aggregate availability is recorded
externally. This is a single
current availability observation, not a repeatability or compatibility claim.
No game is launched to change that state.

The parent reviewed the early source checkpoint and agreed that PE metadata
cannot discriminate the missing camera/draw-schema hypotheses. Therefore no PE
probe was proposed or admitted. PE architecture/image-layout labels could refine
transport compatibility but could not validate the fixed pools, identify an
active camera or prove coordinate semantics. A header-only success would not
make this access matrix usable.

## Smallest next task and discriminating measurement

Before findings 0009's calibration, prefer **one separately bounded same-build
one-frame capture/replay access pilot**. It is potentially more informative than
unanchored executable-schema work because a single original draw can associate
actual inputs, transforms and front/cull state. It remains a conditional future
task; task 0016 establishes no usable original capture and authorizes no attempt.

Its prerequisites are a separately scoped instrumentation/launch procedure for
only the exact licensed Zombies target, verified current executable identity and
active supported API/bitness, an agreed loaded Origins view, replay-capable local
tool/hardware, private external capture storage, finite setup/capture/replay/read/
allocation/output/deadline policies and own-process/handle cleanup. Avoid global
hooks, unrelated processes, all-live-resource capture, source edits and input
retries on uncertain state. CUA's native limitation remains; positioning/control
must have an actually supported separately agreed route. Do not call injection or
hooked launch a passive read, or assume a game is running because it is installed.

The smallest original discriminator is **one identified asymmetric static-world
draw in one successful captured/replayed frame**: associate its bounded vertex/
index inputs with the admitted patch, record its actual front/cull/viewport state
and determine whether bound constants can expose an original view/projection
transform. Preserve build/settings/state/tool/capture identity and exact draw
selection externally. Stop with PARTIAL on unsupported acquisition/replay,
ambiguous patch correspondence or uninterpretable constants. Success means the
access pilot and only the observed fields, not full coordinate calibration.

If that pilot cannot expose camera transformation, the fallback is one
same-executable active-view consumer/address/schema investigation. Completion
requires a reproducible identity/locator/relocation/record-span/scalar-layout/
active-view-selection/readability/partial-read/cleanup chain for camera origin
and ordered basis or matrix, or an evidenced original command/recording facility
with the same contract. No plausible C2M pool address or guessed executable
offset substitutes for that consumer. Original code/body access needs its own
reviewed scope; no general telemetry tool belongs in either prerequisite task.

After access is accepted, repeated baseline/signed single-axis and combined
rotations with paired known angles can discriminate basis and angle hypotheses.
The full findings 0009 calibration still needs original gravity/level, asymmetric
correspondences, actual one-sided state and both-side observations. Runtime
angles/gravity and independent physical units need additional original anchors
even when a valid graphics capture supplies a view transform and rasterizer state.

## Reproduction, preservation and validation

External `0016-original-calibration-access/` retains predeclared policy, staged
source-fetch helpers/logs, immutable public copies and identities, source-access
checks, authoritative RenderDoc sources/tag identities, exact-name existence
helper/result/logs, preservation helper/records and documentation validation.
Resolve roots with `tomllib` from ignored configuration;
neither configuration nor real roots enter tracked output. Exceptions retain
types only in final helpers. Nine authoritative C2M requests total 63,150 bytes;
twelve authoritative RenderDoc requests total 112,756 bytes. Each staged invocation
allows at most 16 requests, 2 MiB and 120 seconds, each request 15 seconds and
262,144 bytes. Existing accepted BO2/OAT source copies are not new fetches.
No tool is built or installed. Failed browser/source-page lookup and the
incomplete RenderDoc stage5 fetch do not supply authoritative evidence; exact
stage5 helper/log hashes are preserved. Its outdated example path returned an
HTTP error. Fresh stage7/8 source captures succeed using the pinned examples
directory. An excerpt-display encoding failure after successful stage6 fetches
does not alter source bytes; those captures are not the final authority. Only
successful final immutable API/raw captures supply RenderDoc source claims.

The exact-name observation policy admits at most 16 matches and an 8,192-byte
record. The original Python supervisor was executed inline; parent review prompted
later retention of its exact recorded command in external
`process-existence-original-invocation.json` and an accompanying provenance
addendum. It generates elapsed time from `time.monotonic`, byte count from the
returned stdout/stderr and terminal state from `poll` after `communicate`.
Syntax-only inspection matches its embedded helper/policy literals byte for byte
and its recorded helper hash. This is later command retention, not a pre-run
supervisor freeze or another process observation.

The inspected supervisor uses `communicate(timeout=120)` and a `finally` block
that kills/awaits its own child if still live and closes both pipes. Its 256-KiB
combined-capture assertion runs **after** `communicate` returns; it is not a
streaming or hard concurrent capture cap. Buffering relies on this known child's
bounded aggregate/error contract, with no process-memory cap. The successful
zero-match run exercises the normal terminal-child/pipe-cleanup path only.
Timeout, exception, nonzero-child, capture-limit and kill/wait/close failure paths
were not exercised; their cleanup is inspected implementation, not tested
failure-path evidence.
No original executable probe or new original-data parser requires synthetic
header/malformed/overflow tests; none is claimed. Source-anchor/content checks
and documentation/preservation checks are the applicable validation.

Before/after same-handle SHA/length/write-time/device/file-ID preservation covers
387 existing source/helper/tracked/config records (63,991,356 baseline bytes),
including 13 accepted pristine sources, OAT HEAD, nine clean submodules, exact
task9 source/header/binary/Git patch and official/task10/task11/task13/task14/
task15 artifacts. Only the three owned existing documentation files may differ;
the two new files are also within the assigned five-file scope. Full file identity
hashing uses 65,536-byte blocks with policies of 1,024 files, 256 MiB each, 2 GiB
total and 120 seconds. These logical policies do not cap Python/OS process memory.
No original container, snapshot or executable is included in these hash reads.

All five candidates pass checks for private roots (including escaped forms),
secrets, prohibited content, scope and whitespace, with 73 relative links,
26 pinned anchors and 26 anchor-content checks. Handoff 0016 has exactly the ten
required headings; 23 source records and 41 frozen helper/source identities pass.
The original frozen record remains unchanged; three separate provenance files
are frozen after review, with syntax/literal/result-hash checks passing and no
live-check rerun.
`git diff --check` and preservation checks pass; actual counts are in
[handoff 0016](../handoffs/0016-original-calibration-access.md). No non-owned code,
manifest, lockfile, test, configuration, tool or source identity changes.
Unchanged Cargo metadata/test/fmt/Clippy, smoke and inspector gates are
intentionally not rerun; the independently accepted 67-test task 0014 baseline
remains the executable evidence. No live comparison or graphical fidelity claim.

Outer review accepts this investigation with its PARTIAL result. Independent
checks confirm 387 preservation records, 85 non-owned tracked files, 23 source
identities, 41 original frozen identities, three supplemental provenance
identities, 73 relative links, 26 pinned anchors/content checks and ten handoff
headings. The retained invocation's syntax and exact helper/policy literals were
checked without execution. No live check, original read or Cargo gate was
repeated. Tool capability remains distinct from untested installed-game access;
the next task is one bounded capture/replay access pilot.
