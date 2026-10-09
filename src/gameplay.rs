use crate::{
    GameState,
    assets::GameDefinition,
    player::{Player, TickActions},
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
    pub score: u32,
    pub downs: u32,
    pub spawn_timer: Timer,
    pub intermission: Option<Timer>,
    pub spawn_cursor: usize,
    pub seed: u64,
    pub character: usize,
    pub kills: u32,
    pub shots: u32,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            round: 1,
            pending: 6,
            points: 500,
            score: 500,
            downs: 0,
            spawn_timer: Timer::from_seconds(1., TimerMode::Once),
            intermission: None,
            spawn_cursor: 0,
            seed: 0xB02,
            character: 0,
            kills: 0,
            shots: 0,
        }
    }
}

impl Session {
    pub fn award(&mut self, points: u32) {
        self.points = self.points.saturating_add(points);
        self.score = self.score.saturating_add(points);
    }
    pub fn random(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.seed >> 32) as u32
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
            .add_systems(
                FixedUpdate,
                (
                    combat.in_set(GameStep::Combat),
                    crate::rounds::advance.in_set(GameStep::Rounds),
                    crate::economy::record_downs
                        .after(GameStep::PlayerLife)
                        .before(GameStep::Rewards),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

pub(crate) fn combat(
    mut commands: Commands,
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
                if inventory.active().is_some_and(|w| {
                    if w.definition == id {
                        w.magazine == 1
                    } else {
                        w.left_magazine == Some(1)
                    }
                }) {
                    crate::audio::voice(
                        &mut commands,
                        &definition,
                        &session,
                        "ammo_low",
                        entity,
                        session.shots,
                    );
                }
                crate::audio::binding(
                    &mut commands,
                    &definition,
                    &format!("{}.shot", id.0),
                    crate::audio::SoundSource::Entity {
                        entity,
                        offset: Vec3::Y * (player.eye_height - player.height * 0.5),
                    },
                );
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
