TASK:
Qualify the ordinary jump eligibility/launch transition from the native single-step owner.

STATUS:
PARTIAL

FINDINGS:
- Qualified ordered rejection predicates, bit21 edge mutation and >=500ms eligibility boundary.
- Qualified ordinary handler owner permission reads, first-launch height39 and SSE impulse order, authoritative launch writes and event parameter selection.
- Landing does not imply unconditional jump flag reset; qualified explicit reset and slowdown expiration paths.
- Full jump remains dependent on player accumulator provider/x87 behavior, gravity/profile, collision receiver, timer progression and landing consequences.

EVIDENCE:
- movement-command-update.md, movement-grounded-contract.md, baseline parameters, kernel05 handoff, existing gameplay/physics tests and current diff inspected.
- Identity-bound static disk bodies and literals; external movement-native-06/qualification-receipt.json records10 bounded body identities.
- Detailed source conditions and finite missing links: ../reverse_engineering/movement-jump-contract.md.
- No original process access, invocation, play or capture.

CHANGES:
- Added movement-jump-contract.md and this handoff only; no shared API or Rust modifications.

VALIDATION:
- External independent Python source-flow reconstruction:15 eligibility checks passed, five synthetic gravity impulse fixtures emitted.
- Reviewed bounded function ends; truncated0af620 at adjacent function rather than accepting padding delimiter.
- git diff --check passed for owned documentation.
- No Rust/build changes; workspace checks remain primary integration responsibility.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Provider31fc40c/default and x87 FP control for state588 update;31fc410 repeated-jump branch.
- Actual timer1c update owner and full landing thresholds/providers/perk consequences; active gravity/profile binding.
- Owner storage bounds/lifetime and environment binding; air/step solver receiver remains collision lane responsibility.
- Floating-point startup/control policy; no original comparison.

RECOMMENDED NEXT WORK:
- Implement separately reviewed checked first-launch impulse and supplied eligibility decision boundary, clearly excluding full jump.
- Keep startup FP policy/parameter initializer investigation separate; then bound timer/landing ownership qualification.

SCOPE NOT IMPLEMENTED:
- Full motor/jump, repeated-jump slowdown, landing damage/events, collision families, sprint/stance/camera and ECS.
