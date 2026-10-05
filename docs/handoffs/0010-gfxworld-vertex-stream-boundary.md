TASK:
Resolve the selected zm_tomb GfxWorld's zero vertexCount cases and supported per-surface vertex-stream boundary through pinned public source and same-zone aggregate observations; no geometry implementation.

STATUS:
COMPLETE
The bounded investigation is complete; attribute encoding, required stream-1 spans and original draw semantics remain explicitly unverified.

FINDINGS:
- All 6,469 surfaces, including all 6,236 zero-count cases, fit the public Black Ops II extractor's stream0 offset + local index * 36 addressing candidate. No surface/index/category is omitted.
- 320 offset groups: all indexed extents fit their next boundary and round exactly to it at 16-byte alignment. Extent sum 191,259; 6,885,324 candidate record bytes plus 1,492 padding bytes account for all 6,886,816 stream0 bytes, with zero leading gap.
- 285 groups contain only zero counts; 35 only positive counts, covering 6,236 and 233 surfaces respectively. No zero-count surface shares a group with a positive-count owner. The simple ownership/sentinel explanation is rejected; the field is not a universal access bound. Why it is populated selectively remains unknown.
- The 320 firstVertex + derived-group-extent intervals partition all 191,259 logical positions with zero gaps/repeats, in an order different from stream0 byte order. This supports a group logical-base candidate, not an original draw trace.
- Seven worldVertFormat categories, 56 technique sets, 1,120 techniques/passes, fifteen declarations and 75 routes observed; no route enum-range failures. Runtime input layouts are not loaded, and routing names do not establish byte stride/required physical streams.
- Stream1 has no negative or above-capacity offsets. Fifteen surfaces point exactly to its end, all format0; twelve have positive counts and exactly explain the previous strict-start-check failures. Whether those surfaces consume stream1 is unknown; complete required stream1 spans remain unestablished for all surfaces.

EVIDENCE:
- Root AGENTS; asset-inspection/bo2-reverse-engineering skills; architecture/state; findings 0004/0005 and handoffs 0008/0009; existing asset API/inspector and synthetic tests.
- Only zone/all/zm_tomb.ff, Steam Zombies build 24784239 English, accepted 78,753,344-byte SHA-256 identity. Original is unchanged; no game launch, other original source, copy/write, preload, raw container or shader/script inspection.
- OAT v0.33.0 immutable commit 7d027e8f89118196713e955b0e11f8404149c54d: T6 schema, generated surface/material-technique loaders, pass/routing rules. Public C2M BlackOps2.cs at 9a3accfb75c9a2f9780446c2b7bef040570357cf supplies the 36-byte addressing candidate; its PC-runtime/version/stream1/layout limitations are explicit in findings 0006.
- External 0010-gfxworld-vertex-stream-boundary retains final standalone tool, supervisor, source/header snapshots, patch, native fixtures, public/source/binary identities, exact commands/build logs, attempt records and authoritative observations-v3. No proprietary values or actual installation/output roots enter tracked documents.

CHANGES:
- Added findings 0006 and this handoff; minimal docs/README, reverse-engineering index and project-state updates.
- External aggregate instrumentation only. Task9 accepted ContentPrinter/header/binary and exact patch restored; task10 artifacts kept separately.
- No tracked Rust/manifests/dependencies/architecture API, unsafe/FFI change, geometry/rendering/gameplay implementation, global environment change, commit or push.

VALIDATION:
- 22 new native synthetic checks plus 26 accepted checks passed before originals and on later builds: signed/null/zero/truncation/overflow/policy/stride/alignment/sentinel/overlap cases.
- Same Release/Win32 Tools\UnlinkerCli target built initially in 224.781 seconds under 900 seconds. HostX86 versus Hostx64 is a supported dependency-rebuild explanation, not a proven sole cause. Same-host follow-ups completed in 11.984, 12.406 and 14.297 seconds; no extension needed.
- The header-only 11.984-second attempt did not compile ContentPrinter and emitted stale keys; preserved separately and not accepted as follow-up validation. Translation-unit changes forced fresh-key revision2/final builds. Final helper requires the revised key set.
- Supervisor combined-output/deadline children passed before each pair. Authoritative final pair: exit0, zero warnings/errors, empty stderr; 1.203/0.719 seconds, 6,163 bytes each, within 120-second/256-KiB limits and record cap 8,192 bytes. Peaks 391,036,928/391,303,168 bytes; no hard native memory cap claimed.
- Exact final repeat equality and category/accounting invariants passed; 899,442 bounded internal index visits per run. No vertex-stream body/coordinate reads or emitted index/pointer/material-name content. No unexpected process outputs beyond supervisor logs.
- Eight total same-input loads across separate preliminary/stale-key/revision2/final pairs. Original SHA-256/length/write time/same-handle identity and official/task9-backup/task10 binaries unchanged per pair. Mixed revisions are not compared as complete records.
- Final task10 binary: 4,793,856 bytes, SHA-256 0d6884ddd1d10748501b26bafc10de53de299ea65b29ce124ef379b29fff8dee. Task9 source/header/binary restored to accepted hashes and exact patch; eleven upstream source files and official binary unchanged. Extra task10 headers removed from reused checkout after external preservation.
- All five owned documents passed 41 relative-link checks, eleven immutable-source-anchor checks, ten required handoff headings, path/content checks and new-file whitespace checks; git diff --check passed. Cargo gates deliberately not repeated because tracked code is unchanged. No independent authentication, live original-game, rendered geometry or attribute-encoding comparison.
- Outer-agent review accepted the scoped findings after inspecting all final instrumentation/supervisor/test changes and independently rerunning 22+26 native cases, both supervisor limit children and two final observations (0.656/0.672 seconds, 6,163 bytes each). All four final aggregates agree exactly; original/tool identities stayed unchanged. Nine clean pinned submodules, thirteen recorded source identities (eleven upstream plus two generated), nine helper/artifact hashes, the C2M blob against its pinned tree, task9 restoration/exact patch and documentation/content/link/whitespace checks passed. External parent-review-050831 retains results. Attribute encoding and original draw semantics remain unverified.

FIDELITY STATUS:
INVESTIGATING
Static descriptor/index/byte-boundary evidence only; gameplay, visual and overall fidelity UNKNOWN.

UNRESOLVED:
- Original reason for selective positive vertexCount fields and actual draw-command use of firstVertex/counts.
- Candidate position/normal/UV/color encoding, stream1 consumption/layout/complete spans, absent format6/8 support, units/axes/winding, material/texture completeness and appearance.
- No hard decoder memory cap, independent authenticity check or original-game comparison.

RECOMMENDED NEXT WORK:
- One bounded same-world candidate position-prefix validation: admitted 36-byte addressing, internal float32 finite/bounds consistency across all seven observed categories and every zero-count case, synthetic invalid/truncated floats first, aggregate-only output. Establish position evidence before Rust/Bevy geometry import.

SCOPE NOT IMPLEMENTED:
- Vertex attributes/coordinates, stream1 decoder, geometry conversion/import/rendering, static models/collision/lights/textures/shaders, other inputs/assets/mechanics, game launch/original writes/copies, Rust decoder/FFI/unsafe/API/dependencies, commits/pushes or nested agents.
