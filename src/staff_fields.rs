use crate::{assets::GameDefinition, damage::{DamageQueue, Impact, ImpactSource}, gameplay::{GameStep, SessionEntity}, life::{LifeState, Vitality}, origins::{DamageCategory, WeaponDefinition, WeaponId}, weapons::HitRegion, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
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
}
pub struct StaffFieldsPlugin;
impl Plugin for StaffFieldsPlugin {
    fn build(&self, app: &mut App) { app.add_systems(FixedUpdate, tick.in_set(GameStep::Combat).run_if(in_state(GameState::Playing))); }
}
pub fn spawn(commands: &mut Commands, owner: Entity, weapon: &WeaponDefinition, center: Vec3) {
    let id = &weapon.id.0;
    let tier = if id.contains("_upgraded3_zm") { 3 } else if id.contains("_upgraded2_zm") { 2 } else { return; };
    if !id.starts_with("weapon.staff_") { return; }
    let (remaining, radius, delay) = match weapon.category {
        DamageCategory::Fire => (5., if tier == 3 { 2.54 } else { 2.032 }, 0.),
        DamageCategory::Ice => (if tier == 3 { 9. } else { 6. }, 4.445, 1.),
        DamageCategory::Wind => (tier as f32 * 3.5, 6.35, 0.),
        DamageCategory::Lightning => (8., if tier == 3 { 6.35 } else { 3.81 }, 0.),
        _ => return,
    };
    commands.spawn((SessionEntity, ElementalField { owner, weapon: weapon.id.clone(), category: weapon.category, remaining, radius, pulse: 0., delay, tier }, Transform::from_translation(center), Visibility::Inherited));
}
fn tick(mut commands: Commands, time: Res<Time<Fixed>>, game: Res<GameDefinition>, query: SpatialQuery, slide: MoveAndSlide,
    mut fields: Query<(Entity, &Transform, &mut ElementalField), Without<crate::enemies::Zombie>>,
    mut enemies: Query<(Entity, &mut Transform, &Collider, &Vitality, &crate::rounds::PopulationKind), (With<crate::enemies::Zombie>, Without<ElementalField>)>, mut damage: ResMut<DamageQueue>) {
    let dt = time.delta_secs();
    for (entity, center, mut field) in &mut fields {
        if !game.content.weapons.iter().any(|w| w.id == field.weapon) { commands.entity(entity).despawn(); continue; }
        field.delay -= dt;
        if field.delay > 0. { continue; }
        field.remaining -= dt;
        if field.remaining <= 0. { commands.entity(entity).despawn(); continue; }
        field.pulse -= dt;
        let pulse = field.pulse <= 0.;
        let interval = if matches!(field.category, DamageCategory::Lightning) { 0.5 } else { 0.2 };
        if pulse { field.pulse += interval; }
        let radius = if matches!(field.category, DamageCategory::Fire) && field.remaining <= 0.2 { field.radius * 2. } else { field.radius };
        for (target, mut pose, shape, health, kind) in &mut enemies {
            if health.state() != LifeState::ActorAlive { continue; }
            let delta = pose.translation - center.translation;
            let distance = delta.length();
            if distance > radius { continue; }
            let Ok(direction) = Dir3::new(delta) else { continue; };
            if query.cast_ray(center.translation + Vec3::Y * 0.3, direction, (distance - 0.4).max(0.), true, &crate::environment::solids([target, entity])).is_some() { continue; }
            if matches!(field.category, DamageCategory::Wind) && *kind == crate::rounds::PopulationKind::Ordinary {
                let result = slide.move_and_slide(shape, pose.translation, pose.rotation, -delta.with_y(0.).normalize_or_zero() * 4., time.delta(), &MoveAndSlideConfig::default(), &crate::environment::solids([target]), |_| MoveAndSlideHitResponse::Accept);
                pose.translation = result.position;
            }
            if !pulse { continue; }
            let (amount, source) = match field.category {
                DamageCategory::Fire | DamageCategory::Ice => (1, ImpactSource::Projectile(field.weapon.clone())),
                DamageCategory::Lightning => (if field.tier == 3 { 1750 } else { 1250 }, ImpactSource::Status(field.weapon.clone())),
                DamageCategory::Wind => (if distance <= 1.5 { health.maximum() } else { 660 }, ImpactSource::Status(field.weapon.clone())),
                _ => continue,
            };
            damage.0.push(Impact { attacker: field.owner, target, source, category: field.category, region: HitRegion::Torso, amount });
        }
    }
}
