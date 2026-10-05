TASK:
Implement episode-bound ordinary-coop bleedout timing admission with Bevy time and the existing ordered expiry queue.

STATUS:
COMPLETE

FINDINGS:
- BleedoutClock uses a Bevy once Timer, exact DownedExpiry and one in-flight request ID. Assembly supplies fixed delta, explicit producer order and global IDs; clock does not mutate Vitality.
- Countdown runs through active revival. Only matching Deferred acknowledgement starts the poll delay; cancellation does not reset countdown. Success/redown/terminal/missing lifecycle retires old admission. Queue/receipt errors and replacement retirement remain explicit.
- Whole-second positive sampled countdowns are admitted; zero/fractional countdown and zero poll are rejected. Origins supplies45s/100ms. No general timing framework or dependencies.

EVIDENCE:
- Reviewed source06: build24784239 default45, sampled-once one-second waits, post-countdown100ms revival polling, no cancellation restart. Runtime-script equivalence and live coop timing unqualified.
- Inspected Bevy0.19.1 Timer once/tick/remaining and Time<Fixed>/FixedTimesteps facilities, existing queue/API/tests and handoff19.

CHANGES:
- crates/gameplay/src/bleedout.rs and tests/bleedout.rs; minimal lib.rs exports.
- Origins src/lib.rs and tests/configuration.rs: source timing configuration.
- Demo: preserved21 assertions plus real fixed-time countdown/deferral/cancel/poll terminal scenario.
- Architecture boundary and this handoff.

VALIDATION:
- cargo test -p bo2-gameplay -p bo2-origins --locked:31 tests pass, exit0.
- cargo metadata --locked --no-deps --format-version 1:10 unchanged members, exit0.
- cargo test --workspace --locked:111 tests pass, exit0.
- cargo fmt --all -- --check:exit0.
- cargo clippy --workspace --all-targets --locked -- -D warnings:exit0.
- cargo run -p bo2-damage-life-state-demo --locked:exit0,21 prior outcomes plus timed assertions; no expiry before45s, Deferred at45s, ordered cancellation/poll terminal at45.1s and retirement.
- Tests cover boundaries, repeated polls/in-flight/repeated receipts, atomic queue rejection, revival/redown/identity/retirement, invalid timing and replay with different render pacing.
- Diff/content review and git diff --check pass. No unsafe, dependency, proprietary content or local asset-path additions. No live original comparison performed.

FIDELITY STATUS:
APPROXIMATE

UNRESOLVED:
- Ideal simulation timing, fixed-step quantization/acknowledgement latency and collapsed network-frame cleanup require multiplayer comparison. No original simulation rate inferred; demo100ms is diagnostic.

RECOMMENDED NEXT WORK:
- Controlled coop countdown/late-revive/cancel comparison from source06 procedure; retain separate spectator-health and actor-comparison questions.

SCOPE NOT IMPLEMENTED:
- Revive-progress timer/raw interaction, regen, solo/game-end/spectator policy, UI/native/observer work, commits or pushes.
