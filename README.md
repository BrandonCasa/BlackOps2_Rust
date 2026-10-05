# Black Ops II Zombies: Origins in Rust

A personal-use Rust reimplementation of Origins, pursuing faithful behavior,
visuals, and reusable Zombies architecture through evidence from a licensed local
copy of the original game. Bevy is the selected engine, ECS, rendering, and
application foundation.

The repository currently contains only a dependency-free workspace scaffold and
a diagnostic executable. Bevy has not been integrated. No game assets are loaded
and no gameplay has been implemented. Every fidelity category is **UNKNOWN**.

Original assets stay external. Never commit, embed, or redistribute proprietary
assets, binaries, scripts, extracted content, generated dumps, or local installation
paths. Copy `config/local_assets.example.toml` to the ignored
`config/local_assets.toml` and replace its placeholders locally when an inspection
or asset-loading task needs configuration. The scaffold does not consume it yet.

Use stable Rust compatible with the Bevy release chosen during the engine
integration task. The scaffold uses Rust edition 2024. No toolchain or Bevy version
has been pinned; neither choice substitutes for compatibility investigation.

```sh
cargo run -p bo2-boot
cargo metadata --no-deps --format-version 1
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Rust tooling was unavailable during foundation work. These commands have not been
executed successfully, and compile/format/lint validation remains pending. Generate
and retain `Cargo.lock` with Cargo when tooling is available; do not hand-author it.

Only `crates/core` and `game/boot` are Cargo members. Other directories reserve
responsibilities, not implemented crates. See [repository rules](AGENTS.md),
[documentation index](docs/README.md), [workspace boundaries](docs/architecture/README.md),
[project state](docs/project-state.md), and [fidelity status](docs/fidelity/README.md).
