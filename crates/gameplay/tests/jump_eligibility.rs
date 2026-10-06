use bo2_gameplay::movement::jump_eligibility::{
    JumpEligibilityDecision, JumpEligibilityError, JumpEligibilityInput, JumpEligibilityResult,
    JumpRejection, evaluate_jump_eligibility,
};

const BUTTON: u64 = 1 << 21;
const OTHER_BUTTONS: u64 = (1 << 63) | (1 << 42) | (1 << 30) | 17;

fn supplied() -> JumpEligibilityInput {
    // Synthetic inputs; no original profile defaults or owner storage inferred.
    JumpEligibilityInput {
        state_flags: 0,
        movement_mode: 0,
        effective_posture: 0,
        ground_owner: 7,
        surface_flags: 0,
        last_jump_time_ms: 1000,
        command_time_ms: 1500,
        current_buttons: BUTTON | OTHER_BUTTONS,
        previous_buttons: OTHER_BUTTONS,
        ground_permission: Some(true),
    }
}

fn rejection(input: JumpEligibilityInput, reason: JumpRejection, buttons: u64) {
    assert_eq!(
        evaluate_jump_eligibility(input),
        Ok(JumpEligibilityResult {
            current_buttons: buttons,
            decision: JumpEligibilityDecision::Rejected(reason),
        })
    );
}

#[test]
fn source_flow_fixtures_cover_launch_and_rejection() {
    // Independent native06 reconstruction: gates return false without writes,
    // except the previous-command held branch, which clears current low bit21.
    let fixtures = [
        (0, 500, 0, 0, 7, true, false, 0, true, None),
        (
            0,
            499,
            0,
            0,
            7,
            true,
            false,
            0,
            true,
            Some(JumpRejection::JumpInterval),
        ),
        (0, 501, 0, 0, 7, true, false, 0, true, None),
        (
            0,
            500,
            0,
            0,
            7,
            true,
            true,
            0,
            true,
            Some(JumpRejection::ButtonHeld),
        ),
        (
            0,
            499,
            0,
            0,
            7,
            true,
            true,
            0,
            true,
            Some(JumpRejection::JumpInterval),
        ),
        (
            0,
            500,
            0,
            0,
            7,
            false,
            false,
            0,
            true,
            Some(JumpRejection::ButtonAbsent),
        ),
        (
            0,
            500,
            10,
            0,
            7,
            true,
            false,
            0,
            true,
            Some(JumpRejection::MovementMode),
        ),
        (
            0,
            500,
            0,
            1,
            7,
            true,
            false,
            0,
            true,
            Some(JumpRejection::EffectivePosture),
        ),
        (0, 500, 0, 1, 0x3ff, true, false, 0, true, None),
        (
            0,
            500,
            0,
            0,
            7,
            true,
            false,
            2,
            true,
            Some(JumpRejection::SurfaceFlag2),
        ),
        (
            0,
            500,
            0,
            0,
            7,
            true,
            false,
            0,
            false,
            Some(JumpRejection::GroundPermission),
        ),
        (0, 500, 0, 0, 0x3ff, true, false, 0, false, None),
        (
            0x80000,
            500,
            0,
            0,
            7,
            true,
            false,
            0,
            true,
            Some(JumpRejection::StateFlag80000),
        ),
        (
            0x400,
            500,
            0,
            0,
            7,
            true,
            false,
            0,
            true,
            Some(JumpRejection::StateFlag400),
        ),
        (
            4,
            500,
            0,
            0,
            7,
            true,
            false,
            0,
            true,
            Some(JumpRejection::StateFlag4),
        ),
    ];
    for (flags, elapsed, mode, posture, owner, pressed, held, surface, permission, reason) in
        fixtures
    {
        let input = JumpEligibilityInput {
            state_flags: flags,
            command_time_ms: 1000 + elapsed,
            movement_mode: mode,
            effective_posture: posture,
            ground_owner: owner,
            current_buttons: OTHER_BUTTONS | if pressed { BUTTON } else { 0 },
            previous_buttons: OTHER_BUTTONS | if held { BUTTON } else { 0 },
            surface_flags: surface,
            ground_permission: Some(permission),
            ..supplied()
        };
        let expected_buttons = if reason == Some(JumpRejection::ButtonHeld) {
            OTHER_BUTTONS
        } else {
            input.current_buttons
        };
        assert_eq!(
            evaluate_jump_eligibility(input),
            Ok(JumpEligibilityResult {
                current_buttons: expected_buttons,
                decision: reason.map_or(
                    JumpEligibilityDecision::Eligible,
                    JumpEligibilityDecision::Rejected
                ),
            })
        );
    }
}

