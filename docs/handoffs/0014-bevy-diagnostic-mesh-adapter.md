TASK:
Implement one safe consuming adapter from the accepted project diagnostic GeometrySnapshot into Bevy 0.19.1 Mesh storage, retaining every ordered surface record and demonstrating complete headless Assets<Mesh> ownership. This bounded diagnostic task does not complete representative rendering or the full Origins goal.

STATUS:
COMPLETE
Implementation, synthetic cases, one complete accepted external snapshot check and workspace gates pass. Outer review accepts this bounded diagnostic adapter after independent verification and documentation/content/dependency/preservation review. The worker made no commit or push.

FINDINGS:
- `diagnostic_mesh(GeometrySnapshot) -> (Mesh, Vec<SnapshotSurface>)` moves the existing three vectors using `into_parts`. Bevy stores exactly one Float32x3 POSITION attribute and explicit U32 indices under TriangleList with MAIN_WORLD usage; the caller retains the unchanged ordered metadata vector.
- Position bits, reference values/order, equal/unreferenced vertices, repeated/degenerate triangles, empty ranges, source declared counts, arbitrary u32 categories and endpoint bits survive. Zero counts never filter metadata. No coordinate/unit/axis/winding mapping or draw/material inference is selected; final_aabb remains unset.
- Mesh construction/insertion satisfies the pinned engine's fixed attribute-format and fresh-data preconditions. Geometry vectors are not cloned; Bevy allocates attribute-map and asset/handle bookkeeping. No fallible adapter allocation policy or new process-memory cap is claimed.
- Actual headless Bevy MinimalPlugins/run-once + in-memory AssetSourceBuilder + AssetPlugin/MeshPlugin assembly registers Mesh/Handle reflection and typed asset systems. Engine add/get/remove and last-strong-handle cleanup pass. The adapter installs no plugins, entity type, custom registry/ECS/scheduler or renderer.
- Active member feature graph includes asset/mesh/image and required color/transform/image/encase/wgpu-types support, with no bevy_render/window/winit/PBR/camera/gizmos package. Lock-only optional and target-conditional entries are distinguished from active dependencies and runtime plugins. The lock adds 67 upstream packages plus bo2-rendering without changing/removing existing package versions.

EVIDENCE:
- Read root AGENTS and rendering-analysis/architecture-extraction/validation skills; architecture README/ADR0001/diagnostic snapshot contract; project state; findings0005/0006/0007/0008; handoffs0012/0013; actual std-only parser/API/tests, core/boot, manifests and lockfile.
- Inspected cached/downloaded exact Bevy0.19.1 umbrella/internal feature manifests, custom_mesh/custom_asset examples and actual Mesh/VertexAttributeValues/Indices/PrimitiveTopology/RenderAssetUsages/AssetPlugin/AssetApp/Assets/Handle/MeshPlugin/source-builder APIs. Exact source links and ownership choices are recorded in the architecture document.
- Only accepted external observations-v2/production1/diagnostic-v1.bo2dgs is read: regular-file/root-containment and SHA/length admission precede parsing. Length4,404,568; SHA256 ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f. Same opened read-only handle supplies metadata to the accepted parser and three streaming comparisons. No new original/native-production input is read or run.
- External harness compares every Mesh component bit, index element/order, surface range/triangle count/category/source count and endpoint bit after conversion, engine retrieval and removal, without retaining a second full geometry copy. Allocation pointers and complete category accounting are also checked. Output contains counts/status only, never individual geometry, bounds, names or paths.
- External0014-bevy-diagnostic-mesh-adapter retains harness/source/manifests/lock/binary, build/test/gate/capture/result logs, actual feature tree, functional hashes and before/after source/helper/tracked/input identities. Native source/helpers are only identity-hashed; no native rebuild/producer/game/original fastfile/script/shader read occurs.

CHANGES:
- Added crates/rendering/Cargo.toml, src/lib.rs and tests/diagnostic_mesh.rs; one minimal workspace member entry and legitimate Cargo.lock updates. Rendering depends on std-only assets and member-scoped pinned Bevy asset/mesh features. Central version/default policy and all existing APIs/pins/tests remain unchanged; six local crates forbid unsafe.
- Added docs/architecture/diagnostic-mesh-adapter.md and this handoff; minimally updated architecture README, documentation index and project state. Current local graph has four acyclic path edges: boot->core, both inspectors->assets, rendering->assets. No lower layer imports Origins or boot.
- All tracked tests are arbitrary synthetic fixtures. External geometry, roots, helper/native binaries and generated build outputs are excluded from candidates. No unsafe/FFI, global settings/installations, commits/pushes or nested agents.

