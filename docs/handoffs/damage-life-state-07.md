TASK:
Implement ordinary coop resolved terminal transition, bound to current downed
episode and ordered with revival in the existing lifecycle queue.

STATUS:
COMPLETE
Bounded implementation and synthetic validation complete; original comparison pending.

FINDINGS:
- ExpireDowned binds private DownedExpiry target Entity generation and episode.
  Active attempt returns Deferred without mutation or automatic retry. Producer
  retains/retries after cancellation with a new increasing request ID.
- Successful revive retires expiry immediately; redown rejects its older episode.
  Accepted ordering controls complete/cancel/expiry with no hidden death priority.
- HumanTerminal preserves last qualified current/max, ignores damage/repeat expiry
  and rejects revival normally. Positive numeric health is not an alive predicate.
- Source network-frame interval is collapsed into resolved caller admission as
  explicit approximation; spectator-spawn health and raw timing remain unknown.

EVIDENCE:
- Current status/state, architecture, accepted handoffs05/06 and source/tests reviewed.
- Architecture-extraction and validation skills applied; existing Bevy0.19.1 fixed
  scheduling/query facilities reused, without new adapter or dependency.
- Source/native observations referenced in ../reverse_engineering/damage-life-state-06.md.
- No original data/process/UI access during this implementation.

CHANGES:
- crates/gameplay/src/lib.rs: explicit HumanTerminal, DownedExpiry/ExpiryOutcome,
  validated expire_downed and ExpireDowned in existing queue/system/results.
- New tests/terminal.rs plus maximum-value unit case preserve prior damage/revive
  coverage and existing checked counters/component-lifetime retirement contract.
- Demo now asserts 21 exact ordered outcomes and terminal current1/max100 alongside
  actor terminal behavior; architecture document updated.
- No Origins configuration, manifest/lock, core, project-state or comparison edits.

VALIDATION:
- cargo test -p bo2-gameplay --locked: all19 tests pass.
- cargo metadata --no-deps --format-version 1 --locked: exit0, same10members.
- cargo test --workspace --locked: exit0, all101 tests pass (prior95 preserved).
- cargo fmt --all -- --check and cargo clippy --workspace --all-targets --locked
  -- -D warnings: exit0. No lint suppression or unsafe-policy relaxation.
- cargo run -p bo2-damage-life-state-demo --locked: exit0; exact21 ordered outcomes
  plus intermediate revive100/100 and final human/actor state assertions pass.
- Deferral/cancel/retry, order permutations, stale/wrong/missing/despawned identity,
  unsupported player policy, actor rejection, positive terminal repeats, u32/u64
  maxima and pending-input retirement exercised. Mixed replay equivalence passes
  repeated runs and0/1/3 fixed steps per application update.
- git diff --check and permitted-content review pass; new tests/handoff inspected.
- Synthetic correctness only; no new original comparison or fidelity promotion.

FIDELITY STATUS:
APPROXIMATE

UNRESOLVED:
- Coop comparison, loaded-runtime script equivalence, native damage arithmetic,
  raw expiry timing and notification/network-frame race ordering.
- Numeric spectator-spawn health and engine alive predicate; retained positive
  health is preservation of qualified values, not a claim about final native health.
- Continuous component lifetime requires caller retirement of held expiry/revive
  tokens, pending requests and result receipts before same-Entity replacement.

RECOMMENDED NEXT WORK:
- Controlled multiplayer comparison of ordinary expiry, revive deferral and
  cancellation after countdown; separately qualify timer admission/ordering.
- Spectator/session or respawn work only as independently evidenced boundaries.

SCOPE NOT IMPLEMENTED:
- 45-second timer/tick count, HUD/raw revive eligibility, solo/session-end,
  spectator camera/respawn, regen/perks, AI/weapons, original UI/process writes,
  framework/dependencies, commits/pushes or nested delegation.
