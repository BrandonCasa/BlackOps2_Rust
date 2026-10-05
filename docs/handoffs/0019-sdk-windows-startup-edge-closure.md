TASK:
Task 0019: characterize pinned RenderDoc v1.46 Win32 Callstack::Init, target
Breakpad ExceptionHandler construction and crash-server MinidumpGenerator
construction using public source only.

STATUS:
COMPLETE for the three named source edges; Windows/library internals and
unrelated/later SDK leaves remain outside the bounded closure.

FINDINGS:
- Capture-only Callstack initialization attempts early SDK-adjacent DbgHelp load
  and symbols with explicit search path "."; replay startup skips this edge.
- Successful named-pipe registration skips Breakpad fallback DLL/thread/path-ID
  setup. Failure prepares a GUID dump pathname in memory and installs handlers;
  neither constructor explicitly creates a dump file.
- Crash-server generator construction stores the supplied TEMP dump path and
  initializes two critical sections. CLI directory creation precedes it; dump
  creation and lazy DLL loads belong to later request paths.
- Protection must precede SDK loading and span owned target/child lifetimes.
  Actual working-directory/module/path identity and registration mode remain
  the minimum runtime discriminator for a separately admitted synthetic route.

EVIDENCE:
- Official v1.46 commit e4bd23b671d3d5a747ff5221dbe08a63eb6ca200; ten reused
  copies and seven new files totaling 139,983 bytes, below eight files / 8 MiB.
- External 0019-sdk-windows-startup-edge-closure retains sources, pinned URL/
  byte/SHA receipts, 17 source identities, 30 anchored spans and frozen helpers.
- No game data, originals, geometry snapshot, process or profile content read.
  Source predicates are not measured runtime effects or original semantics.
- One mistaken marker-detector preparation assertion is retained privately;
  corrected exact-byte corroboration passes. All seven source fetches succeeded.
- Initial documentation checks had an HTTPS/drive-prefix false positive and a
  diagnostic decoding failure; retained receipts distinguish those preparation
  failures from corrected UTF-8/privacy validation and SDK runtime.

CHANGES:
- Added findings 0013 and this handoff; minimally updated docs index,
  reverse-engineering index and project state. Exactly five tracked candidates.
- No tracked Rust/Bevy, architecture, configuration, skill, dependency or tool change.

VALIDATION:
- Exact source identities/predicates pass: 17 sources, 30 anchored spans.
- Preserved task 0018's 45 frozen files; all task 0017 freeze manifests pass:
  26 manifests, 501 references, 146 distinct files.
- Scoped 483-record non-owned/config/public-tooling/synthetic preservation,
  ownership, links/source ranges, ten headings, root/content/secret/LF and
  git diff --check pass in terminal validation; receipts remain external.
- Outer review accepts the three-edge closure after independent source-predicate,
  evidence-identity, documentation and scoped preservation checks. The next
  implementation task qualifies a Bevy raw-geometry graphical diagnostic; original
  startup protection/identity remains a separate prerequisite.
- Two known external OAT line-ending warnings are privately retained/classified.
  No branch-restatement tests, SDK runtime or unchanged Cargo/Bevy/inspector gate
  repetition; accepted 67-test baseline remains unchanged.

FIDELITY STATUS:
Data/access INVESTIGATING; overall, gameplay and visual UNKNOWN. The full faithful
Origins objective in stable Rust with Bevy remains active and unfinished.

UNRESOLVED:
- Windows loader, bundled DbgHelp/dependencies, RPC UUID and system-managed
  effects are untraced implementation leaves, not universal no-write guarantees.
- Later resolver/cache and conditional dump/report/UI leaves are outside this
  three-edge closure. Actual original/fixture/child identity and API success
  are unobserved; no supported pre-init override is established in inspected APIs.

RECOMMENDED NEXT WORK:
- Outer-review one ordinary-name synthetic x86 startup/device/teardown qualifier
  design/admission with process-specific config/cache/directory/module identity,
  pre-load TEMP containment, independent deadline and owned crash-child cleanup.
  A separately admitted run should distinguish registration success/fallback
  without deliberate crash/dump or original game access.

SCOPE NOT IMPLEMENTED:
- No SDK load, native build, fixture launch, GUI/profile observation, original
  header/body/assets/snapshot/process/memory/input access, original launch/attach/
  injection/capture/replay, routing implementation, cleanup, runtime route,
  architecture/gameplay/rendering change, commit, push or nested delegation.
