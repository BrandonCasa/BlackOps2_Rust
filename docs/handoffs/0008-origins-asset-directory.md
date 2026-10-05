TASK:
Establish actual decoded asset-directory availability for the single licensed Origins candidate zone/all/zm_tomb.ff using verified external OAT listing only.

STATUS:
COMPLETE

FINDINGS:
- Two identical successful listings: 7,549 entries across 31 types, 7,549 unique raw type/name pairs; 332 reference-marker entries under OAT's convention.
- One each gfxworld, clipmap, comworld, mapents and gameworldmp shares maps/mp/zm_tomb.d3dbsp. Geometry families include xmodel 1,015, material 1,686 and image 1,985; weapon 129.
- Listing exposes directory entries only, without field values, dependency edges or reference resolution. It establishes no geometry/gameplay/visual fidelity.
- Stock T6 LIST cannot measure world-draw/surface fields; no GfxWorld dumper is registered in the pinned release. A bounded external metadata adapter is the next geometry access route.

EVIDENCE:
- Root AGENTS; asset-inspection/bo2-reverse-engineering skills; architecture/state; findings 0001/0002/0003; handoff 0007; existing assets/inspector source and synthetic tests; ignored local configuration.
- Official OAT v0.33.0 archive, release API size/digest and immutable release source at 7d027e8f89118196713e955b0e11f8404149c54d; precise links/identities in findings 0004.
- Only zone/all/zm_tomb.ff, Steam Zombies manifest build 24784239, English; 78,753,344 bytes, unchanged accepted SHA-256/length/write time/file identity.
- External helper, verified package, raw process logs, complete pair lists, metadata and parser synthetic fixtures remain in configured inspection_output. No asset body, script text, crypto constant or actual root enters tracked files.

CHANGES:
- Added findings 0004 and handoff 0008; minimal documentation indices/project-state updates.
- No tracked Rust, manifests, runtime dependency, loader/entity/gameplay/rendering API or architecture decision added. No commit or push.

VALIDATION:
- Archive SHA-256/length verified before unpack/execute; member path/containment/symlink/expanded-size checks passed. Unlinker binary SHA-256 recorded and unchanged; version/help exit 0. No extra runtime/global installation required.
- LIST --no-color on one explicit input only: exit 0, 0 warnings/errors, empty stderr; 0.765000/0.781000 seconds, 300,766 combined bytes each under 120-second/8-MiB capture limits. Observed peak working set 391,651,328/391,491,584 bytes; no hard memory cap claimed.
- Exact ordered pairs and summary/digest agree; known-type/framing parser validation and four external synthetic cases passed. Counts sum to 7,549; markers to 332.
- The same supervisor killed synthetic combined output beyond 8 MiB and a sleeping child at a 0.1-second test deadline, recording incomplete/timeout and unsuccessful exits. Originals were not opened by these self-checks.
- Isolated process directories contain only the supervisor's stdout/stderr logs, with no dump files/directories. Original source identity unchanged before/after.
- Release source confirms LIST skips raw container loading and dumping. Signed LE loader always schedules signature verification; rejection propagates as load failure. Success indicates OAT signature acceptance on that source path, not independent authenticity/integrity validation.
- Documentation link/path/content review and git diff --check passed. Cargo gates intentionally not repeated for documentation-only work. No original-game comparison or decoded geometry-property measurement performed.
- Outer-agent review accepted the scope/findings and checked the external helpers. Independent archive/digest/all-162-package-file verification, immutable source comparisons, two additional LIST runs, exact ordered-entry comparisons, source/binary preservation and both synthetic supervisor checks passed. Parent runs took 0.687/0.735 seconds and captured 300,766 bytes each; review records remain external. Documentation links/content boundaries and whitespace checks passed.

FIDELITY STATUS:
INVESTIGATING
Decoded asset availability only. Gameplay/visual/overall fidelity UNKNOWN.

UNRESOLVED:
- Surface/index/vertex ranges, material references, dependency resolution/owning zones, patch/shared/localized loading relationships, geometry/texture completeness and map appearance.
- No hard decoder memory cap, independent authenticity check, negative-signature case, Steam integrity validation or live game comparison.

RECOMMENDED NEXT WORK:
- Investigate and observe the one gfxworld's surface triangle-range to world-draw buffer/material-reference boundary using a separately bounded external pinned-decoder metadata adapter. Emit aggregate counts/range-check/reference-resolution results only; corroborate semantics and synthetic validation before Bevy/Rust geometry integration.

SCOPE NOT IMPLEMENTED:
- Asset extraction/conversion, dumps, other input zones/formats, game launch/original writes, Rust decoder/crypto/framing code, geometry/rendering/entity/ECS/gameplay APIs, global environment changes, commits or pushes.
