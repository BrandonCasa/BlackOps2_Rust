use crate::{damage::{DamageApplied, DamageQueue, Impact, ImpactSource}, gameplay::GameStep, life::{LifeState, Vitality}, origins::content::{DamageCategory, WeaponId}, weapons::HitRegion, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct Armor {
    pub helmet: u32,
    pub helmet_maximum: u32,
    pub absorbed: u32,
    pub broken: bool,
}
impl Armor {
    pub fn panzer(health: u32) -> Self { Self { helmet: health / 10, helmet_maximum: health / 10, absorbed: 0, broken: false } }
    pub fn absorb(&mut self, impact: &Impact) -> u32 {
        let amount = impact.amount;
        if amount <= 10 { return amount; }
        let head = impact.region == HitRegion::Head;
        let elemental = impact.source.id().contains("staff_");
        let explosive = matches!(impact.category, DamageCategory::Explosive);
        let multiplier = if elemental && self.broken { 1. }
            else if explosive || elemental { 0.5 }
            else if head && self.broken { 1. } else { 0.1 };
        let delivered = (amount as f32 * multiplier).round() as u32;
        self.absorbed = self.absorbed.saturating_add(amount.saturating_sub(delivered));
        if !self.broken && (head || explosive || elemental) {
            self.helmet = self.helmet.saturating_sub(delivered);
            self.broken = self.helmet == 0;
        }
        delivered
    }
}
#[derive(Component, Default)]
pub struct Wounds {
    pub limbs: [u32; 3],
    pub crawler: bool,
    pub headless: bool,
    pub frozen: f32,
    pub electrical: f32,
    pub burning: f32,
    pub shattered: bool,
    burn_tick: f32,
    burn_amount: u32,
    burn_owner: Option<(Entity, WeaponId)>,
    ice_damage: u32,
    ice_owner: Option<(Entity, WeaponId)>,
    knockback: Vec3,
}
pub struct CombatEffectsPlugin;
impl Plugin for CombatEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, observe.in_set(GameStep::Damage).after(crate::damage::resolve).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, tick.in_set(GameStep::EnemyIntent).before(crate::enemies::lifecycle).run_if(in_state(GameState::Playing)));
    }
}
pub fn elemental_damage(weapon: &WeaponId, native: u32) -> u32 {
    let id = weapon.0.trim_start_matches("weapon.");
    match id {
        "staff_fire_zm" | "staff_air_zm" => 2050,
        "staff_fire_upgraded_zm" | "staff_air_upgraded_zm" => 3300,
        "staff_fire_upgraded2_zm" => 11500,
        "staff_fire_upgraded3_zm" => 20000,
        _ => native,
    }
}
fn observe(mut applied: MessageReader<DamageApplied>,
    mut actors: Query<(Entity, &Vitality, &Transform, &mut Wounds, &mut crate::enemies::Zombie, &crate::rounds::PopulationKind)>,
    positions: Query<&Transform>, mut damage: ResMut<DamageQueue>, spatial: SpatialQuery) {
    for event in applied.read() {
        if matches!(event.impact.source, ImpactSource::Status(_)) { continue; }
        let hit = &event.impact;
        let chain_targets: Vec<_> = if matches!(hit.category, DamageCategory::Lightning) {
            let center = positions.get(hit.target).map_or(Vec3::ZERO, |p| p.translation);
            actors.iter().filter(|(entity, health, pose, _, _, kind)| *entity != hit.target && health.state() == LifeState::ActorAlive && **kind == crate::rounds::PopulationKind::Ordinary && pose.translation.distance_squared(center) < 5.08 * 5.08)
                .filter(|(entity, _, pose, _, _, _)| {
                    let delta = pose.translation - center;
                    Dir3::new(delta).ok().is_some_and(|direction| spatial.cast_ray(center, direction, delta.length(), true, &crate::environment::solids([hit.target, *entity])).is_none())
                }).take(3).map(|(entity, _, _, _, _, _)| entity).collect()
        } else { Vec::new() };
        let Ok((_, health, pose, mut wounds, mut zombie, kind)) = actors.get_mut(hit.target) else { continue; };
        if *kind != crate::rounds::PopulationKind::Ordinary { continue; }
        let previous = health.current().unwrap_or(0).saturating_add(hit.amount).max(1);
        let limb = match hit.region { HitRegion::Head => Some(0), HitRegion::Arm => Some(1), HitRegion::Leg => Some(2), _ => None };
        if let Some(index) = limb { wounds.limbs[index] = wounds.limbs[index].saturating_add(hit.amount); }
        if !event.lethal && hit.amount as f32 / previous as f32 >= 0.1 {
            zombie.stunned = zombie.stunned.max(0.35);
            if matches!(hit.category, DamageCategory::Explosive) && matches!(hit.region, HitRegion::None | HitRegion::Leg) { wounds.crawler = true; }
        }
        if hit.region == HitRegion::Head && health.current().unwrap_or(0) <= health.maximum() / 10 && !hit.source.id().contains("c96_zm") {
            wounds.headless = true;
        }
        let weapon = match &hit.source { ImpactSource::Weapon(id) | ImpactSource::Projectile(id) => id.clone(), _ => continue };
        match hit.category {
            DamageCategory::Fire => {
                wounds.burning = 8.; wounds.burn_tick = 0.;
                wounds.burn_amount = match weapon.0.trim_start_matches("weapon.") {
                    "staff_fire_zm" => 75, "staff_fire_upgraded_zm" => 150,
                    "staff_fire_upgraded2_zm" => 300, "staff_fire_upgraded3_zm" => 450,
                    "one_inch_punch_fire_zm" => 250, _ => (hit.amount / 8).max(1),
                };
                wounds.burn_owner = Some((hit.attacker, weapon));
            }
            DamageCategory::Ice => {
                if wounds.frozen > 0. { continue; }
                wounds.frozen = 1.8 + crate::weapons::sample(hit.target.to_bits()) * 0.5;
                wounds.ice_damage = if weapon.0.contains("staff_water_upgraded2") || weapon.0.contains("staff_water_upgraded3") { health.maximum() } else if weapon.0.contains("one_inch_punch") { 11275 } else if weapon.0.contains("upgraded") { 3300 } else { 2050 };
                wounds.ice_owner = Some((hit.attacker, weapon));
                zombie.stunned = zombie.stunned.max(wounds.frozen);
            }
            DamageCategory::Lightning => {
                wounds.electrical = 3.; zombie.stunned = zombie.stunned.max(3.);
                for target in chain_targets {
                    damage.0.push(Impact { attacker: hit.attacker, target, source: ImpactSource::Status(weapon.clone()), category: DamageCategory::Lightning, region: HitRegion::Torso, amount: hit.amount / 2 });
                }
            }
            DamageCategory::Wind => {
                if let Ok(attacker) = positions.get(hit.attacker) { wounds.knockback = (pose.translation - attacker.translation).with_y(0.).normalize_or_zero() * 10.; }
                zombie.stunned = zombie.stunned.max(1.);
            }
            _ => {}
        }
    }
}
fn tick(time: Res<Time<Fixed>>, mut actors: Query<(Entity, &Vitality, &Collider, &mut Transform, &mut Wounds, &mut crate::enemies::Zombie)>,
    slide: MoveAndSlide, mut damage: ResMut<DamageQueue>) {
    let dt = time.delta_secs();
    for (entity, health, collider, mut pose, mut wounds, mut zombie) in &mut actors {
        if health.state() != LifeState::ActorAlive { wounds.shattered |= wounds.frozen > 0.; continue; }
        wounds.burning = (wounds.burning - dt).max(0.);
        wounds.electrical = (wounds.electrical - dt).max(0.);
        if wounds.frozen > 0. {
            wounds.frozen = (wounds.frozen - dt).max(0.);
            if wounds.frozen == 0. && let Some((owner, weapon)) = wounds.ice_owner.take() {
                damage.0.push(Impact { attacker: owner, target: entity, source: ImpactSource::Status(weapon), category: DamageCategory::Ice, region: HitRegion::None, amount: wounds.ice_damage });
                wounds.shattered = health.current().unwrap_or(0) <= wounds.ice_damage;
            }
        }
        if wounds.burning > 0. {
            wounds.burn_tick -= dt;
            if wounds.burn_tick <= 0. && let Some((owner, weapon)) = wounds.burn_owner.clone() {
                wounds.burn_tick += 1.;
                damage.0.push(Impact { attacker: owner, target: entity, source: ImpactSource::Status(weapon), category: DamageCategory::Fire, region: HitRegion::None, amount: wounds.burn_amount });
            }
        }
        if wounds.knockback.length_squared() > 0.01 {
            let result = slide.move_and_slide(collider, pose.translation, pose.rotation, wounds.knockback, time.delta(), &MoveAndSlideConfig::default(), &crate::environment::solids([entity]), |_| MoveAndSlideHitResponse::Accept);
            pose.translation = result.position;
            wounds.knockback *= (1. - dt * 5.).max(0.);
        }
        if wounds.electrical > 0. || wounds.frozen > 0. { zombie.stunned = zombie.stunned.max(dt * 2.); }
    }
}
