//! Actual-window equipment workload; native stock/limit and naturally admitted actors.
use super::{
    Verification,
    semtex::{aim, standing},
};
use crate::{
    assets::GameDefinition,
    gameplay::{Session, Zombie},
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection, Usable},
    life::{LifeState, Vitality},
    mines::{MineEvidence, MineInventory, PlantedMine},
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
    press: bool,
    home: Vec3,
    yaw: f32,
    primary: String,
    lethal: String,
    rejected: bool,
    duplicate: bool,
    actor: Option<Entity>,
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
fn drive(
    mut commands: Commands,
    verification: Option<ResMut<Verification>>,
    mut coverage: ResMut<Coverage>,
    definition: Option<Res<GameDefinition>>,
    time: Res<Time>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    focus: Res<InteractionFocus>,
    evidence: Res<MineEvidence>,
    damage: Res<crate::damage::CombatHistory>,
    query: SpatialQuery,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &WeaponInventory,
        &MineInventory,
        &crate::equipment::GrenadeInventory,
    )>,
    targets: Query<(&Usable, &Transform), Without<Player>>,
    mines: Query<(&PlantedMine, &Transform), Without<Player>>,
    enemies: Query<
        (Entity, &Transform, &Vitality, &crate::hit_regions::Hitboxes),
        (With<Zombie>, Without<Player>),
    >,
    mut next: ResMut<NextState<crate::GameState>>,
) {
    let (Some(mut verification), Some(definition)) = (verification, definition) else {
        return;
    };
    if verification.stage != 4 {
        return;
    }
    let Ok((owner, mut player, mut body, weapon, inventory, lethal)) = players.single_mut() else {
        return;
    };
    input.clear();
    coverage.elapsed += time.delta_secs();
    player.velocity = Vec3::ZERO;
    if coverage.phase <= 5 {
        session.pending = 0;
        for (entity, _, v, _) in &enemies {
            if v.state() == LifeState::ActorAlive {
                commands.entity(entity).despawn();
            }
        }
    }
    if (coverage.elapsed - time.delta_secs()).floor() != coverage.elapsed.floor() {
        let progress = serde_json::json!({"phase":coverage.phase,"elapsed":coverage.elapsed,"round":session.round,"stock":inventory.count,"placed":evidence.placed,"blocked":evidence.blocked,"triggered":evidence.triggered,"position":body.translation.to_array(),"focus":focus.prompt});
        let _ = std::fs::write(
            verification.output.join("claymore-progress.json"),
            progress.to_string(),
        );
    }
    if coverage.elapsed
        > if coverage.phase == 5 || coverage.phase == 8 {
            100.
        } else {
            15.
        }
    {
        let receipt = serde_json::json!({"passed":false,"phase":coverage.phase,"position":body.translation.to_array(),"focus":focus.prompt,"rejection":format!("{:?}",focus.rejection),"stock":inventory.count,"events":evidence.events,"records":coverage.records});
        let _ = std::fs::write(
            verification.output.join("claymores.json"),
            receipt.to_string(),
        );
        commands.write_message(AppExit::error());
        return;
    }
    match coverage.phase {
        0 => {
            coverage.primary = weapon
                .active()
                .map_or(String::new(), |w| w.definition.0.clone());
            coverage.lethal = lethal.definition.0.clone();
            coverage.home = body.translation;
            coverage.yaw = player.yaw;
            let filter = SpatialQueryFilter::from_excluded_entities([owner]);
            let approach=targets.iter().filter(|(t,_)|matches!(&t.source.action,
                crate::origins::InteractionAction::Weapon{weapon,..} if weapon.0=="weapon.claymore_zm"))
                .find_map(|(_,target)|{
                    for distance in [0.6,0.9,1.25,1.6] {
                        for index in 0..16 {
                            let angle=index as f32*std::f32::consts::TAU/16.;
                            let dir=Vec3::new(angle.cos(),0.,angle.sin());
                            let Some(center)=standing(&query,owner,target.translation+dir*distance,player.height,definition.tuning.radius) else{continue};
                            let eye=center+Vec3::Y*(definition.tuning.eye_height-player.height*0.5);
                            let delta=target.translation-eye;
                            if delta.length()<2. && query.cast_ray(eye,Dir3::new(delta).ok()?,(delta.length()-0.12).max(0.),true,&filter).is_none(){return Some((center,target.translation));}
                        }
                    }
                    None
                });
            if let Some((center, target)) = approach {
                body.translation = center;
                aim(&mut player, &body, target, definition.tuning.eye_height);
                session.points = 0;
                coverage.advance();
            }
        }
        1 => {
            input.use_held = true;
            if focus.rejection == Some(InteractionRejection::InsufficientPoints) {
                coverage.rejected = true;
                coverage
                    .records
                    .push(serde_json::json!({"event":"insufficient","points":session.points}));
                coverage.advance();
            }
        }
        2 => {
            session.points = 1500;
            if coverage.elapsed > 0.1 {
                coverage.advance();
            }
        }
        3 => {
            input.use_held = true;
            if inventory.definition.is_some() {
                coverage.records.push(serde_json::json!({"event":"purchase","points":session.points,"stock":inventory.count}));
                if session.points != 500 || inventory.count != 2 {
                    commands.write_message(AppExit::error());
                    return;
                }
                coverage.advance();
            }
        }
        4 => {
            session.points = 1500;
            if coverage.elapsed > 0.1 {
                input.use_held = true;
            }
            if focus.rejection == Some(InteractionRejection::Duplicate) {
                coverage.duplicate = true;
                coverage.advance();
            }
        }
        5 => {
            body.translation = coverage.home;
            player.yaw = coverage.yaw;
            player.pitch = -0.5;
            // A competing lethal gesture during placement/recovery must preserve its stock.
            input.grenade_held = inventory.busy();
            if evidence.placed < 12 && inventory.count > 0 && !inventory.busy() {
                coverage.press = !coverage.press;
                input.mine_held = coverage.press;
            }
            if evidence.placed == 12 && inventory.count > 0 && !inventory.busy() {
                coverage.advance();
            }
        }
        6 => {
            body.translation = coverage.home;
            player.yaw = coverage.yaw;
            input.mine_held = true;
            if evidence.unsafe_plants > 0 {
                input.mine_held = false;
                coverage.advance();
            }
        }
        7 => {
            body.translation = coverage.home;
            if let Some((_, mine_pose)) = mines.iter().next() {
                aim(
                    &mut player,
                    &body,
                    mine_pose.translation + Vec3::Y * 0.12,
                    definition.tuning.eye_height,
                );
            }
            if evidence.detonated > 0 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(verification.output.join("claymore-limit.png")));
                coverage.advance();
            }
        }
        8 => {
            if coverage.actor.is_none() {
                coverage.actor = enemies
                    .iter()
                    .find(|(_, _, v, h)| {
                        v.state() == LifeState::ActorAlive && !h.volumes.is_empty()
                    })
                    .map(|(e, _, _, _)| e);
            }
            let Some(actor) = coverage.actor else { return };
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            for (other, _, v, _) in &enemies {
                if other != actor && v.state() == LifeState::ActorAlive {
                    commands.entity(other).despawn();
                }
            }
            let Some((_, mine_pose)) = mines.iter().next() else {
                return;
            };
            let forward = mine_pose.rotation * Vec3::NEG_Z;
            let Ok((_, enemy, _, _)) = enemies.get(actor) else {
                return;
            };
            let distance = (enemy.translation - mine_pose.translation).length();
            let probe = mine_pose.translation + forward * if distance > 4. { 2. } else { -1.5 };
            if let Some(center) = standing(
                &query,
                owner,
                probe,
                player.height,
                definition.tuning.radius,
            ) {
                body.translation = center;
                aim(
                    &mut player,
                    &body,
                    mine_pose.translation,
                    definition.tuning.eye_height,
                );
            }
            if evidence.triggered > 0 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        verification.output.join("claymore-trigger.png"),
                    ));
                coverage.advance();
            }
        }
        9 => {
            body.translation = coverage.home;
            if evidence.detonated > 1 && coverage.elapsed > 0.6 {
                let unchanged = weapon
                    .active()
                    .is_some_and(|w| w.definition.0 == coverage.primary)
                    && lethal.definition.0 == coverage.lethal
                    && lethal.count == 4;
                let impacts = damage
                    .recent
                    .iter()
                    .filter(|r| r.impact.source.id() == "weapon.claymore_zm")
                    .count();
                let passed = coverage.rejected
                    && coverage.duplicate
                    && unchanged
                    && evidence.peak_owned == 12
                    && evidence.unsafe_plants == 1
                    && evidence.triggered > 0
                    && impacts > 0;
                let receipt = serde_json::json!({"passed":passed,"scope":"normal initial combat; controlled purchase balance/positions; native two-stock round refills through empty controlled intermissions; twelve overlapping normal placements; unsafe thirteenth; naturally admitted enemy; default limit and damage retained","peak_owned":evidence.peak_owned,"placed":evidence.placed,"detonated":evidence.detonated,"triggered":evidence.triggered,"replenished":evidence.replenished,"impacts":impacts,"primary_and_lethal_unchanged":unchanged,"records":coverage.records,"events":evidence.events});
                let _ = std::fs::write(
                    verification.output.join("claymores.json"),
                    receipt.to_string(),
                );
                if !passed {
                    commands.write_message(AppExit::error());
                    return;
                }
                verification.claymores_exercised = true;
                verification.stage = 1;
                verification.playing = 0.;
                next.set(crate::GameState::Paused);
            }
        }
        _ => {}
    }
}
