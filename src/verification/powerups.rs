use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    gameplay::{Session, SessionEntity, Zombie},
    life::{LifeState, Vitality},
    player::Player,
    powerups::{Drops, Pickup, PowerupEffects},
    weapons::WeaponInventory,
};
use bevy::{prelude::*, world_serialization::WorldAssetRoot};
use std::time::Instant;

#[derive(Resource)]
struct Run {
    started: Instant,
    changed: Instant,
    age: f32,
    phase: u8,
    magazine: u32,
    baseline: u32,
    kills: u32,
    target: Option<Entity>,
    target_position: Vec3,
    owner: Option<Entity>,
    timers: [f32; 3],
    rows: Vec<serde_json::Value>,
}
pub(super) fn install(app: &mut App) {
    app.insert_resource(Run {
        started: Instant::now(),
        changed: Instant::now(),
        age: 0.,
        phase: 0,
        magazine: 0,
        baseline: 0,
        kills: 0,
        target: None,
        target_position: Vec3::ZERO,
        owner: None,
        timers: [0.; 3],
        rows: vec![],
    })
    .add_systems(Update, drive);
}
fn row(run: &mut Run, case: &str, passed: bool) {
    run.rows
        .push(serde_json::json!({"case":case,"passed":passed}));
}
fn next(run: &mut Run) {
    run.phase += 1;
    run.age = 0.;
    run.changed = Instant::now();
}
fn pickup(world: &mut World, id: &str, position: Vec3, seconds: f32) -> Entity {
    let game = world.resource::<GameDefinition>();
    let index = game
        .content
        .powerups
        .iter()
        .position(|d| d.id.0 == id)
        .unwrap();
    let scene = game
        .content
        .presentations
        .iter()
        .find(|p| Some(&p.id) == game.content.powerups[index].presentation.as_ref())
        .unwrap()
        .scene
        .as_ref()
        .unwrap()
        .clone();
    let handle = world.resource::<AssetServer>().load(scene);
    world
        .spawn((
            SessionEntity,
            Pickup {
                definition: index,
                remaining: seconds,
            },
            WorldAssetRoot(handle),
            Transform::from_translation(position),
            Visibility::Inherited,
        ))
        .id()
}
fn actor(world: &mut World, position: Vec3) -> Entity {
    let id = world
        .resource::<GameDefinition>()
        .content
        .enemies
        .iter()
        .find(|e| e.family == "german")
        .unwrap()
        .id
        .clone();
    world
        .spawn((
            SessionEntity,
            Zombie::new(position, id, crate::rounds::Gait::Walk, 1.),
            crate::rounds::PopulationKind::Ordinary,
            Vitality::ordinary_actor(100, 100).unwrap(),
            crate::player::capsule(0.3, 1.7),
            Transform::from_translation(position),
            Visibility::Inherited,
        ))
        .id()
}
fn hit(world: &mut World, owner: Entity, target: Entity) {
    world
        .resource_mut::<crate::damage::DamageQueue>()
        .0
        .push(crate::damage::Impact {
            attacker: owner,
            target,
            source: crate::damage::ImpactSource::Weapon(crate::origins::WeaponId(
                "weapon.c96_zm".into(),
            )),
            category: crate::origins::DamageCategory::Bullet,
            region: crate::weapons::HitRegion::Head,
            amount: 1,
        });
}
fn timers(world: &World) -> [f32; 3] {
    let effects = world.resource::<PowerupEffects>();
    [
        effects.double_points,
        effects.insta_kill,
        effects.zombie_blood,
    ]
}
fn blast(world: &mut World, owner: Entity, target: Entity, damage: u32) {
    let session = *world.resource::<crate::session::SessionId>();
    let center = world.get::<Transform>(target).unwrap().translation;
    let definition = world
        .get::<crate::equipment::GrenadeInventory>(owner)
        .unwrap()
        .definition
        .clone();
    world
        .resource_mut::<crate::equipment::BlastQueue>()
        .0
        .push(crate::equipment::BlastRequest {
            session,
            owner,
            definition,
            center,
            radius: 0.5,
            inner_radius: 0.25,
            inner_damage: damage,
            outer_damage: damage,
            direction: None,
        });
}
fn drive(world: &mut World) {
    world.resource_scope(|world, mut run: Mut<Run>| {
        let state = *world.resource::<State<GameState>>().get();
        if run.started.elapsed().as_secs() > 180 || state == GameState::LoadError {
            let passed = false;
            row(&mut run, "loaded_and_finished", passed);
            finish(world, &run);
            return;
        }
        if state == GameState::Playing {
            run.age += world.resource::<Time>().delta_secs();
        }
        let Some(owner) = world
            .query_filtered::<Entity, With<Player>>()
            .iter(world)
            .next()
        else {
            return;
        };
        let position = world.get::<Transform>(owner).unwrap().translation;
        world
            .get_mut::<crate::player_health::Recovery>(owner)
            .unwrap()
            .immunity = 60.;
        {
            let mut session = world.resource_mut::<Session>();
            session.pending = 6;
            session.spawn_timer = Timer::from_seconds(1000., TimerMode::Once);
        }
        let target_position = run.target_position;
        let (magazine, baseline, kills, timers_before) =
            (run.magazine, run.baseline, run.kills, run.timers);
        match run.phase {
            0 if state == GameState::Playing && run.age > 3. => {
                run.owner = Some(owner);
                let actors: Vec<_> = world
                    .query_filtered::<Entity, With<Zombie>>()
                    .iter(world)
                    .collect();
                for entity in actors {
                    world.despawn(entity);
                }
                world.resource_mut::<crate::input::GameInput>().fire = true;
                world
                    .get_mut::<crate::equipment::GrenadeInventory>(owner)
                    .unwrap()
                    .count = 0;
                next(&mut run);
            }
            1 if run.age > 0.5 => {
                run.magazine = world
                    .get::<WeaponInventory>(owner)
                    .unwrap()
                    .active()
                    .unwrap()
                    .magazine;
                pickup(world, "powerup.full_ammo", position, 26.5);
                next(&mut run);
            }
            2 if run.age > 0.4 => {
                let game = world.resource::<GameDefinition>();
                let inventory = world.get::<WeaponInventory>(owner).unwrap();
                let weapon = inventory.definition(&game.content.weapons).unwrap();
                let grenades = world
                    .get::<crate::equipment::GrenadeInventory>(owner)
                    .unwrap();
                row(
                    &mut run,
                    "max_ammo_reserves_and_equipment_preserves_magazine",
                    inventory.active().unwrap().magazine == magazine
                        && magazine < weapon.magazine
                        && inventory.reserve(weapon) == weapon.reserve
                        && grenades.count == grenades.capacity,
                );
                pickup(world, "powerup.double_points", position, 26.5);
                next(&mut run);
            }
            3 if run.age > 0.4 => {
                row(
                    &mut run,
                    "double_points_pickup",
                    world.resource::<PowerupEffects>().double_points > 29.,
                );
                world.resource_mut::<PowerupEffects>().double_points = 5.;
                pickup(world, "powerup.double_points", position, 26.5);
                pickup(world, "powerup.insta_kill", position, 26.5);
                pickup(world, "powerup.zombie_blood", position, 26.5);
                next(&mut run);
            }
            4 if run.age > 0.5 => {
                row(
                    &mut run,
                    "repeat_pickup_resets_and_other_effects_activate",
                    timers(world).iter().all(|t| *t > 29.),
                );
                let target = actor(world, position + Vec3::X * 3.);
                run.target = Some(target);
                run.target_position = world.get::<Transform>(target).unwrap().translation;
                world.resource_mut::<Session>().score = 2501;
                run.baseline = world.resource::<Session>().points;
                run.kills = world.resource::<Session>().kills;
                next(&mut run);
            }
            5 if run.age > 1.
                && run.target.is_some_and(|e| {
                    world.get::<crate::hit_regions::SkeletalReady>(e).is_some()
                }) =>
            {
                let target = run.target.unwrap();
                row(
                    &mut run,
                    "zombie_blood_suppresses_pursuit",
                    world
                        .get::<Zombie>(target)
                        .is_some_and(|z| z.action == crate::origins::EnemyAnimation::Idle)
                        && world.get::<Transform>(target).is_some_and(|t| {
                            t.translation.xz().distance_squared(target_position.xz()) < 0.01
                        }),
                );
                world.resource_mut::<PowerupEffects>().zombie_blood = 0.;
                next(&mut run);
            }
            6 if run.age > 1. => {
                let target = run.target.unwrap();
                row(
                    &mut run,
                    "expired_zombie_blood_restores_pursuit",
                    world.get::<Transform>(target).is_some_and(|t| {
                        t.translation.xz().distance_squared(target_position.xz()) > 0.01
                    }) && world
                        .get::<Zombie>(target)
                        .is_some_and(|z| z.action != crate::origins::EnemyAnimation::Idle),
                );
                hit(world, owner, target);
                next(&mut run);
            }
            7 if run.age > 0.4 => {
                let session = world.resource::<Session>();
                let dead = world
                    .get::<Vitality>(run.target.unwrap())
                    .is_none_or(|v| v.state() == LifeState::ActorDead);
                row(
                    &mut run,
                    "instakill_and_double_headshot_award",
                    dead && session.points == baseline + 200 && session.kills == kills + 1,
                );
                row(
                    &mut run,
                    "attributed_death_spawns_native_drop",
                    world.resource::<Drops>().spawned >= 1,
                );
                hit(world, owner, run.target.unwrap());
                next(&mut run);
            }
            8 if run.age > 0.4 => {
                row(
                    &mut run,
                    "terminal_damage_awards_once",
                    world.resource::<Session>().points == baseline + 200,
                );
                let drops: Vec<_> = world
                    .query_filtered::<Entity, With<Pickup>>()
                    .iter(world)
                    .collect();
                for drop in drops {
                    world.get_mut::<Pickup>(drop).unwrap().remaining = 0.01;
                }
                actor(world, position + Vec3::Z * 5.5);
                actor(world, position + Vec3::Z * 6.5);
                run.baseline = world.resource::<Session>().points;
                run.kills = world.resource::<Session>().kills;
                pickup(world, "powerup.nuke", position, 26.5);
                next(&mut run);
            }
            9 if run.age > 3. => {
                let alive = world
                    .query_filtered::<&Vitality, With<Zombie>>()
                    .iter(world)
                    .any(|v| v.state() == LifeState::ActorAlive);
                let session = world.resource::<Session>();
                row(
                    &mut run,
                    "nuke_damage_and_double_award_without_kill_credit",
                    !alive && session.points == baseline + 800 && session.kills == kills,
                );
                row(
                    &mut run,
                    "uncollected_drop_expires",
                    world.resource::<Drops>().expired >= 1,
                );
                run.timers = timers(world);
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Paused);
                next(&mut run);
            }
            10 if state == GameState::Paused && run.changed.elapsed().as_secs_f32() > 1. => {
                row(
                    &mut run,
                    "effect_timers_pause",
                    timers(world)
                        .iter()
                        .zip(timers_before)
                        .all(|(a, b)| (*a - b).abs() < 0.05),
                );
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Playing);
                next(&mut run);
            }
            11 if state == GameState::Playing && run.age > 0.5 => {
                row(
                    &mut run,
                    "effect_timers_resume",
                    timers(world)[0] < timers_before[0] - 0.3,
                );
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::GameOver);
                next(&mut run);
            }
            12 if state == GameState::GameOver => {
                world.write_message(crate::input::RestartGame);
                next(&mut run);
            }
            13 if state == GameState::Playing && run.owner != Some(owner) => {
                row(
                    &mut run,
                    "fresh_session_clears_effects_and_drop_budget",
                    timers(world) == [0.; 3] && world.resource::<Drops>().spawned == 0,
                );
                run.baseline = world.resource::<Session>().points;
                run.kills = world.resource::<Session>().kills;
                let target = actor(world, position + Vec3::X * 3.);
                run.target = Some(target);
                blast(world, owner, target, 10);
                next(&mut run);
            }
            14 if run.age > 0.4 => {
                let target = run.target.unwrap();
                row(
                    &mut run,
                    "nonlethal_explosion_awards_ten_points",
                    world.resource::<Session>().points == baseline + 10
                        && world.get::<Vitality>(target).and_then(|v| v.current()) == Some(90),
                );
                blast(world, owner, target, 100);
                next(&mut run);
            }
            15 if run.age > 0.4 => {
                row(
                    &mut run,
                    "explosive_kill_awards_fifty_without_location_bonus",
                    world.resource::<Session>().points == baseline + 60
                        && world.resource::<Session>().kills == kills + 1
                        && world
                            .resource::<crate::damage::CombatHistory>()
                            .headshot_kills
                            == 0,
                );
                world.resource_mut::<PowerupEffects>().double_points = 30.;
                let target = actor(world, position - Vec3::X * 3.);
                run.target = Some(target);
                blast(world, owner, target, 10);
                next(&mut run);
            }
            16 if run.age > 0.4 => {
                row(
                    &mut run,
                    "double_points_applies_to_explosive_damage",
                    world.resource::<Session>().points == baseline + 80,
                );
                blast(world, owner, run.target.unwrap(), 100);
                next(&mut run);
            }
            17 if run.age > 0.4 => {
                row(
                    &mut run,
                    "double_points_applies_to_explosive_kill",
                    world.resource::<Session>().points == baseline + 180
                        && world.resource::<Session>().kills == kills + 2
                        && world
                            .resource::<crate::damage::CombatHistory>()
                            .headshot_kills
                            == 0,
                );
                finish(world, &run);
            }
            _ => (),
        }
    });
}
fn finish(world: &mut World, run: &Run) {
    let passed = !run.rows.is_empty() && run.rows.iter().all(|r| r["passed"] == true);
    let output = world.resource::<LaunchOptions>().verify.as_ref().unwrap();
    let receipt = serde_json::json!({"passed":passed,"cases":run.rows});
    let saved = !output.starts_with(env!("CARGO_MANIFEST_DIR"))
        && std::fs::create_dir_all(output).is_ok()
        && std::fs::write(output.join("powerups.json"), receipt.to_string()).is_ok();
    world.write_message(if passed && saved {
        AppExit::Success
    } else {
        AppExit::error()
    });
}
