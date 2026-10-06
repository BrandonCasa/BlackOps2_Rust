//! Input-driven Semtex purchase, holding, attachment and real-map cover workload.
use super::Verification;
use crate::{
    GameState,
    assets::GameDefinition,
    equipment::{EquipmentEvidence, GrenadeInventory, ThrowState},
    gameplay::{Session, Zombie},
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection, Usable},
    life::{LifeState, Vitality},
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
    approach: usize,
    actor: Option<Entity>,
    wall: Vec3,
    protected: Vec3,
    aim: Vec3,
    accepted: u32,
    primary: String,
    health: u32,
    rejected: bool,
    full_rejected: bool,
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
pub(super) fn standing(
    query: &SpatialQuery,
    player: Entity,
    probe: Vec3,
    height: f32,
    radius: f32,
) -> Option<Vec3> {
    let filter = crate::environment::solids([player]);
    let floor = query.cast_ray(probe + Vec3::Y * 2., Dir3::NEG_Y, 5., false, &filter)?;
    if floor.normal.y < 0.65 {
        return None;
    }
    let center = probe + Vec3::Y * (2. - floor.distance + height * 0.5 + 0.03);
    query
        .shape_intersections(
            &crate::player::capsule(radius, height),
            center,
            Quat::IDENTITY,
            &filter,
        )
        .is_empty()
        .then_some(center)
}
pub(super) fn aim(player: &mut Player, body: &Transform, point: Vec3, eye: f32) {
    let delta = point - (body.translation + Vec3::Y * (eye - player.height * 0.5));
    player.yaw = (-delta.x).atan2(-delta.z);
    player.pitch = delta.y.atan2(delta.xz().length());
}
fn drive(
    mut commands: Commands,
    verification: Option<ResMut<Verification>>,
    mut coverage: ResMut<Coverage>,
    time: Res<Time>,
    definition: Option<Res<GameDefinition>>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    focus: Res<InteractionFocus>,
    equipment: Res<EquipmentEvidence>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &WeaponInventory,
        &GrenadeInventory,
        &Vitality,
    )>,
    targets: Query<(&Usable, &Transform), Without<Player>>,
    projectiles: Query<&Transform, (With<crate::equipment::Grenade>, Without<Player>)>,
    enemies: Query<
        (Entity, &Transform, &Vitality, &crate::hit_regions::Hitboxes),
        (With<Zombie>, Without<Player>),
    >,
    query: SpatialQuery,
    mut next: ResMut<NextState<GameState>>,
) {
    let (Some(mut verification), Some(definition)) = (verification, definition) else {
        return;
    };
    if verification.stage != 4 {
        return;
    }
    let Ok((entity, mut player, mut body, inventory, grenades, health)) = players.single_mut()
    else {
        return;
    };
    input.clear();
    coverage.elapsed += time.delta_secs();
    if let Some(actor) = coverage.actor {
        session.pending = 0;
        for (other, _, v, _) in &enemies {
            if other != actor && v.state() == LifeState::ActorAlive {
                commands.entity(other).despawn();
            }
        }
    } else if let Some((actor, _, _, _)) = enemies
        .iter()
        .find(|(_, _, v, b)| v.state() == LifeState::ActorAlive && !b.volumes.is_empty())
    {
        coverage.actor = Some(actor);
        session.pending = 0;
        session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
    }
    if coverage.elapsed > 15. {
        let receipt = serde_json::json!({"passed":false,"phase":coverage.phase,"position":body.translation.to_array(),
            "focus":focus.prompt,"rejection":format!("{:?}",focus.rejection),"equipment_events":equipment.events,
            "grenade":grenades.definition.0,"state":format!("{:?}",grenades.state),"records":coverage.records});
        let _ = std::fs::write(verification.output.join("semtex.json"), receipt.to_string());
        commands.write_message(AppExit::error());
        return;
    }
    match coverage.phase {
        0 => {
            let Some((_,target))=targets.iter().find(|(t,_)|matches!(&t.source.action,
                crate::origins::InteractionAction::Weapon{weapon,..} if weapon.0=="weapon.sticky_grenade_zm")) else{return;};
            let directions = [
                Vec3::X,
                Vec3::NEG_X,
                Vec3::Z,
                Vec3::NEG_Z,
                Vec3::new(1., 0., 1.).normalize(),
                Vec3::new(-1., 0., 1.).normalize(),
                Vec3::new(1., 0., -1.).normalize(),
                Vec3::new(-1., 0., -1.).normalize(),
            ];
            let filter = crate::environment::solids([entity]);
            let approach = directions
                .into_iter()
                .cycle()
                .skip(coverage.approach)
                .take(8)
                .find_map(|dir| {
                    let center = standing(
                        &query,
                        entity,
                        target.translation + dir * 1.25,
                        player.height,
                        definition.tuning.radius,
                    )?;
                    let origin =
                        center + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5);
                    let delta = target.translation - origin;
                    (delta.length() < 2.
                        && query
                            .cast_ray(
                                origin,
                                Dir3::new(delta).ok()?,
                                (delta.length() - 0.12).max(0.),
                                true,
                                &filter,
                            )
                            .is_none())
                    .then_some(center)
                });
            if let Some(center) = approach {
                body.translation = center;
                player.velocity = Vec3::ZERO;
                aim(
                    &mut player,
                    &body,
                    target.translation,
                    definition.tuning.eye_height,
                );
                coverage.primary = inventory
                    .active()
                    .map_or(String::new(), |w| w.definition.0.clone());
                coverage.accepted = focus.accepted;
                session.points = 0;
                coverage.advance();
            }
        }
        1 => {
            if let Some((_,target))=targets.iter().find(|(t,_)|matches!(&t.source.action,crate::origins::InteractionAction::Weapon{weapon,..} if weapon.0=="weapon.sticky_grenade_zm")) {
                aim(&mut player,&body,target.translation,definition.tuning.eye_height);
            }
            if coverage.elapsed > 0.3 {
                input.use_held = true;
            }
            if !coverage.rejected
                && focus.rejection == Some(InteractionRejection::InsufficientPoints)
            {
                coverage.rejected = true;
                session.points = 1000;
                input.use_held = false;
                coverage.elapsed = 0.;
            } else if focus.accepted > coverage.accepted {
                if grenades.definition.0 != "weapon.sticky_grenade_zm"
                    || session.points != 750
                    || grenades.count != 4
                {
                    return;
                }
                let record = serde_json::json!({"event":"purchase","price":250,"balance":session.points,"count":grenades.count,"primary":coverage.primary});
                coverage.records.push(record);
                coverage.advance();
            } else if coverage.elapsed > 1. && focus.target.is_none() {
                coverage.approach += 1;
                coverage.phase = 0;
                coverage.elapsed = 0.;
            }
        }
        2 => {
            if coverage.elapsed > 0.3 {
                input.use_held = true;
            }
            if focus.rejection == Some(InteractionRejection::FullAmmo) && session.points == 750 {
                coverage.full_rejected = true;
                input.use_held = false;
                coverage.advance();
            }
        }
        3 => {
            let origin =
                body.translation + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5);
            let filter = crate::environment::solids([entity]);
            let cover = (0..24).find_map(|i| {
                let angle = i as f32 * std::f32::consts::TAU / 24.;
                let dir = Vec3::new(angle.cos(), 0., angle.sin());
                let hit = query.cast_ray(origin, Dir3::new(dir).ok()?, 3.5, true, &filter)?;
                if hit.distance < 0.6 || hit.normal.y.abs() > 0.3 {
                    return None;
                }
                let point = origin + dir * hit.distance;
                (2..=7).find_map(|n| {
                    let protected = standing(
                        &query,
                        entity,
                        point + dir * (n as f32 * 0.5),
                        player.height,
                        definition.tuning.radius,
                    )?;
                    let expected = point + hit.normal * 0.05;
                    let delta = protected - expected;
                    (delta.length() < 6.
                        && query
                            .cast_ray(
                                expected,
                                Dir3::new(delta).ok()?,
                                delta.length(),
                                true,
                                &filter,
                            )
                            .is_some())
                    .then_some((point, protected))
                })
            });
            if let Some((wall, protected)) = cover {
                coverage.wall = wall;
                coverage.protected = protected;
                coverage.health = health.current().unwrap_or(0);
                coverage.advance();
            }
        }
        4 => {
            let point = coverage.wall;
            aim(&mut player, &body, point, definition.tuning.eye_height);
            player.pitch -= 0.13;
            input.grenade_held = true;
            if coverage.elapsed > 3.0
                && matches!(grenades.state,ThrowState::Cooking{remaining} if remaining==2.)
                && equipment.detonations == 0
            {
                let record = serde_json::json!({"event":"held_without_cooking","held_seconds":coverage.elapsed,"count":grenades.count,"remaining_fuse":2.});
                coverage.records.push(record);
                coverage.advance();
            }
        }
        5 => {
            if equipment.surface_attachments > 0 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(verification.output.join("semtex-surface.png")));
                coverage.advance();
            }
        }
        6 => {
            if coverage.elapsed > 0.2 {
                if let Ok(projectile) = projectiles.single() {
                    let center = projectile.translation;
                    let filter = crate::environment::solids([entity]);
                    let protected = (0..24).find_map(|i| {
                        let angle = i as f32 * std::f32::consts::TAU / 24.;
                        let dir = Vec3::new(angle.cos(), 0., angle.sin());
                        (2..=9).find_map(|n| {
                            let position = standing(
                                &query,
                                entity,
                                center + dir * (n as f32 * 0.5),
                                player.height,
                                definition.tuning.radius,
                            )?;
                            let delta = position - center;
                            (delta.length() < 6.
                                && query
                                    .cast_ray_predicate(
                                        center,
                                        Dir3::new(delta).ok()?,
                                        delta.length(),
                                        true,
                                        &filter,
                                        &|other| !enemies.contains(other),
                                    )
                                    .is_some())
                            .then_some(position)
                        })
                    });
                    if let Some(protected) = protected {
                        coverage.protected = protected;
                    } else {
                        let record = serde_json::json!({"event":"no_cover_at_actual_contact","center":center.to_array()});
                        coverage.records.push(record);
                        commands.write_message(AppExit::error());
                        return;
                    }
                }
                body.translation = coverage.protected;
                player.velocity = Vec3::ZERO;
            }
            if equipment.detonations >= 1
                && equipment.obstructed > 0
                && health.current() == Some(coverage.health)
            {
                coverage.records.push(serde_json::json!({"event":"protected_blast","health":health.current(),"blocked_targets":equipment.obstructed,"position":body.translation.to_array()}));
                coverage.advance();
            }
        }
        7 => {
            let Some(actor) = coverage.actor else {
                return;
            };
            let Ok((_, target, v, boxes)) = enemies.get(actor) else {
                return;
            };
            if v.state() != LifeState::ActorAlive || boxes.volumes.is_empty() {
                return;
            }
            let filter = crate::environment::solids([entity]);
            let approach = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                .into_iter()
                .find_map(|dir| {
                    let center = standing(
                        &query,
                        entity,
                        target.translation + dir * 3.,
                        player.height,
                        definition.tuning.radius,
                    )?;
                    let eye =
                        center + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5);
                    let delta = target.translation - eye;
                    query
                        .cast_ray(eye, Dir3::new(delta).ok()?, delta.length(), true, &filter)
                        .is_some_and(|hit| hit.entity == actor)
                        .then_some(center)
                });
            if let Some(center) = approach {
                body.translation = center;
                player.velocity = Vec3::ZERO;
                coverage.aim = target.translation;
                coverage.advance();
            }
        }
        8 => {
            if let Some(actor) = coverage.actor
                && let Ok((_, target, _, _)) = enemies.get(actor)
            {
                coverage.aim = target.translation;
            }
            let point = coverage.aim;
            aim(&mut player, &body, point, definition.tuning.eye_height);
            player.pitch -= 0.13;
            input.grenade_held = coverage.elapsed < 0.8;
            if equipment.actor_attachments > 0 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(verification.output.join("semtex-actor.png")));
                coverage.advance();
            }
        }
        9 => {
            if coverage.elapsed > 0.15 && equipment.detonations < 2 {
                let Some(actor) = coverage.actor else {
                    return;
                };
                let Ok((_, target, _, _)) = enemies.get(actor) else {
                    return;
                };
                let escape = [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                    .into_iter()
                    .find_map(|dir| {
                        standing(
                            &query,
                            entity,
                            target.translation + dir * 10.,
                            player.height,
                            definition.tuning.radius,
                        )
                    });
                if let Some(center) = escape {
                    body.translation = center;
                    player.velocity = Vec3::ZERO;
                }
            }
            if equipment.detonations >= 2 && equipment.attached_motion > 0.1 {
                let unchanged = inventory
                    .active()
                    .is_some_and(|w| w.definition.0 == coverage.primary);
                let passed = coverage.rejected
                    && coverage.full_rejected
                    && unchanged
                    && equipment.actor_attachments > 0
                    && equipment.surface_attachments > 0
                    && equipment.obstructed > 0
                    && equipment.cooked_off == 0
                    && grenades.count == 2;
                let receipt = serde_json::json!({"passed":passed,"scope":"normal round-one combat; controlled 1000-point balance after rejection; native Semtex buy; holding beyond fuse; native wall cover; one naturally admitted round-two actor; controlled player positions",
                    "records":coverage.records,"count":grenades.count,"primary_unchanged":unchanged,"attached_motion":equipment.attached_motion,"events":equipment.events});
                let _ =
                    std::fs::write(verification.output.join("semtex.json"), receipt.to_string());
                if !passed {
                    commands.write_message(AppExit::error());
                    return;
                }
                verification.semtex_exercised = true;
                verification.stage = 1;
                verification.playing = 0.;
                next.set(GameState::Paused);
            }
        }
        _ => {}
    }
}
