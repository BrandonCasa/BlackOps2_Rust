use crate::{
    assets::GameDefinition,
    gameplay::{Navigation, Session, SessionEntity, Zombie},
    life::{LifeState, Vitality},
    player::Player,
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
    #[serde(default = "native_corpse_limit")]
    pub corpse_limit: u8,
    #[serde(default = "native_actor_think_seconds")]
    pub actor_think_seconds: f32,
    pub spawn_seconds: f32,
    pub spawn_decay: f32,
    pub minimum_spawn_seconds: f32,
    pub initial_spawn_seconds: f32,
    pub first_spawn_seconds: f32,
    pub between_round_seconds: f32,
    pub speed_multiplier: u32,
    pub speed_random_band: u32,
    pub gait_thresholds: [u32; 2],
}
fn native_corpse_limit() -> u8 {
    8
}
fn native_actor_think_seconds() -> f32 {
    0.05
}
impl RoundProfile {
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
#[derive(Component)]
pub struct SpawnZone(pub crate::origins::ZoneId);
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
    pub transitions: u32,
    pub intermission_seconds: f32,
    pub admitted: Vec<serde_json::Value>,
}

#[derive(Default)]
pub(crate) struct CharacterSelections {
    session: Option<crate::session::SessionId>,
    groups: std::collections::BTreeMap<String, [u64; 6]>,
    round: u32,
    first_round_started: bool,
}

