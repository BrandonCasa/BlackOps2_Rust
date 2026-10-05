# Windows fixture protection qualification

Task0021 is **PARTIAL**. Four admitted scenarios measure exact owned-process/file/
working-directory/module identities, file sharing, owned DACLs, pre-creation TEMP
routing and child termination on entirely user-created Windows fixtures. File
mutation denial, inherited TEMP and termination of the exact parent/child pair are
observed. An attribute-only directory anchor permits creation and rename; job
accounting also reports more active entries than the requested limit. These gaps
prevent treating the protocol as fully qualified for later SDK admission. No SDK
or original execution, profile permission change or game/engine implementation occurs.

Overall/gameplay/visual fidelity remains **UNKNOWN**; data/access understanding
remains **INVESTIGATING**. The full stable Rust with Bevy Origins goal is unfinished.

## Question, sources and build

Read the full objective/root rules, reverse-engineering and validation skills,
architecture README, project state, handoffs0017-0020, findings0011-0014 and the
retained task0017 file guard, directory anchor, owned-job, build and supervisor
helpers. Existing source conclusions suffice; no actual capture SDK binary,
original files, profiles, accepted geometry or existing SDK runtime is accessed.
The source-defined requirements from [0012](0012-sdk-capture-startup-write-conditions.md)
and [0013](0013-sdk-windows-startup-edge-closure.md) motivate these controls.

Host: Windows11, version10.0.26340; CPython3.12.11, 64-bit controller. Installed
MSVC19.51.36260/toolset14.51.36231 and Windows SDK10.0.26100.0 build I386/PE32
console fixtures with C++17, `/W4 /WX /MT`; no tool installation occurs. The ordinary
fixture name contains no SDK crash-handler exclusion. Its test DLL is self-authored:
DllMain returns success and one export returns a constant; Windows imports remain
OS facilities. The fixture itself has
no GUI, network, user-input, graphics, SDK, game, crash or dump behavior. Pipe gates
are controller commands, not user input. It creates at most one explicit descendant;
Windows job accounting is separately unexplained.

External `0021-windows-fixture-protection-qualification` contains all source,
binaries, build versions/failures, private identities, log/ACL receipts and frozen
results. UTF-8/tomllib resolves only the configured inspection output; fresh owned
paths and opened-handle canonical identities are checked. All raw roots/SIDs stay
external. The initial build's explicit export alias failed with LNK2001/LNK1120
before execution. A corrected installed cross-toolchain build passes. A subsequent
preparation-only build adds the controller's child gate before the first run;
both earlier build artifacts remain separate. No native scenario is retried.

| Final native artifact/source | Bytes | SHA-256 |
| --- | ---: | --- |
| Ordinary x86 fixture EXE | 175,104 | `6689b030e0ca2a58a24330ec379f371355e7d03e80826073dfa68b66289f4266` |
| User-created x86 DLL | 84,480 | `ab733b8cbd82588ec56f4e7ff2b6b2fcfee5ca98a56951f247680b300b0fed26` |
| Fixture source | 9,535 | `173d8c736de43da4be9c1c686ffa8125b9f7cebac6546883e44f9bc09d5a183b` |
| DLL source | 196 | `9c2a4a645fc44506ebf7fc84c30240e64cc49af8067d9bc309bd9aa757012de9` |

Every parent launch binds the held EXE/DLL through same-handle hash/length/file-ID/
mtime/attributes before creation. The held EXE is rehashed before initial-thread
resume; each child gate rehashes both held EXE/DLL identities before its resume.
Source/build/controller
hashes are bound in each launch receipt. The pre-execution freeze SHA-256 is
`d2926095c0d133cad51a60d764a7ba81e14aa47e63210f226510d1892b235993`;
controller source SHA-256 is
`f693bbc7071e30881b1f1e330169cc25f09549c681df90bfa52dfb73132aa4bd`.
This supplies the launch-time provenance missing from task0020 without rewriting
that earlier evidence.

## Primary API evidence and interpretation

Exactly ten Microsoft primary pages are retrieved, totaling677,078 bytes, below
the10-page/8-MiB admission. Per-page reads cap at768 KiB with20-second socket
timeouts; URL/status/byte/SHA and retrieval-time receipts are frozen externally.
These are dated content snapshots, not an immutable Microsoft source revision.
Receipt manifest SHA-256 is
`cd6aa7c05ec29b6969b4f28487c35d837c387874b9c642cb669ace5a118d2fa5`.

[CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
defines conflicting-share rejection, creation modes, directory/reparse flags and
handle/error returns; [GetFileInformationByHandle](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfileinformationbyhandle)
provides volume and file-index identity with filesystem limits. The qualified
protocol compares hash and identity; it does not infer write prevention from hashes.
[File security](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights)
and [SetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-setsecurityinfo)
distinguish access checks, inheritance and handle-based DACL changes. Their contract
does not make an owner-controlled fixture DACL a universal sandbox.

[CreateProcessW](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw)
documents environment/current-directory selection and process/thread lifetime;
[job objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[assignment](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject)
and the [extended limit structure](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information)
support the ownership model. API success returns are checked; membership is queried
against the exact job, not a process name. [GetTempPathW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-gettemppathw)
selects TMP before TEMP and does not validate access; actual opened TEMP identity
is measured here. [LoadLibraryExW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibraryexw)
supports the absolute-path/search-flag experiment, not an all-module closure.

Native ABI calls are isolated externally because the safe Rust application has no
process/file protection API and this experiment needs exact Windows handles,
sharing, suspended creation, jobs and security descriptors. CPython's Windows
process wrapper supplies retained process/thread handles; ctypes declarations use
explicit pointer widths, DWORD32 and checked success conventions. Structure-size
checks pass for52-byte file info,8-byte FILETIME,64-byte basic limits,144-byte extended
limits and48-byte accounting. Kernel handles are non-inheritable where required,
owned exactly once and closed after terminal cleanup; allocated security strings
are freed with LocalFree. No production FFI wrapper or Rust unsafe exception is added.

## Four-case design and results

Each scenario has a fresh owned area, the same frozen native artifacts, a32768-byte
combined-output bound, at most64 JSON records/4096 bytes per record,1-MiB file/hash
admission,10-second hash checks, an independent controller deadline and five-second
terminal/reader waits. Native blocking calls are not asynchronously preempted;
actual results are within these budgets. No output-cap overrun or abrupt controller
death is exercised in these four cases.

| Case | Deadline | Elapsed / output | Observed result |
| --- | ---: | ---: | --- |
| 1 normal guarded parent/child | 30s | 0.157s / 2,152B | PASS; exact identities, read access, inherited TEMP, both exit0, job active0, pipe and handle cleanup |
| 2 file/directory mutations | 30s | 0.063s / 2,234B | FAIL of expected anchor-rename denial; all file/DACL denials succeed, anchor creation and rename succeed; parent exit0, job active0, cleanup |
| 3 deadline with live child | 4s | 4.031s / 2,116B | PASS for exact pair; both confirmed live before last job-handle close, both signaled afterward, pipe/handle cleanup |
| 4 image hash mismatch | 30s | 0.016s / 0B | PASS; created/assigned suspended parent rejected before resume; no body marker/report/output; terminal cleanup |

The parent is assigned and verified before its initial thread runs. Its first
fixture preflight reports process creation time/PID/bitness, image/config/cache
file IDs, current-directory/TEMP IDs, actual selected test-DLL file ID and constant
export. Only after the controller accepts that record does the fixture perform
body effects. The child is also created suspended; controller-acquired child handle,
exact image path/creation time and exact-job membership are checked before the
native parent receives permission to resume it. Child environment and identities
then match the parent receipts. This demonstrates cooperation in these fixtures,
not supported callbacks inside an already initializing SDK.

## File, directory and lifetime observations

Config is28bytes and cache27bytes, both user-created. Held read handles with READ sharing
allow both native reads. Eight native mutations—truncate, overwrite, delete and
rename for each file—fail with ERROR_SHARING_VIOLATION32. An owned config-directory
DACL rejects creating the staging `.tmp` with ERROR_ACCESS_DENIED5. A separately
owned strict-directory inheritable DACL rejects new-file write, subdirectory
creation, existing unheld-file write and delete, each with error5. Body/TEMP marker
creation and anchor-only new-file write are allowed. Actual operation records, not
the helper's expected matrix, establish these outcomes.

MoveFileExW successfully renames the attribute-anchored directory even though its
handle omits DELETE sharing. The directory retains its volume/file ID under the
new name, and the old name is absent. This contradicts the helper's expected32
error and fails the final canonical-directory predicate. The renamed directory
and13-byte allowed file are preserved. No stronger access-mask directory guard,
guarded-directory rename attempt or ancestor replacement is qualified. The
[protocol](../architecture/capture-protection-controls.md) therefore labels this
anchor as observation only.

Case2's verification checked all four same-handle/reopened file identities before
reaching the failing directory predicate, but did not serialize those end values.
A separate post-failure receipt, after terminal/pipe/handle confirmation, reopens
and exactly matches EXE/DLL/config/cache hash/length/mtime/file-ID/attributes,
confirms the renamed directory identity and restores only the two owned DACLs to
their declared synthetic baseline. It is reopened evidence, not a replacement
for a missing serialized same-handle end receipt. Cases1/3/4 retain complete
same-handle and reopened preservation receipts. Strict fixture bytes remain equal.
Directory mtime restoration is not claimed; fixture outputs are retained.

Job creation/limit setting/assignment/membership queries succeed. Normal parent+
child accounting totals4, parent-only totals2, and the deadline case reports
active4 despite the requested active-process limit2. The fixture explicitly creates
one child at most; extra accounting entries are not identified and the limit is
not accepted as effectively enforced. No broad process inventory is performed.
Static audit against installed SDK declarations corroborates all relevant host
offsets/widths, constants0x8/0x2000 and information classes1/9; x64 and x86 object-only
static_assert compilations pass. The frozen helper sets LimitFlags0x2008 and limit2.
A fresh empty owned job queries back144 limit bytes, flags8200/limit2 and48 accounting
bytes with total0/active0. Raw buffers and header hashes remain external. Original
scenario jobs have no historical limit query-back receipt; correct source/layout
and the empty-job result neither explain the discrepancy nor attribute it to Windows.
The exact acquired parent/child handles are members and live immediately before
deadline job closure, then signal within cleanup bounds; readers receive EOF and
join. Both termination exit codes happen to be0 on this host, so exit0 alone is
not used as evidence of normal completion. Last-close cases cannot query job
active0 afterward because no job handle remains. Normal cases do retain active0
before closing the job. Whole-job inventory/OS-managed effects remain unresolved.

## Validation, preservation and limits

Preparation passes one valid synthetic admission and ten meaningful rejection
cases: wrong PID, creation time, bitness, job membership, TEMP, module selection,
export value, image file ID, cache byte length and working-directory ID. ABI and
I386/PE32 build/export checks and the subsequent static/empty-job ABI audit pass.
The latter compiles objects and queries an empty job only, adding no fixture run.
These tests do not substitute for native results.
All four admitted native scenarios are consumed exactly once. The failed case and
fresh post-failure cleanup receipt remain distinct; no SDK or original attempt occurs.

Read-only verification preserves all117 task0020 frozen external artifact identities
and its manifest, without reading the accepted geometry input or any originals.
The initial104-record tracked/config baseline yields100 unchanged non-owned
records at terminal review; four existing owned indexes/state and three new docs
are the seven candidates. Prior handoffs/findings, external
source/supervisor copies, local config, Cargo/Rust/Bevy and lock stay unchanged.
Terminal validation checks candidate ownership, local/primary links, ten handoff
headings, privacy/content, LF and `git diff --check`; exact receipts remain external.
Unchanged74-test Cargo/GPU/inspector gates are intentionally not repeated.

Two terminal-validator preparation assertions were corrected without runtime work:
full-dictionary artifact comparison rejected additional timestamp fields, then a
historical worker candidate hash was mistaken for the current committed baseline.
Final prior-artifact checks compare recorded bytes, SHA and mtime individually;
current tracked preservation uses the task0021 starting-HEAD baseline. The first
failure trace filename was reused before versioning was corrected, so its raw
trace is lost; its frozen source and tool observation remain. The second trace,
fresh validation versions and this receipt limitation are retained explicitly.

No actual KnownFolder SDK resolver view, supported pre-init config/cache override,
SDK marker/registration/fallback, DbgHelp/system-module identity, loader/RPC/library
effects, original compatibility, Steam/API behavior, loaded Origins, camera/units
or gameplay/visual comparison is established. The minimum next discriminator is a
separately admitted owned-job PID-list/creation-time observation explaining the
accounting discrepancy, plus a directory-name control qualification if required
by the later route. Outer review then determines whether a concrete protected
ordinary-name SDK qualifier can be admitted. An unknown leaf is scoped uncertainty,
not authorization to execute the SDK or an invented full-goal blocker.

Outer review accepts this PARTIAL evidence after inspecting the external fixture,
guardian, build and ABI sources, all four case results and the restoration receipts.
Independent checks preserve 100 non-owned tracked/config records and all 117 prior
artifacts, verify the 11 frozen launch records and 147 task artifacts, and pass the
seven-document link, privacy, heading, LF and whitespace checks. No native scenario
or unchanged Rust/GPU gate was repeated. The next task is a bounded owned-job
PID-list/accounting discriminator; neither SDK nor original execution is admitted.
