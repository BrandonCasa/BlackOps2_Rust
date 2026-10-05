# Offscreen raw-geometry diagnostic

`bo2-geometry-viewer` is a finite tool under
`tools/comparison_tools/geometry_viewer`. It composes the accepted
[snapshot reader](diagnostic-geometry-snapshot.md) and
[CPU Mesh adapter](diagnostic-mesh-adapter.md) with Bevy 0.19.1 rendering and
screenshot facilities. It implements diagnostic display in a chosen Bevy frame.
Original orientation, units, visible side, camera, materials, draw semantics and
appearance remain unresolved. Overall, gameplay and visual fidelity are **UNKNOWN**;
original-data understanding remains **INVESTIGATING**.

## Input, ownership and output

```sh
cargo run -p bo2-geometry-viewer -- --synthetic --output "<new external directory>"
cargo run -p bo2-geometry-viewer -- --snapshot "<single external snapshot>" --output "<new external directory>"
cargo run -p bo2-geometry-viewer -- --help
cargo run -p bo2-geometry-viewer -- --check-gpu
```

Modes are explicit and mutually exclusive. The tool loads no local configuration,
discovers no original installation and reads no original disk format. Snapshot
input is canonicalized, required to be outside this workspace, opened read-only
and admitted as a regular file using same-handle metadata. The existing reader
supplies all format/count/reference/bounds validation; no second reader or
geometry abstraction is added. The synthetic input is an arbitrary asymmetric
four-position, twelve-index tetrahedron with one opaque surface record.

`prepare` frames the stored positions, including unreferenced records, then calls
`bo2_rendering::diagnostic_mesh`. Only the returned Mesh's usage gains
`RENDER_WORLD`, retaining `MAIN_WORLD`. The existing CPU-only adapter contract
is unchanged. Position bits, indices/order, topology and every ordered surface
record are transferred unchanged; no position/index/surface edit, extra attribute,
centering, scaling, remapping, welding, reversal, filtering or deindexing occurs.
The sole mesh entity has identity Transform. Surface records remain caller-owned
for the entire render; they do not become materials or original draw splits.
After readback, CPU attributes/counts and original position/index allocation
pointers remain accessible in `Assets<Mesh>`. This pointer/count check is distinct
from the adapter's earlier exhaustive external bit comparison.

The absolute output directory's parent must already exist outside this workspace.
Canonical parent identities are checked before exclusive directory creation.
Files use `create_new`; existing directories/files are never overwritten. Three
fixed PNGs (`raw-z.png`, `raw-x.png`, `raw-oblique.png`) and `diagnostic.txt` are
the only intended outputs. PNG writes have a 2 MiB cap each; metadata has a 2 KiB
cap. Success/help exit 0, diagnostic failure 1, invalid arguments 2. Failures may
leave only task-owned partial outputs; the tool performs no deletion. Path identity
checks are not locking or adversarial concurrent-path-change protection.

## Chosen camera and material policy

Bounds use finite f64 arithmetic without altering raw arrays. Camera admission
requires absolute components <= 10,000,000, enclosing half-diagonal radius
`0.001 <= r <= 1,000,000`, and `r >= max_absolute_component * 0.00001`.
These conservative diagnostic policies avoid extreme or poorly resolved f32
camera math; they are not BO2 limits. Empty/reference-free snapshots and
unframeable extents return typed errors, even where the parser contract is valid.
Degenerate or edge-on geometry may yield a typed background-only result rather
than an invented repair.

The camera looks at the raw bounds midpoint from three radii away. Fixed square
orthographic dimensions are `2.5r`; near/far are `r`/`5r`. The bounding sphere
therefore fits all directions with nominal 10% margins per edge; depth lies in
`[2r,4r]`. Bevy owns quaternion/view/projection/frustum math. These are camera
translations/projections, not mesh transforms or original units.

| View | Camera offset direction | Chosen up |
| --- | --- | --- |
| raw-Z | positive raw Z | raw Y |
| raw-X | positive raw X | raw Z |
| raw-oblique | normalized `(1,1,1)` | raw Y |

All three targets are 512x512 RGBA8 sRGB. One opaque neutral StandardMaterial
uses sRGB `(0.72,0.72,0.72)`, `unlit=true`, `cull_mode=None`. The clear color is
sRGB `(0.04,0.04,0.04)`. Tonemapping, deband dithering and MSAA are explicitly
disabled. No lights, textures, original normals/UVs or postprocessing are supplied.
The final display adds Bevy's built-in WireframePlugin with its triangle topology
and one-pixel line width, on the same Mesh3d and original indexed DrawMesh buffers.
Its neutral dark color is sRGB `(0.12,0.12,0.12)`. No mesh data, topology, winding,
attributes or Transform changes accompany the overlay. The pass runs after the
solid main pass, reuses its depth attachment, inherits reverse-depth GreaterEqual,
and sets slope-scale depth bias to 1 for triangles. Unlike the unculled solid,
the built-in overlay inherits MeshPipeline's backface culling. These are Bevy
diagnostic visibility rules: hidden/back-facing source edges are not guaranteed
visible. Broad flat areas remain under this rule and fixed 512x512 framing; this
display does not itself identify an original calibration patch or original side.
No original visible-side/front or appearance inference follows.

## Bevy lifecycle and resource limits

