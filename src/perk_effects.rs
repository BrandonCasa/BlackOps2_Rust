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
pub struct CherryDefinition {
    pub radius: [f32; 2],
    pub damage: [u32; 2],
    pub cooldown_extra: f32,
    pub stun_seconds: f32,
}
#[derive(Component, Default)]
pub struct CherryState {
    pub consecutive: u32,
    pub cooldown: f32,
    blocked: BTreeSet<WeaponId>,
    reload_fx: Option<(Entity, f32)>,
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
        .add_systems(
            FixedUpdate,
            (maintain, discharge, deliver)
                .chain()
                .in_set(GameStep::Combat)
                .after(crate::gameplay::combat)
                .run_if(in_state(GameState::Playing)),
        );
    }
}
fn maintain(
    mut commands: Commands,
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
        if let Some((emitter, remaining)) = &mut state.reload_fx {
            *remaining -= time.delta_secs();
            if *remaining <= 0. {
                crate::native_fx::stop(&mut commands, *emitter);
                state.reload_fx = None;
            }
        }
        let owned = game
            .content
            .perks
            .iter()
            .any(|p| p.cherry.is_some() && perks.owned.contains(&p.id));
        if !owned || health.state() != LifeState::Standing {
            if let Some((emitter, _)) = state.reload_fx.take() {
                crate::native_fx::stop(&mut commands, emitter);
            }
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
        &WeaponInventory,
        &mut CherryState,
    )>,
    enemies: Query<(Entity, &Transform, &Vitality), With<crate::enemies::Zombie>>,
    mut evidence: ResMut<EffectEvidence>,
) {
    for event in events.read() {
        let Ok((player, pose, health, owned, inventory, mut state)) = players.get_mut(event.owner)
        else {
            continue;
        };
        let weapon = game
            .content
            .weapons
            .iter()
            .find(|w| w.dual_weapon.as_ref() == Some(&event.weapon))
            .map_or(&event.weapon, |w| &w.id);
        if event.start.is_none() {
            if inventory.hand_reload_timing(false).is_none()
                && inventory.hand_reload_timing(true).is_none()
            {
                state.blocked.remove(weapon);
            }
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
        if !state.blocked.insert(weapon.clone()) {
            evidence.record(serde_json::json!({"event":"cherry_reload_blocked","weapon":weapon.0}));
            continue;
        }
        let (magazine, capacity, reload) = event.start.unwrap();
        let magazine = if *weapon != event.weapon {
            inventory
                .owned()
                .find(|(_, w)| w.definition == *weapon)
                .map_or(magazine, |(_, w)| w.magazine)
        } else {
            magazine
        };
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
        evidence.record(serde_json::json!({"event":"cherry_discharge","weapon":weapon.0,"fraction":fraction,"radius":radius,"damage":amount,"consecutive":state.consecutive,"limit":if limit==usize::MAX{None}else{Some(limit)},"targets":targets.len(),"cooldown":state.cooldown}));
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
        crate::audio::emit(
            &mut commands,
            "zmb_cherry_explode",
            crate::audio::SoundSource::World(pose.translation),
        );
        // _zm_perk_electric_cherry::{electric_cherry_reload_fx, client callback}:
        // the original ammo fraction selects one attached effect for one second.
        if let Some((emitter, _)) = state.reload_fx.take() {
            crate::native_fx::stop(&mut commands, emitter);
        }
        let effect = if fraction >= 0.67 {
            "maps/zombie_alcatraz/fx_alcatraz_electric_cherry_sm"
        } else if fraction >= 0.33 {
            "maps/zombie_alcatraz/fx_alcatraz_electric_cherry_player"
        } else {
            "maps/zombie_alcatraz/fx_alcatraz_electric_cherry_lg"
        };
        let emitter = crate::native_fx::attach(
            &mut commands,
            effect,
            event.owner,
            Transform::from_translation(Vec3::NEG_Y * (player.height * 0.5)),
        );
        state.reload_fx = Some((emitter, 1.));
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
                point: None,
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
#[derive(Clone, serde::Deserialize)]
pub struct DiveDefinition {
    pub radius: f32,
    pub damage: [u32; 2],
    pub minimum_drop: f32,
}
fn land(
    mut commands: Commands,
    session: Res<crate::session::SessionId>,
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
        let protected = game
            .content
            .perks
            .iter()
            .any(|perk| perk.fall_protection && owned.owned.contains(&perk.id));
        let percent = crate::movement::physics::fall_percent(
            landing.impact_height,
            landing.diving,
            protected,
            landing.surface_flags,
            tuning,
        );
        if percent > 0 {
            let fraction = if percent == 100 {
                1.1
            } else {
                percent as f32 * 0.01
            };
            let amount = (fraction * health.maximum() as f32) as u32;
            damage.0.push(crate::damage::Impact {
                point: None,
                attacker: landing.owner,
                target: landing.owner,
                source: crate::damage::ImpactSource::Fall,
                category: crate::origins::DamageCategory::Environmental,
                region: crate::weapons::HitRegion::Torso,
                amount,
            });
            evidence.record(serde_json::json!({"event":"fall_impact","height":landing.height,"impact_height":landing.impact_height,"diving":landing.diving,"percent":percent,"speed":landing.speed,"amount":amount}));
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
                crate::audio::emit(
                    &mut commands,
                    "zmb_phdflop_explo",
                    crate::audio::SoundSource::World(landing.position),
                );
                crate::native_fx::emit(
                    &mut commands,
                    "maps/zombie/fx_zmb_phdflopper_exp",
                    Transform::from_translation(landing.position),
                );
                blasts.0.push(crate::equipment::BlastRequest {
                    session: *session,
                    owner: landing.owner,
                    definition: drink.bottle.clone(),
                    center: landing.position + Vec3::Y * 0.15,
                    radius: dive.radius,
                    inner_radius: 0.,
                    inner_damage: dive.damage[1],
                    outer_damage: dive.damage[0],
                    direction: None,
                    category: None,
                    player_damage: None,
                    source: None,
                });
                evidence.record(serde_json::json!({"event":"dive_explosion","height":landing.height,"radius":dive.radius,"damage":dive.damage}));
            }
        }
    }
}
