# Project state

Updated: 2026-10-05.

Overall objective: faithful personal-use Origins reimplementation in stable Rust
with Bevy, reusable Zombies architecture, and external licensed assets.

Foundation task is reviewed and accepted. Local Git is initialized on
`main`, with no remote. `bo2-core`, `bo2-boot`, `bo2-assets` and
`bo2-asset-inspector` are Cargo members. Boot runs a minimal headless Bevy lifecycle
smoke check; assets/tooling now provide one signed-PC envelope metadata inspector.
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

The ten required repository-local skills are present under `.agents/skills/`.
Each guides one bounded responsibility with evidence, implementation, validation
and escalation requirements. The skill task is reviewed and accepted; it adds
no original-game evidence or executable functionality.

## Validation state

- Stable Rust 1.99.0, Cargo, rustfmt and Clippy were installed through official
  rustup and validated on `x86_64-pc-windows-msvc`, using the existing linker/SDK.
  Cargo bin was added once to user PATH, preserving existing entries. Existing
  desktop/terminal sessions may require restarting to refresh PATH.
- Bevy 0.19.1 is pinned centrally, with default features disabled and only `std`
  explicitly enabled. Its declared MSRV is 1.95.0; this workspace supports and
  tests the pinned 1.99.0 compiler. Cargo generated the retained `Cargo.lock`.
- `cargo metadata --locked --no-deps --format-version 1`, `cargo test --workspace`
  (25 tests: 14 parser, nine inspector and two lifecycle/failure tests),
  `cargo fmt --all -- --check`, and
  `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo run -p bo2-boot -- --smoke` passed with startup=1, updates=3 and
  probe_updates=3, exiting 0. Default smoke/help exit 0; unsupported arguments
  exit 2. These are engine development checks, not original-game comparisons.
- Dependency/unsafe/asset-boundary review and `git diff --check` passed. No
  graphical or asset-loading plugin is assembled; no gameplay comparisons,
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

## Next bounded work

1. Implement one safe Rust reader and read-only inspector for a project-owned,
   bounded diagnostic position/index snapshot contract, using synthetic fixtures.
   This prepares the accepted position/index evidence for a later external producer
   and Bevy geometry adapter. Preserve raw position values and triangle references;
   do not adopt units, axes, winding, materials or other attributes without evidence.
   No original content enters this reader task or tracked fixtures.
   A complete material/attribute import still requires separate stream1/layout
   investigation, including format 0's at-capacity cases.
2. Document observable shared engine/game structures and evidence-backed
   ECS/asset boundaries in later bounded tasks, selected from actual decoded
   structures rather than additional envelope metadata alone.

The outer agent sequences those tasks with exactly one active subagent and updates
this file after reviewing each handoff. Rendering, BO2 entity architecture, asset
loading and gameplay remain unimplemented. Reverse engineering has the bounded
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
promoted by that measurement. The next bounded task implements the diagnostic
snapshot consumer before any external geometry producer or Bevy mesh adapter.
