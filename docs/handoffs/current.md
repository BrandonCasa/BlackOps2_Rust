TASK:
Repository cleanup and one playable Origins executable.

STATUS:
COMPLETE — repository reduction and playable application foundation. Remaining original systems and visual refinements follow the roadmap.

FINDINGS:
- Capture-defined world membership omitted geometry and static models.
- Source entities provide actual spawn-room and enemy/path anchors.
- Complete source accounting includes 6,469 world surfaces, 149,907 triangles and 14,480 static placements from 494 model providers.

EVIDENCE:
- Source world exchange, packed attributes, native static-placement metadata, OAT models/images and map entities inspected externally.
- Release runtime receipts and screenshots remain external; no licensed content is tracked.

CHANGES:
- One Rust application using standard Bevy rendering and Avian movement.
- Practical combat, population, rounds, HUD, pause and restart.
- External glTF/material preparation and source static placement export.
- Eleven Cargo packages reduced to one; Markdown 282 → 20; tracked Rust 36,490 → 3,547 lines (90.3% reduction, exceeding the planned reduction).
- Retired viewers, inspectors, capture/shader replay, native movement reconstruction, obsolete tests and historical documents removed. Ten concise skills retained; subagents banned.

VALIDATION:
- Release build; locked metadata; workspace release tests (13 passed); formatting; Clippy with warnings denied; Git whitespace check.
- Synthetic preparation tests: 3 passed. Real external preparation completed with 204 world and 350 model textures.
- Actual release application, automated input: movement, six Round 1 kills, Round 2, eight map locations and restart to Round 1 passed. Inspected spawn-room, tunnel, outdoor and chamber screenshots.
- Earlier survey verification stalled after gameplay continued during camera relocation; the survey now pauses simulation and exercises the normal restart message. Two subsequent runtime runs passed.
- This is automated gameplay/render validation, not manual play or an original-game fidelity comparison.

FIDELITY STATUS:
APPROXIMATE.

UNRESOLVED:
- Temporary pistol/zombie visuals, practical movement/combat and local waypoint navigation; original animations and full navigation are pending.
- Distant views expose rough special materials, vegetation transparency, debug-colored surfaces and helper-like geometry. Geometry accounting passes; these visual defects remain and appearance is not claimed complete.
- Preparation relies on the retained external world exchange and OAT installation/build; fresh world extraction from fastfiles is not yet part of the single preparation command.

RECOMMENDED NEXT WORK:
- Refine material/geometry dispositions and original character presentation in the same application, then follow docs/roadmap.md.

SCOPE NOT IMPLEMENTED:
- Remaining original systems listed in docs/roadmap.md.
