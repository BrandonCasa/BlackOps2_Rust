# Renderer plan 3.0

## Persistent goal and constraints

Implement every facet below. Qualify native 1920x1080 with DDGI enabled on the
recorded RTX 4080: at least 80 average FPS in optimized debug and 100+ rendered
FPS in release, both with diagnostic tooling disabled. Preserve accepted material detail, colored transport, shadows,
reflections, gameplay and first-person framing. Mild temporal softness while
moving is permitted; settled detail and HUD remain crisp. No unmeasured guarantee
or completion claim. Ordinary-frame target is 10 ms; record every miss.

One stable-Rust application/window, Bevy rendering/ECS ownership, Avian physics,
application `forbid(unsafe_code)`, locked dependencies, original assets untouched,
and existing total VRAM cap remain requirements. Dependency patches are allowed.
Work sequentially in this chat under the latest project instructions. Subagents,
delegated chats, unit suites, synthetic fixtures and Clippy are banned.
Use one actual-game validation slot. Preserve executable identities immediately
after builds. Keep the current handoff concise; this plan is the persistent scope.

## RC migration first

Use Bevy 0.20.0-rc.2 (tag commit 343a0233d6788a49f224a3684c8995e246663df3)
as the engine base, including the matching vendored render/PBR/Solari crates.
Pin Avian's upstream Bevy 0.20 compatibility revision and maintain one Bevy
version throughout the graph. Port custom shaders to WESL and preserve native
lighting, shadow corrections, material source semantics and DDGI. Adopt the RC's
incremental Solari extraction, retained records, sparse uploads and cached
bindings; remove superseded 0.19 whole-scene gates and material-table adapters.
The compatibility milestone passes the optimized-development DDGI build and
focused actual-game combat, ADS, Round 2 and restart. All 68 engine crates resolve
to rc.2. Required vendor fixes preserve the callable native vertex implementation
and unchanged live shadow inventories under incremental light extraction.
The initial full survey reached thirteen views but exceeded its deadline;
after the light-admission/storage milestone, frozen executable-14 passes all
17 views, combat, ADS, Round 2 and restart with 1502 DDGI/reflection updates.
The port executable also passes all 32 controls checks, including repeat clicks.
Updating the engine does not qualify either FPS target.
Proceed directly to reducing fresh ray, reflection and auxiliary-view work;
avoid another preliminary benchmark campaign.

## Source findings and attribution

The user-reported 6.2 FPS off / 4.3 FPS on means 161.3 / 232.6 ms if matched;
removing all DDGI overhead would still miss 10 ms by 16x. This is not a new
qualified benchmark. The pre-migration 0.19.1 source audit found per-pose ray mesh asset churn and static
BLAS compaction, broad scene extraction/TLAS buffer reconstruction on pose changes,
unconditional 256-square six-face reflection tracing and filtering, GI fields
sampled before cascade ownership selection, exact camera-height sky invalidation,
auxiliary first-person shadow views, disabled GPU light clustering, and general
material evaluation. Existing local face caching, compact skin sources and
unchanged-pose skipping must be extended, not duplicated. Final post-processing
is already shared. First-person ray participation is currently excluded.

The RC focused validation admits 331 shadowed point lights: lanterns explicitly
carry NoFrustumCulling to stabilize old cache ownership. The cache already checks
texture-slot ownership, so retain that correctness while restoring conservative
native light admission. All off-screen lights must remain available to ray GI and
reflections. The RC capture's 106 instrumented frames average approximately
25.2 ms exclusive shared/per-view shadow drawing versus 3.5 ms DDGI tracing;
these figures guide work and do not qualify diagnostic-free performance. Remove
the explicit CPU light-clustering override only after validating native GPU lists
and capacity. Remove unused full-scene mesh/texture inventories left behind by
the retired Solari gates; changed-only native preparation should do that work.
The first post-port milestone implements that admission and inventory cleanup,
including native hidden-light/view-owner retirement and retained high-water
shadow arrays. The latter repairs memory exhaustion from changing exact-size
arrays in the transient texture cache. Actual-game survey acceptance passes;
the development combat receipt is 8.05 FPS average, so performance remains far
below target and matched release adoption remains unqualified.

