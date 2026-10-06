TASK:
Implement ordinary jump eligibility and current-command bit21 mutation from native2b1cd0.

STATUS:
COMPLETE

FINDINGS:
- Added checked pure ordered eligibility kernel with explicitly supplied state/command/surface/owner permission.
- Preserved signed interval500, source rejection order, raw signed mode/posture and all high button bits.
- Held rejection clears only currentbit21; earlier rejection preserves it. Sentinel3ff bypasses posture/permission gates.
- Missing permission errors only on the reached nonsentinel callback route; checked subtraction overflow rejects the unqualified wrapping domain atomically.

EVIDENCE:
- Reviewed timing/acceleration/sprint kernels/tests, architecture, project state and concurrent diff.
- Reviewed native06 complete243-byte2b1cd0 owner and independent retained reference fixtures.
- Source evidence and implementation notes: ../reverse_engineering/movement-jump-contract.md.
- No original process, instruction invocation, play or capture.

CHANGES:
- Created crates/gameplay/src/movement/jump_eligibility.rs and crates/gameplay/tests/jump_eligibility.rs.
- Added brief jump-contract notes and this handoff.
- Primary separately supplied module export; no worker shared/manifests/lockfile edits.

VALIDATION:
- cargo test --locked -p bo2-gameplay --test jump_eligibility: eight tests passed;15 independent source-flow fixtures plus ordering, signed timing/overflow, permission/sentinel and button sequences.
- cargo clippy --locked -p bo2-gameplay --all-targets -- -D warnings: passed.
- Owned Rust rustfmt --edition2024 --check: passed.
- Owned candidate whitespace/local-path review and git diff --check: passed.
- Workspace/client integration gates remain primary-owned; no original comparison or full controller claim.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Callers must supply qualified effective posture, surface/owner and permission; owner storage/lifetime parsing remains external.
- Complete launch/FP/air/landing dependencies and executable baseline remain unresolved in source contracts.

RECOMMENDED NEXT WORK:
- Primary review/integration of this bounded eligibility boundary.
- Continue qualified launch/landing or posture numerical increments with explicit arithmetic policy and dependency contracts.

SCOPE NOT IMPLEMENTED:
- Jump impulse/state/event writes, repeat jump, landing, full motor, ECS/host, FP/default profiles, collision or owner storage parsing.
