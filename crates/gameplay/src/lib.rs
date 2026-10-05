#![forbid(unsafe_code)]

//! Resolved damage and admitted coop revival lifecycle. Native player/actor
//! damage arithmetic remains explicit inference; revival configuration is external.
//! See docs/reverse_engineering/damage-life-state-01.md for evidence and limits.

use std::num::NonZeroU32;

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeStateError {
    ZeroMaximum,
    InvalidCurrent,
    ZeroDamage,
    InvalidDownedHealth,
    UnsupportedPlayerPolicy,
    MissingTarget,
    NonIncreasingRequestId,
    NotHuman,
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

/// Integer admission is a project restriction; original player conversion is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedDamage(NonZeroU32);

impl ResolvedDamage {
    pub fn new(amount: u32) -> Result<Self, LifeStateError> {
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
    /// Caller has established another eligible human teammate remains alive.
    DownWithEligibleTeammate,
    /// Session/game-end behavior must be resolved by a later task.
    SoloOrLastEligiblePlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeState {
    Standing,
    Downed,
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

/// Private fields keep life-state/health invariants together without a registry.
/// Before removing/replacing this component on a living Entity, assembly must
/// retire pending commands and all externally held tokens/episode references.
/// Bevy entity generations do not change when components are replaced. Counters
/// apply only to this continuous component lifetime, not a replacement's lifetime.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vitality {
    current: u32,
    maximum: NonZeroU32,
    state: LifeState,
    route: DamageRoute,
    episode: u64,
    attempt: u64,
    active_revive: Option<ReviveToken>,
}

impl Vitality {
    fn initial(
        current: u32,
        maximum: u32,
        route: DamageRoute,
        state: LifeState,
    ) -> Result<Self, LifeStateError> {
        let maximum = NonZeroU32::new(maximum).ok_or(LifeStateError::ZeroMaximum)?;
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

    pub fn current(self) -> u32 {
        self.current
    }
    pub fn maximum(self) -> u32 {
        self.maximum.get()
    }
    pub fn state(self) -> LifeState {
        self.state
    }

    /// The assembly must refresh this policy before admitting an impact if the
    /// eligible teammate set changes. No session eligibility is inferred here.
    pub fn set_player_policy(&mut self, policy: PlayerLethalPolicy) {
        if let DamageRoute::Human { policy: stored, .. } = &mut self.route {
            *stored = policy;
        }
    }

    /// Lethal player classification/downed assignment follow inspected scripts.
    /// Native health subtraction for both routes and actor zero-clamping are
    /// inferred, pending original-game comparison. Raw impact policy is excluded.
    pub fn apply(&mut self, damage: ResolvedDamage) -> Result<DamageOutcome, LifeStateError> {
        if matches!(self.state, LifeState::Downed | LifeState::ActorDead) {
            return Ok(DamageOutcome::Ignored { state: self.state });
        }
        let before = self.current;
        if damage.amount() < before {
            self.current = before - damage.amount();
            return Ok(DamageOutcome::Hurt {
                before,
                after: self.current,
            });
        }
        match self.route {
            DamageRoute::Human {
                policy: PlayerLethalPolicy::SoloOrLastEligiblePlayer,
                ..
            } => Err(LifeStateError::UnsupportedPlayerPolicy),
            DamageRoute::Human { downed_health, .. } => {
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
            DamageRoute::OrdinaryActor => {
                // Inferred native arithmetic: lethal amount clamps at zero.
                self.current = 0;
                self.state = LifeState::ActorDead;
                Ok(DamageOutcome::ActorDied { before })
            }
        }
    }

    pub fn downed_episode(self) -> Option<DownedEpisode> {
        (self.state == LifeState::Downed).then_some(DownedEpisode(self.episode))
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

    /// Eligibility/timing is externally resolved. No raw revive interaction is modeled.
    pub fn begin_revive(
        &mut self,
        target: Entity,
        reviver: Entity,
        episode: DownedEpisode,
    ) -> Result<ReviveToken, LifeStateError> {
        self.check_episode(episode)?;
        if target == reviver {
            return Err(LifeStateError::InvalidReviver);
        }
        if self.active_revive.is_some() {
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

    /// Qualified restoration is supplied by assembly; current/max are not assumed equal.
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

/// Tokens are scoped to the continuous lifetime of a Vitality component.
/// Removing/replacing that component invalidates its external outstanding inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownedEpisode(u64);

impl DownedEpisode {
    pub fn number(self) -> u64 {
        self.0
    }
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
        let current = NonZeroU32::new(current)
            .filter(|v| v.get() <= maximum.get())
            .ok_or(LifeStateError::InvalidCurrent)?;
        Ok(Self { current, maximum })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageOutcome {
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
        before: u32,
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
    BeginRevive {
        reviver: Entity,
        episode: DownedEpisode,
    },
    CancelRevive(ReviveToken),
    CompleteRevive {
        token: ReviveToken,
        restore: ReviveRestoration,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeCommandOutcome {
    Damage(DamageOutcome),
    ReviveBegan(ReviveToken),
    ReviveCanceled,
    Revived { current: u32, maximum: u32 },
}

/// Explicit caller order: IDs strictly increase for this resource's lifetime.
/// Duplicate or reordered submissions are rejected without changing admission.
/// Submit before FixedUpdate; producer systems must define their own ordering.
#[derive(Resource, Default)]
pub struct LifeCommandQueue {
    last_id: Option<u64>,
    pending: Vec<LifeCommandRequest>,
}

impl LifeCommandQueue {
    pub fn submit(&mut self, request: LifeCommandRequest) -> Result<(), LifeStateError> {
        if self.last_id.is_some_and(|last| request.id <= last) {
            return Err(LifeStateError::NonIncreasingRequestId);
        }
        self.last_id = Some(request.id);
        self.pending.push(request);
        Ok(())
    }

    /// Return pending inputs for explicit retirement before component replacement.
    /// Caller must also retire held tokens/episode references and old result receipts.
    /// This does not change the globally increasing request-ID admission history.
    pub fn retire_pending_target(&mut self, target: Entity) -> Vec<LifeCommandRequest> {
        let (retired, retained) = std::mem::take(&mut self.pending)
            .into_iter()
            .partition(|request| request.target == target);
        self.pending = retained;
        retired
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifeCommandResult {
    pub id: u64,
    pub target: Entity,
    pub result: Result<LifeCommandOutcome, LifeStateError>,
}

/// Consumers drain after the fixed step; results persist across empty steps.
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
        // The resolved boundary checks entity/life admissibility, not team/geometry/timing.
        let reviver_valid = reviver.is_none_or(|id| {
            targets.get(id).is_ok_and(|v| {
                matches!(v.route, DamageRoute::Human { .. }) && v.state == LifeState::Standing
            })
        });
        let result = targets
            .get_mut(request.target)
            .map_err(|_| LifeStateError::MissingTarget)
            .and_then(|mut target| {
                match request.command {
                    LifeCommand::Damage(damage) => {
                        target.apply(damage).map(LifeCommandOutcome::Damage)
                    }
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
                        // Validate target/token before checking current reviver admission.
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
                }
            });
        results.0.push(LifeCommandResult {
            id: request.id,
            target: request.target,
            result,
        });
    }
}

#[cfg(test)]
mod counter_tests {
    use super::*;

    #[test]
    fn episode_exhaustion_is_atomic() {
        let mut player =
            Vitality::human(100, 100, 1, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
        player.episode = u64::MAX - 1;
        player.apply(ResolvedDamage::new(100).unwrap()).unwrap();
        assert_eq!(player.downed_episode().unwrap().number(), u64::MAX);
        let entity = Entity::from_bits(1);
        let token = player
            .begin_revive(
                entity,
                Entity::from_bits(2),
                player.downed_episode().unwrap(),
            )
            .unwrap();
        player
            .complete_revive(entity, token, ReviveRestoration::new(100, 100).unwrap())
            .unwrap();
        let before = player;
        assert_eq!(
            player.apply(ResolvedDamage::new(100).unwrap()),
            Err(LifeStateError::CounterExhausted)
        );
        assert_eq!(player, before);
    }

    #[test]
    fn attempt_exhaustion_does_not_alias_old_token() {
        let mut player =
            Vitality::human(100, 100, 1, PlayerLethalPolicy::DownWithEligibleTeammate).unwrap();
        player.apply(ResolvedDamage::new(100).unwrap()).unwrap();
        player.attempt = u64::MAX - 1;
        let entity = Entity::from_bits(1);
        let token = player
            .begin_revive(
                entity,
                Entity::from_bits(2),
                player.downed_episode().unwrap(),
            )
            .unwrap();
        assert_eq!(token.attempt(), u64::MAX);
        player.cancel_revive(entity, token).unwrap();
        let before = player;
        assert_eq!(
            player.begin_revive(
                entity,
                Entity::from_bits(2),
                player.downed_episode().unwrap()
            ),
            Err(LifeStateError::CounterExhausted)
        );
        assert_eq!(player, before);
        assert_eq!(
            player.complete_revive(entity, token, ReviveRestoration::new(100, 100).unwrap()),
            Err(LifeStateError::StaleAttempt)
        );
    }
}
