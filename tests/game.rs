use avian3d::prelude::*;
use bevy::{prelude::*, time::TimeUpdateStrategy};
use origins::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    gameplay::{GameplayPlugin, Session, SessionEntity, round_budget},
    input::{GameInput, GameInputPlugin, RestartGame},
    life::{LifeState, PlayerLethalPolicy, ResolvedDamage, Vitality},
    player::{Player, PlayerPlugin},
};
use std::time::Duration;

fn definition() -> GameDefinition {
    serde_json::from_value(serde_json::json!({"version":1,"scene":"map.gltf#Scene0","collision_scene":"map.gltf#Scene1","spawn":[0.,0.02,0.],"yaw":0.,"spawn_points":[[5.,0.,0.]],"waypoints":[],"survey_views":[],"source_surfaces":1,"render_meshes":1,"collision_meshes":1,"static_model_instances":0})).unwrap()
}

fn simulation() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
        TransformPlugin,
        bevy::state::app::StatesPlugin,
        bevy::input::InputPlugin,
        PhysicsPlugins::default(),
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1. / 60.,
    )))
    .insert_resource(Time::<Fixed>::from_hz(60.))
    .insert_resource(LaunchOptions {
        verify: Some(std::env::temp_dir().join("origins-headless")),
        ..default()
    })
    .insert_resource(definition())
    .init_resource::<Assets<Mesh>>()
    .add_message::<AssetEvent<Mesh>>()
    .add_message::<bevy::window::WindowFocused>()
    .init_resource::<Assets<StandardMaterial>>()
    .init_state::<GameState>()
    .add_plugins((GameInputPlugin, PlayerPlugin, GameplayPlugin));
    app.finish();
    app.cleanup();
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(30., 0.5, 30.),
        Transform::from_xyz(0., -0.25, 0.),
    ));
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    for _ in 0..5 {
        app.update();
    }
    app
}

#[test]
fn input_edges_survive_empty_ticks_and_are_consumed_once() {
    let mut input = GameInput {
        jump: true,
        fire: true,
        reload: true,
        movement: Vec2::Y,
        ..default()
    };
    assert_eq!(input.take_edges(), (true, true, false, true));
    assert_eq!(input.take_edges(), (false, false, false, false));
    assert_eq!(input.movement, Vec2::Y);
    input.clear();
    assert_eq!(input.movement, Vec2::ZERO);
}

#[test]
fn malformed_definition_and_arguments_fail() {
    let mut data = definition();
    assert!(data.validate().is_ok());
    data.scene = "../escape.gltf#Scene0".into();
    assert!(data.validate().is_err());
    data = definition();
    data.spawn_points.clear();
    assert!(data.validate().is_err());
    data = definition();
    data.tuning.gravity = f32::NAN;
    assert!(data.validate().is_err());
    assert!(
        LaunchOptions::parse(["--config".into()].into_iter())
            .error
            .is_some()
    );
}

#[test]
fn ammunition_spawn_failure_and_round_ledger() {
    let mut session = Session::default();
    assert_eq!(session.pending, 6);
    assert_eq!(round_budget(1), 6);
    assert_eq!(round_budget(2), 8);
    assert!(!session.round_complete(0));
    // A rejected admission does not touch the ledger.
    assert_eq!(session.pending, 6);
    for _ in 0..6 {
        assert!(session.admitted_spawn());
    }
    assert!(!session.admitted_spawn());
    assert!(!session.round_complete(1));
    assert!(session.round_complete(0));
    session.ammo = 2;
    session.reserve = 3;
    session.reload(8);
    assert_eq!((session.ammo, session.reserve), (5, 0));
    session.reload(8);
    assert_eq!(session.ammo, 5);
    let mut repeat = Session::default();
    let mut original = Session::default();
    for _ in 0..12 {
        assert_eq!(repeat.spawn_index(8), original.spawn_index(8));
    }
    assert_eq!(repeat.spawn_index(0), None);
}

#[test]
fn combat_death_and_terminal_repeats() {
    let mut enemy = Vitality::ordinary_actor(150, 150).unwrap();
    for _ in 0..3 {
        enemy.apply(ResolvedDamage::new(50).unwrap()).unwrap();
    }
    assert_eq!(enemy.state(), LifeState::ActorDead);
    enemy.apply(ResolvedDamage::new(150).unwrap()).unwrap();
    assert_eq!(enemy.state(), LifeState::ActorDead);
    let mut human =
        Vitality::human(100, 100, 1, PlayerLethalPolicy::SoloOrLastEligiblePlayer).unwrap();
    human.apply(ResolvedDamage::new(50).unwrap()).unwrap();
    assert_eq!(human.current(), Some(50));
    human.apply(ResolvedDamage::new(50).unwrap()).unwrap();
    assert_eq!(human.state(), LifeState::HumanTerminal);
}