pub(crate) fn advance(
    session_id: Res<crate::session::SessionId>,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    enemies: Query<
        (
            Entity,
            &Vitality,
            &PopulationKind,
            &Transform,
            Option<&Zombie>,
        ),
        Without<crate::enemies::ActorCorpse>,
    >,
    mut commands: Commands,
    query: SpatialQuery,
    navigation: Res<Navigation>,
    mut population: ResMut<Population>,
    mut evidence: ResMut<RoundEvidence>,
    events: (MessageWriter<RoundStarted>, MessageWriter<RoundEnding>),
    players: Query<(&Player, &Transform)>,
    generators: Res<crate::generators::Generators>,
    replacements: Query<(Entity, &ReplacementHealth)>,
    special: (
        Res<crate::quests::MainQuest>,
        Res<crate::panzer::PanzerWaves>,
        Res<crate::robots::Robots>,
        Local<CharacterSelections>,
        Query<Entity, With<crate::enemies::ActorCorpse>>,
        Res<crate::access::ZoneAccess>,
    ),
) {
    let (mut started, mut ending) = events;
    let (quest, panzer_waves, robots, mut characters, corpses, access) = special;
    if characters.session != Some(*session_id) {
        characters.session = Some(*session_id);
        characters.groups.clear();
        characters.round = 0;
        characters.first_round_started = false;
    }
    if characters.round != session.round {
        characters.round = session.round;
    }
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
    if let Some(timer) = session.intermission.as_mut() {
        timer.tick(time.delta());
        if timer.is_finished() {
            evidence.intermission_seconds = timer.elapsed_secs();
            evidence.transitions += 1;
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
    if session.round_complete(population.ordinary + population.special) && panzer_waves.pending == 0
    {
        ending.write(RoundEnding {
            session: *session_id,
            round: session.round,
        });
        session.round = (session.round + 1).min(255);
        session.pending = profile.population(session.round);
        session.intermission = Some(Timer::from_seconds(
            profile.between_round_seconds,
            TimerMode::Once,
        ));
        return;
    }
    session.spawn_timer.tick(time.delta());
    if session.pending == 0 || !session.spawn_timer.is_finished() {
        return;
    }
    if session.round == 1 && !characters.first_round_started {
        if players.single().is_err() {
            return;
        }
        characters.first_round_started = true;
        started.write(RoundStarted {
            session: *session_id,
            round: session.round,
        });
    }
    let reserve = generators.reserve()
        + usize::from(robots.triple_active) * 2
        + usize::from(quest.enabled && quest.stage == crate::quests::QuestStage::RecoverDrone);
    if population.ordinary + population.special
        >= (profile.ordinary_cap as usize).saturating_sub(reserve)
    {
        evidence.cap_waits += 1;
        session.spawn_timer = Timer::from_seconds(0.1, TimerMode::Once);
        return;
    }
    if enemies.iter().count() + corpses.iter().count() >= profile.actor_cap as usize {
        for entity in &corpses {
            commands.entity(entity).despawn();
        }
        evidence.cap_waits += 1;
        session.spawn_timer = Timer::from_seconds(0.1, TimerMode::Once);
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
        players.single().ok()?;
        let admission = navigation.choose_spawn(&mut session)?;
        let base = admission.position;
        let floor = query.cast_ray(
            base + Vec3::Y * 0.1,
            Dir3::NEG_Y,
            4.,
            false,
            &crate::environment::actor_solids([]),
        )?;
        if floor.normal.y < 0.5 {
            return None;
        }

        let source_enemy = variants.first()?;
        let position = base + crate::enemies::body_center(source_enemy, false);
        if enemies.iter().any(|(_, health, _, body, zombie)| {
            let delta = body.translation - position;
            health.state() == LifeState::ActorAlive
                && ((delta.xz().length_squared() < 0.5625 && delta.y.abs() < 1.)
                    || zombie.is_some_and(|z| z.reserves(position)))
        }) {
            return None;
        }
        let shape = crate::enemies::body_shape(source_enemy, false);
        if !query
            .shape_intersections(
                &shape,
                position,
                Quat::IDENTITY,
                &crate::environment::actor_solids([]),
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
        let archetypes: std::collections::BTreeSet<_> = variants
            .iter()
            .map(|enemy| enemy.archetype.as_str())
            .collect();
        let archetype = *archetypes
            .iter()
            .nth(session.random() as usize % archetypes.len().max(1))?;
        let group = variants
            .iter()
            .find(|enemy| enemy.archetype == archetype)?
            .character_group
            .clone();
        let counts = characters.groups.entry(group).or_default();
        let minimum = *counts.iter().min()?;
        let choices: Vec<_> = counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count == minimum)
            .map(|(index, _)| index)
            .collect();
        let index = *choices.get(session.random() as usize % choices.len())?;
        let choices: Vec<_> = variants
            .iter()
            .filter(|enemy| {
                enemy.archetype == archetype && enemy.character_index == Some(index as u8)
            })
            .collect();
        let enemy = choices.get(session.random() as usize % choices.len().max(1))?;
        let spawners: Vec<_> = definition
            .access
            .spawners
            .iter()
            .filter(|spawner| spawner.archetype == enemy.archetype)
            .collect();
        let spawner = *spawners.get(session.random() as usize % spawners.len().max(1))?;
        let entrance = if admission.find_flesh {
            None
        } else {
            Some(navigation.choose_entrance(admission, spawner, &access, &mut session)?)
        };
        let gait = session
            .difficulty
            .gait(profile, session.round, session.random());
        Some((
            position,
            shape,
            health,
            enemy.id.clone(),
            gait,
            admission.source.clone(),
            admission.zone.clone(),
            admission.rising,
            admission.clone(),
            spawner.clone(),
            entrance,
        ))
    })();
    let Some((
        position,
        shape,
        health,
        identity,
        gait,
        spawn_source,
        spawn_zone,
        rising,
        admission,
        spawner,
        entrance,
    )) = candidate
    else {
        evidence.failed_admissions += 1;
        evidence.budget_consumed_on_failure += u64::from(pending != session.pending);
        return;
    };
    if !session.admitted_spawn() {
        return;
    }
    if let Some(enemy) = definition
        .content
        .enemies
        .iter()
        .find(|enemy| enemy.id == identity)
        && let Some(index) = enemy.character_index
    {
        characters
            .groups
            .entry(enemy.character_group.clone())
            .or_default()[index as usize] += 1;
    }
    if let Some((entity, _)) = replacement {
        commands.entity(entity).despawn();
        evidence.replacement_admissions += 1;
    }
    if evidence.admitted.len() < 256 {
        evidence.admitted.push(serde_json::json!({"round":session.round,"health":health.current(),"maximum_health":health.maximum(),
            "gait":format!("{gait:?}"),"time":time.elapsed_secs_f64(),"position":position.to_array(),"spawn_source":spawn_source,"spawn_zone":spawn_zone,"rising_source":rising,
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
                .and_then(|profiles| profiles.first())
        })
        .flatten();
    let zombie = Zombie::new(position, identity, gait).with_locomotion_seed(session.random());
    let zombie = zombie.source_spawn(&spawner, &admission, entrance, rise.map(|_| 0));
    let mut actor = commands.spawn((
        zombie,
        shape,
        health,
        PopulationKind::Ordinary,
        SpawnZone(spawn_zone),
        SessionEntity,
        Visibility::Hidden,
        avian3d::prelude::ColliderDisabled,
        Transform::from_translation(position),
        Name::new("Original zombie"),
    ));
    if quest.stage == crate::quests::QuestStage::UpgradeFists {
        actor.insert(crate::finale::FistCandidate::default());
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
