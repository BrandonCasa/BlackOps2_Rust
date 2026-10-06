# Project rules

## Working set and execution

Start with [project state](docs/project-state.md), then relevant source and validation tools.
Work sequentially in this chat. **Subagents and delegated chats are banned.**
Keep one current [handoff](docs/handoffs/current.md); replace stale state instead
of accumulating audits. Git history holds retired documents and implementations.
Keep at most 20 tracked Markdown files, including the ten required skills.

## Goal

Rebuild the complete Black Ops II Zombies Origins experience in stable Rust and
Bevy. Prioritize a playable, maintainable game with all original systems.
Use licensed original data and retained research where useful; practical, tunable
reconstruction is permitted. Missing exact native arithmetic or pixel matching
must not block implementation. State meaningful approximations and missing features.
RenderDoc captures are research references, never production scene inputs.

## Architecture

One Cargo package and one executable: `origins.exe`. Every game system extends
this application/window. See [architecture](docs/architecture/README.md).
Bevy owns lifecycle, ECS, scheduling, scenes, assets, cameras, materials,
visibility, lighting, post-processing and UI. Avian supplies physics/query
facilities. Do not recreate engine infrastructure or add standalone demo/viewer
applications. Keep shared gameplay independent of Origins configuration.
Use focused modules, typed errors, fixed-step gameplay and explicit ordering.
Use stable Rust; retain `#![forbid(unsafe_code)]` and Cargo.lock.

## Assets

Prepared runtime assets may be committed only under `assets/origins/`, using
the offline importer to validate the complete dependency closure. Track glTF,
binary mesh buffers and PNG textures through Git LFS; keep `game.json` in Git.
Raw archives, original scripts/shaders, preparation inputs/intermediates,
captures, research/inspection reports and receipts remain external. Never commit
installation paths, secrets or generated builds. Real paths belong in ignored
`config/local_assets.toml`; tracked configuration contains placeholders.
Preparation reads originals and emits an external standard asset pack; import
copies only validated runtime files and never modifies originals.
Unit tests and Clippy are banned. Do not run Clippy or add it to validation gates.
Do not add or restore test suites or test fixtures. Validate with asset integrity
checks, compiler gates and the actual game window.
Inspect candidates before staging.

## Validation and Git

Implement a coherent milestone before testing it. Use targeted compile checks
only when needed; run asset validation after assembly and the full gates once
before integration. Repeat affected checks after actual failures/fixes:

```text
cargo metadata --locked --no-deps --format-version 1
cargo build --workspace --release --locked
cargo fmt --all -- --check
```

Exercise the real game window and distinguish compiled, tested, rendered and
played behavior. Do not claim original equivalence from asset integrity checks.
Review diffs and make coherent local commits. Never push unless requested.
Do not install replacement toolchains when the existing Cargo is available.

## Current handoff

Record TASK, STATUS (COMPLETE/PARTIAL/BLOCKED), FINDINGS, EVIDENCE, CHANGES,
VALIDATION, UNRESOLVED, RECOMMENDED NEXT WORK and SCOPE NOT IMPLEMENTED.
Use one concise current record, including actual failures and skipped checks.
