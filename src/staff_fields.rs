use crate::{
    GameState,
    assets::GameDefinition,
    damage::{DamageQueue, Impact, ImpactSource},
    gameplay::{GameStep, SessionEntity},
    life::{LifeState, Vitality},
    origins::{DamageCategory, WeaponDefinition, WeaponId},
    weapons::HitRegion,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::BTreeMap;
#[derive(Component)]
pub struct ElementalField {
    pub owner: Entity,
    pub weapon: WeaponId,
    pub category: DamageCategory,
    pub remaining: f32,
    pub radius: f32,
    pulse: f32,
    delay: f32,
    tier: u32,
    endpoint: Option<Vec3>,
    speed: f32,
    targets: BTreeMap<Entity, FieldTarget>,
    capture_tick: f32,
}
struct FieldTarget {
    remaining: f32,
    phase: u8,
}
#[derive(Resource, Default)]
struct ActiveFields(BTreeMap<u8, Entity>);
pub struct StaffFieldsPlugin;
impl Plugin for StaffFieldsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveFields>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                |mut fields: ResMut<ActiveFields>| fields.0.clear(),
            )
            .add_systems(
                FixedUpdate,
                tick.in_set(GameStep::Combat)
                    .before(crate::gameplay::combat)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
pub fn spawn(commands: &mut Commands, owner: Entity, weapon: &WeaponDefinition, center: Vec3) {
    create(commands, owner, weapon, center, None);
}
pub fn traveling(
    commands: &mut Commands,
    owner: Entity,
    weapon: &WeaponDefinition,
    origin: Vec3,
    direction: Dir3,
    endpoint: Vec3,
) {
    if matches!(weapon.category, DamageCategory::Lightning)
        && (weapon.id.0.contains("upgraded2") || weapon.id.0.contains("upgraded3"))
    {
        create(
            commands,
            owner,
            weapon,
            origin + *direction * 2.54,
            Some(endpoint),
        );
    }
}
fn create(
    commands: &mut Commands,
    owner: Entity,
    weapon: &WeaponDefinition,
    center: Vec3,
    endpoint: Option<Vec3>,
) {
    let id = &weapon.id.0;
    let tier = if id.contains("_upgraded3_zm") {
        3
    } else if id.contains("_upgraded2_zm") {
        2
    } else {
        return;
    };
    if !id.starts_with("weapon.staff_") {
        return;
    }
    if matches!(weapon.category, DamageCategory::Lightning) && endpoint.is_none() {
        return;
    }
    let (remaining, radius, delay, key) = match weapon.category {
        DamageCategory::Fire => (5., if tier == 3 { 2.54 } else { 2.032 }, 0., None),
        DamageCategory::Ice => (if tier == 3 { 9. } else { 6. }, 4.445, 1., Some(1)),
        DamageCategory::Wind => (tier as f32 * 3.5, 6.35, 0.5, Some(2)),
        DamageCategory::Lightning => (8., if tier == 3 { 6.35 } else { 3.81 }, 0., Some(3)),
        _ => return,
    };
    let field = ElementalField {
        owner,
        weapon: weapon.id.clone(),
        category: weapon.category,
        remaining,
        radius,
        pulse: if matches!(weapon.category, DamageCategory::Fire) {
            0.2
        } else {
            0.
        },
        delay,
        tier,
        endpoint,
        speed: if tier == 3 { 30.48 / 8. } else { 20.32 / 8. },
        targets: BTreeMap::new(),
        capture_tick: 0.,
    };
    commands.queue(move |world: &mut World| {
        if let Some(key) = key
            && let Some(previous) = world.resource_mut::<ActiveFields>().0.remove(&key)
        {
            let _ = world.despawn(previous);
        }
        let entity = world
            .spawn((
                SessionEntity,
                field,
                Transform::from_translation(center),
                Visibility::Inherited,
            ))
            .id();
        if let Some(key) = key {
            world.resource_mut::<ActiveFields>().0.insert(key, entity);
        }
    });
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    query: SpatialQuery,
    slide: MoveAndSlide,
    mut fields: Query<
        (Entity, &mut Transform, &mut ElementalField),
        Without<crate::enemies::Zombie>,
    >,
    mut enemies: Query<
        (
            Entity,
            &mut Transform,
            &Collider,
            &Vitality,
            Option<&crate::rounds::PopulationKind>,
            &mut crate::combat_effects::Wounds,
            &mut crate::enemies::Zombie,
        ),
        Without<ElementalField>,
    >,
    mut damage: ResMut<DamageQueue>,
    mut session: ResMut<crate::gameplay::Session>,
    effects: Res<crate::powerups::PowerupEffects>,
) {
    let dt = time.delta_secs();
    for (_, _, _, _, _, mut wounds, _) in &mut enemies {
        wounds.field_captured = false;
    }
    for (entity, mut center, mut field) in &mut fields {
        if !game.content.weapons.iter().any(|w| w.id == field.weapon) {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(endpoint) = field.endpoint {
            let delta = endpoint - center.translation;
            center.translation +=
                delta.normalize_or_zero() * (field.speed * dt).min(delta.length());
        }
        if matches!(field.category, DamageCategory::Wind) {
            field.remaining -= dt;
        }
        field.delay -= dt;
        if field.delay > 0. {
            continue;
        }
        if !matches!(field.category, DamageCategory::Wind) {
            field.remaining -= dt;
        }
        if field.remaining <= 0. {
            commands.entity(entity).despawn();
            continue;
        }
        field.pulse -= dt;
        field.capture_tick -= dt;
        for target in field.targets.values_mut() {
            target.remaining -= dt;
        }
        let pulse = field.pulse <= 0.;
        if pulse {
            field.pulse += if matches!(field.category, DamageCategory::Fire) {
                0.2
            } else {
                0.1
            };
        }
        let radius = if matches!(field.category, DamageCategory::Fire) && field.remaining <= 0.2 {
            field.radius * 2.
        } else {
            field.radius
        };
        let mut candidates: Vec<_> = enemies
            .iter()
            .filter(|(_, _, _, health, kind, _, zombie)| {
                health.state() == LifeState::ActorAlive
                    && (!matches!(
                        field.category,
                        DamageCategory::Wind | DamageCategory::Lightning
                    ) || *kind != Some(&crate::rounds::PopulationKind::Special)
                        && zombie.definition.0 != "enemy.panzer")
            })
            .map(|(target, pose, _, _, _, _, _)| {
                (
                    target,
                    pose.translation.distance_squared(center.translation),
                )
            })
            .filter(|(_, distance)| *distance <= radius * radius)
            .collect();
        candidates.sort_by(|a, b| {
            a.1.total_cmp(&b.1)
                .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
        });
        field
            .targets
            .retain(|target, _| candidates.iter().any(|(entity, _)| entity == target));
        for (target, _) in candidates {
            let Ok((_, mut pose, shape, health, kind, mut wounds, mut zombie)) =
                enemies.get_mut(target)
            else {
                continue;
            };
            let delta = pose.translation - center.translation;
            let distance = delta.length();
            if let Ok(direction) = Dir3::new(delta) {
                if query
                    .cast_ray(
                        center.translation + Vec3::Y * 0.3,
                        direction,
                        (distance - 0.4).max(0.),
                        true,
                        &crate::environment::solids([target, entity]),
                    )
                    .is_some()
                {
                    field.targets.remove(&target);
                    continue;
                }
            }
            match field.category {
                DamageCategory::Fire if pulse => {
                    let native = 11;
                    damage.0.push(Impact {
                        point: None,
                        attacker: field.owner,
                        target,
                        source: ImpactSource::Projectile(field.weapon.clone()),
                        category: DamageCategory::Fire,
                        region: HitRegion::Torso,
                        amount: native,
                    });
                }
                DamageCategory::Ice if pulse => {
                    if kind == Some(&crate::rounds::PopulationKind::Special)
                        || zombie.definition.0 == "enemy.panzer"
                    {
                        if !field.targets.contains_key(&target) {
                            field.targets.insert(
                                target,
                                FieldTarget {
                                    remaining: 0.,
                                    phase: 0,
                                },
                            );
                        }
                        let timer = field.targets.get_mut(&target).unwrap();
                        if timer.remaining <= 0. {
                            timer.remaining = 1.;
                            damage.0.push(Impact {
                                point: None,
                                attacker: field.owner,
                                target,
                                source: ImpactSource::Status(WeaponId(
                                    "weapon.staff_water_upgraded_zm".into(),
                                )),
                                category: DamageCategory::Ice,
                                region: HitRegion::Torso,
                                amount: 3300,
                            });
                        }
                    } else if wounds.frozen == 0. && wounds.ice_recovery == 0. {
                        wounds.freeze(
                            field.owner,
                            field.weapon.clone(),
                            health.maximum(),
                            true,
                            session.random() as f32 / u32::MAX as f32,
                        );
                    }
                }
                DamageCategory::Lightning => {
                    let timer = field.targets.entry(target).or_insert(FieldTarget {
                        remaining: 0.5,
                        phase: 0,
                    });
                    if timer.remaining <= 0. {
                        if timer.phase == 0 {
                            wounds.electrical = 1.1;
                            zombie.stunned = zombie.stunned.max(1.1);
                            timer.phase = 1;
                            timer.remaining = 1.;
                        } else {
                            timer.remaining = 1.;
                            wounds.electrical = 1.1;
                            zombie.stunned = zombie.stunned.max(1.1);
                            let amount = if effects.insta_kill > 0. {
                                health.current().unwrap_or(0)
                            } else if field.tier == 3 {
                                3500
                            } else {
                                2500
                            };
                            damage.0.push(Impact {
                                point: None,
                                attacker: field.owner,
                                target,
                                source: ImpactSource::Status(field.weapon.clone()),
                                category: DamageCategory::Lightning,
                                region: HitRegion::Torso,
                                amount,
                            });
                        }
                    }
                }
                DamageCategory::Wind => {
                    if !field.targets.contains_key(&target) && field.capture_tick <= 0. {
                        field.targets.insert(
                            target,
                            FieldTarget {
                                remaining: 0.,
                                phase: 0,
                            },
                        );
                        field.capture_tick = 0.5;
                    }
                    if !field.targets.contains_key(&target) {
                        continue;
                    }
                    wounds.field_captured = true;
                    if delta.xz().length_squared() <= 0.762_f32.powi(2) {
                        damage.0.push(Impact {
                            point: None,
                            attacker: field.owner,
                            target,
                            source: ImpactSource::Status(field.weapon.clone()),
                            category: DamageCategory::Wind,
                            region: HitRegion::Torso,
                            amount: health.current().unwrap_or(0),
                        });
                        continue;
                    }
                    let speed = 1.27 / if field.tier == 3 { 0.8 } else { 1. };
                    let mut velocity = -delta.with_y(0.).normalize_or_zero() * speed;
                    let ahead = pose.translation + velocity * dt + Vec3::Y * 1.27;
                    if let Some(floor) = query.cast_ray(
                        ahead,
                        Dir3::NEG_Y,
                        3.81,
                        false,
                        &crate::environment::solids([target]),
                    ) {
                        velocity.y = ((ahead.y - floor.distance + 0.85) - pose.translation.y) / dt;
                    }
                    let result = slide.move_and_slide(
                        shape,
                        pose.translation,
                        pose.rotation,
                        velocity,
                        time.delta(),
                        &MoveAndSlideConfig::default(),
                        &crate::environment::solids([target]),
                        |_| MoveAndSlideHitResponse::Accept,
                    );
                    pose.translation = result.position;
                }
                _ => {}
            }
        }
    }
}
