# Lantern and shadow performance findings

Investigation date: 2026-10-08. Source baseline: local main at `8242a7883`, with
concurrent working-tree changes present during inspection. This is the findings
inventory before an implementation plan. No renderer changes, builds or new
game runs were performed for this investigation.

The strongest finding is unnecessary shadow generation, with several mechanisms
behind it: emitter-classification mistakes, repeated preparation despite caching,
overly broad invalidation, and additional shader/ray work.

**Confirmed** means behavior is present in source or assets, not that its
individual FPS cost has been measured. **Measured** refers to existing saved
receipts. **Lead** identifies an opportunity or correctness concern that still
needs focused attribution or visual acceptance.

## Emitter inventory and classification

All 14,480 original static instances were matched to 15,936 prepared static
surfaces, with zero placement-translation mismatches. The source placement and
surface inventories were read externally; originals were not modified.

| Original model | Instances | Point lights created |
|---|---:|---:|
| `p6_zm_tm_gas_lamp_straight_on` | 1 | 1 |
| `p6_zm_tm_hanging_light_fixture_on` | 2 | 2 |
| `p6_zm_tm_lantern_01_on` | 27 | 27 |
| `p6_zm_tm_portable_lantern_on` | 49 | **98** |
| `p6_zm_tm_radio_01` | 14 | **14** |
| `p6_zm_tm_radio_02` | 7 | **7** |
| `p6_zm_tm_robot_lamp` | 38 | 38 |
| `p6_zm_tm_speaker_tube` | 144 | **144** |
| **Total** | | **331** |

1. **Confirmed: material reuse determines whether an object becomes a lantern
   light.** The importer marks four material names as `lamp`, regardless of the
   model using them. Radios and speaker tubes reuse gas-lamp glass, so their
   glowing surfaces become full room-lighting emitters. The code comment says
   these providers are distinct from radios; the implementation does not achieve
   that. Source: `tools/prepare_origins.py:72`.

2. **Confirmed: all 49 portable lanterns emit twice.** Both the body and glass
   material are marked as lamps. Each surface independently spawns a light.
   Their centers differ by approximately 1 cm, so they illuminate almost the
   same volume and generate almost identical shadow work.

3. **Confirmed: speaker tubes create dense groups of lights.** The 144 tubes
   form twelve close groups of twelve. These are roughly 5.6 cm wide and 10 cm
   tall, yet each receives the full lantern configuration. At the densest emitter
   position, 26 current lights lie within 7 meters.

4. **Confirmed: every emitter receives identical parameters.** Tiny tubes, radio
   indicators, flat robot lamps, lanterns and larger hanging fixtures all get
   2,600 lumens, 7-meter range, 12-cm source radius, six 512-square shadow faces
   and soft shadows. There is no fixture-specific configuration.

5. **Confirmed: classification also changes emissive brightness.** Lamp-marked
   materials receive 4,000 nits, versus 150 nits for other emissive materials.
   The radio/speaker classification affects visible glow and transport as well
   as point lights. Correcting analytic emitters and correcting emissive
   appearance are related but separate decisions.

6. **Confirmed: creation is attached to mesh surfaces, rather than physical
   fixtures.** An asset split into more matching primitives can silently produce
   more lights. `SurfaceReady` prevents repeated spawning on one entity; it does
   not prevent multiple emitters belonging to one fixture. Source:
   `src/rendering.rs:241`.

7. **Confirmed: the runtime pack loses source model identity needed for reliable
   classification.** Static nodes retain a Boolean `static_instance` marker.
   The material library is keyed by material identity. Changing one shared
   material's `lamp` flag cannot distinguish a gas lamp from a radio using that
   material. Import validation strictly limits node metadata, so any retained
   fixture identity must also be supported through validation. Source:
   `tools/import_origins.py:361`.