- [ ] Repair per-frame/view/invocation/submission attribution and repeated-path
  aggregation; distinguish inclusive parents from exclusive leaf work.
- [ ] Capture raster shadows, BLAS/TLAS preparation, reflection tracing/filtering,
  DDGI updates/receiving, uploads and CPU render preparation in debug only.
- [ ] Account for CPU busy/wait time, gameplay fixed-step catch-up, allocation,
  submission, GPU execution and presentation with pipeline overlap.
- [ ] Use controlled actual-game ablations for reflections, filtering, ray poses,
  sky, GI updates/receiving, shadow groups, screen effects and auxiliary views.
  Ablations cannot qualify final quality or FPS.
- [ ] Establish matched room, rapid outdoor traversal, indoor transition and
  six-enemy combat baselines; record work counters and outstanding asset work.

## A. Retained ray scene and shared deformation

Persistent posed mesh assets now replace per-pose handle/UV/index churn, with
reused position/normal/tangent scratch buffers and preserved authored sidecar IDs.
An extracted native ECS marker separates dynamic fast-build/no-compaction from
static fast-trace/compaction; replacement jobs cannot alias a newer BLAS under
the same mesh ID. Native previous-TLAS retirement remains intact. Frozen
executable-17 passes focused combat/ADS/Round 2/restart and 604 DDGI/reflection
updates; all 12,746 pose updates reuse their mesh, with zero recovery allocations.
CPU pose calculation and complete native mesh uploads still execute; shared GPU
deformation, real refitting and matched release construction/traversal acceptance
remain open. The earlier actor presentation defect remains visible.

- [ ] Retain stable instances, geometry/descriptors/material records, buffer and
  TLAS capacity; extract/upload changed ranges.
- [ ] Separate membership, transform, pose, material-content, residency and
  lighting revisions with affected old/new bounds.
- [ ] Replace per-pose mesh creation with persistent dynamic geometry; share pose
  preparation/GPU deformation across raster and ray consumers where compatible.
- [ ] Preserve triangle/material mapping, normal transforms, tangent handedness,
  alpha coverage and off-screen GI/reflection blockers.
- [x] Separate static fast-trace/one-time-compaction and dynamic no-compaction
  construction policies; start with retained dynamic rebuilds.
- [ ] Obtain real BLAS update support through safe application APIs and qualify
  build-plus-trace cost against rebuilding. Pinned wgpu-core only rebuilds despite
  PreferUpdate; the flag alone is not implementation.
- [ ] Replace private engine-system-name matching with explicit ECS hooks.
- [ ] Bound expensive rebuild work without exposing mixed poses or stale bindings;
  retain valid rebuild fallback and account for memory and traversal degradation.

## B. Reflections and environment reuse

Native filter preparation now retains per-entity intermediate textures and
generation bindings/uniforms/mip views/placeholders against relevant texture,
layout, sampler and downsampling identities. Per-face adaptive admission,
screen-space reconstruction and matched release performance remain open.

The first bounded capture/filter milestone now uses an ECS RenderBudget for
elapsed-time admission: desired 100 ms age, earlier after 0.25 m translation or
numeric direct-light changes. Reused cubes keep their actual world-space capture
anchor; rotation alone does not invalidate them. Texture replacement stays
pending until the producer records a full new cube. Native generation tracks
completed source/downsampling/filter revisions with the retained binding keys;
unchanged sources skip fresh copy, mip generation and convolution. Ordinary
native producers retain their automatic behavior. Frozen executable-18 passes
the full 17-view/combat/ADS/Round 2/restart survey with 1447 DDGI batches, 895
fresh cube updates and 552 reused frames. This proves reduced fresh admission,
not matched release speedup or full temporal quality. Local dirty faces/tiles,
confidence/variance, adaptive samples and screen-space replacement remain open.
RenderBudget currently covers reflection bounds; other subsystem timing,
memory and admission consumers remain open.

- [ ] Retain cube faces, hit/depth validity, source generations, confidence,
  variance and filtered mip generations in world-anchored regions.
