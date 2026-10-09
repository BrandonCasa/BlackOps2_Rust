use super::*;
use crate::{
    damage::{DamageQueue, Impact, ImpactSource},
    life::Vitality,
    origins::content::{DamageCategory, WeaponId},
    player_health::{HealthEvidence, Recovery},
    weapons::HitRegion,
};

#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    elapsed: f32,
    hits: usize,
    heals: usize,
    rejections: u32,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn strike(queue: &mut DamageQueue, owner: Entity, amount: u32) {
    let impact = Impact {
        attacker: owner,
        target: owner,
        source: ImpactSource::Projectile(WeaponId("weapon.frag_grenade_zm".into())),
        category: DamageCategory::Explosive,
        region: HitRegion::Torso,
        amount,
    };
    queue.0.extend([impact.clone(), impact]);
}
fn drive(
    mut commands: Commands,
    verification: Option<ResMut<Verification>>,
    mut coverage: ResMut<Coverage>,
    time: Res<Time>,
    definition: Option<Res<GameDefinition>>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut queue: ResMut<DamageQueue>,
    evidence: Res<HealthEvidence>,
    players: Query<(Entity, &Vitality, &Recovery), With<Player>>,
    enemies: Query<Entity, With<Zombie>>,
    mut next: ResMut<NextState<GameState>>,
) {
    let (Some(mut verification), Some(definition), Ok((owner, health, recovery))) =
        (verification, definition, players.single())
    else {
        return;
    };
    if verification.stage != 4 || !verification.health {
        return;
    }
    input.clear();
    session.pending = 0;
    session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
    for enemy in &enemies {
        commands.entity(enemy).despawn();
    }
    coverage.elapsed += time.delta_secs();
    let profile = &definition.player_health;
    if coverage.elapsed > 20. {
        let receipt = serde_json::json!({"passed":false,"phase":coverage.phase,
            "health":health.current(),"hits":evidence.hits,"heals":evidence.heals});
        let _ = std::fs::write(
            verification.output.join("health-recovery.json"),
            receipt.to_string(),
        );
        commands.write_message(AppExit::error());
        return;
    }
    match coverage.phase {
        0 if health.current() == Some(100) => {
            coverage.hits = evidence.hits.len();
            coverage.heals = evidence.heals.len();
            coverage.rejections = evidence.immunity_rejections;
            strike(&mut queue, owner, 50);
            coverage.phase = 1;
        }
        1 | 2 => {
            let index = coverage.hits + coverage.phase as usize - 1;
            let Some(hit) = evidence.hits.get(index) else {
                return;
            };
            let expected = if coverage.phase == 1 {
                profile.immunity_regular
            } else {
                profile.immunity_critical_enter
            };
            if recovery.age > expected + 0.05 {
                if health.current() != Some(if coverage.phase == 1 { 50 } else { 15 })
                    || hit["immunity_seconds"]
                        .as_f64()
                        .is_none_or(|v| (v as f32 - expected).abs() > 0.001)
                {
                    commands.write_message(AppExit::error());
                    return;
                }
                strike(&mut queue, owner, if coverage.phase == 1 { 35 } else { 11 });
                coverage.phase += 1;
            }
        }
        3 => {
            let Some(hit) = evidence.hits.get(coverage.hits + 2) else {
                return;
            };
            if health.current() != Some(health.maximum()) {
                return;
            }
            let heals = &evidence.heals[coverage.heals..];
            let spacing = heals.windows(2).all(|pair| {
                let delta = pair[1]["since_hit"].as_f64().unwrap_or(0.)
                    - pair[0]["since_hit"].as_f64().unwrap_or(0.);
                (delta as f32 - profile.critical_step_seconds).abs() < 0.017
            });
            let rates = heals.iter().all(|h| {
                let before = h["before"].as_u64().unwrap_or(0);
                h["after"].as_u64() == Some((before + 10).min(100)) && h["critical"] == true
            });
            let passed = heals.len() == 10
                && spacing
                && rates
                && heals[0]["since_hit"]
                    .as_f64()
                    .is_some_and(|v| v >= profile.critical_delay as f64)
                && hit["immunity_seconds"]
                    .as_f64()
                    .is_some_and(|v| (v as f32 - profile.immunity_critical_repeat).abs() < 0.001)
                && evidence.immunity_rejections - coverage.rejections == 3
                && evidence.hits.len() - coverage.hits == 3;
            let receipt = serde_json::json!({"passed":passed,
                "scope":"normal Round 1 wounds and combat precede controlled resolved 50/35/11 self-projectile impact pairs; this qualifies recovery/immunity branches, not projectile delivery",
                "hits":&evidence.hits[coverage.hits..],"heals":heals,
                "immunity_rejections":evidence.immunity_rejections-coverage.rejections,
                "regular_rate":profile.regular_step_fraction,"critical_rate":profile.critical_step_fraction,
                "critical_step_seconds":profile.critical_step_seconds});
            let _ = std::fs::write(
                verification.output.join("health-recovery.json"),
                receipt.to_string(),
            );
            if !passed {
                commands.write_message(AppExit::error());
                return;
            }
            verification.health_recovery_exercised = true;
            verification.stage = 1;
            verification.playing = 0.;
            session.pending = 1;
            session.intermission = None;
            session.spawn_timer = Timer::from_seconds(0.1, TimerMode::Once);
            coverage.phase = 4;
            next.set(GameState::Paused);
        }
        _ => {}
    }
}
