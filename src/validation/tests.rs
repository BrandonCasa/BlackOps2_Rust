use super::*;
use avian3d::prelude::*;

#[derive(Resource, Default)]
struct Trace(Vec<(bool, bool, bool, Vec2, f32)>);
fn sample_tick(
    mut trace: ResMut<Trace>,
    mut input: ResMut<crate::input::GameInput>,
    players: Query<&crate::player::Player>,
) {
    let yaw = players.single().map_or(0., |p| p.yaw);
    trace
        .0
        .push((input.fire, input.fire_held, input.jump, input.movement, yaw));
    input.take_edges();
}
pub(crate) fn harness() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::state::app::StatesPlugin,
        bevy::input::InputPlugin,
        TransformPlugin,
        AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
        PhysicsPlugins::new(PostUpdate),
    ))
    .init_state::<GameState>()
    .insert_state(GameState::Playing)
    .insert_resource(Time::<Fixed>::from_hz(60.))
    .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::ZERO,
    ))
    .insert_resource(SessionId(1))
    .init_resource::<Session>()
    .init_resource::<crate::input::Actions>()
    .init_resource::<crate::input::ControlSettings>()
    .init_resource::<crate::input::GameInput>()
    .init_resource::<crate::input::menu::ControlMenu>()
    .init_resource::<crate::player::TickActions>()
    .init_resource::<crate::interactions::InteractionFocus>()
    .init_resource::<crate::damage::DamageQueue>()
    .add_message::<crate::damage::DamageApplied>()
    .add_message::<crate::damage::EnemyDeath>()
    .add_message::<crate::movement::Landing>()
    .add_message::<crate::movement::Jump>()
    .init_resource::<Trace>()
    .add_systems(
        PreUpdate,
        crate::input::collect.after(bevy::input::InputSystems),
    )
    .add_systems(
        FixedUpdate,
        (sample_tick, crate::input::finish_tick).chain(),
    )
    .add_systems(
        OnEnter(GameState::Paused),
        |mut time: ResMut<Time<Virtual>>| time.pause(),
    )
    .add_systems(
        OnExit(GameState::Paused),
        |mut time: ResMut<Time<Virtual>>| time.unpause(),
    );
    let mut controller = Controller::new(
        2818,
        std::env::temp_dir().join(format!("origins-validator-test-{}", std::process::id())),
    );
    std::fs::create_dir_all(&controller.report).unwrap();
    controller.session = Some(1);
    install_controller(&mut app, controller);
    app.world_mut().spawn((
        Window {
            focused: true,
            ..default()
        },
        bevy::window::PrimaryWindow,
    ));
    app.finish();
    app.cleanup();
    app.update();
    app
}
fn input(app: &mut App, id: u64, ticks: u32, held: Vec<crate::input::Action>) {
    submit(
        app.world_mut(),
        Request {
            session: 1,
            id,
            action: Command::Input {
                ticks,
                held,
                look: [0., 0.],
                aim: None,
                aim_entity: None,
                aim_offset: None,
                arrival_point: None,
                arrival_entity: None,
                arrival_radius: None,
            },
            label: String::new(),
        },
    )
    .unwrap();
}
fn command(app: &mut App, id: u64, action: Command) {
    submit(
        app.world_mut(),
        Request {
            session: 1,
            id,
            action,
            label: String::new(),
        },
    )
    .unwrap();
}
fn frame(app: &mut App, ticks: u32) {
    let duration = app.world().resource::<Time<Fixed>>().timestep() * ticks;
    *app.world_mut()
        .resource_mut::<bevy::time::TimeUpdateStrategy>() =
        bevy::time::TimeUpdateStrategy::ManualDuration(duration);
    app.update();
}

