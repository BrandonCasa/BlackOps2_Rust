use super::*;
use crate::{
    life::{LifeState, Vitality},
    solo_revive::{PerkInventory, SoloEpisode, SoloReviveEvidence},
    weapons::WeaponInventory,
};
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    elapsed: f32,
    cycle: u32,
    before: Option<Vec3>,
    crawl: f32,
    downed_shot: bool,
    saved: Vec<(String, u32, u32)>,
    restored: u32,
    insufficient: bool,
    held_once: bool,
    machine_removed: bool,
    third_picture: bool,
    restrictions: bool,
    heals: usize,
    snapshots: Vec<serde_json::Value>,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn ammo(inventory: &WeaponInventory, definition: &GameDefinition) -> Vec<(String, u32, u32)> {
    inventory
        .owned()
        .filter_map(|(_, owned)| {
            definition
                .content
                .weapons
                .iter()
                .find(|w| w.id == owned.definition)
                .map(|w| (w.id.0.clone(), owned.magazine, inventory.reserve(w)))
        })
        .collect()
}
fn picture(commands: &mut Commands, output: &std::path::Path, name: &str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(output.join(name)));
}
fn drive(
    mut commands: Commands,
    verification: Option<Res<Verification>>,
    definition: Option<Res<GameDefinition>>,
    mut coverage: ResMut<Coverage>,
    time: Res<Time>,
    state: Res<State<GameState>>,
    failure: Res<LoadFailure>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Vitality,
        &PerkInventory,
        &WeaponInventory,
        Option<&mut SoloEpisode>,
        Option<&crate::perks::PerkDrinking>,
    )>,
    focus: Res<crate::interactions::InteractionFocus>,
    evidence: Res<SoloReviveEvidence>,
    health_evidence: Res<crate::player_health::HealthEvidence>,
    machines: Query<&crate::interactions::Usable>,
    perk_evidence: Res<crate::perks::PerkEvidence>,
    mut impacts: ResMut<crate::damage::DamageQueue>,
) {
    let Some(verification) = verification else {
        return;
    };
    if !verification.solo_revive {
        return;
    }
    if verification.started.elapsed().as_secs() > 240 || **state == GameState::LoadError {
        let _ = std::fs::write(verification.output.join("solo-revive.json"), serde_json::json!({"passed":false,"phase":coverage.phase,"failure":failure.0,"events":evidence.events,"snapshots":coverage.snapshots}).to_string());
        commands.write_message(AppExit::error());
        return;
    }
    let (
        Some(definition),
        Ok((owner, mut player, mut transform, health, perks, inventory, mut episode, drinking)),
    ) = (definition, players.single_mut())
    else {
        return;
    };
    input.clear();
    coverage.elapsed += time.delta_secs();
    if **state == GameState::Playing && matches!(coverage.phase, 0..=2 | 10..=11) {
        commands.queue(|world: &mut World| super::power_fixture(world, "placement.quick_revive"));
    }
    let placement = definition
        .content
        .placements
        .iter()
        .find(|p| p.id.0 == "placement.quick_revive")
        .expect("prepared machine");
    let machine = Vec3::from_array(placement.position);
    let rotation = Quat::from_rotation_y(placement.yaw);
    let travel_to_machine = |player: &mut Player, transform: &mut Transform| {
        transform.translation = machine + rotation * Vec3::new(0., player.height * 0.5 + 0.02, 2.2);
        player.velocity = Vec3::ZERO;
        let target = machine + Vec3::Y + rotation * Vec3::new(0., 0., 0.8);
        let delta = target
            - (transform.translation
                + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5));
        player.yaw = (-delta.x).atan2(-delta.z);
        player.pitch = (delta.y / delta.length()).asin();
    };
    let _ = std::fs::write(verification.output.join("progress.json"), serde_json::json!({"phase":coverage.phase,"seconds":coverage.elapsed,"cycle":coverage.cycle,"health":health.current(),"life":format!("{:?}",health.state()),"purchases":perks.solo_purchases,"charges":perks.solo_charges,"position":transform.translation.to_array(),"prompt":focus.prompt,"rejection":format!("{:?}",focus.rejection),"events":evidence.events}).to_string());
    if matches!(coverage.phase, 1 | 2 | 3 | 6 | 10 | 11 | 15 | 16)
        && health.state() == LifeState::Standing
    {
        travel_to_machine(&mut player, &mut transform);
        commands
            .entity(owner)
            .insert(avian3d::prelude::Position(transform.translation));
    }
    match coverage.phase {
        0 if **state == GameState::Playing && coverage.elapsed > 3. => {
            travel_to_machine(&mut player, &mut transform);
            commands
                .entity(owner)
                .insert(avian3d::prelude::Position(transform.translation));
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            session.points = 499;
            coverage.elapsed = 0.;
            coverage.phase = if verification.solo_bleedout { 10 } else { 1 };
        }
        1 => {
            input.use_held = true;
            if focus.rejection
                == Some(crate::interactions::InteractionRejection::InsufficientPoints)
            {
                coverage.insufficient = session.points == 499 && perks.solo_purchases == 0;
                picture(
                    &mut commands,
                    &verification.output,
                    "quick-revive-machine.png",
                );
                coverage.elapsed = 0.;
                coverage.phase = 2;
            }
        }
        2 if coverage.elapsed > 0.15 => {
            session.points = 500;
            coverage.elapsed = 0.;
            coverage.phase = 3;
        }
        3 => {
            input.use_held = true;
            if let Some(drink) = drinking {
                if drink.elapsed < 3.2 {
                    input.fire = true;
                    input.fire_held = true;
                    input.reload = true;
                    input.melee = true;
                    input.ads = true;
                    input.cycle_weapon = true;
                    input.weapon_slot = Some(1);
                    input.grenade_held = true;
                    input.tactical_held = true;
                    input.mine_held = true;
                }
            }
            if perks.solo_charges == 1 && drinking.is_none() && coverage.elapsed > 0.3 {
                if coverage.cycle == 2 {
                    coverage.machine_removed = !machines
                        .iter()
                        .any(|m| m.source.id.0 == "interaction.quick_revive");
                    if !coverage.third_picture {
                        picture(
                            &mut commands,
                            &verification.output,
                            "third-purchase-machine-gone.png",
                        );
                        coverage.third_picture = true;
                        coverage.elapsed = 0.;
                        return;
                    }
                    if coverage.elapsed < 0.5 {
                        return;
                    }
                }
                coverage.held_once |=
                    perks.solo_purchases == coverage.cycle + 1 && session.points == 0;
                coverage.saved = ammo(inventory, &definition);
                let saved = coverage.saved.clone();
                coverage.snapshots.push(serde_json::json!({"event":"bought","purchase":perks.solo_purchases,"points":session.points,"owned":perks.owned,"ammo":saved}));
                let feet = Vec3::from_array(definition.spawn);
                transform.translation = feet + Vec3::Y * player.height * 0.5;
                player.velocity = Vec3::ZERO;
                commands
                    .entity(owner)
                    .insert(avian3d::prelude::Position(transform.translation));
                session.pending = 1;
                session.intermission = None;
                session.spawn_timer = Timer::from_seconds(0.1, TimerMode::Once);
                coverage.elapsed = 0.;
                coverage.phase = 4;
            }
        }
        4 if health.state() == LifeState::Downed && episode.is_some() => {
            let weapon = inventory.active().expect("downed weapon");
            if weapon.definition != definition.solo_revive.downed_weapon
                || health.current() != Some(1)
                || !perks.owned.is_empty()
            {
                coverage.phase = 99;
                return;
            }
            let cycle = coverage.cycle;
            coverage.snapshots.push(serde_json::json!({"event":"downed","cycle":cycle,"ammo":ammo(inventory,&definition),"height":player.height}));
            picture(
                &mut commands,
                &verification.output,
                &format!("downed-{}.png", coverage.cycle),
            );
            coverage.before = Some(transform.translation);
            coverage.elapsed = 0.;
            coverage.phase = 5;
        }
        5 => {
            if episode.as_ref().is_some_and(|e| e.remaining > 0.2) {
                input.movement = Vec2::X;
                input.sprint = true;
                input.jump = true;
                input.ads = true;
                input.melee = true;
                input.cycle_weapon = true;
                input.weapon_slot = Some(1);
                input.grenade_held = true;
                input.mine_held = true;
                input.tactical_held = true;
                input.use_held = true;
                if coverage.elapsed > 1. && coverage.elapsed < 1.1 {
                    input.fire = true;
                }
            }
            if let Some(before) = coverage.before {
                coverage.crawl = coverage
                    .crawl
                    .max(transform.translation.xz().distance(before.xz()));
            }
            coverage.downed_shot |= inventory.active().is_some_and(|w| w.magazine < 50);
            if episode.is_some() && coverage.elapsed > 2. && coverage.elapsed < 9. {
                coverage.restrictions |= player.crouched
                    && (player.height - definition.solo_revive.downed_height).abs() < 0.001
                    && player.velocity.xz().length() <= definition.solo_revive.downed_speed + 0.01
                    && inventory.ads < 0.01
                    && inventory.active_slot() == 0
                    && !matches!(inventory.action, crate::weapons::WeaponAction::Melee { .. });
            }
            if health.state() == LifeState::Standing && episode.is_none() {
                if ammo(inventory, &definition) != coverage.saved
                    || perks.solo_charges != 0
                    || !perks.owned.is_empty()
                {
                    coverage.phase = 99;
                    return;
                }
                coverage.restored += 1;
                coverage.cycle += 1;
                coverage.elapsed = 0.;
                if coverage.cycle < 3 {
                    travel_to_machine(&mut player, &mut transform);
                    session.points = 500;
                    coverage.phase = 6;
                } else {
                    transform.translation =
                        Vec3::from_array(definition.spawn) + Vec3::Y * player.height * 0.5;
                    commands
                        .entity(owner)
                        .insert(avian3d::prelude::Position(transform.translation));
                    player.velocity = Vec3::ZERO;
                    session.pending = 1;
                    session.intermission = None;
                    session.spawn_timer = Timer::from_seconds(0.1, TimerMode::Once);
                    coverage.phase = 7;
                }
            }
        }
        6 if coverage.elapsed > 0.15 => {
            coverage.elapsed = 0.;
            coverage.phase = 3;
        }
        7 if **state == GameState::GameOver => {
            picture(
                &mut commands,
                &verification.output,
                "no-charge-game-over.png",
            );
            coverage.snapshots.push(serde_json::json!({"event":"terminal","health":health.current(),"purchases":perks.solo_purchases,"charges":perks.solo_charges,"deaths":health_evidence.death_events}));
            commands.write_message(crate::input::RestartGame);
            coverage.phase = 8;
            coverage.elapsed = 0.;
        }
        8 if **state == GameState::Playing && coverage.elapsed > 0.5 => {
            let revives: Vec<_> = evidence
                .events
                .iter()
                .filter(|e| e["event"] == "revived")
                .collect();
            let timings = revives.iter().all(|e| {
                e["seconds"]
                    .as_f64()
                    .is_some_and(|v| (v - definition.solo_revive.seconds as f64).abs() < 0.06)
            });
            let passed = coverage.insufficient
                && coverage.held_once
                && coverage.restored == 3
                && revives.len() == 3
                && timings
                && coverage.crawl > 0.1
                && coverage.downed_shot
                && coverage.machine_removed
                && coverage.restrictions
                && health.state() == LifeState::Standing
                && health.current() == Some(100)
                && perks.solo_purchases == 0
                && perks.solo_charges == 0
                && perks.owned.is_empty()
                && episode.is_none()
                && health_evidence.death_events == 1
                && perk_evidence
                    .events
                    .iter()
                    .filter(|e| e["event"] == "perk_activated")
                    .count()
                    == 3
                && perk_evidence
                    .events
                    .iter()
                    .filter(|e| e["event"] == "perk_activated")
                    .all(|e| {
                        e["seconds"]
                            .as_f64()
                            .is_some_and(|v| (v - 3.3).abs() < 0.04)
                    });
            picture(&mut commands, &verification.output, "restart-clear.png");
            let receipt = serde_json::json!({"passed":passed,"scope":"controlled travel and 499/500-point budgets; real held-F purchases, natural enemy attacks, three automatic revivals, fourth lethal episode and restart; downed firing does not qualify native projectile delivery", "events":evidence.events,"snapshots":coverage.snapshots,"crawl_meters":coverage.crawl,"downed_shot":coverage.downed_shot,"restored":coverage.restored,"timings":timings,"perk_events":perk_evidence.events,"machine_removed":coverage.machine_removed,"restrictions":coverage.restrictions,"restart_owner":owner.to_bits()});
            let _ = std::fs::write(
                verification.output.join("solo-revive.json"),
                receipt.to_string(),
            );
            coverage.phase = if passed { 15 } else { 99 };
            coverage.elapsed = 0.;
        }
        10 if coverage.elapsed > 0.15 => {
            session.points = 500;
            coverage.phase = 11;
            coverage.elapsed = 0.;
        }
        11 => {
            input.use_held = true;
            if perks.solo_charges == 1 && drinking.is_none() {
                transform.translation =
                    Vec3::from_array(definition.spawn) + Vec3::Y * player.height * 0.5;
                commands
                    .entity(owner)
                    .insert(avian3d::prelude::Position(transform.translation));
                player.velocity = Vec3::ZERO;
                session.pending = 1;
                session.intermission = None;
                session.spawn_timer = Timer::from_seconds(0.1, TimerMode::Once);
                coverage.phase = 12;
                coverage.elapsed = 0.;
            }
        }
        12 if health.state() == LifeState::Downed => {
            if let Some(episode) = episode.as_mut() {
                episode.remaining = 100.;
                coverage.heals = health_evidence.heals.len();
                coverage.phase = 13;
                coverage.elapsed = 0.;
                picture(
                    &mut commands,
                    &verification.output,
                    "delayed-revival-downed.png",
                );
            }
        }
        13 if **state == GameState::GameOver => {
            let bleedout = evidence.events.iter().find(|e| e["event"] == "bled_out");
            let passed = bleedout.is_some_and(|e| {
                e["seconds"].as_f64().is_some_and(|v| {
                    (v - definition.solo_revive.bleedout_seconds as f64).abs() < 0.05
                })
            }) && health_evidence.death_events == 1
                && health_evidence.heals.len() == coverage.heals
                && episode.is_none();
            let receipt = serde_json::json!({"passed":passed,"scope":"real paid Quick Revive and natural enemy downing, then controlled delay of automatic revival to expose the unchanged 45-second session bleedout clock; not a normal uninterrupted Quick Revive outcome", "events":evidence.events,"deaths":health_evidence.death_events,"downed_heals":health_evidence.heals.len()-coverage.heals});
            let _ = std::fs::write(
                verification.output.join("solo-bleedout.json"),
                receipt.to_string(),
            );
            picture(
                &mut commands,
                &verification.output,
                "bleedout-game-over.png",
            );
            if passed {
                commands.write_message(crate::input::RestartGame);
                coverage.phase = 14;
                coverage.elapsed = 0.;
            } else {
                coverage.phase = 99;
            }
        }
        14 if **state == GameState::Playing && coverage.elapsed > 0.5 => {
            let passed = health.state() == LifeState::Standing
                && health.current() == Some(100)
                && perks.solo_purchases == 0
                && perks.solo_charges == 0
                && episode.is_none();
            let _ = std::fs::write(
                verification.output.join("bleedout-restart.json"),
                serde_json::json!({"passed":passed}).to_string(),
            );
            coverage.phase = if passed { 9 } else { 99 };
            coverage.elapsed = 0.;
        }
        15 => {
            travel_to_machine(&mut player, &mut transform);
            commands
                .entity(owner)
                .insert(avian3d::prelude::Position(transform.translation));
            session.points = 500;
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            if coverage.elapsed > 0.2 {
                coverage.phase = 16;
                coverage.elapsed = 0.;
            }
        }
        16 => {
            input.use_held = true;
            if drinking.is_some_and(|d| d.elapsed > 1. && !d.activated) {
                if !perks.owned.is_empty() || perks.solo_charges != 0 || perks.solo_purchases != 0 {
                    coverage.phase = 99;
                    return;
                }
                impacts.0.push(crate::damage::Impact {
                    attacker: owner,
                    target: owner,
                    source: crate::damage::ImpactSource::Projectile(crate::origins::WeaponId(
                        "weapon.frag_grenade_zm".into(),
                    )),
                    category: crate::origins::DamageCategory::Explosive,
                    region: crate::weapons::HitRegion::Torso,
                    amount: 300,
                });
                coverage.phase = 17;
                coverage.elapsed = 0.;
            }
        }
        17 if **state == GameState::GameOver => {
            let cancellation = perk_evidence
                .events
                .iter()
                .rev()
                .find(|e| e["event"] == "drink_cancelled");
            let passed = cancellation.is_some_and(|e| e["activated"] == false)
                && perks.owned.is_empty()
                && perks.solo_charges == 0
                && perks.solo_purchases == 0
                && drinking.is_none()
                && session.points == 0
                && health_evidence.death_events == 2;
            let receipt = serde_json::json!({"passed":passed,"scope":"real paid bottle purchase, then controlled resolved lethal self-projectile impact before activation; qualifies drinking cancellation and lost purchase, not projectile delivery", "perk_events":perk_evidence.events,"points":session.points,"purchases":perks.solo_purchases,"charges":perks.solo_charges,"deaths":health_evidence.death_events});
            let _ = std::fs::write(
                verification.output.join("perk-drinking-abort.json"),
                receipt.to_string(),
            );
            if passed {
                commands.write_message(crate::input::RestartGame);
                coverage.phase = 18;
                coverage.elapsed = 0.;
            } else {
                coverage.phase = 99;
            }
        }
        18 if **state == GameState::Playing && coverage.elapsed > 0.5 => {
            let passed = health.state() == LifeState::Standing
                && health.current() == Some(100)
                && perks.owned.is_empty()
                && perks.solo_charges == 0
                && perks.solo_purchases == 0
                && drinking.is_none();
            let _ = std::fs::write(
                verification.output.join("perk-drinking-restart.json"),
                serde_json::json!({"passed":passed}).to_string(),
            );
            coverage.phase = if passed { 9 } else { 99 };
            coverage.elapsed = 0.;
        }
        9 if coverage.elapsed > 1. => {
            commands.write_message(AppExit::Success);
        }
        99 => {
            let _ = std::fs::write(verification.output.join("solo-revive.json"), serde_json::json!({"passed":false,"phase":coverage.phase,"events":evidence.events,"snapshots":coverage.snapshots}).to_string());
            commands.write_message(AppExit::error());
        }
        _ => {}
    }
}
