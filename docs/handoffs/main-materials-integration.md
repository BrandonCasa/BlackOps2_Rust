TASK:
Commit and integrate the current full-scene materials work into main at the user's request.

STATUS:
COMPLETE
Integration is complete; the broader full-original-scene rendering objective remains PARTIAL.

FINDINGS:
- All 6,469 surfaces, 449,721 corners and seven vertex formats are accounted for.
- All 262 shared images and 1,080 material texture bindings resolve externally.
- Native input routing admits 177,318 enabled surface/pass uses without failures.
- Shader pipeline census admits all 1,404 references and 1,065 distinct pairs.

EVIDENCE:
- Asset, secondary-input, semantic-routing, rendering and pipeline-census handoffs.
- Original captured dispatch covers 1,189 surfaces, 180 materials and 43/56 families.
- Captured opaque comparison covers 14,826 fully covered pixels, maximum error 1/255.

CHANGES:
- External validated dependency loaders, native Bevy material bridge and strict pass assembly.
- Semantic vertex adapters, diagnostic full-scene viewer and bounded captured GPU probes.
- Proprietary resources and captures remain external; no push authorized or performed.
- User's latest merge request supersedes the earlier completion-only integration condition.

VALIDATION:
- Workspace metadata PASS; workspace tests PASS; formatting PASS; strict all-target Clippy PASS.
- Default tests intentionally skip explicit external/GPU fixtures; separate actual results are
  recorded in their task handoffs and are distinct from the default workspace test run.
- Reviewed staged candidates and Git whitespace checks; no licensed asset payloads included.
- Main receives a fast-forward from the canonical-coordinate base, preserving its history.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Full original scene dispatch, dynamic uniform producers, final post-processing and traversal.
- Shadow fixture has unready Bevy pipelines; no successful shadow comparison is claimed.
- Thirteen families lack captured dispatch; placement for 100 possible submodel surfaces
  needs original object-transform evidence. Captured dispatch is not visual matching.

RECOMMENDED NEXT WORK:
- Resolve source object placement and runtime resources, then validate complete frame execution.

SCOPE NOT IMPLEMENTED:
- Full-original-scene completion, gameplay completion, proprietary redistribution or remote push.
