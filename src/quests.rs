use crate::{GameState, frontend::Difficulty, gameplay::GameStep};
use bevy::prelude::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum QuestStage {
    #[default]
    UpgradeStaffs,
    PlaceStaffs,
    BreakSeal,
    PanzerEncounter,
    RecoverDrone,
    UpgradeFists,
    Finale,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Element { Fire, Ice, Wind, Lightning }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestTimerKind { StrikeAuthorization }

pub struct QuestTimer {
    pub kind: QuestTimerKind,
    pub stage: QuestStage,
    pub remaining: f32,
}

#[derive(Resource, Default)]
pub struct MainQuest {
    pub enabled: bool,
    pub stage: QuestStage,
    pub upgraded: BTreeSet<Element>,
    pub placed: BTreeSet<Element>,
    pub timers: Vec<QuestTimer>,
    pub round_holds: BTreeSet<QuestStage>,
}

impl MainQuest {
    pub fn reset(difficulty: Difficulty) -> Self {
        Self { enabled: difficulty.quests_enabled(), ..default() }
    }

    pub fn transition(&mut self, expected: QuestStage, next: QuestStage) -> bool {
        if !self.enabled || self.stage != expected { return false; }
        let valid = matches!((expected, next),
            (QuestStage::UpgradeStaffs, QuestStage::PlaceStaffs) |
            (QuestStage::PlaceStaffs, QuestStage::BreakSeal) |
            (QuestStage::BreakSeal, QuestStage::PanzerEncounter) |
            (QuestStage::PanzerEncounter, QuestStage::RecoverDrone) |
            (QuestStage::RecoverDrone, QuestStage::UpgradeFists) |
            (QuestStage::UpgradeFists, QuestStage::Finale) |
            (QuestStage::Finale, QuestStage::Complete));
        if !valid { return false; }
        self.stage = next;
        self.timers.clear();
        self.round_holds.clear();
        true
    }

    pub fn authorize_strike(&mut self) -> bool {
        if !self.enabled || self.stage != QuestStage::BreakSeal { return false; }
        self.timers.retain(|timer| timer.kind != QuestTimerKind::StrikeAuthorization);
        self.timers.push(QuestTimer {
            kind: QuestTimerKind::StrikeAuthorization,
            stage: self.stage,
            remaining: 25.,
        });
        true
    }

    pub fn strike_authorized(&self) -> bool {
        self.enabled && self.stage == QuestStage::BreakSeal && self.timers.iter().any(|timer|
            timer.stage == self.stage && timer.kind == QuestTimerKind::StrikeAuthorization && timer.remaining > 0.)
    }

    pub fn hold_round(&mut self, stage: QuestStage) -> bool {
        if !self.enabled || self.stage != stage { return false; }
        self.round_holds.insert(stage)
    }

    pub fn round_held(&self) -> bool {
        self.enabled && self.round_holds.contains(&self.stage)
    }
}

#[derive(Component)]
pub struct SoulCollector {
    pub radius: f32,
    pub collected: u32,
    pub required: u32,
    pub active: bool,
    pub ordinary_only: bool,
}

#[derive(Message, Clone, Copy)]
pub struct SoulCollected {
    pub collector: Entity,
    pub enemy: Entity,
    pub player: Entity,
    pub completed: bool,
}

pub struct QuestPlugin;

impl Plugin for QuestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MainQuest>()
            .add_message::<SoulCollected>()
            .add_systems(FixedUpdate, timers
                .before(crate::rounds::advance)
                .run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, souls
                .in_set(GameStep::Rewards)
                .after(crate::damage::reward_deaths)
                .run_if(in_state(GameState::Playing)));
    }
}

fn timers(time: Res<Time<Fixed>>, mut quest: ResMut<MainQuest>) {
    let stage = quest.stage;
    for timer in &mut quest.timers { timer.remaining -= time.delta_secs(); }
    quest.timers.retain(|timer| timer.stage == stage && timer.remaining > 0.);
    quest.round_holds.retain(|owner| *owner == stage);
}

pub(crate) fn souls(
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    enemies: Query<(&Transform, &crate::rounds::PopulationKind)>,
    mut collectors: Query<(Entity, &Transform, &mut SoulCollector)>,
    mut collected: MessageWriter<SoulCollected>,
) {
    for death in deaths.read() {
        if !death.0.can_supply_soul() { continue; }
        let Ok((pose, kind)) = enemies.get(death.0.impact.target) else { continue };
        let selected = collectors.iter()
            .filter(|(_, center, collector)| collector.active && collector.collected < collector.required
                && (!collector.ordinary_only || *kind == crate::rounds::PopulationKind::Ordinary)
                && center.translation.distance_squared(pose.translation) <= collector.radius * collector.radius)
            .min_by(|(_, a, _), (_, b, _)| a.translation.distance_squared(pose.translation)
                .total_cmp(&b.translation.distance_squared(pose.translation)))
            .map(|(entity, _, _)| entity);
        let Some(entity) = selected else { continue };
        let Ok((_, _, mut collector)) = collectors.get_mut(entity) else { continue };
        collector.collected = collector.collected.saturating_add(1).min(collector.required);
        let completed = collector.collected == collector.required;
        if completed { collector.active = false; }
        collected.write(SoulCollected { collector: entity, enemy: death.0.impact.target,
            player: death.0.impact.attacker, completed });
    }
}
