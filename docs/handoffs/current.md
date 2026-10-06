TASK:
Distill the supplied rendering transcript with three subagents, then have three new subagents read all findings and propose changes to external PLAN (3).md. Edit the plan for deferred rendering and all applicable quality-preserving performance improvements, retaining near-zero perceptible loss and excluding blur/smear/ghosting. The explicit user request overrides delegation restrictions for this planning review only.

STATUS:
COMPLETE - both three-agent rounds finished and incorporated into the edited external plan. Renderer implementation, image acceptance and savings remain unverified.

FINDINGS:
- Preserve complete materials/BRDF, radiometric ownership, raw directional GI, lossless PNG v2, all authored shadows, six current-frame active cube faces, water/glass, atmosphere, content and gameplay.
- Stock Bevy deferred packing loses required material state; fixed attachments and custom light routing need explicit RenderApp/Core3d integration. Stock depth has no stencil and deferred views force MSAA off; truthful current-frame spatial quality needs a complete custom route or equivalent fixed supersampling.
- Prioritize exact reuse/specialization, conservative multiview relevance, selective prepasses, safe clear/copy/target removal and equivalent composition. Benchmark stencil versus clustered deferred dispatch with complete equations.
- Precision/resolution/sampling reductions, preintegration, warps, proxies and SSR remain conditional. Two-frame AA, stale shadow/cube updates and concealment blur remain excluded. Retain original convergence, add independent critical-region/motion checks and matched whole-frame GPU/CPU/memory evidence.
- Correct capture/filter/consumer ordering, including transmission reads of primary opaque HDR. Reuse only under complete input identity. Deferred costs and unsupported transcript guarantees are explicit.
- The separate rendering milestone landed at f609c4e during review; its runtime evidence remains in Git history. Its acknowledged 512-versus-1024 shadow detail tradeoff must be requalified against the complete intended image, not inherited as free quality preservation.

EVIDENCE:
Read project state, architecture, dirty material/camera source and pinned Bevy 0.19.1/wgpu-types 29.0.4 integration. Round 1: distill_geometry_deferred, distill_lighting_transport, distill_clarity_effects. Round 2: propose_deferred_architecture, propose_lighting_performance, propose_clarity_validation; each read all first-round reports. External reports/index/original backup: C:/Users/brand/Downloads/PLAN3-rendering-review-2026-10-06/. Primary sources are linked there and in the plan. Final plan SHA256: EDA354E93189D7C2BBA84D27476F36B0DFBB359952BB3F9FC8FF64BF69A8AD1E.

CHANGES:
Edit C:/Users/brand/Downloads/PLAN (3).md in place; preserve its byte-identical original backup. Add external findings, proposals, quality checklist and index. Replace this current handoff only in the repository. This chat makes no application/config/asset edits and preserves the other chat's f609c4e milestone.

VALIDATION:
Six reports completed; normalized 939-line transcript preserves all supplied content. Revised plan diff/reread, retained-requirement and stale-conflict checks passed; 15 local evidence paths resolve and Markdown fences balance. Tracked Markdown remains 20. Investigative wgpu-27 path miss and patch-context mismatches were corrected; one handoff patch used a record superseded by the concurrent committed milestone, which was reread before replacement. No application builds/compiler gates, asset integrity, WGSL/game-window runs, gameplay, unit tests/fixtures, Clippy or packaging performed by this planning review. No push.

UNRESOLVED:
Complete custom deferred material/sample layout and adapter support; offline material/bake equivalence; lossless decode and full GI convergence; multiview shadow/capture/transmission correctness; actual native-detail/motion quality and measured net savings. Static GI's changing-bounce approximation and unfinished actor/weapon/HUD/atmosphere content remain explicit. The preceding implementation milestone's sustained/manual-play limits remain recorded at f609c4e.

RECOMMENDED NEXT WORK:
Implement the external plan sequentially in origins.exe: establish contracts and baseline, complete deferred material/lighting/coverage, finish original rendering/content milestones, remove exact redundant work and qualify conditional accelerators against the complete intended renderer. Retain original compiler/asset/game-window gates and image-first acceptance.

SCOPE NOT IMPLEMENTED:
Renderer/gameplay/content code, prepared runtime assets or bakes, executed optimizations, measured savings, rendered/played acceptance, tests/Clippy, packaging or push. This planning revision is not implementation evidence.
