# External asset preparation

Licensed originals, exports, textures, meshes and reports stay outside Git. Only ignored local configuration contains installation/output paths. The tracked template has placeholders.

One command prepares the game pack: `uv run tools/prepare_origins.py --config config/local_assets.toml`. It reads the retained complete world exchange, material/image metadata, packed corner/UV streams, source map entities and OAT model exports. Missing entity/model exports are prepared through external OAT. Missing static placement metadata is exported by a read-only callback compiled against the existing external OAT x86 build.

Prerequisites: licensed Origins installation, the previously prepared external `full-scene-assets/texture-preview.json` and its referenced files, OAT v0.33.0, and the external OAT source/build when placement metadata is absent. Optional `[preparation]` overrides are `scene`, `entities`, `models`, `instances`, `unlinker` and `oat_source`. The converter currently consumes the retained world exchange rather than decoding fastfiles in Rust.

World exchange: BO2DGS v1 has a 64-byte header, float32 XYZ positions, u32 triangle indexes and ordered 48-byte surface ranges. Corner prefixes are 36 bytes, containing position, color, packed UV/normal/tangent and lightmap UV fields. Hashes, finite coordinates and complete ordered surface partitions are checked. Geometry is independent of captured draw lists.

Outputs: external `map.gltf`, `map.bin`, PNG textures, `game.json` and `preparation.json`. The glTF contains render and collision scenes. All 6,469 world surfaces receive a render/collision disposition; shadow helpers are hidden collision geometry. Static providers are shared across all 14,480 source placements. Bevy generates mesh tangents from normals and UVs. Cell grouping keeps world bounds local.

The source `initial_spawn_points` entities supply the spawn-room anchor. `zone_start_spawners` and nearby path nodes supply practical enemy admission/pursuit. Dummy `info_player_start` is not the player spawn. Node projection replaces unimplemented climb/rise animations.

Preparation refuses repository output paths, escaped stream dependencies, malformed exchanges and missing required model textures. It never writes to original inputs. Preparation receipts may contain proprietary identifiers and remain external.
