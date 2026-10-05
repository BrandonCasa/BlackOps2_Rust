TASK:
Implement the reviewed resolved-damage/health boundary with player downing versus
ordinary actor terminal routing and an Origins-configured headless demonstration.

STATUS:
COMPLETE
Assigned scoped implementation and synthetic validation complete; full goal and
original-game comparison remain incomplete.

FINDINGS:
- Private-invariant Bevy Vitality component preserves current/max distinction and
  positive-health human downing independently of ordinary actor zero-health death.
- Explicit positive integer admission and strict request IDs define bounded input
  ordering. Unsupported solo/last-player lethal policy rejects without mutation.
- Native player/actor subtraction and actor zero-clamping remain inferred; no
  original numeric conversion, spawn/round state or raw-impact fidelity claimed.

EVIDENCE:
- Reviewed damage-life-state-01 findings/handoff, current state/architecture/core,
  workspace graph and architecture-extraction skill.
- Pinned Bevy0.19.1 FixedUpdate, TimeUpdateStrategy, query/entity and schedule
  facilities inspected locally before using existing engine ownership.
- No original game/data access during this implementation task.

CHANGES:
- New bo2-gameplay crate: validated values, composed life route/state, normal
  errors/outcomes, ordered input/output resources and FixedUpdate plugin/system.
- New bo2-origins configuration crate: observed max100/downed1/fallback150 with
  explicit caller initial health and evidence caveats.
- New bo2-damage-life-state-demo executable: actual fixed-loop application and
  exact ordered outcomes/end-state validation, no runtime original assets.
- Focused gameplay/configuration tests; docs/architecture/damage-life-state.md.
- Cargo workspace now10 members; lockfile adds only3 local package entries.
  No existing crate behavior, dependency versions or unsafe policies changed.

VALIDATION:
- cargo metadata --locked --no-deps --format-version1 exits0,10 members.
- cargo test -p bo2-gameplay -p bo2-origins --locked:6 tests pass.
- cargo test --workspace --locked:86 tests pass, zero failures/ignored.
- cargo fmt --all -- --check and cargo clippy --workspace --all-targets --locked
  -- -D warnings exit0; git diff --check passes.
- cargo run -p bo2-damage-life-state-demo --locked exits0; ordered outcomes are
  player100->40->downed1, actor150->50->dead0, then ignored repeats.
- Tests cover h-1/h/h+1, maximumu32 values, invalid health/damage, atomic unresolved
  policy, repeats, missing/despawned targets, duplicate/reordered admission,
  actual Bevy fixed schedule and replay equivalence under changed entity order
  and0/1/3 fixed steps per update.
- Initial pacing test revealed Bevy clock initialization has zero elapsed time;
  explicit zero-step initialization fixes the demo/replay harness. Initial Clippy
  rejected a complex test tuple; replaced with named ReplaySnapshot, no suppression.
- All results are synthetic correctness/engine integration, not original comparison.

FIDELITY STATUS:
APPROXIMATE

UNRESOLVED:
- Original comparisons/native arithmetic and conversion; teammate eligibility
  remains caller admission. Queue producers must define explicit schedule order.
- Revival/bleedout, raw impact policy and solo/last-player game-end route.
- Queue/results have caller-drained storage, no inferred capacity/drop policy.

RECOMMENDED NEXT WORK:
- Qualify native arithmetic with original comparisons, then separately investigate
  and implement revival/bleedout before claiming the full life-state goal complete.

SCOPE NOT IMPLEMENTED:
- Revival, regen, bleedout, weapons/raw impacts/shield/same-attacker filters, AI,
  navigation, rounds, appearance, original launch/comparison, commits or pushes.
