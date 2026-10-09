#![forbid(unsafe_code)]

pub mod revive_clock;
pub use revive_clock::{
    ReviveClock, ReviveClockError, ReviveDecision, ReviveEligibility, ReviveTiming,
};

pub mod bleedout;
pub use bleedout::{BleedoutClock, BleedoutClockError, BleedoutTiming};
mod self_revive;
pub use self_revive::SelfReviveToken;

use std::num::NonZeroU32;

use bevy::prelude::*;

const MAX_POSITIVE_NATIVE_INTEGER: u32 = i32::MAX.unsigned_abs();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeStateError {
    ZeroMaximum,
    InvalidCurrent,
    ZeroDamage,
    InvalidDownedHealth,
    HealthOutOfRange,
    DamageOutOfRange,
    UnsupportedPlayerPolicy,
    MissingTarget,
    NonIncreasingRequestId,
    NotHuman,
    NotStanding,
    NotDowned,
    StaleEpisode,
    StaleAttempt,
    WrongTarget,
    AttemptAlreadyActive,
    InvalidReviver,
    CounterExhausted,
}

impl std::fmt::Display for LifeStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for LifeStateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedDamage(NonZeroU32);

impl ResolvedDamage {
    pub fn new(amount: u32) -> Result<Self, LifeStateError> {
        if amount > MAX_POSITIVE_NATIVE_INTEGER {
            return Err(LifeStateError::DamageOutOfRange);
        }
        NonZeroU32::new(amount)
            .map(Self)
            .ok_or(LifeStateError::ZeroDamage)
    }

