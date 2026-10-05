# Bevy raw-geometry render qualification

Task 0020 asks whether the accepted position-only diagnostic Mesh can be rendered
and read back through pinned Bevy facilities without modifying raw geometry.
This is a diagnostic qualification, not investigation of original camera/material
semantics or an appearance comparison. Original-data understanding remains
**INVESTIGATING**; overall, gameplay and visual fidelity remain **UNKNOWN**.

## Sources and procedure

The task inspected architecture/ADR0001, findings0008-0013, handoffs0013/0014/0019,
the actual reader/adapter/tests, inspector error boundary and pinned cached Bevy
0.19.1 release sources. Relevant sources are linked in the
[architecture record](../architecture/diagnostic-geometry-render.md). Standard
mesh specialization guards absent vertex attributes; its built-in unlit fragment
branch uses base color. Actual Vulkan execution qualifies position-only support
on the measured backend without generated normals/UVs or a custom shader.

Only one already accepted snapshot identity is admitted: 4,404,568 bytes,
SHA-256 `ffe639e3779310e300d7562f91f9ebf8c659126472e2d583558882031215181f`,
191,259 positions, 449,721 indices, 149,907 triangles and 6,469 surfaces. Its task0013
provenance remains linked to Steam Zombies build24784239 English and the accepted
external diagnostic producer; neither the producer nor original source is read
or executed here. Beyond this already admitted project-format snapshot, no other
original game, process, executable, fastfile, SDK, input, profile, Steam operation,
script, shader or source-array dump is accessed.
Inspection of Bevy's public shaders is distinct from original shader access.

External orchestration resolves only inspection_output through ignored TOML using
tomllib. Named task0013 observations-v2/production1 supplies the retained artifact;
there is no recursive original discovery. Canonical containment, same-handle
metadata and exact input hash/length admission precede each accepted run. A held
read-only handle is independently rehashed afterward, and a reopened handle
checks path/file identity. Equal byte length/hash/mtime/device/file-ID receipts
are retained for both graphical runs and the later CPU check. They establish
observed preservation, not source locking, authenticity or concurrent-write proof.

The external supervisor uses an owned subprocess, 120-second independent deadline,
256-KiB combined stdout/stderr admission, private logs and joined pipe threads.
Its failure path terminates/reaps the owned process tree; every actual GPU run
exits normally and the process is reaped. This is not a global process inventory
or a claim about unrelated driver services. The executable also has bounded
updates and an inter-call deadline; GPU/OS calls are not preempted by that check.

## Qualified engine output

Hardware/backend: NVIDIA GeForce RTX4080, Vulkan, adapter vendor4318/device9988.
A bounded matching-device CIM check reports driver32.0.16.1692. Rust1.99.0,
Bevy0.19.1 and wgpu29.0.4 are locked; builds use the debug profile. No driver,
SDK or engine installation is performed. Twenty app updates complete each
successful scene run, including actual indexed U32 RenderMesh preparation and
asynchronous ScreenshotCaptured readback, with original CPU allocations retained.
These updates are development policy, not original timing.

The first synthetic attempt stopped at a redacted PipelineFailure in2.797seconds,
35 captured bytes and no PNGs. A separately retained synthetic-only diagnostic
probe exited1 in2.125seconds with2,163 private bytes: ShaderNotLoaded was treated
prematurely as terminal. Pinned cache source explicitly retries ShaderNotLoaded
and ShaderImportNotYetAvailable. Correcting that readiness gate needs no material,
mesh or engine abstraction change; terminal errors still fail. Both failed
artifacts remain excluded from qualification.

The corrected solid baseline succeeds on an arbitrary asymmetric tetrahedron
(4 positions/12 indices/1 opaque surface):4.281seconds,689 stdout bytes,empty
stderr. Independent f64 projected triangle masks match raw-Z/raw-X exactly;
oblique differs by two edge pixels (IoU0.9999323). All targets are512x512 RGBA,
opaque, with a solid dark border and exactly two neutral colors. Its one accepted
snapshot run succeeds in2.782seconds,698 stdout bytes,empty stderr, with foreground
31,619/4,133/22,543 pixels in raw-Z/raw-X/oblique. All complete counts and input
identity checks pass. This baseline proves transport/framing; solid faces alone
do not distinguish enough internal edges for later calibration selection.

Outer review therefore authorized one narrow overlay extension and exactly one
new synthetic qualification followed by one additional run of that same accepted
snapshot. Baseline PNGs, receipts, helper sources and qualifying binary are
preserved. No new input identity or original source is admitted.

Bevy's built-in WireframePlugin requires POLYGON_MODE_LINE and IMMEDIATES.
Feature-only initialization directly checks both adapter support and enabled
device features: exit0 in1.813seconds,161 stdout bytes,empty stderr. Final rendering
explicitly requests both. Thin width1/triangle topology reuses indexed DrawMesh;
the pass runs after the main solid pass against retained depth with reverse-depth
GreaterEqual and triangle slope bias1. The overlay inherits engine backface
culling while the solid remains unculled. It is not original visible-side evidence.

The sole new synthetic overlay run exits0 in2.812seconds,843 stdout bytes,empty
stderr. All images have exactly opaque grayscale10(background),31(edges),184(solid).
Independent projections place all actual edges within1.5pixels of the expected
visible source edges; foreground stays within a one-pixel expansion of projected
triangles and covers their eroded interior. Raw-Z shows three visible faces and
373 internal edge pixels; raw-X/oblique each have one expected visible face and
perimeter edges. Actual visual inspection confirms the internal spokes. This
validates edge display beyond successful initialization or non-background noise.