- [ ] Dirty face/tile invalidation includes geometry, emitters, materials, sky,
  resource generations and translation coverage; rotation alone does not dirty a
  world-space cube. Retain accepted cube resolution initially.
- [ ] Bound dirty tile tracing and convolution; reuse filter uniforms, bindings,
  mip views and placeholder allocations. Preserve cube seams/GGX semantics.
- [ ] Admit roughness/confidence/variance samples before tracing; bound secondary
  reflection work by qualified contribution error.
- [ ] Half-width/half-height screen-space hits with world-space fallback replace
  substantial live cube work rather than stacking another full reflection path.
- [ ] Compact checkerboard work lists before expensive evaluation. Invalid history
  uses valid coarser sources; preserve dynamic lighting and parallax recovery.

## C. DDGI production and receiving

Probe admission now uses a reused bounded set, preserving first admission,
flags/order and uniqueness without scanning the growing output vector.

- [ ] Select finest supported cascade before expensive sampling; sample coarse
  fields only for boundaries or missing fine support. Preserve initialized black,
  distance visibility, relocation and RGB decode-before-interpolation.
- [ ] Measure receiving separately from updates and reflection-field reads.
- [ ] Replace explicit later-bounce walks with one completed previous-field
  feedback; never consume partial transport. Preserve untouched sparse entries
  without blindly copying a full field every frame.
- [ ] Reuse classification until geometry/relocation/maintenance invalidation;
  fuse compatible distance-moment work with admitted ray results.
- [ ] Prioritize nearby dirty probes and reserve maintenance with elapsed-time
  deadlines; do not unconditionally update 1/8 of the 49,152 probes each frame.
- [ ] Qualify confidence/change/variance-based 16/32/64-ray tiers, sparse update
  budgets and thin-wall/door/emissive recovery.

## D. First-person and shadows

The current viewmodel entities already have world-space GlobalTransforms and
native PBR evaluation. They render through a second HDR camera with its own
projection and shadow/view preparation. The RC's core depth preparation shares
the main depth texture by render target/MSAA, so this camera's cleared depth
attachment overwrites world depth. Preserve the existing world transforms,
presentation projection and single final post chain while replacing that
shared depth with owned first-person storage and removing duplicate view work.
First-person geometry is still explicitly excluded from ray-scene admission.

Owned first-person depth now overrides native target-shared depth after core
preparation. Storage survives ordinary movement/ADS/animation, and retires on
view removal, target suspension, size/sample/usage changes or device recovery.
Frozen executable-19 passes the full actual-game survey with 1630 checked views,
zero world-depth aliases or shared-HDR mismatches, one 8294400-byte allocation
and 1629 reuses. The separate late camera and its directional cascades remain;
the architectural checklist below is still partial. Existing actor garment
artifacts remain open. Debug combat averages 8.1943 FPS; targets are unmet.

Point/spot faces with proven direct work and no occlusion now use a two-system
native schedule: early mesh preprocessing, then shared shadow drawing. Routing
checks actual phase/work-item mode and absence of unpacking jobs after resource
preparation; unsupported cases keep Core3d. This removes unused camera-stage
invocations and empty uniform-allocation passes. Frozen executable-20 passes
the full survey with all 177530 fresh cube faces routed and zero fallbacks;
first-person depth/HDR checks remain valid. Debug combat averages 8.7830 FPS,
without matched release adoption or a causal speedup claim. The cache still
forces direct point-face batches. Native mesh drawing explicitly cannot
multi-draw in that mode. A five-stage native indirect candidate compiles, but
frozen executable-21 fails the 180-second game deadline with one kill and no
shots; the combat HUD shows about 3.9 FPS. No completed performance receipt is
available. The candidate is removed and its patch/failure preserved externally.
Per-face uniform allocation, bin unpacking and indirect construction can outweigh
CPU batching savings. Reduce/group this work before another indirect trial.

