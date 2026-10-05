TASK:
Investigate one shovel admission -> excavation consumption -> replenishment
transition using retained original script evidence, without runtime input.

STATUS:
COMPLETE

FINDINGS:
- Shovel capability belongs to one player; pickup removes the shared world item.
  Normal dig preserves shovel capability and globally depletes one mound before
  reward selection. Disconnect respawns a normal shovel at the pickup origin.
- Dig replenishment follows end-of-round plus a nominal source wait; shuffled
  candidate order, weather, roster-bounded draw, quota and count govern admission.
- Observed <=15 count guard permits a 16th mound in a serial immediate-increment
  model; emerging mounds count before native use triggers become ready.
- Snow disables ordinary refill but has a staff-zone exception. No fixed dig rate,
  exact reward probability or actual live cap was asserted.

EVIDENCE:
- [Excavation record](../reverse_engineering/origins-mechanics/world/excavation-02.md).
- Reviewed current status/diff, preceding handoff, project state and architecture;
  reused previously read Origins-analysis/reverse-engineering skill instructions.
- Reused external OAT patch extraction/listings and identity receipts; freshly
  hashed dig, map utility and shared unitrigger compiled inputs.
- Five bounded GSC Tool decompile/disassembly calls exited 0. Fresh outputs stay
  external in mechanics-world-02; record gives source/tool/output hashes.
- Cross-checked dig admission/consumption, event/wait, weather priority and <=
  bytecode comparison; examined shared trigger forwarding/deletion/assignment.
- No original-game measurement or process interaction occurred.

CHANGES:
- Created world/excavation-02.md and excavation_contract_checks.py plus this handoff.
- Proposed separate personal capability, world pickup availability and shared
  site lifecycle; no approved production abstraction or runtime implementation.

VALIDATION:
- Python synthetic proposal checks: 12 tests PASS.
- git diff --check PASS; scoped machine-path/proprietary-opcode scan has no matches.
- Relative record/handoff/evidence links resolve during the task check.
- No Rust change; Cargo gates not applicable to this research-only record.
- No live fidelity comparison. Tests do not prove native geometry, validity,
  scheduler ordering, original RNG, rewards or multiplayer exactly-once behavior.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Effective script precedence/overrides, native trigger geometry/look behavior,
  player-validity/life-state semantics, RNG range/distribution/seed, permutation
  algorithm, concurrency and actual spawn readiness timing/count ceiling.
- Staff-piece selection, reward helpers, snow exception lifecycle and death-state
  shovel persistence remain adjacent unknowns.

RECOMMENDED NEXT WORK:
- Runtime-owner-controlled comparison of the documented admission/depletion and
  refill boundary cases, including simultaneous use and count 15.
- Independent buildable inventory/assembly transition if more source-only world
  progress is needed; preserve damage-chat ownership of robot lethal contact.

SCOPE NOT IMPLEMENTED:
- Robot/buildable/tank behavior, reward table survey, staff quests, movement,
  death-state code, original mutation, broad extraction, production gameplay,
  shared indexes, commits or pushes.
