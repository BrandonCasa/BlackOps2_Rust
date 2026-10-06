TASK:
Implement source-qualified integer posture classification, stage duration and active view-height transition timing.

STATUS:
COMPLETE

FINDINGS:
- Rechecked1a3920: state5b8 is a full signed dword and signed jl selects special target22 behavior; negative nonzero values classify crouching.
- Added reusable Posture separate from life state, explicit stage-duration kernel and checked active-stage progress.
- Preserved direction zero/nonzero, signed subtraction then multiplication100, truncating division and clamp0..100.
- Timestamp0 returns None before arithmetic, excluding full-owner new-stage initialization; percent100 carries no completion writes.

EVIDENCE:
- Reviewed existing motor kernels, native09, project state/architecture and current concurrent diff.
- Retained native09 complete classifier1a3920 and duration037ae0 bodies plus active percentage53ebc6..53ec3e.
- Source associations and implementation notes: ../reverse_engineering/movement-stance-contract.md.
- No original process/instruction invocation/play/capture.

CHANGES:
- Created crates/gameplay/src/movement/posture_timing.rs and crates/gameplay/tests/posture_timing.rs.
- Corrected signed-width classifier description, added brief stance-contract notes and this handoff.
- Primary separately supplied export. No worker shared/manifests/lockfile edits.

VALIDATION:
- cargo test --locked -p bo2-gameplay --test posture_timing: seven tests passed.
- Thirteen classifier/twelve timing source fixtures and2,968 bounded independent wide-integer progress cases passed; included source boundaries, signed extremes, zero timestamp and atomic errors.
- cargo clippy --locked -p bo2-gameplay --all-targets -- -D warnings: passed.
- Owned rustfmt --edition2024 --check, git diff --check and whitespace/local-path review: passed.
- Workspace/client integration gates remain primary-owned; no original comparison or full controller claim.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Special stage22 ordinary reachability; standing target/default and full owner provider/state binding.
- Float curves/FP policy, new-stage/reversal/completion writes and full posture controller remain source dependencies.

RECOMMENDED NEXT WORK:
- Primary review/integration of these integer kernels.
- Continue admitted posture owner transitions after binding provider, curve arithmetic and collision dependencies.

SCOPE NOT IMPLEMENTED:
- Float interpolation, hull/clearance requests/events, standing defaults, FP mutation/policy, full motor/ECS/host or life-state coupling.
