# SDK Windows startup edge closure

Task 0019 is **COMPLETE** for the three named source edges. Capture-mode
`Callstack::Init` loads and initializes the symbol helper before default logging
and configuration. Target Breakpad initialization attempts pipe registration;
success skips the in-process fallback, while failure prepares a dump name and
loads additional libraries. The crash-server `MinidumpGenerator` constructor only
stores state and initializes synchronization. Neither inspected constructor
explicitly creates a disk dump. Windows and dynamically loaded library internals
remain **UNKNOWN**; this is not an exhaustive SDK write audit or runtime proof.

The full faithful Origins reimplementation in stable Rust with Bevy remains
active. Data/access understanding is **INVESTIGATING**; overall, gameplay and
visual fidelity are **UNKNOWN**. These source conclusions extend
[findings 0012](0012-sdk-capture-startup-write-conditions.md) without promoting
original compatibility, geometry/camera mapping, appearance or gameplay fidelity.

## Scope and evidence identity

Read the full objective, root rules, the reverse-engineering, asset-inspection
and validation skills, architecture README/ADR 0001, findings 0009-0012,
[handoff 0018](../handoffs/0018-sdk-capture-startup-write-conditions.md) and
existing safe Rust boundaries/tests. The starting main commit is `a76ffdc`.
All six crates still forbid unsafe code. Bevy owns lifecycle, ECS, scheduling,
assets, Mesh and rendering; no tracked engine, Rust, dependency, tool, test,
skill, configuration or architecture change belongs to this task.

Official RenderDoc v1.46 is pinned to
`e4bd23b671d3d5a747ff5221dbe08a63eb6ca200`. Reuse ten exact retained source
copies from the accepted task 0018 receipts. Seven additional directly relevant
immutable files total 139,983 bytes, each below 1 MiB, within the eight-file/
8 MiB ceiling. No directory receipt, whole-SDK checkout or cap extension was needed.
Fetches use 120-second elapsed admission checks and 20-second socket timeouts;
hard asynchronous network cancellation remains unqualified. All seven fetches
succeeded. One source-corroboration preparation failure assumed an API export
inside the generic marker detector; its private trace and frozen helper remain
external. The corrected check corroborates the actual identifier lookup and CLI
marker, with no additional network or runtime action.
The first documentation validator mistook HTTPS schemes for drive prefixes; a
diagnostic also used the platform-default text decoder. These preparation failures
are retained, with UTF-8 decoding and bounded drive-prefix checks corrected.

All helpers, public sources, receipts and private traces reside externally under
`0019-sdk-windows-startup-edge-closure`. `source-identities.json` binds 17 source
copies to exact pinned URLs, lengths, SHA-256 values and line counts;
`critical-source-anchors.json` binds 30 reviewed spans and predicates.
`source_review_v2.py` checks those bytes, 45 task 0018 frozen files and all
26 task 0017 freeze manifests: 501 references to 146 distinct files.
The following table identifies the seven new copies; paths are public SDK paths.

| Source below `renderdoc/` | Bytes | SHA-256 |
| --- | ---: | --- |
| `os/win32/win32_callstack.cpp` | 31,113 | `848516e06c170dc4821368c9d62b0bd7fa026cc2c7fd4de2061352bc91aa195b` |
| `3rdparty/breakpad/client/windows/handler/exception_handler.cc` | 41,814 | `7290389e1145fd2f423d465a8725c8f65f89ebe4b341e05905ce5d677cd05fef` |
| `3rdparty/breakpad/client/windows/crash_generation/minidump_generator.cc` | 21,525 | `c9b57f5f2918cf4e61750c526fbb4acfd899e552f4573ca4d90a88b3158ce067` |
| `3rdparty/breakpad/client/windows/crash_generation/crash_generation_client.cc` | 13,293 | `a6df5f56679f3bd9b6f417ca1a42116cb07a6aa6efaf69d3dcfcef18d27e9857` |
| `3rdparty/breakpad/client/windows/handler/exception_handler.h` | 22,834 | `d9eb5a5c63f60f072c4ec56aa606fcca7dc213c26b1d0fb6a6ab5bb302b4a602` |
| `3rdparty/breakpad/common/windows/guid_string.cc` | 2,984 | `c3e77ff45388bf93c305cfd97bd981309ee7c10f8aae2116a505b03439345378` |
| `3rdparty/breakpad/client/windows/common/ipc_protocol.h` | 6,420 | `c65d5b3557e05e8feae0b6534f1a21290f7aad3c0aede4d0fd92f8ca43d19db9` |

No original assets, executable/header/body, accepted geometry snapshot, process,
memory, input, profile contents or original session is accessed. Preservation
reads are limited to accepted public tooling/helpers/manifests and synthetic
artifacts. There is no SDK load, native build, fixture launch, injection, capture,
replay, GUI or Cargo/Bevy gate repetition.

