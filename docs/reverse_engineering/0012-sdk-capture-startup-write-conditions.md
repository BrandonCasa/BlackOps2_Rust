# SDK capture startup write conditions

Task 0018 is **PARTIAL**. Pinned RenderDoc v1.46 source establishes that an
injected capture target reads configuration without refreshing its file, while
D3D11 capture device initialization can dirty the shared shader cache and save it
at device destruction. The standard crash-server child is a replay application:
it can refresh configuration during DLL initialization before parsing `--crash`.
Both roles create directories and initialize diagnostic logging. Source conditions
are established; the actual original target's role, folder identity and effects
remain unobserved. This resolves the principal question left by
[task 0017](0011-original-frame-capture-access-pilot.md), without qualifying an
original launch or changing the full faithful stable Rust with Bevy Origins goal.

Data/access understanding remains **INVESTIGATING**; overall, gameplay and visual
fidelity remain **UNKNOWN**. Three directly reached Windows/Breakpad startup
edges remain outside this bounded source closure. There is no claim that this is a complete
audit of every SDK, Windows, graphics-driver or Breakpad write.

## Scope, procedure and identities

Read the full objective, root rules, the reverse-engineering, asset-inspection and
validation skills, architecture README/ADR 0001, findings 0009-0011, handoff 0017
and existing safe Rust boundaries/tests before investigation. Bevy still owns
lifecycle, ECS, scheduling, assets, Mesh and rendering. All six crates retain
`forbid(unsafe_code)`. No tracked Rust, engine, API, dependency, tool, test,
configuration or architecture change belongs to this task.

The public source is official RenderDoc v1.46, commit
`e4bd23b671d3d5a747ff5221dbe08a63eb6ca200`. Reuse fourteen retained task 0017
source/header copies and their exact pinned URL/byte/SHA receipts. Twelve directly
relevant additional immutable source copies total 380,181 bytes, individually
below 1 MiB and collectively below 12 MiB. They cover core declarations, logging,
D3D11 device/debug ownership, Windows marker detection, CLI/build declarations
and the Breakpad crash server. Two small immutable directory receipts select the
correct Windows entry/detection files. Source fetch stages use 120-second elapsed
admission checks and 20-second socket timeouts; hard asynchronous network
cancellation is not qualified. No whole-SDK download or checkout is performed.

All helpers, sources, receipts and private traces remain in external
`0018-sdk-capture-startup-write-conditions`. The final `source-identities.json`
binds all 26 consumed source copies. The frozen `source_check_v4.py` verifies those
identities and all 26 accepted task 0017 freeze manifests: 501 references to
146 distinct files. Two incorrect source-path fetches are retained as failures.
Two preparation checks assumed a uniform receipt status field; their failures
are preserved and the final check matches exact bytes/SHA/pinned URL instead.
The initial documentation validator detected a consumed entry-point source missing
from the identity list; the corrected list includes that retained copy, with no
additional fetch or runtime action.
No failed preparation reached SDK or original execution.

No original asset, executable, snapshot, process, memory, settings/profile file,
capture or game session is read here. Prior preservation reads only the scoped
public tooling/source/helper/manifest artifacts and the user-created synthetic
capture covered by accepted freeze identities. There is no SDK load/execution,
fixture launch, native build, GUI, input or Cargo gate repetition.

## Role selection and initialization order

