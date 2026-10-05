# 0001: Bevy foundation

Status: accepted engine direction and reviewed headless implementation.

## Context and evidence

The user explicitly requested Rust with Bevy. Original game architecture has not
been investigated. The dependency-free diagnostic scaffold had no executable
validation because Rust tooling was unavailable.

Version investigation on 2026-10-04 used the
[crates.io release metadata](https://crates.io/api/v1/crates/bevy/0.19.1),
`cargo info bevy@0.19.1 --verbose` and the downloaded crate manifest. The latest
non-yanked stable release was 0.19.1, declaring Rust 1.95.0. Version 0.20.0-rc.2
was a prerelease and excluded. The [Bevy setup guide](https://bevy.org/learn/quick-start/getting-started/setup/)
was used as background; release metadata resolved the exact version.

The official [Rust stable channel](https://static.rust-lang.org/dist/channel-rust-stable.toml)
and rustup selected 1.99.0, released 2026-10-01. The compiler and Cargo report
1.99.0 on `x86_64-pc-windows-msvc`. Rustup uses the normal per-user installation,
with only its Cargo bin directory added to user PATH; existing native build
tools were reused. No machine path is part of the repository.

## Decision

Use Bevy as the engine/application, ECS, scheduling, and rendering foundation.
Build reusable Zombies systems and an Origins-specific layer using its facilities.
Do not create competing ECS or renderer foundations. Pin Bevy exactly to 0.19.1
centrally with `default-features = false` and `features = ["std"]`. Pin stable
Rust 1.99.0 with rustfmt and Clippy; this is the repository's supported, tested
compiler baseline, not a claim that 1.95.0 cannot compile the locked dependency
tree. Retain Cargo's generated lockfile. Neither nightly nor dynamic linking is
needed for the headless foundation.

Core composes [Bevy's MinimalPlugins](https://docs.rs/bevy/0.19.1/bevy/struct.MinimalPlugins.html)
with a caller-selected [ScheduleRunnerPlugin](https://docs.rs/bevy_app/0.19.1/bevy_app/struct.ScheduleRunnerPlugin.html)
and returns the actual Bevy app for higher-layer plugin assembly. Boot adds a
private synthetic smoke plugin, using Startup, Update and Last, then returns
[AppExit](https://docs.rs/bevy/0.19.1/bevy/app/enum.AppExit.html) as the process exit
result. Bevy's released source was checked for minimal plugin membership and
runner finish/cleanup/update/exit handling. No application scheduler is added.

## Consequences and validation

Future engine-facing tasks first inspect Bevy's relevant facilities and original
game evidence. Add adapters only when a documented gap requires one. Selecting
Bevy and passing development checks do not demonstrate original-game fidelity.
The smoke count and pause remain boot-local test policy; fixed-step gameplay
timing requires original measurements. Graphical facilities, external assets and
game systems will be added only through later bounded tasks with evidence.
See the [build handoff](../handoffs/0003-bevy-build-foundation.md) for actual
compile, test, format, lint and runtime results. Original fidelity is UNKNOWN;
evidence-backed component/system ownership remains unresolved.
