TASK:
Optimize rendering and memory using measured Bevy/Tracy evidence, preserve every dynamic lantern shadow, remove camera-rotation shadow popping, make SMAA default and commit locally.

STATUS:
COMPLETE - rendering optimization milestone. This does not guarantee 60 FPS everywhere or complete the original-game reconstruction.

FINDINGS:
- Baseline room 8 averaged 2.81 FPS. Final uninstrumented release at physical 1920x1080 averages 58.44 FPS settled, 49.32 during continuous rotation and 36.05 during six-kill combat. Rotation p95 is 36.59 ms; combat p95 is 36.59 ms. Loading/capture readback is outside steady averages; combat includes the round intermission. Earlier faster runs are not the final results.
- Tracy identified fragmented mesh/material bindings, repeated cube filtering, shadow preparation/drawing and visibility work. Bindless materials, one shared water clock, immutable probe filtering, CPU batching and reuse of unchanged Bevy shadow views remove repeated work.
- Every one of the 331 lanterns retains shadows. Stable cube ownership replaces camera/distance toggles. Rotation reports 331..331 enabled lights, 6,956 rendered versus 6,848,716 reused faces, and only two texture allocation changes during startup/assembly. Dirty caster bounds, material changes, lights, shaders and projection changes invalidate reuse.
- The cached and uncached settled room PNGs are pixel-identical at the same pose. Full rotation captures show intact geometry and lighting; snapshots do not establish absence of every possible transient artifact.
- Memory working set fell from the earlier measured 6,066 MiB to 4,268 MiB (about 30%). Private committed memory is still 10,589 MiB, versus 12,370 MiB earlier. Bevy releases 2,108,274,384 image CPU bytes and 219,109,056 mesh CPU bytes after assembly. These are process observations, not isolated VRAM measurements.
- SMAA is default, with only its plugin installed. All-lantern shadow face resolution is explicitly 512 instead of 1024, reducing nominal depth storage from 7.8 to 2.0 GiB with less fine shadow detail. Msaa4 and shadow resolution remain configurable.

EVIDENCE:
External profiling receipts/captures: baseline-native, slab-4096-memory-native, delivery-native-full, delivery-smaa-moving, delivery-smaa-combat, delivery-smaa-uncached and delivery-tracy. Native and instrumented measurements are kept separate. Profiling tooling uses the Cargo-locked matching Tracy 0.14.0 client; CPU elapsed zones include waits, GPU timestamps remain separate. The preceding external full-quality rendering plan review remains in Git history at 21c7d28.

CHANGES:
Bevy bindless material/shader adapter and baked-light specialization; shared water clock; filter retained environment cubes once; stable all-lantern shadow allocation and invalidation-aware phase/view/visibility reuse; CPU mesh batching/clustering; preparation residency release; flattened collision-only entities; shared zombie presentation assets; avoid unchanged camera/HUD writes; default SMAA; optional Tracy builds and validated flamegraph exporter; native static/moving/combat measurement modes and permanent pipeline-error detection. Runtime asset closure is unchanged.

VALIDATION:
Actual ordinary release --verify passed movement, six Round 1 kills, Round 2, restart and all fifteen room/map/water views. Moving benchmark passed 66 virtual seconds with twelve direction captures; final uncapped combat and uncached-reference runs passed. Room cached/uncached RGB difference is exactly zero. Exact asset closure validates 1,252 runtime files. Locked Cargo metadata, workspace release build, formatting and diff whitespace gates passed; the final executable is an ordinary uninstrumented release. Final Tracy combat export validates 3,079,332 CPU zones over 339 frame intervals/11.3404 seconds on 14 threads, with zero crossing zones and exact self-time conservation; GPU zones are separate. No unit tests, fixtures or Clippy; no push.

Actual failures repaired: shader symbol rewriting; duplicate diagnostic plugin under tracing; missing zones from log filtering; CPU residency mutation invalidating uploads; incorrect GPU-preprocessing mode dropping opaque geometry; resource exhaustion with large all-lantern GPU work buffers; stale running executable blocking one build. False high-FPS missing-geometry results and the rejected light-count cap are excluded from performance claims.

UNRESOLVED:
Combat and some camera directions remain below 60 FPS; CPU preparation/visibility and gameplay spatial queries remain measurable costs. Startup shader warmup, long-session memory stability and manual play are unverified. CPU mesh bytes are unavailable for hot map reassembly without asset reload; ordinary restart retains the assembled world. Shadow reuse currently assumes one game camera; skinned casters invalidate conservatively. Bindless fallback is present but not exercised on another GPU. Original numeric/pixel equivalence is not claimed.

RECOMMENDED NEXT WORK:
Use the retained native workloads and Tracy tools for any future optimization. Recheck multi-camera/dynamic-alpha/skinned content when introduced, and validate extended manual play before broad performance claims. Continue the complete gameplay/content roadmap separately.

SCOPE NOT IMPLEMENTED:
New Forward+ or GI features from the separate external plan, original actor/weapon replacement, atmosphere reconstruction, other missing gameplay/content, multi-camera cache generalization, packaged distribution, tests/Clippy or push.
