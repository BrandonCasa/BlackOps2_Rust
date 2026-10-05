TASK:
Observe the single decoded zm_tomb GfxWorld's surface triangle/index/vertex-range and material-pointer boundary using bounded external pinned-decoder metadata tooling; no game/Rust geometry implementation.

STATUS:
COMPLETE

FINDINGS:
- Two identical observations: 6,469 surfaces, draw logical vertexCount 191,259, draw indexCount 449,721, triangle-count sum 149,907; streams declare 6,886,816 and 489,788 bytes.
- All index candidate spans fit and partition all 449,721 elements exactly, with zero gaps/repeated elements/invalid ranges. No negatives, policy exhaustion or index values emitted.
- Vertex candidate endpoints all fit arithmetically, but 6,236 surfaces declare zero vertices despite nonzero triangles. Only 233 satisfy the local-index candidate; none satisfies the already-global candidate. A universal logical vertex slice remains unestablished; complete vertex byte spans are unknown for all surfaces.
- Stream-0 starting-offset candidates fit all surfaces; stream 1 fits 6,457 and fails for 12. No sentinel/layout/corruption conclusion is established.
- All surface material pointers match non-reference-marked pool entries, 280 unique pointers; zero null/reference-marked/unmatched/ambiguous surface categories. Pool has 1,564 non-reference-marked and 122 reference-marked material entries under OAT's convention. Pointer joins establish no material completeness or external dependency resolution.

EVIDENCE:
- AGENTS, asset-inspection/bo2-reverse-engineering skills, architecture/state, findings 0004/handoff 0008, existing envelope APIs and synthetic tests; no duplicate asset/game boundary introduced.
- Only zone/all/zm_tomb.ff, Steam Zombies manifest build 24784239, English; accepted 78,753,344-byte SHA-256 identity unchanged. No game launch, other original inputs, writes/copies, containers or preloads.
- Immutable OAT v0.33.0 source at 7d027e8f89118196713e955b0e11f8404149c54d; native field/loading/reference declarations and exact links in findings 0005. Source declarations and candidate interpretations are separated from measured data.
- External tooling/OAT/surface-7d027e8 retains pinned source/submodules, verified Premake, native binaries/tests, instrumentation patch, pristine-loader checks and build/budget logs. External 0009-gfxworld-surface-boundary retains observe.py, tooling identities, supervisor checks and observation records/logs. No actual roots or proprietary contents enter tracked files.

CHANGES:
- Added findings 0005 and this handoff; minimal docs/README, reverse-engineering index and project-state updates.
- External ContentPrinter instrumentation suppresses full directory output; only aggregate metadata is emitted. Official release files and normal LIST/signed loader are unchanged.
- No tracked Rust/manifests/dependencies/architecture APIs, unsafe policy, FFI, loader, geometry/rendering/gameplay implementation, global runtime/PATH change, commit or push.

VALIDATION:
- Pinned checkout/gitlinks and clean submodule worktrees verified; selected loader/source hashes remain pristine. Premake executable matches the pinned upstream digest. Existing Visual Studio 2026/MSBuild 18.10.1.42706 and x86 MSVC 19.51.36260.0 used with process-local environment.
- External Release/Win32 Tools\UnlinkerCli target exited 0 after routine long-path/header/actual-.slnx routing corrections. Initial 15-minute setup/build bound adjusted by outer agent to 30 and at most 40 total minutes while the same build produced prerequisites; initial error log and corrected incremental build log retained. No other final tool/test target was requested.
- Same arithmetic/reference/coverage helper passed 26 user-created native cases: checked arithmetic/overflow, zero/null/nonzero/null, signed negatives, truncation/out-of-range empty spans, oversized policies, five reference categories and union coverage cases. No original data in fixtures.
- Same adapted supervisor terminated synthetic combined output beyond 256 KiB and a sleeping child at a 0.1-second deadline, recording incomplete/unsuccessful exits. Checks passed before observations; originals were not opened by those selfchecks.
- Two single-input LIST metadata loads: exit 0, zero warnings/errors, empty stderr; 1.172/0.671 seconds and 2,541 combined bytes each, within 120-second/256-KiB limits. Peak working sets 391,979,008/391,254,016 bytes; native allocation remains uncapped. Emitted record below 8,192 bytes.
- Aggregate/range/category sums and repeat equality passed; 449,721 bounded internal index reads per run, no values emitted. Process directories contain only stdout/stderr supervisor logs, with no dumps/extra directories. Original SHA-256/length/write time/file identity and both binary identities agree before/after.
- Instrumented binary is 4,776,960 bytes, SHA-256 6ae454d3961d197067a116ebf23a5206c8532bea9fab16944f59ab0f131105f4. Official binary remains 4,756,480 bytes, accepted SHA-256 9d1664baea42fe42b63c20ba04b3afcd866fd74ed7ec7c55d62581d6be78eb63.
- Documentation review passed: five files, 36 relative links, 13 immutable source anchors, required handoff headings and path/pointer/dump-content checks. Aggregate consistency/helper identity/output checks and git diff --check passed; new-file whitespace checked separately. Unchanged Cargo gates deliberately not repeated. No original-game, rendered geometry or independent authentication comparison performed.
- Outer-agent review accepted the scoped findings after independently rerunning 26 native synthetic checks, both supervisor selfchecks and two observations (0.719/0.688 seconds, 2,541 bytes each). All four aggregates agree exactly; source and both binaries retain their identities, with no unexpected process outputs. Pinned source, nine clean submodules, eleven pristine files, scoped patch/header and helper hashes were independently verified; CRLF normalization was limited to Git-blob comparison. Documentation/content and whitespace checks passed. The vertex-stream question remains unresolved.

FIDELITY STATUS:
INVESTIGATING
Static structure/range/material-pointer evidence only; gameplay, visual and overall fidelity UNKNOWN.

UNRESOLVED:
- Per-surface vertexCount/firstVertex meaning, zero-count cases, stream offsets/sentinels, layout-dependent stream requirements, encoding/complete byte spans, axes/units/winding and actual original draw semantics.
- Material/texture/shader completeness, reference-marked entry relationships/ownership, other zones/loading order and representative geometry renderability.
- No hard decoder memory cap, independent/negative signature check or live original-game appearance comparison.

RECOMMENDED NEXT WORK:
- One bounded per-surface vertex-stream interpretation investigation on the same decoded world, using the established external aggregate access route. Resolve zero counts/firstVertex/offsets/layout requirements before defining a safe Bevy geometry adapter. Do not silently assume uniform stride or universal logical vertex intervals.

SCOPE NOT IMPLEMENTED:
- Static models, lights, collision, textures/shaders, coordinates/winding, other asset families/inputs, extraction/dumps/conversion, game launch/original writes/copies, Rust decoder/FFI/game dependencies, rendering/entity/gameplay APIs, commits or pushes.
