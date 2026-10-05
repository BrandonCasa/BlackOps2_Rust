#![forbid(unsafe_code)]

//! Resolved damage only. Native player/actor arithmetic remains explicit inference.
//! See docs/reverse_engineering/damage-life-state-01.md for evidence and limits.

use std::num::NonZeroU32;

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageError {
    ZeroMaximum,
    InvalidCurrent,
    ZeroDamage,
    InvalidDownedHealth,
    UnsupportedPlayerPolicy,
    MissingTarget,
    NonIncreasingRequestId,
}

impl std::fmt::Display for DamageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for DamageError {}

/// Integer admission is a project restriction; original player conversion is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedDamage(NonZeroU32);

impl ResolvedDamage {
    pub fn new(amount: u32) -> Result<Self, DamageError> {
        NonZeroU32::new(amount)
            .map(Self)
            .ok_or(DamageError::ZeroDamage)
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
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vitality {
    current: u32,
    maximum: NonZeroU32,
    state: LifeState,
    route: DamageRoute,
}

impl Vitality {
    fn initial(
        current: u32,
        maximum: u32,
        route: DamageRoute,
        state: LifeState,
    ) -> Result<Self, DamageError> {
        let maximum = NonZeroU32::new(maximum).ok_or(DamageError::ZeroMaximum)?;
        if current == 0 || current > maximum.get() {
            return Err(DamageError::InvalidCurrent);
        }
        Ok(Self {
            current,
            maximum,
            state,
            route,
        })
    }

    pub fn human(
        current: u32,
        maximum: u32,
        downed_health: u32,
        policy: PlayerLethalPolicy,
    ) -> Result<Self, DamageError> {
        let downed_health =
            NonZeroU32::new(downed_health).ok_or(DamageError::InvalidDownedHealth)?;
        if downed_health.get() > maximum {
            return Err(DamageError::InvalidDownedHealth);
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

    pub fn ordinary_actor(current: u32, maximum: u32) -> Result<Self, DamageError> {
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
    pub fn apply(&mut self, damage: ResolvedDamage) -> Result<DamageOutcome, DamageError> {
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
            } => Err(DamageError::UnsupportedPlayerPolicy),
            DamageRoute::Human { downed_health, .. } => {
                self.current = downed_health.get();
                self.state = LifeState::Downed;
                Ok(DamageOutcome::Downed {
                    before,
                    after: self.current,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageOutcome {
    Hurt { before: u32, after: u32 },
    Downed { before: u32, after: u32 },
    ActorDied { before: u32 },
    Ignored { state: LifeState },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageRequest {
    pub id: u64,
    pub target: Entity,
    pub damage: ResolvedDamage,
}

/// Explicit caller order: IDs strictly increase for this resource's lifetime.
/// Duplicate or reordered submissions are rejected without changing admission.
/// Submit before FixedUpdate; producer systems must define their own ordering.
#[derive(Resource, Default)]
pub struct DamageQueue {
    last_id: Option<u64>,
    pending: Vec<DamageRequest>,
}

impl DamageQueue {
    pub fn submit(&mut self, request: DamageRequest) -> Result<(), DamageError> {
        if self.last_id.is_some_and(|last| request.id <= last) {
            return Err(DamageError::NonIncreasingRequestId);
        }
        self.last_id = Some(request.id);
        self.pending.push(request);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageResult {
    pub id: u64,
    pub target: Entity,
    pub result: Result<DamageOutcome, DamageError>,
}

/// Consumers drain after the fixed step; results persist across empty steps.
#[derive(Resource, Default)]
pub struct DamageResults(Vec<DamageResult>);

impl DamageResults {
    pub fn drain(&mut self) -> impl Iterator<Item = DamageResult> + '_ {
        self.0.drain(..)
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApplyResolvedDamage;

pub struct ResolvedDamagePlugin;

impl Plugin for ResolvedDamagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DamageQueue>()
            .init_resource::<DamageResults>()
            .add_systems(FixedUpdate, apply_queued_damage.in_set(ApplyResolvedDamage));
    }
}

fn apply_queued_damage(
    mut queue: ResMut<DamageQueue>,
    mut results: ResMut<DamageResults>,
    mut targets: Query<&mut Vitality>,
) {
    for request in queue.pending.drain(..) {
        let result = targets
            .get_mut(request.target)
            .map_err(|_| DamageError::MissingTarget)
            .and_then(|mut target| target.apply(request.damage));
        results.0.push(DamageResult {
            id: request.id,
            target: request.target,
            result,
        });
    }
}
