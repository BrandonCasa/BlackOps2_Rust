//! Real tactical input and naturally admitted actors, following the native developer grant.
use super::{
    Verification,
    semtex::{aim, standing},
};
use crate::{
    assets::GameDefinition,
    equipment::{EquipmentEvidence, Grenade, GrenadeInventory, TacticalInventory},
    gameplay::{Session, Zombie},
    input::GameInput,
    life::{LifeState, Vitality},
    lures::{Attractions, LureEvidence},
    player::Player,
    weapons::WeaponInventory,
};
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    elapsed: f32,
    actor: Option<Entity>,
    anchor: Vec3,
    primary: String,
    first_distance: f32,
    moved: bool,
    resumed: bool,
    full_rejected: bool,
    frame: bool,
    resume_start: Option<(f32, f32)>,
    records: Vec<serde_json::Value>,
}
impl Coverage {
    fn advance(&mut self) {
        self.phase += 1;
        self.elapsed = 0.;
    }
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn approach(
    query: &SpatialQuery,
    owner: Entity,
    actor: Entity,
    target: Vec3,
    distance: f32,
    height: f32,
    radius: f32,
) -> Option<Vec3> {
    for index in 0..32 {
        let angle = index as f32 * std::f32::consts::TAU / 32.;
        let dir = Vec3::new(angle.cos(), 0., angle.sin());
        let Some(center) = standing(query, owner, target + dir * distance, height, radius) else {
            continue;
        };
        let delta = target - center;
        let Ok(direction) = Dir3::new(delta) else {
            continue;
        };
        if query
            .cast_ray(
                center,
                direction,
                delta.length(),
                false,
                &SpatialQueryFilter::from_excluded_entities([owner, actor]),
            )
            .is_none()
        {
            return Some(center);
        }
    }
    None
}
fn drive(
    mut commands: Commands,
    verification: Option<ResMut<Verification>>,
    mut coverage: ResMut<Coverage>,
    time: Res<Time>,
    definition: Option<Res<GameDefinition>>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    lures: Res<LureEvidence>,
    equipment: Res<EquipmentEvidence>,
    attractions: Res<Attractions>,
    history: Res<crate::damage::CombatHistory>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &WeaponInventory,
        &mut TacticalInventory,
        &GrenadeInventory,
        &Vitality,
    )>,
    enemies: Query<
        (Entity, &Transform, &Vitality, &crate::hit_regions::Hitboxes),
        (With<Zombie>, Without<Player>),
    >,
    grenades: Query<(Entity, &Grenade, &Transform), Without<Player>>,
    query: SpatialQuery,
    states: (
        Res<State<crate::GameState>>,
        ResMut<NextState<crate::GameState>>,
    ),
) {
    let (state, mut next) = states;
    let (Some(mut verification), Some(definition)) = (verification, definition) else {
        return;
    };
    let Ok((owner, mut player, mut body, weapon, mut tactical, lethal, health)) =
        players.single_mut()
    else {
        return;
    };
    if verification.stage == 3
        && coverage.phase == 9
        && *state == crate::GameState::Playing
        && session.round == 1
    {
        let cleared = tactical.0.is_none() && grenades.is_empty() && attractions.0.is_empty();
        let path = verification.output.join("monkeys.json");
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(mut receipt) = serde_json::from_str::<serde_json::Value>(&text)
        {
            receipt["restart_clear"] = cleared.into();
            let _ = std::fs::write(path, receipt.to_string());
        }
        if !cleared {
            commands.write_message(AppExit::error());
        }
        return;
    }
    if verification.stage != 4 {
        return;
    }
    input.clear();
    coverage.elapsed += time.delta_secs();
    player.velocity = Vec3::ZERO;
    if let Some(actor) = coverage.actor {
        session.pending = 0;
        session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
        for (other, _, v, _) in &enemies {
            if other != actor && v.state() == LifeState::ActorAlive {
                commands.entity(other).despawn();
            }
        }
    }
    if coverage.elapsed > 25. {
        let receipt = serde_json::json!({"passed":false,"phase":coverage.phase,"stock":tactical.0.as_ref().map(|i|i.count),"position":body.translation.to_array(),"health":health.current(),"records":coverage.records,"lure_events":lures.events,"equipment":equipment.events});
        let _ = std::fs::write(
            verification.output.join("monkeys.json"),
            receipt.to_string(),
        );
        commands.write_message(AppExit::error());
        return;
    }
    match coverage.phase {
        0 => {
            let Some((actor, _, _, _)) = enemies
                .iter()
                .find(|(_, _, v, h)| v.state() == LifeState::ActorAlive && !h.volumes.is_empty())
            else {
                return;
            };
            coverage.actor = Some(actor);
            coverage.primary = weapon
                .active()
                .map_or(String::new(), |w| w.definition.0.clone());
            let monkey = definition
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.cymbal_monkey_zm")
                .expect("prepared monkey");
            if tactical.receive(monkey).is_err() {
                commands.write_message(AppExit::error());
                return;
            }
            coverage.full_rejected = matches!(
                tactical.receive(monkey),
                Err(crate::equipment::EquipmentError::Full)
            );
            coverage.records.push(serde_json::json!({"event":"native_developer_grant","stock":3,"normal_acquisition":"Mystery Box remains pending"}));
            coverage.advance();
        }
        1 | 6 => {
            let actor = coverage.actor.unwrap();
            let Ok((_, target, _, _)) = enemies.get(actor) else {
                return;
            };
            let distance = if coverage.phase == 1 { 22. } else { 4.5 };
            if let Some(center) = approach(
                &query,
                owner,
                actor,
                target.translation,
                distance,
                player.height,
                definition.tuning.radius,
            ) {
                body.translation = center;
                aim(
                    &mut player,
                    &body,
                    target.translation,
                    definition.tuning.eye_height,
                );
                player.pitch = -1.5;
                coverage.anchor = center;
                coverage.advance();
            }
        }
        2 | 7 => {
            body.translation = coverage.anchor;
            player.pitch = -1.5;
            input.tactical_held = coverage.elapsed < 0.15;
            if let Some((_, _, bomb)) = grenades
                .iter()
                .find(|(_, g, _)| g.definition.0 == "weapon.cymbal_monkey_zm")
            {
                let elapsed = coverage.elapsed;
                coverage.records.push(serde_json::json!({"event":"launch","windup_seconds":elapsed,"stock":tactical.0.as_ref().map(|i|i.count)}));
                let escape = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                    .into_iter()
                    .find_map(|dir| {
                        standing(
                            &query,
                            owner,
                            bomb.translation + dir * 12.,
                            player.height,
                            definition.tuning.radius,
                        )
                    });
                if let Some(center) = escape {
                    body.translation = center;
                    aim(
                        &mut player,
                        &body,
                        bomb.translation,
                        definition.tuning.eye_height,
                    );
                    coverage.advance();
                }
            }
        }
        3 => {
            if let Some((_, g, bomb)) = grenades.iter().find(|(_, g, _)| g.lure_active) {
                let actor = coverage.actor.unwrap();
                let Ok((_, pose, _, _)) = enemies.get(actor) else {
                    return;
                };
                coverage.first_distance = pose.translation.distance(bomb.translation);
                let position = bomb.translation;
                let remaining = g.remaining;
                let distance = coverage.first_distance;
                coverage.records.push(serde_json::json!({"event":"active","remaining":remaining,"actor_distance":distance,"position":position.to_array()}));
                // Outside the native 2.54 m blast, close enough to inspect original grounded animation.
                if let Some(center) = approach(
                    &query,
                    owner,
                    actor,
                    bomb.translation,
                    3.5,
                    player.height,
                    definition.tuning.radius,
                ) {
                    body.translation = center;
                    aim(
                        &mut player,
                        &body,
                        bomb.translation + Vec3::Y * 0.2,
                        definition.tuning.eye_height,
                    );
                }
                coverage.advance();
            }
        }
        4 => {
            if let Some((_, _, bomb)) = grenades.iter().find(|(_, g, _)| g.lure_active) {
                let actor = coverage.actor.unwrap();
                let Ok((_, pose, _, _)) = enemies.get(actor) else {
                    return;
                };
                let distance = pose.translation.distance(bomb.translation);
                if coverage.elapsed > 1. && lures.frames > 0 && !coverage.frame {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(verification.output.join("monkey-active.png")));
                    coverage.moved = distance + 0.5 < coverage.first_distance;
                    coverage.frame = true;
                }
                if coverage.elapsed > 3. {
                    let decreased = coverage.first_distance - distance;
                    coverage.records.push(serde_json::json!({"event":"attracted_motion","distance_decreased":decreased,"assigned":attractions.0.contains_key(&actor),"animation_frames":lures.frames}));
                    coverage.moved |= decreased > 0.5;
                    let escape = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                        .into_iter()
                        .find_map(|dir| {
                            standing(
                                &query,
                                owner,
                                bomb.translation + dir * 12.,
                                player.height,
                                definition.tuning.radius,
                            )
                        });
                    if let Some(center) = escape {
                        body.translation = center;
                        aim(
                            &mut player,
                            &body,
                            bomb.translation,
                            definition.tuning.eye_height,
                        );
                        coverage.advance();
                    }
                }
            }
        }
        5 => {
            if lures.detonation == 0 {
                return;
            }
            let actor = coverage.actor.unwrap();
            let Ok((_, pose, v, _)) = enemies.get(actor) else {
                return;
            };
            if v.state() != LifeState::ActorAlive {
                commands.write_message(AppExit::error());
                return;
            }
            let distance = pose.translation.distance(body.translation);
            if let Some((start, initial)) = coverage.resume_start {
                if coverage.elapsed - start > 1.5 {
                    coverage.resumed =
                        distance + 0.4 < initial && !attractions.0.contains_key(&actor);
                    coverage.records.push(serde_json::json!({"event":"normal_pursuit_resumed","decreased":initial-distance,"unassigned":!attractions.0.contains_key(&actor)}));
                    coverage.advance();
                }
            } else {
                if let Some(center) = approach(
                    &query,
                    owner,
                    actor,
                    pose.translation,
                    6.,
                    player.height,
                    definition.tuning.radius,
                ) {
                    body.translation = center;
                    aim(
                        &mut player,
                        &body,
                        pose.translation,
                        definition.tuning.eye_height,
                    );
                    coverage.resume_start =
                        Some((coverage.elapsed, pose.translation.distance(center)));
                }
            }
        }
        8 => {
            if lures.detonation < 2 {
                return;
            }
            let impacts:Vec<_>=history.recent.iter().filter(|r|r.impact.source.id()=="weapon.cymbal_monkey_zm").map(|r|serde_json::json!({"damage":r.impact.amount,"lethal":r.lethal,"target":r.impact.target.to_bits()})).collect();
            let lethal_hit = impacts.iter().any(|r| r["lethal"] == true);
            let unchanged = weapon
                .active()
                .is_some_and(|w| w.definition.0 == coverage.primary)
                && lethal.count == 4;
            let passed = coverage.moved
                && coverage.resumed
                && coverage.full_rejected
                && unchanged
                && tactical.0.as_ref().is_some_and(|i| i.count == 1)
                && lures.activated == 2
                && lures.acquired >= 2
                && lures.released > 0
                && lures.frames > 0
                && lethal_hit;
            let receipt = serde_json::json!({"passed":passed,"scope":"normal Round 1; production tactical receive following native developer grant; controlled player positions; one naturally admitted Round 2 actor; normal acquisition remains pending","records":coverage.records,"lure_events":lures.events,"animation_frames":lures.frames,"impacts":impacts,"stock":tactical.0.as_ref().map(|i|i.count),"primary_and_lethal_unchanged":unchanged});
            let _ = std::fs::write(
                verification.output.join("monkeys.json"),
                receipt.to_string(),
            );
            if !passed {
                commands.write_message(AppExit::error());
                return;
            }
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(
                    verification.output.join("monkey-detonated.png"),
                ));
            verification.monkeys_exercised = true;
            verification.stage = 1;
            verification.playing = 0.;
            coverage.advance();
            next.set(crate::GameState::Paused);
        }
        _ => {}
    }
}
