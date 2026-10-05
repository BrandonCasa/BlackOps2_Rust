# Project state

Updated: 2026-10-05.

Overall objective: faithful personal-use Origins reimplementation in stable Rust
with Bevy, reusable Zombies architecture, and external licensed assets.

Foundation task is reviewed and accepted. Local Git is initialized on
`main`, with no remote. `bo2-core`, `bo2-boot`, `bo2-assets`,
`bo2-asset-inspector`, `bo2-map-inspector`, `bo2-rendering`, and
`bo2-geometry-viewer` are Cargo members.
Boot runs a minimal
headless Bevy lifecycle smoke check; assets/tooling provide a signed-PC envelope
metadata inspector and a project-owned diagnostic geometry snapshot consumer.
Other responsibility directories remain placeholders. A bounded external asset catalog
has inspected original file metadata and representative container prefixes/hashes.
No original scripts, entity definitions or runtime behavior have been inspected.
Overall, gameplay and visual fidelity remain **UNKNOWN**.

The [fastfile envelope investigation](reverse_engineering/0002-fastfile-envelope.md)
has established a conservative 312-byte signed PC metadata contract on six
samples, corroborated against immutable public OAT source. Format understanding is
**INVESTIGATING**; handoff 0005 is reviewed and accepted. The conservative
envelope-only inspector is implemented with bounded reads, typed redacted errors
and synthetic tests; [handoff 0006](handoffs/0006-fastfile-envelope-inspector.md)
is reviewed and accepted. No payload decoder, signature verification or
full-file validity is established.

The [transport-framing investigation](reverse_engineering/0003-fastfile-block-framing.md)
has repeated bounded noncontiguous metadata scans through the same six files.
Plaintext length fields and zero suffixes give exact byte accounting; chunk bodies
remain unread except for separate identity hashing. Authentication hash state is
internal to the corroborating decoder, not another stored header. Findings and
[handoff 0007](handoffs/0007-fastfile-block-framing-evidence.md) are reviewed and accepted.
The framing task added no Rust scanner/decoder or authenticated/decoded asset evidence.

The [decoded directory investigation](reverse_engineering/0004-origins-asset-directory.md)
has now listed the single `zone/all/zm_tomb.ff` twice through a digest-verified
external OAT v0.33.0 release. Both runs expose the same 7,549 entries across 31
types, including one `gfxworld` and related map-world families. Reference markers
are counted, but dependencies/decoded properties are not established. Findings
and [handoff 0008](handoffs/0008-origins-asset-directory.md) are reviewed and accepted.
No tracked decoder, extraction or Bevy asset-loading integration was added.

The [GfxWorld surface-boundary investigation](reverse_engineering/0005-gfxworld-surface-boundary.md)
has repeated aggregate observations of that same single world through separately
instrumented pinned external OAT source. Its 6,469 surfaces' tested triangle/index
spans partition all 449,721 index elements with zero gaps/repeats, and every surface
material pointer joins to one of 280 unique non-reference-marked material pool
entries. Vertex interpretation remains unestablished: 6,236 surfaces have zero
declared vertices despite nonzero triangles, and the tested stream-1 starting
offset fails for 12 surfaces. This is no corruption conclusion or render-readiness
claim. Findings and [handoff 0009](handoffs/0009-gfxworld-surface-boundary.md) are
reviewed and accepted. Structure availability remains **INVESTIGATING**; no tracked
decoder, unsafe exception, asset loader, geometry or rendering API was added.

The [vertex-stream investigation](reverse_engineering/0006-gfxworld-vertex-stream-boundary.md)
now admits every surface, including all 6,236 zero-count cases, under a public
Black Ops II extractor's 36-byte stream-0 addressing candidate. All 320 group
extents fit their next boundary and align exactly at 16 bytes; 191,259 records
plus 1,492 padding bytes account for the complete stream. Group firstVertex
intervals partition the logical capacity in a different order. The 285 zero-only
groups and 35 positive-only groups reject a simple count-owner sentinel; selective
count population remains unexplained. Stream1's twelve earlier failures are
positive-count surfaces starting exactly at capacity, all format0; required
stream1 spans and all attribute encodings remain unverified. Findings and
[handoff 0010](handoffs/0010-gfxworld-vertex-stream-boundary.md) are reviewed and accepted.
Task9 tooling is restored byte for byte, and separate task10 evidence remains
external. No Rust/Bevy geometry or rendering integration is added.

