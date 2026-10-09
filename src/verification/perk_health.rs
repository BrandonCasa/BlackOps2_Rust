use super::*;
use crate::{
    life::{LifeState, Vitality},
    solo_revive::PerkInventory,
};
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    elapsed: f32,
    insufficient: bool,
    duplicate: bool,
    activation: bool,
    healed: bool,
    hits: u32,
    hit_age: f32,
    lost: bool,
    pictures: bool,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn drive(
    mut commands: Commands,
    verification: Option<Res<Verification>>,
    game: Option<Res<GameDefinition>>,
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut coverage: ResMut<Coverage>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Vitality,
        &PerkInventory,
        Option<&crate::perks::PerkDrinking>,
    )>,
    focus: Res<crate::interactions::InteractionFocus>,
    mut damage: ResMut<crate::damage::DamageQueue>,
    evidence: Res<crate::perks::PerkEvidence>,
    health_evidence: Res<crate::player_health::HealthEvidence>,
) {
    let (Some(v), Some(game), Ok((owner, mut player, mut transform, health, perks, drink))) =
        (verification, game, players.single_mut())
    else {
        return;
    };
    if !v.perk_health {
        return;
    }
    input.clear();
    coverage.elapsed += time.delta_secs();
    if v.started.elapsed().as_secs() > 150 || **state == GameState::LoadError {
        coverage.phase = 99;
    }
    let _=std::fs::write(v.output.join("perk-health-progress.json"),serde_json::json!({"phase":coverage.phase,"health":health.current(),"maximum":health.maximum(),"owned":perks.owned,"seconds":coverage.elapsed,"prompt":focus.prompt,"rejection":format!("{:?}",focus.rejection),"events":evidence.events}).to_string());
    if matches!(coverage.phase, 0..=5) && **state == GameState::Playing {
        let id = if coverage.phase <= 3 {
            "placement.juggernog"
        } else {
            "placement.quick_revive"
        };
        let p = game
            .content
            .placements
            .iter()
            .find(|p| p.id.0 == id)
            .expect("prepared perk");
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
    match coverage.phase {
        0 if **state == GameState::Playing && coverage.elapsed > 3. => {
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            session.points = 2499;
            coverage.phase = 1;
            coverage.elapsed = 0.;
        }
        1 => {
            input.use_held = true;
            if focus.rejection
                == Some(crate::interactions::InteractionRejection::InsufficientPoints)
            {
                coverage.insufficient = session.points == 2499 && health.maximum() == 100;
                coverage.phase = 2;
                coverage.elapsed = 0.;
            }
        }
        2 if coverage.elapsed > 0.15 => {
            session.points = 2500;
            coverage.phase = 3;
            coverage.elapsed = 0.;
        }
        3 => {
            input.use_held = !(4.4..4.6).contains(&coverage.elapsed);
            if let Some(d) = drink {
                if d.elapsed < 3.2
                    && (health.maximum() != 100
                        || perks
                            .owned
                            .contains(&crate::origins::PerkId("perk.juggernog".into())))
                {
                    coverage.phase = 99;
                    return;
                }
                if !coverage.pictures && d.elapsed > 1. {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(v.output.join("juggernog-drinking.png")));
                    coverage.pictures = true;
                }
            }
            if perks
                .owned
                .contains(&crate::origins::PerkId("perk.juggernog".into()))
                && drink.is_none()
            {
                coverage.activation |= health.maximum() == 250 && session.points == 0;
                if (4.4..4.6).contains(&coverage.elapsed) {
                    session.points = 2500;
                }
                if focus.rejection == Some(crate::interactions::InteractionRejection::Duplicate) {
                    coverage.duplicate = session.points == 2500;
                }
                coverage.healed = health.current() == Some(250);
                if coverage.elapsed > 7. && coverage.duplicate && coverage.healed {
                    session.points = 500;
                    coverage.phase = 4;
                    coverage.elapsed = 0.;
                }
            }
        }
        4 if coverage.elapsed > 0.15 => {
            coverage.phase = 5;
            coverage.elapsed = 0.;
        }
        5 => {
            input.use_held = true;
            if perks.solo_charges == 1 && drink.is_none() {
                coverage.phase = 6;
                coverage.elapsed = 0.;
                coverage.hit_age = 1.;
            }
        }
        6 => {
            coverage.hit_age += time.delta_secs();
            if coverage.hits < 5 && coverage.hit_age > 0.65 {
                let expected = 250 - coverage.hits * 50;
                if health.current() != Some(expected) || health.maximum() != 250 {
                    coverage.phase = 99;
                    return;
                }
                damage.0.push(crate::damage::Impact {
                    attacker: owner,
                    target: owner,
                    source: crate::damage::ImpactSource::Projectile(crate::origins::WeaponId(
                        "weapon.frag_grenade_zm".into(),
                    )),
                    category: crate::origins::DamageCategory::Explosive,
                    region: crate::weapons::HitRegion::Torso,
                    amount: 50,
                });
                coverage.hits += 1;
                coverage.hit_age = 0.;
            }
            if health.state() == LifeState::Downed {
                coverage.lost = coverage.hits == 5
                    && perks.owned.is_empty()
                    && health.maximum() == 100
                    && health.current() == Some(1);
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("juggernog-loss-downed.png")));
                coverage.phase = 7;
                coverage.elapsed = 0.;
            }
        }
        7 if health.state() == LifeState::Standing => {
            let passed = coverage.insufficient
                && coverage.duplicate
                && coverage.activation
                && coverage.healed
                && coverage.lost
                && health.maximum() == 100
                && health.current() == Some(100)
                && perks.owned.is_empty();
            let _=std::fs::write(v.output.join("perk-health.json"),serde_json::json!({"passed":passed,"scope":"controlled travel/budgets, real held-F Juggernog and Quick Revive purchases; five resolved 50-point impacts qualify effective-health reconstruction, not native armor arithmetic","events":evidence.events,"hits":health_evidence.hits,"insufficient":coverage.insufficient,"duplicate":coverage.duplicate,"activation":coverage.activation,"healed":coverage.healed,"perk_loss":coverage.lost}).to_string());
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
                coverage.phase = 10;
                coverage.elapsed = 0.;
            } else {
                coverage.phase = 99;
            }
        }
        10 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            coverage.phase = 8;
            coverage.elapsed = 0.;
        }
        8 if **state == GameState::Playing && coverage.elapsed > 0.5 => {
            let passed = health.maximum() == 100
                && health.current() == Some(100)
                && perks.owned.is_empty()
                && perks.solo_charges == 0
                && perks.solo_purchases == 0
                && drink.is_none();
            let _ = std::fs::write(
                v.output.join("perk-health-restart.json"),
                serde_json::json!({"passed":passed}).to_string(),
            );
            coverage.phase = if passed { 9 } else { 99 };
            coverage.elapsed = 0.;
        }
        9 if coverage.elapsed > 0.5 => {
            commands.write_message(AppExit::Success);
        }
        99 => {
            let _=std::fs::write(v.output.join("perk-health-failure.json"),serde_json::json!({"passed":false,"phase":coverage.phase,"health":health.current(),"maximum":health.maximum(),"events":evidence.events}).to_string());
            commands.write_message(AppExit::error());
        }
        _ => {}
    }
}
