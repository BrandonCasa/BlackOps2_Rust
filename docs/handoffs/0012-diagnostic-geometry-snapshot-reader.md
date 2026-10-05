TASK:
Implement one safe Rust read-only consumer and thin inspector for a project-owned diagnostic position/index geometry snapshot, with synthetic tests; no original reader, exporter or renderer.

STATUS:
COMPLETE
The scoped synthetic consumer works, all applicable gates pass, and outer-agent review accepts this bounded task. This does not complete representative original geometry or the overall goal.

FINDINGS:
- Chosen project-owned v1 exchange contract: 64-byte fixed LE header; finite binary32 XYZ positions; u32 explicit snapshot indices; ordered 48-byte surface records containing triangle ranges, opaque source-format/category counts and separate finite ordered endpoint triples. This is not a discovered BO2 disk format or complete engine asset architecture.
- Surface ranges partition all indices in order, including valid empty surfaces. Zero source declared counts and any u32 category remain opaque and never filter references. Own-surface inclusive containment is exact, with no tolerance/repair. Signed zero/subnormal/finite bits, equal/unreferenced records, repeated indices and degenerate triangles are preserved.
- Fixed policies: 64 MiB total bytes, 2 million positions, 10 million indices/reference visits, 100,000 surfaces, 60 MiB combined retained Vec capacity bytes and 1,024 cumulative Interrupted retries. Checked header size/count/platform/length admission precedes body reads and fallible vector reservations. Actual reported capacities are checked before body reads. Allocator/process overhead is not a hard memory cap.
- One 4,096-byte scratch buffer decodes bounded section-aligned chunks directly into validated private arrays. No full input Vec is retained. Admitted ranges stay within L except a one-byte EOF sentinel; consumed bytes <= L+1, while short reads may request an unread span again. A blocking custom Read has no time bound, and same-length rewrites/mutations after EOF remain undetectable.
- The inspector opens one explicit native regular-file path read-only using same-handle metadata length. It emits counts/status and ten fixed opaque category bins (0..8/other), never coordinate/index/endpoint values, names, paths or malformed fields. No config/recursive discovery or output files. Exit codes: success/help 0, inspection/input/output 1, invalid arguments 2; I/O diagnostics retain only ErrorKind.

EVIDENCE:
- Inspected root AGENTS (only applicable AGENTS), asset-inspection/architecture-extraction/validation skills; architecture README/ADR 0001/project-state; accepted findings 0005/0006/0007 and handoffs 0010/0011; existing assets/fastfile API/tests, asset-inspector source/tests, core/boot source, actual Cargo graph and reserved map-inspector directory.
- Local downloaded Bevy 0.19.1 manifest/internal reexports plus custom_asset and generate_custom_mesh examples show AssetLoader/AssetServer/Handle/Assets and Mesh TriangleList/POSITION/U32 index facilities. No new Bevy feature/dependency is needed or added. Bevy retains engine/ECS/render/asset lifecycle ownership.
- Game data, original files, scripts/entity definitions and external native OAT tools inspected by this task: none. Accepted position-prefix findings support why this diagnostic boundary was selected; they do not prove original layout/draw/material/runtime semantics.
- All test arrays are arbitrary user-created values. Worker external validation retains a 136-byte synthetic signed-zero/subnormal degenerate-triangle snapshot and two CLI stdout/stderr pairs. Parent review additionally retains its independent 4,404,568-byte synthetic fixture, driver, CLI and Cargo logs. All remain external, rooted through ignored configuration without emitting actual roots; no original-derived geometry is present.

CHANGES:
- Added crates/assets/src/geometry_snapshot.rs and tests/geometry_snapshot.rs; exported the focused module and minimally updated package description. Public borrowed accessors/owning decomposition expose validated arrays without inventing engine identities.
- Added tools/map_inspector Cargo.toml, src/main.rs and tests/cli.rs; added one path-only workspace member and Cargo's seven-line lock entry. No third-party dependency or core/boot/fastfile change.
- Added docs/architecture/diagnostic-geometry-snapshot.md and this handoff; minimally updated architecture README, docs index and project state. The byte/empty/count/bounds/resource/read/emit contract was documented before implementing the reader/tests.
- No originals, source dumps, proprietary fixtures, generated builds, global environment/toolchain changes, unsafe/FFI, commits/pushes or nested agents.

