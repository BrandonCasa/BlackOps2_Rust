# Ordinary-name SDK startup qualification

Task0023 prepares one external, ordinary-name, user-created x86 RenderDoc
startup/device/teardown qualifier. Preparation is **PARTIAL**: native candidates
compile and synthetic protocol checks pass, but the Windows controller is
uninvoked, its guards/watchdog/cleanup integration is unmeasured, and direct
standard-child resolver/module observations are unavailable in the inspected
interfaces. This record admits no SDK runtime. It defines a concrete next review,
not a new sandbox, production wrapper or game abstraction.

The full faithful stable Rust with Bevy Origins objective remains unfinished.
All seven Rust crates keep `forbid(unsafe_code)`; Bevy retains App, ECS, schedules,
assets and rendering. No Rust, Cargo, configuration, skill or gameplay change
belongs to this task. Data/access remains **INVESTIGATING**; overall, gameplay
and visual fidelity remain **UNKNOWN**.

## Evidence and candidates

Reuse [startup conditions0012](../reverse_engineering/0012-sdk-capture-startup-write-conditions.md),
[Windows edges0013](../reverse_engineering/0013-sdk-windows-startup-edge-closure.md),
[fixture controls0015](../reverse_engineering/0015-windows-fixture-protection-qualification.md)
and [fresh-job inventory0016](../reverse_engineering/0016-windows-owned-job-inventory-discriminator.md).
Seventeen retained public sources are checked against their pinned URL/byte/SHA
receipts. No new SDK extraction, download, installation or source inventory occurs.
The SDK is RenderDoc1.46, source commit
`e4bd23b671d3d5a747ff5221dbe08a63eb6ca200`. Exact retained x86 release identities are:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| SDK DLL | 22,266,512 | `c5aae6bd0403e8c9af599cf3977928f628ba356acbb66192def3d308d1925946` |
| Standard CLI child image | 518,288 | `2da6a50befef89b09be2cf1d969e379beeb485d6198d525cd0cff0af88892450` |

Those are retained-release hash reads, not fresh signature verification or SDK
execution. The existing provenance remains findings0011's signed portable release.
All sources, native binaries, build commands, private paths, file IDs, logs and
machine-readable records stay in external `0023-ordinary-sdk-startup-qualifier-design`.
UTF-8/tomllib resolves only configured inspection output and rejects workspace
containment. Preparation reads no profile/config/cache contents or original data.

The current candidates are `ordinary-qualifier-v3.cpp`, its I386/PE32 executable,
`controller-windows-v3.py`, the small pure `controller-v1.py`, unchanged copies
of task0022's native/PID parser/ABI sources, and `protocol-v3.json`.
The native fixture dynamically loads the SDK and D3D11 only after its gates;
there is no SDK import or exported replay marker. It makes one hardware D3D11
device call, no fallback driver retry, draw, swapchain, capture, replay or crash
request. It releases context/device, calls post-load UnloadCrashHandler, attempts
FreeLibrary and records whether the SDK remains loaded before normal process exit.
FreeLibrary success is not proof of complete SDK unloading. RemoveHooks is not
called after API/device work; its documented ordering does not support that use.

The Windows candidate prepares held guards, one suspended launch, a three-handle
pipe inheritance list, exact-job queries/acquisition, five commands, an independent
watchdog thread and retained-job cleanup. Its `run(spec)` requires separately
reviewed admission before platform bindings or resolver/profile access. There is
no default launch, command-line runtime entry or admitted spec. This is reviewable
source preparation, not a measured Windows integration. Earlier sources/builds,
the failed lookup and all preparation receipts remain distinct and unmodified.

## Phase boundaries and effects

Here `A` is each process's actual RoamingAppData RenderDoc directory and `T` its
actual GetTempPathW directory. They are symbols, not currently observed paths.

