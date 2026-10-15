use std::time::Duration;

use bevy::prelude::*;

use super::{
    DamageRoute, DownedExpiry, ExpiryOutcome, LifeStateError, PlayerLethalPolicy, Vitality,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BleedoutClockError {
    ZeroCountdown,
    UnsupportedFractionalCountdown,
    ZeroPollInterval,
    Lifecycle(LifeStateError),
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
    pub fn retire(&mut self) {
        self.retired = true;
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
        self.timer.tick(delta.min(self.timer.remaining()));
        Ok(self.timer.is_finished().then_some(self.expiry))
    }

    pub fn expire(&mut self, target: Entity, vitality: &mut Vitality) -> ExpiryOutcome {
        let outcome = vitality
            .expire_downed(target, self.expiry)
            .expect("Bleedout belongs to the current downed episode");
        match outcome {
            ExpiryOutcome::Deferred(_) => self.timer = Timer::new(self.poll, TimerMode::Once),
            ExpiryOutcome::Terminated { .. } | ExpiryOutcome::Ignored => self.retire(),
        }
        outcome
    }
}
