# Project state

Updated: 2026-10-04.

Overall objective: faithful personal-use Origins reimplementation in stable Rust
with Bevy, reusable Zombies architecture, and external licensed assets.

Foundation task is reviewed and accepted. Local Git is initialized on
`main`, with no remote. Only `bo2-core` and `bo2-boot` are Cargo members.
Boot now runs a minimal headless Bevy lifecycle smoke check. All other
responsibility directories remain placeholders. No original game data has been
inspected. Overall and system fidelity remain **UNKNOWN**.

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

## Next bounded work

1. Locate/catalog licensed external game data without copying it into Git.
2. Document observable shared engine/game structures and evidence-backed
   ECS/asset boundaries in bounded tasks.

The outer agent sequences those tasks with exactly one active subagent and updates
this file after reviewing each handoff. Rendering, BO2 entity architecture, asset
loading, gameplay and reverse engineering remain unimplemented.

Review: the outer agent inspected all source/configuration and substantive
documentation, independently parsed the manifests/configuration, checked key
ignore rules, and scanned candidates for local paths and unsafe/embedded content.
Structural validation passed. The outer agent also reviewed all ten skill
procedures and independently reran the bundled validator for each skill.
Bevy review included all source changes and independent locked metadata, workspace
tests (two passed), formatting, Clippy and executable smoke checks; all passed.