| Boundary | Source-supported effects | Gate or observation |
| --- | --- | --- |
| Before fixture creation/resume | OS loader/CRT entrypoints precede fixture main; candidate imports no SDK. | Bind full source/build/header closure and held image/library/file fingerprints; create suspended, assign exact job, check image/creation time/membership and rehash before one resume. |
| Fixture preload record, before `load` | Fixture calls the same KnownFolder resolver flags0x2100, opens existing `A` and the three guarded producer names without creating parents; observes CWD, TEMP and image. | Controller compares opened canonical path, volume/file ID, attributes and file metadata against held guards. Reject mismatches before LoadLibrary. DbgHelp/SDK pre-existence is rejected by this candidate. |
| Inside LoadLibrary, before its return | DLL role detector runs before core initialization. Capture Callstack::Init requests SDK-adjacent DbgHelp and symbols with explicit path `.`; then default template parents/logging, crash-handler construction and ProcessConfig. | Held libraries, fixed owned CWD/TEMP and config/cache/staging guards must already exist. No app API gate can interrupt these constructors. First cooperative SDK/module receipt is after these effects. |
| Standard crash-child creation inside that constructor | Pinned SDK requests adjacent unmodified `renderdoccmd.exe crashhandle`, CREATE_NEW_CONSOLE, null environment/CWD and no handle inheritance. It is replay-marked and skips recursive crash construction by name. | Exact job inherits descendants under the admitted job policy. Acquire live members from that job only. This child is neither the fixture's cooperating child nor a separately substituted fixture. |
| Standard child's SDK initialization, before command dispatch | Replay ProcessConfig can refresh `A/renderdoc.conf.tmp` and replace conf. Default log/template parent work also occurs. Later `--crash` cannot undo earlier initialization. | Exact existing conf/cache and owned staging sentinel guards precede parent load. Child-specific pre-init resolver/CWD/module callbacks are unavailable; the route below explicitly uses a same-context inference. |
| Target Breakpad registration | First failure deletes the handler, creates a second server and retries; second failure leaves in-process fallback. Success skips fallback thread/DLL/UUID setup. | Complete identity-bound target log branch evidence supports witnessed failure or source-conditioned inferred success, never a direct success query. The short source waits do not replace the watchdog. |
| Crash command/fault handling | CLI attempts T/RenderDoc and dumps parents, then starts a server. Generator constructor stores state; later requests may create dumps and load libraries. A startup fault can request a dump after handlers are installed. | Pre-create and verify owned TEMP parents before load. Monitor owned outputs and terminate on failure/bounds. No intentional fault/dump is admitted; logical polling is not a hard disk quota. |
| `load_return`, then `bind` | Constructors have already returned. Actual selected SDK/DbgHelp identities can now be reported. | Check module identities, config/CWD/TEMP preservation and registration evidence before API binding/device authorization. This receipt cannot retroactively protect initialization. |
| `device`, then `destroy` | Wrapped D3D11 construction loads shared cache; failed load or successful cached compile dirties it. Destruction may overwrite cache directly. | One explicit hardware device creation; guards remain held through context/device releases, subsequent library release and terminal job cleanup. File-size bounds are not decompression/allocation bounds. |
| `unload`, process exit and teardown | UnloadCrashHandler is post-load; FreeLibrary can leave retained module references. Process exit/TerminateJobObject ends remaining owned execution. | Record return values, remaining module presence and exit cause; require actual terminal handles and still-open-job active0/listempty before releasing protection. |

Windows loader, CRT, DbgHelp/dependencies, symbol initialization, RPC, driver,
IPC and system-managed effects remain untraced leaves. The source also starts
capture target-control facilities; no target-control request is made by the
candidate. Review must account for that SDK startup rather than promise no
network/library effects. Neither preparation nor file identity metadata proves
universal absence of writes.

## Concrete path protection and child inference

The selected route requires **preexisting** `A` and cache, plus a reviewed conf
branch. It rejects a missing parent/cache or preexisting staging file; it does not
create cache/profile directories or restore existing bytes. A later admitted controller resolves
KnownFolder with the same flags0x2100, compares the exact previously admitted
canonical parent, and holds conf/cache read handles with READ sharing. Their
non-inheritable handles deny incompatible write/delete/replace opens. Metadata
and hashes bind what is protected; they do not supply denial by themselves.

