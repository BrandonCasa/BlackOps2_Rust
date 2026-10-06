//! Explicit solo revival, without a fabricated standing reviver.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelfReviveToken {
    target: Entity,
    episode: DownedEpisode,
    attempt: u64,
}
impl SelfReviveToken {
    pub fn target(self) -> Entity {
        self.target
    }
    pub fn episode(self) -> DownedEpisode {
        self.episode
    }
}
impl Vitality {
    pub fn begin_self_revive(
        &mut self,
        target: Entity,
        episode: DownedEpisode,
    ) -> Result<SelfReviveToken, LifeStateError> {
        self.check_episode(episode)?;
        if !matches!(
            self.route,
            DamageRoute::Human {
                policy: PlayerLethalPolicy::SoloSelfRevive,
                ..
            }
        ) {
            return Err(LifeStateError::UnsupportedPlayerPolicy);
        }
        if self.active_revive.is_some() || self.active_self_revive.is_some() {
            return Err(LifeStateError::AttemptAlreadyActive);
        }
        let attempt = self
            .attempt
            .checked_add(1)
            .ok_or(LifeStateError::CounterExhausted)?;
        let token = SelfReviveToken {
            target,
            episode,
            attempt,
        };
        self.attempt = attempt;
        self.active_self_revive = Some(token);
        Ok(token)
    }
    fn check_self_token(
        &self,
        target: Entity,
        token: SelfReviveToken,
    ) -> Result<(), LifeStateError> {
        if token.target != target {
            return Err(LifeStateError::WrongTarget);
        }
        self.check_episode(token.episode)?;
        if !matches!(
            self.route,
            DamageRoute::Human {
                policy: PlayerLethalPolicy::SoloSelfRevive,
                ..
            }
        ) {
            return Err(LifeStateError::UnsupportedPlayerPolicy);
        }
        if self.active_self_revive != Some(token) {
            return Err(LifeStateError::StaleAttempt);
        }
        Ok(())
    }
    pub fn cancel_self_revive(
        &mut self,
        target: Entity,
        token: SelfReviveToken,
    ) -> Result<(), LifeStateError> {
        self.check_self_token(target, token)?;
        self.active_self_revive = None;
        Ok(())
    }
    pub fn complete_self_revive(
        &mut self,
        target: Entity,
        token: SelfReviveToken,
        restore: ReviveRestoration,
    ) -> Result<(), LifeStateError> {
        self.check_self_token(target, token)?;
        if let DamageRoute::Human { downed_health, .. } = self.route
            && downed_health.get() > restore.maximum.get()
        {
            return Err(LifeStateError::InvalidDownedHealth);
        }
        self.current = restore.current.get();
        self.maximum = restore.maximum;
        self.state = LifeState::Standing;
        self.active_self_revive = None;
        Ok(())
    }
}
