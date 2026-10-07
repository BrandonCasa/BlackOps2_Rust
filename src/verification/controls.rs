//! Controlled physical-input workload in the actual game and Bevy UI.
use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    gameplay::{Session, Zombie},
    input::{
        Action, Actions, Binding, ControlSettings, GameInput, PressMode,
        menu::{ControlMenu, MenuRow},
    },
    player::Player,
};
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
        mouse::{MouseButtonInput, MouseMotion},
    },
    math::DVec2,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::CursorGrabMode,
};
use std::{path::PathBuf, time::Instant};

#[derive(Resource)]
struct Run {
    output: PathBuf,
    started: Instant,
    changed: Instant,
    phase: u32,
    frame: u64,
    changed_frame: u64,
    rows: Vec<serde_json::Value>,
    initial_settings: ControlSettings,
    loaded: bool,
    startup_error: Option<String>,
    startup_audio: Option<crate::audio::AudioSettings>,
    position: Vec3,
    yaw: f32,
    pitch: f32,
    shots: u32,
    accepted: u32,
    tap_ticks: u32,
    tap_frames: u32,
    focus_shots: u32,
    pause_frames: Vec<serde_json::Value>,
}
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(First, drive)
        .add_systems(PostUpdate, pause_trace)
        .add_systems(PreUpdate, pending.after(crate::input::collect))
        .add_systems(FixedUpdate, tick.before(crate::player::move_player));
}
fn setup(
    mut commands: Commands,
    options: Res<LaunchOptions>,
    settings: Res<ControlSettings>,
    store: Res<crate::input::settings::SettingsStore>,
) {
    let output = options.verify.as_ref().unwrap();
    if output.starts_with(env!("CARGO_MANIFEST_DIR")) || std::fs::create_dir_all(output).is_err() {
        commands.write_message(AppExit::error());
        return;
    }
    commands.insert_resource(Run {
        output: output.clone(),
        started: Instant::now(),
        changed: Instant::now(),
        phase: 0,
        frame: 0,
        changed_frame: 0,
        rows: vec![],
        initial_settings: settings.clone(),
        loaded: store.loaded,
        startup_error: store.error.clone(),
        startup_audio: None,
        position: Vec3::ZERO,
        yaw: 0.,
        pitch: 0.,
        shots: 0,
        accepted: 0,
        tap_ticks: 0,
        tap_frames: 0,
        focus_shots: 0,
        pause_frames: vec![],
    });
}
fn pause_trace(
    run: Option<ResMut<Run>>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    next: Res<NextState<GameState>>,
    windows: Query<&Window>,
) {
    if let Some(mut run) = run
        && run.phase == 2
        && run.pause_frames.len() < 8
    {
        run.pause_frames.push(serde_json::json!({"escape_pressed":keys.just_pressed(KeyCode::Escape),"state":format!("{:?}",**state),"next":format!("{:?}",*next),"focused":windows.iter().next().map(|w|w.focused)}));
    }
}
fn pending(run: Option<ResMut<Run>>, actions: Res<Actions>) {
    if let Some(mut run) = run
        && run.phase == 28
    {
        let state = actions.state(Action::Fire);
        if state.pressed && state.released && !state.held {
            run.tap_frames += 1;
        }
    }
}
fn tick(run: Option<ResMut<Run>>, actions: Res<Actions>, input: Res<GameInput>) {
    if let Some(mut run) = run
        && run.phase == 28
    {
        let state = actions.state(Action::Fire);
        if state.pressed && state.released && input.fire && input.fire_held {
            run.tap_ticks += 1;
        }
    }
}
fn key(world: &mut World, window: Entity, key_code: KeyCode, down: bool) {
    world.write_message(KeyboardInput {
        key_code,
        logical_key: Key::Character("".into()),
        state: if down {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
        text: None,
        repeat: false,
        window,
    });
}
fn tap(world: &mut World, window: Entity, code: KeyCode) {
    key(world, window, code, true);
    key(world, window, code, false);
}
fn mouse(world: &mut World, window: Entity, button: MouseButton, down: bool) {
    world.write_message(MouseButtonInput {
        button,
        state: if down {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        },
        window,
    });
}
fn click(world: &mut World, window: Entity, button: MouseButton) {
    mouse(world, window, button, true);
    mouse(world, window, button, false);
}
fn point(world: &mut World, window: Entity, row: usize) -> bool {
    let center = world
        .query::<(&MenuRow, &UiGlobalTransform, &ComputedNode)>()
        .iter(world)
        .find(|(r, _, node)| r.index == row && node.size().length_squared() > 0.)
        .map(|(_, transform, _)| transform.to_scale_angle_translation().2);
    if let Some(center) = center {
        world
            .get_mut::<Window>(window)
            .unwrap()
            .set_physical_cursor_position(Some(center.as_dvec2()));
        true
    } else {
        false
    }
}
fn click_row(world: &mut World, window: Entity, row: usize) -> bool {
    if !point(world, window, row) {
        return false;
    }
    click(world, window, MouseButton::Left);
    true
}
impl Run {
    fn record(&mut self, name: &str, passed: bool, detail: serde_json::Value) {
        self.rows
            .push(serde_json::json!({"case":name,"passed":passed,"detail":detail}));
        if !passed {
            self.phase = 99;
        }
    }
}
fn picture(world: &mut World, run: &Run, name: &str) {
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(run.output.join(name)));
}
fn advance(run: &mut Run) {
    if run.phase != 99 {
        run.phase += 1;
        run.changed = Instant::now();
        run.changed_frame = run.frame;
    }
}
fn go(run: &mut Run, phase: u32) {
    if run.phase != 99 {
        run.phase = phase;
        run.changed = Instant::now();
        run.changed_frame = run.frame;
    }
}

