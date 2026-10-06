# Project state

The playable foundation replaces eleven packages and capture-dependent tools with one `origins` Cargo game application. Work is sequential in this chat; subagents and delegated chats are banned.

Bevy 0.19.1 owns rendering and engine facilities; Avian 0.7 supplies physics queries/move-and-slide. Rust 1.99 stable is selected. Runtime coordinates are meters and Y-up. Original data, prepared glTF/PNG assets and reports remain external.

The implementation assembles source map geometry/static placements, spawn-room player movement, pistol/reload/melee, temporary zombies, pursuit/attacks, normal rounds, Bevy HUD, pause, game-over and restart. Practical reconstruction is authorized. Exact captured-frame/native numeric parity is not a delivery gate.

The release game passed automated real-window movement, six Round 1 kills, Round 2 advancement, eight map views and restart at Round 1. Thirteen Rust tests, three preparation tests, metadata, formatting and Clippy pass. Markdown decreased from 282 to 20 and tracked Rust from 36,490 to 3,547 lines (90.3% reduction). Material presentation still needs refinement; this is the playable foundation for the remaining original systems.

Current validation and limitations are recorded in the [current handoff](handoffs/current.md). Follow the [architecture](architecture/README.md), [asset reference](reverse_engineering/assets.md), [rendering reference](reverse_engineering/rendering.md), [gameplay reference](reverse_engineering/gameplay.md) and [complete systems roadmap](roadmap.md).

Documentation working set: at most 20 Markdown files, including ten skills. Replace stale state instead of appending audits. Historical documentation and superseded code are recovered from Git.