The role is a loader-time source predicate, not an executable-name heuristic.
The core constructor starts with `m_Replay = false`. On Windows DLL process
attach, `add_hooks` searches loaded modules for the exported replay marker.
Finding it sets replay mode before `RenderDoc::Initialise`; otherwise initialization
runs in capture mode and registers capture hooks afterward. The shell-process
early return is a separate exception. See the
[constructor](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/core.cpp#L598-L630),
[DLL entry](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_libentry.cpp#L34-L82)
and [module/export detector](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_hook.cpp#L1018-L1026).

The public [marker contract](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/api/replay/renderdoc_replay.h#L47-L53)
requires the symbol in the executable or a DLL loaded before RenderDoc. The
standard CLI [exports it](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd.cpp#L1462-L1462);
its [Windows project](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd.vcxproj#L212-L219)
links the SDK project. Thus the ordinary-name target and its bundled
`renderdoccmd.exe crashhandle` child have different source-supported roles. This
task does not inspect packaged exports or the target's loaded modules. Original
I386/PE32 identity from task 0017 does not prove that the marker is absent, that
D3D11 initializes, or that any hook succeeds.

Core [initialization](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/core.cpp#L632-L762)
runs platform helpers, capture-only environment application/target control,
default capture-template directory setup and logging, crash-handler recreation,
replay-only registered environment setup callbacks, then `ProcessConfig`.
`InitialiseReplay` is later; calling it does not establish the loader-time role.
Its [argument handling](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/core.cpp#L831-L860)
processes `--crash` after the earlier initialization has completed.

## Role, phase and write-condition matrix

`A` denotes the process's resolved RoamingAppData RenderDoc directory; `T` denotes
its `GetTempPathW` result. Neither symbol names a currently observed target path.
"Attempt" describes source control flow; success depends on permissions, sharing,
path resolution, build switches and normal lifetime completion.

| Role and phase | Configuration and parent directories | D3D11 cache | Diagnostic/TEMP/crash effects |
| --- | --- | --- | --- |
| Injected target, no replay marker; DLL initialization | Resolve `A`, create missing parents, read `renderdoc.conf`; no automatic `.tmp` write or replacement. | Not created by core initialization itself. | Default capture-template parents under `T/RenderDoc`; log open/append; ordinary eligible name constructs crash handler, starts standard CLI child and creates dump parents. |
| Same capture target; wrapped D3D11 device construction and destruction | No new configuration refresh through the inspected path. | Read `A/d3dshaders.cache`; failed load or cached successful compile makes it dirty; dirty destruction attempts direct overwrite. | Common debug resources run in capture mode too; replay-only debug resources remain guarded. |
| Standard crash child; SDK DLL initialization before command dispatch | Replay role: resolve/create parents, read config, and refresh through `.tmp` when merged settings are out of date. | No wrapped D3D11 device construction is called by the inspected crash command; this is not an exhaustive callback/driver audit. | Default replay log/template directory; executable-path substring `renderdoccmd` skips recursive crash-handler construction. |
| Standard crash child; `crashhandle` execution | Prior config refresh cannot be undone by the later `--crash`. | No cache load/save in the inspected command body. | Create `T/RenderDoc` and `dumps`; start pipe server. Dump request and nonempty dump result lead to further report/JSON/UI effects. |
| Task 0017 replay harness; SDK initialization, then D3D11 replay | Replay refresh may write config; exact harness folder/file guards remain applicable. | Wrapped replay device has the same dirty-save rule. | Harness name excludes crash construction; later `--crash` unloads the handler. This qualified role does not qualify the ordinary target or child. |

## Configuration: a read can still create directories

Every `ProcessConfig` call resolves `renderdoc.conf` and opens it read-only, imports
the XML and merges values. Only `IsReplayApp() && outofDate` opens
`renderdoc.conf.tmp` in write-binary mode, exports XML, then replaces the original
when the writer reports no error. `SaveConfigSettings` is also replay-guarded and
is not automatically called by the inspected capture startup. The read and refresh
branches are exact in [settings initialization and save](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/settings.cpp#L467-L535).
Missing setting nodes/descriptions can make the merge out of date; this task does
not observe the actual target/child's configuration content.

The [Windows resolver](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_stringio.cpp#L389-L407)
uses `SHGetKnownFolderPath(FOLDERID_RoamingAppData, 0x2100, NULL, ...)`, appends the
RenderDoc directory and filename, and calls `CreateParentDirectory` even for a
read. That helper [attempts missing parent creation](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_stringio.cpp#L242-L275).
An ordinary capture target therefore needs directory-identity/creation policy
even when its own config-file writer is unreachable. APPDATA alone supplies no
override in this resolver. The qualified harness's canonical handle/file IDs
cannot establish a different process's resolver view.

## D3D11 cache: capture ownership and deferred writes

`WrappedID3D11Device` selects replay or background-capture state, then constructs
`D3D11ShaderCache` outside that role branch. A real device also creates the debug
manager; device destruction deletes the cache. These are the
[device ownership edges](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_device.cpp#L101-L116),
[debug-manager construction](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_device.cpp#L237-L238)
and [destruction](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_device.cpp#L298-L302).
The cache constructor always tries the shared cache read and sets dirty to the
inverse of load success. Its destructor saves only when dirty; there is no replay
predicate at this [read/destruction writer](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_shader_cache.cpp#L84-L119).

The debug manager enables shader caching, initializes common resources in both
roles, adds replay-only resources under the replay guard, then disables further
caching. Common shader creation also depends on feature level. See
[debug initialization](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_debug.cpp#L57-L74)
and [common resources](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_debug.cpp#L149-L198).
A cache hit returns without dirtying; a successful compile while caching is
enabled inserts the blob and sets dirty in
[GetShaderBlob](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_shader_cache.cpp#L147-L204).
Disabling future caching does not clear an already dirty cache, including the
failed-load case. These facts do not prove the original uses D3D11, reaches device
construction, misses a shader, or runs its destructor after termination.

The [shared writer](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/common/shader_cache.h#L88-L139)
resolves the same `A`, opens `d3dshaders.cache` in write-binary mode, and writes
headers plus compressed entries directly. There is no config-style `.tmp` staging.
An open failure logs and returns. Therefore a cache guard must remain held through
all owned D3D11 device lifetimes and terminal cleanup, not merely through startup.
For the inspected crash command, a cache guard is not justified by a demonstrated
D3D11 constructor edge; it becomes necessary if that child actually creates one.

File admission bounds are not SDK allocation bounds. Configuration import
[resizes to the reader's size](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/settings.cpp#L244-L267).
Cache loading [uses declared decompressed and entry lengths](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/common/shader_cache.h#L34-L85).
Task 0017's 1 MiB config/16 MiB cache file policies do not themselves cap every
declared-size allocation in the SDK. Likewise its replay descriptor's 65,536-byte
logical binding range did not imply that the synthetic 80-byte constant buffer
was allocated or read at that range. Task 0018 performs no SDK buffer read,
decompression, allocator measurement or memory-cap qualification.

## Logs, capture-template directories and routing order

`GetDefaultFiles` derives default capture/log names from `GetTempPathW`, executable
basename and time. Core chooses `RenderDoc_app` for capture logs and `RenderDoc`
for replay logs. Setting the default capture template creates its parent directory
before logging begins; it does not capture a frame or write an `.rdc` body. See
[default names](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_stringio.cpp#L327-L370)
and [template setup](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/core.cpp#L2143-L2155).
With the relevant logging build switches enabled, default initialization opens
an append handle with `OPEN_ALWAYS` and later messages use `WriteFile`; see
[log open and append](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_stringio.cpp#L746-L758)
and [append/close deletion](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_stringio.cpp#L846-L865).
Log stripping/disk-output compile predicates are source conditions, not freshly
measured properties of a loaded binary in this task.

Changing the debug log is another writer: it opens the destination, copies a
previous log's contents when available and attempts to delete the previous path.
Normal log closure also attempts deletion. The [log switcher](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/common/common.cpp#L278-L317)
and [disk-output guard](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/common/common.cpp#L350-L355)
establish those effects. The public setter's
[implementation](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/replay/entry_points.cpp#L255-L263)
also recreates the crash handler to propagate the changed log. It is not a
pre-initialization crash-disable mechanism.

The supported injector [loads the DLL before setting capture/log/options](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_process.cpp#L973-L1003),
and [applies supplied injection environment changes afterward](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_process.cpp#L1008-L1030).
Those setters cannot protect earlier default initialization. A process environment
provided before DLL loading can route the observed TEMP consumers: Windows
[GetTempPathW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-gettemppathw)
checks TMP before TEMP, then other fallbacks, without proving existence, access or
canonical identity. The child creation below inherits the target environment.
Task 0017 already qualified this idea for its named synthetic roles; target and
ordinary crash-child path/identity/cleanup still need separate admission.

No inspected public API or resolver branch establishes a pre-init task-local
override for RoamingAppData config/cache, a public dirty-cache disable option, or
a supported remote pre-init crash-handler disable. The marker deliberately selects
a replay application and suppresses capture hooks, so it is not a target-routing
solution. `UnloadCrashHandler` is an
[in-process post-load API](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/api/app/renderdoc_app.h#L385-L390).
No environment variable, patched SDK, injected custom callback or unsupported
remote function is proposed.

## Ordinary target and crash child

The release/official Windows [crash-handler build condition and constructor](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/crash_handler.h#L26-L117)
cover the target-side effect. Core skips construction when the lowercased full
executable path contains `renderdoccmd`, plus specific browser exclusions; there
is no capture-only or replay-only guard at
[RecreateCrashHandler](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/core.cpp#L516-L567).
The target starts the SDK-directory `renderdoccmd.exe crashhandle --pipe ...`,
then creates TEMP dump parents and registers Breakpad. A failed out-of-process
connection can start a second server. Child creation uses a null environment
pointer and disables handle inheritance. The method's 400 ms readiness wait does
not establish child termination; the shown method does not close its returned
process/thread handles. These are source observations at
[server launch](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/crash_handler.h#L119-L157),
not measured leaks or actual descendants.

The child is replay-marked and name-excluded from recursive crash creation. Its
`CrashHandlerCommand` contributes `--crash`, but common dispatch appends that
argument only after parsing and then calls `RENDERDOC_InitialiseReplay` before
executing the command. Thus `--crash` cannot prevent its prior config refresh.
See [crash command](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd_win32.cpp#L558-L601)
and [dispatch order](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd.cpp#L1701-L1709).
The common CLI also [disables GPU enumeration](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd.cpp#L1530-L1534)
before that later replay call; this does not change earlier SDK initialization.

Command execution explicitly creates the TEMP RenderDoc/dumps directories and
constructs a Breakpad server with dump generation enabled. The inspected
[server Start](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/crash_generation_server.cc#L206-L250)
creates synchronization and named-pipe objects, rather than writing a dump there.
The [dump-request handler](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/crash_generation_server.cc#L898-L945)
calls `WriteMinidump` conditionally. A crash callback sets the command's dump path;
only a nonempty path takes the later ZIP/JSON creation, qrenderdoc launch/wait and
file-deletion branch in [report handling](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd_win32.cpp#L639-L765).
That UI branch matters to descendant ownership and is outside permitted runtime
work. No upload is inferred from an included HTTP header.

Completeness stops at the
[MinidumpGenerator construction](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/crash_generation_server.cc#L88-L126),
the target's Breakpad ExceptionHandler constructor and core's Win32
`Callstack::Init` implementation. Their downstream startup effects are **UNKNOWN**
in this closure. Replay environment setup callbacks and other APIs/drivers are
also not exhaustively traced. The 12 additional-source ceiling is reached;
no further edge is fetched automatically and no universal absence claim is made.

## Guards, remaining discriminator and next task

For a proven capture-mode target, config-file write denial is not required to
stop the inspected refresh/save branches; directory protection still matters.
For an ordinary target's replay-marked crash child, config and `.tmp`/replacement
protection are source-justified from startup. A shared D3D11 cache guard is justified
for capture and replay device lifetimes whenever they reach wrapped-device
construction, even before any requested frame. No cache guard is justified solely
by the inspected plain crash command. TEMP routing and own-descendant supervision
remain required for each role's directly observed logging/directory/crash effects.
Task 0017's harness guards retain their proven use; they do not prove target or
crash-child protection. Package status 15700 and APPDATA/environment labels are
weaker than process-specific canonical directory/file-ID evidence.

The smallest later runtime discriminator is the candidate process's **actual
loader-time role plus exact resolver/file identity before the first relevant
writer**, covering the separately launched standard crash child too. One
ordinary-name user-created x86 target with protected known identities, task-local
TEMP set before DLL load, bounded startup/device/teardown and owned-descendant
cleanup would discriminate these source predictions without an original game.
This is a proposed later task, not an action or a proven supported target-side
pre-init callback. A controller's or package peer's preflight cannot substitute
for the target/child's unobserved view.

The concrete next bounded task is to close the directly reached Win32
`Callstack::Init`, Breakpad ExceptionHandler and MinidumpGenerator constructor
startup edges using a small separately admitted immutable public-source closure.
Record any further writer, its role/phase/path predicate, and the minimum identity
gate needed before an ordinary-name synthetic qualification. Do not load the SDK
or original while closing those source edges. A later protected synthetic startup
qualification and an exact-target menu transport frame remain separately reviewed
tasks; neither would establish loaded Origins, patch/draw correspondence, camera
meaning, physical units or gameplay/visual fidelity.

## Validation and limits

Exact retained source identities and critical linked line ranges are corroborated
against pinned bytes. Validation passes 80 local links, 41 pinned source anchors,
21 critical predicate anchors, 27 content checks, ten exact handoff headings,
20 private-root variants, eight root-form patterns and `git diff --check`.
All 674 initial preservation records and 481 prior scoped preservation records
match. The accepted task 0017 freeze identities, five-file ownership and LF checks
also pass; actual records and candidate hashes are external and summarized in
[handoff 0018](../handoffs/0018-sdk-capture-startup-write-conditions.md).
Checks validate evidence and documentation; no branch-model tests are presented
as runtime evidence. Unchanged Cargo/Bevy/inspector gates are intentionally not
rerun; task 0014's accepted 67-test baseline remains. No live original comparison,
SDK default/config/cache log signal, captured original frame, original coordinate
mapping, architectural promotion or fidelity promotion occurs.

Outer review accepts this bounded PARTIAL source investigation after inspecting
the role, configuration, D3D11 ownership and crash-child source predicates and
independently checking terminal evidence identities, documentation and scoped
preservation. The next task closes the three named startup edges; acceptance
does not admit an SDK or original runtime action.
