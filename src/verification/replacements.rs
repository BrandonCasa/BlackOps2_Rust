use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    gameplay::{GameStep, Session, SessionEntity, Zombie},
    life::Vitality,
    player::Player,
    rounds::{PopulationKind, ReplacementHealth, RoundEvidence},
};
use bevy::prelude::*;
use std::time::Instant;

#[derive(Resource)]
struct Run {
    started: Instant,
    phase: u8,
    age: f32,
    origin: Option<Vec3>,
    targets: Vec<(Entity, Vec3)>,
    checks: u64,
    retired: u64,
    cap_waits: u64,
    pending: u32,
    corpses: Vec<Entity>,
    corpse_score: (u32, u32),
    rows: Vec<serde_json::Value>,
}
pub(super) fn install(app: &mut App) {
    app.insert_resource(Run {
        started: Instant::now(),
        phase: 0,
        age: 0.,
        origin: None,
        targets: vec![],
        checks: 0,
        retired: 0,
        cap_waits: 0,
        pending: 0,
        corpses: vec![],
        corpse_score: (0, 0),
        rows: vec![],
    })
    .add_systems(Update, drive)
    .add_systems(
        FixedUpdate,
        hold.in_set(GameStep::EnemyIntent)
            .after(crate::enemies::lifecycle),
    );
}
fn hold(
    time: Res<Time<Fixed>>,
    mut run: ResMut<Run>,
    state: Res<State<GameState>>,
    mut input: ResMut<crate::input::GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<(&mut Player, &mut Transform), Without<Zombie>>,
    mut enemies: Query<(&mut Zombie, &mut Transform), Without<Player>>,
) {
    input.clear();
    if **state != GameState::Playing {
        return;
    }
    run.age += time.delta_secs();
    if run.phase < 3 || run.phase == 5 {
        session.spawn_timer = Timer::from_seconds(1000., TimerMode::Once);
    }
    if let Some(origin) = run.origin {
        for (mut player, mut pose) in &mut players {
            pose.translation = origin;
            player.yaw = 0.;
            player.pitch = 0.;
            player.velocity = Vec3::ZERO;
        }
    }
    for (index, (entity, position)) in run.targets.iter().enumerate() {
        if let Ok((mut zombie, mut pose)) = enemies.get_mut(*entity) {
            pose.translation = *position;
            if index == 4 && run.phase < 5 {
                zombie.stunned = 1.;
            }
        }
    }
}
fn row(run: &mut Run, name: &str, passed: bool) {
    run.rows
        .push(serde_json::json!({"case":name,"passed":passed}));
}
fn actor(world: &mut World, position: Vec3, kind: PopulationKind) -> Entity {
    let game = world.resource::<GameDefinition>();
    let id = game
        .content
        .enemies
        .iter()
        .find(|e| e.family == "german")
        .unwrap()
        .id
        .clone();
    let maximum = game.rounds.health(world.resource::<Session>().round);
    world
        .spawn((
            SessionEntity,
            Zombie::new(position, id, crate::rounds::Gait::Walk),
            kind,
            Vitality::ordinary_actor(maximum, maximum).unwrap(),
            crate::player::capsule(0.3, 1.7),
            Transform::from_translation(position),
            Visibility::Inherited,
        ))
        .id()
}
fn hit(world: &mut World, player: Entity, target: Entity, known: bool) {
    world
        .resource_mut::<crate::damage::DamageQueue>()
        .0
        .push(crate::damage::Impact {
            point: None,
            attacker: player,
            target,
            source: if known {
                crate::damage::ImpactSource::Weapon(crate::origins::WeaponId(
                    "weapon.c96_zm".into(),
                ))
            } else {
                crate::damage::ImpactSource::Environment("hazard.environment".into())
            },
            category: if known {
                crate::origins::DamageCategory::Bullet
            } else {
                crate::origins::DamageCategory::Environmental
            },
            region: crate::weapons::HitRegion::None,
            amount: 25,
        });
}
fn drive(world: &mut World) {
    world.resource_scope(|world, mut run: Mut<Run>| {
        if run.started.elapsed().as_secs() > 180 {
            row(&mut run, "runtime_timeout", false);
            finish(world, &run);
            return;
        }
        if *world.resource::<State<GameState>>().get() != GameState::Playing {
            return;
        }
        let Some(player) = world
            .query_filtered::<Entity, With<Player>>()
            .iter(world)
            .next()
        else {
            return;
        };
        match run.phase {
            0 if run.age > 1. => {
                let previous: Vec<_> = world
                    .query_filtered::<Entity, With<Zombie>>()
                    .iter(world)
                    .collect();
                for entity in previous {
                    world.despawn(entity);
                }
                let origin = world.get::<Transform>(player).unwrap().translation;
                run.origin = Some(origin);
                world.resource_mut::<Session>().pending = 6;
                let offsets = [
                    Vec3::new(-3., 0., 45.),
                    Vec3::new(0., 0., 45.),
                    Vec3::new(3., 0., 45.),
                    Vec3::new(6., 0., 45.),
                    Vec3::new(9., 0., 45.),
                    Vec3::new(0., 0., 10.),
                    Vec3::new(0., 0., -45.),
                ];
                for (index, offset) in offsets.into_iter().enumerate() {
                    let position = origin + offset;
                    let entity = actor(
                        world,
                        position,
                        if index == 3 {
                            PopulationKind::Special
                        } else {
                            PopulationKind::Ordinary
                        },
                    );
                    run.targets.push((entity, position));
                }
                hit(world, player, run.targets[0].0, true);
                hit(world, player, run.targets[1].0, false);
                run.checks = world.resource::<RoundEvidence>().distance_checks;
                run.retired = world.resource::<RoundEvidence>().retired;
                run.phase = 1;
                run.age = 0.;
            }
            1 if run.age > 0.2 => {
                let known = world
                    .get::<Zombie>(run.targets[0].0)
                    .is_some_and(|z| z.known_damage);
                let unknown = world
                    .get::<Zombie>(run.targets[1].0)
                    .is_some_and(|z| !z.known_damage);
                row(
                    &mut run,
                    "accepted_damage_tracks_last_known_mod",
                    known && unknown,
                );
                run.phase = 2;
            }
            2 if world.resource::<RoundEvidence>().distance_checks > run.checks => {
                let gone = |index: usize| world.get_entity(run.targets[index].0).is_err();
                let checks = [
                    gone(0),
                    gone(1),
                    gone(2),
                    !gone(3),
                    !gone(4),
                    !gone(5),
                    !gone(6),
                ];
                for (name, passed) in [
                    "known_damaged_retired",
                    "unknown_damaged_retired",
                    "full_health_retired",
                    "special_population_protected",
                    "electrified_protected",
                    "nearby_protected",
                    "visible_protected",
                ]
                .into_iter()
                .zip(checks)
                {
                    row(&mut run, name, passed);
                }
                let pending = world.resource::<Session>().pending;
                row(&mut run, "native_late_round_budget_rules", pending == 8);
                let maximum = world
                    .resource::<GameDefinition>()
                    .rounds
                    .health(world.resource::<Session>().round);
                let queued: Vec<_> = world
                    .query::<&ReplacementHealth>()
                    .iter(world)
                    .map(|h| h.current)
                    .collect();
                row(
                    &mut run,
                    "unknown_damage_health_preserved",
                    queued == vec![maximum - 25],
                );
                let retired = world.resource::<RoundEvidence>().retired;
                let once = retired == run.retired + 3;
                row(&mut run, "one_retirement_per_actor", once);
                for index in 0..20 {
                    let position = run.origin.unwrap()
                        + Vec3::new((index % 7) as f32 - 3., 0., 15. + (index / 7) as f32 * 2.);
                    let entity = actor(world, position, PopulationKind::Ordinary);
                    run.targets.push((entity, position));
                }
                run.cap_waits = world.resource::<RoundEvidence>().cap_waits;
                run.phase = 3;
                run.age = 0.;
                world.resource_mut::<Session>().spawn_timer =
                    Timer::from_seconds(0.01, TimerMode::Once);
            }
            3 if run.age > 1. => {
                let queued = world.query::<&ReplacementHealth>().iter(world).count();
                let evidence = world.resource::<RoundEvidence>();
                let stalled =
                    evidence.cap_waits > run.cap_waits && evidence.replacement_admissions == 0;
                row(
                    &mut run,
                    "ordinary_and_special_cap_preserves_replacement_queue_and_budget",
                    stalled && queued == 1 && world.resource::<Session>().pending == 8,
                );
                let extras: Vec<_> = run.targets.drain(7..).map(|(entity, _)| entity).collect();
                for entity in extras {
                    world.despawn(entity);
                }
                run.phase = 4;
                run.age = 0.;
            }
            4 if world.resource::<RoundEvidence>().replacement_admissions > 0 || run.age > 25. => {
                let admitted = world.resource::<RoundEvidence>().replacement_admissions > 0;
                let maximum = world
                    .resource::<GameDefinition>()
                    .rounds
                    .health(world.resource::<Session>().round);
                let health = world
                    .query::<(&Vitality, &Zombie)>()
                    .iter(world)
                    .any(|(h, _)| h.current() == Some(maximum - 25) && h.maximum() == maximum);
                row(
                    &mut run,
                    "replacement_admitted_with_preserved_health",
                    admitted && health,
                );
                let queued = world.query::<&ReplacementHealth>().iter(world).count();
                row(
                    &mut run,
                    "health_queue_consumed_only_on_admission",
                    admitted && queued == 0,
                );
                let actors: Vec<_> = world
                    .query_filtered::<Entity, With<Zombie>>()
                    .iter(world)
                    .collect();
                for entity in actors {
                    world.despawn(entity);
                }
                run.targets.clear();
                let tunnel = world
                    .resource::<GameDefinition>()
                    .environment
                    .triggers
                    .iter()
                    .find(|t| {
                        t.source
                            .get("targetname")
                            .is_some_and(|name| name == "zone_air_stairs")
                    })
                    .and_then(|t| t.interior_points().into_iter().next());
                row(&mut run, "native_tunnel_volume_available", tunnel.is_some());
                let Some(tunnel) = tunnel else {
                    finish(world, &run);
                    return;
                };
                let origin = tunnel + Vec3::Y * world.get::<Player>(player).unwrap().height * 0.5;
                run.origin = Some(origin);
                world
                    .entity_mut(player)
                    .remove::<crate::environment::hazards::BoundaryMonitor>();
                world.resource_mut::<Session>().pending = 30;
                for offset in [
                    Vec3::new(0., 0., 45.),
                    Vec3::new(0., 0., -70.),
                    Vec3::new(0., 0., 65.),
                ] {
                    let position = origin + offset;
                    let entity = actor(world, position, PopulationKind::Ordinary);
                    run.targets.push((entity, position));
                }
                hit(world, player, run.targets[2].0, true);
                run.checks = world.resource::<RoundEvidence>().distance_checks;
                run.phase = 5;
                run.age = 0.;
            }
            5 if world.resource::<RoundEvidence>().distance_checks > run.checks => {
                let protected = world.get_entity(run.targets[0].0).is_ok();
                let distant_visible = world.get_entity(run.targets[1].0).is_err();
                let damaged = world.get_entity(run.targets[2].0).is_err();
                row(&mut run, "native_tunnel_distance_multiplier", protected);
                row(
                    &mut run,
                    "visible_beyond_native_distance_retired",
                    distant_visible,
                );
                let pending = world.resource::<Session>().pending;
                row(
                    &mut run,
                    "known_damaged_high_population_requeued",
                    damaged && pending == 32,
                );
                let maximum = world
                    .resource::<GameDefinition>()
                    .rounds
                    .health(world.resource::<Session>().round);
                let queued: Vec<_> = world
                    .query::<&ReplacementHealth>()
                    .iter(world)
                    .map(|h| h.current)
                    .collect();
                row(
                    &mut run,
                    "high_population_damage_health_preserved",
                    queued == vec![maximum - 25],
                );
                let actors: Vec<_> = world
                    .query_filtered::<Entity, With<Zombie>>()
                    .iter(world)
                    .collect();
                for entity in actors {
                    world.despawn(entity);
                }
                let queue: Vec<_> = world
                    .query_filtered::<Entity, With<ReplacementHealth>>()
                    .iter(world)
                    .collect();
                for entity in queue {
                    world.despawn(entity);
                }
                run.targets.clear();
                let game = world.resource::<GameDefinition>();
                let generator = game.content.generators.first().unwrap();
                let placement = game
                    .content
                    .placements
                    .iter()
                    .find(|p| p.id == generator.placement)
                    .unwrap();
                let generator_id = generator.id.clone();
                let origin = Vec3::from_array(placement.position)
                    + Vec3::Y * world.get::<Player>(player).unwrap().height * 0.5;
                run.origin = Some(origin);
                world.get_mut::<Transform>(player).unwrap().translation = origin;
                world
                    .resource_mut::<crate::generators::Generators>()
                    .activate(&generator_id, player)
                    .unwrap();
                for index in 0..20 {
                    let position = origin
                        + Vec3::new(
                            (index % 5) as f32 * 2. - 4.,
                            0.,
                            12. + (index / 5) as f32 * 2.,
                        );
                    let entity = actor(world, position, PopulationKind::Ordinary);
                    run.targets.push((entity, position));
                }
                world.resource_mut::<Session>().pending = 6;
                world.resource_mut::<Session>().spawn_timer =
                    Timer::from_seconds(0.01, TimerMode::Once);
                run.cap_waits = world.resource::<RoundEvidence>().cap_waits;
                run.phase = 6;
                run.age = 0.;
            }
            6 if run.age > 0.7 => {
                let busy = world.resource::<crate::generators::Generators>().busy();
                let stalled = world.resource::<RoundEvidence>().cap_waits > run.cap_waits;
                row(
                    &mut run,
                    "generator_reserves_four_round_enemy_slots",
                    busy && stalled && world.resource::<Session>().pending == 6,
                );
                for capture in world
                    .resource_mut::<crate::generators::Generators>()
                    .sites
                    .values_mut()
                {
                    capture.state = crate::generators::GeneratorState::Inactive;
                }
                let guards: Vec<_> = world
                    .query::<(Entity, &PopulationKind)>()
                    .iter(world)
                    .filter_map(|(e, k)| (*k == PopulationKind::GeneratorDefender).then_some(e))
                    .collect();
                for entity in guards {
                    world.despawn(entity);
                }
                run.corpse_score = (
                    world.resource::<Session>().score,
                    world.resource::<Session>().kills,
                );
                let origin = run.origin.unwrap();
                for index in 0..11 {
                    let position = origin + Vec3::new(index as f32 - 5., 0., 25.);
                    let entity = actor(world, position, PopulationKind::Ordinary);
                    run.corpses.push(entity);
                    world.resource_mut::<crate::damage::DamageQueue>().0.push(
                        crate::damage::Impact {
                            point: None,
                            attacker: player,
                            target: entity,
                            source: crate::damage::ImpactSource::Powerup,
                            category: crate::origins::DamageCategory::Explosive,
                            region: crate::weapons::HitRegion::None,
                            amount: 10000,
                        },
                    );
                }
                run.pending = world.resource::<Session>().pending;
                world.resource_mut::<Session>().spawn_timer =
                    Timer::from_seconds(0.01, TimerMode::Once);
                run.phase = 7;
                run.age = 0.;
            }
            7 if run.age
                > world
                    .resource::<GameDefinition>()
                    .content
                    .enemies
                    .iter()
                    .flat_map(|enemy| enemy.combat_states.values())
                    .flatten()
                    .map(|clip| clip.seconds)
                    .fold(0., f32::max)
                    + 1. =>
            {
                let limit = world.resource::<GameDefinition>().rounds.corpse_limit as usize;
                let retained = run
                    .corpses
                    .iter()
                    .filter(|entity| world.get::<crate::enemies::ActorCorpse>(**entity).is_some())
                    .count();
                let absent = run
                    .corpses
                    .iter()
                    .filter(|entity| world.get_entity(**entity).is_err())
                    .count();
                row(
                    &mut run,
                    "native_retirement_preserves_eight_corpse_slots",
                    retained == limit && absent == 11 - limit,
                );
                let admitted = world.resource::<Session>().pending < run.pending;
                row(&mut run, "corpse_retirement_reopens_admission", admitted);
                let unchanged = run.corpse_score
                    == (
                        world.resource::<Session>().score,
                        world.resource::<Session>().kills,
                    );
                row(
                    &mut run,
                    "corpse_probe_has_no_kill_or_score_rewards",
                    unchanged,
                );
                finish(world, &run);
            }
            _ => (),
        }
    });
}
fn finish(world: &mut World, run: &Run) {
    let passed = !run.rows.is_empty() && run.rows.iter().all(|row| row["passed"] == true);
    let receipt = serde_json::json!({"passed":passed,"cases":run.rows});
    let output = world.resource::<LaunchOptions>().verify.as_ref().unwrap();
    let saved = !output.starts_with(env!("CARGO_MANIFEST_DIR"))
        && std::fs::create_dir_all(output).is_ok()
        && std::fs::write(output.join("replacements.json"), receipt.to_string()).is_ok();
    info!("Replacement verification: {receipt}");
    world.write_message(if passed && saved {
        AppExit::Success
    } else {
        AppExit::error()
    });
}
