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

#[derive(SystemParam)]
pub(crate) struct Attribution<'w, 's> {
    players: Query<'w, 's, &'static crate::weapons::WeaponInventory, With<Player>>,
    emitters: Query<'w, 's, &'static crate::environment::support::DamageEmitter>,
    diggers: Query<'w, 's, &'static crate::environment::digging::Digger>,
    shields: Query<
        'w,
        's,
        (
            &'static Player,
            &'static Transform,
            &'static mut crate::crafting::BuildableInventory,
        ),
    >,
    positions: Query<'w, 's, &'static Transform>,
}

#[derive(Clone, Debug)]
pub enum ImpactSource {
    Weapon(WeaponId),
    Projectile(WeaponId),
    Status(WeaponId),
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
            Self::Weapon(id) | Self::Projectile(id) | Self::Status(id) => &id.0,
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
            Self::Weapon(_) | Self::Projectile(_) | Self::Status(_) | Self::Perk(_) | Self::Powerup
        )
    }
}
#[derive(Clone, Debug)]
pub struct Impact {
    pub point: Option<Vec3>,
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
    pub(crate) before: u32,
    pub(crate) base_amount: u32,
    pub(crate) lethal: bool,
    pub(crate) aimed_bullet: bool,
    pub(crate) insta_kill: bool,
}
#[derive(Resource, Default)]
pub struct DamageQueue(pub Vec<Impact>);
pub struct DamagePlugin;
impl Plugin for DamagePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageApplied>()
            .add_message::<EnemyDeath>()
            .init_resource::<DamageQueue>()
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
pub(crate) fn resolve(
    mut queue: ResMut<DamageQueue>,
    mut targets: Query<(
        &mut Vitality,
        Option<&mut crate::player_health::Recovery>,
        Option<&mut crate::combat_effects::Armor>,
        Has<crate::quest_recovery::BloodRestricted>,
        Has<crate::quest_recovery::PilotLanding>,
        Option<&crate::panzer::Panzer>,
        Has<crate::portals::Transit>,
        Has<crate::robots::RobotTransit>,
        Option<&mut crate::combat_effects::Wounds>,
    )>,
    attribution: Attribution,
    enemies: Query<&Zombie>,
    definition: Res<GameDefinition>,
    mut evidence: ResMut<crate::player_health::HealthEvidence>,
    mut accepted: MessageWriter<DamageApplied>,
    life_events: (
        MessageWriter<crate::player_health::PlayerDamage>,
        MessageWriter<crate::player_health::PlayerDeath>,
        MessageWriter<crate::solo_revive::PlayerDowned>,
    ),
    perks: Query<&crate::solo_revive::PerkInventory>,
    mut perk_evidence: ResMut<crate::perk_effects::EffectEvidence>,
    effects: Res<crate::powerups::PowerupEffects>,
    nuked: Query<(), With<crate::powerups::NukeTarget>>,
    mut session: ResMut<crate::gameplay::Session>,
) {
    let (mut player_damage, mut player_deaths, mut downed) = life_events;
    let Attribution {
        players,
        emitters,
        diggers,
        mut shields,
        positions,
    } = attribution;
    queue
        .0
        .sort_by_key(|impact| matches!(impact.source, ImpactSource::Enemy(_)));
    for mut impact in queue.0.drain(..) {
        let sender_alive = targets
            .get(impact.attacker)
            .is_ok_and(|(v, _, _, _, _, _, _, _, _)| {
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
            | ImpactSource::Status(_)
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
                    | ImpactSource::Status(_)
                    | ImpactSource::Powerup
                    | ImpactSource::Robot(_)
                    | ImpactSource::Vehicle(_)
            )
            && !(matches!(impact.source, ImpactSource::Enemy(_))
                && matches!(impact.category, DamageCategory::Explosive)))
            || !valid_source
        {
            continue;
        }
        let Ok((
            mut health,
            mut recovery,
            mut armor,
            restricted,
            pilot_landing,
            panzer,
            transit,
            robot_transit,
            mut wounds,
        )) = targets.get_mut(impact.target)
        else {
            continue;
        };
        if transit
            || robot_transit
            || pilot_landing
            || panzer.is_some_and(|p| matches!(p.phase, crate::panzer::PanzerPhase::Landing(_)))
        {
            continue;
        }
        if restricted
            && (!players.contains(impact.attacker)
                || effects.zombie_blood <= 0.
                || matches!(impact.source, ImpactSource::Powerup))
        {
            continue;
        }
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
        let tank_crush = matches!(&impact.source, ImpactSource::Vehicle(id) if id == "vehicle.tank")
            && matches!(impact.category, DamageCategory::Crush);
        if player
            && !out_of_bounds
            && !tank_crush
            && recovery.as_ref().is_some_and(|r| r.immunity > 0.)
        {
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
        if player && matches!(impact.source, ImpactSource::Enemy(_)) {
            if let (Ok(attacker), Ok((body, pose, mut shield))) = (
                positions.get(impact.attacker),
                shields.get_mut(impact.target),
            ) {
                let incoming = (attacker.translation - pose.translation)
                    .with_y(0.)
                    .normalize_or_zero();
                let facing = Quat::from_rotation_y(body.yaw) * Vec3::NEG_Z;
                let dot = incoming.dot(facing);
                if shield.shield_health > 0
                    && (if shield.shield_held {
                        dot > 0.3
                    } else {
                        dot < -0.3
                    })
                {
                    shield.shield_health = shield.shield_health.saturating_sub(impact.amount);
                    if shield.shield_health == 0 {
                        shield.owned.remove("tomb_shield_zm");
                        shield.shield_held = false;
                    }
                    continue;
                }
            }
        }
        if !matches!(impact.category, DamageCategory::Melee)
            && !matches!(
                impact.source,
                ImpactSource::Weapon(_) | ImpactSource::Projectile(_) | ImpactSource::Status(_)
            )
        {
            impact.amount = (impact.amount as f64
                * definition.tuning.global_hit_multipliers[impact.region.native_index()] as f64)
                .trunc()
                .max(1.) as u32;
        }
        if player
            && (matches!(impact.category, DamageCategory::Explosive)
                || matches!(impact.source, ImpactSource::Projectile(_)))
            && health.current().is_some_and(|health| health > 75)
        {
            impact.amount = 75;
        }
        let base_amount = impact.amount;
        if panzer.is_some()
            && matches!(impact.category, DamageCategory::Ice)
            && impact.source.id().contains("staff_water")
        {
            if let Some(armor) = armor.as_mut() {
                if armor.ice_cooldown > 0. {
                    continue;
                }
                armor.ice_cooldown = 1.;
            }
            impact.amount = if impact.source.id().contains("upgraded") {
                3300
            } else {
                2050
            };
        }
        if player && impact.source.id().contains("staff_") {
            continue;
        }
        if !player
            && !matches!(impact.source, ImpactSource::Status(_))
            && matches!(impact.category, DamageCategory::Fire)
        {
            if let ImpactSource::Weapon(id) | ImpactSource::Projectile(id) = &impact.source {
                impact.amount = crate::combat_effects::elemental_damage(id, impact.amount);
                if panzer.is_none()
                    && id.0.contains("staff_fire")
                    && let Some(wounds) = wounds.as_mut()
                {
                    let multiplier = 0.5 + 0.5 * ((base_amount as f32 - 1.) / 10.).clamp(0., 1.);
                    if id.0.contains("upgraded")
                        && multiplier > 0.5
                        && impact.amount > health.current().unwrap_or(0)
                        && session.random() % 2 == 0
                    {
                        wounds.disintegrated = true;
                        let _ = session.random();
                        impact.amount = health.current().unwrap_or(0);
                    } else if wounds.burning <= 0. {
                        wounds.ignite(impact.attacker, id.clone(), multiplier);
                        let mut delayed = impact.clone();
                        delayed.source = ImpactSource::Status(id.clone());
                        wounds.initial_fire = Some((0.5, delayed));
                        continue;
                    }
                }
            }
        }
        let armored = armor.is_some();
        if let Some(mut armor) = armor
            && !matches!(impact.source, ImpactSource::Powerup)
        {
            let shotgun = definition
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == impact.source.id())
                .is_some_and(|w| w.mechanics.weapon_class.as_deref() == Some("spread"));
            impact.amount = armor.absorb(
                &impact,
                shotgun,
                effects.insta_kill > 0. && sender_alive && players.contains(impact.attacker),
            );
        }
        let before = health.current().unwrap_or(0);
        let aimed_bullet = matches!(impact.category, DamageCategory::Bullet)
            && players
                .get(impact.attacker)
                .is_ok_and(|inventory| inventory.ads > 0.5);
        let cherry = impact.source.is_cherry(&definition);
        let insta_kill = !player
            && !armored
            && !matches!(impact.category, DamageCategory::Ice)
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
                continue;
            }
        };
        accepted.write(DamageApplied {
            impact,
            before,
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
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    effects: Res<crate::powerups::PowerupEffects>,
    nuked: Query<(), With<crate::powerups::NukeTarget>>,
    panzers: Query<(), With<crate::panzer::Panzer>>,
    zombies: Query<(), With<Zombie>>,
    capture_actors: Query<&crate::rounds::PopulationKind>,
    mut generators: ResMut<crate::generators::Generators>,
) {
    for applied in applied.read() {
        if !zombies.contains(applied.impact.target) {
            continue;
        }
        let impact = &applied.impact;
        let lethal = applied.lethal;
        let [hit, kill, head, torso, melee] = definition.tuning.combat_points;
        let is_melee = matches!(impact.category, DamageCategory::Melee);
        let cherry = impact.source.is_cherry(&definition);
        let bonus = if cherry {
            0
        } else if is_melee {
            melee
        } else if impact.region.head() {
            head
        } else if impact.region == HitRegion::Neck {
            definition.tuning.combat_neck_points
        } else if matches!(
            impact.region,
            HitRegion::Torso | HitRegion::UpperTorso | HitRegion::LowerTorso
        ) {
            torso
        } else {
            0
        };
        let capture = capture_actors
            .get(impact.target)
            .is_ok_and(|kind| *kind == crate::rounds::PopulationKind::GeneratorDefender);
        let points = if capture {
            if lethal && impact.source.player_attributed() {
                generators.capture_kill_points(session.round)
            } else {
                0
            }
        } else if lethal && panzers.contains(impact.target) && impact.source.player_attributed() {
            750
        } else if panzers.contains(impact.target) {
            0
        } else if !impact.source.player_attributed()
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
        let cherry_bonus = if !capture && cherry && lethal && !applied.insta_kill {
            40
        } else {
            0
        };
        let points = points
            .saturating_mul(effects.multiplier())
            .saturating_add(cherry_bonus);
        if !lethal {
            session.award(points);
        }
        if lethal {
            deaths.write(EnemyDeath(DamageReceipt {
                impact: impact.clone(),
                base_amount: applied.base_amount,
                lethal,
                points,
                aimed_bullet: applied.aimed_bullet,
                insta_kill: applied.insta_kill,
            }));
        }
    }
}

pub(crate) fn reward_deaths(
    mut deaths: MessageReader<EnemyDeath>,
    mut session: ResMut<Session>,
    capture_actors: Query<&crate::rounds::PopulationKind>,
) {
    for EnemyDeath(receipt) in deaths.read() {
        if !receipt.impact.source.player_attributed()
            || matches!(receipt.impact.source, ImpactSource::Powerup)
        {
            continue;
        }
        session.award(receipt.points);
        session.kills = session.kills.saturating_add(1);
        if !matches!(receipt.impact.category, DamageCategory::Melee) && receipt.impact.region.head()
        {
            session.headshots = session.headshots.saturating_add(1);
            if capture_actors
                .get(receipt.impact.target)
                .is_ok_and(|kind| *kind == crate::rounds::PopulationKind::GeneratorDefender)
            {
                continue;
            }
            session.challenge_headshots = session.challenge_headshots.saturating_add(1);
        }
    }
}
