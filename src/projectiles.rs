use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    origins::content::{WeaponDefinition, WeaponId},
};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct WeaponProjectile {
    pub owner: Entity,
    pub weapon: WeaponId,
    pub velocity: Vec3,
    pub remaining: f32,
    pub radius: f32,
    traveled: f32,
    initialized: bool,
    detonate: bool,
}
#[derive(Component)]
struct AdditionalFireShots {
    owner: Entity,
    weapon: WeaponId,
    remaining: u32,
    index: u32,
    delay: f32,
}
pub fn spawn(
    commands: &mut Commands,
    owner: Entity,
    weapon: &WeaponDefinition,
    origin: Vec3,
    direction: Dir3,
) {
    spawn_one(commands, owner, weapon, origin, direction);
    if weapon.id.0.contains("staff_fire_upgraded2") || weapon.id.0.contains("staff_fire_upgraded3")
    {
        commands.spawn((
            SessionEntity,
            AdditionalFireShots {
                owner,
                weapon: weapon.id.clone(),
                remaining: if weapon.id.0.contains("upgraded3") {
                    2
                } else {
                    1
                },
                index: 1,
                delay: 0.35,
            },
        ));
    }
}
fn spawn_one(
    commands: &mut Commands,
    owner: Entity,
    weapon: &WeaponDefinition,
    origin: Vec3,
    direction: Dir3,
) {
    let entity = commands
        .spawn((
            WeaponProjectile {
                owner,
                weapon: weapon.id.clone(),
                velocity: direction.as_vec3() * weapon.mechanics.projectile_speed
                    + Vec3::Y * weapon.mechanics.projectile_up_speed,
                remaining: weapon.mechanics.projectile_lifetime,
                radius: 0.0254,
                traveled: 0.,
                initialized: false,
                detonate: false,
            },
            SessionEntity,
            Transform::from_translation(origin)
                .with_rotation(Quat::from_rotation_arc(Vec3::X, *direction)),
            Name::new(format!("Projectile {}", weapon.id.0)),
        ))
        .id();
    if let Some(effect) = &weapon.mechanics.projectile_trail {
        crate::native_fx::attach(commands, effect.clone(), entity, Transform::default());
    }
    if let Some(alias) = &weapon.mechanics.projectile_loop {
        crate::audio::emit_loop(
            commands,
            alias.clone(),
            crate::audio::SoundSource::attached(entity),
        );
    }
}
#[derive(Message, Clone)]
pub struct ProjectileImpact {
    pub owner: Entity,
    pub weapon: WeaponId,
    pub point: Vec3,
}
pub struct ProjectilePlugin;
impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<ProjectileImpact>().add_systems(
            FixedUpdate,
            (additional, flight, remote_revive)
                .chain()
                .in_set(GameStep::Combat)
                .after(crate::gameplay::combat)
                .before(crate::equipment::resolve_blasts)
                .run_if(in_state(GameState::Playing)),
        );
    }
}
fn additional(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut session: ResMut<crate::gameplay::Session>,
    mut sequences: Query<(Entity, &mut AdditionalFireShots)>,
    players: Query<(
        &crate::player::Player,
        &Transform,
        &crate::weapons::WeaponInventory,
        &crate::life::Vitality,
        Has<crate::actions::HandReservation>,
        Option<&crate::crafting::BuildableInventory>,
        Has<crate::robots::StompReaction>,
        Has<crate::robots::RobotTransit>,
        Has<crate::tank::TankRescue>,
    )>,
    mut shot_effects: MessageWriter<crate::native_fx::shooting::Shot>,
) {
    for (entity, mut sequence) in &mut sequences {
        let Ok((
            player,
            pose,
            inventory,
            health,
            reserved,
            buildables,
            stomp,
            robot_transit,
            tank_rescue,
        )) = players.get(sequence.owner)
        else {
            commands.entity(entity).despawn();
            continue;
        };
        if health.state() != crate::life::LifeState::Standing
            || reserved
            || buildables.is_some_and(|e| e.shield_held)
            || stomp
            || robot_transit
            || tank_rescue
            || !inventory
                .active()
                .is_some_and(|w| w.definition.0 == "weapon.staff_fire_upgraded_zm")
        {
            commands.entity(entity).despawn();
            continue;
        }
        sequence.delay -= time.delta_secs();
        if sequence.delay > 0. {
            continue;
        }
        if let Some(weapon) = game
            .content
            .weapons
            .iter()
            .find(|w| w.id == sequence.weapon)
        {
            let random = session.random() as f32 / u32::MAX as f32;
            let rotation = Quat::from_rotation_y(player.yaw + (-15. + random * 30.).to_radians())
                * Quat::from_rotation_x(player.pitch - (5. * sequence.index as f32).to_radians());
            let origin = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
            shot_effects.write(crate::native_fx::shooting::Shot {
                owner: sequence.owner,
                weapon: sequence.weapon.clone(),
                view_weapon: inventory
                    .active()
                    .expect("active staff sequence")
                    .definition
                    .clone(),
                left: false,
                origin,
                end: None,
                muzzle: true,
                last: false,
            });
            spawn_one(
                &mut commands,
                sequence.owner,
                weapon,
                origin,
                Dir3::new(rotation * Vec3::NEG_Z).unwrap_or(Dir3::NEG_Z),
            );
        }
        sequence.remaining -= 1;
        sequence.index += 1;
        sequence.delay += 0.35;
        if sequence.remaining == 0 {
            commands.entity(entity).despawn();
        }
    }
}
pub(crate) fn flight(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    (game, effects): (Res<GameDefinition>, Res<crate::native_fx::Catalog>),
    session: Res<crate::session::SessionId>,
    mut projectiles: Query<
        (Entity, &mut WeaponProjectile, &mut Transform),
        (
            Without<crate::life::Vitality>,
            Without<crate::environment::support::DamageTriggerState>,
        ),
    >,
    targets: crate::hit_regions::RayTargets,
    zombies: Query<(), With<crate::enemies::Zombie>>,
    (query, surfaces, armor): (
        SpatialQuery,
        Query<&crate::movement::physics::SurfaceFlags>,
        Query<&crate::combat_effects::Armor>,
    ),
    mut damage: ResMut<crate::damage::DamageQueue>,
    mut blasts: ResMut<crate::equipment::BlastQueue>,
    (environmental, trigger_targets): (
        crate::environment::support::ShootableTargets,
        crate::environment::support::DamageTriggerTargets,
    ),
    mut environment_hits: MessageWriter<crate::environment::support::EnvironmentalHit>,
    mut impacts: MessageWriter<ProjectileImpact>,
) {
    for (entity, mut projectile, mut pose) in &mut projectiles {
        let Some(weapon) = game
            .content
            .weapons
            .iter()
            .find(|weapon| weapon.id == projectile.weapon)
        else {
            commands.entity(entity).despawn();
            continue;
        };
        let dt = time.delta_secs().min(projectile.remaining.max(0.));
        projectile.remaining -= time.delta_secs();
        let direction = Dir3::new(projectile.velocity).unwrap_or(Dir3::NEG_Z);
        pose.rotation = Quat::from_rotation_arc(Vec3::X, *direction);
        if !projectile.initialized {
            projectile.initialized = true;
            if weapon.id.0 == "weapon.staff_air_zm" || weapon.id.0 == "weapon.staff_air_upgraded_zm"
            {
                let range = 10.16;
                let cosine = if weapon.id.0.contains("upgraded") {
                    60_f32
                } else {
                    45_f32
                }
                .to_radians()
                .cos();
                let mut cone: Vec<_> = targets
                    .iter()
                    .filter(|(target, health, _, _)| {
                        zombies.contains(*target)
                            && health.state() == crate::life::LifeState::ActorAlive
                    })
                    .map(|(target, _, pose, _)| {
                        (
                            target,
                            pose.translation,
                            pose.translation.distance_squared(pose.translation),
                        )
                    })
                    .collect();
                for candidate in &mut cone {
                    candidate.2 = candidate.1.distance_squared(pose.translation);
                }
                cone.retain(|(_, _, distance)| *distance <= range * range);
                cone.sort_by(|a, b| {
                    a.2.total_cmp(&b.2)
                        .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
                });
                for (target, position, _) in cone.into_iter().take(12) {
                    let delta = position - pose.translation;
                    let Ok(ray) = Dir3::new(delta) else {
                        continue;
                    };
                    if ray.dot(*direction) < cosine
                        || query
                            .cast_ray(
                                pose.translation,
                                ray,
                                delta.length(),
                                true,
                                &crate::environment::solids([entity, target, projectile.owner]),
                            )
                            .is_some()
                    {
                        continue;
                    }
                    damage.0.push(crate::damage::Impact {
                        point: None,
                        attacker: projectile.owner,
                        target,
                        source: crate::damage::ImpactSource::Weapon(projectile.weapon.clone()),
                        category: weapon.category,
                        region: crate::weapons::HitRegion::Torso,
                        amount: if weapon.id.0.contains("upgraded") {
                            3300
                        } else {
                            2050
                        },
                    });
                }
                commands.entity(entity).despawn();
                continue;
            }
            if matches!(weapon.category, crate::origins::DamageCategory::Lightning) {
                let range = if weapon.id.0.contains("upgraded3") {
                    30.48
                } else {
                    20.32
                };
                let distance = query
                    .cast_ray(
                        pose.translation,
                        direction,
                        range,
                        true,
                        &crate::environment::solids([entity, projectile.owner]),
                    )
                    .map_or(range, |hit| hit.distance);
                crate::staff_fields::traveling(
                    &mut commands,
                    projectile.owner,
                    weapon,
                    pose.translation,
                    direction,
                    pose.translation + *direction * distance,
                );
            }
        }
        let gravity_step = if weapon.mechanics.projectile_bounce {
            game.tuning.gravity * dt
        } else {
            0.
        };
        let delta = (projectile.velocity - Vec3::Y * gravity_step * 0.5) * dt;
        projectile.velocity.y -= gravity_step;
        let distance = delta.length();
        let direction = Dir3::new(delta).unwrap_or(direction);
        let actor = crate::hit_regions::cast(
            &targets,
            &query,
            projectile.owner,
            pose.translation,
            direction,
            distance,
        );
        let filter = crate::environment::solids(
            std::iter::once(projectile.owner).chain(targets.iter().map(|(entity, _, _, _)| entity)),
        );
        let solid = query.cast_shape(
            &Collider::sphere(projectile.radius),
            pose.translation,
            Quat::IDENTITY,
            direction,
            &ShapeCastConfig::from_max_distance(distance),
            &filter,
        );
        let environment = crate::environment::support::cast(
            &environmental,
            &query,
            projectile.owner,
            pose.translation,
            direction,
            distance,
        );
        let solid_distance = solid.map_or(f32::INFINITY, |hit| hit.distance);
        let actor_distance = actor.map_or(f32::INFINITY, |hit| hit.hit.distance);
        let environment_distance = environment.map_or(f32::INFINITY, |(_, distance)| distance);
        let impact_distance = solid_distance.min(actor_distance).min(environment_distance);
        crate::environment::support::trace_damage(
            &trigger_targets,
            &query,
            &mut environment_hits,
            projectile.owner,
            &projectile.weapon,
            weapon.category,
            crate::environment::support::weapon_damage_mod(Some(weapon), false),
            weapon.damage,
            pose.translation,
            direction,
            distance.min(impact_distance),
        );
        if !projectile.detonate && !impact_distance.is_finite() && projectile.remaining > 0. {
            pose.translation += delta;
            projectile.traveled += distance;
            continue;
        }
        if !projectile.detonate
            && weapon.mechanics.projectile_bounce
            && solid_distance <= actor_distance
            && solid_distance <= environment_distance
            && let Some(hit) = solid
        {
            let normal = hit.normal1;
            pose.translation += *direction * hit.distance + normal * (projectile.radius + 0.003);
            projectile.velocity =
                (projectile.velocity - 2. * projectile.velocity.dot(normal) * normal) * 0.5;
            projectile.detonate = true;
            continue;
        }
        if !impact_distance.is_finite()
            && !projectile.detonate
            && !weapon.mechanics.projectile_bounce
        {
            commands.entity(entity).despawn();
            continue;
        }
        let center = if projectile.detonate || !impact_distance.is_finite() {
            pose.translation
        } else {
            pose.translation + *direction * impact_distance
        };
        impacts.write(ProjectileImpact {
            owner: projectile.owner,
            weapon: projectile.weapon.clone(),
            point: center,
        });
        let actor_impact = actor.is_some()
            && actor_distance <= solid_distance
            && actor_distance <= environment_distance;
        let normal = if actor_impact {
            -*direction
        } else {
            solid.map_or(-*direction, |hit| hit.normal1)
        };
        let surface = if actor_impact {
            if actor.is_some_and(|hit| {
                armor
                    .get(hit.target)
                    .is_ok_and(|armor| !hit.hit.region.head() || !armor.broken)
            }) {
                13
            } else {
                7
            }
        } else {
            solid
                .and_then(|hit| surfaces.get(hit.entity).ok())
                .map_or(0, |flags| ((flags.0 >> 20) & 0x3f) as u8)
        };
        if let Some(effect) = effects.impact(weapon.mechanics.impact_type, surface, 0) {
            crate::native_fx::emit(
                &mut commands,
                effect,
                effects.explosion_pose(center, normal),
            );
        }
        if let Some(effect) = &weapon.mechanics.projectile_effect {
            crate::native_fx::emit(
                &mut commands,
                effect.clone(),
                effects.explosion_pose(center, normal),
            );
        }
        if environment_distance <= solid_distance && environment_distance <= actor_distance {
            if let Some((target, _)) = environment {
                environment_hits.write(crate::environment::support::EnvironmentalHit {
                    target,
                    attacker: projectile.owner,
                    weapon: projectile.weapon.clone(),
                    category: weapon.category,
                    native_mod: crate::environment::support::weapon_damage_mod(Some(weapon), false),
                    amount: weapon.damage,
                    point: center,
                    direction: *direction,
                });
            }
        } else if actor_distance <= solid_distance
            && let Some(hit) = actor
        {
            if !matches!(weapon.category, crate::origins::DamageCategory::Fire)
                || weapon.mechanics.explosion_radius <= 0.
            {
                damage.0.push(crate::damage::Impact {
                    point: Some(hit.hit.point),
                    attacker: projectile.owner,
                    target: hit.target,
                    source: crate::damage::ImpactSource::Projectile(projectile.weapon.clone()),
                    category: weapon.category,
                    region: hit.hit.region,
                    amount: crate::weapons::damage_at(
                        weapon,
                        projectile.traveled + impact_distance,
                        hit.hit.region,
                    ),
                });
            }
        }
        if weapon.mechanics.explosion_radius > 0. {
            blasts.0.push(crate::equipment::BlastRequest {
                session: *session,
                owner: projectile.owner,
                definition: projectile.weapon.clone(),
                center: center - *direction * 0.003,
                radius: weapon.mechanics.explosion_radius,
                inner_radius: 0.,
                inner_damage: weapon.mechanics.explosion_damage[0],
                outer_damage: weapon.mechanics.explosion_damage[1],
                direction: None,
                category: Some(weapon.category),
                player_damage: Some(weapon.mechanics.player_damage),
                source: None,
            });
        } else {
            crate::audio::binding(
                &mut commands,
                &game,
                &format!("{}.impact", projectile.weapon.0),
                crate::audio::SoundSource::World(center),
            );
        }
        crate::staff_fields::spawn(&mut commands, projectile.owner, weapon, center);
        commands.entity(entity).despawn();
    }
}

fn remote_revive(
    mut commands: Commands,
    mut impacts: MessageReader<ProjectileImpact>,
    players: Query<(Entity, &Transform, &crate::life::Vitality), With<crate::player::Player>>,
    mut revive: MessageWriter<crate::solo_revive::RemoteRevive>,
) {
    for impact in impacts
        .read()
        .filter(|i| i.weapon.0 == "weapon.staff_revive_zm")
    {
        let mut eligible: Vec<_> = players
            .iter()
            .filter(|(entity, pose, health)| {
                *entity != impact.owner
                    && health.state() == crate::life::LifeState::Downed
                    && pose.translation.distance_squared(impact.point) < 0.8128_f32.powi(2)
            })
            .map(|(entity, _, _)| entity)
            .collect();
        eligible.sort_by_key(|e| e.to_bits());
        if let Some(player) = eligible.last().copied() {
            revive.write(crate::solo_revive::RemoteRevive {
                player,
                reviver: impact.owner,
                method: crate::solo_revive::ReviveMethod::Staff,
            });
            crate::audio::emit(
                &mut commands,
                "wpn_revivestaff_revive_plr",
                crate::audio::SoundSource::attached(player),
            );
        }
    }
}
