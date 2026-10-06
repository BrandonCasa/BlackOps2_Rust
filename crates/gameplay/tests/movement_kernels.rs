use bo2_gameplay::movement::{
    acceleration::{AccelerationError, OrdinaryAcceleration, accelerate_ordinary},
    timing::{CommandTiming, SingleStepDuration, TimingError},
};

fn acceleration() -> OrdinaryAcceleration {
    OrdinaryAcceleration {
        direction: [1.0, 0.0, 0.0],
        wish_speed: 100.0,
        acceleration: 9.0,
        stop_speed: 50.0,
        duration: SingleStepDuration::from_elapsed_ms(10),
    }
}

fn bits(vector: [f32; 3]) -> [u32; 3] {
    vector.map(f32::to_bits)
}

#[test]
fn explicit_substep_boundaries_and_discarded_backlog() {
    for (gap, discarded, durations) in [
        (0, 0, vec![]),
        (1, 0, vec![1]),
        (66, 0, vec![66]),
        (67, 0, vec![66, 1]),
        (132, 0, vec![66, 66]),
        (150, 0, vec![66, 66, 18]),
        (151, 1, vec![66, 66, 18]),
        (1000, 850, vec![66, 66, 18]),
    ] {
        let plan = CommandTiming::new(1000, 1000 + gap).unwrap();
        assert_eq!(plan.discarded_ms(), discarded);
        assert_eq!(plan.initial_time_ms(), 1000 + discarded as i32);
        assert_eq!(plan.final_time_ms(), 1000 + gap);
        let steps: Vec<_> = plan.substeps().collect();
        assert_eq!(
            steps
                .iter()
                .map(|s| s.duration.milliseconds())
                .collect::<Vec<_>>(),
            durations
        );
        let mut previous = plan.initial_time_ms();
        for step in steps {
            assert_eq!(
                step.end_time_ms - previous,
                i32::from(step.duration.milliseconds())
            );
            previous = step.end_time_ms;
        }
        assert_eq!(previous, plan.final_time_ms());
    }
}

#[test]
fn backwards_command_returns_unchanged_time_even_at_signed_extremes() {
    for (current, requested) in [(1000, 999), (-10, -11), (i32::MAX, i32::MIN)] {
        let plan = CommandTiming::new(current, requested).unwrap();
        assert!(plan.ignored_backwards());
        assert_eq!(plan.initial_time_ms(), current);
        assert_eq!(plan.final_time_ms(), current);
        assert_eq!(plan.discarded_ms(), 0);
        assert_eq!(plan.substeps().count(), 0);
    }
}

#[test]
fn unqualified_wrapping_domain_is_rejected() {
    assert_eq!(
        CommandTiming::new(i32::MAX, i32::MAX),
        Err(TimingError::TimeOverflow)
    );
    assert_eq!(
        CommandTiming::new(i32::MIN, i32::MAX),
        Err(TimingError::TimeOverflow)
    );
    let edge = CommandTiming::new(i32::MAX - 150, i32::MAX).unwrap();
    assert_eq!(edge.substeps().last().unwrap().end_time_ms, i32::MAX);
}

#[test]
fn iterator_conserves_qualified_elapsed_time_and_stays_exhausted() {
    for gap in 0..=1000 {
        let plan = CommandTiming::new(-500, -500 + gap).unwrap();
        let mut steps = plan.substeps();
        let original_count = steps.len();
        let mut elapsed = 0;
        let mut count = 0;
        while let Some(step) = steps.next() {
            assert!((1..=66).contains(&step.duration.milliseconds()));
            elapsed += u32::from(step.duration.milliseconds());
            count += 1;
            assert_eq!(steps.len(), original_count - count);
        }
        assert!(count <= 3);
        assert_eq!(elapsed + plan.discarded_ms(), gap as u32);
        assert_eq!(steps.next(), None);
        assert_eq!(steps.next(), None);
    }
}

#[test]
fn single_step_clamp_and_native_seconds_rounding_are_distinct_from_regular_chunks() {
    for (elapsed, milliseconds, seconds_bits) in [
        (i32::MIN, 1, 0x3a83_126f),
        (0, 1, 0x3a83_126f),
        (10, 10, 0x3c23_d70b),
        (66, 66, 0x3d87_2b02),
        (200, 200, 0x3e4c_cccd),
        (i32::MAX, 200, 0x3e4c_cccd),
    ] {
        let duration = SingleStepDuration::from_elapsed_ms(elapsed);
        assert_eq!(duration.milliseconds(), milliseconds);
        assert_eq!(duration.seconds().to_bits(), seconds_bits);
    }
}

#[test]
fn acceleration_starts_saturates_and_preserves_already_sufficient_velocity() {
    let input = acceleration();
    assert_eq!(
        accelerate_ordinary([0.0; 3], input).unwrap(),
        [9.0, 0.0, 0.0]
    );
    assert_eq!(
        accelerate_ordinary([99.0, 0.0, 0.0], input).unwrap(),
        [100.0, 0.0, 0.0]
    );
    let already = [110.0, -0.0, 1.5];
    assert_eq!(
        bits(accelerate_ordinary(already, input).unwrap()),
        bits(already)
    );
    let zero_acceleration = OrdinaryAcceleration {
        acceleration: 0.0,
        ..input
    };
    assert_eq!(
        accelerate_ordinary([4.0, 5.0, 6.0], zero_acceleration).unwrap(),
        [4.0, 5.0, 6.0]
    );
}

