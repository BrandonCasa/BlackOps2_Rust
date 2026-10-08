# Renderer plan 3.0

## Persistent goal and constraints

Implement every facet below. Qualify native 1920x1080 with DDGI enabled on the
recorded RTX 4080: at least 80 average FPS in optimized debug and 100+ rendered
FPS in release. Preserve accepted material detail, colored transport, shadows,
reflections, gameplay and first-person framing. Mild temporal softness while
moving is permitted; settled detail and HUD remain crisp. No unmeasured guarantee
or completion claim. Ordinary-frame target is 10 ms; record every miss.

One stable-Rust application/window, Bevy rendering/ECS ownership, Avian physics,
application `forbid(unsafe_code)`, locked dependencies, original assets untouched,
and existing total VRAM cap remain requirements. Dependency patches are allowed.
Consult subagents for implementation decisions; every atomic commit requires a
critic score greater than 8/10 for its exact diff. Rendering-logic unit tests are
authorized; Clippy, unrelated suites and synthetic asset fixtures remain banned.
Use one actual-game validation slot. Preserve executable identities immediately
after builds. Keep the current handoff concise; this plan is the persistent scope.

## Source findings and attribution

The user-reported 6.2 FPS off / 4.3 FPS on means 161.3 / 232.6 ms if matched;
removing all DDGI overhead would still miss 10 ms by 16x. This is not a new
qualified benchmark. Source audit found per-pose ray mesh asset churn and static
BLAS compaction, broad scene extraction/TLAS buffer reconstruction on pose changes,
unconditional 256-square six-face reflection tracing and filtering, GI fields
sampled before cascade ownership selection, exact camera-height sky invalidation,
auxiliary first-person shadow views, disabled GPU light clustering, and general
material evaluation. Existing local face caching, compact skin sources and
unchanged-pose skipping must be extended, not duplicated. Final post-processing
is already shared. First-person ray participation is currently excluded.

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

- [ ] Retain stable instances, geometry/descriptors/material records, buffer and
  TLAS capacity; extract/upload changed ranges.
- [ ] Separate membership, transform, pose, material-content, residency and
  lighting revisions with affected old/new bounds.
- [ ] Replace per-pose mesh creation with persistent dynamic geometry; share pose
  preparation/GPU deformation across raster and ray consumers where compatible.
- [ ] Preserve triangle/material mapping, normal transforms, tangent handedness,
  alpha coverage and off-screen GI/reflection blockers.
- [ ] Separate static fast-trace/one-time-compaction and dynamic no-compaction
  construction policies; start with retained dynamic rebuilds.
- [ ] Obtain real BLAS update support through safe application APIs and qualify
  build-plus-trace cost against rebuilding. Pinned wgpu-core only rebuilds despite
  PreferUpdate; the flag alone is not implementation.
- [ ] Replace private engine-system-name matching with explicit ECS hooks.
- [ ] Bound expensive rebuild work without exposing mixed poses or stale bindings;
  retain valid rebuild fallback and account for memory and traversal degradation.

## B. Reflections and environment reuse

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

- [ ] Coherent milestones pass rendering-logic tests, locked metadata/build,
  formatting and affected asset closure; final full gates before integration.
- [ ] Debug attribution and uninstrumented release measurements remain separate.
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
