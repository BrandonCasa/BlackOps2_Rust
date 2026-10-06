TASK:
Qualify ordinary unperked sprint transition, budget and speed-modifier boundary.

STATUS:
PARTIAL

FINDINGS:
- Qualified active/latch/exhaustion/time fields and ordered start/stop/debit/recharge transitions, including exact exhaustion and source >1000ms start boundary.
- Origins/C96 duration4000ms and cooldown0ms remain qualified inputs; continuation does not require holding sprint button30.
- Qualified conditional source speed factor1.5, two posture clearance consumers and raw start/continuation eligibility conditions.
- Full sprint remains dependent on forward/view-target/base-speed providers, profile/weapon-state ownership, alternate clearance receiver and x87 control.

EVIDENCE:
- Current state/diff, native02/03/04/05, jump06 and existing movement/physics test abstractions inspected.
- Identity-bound disk bodies/literals; external movement-native-07/qualification-receipt.json records13 bounded identities.
- Detailed conditions and equations: ../reverse_engineering/movement-sprint-contract.md.
- No original process reads, invocation, play or capture.

CHANGES:
- Created movement-sprint-contract.md and this handoff; no shared edits or Rust changes.

VALIDATION:
- Independent synthetic source-flow reconstruction passed16 integer-budget cases, two minimum-boundary checks and one jump-delay formula check.
- Reviewed padding-free207200 end before unrelated207260, source constants and stop call-site associations.
- Owned-document git diff --check passed.
- No Rust/build changes; workspace integration checks remain primary responsibility.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Parameter31fc4a0 forward threshold,31fe6a4 view target, native04 base-speed/axis provider associations and values.
- Player/weapon state initialization and production, optional command-bit23 helper11c830, posture/clearance receiver.
- X87 FP initialization for sprint speed multiplication; original comparison absent.

RECOMMENDED NEXT WORK:
- Implement separately reviewed checked integer sprint budget and stop writes using explicit timing/configuration inputs.
- Qualify stance and profile bindings separately before executable full sprint/host integration.

SCOPE NOT IMPLEMENTED:
- Full sprint/motor, stance/camera, collision, profile defaults, jump timer/landing, FP initialization and ECS.
