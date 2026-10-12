# Upstream Vulkan baseline, 2026-10-12

This is an intermediate baseline, not acceptance of the requested Solari/DLSS
remaster. The user's later mandate requires `dev-dynamic` for all benchmarks.
These are development measurements; they do not establish release performance.
No final migration benchmark exists.

Source parent: `5ec52fb3231c8e53a44a67a71d31b552809163db`, with the dirty source
snapshot in bundle `vulkan-upstream-dev1080-20261012`. The snapshot and archive
identify all compiled inputs. All Bevy 0.20.0 and wgpu 30.0.1 crates resolve from
the upstream registry; project Cargo patch overrides were removed. The unused
vendor directories remain preserved. Vulkan is mandatory and conflicting
`WGPU_BACKEND` settings fail at initialization.

Baseline implementation was subsequently committed as `05c870027`. That commit
also consolidates flythrough mode into `origins.exe --flythrough`; its matching
development bundle is `vulkan-upstream-single-exe-20261012`. The measured bundle
predates that executable consolidation. Its compiled snapshot, rather than a
claim of a clean pre-build commit, is the source identity for measurements.
The surface shader now uses upstream `Globals.time` instead of the removed
vendor-only `real_time`. Upstream virtual time and its rollover semantics apply;
long-session surface-animation continuity has not been validated.

Executable SHA256:
`624fcb7f30cc238d51b5caa324a1f6523667b3b6c53e288247b6dcc0f01aaaf5`.
Identity JSON SHA256:
`6a55f63fdf08b9f07618377ce26991942a540d24398e3724dec538725eb44994`.
The bundle pins the imported Bevy and Rust DLLs and the 51,328-file asset pack.
Asset closure was not changed; before/after manifest checks found no changes.

## Procedure and settings

RTX 4080, NVIDIA 616.92, Intel i7-12700KF (12 cores, 20 logical processors),
Windows 11 build 26340. Build: Cargo dev profile, `dev-dynamic`, project opt-level
1 and dependency opt-level 3. Output and native rendering: 1920x1080, TAA,
VSync off, existing raster lighting and shadows, GPU culling, resident textures.
Solari, DLSS Ray Reconstruction and DLSS scaling are not active in this baseline.
Optional Vulkan layers were disabled with `VK_LOADER_LAYERS_DISABLE=*`.

`benchmark_development.py --workload combat --repeats 5 --profile debug` reused
the same pinned executable. Five separate processes were used. Each loaded and
prepared the workload, admitted the complete six-zombie cohort, warmed combat
for ten seconds, then recorded thirty wall-clock seconds of completed frames.
Round 8, fixed seed, MG08 firing/ADS/reload/movement, health fixtures keep the
cohort constant. Reset and warmup occur before each sample. Loading is outside
the sample. This is representative combat, not whole-map streaming acceptance.
The existing readiness protocol was used; no per-frame pipeline-cache polling
or independent frame-time convergence criterion was added. Variation remains
visible below, so a steady-state convergence claim is not justified.

## Measurements

| Sample | Average FPS | Median ms | p95 ms | Maximum ms |
| --- | ---: | ---: | ---: | ---: |
| 1 | 25.865 | 35.905 | 56.226 | 110.321 |
| 2 | 24.992 | 36.063 | 58.001 | 93.708 |
| 3 | 21.960 | 44.491 | 63.644 | 109.419 |
| 4 | 25.274 | 35.897 | 56.851 | 90.424 |
| 5 | 25.176 | 35.608 | 58.580 | 82.486 |

Aggregate: 24.654 FPS; median sample FPS: 25.176. All slow samples and spikes
were retained. Every process exited normally. No overlapping compiler, game
or capture was detected. GPU samples late in the runs were generally 27–53%
utilization, roughly 82–120 W, and 39–47 C. This suggests CPU/submission or
synchronization pressure, but does not identify a specific system to optimize.
No optimization is justified by these counters alone.

