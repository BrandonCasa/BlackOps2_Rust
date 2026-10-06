TASK:
Review the external PLAN (3).md with at least three independent subagents and edit it for very high realism, modern rendering, clear detail and Forward+. Follow-up: align the plan with the new SMAA default.

STATUS:
COMPLETE - three source-grounded reviews incorporated into the external plan. Rendering implementation remains unverified.

FINDINGS:
- The existing material adapter changes baked-light guards, not the custom BRDF in point/spot/sun evaluations.
- Current-frame cube filtering requires explicit render-graph dependencies; the existing shadow cache assumes one camera.
- GI needs defined source/path ownership, Bevy/Cycles unit conversion, equivalent offline material closures, topology/visibility and a new HDR decode contract.
- Metallic response, dual GGX versus coatings, spatial/alpha filtering and full closure migration need explicit correctness criteria.
- Verified config selects Smaa; the camera uses spatial SMAA with MSAA off. Updated baseline and masked-surface coverage requirements accordingly.
- User requires every feature in the finished runtime, accepts even 1 FPS and excludes optimization. No reduced-quality shipping path or FPS acceptance target.

EVIDENCE:
Read the dirty worktree, project state, rendering/asset source and pinned Bevy 0.19.1 modules. Independent reviewers: forward_plus_review, gi_asset_review and realism_clarity_review. Primary Callisto, Filament, Cycles and irradiance-field references are linked in the external plan. Revised plan SHA256: 647B9674121A21294AA259CEFE15357116788579C3A8EC21013A590DDEE74818.

CHANGES:
Replace the external plan with separate prioritized critique and a correctness-first full-runtime implementation/acceptance plan. Replace this current handoff. Preserve all pre-existing dirty source/assets and the prior rubble implementation at 39c5021.

VALIDATION:
All three reviews completed read-only. Revised file reread; 15 local evidence links resolve; Markdown fences balanced; explicit Forward+, full-runtime quality and optimization exclusions present. Tracked Markdown remains 20. No builds, game runs, asset validation, tests, fixtures, Clippy or packaging performed for this planning-only request. No push.

UNRESOLVED:
Shader integration, offline material equivalence, lossless field decoding, GI convergence, multiview/current-frame reflection correctness and actual-window image quality require implementation and runtime evidence. Static GI does not establish dynamic bounce lighting. Temporary actor/weapon and missing atmosphere content still limit final realism.

RECOMMENDED NEXT WORK:
Implement the revised external plan sequentially in origins.exe, beginning with radiometric/path contracts and complete Forward+ material integration. Sampling and resolution follow image correctness; optimization is outside scope.

SCOPE NOT IMPLEMENTED:
New renderer features, new baked/runtime assets, gameplay/content changes, performance optimization, tests/Clippy, packaging or push. This review is not new rendered or played evidence.