fn drive(world: &mut World) {
    let Some(mut run) = world.remove_resource::<Run>() else {
        return;
    };
    run.frame += 1;
    let state = **world.resource::<State<GameState>>();
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .iter(world)
        .next();
    let owner = world
        .query_filtered::<Entity, With<Player>>()
        .iter(world)
        .next();
    if state == GameState::LoadError || run.started.elapsed().as_secs() > 180 {
        run.phase = 99;
    }
    let (Some(window), Some(owner)) = (window, owner) else {
        world.insert_resource(run);
        return;
    };
    if run.phase == 0 && (state != GameState::Playing || run.changed.elapsed().as_secs_f32() < 3.) {
        world.insert_resource(run);
        return;
    }
    let wait = match run.phase {
        19 | 20 | 21 => 0.7,
        28 => 0.85,
        31 => 3.,
        _ => 0.3,
    };
    if run.phase > 0
        && (run.changed.elapsed().as_secs_f32() < wait || run.frame - run.changed_frame < 2)
    {
        world.insert_resource(run);
        return;
    }
    match run.phase {
        0 => {
            run.startup_audio = Some(world.resource::<crate::audio::AudioSettings>().clone());
            if run.loaded {
                run.record(
                    "startup restores persisted controls and audio",
                    *world.resource::<ControlSettings>() == run.initial_settings
                        && run.startup_audio.as_ref() == Some(&run.initial_settings.audio),
                    serde_json::json!({"audio":run.startup_audio}),
                );
            } else if run.startup_error.is_some() {
                run.record(
                    "invalid settings preserve safe defaults and report recovery",
                    *world.resource::<ControlSettings>() == ControlSettings::default()
                        && std::fs::read_to_string(run.output.join("settings.json"))
                            .is_ok_and(|s| s.trim() == "{"),
                    serde_json::json!({"error":run.startup_error}),
                );
            }
            let enemies = world
                .query_filtered::<Entity, With<Zombie>>()
                .iter(world)
                .collect::<Vec<_>>();
            for enemy in enemies {
                world.despawn(enemy);
            }
            let mut session = world.resource_mut::<Session>();
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(600., TimerMode::Once));
            *world.resource_mut::<ControlSettings>() = ControlSettings::default();
            *world.resource_mut::<crate::audio::AudioSettings>() =
                crate::audio::AudioSettings::default();
            world.get_mut::<Window>(window).unwrap().focused = true;
            advance(&mut run);
        }
        1 => {
            if state != GameState::Playing {
                world.get_mut::<Window>(window).unwrap().focused = true;
                world
                    .resource_mut::<NextState<GameState>>()
                    .set(GameState::Playing);
                run.changed = Instant::now();
                run.changed_frame = run.frame;
                world.insert_resource(run);
                return;
            }
            tap(world, window, KeyCode::Escape);
            advance(&mut run);
        }
        2 => {
            let cursor = world.get::<bevy::window::CursorOptions>(window).unwrap();
            run.record(
                "pause releases cursor",
                state == GameState::Paused
                    && cursor.grab_mode == CursorGrabMode::None
                    && cursor.visible,
                serde_json::json!({"state":format!("{state:?}"),"trace":run.pause_frames}),
            );
            world
                .get_mut::<Window>(window)
                .unwrap()
                .set_physical_cursor_position(Some(DVec2::new(1000., 900.)));
            tap(world, window, KeyCode::ArrowDown);
            advance(&mut run);
        }
        3 => {
            tap(world, window, KeyCode::Enter);
            advance(&mut run);
        }
        4 => {
            let page = world.resource::<ControlMenu>().page;
            run.record(
                "keyboard opens controls",
                page == Some(0),
                serde_json::json!({"page":page}),
            );
            picture(world, &run, "controls-default.png");
            if run.phase != 99 && click_row(world, window, 2) {
                advance(&mut run);
            }
        }
        5 => {
            let selected = world.resource::<ControlMenu>().selected;
            let inverted = world.resource::<ControlSettings>().invert_mouse_y;
            run.record(
                "mouse focuses and activates setting",
                selected == 2 && inverted,
                serde_json::json!({"selected":selected,"inverted":inverted}),
            );
            if run.phase != 99 && click_row(world, window, 6) {
                advance(&mut run);
            }
        }
        6 => {
            tap(world, window, KeyCode::KeyI);
            advance(&mut run);
        }
        7 => {
            let settings = world.resource::<ControlSettings>();
            run.record(
                "keyboard remapping",
                settings.bindings[&Action::Forward] == [Binding::Key(KeyCode::KeyI)],
                serde_json::json!({"forward":settings.label(Action::Forward)}),
            );
            if run.phase != 99 && click_row(world, window, 7) {
                advance(&mut run);
            }
        }
        8 => {
            click(world, window, MouseButton::Back);
            advance(&mut run);
        }
        9 => {
            let settings = world.resource::<ControlSettings>();
            run.record(
                "mouse button remapping",
                settings.bindings[&Action::Backward] == [Binding::Mouse(MouseButton::Back)],
                serde_json::json!({"backward":settings.label(Action::Backward)}),
            );
            if run.phase != 99 && point(world, window, 0) {
                advance(&mut run);
            }
        }
        10 => {
            tap(world, window, KeyCode::ArrowRight);
            advance(&mut run);
        }
        11 => {
            tap(world, window, KeyCode::ArrowDown);
            tap(world, window, KeyCode::ArrowRight);
            advance(&mut run);
        }
        12 => {
            let settings = world.resource::<ControlSettings>();
            run.record(
                "keyboard adjusts both sensitivities",
                (settings.sensitivity - 1.1).abs() < 0.001
                    && (settings.ads_sensitivity - 1.1).abs() < 0.001,
                serde_json::json!({"hip":settings.sensitivity,"ads":settings.ads_sensitivity}),
            );
            if run.phase != 99 && click_row(world, window, 3) {
                key(world, window, KeyCode::ShiftLeft, true);
                tap(world, window, KeyCode::F7);
                advance(&mut run);
            }
        }
        13 => {
            key(world, window, KeyCode::ShiftLeft, false);
            if click_row(world, window, 4) {
                advance(&mut run);
            }
        }
        14 => {
            if click_row(world, window, 5) {
                advance(&mut run);
            }
        }
        15 => {
            let settings = world.resource::<ControlSettings>();
            run.record(
                "three toggle settings",
                settings.aim_mode == PressMode::Toggle
                    && settings.crouch_mode == PressMode::Toggle
                    && settings.sprint_mode == PressMode::Toggle,
                serde_json::to_value(settings).unwrap(),
            );
            picture(world, &run, "controls-remapped.png");
            tap(world, window, KeyCode::Escape);
            advance(&mut run);
        }
        16 => {
            let cursor = world.get::<bevy::window::CursorOptions>(window).unwrap();
            run.record(
                "resume restores cursor capture",
                state == GameState::Playing
                    && cursor.grab_mode == CursorGrabMode::Locked
                    && !cursor.visible,
                serde_json::json!({"state":format!("{state:?}")}),
            );
            let settings = world.resource::<ControlSettings>();
            let saved = crate::input::settings::read(&run.output.join("settings.json"));
            run.record(
                "saved bindings and modes",
                saved.as_ref().is_ok_and(|s| s == settings),
                serde_json::json!({"read_ok":saved.is_ok()}),
            );
            run.record(
                "existing audio controls persist",
                (settings.audio.music - 0.6).abs() < 0.001
                    && (world.resource::<crate::audio::AudioSettings>().music - 0.6).abs() < 0.001
                    && saved
                        .as_ref()
                        .is_ok_and(|s| (s.audio.music - 0.6).abs() < 0.001),
                serde_json::json!({"music":settings.audio.music}),
            );
            run.position = world.get::<Transform>(owner).unwrap().translation;
            run.yaw = world.get::<Player>(owner).unwrap().yaw;
            run.pitch = world.get::<Player>(owner).unwrap().pitch;
            key(world, window, KeyCode::KeyI, true);
            world.write_message(MouseMotion {
                delta: Vec2::new(12., 8.),
            });
            advance(&mut run);
        }
        17 => {
            let player = world.get::<Player>(owner).unwrap();
            let moved = world
                .get::<Transform>(owner)
                .unwrap()
                .translation
                .distance(run.position);
            let tuning = &world.resource::<GameDefinition>().tuning;
            let expected = tuning.mouse_sensitivity * 1.1;
            run.record("remapped movement and inverted scaled look",
                moved > 0.1
                    && (player.yaw - run.yaw + 12. * expected).abs() < 0.001
                    && (player.pitch - run.pitch - 8. * expected).abs() < 0.001,
                serde_json::json!({"meters":moved,"yaw_delta":player.yaw-run.yaw,"pitch_delta":player.pitch-run.pitch}),
            );
            key(world, window, KeyCode::KeyI, false);
            tap(world, window, KeyCode::ShiftLeft);
            advance(&mut run);
        }
        18 => {
            let sprint = world.resource::<GameInput>().sprint;
            run.record(
                "sprint toggle survives release",
                sprint,
                serde_json::json!({"sprint":sprint}),
            );
            tap(world, window, KeyCode::ControlLeft);
            advance(&mut run);
        }
        19 => {
            let crouched = world.get::<Player>(owner).unwrap().crouched;
            run.record(
                "crouch toggle changes live capsule",
                crouched,
                serde_json::json!({"crouched":crouched}),
            );
            tap(world, window, KeyCode::ControlLeft);
            click(world, window, MouseButton::Right);
            advance(&mut run);
        }
        20 => {
            let ads = world
                .get::<crate::weapons::WeaponInventory>(owner)
                .unwrap()
                .ads;
            run.record(
                "aim toggle reaches live ADS",
                ads > 0.95 && !world.get::<Player>(owner).unwrap().crouched,
                serde_json::json!({"ads":ads}),
            );
            tap(world, window, KeyCode::ControlLeft);
            tap(world, window, KeyCode::Escape);
            go(&mut run, 200);
        }
        200 => {
            let ads = world
                .get::<crate::weapons::WeaponInventory>(owner)
                .unwrap()
                .ads;
            run.record("pause retains active ADS and crouch", state == GameState::Paused && ads > 0.95 && world.get::<Player>(owner).unwrap().crouched,
                serde_json::json!({"state":format!("{state:?}"),"ads":ads,"crouched":world.get::<Player>(owner).unwrap().crouched}));
            picture(world, &run, "pause-posture.png");
            tap(world, window, KeyCode::Escape);
            go(&mut run, 201);
        }
        201 => {
            let input = world.resource::<GameInput>();
            run.record("resume retains toggle posture", state == GameState::Playing && input.ads && input.crouch && world.get::<Player>(owner).unwrap().crouched,
                serde_json::json!({"state":format!("{state:?}"),"aim":input.ads,"crouch":input.crouch}));
            tap(world, window, KeyCode::ControlLeft);
            click(world, window, MouseButton::Right);
            mouse(world, window, MouseButton::Back, true);
            go(&mut run, 21);
        }
        21 => {
            let input = world.resource::<GameInput>();
            let ads = world
                .get::<crate::weapons::WeaponInventory>(owner)
                .unwrap()
                .ads;
            run.record(
                "mouse movement and second ADS toggle",
                input.movement.y == -1. && ads < 0.05,
                serde_json::json!({"movement":input.movement.to_array(),"ads":ads}),
            );
            tap(world, window, KeyCode::Escape);
            advance(&mut run);
        }
        22 => {
            tap(world, window, KeyCode::Escape);
            advance(&mut run);
        }
        23 => {
            let input = world.resource::<GameInput>();
            run.record("held actions suppressed across pause",
                input.movement == Vec2::ZERO && !input.ads && !input.crouch && !input.sprint,
                serde_json::json!({"movement":input.movement.to_array(),"sprint":input.sprint,"crouch":input.crouch,"ads":input.ads}),
            );
            mouse(world, window, MouseButton::Back, false);
            run.focus_shots = world.resource::<Session>().shots;
            mouse(world, window, MouseButton::Left, true);
            world.write_message(MouseMotion {
                delta: Vec2::splat(100.),
            });
            world.get_mut::<Window>(window).unwrap().focused = false;
            advance(&mut run);
        }
        24 => {
            let input = world.resource::<GameInput>();
            run.record("focus loss clears actions and motion",
                state == GameState::Paused
                    && !input.fire
                    && !input.fire_held
                    && input.look == Vec2::ZERO,
                serde_json::json!({"state":format!("{state:?}"),"fire":input.fire,"look":input.look.to_array()}),
            );
            world.get_mut::<Window>(window).unwrap().focused = true;
            tap(world, window, KeyCode::Escape);
            advance(&mut run);
        }
        25 => {
            let shots = world.resource::<Session>().shots;
            run.record(
                "held fire suppressed after refocus",
                shots == run.focus_shots && !world.resource::<GameInput>().fire_held,
                serde_json::json!({"before":run.focus_shots,"after":shots}),
            );
            mouse(world, window, MouseButton::Left, false);
            world
                .resource_mut::<Time<Fixed>>()
                .set_timestep_seconds(0.5);
            advance(&mut run);
        }
        26 => {
            advance(&mut run);
        }
        27 => {
            run.shots = world.resource::<Session>().shots;
            click(world, window, MouseButton::Left);
            advance(&mut run);
        }
        28 => {
            let shots = world.resource::<Session>().shots;
            run.record("quick tap persists through rendered frames and fires once",
                run.tap_frames >= 2 && run.tap_ticks == 1 && shots == run.shots + 1,
                serde_json::json!({"pending_rendered_frames":run.tap_frames,"ticks_with_both_edges":run.tap_ticks,"shots_before":run.shots,"shots_after":shots}),
            );
            world.resource_mut::<Time<Fixed>>().set_timestep_hz(60.);
            world
                .resource_mut::<ControlSettings>()
                .rebind(Action::Interact, Some(Binding::Mouse(MouseButton::Middle)))
                .unwrap();
            let game = world.resource::<GameDefinition>();
            let placement = game
                .content
                .placements
                .iter()
                .find(|p| p.id.0 == "placement.quick_revive")
                .unwrap();
            let machine = Vec3::from_array(placement.position);
            let rotation = Quat::from_rotation_y(placement.yaw);
            let player = world.get::<Player>(owner).unwrap();
            let position = machine + rotation * Vec3::new(0., player.height * 0.5 + 0.02, 2.2);
            let delta = machine + Vec3::Y + rotation * Vec3::new(0., 0., 0.8)
                - (position + Vec3::Y * (game.tuning.eye_height - player.height * 0.5));
            let mut player = world.get_mut::<Player>(owner).unwrap();
            player.yaw = (-delta.x).atan2(-delta.z);
            player.pitch = (delta.y / delta.length()).asin();
            player.velocity = Vec3::ZERO;
            world.get_mut::<Transform>(owner).unwrap().translation = position;
            world
                .entity_mut(owner)
                .insert(avian3d::prelude::Position(position));
            advance(&mut run);
        }
        29 => {
            let focus = world.resource::<crate::interactions::InteractionFocus>();
            run.record(
                "interaction prompt uses remapped mouse binding",
                focus.prompt.contains("Middle mouse"),
                serde_json::json!({"prompt":focus.prompt}),
            );
            run.accepted = focus.accepted;
            picture(world, &run, "remapped-interaction.png");
            click(world, window, MouseButton::Middle);
            advance(&mut run);
        }
        30 => {
            let focus = world.resource::<crate::interactions::InteractionFocus>();
            run.record(
                "quick remapped interaction accepted once",
                focus.accepted == run.accepted + 1,
                serde_json::json!({"before":run.accepted,"after":focus.accepted}),
            );
            advance(&mut run);
        }
        31 => {
            tap(world, window, KeyCode::Escape);
            advance(&mut run);
        }
        32 => {
            world
                .get_mut::<Window>(window)
                .unwrap()
                .set_physical_cursor_position(Some(DVec2::new(1000., 900.)));
            tap(world, window, KeyCode::ArrowDown);
            advance(&mut run);
        }
        33 => {
            tap(world, window, KeyCode::Enter);
            advance(&mut run);
        }
        34 => {
            if click_row(world, window, 15) {
                advance(&mut run);
            }
        }
        35 => {
            let settings = world.resource::<ControlSettings>();
            let mut expected = ControlSettings::default();
            expected.audio = settings.audio.clone();
            let saved = crate::input::settings::read(&run.output.join("settings.json"));
            run.record(
                "menu restores and persists defaults",
                *settings == expected && saved.as_ref().is_ok_and(|s| s == settings),
                serde_json::json!({"read_ok":saved.is_ok()}),
            );
            picture(world, &run, "controls-restored.png");
            advance(&mut run);
        }
        36 => {
            if click_row(world, window, 14) {
                advance(&mut run);
            }
        }
        37 => {
            run.record(
                "second binding page",
                world.resource::<ControlMenu>().page == Some(1),
                serde_json::json!({"page":world.resource::<ControlMenu>().page}),
            );
            if run.phase != 99 && click_row(world, window, 15) {
                advance(&mut run);
            }
        }
        38 => {
            run.record(
                "third binding page",
                world.resource::<ControlMenu>().page == Some(2),
                serde_json::json!({"page":world.resource::<ControlMenu>().page}),
            );
            picture(world, &run, "controls-last-page.png");
            advance(&mut run);
        }
        39 => {
            if run.phase != 99 && click_row(world, window, 12) {
                go(&mut run, 390);
            }
        }
        390 => {
            if click_row(world, window, 14) {
                go(&mut run, 40);
            }
        }
        40 => {
            run.record(
                "previous page navigation",
                world.resource::<ControlMenu>().page == Some(0),
                serde_json::json!({"page":world.resource::<ControlMenu>().page}),
            );
            if run.phase != 99 && click_row(world, window, 8) {
                advance(&mut run);
            }
        }
        41 => {
            tap(world, window, KeyCode::KeyW);
            advance(&mut run);
        }
        42 => {
            let menu = world.resource::<ControlMenu>();
            run.record(
                "conflicting binding rejected",
                menu.capturing == Some(Action::Left)
                    && menu.notice.contains("already assigned")
                    && world.resource::<ControlSettings>().bindings[&Action::Left]
                        == [Binding::Key(KeyCode::KeyA)],
                serde_json::json!({"notice":menu.notice}),
            );
            tap(world, window, KeyCode::Escape);
            advance(&mut run);
        }
        43 => {
            run.record(
                "binding cancellation keeps menu paused",
                state == GameState::Paused && world.resource::<ControlMenu>().capturing.is_none(),
                serde_json::json!({"state":format!("{state:?}")}),
            );
            if run.phase != 99 && click_row(world, window, 8) {
                advance(&mut run);
            }
        }
        44 => {
            tap(world, window, KeyCode::Backspace);
            advance(&mut run);
        }
        45 => {
            run.record("binding can be cleared", world.resource::<ControlSettings>().bindings[&Action::Left].is_empty(), serde_json::json!({"label":world.resource::<ControlSettings>().label(Action::Left)}));
            if run.phase != 99 && click_row(world, window, 15) {
                advance(&mut run);
            }
        }
        46 => {
            if point(world, window, 0) {
                advance(&mut run);
            }
        }
        47 => {
            click(world, window, MouseButton::Right);
            advance(&mut run);
        }
        48 => {
            run.record(
                "mouse decreases sensitivity",
                (world.resource::<ControlSettings>().sensitivity - 0.9).abs() < 0.001,
                serde_json::json!({"sensitivity":world.resource::<ControlSettings>().sensitivity}),
            );
            if run.phase != 99 && click_row(world, window, 15) {
                advance(&mut run);
            }
        }
        49 => {
            let settings = world.resource::<ControlSettings>();
            let mut expected = ControlSettings::default();
            expected.audio = settings.audio.clone();
            run.record(
                "final defaults and valid settings",
                *settings == expected && settings.valid(),
                serde_json::json!({"valid":settings.valid()}),
            );
            picture(world, &run, "controls-final.png");
            advance(&mut run);
        }
        50 | 99 => {
            let passed = run.phase == 50 && run.rows.iter().all(|r| r["passed"] == true);
            let receipt = serde_json::json!({"passed":passed,"phase":run.phase,"rows":run.rows,"startup_loaded":run.loaded,"startup_settings":run.initial_settings,
                "startup_audio":run.startup_audio,"startup_error":run.startup_error,
                "elapsed_seconds":run.started.elapsed().as_secs_f32(),"native_equivalence":false});
            let _ = std::fs::write(
                run.output.join("controls.json"),
                serde_json::to_vec_pretty(&receipt).unwrap(),
            );
            world.write_message(if passed {
                AppExit::Success
            } else {
                AppExit::error()
            });
        }
        _ => {}
    }
    let _ = std::fs::write(
        run.output.join("controls-progress.json"),
        serde_json::json!({"phase":run.phase,"rows":run.rows}).to_string(),
    );
    world.insert_resource(run);
}
