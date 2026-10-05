TASK:
Determine source-supported startup file-write requirements of pinned RenderDoc v1.46 for an injected Windows x86 capture target and its standard ordinary-target crash child, separately from local replay. The full faithful stable Rust with Bevy Origins objective remains active and unfinished.

STATUS:
PARTIAL
The principal role/config/cache/log/crash conditions are source-supported. Directly reached downstream Windows/Breakpad startup constructor effects and actual target/child identity remain unknown; no runtime qualification occurred.

FINDINGS:
- Capture-mode ProcessConfig reads configuration without refreshing it; replay-only refresh writes .tmp and then replaces the original after a successful writer. Every successful KnownFolder resolution can create missing parent directories, including on the read path.
- Wrapped D3D11 device construction creates the shared shader cache in capture and replay. Failed cache load or cached successful compilation marks it dirty; dirty device/cache destruction directly overwrites the cache. Common debug shader initialization is not replay-only.
- The standard crash child exports the replay marker. It can refresh configuration during SDK initialization before parsing the crash command's --crash. Its executable-path name excludes recursive crash construction. No wrapped-device edge appears in the inspected crash command body.
- Defaults create TEMP capture-template parents and diagnostic log open/append effects. Changing the debug log can copy/delete the previous log and recreate the crash handler. Injection setters/environment modifications run after DLL loading; they cannot route earlier initialization.
- Ordinary eligible targets create crash-server descendants and dump parents. The server's inspected Start builds synchronization/pipe objects; dump requests and nonempty dump results enable later dump/report/JSON/UI/deletion branches. Downstream ExceptionHandler/MinidumpGenerator/Win32 Callstack startup implementations remain outside the source ceiling.
- Config guards remain justified for replay and the standard crash child; D3D11 cache guards remain justified for capture and replay device lifetimes. Capture-only config-file refresh denial has no demonstrated writer to block, but parent directory identity/creation policy remains necessary. No supported pre-init config/cache override or remote crash disable is established.

EVIDENCE:
- docs/reverse_engineering/0012-sdk-capture-startup-write-conditions.md links exact call chains, predicates, phases, resolver/directory/buffer effects, mode matrix and completeness limits.
- Official RenderDoc v1.46 commit e4bd23b671d3d5a747ff5221dbe08a63eb6ca200; fourteen reused pinned copies plus twelve additional immutable source copies totaling 380,181 bytes. Each new source is below 1 MiB; aggregate below 12 MiB. Two directly relevant immutable directory receipts; two wrong-path fetch failures retained externally.
- External 0018-sdk-capture-startup-write-conditions retains source identities/receipts, bounded fetch helpers, source checker/freeze, preparation failure traces and scoped preservation records. Exact prior 26 freeze manifests/501 references/146 files pass without SDK/original execution.
- No new game data, original executable/header/body, original assets/snapshot/capture, process/loaded module, input, profile setting or live behavior is inspected. Task 0017's synthetic qualification and original I386/PE32 observation are prior evidence only.

CHANGES:
- Added findings 0012 and this handoff; minimal links/state updates in docs/README.md, docs/reverse_engineering/README.md and docs/project-state.md. Exactly five owned documentation candidates, LF-only.
- No Rust/API/engine/architecture/Cargo/lock/config/skill/test change. External source/evidence helpers are documentation validation tooling. No commit, push, SDK/native execution, original action or nested delegation.

VALIDATION:
- Final frozen source checker verifies all 26 source identities against immutable URLs/receipts and all 501 accepted task 0017 references to 146 files. Earlier receipt-status schema assumptions failed in preparation and were corrected without runtime access. Initial anchor validation found one consumed retained entry-point source omitted from the identity list; the correction fetched nothing and executed no SDK.
- All 674 initial and 481 prior scoped non-owned tracked/config/public-tool/source preservation records match. Five-file scope, 80 local links, 41 pinned source anchors, 21 critical predicate anchors, 27 content checks, ten handoff headings, LF/secret/whitespace, 20 private-root variants, eight root-form patterns and git diff --check pass. Terminal candidate/helper/source/evidence byte/SHA records remain external.
- Prior preservation excludes original assets and the accepted geometry snapshot. Two standard Git LF-to-CRLF warnings from the unchanged external OAT patch total 278 captured stderr bytes and are retained privately; no source mutation is inferred from them. One initial preservation-definition wrapper omitted its required argument; retained failure precedes the successful scoped baseline. Fetch source stages use 120-second elapsed admission checks and 20-second socket timeouts; asynchronous cancellation is not qualified.
- Outer review accepts the bounded PARTIAL result after independently inspecting critical source predicates, terminal evidence identities, documentation and scoped preservation. The three named startup edges remain for a separate source-only task; no SDK or original runtime is admitted.
- Unchanged Cargo metadata/test/fmt/Clippy, Bevy smoke and inspector gates intentionally not rerun. The accepted task 0014 67-test baseline is unchanged. No branch-restatement tests, SDK loading, runtime file-write observation, original comparison or performance/memory qualification is claimed.

FIDELITY STATUS:
INVESTIGATING
Original-data/access understanding only. Overall, gameplay and visual fidelity UNKNOWN; no MATCHED or VERIFIED claim. Public source conditions are not actual original compatibility or effects.

UNRESOLVED:
- Downstream startup side effects of Win32 Callstack::Init, Breakpad ExceptionHandler and MinidumpGenerator constructors, plus untraced replay setup callbacks/other API-driver paths. This is a bounded closure, not an exhaustive SDK audit.
- Actual target replay-marker absence, supported API/device initialization, target and standard crash-child canonical folder/file identity, protected pre-init route and bounded descendant cleanup. Harness/package/environment observations do not establish those facts.
- Original launch/Steam/API/hook/capture/replay compatibility, loaded Origins, static patch/draw correspondence, transform meaning, basis/front/angles/gravity and physical units remain unknown.

RECOMMENDED NEXT WORK:
- One bounded public-source task closes the directly reached Win32 Callstack and Breakpad ExceptionHandler/MinidumpGenerator constructor startup edges, recording any writer and role/phase/path predicate without runtime access. Then separately admit an ordinary-name user-created x86 protected startup/device/teardown qualification before any exact-original menu frame. Preserve the full calibration requirements from findings 0009.

SCOPE NOT IMPLEMENTED:
- SDK load/execution, fixture/native build/launch, original process query/launch/attach/injection/capture/replay/memory/input, profile observation/modification, config routing implementation, unknown-content deletion, Steam control, geometry access/conversion, coordinate calibration, rendering/gameplay architecture, gameplay/performance implementation, commit/push and nested delegation.
