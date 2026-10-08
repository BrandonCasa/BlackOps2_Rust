# Project rules

## Working set and execution

Start with [project state](docs/project-state.md), then relevant source and validation tools.
Work sequentially in this chat. **Subagents and delegated chats are banned.**
Keep one current [handoff](docs/handoffs/current.md); replace stale state instead
of accumulating audits. Git history holds retired documents and implementations.
Keep at most 20 tracked Markdown files.
Do not create, restore, load or run repository skills.

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
The user permits any necessary file types and raw inputs to be committed for
this local-only project. Track substantial binary additions through Git LFS.
Keep optional source inputs separate from the validated runtime pack. This
permission overrides narrower source-file restrictions in retained documentation
or prior skill instructions. Originals must remain untouched. Never commit
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
cargo build --workspace --locked
cargo fmt --all -- --check
```

Use optimized development builds for routine development and actual-game
validation; omit `--release` to retain debug assertions and development
diagnostics. Cargo.toml already sets opt-level = 1 for game code and 3 for
dependencies. Keep `lto = "thin"` in development, `lto = true` in release,
and `codegen-units = 16` in development / `1` in release. Development build and
profiling commands enable `dev-dynamic` (Bevy dynamic linking); release omits it.
Retain development debug assertions and diagnostics.
Launch the development-profile executable produced by this build.
Use release builds for performance measurements and release/package work.
Release uses opt-level = 3, no incremental compilation, no debug information
and no debug assertions, with `panic = "abort"`. Dependency log calls above DEBUG are removed in debug
builds. All log calls and tracing events/spans are compiled out in release;
Tracy, Tracy memory and Chrome profiling features are forbidden in release.
Release disables LogPlugin, renderer diagnostics and pipeline-error inspection.
Profiler captures use development builds only. `tools/build.ps1` defaults to Debug;
`tools/build.ps1 -Profile Release` builds and validates the release package. Compare
performance within the same build profile; existing release baselines do not
qualify development-build performance.

Exercise the real game window and distinguish compiled, tested, rendered and
played behavior. Do not claim original equivalence from asset integrity checks.
Share one actual-game validation slot across all chats and worktrees. Before
launching any validation or benchmark, check for active `origins.exe` runs and
wait until the slot is free. Never run more than one validation at a time. If
another chat starts a run while yours is active, stop only your own run and wait;
never interrupt another chat's process.
Review diffs and make coherent local commits. Never push unless requested.
Every commit message must include a clear summary and a detailed body explaining
what changed, why the change was needed, and how it was implemented. Describe
the affected systems or files, the resulting behavior, and relevant validation
results or skipped checks. Apply this requirement to all commits, including
documentation-only, merge, revert and squash commits; vague summaries or lists
of filenames alone are insufficient.
Do not install replacement toolchains when the existing Cargo is available.

## Current handoff

Record TASK, STATUS (COMPLETE/PARTIAL/BLOCKED), FINDINGS, EVIDENCE, CHANGES,
VALIDATION, UNRESOLVED, RECOMMENDED NEXT WORK and SCOPE NOT IMPLEMENTED.
Use one concise current record, including actual failures and skipped checks.