    pub fn amount(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerLethalPolicy {
    DownWithEligibleTeammate,
    SoloOrLastEligiblePlayer,
    SoloSelfRevive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeState {
    Standing,
    Downed,
    HumanTerminal,
    ActorAlive,
    ActorDead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DamageRoute {
    Human {
        policy: PlayerLethalPolicy,
        downed_health: NonZeroU32,
    },
    OrdinaryActor,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vitality {
    current: u32,
    maximum: NonZeroU32,
    state: LifeState,
    route: DamageRoute,
    episode: u64,
    attempt: u64,
    active_revive: Option<ReviveToken>,
    active_self_revive: Option<SelfReviveToken>,
    actor_subtraction: Option<ActorSubtraction>,
}

impl Vitality {
    fn initial(
        current: u32,
        maximum: u32,
        route: DamageRoute,
        state: LifeState,
    ) -> Result<Self, LifeStateError> {
        let maximum = NonZeroU32::new(maximum).ok_or(LifeStateError::ZeroMaximum)?;
        if maximum.get() > MAX_POSITIVE_NATIVE_INTEGER || current > MAX_POSITIVE_NATIVE_INTEGER {
            return Err(LifeStateError::HealthOutOfRange);
        }
        if current == 0 || current > maximum.get() {
            return Err(LifeStateError::InvalidCurrent);
        }
        Ok(Self {
            current,
            maximum,
            state,
            route,
            episode: 0,
            attempt: 0,
            active_revive: None,
            active_self_revive: None,
            actor_subtraction: None,
        })
    }

    pub fn human(
        current: u32,
        maximum: u32,
        downed_health: u32,
        policy: PlayerLethalPolicy,
    ) -> Result<Self, LifeStateError> {
        let downed_health =
            NonZeroU32::new(downed_health).ok_or(LifeStateError::InvalidDownedHealth)?;
        if downed_health.get() > MAX_POSITIVE_NATIVE_INTEGER {
            return Err(LifeStateError::HealthOutOfRange);
        }
        if downed_health.get() > maximum {
            return Err(LifeStateError::InvalidDownedHealth);
        }
        Self::initial(
            current,
            maximum,
            DamageRoute::Human {
                policy,
                downed_health,
            },
            LifeState::Standing,
        )
    }

    pub fn ordinary_actor(current: u32, maximum: u32) -> Result<Self, LifeStateError> {
        Self::initial(
            current,
            maximum,
            DamageRoute::OrdinaryActor,
            LifeState::ActorAlive,
        )
    }

    pub fn current(self) -> Option<u32> {
        (self.state != LifeState::ActorDead).then_some(self.current)
    }

    pub fn actor_subtraction(self) -> Option<ActorSubtraction> {
        self.actor_subtraction
    }
    pub fn maximum(self) -> u32 {
        self.maximum.get()
    }
    pub fn set_human_maximum(&mut self, maximum: u32) -> Result<(), LifeStateError> {
        if !matches!(self.route, DamageRoute::Human { .. }) {
            return Err(LifeStateError::NotHuman);
        }
        let maximum = NonZeroU32::new(maximum).ok_or(LifeStateError::ZeroMaximum)?;
        if maximum.get() > MAX_POSITIVE_NATIVE_INTEGER {
            return Err(LifeStateError::HealthOutOfRange);
        }
        if let DamageRoute::Human { downed_health, .. } = self.route
            && maximum.get() < downed_health.get()
        {
            return Err(LifeStateError::InvalidDownedHealth);
        }
        self.maximum = maximum;
        self.current = self.current.min(maximum.get());
        Ok(())
    }
    pub fn state(self) -> LifeState {
        self.state
    }

    pub fn set_player_policy(&mut self, policy: PlayerLethalPolicy) {
        if let DamageRoute::Human { policy: stored, .. } = &mut self.route {
            *stored = policy;
        }
    }

    pub fn apply(&mut self, damage: ResolvedDamage) -> Result<DamageOutcome, LifeStateError> {
        if matches!(
            self.state,
            LifeState::Downed | LifeState::HumanTerminal | LifeState::ActorDead
        ) {
            return Ok(DamageOutcome::Ignored { state: self.state });
        }
        let before = self.current;
        match self.route {
            DamageRoute::OrdinaryActor => {
                let signed_before =
                    i32::try_from(before).map_err(|_| LifeStateError::HealthOutOfRange)?;
                let signed_amount =
                    i32::try_from(damage.amount()).map_err(|_| LifeStateError::DamageOutOfRange)?;
                let after = signed_before
                    .checked_sub(signed_amount)
                    .ok_or(LifeStateError::HealthOutOfRange)?;
                let subtraction = ActorSubtraction {
                    before,
                    amount: damage.amount(),
                    after,
                };
                self.actor_subtraction = Some(subtraction);
                if after > 0 {
                    self.current = before - damage.amount();
                    Ok(DamageOutcome::Hurt {
                        before,
                        after: self.current,
                    })
                } else {
                    self.state = LifeState::ActorDead;
                    Ok(DamageOutcome::ActorDied { subtraction })
                }
            }
            DamageRoute::Human {
                policy,
                downed_health,
            } => {
                if damage.amount() < before {
                    self.current = before - damage.amount();
                    return Ok(DamageOutcome::Hurt {
                        before,
                        after: self.current,
                    });
                }
                if policy == PlayerLethalPolicy::SoloOrLastEligiblePlayer {
                    self.state = LifeState::HumanTerminal;
                    return Ok(DamageOutcome::HumanDied { before });
                }
                let episode = self
                    .episode
                    .checked_add(1)
                    .ok_or(LifeStateError::CounterExhausted)?;
                self.current = downed_health.get();
                self.state = LifeState::Downed;
                self.episode = episode;
                Ok(DamageOutcome::Downed {
                    before,
                    after: self.current,
                    episode: DownedEpisode(episode),
                })
            }
        }
    }

    pub fn regenerate(&mut self, amount: u32) -> Result<u32, LifeStateError> {
        if !matches!(self.route, DamageRoute::Human { .. }) {
            return Err(LifeStateError::NotHuman);
        }
        if self.state != LifeState::Standing {
            return Err(LifeStateError::NotStanding);
        }
        if amount > MAX_POSITIVE_NATIVE_INTEGER {
            return Err(LifeStateError::HealthOutOfRange);
        }
        let before = self.current;
        self.current = self.current.saturating_add(amount).min(self.maximum.get());
        Ok(self.current - before)
    }

    pub fn downed_episode(self) -> Option<DownedEpisode> {
        (self.state == LifeState::Downed).then_some(DownedEpisode(self.episode))
    }

    pub fn downed_expiry(self, target: Entity) -> Result<DownedExpiry, LifeStateError> {
        let episode = DownedEpisode(self.episode);
        self.check_episode(episode)?;
        Ok(DownedExpiry { target, episode })
    }

    pub fn expire_downed(
        &mut self,
        target: Entity,
        expiry: DownedExpiry,
    ) -> Result<ExpiryOutcome, LifeStateError> {
        if expiry.target != target {
            return Err(LifeStateError::WrongTarget);
        }
        if !matches!(self.route, DamageRoute::Human { .. }) {
            return Err(LifeStateError::NotHuman);
        }
        if self.episode != expiry.episode.0 {
            return Err(LifeStateError::StaleEpisode);
        }
        if self.state == LifeState::HumanTerminal {
            return Ok(ExpiryOutcome::Ignored);
        }
        if self.state != LifeState::Downed {
            return Err(LifeStateError::StaleEpisode);
        }
        if matches!(
            self.route,
            DamageRoute::Human {
                policy: PlayerLethalPolicy::SoloOrLastEligiblePlayer,
                ..
            }
        ) {
            return Err(LifeStateError::UnsupportedPlayerPolicy);
        }
        if self.active_revive.is_some() {
            return Ok(ExpiryOutcome::Deferred(expiry));
        }
        self.state = LifeState::HumanTerminal;
        self.active_revive = None;
        self.active_self_revive = None;
        Ok(ExpiryOutcome::Terminated {
            current: self.current,
            maximum: self.maximum.get(),
        })
    }

    fn check_episode(&self, episode: DownedEpisode) -> Result<(), LifeStateError> {
        if !matches!(self.route, DamageRoute::Human { .. }) {
            return Err(LifeStateError::NotHuman);
        }
        if self.state != LifeState::Downed {
            return Err(LifeStateError::NotDowned);
        }
        if self.episode != episode.0 {
            return Err(LifeStateError::StaleEpisode);
        }
        Ok(())
    }

    pub fn begin_revive(
        &mut self,
        target: Entity,
        reviver: Entity,
        episode: DownedEpisode,
    ) -> Result<ReviveToken, LifeStateError> {
        self.check_episode(episode)?;
        if !matches!(
            self.route,
            DamageRoute::Human {
                policy: PlayerLethalPolicy::DownWithEligibleTeammate,
                ..
            }
        ) {
            return Err(LifeStateError::UnsupportedPlayerPolicy);
        }
        if target == reviver {
            return Err(LifeStateError::InvalidReviver);
        }
        if self.active_revive.is_some() || self.active_self_revive.is_some() {
            return Err(LifeStateError::AttemptAlreadyActive);
        }
        let attempt = self
            .attempt
            .checked_add(1)
            .ok_or(LifeStateError::CounterExhausted)?;
        let token = ReviveToken {
            target,
            reviver,
            episode,
            attempt,
        };
        self.attempt = attempt;
        self.active_revive = Some(token);
        Ok(token)
    }

    fn check_token(&self, target: Entity, token: ReviveToken) -> Result<(), LifeStateError> {
        if token.target != target {
            return Err(LifeStateError::WrongTarget);
        }
        self.check_episode(token.episode)?;
        if self.active_revive != Some(token) {
            return Err(LifeStateError::StaleAttempt);
        }
        Ok(())
    }

    pub fn cancel_revive(
        &mut self,
        target: Entity,
        token: ReviveToken,
    ) -> Result<(), LifeStateError> {
        self.check_token(target, token)?;
        self.active_revive = None;
        Ok(())
    }

    pub fn complete_revive(
        &mut self,
        target: Entity,
        token: ReviveToken,
        restore: ReviveRestoration,
    ) -> Result<(), LifeStateError> {
        self.check_token(target, token)?;
        if let DamageRoute::Human { downed_health, .. } = self.route
            && downed_health.get() > restore.maximum.get()
        {
            return Err(LifeStateError::InvalidDownedHealth);
        }
        self.current = restore.current.get();
        self.maximum = restore.maximum;
        self.state = LifeState::Standing;
        self.active_revive = None;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownedEpisode(u64);

impl DownedEpisode {
    pub fn number(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownedExpiry {
    target: Entity,
    episode: DownedEpisode,
}

impl DownedExpiry {
    pub fn target(self) -> Entity {
        self.target
    }
    pub fn episode(self) -> DownedEpisode {
        self.episode
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryOutcome {
    Deferred(DownedExpiry),
    Terminated { current: u32, maximum: u32 },
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviveToken {
    target: Entity,
    reviver: Entity,
    episode: DownedEpisode,
    attempt: u64,
}

impl ReviveToken {
    pub fn reviver(self) -> Entity {
        self.reviver
    }
    pub fn episode(self) -> DownedEpisode {
        self.episode
    }
    pub fn attempt(self) -> u64 {
        self.attempt
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviveRestoration {
    current: NonZeroU32,
    maximum: NonZeroU32,
}

impl ReviveRestoration {
    pub fn new(current: u32, maximum: u32) -> Result<Self, LifeStateError> {
        let maximum = NonZeroU32::new(maximum).ok_or(LifeStateError::ZeroMaximum)?;
        if maximum.get() > MAX_POSITIVE_NATIVE_INTEGER || current > MAX_POSITIVE_NATIVE_INTEGER {
            return Err(LifeStateError::HealthOutOfRange);
        }
        let current = NonZeroU32::new(current)
            .filter(|v| v.get() <= maximum.get())
            .ok_or(LifeStateError::InvalidCurrent)?;
        Ok(Self { current, maximum })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSubtraction {
    pub before: u32,
    pub amount: u32,
    pub after: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageOutcome {
    HumanDied {
        before: u32,
    },
    Hurt {
        before: u32,
        after: u32,
    },
    Downed {
        before: u32,
        after: u32,
        episode: DownedEpisode,
    },
    ActorDied {
        subtraction: ActorSubtraction,
    },
    Ignored {
        state: LifeState,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifeCommandRequest {
    pub id: u64,
    pub target: Entity,
    pub command: LifeCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeCommand {
    Damage(ResolvedDamage),
    ExpireDowned(DownedExpiry),
    BeginRevive {
        reviver: Entity,
        episode: DownedEpisode,
    },
    CancelRevive(ReviveToken),
    CompleteRevive {
        token: ReviveToken,
        restore: ReviveRestoration,
    },
    BeginSelfRevive {
        episode: DownedEpisode,
    },
    CancelSelfRevive(SelfReviveToken),
    CompleteSelfRevive {
        token: SelfReviveToken,
        restore: ReviveRestoration,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeCommandOutcome {
    Damage(DamageOutcome),
    Expiry(ExpiryOutcome),
    ReviveBegan(ReviveToken),
    ReviveCanceled,
    SelfReviveBegan(SelfReviveToken),
    Revived { current: u32, maximum: u32 },
}

#[derive(Resource, Default)]
pub struct LifeCommandQueue {
    last_id: Option<u64>,
    pending: Vec<LifeCommandRequest>,
}

impl LifeCommandQueue {
    pub fn submit_next(
        &mut self,
        target: Entity,
        command: LifeCommand,
    ) -> Result<u64, LifeStateError> {
        let id = self
            .last_id
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(LifeStateError::CounterExhausted)?;
        self.submit(LifeCommandRequest {
            id,
            target,
            command,
        })?;
        Ok(id)
    }
    pub fn submit(&mut self, request: LifeCommandRequest) -> Result<(), LifeStateError> {
        if self.last_id.is_some_and(|last| request.id <= last) {
            return Err(LifeStateError::NonIncreasingRequestId);
        }
        self.last_id = Some(request.id);
        self.pending.push(request);
        Ok(())
    }

    pub fn retire_pending_target(&mut self, target: Entity) -> Vec<LifeCommandRequest> {
        self.pending
            .extract_if(.., |request| request.target == target)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifeCommandResult {
    pub id: u64,
    pub target: Entity,
    pub result: Result<LifeCommandOutcome, LifeStateError>,
}

#[derive(Resource, Default)]
pub struct LifeCommandResults(Vec<LifeCommandResult>);

impl LifeCommandResults {
    pub fn drain(&mut self) -> impl Iterator<Item = LifeCommandResult> + '_ {
        self.0.drain(..)
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApplyLifeCommands;

pub struct LifeStatePlugin;

impl Plugin for LifeStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LifeCommandQueue>()
            .init_resource::<LifeCommandResults>()
            .add_systems(FixedUpdate, apply_life_commands.in_set(ApplyLifeCommands));
    }
}

fn apply_life_commands(
    mut queue: ResMut<LifeCommandQueue>,
    mut results: ResMut<LifeCommandResults>,
    mut targets: Query<&mut Vitality>,
) {
    for request in queue.pending.drain(..) {
        let reviver = match request.command {
            LifeCommand::BeginRevive { reviver, .. } => Some(reviver),
            LifeCommand::CompleteRevive { token, .. } => Some(token.reviver),
            _ => None,
        };
        let reviver_valid = reviver.is_none_or(|id| {
            targets.get(id).is_ok_and(|v| {
                matches!(v.route, DamageRoute::Human { .. }) && v.state == LifeState::Standing
            })
        });
        let result = targets
            .get_mut(request.target)
            .map_err(|_| LifeStateError::MissingTarget)
            .and_then(|mut target| match request.command {
                LifeCommand::Damage(damage) => target.apply(damage).map(LifeCommandOutcome::Damage),
                LifeCommand::ExpireDowned(expiry) => target
                    .expire_downed(request.target, expiry)
                    .map(LifeCommandOutcome::Expiry),
                LifeCommand::BeginRevive { reviver, episode } => {
                    if !reviver_valid {
                        return Err(LifeStateError::InvalidReviver);
                    }
                    target
                        .begin_revive(request.target, reviver, episode)
                        .map(LifeCommandOutcome::ReviveBegan)
                }
                LifeCommand::CancelRevive(token) => target
                    .cancel_revive(request.target, token)
                    .map(|()| LifeCommandOutcome::ReviveCanceled),
                LifeCommand::CompleteRevive { token, restore } => {
                    target.check_token(request.target, token)?;
                    if !reviver_valid {
                        return Err(LifeStateError::InvalidReviver);
                    }
                    target.complete_revive(request.target, token, restore)?;
                    Ok(LifeCommandOutcome::Revived {
                        current: target.current,
                        maximum: target.maximum.get(),
                    })
                }
                LifeCommand::BeginSelfRevive { episode } => target
                    .begin_self_revive(request.target, episode)
                    .map(LifeCommandOutcome::SelfReviveBegan),
                LifeCommand::CancelSelfRevive(token) => target
                    .cancel_self_revive(request.target, token)
                    .map(|()| LifeCommandOutcome::ReviveCanceled),
                LifeCommand::CompleteSelfRevive { token, restore } => {
                    target.complete_self_revive(request.target, token, restore)?;
                    Ok(LifeCommandOutcome::Revived {
                        current: target.current,
                        maximum: target.maximum.get(),
                    })
                }
            });
        results.0.push(LifeCommandResult {
            id: request.id,
            target: request.target,
            result,
        });
    }
}