Actor bounds now retain Bevy's native DynamicSkinnedMeshBounds instead of
overwriting the influencing-joint envelopes with all rig joints plus a margin.
Missing/invalid skin data or morph deformation retains the former fallback.
Frozen executable-22 passes combat, ADS/recoil, Round 2, all 17 views and restart:
24201 native mesh-frames and no fallback. Debug combat averages 8.1691 FPS;
this qualifies behavior, not matched performance adoption. Native bounds still
include unindexed accessor vertices; body primitives share 6221 source vertices,
so indexed-envelope tightening remains open. The run also records 45534 phase
resets and 1613271 invalidated specialization entries. Deferred restoration
exposed lost phase-mode ownership; the following milestone repairs it.

Deferred revival retention is repaired: select records every retained view it
processes before queuing restoration, rather than pruning mode ownership from
the not-yet-updated query. Its set reuses allocation capacity. Frozen
executable-23 passes combat, ADS, Round 2, all 17 views and restart. Resets are
12144 versus 45534 in the preceding survey; specialization invalidations are
1285231. These are separate survey work counts, not matched release speedup.
Combat averages 8.4972 FPS and all 493 samples exceed 10 ms. Further reset causes
remain open. The world map already has spatial chunks; sample inspected chunks
have compact positions. Recoil was not observed in this run.

Native local-light visibility now retains a conservative 16-meter caster grid,
refreshing changed bounds/transforms and retiring departed query members once
per frame. Native range/layer/visibility/frustum tests receive deduplicated nearby
candidates. Missing, invalid, oversized or forced-visible bounds remain global;
oversized light queries retain exhaustive fallback. Frozen executable-24 passes
combat, ADS, Round 2, all 17 views and restart. It retains 18188 casters, eight
global casters and 1063 occupied cells. Across 34074 spatial queries, 28679813
candidates replace 621875810 possible exhaustive visits, a 95.388% reduction;
no exhaustive fallback occurs. Combat averages 8.4957 FPS, effectively unchanged
from the preceding unmatched survey. Median/p95/p99/worst are
79.3587/268.2606/323.1268/661.7119 ms; all 497 samples exceed 10 ms. The grid adds
hash lookup/membership and candidate collection costs. These work counts prove
pruning, not a frame speedup or optional release adoption. Both FPS targets remain
open. Removing this repeated scan alone is insufficient; proceed to replacing
fresh multi-bounce DDGI work and reducing the remaining frame critical path.

- [ ] Late first-person HDR phase replaces auxiliary full view, with separate
  cleared depth and ordinary internal depth testing. Preserve world depth.
- [ ] Preserve FOV, hip/ADS framing, animation, recoil, attachments and muzzle
  transforms; evaluate lighting in world space, share main world shadow coverage.
- [ ] Supply first-person depth/normal/motion/ownership and localized self-AO;
  composite before the existing single bloom/tone/exposure/output chain.
- [ ] Add appropriate first-person ray participation through retained geometry;
  gameplay obstruction/bullets/melee/physics remain unchanged.
- [ ] Tighten animated caster bounds with influenced-bone envelopes while proving
  coverage. Retain local face caching and reduce dirty-face preparation/submissions.
- [ ] Keep current directional correctness; tighten broad depth interval only with
  proven coverage. Never reuse cascades solely because sun direction is unchanged.
- [ ] Qualify adaptive PCSS and temporal sun visibility only after measuring cost;
  retain thin blockers, current contacts and responsive direct illumination.

## E. Raster, materials, CPU and residency

- [ ] Specialize bounded simple/tiled/layer-count/masked/animated/water material
  families; remove replaced samples and unused vertex inputs while preserving
  normal response, anisotropy, complete mips and authored boundaries.
- [ ] Shared offline world LOD: initial 1-pixel projected error, 15% hysteresis,
  native first-person detail, preserved silhouettes/blockers/tangents/coverage.
- [ ] Instance repeated geometry and batch compatible draws with useful spatial
  visibility granularity; retain native frustum/indirect work.
- [ ] Restore/qualify native GPU light clustering with custom shadow/layer support;
  bound influences and retain light data. Profile CPU and GPU critical paths.
- [ ] Precompute residency camera constants; remove per-region temporary
  allocations; retain demand history, movement prefetch and hysteresis.
- [ ] Distinguish residency binding changes from authored material changes;
  time/byte-bound uploads and record oversized/outstanding work.
