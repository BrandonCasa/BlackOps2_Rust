//! Source-driven reload discharges extend shared inventory, enemy and damage systems.
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    life::{LifeState, Vitality},
    origins::{PerkId, WeaponId},
    player::Player,
    solo_revive::PerkInventory,
    weapons::WeaponInventory,
};
use bevy::prelude::*;
use std::collections::BTreeSet;
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CherryDefinition {
    pub radius: [f32; 2],
    pub damage: [u32; 2],
    pub cooldown_extra: f32,
    pub stun_seconds: f32,
}
impl CherryDefinition {
    pub fn valid(&self) -> bool {
        self.radius.iter().all(|r| r.is_finite() && *r > 0.)
            && self.radius[0] < self.radius[1]
            && self.damage[0] > 0
            && self.damage[0] <= self.damage[1]
            && self.damage[1] <= i32::MAX as u32
            && [self.cooldown_extra, self.stun_seconds]
                .iter()
                .all(|x| x.is_finite() && *x > 0.)
    }
}
#[derive(Component, Default)]
pub struct CherryState {
    pub consecutive: u32,
    pub cooldown: f32,
    blocked: BTreeSet<WeaponId>,
}
#[derive(Message, Clone)]
pub struct ReloadEvent {
    pub owner: Entity,
    pub weapon: WeaponId,
    pub start: Option<(u32, u32, f32)>,
}
#[derive(Resource, Default)]
pub struct EffectEvidence {
    pub events: Vec<serde_json::Value>,
}
impl EffectEvidence {
    pub fn record(&mut self, event: serde_json::Value) {
        if self.events.len() < 128 {
            self.events.push(event);
        }
    }
}
#[derive(Component)]
struct Shock {
    owner: Entity,
    target: Entity,
    perk: PerkId,
    amount: u32,
    remaining: f32,
    stun: f32,
}
#[derive(Component)]
struct Flash(f32);
#[derive(Resource, Default)]
struct FlashAssets {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
}
pub struct PerkEffectsPlugin;
impl Plugin for PerkEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            land.in_set(GameStep::Combat)
                .before(crate::equipment::resolve_blasts)
                .run_if(in_state(GameState::Playing)),
        )
        .add_message::<ReloadEvent>()
        .init_resource::<EffectEvidence>()
        .init_resource::<FlashAssets>()
        .add_systems(
            FixedUpdate,
            (maintain, discharge, deliver, fade)
                .chain()
                .in_set(GameStep::Combat)
                .after(crate::gameplay::combat)
                .run_if(in_state(GameState::Playing)),
        );
    }
}
fn maintain(
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut players: Query<(
        Entity,
        &PerkInventory,
        &Vitality,
        &WeaponInventory,
        &mut CherryState,
    )>,
    mut evidence: ResMut<EffectEvidence>,
) {
    for (owner, perks, health, inventory, mut state) in &mut players {
        let owned = game
            .content
            .perks
            .iter()
            .any(|p| p.cherry.is_some() && perks.owned.contains(&p.id));
        if !owned || health.state() != LifeState::Standing {
            *state = CherryState::default();
            continue;
        }
        state
            .blocked
            .retain(|id| inventory.owned().any(|(_, w)| w.definition == *id));
        if state.cooldown > 0. {
            state.cooldown = (state.cooldown - time.delta_secs()).max(0.);
            if state.cooldown == 0. {
                state.consecutive = 0;
                evidence.record(
                    serde_json::json!({"event":"cherry_cooldown_reset","owner":owner.to_bits()}),
                );
            }
        }
    }
}
fn discharge(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut events: MessageReader<ReloadEvent>,
    game: Res<GameDefinition>,
    mut players: Query<(
        &Player,
        &Transform,
        &Vitality,
        &PerkInventory,
        &mut CherryState,
    )>,
    enemies: Query<(Entity, &Transform, &Vitality), With<crate::enemies::Zombie>>,
    mut evidence: ResMut<EffectEvidence>,
    mut visual: ResMut<FlashAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in events.read() {
        let Ok((player, pose, health, owned, mut state)) = players.get_mut(event.owner) else {
            continue;
        };
        if event.start.is_none() {
            state.blocked.remove(&event.weapon);
            continue;
        }
        let Some(perk) = game
            .content
            .perks
            .iter()
            .find(|p| p.cherry.is_some() && owned.owned.contains(&p.id))
        else {
            continue;
        };
        if health.state() != LifeState::Standing {
            continue;
        }
        if !state.blocked.insert(event.weapon.clone()) {
            evidence.record(
                serde_json::json!({"event":"cherry_reload_blocked","weapon":event.weapon.0}),
            );
            continue;
        }
        let (magazine, capacity, reload) = event.start.unwrap();
        let fraction = (magazine as f32 / capacity.max(1) as f32).clamp(0., 1.);
        let settings = perk.cherry.as_ref().unwrap();
        let radius = settings.radius[1] + (settings.radius[0] - settings.radius[1]) * fraction;
        let amount = (settings.damage[1] as f32
            + (settings.damage[0] as f32 - settings.damage[1] as f32) * fraction)
            .round() as u32;
        state.consecutive = state.consecutive.saturating_add(1);
        state.cooldown = reload + settings.cooldown_extra;
        let limit = match state.consecutive {
            0 | 1 => usize::MAX,
            2 => 8,
            3 => 4,
            4 => 2,
            _ => 0,
        };
        let center = pose.translation - Vec3::Y * (player.height * 0.5);
        let mut targets: Vec<_> = enemies
            .iter()
            .filter(|(_, _, v)| v.state() == LifeState::ActorAlive)
            .map(|(e, t, _)| (e, (t.translation - Vec3::Y * 0.85).distance_squared(center)))
            .filter(|(_, d)| *d <= radius * radius)
            .collect();
        targets.sort_by(|a, b| {
            a.1.total_cmp(&b.1)
                .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
        });
        targets.truncate(limit);
        evidence.record(serde_json::json!({"event":"cherry_discharge","weapon":event.weapon.0,"fraction":fraction,"radius":radius,"damage":amount,"consecutive":state.consecutive,"limit":if limit==usize::MAX{None}else{Some(limit)},"targets":targets.len(),"cooldown":state.cooldown}));
        for (i, (target, _)) in targets.into_iter().enumerate() {
            commands.spawn((
                SessionEntity,
                Shock {
                    owner: event.owner,
                    target,
                    perk: perk.id.clone(),
                    amount,
                    remaining: 0.1 * (i + 1) as f32 + time.delta_secs(),
                    stun: settings.stun_seconds,
                },
            ));
        }
        if limit == 0 {
            continue;
        }
        let mesh = visual
            .mesh
            .get_or_insert_with(|| meshes.add(Sphere::new(0.05).mesh().ico(1).unwrap()))
            .clone();
        let material = visual
            .material
            .get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::srgb(0.3, 0.55, 1.),
                    emissive: LinearRgba::new(3., 8., 20., 1.),
                    unlit: true,
                    ..default()
                })
            })
            .clone();
        for i in 0..16 {
            let angle = i as f32 * std::f32::consts::TAU / 16.;
            commands.spawn((
                SessionEntity,
                Flash(1.),
                bevy::light::NotShadowCaster,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(
                    center + Vec3::new(angle.cos() * radius, 0.25, angle.sin() * radius),
                ),
            ));
        }
    }
}
fn deliver(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut shocks: Query<(Entity, &mut Shock)>,
    owners: Query<(&Vitality, &PerkInventory)>,
    mut enemies: Query<(&Vitality, &mut crate::enemies::Zombie)>,
    mut queue: ResMut<crate::damage::DamageQueue>,
    mut evidence: ResMut<EffectEvidence>,
) {
    for (entity, mut shock) in &mut shocks {
        if !owners
            .get(shock.owner)
            .is_ok_and(|(v, p)| v.state() == LifeState::Standing && p.owned.contains(&shock.perk))
        {
            commands.entity(entity).despawn();
            continue;
        }
        shock.remaining -= time.delta_secs();
        if shock.remaining > 0.00001 {
            continue;
        }
        if let Ok((health, mut zombie)) = enemies.get_mut(shock.target)
            && health.state() == LifeState::ActorAlive
        {
            if health.current().is_some_and(|h| h > shock.amount) {
                zombie.stunned = zombie.stunned.max(shock.stun);
                queue.0.retain(|impact| {
                    !matches!(impact.source, crate::damage::ImpactSource::Enemy(_))
                        || impact.attacker != shock.target
                });
            }
            queue.0.push(crate::damage::Impact {
                attacker: shock.owner,
                target: shock.target,
                source: crate::damage::ImpactSource::Perk(shock.perk.clone()),
                category: crate::origins::DamageCategory::Lightning,
                region: crate::weapons::HitRegion::Torso,
                amount: shock.amount,
            });
            evidence.record(serde_json::json!({"event":"cherry_impact","target":shock.target.to_bits(),"amount":shock.amount,"stunned":zombie.stunned}));
        }
        commands.entity(entity).despawn();
    }
}
fn fade(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut flashes: Query<(Entity, &mut Flash, &mut Transform)>,
) {
    for (e, mut flash, mut pose) in &mut flashes {
        flash.0 -= time.delta_secs();
        pose.scale = Vec3::splat(flash.0.max(0.));
        if flash.0 <= 0. {
            commands.entity(e).despawn();
        }
    }
}

