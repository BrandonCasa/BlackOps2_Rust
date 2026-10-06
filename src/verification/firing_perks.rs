//! Actual-window firing comparisons; controlled awards isolate engine modifiers.
use super::*;
use crate::{
    life::{LifeState, Vitality},
    solo_revive::PerkInventory,
    weapons::{WeaponAction, WeaponInventory},
};
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    case: usize,
    elapsed: f32,
    begin: usize,
    magazine: u32,
    before: Vec2,
    mouse_ok: bool,
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
    state: Res<State<GameState>>,
    mut c: ResMut<Coverage>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<(
        Entity,
        &mut Player,
        &Vitality,
        &mut PerkInventory,
        &mut WeaponInventory,
        Option<&crate::perks::PerkDrinking>,
    )>,
    enemies: Query<Entity, With<Zombie>>,
    firing: Res<crate::perks::FiringEvidence>,
    mut damage: ResMut<crate::damage::DamageQueue>,
) {
    let (Some(v), Some(game), Ok((owner, mut player, health, mut perks, mut inv, drink))) =
        (v, game, players.single_mut())
    else {
        return;
    };
    if !v.firing_perks {
        return;
    }
    input.clear();
    c.elapsed += time.delta_secs();
    if v.started.elapsed().as_secs() > 150 || **state == GameState::LoadError {
        c.phase = 99;
    }
    let _=std::fs::write(v.output.join("firing-perks-progress.json"),serde_json::json!({"phase":c.phase,"case":c.case,"elapsed":c.elapsed,"rows":c.rows,"shots":firing.shots}).to_string());
    match c.phase {
        0 if **state == GameState::Playing && c.elapsed > 3. => {
            for e in &enemies {
                commands.entity(e).despawn();
            }
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            c.phase = 1;
            c.elapsed = 0.;
        }
        1 => {
            let id = match c.case {
                4 => "weapon.870mcs_zm",
                5 => "weapon.ray_gun_zm",
                _ => "weapon.mp40_zm",
            };
            let w = game.content.weapons.iter().find(|w| w.id.0 == id).unwrap();
            let slot = inv.active_slot();
            let _ = inv.replace(slot, w);
            inv.cancel_action();
            player.velocity = Vec3::ZERO;
            c.magazine = inv.active().unwrap().magazine;
            c.begin = firing.shots.len();
            c.phase = 2;
            c.elapsed = 0.;
        }
        2 => {
            let ads = c.case == 3;
            input.ads = ads;
            if (!ads && inv.ads < 0.001) || (ads && inv.ads > 0.999) {
                c.phase = 3;
                c.elapsed = 0.;
            }
        }
        3 => {
            let count = if matches!(c.case, 4 | 5) { 2 } else { 16 };
            input.ads = c.case == 3;
            input.fire_held = true;
            input.fire = true;
            if firing.shots.len() >= c.begin + count {
                input.fire = false;
                input.fire_held = false;
                let shots = &firing.shots[c.begin..];
                let count = shots.len();
                let interval = (shots[count - 1]["seconds"].as_f64().unwrap()
                    - shots[0]["seconds"].as_f64().unwrap())
                    / (count - 1) as f64;
                let debit = c.magazine - inv.active().unwrap().magazine;
                let copies = if matches!(c.case, 1..=4) { 2 } else { 1 };
                let hip = if matches!(c.case, 2..=5) { 0.65 } else { 1. };
                let passed = debit == count as u32
                    && shots.iter().all(|s| {
                        s["copies"] == copies
                            && (s["hip_spread_scale"].as_f64().unwrap() - hip).abs() < 0.001
                    });
                let row = serde_json::json!({"case":c.case,"passed":passed,"interval":interval,"debit":debit,"shots":shots});
                c.rows.push(row);
                if !passed {
                    c.phase = 99;
                } else {
                    c.phase = 4;
                }
                c.elapsed = 0.;
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        v.output.join(format!("firing-case-{}.png", c.case)),
                    ));
            }
        }
        4 if c.elapsed > 0.6 => match c.case {
            0 | 1 => {
                let id = if c.case == 0 {
                    "perk.double_tap"
                } else {
                    "perk.deadshot"
                };
                let perk = game.content.perks.iter().find(|p| p.id.0 == id).unwrap();
                inv.cancel_action();
                let action = crate::perks::PerkDrinking::new(perk, &inv, &game).unwrap();
                commands.entity(owner).insert(action);
                c.phase = 5;
                c.elapsed = 0.;
            }
            2 => {
                c.phase = 6;
                c.elapsed = 0.;
            }
            3 | 4 => {
                c.case += 1;
                c.phase = 1;
            }
            5 => {
                let _ = perks.buy_solo(&game.solo_revive);
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
                c.phase = 8;
                c.elapsed = 0.;
            }
            6 => {
                c.phase = 9;
            }
            _ => {
                c.phase = 99;
            }
        },
        5 => {
            if drink.is_some() && c.elapsed > 0.6 && c.elapsed < 0.8 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        v.output.join(format!("firing-perk-bottle-{}.png", c.case)),
                    ));
            }
            if drink.is_none() && c.elapsed > 4.5 {
                c.case += 1;
                c.phase = 1;
                c.elapsed = 0.;
            }
        }
        6 => {
            c.before = Vec2::new(player.yaw, player.pitch);
            input.look = Vec2::new(8., -4.);
            c.phase = 7;
        }
        7 => {
            let expected = c.before + Vec2::new(-8., 4.) * game.tuning.mouse_sensitivity;
            c.mouse_ok = (Vec2::new(player.yaw, player.pitch) - expected).length() < 0.00001;
            c.case += 1;
            c.phase = 1;
            c.elapsed = 0.;
        }
        8 if health.state() == LifeState::Standing && c.elapsed > 11. => {
            if perks.owned.is_empty()
                && inv.firing.interval_scale == 1.
                && inv.firing.bullet_copies == 1
                && inv.firing.hip_spread_scale == 1.
            {
                c.case = 6;
                c.phase = 1;
                c.elapsed = 0.;
            } else {
                c.phase = 99;
            }
        }
        9 => {
            let base = c.rows[0]["interval"].as_f64().unwrap();
            let fast = c.rows[1]["interval"].as_f64().unwrap();
            let lost = c.rows[6]["interval"].as_f64().unwrap();
            let base_spread = c.rows[0]["shots"][0]["spread"].as_f64().unwrap();
            let reduced = c.rows[2]["shots"][0]["spread"].as_f64().unwrap();
            let passed = c.mouse_ok
                && (fast / base - 0.75).abs() < 0.04
                && (base - lost).abs() < 0.02
                && (reduced / base_spread - 0.65).abs() < 0.001
                && c.rows[3]["shots"][0]["spread"].as_f64().unwrap().abs() < 0.00001;
            let _=std::fs::write(v.output.join("firing-perks.json"),serde_json::json!({"passed":passed,"mouse_raw":c.mouse_ok,"rows":c.rows,"scope":"actual input-driven firing and native bottles; controlled weapon grants/perk awards, empty admission and resolved downing; acquisition/power/audio/icons and native numerical equivalence unqualified"}).to_string());
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
                c.phase = 10;
            } else {
                c.phase = 99;
            }
            c.elapsed = 0.;
        }
        10 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            c.phase = 11;
            c.elapsed = 0.;
        }
        11 if **state == GameState::Playing && c.elapsed > 0.5 => {
            let passed = perks.owned.is_empty()
                && inv.firing.interval_scale == 1.
                && inv.firing.bullet_copies == 1
                && inv.firing.hip_spread_scale == 1.
                && inv.action == WeaponAction::Ready;
            let _ = std::fs::write(
                v.output.join("firing-perks-restart.json"),
                serde_json::json!({"passed":passed}).to_string(),
            );
            c.phase = if passed { 12 } else { 99 };
            c.elapsed = 0.;
        }
        12 if c.elapsed > 0.5 => {
            commands.write_message(AppExit::Success);
        }
        99 => {
            let _ = std::fs::write(
                v.output.join("firing-perks-failure.json"),
                serde_json::json!({"passed":false,"case":c.case,"rows":c.rows}).to_string(),
            );
            commands.write_message(AppExit::error());
        }
        _ => {}
    }
}
