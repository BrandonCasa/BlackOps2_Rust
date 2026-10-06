# Origins

Install Git LFS before checkout (`git lfs install`), then run `git lfs pull` to materialize the runtime assets; `git lfs fsck` checks local LFS integrity. A checkout containing pointer text is not a usable asset pack. Import prerequisites and offline verification are documented in [asset preparation](docs/reverse_engineering/assets.md).

One Rust/Bevy game application. All future gameplay and rendering work extends `origins.exe`.

Play directly from the committed runtime pack:

```powershell
cargo run --release --locked
```

No BO2 installation or local configuration is needed to play. Discovery uses `assets/origins/` beside the executable first, then the repository containing a Cargo-built executable. It never depends on the working directory or a compiled-in repository path. Local asset configuration is used only by the offline preparation/import tools; runtime overrides have been removed.

Build with `tools/build.ps1`. The resulting `release/` contains `origins.exe` and `assets/origins/`. Copy that entire directory anywhere and launch its executable from any working directory. Keep the assets beside the executable when relocating it. Packaging runs the compiler gates and reuses the offline importer to validate the dependency closure and copied hashes, without original inputs or local configuration. Building needs existing Cargo, `uv`, Python 3.11+, NumPy 2 and Pillow 12; playing the package needs none of those tools.

Missing or corrupt assets show a loading error in the game window and application log. Restore the complete package asset directory; for a checkout containing Git LFS pointers, run `git lfs pull`.

To regenerate assets from licensed originals, copy `config/local_assets.example.toml` to ignored `config/local_assets.toml`, set its external preparation paths, then run:

```powershell
uv run tools/prepare_origins.py --config config/local_assets.toml
uv run tools/import_origins.py --config config/local_assets.toml
```

WASD moves; mouse looks; Shift sprints; Space jumps; Control crouches. Left click fires; right click or V melees; R reloads. Escape pauses; R restarts after death. Starts in the spawn room at Round 1, with six enemies.

Rendering uses Bevy glTF/PBR with layered terrain, explicit surface states, animated water, wetness, baked light/probes, HDR, AgX, bloom, storm sky and fog. `config/origins-visual.json` holds the embedded adjustable visual profile; edit and rebuild to tune it. Avian supplies collision queries and move-and-slide. Weapons and zombies currently use temporary visuals. Practical differences are documented in [gameplay](docs/reverse_engineering/gameplay.md) and [rendering](docs/reverse_engineering/rendering.md).

Read [project state](docs/project-state.md), [architecture](docs/architecture/README.md), [asset preparation](docs/reverse_engineering/assets.md) and the [complete systems roadmap](docs/roadmap.md). Historical research is recoverable from Git.

Validate a completed milestone with metadata, a release workspace build and formatting. Unit tests and Clippy are banned. The real-asset check is opt-in: `release/origins.exe --verify <external-output-directory>`. It drives this same game application, captures the map, and checks combat, movement, round progression and restart.