Conf can instead use the already accepted task-owned absent-conf guard strategy.
The selectable `owned_absent` branch requires absence before CREATE_NEW, records
ownership/zero-length held identity and uses delete-on-close protection. The
fixture observes conf metadata only. Findings0011 measured CRT reads failing for
absence guards; this branch does not claim the sentinel is valid/readable SDK
configuration. Source ProcessConfig handles its read failure/default merge, and
the same staging guard blocks replay refresh. Keep the owned conf handle through
terminal preservation and remove only that sentinel afterward, confirming absence.
Use held-handle plus owned-name/removal receipts; no reopened-for-data claim.
Existing conf uses the normal held/reopened equality route.

Current profile state is unobserved. Task0017 removed its owned absent-conf
placeholder, so existing conf is not assumed and no user-created SDK configuration
is silently required. The smallest follow-up is a separately admitted named-path
metadata selection of the existing or owned-absent branch, not an SDK invocation.
If the exact parent/cache is absent too, prepare that exact accepted owned guard/
parent lifecycle as a bounded follow-up before execution admission; do not invent
a KnownFolder override or create them implicitly in this candidate.

The staging guard names exactly `A/renderdoc.conf.tmp`, the pinned refresh producer.
After an absent-before check, CREATE_NEW is the decisive ownership proof. A
task-owned zero-length, non-inheritable, delete-on-close sentinel denies the
SDK's write-binary staging open. The fixture uses a metadata-only observation
compatible with that guard; that compatibility remains unmeasured until a later
admitted run. If construction fails, retain its failure and owned-handle state.
Only the owned sentinel may be removed; preexisting bytes are never overwritten,
restored or deleted. Removal follows terminal job/member/readers and held-file
preservation receipts. Confirm sentinel absence afterward and retain unknown
entries. Directory write-time changes are not restored or claimed equal.

The resolver's CreateParentDirectory walks upward until an existing directory
and creates only missing parents. Requiring the named canonical `A` and its
source-identified parent to exist removes that ordinary missing-parent branch
under stable names. Recheck these exact names/IDs before load, each gate and
terminal release; CWD and TEMP use owned fixed existing directories. Parent
directory handles are identity observations. Task0021's attribute-anchor rename
failure is not repaired, and no shared/profile DACL mutation is authorized.

The standard child uses the target's ordinary token, null environment and null
CWD arguments in the inspected CreateProcess call. Under the explicitly admitted
same-context path hypothesis, that child inherits the guarded CWD/TEMP and uses
the same unmodified SDK/KnownFolder resolver. This supports a **source-backed
expectation**, not measured child pre-init resolver equality. Guarding those exact
existing producer files before the parent load still protects those files if the
expectation holds. A late live PID/image receipt establishes child identity and
job membership, not its earlier resolver or loaded-module view. Expected child
SDK/CWD/TEMP identities must be recorded as inferred where no direct receipt exists.

The concrete residual risk is a different resolver context or a concurrent rename/
replacement of one of these named parents between checks and path use. Do not
silently assume adversarial namespace stability or make universal ancestor locking
a condition of the full game goal. If a later route actually admits such concurrent
changes, qualify a name control for that exact parent/race first, or choose another
context with fixed existing names. Owned DACL results authorize only owned paths.
A post-run different parent or guard identity rejects the measurement; it cannot
retroactively protect a path the child resolved elsewhere.

The smallest actionable alternatives are to outer-accept this bounded same-context
inference for one user-created qualifier, or prepare one separately scoped supported
child observation/control mechanism at its first SDK initialization boundary.
The inspected APIs establish no pre-init path override or direct child callback;
APPDATA/TEMP substitution, replay markers, a patched child, injected remote calls
or the earlier cooperating fixture child do not fill that gap. Lack of that direct
receipt is a scoped evidence limit, not a statement that the whole goal is blocked.

## Registration evidence

The public app API does not expose IsOutOfProcess. The pinned constructor emits
one warning after initial failure and one error after final failure. After a
matching load-return record, retain the exact owned target log, its identity,
complete bounded bytes and phase association. An initial warning plus final
error witnesses final fallback. Initial warning without final error supports
retry-success inference; a connecting record without either supports first-success
inference only when the complete logging assumption is explicitly accepted.
Missing, stripped, truncated, ambiguous or rotated-away evidence is UNKNOWN and
does not authorize the device gate. A connection attempt, readiness event, live
CLI, API bind, absence of an error or device success alone is not registration proof.

