//! Integer posture classification and active view-height stage timing.
//!
//! Native 1a3920 reads target height and signed state5b8; 037ae0 supplies stage
//! duration. This module does not apply curves, hulls, camera or life-state writes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Posture {
    Standing,
    Crouching,
    Prone,
}

/// Classifies target height, independently of raw posture flags/current height.
///
/// Special target22 reads a full signed i32: values >=3 mean standing, zero
/// means prone and every other value (including negatives) means crouching.
/// Stage22's reachability is not supplied by this classification boundary.
pub fn classify_posture(target_height: i32, special_state: i32) -> Posture {
    match target_height {
        11 => Posture::Prone,
        40 => Posture::Crouching,
        22 if special_state >= 3 => Posture::Standing,
        22 if special_state == 0 => Posture::Prone,
        22 => Posture::Crouching,
        _ => Posture::Standing,
    }
}

/// Native stage duration with raw direction tested for zero, not normalized.
pub fn view_height_duration_ms(stage_height: i32, direction: i32) -> u16 {
    match stage_height {
        11 => 600,
        40 if direction == 0 => 400,
        _ => 200,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewHeightProgress {
    pub duration_ms: u16,
    pub percent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostureTimingError {
    /// Source-ordered subtraction or multiplication leaves the signed domain.
    TimeOverflow,
}

impl std::fmt::Display for PostureTimingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for PostureTimingError {}

/// Computes only the active-stage integer timing boundary of native 53eaa0.
///
/// Zero timestamp skips this arithmetic and returns None. The full owner may
/// initialize a stage there; None does not mean a whole posture update is a no-op.
/// Nonzero timestamps use checked i32 subtraction, then checked *100, signed
/// division truncating toward zero and clamp0..100. Overflow rejection is a safe
/// boundary policy; native wrapping behavior is not admitted. Percent100 carries
/// no completion/height/mode writes. Input and authoritative state are untouched.
pub fn view_height_timing(
    command_time_ms: i32,
    transition_timestamp_ms: i32,
    stage_height: i32,
    direction: i32,
) -> Result<Option<ViewHeightProgress>, PostureTimingError> {
    if transition_timestamp_ms == 0 {
        return Ok(None);
    }
    let duration_ms = view_height_duration_ms(stage_height, direction);
    let scaled_elapsed = command_time_ms
        .checked_sub(transition_timestamp_ms)
        .ok_or(PostureTimingError::TimeOverflow)?
        .checked_mul(100)
        .ok_or(PostureTimingError::TimeOverflow)?;
    let percent = (scaled_elapsed / i32::from(duration_ms)).clamp(0, 100) as u8;
    Ok(Some(ViewHeightProgress {
        duration_ms,
        percent,
    }))
}
