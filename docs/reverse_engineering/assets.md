# Asset preparation and runtime import

Licensed originals, extraction intermediates, original scripts/shaders, captures, research reports and preparation/inspection receipts stay outside Git. The prepared runtime dependency closure is permitted under `assets/origins/`: `game.json` in ordinary Git, referenced glTF scenes, binary mesh buffers and PNG textures in Git LFS. Only ignored local configuration contains installation/output paths. The tracked template has placeholders.

One command prepares the game pack: `uv run tools/prepare_origins.py --config config/local_assets.toml`. It reads the retained complete world exchange, material/image metadata, packed corner/UV streams, source map entities and OAT model exports. Missing entity/model exports are prepared through external OAT. Missing static placement metadata is exported by a read-only callback compiled against the existing external OAT x86 build.

Prerequisites: licensed Origins installation, the previously prepared external `full-scene-assets/texture-preview.json` and its referenced files, OAT v0.33.0, and the external OAT source/build when placement metadata is absent. Optional `[preparation]` overrides are `scene`, `entities`, `models`, `instances`, `unlinker` and `oat_source`. The converter currently consumes the retained world exchange rather than decoding fastfiles in Rust.

World exchange: BO2DGS v1 has a 64-byte header, float32 XYZ positions, u32 triangle indexes and ordered 48-byte surface ranges. Corner prefixes are 36 bytes, containing position, color, packed UV/normal/tangent and lightmap UV fields. Hashes, finite coordinates and complete ordered surface partitions are checked. Geometry is independent of captured draw lists.

Outputs: external `map.gltf`, `map.bin`, PNG textures, `game.json` and `preparation.json`. The glTF contains render and collision scenes. All 6,469 world surfaces receive a render/collision disposition; shadow helpers are hidden collision geometry. Static providers are shared across all 14,480 source placements. Bevy generates mesh tangents from normals and UVs. Cell grouping keeps world bounds local.

The source `initial_spawn_points` entities supply the spawn-room anchor. `zone_start_spawners` and nearby path nodes supply practical enemy admission/pursuit. Dummy `info_player_start` is not the player spawn. Node projection replaces unimplemented climb/rise animations.

Preparation refuses repository output paths, escaped stream dependencies, malformed exchanges and missing required model textures. It never writes to original inputs. Preparation receipts may contain proprietary identifiers and remain external.

## Offline import and checkout

Prerequisites for import: Python 3.11+ through existing `uv`, NumPy 2 and Pillow 12 (script metadata resolves them), a complete external prepared pack, its `preparation.json`, and the retained world exchange/static-placement metadata/OAT model exports used to reconcile receipt counts. No game process or network service is needed for validation once Python dependencies are cached. Preparation still relies on retained external world exchanges; this is not a complete fresh fastfile extraction command.

```powershell
git lfs install --local
uv run tools/prepare_origins.py --config config/local_assets.toml
uv run tools/import_origins.py --config config/local_assets.toml
uv run tools/import_origins.py --config config/local_assets.toml --check
```

Skip preparation when importing an existing prepared pack. The importer discovers `[game] asset_pack`, validates the entire supported prepared glTF profile before copying, preserves relative paths and shared mesh/placement references byte for byte, checks PNG decoding and node-extra lightmaps, and reconciles scene/texture/triangle counts. Retained source geometry verifies the original surface count; original placement metadata and model exports verify provider counts, placement order, reusable mesh references and transforms. Grouped runtime meshes alone cannot recover original surface/provider counts. Unknown glTF features fail closed. Source files are never modified. The destination rejects unrelated files; the importer does not remove them. `--check` performs validation/hash comparison without copying. Optional `--receipt <external-file.json>` writes a hash manifest outside the repository and source pack.

The imported closure is 557 files: `game.json`, `map.gltf`, `map.bin`, and 554 PNGs (204 world and 350 model textures). The external receipt reports 6,469 world surfaces, 149,907 world triangles, 14,480 static placements and 494 model providers. Shared providers expand to 632 meshes; render/collision scenes contain 17,759/15,613 mesh nodes. The importer checks these against current input data, rather than relying on these documented counts.

Scoped `.gitattributes` must be present before staging assets. Inspect candidates, run `git check-attr filter diff merge text -- assets/origins/map.gltf assets/origins/map.bin assets/origins/textures/model-0.png assets/origins/game.json`, inspect staged pointer contents with `git show :assets/origins/map.gltf`, then run `uv run tools/import_origins.py --config config/local_assets.toml --check-index` and `git lfs fsck`. The index check verifies every staged runtime filename, attribute, pointer hash/size and local LFS payload; it also compares the ordinary Git `game.json` blob against the source. Newline conversion is disabled for runtime bytes, including `game.json`. A new checkout needs Git LFS installed and `git lfs pull`; unresolved pointers are rejected by the importer. Local commits retain LFS objects locally; transferring a commit requires its LFS objects as well.

Playing uses `assets/origins/` beside `origins.exe`, or the repository pack when launched from its Cargo output directory. Discovery is independent of the working directory and reads no local configuration. Runtime overrides have been removed; `[game] asset_pack` remains an offline preparation/import setting.

`tools/build.ps1` packages `release/origins.exe` with `release/assets/origins/`. It calls the existing importer directly, validates the complete runtime closure, rejects unresolved LFS pointers/unrelated files, decodes PNGs, and compares all copied SHA-256 hashes. Packaging does not need originals, preparation receipts or machine-specific config; those source-reconciliation checks remain part of offline import. Copy the whole release directory to relocate the game. Missing dependencies produce loading errors in the game window/log with repair steps. Current real-window evidence is recorded in the handoff. Material fidelity remains unchanged; pack integrity and automated gameplay do not establish original-game equivalence.