The [position-prefix investigation](reverse_engineering/0007-gfxworld-position-prefix.md)
now checks every candidate record and surface reference. All 191,259 addressed
records are finite little-endian float32 triples, and all 449,721 reference visits
fit their own surface's separate bounds-array endpoint candidate exactly, across
all seven observed formats and all 6,236 zero-count surfaces. The embedded tris
endpoint candidate fails on every surface; the bounds families are not interchangeable.
The predeclared four-ULP diagnostic changes no outcomes. Findings and
[handoff 0011](handoffs/0011-gfxworld-position-prefix.md) are reviewed and accepted.
Task9 tooling is restored exactly and task10 remains unchanged. Position consistency
supports a candidate prefix without proving original layout/draw semantics, units,
appearance or complete geometry support. No Rust/Bevy API or import is added.

The [diagnostic geometry snapshot v1 contract](architecture/diagnostic-geometry-snapshot.md)
and safe std-only reader now validate synthetic finite XYZ positions, explicit u32
indices, ordered surface triangle partitions, opaque source category/count metadata
and separate own-surface endpoint containment. Values/order are preserved, including
unreferenced/equal positions, degenerate triangles and zero source counts. Fixed
count/byte/allocation/reference/read-retry policies precede body reads; typed errors
redact paths/raw fields. `bo2-map-inspector` reads one explicit regular snapshot
file read-only and emits bounded counts/categories/status. Task 0012 is implemented
with synthetic fixtures; [handoff 0012](handoffs/0012-diagnostic-geometry-snapshot-reader.md)
is reviewed and accepted. Task0012 added no original snapshot, external producer,
Bevy Mesh/asset loader, renderer or original comparison. The format is ours,
not a BO2 disk format; task0013's subsequent bridge is recorded below.

The [external diagnostic snapshot bridge](reverse_engineering/0008-origins-diagnostic-geometry-snapshot.md)
now preserves all 191,259 candidate group records, 449,721 ordered references and
6,469 surfaces, including all 6,236 zero-count cases, in two identical 4,404,568-byte
v1 files. Exact source-bit readback, independent section/reference checks and each
safe Rust consumer agree on complete counts/categories/metadata/bounds. All tooling
and derivative geometry stay external; task9 source/header/exe/exact patch are
restored and task10/task11 remain unchanged. [Handoff 0013](handoffs/0013-origins-diagnostic-geometry-snapshot.md)
is reviewed and accepted for this bounded bridge. Rust with
Bevy remains the engine/game implementation; no tracked code, Bevy Mesh/loader,
renderer, units/draw/appearance proof or original-game comparison is added.

The [diagnostic Bevy Mesh adapter](architecture/diagnostic-mesh-adapter.md) now
consumes the accepted snapshot into Float32x3 position/U32 index TriangleList
storage with MAIN_WORLD usage and returns all ordered surface metadata unchanged.
All three vectors transfer ownership without another full geometry copy. Synthetic
tests and one complete accepted external snapshot check demonstrate actual Bevy
Assets<Mesh> registration/retrieval/removal and exact data preservation. Rendering
adds member-specific asset/mesh features; assets stays std-only, and core/boot/
inspectors remain unchanged. [Handoff 0014](handoffs/0014-bevy-diagnostic-mesh-adapter.md)
is reviewed and accepted. No original coordinate mapping, draw/material
semantics, asynchronous loader, graphical output or fidelity comparison is added.

The [coordinate evidence investigation](reverse_engineering/0009-origins-coordinate-evidence.md)
is PARTIAL: pinned Bevy destination conventions and C2M's OBJ scale/Face choices
are distinguished from original units/up/basis/angles/front state. Two complete
accepted-snapshot analyses preserve own-bounds inclusion under nine conflicting
mappings/scales. That ambiguity supports no graphical conversion. No new original
decode, native mutation, Rust/API/transform or renderer is added.
[Handoff 0015](handoffs/0015-origins-coordinate-evidence.md) records the remaining
original calibration and validation limits; outer review accepts this bounded
investigation with its PARTIAL result and unresolved coordinate semantics.

The [original calibration access investigation](reverse_engineering/0010-original-coordinate-calibration-access.md)
is PARTIAL: pinned C2M asset access lacks executable/build validation and an active
camera schema. Public OAT camera/matrix/cull declarations provide leads without
original consumers. Official pinned RenderDoc sources provide a conditional
per-draw inputs/constants/front/cull access candidate, without this game's actual
capture/replay compatibility. One exact-name check finds no current Zombies process;
native CUA capture is unavailable. No original executable, memory, container or snapshot
is opened, and no telemetry/renderer/API is added. [Handoff 0016](handoffs/0016-original-calibration-access.md)
records the access matrix and next bounded same-build frame-access pilot, with an
original active-view consumer/schema task as fallback;
original units/up/basis/angles/front and the full calibration remain unresolved.
Outer review accepts the PARTIAL access investigation after independent source,
preservation, documentation and supervisor-provenance checks. No original read,
live availability check or unchanged Cargo gate was repeated. The capture pilot
below investigates that candidate without establishing a calibration route.