#[test]
fn held_button_is_cleared_only_after_all_earlier_gates() {
    let held = JumpEligibilityInput {
        previous_buttons: u64::MAX,
        ground_permission: None,
        ..supplied()
    };
    for (flags, now, mode, posture, reason) in [
        (0x80404, 1499, 10, 1, JumpRejection::StateFlag80000),
        (0x404, 1499, 10, 1, JumpRejection::JumpInterval),
        (0x404, 1500, 10, 1, JumpRejection::StateFlag400),
        (4, 1500, 10, 1, JumpRejection::StateFlag4),
        (0, 1500, 10, 1, JumpRejection::MovementMode),
        (0, 1500, 0, 1, JumpRejection::EffectivePosture),
    ] {
        let input = JumpEligibilityInput {
            state_flags: flags,
            command_time_ms: now,
            movement_mode: mode,
            effective_posture: posture,
            ..held
        };
        rejection(input, reason, input.current_buttons);
    }
    // Even a forbidden surface and unavailable permission occur after held clear.
    rejection(
        JumpEligibilityInput {
            surface_flags: 2,
            ..held
        },
        JumpRejection::ButtonHeld,
        OTHER_BUTTONS,
    );
    let absent = JumpEligibilityInput {
        current_buttons: OTHER_BUTTONS,
        ..held
    };
    rejection(absent, JumpRejection::ButtonAbsent, OTHER_BUTTONS);
}

#[test]
fn sentinel_bypasses_posture_and_permission_but_preserves_other_gates() {
    for posture in [i32::MIN, -1, 0, 1, 2, i32::MAX] {
        for permission in [None, Some(false), Some(true)] {
            let input = JumpEligibilityInput {
                effective_posture: posture,
                ground_owner: 0x3ff,
                ground_permission: permission,
                ..supplied()
            };
            assert_eq!(
                evaluate_jump_eligibility(input).unwrap().decision,
                JumpEligibilityDecision::Eligible
            );
            rejection(
                JumpEligibilityInput {
                    surface_flags: 2,
                    ..input
                },
                JumpRejection::SurfaceFlag2,
                input.current_buttons,
            );
        }
    }
}

#[test]
fn permission_is_requested_only_when_native_callback_gate_is_reached() {
    let missing = JumpEligibilityInput {
        ground_permission: None,
        ..supplied()
    };
    assert_eq!(
        evaluate_jump_eligibility(missing),
        Err(JumpEligibilityError::MissingGroundPermission)
    );
    rejection(
        JumpEligibilityInput {
            ground_permission: Some(false),
            ..missing
        },
        JumpRejection::GroundPermission,
        missing.current_buttons,
    );
    for owner in [0, 7, 0x3fe, 0x400, u32::MAX] {
        assert_eq!(
            evaluate_jump_eligibility(JumpEligibilityInput {
                ground_owner: owner,
                ..missing
            }),
            Err(JumpEligibilityError::MissingGroundPermission)
        );
    }
    rejection(
        JumpEligibilityInput {
            surface_flags: 2,
            ..missing
        },
        JumpRejection::SurfaceFlag2,
        missing.current_buttons,
    );
}

