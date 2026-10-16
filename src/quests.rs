use crate::{GameState, frontend::Difficulty, gameplay::GameStep};
use avian3d::prelude::*;
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
pub enum Element {
    Fire,
    Ice,
    Wind,
    Lightning,
}

#[derive(Resource, Default)]
pub struct MainQuest {
    pub enabled: bool,
    started: bool,
    pub stage: QuestStage,
    pub upgraded: BTreeSet<Element>,
    pub placed: BTreeSet<Element>,
    strike_remaining: f32,
}

impl MainQuest {
    pub fn reset(difficulty: Difficulty) -> Self {
        Self {
            enabled: difficulty.quests_enabled(),
            ..default()
        }
    }

    pub fn transition(&mut self, expected: QuestStage, next: QuestStage) -> bool {
        if !self.enabled
            || self.stage != expected
            || expected == QuestStage::UpgradeStaffs && !self.started
        {
            return false;
        }
        let valid = matches!(
            (expected, next),
            (QuestStage::UpgradeStaffs, QuestStage::PlaceStaffs)
                | (QuestStage::PlaceStaffs, QuestStage::BreakSeal)
                | (QuestStage::BreakSeal, QuestStage::PanzerEncounter)
                | (QuestStage::PanzerEncounter, QuestStage::RecoverDrone)
                | (QuestStage::RecoverDrone, QuestStage::UpgradeFists)
                | (QuestStage::UpgradeFists, QuestStage::Finale)
                | (QuestStage::Finale, QuestStage::Complete)
        );
        if !valid {
            return false;
        }
        self.stage = next;
        self.strike_remaining = 0.;
        true
    }

    pub(crate) fn update_prerequisites(&mut self, staffs_crafted: bool, generators_powered: bool) {
        // sidequest_logic waits for all crafted staffs, then all_zones_captured.
        // Once step 1 begins, later generator raids do not restart that wait.
        self.started |= self.enabled && staffs_crafted && generators_powered;
    }

    pub fn authorize_strike(&mut self) -> bool {
        if !self.enabled || self.stage != QuestStage::BreakSeal {
            return false;
        }
        self.strike_remaining = 25.;
        true
    }

    pub fn strike_authorized(&self) -> bool {
        self.enabled && self.stage == QuestStage::BreakSeal && self.strike_remaining > 0.
    }
}

#[cfg(test)]
mod main_quest_tests {
    use super::*;

