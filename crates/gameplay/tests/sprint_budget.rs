use bo2_gameplay::movement::sprint_budget::{
    SprintBudgetError, SprintBudgetParameters, SprintBudgetState, SprintStopDelay,
    available_sprint_budget, exceeds_sprint_restart_minimum, stop_sprint,
};

fn state(start: i32, stop: i32, stored: i32, exhausted: bool) -> SprintBudgetState {
    SprintBudgetState {
        active: true,
        restart_latched: false,
        exhausted,
        start_time_ms: start,
        stop_time_ms: stop,
        remaining_at_start_ms: stored,
    }
}

fn parameters() -> SprintBudgetParameters {
    // Synthetic supplied inputs; the module does not supply a profile.
    SprintBudgetParameters {
        maximum_duration_ms: 4000,
        cooldown_ms: 0,
    }
}

#[test]
fn authoritative_budget_fixtures_cover_debit_recharge_and_equal_times() {
    // Independently reconstructed native07 fixtures, not native invocation.
    for (start, stop, stored, now, exhausted, expected) in [
        (0, 0, 0, 100, false, 4000),
        (100, 0, 4000, 100, false, 4000),
        (100, 0, 4000, 101, false, 3999),
        (100, 0, 4000, 3099, false, 1001),
        (100, 0, 4000, 3100, false, 1000),
        (100, 0, 4000, 4100, false, 0),
        (100, 0, 4000, 4101, false, 0),
        (100, 1100, 4000, 1100, false, 3000),
        (100, 1100, 4000, 1101, false, 3001),
        (100, 1100, 4000, 2100, false, 4000),
        (100, 4100, 4000, 4100, true, 0),
        (100, 4100, 4000, 4101, true, 1),
        (100, 4100, 4000, 5100, true, 1000),
        (100, 4100, 4000, 5101, true, 1001),
        (100, 100, 4000, 100, false, 4000),
        (100, 1900, 4000, 2100, false, 2400),
    ] {
        assert_eq!(
            available_sprint_budget(state(start, stop, stored, exhausted), parameters(), now),
            Ok(expected),
            "start={start}, stop={stop}, now={now}, exhausted={exhausted}"
        );
    }
}

#[test]
fn restart_minimum_is_strict_and_cooldown_changes_exhausted_recharge() {
    for remaining in [i32::MIN, -1, 0, 999, 1000] {
        assert!(!exceeds_sprint_restart_minimum(remaining));
    }
    for remaining in [1001, 4000, i32::MAX] {
        assert!(exceeds_sprint_restart_minimum(remaining));
    }
    let input = state(100, 4100, 4000, true);
    let config = SprintBudgetParameters {
        cooldown_ms: 500,
        ..parameters()
    };
    for (now, expected) in [(4100, 0), (4599, 0), (4600, 0), (4601, 1), (5601, 1001)] {
        assert_eq!(available_sprint_budget(input, config, now), Ok(expected));
    }
    // Non-exhausted recharge ignores cooldown entirely.
    assert_eq!(
        available_sprint_budget(
            SprintBudgetState {
                exhausted: false,
                ..input
            },
            config,
            4101
        ),
        Ok(1)
    );
}

#[test]
fn zero_start_bypasses_arithmetic_and_raw_signed_caps_are_preserved() {
    let input = state(0, i32::MIN, i32::MAX, true);
    for maximum in [i32::MIN, -1, 0, 4000, i32::MAX] {
        let config = SprintBudgetParameters {
            maximum_duration_ms: maximum,
            cooldown_ms: i32::MAX,
        };
        assert_eq!(
            available_sprint_budget(input, config, i32::MAX),
            Ok(maximum)
        );
    }
    for now in [-100, 100, 10000] {
        assert_eq!(
            available_sprint_budget(
                state(1, 0, 100, false),
                SprintBudgetParameters {
                    maximum_duration_ms: -7,
                    cooldown_ms: 0,
                },
                now
            ),
            Ok(-7)
        );
    }
}

#[test]
fn timestamp_branch_and_budget_do_not_infer_active_or_latch() {
    let input = state(100, 200, 4000, true);
    assert_eq!(available_sprint_budget(input, parameters(), 201), Ok(3901));
    assert_eq!(
        available_sprint_budget(
            SprintBudgetState {
                active: false,
                restart_latched: true,
                ..input
            },
            parameters(),
            201
        ),
        Ok(3901)
    );
    // Equal start and stop selects recharge and reads exhaustion cooldown.
    assert_eq!(
        available_sprint_budget(
            state(100, 100, 4000, true),
            SprintBudgetParameters {
                cooldown_ms: 500,
                ..parameters()
            },
            101
        ),
        Ok(3501)
    );
}

