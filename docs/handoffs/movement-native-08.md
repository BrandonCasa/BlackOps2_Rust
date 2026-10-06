TASK:
Implement source-qualified checked integer sprint budget and ordinary stop transition building blocks.

STATUS:
COMPLETE

FINDINGS:
- Implemented explicit integer budget fields/parameters, zero-start sentinel, source timestamp branches, ordered debit/recharge arithmetic and strict restart minimum.
- Implemented atomic stop output with preserved latch/start/stored fields, inactive no-op and supplied qualified jump interruption delay800ms.
- Signed overflow rejection is a Rust boundary policy; no native wrapping/default/profile claim.

EVIDENCE:
- Reviewed current gameplay timing/acceleration code/tests, kernel05 handoff, concurrent diff and sprint07 direct bodies/independent reference.
- Source068770 and540c10/541ef0; detailed evidence and implementation limits in ../reverse_engineering/movement-sprint-contract.md.
- Independent external sprint07 reference supplies 16 synthetic authoritative fixtures.

CHANGES:
- Added crates/gameplay/src/movement/sprint_budget.rs and crates/gameplay/tests/sprint_budget.rs.
- Added brief sprint-contract implementation notes and this handoff.
- Primary added the movement mod export; no worker edits to manifests/lib/shared exports/lockfile.
- API supplies all fields/parameters; negative maximum cap preserves the raw query while full owner's duration-stop gate stays external.

VALIDATION:
- cargo test --locked -p bo2-gameplay --test sprint_budget:10 passed, including 9,600 independent bounded accounting cases.
- cargo clippy --locked -p bo2-gameplay --all-targets -- -D warnings:passed.
- Owned rustfmt --check and git diff --check:passed.
- Meaningful tests cover sentinel/equality/debit/recharge/minimum/cooldown, latch and inactive stop, future stop time, overflow at source intermediates and atomic errors.
- Used installed pinned toolchain shims; no dependency/toolchain changes.
- Workspace/headless/client/integration gates remain primary-owned. No original execution or comparison.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Full sprint start/continuation eligibility, missing profile/provider values, weapon state, posture/clearance receiver and x87 speed/control policy.
- Full motor, other modes, ECS/host integration and original comparison.

RECOMMENDED NEXT WORK:
- Qualify ordinary stance transition independently while profile/FP/collision work proceeds; retain one numerical motor owner.
- Integrate these budget/stop kernels only with explicitly qualified caller decisions and supplied configuration.

SCOPE NOT IMPLEMENTED:
- Full sprint/motor, start authorization, duration weapon conversion, speed scaling, stance clearance, jump/landing, collision and ECS.
