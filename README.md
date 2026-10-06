# Origins

One Rust/Bevy game application. All future gameplay and rendering work extends `origins.exe`.

Copy `config/local_assets.example.toml` to ignored `config/local_assets.toml` and set external paths. Prepare the licensed local assets:

```powershell
uv run tools/prepare_origins.py --config config/local_assets.toml
cargo run --locked
```

Build the double-clickable release with `tools/build.ps1`. It copies local configuration beside `release/origins.exe`. No command-line arguments are needed to play. Assets remain external.

WASD moves; mouse looks; Shift sprints; Space jumps; Control crouches. Left click fires; right click or V melees; R reloads. Escape pauses; R restarts after death. Starts in the spawn room at Round 1, with six enemies.

Rendering uses standard Bevy glTF/PBR, lightmaps, HDR, AgX, bloom and fog. Avian supplies collision queries and move-and-slide. Weapons and zombies currently use temporary visuals. Practical differences are documented in [gameplay](docs/reverse_engineering/gameplay.md) and [rendering](docs/reverse_engineering/rendering.md).

Read [project state](docs/project-state.md), [architecture](docs/architecture/README.md), [asset preparation](docs/reverse_engineering/assets.md) and the [complete systems roadmap](docs/roadmap.md). Historical research is recoverable from Git.

Validate a completed milestone with metadata, release workspace tests, formatting and Clippy. The real-asset check is opt-in: `release/origins.exe --verify <external-output-directory>`. It drives this same game application, captures the map, and checks combat, movement, round progression and restart.
