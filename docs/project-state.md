# Project state

Updated: 2026-10-04.

Overall objective: faithful personal-use Origins reimplementation in stable Rust
with Bevy, reusable Zombies architecture, and external licensed assets.

Foundation task is reviewed and accepted. Local Git is initialized on
`main`, with no remote. Only `bo2-core` and `bo2-boot` are Cargo members.
Boot now runs a minimal headless Bevy lifecycle smoke check. All other
responsibility directories remain placeholders. A bounded external asset catalog
has inspected original file metadata and representative container prefixes/hashes.
No original scripts, entity definitions or runtime behavior have been inspected.
Overall, gameplay and visual fidelity remain **UNKNOWN**.

The [fastfile envelope investigation](reverse_engineering/0002-fastfile-envelope.md)
has established a conservative 312-byte signed PC metadata contract on six
samples, corroborated against immutable public OAT source. Format understanding is
**INVESTIGATING**; handoff 0005 is reviewed and accepted. No inspector or payload
decoder is implemented, and no signature or full-file validity is established.

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
- `cargo metadata --no-deps --format-version 1`, `cargo test --workspace` (two
  lifecycle/failure tests), `cargo fmt --all -- --check`, and
  `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo run -p bo2-boot -- --smoke` passed with startup=1, updates=3 and
  probe_updates=3, exiting 0. Default smoke/help exit 0; unsupported arguments
  exit 2. These are engine development checks, not original-game comparisons.
- Dependency/unsafe/asset-boundary review and `git diff --check` passed. No
  graphical or asset-loading plugin is assembled; no gameplay comparisons,
  other-platform validation or performance measurements have occurred.
- The bundled skill-creator validator accepted all ten skills using an isolated
  uv/PyYAML environment. Exact skill names, folder names and all 30 relative
  skill links/handoff anchors were checked. Skill acceptance remains unchanged.

The [Bevy build handoff](handoffs/0003-bevy-build-foundation.md) records exact
versions, checks and limits. Its code and validation are reviewed and accepted.

## External source availability

The [local asset catalog](reverse_engineering/0001-local-asset-catalog.md) identifies
one Steam-registered BO2 installation, Zombies app `212910` manifest build
`24784239`, English labels, and `zm_tomb`/`zmb_tomb` zone, sound and video
candidates. Data availability is **INVESTIGATING**; filenames do not yet prove
Origins identity or asset dependencies. Eight representative samples have recorded
SHA-256 identities; no originals were copied, decoded, extracted or modified.

Absolute installation/output roots are set only in ignored
`config/local_assets.toml`; inspection summaries stay external. The schema matches
the tracked example, and boot does not load it. The catalog/handoff is reviewed
and accepted. Code/compiler gates were not rerun for this
documentation/configuration-only task; the accepted Bevy validation above remains
the executable baseline.

## Next bounded work

1. Implement one bounded safe Rust envelope-only metadata inspector using
   findings 0002 and the accepted [handoff 0005](handoffs/0005-fastfile-envelope-evidence.md).
   Payload decoding and Bevy loader integration require separate evidence tasks.
2. Document observable shared engine/game structures and evidence-backed
   ECS/asset boundaries in later bounded tasks.

The outer agent sequences those tasks with exactly one active subagent and updates
this file after reviewing each handoff. Rendering, BO2 entity architecture, asset
loading and gameplay remain unimplemented. Reverse engineering has only the
bounded file-availability and envelope observations described above.

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