- [ ] Profile physics/navigation/catch-up without lowering gameplay tick fidelity.
- [ ] Evaluate depth/prepass economics, lean alpha-correct shadow/depth paths,
  retained pipeline keys/bind groups/allocations and redundant copies/submissions.
- [ ] Introduce 64 MiB bounded surface-response cache only if specialization leaves
  a measured static-material bottleneck; fallback to ordinary evaluation. Include
  all history/AS/shadow allocations inside total VRAM cap.

## F. Temporal policies, execution and advanced candidates

- [ ] ECS RenderBudget: targets, delayed timing estimates, bounded admission,
  memory and time deadlines; SceneRevisions: independent revisions/affected bounds;
  TemporalHistoryState: per-view transforms, ownership, validity, age/confidence.
- [ ] Half-resolution diffuse receiving and rough reflections; rotating sparse AO
  with native silhouettes; retained reduced-resolution volumetrics; effect-specific
  depth/normal/motion/reactive rejection and reconstruction.
- [ ] Camera cuts/resizes reset screen histories; ordinary movement preserves
  valid world caches. Lighting stays consistent linear/pre-exposure.
- [ ] Initial stale dynamic visible limit 50 ms; low-confidence stable maintenance
  250 ms, qualified against motion. Valid unchanged world caches may persist longer.
  Dirty direct-light/visibility changes have immediate priority.
- [ ] Visible atmosphere remains current; separate its updates from quantized,
  error-bounded distant sky lighting. Cache atmospheric lookups by input revision.
- [ ] Water/transparency share scene resources, bound work to coverage and mark
  reactive pixels; current normals/edges and depth-aware composition are required.
- [ ] Cull/batch particles/decals; qualify reduced-resolution soft effects while
  sparks/impacts retain native detail and bounded light/shadow influence.
- [ ] Share compatible downsample chains, update HUD on changes, reuse persistent
  targets/buffers, remove redundant clears/copies/synchronization and inspect shader
  divergence/register pressure/workgroup/cache behavior.
- [ ] Candidate history/classification/reconstruction cost must be less than
  avoided fresh shading/tracing/filtering work. Budget controllers need hysteresis
  and bounded backlog; record deadline misses rather than retain invalid history.
- [ ] Only after simpler work, qualify async complementary workloads, sorted
  ray-hit materials, selective VRS, reservoir reuse and reduced precision through
  supported facilities. Defer unsupported methods. Two-phase occlusion requires
  new favorable economics and cannot coexist with incompatible deferred paths.

## Qualification and completion

- [ ] Coherent milestones pass actual-game checks, locked metadata/build,
  formatting and affected asset closure; final full gates before integration.
- [ ] Instrumented attribution and diagnostic-disabled debug/release measurements remain separate.
  Target 8 ms GPU and <5 ms CPU preparation with overlapping execution, not sums.
- [ ] Matched three-run debug average >=80 FPS in each defined scenario at native
  1080p/DDGI with approved quality. Release requires ordinary warmed frames <=10 ms,
  not merely a 100+ FPS average. Include median/p95/p99/worst and every >10 ms
  ordinary frame; minimum100 qualification requires zero observed ordinary misses.
- [ ] Guard game/compiler/profiler/capture exclusivity and preserved executable
  identity. Separate initialization/cold streaming and warmed measurements.
- [ ] Actual motion: room/outdoor/indoor/combat, rapid turns/strafing/sprint,
  ADS/recoil/reload/hands at walls, zombies/machinery/doors, lights/emissives/water,
  streaming, game-over/restart; inspect RGB GI/leaks/shadows/LOD/textures/reflection
  trails/particles/first-person self-occlusion and invalidation recovery.
- [ ] Optional candidate adoption requires >=3% matched release median FPS gain,
  <=3% p95 regression and quality approval. Required architecture remains required.
- [ ] If unmet, identify residual measured bottleneck and explicit quality/cost
  proposals. No silent DDGI removal/resolution reduction/gameplay simplification.
- [ ] Final requirement-by-requirement evidence audit proves all mandatory facets;
  keep goal active while implementation or qualification remains incomplete.
