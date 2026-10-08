# Origins

Install Git LFS before checkout (`git lfs install`), then run `git lfs pull` to materialize the runtime assets; `git lfs fsck` checks local LFS integrity. A checkout containing pointer text is not a usable asset pack. Import prerequisites and offline verification are documented in [asset preparation](docs/reverse_engineering/assets.md).

One Rust/Bevy game application. All future gameplay and rendering work extends `origins.exe`.

Play directly from the committed runtime pack:

```powershell
cargo run --locked
```

No BO2 installation or local configuration is needed to play. Discovery uses `assets/origins/` beside the executable first, then the repository containing a Cargo-built executable. It never depends on the working directory or a compiled-in repository path. Local asset configuration is used only by the offline preparation/import tools; runtime overrides have been removed.

Build for development with `tools/build.ps1` (or `-Profile Debug`), producing `target/debug/origins.exe` with debug information and assertions. Game code uses optimization level 1 and dependencies level 3; debug uses thin LTO and release uses full LTO, with one codegen unit in both profiles. Windows MSVC uses the stable toolchain's bundled rust-lld linker. Build for performance measurements or shipping with `tools/build.ps1 -Profile Release`: optimization level 3, no incremental compilation, debug information or assertions. Dependency `log` calls are capped at DEBUG in development. All logs and tracing events/spans are compiled out in release; Tracy/Chrome profiling features are forbidden in release and captures use development builds. Compare performance using the same profile and instrumentation.

The release command produces `release/origins.exe` and `release/assets/origins/`. Copy that entire directory anywhere and launch its executable from any working directory. Keep the assets beside the executable when relocating it. Packaging runs the compiler gates and reuses the offline importer to validate the dependency closure and copied hashes, without original inputs or local configuration. Packaging needs existing Cargo, `uv`, Python 3.11+, NumPy 2 and Pillow 12; playing the package needs none of those tools.

Missing or corrupt assets show a loading error in the game window and application log. Restore the complete package asset directory; for a checkout containing Git LFS pointers, run `git lfs pull`.

To regenerate assets from licensed originals, copy `config/local_assets.example.toml` to ignored `config/local_assets.toml`, set its external preparation paths, then run:

```powershell
uv run tools/prepare_origins.py --config config/local_assets.toml
uv run tools/import_origins.py --config config/local_assets.toml
```

WASD moves; mouse looks; Shift sprints; Space jumps; Control crouches. Left click fires; right click or V melees; R reloads. Escape pauses; R restarts after death. Starts in the spawn room at Round 1, with six enemies.

Rendering uses Bevy glTF/PBR with layered terrain, explicit surface states, animated water, wetness, baked light/probes, HDR, AgX, bloom, storm sky and fog. `config/origins-visual.json` holds the embedded adjustable visual profile; edit and rebuild to tune it. Avian supplies collision queries and move-and-slide. Weapons and zombies currently use temporary visuals. Practical differences are documented in [gameplay](docs/reverse_engineering/gameplay.md) and [rendering](docs/reverse_engineering/rendering.md).

Read [project state](docs/project-state.md), [architecture](docs/architecture/README.md), [asset preparation](docs/reverse_engineering/assets.md) and the [atomic gameplay checklist](todo.md). Every checklist item remains outstanding until its individual implementation is complete. Historical research is recoverable from Git.

Validate a completed milestone with locked metadata, a development workspace build and formatting. Unit tests and Clippy are banned. The real-asset check is opt-in: `target/debug/origins.exe --verify <external-output-directory>`. It drives this same game application, captures the map, and checks combat, movement, round progression and restart. Use the release executable for performance measurements and package validation.