## Roles and ordering

Role selection precedes core initialization: the Windows
[loader](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_libentry.cpp#L34-L82) uses the
[module/export detector](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_hook.cpp#L1018-L1026);
the standard CLI contains the
[replay marker](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd.cpp#L1462-L1462). Thus an injected
capture target without that marker and its standard `renderdoccmd.exe crashhandle`
child follow different roles. An ordinary name alone does not prove marker absence.
The actual original target's loaded-module view remains unobserved.

Core [initialization order](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/core.cpp#L632-L762) calls `Callstack::Init` first,
then capture environment application, default capture-template/log setup,
`RecreateCrashHandler`, replay environment callbacks, and `ProcessConfig`.
The target's Breakpad constructor therefore follows default log initialization
and precedes configuration processing. Core initialization itself does not
construct the wrapped D3D11 device/cache; that later ownership remains as traced
in findings 0012. No new ordering of the original application's device creation
is inferred.

The [crash-handler build guard](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/crash_handler.h#L26-L117) requires release, Win32 and
an official build. [RecreateCrashHandler](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/core.cpp#L516-L567) also skips an
executable path containing `renderdoccmd` and the specified browser conditions.
It has no capture/replay-only guard. The ordinary eligible target constructs
Breakpad; the standard crash child avoids recursive construction.
The CLI's own [build condition](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd_win32.cpp#L67-L74) is
`defined(RELEASE) && RENDERDOC_OFFICIAL_BUILD`. Its
[crash command](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd_win32.cpp#L557-L600) returns replay argument `--crash`, creates
TEMP dump parents, then constructs the server. Its SDK DLL initialization and
configuration processing precede this command execution. Source build predicates
are not fresh measurements of a loaded binary in this task.

## Role, phase and direct-effect matrix

`T` means each process's actual `GetTempPathW` result; no actual path identity is
observed here. “No explicit writer” is a finding about the inspected code body,
not a promise about Windows or library internals.

| Role and phase | Predicate / direct call | Explicit effects and path | Limit or later effect |
| --- | --- | --- | --- |
| Capture target, first core initialization | `!IsReplayApp()` calls `InitDbgHelp`; once-only latch; an already loaded `dbghelp.dll` skips it. | Resolve SDK module directory, request adjacent `dbghelp.dll`, initialize symbols with `L"."` and invade-process true; memory module metadata. No explicit disk/directory writer. | DLL loader and DbgHelp internals untraced; actual module and working-directory identity unknown. |
| Replay process, including standard crash child, core initialization | `IsReplayApp()` makes `Callstack::Init` a no-op. | No symbol-helper load through this edge. | Later resolver/loaded-module calls can initialize DbgHelp/DIA and use other paths. |
| Eligible target, Breakpad registration success | Pipe supplied; `client->Register()` succeeds and sets out-of-process client. | Open existing named pipe, registration transaction/ACK, store handles; install requested exception/CRT handlers. No explicit disk-file or directory creation by this constructor. | Handler activation permits later dump requests, including faults during remaining initialization. |
| Same target, registration failure / no usable client | `!IsOutOfProcess()` fallback. | Critical section, two initially zero semaphores, conditional handler thread; bare-name `dbghelp.dll`/`rpcrt4.dll` loads; UUID and bounded `.dmp` pathname in memory. | No constructor dump creation. Loader/UUID internals unknown; later in-process dump writer uses that supplied TEMP directory. |
| Standard crash child, command execution | Non-null dump path passed to `CrashGenerationServer`; constructor creates generator. | CLI already attempts `T/RenderDoc` and `dumps`; generator stores path, null library/function pointers and two critical sections. | Generator constructor loads no DLL and creates no dump. Dump-request paths later load libraries and create files. |

## Callstack initialization: early library work, later cache paths

[Callstack::Init](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_callstack.cpp#L1112-L1118) calls
[InitDbgHelp](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_callstack.cpp#L470-L572) only in capture mode. The helper sets its
once-only latch before attempting work. If `dbghelp.dll` is already loaded, it
logs and returns false. Otherwise it derives the directory from the RenderDoc
module filename and requests adjacent `dbghelp.dll`; both x86 and x64 branches
use that same suffix. Missing directory separator, failed load or missing required
symbol exports return false. It then calls
`SymInitializeW(GetCurrentProcess(), L".", TRUE)` without checking the return
value, and attempts module metadata queries. Its eventual true return is therefore
not proof that symbol initialization succeeded.

Microsoft's [SymInitialize contract](https://learn.microsoft.com/en-us/windows/win32/api/dbghelp/nf-dbghelp-syminitialize)
describes a supplied search path and process-module enumeration. The explicit
`.` source argument identifies the process working directory; it is not the
RoamingAppData fallback assembled by the SDK. This contract does not prove that
the bundled DbgHelp performs no writes. Its DLL initialization, dependencies,
symbol acquisition/cache behavior and external Windows implementation remain
untraced leaves. The inspected SDK body itself contains no file/directory creation.

`InitDbgHelp` has a replay-only `DIA2::Init` branch, but capture startup cannot
enter it and replay startup does not call the helper through `Callstack::Init`.
The [DIA search-path builder](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_callstack.cpp#L92-L120) uses `_NT_SYMBOL_PATH` when
present; otherwise it assembles a RoamingAppData `renderdoc/symbols`/`symsrv`
path. [DIA initialization](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_callstack.cpp#L405-L411) and
[MakeResolver](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_callstack.cpp#L1130-L1143) belong to later replay/resolution calls.
The resolver's conditional [config.ini append open](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_callstack.cpp#L742-L754) is also
later. Those paths are not additional writers reached by these three startup
edges; their downstream behavior has not been audited here. Existing
RoamingAppData config/cache/directory protections from findings 0012 remain
independently justified.

## Target ExceptionHandler: registration and fallback

RenderDoc [constructs the selected overload](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/crash_handler.h#L89-L117) with a supplied
TEMP dump folder, null filter/callback/context, `HANDLER_ALL`,
`MiniDumpNormal | MiniDumpWithIndirectlyReferencedMemory`, pipe name and custom
information. The [overload](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/handler/exception_handler.cc#L61-L79) forwards to
[Initialize](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/handler/exception_handler.cc#L139-L292) with no supplied client/pipe handle.
`CrashGenerationClient` [construction](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/crash_generation_client.cc#L93-L112) stores state.
Its [Register](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/crash_generation_client.cc#L182-L195) connects and registers, closing its pipe
handle afterward. [Connection and registration](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/crash_generation_client.cc#L219-L315) use
`CreateFile(..., OPEN_EXISTING, ...)` on the caller's
[named-pipe identifier](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/crash_handler.h#L166-L168), then `SetNamedPipeHandleState`,
synchronous `TransactNamedPipe`, response validation and `WriteFile` for the
registration ACK. These are IPC operations, not creation of a disk file despite
the Win32 function names. The directly used
[protocol constructors](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/common/ipc_protocol.h#L124-L163)
only initialize fields. Registration does not itself signal a dump request.
Two pipe-open attempts and a 2,000 ms busy-pipe wait do not establish a full startup
deadline: the shown registration transaction supplies no overlapped timeout.
A later runtime qualifier needs an independent supervisor deadline.

When registration succeeds, `crash_generation_client_` becomes non-null and
[IsOutOfProcess](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/handler/exception_handler.h#L251-L251) is true. This skips all fallback thread,
DLL and path-ID setup. Failed registration enters that fallback: two zero-count
semaphores, a thread only if both handles exist, dynamic `dbghelp.dll` and
`rpcrt4.dll` loads, symbol lookups and
[set_dump_path](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/handler/exception_handler.h#L205-L209). The
[ID/path preparation](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/handler/exception_handler.cc#L1028-L1046) calls the UUID function when available,
formats the GUID using the inspected memory-only
[formatter](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/common/windows/guid_string.cc#L43-L57),
and stores a `MAX_PATH`-bounded `.dmp` string. It does not reserve or create a file.
The [thread entry](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/handler/exception_handler.cc#L383-L405) waits on the initially zero start semaphore;
ordinary construction does not request a dump. `HANDLER_ALL` installs exception,
invalid-parameter (compiler guarded) and purecall handlers in both modes.
RenderDoc retries a failed registration by deleting the first handler, starting
another server, and constructing another handler; a second failure leaves the
fallback handler. This is conditional source flow, not an observed child count.

Bare-name loads resolve under Windows loader policy, including existing modules
and dependencies; they do not establish the actual binary identity.
[Microsoft's loader documentation](https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order)
supports this distinction. Loader/DLL entrypoints, RPC UUID implementation,
Windows synchronization/IPC internals and their possible system-managed effects
are outside this source closure. Direct constructor code has no config/cache/dump
file writer or directory creator. Earlier caller directory/log effects still apply.
The inspected destructor restores handlers, frees libraries and releases thread/
synchronization resources; its shutdown wake checks `is_shutdown_` before dumping.
No additional explicit disk writer was found in that destructor body.

## Crash-server MinidumpGenerator: directory caller, lazy dump writer

The [CLI caller](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoccmd/renderdoccmd_win32.cpp#L575-L600) obtains TEMP, attempts both dump directories
and passes their non-null path with `generate_dumps = true`.
The [server constructor](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/crash_generation_server.cc#L88-L126) then creates
[MinidumpGenerator](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/minidump_generator.cc#L289-L310). Its constructor initializes null
library/function pointers, copies the path and initializes two critical sections.
There is no directory creation, file open, symbol initialization, UUID call or
library load in that constructor. Its destructor only conditionally frees already
loaded libraries and deletes the two critical sections.

The [later dump writer](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/minidump_generator.cc#L329-L409) first generates a name, then attempts
`CreateFile(..., CREATE_NEW, ...)` with write access. A full-memory dump flag adds
a second `-full.dmp` path; the inspected RenderDoc target constructor does not
request that flag. The [lazy library/name helpers](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/crash_generation/minidump_generator.cc#L563-L619) load
`rpcrt4.dll`/`dbghelp.dll` as needed for later dump generation. The target's
[in-process writer](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/3rdparty/breakpad/client/windows/handler/exception_handler.cc#L902-L916) similarly creates a dump only on the later
request path and with an available write function. These phase distinctions do
not prohibit a fault during SDK initialization after handler installation.
Conditional dump serialization, callback/report/JSON/UI/upload leaves and complete
post-crash cleanup are not expanded here; findings 0012 retain their prior limits.

## Minimum implication and next discriminator

A later protected ordinary-name synthetic x86 qualifier must establish protection
**before SDK loading**, covering both the fixture and owned crash children until
terminal cleanup. The source-supported minimum still includes process-specific
RoamingAppData config/cache/parent identity protections from findings 0012,
pre-load task-local TEMP routing for logs/template/dump parents, and owned child
supervision. The child can refresh configuration before dispatch; the target can
save a dirty D3D11 cache at destruction. Config/cache file size admission remains
separate from declared-size SDK allocation bounds; no allocation qualification
or logical binding-range measurement occurs in this task.

The newly closed edges add a concrete early-library identity gate: qualify the
fixture's working directory and actual selected DbgHelp module, and distinguish
pre-existing-module skip, symbol-helper initialization attempt, successful
out-of-process registration and in-process fallback. A successful source call does
not prove API success or module identity. No extra RoamingAppData symbol-cache or
`config.ini` guard is justified solely by `Callstack::Init`; those later resolver
paths require separate admission if invoked. TEMP dump containment must remain
active for initialization faults as well as ordinary teardown. An outer deadline
and owned descendant cleanup cannot be replaced by the source's short readiness
or busy-pipe waits.

No supported pre-init symbol-helper disable, config/cache override or remote
crash-disable route is established by these inspected entries and previously
inspected public APIs. The explicit symbol path is `.`; changing `_NT_SYMBOL_PATH`
is not an evidenced routing solution for this capture branch. This is absence
within the inspected closure, not proof of universal impossibility. No patched
SDK, invented environment variable, unsupported remote call or new routing
implementation is proposed.

The concrete next bounded task is an outer-reviewed design/admission of one
ordinary-name user-created x86 startup/device/teardown qualifier, with exact
per-process folder/file/module identity, pre-load TEMP containment, independent
deadline and owned crash-child cleanup as acceptance gates. Only after those gates
are implementable and separately admitted should its SDK runtime run. No original
game action is authorized by this source-only result. The smallest remaining
runtime discriminator is whether that controlled fixture and its standard child
resolve the protected identities and take registration-success or fallback paths;
no deliberate crash, dump generation or original injection is needed to distinguish
ordinary startup. Actual original role, compatibility and loaded Origins remain
separately unproven.

## Validation and limits

Pinned identity and predicate corroboration passed for 17 sources and 30 spans.
Task 0018's 45 frozen files and task 0017's 26 manifests/501 references/146 distinct
files remain byte-identical. Scoped preservation reuses the accepted definitions
for 483 non-owned tracked/config/public-tooling/synthetic records, excluding
original assets, accepted geometry and unrelated profile contents. Final bounded
checks cover exactly five documentation candidates, local links, pinned source
anchor ranges, ten handoff headings, private-root/content/secret exclusion, LF
bytes and Git whitespace. Standard external OAT LF-to-CRLF warning output is
retained privately and classified separately from validation errors.
The terminal validation receipt and final helper/evidence identities remain
external; [handoff 0019](../handoffs/0019-sdk-windows-startup-edge-closure.md)
records the actual results.

No branch-restatement tests were added. The accepted 67-test Rust baseline and
unchanged Cargo, Bevy and inspector gates were intentionally not repeated.
No runtime file effect was observed or source-label meaning promoted to original
semantics. Windows/library implementation effects, later resolver/dump leaves,
actual target identity and whole-SDK completeness remain UNKNOWN.

Outer review accepts the three-edge source closure after inspecting the critical
initializer/constructor predicates and independently checking source, evidence,
documentation and scoped preservation identities. The next implementation task
qualifies a Bevy graphical diagnostic for the existing raw snapshot boundary;
original startup identity/protection remains a separate prerequisite. No original
coordinate convention or fidelity status is promoted by that sequencing choice.