The [original frame-capture access pilot](reverse_engineering/0011-original-frame-capture-access-pilot.md)
is PARTIAL and reviewed and accepted by the outer agent. A signed portable x86 RenderDoc route captures
and replays a user-created D3D11 frame with exact known input/raster/36-byte vertex/
12-byte index/80-byte constant state and reflection. Corrected same-handle minimal
identity/headers confirm the catalogued I386/PE32 target. No original process
query,launch,instrumentation,capture or replay occurred. Actual target SDK folder
routing and ordinary-name crash startup remain unqualified;capture-mode versus
replay-mode side effects need source investigation before original admission.
[Handoff 0017](handoffs/0017-original-frame-capture-access-pilot.md) preserves consumed
attempts,source-backed contract corrections,guard/job evidence and exact limits.
Complete synthetic access supplies no original compatibility,Origins view or
coordinate/fidelity claim. No tracked Rust/Cargo/architecture change occurred;
task 0014's accepted 67-test baseline is retained without rerunning unchanged gates.
Outer review independently accepts the exact synthetic result, all 501 frozen
artifact references, 482 preservation records, 75 local links, 16 pinned source
anchors, privacy/content/heading checks and the 20 archive-policy cases. The
482-record parent and worker identities agree exactly; two standard Git line-ending
warnings were classified without repeating preservation. No original read, SDK
operation or unchanged Cargo gate was repeated during this final review.

The [SDK capture startup write investigation](reverse_engineering/0012-sdk-capture-startup-write-conditions.md)
is reviewed and accepted as PARTIAL. Pinned source distinguishes capture config
reads from replay-only refresh, while the resolver can create parent directories
in either role. D3D11 device/cache ownership and common shader initialization also
apply to capture; dirty cache destruction can write the shared cache. The standard
crash child exports the replay marker and can refresh config before parsing --crash.
Default logging, TEMP/template/dump directories and conditional crash-report/UI
effects are traced. Directly reached Windows/Breakpad constructor edges and actual
target/child resolver identity remain unknown within the bounded source closure.
[Handoff 0018](handoffs/0018-sdk-capture-startup-write-conditions.md) records the
role/phase/predicate conclusions and validation. No SDK, original or Cargo runtime
action occurs; six safe Rust crates and Bevy boundaries remain unchanged. Data/access
understanding remains INVESTIGATING; overall, gameplay and visual fidelity UNKNOWN.
The full stable Rust with Bevy Origins objective remains active and unfinished.

The [Windows startup edge closure](reverse_engineering/0013-sdk-windows-startup-edge-closure.md)
is reviewed and accepted as COMPLETE for the three assigned public-source edges.
Capture startup requests adjacent DbgHelp and initializes symbols with "." before
log/config setup; replay startup skips it. Breakpad pipe registration success
skips fallback DLL/thread/path-ID setup. The generator constructor stores state
without loading libraries or creating a dump; caller TEMP directory creation and
conditional later dump writes remain distinct. Windows/library internals and
actual process/module/path identities stay UNKNOWN. [Handoff 0019](handoffs/0019-sdk-windows-startup-edge-closure.md)
records bounded source/preservation/documentation validation. No SDK or original
runtime occurred; the 67-test baseline, six safe crates and Bevy ownership are
unchanged. The full stable Rust with Bevy Origins objective remains active;
data/access INVESTIGATING and overall/gameplay/visual UNKNOWN.

The [offscreen geometry diagnostic](architecture/diagnostic-geometry-render.md)
is reviewed and accepted. Explicit synthetic/single-snapshot modes
use the existing reader/CPU Mesh adapter, add render-world usage while retaining
CPU data, and render three chosen raw-coordinate views to external images.
Neutral unlit solid, orthographic framing and built-in one-pixel triangle-edge
overlay are diagnostic choices; original axes, units, visible side, camera,
materials and appearance remain unresolved. Actual Vulkan extraction/readback,
independent synthetic projection/internal-edge checks and the one accepted
snapshot identity pass. Solid baseline and an outer-authorized overlay extension
are separately retained; two successful accepted runs use the same unchanged
snapshot. An external CPU check explains18 raw-X line-only/subpixel additions
without claiming exact original or per-face raster equivalence.
[Handoff 0020](handoffs/0020-bevy-offscreen-geometry-diagnostic.md) records74 passing
workspace tests, locked gates, image identities and preservation. Outer review
checked all source and documentation, all six overlay images, the bounded CPU
explanation, 15 candidate identities, 114 local links and 117 frozen external
artifact identities. Final code hashes match the passing gate receipts; broad
Rust gates and GPU runs were not repeated. All seven local crates forbid unsafe
code; no original game/SDK action or fidelity comparison is introduced.

