# Diagnostic snapshot to Bevy Mesh

Task 0014 adds one reviewed and accepted safe consuming adapter in `bo2-rendering`. The accepted
[snapshot contract](diagnostic-geometry-snapshot.md) and complete external
[bridge findings](../reverse_engineering/0008-origins-diagnostic-geometry-snapshot.md)
support this project diagnostic boundary. This is not an original runtime layout,
draw-command model, asset loader or completed representative rendering task.
Original data understanding is **INVESTIGATING**; overall, gameplay and visual
fidelity remain **UNKNOWN**.

## API and preservation

`diagnostic_mesh(GeometrySnapshot) -> (Mesh, Vec<SnapshotSurface>)` consumes the
validated snapshot once using its existing `into_parts` API. The returned mesh
has exactly one `Mesh::ATTRIBUTE_POSITION` containing `Float32x3`, explicit
`Indices::U32`, and `PrimitiveTopology::TriangleList`. This topology represents
the diagnostic contract's three-element grouping; it proves no original draw
semantics. `RenderAssetUsages::MAIN_WORLD` deliberately retains CPU access without
requesting render-world extraction. No GPU upload is claimed.

Position bits, index values/order, equal and unreferenced vertices, repeated
references and degenerate triangles are unchanged. No unit/axis conversion,
scaling, winding reversal, centering, welding, filtering, attribute repair or
generation occurs. Original units, axes and winding remain unconfirmed: placing
unchanged numeric arrays in Bevy storage selects no original-to-Bevy mapping.
No normals, UVs, colors, materials, lights, cameras or draw splits are inferred.

All ordered surface records are returned in the original vector. Their ranges
are still in index elements, with the same triangle counts, opaque categories,
source declared counts and exact endpoint bits. Zero counts and any u32 category
never remove a record or imply a material. The metadata remains caller-owned;
it is not installed as a new engine asset or entity type. Surface bounds are not
recomputed or assigned to Bevy's `final_aabb`, which remains unset.

## Ownership and engine facilities

The three `Vec` allocations move into Bevy's position value, U32 index value and
the returned metadata vector. There is no second full geometry copy. Bevy's mesh
attribute `BTreeMap` requires bookkeeping allocation; engine asset insertion
allocates storage, a strong handle and related engine metadata. These allocations
use Bevy's normal allocation behavior, not the parser's fallible reservation
policy. No allocator-overhead or process-memory guarantee is added. Synthetic and
complete external checks confirm the three allocation pointers survive conversion;
the geometry pointers also survive engine insertion/retrieval/removal.

The pinned Bevy 0.19.1 implementation was inspected directly:

