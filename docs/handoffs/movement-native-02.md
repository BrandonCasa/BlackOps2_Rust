TASK:
Recover the ordinary player command owner, timing/substeps, query associations,
stance bounds/masks, and exact next numerical motor roots from qualified disk code.

STATUS:
PARTIAL

FINDINGS:
- Qualified owner 034ad0 -> environment wrapper 1a6d30 -> loop 23af10 -> step544b30.
- Explicit server clamp -1000/+200ms, motor backlog150ms and <=66ms substeps;
  complete60-byte previous command copies occur after every substep.
- Qualified signed command time,64-bit buttons,signed byte axes,origin/velocity,
  posture hulls30/50/70,masks and ignored-owner field; camera conversion pending.
- Ordinary world callback table row1 resolves2fa5e0. Rise clearance uses a
  separate025170 producer; its response byte+2b derives from a negative
  intermediate scalar, and cannot yet be equated to classic brush booleans.

EVIDENCE:
- Current project state, architecture, native task01, collision/task0037 docs,
  physics source/tests and concurrent diffs inspected.
- Qualified static executable identity from task01; fourteen bounded bodies and
  referenced constants/callback/switch tables inspected; no process/game access.
- Detailed abstract findings: docs/reverse_engineering/movement-command-update.md.
- External movement-native-02/qualification-receipt.json holds hashes/lengths,
  switch targets and synthetic check receipt. All proprietary outputs external.

CHANGES:
- Added movement-command-update.md and this handoff. No Rust/shared contract edits.

VALIDATION:
- Independent Python native-loop reconstruction:9 timing cases,2 mask checks,
  3 prior-command substep copy cases and3 hull-schema cases passed.
- git diff --check and owned-doc prohibited-path/content review passed.
- Cargo checks not run for this documentation-only task; primary integration owns
  workspace/headless/client gates. No original comparison evidence.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Full numerical state/ground classification, movement kernels and angle encoding.
- Clearance intermediate27b1e0/00ff70 and refinement103b20 semantics/ownership.
- Standing view dvar,53eaa0 interpolation,prone03c790/18eb40 conditions.

RECOMMENDED NEXT WORK:
- Numerical owner qualifies walk544200 with friction53d9b0,accel53dc10 and
  step/slide0246c0; air543ec0 follows as one bounded transition increment.
- Collision owner qualifies alternate clearance producer and response equivalence.

SCOPE NOT IMPLEMENTED:
- Numerical motor,profile,ECS facade,host assembly and collision-family dispatch.