The tool's member-specific `bevy_pbr` feature enables the pinned engine's render,
material, camera, light, shader and core-pipeline dependencies. Central exact
version/default policy stays unchanged. Direct `image=0.25.10` enables only PNG
encoding and reuses the existing locked image version. The lock adds this member
and 79 upstream packages, without removing or changing existing package versions.
Lock-only optional/target packages are not runtime plugins. No winit, native
window creation, audio, input automation, new AssetLoader or game startup is added.
All seven local crates forbid unsafe code.

`--check-gpu` initializes the bounded offscreen engine without a scene or image
capture, and checks both adapter-supported and device-enabled POLYGON_MODE_LINE
and IMMEDIATES. Rendering explicitly requests those features and repeats that
admission before scene creation; missing features produce a typed error, and
initialization failure/panic remains redacted. Other backends/adapters remain
unqualified. No fallback renderer or generated attributes conceal a gap.

DefaultPlugins are assembled with WindowPlugin's primary window disabled and
no-window exit disabled. The default asset source is a Bevy MemoryAssetReader;
built-in embedded shaders use Bevy's asset facilities. No filesystem asset load
is requested. The tool drives the engine's existing `App::update` lifecycle;
Bevy retains schedules, ECS, asset ownership, render extraction and renderer.
There is no custom render graph, pipeline, ECS or scheduler. The renderer is not
pipelined because `multi_threaded` is not requested by this member.

Capture waits for the actual indexed U32 RenderMesh/counts, nonempty pipeline
cache and no pending pipelines, after at least sixteen engine updates. Bevy's
transient ShaderNotLoaded/ShaderImportNotYetAvailable errors are retried by its
cache and do not become premature tool failures. Terminal pipeline errors remain
typed. Three Screenshot image entities request one asynchronous capture each;
ScreenshotCaptured delivers bounded CPU images and Bevy removes its capture
entities. Layout validation, dark opaque background, neutral foreground and an
eight-pixel clear border precede output. Actual synthetic qualification additionally
checks independent projected triangle masks, as recorded in
[findings 0014](../reverse_engineering/0014-bevy-raw-geometry-render-qualification.md).

There are at most 600 updates and a 60-second deadline checked between engine
calls. Initialization, GPU/driver/file calls can still block; an independent
120-second supervisor is required for qualification. The app is dropped before
encoding output. Parser logical allocation limits do not cap Bevy, shader compiler,
GPU staging, driver allocations or process memory. Three targets and three retained
RGBA captures are fixed-size; engine readback/staging and mesh extraction have
their own temporary allocations. Bevy may clone CPU arrays for render extraction;
the tool's original CPU geometry remains retained.

Public diagnostics contain type/stage, ErrorKind, counts and chosen-policy status,
never source paths, coordinate/index values, malformed fields or raw OS messages.
Recoverable panics are redacted and mapped to EnginePanic; aborts/allocator aborts
and errors on independent engine threads are not a recoverability guarantee.
The tool installs no log subscriber. Detailed excluded synthetic pipeline-probe
logs and all qualification data remain external. Upstream wgpu-hal's transitive
renderdoc-sys bindings are not SDK execution: the inspected Windows hook probes
an already loaded module, never loads an SDK, and capture APIs are unused here.

## Pinned technical evidence

- [Bevy 0.19.1 offscreen example](https://docs.rs/crate/bevy/0.19.1/source/examples/app/headless_renderer.rs)
  and [image-target example](https://docs.rs/crate/bevy/0.19.1/source/examples/3d/render_to_texture.rs).
- [Mesh render extraction and RenderMesh](https://docs.rs/bevy_render/0.19.1/src/bevy_render/mesh/mod.rs.html),
  [render asset ownership](https://docs.rs/bevy_render/0.19.1/src/bevy_render/render_asset.rs.html),
  [screenshot image targets/readback](https://docs.rs/bevy_render/0.19.1/src/bevy_render/view/window/screenshot.rs.html),
  and [pipeline cache retries](https://docs.rs/bevy_render/0.19.1/src/bevy_render/render_resource/pipeline_cache.rs.html).
- [Position-only vertex specialization](https://docs.rs/bevy_pbr/0.19.1/src/bevy_pbr/render/mesh.rs.html),
  [guarded mesh shader](https://docs.rs/crate/bevy_pbr/0.19.1/source/src/render/mesh.wgsl),
  [unlit fragment branch](https://docs.rs/crate/bevy_pbr/0.19.1/source/src/render/pbr.wgsl),
  and [material/culling controls](https://docs.rs/bevy_pbr/0.19.1/src/bevy_pbr/pbr_material.rs.html).
- [Built-in wireframe feature admission, indexed thin draw and depth pass](https://docs.rs/bevy_pbr/0.19.1/src/bevy_pbr/wireframe.rs.html)
  and [wireframe example](https://docs.rs/crate/bevy/0.19.1/source/examples/3d/wireframe.rs).
- [Orthographic projection/scaling](https://docs.rs/bevy_camera/0.19.1/src/bevy_camera/projection.rs.html)
  and [image target allocation](https://docs.rs/bevy_image/0.19.1/src/bevy_image/image.rs.html).

These exact cached release sources were inspected locally. Graphical qualification
covers the measured local Vulkan path only; original comparison, exhaustive hidden-edge
visualization, other platforms/backends, forced allocation failure and sustained
runtime are not established. See [handoff 0020](../handoffs/0020-bevy-offscreen-geometry-diagnostic.md).
