use super::*;
use crate::{
    life::{LifeState, Vitality},
    movement::SprintState,
    solo_revive::PerkInventory,
};
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    elapsed: f32,
    previous: Option<Vec3>,
    distance: f32,
    peak: f32,
    lock: bool,
    recovery: bool,
    insufficient: bool,
    charged: bool,
    stationary: bool,
    ads: bool,
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
        &SprintState,
        Option<&crate::perks::PerkDrinking>,
    )>,
    focus: Res<crate::interactions::InteractionFocus>,
    mut damage: ResMut<crate::damage::DamageQueue>,
    evidence: Res<crate::perks::PerkEvidence>,
    enemies: Query<Entity, With<Zombie>>,
) {
    let (Some(v), Some(game), Ok((owner, mut player, mut transform, health, perks, sprint, drink))) =
        (v, game, players.single_mut())
    else {
        return;
    };
    if !v.stamin_up {
        return;
    }
    input.clear();
    c.elapsed += time.delta_secs();
    if v.started.elapsed().as_secs() > 150
        || **state == GameState::LoadError
        || (**state == GameState::GameOver && c.phase < 16)
    {
        c.phase = 99;
    }
    let _=std::fs::write(v.output.join("stamin-up-progress.json"),serde_json::json!({"phase":c.phase,"seconds":c.elapsed,"capacity":sprint.capacity,"remaining":sprint.remaining,"sprinting":sprint.sprinting,"exhausted":sprint.exhausted,"last_run":sprint.last_run_seconds,"speed":player.velocity.xz().length(),"distance":c.distance,"prompt":focus.prompt,"rejection":format!("{:?}",focus.rejection),"rows":c.rows}).to_string());
    if matches!(c.phase,3..=5|11..=12) && **state == GameState::Playing {
        let id = if c.phase <= 5 {
            "placement.stamin_up"
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
    match c.phase {
        0 if **state == GameState::Playing && c.elapsed > 3. => {
            for enemy in &enemies {
                commands.entity(enemy).despawn();
            }
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            c.phase = 1;
            c.elapsed = 0.;
            c.previous = None;
        }
        1 | 8 | 14 => {
            input.sprint = true;
            input.movement = Vec2::Y;
            player.yaw = c.elapsed * 4.;
            player.pitch = 0.;
            if let Some(previous) = c.previous {
                let distance = transform.translation.xz().distance(previous.xz());
                if distance < 2. {
                    c.distance += distance;
                }
            }
            c.previous = Some(transform.translation);
            c.peak = c.peak.max(player.velocity.xz().length());
            if !c.picture && c.elapsed > 1. {
                let phase = c.phase;
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        v.output.join(format!("sprint-phase-{phase}.png")),
                    ));
                c.picture = true;
            }
            if sprint.exhausted && !sprint.sprinting && c.elapsed > 1. {
                let expected = if c.phase == 8 { 8. } else { 4. };
                let phase = c.phase;
                let passed = (sprint.last_run_seconds - expected).abs() < 0.05 && c.distance > 5.;
                let row = serde_json::json!({"phase":phase,"seconds":sprint.last_run_seconds,"capacity":sprint.capacity,"distance":c.distance,"peak_speed":c.peak,"passed":passed});
                c.rows.push(row);
                if !passed {
                    c.phase = 99;
                    return;
                }
                c.phase = match c.phase {
                    1 => 2,
                    8 => 9,
                    _ => 15,
                };
                c.elapsed = 0.;
                c.picture = false;
            }
        }
        2 => {
            input.sprint = true;
            input.movement = Vec2::Y;
            if c.elapsed > 1. {
                c.lock = !sprint.sprinting && sprint.exhausted && sprint.remaining > 0.;
                input.sprint = false;
                c.phase = 3;
                c.elapsed = 0.;
            }
        }
        3 => {
            if sprint.remaining >= sprint.capacity - 0.001 {
                c.recovery = !sprint.exhausted && c.elapsed > 2.5;
                session.points = 1999;
                input.use_held = true;
                if focus.rejection
                    == Some(crate::interactions::InteractionRejection::InsufficientPoints)
                {
                    c.insufficient = session.points == 1999;
                    c.phase = 4;
                    c.elapsed = 0.;
                }
            }
        }
        4 if c.elapsed > 0.2 => {
            session.points = 2000;
            c.phase = 5;
            c.elapsed = 0.;
        }
        5 => {
            input.use_held = true;
            if drink.is_some_and(|d| d.elapsed < 3.2) && sprint.capacity != 4. {
                c.phase = 99;
                return;
            }
            if drink.is_none()
                && perks
                    .owned
                    .contains(&crate::origins::PerkId("perk.stamin_up".into()))
            {
                c.charged = session.points == 0 && sprint.capacity == 8.;
                c.phase = 6;
                c.elapsed = 0.;
            }
        }
        6 => {
            input.sprint = true;
            if c.elapsed > 0.5 {
                c.stationary = !sprint.sprinting && sprint.remaining == sprint.capacity;
                c.phase = 7;
                c.elapsed = 0.;
            }
        }
        7 => {
            input.sprint = true;
            input.ads = true;
            input.movement = Vec2::Y;
            if c.elapsed > 0.6 {
                c.ads = !sprint.sprinting && sprint.remaining == sprint.capacity;
                transform.translation =
                    Vec3::from_array(game.spawn) + Vec3::Y * player.height * 0.5;
                player.velocity = Vec3::ZERO;
                commands
                    .entity(owner)
                    .insert(avian3d::prelude::Position(transform.translation));
                c.phase = 8;
                c.elapsed = 0.;
                c.previous = None;
                c.distance = 0.;
                c.peak = 0.;
            }
        }
        9 => {
            input.sprint = false;
            if sprint.remaining >= sprint.capacity - 0.001 {
                c.phase = 11;
                c.elapsed = 0.;
            }
        }
        11 => {
            session.points = 500;
            if c.elapsed > 0.2 {
                c.phase = 12;
                c.elapsed = 0.;
            }
        }
        12 => {
            input.use_held = true;
            if perks.solo_charges == 1 && drink.is_none() {
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
                c.phase = 13;
                c.elapsed = 0.;
            }
        }
        13 => {
            if health.state() == LifeState::Downed && sprint.capacity == 4. {
                c.lost = perks.owned.is_empty() && !sprint.sprinting;
            }
            if health.state() == LifeState::Standing && c.elapsed > 1. {
                transform.translation =
                    Vec3::from_array(game.spawn) + Vec3::Y * player.height * 0.5;
                player.velocity = Vec3::ZERO;
                commands
                    .entity(owner)
                    .insert(avian3d::prelude::Position(transform.translation));
                c.phase = 14;
                c.elapsed = 0.;
                c.distance = 0.;
                c.peak = 0.;
                c.previous = None;
            }
        }
        15 => {
            let speed = (c.rows[1]["peak_speed"].as_f64().unwrap()
                / c.rows[0]["peak_speed"].as_f64().unwrap()
                - 1.07)
                .abs()
                < 0.005;
            let passed = c.lock
                && c.recovery
                && c.insufficient
                && c.charged
                && c.stationary
                && c.ads
                && c.lost
                && speed;
            let _=std::fs::write(v.output.join("stamin-up.json"),serde_json::json!({"passed":passed,"scope":"input-driven sprint/recovery on actual map; normal held-F Stamin-Up/Quick Revive purchases with controlled travel/budgets/empty population and resolved downing damage","rows":c.rows,"lock":c.lock,"recovery":c.recovery,"insufficient":c.insufficient,"charged_once":c.charged,"stationary":c.stationary,"ads":c.ads,"lost":c.lost,"speed_ratio":speed,"perk_events":evidence.events}).to_string());
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
                c.phase = 16;
            } else {
                c.phase = 99;
            }
            c.elapsed = 0.;
        }
        16 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            c.phase = 17;
            c.elapsed = 0.;
        }
        17 if **state == GameState::Playing && c.elapsed > 0.5 => {
            let passed = sprint.capacity == 4.
                && sprint.remaining == 4.
                && !sprint.exhausted
                && !sprint.sprinting
                && perks.owned.is_empty();
            let _ = std::fs::write(
                v.output.join("stamin-up-restart.json"),
                serde_json::json!({"passed":passed}).to_string(),
            );
            c.phase = if passed { 18 } else { 99 };
            c.elapsed = 0.;
        }
        18 if c.elapsed > 0.5 => {
            commands.write_message(AppExit::Success);
        }
        99 => {
            let _ = std::fs::write(
                v.output.join("stamin-up-failure.json"),
                serde_json::json!({"passed":false,"phase":c.phase,"rows":c.rows}).to_string(),
            );
            commands.write_message(AppExit::error());
        }
        _ => {}
    }
}