The candidate locates one target log only inside fresh owned TEMP. A complete
file read is still weaker than an independently proven lossless log delivery
contract. Its classifications deliberately say INFERRED or WITNESSED; outer
review must accept that evidence level or select a smaller direct discriminator.
No forced registration failure, additional server, SetDebugLogFile recreation,
deliberate crash or dump is added to manufacture a branch measurement.

## Finite later budget and cleanup

`protocol-v3.json` proposes exactly one separately admitted scenario, consumed
once even if rejected. Maximum explicit process creations are four: one external
controller process, one fixture, and at most two source-predicted standard SDK
children during its single crash-handler construction. The independent watchdog
is a thread in the controller; no extra cooperating child or capacity probe exists.
OS-added members are not counted away or assumed prevented by a quota.

Body work is at most60s, teardown15s and an outer supervision admission ceiling90s.
Combined stdout/stderr is32,768 bytes, at most64 records/4,096 bytes each. Own TEMP
has at most64 entries,262,144 bytes per file and1 MiB total; receipts have1 MiB
aggregate. Inventory uses16 PID slots, at most10 stable double-snapshot attempts
per gate,200 pairs total and262,144 raw-buffer bytes. Distinct acquired members
are capped at16. Conf/cache admission is1 MiB/16 MiB, held binary admission64 MiB,
same-handle hash phases10s. These are finite logical admission/observation bounds,
not universal hard memory, disk or total-process limits. The candidate's independent
watchdog covers native startup/body; its own thread behavior is unmeasured.

Before creation, freeze the final actual controller/native/parser/protocol/spec,
build commands, compiler/linker tools, dependency/header closure and compiled
image identities. Hold and fingerprint launch image, pinned DLL/CLI/DbgHelp and
declared early modules, conf/cache/staging plus named directories. Bind canonical
paths, volume/file IDs, lengths, SHA-256, write times and attributes on the same
handles. Write an immutable prelaunch receipt before CreateProcess; rehash and
write exact image/PID/creation-time/job/held-artifact receipts before the single
ResumeThread. Preparation's candidate manifest is not this future launch receipt.

Query job flags/limits and raw buffer lengths. Request flags8200/limit8 as a
defensive setting, without assuming an enforced hard total-member cap. Stable
accounting and PID-list pairs must agree; retain ordered entries and reject
truncation, duplicates, overflow or unresolved races. Acquire only returned live
PIDs with retained handles; check exact-job membership, PID, image and creation
time, retaining failed acquisitions for cleanup. PID reuse cannot be repaired by
a global lookup, ancestor inventory or name-based kill. Inspect extra members'
image metadata only; their binaries/creator/cause remain outside scope.

On normal exit, rejection, suspended mismatch, output/deadline or SDK failure,
explicitly TerminateJobObject while its exact handle is retained. Clean an owned
unassigned suspended process by its exact creation handle, never a name/PID guess.
Wait every acquired/created owned handle, require signaled exit receipts, then
still-open-job active0 and listempty. Polling must acquire any additional live
members within bounds. Retain total/terminated counters and report historical
members missed between samples; stable empty cleanup is not historical recovery.

Require pipe EOF/readers joined, watchdog joined and private pipe closure;
serialize held and reopened file equality and named-directory predicates before
closing job/file protection. The share-exclusive staging sentinel has a held-handle
receipt and owned-name/removal receipt; it is not falsely claimed reopened for data.
Rejected/failure paths require the same terminal evidence. If terminal proof fails,
fail-stop with protection/job handles retained and an explicit unresolved receipt;
no automatic retry, guard release or completion claim. Such a guardian hold needs
outer resolution and is not a promised finite successful cleanup. Abrupt controller
death/last-handle-close historical coverage is unqualified by this candidate.

## Outer review and required repair

The outer agent accepts the scoped PARTIAL preparation, with SDK execution still
unadmitted. All134 frozen artifacts and six candidate documents match the terminal
manifest, SHA-256
`59ec43540c938e9bb519ecd4cc53c2104cad9b1d26cafa8e0a0be1cff593a7b4`.
Independent review matches289 build identities, reproduces66 copied pure tests
and four pre-admission rejections, and preserves105 non-owned records plus both
task0022 freezes. Receipts remain in external0023-ordinary-sdk-parent-review.

