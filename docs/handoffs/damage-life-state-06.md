TASK:
Establish ordinary coop downed-player bleedout expiry and terminal disposition.

STATUS:
COMPLETE
Bounded source contract resolved; terminal numeric/native predicate limits explicit.

FINDINGS:
- Shared source default 45 seconds; delay sampled on downing. Counter subtracts
  before each 1 second wait; expiry follows final wait, not first displayed 0.
- Active revival defers only post-countdown expiry; cancellation does not reset
  bleedout countdown. Successful player_revived notification cancels the thread.
- Notify bled_out precedes network-frame wait/cleanup; simultaneous revival ordering
  remains unqualified. Ordinary default callback routes to spectator, not actor AI.
- No source health0 assignment. Qualified undolaststand has no direct health-field
  store; helper/native spawn effects and final current/max/alive predicate unresolved.

EVIDENCE:
- State/architecture, handoffs03/04/05, evidence03, actual Vitality/API/tests reviewed;
  bo2-reverse-engineering skill applied.
- Two Steam build24784239 compiled buffers freshly decoded in both tool modes;
  relevant Origins/standard evidence reused. External task06 identity receipt.
- Read-only existing Plutonium process registration rediscovery and exact 145-byte
  native span, executable-page checks and unchanged repetition; no UI or invocation.
- Details: ../reverse_engineering/damage-life-state-06.md.

CHANGES:
- Findings and this handoff only; raw decoded content/helpers/receipts stay external.
- Recommend episode-bound resolved terminal command in existing ordered queue,
  separate expiry admission, active-attempt deferral and spectator assembly.
- Preserve qualified numeric health pending spectator-health evidence; terminal
  status must remain non-revivable independently of positive stored health.

VALIDATION:
- Fresh 2 decompilations / 2 disassemblies succeed; original and compiled hashes unchanged.
- Native name/registration/range/page/repeated span and malformed-entry checks pass.
- Permitted-content/path review and git diff --check pass; unchanged Cargo not rerun.
- No live multiplayer timing/health/terminal comparison. Solo is excluded by user
  admission constraint and does not establish coop evidence.

FIDELITY STATUS:
INVESTIGATING

UNRESOLVED:
- Loaded Plutonium script identity/equivalence; coop timing and notification races.
- Final health/max and native alive predicate after spectator spawn; internal
  unnamed helper effects are not fully traced. Do not invent terminal health0.

RECOMMENDED NEXT WORK:
- Implement bounded resolved coop terminal transition with explicit health/state
  independence, deferred expiry and stale-episode rejection, then compare in coop.
- Qualify spectator numeric/native predicates only if next implementation needs them.

SCOPE NOT IMPLEMENTED:
- Gameplay/timer, solo/session-end, round/respawn, regen/perks, UI/console, original
  mutation/injection, weapons/AI/geometry, framework/dependencies, commits or agents.
