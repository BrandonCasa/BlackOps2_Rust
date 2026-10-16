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
    pub difficulty: crate::frontend::Difficulty,
    pub round: u32,
    pub pending: u32,
    pub points: u32,
    pub score: u32,
    pub spent: u64,
    pub headshots: u32,
    pub challenge_headshots: u32,
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
            difficulty: crate::frontend::Difficulty::Original,
            round: 1,
            pending: 6,
            points: 500,
            score: 500,
            spent: 0,
            headshots: 0,
            challenge_headshots: 0,
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
        app.add_plugins(crate::projectiles::ProjectilePlugin);
        app.init_resource::<Session>()
            .add_message::<crate::rounds::RoundStarted>()
            .add_message::<crate::rounds::RoundEnding>()
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
                    ApplyDeferred.before(GameStep::Movement),
                    ApplyDeferred
                        .after(GameStep::Interactions)
                        .before(GameStep::EnemyIntent),
                    ApplyDeferred
                        .after(GameStep::EnemyIntent)
                        .before(GameStep::Combat),
                    ApplyDeferred
                        .after(GameStep::Damage)
                        .before(GameStep::PlayerLife),
                    combat.in_set(GameStep::Combat).run_if(crate::cheats::weapon_equipped),
                    crate::rounds::advance.in_set(GameStep::Rounds).run_if(crate::cheats::rounds_active),
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
    input: Res<crate::input::GameInput>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    mut players: Query<(
        Entity,
        &mut Player,
        &Transform,
        &mut WeaponInventory,
        &crate::equipment::GrenadeInventory,
        &crate::mines::MineInventory,
        &crate::equipment::TacticalInventory,
        Option<&crate::perks::PerkDrinking>,
        &crate::solo_revive::PerkInventory,
        Has<crate::actions::HandReservation>,
        &crate::crafting::BuildableInventory,
        &crate::movement::MotionFeedback,
        Has<crate::robots::StompReaction>,
        Has<crate::robots::RobotTransit>,
        Has<crate::tank::TankRescue>,
    )>,
    (enemies, zombies, armor): (
        crate::hit_regions::RayTargets,
        Query<(), With<crate::enemies::Zombie>>,
        Query<&crate::combat_effects::Armor>,
    ),
    (mut impacts, mut shot_effects, effects): (
        ResMut<crate::damage::DamageQueue>,
        MessageWriter<crate::native_fx::shooting::Shot>,
        Res<crate::native_fx::Catalog>,
    ),
    query: SpatialQuery,
    mut hit_evidence: ResMut<crate::hit_regions::HitEvidence>,
    mut firing_evidence: ResMut<crate::perks::FiringEvidence>,
    mut reloads: MessageWriter<crate::perk_effects::ReloadEvent>,
    (environmental, trigger_targets): (
        crate::environment::support::ShootableTargets,
        crate::environment::support::DamageTriggerTargets,
    ),
    surfaces: crate::hit_regions::PenetrationSurfaces,
    (mut environment_hits, mut ragdoll_hits): (
        MessageWriter<crate::environment::support::EnvironmentalHit>,
        MessageWriter<crate::enemies::RagdollBullet>,
    ),
) {
    let dt = time.delta_secs();
    let Ok((
        entity,
        mut player,
        transform,
        mut inventory,
        grenades,
        mines,
        tactical,
        drinking,
        perks,
        reserved,
        buildables,
        feedback,
        stomp,
        robot_transit,
        tank_rescue,
    )) = players.single_mut()
    else {
        return;
    };
    if reserved || buildables.shield_held || stomp || robot_transit || tank_rescue {
        inventory.suspend();
        inventory.cancel_charge();
        for emission in inventory.tick(
            dt,
            WeaponIntent {
                suspended: true,
                ..default()
            },
            &definition.content.weapons,
        ) {
            if let WeaponEmission::Ads { weapon, aiming } = emission {
                crate::audio::binding(
                    &mut commands,
                    &definition,
                    &format!(
                        "{}.{}",
                        weapon.0,
                        if aiming { "ads_enter" } else { "ads_leave" }
                    ),
                    crate::audio::SoundSource::attached(entity),
                );
            }
        }
        return;
    }
    let reload_scale = definition
        .content
        .perks
        .iter()
        .filter(|p| perks.owned.contains(&p.id))
        .filter_map(|p| p.reload_duration_scale)
        .fold(1., f32::min);
    inventory.set_reload_duration_scale(reload_scale);
    inventory.firing = crate::perks::firing_modifiers(perks, &definition);
    if grenades.busy() || mines.busy() || tactical.busy() || drinking.is_some() {
        inventory.suspend();
        inventory.cancel_charge();
    }
    if input.staff_secondary
        && !grenades.busy()
        && !mines.busy()
        && !tactical.busy()
        && drinking.is_none()
    {
        if let Some(weapon) = inventory.definition(&definition.content.weapons)
            && inventory.ads > 0.5
            && weapon
                .attachments
                .get(inventory.attachment())
                .is_some_and(|a| a.ads_fov.len() > 1)
        {
            inventory.toggle_zoom(weapon);
        } else {
            inventory.toggle_staff_mode(&definition.content.weapons);
        }
    }
    let emissions = inventory.tick(
        dt,
        if actions.suspended
            || grenades.busy()
            || mines.busy()
            || tactical.busy()
            || drinking.is_some()
        {
            WeaponIntent {
                suspended: true,
                ..default()
            }
        } else {
            WeaponIntent {
                suspended: false,
                sprint: actions.sprint,
                pressed: actions.fire,
                held: actions.fire_held,
                secondary_pressed: actions.secondary_fire,
                secondary_held: actions.secondary_fire_held,
                reload: actions.reload,
                melee: actions.melee,
                cycle: actions.cycle_weapon,
                slot: actions.weapon_slot,
                ads: actions.ads,
            }
        },
        &definition.content.weapons,
    );
    inventory.update_spread(
        &player,
        input.movement != Vec2::ZERO,
        definition.tuning.walk_speed,
        dt,
        &definition.content.weapons,
    );
    let eye = player.eye_height;
    let origin = transform.translation + Vec3::Y * (eye - player.height * 0.5);
    let rotation = Quat::from_rotation_y(
        player.yaw + inventory.recoil.x + feedback.view_angles.y.to_radians(),
    ) * Quat::from_rotation_x(
        (player.pitch + inventory.recoil.y - feedback.view_angles.x.to_radians()).clamp(-1.5, 1.5),
    );
    let direction = Dir3::new(rotation * Vec3::NEG_Z).unwrap_or(Dir3::NEG_Z);
    for emission in emissions {
        let (melee, damage, weapon_id, weapon) = match emission {
            WeaponEmission::Ads { weapon, aiming } => {
                crate::audio::binding(
                    &mut commands,
                    &definition,
                    &format!(
                        "{}.{}",
                        weapon.0,
                        if aiming { "ads_enter" } else { "ads_leave" }
                    ),
                    crate::audio::SoundSource::attached(entity),
                );
                continue;
            }
            WeaponEmission::DryFire(id) => {
                crate::audio::binding(
                    &mut commands,
                    &definition,
                    &format!("{}.dry_fire", id.0),
                    crate::audio::SoundSource::attached(entity),
                );
                continue;
            }
            WeaponEmission::ReloadStarted {
                weapon,
                magazine,
                capacity,
                seconds,
            } => {
                crate::audio::binding(
                    &mut commands,
                    &definition,
                    &format!(
                        "{}.{}",
                        weapon.0,
                        if magazine == 0 {
                            "reload_empty"
                        } else {
                            "reload"
                        }
                    ),
                    crate::audio::SoundSource::attached(entity),
                );
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
                crate::audio::binding(
                    &mut commands,
                    &definition,
                    &format!(
                        "{}.{}",
                        id.0,
                        if inventory.active().is_some_and(|w| {
                            if w.definition == id {
                                w.magazine == 0
                            } else {
                                w.left_magazine == Some(0)
                            }
                        }) {
                            "last_shot"
                        } else {
                            "shot"
                        }
                    ),
                    crate::audio::SoundSource::Entity {
                        entity,
                        offset: Vec3::Y * (player.eye_height - player.height * 0.5),
                    },
                );
                let Some(weapon) = definition.content.weapons.iter().find(|w| w.id == id) else {
                    continue;
                };
                (false, weapon.damage, id, Some(weapon))
            }
            WeaponEmission::Melee => {
                let staff = inventory.melee_definition(&definition.content.weapons);
                let id = staff.map_or_else(
                    || crate::origins::WeaponId("weapon.knife_zm".into()),
                    |w| w.id.clone(),
                );
                if id.0.contains("one_inch_punch") {
                    crate::rituals::punch(
                        &enemies,
                        &zombies,
                        &query,
                        &mut impacts,
                        entity,
                        transform.translation,
                        player.yaw,
                        &id,
                    );
                    continue;
                }
                let melee_weapon = definition.content.weapons.iter().find(|w| w.id == id);
                let damage = melee_weapon.map_or(definition.tuning.melee_damage, |w| w.damage);
                (true, damage, id, melee_weapon)
            }
        };
        let pellets = if melee {
            1
        } else {
            weapon.map_or(1, |w| w.mechanics.pellets)
        };
        let spread = weapon.map_or(0., |w| inventory.spread_angle(w, player.eye_height));
        if !melee && let Some(weapon) = weapon {
            inventory.recoil_impulse(weapon, session.shots.wrapping_mul(31) as u64);
        }
        let copies = if melee {
            1
        } else {
            weapon.map_or(1, |w| inventory.firing.copies(w))
        };
        let range = weapon.map_or(if melee { 55. * 0.0254 } else { 8192. * 0.0254 }, |w| {
            w.mechanics.range
        });
        if !melee && firing_evidence.shots.len() < 128 {
            firing_evidence.shots.push(serde_json::json!({"weapon":weapon_id.0,"seconds":time.elapsed_secs_f64(),"pellets":pellets,"copies":copies,"rays":pellets*copies,"spread":spread,"ads":inventory.ads,"prone":player.prone,"crouched":player.crouched,"speed":player.velocity.xz().length(),"interval_scale":inventory.firing.interval_scale,"hip_spread_scale":inventory.firing.hip_spread_scale}));
        }
        if !melee
            && let Some(weapon) = weapon
            && weapon.mechanics.delivery == crate::origins::content::WeaponDelivery::Projectile
            && weapon.mechanics.weapon_class.as_deref() != Some("grenade")
            && !matches!(
                weapon.mechanics.fire,
                crate::origins::content::FirePattern::Automatic
            )
            && inventory
                .active()
                .is_some_and(|held| held.definition == weapon.id)
        {
            // Native missile recoil uses the unspread forward vector, once per emitted shot.
            let recoil = -*direction * (64. * 0.0254);
            player.velocity += recoil.with_y(recoil.y.min(0.));
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
            if !melee
                && let Some(weapon) = weapon
                && weapon.mechanics.delivery == crate::origins::content::WeaponDelivery::Projectile
            {
                if ray == 0 {
                    shot_effects.write(crate::native_fx::shooting::Shot {
                        owner: entity,
                        weapon: weapon_id.clone(),
                        origin,
                        end: None,
                        view_weapon: inventory
                            .active()
                            .expect("emitted shot has an active weapon")
                            .definition
                            .clone(),
                        left: inventory.active().is_some_and(|held| {
                            held.left_magazine.is_some() && held.definition != weapon_id
                        }),
                        muzzle: true,
                        last: inventory.active().is_some_and(|held| held.magazine == 0),
                    });
                }
                crate::projectiles::spawn(&mut commands, entity, weapon, origin, direction);
                continue;
            }
            let (actor_hits, bullet_trace) = if melee {
                (
                    crate::hit_regions::melee(
                        &enemies,
                        &query,
                        entity,
                        origin,
                        rotation,
                        range,
                        weapon.map_or([10. * 0.0254; 2], |w| w.mechanics.melee_extent),
                    )
                    .into_iter()
                    .map(|h| (h, 1.))
                    .collect::<Vec<_>>(),
                    None,
                )
            } else {
                let trace = crate::hit_regions::penetrating(
                    &enemies,
                    &surfaces,
                    &query,
                    entity,
                    origin,
                    direction,
                    range,
                    weapon.and_then(|w| w.mechanics.penetration.as_ref()),
                );
                (trace.hits, Some((trace.impacts, trace.distance)))
            };
            let environmental_hit = crate::environment::support::cast(
                &environmental,
                &query,
                entity,
                origin,
                direction,
                range,
            );
            if let Some((impact_points, distance)) = bullet_trace {
                let distance =
                    distance.min(environmental_hit.map_or(range, |(_, distance)| distance));
                for impact in impact_points {
                    if origin.distance(impact.point) > distance + 0.001 {
                        continue;
                    }
                    let surface = if impact.target.is_some_and(|target| {
                        armor
                            .get(target)
                            .is_ok_and(|armor| !impact.region.head() || !armor.broken)
                    }) {
                        13
                    } else {
                        impact.surface
                    };
                    let flags = u32::from(impact.exit) * 4
                        | u32::from(impact.region.head())
                        | u32::from(surface == 20) * 8;
                    if let Some(effect) = weapon.and_then(|weapon| {
                        effects.impact(weapon.mechanics.impact_type, surface, flags)
                    }) {
                        crate::native_fx::emit(
                            &mut commands,
                            effect,
                            effects.explosion_pose(impact.point, impact.normal),
                        );
                    }
                }
                shot_effects.write(crate::native_fx::shooting::Shot {
                    owner: entity,
                    weapon: weapon_id.clone(),
                    origin,
                    view_weapon: inventory
                        .active()
                        .expect("emitted shot has an active weapon")
                        .definition
                        .clone(),
                    left: inventory.active().is_some_and(|held| {
                        held.left_magazine.is_some() && held.definition != weapon_id
                    }),
                    end: Some(origin + *direction * distance),
                    muzzle: ray == 0,
                    last: inventory.active().is_some_and(|held| {
                        if held.definition == weapon_id {
                            held.magazine == 0
                        } else {
                            held.left_magazine == Some(0)
                        }
                    }),
                });
            }
            let category = if melee {
                crate::origins::DamageCategory::Melee
            } else {
                weapon.map_or(crate::origins::DamageCategory::Bullet, |w| w.category)
            };
            let native_mod = crate::environment::support::weapon_damage_mod(weapon, melee);
            if !melee && matches!(category, crate::origins::DamageCategory::Bullet) {
                let mut distance = actor_hits
                    .first()
                    .map_or(range, |(hit, _)| hit.hit.distance);
                if let Some(blocker) = query.cast_ray_predicate(
                    origin,
                    direction,
                    distance,
                    true,
                    &crate::environment::solids([entity]),
                    &|target| !enemies.contains(target),
                ) {
                    distance = distance.min(blocker.distance);
                }
                if let Some((_, hit)) = environmental_hit.filter(|(_, hit)| *hit <= distance) {
                    distance = distance.min(hit);
                }
                ragdoll_hits.write(crate::enemies::RagdollBullet {
                    weapon: weapon_id.clone(),
                    origin,
                    end: origin + *direction * distance,
                });
            }
            crate::environment::support::trace_damage(
                &trigger_targets,
                &query,
                &mut environment_hits,
                entity,
                &weapon_id,
                category,
                native_mod,
                damage,
                origin,
                direction,
                actor_hits
                    .first()
                    .map_or(range, |(hit, _)| hit.hit.distance),
            );
            if let Some((target, distance)) = environmental_hit
                && actor_hits
                    .first()
                    .is_none_or(|(hit, _)| distance < hit.hit.distance)
            {
                environment_hits.write(crate::environment::support::EnvironmentalHit {
                    target,
                    attacker: entity,
                    weapon: weapon_id.clone(),
                    category,
                    native_mod,
                    amount: if melee {
                        damage
                    } else {
                        weapon.map_or(damage, |w| {
                            crate::weapons::damage_at(w, distance, crate::weapons::HitRegion::Torso)
                        })
                    },
                    point: origin + direction.as_vec3() * distance,
                    direction: direction.as_vec3(),
                });
                continue;
            }
            for (hit, attenuation) in actor_hits {
                let region = hit.hit.region;
                if melee {
                    let effect = weapon.and_then(|weapon| {
                        if armor.contains(hit.target) {
                            weapon.effects.melee_no_blood.as_deref()
                        } else {
                            weapon.effects.melee.as_deref()
                        }
                    });
                    if let Some(effect) = effect {
                        crate::native_fx::emit(
                            &mut commands,
                            effect,
                            effects.explosion_pose(hit.hit.point, -*direction),
                        );
                    }
                }
                if hit_evidence.hits.len() < 128 {
                    hit_evidence.hits.push(serde_json::json!({"target":hit.target.to_bits(),"region":format!("{region:?}"),
                        "distance":hit.hit.distance,"point":hit.hit.point.to_array(),"melee":melee}));
                }
                let damage = if melee {
                    damage
                } else {
                    weapon.map_or(damage, |weapon| {
                        crate::weapons::damage_at(
                            weapon,
                            hit.hit.distance / inventory.attachment_range(),
                            region,
                        )
                    })
                };
                impacts.0.push(crate::damage::Impact {
                    point: Some(hit.hit.point),
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
                    amount: (damage as f32 * attenuation).max(1.) as u32,
                });
            }
        }
    }
}
