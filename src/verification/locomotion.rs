use super::*;
use crate::{
    enemies,
    life::Vitality,
    origins::content::{ContentId, EnemyAnimation, EnemyLocomotion},
};

#[derive(Resource)]
struct Run {
    output: PathBuf,
    started: Instant,
    age: f32,
    index: usize,
    cases: Vec<(EnemyAnimation, String)>,
    actors: Vec<(Entity, ContentId)>,
    earlier: Vec<serde_json::Value>,
    rows: Vec<serde_json::Value>,
    finished: bool,
    settle: u32,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(Update, drive.before(enemies::EnemyAnimationPlayback))
        .add_systems(PostUpdate, observe.after(bevy::app::AnimationSystems));
}
fn setup(mut commands: Commands, options: Res<LaunchOptions>) {
    let output = options.verify.as_ref().unwrap();
    if output.starts_with(env!("CARGO_MANIFEST_DIR")) || std::fs::create_dir_all(output).is_err() {
        commands.write_message(AppExit::error());
        return;
    }
    commands.insert_resource(Run {
        output: output.clone(),
        started: Instant::now(),
        age: 0.,
        index: 0,
        cases: vec![],
        actors: vec![],
        earlier: vec![],
        rows: vec![],
        finished: false,
        settle: 0,
    });
}
fn drive(world: &mut World) {
    let Some(mut run) = world.remove_resource::<Run>() else {
        return;
    };
    let state = **world.resource::<State<GameState>>();
    if run.started.elapsed().as_secs() > 180 || state == GameState::LoadError {
        finish(world, &mut run, false);
    } else if state == GameState::Playing && !run.finished {
        world.resource_mut::<GameInput>().clear();
        let mut session = world.resource_mut::<Session>();
        session.pending = 0;
        session.intermission = Some(Timer::from_seconds(600., TimerMode::Once));
        if run.cases.is_empty() {
            let game = world.resource::<GameDefinition>();
            let enemies = game.content.enemies.clone();
            for action in [
                EnemyAnimation::Walk,
                EnemyAnimation::Run,
                EnemyAnimation::Sprint,
            ] {
                let state = EnemyLocomotion::for_action(action).unwrap();
                for id in &enemies[0].locomotion_states[&state] {
                    let name = id.0.rsplit('.').next().unwrap().to_owned();
                    if !run.cases.iter().any(|(_, n)| *n == name) {
                        run.cases.push((action, name));
                    }
                }
            }
            let player = world
                .query_filtered::<(&Transform, &Player), With<Player>>()
                .iter(world)
                .next()
                .map(|(t, p)| (t.translation, p.yaw));
            if let Some((origin, yaw)) = player {
                let orientation = Quat::from_rotation_y(yaw);
                let forward = orientation * Vec3::NEG_Z;
                let right = orientation * Vec3::X;
                let old: Vec<_> = world
                    .query_filtered::<Entity, With<Zombie>>()
                    .iter(world)
                    .collect();
                for e in old {
                    world.despawn(e);
                }
                for (i, enemy) in enemies.iter().enumerate() {
                    let position = origin
                        + forward * (2.5 + (i / 7) as f32 * 1.3)
                        + right * ((i % 7) as f32 - 3.) * 0.7;
                    let mut zombie =
                        Zombie::new(position, enemy.id.clone(), crate::rounds::Gait::Walk);
                    zombie.stunned = 600.;
                    let e = world
                        .spawn((
                            zombie,
                            crate::player::capsule(0.3, 1.7),
                            avian3d::prelude::ColliderDisabled,
                            Vitality::ordinary_actor(150, 150).unwrap(),
                            crate::gameplay::SessionEntity,
                            Transform::from_translation(position)
                                .with_rotation(Quat::from_rotation_y(yaw + std::f32::consts::PI)),
                            Visibility::Inherited,
                        ))
                        .id();
                    run.actors.push((e, enemy.id.clone()));
                }
            }
        }
        if run.index < run.cases.len() {
            let (action, name) = &run.cases[run.index];
            let ready = run
                .actors
                .iter()
                .all(|(e, _)| world.get::<crate::hit_regions::SkeletalReady>(*e).is_some());
            if ready {
                run.age += world.resource::<Time>().delta_secs();
            }
            for (entity, id) in &run.actors {
                let enemy = world
                    .resource::<GameDefinition>()
                    .content
                    .enemies
                    .iter()
                    .find(|e| e.id == *id)
                    .unwrap()
                    .clone();
                if let Some(mut zombie) = world.get_mut::<Zombie>(*entity) {
                    zombie.action = *action;
                    for seed in 0..64 {
                        let candidate =
                            Zombie::new(Vec3::ZERO, id.clone(), crate::rounds::Gait::Walk)
                                .with_locomotion_seed(seed);
                        let mut candidate = candidate;
                        candidate.action = *action;
                        if candidate.locomotion_clip(&enemy) == Some(name.as_str()) {
                            zombie.set_locomotion_seed(seed);
                            zombie.elapsed = run.age;
                            break;
                        }
                    }
                }
            }
        }
    }
    world.insert_resource(run);
}
fn finish(world: &mut World, run: &mut Run, passed: bool) {
    if run.finished {
        return;
    }
    let receipt = serde_json::json!({"passed":passed,"cases":run.rows,"clips":run.cases.len(),
        "variants":run.actors.len(),"elapsed_seconds":run.started.elapsed().as_secs_f32(),
        "scope":"Controlled stationary native pose/clock/attachment coverage; ordinary combat and restart qualify separately."});
    let _ = std::fs::write(run.output.join("locomotion.json"), receipt.to_string());
    world.write_message(if passed {
        AppExit::Success
    } else {
        AppExit::error()
    });
    run.finished = true;
}
fn observe(world: &mut World) {
    let Some(mut run) = world.remove_resource::<Run>() else {
        return;
    };
    if !run.finished && run.age >= 0.25 && run.index < run.cases.len() {
        let observations: Vec<_> = run
            .actors
            .iter()
            .map(|(e, _)| serde_json::json!(enemies::locomotion_observation(world, *e)))
            .collect();
        if run.earlier.is_empty() {
            run.earlier = observations.clone();
        }
        if run.age >= 0.9 {
            let (_, name) = &run.cases[run.index];
            for (i, observed) in observations.iter().enumerate() {
                let rigs = observed.as_array().unwrap();
                let passed = rigs.len() == 2
                    && rigs.iter().all(|r| {
                        r["ready"] == true
                            && r["clip"] == name.as_str()
                            && r["clocks"].as_array().is_some_and(|c| {
                                !c.is_empty()
                                    && c.iter().all(|t| {
                                        (t.as_f64().unwrap_or(-1.) - run.age as f64).abs() < 0.001
                                    })
                            })
                            && r["pose"].as_array().is_some_and(|p| p.len() == 4)
                    })
                    && rigs
                        .iter()
                        .zip(run.earlier[i].as_array().unwrap())
                        .any(|(a, b)| a["pose"] != b["pose"]);
                run.rows.push(serde_json::json!({"enemy":run.actors[i].1.0,"clip":name,"passed":passed,"rigs":observed,"earlier":run.earlier[i]}));
            }
            if run.index == 6 || run.index == 9 || run.index == 14 {
                world
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        run.output.join(format!("locomotion-{}.png", run.index)),
                    ));
            }
            run.index += 1;
            run.age = 0.;
            run.earlier.clear();
            let _ = std::fs::write(
                run.output.join("locomotion-progress.json"),
                serde_json::json!({"completed_clips":run.index,"cases":run.rows}).to_string(),
            );
        }
    }
    if !run.finished && !run.cases.is_empty() && run.index == run.cases.len() {
        run.settle += 1;
        let passed = !run.actors.is_empty()
            && run.rows.len() == run.cases.len() * run.actors.len()
            && run.rows.iter().all(|r| r["passed"] == true);
        if run.settle >= 4 {
            finish(world, &mut run, passed);
        }
    }
    world.insert_resource(run);
}
