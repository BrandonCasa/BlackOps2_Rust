TASK:
Implement ordinary resolved coop revival begin/cancel/completion with stale
episode/attempt rejection, shared fixed ordering and qualified Origins restoration.

STATUS:
COMPLETE
Assigned lifecycle increment complete; full goal/original comparison incomplete.

FINDINGS:
- Cancel remains Downed; complete restores map-supplied current/max and Standing.
- Tokens bind target Entity generation, reviver, checked episode and attempt.
  Cancel/restart and redown reject old completions without health mutation.
- Command order is explicit across damage/revival, with no hidden death priority.
- Same-Entity component replacement is a caller admission boundary: retire pending
  commands/held tokens/episode references/old receipts before replacing Vitality.
  Entity generation alone does not prevent replacement counter-reset aliases.

EVIDENCE:
- Current state/status/architecture/Vitality/tests and handoffs03/04 reviewed;
  architecture-extraction skill applied.
- Steam-script completion/cancellation plus task04 qualified Plutonium native
  setter evidence supports Origins ordinary no-bonus restoration100/100.
- No original data, runtime health, UI or source inspection during this task.

CHANGES:
- Gameplay: one LifeCommandQueue/Results replaces damage-only API with no aliases;
  Damage/Begin/Cancel/Complete run in accepted ID order on FixedUpdate.
- Vitality adds private checked counters/active token, validated restoration and
  normal wrong-state/nonhuman/missing/stale/invalid-reviver/exhaustion errors.
- Explicit retire_pending_target returns pending inputs for caller retirement and
  preserves other targets/global request-ID admission history; no entity registry.
- Origins supplies qualified100/100 restoration independently of shared gameplay.
- Expanded demo verifies13 exact outcomes and intermediate Standing100/100 state.
- Migrated retained damage tests, new lifecycle/counter/configuration tests,
  updated docs/architecture/damage-life-state.md and this handoff.
- No manifest/dependency/lock/core changes; outer comparison documents untouched.

VALIDATION:
- Locked metadata exits0 (same10members); all95 workspace tests pass.
- Focused gameplay/Origins coverage totals15 tests, preserving previous damage
  cases and adding success/cancel/restart/redown, wrong/stale identity, actor,
  missing/despawned target/reviver, restore invariants, integer/counter maxima,
  atomic exhaustion and component-input retirement.
- Mixed replay repeated equivalence passes including0/1/3 fixed steps per update;
  ordered complete->damage->stale completion exercised across actual fixed steps.
- cargo fmt --all -- --check, cargo clippy --workspace --all-targets --locked
  -- -D warnings and git diff --check pass; no lint suppression or unsafe change.
- cargo run -p bo2-damage-life-state-demo --locked exits0, asserting ordered down,
  cancellation, stale attempt, restart, revive100, damage/redown, stale episode,
  actor death/repeat. Initial health/interaction/timing are explicit admitted inputs.
- Initial Clippy collapsible-if repaired directly; final checks pass.
- Synthetic correctness only. Outer original comparison trial remains PARTIAL;
  this worker performed no original comparison and claims no comparison success.

FIDELITY STATUS:
APPROXIMATE

UNRESOLVED:
- Live original agreement and loaded-runtime source identity; damage arithmetic
  remains inferred. Numeric restoration uses qualified mixed source/runtime evidence.
- Raw interaction/timing and same-frame original ordering; eligibility externally
  admitted, reviver unavailable at completion rejects and caller can cancel cleanup.
- Continuous-component lifetime requires input retirement before replacement;
  there is no automatic interception of arbitrary external ECS replacement.

RECOMMENDED NEXT WORK:
- Resolve controlled original comparison ownership/input issue, then compare.
- Separately investigate/implement bleedout/terminal human/session-end route.

SCOPE NOT IMPLEMENTED:
- Raw revive geometry/HUD/3-second timer, bleedout, regen/perks/solo, AI/weapons,
  original UI/health access, broad lifecycle framework, commits/pushes or delegation.
