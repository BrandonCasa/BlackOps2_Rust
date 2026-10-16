use crate::{
    GameState,
    damage::{DamageApplied, DamageQueue, Impact, ImpactSource},
    gameplay::GameStep,
    life::{LifeState, Vitality},
    origins::content::{DamageCategory, WeaponId},
    weapons::HitRegion,
};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct Armor {
    pub helmet: u32,
    pub helmet_maximum: u32,
    pub absorbed: u32,
    pub broken: bool,
    pub powerplant_cover: u32,
    pub powerplant: u32,
    pub powerplant_destroyed: bool,
    pub explosive_damage: u32,
    pub ice_cooldown: f32,
}
impl Armor {
    pub fn panzer(health: u32) -> Self {
        Self {
            helmet: health / 10,
            helmet_maximum: health / 10,
            absorbed: 0,
            broken: false,
            powerplant_cover: health / 20,
            powerplant: health / 40,
            powerplant_destroyed: false,
            explosive_damage: 0,
            ice_cooldown: 0.,
        }
    }
    pub fn absorb(&mut self, impact: &Impact, shotgun: bool, insta_kill: bool) -> u32 {
        let amount = impact.amount;
        let head = impact.region.head();
        let elemental = impact.source.id().contains("staff_");
        let explosive = matches!(impact.category, DamageCategory::Explosive);
        let mut body: f32 = if insta_kill { 1. } else { 0.1 };
        let mut exposed_head: f32 = if insta_kill { 2. } else { 1. };
        if shotgun {
            body *= 1.5;
            exposed_head *= 1.5;
        }
        if amount <= 10 {
            body = 1.;
        }
        let multiplier = if explosive || elemental {
            body.max(if elemental && self.broken { 1. } else { 0.5 })
        } else if head && self.broken {
            exposed_head
        } else {
            body
        };
        let delivered = (amount as f32 * multiplier).round() as u32;
        self.absorbed = self
            .absorbed
            .saturating_add(amount.saturating_sub(delivered));
        if explosive || elemental {
            self.explosive_damage = self.explosive_damage.saturating_add(delivered);
        }
        if !self.broken && (head || explosive || elemental) {
            self.helmet = self.helmet.saturating_sub(delivered);
            self.broken = self.helmet == 0;
        }
        if impact.region == HitRegion::Powerplant && !explosive && !elemental {
            if self.powerplant_cover > 0 {
                self.powerplant_cover = self.powerplant_cover.saturating_sub(delivered);
            } else if !self.powerplant_destroyed {
                self.powerplant = self.powerplant.saturating_sub(delivered);
                self.powerplant_destroyed = self.powerplant == 0;
            }
        }
        delivered
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyGib {
    Guts,
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
    NoLegs,
}

#[cfg(test)]
mod native_gib_tests {
    use super::*;

    #[test]
    fn source_gib_choices_preserve_living_leg_exclusion_and_explosion_weights() {
        assert!(gib_refs(HitRegion::LeftLowerLeg, false, None, |_| None).is_empty());
        assert_eq!(
            gib_refs(HitRegion::LeftLowerLeg, true, None, |_| None),
            vec![
                BodyGib::LeftLeg,
                BodyGib::LeftLeg,
                BodyGib::LeftLeg,
                BodyGib::NoLegs
            ]
        );
        assert_eq!(
            gib_refs(HitRegion::None, false, Some(Vec3::ZERO), |name| Some(
                if name == "j_knee_le" {
                    Vec3::ZERO
                } else {
                    Vec3::X
                }
            )),
            vec![BodyGib::LeftLeg, BodyGib::NoLegs]
        );
        assert_eq!(
            gib_refs(HitRegion::UpperTorso, false, None, |_| None),
            vec![BodyGib::Guts, BodyGib::RightArm]
        );
        assert_eq!(
            gib_refs(HitRegion::MidTorso, false, None, |_| None).len(),
            6
        );
    }
}

fn gib_refs(
    region: HitRegion,
    lethal: bool,
    point: Option<Vec3>,
    joint: impl Fn(&str) -> Option<Vec3>,
) -> Vec<BodyGib> {
    use BodyGib::*;
    if region.head() || region == HitRegion::Neck {
        return Vec::new();
    }
    if matches!(
        region,
        HitRegion::UpperTorso | HitRegion::LowerTorso | HitRegion::Torso
    ) {
        return vec![Guts, RightArm];
    }
    if region.right_arm() {
        return vec![RightArm];
    }
    if region.left_arm() {
        return vec![LeftArm];
    }
    if region.leg() {
        if !lethal {
            return Vec::new();
        }
        return if matches!(
            region,
            HitRegion::LeftUpperLeg | HitRegion::LeftLowerLeg | HitRegion::LeftFoot
        ) {
            vec![LeftLeg, LeftLeg, LeftLeg, NoLegs]
        } else {
            vec![RightLeg, RightLeg, RightLeg, NoLegs]
        };
    }
    if region == HitRegion::None {
        let Some(point) = point else {
            return Vec::new();
        };
        let tags = [
            "j_spinelower",
            "j_spineupper",
            "j_spine4",
            "j_shoulder_le",
            "j_elbow_le",
            "j_wrist_le",
            "j_shoulder_ri",
            "j_elbow_ri",
            "j_wrist_ri",
            "j_hip_le",
            "j_knee_le",
            "j_ankle_le",
            "j_hip_ri",
            "j_knee_ri",
            "j_ankle_ri",
        ];
        let nearest = tags
            .into_iter()
            .enumerate()
            .filter_map(|(index, name)| {
                joint(name).map(|position| (index, position.distance_squared(point)))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|entry| entry.0);
        return match nearest {
            Some(0..=2) => vec![Guts, RightArm],
            Some(3..=5) => vec![LeftArm],
            Some(6..=8) => vec![RightArm],
            Some(9..=11) => vec![LeftLeg, NoLegs],
            Some(12..=14) => vec![RightLeg, NoLegs],
            _ => Vec::new(),
        };
    }
    vec![Guts, RightArm, LeftArm, RightLeg, LeftLeg, NoLegs]
}

#[derive(Component, Default)]
pub struct Wounds {
    pub limbs: [u32; 3],
    pub crawler: bool,
    pub headless: bool,
    pub frozen: f32,
    pub electrical: f32,
    pub burning: f32,
    pub burned: bool,
    pub shattered: bool,
    pub left_arm: bool,
    pub right_arm: bool,
    pub body_gib: Option<BodyGib>,
    pub hatless: bool,
    pub(crate) gib_events: Vec<u8>,
    pub ice_recovery: f32,
    pub field_captured: bool,
    pub disintegrated: bool,
    pub(crate) initial_fire: Option<(f32, Impact)>,
    burn_tick: f32,
    burn_amount: u32,
    burn_owner: Option<(Entity, WeaponId)>,
    ice_damage: u32,
    ice_owner: Option<(Entity, WeaponId)>,
    bleed_tick: f32,
    bleed_amount: u32,
    bleed_impact: Option<Impact>,
    knockback: Vec3,
}
impl Wounds {
    pub fn movement_scale(&self) -> f32 {
        if self.frozen > 0. || self.ice_recovery > 0. {
            0.3
        } else {
            1.
        }
    }
    pub fn melee_disabled(&self) -> bool {
        self.burning > 0. || self.field_captured
    }
    pub fn ignite(&mut self, owner: Entity, weapon: WeaponId, multiplier: f32) {
        if self.burning > 0. {
            return;
        }
        self.burning = 8.5;
        self.burned = true;
        self.burn_tick = 0.5;
        self.burn_amount = (match weapon.0.trim_start_matches("weapon.") {
            "staff_fire_zm" => 75.,
            "staff_fire_upgraded_zm" => 150.,
            "staff_fire_upgraded2_zm" => 300.,
            "staff_fire_upgraded3_zm" => 450.,
            "one_inch_punch_fire_zm" => 250.,
            _ => 0.,
        } * multiplier)
            .round() as u32;
        self.burn_owner = Some((owner, weapon));
    }
    pub fn freeze(
        &mut self,
        owner: Entity,
        weapon: WeaponId,
        maximum: u32,
        always_kill: bool,
        random: f32,
    ) {
        if self.frozen > 0. || self.ice_recovery > 0. {
            return;
        }
        self.frozen = if always_kill {
            0.5 + random * 0.2
        } else {
            1.8 + random * 0.5
        };
        self.ice_damage = if always_kill {
            maximum
        } else if weapon.0.contains("one_inch_punch") {
            11275
        } else if weapon.0.contains("upgraded") {
            3300
        } else {
            2050
        };
        self.ice_owner = Some((owner, weapon));
    }
}
pub fn knockdown(
    wounds: &mut Wounds,
    zombie: &mut crate::enemies::Zombie,
    from: Vec3,
    position: Vec3,
    seconds: f32,
) {
    wounds.knockback = (position - from).with_y(0.).normalize_or_zero() * 10.;
    zombie.knockdown(seconds);
}
pub struct CombatEffectsPlugin;
impl Plugin for CombatEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            observe
                .in_set(GameStep::Damage)
                .after(crate::damage::resolve)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            tick.in_set(GameStep::EnemyIntent)
                .before(crate::enemies::lifecycle)
                .run_if(in_state(GameState::Playing)),
        );
    }
}
pub fn elemental_damage(weapon: &WeaponId, native: u32) -> u32 {
    match weapon.0.trim_start_matches("weapon.") {
        "staff_fire_zm" => {
            (2050. * (0.5 + 0.5 * ((native as f32 - 1.) / 10.).clamp(0., 1.))).round() as u32
        }
        "staff_fire_upgraded_zm" => {
            (3300. * (0.5 + 0.5 * ((native as f32 - 1.) / 10.).clamp(0., 1.))).round() as u32
        }
        "staff_fire_upgraded2_zm" => {
            (11500. * (0.5 + 0.5 * ((native as f32 - 1.) / 10.).clamp(0., 1.))).round() as u32
        }
        "staff_fire_upgraded3_zm" => {
            (20000. * (0.5 + 0.5 * ((native as f32 - 1.) / 10.).clamp(0., 1.))).round() as u32
        }
        _ => native,
    }
}
fn observe(
    mut applied: MessageReader<DamageApplied>,
    mut actors: Query<(
        &Vitality,
        &Transform,
        &mut Wounds,
        &mut crate::enemies::Zombie,
        &crate::rounds::PopulationKind,
    )>,
    positions: Query<&Transform>,
    mut session: ResMut<crate::gameplay::Session>,
    effects: Res<crate::powerups::PowerupEffects>,
    definition: Res<crate::origins::GameDefinition>,
    shooters: Query<&crate::weapons::WeaponInventory, With<crate::player::Player>>,
    rigs: Query<&crate::enemies::ActorRig>,
    joints: Query<&GlobalTransform>,
) {
    for event in applied.read() {
        let hit = &event.impact;
        let Ok((health, pose, mut wounds, mut zombie, kind)) = actors.get_mut(hit.target) else {
            continue;
        };
        let status = matches!(hit.source, ImpactSource::Status(_));
        if status {
            if event.lethal && matches!(hit.category, DamageCategory::Ice) {
                wounds.shattered = true;
            }
        }
        let weapon = match &hit.source {
            ImpactSource::Weapon(id) | ImpactSource::Projectile(id) | ImpactSource::Status(id) => {
                Some(id.clone())
            }
            _ => None,
        };
        let joint = |name: &str| {
            rigs.iter()
                .find_map(|rig| rig.socket(hit.target, name))
                .and_then(|entity| joints.get(entity).ok())
                .map(GlobalTransform::translation)
        };
        let current_weapon = shooters
            .get(hit.attacker)
            .ok()
            .and_then(|inventory| inventory.active())
            .map(|weapon| weapon.definition.0.trim_start_matches("weapon."));
        let grenade = weapon
            .as_ref()
            .and_then(|id| {
                definition
                    .content
                    .weapons
                    .iter()
                    .find(|weapon| weapon.id == *id)
            })
            .is_some_and(|weapon| weapon.throwable.is_some());
        let gib = matches!(
            kind,
            crate::rounds::PopulationKind::Ordinary
                | crate::rounds::PopulationKind::GeneratorDefender
        ) && wounds.burning == 0.
            && matches!(
                hit.category,
                DamageCategory::Bullet | DamageCategory::Explosive
            )
            && (!matches!(hit.category, DamageCategory::Bullet)
                || current_weapon.is_some_and(|weapon| weapon != "c96_zm"))
            && (!matches!(hit.category, DamageCategory::Explosive)
                || grenade
                || !current_weapon
                    .is_some_and(|weapon| matches!(weapon, "slipgun_zm" | "slipgun_upgraded_zm")))
            && hit.amount as f32 / event.before.max(1) as f32 >= 0.1;
        if gib {
            let limb = if hit.region.head() || hit.region == HitRegion::Neck {
                Some(0)
            } else if hit.region.left_arm() || hit.region.right_arm() {
                Some(1)
            } else if hit.region.leg() {
                Some(2)
            } else {
                None
            };
            if let Some(index) = limb {
                wounds.limbs[index] = wounds.limbs[index].saturating_add(hit.amount);
            }
            let head_point = if matches!(hit.category, DamageCategory::Explosive) {
                hit.point.zip(joint("j_head")).is_some_and(|(point, head)| {
                    point.distance(head) <= if grenade { 55. * 0.0254 } else { 10. * 0.0254 }
                })
            } else {
                (hit.region.head() || hit.region == HitRegion::Neck)
                    && health.current().unwrap_or(0) <= health.maximum() / 10
            };
            if head_point && !wounds.headless && shooters.contains(hit.attacker) {
                let event = (1 << 5) | if wounds.hatless { 0 } else { 1 << 7 };
                wounds.gib_events.push(event);
                wounds.headless = true;
                wounds.hatless = true;
                wounds.bleed_tick = 1.;
                wounds.bleed_amount = health.current().unwrap_or(0).div_ceil(5);
                let mut bleed = hit.clone();
                if let Some(weapon) = weapon.clone() {
                    bleed.source = ImpactSource::Status(weapon);
                }
                bleed.amount = wounds.bleed_amount;
                wounds.bleed_impact = Some(bleed);
            } else {
                if (hit.region.head() || hit.region == HitRegion::Neck)
                    && health.current().unwrap_or(0) > health.maximum() / 10
                    && shooters.contains(hit.attacker)
                {
                    if !wounds.hatless {
                        wounds.gib_events.push(1 << 7);
                    }
                    wounds.hatless = true;
                }
                if wounds.body_gib.is_none() {
                    let point = if matches!(hit.category, DamageCategory::Explosive) {
                        hit.point
                    } else {
                        None
                    };
                    let refs = gib_refs(hit.region, event.lethal, point, joint);
                    if !refs.is_empty() {
                        let selected = refs[session.random() as usize % refs.len()];
                        wounds.body_gib = Some(selected);
                        let pieces = match selected {
                            BodyGib::Guts => 0,
                            BodyGib::RightArm => 1 << 1,
                            BodyGib::LeftArm => 1 << 2,
                            BodyGib::RightLeg => 1 << 3,
                            BodyGib::LeftLeg => 1 << 4,
                            BodyGib::NoLegs => (1 << 3) | (1 << 4),
                        };
                        if pieces != 0 {
                            wounds.gib_events.push(pieces);
                        }
                        wounds.left_arm = selected == BodyGib::LeftArm;
                        wounds.right_arm = selected == BodyGib::RightArm;
                        wounds.crawler = matches!(
                            selected,
                            BodyGib::LeftLeg | BodyGib::RightLeg | BodyGib::NoLegs
                        );
                    }
                }
            }
        }
        if status {
            continue;
        }
        if event.lethal && (wounds.frozen > 0. || wounds.ice_recovery > 0.) {
            wounds.shattered = true;
        }
        if *kind == crate::rounds::PopulationKind::Special {
            continue;
        }
        let Some(weapon) = weapon else {
            continue;
        };
        let category = if matches!(hit.category, DamageCategory::Melee) {
            if weapon.0.contains("one_inch_punch_fire") {
                DamageCategory::Fire
            } else if weapon.0.contains("one_inch_punch_ice") {
                DamageCategory::Ice
            } else if weapon.0.contains("one_inch_punch_lightning") {
                DamageCategory::Lightning
            } else {
                hit.category
            }
        } else {
            hit.category
        };
        match category {
            DamageCategory::Fire => {
                let multiplier = if weapon.0.contains("staff_fire") {
                    0.5 + 0.5 * ((event.base_amount as f32 - 1.) / 10.).clamp(0., 1.)
                } else {
                    1.
                };
                wounds.ignite(hit.attacker, weapon, multiplier);
            }
            DamageCategory::Ice
                if wounds.frozen <= 0. && wounds.ice_recovery <= 0. && !event.lethal =>
            {
                let random = session.random() as f32 / u32::MAX as f32;
                wounds.freeze(
                    hit.attacker,
                    weapon,
                    health.maximum(),
                    effects.insta_kill > 0.,
                    random,
                );
            }
            DamageCategory::Lightning => {
                // is_electrocuted rejects another stun until this one finishes.
                if wounds.electrical <= 0. {
                    wounds.electrical = 3.;
                    zombie.stunned = zombie.stunned.max(3.);
                }
            }
            DamageCategory::Wind => {
                if let Ok(attacker) = positions.get(hit.attacker) {
                    wounds.knockback = (pose.translation - attacker.translation)
                        .with_y(0.)
                        .normalize_or_zero()
                        * 10.;
                }
                zombie.stunned = zombie.stunned.max(1.);
            }
            _ => {}
        }
    }
}
fn tick(
    time: Res<Time<Fixed>>,
    mut actors: Query<(
        Entity,
        &Vitality,
        &Collider,
        &mut Transform,
        &mut Wounds,
        &mut crate::enemies::Zombie,
    )>,
    slide: MoveAndSlide,
    mut damage: ResMut<DamageQueue>,
    effects: Res<crate::powerups::PowerupEffects>,
    definition: Res<crate::origins::GameDefinition>,
    rigs: Query<&crate::enemies::ActorRig>,
    joints: Query<&GlobalTransform>,
) {
    let dt = time.delta_secs();
    for (entity, health, collider, mut pose, mut wounds, mut zombie) in &mut actors {
        if health.state() != LifeState::ActorAlive {
            continue;
        }
        wounds.burning = (wounds.burning - dt).max(0.);
        if wounds.burning == 0. {
            wounds.burned = false;
        }
        wounds.electrical = (wounds.electrical - dt).max(0.);
        wounds.ice_recovery = (wounds.ice_recovery - dt).max(0.);
        if let Some((remaining, _)) = &mut wounds.initial_fire {
            *remaining -= dt;
            if *remaining <= 0.
                && let Some((_, impact)) = wounds.initial_fire.take()
            {
                damage.0.push(impact);
            }
        }
        if wounds.frozen > 0. {
            wounds.frozen = (wounds.frozen - dt).max(0.);
            if wounds.frozen == 0.
                && let Some((owner, weapon)) = wounds.ice_owner.take()
            {
                damage.0.push(Impact {
                    point: None,
                    attacker: owner,
                    target: entity,
                    source: ImpactSource::Status(weapon),
                    category: DamageCategory::Ice,
                    region: HitRegion::None,
                    amount: wounds.ice_damage,
                });
                wounds.ice_recovery = 0.5;
            }
        }
        if wounds.burning > 0. {
            wounds.burn_tick -= dt;
            if wounds.burn_tick <= 0.
                && let Some((owner, weapon)) = wounds.burn_owner.clone()
            {
                wounds.burn_tick += 1.;
                let amount = if effects.insta_kill > 0. {
                    health.current().unwrap_or(0)
                } else {
                    wounds.burn_amount
                };
                damage.0.push(Impact {
                    point: None,
                    attacker: owner,
                    target: entity,
                    source: ImpactSource::Status(weapon),
                    category: DamageCategory::Fire,
                    region: HitRegion::None,
                    amount,
                });
            }
        }
        if wounds.headless {
            wounds.bleed_tick -= dt;
            if wounds.bleed_tick <= 0.
                && let Some(mut impact) = wounds.bleed_impact.clone()
            {
                wounds.bleed_tick += 1.;
                impact.point = rigs
                    .iter()
                    .find_map(|rig| rig.socket(entity, "j_neck"))
                    .and_then(|entity| joints.get(entity).ok())
                    .map(GlobalTransform::translation);
                if let Some(weapon) = definition
                    .content
                    .weapons
                    .iter()
                    .find(|weapon| weapon.id.0 == impact.source.id())
                {
                    impact.amount =
                        crate::weapons::location_damage(weapon, impact.amount, impact.region);
                }
                damage.0.push(impact);
            }
        }
        if wounds.knockback.length_squared() > 0.01 {
            let result = slide.move_and_slide(
                collider,
                pose.translation,
                pose.rotation,
                wounds.knockback,
                time.delta(),
                &MoveAndSlideConfig::default(),
                &crate::environment::actor_solids([entity]),
                |_| MoveAndSlideHitResponse::Accept,
            );
            pose.translation = result.position;
            wounds.knockback *= (1. - dt * 5.).max(0.);
        }
        if wounds.electrical > 0. || wounds.field_captured {
            zombie.stunned = zombie.stunned.max(dt * 2.);
        }
    }
}
