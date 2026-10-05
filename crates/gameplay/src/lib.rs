#![forbid(unsafe_code)]

//! Resolved damage, admitted coop revival and bleedout disposition. Ordinary positive
//! player subtraction has static evidence and one scoped live comparison. Native
//! actor subtraction is qualified before notifications; final ActorDead health is unknown.
//! Active original dispatch is unqualified; revival configuration is external.
//! Coop timing admission is a separate Bevy clock; terminal human health is
//! preserved pending qualification.
//! See docs/reverse_engineering/damage-life-state-{01,06,10,14,15}.md and
//! docs/fidelity/damage-life-state-comparison.md for evidence and limits.

pub mod revive_clock;
pub use revive_clock::{
    ReviveClock, ReviveClockError, ReviveDecision, ReviveEligibility, ReviveTiming,
};

pub mod bleedout;
pub use bleedout::{BleedoutClock, BleedoutClockError, BleedoutTiming};

use std::num::NonZeroU32;

use bevy::prelude::*;

const MAX_POSITIVE_NATIVE_INTEGER: u32 = i32::MAX.unsigned_abs();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeStateError {
    ZeroMaximum,
    InvalidCurrent,
    ZeroDamage,
    InvalidDownedHealth,
    /// A supplied current, maximum or downed health exceeds i32::MAX.
    HealthOutOfRange,
    /// A supplied resolved damage amount exceeds i32::MAX.
    DamageOutOfRange,
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

/// Admits only positive signed-32-bit integer payloads, matching the inspected
/// native player entry. Upstream conversion and active VM dispatch remain unqualified.
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
    /// Caller has established another eligible human teammate remains alive.
    DownWithEligibleTeammate,
    /// Session/game-end behavior must be resolved by a later task.
    SoloOrLastEligiblePlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeState {
    Standing,
    Downed,
    /// No longer damageable/revivable; numeric health is not an alive predicate.
    /// Spectator/session assembly and final native health effects are outside this state.
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

/// Private fields keep life-state/health invariants together without a registry.
/// Admitted current/maximum/downed health is in 1..=i32::MAX. ActorDead has
/// unresolved final current health; its signed subtraction stage is separate.
/// Before removing/replacing this component on a living Entity, assembly must
/// retire pending commands and all externally held tokens/episode references.
/// Bevy entity generations do not change when components are replaced. Counters
/// apply only to this continuous component lifetime, not a replacement's lifetime.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vitality {
    // Last modeled positive health; retained on ActorDead, never exposed as final.
    current: u32,
    maximum: NonZeroU32,
    state: LifeState,
    route: DamageRoute,
    episode: u64,
    attempt: u64,
    active_revive: Option<ReviveToken>,
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

    /// Current modeled health is unknown after actor terminal routing. No numeric
    /// zero or subtraction-stage value is substituted for post-callback health.
    pub fn current(self) -> Option<u32> {
        (self.state != LifeState::ActorDead).then_some(self.current)
    }

    /// Last modeled direct actor subtraction, before notification/callback effects.
    /// It is not a measurement or the actor's final post-callback current health.
    pub fn actor_subtraction(self) -> Option<ActorSubtraction> {
        self.actor_subtraction
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
    /// Actor subtraction models native14's direct instruction before notifications.
    /// Caller admits ordinary lethal routing; final actor health remains unresolved.
    /// No later -999 floor is applied across unqualified notification side effects.
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
                    return Err(LifeStateError::UnsupportedPlayerPolicy);
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

    pub fn downed_episode(self) -> Option<DownedEpisode> {
        (self.state == LifeState::Downed).then_some(DownedEpisode(self.episode))
    }

    /// Bind an externally resolved expiry to this Entity and downed episode.
    /// This creates no timer or scheduled retry. Retire it before component replacement.
    pub fn downed_expiry(self, target: Entity) -> Result<DownedExpiry, LifeStateError> {
        let episode = DownedEpisode(self.episode);
        self.check_episode(episode)?;
        Ok(DownedExpiry { target, episode })
    }

    /// Caller admits ordinary coop expiry timing. The source notification/network-frame
    /// interval is collapsed into this resolved transition, an explicit approximation.
    /// Active revival defers without mutation or retry scheduling. Current/max remain
    /// the last qualified values; native spectator-spawn health is not established.
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
        // A successful revive retires this episode's expiry even before redown.
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

/// Episode and Entity generation bind expiry to one continuous component lifetime.
/// Timing/coop admission is external; this is not a deadline or raw timer.
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
    /// Producer may retain/retry this input with a new request ID after cancellation.
    Deferred(DownedExpiry),
    Terminated {
        current: u32,
        maximum: u32,
    },
    /// The same episode is already terminal; no state or health changed.
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

/// Model of the qualified direct SUB instruction, before any native notification.
/// Negative/zero after values are not final health or an automatic -999 floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorSubtraction {
    pub before: u32,
    pub amount: u32,
    pub after: i32,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeCommandOutcome {
    Damage(DamageOutcome),
    Expiry(ExpiryOutcome),
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
    fn terminal_transition_needs_no_counter_increment_or_health_arithmetic() {
        let mut player = Vitality::human(
            i32::MAX.unsigned_abs(),
            i32::MAX.unsigned_abs(),
            i32::MAX.unsigned_abs(),
            PlayerLethalPolicy::DownWithEligibleTeammate,
        )
        .unwrap();
        player.episode = u64::MAX - 1;
        player.attempt = u64::MAX - 1;
        player
            .apply(ResolvedDamage::new(i32::MAX.unsigned_abs()).unwrap())
            .unwrap();
        let target = Entity::from_bits(1);
        let expiry = player.downed_expiry(target).unwrap();
        let token = player
            .begin_revive(target, Entity::from_bits(2), expiry.episode())
            .unwrap();
        assert_eq!(
            (expiry.episode().number(), token.attempt()),
            (u64::MAX, u64::MAX)
        );
        player.cancel_revive(target, token).unwrap();
        assert_eq!(
            player.expire_downed(target, expiry),
            Ok(ExpiryOutcome::Terminated {
                current: i32::MAX.unsigned_abs(),
                maximum: i32::MAX.unsigned_abs()
            })
        );
        let terminal = player;
        assert_eq!(
            player.expire_downed(target, expiry),
            Ok(ExpiryOutcome::Ignored)
        );
        assert_eq!(player, terminal);
    }

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