The ten required repository-local skills are present under `.agents/skills/`.
Each guides one bounded responsibility with evidence, implementation, validation
and escalation requirements. The skill task is reviewed and accepted; it adds
no original-game evidence or executable functionality.

The [Windows fixture protection qualification](reverse_engineering/0015-windows-fixture-protection-qualification.md)
is reviewed and accepted as PARTIAL. Four finite self-authored x86 cases establish
held-file mutation denial, owned DACL creation/write denial, parent/child TEMP and
identity admission, suspended mismatch rejection and exact-pair last-job-handle
cleanup. Attribute-only directory anchors allow creation and rename; accounting
reports active4 despite requested limit2. Installed-header static assertions and
empty-job flags0x2008/limit2 query-back pass without explaining prior accounting.
[Handoff 0021](handoffs/0021-windows-fixture-protection-qualification.md) and the
[external protection protocol](architecture/capture-protection-controls.md) retain
failures, actual limits and next admission gaps. No SDK/original/profile/geometry
access or tracked code/config/dependency change occurs; the74-test foundation is
unchanged. Data/access INVESTIGATING and overall/gameplay/visual UNKNOWN.

Task0021 outer review inspected fixture/guardian/build/ABI source and all four
case receipts, independently checked 100 non-owned tracked/config records, 117
prior artifacts including timestamps, 11 launch-source/build records and 147
frozen task artifacts, and passed all seven candidates' 108 local links, ten
primary links, privacy/LF/heading/whitespace checks. The allowed directory rename,
missing case2 serialized end-handle values and unproven unidentified-entry teardown
remain explicit. No additional native case or unchanged Rust/GPU gate was run.

## Validation state

- Stable Rust 1.99.0, Cargo, rustfmt and Clippy were installed through official
  rustup and validated on `x86_64-pc-windows-msvc`, using the existing linker/SDK.
  Cargo bin was added once to user PATH, preserving existing entries. Existing
  desktop/terminal sessions may require restarting to refresh PATH.
- Bevy 0.19.1 is pinned centrally, with default features disabled and only `std`
  explicitly enabled centrally; rendering additionally enables asset/mesh features
  at its member boundary; the viewer locally enables PBR/rendering and PNG support.
  Its declared MSRV is 1.95.0; this workspace supports and
  tests the pinned 1.99.0 compiler. Cargo generated the retained `Cargo.lock`.
- `cargo metadata --locked --no-deps --format-version 1`, `cargo test --workspace`
  (74 tests: retained67 plus seven diagnostic camera/preservation/CLI/output cases),
  `cargo fmt --all -- --check`, and
  `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo run -p bo2-boot -- --smoke` passed with startup=1, updates=3 and
  probe_updates=3, exiting 0. Default smoke/help exit 0; unsupported arguments
  exit 2. These are engine development checks, not original-game comparisons.
- Boot still assembles no graphical plugin. Headless adapter checks assemble
  Bevy's asset and mesh plugins with an in-memory source; the separate finite
  viewer now qualifies actual Bevy graphical extraction/readback on local Vulkan.
  Dependency/unsafe/asset-boundary and whitespace reviews pass for task 0020;
  no gameplay comparisons,
  other-platform validation or performance measurements have occurred.
- The inspector passed all six findings-0002 external samples, with expected
  metadata and ranges, exit 0 and matching disk stems. Its envelope reads totaled
  1,872 bytes. Separate before/after full-stream hashes (12 passes), lengths and
  write times matched the accepted sample identities and showed no mutation.
  The validation driver and metadata JSON remain in the configured external output.
  Those checks establish metadata support, not authentication or gameplay fidelity.
- Framing review independently reproduced chunk counts, ranges, stream rotation,
  suffix alignment and exact accounting across all six samples, reading 67,211
  structural bytes. Twelve additional hash passes and same-handle file metadata
  confirmed unchanged accepted identities. Public source and official release
  metadata corroborate the next external listing route. Boundary filler remains
  source-only evidence; authentication and decoded content remain unverified.
- The bundled skill-creator validator accepted all ten skills using an isolated
  uv/PyYAML environment. Exact skill names, folder names and all 30 relative
  skill links/handoff anchors were checked. Skill acceptance remains unchanged.
- Verified external OAT version/help and two one-file LIST runs exited 0 with
  zero warnings/errors, within 120-second/8-MiB capture limits. Ordered entries,
  type totals and source/binary identity checks agreed; no dump outputs occurred.
  Observed peak working set was approximately 391.6 MB; no hard decoder memory
  cap is claimed. Parser framing/type-table checks and four external synthetic
  cases passed. Signature acceptance is a source-supported OAT result, not an
  independent authenticity or installation-integrity verification.
