use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    environment::{
        hazards::{BoundaryMonitor, BoundaryVolume, DamageVolume},
        triggers::{MapTrigger, TRIGGER_LAYER},
    },
    gameplay::{GameStep, Session, SessionEntity, Zombie},
    life::{LifeState, Vitality},
    player::Player,
    player_health::{PlayerDamage, Recovery},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::time::Instant;

#[derive(Resource)]
struct Run {
    started: Instant,
    changed: Instant,
    phase: u8,
    age: f32,
    position: Option<Vec3>,
    spawn: Vec3,
    closed: Option<Vec3>,
    boxes: Vec<Vec3>,
    player: Option<Entity>,
    rows: Vec<serde_json::Value>,
    hits: Vec<f32>,
    paused_hits: usize,
    safe: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.insert_resource(Run {
        started: Instant::now(),
        changed: Instant::now(),
        phase: 0,
        age: 0.,
        position: None,
        spawn: Vec3::ZERO,
        closed: None,
        boxes: vec![],
        player: None,
        rows: vec![],
        hits: vec![],
        paused_hits: 0,
        safe: None,
    })
    .add_systems(Update, drive)
    .add_systems(
        FixedUpdate,
        hold.in_set(GameStep::Movement)
            .after(crate::player::move_player),
    )
    .add_systems(FixedUpdate, observe.after(GameStep::PlayerLife));
}
fn hold(
    mut commands: Commands,
    mut run: ResMut<Run>,
    state: Res<State<GameState>>,
    time: Res<Time<Fixed>>,
    game: Option<Res<GameDefinition>>,
    access: Res<crate::access::ZoneAccess>,
    spatial: SpatialQuery,
    volumes: Query<&BoundaryVolume>,
    mut players: Query<(
        &mut Transform,
        &mut Player,
        &Collider,
        &mut crate::movement::Mobility,
    )>,
    actors: Query<Entity, With<Zombie>>,
    mut session: ResMut<Session>,
    mut input: ResMut<crate::input::GameInput>,
) {
    input.clear();
    if **state != GameState::Playing {
        return;
    }
    run.age += time.delta_secs();
    session.pending = 6;
    session.spawn_timer = Timer::from_seconds(1000., TimerMode::Once);
    for entity in &actors {
        commands.entity(entity).despawn();
    }
    let Ok((mut pose, mut player, shape, mut mobility)) = players.single_mut() else {
        return;
    };
    if run.closed.is_none()
        && let Some(game) = game
    {
        run.closed = game
            .environment
            .triggers
            .iter()
            .flat_map(|t| t.interior_points())
            .find(|point| {
                let mut closed = false;
                let mut unsafe_candidate = false;
                for entity in spatial.shape_intersections(
                    shape,
                    *point,
                    Quat::IDENTITY,
                    &SpatialQueryFilter::from_mask(TRIGGER_LAYER),
                ) {
                    match volumes.get(entity) {
                        Ok(BoundaryVolume::Playable(zone)) => {
                            closed |= !access.reachable.contains(zone);
                            unsafe_candidate |= access.reachable.contains(zone);
                        }
                        Ok(BoundaryVolume::Safe | BoundaryVolume::Lethal) => {
                            unsafe_candidate = true
                        }
                        _ => (),
                    }
                }
                closed && !unsafe_candidate
            });
    }
    if let Some(position) = run.position {
        pose.translation = position;
        player.velocity = Vec3::ZERO;
        mobility.airborne = false;
        mobility.peak = position.y - player.height * 0.5;
        mobility.fall_speed = 0.;
    }
}
fn observe(time: Res<Time<Fixed>>, mut run: ResMut<Run>, mut hits: MessageReader<PlayerDamage>) {
    for hit in hits.read() {
        if hit.impact.source.id() == "hazard.environment" {
            run.hits.push(time.elapsed_secs());
        }
    }
}
fn advance(run: &mut Run, phase: u8) {
    run.phase = phase;
    run.age = 0.;
    run.changed = Instant::now();
}
fn row(run: &mut Run, case: &str, passed: bool) {
    run.rows
        .push(serde_json::json!({"case":case,"passed":passed,"seconds":run.age}));
}
fn volume(world: &mut World, position: Vec3) -> Entity {
    world
        .spawn((
            SessionEntity,
            BoundaryVolume::Safe,
            Collider::cuboid(4., 4., 4.),
            Sensor,
            CollisionLayers::new(TRIGGER_LAYER, LayerMask::NONE),
            Transform::from_translation(position),
        ))
        .id()
}
fn drive(world: &mut World) {
    world.resource_scope(|world, mut run: Mut<Run>| {
        let state = *world.resource::<State<GameState>>().get();
        if run.started.elapsed().as_secs() > 240 {
            row(&mut run, "timeout", false);
            finish(world, &run);
            return;
        }
        let player = world.query_filtered::<Entity, With<Player>>().iter(world).next();
        let Some(entity) = player else { return; };
        let health = *world.get::<Vitality>(entity).unwrap();
        let alive = health.state() == LifeState::Standing;
        match run.phase {
            0 if state == GameState::Playing && run.age > 1. => {
                run.spawn = world.get::<Transform>(entity).unwrap().translation;
                run.position = Some(run.spawn);
                run.player = Some(entity);
                run.boxes = world.query_filtered::<(&BoundaryVolume, &Transform), Without<MapTrigger>>()
                    .iter(world).filter_map(|(v,p)| matches!(v, BoundaryVolume::Lethal).then_some(p.translation)).collect();
                let count = run.boxes.len();
                row(&mut run, "two_native_kill_boxes", count == 2);
                world.entity_mut(entity).insert(BoundaryMonitor::default());
                advance(&mut run, 1);
            }
            1 if run.age > 4. => {
                row(&mut run, "authored_spawn_playable", alive);
                let closed = run.closed;
                row(&mut run, "authored_closed_zone_found", closed.is_some());
                if closed.is_none() || run.boxes.len() != 2 { finish(world, &run); return; }
                run.position = closed;
                world.entity_mut(entity).insert(BoundaryMonitor::default());
                let perk = world.resource::<GameDefinition>().solo_revive.perk.clone();
                let mut owned = world.get_mut::<crate::solo_revive::PerkInventory>(entity).unwrap();
                owned.owned.insert(perk);
                owned.solo_charges = 1;
                world.get_mut::<Recovery>(entity).unwrap().immunity = 30.;
                advance(&mut run, 2);
            }
            2 | 4 | 6 if state == GameState::GameOver || run.age > 4.5 => {
                let case = match run.phase { 2 => "closed_zone_bypasses_immunity_and_solo_revive", 4 => "native_box_one_lethal", _ => "native_box_two_lethal" };
                row(&mut run, case, health.state() == LifeState::HumanTerminal && state == GameState::GameOver && world.resource::<Session>().downs == 1);
                if state != GameState::GameOver { finish(world, &run); return; }
                world.write_message(crate::input::RestartGame);
                let phase = run.phase + 1;
                advance(&mut run, phase);
                run.position = None;
            }
            3 | 5 | 7 if state == GameState::Playing && run.player != Some(entity) => {
                row(&mut run, "fresh_player_after_death", alive && health.current() == Some(health.maximum()) && world.resource::<Session>().downs == 0);
                run.player = Some(entity);
                let phase = run.phase;
                run.position = Some(run.boxes[usize::from(phase == 5)]);
                if phase == 7 {
                    let position = run.position.unwrap();
                    run.safe = Some(volume(world, position));
                }
                advance(&mut run, phase + 1);
            }
            8 if run.age > 4. => {
                row(&mut run, "safe_volume_overrides_native_kill_box", alive);
                if let Some(safe) = run.safe.take() { world.despawn(safe); }
                let spawn = run.spawn;
                let safe = volume(world, spawn);
                world.entity_mut(safe).insert(DamageVolume { amount: 1, interval: 0.5, category: crate::origins::DamageCategory::Environmental });
                run.safe = Some(safe);
                run.position = Some(spawn);
                run.hits.clear();
                world.get_mut::<Recovery>(entity).unwrap().immunity = 0.;
                advance(&mut run, 9);
            }
            9 if run.age > 2.2 => {
                let intervals: Vec<_> = run.hits.windows(2).map(|v|v[1]-v[0]).collect();
                let passed = run.hits.len() >= 4 && intervals.iter().all(|dt| (*dt - 0.5).abs() < 0.04);
                let result = serde_json::json!({"case":"periodic_environmental_damage","passed":passed,"intervals":intervals,"hits":run.hits});
                run.rows.push(result);
                run.paused_hits = run.hits.len();
                world.resource_mut::<NextState<GameState>>().set(GameState::Paused);
                advance(&mut run, 10);
            }
            10 if state == GameState::Paused && run.changed.elapsed().as_secs_f32() > 1. => {
                let passed = run.hits.len() == run.paused_hits && run.age < 0.05;
                row(&mut run, "paused_hazard_clocks", passed);
                world.resource_mut::<NextState<GameState>>().set(GameState::Playing);
                advance(&mut run, 11);
            }
            11 if state == GameState::Playing && run.age > 0.65 => {
                let passed = run.hits.len() > run.paused_hits && alive;
                row(&mut run, "resumed_periodic_damage", passed);
                world.resource_mut::<NextState<GameState>>().set(GameState::GameOver);
                advance(&mut run, 12);
            }
            12 if state == GameState::GameOver => {
                world.write_message(crate::input::RestartGame);
                run.position = None;
                advance(&mut run, 13);
            }
            13 if state == GameState::Playing && run.player != Some(entity) => {
                let safe_removed = run.safe.is_none_or(|e| world.get_entity(e).is_err());
                row(&mut run, "restart_removes_session_damage_volume", safe_removed && alive);
                run.player = Some(entity);
                run.position = Some(run.spawn);
                run.paused_hits = run.hits.len();
                advance(&mut run, 14);
            }
            14 if run.age > 4. => {
                let passed = alive && run.hits.len() == run.paused_hits;
                row(&mut run, "fresh_restart_boundary_and_damage_clocks", passed);
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
        && std::fs::write(output.join("hazards.json"), receipt.to_string()).is_ok();
    info!("Hazard verification: {receipt}");
    world.write_message(if passed && saved {
        AppExit::Success
    } else {
        AppExit::error()
    });
}