#[test]
fn jump_evidence_requires_native_takeoff_and_keeps_the_previous_platform() {
    let mut app = harness();
    let platform = app.world_mut().spawn_empty().id();
    let mut support = crate::movement::platforms::PlatformSupport::default();
    support.entity = Some(platform);
    let player = app
        .world_mut()
        .spawn((
            crate::player::Player {
                yaw: 0.,
                pitch: 0.,
                velocity: Vec3::Y,
                height: 1.8,
                grounded: false,
                crouched: false,
                prone: false,
                eye_height: 1.62,
            },
            Transform::default(),
            support,
        ))
        .id();
    frame(&mut app, 1);
    assert!(
        !app.world()
            .resource::<Controller>()
            .evidence
            .events
            .iter()
            .any(|event| event["kind"] == "jump"),
        "An upward step or platform carry is not a native jump"
    );
    app.world_mut()
        .get_mut::<crate::movement::platforms::PlatformSupport>(player)
        .unwrap()
        .entity = None;
    app.world_mut().write_message(crate::movement::Jump {
        owner: player,
        position: Vec3::new(1., 2., 3.),
        velocity: Vec3::new(0., 6.3, 0.),
    });
    frame(&mut app, 1);
    let evidence = &app.world().resource::<Controller>().evidence;
    let jumps: Vec<_> = evidence
        .events
        .iter()
        .filter(|event| event["kind"] == "jump")
        .collect();
    assert_eq!(jumps.len(), 1);
    assert_eq!(jumps[0]["data"]["platform"], platform.to_bits());
    assert_eq!(jumps[0]["data"]["position"], json!([1., 2., 3.]));
    frame(&mut app, 2);
    assert_eq!(
        app.world()
            .resource::<Controller>()
            .evidence
            .events
            .iter()
            .filter(|event| event["kind"] == "jump")
            .count(),
        1
    );
}

#[test]
fn missed_waypoint_stops_a_queued_jump_before_the_next_fixed_tick() {
    let mut app = harness();
    command(
        &mut app,
        1,
        Command::Input {
            ticks: 2,
            held: vec![],
            look: [0., 0.],
            aim: Some([10., 0., 0.]),
            aim_entity: None,
            aim_offset: None,
            arrival_point: None,
            arrival_entity: None,
            arrival_radius: Some(0.2),
        },
    );
    input(
        &mut app,
        2,
        10,
        vec![crate::input::Action::Forward, crate::input::Action::Jump],
    );
    frame(&mut app, 4);
    let controller = app.world().resource::<Controller>();
    assert_eq!(
        controller.failure.as_deref(),
        Some("Input arrival waypoint deadline expired")
    );
    assert!(controller.paused && controller.active.is_none() && controller.queue.is_empty());
    assert!(
        !app.world()
            .resource::<Trace>()
            .0
            .iter()
            .any(|tick| tick.2 || tick.3 != Vec2::ZERO)
    );
}

