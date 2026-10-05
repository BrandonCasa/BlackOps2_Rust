# Original frame-capture access pilot

Task 0017 is **PARTIAL**. The signed, portable RenderDoc v1.46 route captures and
replays a user-created D3D11 frame, and its supported API returns the exact known
input, rasterizer and constant state. The catalogued Zombies executable is
currently the expected I386/PE32 file. No original process query, game launch,
instrumentation, capture or replay occurred in this task. The game's graphics
API, Steam launch behavior, capture compatibility and loaded-map state remain
unobserved. This result does not select an Origins coordinate transform or reduce
the full Rust with Bevy goal.

The missing prerequisite is establishing the actual target's SDK startup side
effects and protecting the paths that its capture mode can write. A nominal Windows folder
resolved to a different canonical directory in this environment; the qualified
harness's handles do not establish the game's view. An ordinary executable name
also takes a crash-handler startup branch excluded by the synthetic fixture.
These are access questions, not evidence of game incompatibility or a full-goal
blocker.

## Scope and source identity

This follows [access findings 0010](0010-original-coordinate-calibration-access.md)
and [coordinate findings 0009](0009-origins-coordinate-evidence.md). The task read
the goal/rules, four relevant skills, handoffs 0014-0016, architecture/ADR 0001,
snapshot/Mesh contracts and actual core/boot/snapshot/Mesh APIs/tests. Bevy retains
engine, ECS, scheduling, assets, Mesh and rendering ownership; all six Rust crates
retain `forbid(unsafe_code)`. No tracked Rust, dependency, architecture, tool,
configuration or skill file changed. External C++ is a graphics-tool fixture, not a game engine.

