TASK:
Implement episode/attempt-bound ordinary-coop sampled revive timing through the existing ordered queue.

STATUS:
COMPLETE

FINDINGS:
- ReviveClock binds a checked token/restoration and Bevy once Timer. Eligibility is sampled before each admitted wait; standing reviver is checked after it, before threshold completion.
- Final-wait release is not continuously resampled; downed/absent/nonhuman reviver cancels before success. New attempt starts at0; oversized delta cannot speculate multiple waits or eligibility samples.
- One Cancel/Complete decision/request; exact ID/target/kind/restored-value acknowledgement; queue/receipt rejection atomic. Stale lifecycle and replacement input retirement explicit. No health mutation outside the existing queue.
- Origins3s/50ms is an ideal Duration adaptation.60 ideal waits here do not establish60 original floating-loop iterations.

EVIDENCE:
- Source03 sampling/float accumulator, native04 restoration and source06 active-revive deferral; prior clocks/queue/token tests and handoffs20/21.
- Reused Bevy0.19.1 Timer/Time<Fixed> facilities; no original access, native investigation or new dependencies.

CHANGES:
- crates/gameplay/src/revive_clock.rs and tests/revive_clock.rs; minimal lib.rs exports.
- Origins src/lib.rs and tests/configuration.rs: explicit ideal timing configuration.
- Demo: preserves21 resolved outcomes/elapsed bleedout case, adds actual timed restoration after final-wait release.
- Architecture boundary and this handoff. Parent-owned state/fidelity edits preserved and excluded from worker changes.

VALIDATION:
- cargo test -p bo2-gameplay -p bo2-origins --locked:39 tests pass, exit0.
- cargo metadata --locked --no-deps --format-version 1:10 unchanged members, exit0.
- cargo test --workspace --locked:119 tests pass, exit0.
- cargo fmt --all -- --check:exit0.
- cargo clippy --workspace --all-targets --locked -- -D warnings:exit0.
- cargo run -p bo2-damage-life-state-demo --locked:exit0;21 prior outcomes,45s/45.1s expiry case and ideal3s timed100/100 restoration/retirement all asserted.
- Tests include cancel/restart, final-wait release/downed reviver, overshoot, identity/receipt/value errors, invalid timing/restoration and deterministic mixed replay. Already-expired bleedout defers; assembly orders completion before same-step poll, which becomes StaleEpisode; both clocks retire under varied render pacing.
- Diff/content review and git diff --check pass. No proprietary content, local paths, unsafe/dependency additions. All operations terminal; no live coop comparison performed.
- Outer reviewed scope/API/evidence/tests/diff and independently repeated39 focused tests and the demo, exit0; original comparison remains separate.

FIDELITY STATUS:
APPROXIMATE

UNRESOLVED:
- Original float precision/iteration count, loaded-script equivalence, VM/fixed-step scheduling and live coop timing/completion/bleedout races.

RECOMMENDED NEXT WORK:
- Controlled ordinary coop progress/release/interruption/late-completion comparisons; separately qualify raw interaction admission before implementing it.

SCOPE NOT IMPLEMENTED:
- Geometry/tool/team/trace eligibility, auto/staff/perks, trigger initiation arbitration, UI/native/observer work, regen/session/spectator policy, commits or pushes.