#[test]
fn zero_fixed_ticks_preserve_input_and_multiple_ticks_emit_one_edge() {
    let mut app = harness();
    input(
        &mut app,
        1,
        3,
        vec![crate::input::Action::Fire, crate::input::Action::Jump],
    );
    frame(&mut app, 0);
    assert!(app.world().resource::<Trace>().0.is_empty());
    assert_eq!(app.world().resource::<Controller>().queue.len(), 1);
    frame(&mut app, 3);
    let trace = &app.world().resource::<Trace>().0;
    assert_eq!(trace.len(), 3);
    assert_eq!(trace.iter().filter(|t| t.0).count(), 1);
    assert_eq!(trace.iter().filter(|t| t.2).count(), 1);
    assert!(trace.iter().all(|t| t.1));
    assert_eq!(
        app.world().resource::<Controller>().receipts[&(1, 1)].phase,
        Phase::Executed
    );
}
#[test]
fn batches_cross_a_multi_tick_frame_without_repeating_or_underflowing() {
    let mut app = harness();
    let requests = json!([
        {"session":1,"id":1,"action":{"op":"input","ticks":1,"held":["fire"]}},
        {"session":1,"id":2,"action":{"op":"input","ticks":1,"held":[]}},
        {"session":1,"id":3,"action":{"op":"input","ticks":1,"held":["fire"]}}
    ]);
    remote_input_batch(In(Some(requests.clone())), app.world_mut()).unwrap();
    remote_input_batch(In(Some(requests)), app.world_mut()).unwrap();
    assert_eq!(app.world().resource::<Controller>().queue.len(), 3);
    frame(&mut app, 3);
    assert_eq!(
        app.world()
            .resource::<Trace>()
            .0
            .iter()
            .map(|t| t.0)
            .collect::<Vec<_>>(),
        vec![true, false, true]
    );
    assert!([1, 2, 3].iter().all(
        |id| app.world().resource::<Controller>().receipts[&(1, *id)].phase == Phase::Executed
    ));
    let invalid = json!([
        {"session":1,"id":4,"action":{"op":"input","ticks":1,"held":["fire"]}},
        {"session":1,"id":5,"action":{"op":"input","ticks":0,"held":[]}}
    ]);
    assert!(remote_input_batch(In(Some(invalid)), app.world_mut()).is_err());
    assert!(app.world().resource::<Controller>().queue.is_empty());
    assert!(
        !app.world()
            .resource::<Controller>()
            .receipts
            .contains_key(&(1, 4))
    );
}
#[test]
fn paused_step_advances_one_fixed_tick_and_post_update_physics() {
    let mut app = harness();
    assert_physics_step(&mut app);
}
pub(crate) fn assert_physics_step(app: &mut App) {
    command(app, 1, Command::Pause);
    frame(app, 3);
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Paused
    );
    let physics_before = app.world().resource::<Time<Physics>>().elapsed();
    let real_before = app.world().resource::<Time<Real>>().elapsed();
    command(app, 2, Command::Step);
    frame(app, 0);
    assert_eq!(app.world().resource::<Trace>().0.len(), 1);
    assert_eq!(
        app.world().resource::<Time<Physics>>().elapsed() - physics_before,
        app.world().resource::<Time<Fixed>>().timestep()
    );
    assert_eq!(app.world().resource::<Time<Real>>().elapsed(), real_before);
    frame(app, 3);
    assert_eq!(app.world().resource::<Trace>().0.len(), 1);
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Paused
    );
    assert_eq!(
        app.world().resource::<Controller>().receipts[&(1, 2)].phase,
        Phase::Executed
    );
}
#[test]
fn pause_resume_restores_hold_without_repeating_a_fire_edge() {
    let mut app = harness();
    input(&mut app, 1, 5, vec![crate::input::Action::Fire]);
    frame(&mut app, 1);
    command(&mut app, 2, Command::Pause);
    frame(&mut app, 1);
    assert!(!app.world().resource::<crate::input::GameInput>().fire_held);
    command(&mut app, 3, Command::Resume);
    frame(&mut app, 0);
    frame(&mut app, 1);
    let trace = &app.world().resource::<Trace>().0;
    assert_eq!(trace.len(), 2);
    assert!(trace[1].1);
    assert!(!trace[1].0);
}
#[test]
fn focus_loss_pauses_before_consuming_input_and_physical_input_cannot_move() {
    let mut app = harness();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyW);
    input(&mut app, 1, 2, vec![]);
    frame(&mut app, 1);
    assert_eq!(app.world().resource::<Trace>().0[0].3, Vec2::ZERO);
    let window = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .find(|e| app.world().get::<Window>(*e).is_some())
        .unwrap();
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    frame(&mut app, 1);
    assert_eq!(app.world().resource::<Trace>().0.len(), 1);
    assert_eq!(
        app.world()
            .resource::<Controller>()
            .active
            .as_ref()
            .unwrap()
            .1,
        1
    );
    assert!(app.world().resource::<Controller>().paused);
}
#[test]
fn cancelled_hold_releases_controls_and_never_executes_remaining_ticks() {
    let mut app = harness();
    input(&mut app, 1, 60, vec![crate::input::Action::Fire]);
    frame(&mut app, 1);
    command(&mut app, 2, Command::Cancel { command: 1 });
    frame(&mut app, 2);
    assert_eq!(
        app.world().resource::<Controller>().receipts[&(1, 1)].phase,
        Phase::Cancelled
    );
    assert!(!app.world().resource::<crate::input::GameInput>().fire_held);
    assert!(
        app.world()
            .resource::<Trace>()
            .0
            .iter()
            .skip(1)
            .all(|t| !t.0 && !t.1)
    );
}
#[test]
fn expired_real_time_watchdog_recovers_without_a_runner_even_while_paused() {
    let mut app = harness();
    input(
        &mut app,
        1,
        120,
        vec![crate::input::Action::Fire, crate::input::Action::Forward],
    );
    frame(&mut app, 1);
    app.world_mut().resource_mut::<Controller>().heartbeat =
        Instant::now() - Duration::from_secs(6);
    frame(&mut app, 2);
    let c = app.world().resource::<Controller>();
    assert!(c.failure.as_ref().unwrap().contains("heartbeat"));
    assert!(c.paused);
    assert!(c.active.is_none());
    assert!(c.queue.is_empty());
    assert!(c.report.join("game-failure.json").exists());
    assert_eq!(app.world().resource::<Trace>().0.len(), 1);
    assert!(!app.world().resource::<crate::input::GameInput>().fire_held);
}