- Outer review verified the archive and all 162 unpacked package files, checked
  immutable primary sources, and independently reproduced two matching listings
  and both synthetic supervisor limits. All 7,549 ordered entries and selected
  map identifiers agreed; source and binary identities remained unchanged.
  Synthetic output-cap/deadline self-checks also terminated their children and
  recorded incomplete/timeout without opening originals.
- The separately instrumented external OAT Release/Win32 target built successfully
  from pinned source/submodules after process-local prerequisite/header corrections.
  The build's initial 15-minute bound was extended with outer-agent authorization
  to 30 and at most 40 total minutes while the same target progressed. All 26 native
  synthetic arithmetic/null/signed/truncation/policy/reference/coverage checks passed.
  The adapted supervisor passed its 256-KiB combined-output and 0.1-second synthetic
  deadline checks, then two one-input metadata observations exited 0 within the
  unchanged 120-second/256-KiB decode limits (1.172/0.671 seconds, 2,541 bytes each).
  Observed peak working sets were 391,979,008/391,254,016 bytes; no hard decoder memory
  cap is claimed. Aggregate equality, source/binary preservation and no dump/extra
  output checks passed. Complete vertex byte spans and original-game comparison
  remain unverified; unchanged Cargo gates were not repeated.
- Outer-agent review independently reran all 26 native synthetic checks and both
  supervisor selfchecks, then repeated two metadata observations (0.719/0.688
  seconds, 2,541 bytes each). All four observations agree exactly; originals and
  both binaries remain unchanged, and no unexpected process outputs were created.
  The pinned checkout, nine clean submodules, eleven pristine source files,
  scoped instrumentation, helper identities and documentation boundaries passed
  independent review. Checkout line endings were normalized only for comparison
  with Git blobs; recorded working-file hashes matched exactly.
- Task0010's 22 new native synthetic checks and the accepted 26 checks passed.
  The initial same-target build completed in 224.781 seconds within 900 seconds;
  same-host follow-ups completed within 15 seconds. One header-only build emitted
  stale keys and is excluded from follow-up validation; translation-unit changes
  forced fresh-key builds. The authoritative final observation pair exited0 in
  1.203/0.719 seconds with 6,163 bytes each, under 120-second/256-KiB limits.
  Aggregate equality, group/index/byte/category accounting, original/tool
  identities, supervisor selfchecks and no-extra-output checks passed. Task9's
  exact source/header/binary hashes and patch were restored; eleven pristine
  sources and official binary remain unchanged. Native memory remains uncapped;
  no vertex bytes or original-game comparison were used. Cargo gates remain
  unchanged and were not repeated for this documentation-only task.
  Outer-agent review also reran 22+26 native cases, supervisor limit children and
  two final observations (0.656/0.672 seconds, 6,163 bytes each); all four final
  aggregates agree. Final review verified nine clean pinned submodules, all thirteen
  recorded source identities (eleven upstream files and two generated loaders),
  nine helper/artifact hashes, the public C2M blob against its pinned tree, and
  task9's restored source/header/binary/exact patch. Documentation/content,
  source-link and whitespace checks passed; encoding and fidelity remain unknown.
- Task0011's 46 new position-prefix synthetic cases and accepted 22+26 checks
  passed before originals. The final same-host native build exited 0 without
  warnings in 14.625 seconds; cumulative 46.312 seconds including two type corrections
  stayed within 900 seconds. Two fresh schema 11 observations exited 0 in 7.797/7.407
  seconds, 5,751 bytes each, within 120 seconds/256 KiB and the 8,192-byte record cap.
  Exact repeated aggregates, complete group/unique-address/reference categories,
  admitted reads, supervisor limit children, no-extra-output and original/four-tool
  identities passed. Peaks 391,479,296/390,627,328 bytes were observed, not capped.
  Task9 source/header/binary/exact patch restored; 13 pristine source identities and
  nine clean pinned submodules checked; task10 unchanged. No original-game comparison
  or tracked-code change occurred; Cargo gates were not repeated. Outer review
  independently reran all 46+22+26 native checks, supervisor limit children and
  two observations (5.375/5.406 seconds, 5,751 bytes each). All four records agree
  exactly; original/four-tool identities and no-extra-output checks passed.
  Nineteen external artifacts, thirteen pristine source identities, eleven upstream
  Git blobs, nine clean pinned submodules and exact task9 restoration passed review.
  All 46 relative links, six source anchors, ten handoff headings, content boundaries
  and whitespace checks passed. Accepted position consistency establishes no
  original-game behavior or appearance fidelity.

The [Bevy build handoff](handoffs/0003-bevy-build-foundation.md) records exact
versions, checks and limits. Its code and validation are reviewed and accepted.