#[test]
fn supplied_stop_speed_controls_acceleration_without_becoming_a_default() {
    let input = OrdinaryAcceleration {
        stop_speed: 200.0,
        ..acceleration()
    };
    assert_eq!(
        accelerate_ordinary([0.0; 3], input).unwrap(),
        [18.0, 0.0, 0.0]
    );
}

// Expected bits were independently reconstructed with IEEE-f32 stores between
// each recovered scalar operation, outside Rust. These are synthetic inputs.
#[test]
fn heterogeneous_velocity_matches_independent_scalar_reference_bits() {
    let input = OrdinaryAcceleration {
        direction: [0.3, 0.4, 0.5],
        wish_speed: 120.125,
        acceleration: 9.25,
        stop_speed: 200.5,
        duration: SingleStepDuration::from_elapsed_ms(17),
    };
    let actual = accelerate_ordinary([1.234_567, -0.987_654, 0.456_789], input).unwrap();
    assert_eq!(bits(actual), [0x412b_1729, 0x4139_fb12, 0x4181_c4d1]);
}

#[test]
fn dot_product_keeps_native_addition_order_without_direction_normalization() {
    let input = OrdinaryAcceleration {
        direction: [1.0; 3],
        wish_speed: 0.5,
        ..acceleration()
    };
    let actual = accelerate_ordinary([16_777_216.0, 1.0, -16_777_216.0], input).unwrap();
    assert_eq!(bits(actual), [0x4b80_0000, 0x3fc0_0000, 0xcb80_0000]);
}

#[test]
fn velocity_update_keeps_product_and_add_rounding_separate() {
    let input = OrdinaryAcceleration {
        direction: [0x3e8a_50f7, 0x3f53_acfc, 0x3c4a_dd2b].map(f32::from_bits),
        duration: SingleStepDuration::from_elapsed_ms(17),
        stop_speed: 100.0,
        ..acceleration()
    };
    let velocity = [0x3f84_ad8e, 0xc00d_a289, 0xc115_3f42].map(f32::from_bits);
    let actual = accelerate_ordinary(velocity, input).unwrap();
    assert_eq!(bits(actual), [0x40a5_6f36, 0x4127_0196, 0xc112_374e]);
    // Fusing the final product/add instead produces first-component 40a56f37.
}

#[test]
fn equal_zero_control_selects_native_wish_speed_operand() {
    let input = OrdinaryAcceleration {
        direction: [1.0; 3],
        wish_speed: -0.0,
        stop_speed: 0.0,
        ..acceleration()
    };
    let actual = accelerate_ordinary([-0.0, -1.0, -0.0], input).unwrap();
    assert_eq!(bits(actual), [0x8000_0000, 0xbf80_0000, 0x8000_0000]);
}

#[test]
fn invalid_inputs_and_overflow_fail_atomically() {
    let velocity = [1.0, 2.0, 3.0];
    let saved = bits(velocity);
    for input in [
        OrdinaryAcceleration {
            wish_speed: f32::NAN,
            ..acceleration()
        },
        OrdinaryAcceleration {
            direction: [f32::INFINITY, 0.0, 0.0],
            ..acceleration()
        },
    ] {
        assert_eq!(
            accelerate_ordinary(velocity, input),
            Err(AccelerationError::NonFiniteInput)
        );
    }
    for input in [
        OrdinaryAcceleration {
            wish_speed: -1.0,
            ..acceleration()
        },
        OrdinaryAcceleration {
            acceleration: -1.0,
            ..acceleration()
        },
        OrdinaryAcceleration {
            stop_speed: -1.0,
            ..acceleration()
        },
    ] {
        assert_eq!(
            accelerate_ordinary(velocity, input),
            Err(AccelerationError::NegativeParameter)
        );
    }
    let overflow = OrdinaryAcceleration {
        acceleration: f32::MAX,
        stop_speed: f32::MAX,
        ..acceleration()
    };
    assert_eq!(
        accelerate_ordinary(velocity, overflow),
        Err(AccelerationError::NonFiniteArithmetic)
    );
    let dot_overflow = OrdinaryAcceleration {
        direction: [f32::MAX, 0.0, 0.0],
        ..acceleration()
    };
    assert_eq!(
        accelerate_ordinary([2.0, 0.0, 0.0], dot_overflow),
        Err(AccelerationError::NonFiniteArithmetic)
    );
    let output_overflow = OrdinaryAcceleration {
        direction: [1.0, 1.0, 0.0],
        wish_speed: f32::MAX,
        acceleration: 1.0,
        duration: SingleStepDuration::from_elapsed_ms(200),
        ..acceleration()
    };
    assert_eq!(
        accelerate_ordinary([f32::MAX, -f32::MAX, 0.0], output_overflow),
        Err(AccelerationError::NonFiniteArithmetic)
    );
    assert_eq!(
        accelerate_ordinary([f32::NAN, 0.0, 0.0], acceleration()),
        Err(AccelerationError::NonFiniteInput)
    );
    assert_eq!(bits(velocity), saved);
}
