TASK:
Analyze the first user-controlled live actor numeric sequence and add a scoped resolved-boundary replay.

STATUS:
COMPLETE

FINDINGS:
- 487 accepted samples show 150, 128, 106, 84, 62, 40, 23, 11, then -11; maximum 150. Gate remains 1 until 14 negative samples with gate 0. Sample 487 refuses team change without health; final qualification passes.
- User reports roughly two magazines fired and all round zombies killed. No exact shot/hit-location/visible-target mapping or perk/objective exclusions established.
- Supplied resolved differences replay seven positive values exactly; conditional final 22 produces signed stage -11, while ActorDead.current remains None. No weapon constant or final callback-health rule inferred.

EVIDENCE:
- External trial03-user-control.jsonl hash matches 072df30aee2f91b7d0b091161c5a1a6d667b318baaa6ba957b7a09e192bb841e; 489 records include refusal/final qualification, 56,976 bytes read. Parent session 45453 terminal shell exit 1; observer refusal path 2.
- Task21 observer/native identity limits, source architecture and prior handoff22; parent UI/user action reports. Comparison document records groups and conservative acquisition brackets, not attack cadence.

CHANGES:
- docs/fidelity/damage-life-state-comparison.md: scoped methods/counts/limits and historical-state wording repair.
- crates/gameplay/tests/damage.rs: one exact ordered-queue replay with negative arithmetic stage, unknown final health, terminal repeat and deliberately different-input detection.
- This handoff. Parent-owned project-state/fidelity README changes preserved/excluded.

VALIDATION:
- cargo test -p bo2-gameplay -p bo2-origins --locked: 40 tests pass, exit 0.
- cargo metadata --locked --no-deps --format-version 1: 10 unchanged members, exit 0.
- cargo test --workspace --locked: 120 tests pass, exit 0.
- cargo fmt --all -- --check and cargo clippy --workspace --all-targets --locked -- -D warnings: exit 0.
- Diff/content/UTF-8 checks pass; ASCII range separators, no U+FFFD or proprietary/local-path additions. No new original sampling or game controls; all operations terminal.
- Parent independently reran the exact new test: pass, 8 filtered.

FIDELITY STATUS:
APPROXIMATE

UNRESOLVED:
- Raw delivered payloads, shot association/cadence, ordinary archetype, between-sample reuse, callback stage/final health and repeatability. Inputs inferred from snapshots do not independently measure native raw damage.

RECOMMENDED NEXT WORK:
- Repeat an isolated visibly associated actor hit with action details; qualify raw impact and final-state questions separately. Solo/last-player disposition remains adjacent work.

SCOPE NOT IMPLEMENTED:
- Weapons/gameplay changes, native/observer/UI work, policy changes, commits or pushes.
