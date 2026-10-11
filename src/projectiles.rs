use crate::{GameState, assets::GameDefinition, gameplay::{GameStep, SessionEntity}, origins::content::{WeaponDefinition, WeaponId}};
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
}

pub fn spawn(commands: &mut Commands, owner: Entity, weapon: &WeaponDefinition, origin: Vec3, direction: Dir3) {
    commands.spawn((
        WeaponProjectile {
            owner,
            weapon: weapon.id.clone(),
            velocity: direction.as_vec3() * weapon.mechanics.projectile_speed,
            remaining: weapon.mechanics.projectile_lifetime,
            radius: 0.0254,
            traveled: 0.,
        },
        SessionEntity,
        Transform::from_translation(origin),
        Name::new(format!("Projectile {}", weapon.id.0)),
    ));
}

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, flight
            .in_set(GameStep::Combat)
            .after(crate::gameplay::combat)
            .before(crate::equipment::resolve_blasts)
            .run_if(in_state(GameState::Playing)));
    }
}

fn flight(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    session: Res<crate::session::SessionId>,
    mut projectiles: Query<(Entity, &mut WeaponProjectile, &mut Transform), Without<crate::life::Vitality>>,
    targets: crate::hit_regions::RayTargets,
    query: SpatialQuery,
    mut damage: ResMut<crate::damage::DamageQueue>,
    mut blasts: ResMut<crate::equipment::BlastQueue>,
    environmental: crate::environment::support::ShootableTargets,
    mut environment_hits: MessageWriter<crate::environment::support::EnvironmentalHit>,
) {
    for (entity, mut projectile, mut pose) in &mut projectiles {
        let Some(weapon) = game.content.weapons.iter().find(|weapon| weapon.id == projectile.weapon) else {
            commands.entity(entity).despawn();
            continue;
        };
        let dt = time.delta_secs().min(projectile.remaining.max(0.));
        projectile.remaining -= time.delta_secs();
        let delta = projectile.velocity * dt;
        let Ok(direction) = Dir3::new(delta) else { commands.entity(entity).despawn(); continue };
        let distance = delta.length();
        let actor = crate::hit_regions::cast(&targets, &query, projectile.owner, pose.translation, direction, distance);
        let filter = crate::environment::solids(std::iter::once(projectile.owner)
            .chain(targets.iter().map(|(entity, _, _, _)| entity)));
        let solid = query.cast_shape(&Collider::sphere(projectile.radius), pose.translation,
            Quat::IDENTITY, direction, &ShapeCastConfig::from_max_distance(distance), &filter);
        let environment = crate::environment::support::cast(&environmental, &query, projectile.owner,
            pose.translation, direction, distance);
        let solid_distance = solid.map_or(f32::INFINITY, |hit| hit.distance);
        let actor_distance = actor.map_or(f32::INFINITY, |hit| hit.hit.distance);
        let environment_distance = environment.map_or(f32::INFINITY, |(_, distance)| distance);
        let impact_distance = solid_distance.min(actor_distance).min(environment_distance);
        if !impact_distance.is_finite() {
            pose.translation += delta;
            projectile.traveled += distance;
            if projectile.remaining <= 0. { commands.entity(entity).despawn(); }
            continue;
        }
        let center = pose.translation + direction.as_vec3() * impact_distance;
        if environment_distance <= solid_distance && environment_distance <= actor_distance {
            if let Some((target, _)) = environment {
                environment_hits.write(crate::environment::support::EnvironmentalHit {
                    target, attacker: projectile.owner, weapon: projectile.weapon.clone(),
                    amount: weapon.damage, point: center,
                });
            }
        } else if actor_distance <= solid_distance && let Some(hit) = actor {
            damage.0.push(crate::damage::Impact {
                attacker: projectile.owner, target: hit.target,
                source: crate::damage::ImpactSource::Projectile(projectile.weapon.clone()),
                category: weapon.category, region: hit.hit.region,
                amount: crate::weapons::damage_at(weapon, projectile.traveled + impact_distance, hit.hit.region),
            });
        }
        if weapon.mechanics.explosion_radius > 0. {
            blasts.0.push(crate::equipment::BlastRequest {
                session: *session, owner: projectile.owner, definition: projectile.weapon.clone(),
                center: center - direction.as_vec3() * 0.003,
                radius: weapon.mechanics.explosion_radius,
                inner_radius: 0., inner_damage: weapon.mechanics.explosion_damage[0],
                outer_damage: weapon.mechanics.explosion_damage[1], direction: None,
            });
        } else {
            crate::audio::binding(&mut commands, &game, &format!("{}.impact", projectile.weapon.0),
                crate::audio::SoundSource::World(center));
        }
        commands.entity(entity).despawn();
    }
}