VALIDATION:
- Focused initial parser run exited 0 with 22 cases; focused map-inspector run exited 0 with 2 unit + 9 integration cases. Three additional parser cases cover endpoint bit preservation, cumulative staged Interrupted budget and impossible body/EOF Read return counts.
- Final `cargo metadata --locked --no-deps --format-version 1` exited 0: five expected members, assets std-only, inspectors -> assets, boot -> core, core/boot -> existing Bevy. `cargo test --workspace` exited 0 with 61 tests: 25 new geometry parser, 2 map-inspector unit, 9 map-inspector integration and 25 unchanged existing tests. No ignored/failed cases.
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` exited 0. Initial Clippy array-chunk/slice-size suggestions were corrected without lint suppression; final implementation uses stable safe array chunks.
- Parser cases cover each actual-file truncation byte and unexpected reader-EOF byte, unsupported header variants, multiplication/addition overflow, count/byte/retention preflight, exact header/caller lengths and trailing sentinel, chunks/short/Interrupted/error/invalid-count readers, NaN/either infinity, invalid indices, nontriangular/misaligned/out-of-range/overflow/gap/overlap/incomplete ranges, nonfinite/reversed/empty bounds, own-reference containment and one-float-step rejection. Categories 6/8/u32::MAX and zero source counts do not reject. Synthetic allocator exhaustion is not forced; try_reserve failures are propagated/redacted.
- CLI cases cover help/invalid args, actual subprocess success, source bytes/write-time preservation, identical summary across changed coordinates/endpoints/reference order, missing/directory/option-like/non-Unicode native paths, malformed/truncated/trailing input redaction with no partial summary, I/O/output errors and 1,024 distinct opaque categories producing 20 fixed output lines under 2,048 bytes.
- `cargo run -p bo2-boot -- --smoke` exited 0 with startup 1/updates 3/probe_updates 3; `cargo run -p bo2-map-inspector -- --help` exited 0. Two additional external synthetic CLI runs exited 0 with empty stderr, 1,070 output bytes each, exact repeat/count/category/redaction checks and unchanged SHA-256/write time. No original input or game/runtime comparison.
- Parent reported an independent Python-encoded entirely synthetic 4,404,568-byte fixture with 191,259 positions/449,721 references/6,469 surfaces passing twice, 1,162 output bytes each, exact category/count equality and same-handle preservation; parent-review-020124 retains its records. This is parent-reported synthetic validation, not original geometry or fidelity evidence.
- Parent independently reran locked metadata, all 61 workspace tests, formatting, Clippy, Bevy smoke and snapshot help successfully, reviewing the final code/CLI tests and confirming nine source/test/manifest hashes unchanged during checks. Functional files remained frozen while documentation was completed. The final binary also reproduced the independent full-size fixture result; 14 candidates, 48 relative links, ten headings, content/unsafe/graph boundaries and whitespace passed independent review. The bounded consumer is accepted; no original or fidelity claim is promoted.
- Dependency/unsafe/content/whitespace checks passed for all 14 candidate files, including untracked additions; all 48 relative documentation links and ten required handoff headings passed. `git diff --check` passed. New Rust test/entrypoint crates forbid unsafe, and the module inherits assets' existing prohibition. No original assets/scripts/binaries/dumps, real installation/output paths, secrets or generated builds are candidates.

FIDELITY STATUS:
INVESTIGATING
Original data understanding only; overall, gameplay and visual fidelity UNKNOWN. Synthetic diagnostic-format validation supplies no MATCHED/VERIFIED or original authenticity/rendering claim.

UNRESOLVED:
- No original-derived diagnostic snapshot exists from this task, and no external producer/accounting or Bevy mesh/asset integration has been executed.
- Original draw commands, units/axes/winding, embedded-tris bounds use, selective source counts, stream1 consumption/layout, absent-format support, normals/UV/colors/materials/textures and appearance remain unknown.
- The parser does not provide source locking/authentication, a blocking-I/O deadline or a hard allocator/process-memory cap. Other platforms and forced allocator exhaustion were not tested.

RECOMMENDED NEXT WORK:
- After outer acceptance, assign one supervised external v1 producer preserving every admitted group record and every triangle reference/source surface order, including unreferenced/equal positions, zero source counts and degenerate triangles. Remap group-local indices with checked explicit snapshot bases, retain separate bounds and verify full counts/categories/source preservation; originals/output remain external.
- Separately assign a Bevy Mesh adapter/graphical comparison after accepting that producer. Use Bevy Mesh/assets facilities and establish any coordinate/winding decisions through evidence. Complete stream1/layout investigation precedes full attribute/material import.

SCOPE NOT IMPLEMENTED:
- Original/proprietary file reads, decoder/exporter, original-derived snapshots/fixtures, Bevy asset loader/Mesh conversion/renderer, units/axes/winding/scaling/welding/triangle filtering, other attributes/materials/entities/gameplay/audio/collision/navigation/rounds/mechanics, profiling/fidelity comparisons, original writes/copies, infrastructure/toolchain changes, commits/pushes and nested agents.
