use std::time::Duration;

use bevy::prelude::*;

use super::{
    DamageRoute, DownedExpiry, ExpiryOutcome, LifeCommand, LifeCommandOutcome, LifeCommandQueue,
    LifeCommandRequest, LifeCommandResult, LifeStateError, PlayerLethalPolicy, Vitality,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleedoutClockError {
    ZeroCountdown,
    UnsupportedFractionalCountdown,
    ZeroPollInterval,
    Lifecycle(LifeStateError),
    NotDue,
    InFlight,
    Retired,
    UnexpectedAcknowledgement,
}

impl std::fmt::Display for BleedoutClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BleedoutClockError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BleedoutTiming {
    countdown: Duration,
    poll: Duration,
}

impl BleedoutTiming {
    pub fn new(countdown: Duration, poll: Duration) -> Result<Self, BleedoutClockError> {
        if countdown.is_zero() {
            return Err(BleedoutClockError::ZeroCountdown);
        }
        if countdown.subsec_nanos() != 0 {
            return Err(BleedoutClockError::UnsupportedFractionalCountdown);
        }
        if poll.is_zero() {
            return Err(BleedoutClockError::ZeroPollInterval);
        }
        Ok(Self { countdown, poll })
    }
    pub fn countdown(self) -> Duration {
        self.countdown
    }
    pub fn poll_interval(self) -> Duration {
        self.poll
    }
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct BleedoutClock {
    expiry: DownedExpiry,
    timer: Timer,
    poll: Duration,
    in_flight: Option<u64>,
    retired: bool,
}

impl BleedoutClock {
    pub fn new(
        target: Entity,
        vitality: Vitality,
        timing: BleedoutTiming,
    ) -> Result<Self, BleedoutClockError> {
        let expiry = vitality
            .downed_expiry(target)
            .map_err(BleedoutClockError::Lifecycle)?;
        Self::check_coop(vitality)?;
        Ok(Self {
            expiry,
            timer: Timer::new(timing.countdown, TimerMode::Once),
            poll: timing.poll,
            in_flight: None,
            retired: false,
        })
    }

    fn check_coop(vitality: Vitality) -> Result<(), BleedoutClockError> {
        if matches!(
            vitality.route,
            DamageRoute::Human {
                policy: PlayerLethalPolicy::SoloOrLastEligiblePlayer,
                ..
            }
        ) {
            return Err(BleedoutClockError::Lifecycle(
                LifeStateError::UnsupportedPlayerPolicy,
            ));
        }
        Ok(())
    }

    pub fn expiry(&self) -> DownedExpiry {
        self.expiry
    }
    pub fn is_retired(&self) -> bool {
        self.retired
    }
    pub fn in_flight(&self) -> Option<u64> {
        self.in_flight
    }

    pub fn retire(&mut self) {
        self.retired = true;
        self.in_flight = None;
    }

    pub fn tick(
        &mut self,
        target: Entity,
        vitality: Option<Vitality>,
        delta: Duration,
    ) -> Result<Option<DownedExpiry>, BleedoutClockError> {
        if target != self.expiry.target() {
            return Err(BleedoutClockError::Lifecycle(LifeStateError::WrongTarget));
        }
        if self.retired {
            return Ok(None);
        }
        let Some(vitality) = vitality else {
            self.retire();
            return Err(BleedoutClockError::Lifecycle(LifeStateError::MissingTarget));
        };
        if vitality.downed_episode() != Some(self.expiry.episode()) {
            self.retire();
            return Ok(None);
        }
        if let Err(error) = Self::check_coop(vitality) {
            self.retire();
            return Err(error);
        }
        if self.in_flight.is_some() {
            return Ok(None);
        }
        self.timer.tick(delta.min(self.timer.remaining()));
        Ok(self.timer.is_finished().then_some(self.expiry))
    }

    pub fn enqueue(
        &mut self,
        queue: &mut LifeCommandQueue,
        id: u64,
    ) -> Result<(), BleedoutClockError> {
        if self.retired {
            return Err(BleedoutClockError::Retired);
        }
        if self.in_flight.is_some() {
            return Err(BleedoutClockError::InFlight);
        }
        if !self.timer.is_finished() {
            return Err(BleedoutClockError::NotDue);
        }
        queue
            .submit(LifeCommandRequest {
                id,
                target: self.expiry.target(),
                command: LifeCommand::ExpireDowned(self.expiry),
            })
            .map_err(BleedoutClockError::Lifecycle)?;
        self.in_flight = Some(id);
        Ok(())
    }

    pub fn acknowledge(&mut self, receipt: LifeCommandResult) -> Result<(), BleedoutClockError> {
        if self.retired {
            return Err(BleedoutClockError::Retired);
        }
        if self.in_flight != Some(receipt.id) || receipt.target != self.expiry.target() {
            return Err(BleedoutClockError::UnexpectedAcknowledgement);
        }
        match receipt.result {
            Ok(LifeCommandOutcome::Expiry(ExpiryOutcome::Deferred(expiry)))
                if expiry == self.expiry =>
            {
                self.in_flight = None;
                self.timer = Timer::new(self.poll, TimerMode::Once);
            }
            Ok(LifeCommandOutcome::Expiry(
                ExpiryOutcome::Terminated { .. } | ExpiryOutcome::Ignored,
            ))
            | Err(_) => self.retire(),
            _ => return Err(BleedoutClockError::UnexpectedAcknowledgement),
        }
        Ok(())
    }
}
