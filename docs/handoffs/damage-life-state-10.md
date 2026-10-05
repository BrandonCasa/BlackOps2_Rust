TASK:
Qualify player damage writer/arithmetic against observer route and active dispatch.

STATUS:
PARTIAL
Native implementation resolved; active VM dispatch/execution unqualified.

FINDINGS:
- Registered finishplayerdamage uses observer table/current; direct32-bit SUB,
  signed post-health test and later client+238 mirror observed.
- Integer helper accepts tag7 payload; other tags type-error. Signedpositive
  damage rejects u32::MAX interpreted as signed -1. Recommend positive<=i32MAX admission.
- Signed health/max getter/storage supports bounded positive project subset;
  native setters do not enforce positivity. Raw lethal-999 is layered, not Downed1.

EVIDENCE:
- Source01/native04/observer08/diagnostic09 and actual API/tests;
  ../reverse_engineering/damage-life-state-10.md, external task10 receipts.

CHANGES:
- Findings/handoff only; bounded external inspector. No gameplay/API changes.

VALIDATION:
- Profile, name/record/page/span/repeat and five parser synthetic cases pass.
- Content/whitespace checks pass; Cargo unchanged/not rerun. No UI/sampling.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Active builtin ID0x197 lookup/dispatch/override and live execution; callback
  side effects, upstream conversions and actor arithmetic.

RECOMMENDED NEXT WORK:
- Bounded signedpositive admission correction; separately qualify active dispatch.

SCOPE NOT IMPLEMENTED:
- Gameplay, live comparison, actor/other policy tracing, UI/writes/injection,
  broad framework, commits/pushes or nested agents.