VALIDATION:
- Six focused cases pass: exact float/reference/ordered metadata preservation including signed zero, both-sign subnormals and finite maxima; all three allocation pointers; empty arrays/empty metadata/unreferenced positions; U32 index65,536; actual engine registration/add/get/remove; last-strong-handle drop. Categories6/8/u32::MAX and zero counts remain opaque. Parser rejection coverage is retained without duplication.
- First fresh focused build exited101 in42.828seconds after dependencies compiled because a test source-builder constructor was wrong. A second test-only constructor attempt exited101 in0.703seconds. Both excluded failed logs are preserved. Actual pinned AssetSourceBuilder::new correction precedes final focused exit0 in6.047seconds. All work stayed within900seconds; no extension or restart of a live build.
- Frozen final `cargo metadata --locked --no-deps --format-version 1` exit0; `cargo test --workspace` exit0 with67 tests (61 retained +6 new), none failed/ignored; `cargo fmt --all -- --check` exit0; `cargo clippy --workspace --all-targets -- -D warnings` exit0. Existing `cargo run -p bo2-boot -- --smoke` remains startup1/updates3/probe3 exit0; both inspector `--help` commands exit0. Actual locked feature graph and acyclic local graph reviewed.
- External harness build exit0 in3.422seconds,445 captured bytes. Complete accepted-snapshot run exit0 in1.406seconds,477 stdout bytes,empty stderr under120seconds/256KiB capture. Exact three full comparisons preserve191,259 positions/449,721 indices/149,907 triangles/6,469 surfaces/6,236 zero-count records/seven categories; typed Mesh/Handle registration, Assets retrieval/removal and all vector pointers pass. Before/after input SHA/length/write-time/file ID match. No GPU/original-game timing/appearance claim.
- Parent independently reran locked metadata, all67 workspace tests, fmt, warning-denying Clippy, Bevy smoke and both inspector help successfully in parent-review-072632. The same final harness independently exited0 in0.250seconds with477 stdout bytes and empty stderr, all complete counts and three exact comparisons. Accepted snapshot and harness source/manifests/binary identities remain unchanged. Parent documentation/content/dependency/preservation review passes; this bounded adapter is accepted.
- Five functional files remained frozen through worker and parent final gates. Before/after SHA256 values are identical:

| Functional file | SHA-256 before = after |
| --- | --- |
| Cargo.toml | 93ae8aced2b3642e745b94dcb744b9517b236e59e9342e574a8296676ab23953 |
| Cargo.lock | 12f57c802185f8b4e1da4fe16949f53a5c355edac0af7c7370ce5e4cab052be1 |
| crates/rendering/Cargo.toml | f67cc9b79c81bfeadb0f40b7acc8be2ba160af0485eac747eb58e87c0fc089bc |
| crates/rendering/src/lib.rs | 69a14ea225bbf7b6525ca71c1a1a25654e5f6352d70beef7706fbaecd3d16be2 |
| crates/rendering/tests/diagnostic_mesh.rs | b584fe1afc1eed70f6b11ed54e46f5cb8582dc5905591993558ecdce1b965e22 |

- All ten candidates pass content/root/secret/whitespace checks, 56 relative links and ten handoff headings. Before/after preservation covers 217 source/helper/tracked/config identity records; only three assigned tracked documentation files change. Thirteen pristine native sources and task13 tooling also match accepted identity manifests; restored task9 source/header/binary and task10/11 artifacts retain identity. Parent independently passes all 188 non-documentation baseline identities with same-handle byte/SHA/write-time/device/file-ID equality, recorded in parent-preservation-review.json. Six final external driver/review/harness/manifest/lock/binary hashes are recorded. No original fastfile read is needed for these boundary checks. Other platforms, forced allocator exhaustion, GPU extraction/appearance and original-game comparisons were not run.

FIDELITY STATUS:
INVESTIGATING
Original candidate-data understanding and diagnostic preservation only; overall, gameplay and visual fidelity UNKNOWN. No MATCHED/VERIFIED or original renderability/draw/appearance/authenticity claim.

UNRESOLVED:
- Original units/axes/winding/draw commands, selective declared counts and bounds-family purpose remain unknown. Raw numeric Mesh storage selects no original-to-Bevy transform.
- Stream1 requirements/layout, normals/UV/colors/materials/textures, GPU integration, other world/model data and all gameplay require separate evidence/tasks.
- Bevy bookkeeping allocation is not fallibly budgeted here. No new process-memory cap, source locking/authentication, other-platform or forced-allocation-failure proof is established.

RECOMMENDED NEXT WORK:
- Choose one evidence task establishing original units/axes/winding before graphical placement/comparison, following acceptance of this bounded diagnostic adapter.
- Keep asynchronous AssetLoader and stream1/attribute/material investigation separately bounded; investigate format0's at-capacity stream1 cases before a complete importer. Preserve the full Origins objective.

SCOPE NOT IMPLEMENTED:
- Original .ff/game/native producer execution or new extraction; original writes/copies/proprietary tracked fixtures; async loader/filesystem discovery/viewer/window/GPU renderer/boot CLI changes; scaling/axis/winding/centering/welding/filtering/bounds recomputation; attribute/material/light/camera/draw inference; models/audio/HUD/physics/collision/movement/weapons/zombies/rounds/perks/Origins mechanics; profiling/original comparison/authentication; unsafe/global infrastructure changes; commits/pushes/nested agents.