8. **Confirmed: lamp-marked surfaces are excluded from raster shadow casting.**
   This includes the portable lantern's body surface, not merely transparent
   glass. Separating emitter ownership from surface appearance needs a visual
   correctness check; removing duplicate lights alone does not resolve fixture
   self-occlusion.

Excluding radio/speaker analytic emitters and retaining one light per portable
lantern gives **117 remaining candidates instead of 331**: 214 fewer lights,
or 64.7%. This is an inventory reduction, not a predicted FPS improvement or
proof of original-game lighting equivalence. Whether every remaining candidate
should use the same shadow policy remains to be assessed.

## Distance, attenuation and admission

9. **Confirmed: illumination range is finite.** The range is 7 meters. Raster
   lighting uses inverse-square attenuation with a smooth falloff to zero at
   that boundary. This matches normal physically based punctual-light practice.
   Source: `vendor/bevy_pbr/src/render/pbr_lighting.wesl:128`.

10. **Confirmed: radius and range are separate.** The 12-cm radius describes the
    emitting source and affects highlights and soft-shadow penumbra. It is not
    the influence distance. Lumens are converted to luminous intensity using
    `4*pi` in both native raster extraction and the custom ray inventory; no
    missing conversion was found.

11. **Confirmed: there is no separate camera-distance or contribution-based
    shadow budget.** Native light-sphere visibility controls admission. Once a
    light is admitted and dirty, its faces can render even when eventual visible
    contribution is tiny. The shared `RenderBudget` contains reflection limits,
    not shadow limits.

12. **Confirmed: frustum admission does not establish room visibility.** A light
    behind a wall can overlap the camera frustum and remain admitted. Illumination
    can be blocked correctly while shadow preparation still costs time. This is
    an opportunity, not evidence that native frustum culling is broken.

13. **Confirmed correction: the current lantern `NoFrustumCulling` override is
    gone.** Forced visibility describes the earlier integration/capture, not
    current source. GPU clustering has also been restored with bounded initial
    capacities.

