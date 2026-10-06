//! Native 53dc10 ordinary scalar SSE branch. Inputs are supplied explicitly;
//! this module supplies no game profile defaults or mode predicates.

use super::timing::SingleStepDuration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrdinaryAcceleration {
    /// Caller supplies the qualified direction; this kernel does not normalize it.
    pub direction: [f32; 3],
    pub wish_speed: f32,
    pub acceleration: f32,
    pub stop_speed: f32,
    pub duration: SingleStepDuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccelerationError {
    NonFiniteInput,
    /// Safe-boundary domain restriction, not a recovered dvar registration range.
    NegativeParameter,
    NonFiniteArithmetic,
}

impl std::fmt::Display for AccelerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AccelerationError {}

/// Returns a new authoritative velocity, or an error without modifying its input.
/// Eligibility for the native flag-bit-3-clear branch belongs to the caller.
pub fn accelerate_ordinary(
    velocity: [f32; 3],
    input: OrdinaryAcceleration,
) -> Result<[f32; 3], AccelerationError> {
    if velocity
        .into_iter()
        .chain(input.direction)
        .any(|x| !x.is_finite())
        || [input.wish_speed, input.acceleration, input.stop_speed]
            .into_iter()
            .any(|x| !x.is_finite())
    {
        return Err(AccelerationError::NonFiniteInput);
    }
    if input.wish_speed < 0.0 || input.acceleration < 0.0 || input.stop_speed < 0.0 {
        return Err(AccelerationError::NegativeParameter);
    }

    // Separate scalar products/additions preserve the recovered SSE rounding
    // boundaries. Do not replace these expressions with mul_add or reassociate.
    let x_product = input.direction[0] * velocity[0];
    let y_product = velocity[1] * input.direction[1];
    let xy_sum = x_product + y_product;
    let z_product = velocity[2] * input.direction[2];
    let projected = xy_sum + z_product;
    let available = input.wish_speed - projected;
    if [
        x_product, y_product, xy_sum, z_product, projected, available,
    ]
    .into_iter()
    .any(|x| !x.is_finite())
    {
        return Err(AccelerationError::NonFiniteArithmetic);
    }
    if available <= 0.0 {
        return Ok(velocity);
    }

    let time_acceleration = input.duration.seconds() * input.acceleration;
    // The native comparison chooses wish_speed on equality, including signed
    // zero. Generic float max/min need not preserve that operand choice.
    let control_speed = if input.stop_speed > input.wish_speed {
        input.stop_speed
    } else {
        input.wish_speed
    };
    let uncapped = time_acceleration * control_speed;
    if !time_acceleration.is_finite() || !uncapped.is_finite() {
        return Err(AccelerationError::NonFiniteArithmetic);
    }
    let amount = if uncapped > available {
        available
    } else {
        uncapped
    };
    let mut output = velocity;
    for axis in 0..3 {
        let increment = input.direction[axis] * amount;
        let component = increment + velocity[axis];
        if !increment.is_finite() || !component.is_finite() {
            return Err(AccelerationError::NonFiniteArithmetic);
        }
        output[axis] = component;
    }
    Ok(output)
}