#[test]
fn timing_is_signed_and_exact_at_499_and_500() {
    for (last, now, expected) in [
        (1000, 1499, false),
        (1000, 1500, true),
        (1000, 1501, true),
        (-1000, -501, false),
        (-1000, -500, true),
        (0, -1, false),
        (i32::MAX - 500, i32::MAX, true),
        (i32::MIN, i32::MIN + 500, true),
    ] {
        let input = JumpEligibilityInput {
            last_jump_time_ms: last,
            command_time_ms: now,
            ..supplied()
        };
        assert_eq!(
            evaluate_jump_eligibility(input).unwrap().decision,
            if expected {
                JumpEligibilityDecision::Eligible
            } else {
                JumpEligibilityDecision::Rejected(JumpRejection::JumpInterval)
            }
        );
    }
}

#[test]
fn signed_mode_and_posture_values_are_not_normalized() {
    for mode in [i32::MIN, -1, 0, 9] {
        assert_eq!(
            evaluate_jump_eligibility(JumpEligibilityInput {
                movement_mode: mode,
                ..supplied()
            })
            .unwrap()
            .decision,
            JumpEligibilityDecision::Eligible
        );
    }
    for mode in [10, i32::MAX] {
        let input = JumpEligibilityInput {
            movement_mode: mode,
            ..supplied()
        };
        rejection(input, JumpRejection::MovementMode, input.current_buttons);
    }
    for posture in [i32::MIN, -1, 1, 2, i32::MAX] {
        let input = JumpEligibilityInput {
            effective_posture: posture,
            ..supplied()
        };
        rejection(
            input,
            JumpRejection::EffectivePosture,
            input.current_buttons,
        );
    }
}

#[test]
fn press_release_press_uses_previous_native_command_and_preserves_high_bits() {
    let initial = supplied();
    assert_eq!(
        evaluate_jump_eligibility(initial).unwrap().decision,
        JumpEligibilityDecision::Eligible
    );
    let held = JumpEligibilityInput {
        previous_buttons: initial.current_buttons,
        ..initial
    };
    rejection(held, JumpRejection::ButtonHeld, OTHER_BUTTONS);
    let released = JumpEligibilityInput {
        current_buttons: OTHER_BUTTONS,
        previous_buttons: held.current_buttons,
        ..initial
    };
    rejection(released, JumpRejection::ButtonAbsent, OTHER_BUTTONS);
    let repressed = JumpEligibilityInput {
        previous_buttons: released.current_buttons,
        ..initial
    };
    assert_eq!(
        evaluate_jump_eligibility(repressed)
            .unwrap()
            .current_buttons,
        BUTTON | OTHER_BUTTONS
    );
    assert_eq!(
        evaluate_jump_eligibility(repressed).unwrap().decision,
        JumpEligibilityDecision::Eligible
    );
}

#[test]
fn overflow_is_atomic_and_occurs_before_later_rejections() {
    for (last, now) in [(i32::MIN, i32::MAX), (i32::MAX, i32::MIN)] {
        let input = JumpEligibilityInput {
            last_jump_time_ms: last,
            command_time_ms: now,
            state_flags: 0x404,
            previous_buttons: u64::MAX,
            ground_permission: None,
            ..supplied()
        };
        let retained = input;
        assert_eq!(
            evaluate_jump_eligibility(input),
            Err(JumpEligibilityError::TimeOverflow)
        );
        assert_eq!(input, retained);
        rejection(
            JumpEligibilityInput {
                state_flags: 0x80000,
                ..input
            },
            JumpRejection::StateFlag80000,
            input.current_buttons,
        );
    }
    let input = JumpEligibilityInput {
        ground_permission: None,
        ..supplied()
    };
    let retained = input;
    assert_eq!(
        evaluate_jump_eligibility(input),
        Err(JumpEligibilityError::MissingGroundPermission)
    );
    assert_eq!(input, retained);
}