    #[test]
    fn staff_placement_waits_for_initial_prerequisites_and_survives_later_raids() {
        let mut quest = MainQuest {
            enabled: true,
            ..default()
        };
        quest.update_prerequisites(false, true);
        assert!(!quest.transition(QuestStage::UpgradeStaffs, QuestStage::PlaceStaffs));
        quest.update_prerequisites(true, false);
        assert!(!quest.transition(QuestStage::UpgradeStaffs, QuestStage::PlaceStaffs));
        quest.update_prerequisites(true, true);
        quest.update_prerequisites(true, false);
        assert!(quest.transition(QuestStage::UpgradeStaffs, QuestStage::PlaceStaffs));
        assert!(quest.transition(QuestStage::PlaceStaffs, QuestStage::BreakSeal));
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

#[derive(Component)]
struct SoulTransit {
    delay: f32,
    elapsed: f32,
    duration: f32,
    from: Vec3,
    to: Vec3,
    fx: Option<Entity>,
    arrived: bool,
}

pub(crate) fn soul_effect(
    commands: &mut Commands,
    from: Vec3,
    to: Vec3,
    delay: f32,
    duration: f32,
) {
    commands.spawn((
        crate::gameplay::SessionEntity,
        Transform::from_translation(from),
        SoulTransit {
            delay,
            elapsed: 0.,
            duration,
            from,
            to,
            fx: None,
            arrived: false,
        },
    ));
}

fn soul_effects(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut souls: Query<(Entity, &mut Transform, &mut SoulTransit)>,
) {
    for (entity, mut pose, mut soul) in &mut souls {
        soul.delay -= time.delta_secs();
        if soul.delay > 0. {
            continue;
        }
        if soul.fx.is_none() && !soul.arrived {
            soul.fx = Some(crate::native_fx::attach(
                &mut commands,
                "weapon/zmb_staff/fx_zmb_staff_charge_souls",
                entity,
                Transform::IDENTITY,
            ));
            crate::audio::emit(
                &mut commands,
                "zmb_squest_charge_soul_leave",
                crate::audio::SoundSource::attached(entity),
            );
        }
        soul.elapsed += time.delta_secs();
        pose.translation = soul
            .from
            .lerp(soul.to, (soul.elapsed / soul.duration).clamp(0., 1.));
        if !soul.arrived && soul.elapsed >= soul.duration {
            soul.arrived = true;
            if let Some(fx) = soul.fx.take() {
                commands.entity(fx).try_despawn();
            }
            crate::native_fx::emit(
                &mut commands,
                "weapon/zmb_staff/fx_zmb_staff_charge",
                Transform::from_translation(soul.to),
            );
            crate::audio::emit(
                &mut commands,
                "zmb_squest_charge_soul_impact",
                crate::audio::SoundSource::World(soul.to),
            );
        }
        if soul.elapsed >= soul.duration + 0.3 {
            commands.entity(entity).despawn();
        }
    }
}

pub struct QuestPlugin;

impl Plugin for QuestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MainQuest>()
            .add_message::<SoulCollected>()
            .add_systems(
                FixedUpdate,
                soul_effects
                    .in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                strike_window
                    .before(crate::rounds::advance)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                souls
                    .in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .after(crate::rituals::track)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn strike_window(time: Res<Time<Fixed>>, mut quest: ResMut<MainQuest>) {
    quest.strike_remaining = (quest.strike_remaining - time.delta_secs()).max(0.);
}

pub(crate) fn souls(
    mut commands: Commands,
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    enemies: Query<(&Transform, &Collider, &crate::rounds::PopulationKind)>,
    mut collectors: Query<(Entity, &Transform, &mut SoulCollector)>,
    mut collected: MessageWriter<SoulCollected>,
    quest: Res<MainQuest>,
    upgrades: Res<crate::staff_upgrades::StaffUpgrades>,
    mut finale: ResMut<crate::finale::Finale>,
    mut rituals: ResMut<crate::rituals::Rituals>,
    spatial: SpatialQuery,
    game: Res<crate::assets::GameDefinition>,
    volumes: Query<(
        &crate::environment::triggers::MapTrigger,
        &crate::environment::triggers::TriggerState,
    )>,
    mut boxes: Query<(
        Entity,
        &crate::rituals::SoulChest,
        &mut crate::rituals::SoulBoxGlow,
    )>,
) {
    for death in deaths.read() {
        if !death.0.can_supply_soul() {
            continue;
        }
        let Ok((pose, shape, kind)) = enemies.get(death.0.impact.target) else {
            continue;
        };
        // zm_tomb_main_quest::zombie_killed_override dispatches one soul:
        // active finale first, unfinished footprint chest next, then staff.
        if finale.collect_soul(&mut commands, &quest, &upgrades, pose.translation) {
            continue;
        }
        if rituals.chests < 4 {
            let touched = spatial.shape_intersections(
                shape,
                pose.translation,
                pose.rotation,
                &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER),
            );
            if crate::rituals::collect_soul(
                &mut commands,
                &mut rituals,
                pose,
                &touched,
                &volumes,
                &game,
                &mut boxes,
            ) {
                continue;
            }
        }
        let selected = collectors
            .iter()
            .filter(|(_, center, collector)| {
                collector.active
                    && collector.collected < collector.required
                    && (!collector.ordinary_only
                        || *kind == crate::rounds::PopulationKind::Ordinary)
                    && center
                        .translation
                        .xz()
                        .distance_squared(pose.translation.xz())
                        <= collector.radius * collector.radius
            })
            .min_by(|(_, a, _), (_, b, _)| {
                a.translation
                    .xz()
                    .distance_squared(pose.translation.xz())
                    .total_cmp(&b.translation.xz().distance_squared(pose.translation.xz()))
            })
            .map(|(entity, _, _)| entity);
        let Some(entity) = selected else { continue };
        let Ok((_, _, mut collector)) = collectors.get_mut(entity) else {
            continue;
        };
        collector.collected = collector
            .collected
            .saturating_add(1)
            .min(collector.required);
        let completed = collector.collected == collector.required;
        if completed {
            collector.active = false;
        }
        collected.write(SoulCollected {
            collector: entity,
            enemy: death.0.impact.target,
            player: death.0.impact.attacker,
            completed,
        });
    }
}
