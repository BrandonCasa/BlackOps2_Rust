//! Ideal ordinary-coop sampled revival timing (source03), not a float/VM emulator.
//! Eligibility precedes each wait; standing reviver is checked after it, before
//! threshold success. Final-wait release is not a continuous cancellation rule.

use crate::{
    DamageRoute, LifeCommand, LifeCommandOutcome, LifeCommandQueue, LifeCommandRequest,
    LifeCommandResult, LifeState, LifeStateError, PlayerLethalPolicy, ReviveRestoration,
    ReviveToken, Vitality,
};
use bevy::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviveClockError {
    ZeroDuration,
    ZeroInterval,
    UnsupportedInterval,
    Lifecycle(LifeStateError),
    WrongPhase,
    NotReady,
    InFlight,
    Retired,
    UnexpectedAcknowledgement,
}
impl std::fmt::Display for ReviveClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ReviveClockError {}

/// Ideal Duration adaptation, not original floating accumulation or an exact
/// VM step count. One interval is credited per completed admitted wait; overshoot
/// is discarded, since no eligibility for further waits has been sampled yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviveTiming {
    duration: Duration,
    interval: Duration,
}
impl ReviveTiming {
    pub fn new(duration: Duration, interval: Duration) -> Result<Self, ReviveClockError> {
        if duration.is_zero() {
            return Err(ReviveClockError::ZeroDuration);
        }
        if interval.is_zero() {
            return Err(ReviveClockError::ZeroInterval);
        }
        if interval > duration {
            return Err(ReviveClockError::UnsupportedInterval);
        }
        Ok(Self { duration, interval })
    }
    pub fn duration(self) -> Duration {
        self.duration
    }
    pub fn interval(self) -> Duration {
        self.interval
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviveEligibility {
    Eligible,
    Ineligible,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviveDecision {
    Cancel,
    Complete,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    AwaitingEligibility,
    Waiting,
    Ready(ReviveDecision),
    InFlight { id: u64, decision: ReviveDecision },
    Retired,
}

/// One checked attempt on a continuous Vitality lifetime. Assembly supplies
/// ordered eligibility/lifecycle samples and Time<Fixed>::delta(); no raw use,
/// touch/facing/team/trace/tool, initiation polling or auto-revive is inferred.
/// Retire clock, pending inputs/receipts/tokens before component replacement.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ReviveClock {
    token: ReviveToken,
    timing: ReviveTiming,
    restore: ReviveRestoration,
    progress: Duration,
    wait: Timer,
    phase: Phase,
}
impl ReviveClock {
    pub fn new(
        target: Entity,
        vitality: Vitality,
        token: ReviveToken,
        timing: ReviveTiming,
        restore: ReviveRestoration,
    ) -> Result<Self, ReviveClockError> {
        Self::check_target(target, vitality, token)?;
        if let DamageRoute::Human { downed_health, .. } = vitality.route
            && downed_health.get() > restore.maximum.get()
        {
            return Err(ReviveClockError::Lifecycle(
                LifeStateError::InvalidDownedHealth,
            ));
        }
        Ok(Self {
            token,
            timing,
            restore,
            progress: Duration::ZERO,
            wait: Timer::new(timing.interval, TimerMode::Once),
            phase: Phase::AwaitingEligibility,
        })
    }
    fn check_target(
        target: Entity,
        vitality: Vitality,
        token: ReviveToken,
    ) -> Result<(), ReviveClockError> {
        vitality
            .check_token(target, token)
            .map_err(ReviveClockError::Lifecycle)?;
        if matches!(
            vitality.route,
            DamageRoute::Human {
                policy: PlayerLethalPolicy::SoloOrLastEligiblePlayer,
                ..
            }
        ) {
            return Err(ReviveClockError::Lifecycle(
                LifeStateError::UnsupportedPlayerPolicy,
            ));
        }
        Ok(())
    }
    fn validate(
        &mut self,
        target: Entity,
        vitality: Option<Vitality>,
        reviver: Entity,
    ) -> Result<(), ReviveClockError> {
        // Caller identity mistakes reject atomically; real lifecycle loss retires.
        if target != self.token.target {
            return Err(ReviveClockError::Lifecycle(LifeStateError::WrongTarget));
        }
        if reviver != self.token.reviver {
            return Err(ReviveClockError::Lifecycle(LifeStateError::InvalidReviver));
        }
        if self.is_retired() {
            return Err(ReviveClockError::Retired);
        }
        let Some(vitality) = vitality else {
            self.retire();
            return Err(ReviveClockError::Lifecycle(LifeStateError::MissingTarget));
        };
        if let Err(error) = Self::check_target(target, vitality, self.token) {
            self.retire();
            return Err(error);
        }
        Ok(())
    }
    pub fn progress(&self) -> Duration {
        self.progress
    }
    pub fn token(&self) -> ReviveToken {
        self.token
    }
    pub fn awaits_eligibility(&self) -> bool {
        self.phase == Phase::AwaitingEligibility
    }
    pub fn is_retired(&self) -> bool {
        self.phase == Phase::Retired
    }
    pub fn in_flight(&self) -> Option<u64> {
        if let Phase::InFlight { id, .. } = self.phase {
            Some(id)
        } else {
            None
        }
    }
    pub fn decision(&self) -> Option<ReviveDecision> {
        if let Phase::Ready(decision) = self.phase {
            Some(decision)
        } else {
            None
        }
    }
    pub fn retire(&mut self) {
        self.phase = Phase::Retired;
    }

    /// Sample eligibility only at the loop boundary. Waiting rejects a new sample,
    /// preserving a final admitted wait despite release within that interval.
    pub fn sample_eligibility(
        &mut self,
        target: Entity,
        vitality: Option<Vitality>,
        reviver: Entity,
        eligibility: ReviveEligibility,
    ) -> Result<(), ReviveClockError> {
        self.validate(target, vitality, reviver)?;
        if !self.awaits_eligibility() {
            return Err(ReviveClockError::WrongPhase);
        }
        self.phase = match eligibility {
            ReviveEligibility::Eligible => {
                self.wait.reset();
                Phase::Waiting
            }
            ReviveEligibility::Ineligible => Phase::Ready(ReviveDecision::Cancel),
        };
        Ok(())
    }

    /// At most one admitted wait per call. Assembly must pass fresh target and
    /// reviver state after its explicitly ordered damage/state changes. Reviver
    /// absent/nonhuman/downed after the wait cancels before threshold completion.
    /// Eligibility is not reread at the end of the final wait.
    pub fn tick(
        &mut self,
        target: Entity,
        vitality: Option<Vitality>,
        reviver: Entity,
        reviver_vitality: Option<Vitality>,
        delta: Duration,
    ) -> Result<Option<ReviveDecision>, ReviveClockError> {
        self.validate(target, vitality, reviver)?;
        if self.phase == Phase::Waiting {
            self.wait.tick(delta.min(self.wait.remaining()));
            if self.wait.is_finished() {
                let standing = reviver_vitality.is_some_and(|v| {
                    matches!(v.route, DamageRoute::Human { .. }) && v.state == LifeState::Standing
                });
                if !standing {
                    self.phase = Phase::Ready(ReviveDecision::Cancel);
                } else {
                    self.progress += self
                        .timing
                        .interval
                        .min(self.timing.duration - self.progress);
                    self.phase = if self.progress >= self.timing.duration {
                        Phase::Ready(ReviveDecision::Complete)
                    } else {
                        Phase::AwaitingEligibility
                    };
                }
            }
        }
        Ok(self.decision())
    }

    /// Assembly controls ID/order relative to damage, cancellation and expiry.
    /// Queue refusal leaves the decision unchanged. One request blocks production.
    pub fn enqueue(
        &mut self,
        queue: &mut LifeCommandQueue,
        id: u64,
    ) -> Result<(), ReviveClockError> {
        let decision = match self.phase {
            Phase::Ready(decision) => decision,
            Phase::InFlight { .. } => return Err(ReviveClockError::InFlight),
            Phase::Retired => return Err(ReviveClockError::Retired),
            _ => return Err(ReviveClockError::NotReady),
        };
        let command = match decision {
            ReviveDecision::Cancel => LifeCommand::CancelRevive(self.token),
            ReviveDecision::Complete => LifeCommand::CompleteRevive {
                token: self.token,
                restore: self.restore,
            },
        };
        queue
            .submit(LifeCommandRequest {
                id,
                target: self.token.target,
                command,
            })
            .map_err(ReviveClockError::Lifecycle)?;
        self.phase = Phase::InFlight { id, decision };
        Ok(())
    }
    /// Exact acknowledgement consumes only the pending decision. A matching error
    /// retires normally; wrong ID/target/kind/restored values reject atomically.
    pub fn acknowledge(&mut self, receipt: LifeCommandResult) -> Result<(), ReviveClockError> {
        if self.is_retired() {
            return Err(ReviveClockError::Retired);
        }
        let Phase::InFlight { id, decision } = self.phase else {
            return Err(ReviveClockError::UnexpectedAcknowledgement);
        };
        if receipt.id != id || receipt.target != self.token.target {
            return Err(ReviveClockError::UnexpectedAcknowledgement);
        }
        let matches = match (decision, receipt.result) {
            (_, Err(_)) => true,
            (ReviveDecision::Cancel, Ok(LifeCommandOutcome::ReviveCanceled)) => true,
            (ReviveDecision::Complete, Ok(LifeCommandOutcome::Revived { current, maximum })) => {
                current == self.restore.current.get() && maximum == self.restore.maximum.get()
            }
            _ => false,
        };
        if !matches {
            return Err(ReviveClockError::UnexpectedAcknowledgement);
        }
        self.retire();
        Ok(())
    }
}