- [Mesh storage and constructor](https://docs.rs/bevy_mesh/0.19.1/src/bevy_mesh/mesh.rs.html#227-256),
  [new/insertion](https://docs.rs/bevy_mesh/0.19.1/src/bevy_mesh/mesh.rs.html#341-410)
  and [U32 index insertion](https://docs.rs/bevy_mesh/0.19.1/src/bevy_mesh/mesh.rs.html#684-699).
  A fresh mesh has accessible data, and the fixed position value variant matches
  the fixed attribute format, satisfying Bevy's insertion preconditions.
- [MeshPlugin](https://docs.rs/bevy_mesh/0.19.1/src/bevy_mesh/lib.rs.html#58-73)
  uses `AssetApp::init_asset`, registers mesh reflection and engine asset systems.
  [AssetApp registration](https://docs.rs/bevy_asset/0.19.1/src/bevy_asset/lib.rs.html#639-704)
  installs typed storage, handle registration, messages and tracking/event systems.
- [Assets add/get/remove](https://docs.rs/bevy_asset/0.19.1/src/bevy_asset/assets.rs.html#399-478)
  and [usage flags](https://docs.rs/bevy_asset/0.19.1/src/bevy_asset/render_asset.rs.html#5-50)
  provide ownership and CPU storage policy. `PrimitiveTopology` is reexported by
  `bevy_mesh`; no render crate dependency is needed to name it.

The adapter itself assembles no plugins. Headless validation creates an actual
Bevy `App` with `MinimalPlugins`/run-once runner, registers Bevy's
`MemoryAssetReader` with `AssetSourceBuilder::new`, then adds `AssetPlugin` and
`MeshPlugin`, finishes/cleans up and updates the app. The tests verify Mesh and
Handle reflection registration, typed `Assets<Mesh>` add/get/remove and removal
after the last strong handle is dropped. No filesystem asset load is requested.
This development lifecycle has no original timing meaning. Bevy retains all
ECS, scheduling, asset identity, mesh and runtime ownership.

## Dependency boundary

New local edge: `bo2-rendering -> bo2-assets`; `bo2-assets` stays std-only and
independent of Bevy. Rendering depends on the existing workspace-pinned Bevy
0.19.1 with member-specific `bevy_asset` and `bevy_mesh` features; the central
`std`, disabled umbrella defaults and exact version remain unchanged. Existing
core, boot, inspectors and parser APIs are unchanged. Local edges are acyclic and
no lower layer imports Origins or application assembly. All six local crates
forbid unsafe code.

The actual locked `cargo tree -e features -p bo2-rendering` shows mesh enabling
image support and asset support through `bevy_internal`. Active support includes
`bevy_asset`, `bevy_asset_macros`, `bevy_mesh`, `bevy_image`, `bevy_color`,
`bevy_transform`, `bevy_encase_derive`, `image`, `wgpu-types`, `encase`,
`hexasphere` and their required dependencies. Upstream mesh defaults are empty;
upstream image defaults include reflection. No image codec, asset watcher or
processor feature is requested. The active graph has no `bevy_render`, window,
winit, PBR, camera or gizmos package. Cargo's lockfile adds 67 upstream packages
plus the new member, with no existing package version removed or changed.
Lock-only optional packages such as `bevy_camera`, `bevy_gizmos` and `bevy_log`,
and target-conditional Android/Wasm support entries, are not proof of active
compilation or runtime plugin assembly. Boot still assembles only its existing
headless smoke application.

## Validation and remaining limits

Six arbitrary synthetic tests cover signed zero, positive/negative subnormals,
maximum finite floats, equal/unreferenced positions, degenerate/repeated ordered
indices, values beyond u16, categories 6/8/u32::MAX, zero declared counts, empty
arrays/surfaces, exact endpoint metadata, allocation transfer and actual engine
storage/lifecycle. The accepted parser's exhaustive rejection suite is retained
without duplication.

One already accepted complete external snapshot was checked read-only after
regular-file/root-containment, length and SHA-256 admission. It is 4,404,568 bytes,
SHA-256 `ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f`.
A separate external Rust harness uses same-handle metadata and the accepted
parser, then compares every position bit, index and surface field against that
same handle through fixed-buffer streaming scans after conversion, typed engine
retrieval and removal. Complete counts are 191,259 positions, 449,721 indices,
149,907 triangles, 6,469 surfaces, 6,236 zero source counts and seven opaque
categories. The harness adds no full geometry comparison copy and emits only
counts/status. Input SHA/length/write-time/file identity remain unchanged.
No original fastfile or native producer is read or executed by this task.

Locked workspace metadata, all 67 workspace tests, formatting, warning-denying
Clippy, the existing Bevy smoke and both inspector help commands pass. Five
functional-file hashes are unchanged across final gates. External harness,
build/capture/result logs and preservation records remain outside the repository.
See [handoff 0014](../handoffs/0014-bevy-diagnostic-mesh-adapter.md).

Outer review independently passed the frozen workspace gates and complete external
harness, confirmed the dependency boundary and 188 non-documentation preservation
records, and checked all ten candidates, 56 relative links, seven pinned source
anchors and ten handoff headings. Acceptance covers diagnostic storage and engine
ownership; it does not promote original fidelity.

GPU extraction, graphical output, original appearance/timing comparison,
coordinate/winding measurement, asynchronous loaders, full attributes/materials,
profiling, other platforms and forced allocator exhaustion remain unperformed.
The original full Origins objective is unchanged; this adapter proves diagnostic
storage preservation and engine ownership only.
