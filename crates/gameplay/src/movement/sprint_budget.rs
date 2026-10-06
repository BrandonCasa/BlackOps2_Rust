//! Source-qualified native 068770 budget and 540c10/541ef0 stop writes.
//!
//! Parameters and eligibility are supplied explicitly. This module does not
//! implement sprint start predicates, weapon conversion, speed or a full motor.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SprintBudgetState {
    pub active: bool,
    pub restart_latched: bool,
    pub exhausted: bool,
    pub start_time_ms: i32,
    pub stop_time_ms: i32,
    pub remaining_at_start_ms: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SprintBudgetParameters {
    /// Supplied native duration; this boundary adds no profile defaults or ranges.
    pub maximum_duration_ms: i32,
    pub cooldown_ms: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprintBudgetError {
    /// A source-ordered signed intermediate leaves the qualified non-wrapping domain.
    TimeOverflow,
}

impl std::fmt::Display for SprintBudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SprintBudgetError {}

/// Computes available budget without inferring active state from timestamps.
///
/// Native 068770 handles the zero-start sentinel before arithmetic, then clamps
/// negative remaining time to zero before applying the supplied upper cap. A
/// negative supplied maximum therefore remains negative; the full sprint owner
/// separately stops for nonpositive duration. Signed overflow rejection is a
/// safe-boundary policy, not a claim about native wraparound behavior.
pub fn available_sprint_budget(
    state: SprintBudgetState,
    parameters: SprintBudgetParameters,
    now_ms: i32,
) -> Result<i32, SprintBudgetError> {
    if state.start_time_ms == 0 {
        return Ok(parameters.maximum_duration_ms);
    }
    let overflow = SprintBudgetError::TimeOverflow;
    let remaining = if state.start_time_ms > state.stop_time_ms {
        state
            .remaining_at_start_ms
            .checked_sub(now_ms)
            .ok_or(overflow)?
            .checked_add(state.start_time_ms)
            .ok_or(overflow)?
    } else if state.exhausted {
        // Native negates stop, doubles that result, then subtracts cooldown.
        let negative_stop = state.stop_time_ms.checked_neg().ok_or(overflow)?;
        negative_stop
            .checked_add(negative_stop)
            .ok_or(overflow)?
            .checked_sub(parameters.cooldown_ms)
            .ok_or(overflow)?
            .checked_add(state.remaining_at_start_ms)
            .ok_or(overflow)?
            .checked_add(state.start_time_ms)
            .ok_or(overflow)?
            .checked_add(now_ms)
            .ok_or(overflow)?
    } else {
        // This branch doubles positive stop before subtracting it from budget.
        let doubled_stop = state
            .stop_time_ms
            .checked_add(state.stop_time_ms)
            .ok_or(overflow)?;
        state
            .remaining_at_start_ms
            .checked_sub(doubled_stop)
            .ok_or(overflow)?
            .checked_add(state.start_time_ms)
            .ok_or(overflow)?
            .checked_add(now_ms)
            .ok_or(overflow)?
    };
    let nonnegative = if remaining < 0 { 0 } else { remaining };
    Ok(if nonnegative > parameters.maximum_duration_ms {
        parameters.maximum_duration_ms
    } else {
        nonnegative
    })
}

/// Only the native strict budget minimum; other start conditions are external.
pub fn exceeds_sprint_restart_minimum(remaining_ms: i32) -> bool {
    remaining_ms > 1000
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprintStopDelay {
    None,
    /// Caller qualified the active interruption and current jump button bit21.
    JumpInterrupt,
}

/// Returns stop writes atomically, preserving start time and stored budget.
///
/// This implements the ordinary stop helper, including the caller's optional
/// jump delay. It does not set the exhaustion marker after budget depletion,
/// clear the restart latch on button release, or infer a stop from input alone.
pub fn stop_sprint(
    state: SprintBudgetState,
    now_ms: i32,
    sprint_button_pressed: bool,
    delay: SprintStopDelay,
) -> Result<SprintBudgetState, SprintBudgetError> {
    if !state.active {
        return Ok(state);
    }
    let stop_time_ms = match delay {
        SprintStopDelay::None => now_ms,
        SprintStopDelay::JumpInterrupt => now_ms
            .checked_add(800)
            .ok_or(SprintBudgetError::TimeOverflow)?,
    };
    Ok(SprintBudgetState {
        active: false,
        restart_latched: state.restart_latched || sprint_button_pressed,
        exhausted: false,
        stop_time_ms,
        ..state
    })
}
