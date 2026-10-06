# Project state

The playable foundation replaces eleven packages and capture-dependent tools with one `origins` Cargo game application. Work is sequential in this chat; subagents and delegated chats are banned.

Bevy 0.19.1 owns rendering and engine facilities; Avian 0.7 supplies physics queries/move-and-slide. Rust 1.99 stable is selected. Runtime coordinates are meters and Y-up. Original data, preparation intermediates and reports remain external. The validated prepared runtime pack is tracked under `assets/origins/`, with glTF/buffers/PNGs in Git LFS. Runtime discovery uses the committed pack for Cargo builds and the adjacent pack for release packages, independent of the working directory. Play requires no original installation or local configuration. Runtime overrides are removed; external paths are used only by offline preparation/import.

The implementation assembles source map geometry/static placements, spawn-room player movement, pistol/reload/melee, temporary zombies, pursuit/attacks, normal rounds, Bevy HUD, pause, game-over and restart. Practical reconstruction is authorized. Exact captured-frame/native numeric parity is not a delivery gate.

The release game passed automated real-window movement, six Round 1 kills, Round 2 advancement, eight map views and restart at Round 1. All test suites have been removed; unit tests and Clippy are banned. Validation uses asset integrity checks, compiler gates and the actual game window. Documentation remains at 20 tracked Markdown files. Material presentation still needs refinement; this is the playable foundation for the remaining original systems.

Current validation and limitations are recorded in the [current handoff](handoffs/current.md). Follow the [architecture](architecture/README.md), [asset reference](reverse_engineering/assets.md), [rendering reference](reverse_engineering/rendering.md), [gameplay reference](reverse_engineering/gameplay.md) and [complete systems roadmap](roadmap.md).

The prepared runtime pack now retains source tangent handedness, RGBA vertex colors, UV seams, material identities, raw texture layers/specular-gloss channels, both lightmap channels, all probe faces/coefficients and world/static lighting assignments. Standard PBR still approximates the original response; retained data enables later material refinements without another source extraction.

Documentation working set: at most 20 Markdown files, including ten skills. Replace stale state instead of appending audits. Historical documentation and superseded code are recovered from Git.