References: [Bevy PointLight](https://docs.rs/bevy/latest/bevy/light/struct.PointLight.html)
and [Filament's lighting model](https://google.github.io/filament/Filament.html).
Pinned local source is authoritative for this application's exact behavior.

## Shadow generation, preparation and caching

14. **Measured: the latest saved survey processed 176,221 fresh cube faces.** It
    reused 539,561 faces, avoiding approximately 75.4% of counted executions.
    Caching works, but the remaining quarter is large. The aggregate is about
    108 fresh faces per first-person depth check across the full survey, not a
    steady-room measurement. Evidence: migration `game-24/performance.json`.

15. **Confirmed: point/spot shadow phases are forced into direct drawing.** They
    use `NoIndirectDrawing` and preprocessing without GPU culling. Native CPU
    caster culling still runs; this does not mean every scene mesh is drawn.
    Native indirect/multi-draw benefits are unavailable for these phases.
    Source: `src/rendering/shadow_cache.rs:1273`.

16. **Confirmed: geometry and skinning repeat across dirty faces.** Each fresh
    face performs mesh preparation and shadow drawing. Skinned shadow vertices
    execute native skinning again for that viewpoint. Cached ray geometry does
    not automatically provide shared raster deformation for these passes.

17. **Confirmed: unchanged extracted lights are inserted as changed data.**
    Extraction unconditionally reinserts `ExtractedPointLight`, frusta and layers.
    Preparation watches their change ticks and updates six shadow-view
    descriptions. Numeric cache stamps can subsequently skip drawing, but that
    happens after this preparation. Source:
    `vendor/bevy_pbr/src/render/light.rs:609`.

18. **Confirmed: repeated preparation creates attachment views and rebuilds
    view descriptions.** The per-preparation attachment lookup is fresh, and
    updating point shadow maps creates texture views and inserts view components.
    The reported three texture allocation changes measure backing texture
    identity changes; they do not establish retained texture-view/component
    preparation.

19. **Confirmed: culling can remap packed shadow slots.** Lights are sorted and
    assigned compact indices every preparation. Removing an earlier light can
    shift later lights into different cube slots. The cache correctly detects
    ownership changes, but those remaps can redraw unchanged lights. Stable
    slots are a separate concern from stable backing allocation.

20. **Confirmed: cache completion depends on every pipeline being ready.**
    `pipelines_ready()` checks the entire pipeline cache. An unrelated queued,
    compiling or failed pipeline can prevent newly drawn maps from becoming
    reusable. Existing completed maps can remain reusable, but affected dirty
    maps can repeatedly draw. The failure mechanism is present; its current
    timing impact has not been isolated. Source:
    `src/rendering/shadow_cache.rs:1267`.

21. **Confirmed: material invalidation is broader than shadow coverage.** Any
    modified standard or extended material invalidates its caster's nearby
    shadows. Roughness, response or other changes leaving depth coverage
    unchanged can redraw maps. Image-only invalidation is more selective, but
    material events remain broad. The survey reports 10,651 material refreshes,
    making this worth attributing. Source:
    `src/rendering/shadow_cache.rs:887`.

22. **Measured: phase/specialization churn remains.** The latest receipt records
    11,790 direct phase resets, 1,292,524 invalidated specialization entries and
    18 specialization epoch resets. These are work counts, not elapsed times;
    retained shadow pixels do not eliminate all preparation churn.

23. **Confirmed: some changes invalidate all lights conservatively.** Modified
    raster geometry, shaders, inverse binds and certain culling changes advance
    a global epoch. Some require broad invalidation for correctness; others may
    permit narrower ownership-based invalidation. Preserve growing bounds,
    changed silhouettes and off-screen blockers when narrowing this behavior.

24. **Confirmed: animation invalidation remains broad within an intersecting
    light.** Dirty caster spheres are tested against lights and cube-face
    frusta. Large or elongated bounds can dirty several faces. The dirty list
    can contain old bounds, new bounds and a skin-change entry for one caster.
    This is conservative but can repeat intersection work. Source:
    `src/rendering/shadow_cache.rs:632`.

25. **Confirmed: four sun cascades are generated for each of two cameras.** The
    latest survey records exactly eight directional cascade executions per
    first-person depth check. The overlay adds directional shadow work, although
    point-light maps are already shared across cameras.

26. **Confirmed: first-person directional shadows can include world casters.**
    Native directional visibility uses the light's caster layers; sunlight
    includes layers 0 and 1. The overlay camera's layer-1 receiver restriction
    does not itself restrict its shadow caster query to layer 1. Shared coverage
    deserves investigation with self-shadowing and wall occlusion preserved.

27. **Confirmed: shadow rendering disables backface culling broadly for extended
    materials.** This preserves thin wall/ceiling blocking but can increase
    geometry/raster work. A narrower policy might help thick closed geometry;
    globally restoring backface culling risks reintroducing leaks.

28. **Lead: range limits objects more reliably than submitted triangles.** Point
    shadows use an infinite reverse-Z projection. A large mesh intersecting the
    7-meter sphere can submit triangles extending beyond useful illumination.
    No triangle-level range clipping was demonstrated in this path. Importance
    depends on actual mesh sizes and shadow geometry counters.

29. **Confirmed: the single-threaded executor override affects all `Core3d`
    execution.** The smaller direct-shadow schedule is also single-threaded,
    but the earlier `Core3d` override remains for world/overlay camera schedules.
    This may suit short render systems; its broader effect has not been isolated.

## CPU work around shadows

30. **Confirmed: caster-change tracking scans the population each frame.** It
    compares transforms/bounds and checks skin poses per primitive. Nearby-light
    invalidation searches dirty bounds for every point light. Reducing bogus
    emitters reduces this CPU work as well as GPU passes.

31. **Confirmed: spatial visibility does not cover every surrounding scan.**
    Native membership maintenance still visits retained casters; dirty-light
    queries then use the grid. The latest run reduced candidate visits by
    95.388%, with zero exhaustive fallbacks, but did not establish a frame
    speedup. Broadphase optimization alone has already proved insufficient.

32. **Confirmed: shadow texture demand runs every frame.** Camera demand is
    throttled to 100 ms, but `shadow_demand` has no equivalent timer. It walks
    coverage-bearing regions and loops over lights, cascades and spots. The
    latest final snapshot reports 21,969 shadow texture requests. Bogus lights
    can also keep nearby coverage textures at unnecessarily detailed mips.
    Source: `src/rendering/streaming.rs:1352`.

33. **Confirmed: the shadow grid is coarse relative to lamps.** Cells are 16
    meters while lamp range is 7 meters. Latest query totals average roughly
    842 candidate casters per light query before final tests. Smaller cells
    might reduce this but increase membership/storage work. The current grid
    has already removed most exhaustive visits.

34. **Confirmed: completed-light records have no visible retirement path.** The
    completion map inserts entity records but does not prune departed lights.
    Repeated scene restarts can accumulate retired identities. This is a lifetime
    issue, not an established explanation for current FPS.

## Pixel shading and ray transport

35. **Confirmed: raster shadow sampling precedes attenuation evaluation.**
    Conservative cluster membership can trigger sampling before the BRDF
    calculates zero contribution outside range. An exact range rejection could
    avoid work, but transmission lobes need their own receiver-position handling.
    Source: `vendor/bevy_pbr/src/render/pbr_functions.wesl:477`.

36. **Confirmed: every lamp enables PCSS.** The cube path performs eight blocker
    search samples followed by eight filtered samples, repeated for overlapping
    lights. This is a receiving-pass cost, but saved timing evidence places
    shadow-map drawing ahead of the entire main opaque pass. Source:
    `vendor/bevy_pbr/src/render/shadow_sampling.wesl:539`.

37. **Lead: the no-blocker PCSS case still filters.** Blocker search returns zero
    when none are found; the caller still executes filtering. A fully lit early
    return deserves image comparison, especially at thin blockers and edges.

38. **Confirmed: point-light contact shadows are disabled.** They inherit the
    default `false`. There is no separate screen-space contact-shadow march for
    every lantern. Sun contact shadows are enabled.

39. **Confirmed: bogus and duplicate lights enter GI/reflection transport.** The
    ray inventory includes all point lights, deliberately retaining off-screen
    transport. Each qualifying lamp can add a visibility ray at a surface hit.
    Correcting emitter identity removes redundant transport work too.

40. **Confirmed: outside-grid lookups unnecessarily scan all lamps.** A valid
    lamp grid encloses every finite lamp influence volume. A position outside it
    nevertheless receives the complete list. Such positions should have no
    local lamps; full-list fallback is necessary when a usable grid was not
    built, not for positions outside a valid grid. Source:
    `src/rendering/static_gi.wesl:28`.

41. **Confirmed: ray visibility uses closest-hit traversal.** Visibility calls
    the same closest-surface routine used by shading, rather than a dedicated
    accepted-occluder query. Native Solari has first-hit visibility paths. A faster
    path must preserve alpha-cutout rejection: blindly changing the ray flag is
    unsafe when a rejected masked hit lies beyond another blocker. Source:
    `src/rendering/ray_surface.wesl:268`.

42. **Confirmed: DDGI repeats local-light work across four explicit bounces.**
    Each admitted diffuse bounce evaluates direct irradiance and qualifying
    lamp visibility. Extra emitters multiply work across hits per probe ray.
    This applies to DDGI-enabled builds; default builds disable DDGI. Reflections
    still evaluate local lamps. Source: `src/rendering/static_gi.wesl:186`.

43. **Quality lead: analytic lights and emissive meshes are separate energy
    sources.** The shader receives mesh emission and adds analytic irradiance.
    No fixture-level mechanism was found matching their energy or identifying
    them as two representations of one source. This may overstate transport;
    radiance/visual acceptance is required before claiming a quantified error.

44. **Confirmed: raster and ray occlusion treat some lamp surfaces differently.**
    Lamp-marked surfaces are excluded from raster casting, while opaque/masked
    versions can enter ray geometry. Ray visibility stops short by the uniform
    12-cm source radius, larger than some actual tube geometry. Near-source
    occlusion consistency needs inspection.

45. **Confirmed: ray-light collection does not distinguish deliberate hiding
    from off-screen visibility.** Keeping off-screen lights is correct, but the
    query lacks inherited-visibility filtering. Using `Visibility::Hidden` to
    turn a future fixture off would not automatically remove ray illumination.
    Current static lights do not demonstrate that failure.

## Memory, measurements and interpretation

46. **Confirmed: shadow storage is substantial at 512-square resolution.** At
    32-bit depth, one six-face cube is approximately 6 MiB. Storage for 331 cubes
    is 1,986 MiB before capacity rounding. Actual retained capacity depends on
    device limits and peak admitted count; current allocation was not measured
    directly. Eight 4096-square directional layers would add 512 MiB. Source:
    `vendor/bevy_pbr/src/render/light.rs:1527`.

47. **Confirmed: high-water retention fixes churn but retains peak memory.**
    Arrays grow and remain allocated when fewer lights become relevant. This
    repairs the earlier transient-allocation failure, but reducing work after
    visiting a dense view does not necessarily release that storage.

48. **Confirmed: the streaming ledger is not total renderer VRAM accounting.**
    It accounts for managed textures, replacements, retirement and bootstrap
    reservation, not shadow arrays, acceleration structures and all targets.
    Its 75%-of-VRAM limit is not a complete renderer memory cap. The latest
    texture receipt shows no budget drains, so no evidence establishes current
    texture-budget pressure as the FPS-collapse cause.

49. **Measured: the older room capture ranks shadow drawing first.** It records
    26.03 ms GPU and 23.96 ms CPU versus 4.48 ms GPU for main opaque rendering.
    These values were verified. CPU/GPU overlap; they must not be added. The
    shadow aggregate includes point and directional maps. Saved invocation
    context does not identify which light type produced each span.

50. **Confirmed reporting problem: the shadowed-point-light counter measures
    inventory, not admission.** It counts all ECS point lights with shadows
    enabled, without visibility/extraction filtering. The older forced-visible
    implementation supports its 331-light admission claim, but this counter
    alone cannot establish current admitted count. Source:
    `src/verification/performance.rs:126`.

51. **Evidence gap: current timings do not isolate these mechanisms.** The latest
    game-24 survey has work counters but no render attribution. Earlier timing
    captures precede later culling/cache/preparation fixes. Neither establishes
    each finding's contribution to approximately 8-FPS development combat or
    diagnostic-free release performance.

52. **Confirmed: several suspected problems are already addressed.** Point
    shadow maps are shared across cameras; collision render meshes are removed
    after preparation; native actor bounds are retained; water animation updates
    a shared buffer rather than material assets; ray lamp buffers upload only
    when their generation changes. These are not missing optimizations.

## Evidence and implementation boundary

External artifact identifiers, relative to the existing capture archive:

- `origins-bevy-020rc2-migration-20261008/game-24/performance.json`: latest
  full-survey shadow work, cache, visibility and development-combat receipts.
- `origins-bevy-020rc2-migration-20261008/game-24/verification.json`: texture
  streaming snapshot, including material refreshes and shadow requests.
- `origins-renderer-plan3-attribution-20261008-155947/room-17/performance.json`:
  40 recorded frames, 23,421 shadow spans; verified exclusive GPU/CPU means.
- Original `static-instances.json` and `model-surfaces.json` preparation inputs,
  matched read-only against `assets/origins/map.gltf`. Repository and configured
  external runtime maps both contain 331 lamp-emitting surfaces; their full
  document hashes differ, so they are not asserted to be identical packs.

The highest-confidence waste is 165 radio/speaker emitters plus 49 duplicate
portable-lantern emitters. Remaining major pipeline leads are repeated view
preparation, direct drawing, broad material invalidation, slot remapping and
global pipeline readiness. Individual savings remain unmeasured. No fixes,
implementation plan, compiler gates, new captures or performance claims are
completed by this document.

## Implementation tracking

The lantern findings are the starting audit. Every supported fix must be checked
against other affected rendering systems and applied there when appropriate.
These entries track implementations separately from the source findings above.

- [x] Separate physical fixture emitters from material glow (findings 1-3, 6-7).
  Source-validated metadata retains model identity and one emitting glass surface;
  dedicated ECS creation waits for preparation. Full 1080p/DDGI game and focused
  inventory checks pass with 117 lights. Existing emissive brightness and caster
  rules remain; fixture-specific tuning and housing self-occlusion (4-5, 8, 43-44)
  require separate quality work because source geometry does not establish flux.
- [x] Return no lamps outside a valid influence grid; retain unusable-grid
  fallback (40). Compiler and actual DDGI/reflection shader execution pass.
- [ ] Retain unchanged extraction, attachment/view preparation and slot ownership
  (17-19), accounting for resource replacement and admission remaps.
- [x] Narrow material invalidation to shadow depth coverage (21, 30). The renderer
  retains native depth signatures for standard, layered and diagnostic
  materials; actors and first-person meshes use these same paths. Unknown/removal
  events, alpha-image generations, geometry growth, poses and shaders remain
  conservative. Extension-only layer images are not native shadow inputs. This
  does not suppress material updates for DDGI, reflections or color rendering.
  Locked development compilation/metadata/format and the full native 1080p/DDGI
  combat/ADS/17-view/Round 2/restart game pass. The run classifies 3159 initial or
  changed signatures and avoids depth invalidation for 32460 equivalent material
  events. This is workload evidence, not a measured FPS gain. Specialization
  churn/global invalidation/animation bounds (22-24) remain separate work.
- [ ] Replace global readiness with complete per-shadow readiness (20), including
  pending caster queues and GPU preparation.
- [x] Retire completed-light records independently of pipeline readiness (34).
  The system covers departed point/spot/directional owners. Compiler and full
  actual-game/restart gates pass; this run observes zero departed light records,
  so it does not dynamically qualify the removal branch or prove a speedup.
- [ ] Reduce direct submission, repeated deformation and first-person directional
  work (15-16, 25-29) after shared preparation is correct. The previous indirect
  candidate failed its game deadline and remains excluded.
- [ ] Reduce redundant receiving/visibility and streaming demand (32, 35-37, 41,
  45); qualify changes against thin blockers, transmission and hidden fixtures.
- [ ] Establish matched release gains and audit renderer memory accounting
  (46-51). The new development combat survey records 12.8623 FPS and 92283 fresh
  cube faces; these are unmatched against the earlier 8.4957 / 176221 survey.

Evidence: E:/Coding/Captures/origins-lantern-optimization-20261008, including source
metadata preparation, full import receipt, two builds, frozen executable hashes,
full game-1 and focused game-2. Game-2's inventory is 117 throughout; its timed view
uses render diagnostics and is not a diagnostic-free release measurement. No
overall goal completion, release speedup or minimum-FPS claim is established.

The shared shadow-coverage milestone is frozen in executable-3 and full game-3
in the same archive. Build-coverage-3.log passes after JSON macro-depth and UV
ownership corrections; metadata-coverage.json and format pass. Game-3 exits zero
in 158.058 seconds with 2482 DDGI batches, 1057 fresh reflection frames, 93874
fresh/278750 reused cube faces, zero streaming errors and zero first-person
world-depth aliases/HDR mismatches. Combat is 12.6880 FPS, median 56.0129 ms,
p95 181.1741, p99 215.4041 and worst 319.3733; all 707 samples exceed 10 ms.
This is unmatched development evidence; matched release adoption remains open.
