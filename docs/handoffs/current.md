TASK:
Layered terrain, source-informed special surfaces, animated water/wetness and a coherent Origins atmosphere within the existing Bevy game. Compare the actual map with the supplied reference screenshots.

STATUS:
COMPLETE - supported materials and atmosphere milestone delivered as a practical reference-informed reconstruction; whole-game visual parity remains outside this milestone.

FINDINGS:
- Retained secondary corners supply half UVs and signed normal-basis rotations; vertex G/B plus texture alpha controls authored layers. Binding hashes identify diffuse/normal/specular layers, including sparse channels. Existing tile/macro diffuse bakes must be sampled once.
- Source world corners were clockwise relative to normals. Reversing complete triangles restores correct source culling without blanket double-sided rendering. Explicit retained blend/cull states distinguish cutouts, transparency, add/multiply, glow and invisible helpers.
- The giant smoke effect proxy previously covered sky with solid rectangular planes. Its unreconstructed native effect is hidden; one Bevy distance fog supplies the practical volume approximation.
- The supplied original/current pair revealed overly bright SH-lit props and lost within-model lighting gradients. The existing runtime pack already contains original LOD vertex identities and instance lighting colors. They now interpolate through material lighting textures while geometry and placement references remain shared.

EVIDENCE:
- Original findings, texture-preview/inventory, secondary corner streams, model-surfaces/static-instances and installed Bevy 0.19.1 sources informed the implementation. Original shaders are never loaded.
- Supplied C:/Users/brand/Pictures/refs and original-versus-current clipboard images remain external comparison references. Evidence is under the configured external inspection directory: visual-profile/references.jpg, material-summary.json, import-final.json, index-final.json and actual-window run-1 through run-11 captures/logs/receipts. Final room/outdoor/water views were inspected, and comparison-final.jpg retains the original/final room pair.

CHANGES:
- Extend preparation/export/import metadata and glTF registration for authored terrain UVs, rotations, four color/three normal/specular layers, reveal controls and base/blend/multiply/add combinations. Preserve tile/macro conversions and all 1,249 PNG bytes; regenerate only map.gltf/map.bin/game.json. Preserve 14,480 placements and shared providers/relative references.
- Extend fresh project-owned Bevy ExtendedMaterial WGSL for layered lighting, source normal/gloss response, transparent alpha, moving two-normal water/reflections and modest up-facing wetness. Source presentation metadata supplies alpha, cull, emissive/fog, visibility and solid/wettable/lamp dispositions.
- Restore per-instance vertex-light gradients without instance geometry copies; independently calibrate SH, vertex, atlas and reflection radiance. Remove the SH minimum fill. Reduce pale glass coverage, soften reveal masks, reconstruct linear-light mips and use anisotropic Bevy sampling.
- Embed config/origins-visual.json for cool actor illumination, warm local lamps, separate lamp/equipment emission, exposure, restrained saturation, AgX/bloom, one fog and procedural storm sky. Sky exposure scales once; baked diffuse excludes duplicate ambient/sun.
- Extend the existing real-window driver with room directions, moving map views, repeated stationary water and camera position/yaw/pitch/eye-height/FOV/aspect receipts. Hide player overlays only during opt-in comparison views; the ordinary player camera remains unchanged.

VALIDATION:
- Locked metadata, release build and formatting passed with the installed Cargo. Final actual-window run-11 passed all 15 views, movement, six kills/Round 2 and restart with no shader/render errors. No tests/suites/fixtures or Clippy were added or run.
- Full import/source reconciliation and staged closure checks passed: 1,252 files, 1,249 PNGs, 717,944,597 bytes; 1,251 LFS pointers and one ordinary Git game.json. Git LFS fsck passed. Only affected runtime files changed. Tracked Markdown remains 20.
- Corrected actual failures: adapter layout mismatch in Bevy prepass; clockwise world winding; accidental double sky exposure; straight DDS color reaching a premultiplied blend; emissive radiance bypassing exposure through Bevy's default weight; a Rust borrow in camera receipts; a release executable locked by an active game process. Compiler/render failures were repaired and affected checks repeated.
- Runs 7-11 passed movement, six kills/Round 2, restart and 15 views with pending=0 and no shader/render errors. Runtime coverage: 2,179 world groups, 15,535 static primitives, 989 layered, 232 tile/macro, 3,005 masked, 570 transparent, 47 water and 13,314 wettable primitives across 3,817 shared material variants. These are runtime primitive/group counts, not source surface/placement counts.
- Final comparison: props are darker and less uniformly pale, concrete shows retained lighting gradients, gauges are subdued, warm lamp pools remain distinct from cool exposed stone, and terrain/water/fences remain readable in outdoor views. At a fixed camera the water-region mean RGB byte difference is 7.104 versus 0.00121 on solid ground across 8.20 seconds (run-11/water-comparison.json). This establishes animation, not native image parity.
- Local commits: eb3a9d8 prepares layers/dispositions and validated runtime assets; the rendering/profile/documentation commit completes integration. No push.
- Skipped: banned tests/Clippy, packaging, sustained manual play, exact native/pixel parity and push. The existing driver exercised the real application/window; it is not manual play.

UNRESOLVED:
Reveal width, gloss/F0, radiance gains, vertex probe fill, transparent coverage and canopy cutoff are practical controls. SH interpretation and native light transfer remain approximate. Water uses tint transparency for simplified shallow refraction and static assigned surroundings for reflection; actors do not enter those cubes. Mips are reconstructed from retained top levels. Procedural sky and distance fog do not reproduce original animated clouds or localized drifting smoke/light pockets. Blood transitions and individual special lobe responses may still benefit from further reference tuning.

RECOMMENDED NEXT WORK:
Replace the temporary weapon/arms, actor visuals and diagnostic HUD through the existing licensed preparation path in a separate coherent gameplay-visual milestone. Continue localized smoke and individual surface refinements against controlled actual-game viewpoints as useful; exact pixel/native arithmetic must not block playable work.

SCOPE NOT IMPLEMENTED:
Original shaders or renderer, another rendering pipeline/viewer, capture-fed scene imagery, dynamic planar/screen-space reflections, depth-distorting refraction, original volumetric effects, original first-person arms/animations/HUD, unrelated gameplay systems, tests, Clippy, packaging or push.