Task 0012's synthetic consumer checks now pass: locked no-deps metadata, 61 workspace
tests (25 new parser, 2 map-inspector unit, 9 map-inspector integration and 25 existing),
formatting and warning-denying Clippy. Bevy smoke remains 1/3/3 with exit 0; snapshot
help exits 0. The CLI's two additional 136-byte external synthetic runs each emitted
1,070 bytes, with exact repeat/count/category/redaction and unchanged hash/write
time. The parent also reports two matching independent 4,404,568-byte full-count
synthetic CLI runs, 1,162 output bytes each and unchanged same-handle identity.
Outer review independently passed all 61 workspace tests, locked metadata,
formatting, Clippy, Bevy smoke and snapshot help, with nine functional-file hashes
unchanged during verification. All 14 candidates, 48 relative links, ten handoff
headings, dependency/unsafe/content boundaries and whitespace checks passed.
The final binary also reproduced the independently encoded full-size synthetic
fixture's complete counts and unchanged identity; records remain external in
`parent-review-020124/`. This bounded consumer task is reviewed and accepted.
These are diagnostic consumer checks only; no
original geometry or graphical/gameplay comparison has occurred. See handoff 0012
for the fixed policies, actual read/emit boundaries and remaining limits.

Task 0013's complete external producer passes 129 new native synthetic checks plus
46+22+26 accepted checks, synthetic C++ to Rust/Python proof, and the fresh native
build without warnings. Its authoritative pair exits 0 in 0.750/0.750 seconds,
2,436 capture bytes each; both 4,404,568-byte snapshots have SHA-256
`ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f`.
Complete source-bit/section/category/accounting matches both Rust reader results
(0.062/0.078 seconds, 1,138 bytes each). Capture/deadline/artifact/unexpected-output/
rename-race/exception cleanup tests pass. One earlier monitoring-race attempt is
excluded and explicitly invalid; repaired supervision precedes fresh original
loads. Before/after source/tool identities and exact task9 restoration hold;
13 pristine sources, nine clean pinned submodules and task10/task11 are unchanged.
Parent independently passes all 223 native checks and seven supervisor selfchecks,
then reproduces two equal full snapshots (native 0.782/0.750 seconds, Rust
0.078/0.078 seconds). All four authoritative outputs and aggregates agree, with
source/tool preservation. Parent preservation and all30 frozen task13 helper/
artifact identities also pass independent review. Parent content/link/heading/
whitespace review passes for all five documentation files; this bounded bridge is
accepted. Unchanged Cargo gates intentionally not rerun. Data understanding INVESTIGATING; overall,
visual and gameplay fidelity UNKNOWN. No graphical/gameplay comparison occurred.

Task 0014 passes locked no-deps metadata, all 67 workspace tests (61 retained plus
six new adapter/engine cases), formatting, warning-denying Clippy, unchanged Bevy
smoke 1/3/3 and both inspector help commands. Five functional-file hashes match
before/after final gates. Active feature-graph review distinguishes asset/mesh/image
support from lock-only optional/target entries; no renderer/window/PBR is active.
The one accepted 4,404,568-byte snapshot with SHA-256
`ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f`
passes three complete streaming bit/reference/metadata comparisons after conversion,
engine retrieval and removal: 191,259 positions, 449,721 indices, 149,907 triangles,
6,469 surfaces, 6,236 zero source counts and seven categories. The external harness
exits 0 in 1.406 seconds with 477 stdout bytes and empty stderr, under 120 seconds/
256 KiB capture. Input SHA/length/write-time/file identity, native source/helpers
and non-owned tracked files remain unchanged. No original fastfile/native producer
is accessed; GPU/appearance/original-game checks remain unperformed. Parent
independently reran all frozen final gates and the complete harness successfully
in parent-review-072632 (harness0.250seconds,477 stdout bytes,empty stderr), with
input/harness/functional identities unchanged. Parent dependency and 188-record
non-documentation preservation review pass, along with all ten candidates,
56 relative links, seven pinned source anchors, ten handoff headings and content/
whitespace checks. This bounded diagnostic adapter is reviewed and accepted.

Task 0015's snapshot-only external checks pass 24 synthetic parser/numeric cases,
48 signed-permutation/96 face-order cases and six supervisor cleanup cases.
An excluded buffered-handle/mmap rehash failure is preserved and reproduced with
synthetic data; corrected unbuffered same-handle hashes and exception cleanup pass.
Two frozen authoritative runs exit 0 in 3.031/3.000 seconds, 695 stdout bytes each,
empty stderr, complete accepted counts and all nine own-bounds violation counts
zero. Source SHA/length/write-time/device/file-ID agree with the accepted task 0014
identity. No original-game/render/unit comparison occurs. Final 230-record
preservation and five-document checks pass; unchanged Rust/Cargo/Bevy/inspector
gates retain task 0014's accepted 67-test baseline and are intentionally not rerun.
Parent independently reproduces all frozen synthetic/supervisor cases, its own
mmap regression and two matching observations (3.000/2.969 seconds, 695 stdout
bytes, empty stderr), with accepted input identity unchanged. Parent confirms
230 preservation records, 84 independent non-owned tracked/config identities,
four frozen helpers, 31 artifact byte/SHA identities, pinned sources, five
candidates, 75 relative links, 12 source anchors, ten headings and content/
whitespace checks. The coordinate investigation is reviewed and accepted with
PARTIAL semantics; no original mapping or fidelity claim is added.