#[test]
fn player_is_grounded_and_restart_rebuilds_round_one() {
    let mut app = simulation();
    for _ in 0..15 {
        app.update();
    }
    let mut players = app.world_mut().query::<(&Player, &Transform)>();
    let (player, transform) = players.single(app.world()).unwrap();
    assert!(player.grounded, "position {:?}", transform.translation);
    assert!(transform.translation.y > 0.85 && transform.translation.y < 1.);
    app.world_mut().resource_mut::<Session>().round = 7;
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::GameOver);
    app.update();
    app.world_mut().write_message(RestartGame);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(app.world().resource::<Session>().round, 1);
    let mut roots = app
        .world_mut()
        .query_filtered::<Entity, With<SessionEntity>>();
    assert_eq!(roots.iter(app.world()).count(), 1);
}

#[test]
fn walking_collides_jumping_and_crouching_work() {
    let mut app = simulation();
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(6., 3., 0.4),
        Transform::from_xyz(0., 1.5, -2.),
    ));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    for _ in 0..90 {
        app.update();
    }
    let mut players = app.world_mut().query::<(&Player, &Transform)>();
    let (_, transform) = players.single(app.world()).unwrap();
    assert!(
        transform.translation.z < -1. && transform.translation.z > -1.6,
        "{:?}",
        transform.translation
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyW);
    app.world_mut().resource_mut::<GameInput>().jump = true;
    for _ in 0..8 {
        app.update();
    }
    let (player, transform) = players.single(app.world()).unwrap();
    assert!(!player.grounded);
    assert!(transform.translation.y > 1.25);
    for _ in 0..60 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.update();
    let (player, transform) = players.single(app.world()).unwrap();
    assert!(player.crouched);
    assert!(transform.translation.y < 0.7);
}

#[test]
fn actual_hit_detection_kills_and_round_advances_only_after_clearance() {
    let mut app = simulation();
    let position = Vec3::new(0., 0.87, -3.);
    app.world_mut().spawn((
        origins::gameplay::Zombie::new(position),
        origins::player::capsule(0.3, 1.7),
        Vitality::ordinary_actor(150, 150).unwrap(),
        Transform::from_translation(position),
        SessionEntity,
    ));
    app.world_mut().resource_mut::<Session>().pending = 0;
    for _ in 0..3 {
        app.update();
    }
    for _ in 0..3 {
        app.world_mut().resource_mut::<GameInput>().fire = true;
        app.update();
        for _ in 0..13 {
            app.update();
        }
    }
    assert_eq!(app.world().resource::<Session>().kills, 1);
    assert_eq!(app.world().resource::<Session>().ammo, 5);
    for _ in 0..310 {
        app.update();
    }
    assert_eq!(app.world().resource::<Session>().round, 2);
}

#[test]
fn missing_floor_keeps_pending_budget() {
    let mut app = simulation();
    app.world_mut()
        .resource_mut::<GameDefinition>()
        .spawn_points = vec![[100., 20., 100.]];
    for _ in 0..230 {
        app.update();
    }
    assert_eq!(app.world().resource::<Session>().pending, 6);
    assert_eq!(app.world().resource::<Session>().round, 1);
}

#[test]
fn capsule_traverses_a_step_and_focus_loss_clears_controls() {
    let mut app = simulation();
    app.world_mut().spawn((
        RigidBody::Static,
        Collider::cuboid(4., 0.3, 3.),
        Transform::from_xyz(0., 0.15, -3.),
    ));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    for _ in 0..40 {
        app.update();
    }
    let mut players = app.world_mut().query_filtered::<&Transform, With<Player>>();
    let position = players.single(app.world()).unwrap().translation;
    assert!(position.z < -2.5 && position.y > 1.1, "{position:?}");
    app.world_mut().resource_mut::<LaunchOptions>().verify = None;
    app.world_mut().spawn((
        Window {
            focused: false,
            ..default()
        },
        bevy::window::CursorOptions::default(),
    ));
    app.world_mut().resource_mut::<GameInput>().fire = true;
    for _ in 0..2 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Paused
    );
    assert_eq!(app.world().resource::<GameInput>().movement, Vec2::ZERO);
    assert!(!app.world().resource::<GameInput>().fire);
}

#[test]
fn missing_configuration_returns_actionable_failure() {
    let missing = std::env::temp_dir().join("origins-no-such-config.toml");
    assert!(
        origins::assets::LocalConfig::discover(Some(&missing))
            .err()
            .unwrap()
            .contains("configuration")
    );
}
