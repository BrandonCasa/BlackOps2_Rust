use super::*;
use crate::{
    life::{LifeState, Vitality},
    solo_revive::PerkInventory,
    weapons::{ShellPhase, WeaponAction, WeaponInventory},
};
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    case: u8,
    elapsed: f32,
    shot_age: f32,
    started: Option<f64>,
    before: u32,
    reserve: u32,
    phases: u8,
    interrupted: bool,
    insufficient: bool,
    charged: bool,
    lost: bool,
    picture: bool,
    rows: Vec<serde_json::Value>,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn drive(
    mut commands: Commands,
    v: Option<Res<Verification>>,
    game: Option<Res<GameDefinition>>,
    time: Res<Time>,
    fixed: Res<Time<Fixed>>,
    state: Res<State<GameState>>,
    mut c: ResMut<Coverage>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Vitality,
        &PerkInventory,
        &mut WeaponInventory,
        Option<&crate::perks::PerkDrinking>,
    )>,
    focus: Res<crate::interactions::InteractionFocus>,
    mut damage: ResMut<crate::damage::DamageQueue>,
    evidence: Res<crate::perks::PerkEvidence>,
    enemies: Query<Entity, With<Zombie>>,
) {
    let (
        Some(v),
        Some(game),
        Ok((owner, mut player, mut transform, health, perks, mut inv, drink)),
    ) = (v, game, players.single_mut())
    else {
        return;
    };
    if !v.speed_cola {
        return;
    }
    input.clear();
    c.elapsed += time.delta_secs();
    c.shot_age += time.delta_secs();
    if v.started.elapsed().as_secs() > 180 || **state == GameState::LoadError {
        c.phase = 99;
    }
    if **state == GameState::GameOver && c.phase < 20 {
        c.phase = 99;
    }
    let _=std::fs::write(v.output.join("speed-cola-progress.json"),serde_json::json!({"phase":c.phase,"case":c.case,"elapsed":c.elapsed,"weapon":inv.active().map(|w|&w.definition.0),"magazine":inv.active().map(|w|w.magazine),"action":format!("{:?}",inv.action),"scale":inv.reload_duration_scale(),"prompt":focus.prompt,"rejection":format!("{:?}",focus.rejection),"rows":c.rows,"perk_events":evidence.events}).to_string());
    if matches!(c.phase,10..=12|15..=16) && **state == GameState::Playing {
        let id = if c.phase <= 12 {
            "placement.speed_cola"
        } else {
            "placement.quick_revive"
        };
        let p = game
            .content
            .placements
            .iter()
            .find(|p| p.id.0 == id)
            .expect("native perk");
        let machine = Vec3::from_array(p.position);
        commands.queue(move |world: &mut World| super::power_fixture(world, id));
        let rotation = Quat::from_rotation_y(p.yaw);
        transform.translation = machine + rotation * Vec3::new(0., player.height * 0.5 + 0.02, 2.2);
        let delta = machine + Vec3::Y + rotation * Vec3::new(0., 0., 0.8)
            - (transform.translation + Vec3::Y * (game.tuning.eye_height - player.height * 0.5));
        player.yaw = (-delta.x).atan2(-delta.z);
        player.pitch = (delta.y / delta.length()).asin();
        player.velocity = Vec3::ZERO;
        commands
            .entity(owner)
            .insert(avian3d::prelude::Position(transform.translation));
    }
    let shotgun = matches!(c.case, 2 | 5 | 6);
    let desired = if shotgun {
        "weapon.870mcs_zm"
    } else {
        "weapon.c96_zm"
    };
    match c.phase {
        0 if **state == GameState::Playing && c.elapsed > 3. => {
            for enemy in &enemies {
                commands.entity(enemy).despawn();
            }
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            let w = game
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.870mcs_zm")
                .expect("prepared shotgun");
            if inv.grant(w).is_err() {
                c.phase = 99;
                return;
            }
            c.phase = 1;
            c.elapsed = 0.;
        }
        1 => {
            let slot = inv
                .owned()
                .find(|(_, w)| w.definition.0 == desired)
                .map(|(s, _)| s)
                .expect("owned family");
            input.weapon_slot = Some(slot);
            if inv.active().is_some_and(|w| w.definition.0 == desired)
                && inv.action == WeaponAction::Ready
                && c.elapsed > 0.2
            {
                c.phase = 2;
                c.shot_age = 1.;
                c.elapsed = 0.;
                c.started = None;
                c.phases = 0;
                c.interrupted = false;
                c.picture = false;
            }
        }
        2 => {
            let w = inv.definition(&game.content.weapons).expect("active");
            let target = if matches!(c.case, 1 | 4) {
                0
            } else {
                w.magazine - if shotgun { 2 } else { 1 }
            };
            if inv.active().is_some_and(|a| a.magazine <= target) && c.shot_age > 0.25 {
                c.before = inv.active().expect("active").magazine;
                c.reserve = inv.reserve(w);
                c.phase = 3;
                c.elapsed = 0.;
            } else if c.shot_age > 0.4 {
                input.fire = true;
                c.shot_age = 0.;
            }
        }
        3 => {
            input.reload = true;
            let w = inv.definition(&game.content.weapons).expect("active");
            if let Some((duration, remaining)) = inv.reload_timing(w) {
                c.started = Some(
                    fixed.elapsed_secs_f64()
                        - f64::from((duration - remaining) * inv.reload_duration_scale()),
                );
                c.phase = 4;
                c.elapsed = 0.;
            }
        }
        4 => {
            let w = inv.definition(&game.content.weapons).expect("active");
            if let WeaponAction::ShellReload { phase, .. } = inv.action {
                c.phases |= match phase {
                    ShellPhase::Start => 1,
                    ShellPhase::Loop => 2,
                    ShellPhase::End => 4,
                };
                if c.case == 6 && phase == ShellPhase::Loop && !c.interrupted {
                    input.fire = true;
                    input.fire_held = true;
                    c.interrupted = true;
                }
            }
            if !shotgun
                && inv.action != WeaponAction::Ready
                && inv.active().expect("active").magazine != c.before
            {
                c.phase = 99;
                return;
            }
            if !c.picture && c.elapsed > 0.25 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        v.output.join(format!("reload-case-{}.png", c.case)),
                    ));
                c.picture = true;
            }
            if inv.action == WeaponAction::Ready
                && (c.case != 6 || inv.active().is_some_and(|w| w.magazine == c.before))
            {
                let seconds = fixed.elapsed_secs_f64() - c.started.expect("reload start");
                let mag = inv.active().expect("active").magazine;
                let remaining = inv.reserve(w);
                let passed = if c.case == 6 {
                    c.interrupted
                        && c.phases & 4 != 0
                        && mag == c.before
                        && remaining + 1 == c.reserve
                } else {
                    mag == w.magazine
                        && remaining + (mag - c.before) == c.reserve
                        && (!shotgun || c.phases == 7)
                };
                let case = c.case;
                let row = serde_json::json!({"case":case,"weapon":w.id.0,"seconds":seconds,"scale":inv.reload_duration_scale(),"magazine_before":c.before,"magazine_after":mag,"reserve_before":c.reserve,"reserve_after":remaining,"shell_phases":c.phases,"interrupted":c.interrupted,"passed":passed});
                c.rows.push(row);
                if !passed {
                    c.phase = 99;
                    return;
                }
                c.case += 1;
                c.elapsed = 0.;
                c.phase = match c.case {
                    3 => 10,
                    7 => 15,
                    8 => 20,
                    _ => 1,
                };
            }
        }
        10 => {
            session.points = 2999;
            if c.elapsed > 0.2 {
                input.use_held = true;
                if focus.rejection
                    == Some(crate::interactions::InteractionRejection::InsufficientPoints)
                {
                    c.insufficient = session.points == 2999;
                    c.phase = 11;
                    c.elapsed = 0.;
                }
            }
        }
        11 if c.elapsed > 0.2 => {
            session.points = 3000;
            c.phase = 12;
            c.elapsed = 0.;
        }
        12 => {
            input.use_held = true;
            if drink.is_some_and(|d| d.elapsed < 3.2) && inv.reload_duration_scale() != 1. {
                c.phase = 99;
                return;
            }
            if drink.is_none()
                && perks
                    .owned
                    .contains(&crate::origins::PerkId("perk.speed_cola".into()))
            {
                c.charged = session.points == 0 && inv.reload_duration_scale() == 0.5;
                c.phase = 1;
                c.elapsed = 0.;
            }
        }
        15 => {
            session.points = 500;
            if c.elapsed > 0.2 {
                c.phase = 16;
                c.elapsed = 0.;
            }
        }
        16 => {
            input.use_held = true;
            if drink.is_none() && perks.solo_charges == 1 {
                damage.0.push(crate::damage::Impact {
                    attacker: owner,
                    target: owner,
                    source: crate::damage::ImpactSource::Projectile(crate::origins::WeaponId(
                        "weapon.frag_grenade_zm".into(),
                    )),
                    category: crate::origins::DamageCategory::Explosive,
                    region: crate::weapons::HitRegion::Torso,
                    amount: 300,
                });
                c.phase = 17;
                c.elapsed = 0.;
            }
        }
        17 => {
            if health.state() == LifeState::Downed {
                c.lost = perks.owned.is_empty() && inv.reload_duration_scale() == 1.;
            }
            if health.state() == LifeState::Standing && c.elapsed > 1. {
                c.phase = 1;
                c.elapsed = 0.;
            }
        }
        20 => {
            let comparison = (0..3).all(|i| {
                let base = c.rows[i]["seconds"].as_f64().unwrap();
                let fast = c.rows[i + 3]["seconds"].as_f64().unwrap();
                (fast - base * 0.5).abs() < 0.12
            });
            let reset = (c.rows[7]["seconds"].as_f64().unwrap()
                - c.rows[0]["seconds"].as_f64().unwrap())
            .abs()
                < 0.12;
            let passed = comparison && reset && c.insufficient && c.charged && c.lost;
            let _=std::fs::write(v.output.join("speed-cola.json"),serde_json::json!({"passed":passed,"scope":"real held-F paid Speed Cola/Quick Revive; controlled travel/budgets/second-weapon grant and resolved downing damage; input-driven normal/empty/shell/interrupted reloads before/after acquisition and loss","rows":c.rows,"perk_events":evidence.events,"insufficient":c.insufficient,"charged_once":c.charged,"lost":c.lost,"timing_comparison":comparison,"timing_reset":reset}).to_string());
            if passed {
                damage.0.push(crate::damage::Impact {
                    attacker: owner,
                    target: owner,
                    source: crate::damage::ImpactSource::Projectile(crate::origins::WeaponId(
                        "weapon.frag_grenade_zm".into(),
                    )),
                    category: crate::origins::DamageCategory::Explosive,
                    region: crate::weapons::HitRegion::Torso,
                    amount: 300,
                });
                c.phase = 21;
            } else {
                c.phase = 99;
            }
            c.elapsed = 0.;
        }
        21 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            c.phase = 22;
            c.elapsed = 0.;
        }
        22 if **state == GameState::Playing && c.elapsed > 0.5 => {
            let passed =
                perks.owned.is_empty() && inv.reload_duration_scale() == 1. && drink.is_none();
            let _ = std::fs::write(
                v.output.join("speed-cola-restart.json"),
                serde_json::json!({"passed":passed}).to_string(),
            );
            c.phase = if passed { 23 } else { 99 };
            c.elapsed = 0.;
        }
        23 if c.elapsed > 0.5 => {
            commands.write_message(AppExit::Success);
        }
        99 => {
            let _=std::fs::write(v.output.join("speed-cola-failure.json"),serde_json::json!({"passed":false,"rows":c.rows,"phase":c.phase,"case":c.case,"perk_events":evidence.events}).to_string());
            commands.write_message(AppExit::error());
        }
        _ => {}
    }
}