## External source availability

The [local asset catalog](reverse_engineering/0001-local-asset-catalog.md) identifies
one Steam-registered BO2 installation, Zombies app `212910` manifest build
`24784239`, English labels, and `zm_tomb`/`zmb_tomb` zone, sound and video
candidates. Data availability is **INVESTIGATING**; filenames do not yet prove
Origins identity or asset dependencies. The catalog recorded eight representative
SHA-256 identities without copying, decoding, extracting or modifying originals
at that stage.

The subsequent directory task decoded one fastfile in memory through external
OAT LIST, without dumping asset contents. Original files remain unchanged; raw
listings and the released tool remain external. The surface-boundary task built
separate pinned external tooling and inspected that same decoded world's aggregate
descriptors/index consistency/material pointer matches. Only permitted summaries
enter the repository; vertex/index values, material names/pointer addresses, bodies and
scripts remain unemitted by that metadata adapter. No originals were copied.

Absolute installation/output roots are set only in ignored
`config/local_assets.toml`; inspection summaries stay external. The schema matches
the tracked example, and boot does not load it. The catalog/handoff is reviewed
and accepted. The catalog task did not rerun compiler gates because it changed
only documentation/local configuration. Current executable checks are listed above.

The [owned-job inventory discriminator](reverse_engineering/0016-windows-owned-job-inventory-discriminator.md)
now supplies two fresh job measurements with live flags8200/limits1/2. Accounting2/4
agrees with distinct2/4 acquired member IDs; extra images report conhost.exe
basename, without causal or historical attribution. B's one never-resumed probe
assignment fails with quota error1816; A's probe is omitted after a preserved
controller assertion. The full qualification is PARTIAL. All acquired2/4 members
are terminal after explicit retained-job termination; still-open-job accounting
active0/listempty, EOF/readers/handles and held/reopened file preservation pass.
[Handoff0022](handoffs/0022-windows-owned-job-inventory-discriminator.md) records
32 synthetic checks, ABI/build validation and exactly two consumed scenarios.
General quota rules, historical task0021 last-close inventory and its directory-name
failure remain unproven. No SDK/original action, unsafe change or fidelity promotion
occurs; unchanged74-test Cargo/GPU/inspector gates are intentionally not repeated.
The outer agent reviewed and accepted this scoped PARTIAL result. Independent
raw-buffer/member/launch/teardown/preservation checks pass; all242 frozen worker
artifacts and six candidate identities match before outer documentation edits.
The [ordinary SDK qualifier design](architecture/ordinary-sdk-startup-qualification.md)
and [handoff0023](handoffs/0023-ordinary-sdk-startup-qualifier-design.md) now prepare
that concrete protocol with compile-only x86 candidates, synthetic checks and an
uninvoked Windows controller. Outer review accepts the PARTIAL preparation with
specific repairs required before execution. Frozen artifact/candidate/build
identities,66 copied pure tests and four pre-admission rejections pass independent
review; the canonical-prefix mismatch is independently reproduced on an owned file.
Existing cache, a selectable existing/owned-absent conf guard and an owned
absent-before staging guard precede loading; current profile state is unobserved.
standard-child path/CWD/module correspondence uses an explicit same-context source
inference. Registration success has no public direct query; complete target logs
support inferred success or witnessed fallback. Namespace/log-delivery assumptions
and actual controller/watchdog/guard/teardown behavior remain unmeasured. No SDK,
native fixture or profile runtime is consumed, no prior scenario is repeated, and
the unchanged74-test Cargo/GPU/inspector baseline is intentionally SKIPPED.
All seven safe Rust crates and Bevy ownership remain unchanged; data/access stays
INVESTIGATING and overall/gameplay/visual fidelity UNKNOWN. The concrete next step
is outer review of exact candidates and those scoped admission predicates before
any separately admitted startup/device/teardown runtime.

## Next bounded work

1. Repair task0023's concrete ordinary-name x86 qualifier: canonical paths,
   partial-construction ownership, pre-effect module gates, normal-exit observation,
   identity-bound raw evidence and accurate admission/budget/cleanup reporting.
   Test its actual controller path and compile the corrected native fixture without
   runtime. Then review exact source/build manifests, same-context child inference
   and target-log evidence before separate execution admission. Qualify
   a directory-name control only for a named race required by that route.
   Preserve task0021's four cases and task0022's two cases/A probe omission; no
   automatic scenario repeat. Historical inventory/general quota rules and actual
   SDK identities remain unproven. Avoid a generic process-control detour.
