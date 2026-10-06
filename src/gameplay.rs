use crate::{
    GameState,
    assets::GameDefinition,
    life::{LifeState, Vitality},
    player::{Player, TickActions, capsule},
    weapons::{WeaponEmission, WeaponIntent, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameStep {
    Movement,
    Interactions,
    EnemyIntent,
    Combat,
    Damage,
    PlayerLife,
    Rewards,
    Zombies,
    Rounds,
}
#[derive(Component)]
pub struct SessionEntity;
pub use crate::enemies::Zombie;

#[derive(Resource, Clone)]
pub struct Session {
    pub round: u32,
    pub pending: u32,
    pub points: u32,
    pub spawn_timer: Timer,
    pub intermission: Option<Timer>,
    pub spawn_cursor: usize,
    pub seed: u64,
    pub kills: u32,
    pub shots: u32,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            round: 1,
            pending: 6,
            points: 500,
            spawn_timer: Timer::from_seconds(1., TimerMode::Once),
            intermission: None,
            spawn_cursor: 0,
            seed: 0xB02,
            kills: 0,
            shots: 0,
        }
    }
}

impl Session {
    pub fn random(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.seed >> 32) as u32
    }
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
    pub fn spawn_index(&mut self, count: usize) -> Option<usize> {
        if count == 0 {
            return None;
        }
        let random = self.random();
        self.spawn_cursor = self.spawn_cursor.wrapping_add(1);
        Some((random as usize) % count)
    }
}

pub(crate) use crate::navigation::Navigation;

pub struct GameplayPlugin;
impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Session>()
            .add_message::<crate::rounds::RoundStarted>()
            .init_resource::<crate::rounds::Population>()
            .init_resource::<crate::rounds::RoundEvidence>()
            .configure_sets(
                FixedUpdate,
                (
                    GameStep::Movement,
                    GameStep::Interactions,
                    GameStep::EnemyIntent,
                    GameStep::Combat,
                    GameStep::Damage,
                    GameStep::PlayerLife,
                    GameStep::Rewards,
                    GameStep::Zombies,
                    GameStep::Rounds,
                )
                    .chain(),
            )
            .add_systems(OnEnter(GameState::Playing), begin_session)
            .add_systems(
                FixedUpdate,
                (
                    combat.in_set(GameStep::Combat),
                    rounds.in_set(GameStep::Rounds),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn begin_session(
    mut session: ResMut<Session>,
    mut impacts: ResMut<crate::damage::DamageQueue>,
    mut history: ResMut<crate::damage::CombatHistory>,
    definition: Res<GameDefinition>,
    players: Query<(), With<Player>>,
) {
    if !players.is_empty() {
        return;
    }
    *session = Session::default();
    session.pending = definition.rounds.population(1);
    session.spawn_timer =
        Timer::from_seconds(definition.rounds.first_spawn_seconds, TimerMode::Once);
    impacts.0.clear();
    *history = default();
}

pub(crate) fn combat(
    time: Res<Time<Fixed>>,
    actions: Res<TickActions>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &mut WeaponInventory,
        &crate::equipment::GrenadeInventory,
        &crate::mines::MineInventory,
        &crate::equipment::TacticalInventory,
        Option<&crate::perks::PerkDrinking>,
        &crate::solo_revive::PerkInventory,
    )>,
    enemies: crate::hit_regions::RayTargets,
    mut impacts: ResMut<crate::damage::DamageQueue>,
    query: SpatialQuery,
    mut hit_evidence: ResMut<crate::hit_regions::HitEvidence>,
    mut firing_evidence: ResMut<crate::perks::FiringEvidence>,
    mut reloads: MessageWriter<crate::perk_effects::ReloadEvent>,
) {
    let dt = time.delta_secs();
    let Ok((entity, player, transform, mut inventory, grenades, mines, tactical, drinking, perks)) =
        players.single_mut()
    else {
        return;
    };
    let reload_scale = definition
        .content
        .perks
        .iter()
        .filter(|p| perks.owned.contains(&p.id))
        .filter_map(|p| p.reload_duration_scale)
        .fold(1., f32::min);
    inventory.set_reload_duration_scale(reload_scale);
    inventory.firing = crate::perks::firing_modifiers(perks, &definition);
    let emissions = inventory.tick(
        dt,
        if grenades.busy() || mines.busy() || tactical.busy() || drinking.is_some() {
            WeaponIntent::default()
        } else {
            WeaponIntent {
                pressed: actions.fire,
                held: actions.fire_held,
                reload: actions.reload,
                melee: actions.melee,
                cycle: actions.cycle_weapon,
                slot: actions.weapon_slot,
                ads: actions.ads,
            }
        },
        &definition.content.weapons,
    );
    let eye = player.eye_height;
    let origin = transform.translation + Vec3::Y * (eye - player.height * 0.5);
    let rotation = Quat::from_rotation_y(player.yaw + inventory.recoil.x)
        * Quat::from_rotation_x((player.pitch + inventory.recoil.y).clamp(-1.5, 1.5));
    let direction = Dir3::new(rotation * Vec3::NEG_Z).unwrap_or(Dir3::NEG_Z);
    for emission in emissions {
        let weapon = inventory.definition(&definition.content.weapons);
        let pellets = if matches!(emission, WeaponEmission::Melee) {
            1
        } else {
            weapon.map_or(1, |w| w.mechanics.pellets)
        };
        let (melee, damage, weapon_id) = match emission {
            WeaponEmission::ReloadStarted {
                weapon,
                magazine,
                capacity,
                seconds,
            } => {
                reloads.write(crate::perk_effects::ReloadEvent {
                    owner: entity,
                    weapon,
                    start: Some((magazine, capacity, seconds)),
                });
                continue;
            }
            WeaponEmission::ReloadCompleted(weapon) => {
                reloads.write(crate::perk_effects::ReloadEvent {
                    owner: entity,
                    weapon,
                    start: None,
                });
                continue;
            }
            WeaponEmission::Shot(id) => {
                session.shots += 1;
                let Some(weapon) = definition.content.weapons.iter().find(|w| w.id == id) else {
                    continue;
                };
                (false, weapon.damage, id)
            }
            WeaponEmission::Melee => (
                true,
                definition
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id.0 == "weapon.knife_zm")
                    .map_or(definition.tuning.melee_damage, |w| w.damage),
                crate::origins::content::WeaponId("weapon.knife_zm".into()),
            ),
        };
        let spread = weapon.map_or(0., |w| {
            let hip = w.mechanics.spread[if player.crouched { 1 } else { 0 }]
                * inventory.firing.hip_spread_scale
                * (1. + player.velocity.xz().length().min(6.) / 6.);
            hip + (w.mechanics.spread[3] - hip) * inventory.ads
        });
        if !melee && let Some(weapon) = weapon {
            inventory.recoil_impulse(weapon, session.shots.wrapping_mul(31) as u64);
        }
        let copies = if melee {
            1
        } else {
            weapon.map_or(1, |w| inventory.firing.copies(w))
        };
        if !melee && firing_evidence.shots.len() < 128 {
            firing_evidence.shots.push(serde_json::json!({"weapon":weapon_id.0,"seconds":time.elapsed_secs_f64(),"pellets":pellets,"copies":copies,"rays":pellets*copies,"spread":spread,"ads":inventory.ads,"interval_scale":inventory.firing.interval_scale,"hip_spread_scale":inventory.firing.hip_spread_scale}));
        }
        for ray in 0..pellets * copies {
            // Paired bullets share their originating pellet direction. They resolve
            // independently through damage/life, with one ammunition/recoil debit.
            let pellet = ray / copies;
            let direction = if melee {
                direction
            } else {
                crate::weapons::spread_direction(
                    rotation,
                    spread,
                    session.shots as u64 * 137 + pellet as u64 * 19,
                )
            };
            if let Some(hit) = crate::hit_regions::cast(
                &enemies,
                &query,
                entity,
                origin,
                direction,
                if melee { 2.2 } else { 100. },
            ) {
                let region = hit.hit.region;
                if hit_evidence.hits.len() < 128 {
                    hit_evidence.hits.push(serde_json::json!({"target":hit.target.to_bits(),"region":format!("{region:?}"),
                        "distance":hit.hit.distance,"point":hit.hit.point.to_array(),"melee":melee}));
                }
                let damage = if melee {
                    damage
                } else {
                    inventory
                        .definition(&definition.content.weapons)
                        .map_or(damage, |weapon| {
                            crate::weapons::damage_at(weapon, hit.hit.distance, region)
                        })
                };
                impacts.0.push(crate::damage::Impact {
                    attacker: entity,
                    target: hit.target,
                    source: crate::damage::ImpactSource::Weapon(weapon_id.clone()),
                    category: if melee {
                        crate::origins::content::DamageCategory::Melee
                    } else {
                        weapon.map_or(crate::origins::content::DamageCategory::Bullet, |w| {
                            w.category
                        })
                    },
                    region,
                    amount: damage,
                });
            }
        }
    }
}