Background CPU deltas cover each entire process lifetime, including load and
warmup, not just the measurement interval. The largest background process was
ChatGPT at approximately 86–89 CPU seconds per run; Discord and Codex also ran.
The observer uses periodic CIM/process queries and nvidia-smi. Its overhead and
other desktop work are included. No affinity, worker limit or broad profiler
was used. SDK download activity around the final sample was not instrumented
as a separate interference category; the sample is retained, not selectively
discarded. Raw per-frame intervals, GPU/load samples and identities are retained
in external bundle `vulkan-upstream-baseline1080-20261012`.

An earlier 2880x1620 run was rejected for incorrect Windows-scaled resolution.
The game now fixes benchmark and BRP validation windows to 1920x1080 with scale
factor 1. A separate startup probe reached Playing with Vulkan and no pending
pipelines, but its nonindexed count-clamp GPU test failed. That run also panicked
on swapchain-semaphore destruction after normal BRP shutdown. Subsequent five
baseline runs with optional layers disabled exited normally. The cause of the
probe failure and initial teardown panic has not been established.

## Migration capability findings

Pinned `bevy_solari-0.20.0/src/scene/blas.rs:248` requires an exact four-attribute
position/normal/UV0/tangent layout and u32 triangle indices. It excludes joint
weights, joint indices, secondary UVs and Origins material attributes. The BLAS
builder uses extracted mesh allocation data without the raster skinning/morph
deformation path; instance transforms alone do not establish animated shadows.
Geometry is marked `OPAQUE` at line 229.

`scene/extract.rs:28` extracts `MeshMaterial3d<StandardMaterial>`, whereas Origins
uses extended materials and custom surface shaders. The ray material binding
and resolver do not implement Origins' multilayer control, authored specular
response, animated surface effects or transparent ray behavior. The pinned
official `examples/3d/solari.rs` removes secondary UVs and turns glass opaque;
doing that here would violate material/content preservation requirements.

These are concrete missing integration paths, not proof that the RTX 4080
lacks hardware support. DLSS SDK v310.9.1, Vulkan headers v1.4.328 and libclang
18.1.1 were obtained externally, but no Solari/DLSS build or runtime proof was
produced. Required work remains: deformation-aware acceleration structures,
material-preserving primary and ray shading, transparent/effect composition,
world/first-person temporal guides, capability errors, project identity and
NGX runtime packaging. Existing baked/local lighting has not been removed.
The 60 FPS at DLSS Quality target and the complete gameplay audit remain unmet.

## Separate gameplay evidence

The single-executable bundle's bounded BRP integration run recorded 224
assertions: 223 passed and one landing assertion failed. UI/session, projectiles,
physics, mobility/diving, solo pistols, Nuke/Insta-Kill, environment/progression
and quest sections passed. The landing failure retained its full trace: a 4.5 m
drop reached the floor, but Avian's projected vertical velocity was +0.000121
m/s, exceeding the 0.0001 grounding tolerance. The next airborne tick overwrote
the precontact impact energy, producing zero impact height and omitting the
native no-damage-surface landing slowdown. The game shut down through BRP and
exited normally. This failed run is not accepted as a complete gameplay pass.

The follow-up adjusts the contact tolerance to 0.001 m/s and uses `try_remove`
for shadow-view retirement when its owner has already despawned. Matching build
and focused movement recheck evidence are recorded separately from the baseline.

A separate static audit reads the canonical C: research's hashed
`evidence/original-game/weapon-profiles.json` and its original recovered profile
files. All 129 prepared weapon locomotion timing sets and 5,081 animation/rig
bindings matched. This is source/asset evidence, not runtime animation or visual
proof. No research evidence was moved or modified and the original game was
never launched. Older station/animation scenarios require inspection exports
(`solo-content/doors-recovery/entities.json` and `recovered-gameplay.json`) that
are not present at those paths in the canonical master. This is a scenario input
gap; it does not establish that equivalent evidence is absent elsewhere in the
master. The static weapon audit used its canonical profiles directly instead.

No claim is made that every one of the 557 checked gameplay checklist items was
audited, that the full main quest was played through, or that audible quality,
whole-map visual fidelity, moving raytraced shadows or temporal RR behavior were
accepted. Rendering work remains outside `todo.md`.
