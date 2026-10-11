use crate::{
    assets::GameDefinition,
    gameplay::{Navigation, Session, SessionEntity, Zombie},
    life::{LifeState, Vitality},
    player::{Player, capsule},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Message, Clone, Copy)]
pub struct RoundStarted {
    pub(crate) session: crate::session::SessionId,
    pub round: u32,
}

#[derive(Message, Clone, Copy)]
pub struct RoundEnding {
    pub(crate) session: crate::session::SessionId,
    pub round: u32,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoundProfile {
    pub health_start: u32,
    pub health_increase: u32,
    pub health_multiplier: f64,
    pub exponential_round: u32,
    pub population_base: u32,
    pub solo_population_increment: f64,
    pub population_round_divisor: u32,
    pub population_acceleration_round: u32,
    pub population_acceleration: f64,
    pub early_population: [f64; 5],
    pub ordinary_cap: u32,
    pub actor_cap: u32,
    pub spawn_seconds: f32,
    pub spawn_decay: f32,
    pub minimum_spawn_seconds: f32,
    pub first_spawn_seconds: f32,
    pub between_round_seconds: f32,
    pub speed_multiplier: u32,
    pub speed_random_band: u32,
    pub gait_thresholds: [u32; 2],
    pub gait_speeds: [f32; 3],
}
impl RoundProfile {
    pub fn valid(&self) -> bool {
        self.health_start > 0
            && self.health_start <= i32::MAX as u32
            && self.health_increase > 0
            && self.health_increase <= i32::MAX as u32
            && self.exponential_round >= 2
            && self.population_base > 0
            && self.population_round_divisor > 0
            && self.population_acceleration_round > 0
            && self.ordinary_cap > 0
            && self.actor_cap >= self.ordinary_cap
            && self.speed_random_band > 0
            && self.speed_multiplier > 0
            && self.gait_thresholds[0] < self.gait_thresholds[1]
            && [
                self.health_multiplier,
                self.solo_population_increment,
                self.population_acceleration,
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
            && self
                .early_population
                .iter()
                .all(|v| v.is_finite() && *v > 0. && *v <= 1.)
            && [
                self.spawn_seconds,
                self.spawn_decay,
                self.minimum_spawn_seconds,
                self.first_spawn_seconds,
                self.between_round_seconds,
            ]
            .iter()
            .chain(self.gait_speeds.iter())
            .all(|v| v.is_finite() && *v > 0.)
            && self.spawn_decay <= 1.
            && self.minimum_spawn_seconds <= self.spawn_seconds
            && self.gait_speeds.windows(2).all(|v| v[0] < v[1])
    }
    pub fn health(&self, round: u32) -> u32 {
        let mut health = self.health_start;
        for r in 2..=round.clamp(1, 255) {
            let increment = if r >= self.exponential_round {
                (health as f64 * self.health_multiplier) as u32
            } else {
                self.health_increase
            };
            let Some(next) = health
                .checked_add(increment)
                .filter(|v| *v <= i32::MAX as u32)
            else {
                return health;
            };
            health = next;
        }
        health
    }
    pub fn population(&self, round: u32) -> u32 {
        let round = round.clamp(1, 255);
        let mut multiplier = (round as f64 / self.population_round_divisor as f64).max(1.);
        if round >= self.population_acceleration_round {
            multiplier *= round as f64 * self.population_acceleration;
        }
        let base = self
            .population_base
            .saturating_add((self.solo_population_increment * multiplier) as u32);
        if round <= self.early_population.len() as u32 {
            (base as f64 * self.early_population[(round - 1) as usize]) as u32
        } else {
            base
        }
    }
    pub fn spawn_delay(&self, round: u32) -> f32 {
        let mut delay = self.spawn_seconds;
        for _ in 1..round.clamp(1, 255) {
            delay = if delay > self.minimum_spawn_seconds {
                delay * self.spawn_decay
            } else {
                self.minimum_spawn_seconds
            };
        }
        delay
    }
    pub fn gait(&self, round: u32, random: u32) -> Gait {
        let base = if round <= 1 {
            1
        } else {
            round
                .clamp(1, 255)
                .saturating_sub(1)
                .saturating_mul(self.speed_multiplier)
        };
        let roll = base.saturating_add(random % self.speed_random_band);
        if roll <= self.gait_thresholds[0] {
            Gait::Walk
        } else if roll <= self.gait_thresholds[1] {
            Gait::Run
        } else {
            Gait::Sprint
        }
    }
    pub fn speed(&self, gait: Gait) -> f32 {
        self.gait_speeds[gait as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gait {
    Walk,
    Run,
    Sprint,
}
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopulationKind {
    #[default]
    Ordinary,
    Special,
    GeneratorDefender,
}
#[derive(Resource, Default)]
pub struct Population {
    pub ordinary: usize,
    pub special: usize,
    pub defenders: usize,
}
impl Population {
    pub fn total(&self) -> usize {
        self.ordinary + self.special + self.defenders
    }
}
#[derive(Component)]
pub(crate) struct ReplacementHealth {
    pub current: u32,
    pub sequence: u64,
}

#[derive(Resource, Default)]
pub struct RoundEvidence {
    pub distance_checks: u64,
    pub retired: u64,
    pub requeued: u64,
    pub replacement_admissions: u64,
    pub attempts: u64,
    pub failed_admissions: u64,
    pub budget_consumed_on_failure: u64,
    pub peak_ordinary: usize,
    pub cap_waits: u64,
    pub corpses_cleared_for_cap: u64,
    pub transitions: u32,
    pub intermission_seconds: f32,
    pub admitted: Vec<serde_json::Value>,
}

pub(crate) fn advance(
    session_id: Res<crate::session::SessionId>,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    enemies: Query<(
        Entity,
        &Vitality,
        &PopulationKind,
        &Transform,
        Option<&Zombie>,
    )>,
    mut commands: Commands,
    query: SpatialQuery,
    navigation: Res<Navigation>,
    mut population: ResMut<Population>,
    mut evidence: ResMut<RoundEvidence>,
    events: (MessageWriter<RoundStarted>, MessageWriter<RoundEnding>),
    players: Query<(&Player, &Transform)>,
    generators: Res<crate::generators::Generators>,
    replacements: Query<(Entity, &ReplacementHealth)>,
    quest: Res<crate::quests::MainQuest>,
) {
    let (mut started, mut ending) = events;
    *population = default();
    for (_, vitality, kind, _, _) in &enemies {
        if vitality.state() != LifeState::ActorAlive {
            continue;
        }
        match kind {
            PopulationKind::Ordinary => population.ordinary += 1,
            PopulationKind::Special => population.special += 1,
            PopulationKind::GeneratorDefender => population.defenders += 1,
        }
    }
    evidence.peak_ordinary = evidence.peak_ordinary.max(population.ordinary);
    let profile = &definition.rounds;
    if session.round_complete(population.ordinary + population.special) {
        if quest.round_held() { return; }
        if session.intermission.is_none() {
            ending.write(RoundEnding {
                session: *session_id,
                round: session.round,
            });
        }
        let timer = session.intermission.get_or_insert_with(|| {
            Timer::from_seconds(profile.between_round_seconds, TimerMode::Once)
        });
        timer.tick(time.delta());
        if timer.is_finished() {
            evidence.intermission_seconds = timer.elapsed_secs();
            evidence.transitions += 1;
            session.round = (session.round + 1).min(255);
            session.pending = profile.population(session.round);
            session.intermission = None;
            session.spawn_timer = Timer::from_seconds(profile.first_spawn_seconds, TimerMode::Once);
            started.write(RoundStarted {
                session: *session_id,
                round: session.round,
            });
            info!("Round {} begins, budget {}", session.round, session.pending);
        }
        return;
    }
    session.spawn_timer.tick(time.delta());
    if session.pending == 0 || !session.spawn_timer.is_finished() {
        return;
    }
    let reserve = generators.reserve();
    if population.ordinary + population.special
        >= (profile.ordinary_cap as usize).saturating_sub(reserve)
    {
        evidence.cap_waits += 1;
        return;
    }
    if enemies.iter().count() >= profile.actor_cap as usize {
        for (entity, health, _, _, _) in &enemies {
            if health.state() == LifeState::ActorDead {
                commands.entity(entity).despawn();
                evidence.corpses_cleared_for_cap += 1;
            }
        }
        evidence.cap_waits += 1;
        return;
    }
    session.spawn_timer = Timer::from_seconds(profile.spawn_delay(session.round), TimerMode::Once);
    evidence.attempts += 1;
    let pending = session.pending;
    let variants: Vec<_> = definition
        .content
        .enemies
        .iter()
        .filter(|e| e.family == "german")
        .collect();
    let replacement = replacements
        .iter()
        .min_by_key(|(_, health)| health.sequence);
    let candidate = (|| {
        let (player, body) = players.single().ok()?;
        let forward = Quat::from_rotation_y(player.yaw) * Vec3::NEG_Z;
        let admission = navigation.choose_spawn(&mut session, body.translation, forward)?;
        let base = admission.position;
        let floor = query.cast_ray(
            base + Vec3::Y * 0.1,
            Dir3::NEG_Y,
            4.,
            false,
            &crate::environment::solids([]),
        )?;
        if floor.normal.y < 0.5 {
            return None;
        }
        let position = base + Vec3::Y * 0.85;
        if enemies.iter().any(|(_, health, _, body, zombie)| {
            let delta = body.translation - position;
            health.state() == LifeState::ActorAlive
                && ((delta.xz().length_squared() < 0.5625 && delta.y.abs() < 1.)
                    || zombie.is_some_and(|z| z.reserves(position)))
        }) {
            return None;
        }
        let shape = capsule(0.3, 1.7);
        if !query
            .shape_intersections(
                &shape,
                position,
                Quat::IDENTITY,
                &crate::environment::solids([]),
            )
            .is_empty()
        {
            return None;
        }
        let amount = profile.health(session.round);
        let health = Vitality::ordinary_actor(
            replacement.map_or(amount, |(_, health)| health.current),
            amount,
        )
        .ok()?;
        let enemy = variants.get(session.spawn_cursor % variants.len().max(1))?;
        let gait = session.difficulty.gait(profile, session.round, session.random());
        Some((
            position,
            shape,
            health,
            enemy.id.clone(),
            gait,
            admission.source.clone(),
            admission.zone.clone(),
            admission.rising,
        ))
    })();
    let Some((position, shape, health, identity, gait, spawn_source, spawn_zone, rising)) =
        candidate
    else {
        evidence.failed_admissions += 1;
        evidence.budget_consumed_on_failure += u64::from(pending != session.pending);
        return;
    };
    if !session.admitted_spawn() {
        return;
    }
    if let Some((entity, _)) = replacement {
        commands.entity(entity).despawn();
        evidence.replacement_admissions += 1;
    }
    if evidence.admitted.len() < 256 {
        evidence.admitted.push(serde_json::json!({"round":session.round,"health":health.current(),"maximum_health":health.maximum(),
            "gait":format!("{gait:?}"),"speed":profile.speed(gait),"time":time.elapsed_secs_f64(),"position":position.to_array(),"spawn_source":spawn_source,"spawn_zone":spawn_zone,"rising_source":rising,
            "spawn_delay":profile.spawn_delay(session.round),"pending_before":pending,"pending_after":session.pending}));
    }
    let rise = rising
        .then(|| {
            definition
                .content
                .enemies
                .iter()
                .find(|e| e.id == identity)
                .and_then(|e| e.traversals.get("rise"))
        })
        .flatten();
    let zombie = Zombie::new(position, identity, gait, profile.speed(gait))
        .with_locomotion_seed(session.spawn_cursor as u32);
    let zombie = if let Some(profile) = rise {
        zombie.rising(profile)
    } else {
        zombie
    };
    let mut actor = commands.spawn((
        zombie,
        shape,
        health,
        PopulationKind::Ordinary,
        SessionEntity,
        Visibility::Inherited,
        Transform::from_translation(position),
        Name::new("Original zombie"),
    ));
    if rise.is_some() {
        actor.insert(avian3d::prelude::ColliderDisabled);
    }
}

impl Session {
    pub fn admitted_spawn(&mut self) -> bool {
        if self.pending == 0 {
            return false;
        }
        self.pending -= 1;
        true
    }
    pub fn round_complete(&self, living: usize) -> bool {
        self.pending == 0 && living == 0
    }
}
