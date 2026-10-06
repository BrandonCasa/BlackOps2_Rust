//! Explicit native command time, independent of Bevy's fixed update frequency.

const BACKLOG_MS: i32 = 150;
const SUBSTEP_MS: i32 = 66;
const MILLISECONDS_TO_SECONDS: f32 = f32::from_bits(0x3a83_126f);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingError {
    /// The native signed arithmetic would leave the qualified non-wrapping domain.
    TimeOverflow,
}

impl std::fmt::Display for TimingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for TimingError {}

/// Single-step duration is clamped to 1..=200ms by native 544b30.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SingleStepDuration(u8);

impl SingleStepDuration {
    pub fn from_elapsed_ms(elapsed_ms: i32) -> Self {
        Self(elapsed_ms.clamp(1, 200) as u8)
    }

    pub fn milliseconds(self) -> u8 {
        self.0
    }

    pub fn seconds(self) -> f32 {
        f32::from(self.0) * MILLISECONDS_TO_SECONDS
    }
}

/// Validated command-loop plan. Construction does not mutate authoritative state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandTiming {
    initial_time_ms: i32,
    final_time_ms: i32,
    discarded_ms: u32,
    ignored_backwards: bool,
}

impl CommandTiming {
    pub fn new(current_ms: i32, requested_ms: i32) -> Result<Self, TimingError> {
        if requested_ms < current_ms {
            return Ok(Self {
                initial_time_ms: current_ms,
                final_time_ms: current_ms,
                discarded_ms: 0,
                ignored_backwards: true,
            });
        }
        // Native evaluates current+150 before its equality check. Signed wrapping
        // and gaps outside signed elapsed arithmetic have not been qualified.
        current_ms
            .checked_add(BACKLOG_MS)
            .ok_or(TimingError::TimeOverflow)?;
        let gap = requested_ms
            .checked_sub(current_ms)
            .ok_or(TimingError::TimeOverflow)?;
        let discarded_ms = gap.saturating_sub(BACKLOG_MS).max(0) as u32;
        let initial_time_ms = requested_ms - gap.min(BACKLOG_MS);
        Ok(Self {
            initial_time_ms,
            final_time_ms: requested_ms,
            discarded_ms,
            ignored_backwards: false,
        })
    }

    /// State time after the backlog discard, before the first numerical step.
    pub fn initial_time_ms(self) -> i32 {
        self.initial_time_ms
    }

    pub fn final_time_ms(self) -> i32 {
        self.final_time_ms
    }

    pub fn discarded_ms(self) -> u32 {
        self.discarded_ms
    }

    pub fn ignored_backwards(self) -> bool {
        self.ignored_backwards
    }

    pub fn substeps(self) -> CommandSubsteps {
        CommandSubsteps {
            current_ms: self.initial_time_ms,
            final_ms: self.final_time_ms,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandSubstep {
    pub end_time_ms: i32,
    pub duration: SingleStepDuration,
}

/// Allocation-free iteration; at most three substeps remain after backlog capping.
#[derive(Debug, Clone)]
pub struct CommandSubsteps {
    current_ms: i32,
    final_ms: i32,
}

impl Iterator for CommandSubsteps {
    type Item = CommandSubstep;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_ms == self.final_ms {
            return None;
        }
        // Construction establishes a nonnegative remaining gap <=150ms; this
        // addition never passes final_ms or overflows the admitted signed range.
        let duration_ms = (self.final_ms - self.current_ms).min(SUBSTEP_MS);
        self.current_ms += duration_ms;
        Some(CommandSubstep {
            end_time_ms: self.current_ms,
            duration: SingleStepDuration::from_elapsed_ms(duration_ms),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.final_ms - self.current_ms;
        let count = ((remaining + SUBSTEP_MS - 1) / SUBSTEP_MS) as usize;
        (count, Some(count))
    }
}

impl ExactSizeIterator for CommandSubsteps {}
impl std::iter::FusedIterator for CommandSubsteps {}