#[test]
fn bounded_budget_grid_matches_independent_elapsed_time_accounting() {
    for start in [-1000, -1, 0, 1, 1000] {
        for stop in [-1000, -1, 0, 1, 1000] {
            for stored in [-500, 0, 1000, 4000] {
                for now in [-1000, 0, 1000, 5000] {
                    for exhausted in [false, true] {
                        for cooldown in [-500, 0, 500] {
                            for maximum in [-1, 0, 1000, 4000] {
                                // Independent i64 accounting: subtract running
                                // elapsed time, then add time since stopping.
                                let expected = if start == 0 {
                                    i64::from(maximum)
                                } else {
                                    let run_end = if start > stop { now } else { stop };
                                    let running = i64::from(run_end) - i64::from(start);
                                    let recharged = if start > stop {
                                        0
                                    } else {
                                        i64::from(now)
                                            - i64::from(stop)
                                            - if exhausted { i64::from(cooldown) } else { 0 }
                                    };
                                    (i64::from(stored) - running + recharged)
                                        .max(0)
                                        .min(i64::from(maximum))
                                };
                                assert_eq!(
                                    available_sprint_budget(
                                        state(start, stop, stored, exhausted),
                                        SprintBudgetParameters {
                                            maximum_duration_ms: maximum,
                                            cooldown_ms: cooldown
                                        },
                                        now,
                                    ),
                                    Ok(expected as i32)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn source_ordered_overflows_are_errors_even_when_algebra_could_cancel() {
    let config = parameters();
    for (input, now, cooldown) in [
        // Running subtract overflow, though adding start could restore the range.
        (state(2, 0, i32::MIN, false), 1, 0),
        (state(1, 0, i32::MAX, false), 0, 0),
        // Exhausted: negate, double, cooldown subtract, stored/start/now adds.
        (state(i32::MIN, i32::MIN, 0, true), 0, 0),
        (state(1, 1_100_000_000, i32::MAX, true), 1_100_000_000, 0),
        (state(1, 1, 0, true), 0, i32::MAX),
        (state(-1, -1, i32::MAX, true), 0, 0),
        (state(-1, 0, i32::MIN, true), 0, 0),
        (state(1, 1, i32::MAX, true), 2, 0),
        // Non-exhausted: doubled stop, stored subtract, start/now adds.
        (state(1, i32::MAX, 0, false), 0, 0),
        (state(1, 1, i32::MIN, false), 0, 0),
        (state(-1, 0, i32::MIN, false), 0, 0),
        (state(1, 1, i32::MAX, false), 2, 0),
    ] {
        assert_eq!(
            available_sprint_budget(
                input,
                SprintBudgetParameters {
                    cooldown_ms: cooldown,
                    ..config
                },
                now
            ),
            Err(SprintBudgetError::TimeOverflow)
        );
    }
}

#[test]
fn stop_preserves_untouched_fields_and_only_sets_a_held_restart_latch() {
    for latched in [false, true] {
        for pressed in [false, true] {
            let input = SprintBudgetState {
                restart_latched: latched,
                ..state(100, 0, 3000, true)
            };
            let result = stop_sprint(input, 500, pressed, SprintStopDelay::None).unwrap();
            assert_eq!(
                result,
                SprintBudgetState {
                    active: false,
                    restart_latched: latched || pressed,
                    exhausted: false,
                    stop_time_ms: 500,
                    ..input
                }
            );
        }
    }
}

#[test]
fn inactive_stop_is_a_no_op_even_with_extreme_time_and_delay() {
    let input = SprintBudgetState {
        active: false,
        restart_latched: true,
        ..state(123, 456, 789, true)
    };
    assert_eq!(
        stop_sprint(input, i32::MAX, true, SprintStopDelay::JumpInterrupt),
        Ok(input)
    );
}

#[test]
fn qualified_jump_interrupt_delays_stop_and_recharge_by_source_equation() {
    let input = state(100, 0, 4000, false);
    let stopped = stop_sprint(input, 1100, true, SprintStopDelay::JumpInterrupt).unwrap();
    assert_eq!(stopped.stop_time_ms, 1900);
    assert!(stopped.restart_latched);
    // Do not independently clamp future stop time to current time.
    assert_eq!(
        available_sprint_budget(stopped, parameters(), 1100),
        Ok(1400)
    );
    assert_eq!(
        available_sprint_budget(stopped, parameters(), 2100),
        Ok(2400)
    );
    assert_eq!(
        stop_sprint(input, i32::MAX - 800, false, SprintStopDelay::JumpInterrupt)
            .unwrap()
            .stop_time_ms,
        i32::MAX
    );
}

#[test]
fn stop_and_budget_failures_leave_authoritative_state_unchanged() {
    let mut authoritative = state(100, 0, 4000, true);
    let before = authoritative;
    let result = stop_sprint(
        authoritative,
        i32::MAX - 799,
        true,
        SprintStopDelay::JumpInterrupt,
    );
    assert_eq!(result, Err(SprintBudgetError::TimeOverflow));
    if let Ok(updated) = result {
        authoritative = updated;
    }
    assert_eq!(authoritative, before);
    let invalid_budget = state(1, i32::MAX, 0, true);
    assert_eq!(
        available_sprint_budget(invalid_budget, parameters(), 0),
        Err(SprintBudgetError::TimeOverflow)
    );
    assert_eq!(invalid_budget, state(1, i32::MAX, 0, true));
    // Without delay, the same extreme command time is a valid source write.
    assert_eq!(
        stop_sprint(authoritative, i32::MAX, true, SprintStopDelay::None)
            .unwrap()
            .stop_time_ms,
        i32::MAX
    );
}