2. Separately apply accepted protections to the ordinary-name user-created x86
   startup/device/teardown qualifier with process-specific config/cache/directory
   and actual working-directory/module identity protection, pre-load TEMP routing,
   independent deadline and owned crash-child cleanup. Distinguish registration
   success/fallback under those protections before any separately admitted SDK run.
   A supported pre-init config/cache override is not established
   within the inspected APIs; no universal impossibility is claimed. The outer agent
   reviews that evidence before any exact-target x86 menu capture/replay admission.
   A menu frame proves transport only; loaded Origins evidence and
   exact patch/draw correspondence precede the asymmetric-patch camera/visible-side
   calibration in findings0009. C2M's fixed
   asset addresses and public field labels are not validated access. Original
   active-view consumer/address/schema work is the fallback for missing transform
   access;synthetic capture/replay is qualified but original compatibility remains
   untested. Original
   gravity/basis/angle/front state and an independent unit reference remain
   unresolved; diagnostic ambiguity selects no transform. Asynchronous AssetLoader and complete
   attribute/material import remain separately scoped; stream1/layout investigation
   includes format0's at-capacity cases. Original geometry remains external and
   never enters tracked fixtures.
3. Document observable shared engine/game structures and evidence-backed
   ECS/asset boundaries in later bounded tasks, selected from actual decoded
   structures rather than additional envelope metadata alone.

The outer agent sequences those tasks with exactly one active subagent and updates
this file after reviewing each handoff. Diagnostic graphical output now exists;
original rendering semantics, BO2 entity architecture, asynchronous asset loading
and gameplay remain unimplemented. Reverse engineering
has the bounded
file-availability, envelope, transport-framing and single decoded-directory
observations described above; no original-game behavior or appearance comparison.
The surface-boundary observation adds aggregate index/material relationships and
explicitly unestablished vertex interpretation; it does not establish renderability.

Review: the outer agent inspected all source/configuration and substantive
documentation, independently parsed the manifests/configuration, checked key
ignore rules, and scanned candidates for local paths and unsafe/embedded content.
Structural validation passed. The outer agent also reviewed all ten skill
procedures and independently reran the bundled validator for each skill.
Bevy review included all source changes and independent locked metadata, workspace
tests (two passed), formatting, Clippy and executable smoke checks; all passed.
Asset review independently confirmed aggregate file counts/lengths, all three
fastfile prefix observations, one representative hash, external configuration
boundaries and exclusion of real source/output paths from tracked documentation.
Envelope review independently confirmed marker/version/auth-marker/flags/name
fields and bounds across all six samples and inspected cited primary format
sources. The support contract remains intentionally narrower than a full decoder.
Inspector review covered every new source/test and the format/tool boundary.
Independent locked metadata, 25 workspace tests, formatting, Clippy, Bevy smoke,
inspector build and six real-sample CLI checks passed. Sample hashes, lengths and
write times were unchanged; paths and opaque bytes remain excluded from output.
Framing review covered findings/handoff 0007, source corroboration, the optional
scanner contract and the external-tool recommendation. Independent sample scans,
identity checks, documentation links/content boundaries and whitespace checks
passed. Rust/compiler gates were not repeated for this documentation-only task.
Directory review covered findings/handoff 0008, the external observation and
supervisor helpers, release contents, listing framing and source interpretations.
Independent listing/identity/budget checks, documentation links/content boundaries
and whitespace checks passed. No tracked Rust or dependency changes required
new Cargo gates.
Surface review covered findings/handoff 0009 and every substantive external
instrumentation/check/supervisor change. Independent synthetic, repeated original
observation, identity, documentation link/content and whitespace checks passed.
Index/material boundaries are accepted as scoped observations; universal vertex
slices and complete stream spans remained unestablished at that stage. The next
bounded task investigated per-surface vertex-stream interpretation before an adapter.
Vertex-stream review covered findings/handoff 0010, the source-supported candidate,
all final instrumentation/supervisor/test changes, revision-specific observations
and preservation records. Independent checks accepted the complete candidate
group accounting and logical-base intervals without promoting attribute encoding
or original draw semantics. The next task measures candidate positions against
finite-value and declared surface-bounds checks before import.
The subsequent position-prefix task supplies complete finite/containment evidence
and rejects the embedded-bounds endpoint candidate. Its findings/handoff 0011,
external instrumentation, synthetic tests, repeated observations and preservation
records passed outer review and independent reproduction; no fidelity status is
promoted by that measurement. The diagnostic snapshot consumer is now implemented
on synthetic inputs and passed outer review before any external geometry producer
or Bevy mesh adapter.
