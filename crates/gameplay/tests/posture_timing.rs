use bo2_gameplay::movement::posture_timing::{
    Posture, PostureTimingError, ViewHeightProgress, classify_posture, view_height_duration_ms,
    view_height_timing,
};

#[test]
fn independent_classifier_fixtures_preserve_target_and_signed_dword_state() {
    // Native09 fixtures extended after rechecking dword load and signed jl.
    for (target, special, expected) in [
        (11, 0, Posture::Prone),
        (40, 0, Posture::Crouching),
        (60, 0, Posture::Standing), // Synthetic supplied target; no standing default.
        (22, 0, Posture::Prone),
        (22, 1, Posture::Crouching),
        (22, 2, Posture::Crouching),
        (22, 3, Posture::Standing),
        (22, 4, Posture::Standing),
        (22, -1, Posture::Crouching),
        (22, -256, Posture::Crouching),
        (22, 256, Posture::Standing),
        (22, i32::MIN, Posture::Crouching),
        (22, i32::MAX, Posture::Standing),
    ] {
        assert_eq!(classify_posture(target, special), expected);
    }
    for special in [i32::MIN, -1, 0, 1, 2, 3, 256, i32::MAX] {
        assert_eq!(classify_posture(11, special), Posture::Prone);
        assert_eq!(classify_posture(40, special), Posture::Crouching);
        for target in [i32::MIN, -1, 0, 21, 23, 41, i32::MAX] {
            assert_eq!(classify_posture(target, special), Posture::Standing);
        }
    }
}

#[test]
fn duration_preserves_all_zero_and_nonzero_direction_cases() {
    for direction in [i32::MIN, -256, -1, 0, 1, 256, i32::MAX] {
        assert_eq!(view_height_duration_ms(11, direction), 600);
        assert_eq!(
            view_height_duration_ms(40, direction),
            if direction == 0 { 400 } else { 200 }
        );
        for stage in [i32::MIN, -1, 0, 22, 39, 41, i32::MAX] {
            assert_eq!(view_height_duration_ms(stage, direction), 200);
        }
    }
}

#[test]
fn independent_progress_fixtures_cover_truncation_and_completion() {
    // Native09 independent integer-flow timing fixtures, with supplied timestamp1000.
    for (stage, direction, elapsed, expected_duration, expected_percent) in [
        (11, 0, -6, 600, 0),
        (11, 0, 0, 600, 0),
        (11, 0, 1, 600, 0),
        (11, 0, 5, 600, 0),
        (11, 0, 6, 600, 1),
        (11, 0, 599, 600, 99),
        (11, 0, 600, 600, 100),
        (11, 0, 601, 600, 100),
        (40, 0, 399, 400, 99),
        (40, 0, 400, 400, 100),
        (40, 1, 199, 200, 99),
        (40, 1, 200, 200, 100),
    ] {
        assert_eq!(
            view_height_timing(1000 + elapsed, 1000, stage, direction),
            Ok(Some(ViewHeightProgress {
                duration_ms: expected_duration,
                percent: expected_percent
            }))
        );
    }
}

#[test]
fn wide_integer_reference_checks_each_percentage_boundary() {
    // Independent mathematical reconstruction in i64; not the kernel's checked
    // i32 operation sequence. Supplied values stay within its admitted domain.
    for (stage, direction, duration) in
        [(11, 0, 600_i64), (40, 0, 400), (40, -1, 200), (22, 0, 200)]
    {
        for timestamp in [-1000, 1000] {
            for elapsed in -10..=duration as i32 + 10 {
                let expected = ((i64::from(elapsed) * 100) / duration).clamp(0, 100) as u8;
                assert_eq!(
                    view_height_timing(timestamp + elapsed, timestamp, stage, direction),
                    Ok(Some(ViewHeightProgress {
                        duration_ms: duration as u16,
                        percent: expected
                    }))
                );
            }
        }
    }
}

#[test]
fn zero_timestamp_skips_active_stage_arithmetic_even_at_extreme_times() {
    for now in [i32::MIN, -1, 0, 1, i32::MAX] {
        assert_eq!(view_height_timing(now, 0, 11, 0), Ok(None));
    }
    assert_eq!(
        view_height_timing(0, -1, 11, 0),
        Ok(Some(ViewHeightProgress {
            duration_ms: 600,
            percent: 0
        }))
    );
    assert_eq!(
        view_height_timing(0, 1, 11, 0),
        Ok(Some(ViewHeightProgress {
            duration_ms: 600,
            percent: 0
        }))
    );
    assert_eq!(
        view_height_timing(600, 1, 11, 0).unwrap().unwrap().percent,
        99
    );
    assert_eq!(
        view_height_timing(601, 1, 11, 0).unwrap().unwrap().percent,
        100
    );
}

#[test]
fn multiplication_overflow_is_rejected_before_saturation() {
    assert_eq!(
        view_height_timing(21_474_837, 1, 11, 0)
            .unwrap()
            .unwrap()
            .percent,
        100
    );
    assert_eq!(
        view_height_timing(-21_474_835, 1, 11, 0)
            .unwrap()
            .unwrap()
            .percent,
        0
    );
    for now in [21_474_838, -21_474_836] {
        assert_eq!(
            view_height_timing(now, 1, 11, 0),
            Err(PostureTimingError::TimeOverflow)
        );
    }
}

#[test]
fn subtraction_extremes_and_errors_leave_supplied_state_unchanged() {
    for (now, timestamp) in [
        (i32::MIN, 1),
        (i32::MAX, -1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
    ] {
        let authoritative_progress = ViewHeightProgress {
            duration_ms: 400,
            percent: 25,
        };
        let mut applied_progress = authoritative_progress;
        let result = view_height_timing(now, timestamp, 40, 0);
        if let Ok(Some(progress)) = result {
            applied_progress = progress;
        }
        assert_eq!(result, Err(PostureTimingError::TimeOverflow));
        assert_eq!(applied_progress, authoritative_progress);
    }
    for equal_time in [i32::MIN, -1, 1, i32::MAX] {
        assert_eq!(
            view_height_timing(equal_time, equal_time, 40, 0),
            Ok(Some(ViewHeightProgress {
                duration_ms: 400,
                percent: 0
            }))
        );
    }
}