#[test]
fn aiming_tracks_the_animated_torso_without_new_network_inputs() {
    let mut app = harness();
    let game: crate::assets::GameDefinition = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("gameplay/origins/game.json"),
        )
        .unwrap(),
    )
    .unwrap();
    app.insert_resource(game);
    let player = app
        .world_mut()
        .spawn((
            crate::player::Player {
                yaw: 0.,
                pitch: 0.,
                velocity: Vec3::ZERO,
                height: 1.8,
                grounded: true,
                crouched: false,
                prone: false,
                eye_height: 1.62,
            },
            Transform::from_xyz(0., 0.9, 0.),
            crate::weapons::WeaponInventory::default(),
        ))
        .id();
    let target = app
        .world_mut()
        .spawn((
            Transform::from_xyz(0., 1., -5.),
            crate::hit_regions::Hitboxes::humanoid(1.8, 0.3),
        ))
        .id();
    command(
        &mut app,
        1,
        Command::Input {
            ticks: 30,
            held: vec![],
            look: [0., 0.],
            aim: None,
            aim_entity: Some(target.to_bits()),
            aim_offset: None,
            arrival_point: None,
            arrival_entity: None,
            arrival_radius: None,
        },
    );
    frame(&mut app, 6);
    let expected = ((1.072_f32 - 1.62) / 5.).atan();
    assert!(
        (app.world()
            .get::<crate::player::Player>(player)
            .unwrap()
            .pitch
            - expected)
            .abs()
            < 1e-6
    );
    {
        let mut boxes = app
            .world_mut()
            .get_mut::<crate::hit_regions::Hitboxes>(target)
            .unwrap();
        let torso = boxes
            .volumes
            .iter_mut()
            .find(|v| v.region == crate::weapons::HitRegion::Torso)
            .unwrap();
        torso.start.y += 1.;
        torso.end.y += 1.;
    }
    app.world_mut()
        .get_mut::<crate::weapons::WeaponInventory>(player)
        .unwrap()
        .recoil = Vec2::new(0.05, 0.1);
    app.world_mut()
        .get_mut::<crate::movement::MotionFeedback>(player)
        .unwrap()
        .view_angles = Vec2::new(1., 2.);
    frame(&mut app, 12);
    let expected = ((2.072_f32 - 1.62) / 5.).atan() - 0.1 + 1_f32.to_radians();
    assert!(
        (app.world()
            .get::<crate::player::Player>(player)
            .unwrap()
            .pitch
            - expected)
            .abs()
            < 1e-6
    );
    assert!(
        (app.world()
            .get::<crate::player::Player>(player)
            .unwrap()
            .yaw
            + 0.05
            + 2_f32.to_radians())
        .abs()
            < 1e-6
    );
}

