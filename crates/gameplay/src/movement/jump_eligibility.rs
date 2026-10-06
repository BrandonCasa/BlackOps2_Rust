//! Native 2b1cd0's ordered jump authorization and held-command mutation.
//!
//! Supplied posture, surface and owner permission must come from qualified callers.
//! This kernel does not launch a jump or derive ground permission from normals.

const JUMP_BUTTON: u64 = 1 << 21;
const OWNER_SENTINEL: u32 = 0x3ff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JumpEligibilityInput {
    pub state_flags: u32,
    pub movement_mode: i32,
    pub effective_posture: i32,
    pub ground_owner: u32,
    pub surface_flags: u32,
    pub last_jump_time_ms: i32,
    pub command_time_ms: i32,
    pub current_buttons: u64,
    /// Previous native subcommand, not previous render-frame input.
    pub previous_buttons: u64,
    /// Qualified callback result; only required if its source gate is reached.
    pub ground_permission: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpRejection {
    StateFlag80000,
    JumpInterval,
    StateFlag400,
    StateFlag4,
    MovementMode,
    EffectivePosture,
    ButtonAbsent,
    ButtonHeld,
    SurfaceFlag2,
    GroundPermission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpEligibilityDecision {
    /// Authorization only; launch writes and event delivery remain external.
    Eligible,
    Rejected(JumpRejection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JumpEligibilityResult {
    pub current_buttons: u64,
    pub decision: JumpEligibilityDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpEligibilityError {
    /// Source subtraction leaves the qualified non-wrapping signed domain.
    TimeOverflow,
    MissingGroundPermission,
}

impl std::fmt::Display for JumpEligibilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for JumpEligibilityError {}

/// Evaluates the native gates in order, returning command writes atomically.
///
/// Earlier rejection preserves every button; a held jump clears only current
/// bit21 before surface/permission checks. Sentinel owner3ff bypasses posture
/// rejection and permission access. Checked overflow and missing permission are
/// safe-boundary errors, distinct from native rejection and leaving input intact.
pub fn evaluate_jump_eligibility(
    input: JumpEligibilityInput,
) -> Result<JumpEligibilityResult, JumpEligibilityError> {
    let reject = |reason| {
        Ok(JumpEligibilityResult {
            current_buttons: input.current_buttons,
            decision: JumpEligibilityDecision::Rejected(reason),
        })
    };
    if input.state_flags & 0x80000 != 0 {
        return reject(JumpRejection::StateFlag80000);
    }
    let elapsed = input
        .command_time_ms
        .checked_sub(input.last_jump_time_ms)
        .ok_or(JumpEligibilityError::TimeOverflow)?;
    if elapsed < 500 {
        return reject(JumpRejection::JumpInterval);
    }
    if input.state_flags & 0x400 != 0 {
        return reject(JumpRejection::StateFlag400);
    }
    if input.state_flags & 4 != 0 {
        return reject(JumpRejection::StateFlag4);
    }
    if input.movement_mode >= 10 {
        return reject(JumpRejection::MovementMode);
    }
    if input.effective_posture != 0 && input.ground_owner != OWNER_SENTINEL {
        return reject(JumpRejection::EffectivePosture);
    }
    if input.current_buttons & JUMP_BUTTON == 0 {
        return reject(JumpRejection::ButtonAbsent);
    }
    if input.previous_buttons & JUMP_BUTTON != 0 {
        return Ok(JumpEligibilityResult {
            current_buttons: input.current_buttons & !JUMP_BUTTON,
            decision: JumpEligibilityDecision::Rejected(JumpRejection::ButtonHeld),
        });
    }
    if input.surface_flags & 2 != 0 {
        return reject(JumpRejection::SurfaceFlag2);
    }
    if input.ground_owner != OWNER_SENTINEL
        && !input
            .ground_permission
            .ok_or(JumpEligibilityError::MissingGroundPermission)?
    {
        return reject(JumpRejection::GroundPermission);
    }
    Ok(JumpEligibilityResult {
        current_buttons: input.current_buttons,
        decision: JumpEligibilityDecision::Eligible,
    })
}
