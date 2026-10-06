//! Fixed-step attributed impacts and once-only solo combat rewards.
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, Zombie},
    life::{DamageOutcome, ResolvedDamage, Vitality},
    origins::content::{ContentId, DamageCategory, WeaponId},
    player::Player,
    weapons::HitRegion,
};
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub enum ImpactSource {
    Weapon(WeaponId),
    Projectile(WeaponId),
    Enemy(ContentId),
}
impl ImpactSource {
    pub fn id(&self) -> &str {
        match self {
            Self::Weapon(id) | Self::Projectile(id) => &id.0,
            Self::Enemy(id) => &id.0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Impact {
    pub attacker: Entity,
    pub target: Entity,
    pub source: ImpactSource,
    pub category: DamageCategory,
    pub region: HitRegion,
    pub amount: u32,
}
#[derive(Clone, Debug)]
pub struct DamageReceipt {
    pub impact: Impact,
    pub lethal: bool,
    pub points: u32,
}
#[derive(Message, Clone, Debug)]
pub struct EnemyDeath(pub DamageReceipt);
#[derive(Message, Clone, Debug)]
struct DamageApplied {
    impact: Impact,
    lethal: bool,
}
#[derive(Resource, Default)]
pub struct DamageQueue(pub Vec<Impact>);
#[derive(Resource, Default)]
pub struct CombatHistory {
    pub recent: VecDeque<DamageReceipt>,
    pub melee_hits: u32,
    pub melee_kills: u32,
    pub headshot_kills: u32,
    pub rejected_terminal: u32,
    pub awarded_points: u32,
}
pub struct DamagePlugin;
impl Plugin for DamagePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageApplied>()
            .add_message::<EnemyDeath>()
            .init_resource::<DamageQueue>()
            .init_resource::<CombatHistory>()
            .add_systems(
                FixedUpdate,
                resolve
                    .in_set(GameStep::Damage)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (rewards, reward_deaths)
                    .chain()
                    .in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn resolve(
    mut queue: ResMut<DamageQueue>,
    mut history: ResMut<CombatHistory>,
    mut targets: Query<(&mut Vitality, Option<&mut crate::player_health::Recovery>)>,
    players: Query<(), With<Player>>,
    enemies: Query<&Zombie>,
    definition: Res<GameDefinition>,
    mut evidence: ResMut<crate::player_health::HealthEvidence>,
    mut enemy_evidence: ResMut<crate::enemies::EnemyEvidence>,
    mut accepted: MessageWriter<DamageApplied>,
    mut player_damage: MessageWriter<crate::player_health::PlayerDamage>,
    mut player_deaths: MessageWriter<crate::player_health::PlayerDeath>,
) {
    // Resolve weapon combat first: a lethal shot cancels that enemy's queued impact.
    queue
        .0
        .sort_by_key(|impact| matches!(impact.source, ImpactSource::Enemy(_)));
    for impact in queue.0.drain(..) {
        let sender_alive = targets.get(impact.attacker).is_ok_and(|(v, _)| {
            matches!(
                v.state(),
                crate::life::LifeState::Standing | crate::life::LifeState::ActorAlive
            )
        });
        let valid_source = match &impact.source {
            ImpactSource::Weapon(_) | ImpactSource::Projectile(_) => {
                players.contains(impact.attacker)
            }
            ImpactSource::Enemy(id) => enemies
                .get(impact.attacker)
                .is_ok_and(|z| z.definition == *id),
        };
        if (!sender_alive && !matches!(impact.source, ImpactSource::Projectile(_))) || !valid_source
        {
            continue;
        }
        let Ok((mut health, mut recovery)) = targets.get_mut(impact.target) else {
            continue;
        };
        let player = players.contains(impact.target);
        if player && health.state() != crate::life::LifeState::Standing {
            evidence.terminal_rejections += 1;
            continue;
        }
        if player && recovery.as_ref().is_some_and(|r| r.immunity > 0.) {
            evidence.immunity_rejections += 1;
            continue;
        }
        let Ok(amount) = ResolvedDamage::new(impact.amount) else {
            continue;
        };
        let before = health.current().unwrap_or(0);
        let Ok(outcome) = health.apply(amount) else {
            continue;
        };
        if player {
            if matches!(outcome, DamageOutcome::Ignored { .. }) {
                evidence.terminal_rejections += 1;
                continue;
            }
            let terminal = matches!(outcome, DamageOutcome::HumanDied { .. });
            let after = if terminal {
                0
            } else {
                health.current().unwrap_or(0)
            };
            if let Some(recovery) = recovery.as_mut() {
                recovery.hit(before, after, health.maximum(), &definition.player_health);
            }
            if matches!(impact.source, ImpactSource::Enemy(_)) {
                enemy_evidence.attack_hits += 1;
            }
            player_damage.write(crate::player_health::PlayerDamage {
                impact: impact.clone(),
                before,
                after,
                terminal,
            });
            if terminal {
                player_deaths.write(crate::player_health::PlayerDeath {
                    target: impact.target,
                    attacker: impact.attacker,
                });
            }
            continue;
        }
        let lethal = match outcome {
            DamageOutcome::Hurt { .. } => false,
            DamageOutcome::ActorDied { .. } => true,
            _ => {
                history.rejected_terminal = history.rejected_terminal.saturating_add(1);
                continue;
            }
        };
        accepted.write(DamageApplied { impact, lethal });
    }
}
fn rewards(
    mut applied: MessageReader<DamageApplied>,
    mut deaths: MessageWriter<EnemyDeath>,
    mut history: ResMut<CombatHistory>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
) {
    for applied in applied.read() {
        let impact = applied.impact.clone();
        let lethal = applied.lethal;
        let [hit, kill, head, torso, melee] = definition.tuning.combat_points;
        let is_melee = matches!(impact.category, DamageCategory::Melee);
        let bonus = if is_melee {
            melee
        } else if matches!(impact.region, HitRegion::Head) {
            head
        } else if matches!(impact.region, HitRegion::Torso | HitRegion::UpperTorso) {
            torso
        } else {
            0
        };
        // Original death replaces a damage award; it does not add a second hit award.
        let points = if lethal {
            kill.saturating_add(bonus)
        } else {
            hit
        };
        if !lethal {
            session.points = session.points.saturating_add(points);
            history.awarded_points = history.awarded_points.saturating_add(points);
        }
        if is_melee {
            history.melee_hits = history.melee_hits.saturating_add(1);
        }
        if history.recent.len() == 128 {
            history.recent.pop_front();
        }
        let receipt = DamageReceipt {
            impact,
            lethal,
            points,
        };
        if lethal {
            deaths.write(EnemyDeath(receipt.clone()));
        }
        history.recent.push_back(receipt);
    }
}

fn reward_deaths(
    mut deaths: MessageReader<EnemyDeath>,
    mut history: ResMut<CombatHistory>,
    mut session: ResMut<Session>,
) {
    for EnemyDeath(receipt) in deaths.read() {
        session.points = session.points.saturating_add(receipt.points);
        history.awarded_points = history.awarded_points.saturating_add(receipt.points);
        session.kills = session.kills.saturating_add(1);
        if matches!(receipt.impact.category, DamageCategory::Melee) {
            history.melee_kills = history.melee_kills.saturating_add(1);
        } else if matches!(receipt.impact.region, HitRegion::Head) {
            history.headshot_kills = history.headshot_kills.saturating_add(1);
        }
    }
}