pub(crate) fn rounds(
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    enemies: Query<(
        &Vitality,
        &crate::rounds::PopulationKind,
        &Transform,
        Option<&Zombie>,
    )>,
    mut commands: Commands,
    query: SpatialQuery,
    navigation: Res<Navigation>,
    mut population: ResMut<crate::rounds::Population>,
    mut evidence: ResMut<crate::rounds::RoundEvidence>,
    mut started: MessageWriter<crate::rounds::RoundStarted>,
    players: Query<(&Player, &Transform)>,
) {
    use crate::rounds::PopulationKind;
    *population = default();
    for (vitality, kind, _, _) in &enemies {
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
    // Generator attacks run independently; ordinary and scheduled special actors hold the round.
    if session.round_complete(population.ordinary + population.special) {
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
            started.write(crate::rounds::RoundStarted {
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
    if population.ordinary >= profile.ordinary_cap as usize
        || population.total() >= profile.actor_cap as usize
    {
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
        // Preserve capsule clearance already admitted by the map graph.
        let position = base + Vec3::Y * 0.85;
        // Noclip traversal still reserves its landing; never admit overlapping risers.
        if enemies.iter().any(|(health, _, body, zombie)| {
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
        let health = Vitality::ordinary_actor(amount, amount).ok()?;
        let enemy = variants.get(session.spawn_cursor % variants.len().max(1))?;
        let gait = profile.gait(session.round, session.random());
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
    if evidence.admitted.len() < 256 {
        evidence.admitted.push(serde_json::json!({"round":session.round,"health":profile.health(session.round),
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
    let zombie = Zombie::new(position, identity, gait, profile.speed(gait));
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