The sole new accepted overlay run exits0 in2.843seconds,852 stdout bytes,empty
stderr. Complete counts, retained CPU access and input identity receipts pass.
Independent PNG checks find three exact neutral opaque shades, finite coverage
and clear borders in every view:

| View | Synthetic foreground / edge pixels | Accepted foreground / edge pixels |
| --- | ---: | ---: |
| raw-Z | 25,433 / 1,220 | 31,817 / 1,639 |
| raw-X | 20,401 / 650 | 4,520 / 1,574 |
| raw-oblique | 29,937 / 788 | 22,715 / 2,942 |

Worker and outer visual reviews find localized internal detail on accepted raw-Z
and oblique images. Broad flat areas remain under fixed resolution and Bevy's
depth/backface rule; every hidden/back-facing edge is not exposed, and no original
calibration patch is established by these chosen views.

## Raw-X line-rasterization limit

Outer independent baseline/overlay comparison found18 accepted raw-X pixels
beyond a one-pixel Chebyshev expansion of solid coverage. Its extra all-six
one-pixel assertion failed and remains external; that assertion is not adopted
as a universal geometry-preservation claim. Synthetic, accepted raw-Z and
accepted oblique remain within that expansion.

A separately bounded external CPU check reads only the same admitted snapshot,
projects original indexed edges in the chosen raw-X camera and tests those18
pixels. All are the overlay shade and within0.441940pixels of an existing projected
source edge. Nearest-edge triangles have projected minimum altitude at most
0.113572pixels; five have zero projected area. None contains any of the nine
nearby pixel centers under strict triangle-interior tests. Existing subpixel/line
geometry therefore accounts for additional line visibility without a source-array
edit. Exact GPU-selected face attribution remains unproven: shared/overlapping
edges, mixed orientation, depth bias and raster precision preclude identifying
the chosen face from nearest-edge proximity alone. No blanket identical-silhouette
or original-rasterizer claim follows. Same-handle/reopened identity preservation
passes, and no new GPU execution occurs.

An initial CPU receipt write failed on NumPy scalar JSON serialization after the
calculations; its source and invalid empty receipt are retained. A nested system
Python invocation then lacked NumPy before any input read. The final direct
bundled-Python check exits0 with the typed summary and empty stderr. These are
excluded orchestration failures, not geometry evidence. Source coordinates,
index arrays and per-pixel details remain external.

## Retained output identifiers and limits

The named external0020-bevy-offscreen-geometry-diagnostic area retains
synthetic-images, accepted-images, overlay-synthetic-images, overlay-accepted-images,
supervisor.py, solid-supervisor.py, image-check.py, solid-image-check.py,
overlay-image-check.py, raw-x-edge-check.py, private logs, identity receipts and
final-gates.json. The terminal-review.json record covers the final ownership/content/link
review; terminal-identities.json freezes candidate source, logs/receipts, PNGs and
the separately retained qualified-overlay-tool.exe after the final gates. Failed
synthetic/CPU records are preserved separately.
Original-derived PNGs never enter the repository.

Final accepted overlay PNG identities:

| View | Bytes | SHA-256 |
| --- | ---: | --- |
| raw-Z | 11,779 | `d5236bfed6c55b6b33732436f39eca6f99d5942328d49985efcbbb79197948ec` |
| raw-X | 8,454 | `016d937e696ed08dcd3a948acd925c00d923ede7ac2adb441782c01590ea2d45` |
| raw-oblique | 17,483 | `f3cb3feefc3a5d29feb6e437fdadea44cc7554b9e27996f356e69c1b85733173` |

Fixed output/capture dimensions and parser policies are separate from unbounded
engine/driver allocation overhead. No memory-cap, source-authentication, original
units/front/materials, appearance comparison, sustained runtime, other-platform,
other-backend, forced-allocation-failure or gameplay qualification is established.
The [handoff](../handoffs/0020-bevy-offscreen-geometry-diagnostic.md) records code,
locked checks and integration limits; the overall Origins objective is unfinished.

## Outer review

The outer agent reviewed all source, manifests and changed documentation, visually
inspected all six overlay images, checked the CPU edge explanation, and independently
verified all 15 candidate identities and 117 external artifact identities against
the worker terminal manifest. Its SHA-256 is
`d120ea08ad453b6512c1caf6515a08bd28d55e0a8db2bed6d305e1d71cac6ccb`.
Three input-preservation receipts and four successful owned scene-process/pipe
cleanup receipts pass. All six functional hashes match the final 74-test/gate
receipts; 114 local links, exact handoff headings, privacy and whitespace checks
pass. Already passing Rust gates and GPU runs were not repeated.

The retained final executable is 86,028,800 bytes, SHA-256
`8ff17ada808dc5e8a6400424d5fc451732fc65c50cae914037c9c70145c50036`,
verified after the final gates. No launch-time binary hash was captured; the
rendered PNGs therefore lack a cryptographic binding to that retained executable
identity. This limitation is distinct from the checked source/gate identities
and snapshot preservation. The bounded diagnostic is accepted, with original
rendering and overall/gameplay/visual fidelity still UNKNOWN.

Parent review artifacts remain in the named external
0020-bevy-offscreen-parent-review area. The stricter all-view one-pixel assertion
and initial system-Python NumPy import failure remain excluded and retained;
configured-runtime comparison observations and the source-edge explanation
account for the scoped raw-X exception without weakening original fidelity claims.
