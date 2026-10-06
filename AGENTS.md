# Project rules

## Working set and execution

Start with [project state](docs/project-state.md), then relevant source and tests.
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

Licensed originals, extracted content, converted assets, captures and reports
remain external. Never commit proprietary assets/scripts/binaries, local
installation paths, secrets or generated builds. Real paths belong in ignored
`config/local_assets.toml`; tracked configuration contains placeholders.
Preparation tools read originals and emit an external standard asset pack.
Use synthetic fixtures in repository tests. Inspect candidates before staging.

## Validation and Git

Implement a coherent milestone before testing it. Use targeted compile checks
only when needed; run focused tests after assembly and the full gates once
before integration. Repeat affected checks after actual failures/fixes:

```text
cargo metadata --locked --no-deps --format-version 1
cargo test --workspace --release --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Exercise the real game window and distinguish compiled, tested, rendered and
played behavior. Do not claim original equivalence from synthetic tests.
Review diffs and make coherent local commits. Never push unless requested.
Do not install replacement toolchains when the existing Cargo is available.

## Current handoff

Record TASK, STATUS (COMPLETE/PARTIAL/BLOCKED), FINDINGS, EVIDENCE, CHANGES,
VALIDATION, UNRESOLVED, RECOMMENDED NEXT WORK and SCOPE NOT IMPLEMENTED.
Use one concise current record, including actual failures and skipped checks.
