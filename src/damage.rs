use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, Zombie},
    life::{DamageOutcome, ResolvedDamage, Vitality},
    origins::content::{ContentId, DamageCategory, WeaponId},
    player::Player,
    weapons::HitRegion,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(SystemParam)]
struct Attribution<'w, 's> {
    players: Query<'w, 's, &'static crate::weapons::WeaponInventory, With<Player>>,
    emitters: Query<'w, 's, &'static crate::environment::support::DamageEmitter>,
    diggers: Query<'w, 's, &'static crate::environment::digging::Digger>,
}

#[derive(Clone, Debug)]
pub enum ImpactSource {
    Weapon(WeaponId),
    Projectile(WeaponId),
    Enemy(ContentId),
    Fall,
    Environment(String),
    Robot(String),
    Vehicle(String),
    OutOfBounds,
    Powerup,
    Perk(crate::origins::PerkId),
}
impl ImpactSource {
    pub fn id(&self) -> &str {
        match self {
            Self::Weapon(id) | Self::Projectile(id) => &id.0,
            Self::Enemy(id) => &id.0,
            Self::Powerup => "powerup.nuke",
            Self::Fall => "hazard.fall",
            Self::Environment(id) | Self::Robot(id) | Self::Vehicle(id) => id,
            Self::OutOfBounds => "hazard.out_of_bounds",
            Self::Perk(id) => &id.0,
        }
    }
    fn is_cherry(&self, definition: &GameDefinition) -> bool {
        matches!(self, Self::Perk(id) if definition.content.perks.iter().any(|perk| perk.id == *id && perk.cherry.is_some()))
    }
    pub fn player_attributed(&self) -> bool {
        matches!(
            self,
            Self::Weapon(_) | Self::Projectile(_) | Self::Perk(_) | Self::Powerup
        )
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
    pub base_amount: u32,
    pub lethal: bool,
    pub points: u32,
    pub aimed_bullet: bool,
    pub insta_kill: bool,
}
impl DamageReceipt {
    pub fn can_supply_soul(&self) -> bool {
        self.lethal
            && self.impact.source.player_attributed()
            && !matches!(self.impact.source, ImpactSource::Powerup)
    }
}
#[derive(Message, Clone, Debug)]
pub struct EnemyDeath(pub DamageReceipt);
#[derive(Message, Clone, Debug)]
pub(crate) struct DamageApplied {
    pub(crate) impact: Impact,
    pub(crate) base_amount: u32,
    pub(crate) lethal: bool,
    pub(crate) aimed_bullet: bool,
    pub(crate) insta_kill: bool,
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
    attribution: Attribution,
    enemies: Query<&Zombie>,
    definition: Res<GameDefinition>,
    mut evidence: ResMut<crate::player_health::HealthEvidence>,
    mut enemy_evidence: ResMut<crate::enemies::EnemyEvidence>,
    mut accepted: MessageWriter<DamageApplied>,
    mut player_damage: MessageWriter<crate::player_health::PlayerDamage>,
    mut player_deaths: MessageWriter<crate::player_health::PlayerDeath>,
    mut downed: MessageWriter<crate::solo_revive::PlayerDowned>,
    perks: Query<&crate::solo_revive::PerkInventory>,
    mut perk_evidence: ResMut<crate::perk_effects::EffectEvidence>,
    effects: Res<crate::powerups::PowerupEffects>,
    nuked: Query<(), With<crate::powerups::NukeTarget>>,
) {
    let Attribution {
        players,
        emitters,
        diggers,
    } = attribution;
    queue
        .0
        .sort_by_key(|impact| matches!(impact.source, ImpactSource::Enemy(_)));
    for mut impact in queue.0.drain(..) {
        let sender_alive = targets.get(impact.attacker).is_ok_and(|(v, _)| {
            matches!(
                v.state(),
                crate::life::LifeState::Standing
                    | crate::life::LifeState::Downed
                    | crate::life::LifeState::ActorAlive
            )
        });
        let valid_source = match &impact.source {
            ImpactSource::Powerup
            | ImpactSource::Weapon(_)
            | ImpactSource::Projectile(_)
            | ImpactSource::Perk(_)
            | ImpactSource::Environment(_)
            | ImpactSource::OutOfBounds
            | ImpactSource::Fall => players.contains(impact.attacker),
            ImpactSource::Enemy(id) => enemies
                .get(impact.attacker)
                .is_ok_and(|z| z.definition == *id),
            ImpactSource::Robot(_) | ImpactSource::Vehicle(_) => emitters
                .get(impact.attacker)
                .is_ok_and(|e| e.matches(&impact.source)),
        };
        if (!sender_alive
            && !matches!(
                impact.source,
                ImpactSource::Projectile(_)
                    | ImpactSource::Powerup
                    | ImpactSource::Robot(_)
                    | ImpactSource::Vehicle(_)
            ))
            || !valid_source
        {
            continue;
        }
        let Ok((mut health, mut recovery)) = targets.get_mut(impact.target) else {
            continue;
        };
        let player = players.contains(impact.target);
        if player
            && matches!(impact.source, ImpactSource::Robot(_))
            && diggers.get(impact.target).is_ok_and(|d| d.helmet)
        {
            continue;
        }
        let out_of_bounds = matches!(impact.source, ImpactSource::OutOfBounds);
        if player
            && health.state() != crate::life::LifeState::Standing
            && !(out_of_bounds && health.state() == crate::life::LifeState::Downed)
        {
            evidence.terminal_rejections += 1;
            continue;
        }
        if player
            && matches!(impact.source, ImpactSource::Fall)
            && perks.get(impact.target).is_ok_and(|owned| {
                definition
                    .content
                    .perks
                    .iter()
                    .any(|p| p.fall_protection && owned.owned.contains(&p.id))
            })
        {
            perk_evidence
                .record(serde_json::json!({"event":"fall_protected","amount":impact.amount}));
            continue;
        }
        if player && !out_of_bounds && recovery.as_ref().is_some_and(|r| r.immunity > 0.) {
            evidence.immunity_rejections += 1;
            continue;
        }
        if player
            && matches!(impact.category, DamageCategory::Explosive)
            && perks.get(impact.target).is_ok_and(|owned| {
                definition
                    .content
                    .perks
                    .iter()
                    .any(|p| p.explosive_protection && owned.owned.contains(&p.id))
            })
        {
            perk_evidence.record(serde_json::json!({"event":"explosive_protected","target":impact.target.to_bits(),"amount":impact.amount,"source":impact.source.id()}));
            continue;
        }
        let base_amount = impact.amount;
        let before = health.current().unwrap_or(0);
        let aimed_bullet = matches!(impact.category, DamageCategory::Bullet)
            && players
                .get(impact.attacker)
                .is_ok_and(|inventory| inventory.ads > 0.5);
        let cherry = impact.source.is_cherry(&definition);
        let insta_kill = !player
            && sender_alive
            && base_amount > 0
            && base_amount < before
            && effects.insta_kill > 0.
            && (matches!(
                impact.source,
                ImpactSource::Weapon(_) | ImpactSource::Projectile(_)
            ) || cherry)
            && (!nuked.contains(impact.target) || (!cherry && aimed_bullet));
        if insta_kill {
            impact.amount = before;
        }
        let Ok(amount) = ResolvedDamage::new(impact.amount) else {
            continue;
        };
        let Ok(outcome) = (if out_of_bounds {
            health.kill_human()
        } else {
            health.apply(amount)
        }) else {
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
            if let DamageOutcome::Downed { episode, .. } = outcome {
                downed.write(crate::solo_revive::PlayerDowned {
                    target: impact.target,
                    attacker: impact.attacker,
                    episode,
                });
            }
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
        accepted.write(DamageApplied {
            impact,
            base_amount,
            lethal,
            aimed_bullet,
            insta_kill,
        });
    }
}
fn rewards(
    mut applied: MessageReader<DamageApplied>,
    mut deaths: MessageWriter<EnemyDeath>,
    mut history: ResMut<CombatHistory>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    effects: Res<crate::powerups::PowerupEffects>,
    nuked: Query<(), With<crate::powerups::NukeTarget>>,
) {
    for applied in applied.read() {
        let impact = applied.impact.clone();
        let lethal = applied.lethal;
        let [hit, kill, head, torso, melee] = definition.tuning.combat_points;
        let is_melee = matches!(impact.category, DamageCategory::Melee);
        let cherry = impact.source.is_cherry(&definition);
        let bonus = if cherry {
            0
        } else if is_melee {
            melee
        } else if matches!(impact.region, HitRegion::Head) {
            head
        } else if matches!(impact.region, HitRegion::Torso | HitRegion::UpperTorso) {
            torso
        } else {
            0
        };
        let points = if !impact.source.player_attributed()
            || matches!(impact.source, ImpactSource::Powerup)
        {
            0
        } else if nuked.contains(impact.target) {
            if !cherry && applied.aimed_bullet && (!lethal || applied.insta_kill) {
                hit
            } else {
                0
            }
        } else if lethal {
            kill.saturating_add(bonus)
                .saturating_add(if applied.insta_kill { hit } else { 0 })
        } else {
            hit
        };
        let cherry_bonus = if cherry && lethal && !applied.insta_kill {
            40
        } else {
            0
        };
        let points = points
            .saturating_mul(effects.multiplier())
            .saturating_add(cherry_bonus);
        if !lethal {
            session.award(points);
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
            base_amount: applied.base_amount,
            lethal,
            points,
            aimed_bullet: applied.aimed_bullet,
            insta_kill: applied.insta_kill,
        };
        if lethal {
            deaths.write(EnemyDeath(receipt.clone()));
        }
        history.recent.push_back(receipt);
    }
}

pub(crate) fn reward_deaths(
    mut deaths: MessageReader<EnemyDeath>,
    mut history: ResMut<CombatHistory>,
    mut session: ResMut<Session>,
) {
    for EnemyDeath(receipt) in deaths.read() {
        if !receipt.impact.source.player_attributed()
            || matches!(receipt.impact.source, ImpactSource::Powerup)
        {
            continue;
        }
        session.award(receipt.points);
        history.awarded_points = history.awarded_points.saturating_add(receipt.points);
        session.kills = session.kills.saturating_add(1);
        if matches!(receipt.impact.category, DamageCategory::Melee) {
            history.melee_kills = history.melee_kills.saturating_add(1);
        } else if matches!(receipt.impact.region, HitRegion::Head) {
            history.headshot_kills = history.headshot_kills.saturating_add(1);
        }
    }
}
