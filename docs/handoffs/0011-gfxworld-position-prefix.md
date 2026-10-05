TASK:
Validate the selected zm_tomb GfxWorld's candidate stream-0 float32 position prefix against finite-value and declared surface-bounds checks across all 6,469 surfaces, seven observed formats and every 6,236 zero-count case; investigation only.

STATUS:
COMPLETE
The bounded candidate-prefix consistency measurement is complete. The separate surface bounds support the candidate exactly; the embedded tris endpoint candidate is contradicted. Original bounds/draw semantics and full geometry layout remain unknown.

FINDINGS:
- All 320 complete 36-byte candidate group prefixes are admitted before position reads; all 191,259 candidate records contain finite IEEE-754 binary32 little-endian triples. All 191,259 addressed records are referenced; no unreferenced group records or nonfinite components.
- All 449,721 per-surface referenced position visits fit that surface's bounds[0]/bounds[1] ordered-endpoint candidate exactly, across all seven observed formats and all 6,236 zero-count surfaces. Four-ULP tolerance rescues no failures because exact violations are zero.
- The separate bounds array and embedded tris.mins/maxs pairs are finite/ordered but unequal on all 6,469 surfaces. The embedded endpoint candidate fails on every surface and 449,708 visits under both comparisons, including every zero-count surface. Interchangeability is rejected; no corrupt-original conclusion or purpose for the embedded fields is invented.
- Candidate-position consistency is supported without adopting C2M's 2.54 scaling, other attribute layout or original-engine truth. Formats 6/8 are absent; units/axes/winding and required stream1 spans remain unknown.

EVIDENCE:
- Root AGENTS, asset-inspection/bo2-reverse-engineering skills, architecture/state, findings 0005/0006, handoffs 0009/0010, existing safe assets API and synthetic tests inspected.
- Only zone/all/zm_tomb.ff, Steam Zombies build 24784239 English, accepted 78,753,344-byte SHA-256 663c2f6e275fb6a8a2844bcc0e31808abe681ce7c2a2ddc6f616ef98fc7cd0b5; selected maps/mp/zm_tomb.d3dbsp. Originals unchanged/external; no other original input, preload, game/executable access, copies/writes or proprietary dumps.
- Pinned OAT 7d027e8f89118196713e955b0e11f8404149c54d surface declarations/count rules and generated loader confirm two separate bounds families in each 80-byte record, with scalar bounds unchanged during loading. Public C2M 9a3accfb75c9a2f9780446c2b7bef040570357cf supplies first 12-byte/36-byte addressing candidate only. Source links and evidence limits are in findings 0007.
- External 0011-gfxworld-position-prefix retains source/checks/fixtures/patch/binary, predeclared numerical policy, actual build commands/failed and final logs, supervisor, source/config/tool identities, reproduction procedure, restoration records and authoritative observations-v1/observation.json. Only aggregate counts/categories, hashes and the selected identifier are emitted; no coordinates, index values, pointers, names lists or original buffers.

CHANGES:
- Added docs/reverse_engineering/0007-gfxworld-position-prefix.md and this handoff; minimally updated docs/README.md, reverse-engineering index and project-state.
- External observation instrumentation only. Accepted task 9 exact source/header/binary/patch restored; separate task 10 artifacts unchanged; task 11 preserved independently.
- No tracked Rust/manifests/dependencies, unsafe/FFI or architecture API changes; no geometry/import/export/rendering/gameplay implementation, global environment changes, commits, pushes or nested agents.

VALIDATION:
- Before originals, same helper passed 46 new native cases plus accepted 22+26. Valid/negative/null/zero/exact-end/truncated/overflow/oversize/unsupported-endian/unaligned-memcpy/NaN/infinity/invalid-bounds/numeric boundary cases include signed zero, one/four/five-ULP excursions and expansion-to-infinity rejection.
- Final same-host Release/Win32 Tools\UnlinkerCli build exited 0 with zero warnings in 14.625 seconds; cumulative 46.312 seconds within 900 seconds. Two initial typed-vector/Win32-index corrections and failed logs retained. Production translation unit recompiled; fresh schema 11/revision 1 required.
- Supervisor output-cap/deadline synthetic children passed before originals; two observations exited 0 with empty stderr and zero warnings/errors in 7.797/7.407 seconds, 5,751 bytes each, under 120 seconds/256 KiB/8,192-byte-record limits. Observed peak working sets 391,479,296/390,627,328 bytes; native decoder allocation is not hard-capped or a bounded-prefix parser.
- Per run: 899,442 index visits, 640,980 position visits / 1,922,940 float visits / 7,691,760 prefix bytes, 77,628 bounds float reads / 310,512 bounds bytes. Complete-group, unique-address and per-surface reference categories are distinct. Admitted spans, category/accounting sums, logical partition, exact repeated records and no-extra-process-output checks passed.
- Original SHA-256/length/write-time/same-handle identity and official/task 9/task 10/task 11 binaries unchanged around observations. Task 11 binary 4,792,832 bytes SHA-256 4f94d5e8c2c76bc82f02477026f1da063bb4d00d80568d9f73d6926b5351bde3. Task 9 restoration/exact patch, 13 pristine source identities and nine clean pinned submodules checked.
- Five owned documents passed 46 relative-link checks, six immutable-source-anchor checks, ten required handoff headings, configured-root/content and all-file whitespace checks; git diff --check passed. Nineteen external helper/support/reproduction identities checked. Unchanged Cargo gates deliberately not repeated. No live original-game/visual comparison, independent authenticity check or absent-format validation.
- Outer review accepted the bounded measurement after independently rerunning 46+22+26 native cases, supervisor cap/deadline children and two observations (5.375/5.406 seconds, 5,751 bytes each; observed peaks 390,979,584/391,356,416 bytes). All four records agree exactly, with originals/four binaries unchanged and no unexpected outputs. External parent-review-014230 retains results. Review verified nineteen artifacts, thirteen pristine sources, eleven upstream Git blobs, nine clean submodules, exact task9 restoration, all documentation links/anchors/headings and content/whitespace boundaries. Production compilation declares no fast floating-point override.

FIDELITY STATUS:
INVESTIGATING
Candidate structural/position consistency only; overall, gameplay and visual fidelity UNKNOWN. No MATCHED/VERIFIED or overall-goal completion claim.

UNRESOLVED:
- Original semantic use of both bounds families, GPU/layout/draw commands, selective positive counts, normal/UV/color encoding, stream1 consumption/required spans and absent formats 6/8.
- Units/axes/winding, material/texture completeness, representative geometry appearance and all gameplay.
- Native allocations remain entrusted to the pinned decoder; no hard memory cap, independent authentication or original-game comparison.

RECOMMENDED NEXT WORK:
- One bounded format 0 required-stream investigation: establish whether its at-capacity surfaces consume stream1, and a source-supported required-span/layout rule before a complete geometry/material import.

SCOPE NOT IMPLEMENTED:
- Rust/gameplay/rendering/geometry exporter/importer/Bevy asset loader/architecture APIs; normals/UV/colors/stream1 bodies/runtime layouts/units/axes/winding/collision/lights/models/other mechanics; other originals/inputs/game processes; original copies/writes/dumps; commits/pushes/nested agents.
