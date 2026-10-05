TASK:
Establish a working, validated headless Bevy application foundation on stable Rust.

STATUS:
COMPLETE

FINDINGS:
- Latest non-yanked stable Bevy is 0.19.1 with declared MSRV 1.95.0; 0.20.0-rc.2 was excluded as a prerelease. Official stable Rust is 1.99.0.
- MinimalPlugins and ScheduleRunnerPlugin provide the needed lifecycle, ECS, time and schedules without a custom engine foundation.
- Core composes Bevy; boot owns synthetic smoke policy. No BO2 semantic abstraction is inferred from this check.

EVIDENCE:
- Inspected objective, AGENTS.md, manifests/source, architecture/ADR/state, prior handoffs and local architecture-extraction/validation skills.
- Inspected crates.io metadata, cargo info bevy@0.19.1 --verbose and released Bevy manifests; checked MinimalPlugins, schedule runner and AppExit implementation/docs. Sources are linked in ADR 0001.
- rustc 1.99.0 (b940084d7 2026-09-28), Cargo 1.99.0 (5f94df478 2026-08-27), x86_64-pc-windows-msvc; rustfmt and Clippy installed.
- No original game data, scripts/entities, behavior comparisons or performance measurements inspected.

CHANGES:
- Cargo.toml, both member manifests, rust-toolchain.toml and Cargo-generated Cargo.lock: exact released Bevy dependency, std-only explicit feature selection, pinned stable Rust baseline.
- crates/core/src/lib.rs: reusable headless_app factory over Bevy MinimalPlugins with caller-selected runner.
- game/boot/src/main.rs: bounded smoke plugin exercising deferred spawn, resource/component updates, final validation and AppExit; help/unsupported-argument handling and two genuine lifecycle/error-path tests.
- README.md, docs/README.md, docs/architecture/README.md, ADR 0001 and docs/project-state.md updated; no unsafe-policy changes.
- Installed official rustup/stable and exact 1.99.0 toolchains outside the repository; added only Cargo bin to user PATH once, preserving existing entries. Current shells use session PATH; future sessions need a refreshed environment. Existing linker/SDK reused.

VALIDATION:
- cargo metadata --no-deps --format-version 1: exit 0; two local members, boot -> core, identical exact Bevy dependency/default-feature policy.
- cargo test --workspace: exit 0; two tests pass, including missing synthetic entity failure without panic; no other unit/doc tests defined.
- cargo fmt --all -- --check: exit 0.
- cargo clippy --workspace --all-targets -- -D warnings: exit 0.
- cargo run -p bo2-boot -- --smoke: exit 0, Smoke: PASS (startup=1, updates=3, probe_updates=3).
- Direct executable default smoke/help/unsupported-argument checks: exits 0/0/2 respectively.
- cargo tree --edges normal --prefix none: released Bevy shared by core/boot; no active bevy_render, bevy_winit, wgpu or bevy_asset dependency.
- git diff --check and candidate review: passed; lockfile is trackable, build output ignored, no proprietary content or local installation paths added.
- No original-game comparison, graphical runtime, cross-platform build or profiling performed.

FIDELITY STATUS:
UNKNOWN

UNRESOLVED:
- Outer-agent review/integration of the task; refresh older shell/app environments for user PATH.
- Original asset locations, formats, shared semantics and gameplay timing remain unknown.

RECOMMENDED NEXT WORK:
- Outer-agent review and independent gates, then bounded licensed external asset cataloging.
- Investigate original structures before approving game component/asset boundaries or fixed-step policy.

SCOPE NOT IMPLEMENTED:
- Original asset loading/inspection, graphics/window, gameplay/entity schemas, movement, weapons, zombies, gameplay timing, live comparison, profiling, commits or pushes.