The official RenderDoc v1.46 annotated tag resolves to
`e4bd23b671d3d5a747ff5221dbe08a63eb6ca200`. Existing task 0016 primary copies were
reused, with bounded directly relevant API/startup/producer additions. The
[official build index](https://renderdoc.org/builds) supplied the exact
[portable archive](https://renderdoc.org/stable/1.46/RenderDoc_1.46_64.zip),
[detached signature](https://renderdoc.org/stable/1.46/RenderDoc_1.46_64.zip.sig)
and [official public key](https://renderdoc.org/baldurk-pubkey.asc).

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| One downloaded ZIP | 101,000,153 | `9ca4d09ecaba2cc791168660d6fc2a7e3d70fe67146e87ec05c5f8dea70772f7` |
| Detached signature | 833 | `f2773031ed354f5e0c98fd87bdffdb01ceba1559776bf7e79377215f23b32077` |
| Official key | 3,113 | `29062b19e84c72e2f2d3ffa4c0578cf9627e6e710ebfa78da513b6c33471523a` |
| Packaged x86 SDK DLL | 22,266,512 | `c5aae6bd0403e8c9af599cf3977928f628ba356acbb66192def3d308d1925946` |
| Packaged x86 CLI | 518,288 | `2da6a50befef89b09be2cf1d969e379beeb485d6198d525cd0cff0af88892450` |

The task-only GPG keyring verified VALIDSIG for primary fingerprint
`1B039DB9A4718A2D699DE031AC612C3120C34695`. Five packaged executables/DLLs report
Authenticode Valid with Baldur Scott Karlsson as signer and certificate thumbprint
`F91353AE1E90CBEFF15585165D1611C5CE72E346`. This authenticates the tool artifact;
it does not establish compatibility with the game.

The policy admitted one download at most 256 MiB, network phases at most 120 seconds
with 20-second socket bounds, and extraction at most 4,096 entries/1 GiB aggregate/
256 MiB per file. Actual extraction contains 88 entries and210,237,254 bytes. Final
archive checks reject absolute/traversal/drive paths, original-name normalization,
links, case/device/trailing-character collisions and file/parent conflicts. No
global installation, keyring, PATH, registry, association or global hook changed.
The same ZIP was reused after two GPG-path/agent corrections; no second binary
download occurred. Final archive policy checks and20 synthetic cases pass.

## Current executable identity: limited original observation

Only the exact configured-root-relative `t6zm.exe` was opened. The corrected
observation confirms 12,543,488 bytes and SHA-256
`f6f7104af2bd0c2b931ea1e739969e85845e499affbf686144c6b490b344ab1e`, matching the
catalogue. Minimal fields report I386, PE32, subsystem2, image size66,289,664 and
header size1,024. These headers establish x86 routing, not the graphics API,
consumer schema or current Steam metadata. App212910, manifest24784239 and English
remain prior catalogue records; no manifest was reread.

The frozen parser resolves containment, requires a regular non-linked target,
opens read-only/unbuffered once, caps the file at16 MiB and uses two streaming
SHA passes on that same handle with 64 KiB scratch and a30-second phase bound.
Metadata reads total at most 160 bytes: DOS64, PE/COFF24, optional-prefix72.
Subtraction bounds check all offsets and the complete declared optional-header
span before reads; `e_lfanew` must be64..1 MiB and the optional span72..512 bytes.
Unsupported/insufficient/truncated headers are rejected. No sections, `.text`,
strings, signatures, constants, imports or resource bodies were inspected.

The first observation's frozen helper obtained its final metadata before the
second hash. Its header/hash result is retained but cannot prove end-of-read
metadata equality. Parent review admitted one corrected repeat. In v2 the final
hash precedes the final stat, while the handle remains open; both hashes and final
size/write-time/device/file-ID equal the initial identity. The corrected phase
exits 0 in0.125 seconds with empty stderr and terminal/readers/pipes cleanup.
Twenty-five synthetic header cases, six supervisor cases and three final-hash
metadata-timing cases pass, including an actual metadata change during the final
pass being rejected. Parent independently reproduced these gates. No further
original read or runtime observation occurred.

## Supported automation and startup boundaries

The public replay header exposes
[pre-start process injection](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/api/replay/renderdoc_replay.h#L2091-L2121),
target-control capture and local replay. The fixture uses an owned suspended child,
same-bitness x86 injection before API startup, then supported target-control APIs
to request one frame. There is no global hook or child-process instrumentation.
The release's qrenderdoc Python UI route was rejected during source inspection
because UI initialization writes preferences/analytics; qrenderdoc was not run.
The external harness instead uses the pinned public C++ header closure and DLL
with a process-local compiler environment. Failed builds/helpers/logs are retained.

The SDK
[settings initialization](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/settings.cpp#L467-L510)
reads `renderdoc.conf`; optional refresh uses `renderdoc.conf.tmp`.
[KnownFolder resolution](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_stringio.cpp#L389-L407)
uses SHGetKnownFolderPath with flags0x2100, so changing APPDATA is insufficient.
The
[D3D11 cache](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_shader_cache.cpp#L84-L119)
and its
[common loader/writer](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/common/shader_cache.h#L38-L96)
also resolve `d3dshaders.cache` there. SDK DLL initialization precedes main when
statically imported; the qualified harness therefore delays DLL loading until
native file-identity/share-lock preflight finishes.
These source paths do not prove that every capture target writes them. The
capture-mode versus replay-mode read/write conditions remain a source-level
discriminator before requiring target-side guards or an original launch.

Process-only TEMP/TMP redirect
[default SDK logs](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_stringio.cpp#L327-L370)
and supported SetDebugLogFile selects an external log after initialization.
Names containing `renderdoccmd` are
[excluded from crash-handler creation](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/core.cpp#L516-L567).
The harness and fixture use that source-supported exclusion; replay also passes
the supported `--crash` argument. An ordinary target instead
[starts a bundled crash server and creates a TEMP dump directory](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/core/crash_handler.h#L38-L157).
That branch was inspected, not exercised. Public UnloadCrashHandler is an
in-process API; no supported remote disabling was invented.

The
[alternate-bitness helper branch](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/os/win32/win32_process.cpp#L919-L970)
normally waits for its helper thread but can leave descendants on abnormal owner
exit. Same-bitness routing avoids that branch. A parent-owned kill-on-close job
covers the harness and its own descendants; no child holds a job handle that would
defeat owner-death cleanup. This does not authorize control of Steam or any
preexisting/unrelated process.

## Guard qualification and measured limits

Early SDK attempts revealed that the nominal RenderDoc directory resolved to an
exact retained canonical directory with an MSIX LocalCache/Roaming shape.
The source-only explanation is consistent with
[Microsoft's virtualization documentation](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes).
Own Python/native package-family queries return public error15700
APPMODEL_ERROR_NO_PACKAGE; package labels alone do not establish file identity.
No broad profile inventory or unknown-content deletion occurred.

Final protection anchors that exact observed directory handle/file ID, rejecting
arbitrary mapped paths and reparse ambiguity. Existing regular conf/cache files
are held read-only with READ sharing for the entire tool lifetime, denying writes,
deletion and replacement. Existing limits are1 MiB for conf and16 MiB for cache,
with exclusive external backups. Preexisting `.tmp` is rejected. Absent exact
files use task-owned CREATE_NEW/delete-on-close sentinels; no preexisting bytes
are restored or overwritten. New parent removal is allowed only when empty.
Directory write-time may change and is not claimed restored. Unknown content is
preserved. Exact existing bytes/SHA/size/write-time/device/file-ID/attributes are
checked after all owned tools terminate, while protection handles remain open.

The delayed x86 native preflight observes the DLL absent, independently opens
metadata handles and compares canonical path/volume/file ID/metadata against the
parent-held files and directory. Actual write/delete-sharing attempts fail with
Windows error32. Native CRT reads succeed for protected existing files and fail
for absence sentinels; this is measured preflight behavior. Direct SDK log signals
for default configuration or denied cache writes were not retained, so they remain
unobserved. Functional capture/replay success is separate evidence.

Final task-only tests pass 39 Windows guard cases, seven own-job cases and four
anchored native-preflight cases. Tests cover actual injected initializer
exceptions, interleaved writers, unknown concurrent directory content, failure,
deadline/output limit, abrupt owner death, partial constructor cleanup, deliberate
wrong file ID and exact16 MiB/16 MiB+1 cache boundary. The CloseHandle failure case
reports failure after an actual kernel close; it does not simulate a kernel leak.
Parent independently reproduced 39 guard-v3 cases, seven own-job-v3 cases and four
anchored native-preflight-v3 cases. The guard-v4 cache extension was worker-tested;
root's native preflight independently exercised its cache cap and reviewed final
SDK preservation. Root did not rerun capture/replay. The three-attempt terminal-before-
guard-release fail-stop branch is inspected only, not exercised; ordinary and
abnormal own-job termination are exercised.

## Actual synthetic capture/replay evidence

The user-created fixture binds a float32 three-component input,36 vertex bytes,
12 u32 index bytes and80 known constant bytes, with Back culling/frontCCW and a
320x240 viewport. It is not an Origins view. The final guarded acquisition uses
one x86 fixture launch and one frame; the existing retained frame is28,098 bytes,
SHA-256 `82f7872a17bad42d98ac3472d8c6549cdaf4a922c8e9327fa774b09b50d570e7`.
Acquisition/OpenFile/local replay/API checks succeeded, but the initial state
qualifier failed before selected-state output. Capture existence alone was never
counted as known-state qualification.

Subsequent separately reviewed replays use that same frame, without another
fixture launch or capture. Each holds a read-only capture handle and checks exact
same-handle identity before/after. Actions are capped at4,096 and numeric
combinations at32; no action names, resource exports or shader bodies are emitted.
Final replay lasts 0.500 seconds, exits 0, captures 119 process-output bytes and emits
2,083 bytes across four result JSON files, within120 seconds/8 KiB JSON plus8 KiB
process output. Captures are logically monitored at512 MiB per file,1 GiB aggregate,
4,096 files and256 KiB temporary/log output; no hard graphics-tool memory cap is
claimed. The final same-handle frame, config/cache guards and directory anchor all
pass; harness terminal/readers joined/pipes closed, own-job active0 and job/event
handles closed. The preexisting33,634-byte cache has exact before/after identity;
its producer is unknown. The task-owned absent-conf placeholder is removed.

| Observed synthetic field | Final supported replay result |
| --- | --- |
| Root/action/draw counts | 3 / 3 / 1 |
| Draw selection | Exactly one indexed, non-instanced draw;3 indices, base/index offset0 |
| Numeric action combinations | `(0,0,256)`, `(0,0,1048577)`, `(3,0,65538)`, each once |
| Input layout | One entry; slot0, offset0, three4-byte Float components, per-instance false, step0 |
| Vertex/index binding | 32 vertex slots; bound slot0 stride12/offset0; bound index stride4/offset0 |
| Rasterizer | Back, frontCCW true |
| Viewport | 16 API slots; exactly one enabled slot0, values `(0,0,320,240,0,1)` |
| Selected bytes | Vertex36, index12, constants80 match the known synthetic values exactly |
| Constant access/reflection | Two accesses; one Vertex constant descriptor; one reflected block |
| Descriptor range | Bound ConstantBuffer, offset0, logical range65,536; only80 bytes requested/returned |

Three earlier false qualifier expectations are preserved, not compatibility
failures. The
[controller owns the root-action storage](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/replay/replay_controller.cpp#L220-L224).
Regular DrawIndexed leaves `numInstances` at0, whereas instanced producer code
assigns InstanceCount in the pinned
[draw producers](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_context_wrap.cpp#L3949-L4135).
The
[producer emits all vertex slots](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_replay.cpp#L792-L806)
and a
[full viewport array with enabled flags](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_replay.cpp#L964-L972),
rather than an array of one active viewport. Finally,
[default CBCounts4096](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_context.cpp#L91-L95)
are used by
[ordinary constant binding](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_context_wrap.cpp#L817-L822);
the
[descriptor reports that logical range](https://github.com/baldurk/renderdoc/blob/e4bd23b671d3d5a747ff5221dbe08a63eb6ca200/renderdoc/driver/d3d11/d3d11_replay.cpp#L1336-L1342),
not allocated buffer length. These conventions were source-derived first and then
measured on the retained synthetic frame. They are not original camera semantics.

## Compatibility and signal matrix

| Prerequisite/signal | Evidence | Remaining limit |
| --- | --- | --- |
| Exact current licensed target/bitness | Corrected same-handle hash/header observation | No body, API or current manifest proof |
| Official tool provenance | Signed ZIP, isolated GPG, Authenticode, pinned DLL/header identities | No game compatibility implication |
| Automation/injection/frame transport | Actual user-created x86 D3D11 fixture capture | Original launch/Steam/hook behavior untested |
| Local replay/selected state | Actual retained synthetic frame, exact known36/12/80 and state | Original resource/constant meaning unknown |
| Harness config/cache protection | Actual canonical/file-ID/share denial and exact preservation | Actual game target's pre-init SDK folder view unknown |
| Ordinary-name crash startup | Pinned source child/TEMP branch | Not exercised by qualified fixture |
| Loaded Origins/control | No registered loaded-map observation; native CUA input unavailable | User navigation prerequisite unanswered; no invented view |
| Static-world correspondence/camera/basis/units | None | Requires loaded-map evidence and exact patch/draw correspondence |

## Excluded attempts, preservation and next discriminator

External `0017-original-frame-capture-access-pilot` retains sources, signed tool,
every helper version/freeze, failed build/invocation logs, synthetic fixtures,
captures, result files and scoped preservation/validation records. Identity v1's
metadata-timing defect, GPG failures, policy decoding failure, wrapper module-path
failure, first log-rotation monitor failure and second ambiguous config cleanup
are explicit exclusions. Earlier startup runs cannot prove protection of the
SDK's effective folder. A maximum16-entry side-effect observation failed before
enumeration; a separate admitted three-path metadata diagnostic read no contents
and found config/tmp absent. No unknown directory entries were enumerated or
deleted. Parent's mistaken result/manifest/output-name assertions are review
errors, not reruns or product failures.

Consumed graphics attempts are distinct: runtime v1 failed on log rotation before
capture; runtime v2 produced an excluded28,044-byte frame and failed selected-state
and directory cleanup; source-v5/output-v4 produced the retained28,098-byte frame
but failed the old instance-count predicate. Three existing-frame replays then
measured the instance convention, full viewport array and constant range in turn:
replay-only-v3 failed binding/raster (0.484 seconds,158 output bytes), v4 reached
constants (0.453 seconds,156 bytes), and v5 passed (0.500 seconds,119 bytes).
Each revision was frozen/reviewed separately; none was an automatic uncertain
retry. No original runtime attempt was consumed.

The final47-record replay freeze has SHA-256
`6ba1955dbd9a7eb4b6640bf8d8e13ef03ad2313cc4631a7c64f094519ce763b7`.
Its external335,360-byte x86 harness has SHA-256
`3c89ef733526b819623932f5653c1f92cddcd0aab3cc2db04599aac5ea1ef4f2`;
the authoritative7,894-byte result has SHA-256
`fb6a4285a00431a60dc9a4370df0c8d7229fcd6ac075bd6e4c6c03bb356e8d5c`.
Parent independently checked the final closure, capture identity, all measured
known-state fields and cleanup. Frozen identity/parser helpers were not modified
to accommodate graphics lifecycle changes.

Terminal scoped preservation passes482 existing records with only the three
owned index/state files changed:13 pristine sources,nine clean submodules,pinned
OAT/task9 restoration,official/prior-task artifacts and ignored config remain
unchanged. Final five-candidate checks pass75 local links,16 pinned source anchors,
29 content checks,ten headings,48 private forms plus eight mixed-separator root
patterns/48 synthetic privacy cases,secrets/prohibited-content/scope/whitespace
gates. All26 frozen manifests'501 references to146 distinct files match exact
bytes/SHA. Validation and preservation outputs remain external. Unchanged Cargo
gates were intentionally not repeated;task0014's accepted67-test baseline remains.

Earlier raw exception/log excerpts exposed local path forms in tool output;
identified failures are retained without asserting an exhaustive excerpt count.
Later invocations emit aggregate/type/stage only and
retain bounded private logs. Tracked candidates must pass an independent scan for
normal, escaped, forward, mixed-MSYS and canonical private-root forms. No raw logs,
original bytes, captures, geometry, shaders or actual roots enter Git.

The smallest next task first checks capture-mode versus replay-mode global
read/write conditions in pinned SDK startup consumers. For each actual target
write, establish a supported route that protects its SDK folder view before
initialization; qualify ordinary-name crash child/TEMP/owned-descendant behavior.
The inspected public
injection/target-control interface exposes capture/log/environment controls but
no proven pre-init folder override or target-context metadata callback. Do not
invent remote calls or relax containment to any mapped directory. Once that
prerequisite is established, separately review one exact-target, pre-API x86 launch
and one bounded menu frame with existing-session avoidance, finite capture/replay
limits and own-child cleanup. A menu frame can establish transport only.

An Origins static-world draw still requires registered loaded-map evidence and
exact raw-patch correspondence; visual resemblance, anonymous constants, guessed
matrices/FOV, mesh-view axes and public field labels establish no mapping. The
C2M fixed-address route remains unvalidated and was not probed. Original active-
view consumer/schema investigation remains a fallback if a supported capture
route cannot establish transform access. Full calibration, independent units,
materials/stream1, gameplay and rendering integration remain separate work.

Original-data understanding is **INVESTIGATING**; overall, gameplay and visual
fidelity remain **UNKNOWN**. No fidelity promotion, approximation, engine boundary
or original coordinate transform is approved by this access pilot.

Outer review accepts this bounded PARTIAL result after independent source and
authoritative synthetic-result review, 501 frozen artifact references, 482 scoped
preservation records, documentation/privacy checks and 20 archive-policy cases.
Parent and worker preservation identities agree exactly. A parent wrapper initially
rejected two standard Git LF-to-CRLF warnings despite successful preservation;
retained warning classification resolved that reporting issue without a rerun.
No SDK capture/replay, original read or unchanged Cargo gate was repeated for this
final review. The full Rust with Bevy objective remains active and unfinished.
