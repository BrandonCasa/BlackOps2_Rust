# Project state

Updated: 2026-10-04.

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
listings and the released tool remain external. No payload field values, geometry
buffers or scripts have been copied into the repository.

Absolute installation/output roots are set only in ignored
`config/local_assets.toml`; inspection summaries stay external. The schema matches
the tracked example, and boot does not load it. The catalog/handoff is reviewed
and accepted. The catalog task did not rerun compiler gates because it changed
only documentation/local configuration. Current executable checks are listed above.

## Next bounded work

1. Investigate the one listed `gfxworld`'s surface triangle-range to world-draw
   buffer/material-reference boundary. Stock OAT LIST exposes names only and
   the release has no registered T6 `GfxWorld` dumper; separately scope an external
   pinned-decoder metadata adapter for permitted aggregate/range/reference
   observations, with synthetic checks and source-preservation limits.
2. Document observable shared engine/game structures and evidence-backed
   ECS/asset boundaries in later bounded tasks, selected from actual decoded
   structures rather than additional envelope metadata alone.

The outer agent sequences those tasks with exactly one active subagent and updates
this file after reviewing each handoff. Rendering, BO2 entity architecture, asset
loading and gameplay remain unimplemented. Reverse engineering has the bounded
file-availability, envelope, transport-framing and single decoded-directory
observations described above; no original-game behavior or appearance comparison.

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
new Cargo gates. The next task remains one bounded decoded geometry observation.
