# Project state

Updated: 2026-10-04.

Overall objective: faithful personal-use Origins reimplementation in stable Rust
with Bevy, reusable Zombies architecture, and external licensed assets.

Foundation task is reviewed and accepted. Local Git is initialized on
`main`, with no remote. Only `bo2-core` and `bo2-boot` are Cargo members.
Boot is diagnostic only; all other responsibility directories are placeholders.
Bevy is selected but not integrated. No original game data has been inspected.
Overall and system fidelity remain **UNKNOWN**.

## Validation state

- Git and Python are available. Rust `cargo`, `rustc`, and `rustup` are unavailable
  on PATH; no toolchain was installed.
- The outer agent confirmed an available Visual Studio x64 linker and Windows
  SDK. This does not replace the missing Rust toolchain or a successful build.
- Cargo manifests and the example configuration were structurally checked using
  Python's TOML parser; workspace paths and the single dependency direction were checked.
- Git ignore behavior and tracked candidates were checked for local asset/output,
  build, configuration, and secret exclusions. Example configuration remains trackable.
- `cargo metadata`, test, format, and Clippy execution remain pending. No successful
  Rust compile, diagnostic run, or Bevy validation is claimed.
- `Cargo.lock` remains pending generation by Cargo. No gameplay comparisons or
  performance measurements have occurred.

## Next bounded work

1. Create the ten required bounded skills under `.agents/skills/`.
2. Resolve Rust toolchain availability through a dedicated setup task and
   run the pending workspace validation.
3. Locate/catalog licensed external game data without copying it into Git.
4. Document observable shared engine/game structures, then integrate a compatible
   Bevy application and evidence-backed ECS/asset boundaries in bounded tasks.

The outer agent sequences those tasks with exactly one active subagent and updates
this file after reviewing each handoff. Rendering, entity architecture, asset
loading, gameplay, reverse engineering, and toolchain installation were outside
the foundation task.

Review: the outer agent inspected all source/configuration and substantive
documentation, independently parsed the manifests/configuration, checked key
ignore rules, and scanned candidates for local paths and unsafe/embedded content.
Structural validation passed; compiler validation remains pending as stated.