#[test]
fn scripted_look_runs_once_per_fixed_tick_before_movement_and_never_in_zero_tick_frames() {
    let mut app = harness();
    let game: crate::assets::GameDefinition = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("gameplay/origins/game.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let sensitivity = game.tuning.mouse_sensitivity;
    app.insert_resource(game);
    let player = app
        .world_mut()
        .spawn((
            crate::player::Player {
                yaw: 0.,
                pitch: 0.,
                velocity: Vec3::ZERO,
                height: 1.8,
                grounded: true,
                crouched: false,
                prone: false,
                eye_height: 1.62,
            },
            Transform::default(),
            crate::weapons::WeaponInventory::default(),
        ))
        .id();
    let mut request = Request {
        session: 1,
        id: 1,
        action: Command::Input {
            ticks: 2,
            held: vec![],
            look: [6., 0.],
            aim: None,
            aim_entity: None,
            aim_offset: None,
            arrival_point: None,
            arrival_entity: None,
            arrival_radius: None,
        },
        label: String::new(),
    };
    submit(app.world_mut(), request.clone()).unwrap();
    frame(&mut app, 0);
    assert_eq!(
        app.world()
            .get::<crate::player::Player>(player)
            .unwrap()
            .yaw,
        0.
    );
    frame(&mut app, 2);
    let trace = &app.world().resource::<Trace>().0;
    assert!(
        (trace[0].4 + 6. * sensitivity).abs() < 1e-7,
        "trace={trace:?}; sensitivity={sensitivity}"
    );
    assert!((trace[1].4 + 12. * sensitivity).abs() < 1e-7);
    request.id = 2;
    assert!(submit(app.world_mut(), request).is_ok());
}

#[test]
fn command_retry_has_one_queue_entry_and_conflicting_retry_fails() {
    let mut app = App::new();
    app.insert_resource(Controller::new(2818, std::env::temp_dir()))
        .insert_resource(SessionId(4));
    let request = Request {
        session: 4,
        id: 1,
        action: Command::Input {
            ticks: 1,
            held: vec![],
            look: [0., 0.],
            aim: None,
            aim_entity: None,
            aim_offset: None,
            arrival_point: None,
            arrival_entity: None,
            arrival_radius: None,
        },
        label: String::new(),
    };
    // Receipt matching precedes all world reads and cannot execute a retry twice.
    let c = app.world().resource::<Controller>();
    let receipt = Receipt {
        request: request.clone(),
        phase: Phase::Accepted,
        accepted_tick: c.tick,
        executed_tick: None,
        resulting_session: None,
        error: None,
        after: Value::Null,
    };
    app.world_mut()
        .resource_mut::<Controller>()
        .receipts
        .insert((4, 1), receipt);
    assert!(submit(app.world_mut(), request.clone()).is_ok());
    assert!(app.world().resource::<Controller>().queue.is_empty());
    let mut conflicting = request;
    conflicting.label = "different".into();
    assert!(
        submit(app.world_mut(), conflicting)
            .unwrap_err()
            .contains("reused")
    );
}

#[test]
fn waypoint_arrival_releases_forward_without_waiting_for_the_runner() {
    let mut app = harness();
    let player = app
        .world_mut()
        .spawn((
            crate::player::Player {
                yaw: 0.,
                pitch: 0.,
                velocity: Vec3::ZERO,
                height: 1.8,
                grounded: true,
                crouched: false,
                prone: false,
                eye_height: 1.62,
            },
            Transform::default(),
        ))
        .id();
    let action = Command::Input {
        ticks: 1800,
        held: vec![crate::input::Action::Forward],
        look: [0., 0.],
        aim: Some([10., 1.62, 0.]),
        aim_entity: None,
        aim_offset: None,
        arrival_point: None,
        arrival_entity: None,
        arrival_radius: Some(0.25),
    };
    assert!(action.validate().is_ok());
    command(&mut app, 1, action);
    frame(&mut app, 1);
    assert_ne!(app.world().resource::<Trace>().0[0].3, Vec2::ZERO);
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation
        .x = 9.8;
    frame(&mut app, 3);
    assert!(
        app.world().resource::<Trace>().0[1..]
            .iter()
            .all(|tick| tick.3 == Vec2::ZERO)
    );
    assert_eq!(
        app.world().resource::<Controller>().receipts[&(1, 1)].phase,
        Phase::Executed
    );
    assert!(app.world().resource::<Controller>().active.is_none());
    let platform = app
        .world_mut()
        .spawn(
            Transform::from_xyz(10., 0., 4.)
                .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        )
        .id();
    command(
        &mut app,
        2,
        Command::Input {
            ticks: 1800,
            held: vec![crate::input::Action::Forward],
            look: [0., 0.],
            aim: None,
            aim_entity: Some(platform.to_bits()),
            aim_offset: Some([4., 1.62, 0.]),
            arrival_point: None,
            arrival_entity: None,
            arrival_radius: Some(0.25),
        },
    );
    app.world_mut()
        .get_mut::<Transform>(platform)
        .unwrap()
        .translation
        .z += 5.;
    frame(&mut app, 1);
    assert_ne!(
        app.world().resource::<Trace>().0.last().unwrap().3,
        Vec2::ZERO
    );
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation = Vec3::new(10., 0., 5.);
    frame(&mut app, 3);
    assert_eq!(
        app.world().resource::<Controller>().receipts[&(1, 2)].phase,
        Phase::Executed
    );
    command(
        &mut app,
        3,
        Command::Input {
            ticks: 1800,
            held: vec![crate::input::Action::Forward],
            look: [0., 0.],
            aim: Some([100., 1.62, 0.]),
            aim_entity: None,
            aim_offset: None,
            arrival_point: Some([10., 0., 5.]),
            arrival_entity: None,
            arrival_radius: Some(0.25),
        },
    );
    frame(&mut app, 3);
    assert_eq!(
        app.world().resource::<Controller>().receipts[&(1, 3)].phase,
        Phase::Executed,
        "Riding to a world waypoint must complete independently of the gaze target"
    );
    assert_eq!(
        app.world().resource::<Trace>().0.last().unwrap().3,
        Vec2::ZERO
    );
    app.world_mut()
        .get_mut::<Transform>(player)
        .unwrap()
        .translation = Vec3::new(20., 0., 5.);
    command(
        &mut app,
        4,
        Command::Input {
            ticks: 1800,
            held: vec![],
            look: [0., 0.],
            aim: Some([100., 1.62, 0.]),
            aim_entity: None,
            aim_offset: None,
            arrival_point: Some([20., 0., 5.]),
            arrival_entity: Some(platform.to_bits()),
            arrival_radius: Some(0.25),
        },
    );
    frame(&mut app, 1);
    assert!(
        app.world().resource::<Controller>().active.is_some(),
        "The rider reaching a point cannot complete a wait on the vehicle"
    );
    app.world_mut()
        .get_mut::<Transform>(platform)
        .unwrap()
        .translation = Vec3::new(20., 0., 5.);
    frame(&mut app, 2);
    assert_eq!(
        app.world().resource::<Controller>().receipts[&(1, 4)].phase,
        Phase::Executed
    );
    assert!(
        Command::Input {
            ticks: 1,
            held: vec![],
            look: [0., 0.],
            aim: None,
            aim_entity: None,
            aim_offset: None,
            arrival_point: None,
            arrival_entity: None,
            arrival_radius: Some(0.25)
        }
        .validate()
        .is_err()
    );
}

#[test]
fn stale_commands_are_rejected_before_world_access() {
    let mut app = App::new();
    app.insert_resource(Controller::new(2818, std::env::temp_dir()))
        .insert_resource(SessionId(2));
    let request = Request {
        session: 1,
        id: 1,
        action: Command::Pause,
        label: String::new(),
    };
    assert!(
        submit(app.world_mut(), request)
            .unwrap_err()
            .contains("Stale")
    );
}

#[test]
fn completed_checkpoint_releases_lease_without_triggering_a_dead_runner_failure() {
    let mut app = harness();
    command(&mut app, 1, Command::Finish);
    frame(&mut app, 1);
    assert!(!app.world().resource::<Controller>().owns_input());
    assert!(app.world().resource::<Controller>().paused);
    app.world_mut().resource_mut::<Controller>().heartbeat =
        Instant::now() - Duration::from_secs(10);
    frame(&mut app, 2);
    assert!(app.world().resource::<Controller>().failure.is_none());
}

#[test]
fn automatic_session_initialization_claims_input_and_seeds_before_world_setup() {
    let mut app = harness();
    app.world_mut().resource_mut::<Controller>().session = None;
    let mut session = Session::default();
    initialize(app.world_mut(), &mut session);
    assert_eq!(session.seed, 2818);
    assert_eq!(app.world().resource::<Controller>().session, Some(1));
    assert!(app.world().resource::<Controller>().queue.is_empty());
}

#[test]
fn startup_observation_does_not_require_the_unloaded_game_definition() {
    let mut app = harness();
    app.insert_state(GameState::Loading);
    let snapshot = state::snapshot(app.world_mut());
    assert_eq!(snapshot["game_state"], "Loading");
    assert_eq!(snapshot["ready"], false);
    assert!(snapshot["targets"].as_array().unwrap().is_empty());
}