#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiveDefinition {
    pub radius: f32,
    pub damage: [u32; 2],
    pub minimum_drop: f32,
}
impl DiveDefinition {
    pub fn valid(&self) -> bool {
        self.radius.is_finite()
            && self.radius > 0.
            && self.minimum_drop.is_finite()
            && self.minimum_drop > 0.
            && self.damage[0] > 0
            && self.damage[0] <= self.damage[1]
            && self.damage[1] <= i32::MAX as u32
    }
}
fn land(
    mut landings: MessageReader<crate::movement::Landing>,
    players: Query<(&Vitality, &PerkInventory), With<Player>>,
    game: Res<GameDefinition>,
    mut damage: ResMut<crate::damage::DamageQueue>,
    mut blasts: ResMut<crate::equipment::BlastQueue>,
    mut evidence: ResMut<EffectEvidence>,
) {
    for landing in landings.read() {
        let Ok((health, owned)) = players.get(landing.owner) else {
            continue;
        };
        if health.state() != LifeState::Standing {
            continue;
        }
        let tuning = &game.tuning.mobility;
        if landing.height > tuning.fall_minimum {
            let fraction = ((landing.height - tuning.fall_minimum)
                / (tuning.fall_fatal - tuning.fall_minimum))
                .clamp(0., 1.);
            let amount = (fraction * health.maximum() as f32).ceil() as u32;
            damage.0.push(crate::damage::Impact {
                attacker: landing.owner,
                target: landing.owner,
                source: crate::damage::ImpactSource::Fall,
                category: crate::origins::DamageCategory::Environmental,
                region: crate::weapons::HitRegion::Torso,
                amount,
            });
            evidence.record(serde_json::json!({"event":"fall_impact","height":landing.height,"speed":landing.speed,"amount":amount}));
        }
        if !landing.diving {
            continue;
        }
        for perk in game
            .content
            .perks
            .iter()
            .filter(|p| owned.owned.contains(&p.id))
        {
            if let (Some(dive), Some(drink)) = (&perk.dive, &perk.drink) {
                if landing.height < dive.minimum_drop {
                    continue;
                }
                blasts.0.push(crate::equipment::BlastRequest {
                    owner: landing.owner,
                    definition: drink.bottle.clone(),
                    center: landing.position + Vec3::Y * 0.15,
                    radius: dive.radius,
                    inner_radius: 0.,
                    inner_damage: dive.damage[1],
                    outer_damage: dive.damage[0],
                    direction: None,
                });
                evidence.record(serde_json::json!({"event":"dive_explosion","height":landing.height,"radius":dive.radius,"damage":dive.damage}));
            }
        }
    }
}