Source review identifies concrete repairs for this single qualifier:

- Correct the Windows extended-path prefix. An owned-file reproduction confirms
  that the current consumer leaves a prefix that changes resolved-path equality.
- Retain successful handles/descriptors before fallible identity work or later
  construction, including guards, directories, acquired processes and partial pipes.
- Report and validate DbgHelp before API-bind authorization and D3D11 before
  device authorization. The current fixture emits those identities after the gate.
- Observe bounded normal fixture exit after its final record before terminating
  remaining job members; the current controller immediately terminates the job.
- Bind target logs to actual file identity/target/phase, retain bounded raw evidence
  and enforce mandatory identities/phases in the actual Windows consumer.
- Check owned sentinel absence, report partial release accurately and test actual
  construction/output/cleanup failure paths rather than only the pure model.
- Replace optimizable admission assertions, apply bounds before source reads,
  preserve aggregate receipt accounting and make watchdog/supervision outcomes
  and time bases explicit. The proposed outer90s supervisor is not yet wired.

These findings require a bounded review/repair task before runtime admission;
they do not establish a full-goal blocker or require a generic sandbox. Existing
and owned-absent configuration branches remain unexecuted. An outer schema
diagnostic exposed private artifact dictionary keys in tool output; the limitation
is recorded, subsequent output is sanitized and tracked privacy checks retain
no exception for those paths.

## Review predicates and preparation validation

A later run requires all of the following concrete review predicates:

1. Exact final source/build/spec/artifact manifests and preserved failed preparations;
   runtime admission disabled until outer review supplies a new immutable spec.
2. Existing named resolver directories/cache, reviewed existing or owned-absent
   conf branch, absent-before owned staging guard, exact source path correspondence
   and permitted profile guard operations.
3. Explicit same-context child resolver/CWD/module inference and namespace-stability
   acceptance, or an actual supported direct alternative; no invented callback.
4. Accepted target log completeness/inference level, exact pre/post-load module
   identity comparisons and explicit handling of UNKNOWN registration evidence.
5. Review of the uninvoked Windows controller's constructor/failure cleanup,
   raw-query/record/storage bounds, watchdog and guard-release ordering. Compile
   and pure tests do not establish these Windows paths; unresolved defects must
   be repaired before execution admission.
6. One finite scenario, no automatic retry, exact suspended image admission and
   complete retained-job teardown evidence including failure cases.

Preparation compiles the x86 native candidate and x86/x64 job-ABI objects with
installed public MSVC/Windows headers. Actual source/header/tool/build hashes are
retained. Synthetic tests cover record framing, duplicate keys, PID list widths/
truncation/capacity, ordered duplicate retention, stability, PID reuse, source
binding, registration classifications, phase rejection and guard retention until
cleanup proof. Separate pre-admission checks reject false/missing admission
before loading Windows bindings. They execute no native fixture, SDK or profile
operation. [Handoff0023](../handoffs/0023-ordinary-sdk-startup-qualifier-design.md)
records actual counts and terminal artifact identities.

Task0021 remains PARTIAL for directory rename and missing historical last-close
inventory. Task0022 A remains PARTIAL with its omitted probe after the preserved
equality failure; B's measurement remains COMPLETE. Fresh flags8200/limits1/2,
distinct2/4 members including conhost.exe basenames, one B never-resumed denial1816
and acquired-member exit22/active0/listempty are unchanged. They establish neither
a universal quota nor extra-member creation attribution, SDK behavior, historical
recovery or controller-death safety. No prior scenario is repeated.

All non-owned tracked/config bytes, hashes and mtimes and both task0022 freezes
are preserved. Six-document scope, relative links, exact handoff headings,
LF/whitespace and private-root variants are checked. The unchanged74-test Cargo,
GPU/Bevy/inspector baseline is **SKIPPED**, as are SDK/native/profile runtime,
original comparison, sustained behavior and performance measurements. No fidelity
promotion follows preparation.
