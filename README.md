# Origins

A Rust recreation of **Black Ops II Zombies: Origins** using Bevy and Avian.
Windows x64 / Vulkan. Active development; full gameplay and visual parity remain unfinished.

**How it works:** export your BO2 installation into `target/assets`, then run the
Rust game. Bevy handles the app, assets, and rendering; Avian handles physics;
gameplay plugins reproduce Origins mechanics on a 60 Hz fixed schedule.
The repository pins its Bevy fork in `vendor/bevy` and dependencies in `Cargo.lock`.

## Features

- Survival rounds, zombie AI, Panzer, combat, points, perks, power-ups, and revives.
- Wall buys, Mystery Box, Pack-a-Punch, weapons, equipment, and buildables.
- Generators, tank, robots, digging, portals, staffs, upgrades, and quest systems.
- Native materials, lighting, shadows, weather, effects, audio, cinematics, and HUD.
- Remappable controls, optional DLSS, profiling, and scripted gameplay validation.

See [validation progress](tools/ee_validator/README.md) for tested gameplay checkpoints.

## Setup

Install Git, PowerShell 7, Rust via rustup, and Visual Studio's C++ build tools
with the Windows SDK. `rust-toolchain.toml` pins Rust **1.99.0**.

The build also needs **DLSS SDK 310.9.1**, **Vulkan SDK**, and **LLVM/Clang**.
Set their locations in your Visual Studio Developer PowerShell:

```powershell
$env:DLSS_SDK = 'C:\SDKs\DLSS-310.9.1'
$env:VULKAN_SDK = 'C:\VulkanSDK\<version>'
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'

git clone --recurse-submodules https://github.com/BrandonCasa/BlackOps2_Rust.git
cd BlackOps2_Rust
```

For an existing checkout: `git pull --ff-only`, then `git submodule update --init --recursive`.

## Assets

You need your own **BO2 Zombies + Origins/Apocalypse** installation. Game assets
are not included. Export requires Python 3, Node.js/npm, and 7-Zip at
`C:\Program Files\7-Zip\7z.exe`.

```powershell
pwsh -File tools/export_assets.ps1
```

The script expects the English Steam install at
`C:\Program Files (x86)\Steam\steamapps\common\Call of Duty Black Ops II`.
Edit its `$steam` path for another library. Output goes to `target/assets`,
tools to `target/tools`, and extraction reports to `target/reports`.

**Fresh exports still need manual work:** FX recovery uses separate local research
inputs through `tools/export_fx.py` and `tools/export_fx_shaders.py --research`.
Those inputs are not included. A clone and successful build do not supply a complete
playable asset set. Cargo stages authored gameplay JSON; it does not extract game files.
Preserve local assets and tools before cleaning `target`.

## Build and run

Run these from the repository root after configuring the SDK paths:

| | Build | Run existing build |
| --- | --- | --- |
| Dev | `cargo build --features dev-tools --locked` | `pwsh -File tools/run.ps1 -Profile dev` |
| Release | `cargo build --release --locked` | `pwsh -File tools/run.ps1 -Profile release` |

Executables are `target/debug/origins.exe` and `target/release/origins.exe`.
Both use `target/assets`. Override it with `origins.exe --asset-root 'D:\Origins\assets'`.
Release builds reject `dev-tools` and profiling features.

The build copies `nvngx_dlss.dll` beside the executable. Neural Rendering additionally
needs `nvngx_dlssnr.dll` there or an `ORIGINS_DLSS_NR_DLL` path; F11 toggles it.
See [DLL compatibility](vendor/bevy/crates/bevy_anti_alias/src/dlss/NEURAL_RENDERING.md).
Cinematics need `<asset-root>/../tools/mpv/mpv.exe`, staged by the exporter.

## Developer reference

| Path | Purpose |
| --- | --- |
| `src/lib.rs`, `src/gameplay.rs` | Plugins, states, gameplay scheduling. |
| `gameplay/origins/` | Authored definitions and source values. |
| `src/origins/`, `src/assets.rs` | Source bindings, loading, collision. |
| `src/rendering/`, `src/native_fx/` | Rendering and effects. |
| Other `src/` modules | Each gameplay system and its state. |
| `tools/` | Export, launch, and validation. |
| `vendor/bevy/` | Pinned engine fork; [dependency workflow](vendor/README.md). |

```powershell
cargo fmt --all -- --check
cargo check --features dev-tools --locked
cargo test --features dev-tools --lib --locked
python tools/ee_validator/run.py --build --until foundation
```

Read [AGENTS.md](AGENTS.md) before contributing. Profiling features:
`profile-tracy`, `profile-tracy-memory`, `profile-chrome` (dev profile only).

- **Logs:** `logs/origins-*.log` beside the executable; export errors in `target/reports`.
- **Settings:** `%LOCALAPPDATA%\Origins\settings.json`. Controls can be remapped in-game.
- **Controls:** WASD move, Shift sprint, Space jump, C crouch, Ctrl prone, mouse fire/aim,
  R reload, F interact, V melee, Q/wheel switch weapon, G/T equipment, Esc pause.
- **Build errors:** check SDK paths and the VS developer shell; initialize submodules for missing Bevy paths.
- **Missing assets:** inspect export reports. Rebuilding Rust does not regenerate extracted content.

## License

Copyright (c) 2026 BrandonCasa and contributors. Original project code is licensed
under [GPL-3.0-only](LICENSE). Third-party code, tools, SDKs, and game assets retain
their own terms. Unofficial project; not affiliated with Activision or Treyarch.
