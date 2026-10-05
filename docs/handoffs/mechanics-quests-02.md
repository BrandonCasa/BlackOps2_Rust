TASK:
Establish one air staff upgrade-unlocked -> insertion -> initial charge -> upgraded
pickup transition, including its event dependencies and ownership boundary.

STATUS:
COMPLETE

FINDINGS:
- Base weapon removal/insertion, accumulated charge, upgraded availability and
  eventual player grant are separate. Base/upgraded metadata share a charger;
  shared ownership uses base identity.
- Crypt gem completion produces unlock after base-staff damage, clearance and
  orb return. Earlier puzzle names are external struct prerequisites; exact
  mapping is unresolved, rather than assumed from air puzzle flag names.
- Eligible death increments initial charge before delayed soul-received; initial
  checker uses count20, not notification arrival. Retrieved notification precedes
  eligibility and can occur without grant.
- Undefined-player insertion branch, later-array distance ties, callback consumers
  and interruption without an observed rollback are explicit counterexamples.

EVIDENCE:
- Read current Git status, project state, architecture, prior review/quest01
  handoffs, Origins and weapon-analysis skills and preceding staff ownership record.
- Fresh five compiled-script hashes: main quest, craftables, air quest, crypt quest
  and map utility. Five decompile/five disassemble invocations exit0 with retained
  GSC Tool1.4.10.333; raw outputs and input/output receipts stay external.
- [Findings02](../reverse_engineering/origins-mechanics/quests/staff-upgrade-02.md)
  records build, hashes, reproduction, functions/opcode anchors, ordered state
  transitions, comparison conditions and uncertainty.
- No original process control, live input, extraction rerun or new measurements.

CHANGES:
- Added one bounded findings record and this handoff only. No shared-document,
  production, neighbor edits, approved API, commits or push.

VALIDATION:
- Manually cross-checked insertion possession/undefined branch, death-credit filter,
  squared-distance comparison/ties, increment-before-notification, threshold/cheat
  grouping, completion loop exit and notify-before-pickup-grant in disassembly.
- Cross-checked unlock's weapon/clearance/final flag sequence, ordered external
  prerequisite flag waits and base ownership lookup against fresh views.
- Relative-link scan: zero missing targets. Privacy/source-text regex: no absolute
  drive paths, common secret markers, generated-source headers or opcode dumps;
  manual content review contains abstract semantics only. Git diff --check exits0;
  direct new-file trailing-whitespace check passes.
- No synthetic checker or original comparison executed. Cargo gates skipped for
  documentation-only change; source interpretation does not establish fidelity.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Effective loaded patches and actor-death dispatch, native helpers/trigger routing,
  exact map prerequisite flags, shared flag wake behavior, numeric/timing semantics,
  early trigger retention, concurrent input and disconnect/rollback behavior.
- Post-upgrade soul routing/recharge requires separate evidence; ammo capacities,
  firing/damage, full puzzle/quest completion and death transfer not established.

RECOMMENDED NEXT WORK:
- Bind one air gem's actual two struct prerequisites or trace one effective
  initial-charge callback consumer, then coordinate a selected original comparison.

SCOPE NOT IMPLEMENTED:
- Rust gameplay, full staff/quest, visuals/collision, weapon damage/firing, runtime
  launch/input, shared indexes/architecture/state, commits/push or nested delegation.
