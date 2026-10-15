mod overlay;
mod route;
mod telemetry;

use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    gameplay::GameStep,
    rendering::GameCamera,
};
use bevy::{
    prelude::*,
    window::{PrimaryWindow, WindowCloseRequested},
};
use std::{
    ffi::OsString,
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) fn normal_play(options: Res<LaunchOptions>) -> bool {
    !options.flythrough
}

#[derive(Resource)]
struct Settings {
    output: PathBuf,
    speed: f64,
    exit_on_finish: bool,
}

#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
pub struct TourStatus {
    pub phase: String,
    pub label: String,
    pub shot: usize,
    pub shots: usize,
    pub elapsed_seconds: f64,
    pub shot_elapsed_seconds: f64,
    pub duration_seconds: f64,
    pub paused: bool,
    pub report_error: String,
}

#[derive(Resource)]
struct Tour {
    created: Instant,
    ready: Option<Instant>,
    shots: Vec<route::Shot>,
    shot_seconds: f64,
    frames: Vec<f64>,
    telemetry: Vec<telemetry::Sample>,
    trace: Vec<serde_json::Value>,
    next_trace: f64,
    saved: bool,
    warmed: bool,
    discard_interval: bool,
}

impl Default for Tour {
    fn default() -> Self {
        Self {
            created: Instant::now(),
            ready: None,
            shots: Vec::new(),
            shot_seconds: 0.,
            frames: Vec::with_capacity(100_000),
            telemetry: Vec::new(),
            trace: Vec::new(),
            next_trace: 0.,
            saved: false,
            warmed: false,
            discard_interval: true,
        }
    }
}

pub fn app(mut arguments: impl Iterator<Item = OsString>) -> Result<App, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/reports");
    let mut settings = Settings {
        output: base.join(format!("flythrough-{timestamp}")),
        speed: 1.,
        exit_on_finish: false,
    };
    let mut options = LaunchOptions {
        benchmark: true,
        flythrough: true,
        ..default()
    };
    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--asset-root") => options.asset_root = arguments.next().map(PathBuf::from).ok_or("--asset-root requires a directory")?,
            Some("--output") => settings.output = arguments.next().map(PathBuf::from).ok_or("--output requires a directory")?,
            Some("--speed") => {
                settings.speed = arguments.next().and_then(|v| v.to_str()?.parse().ok())
                    .filter(|v: &f64| v.is_finite() && (0.25..=8.).contains(v))
                    .ok_or("--speed requires a number from 0.25 to 8")?;
            },
            Some("--exit-on-finish") => settings.exit_on_finish = true,
            Some("--validation-remote") if cfg!(all(debug_assertions, feature = "dev-tools")) => {
                options.validation_remote = Some(arguments.next().and_then(|v| v.to_str()?.parse().ok())
                    .filter(|p: &u16| *p > 0 && *p != 15703).ok_or("--validation-remote requires a loopback port other than 15703")?);
            },
            _ => return Err("Usage: origins --flythrough --asset-root absolute-directory [--output directory] [--speed 0.25..8] [--exit-on-finish]".into()),
        }
    }
    if !options.asset_root.is_absolute() || !options.asset_root.is_dir() {
        return Err("--asset-root requires an existing absolute directory".into());
    }
    if !settings.output.is_absolute() {
        settings.output = std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(settings.output);
    }
    if !settings.output.starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")) {
        return Err("Flythrough output must be under this checkout target directory".into());
    }
    std::fs::create_dir_all(&settings.output)
        .map_err(|e| format!("Cannot create flythrough output: {e}"))?;
    let mut app = crate::configured_app(options);
    app.insert_resource(settings)
        .insert_resource(TourStatus {
            phase: "loading".into(),
            label: "Loading Origins".into(),
            ..default()
        })
        .init_resource::<Tour>()
        .init_resource::<telemetry::Telemetry>()
        .register_type::<TourStatus>()
        .configure_sets(
            FixedUpdate,
            (
                GameStep::Movement,
                GameStep::EnemyIntent,
                GameStep::Combat,
                GameStep::Damage,
                GameStep::PlayerLife,
                GameStep::Rewards,
                GameStep::Zombies,
                GameStep::Rounds,
            )
                .run_if(normal_play),
        )
        .add_systems(
            OnEnter(GameState::Playing),
            prepare.after(crate::session::SessionSetup::Player),
        )
        .add_systems(Update, controls)
        .add_systems(
            PostUpdate,
            fly.after(crate::presentation::CameraPose)
                .before(bevy::transform::TransformSystems::Propagate),
        )
        .add_systems(PostUpdate, conceal.after(fly))
        .add_systems(Last, (record, save_on_exit).chain())
        .add_plugins(overlay::TourOverlayPlugin);
    Ok(app)
}

fn prepare(
    game: Res<GameDefinition>,
    mut access: ResMut<crate::access::ZoneAccess>,
    mut generators: ResMut<crate::generators::Generators>,
    mut tour: ResMut<Tour>,
    mut status: ResMut<TourStatus>,
) {
    *tour = Tour::default();
    *status = TourStatus::default();
    for door in &game.access.doors {
        let _ = access.unlock(&game, &door.connection);
    }
    for site in generators.sites.values_mut() {
        site.state = crate::generators::GeneratorState::Active;
    }
    tour.ready = Some(Instant::now());
    status.phase = "starting".into();
    status.label = "Preparing camera tour".into();
}

fn controls(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    settings: Res<Settings>,
    mut tour: ResMut<Tour>,
    mut status: ResMut<TourStatus>,
    mut exit: MessageWriter<AppExit>,
) {
    let mut pressed = [false; 3];
    for event in keys.read() {
        if event.state == bevy::input::ButtonState::Pressed && !event.repeat {
            match event.key_code {
                KeyCode::Escape => pressed[0] = true,
                KeyCode::Space => pressed[1] = true,
                KeyCode::KeyR => pressed[2] = true,
                _ => {}
            }
        }
    }
    if pressed[0] {
        exit.write(AppExit::Success);
    }
    if pressed[1] && status.phase == "running" {
        status.paused = !status.paused;
        tour.discard_interval = true;
    }
    if pressed[2] && tour.warmed {
        status.phase = "running".into();
        status.paused = false;
        status.shot = 0;
        status.elapsed_seconds = 0.;
        status.shot_elapsed_seconds = 0.;
        status.report_error.clear();
        tour.shot_seconds = 0.;
        tour.frames.clear();
        tour.telemetry.clear();
        tour.trace.clear();
        tour.next_trace = 0.;
        tour.saved = false;
        tour.discard_interval = true;
    }
    if status.phase == "finished" && settings.exit_on_finish && tour.saved {
        exit.write(if status.report_error.is_empty() {
            AppExit::Success
        } else {
            AppExit::error()
        });
    }
}

fn fly(
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    game: Option<Res<GameDefinition>>,
    settings: Res<Settings>,
    mut cameras: Query<&mut Transform, With<GameCamera>>,
    mut tour: ResMut<Tour>,
    mut status: ResMut<TourStatus>,
    mut exit: MessageWriter<AppExit>,
) {
    if **state == GameState::LoadError
        || (tour.ready.is_none() && tour.created.elapsed().as_secs() > 180)
    {
        status.phase = "failed".into();
        status.label = "Asset loading failed or timed out".into();
        exit.write(AppExit::error());
        return;
    }
    let (Some(game), Ok(mut camera), Some(_ready)) = (game, cameras.single_mut(), tour.ready) else {
        return;
    };
    if **state != GameState::Playing {
        return;
    }
    if !tour.warmed {
        let spawn = Vec3::from_array(game.spawn) + Vec3::Y * game.tuning.eye_height;
        *camera = Transform::from_translation(spawn).with_rotation(Quat::from_rotation_y(game.yaw));
        let positions = Vec::new();
        tour.shots = route::build(&game, &positions);
        status.shots = tour.shots.len();
        status.duration_seconds = tour.shots.iter().map(|s| s.seconds / settings.speed).sum();
        tour.warmed = true;
        status.phase = "running".into();
        return;
    }
    if status.phase != "running" || status.paused {
        return;
    }
    let dt = time.delta_secs_f64();
    tour.shot_seconds += dt * settings.speed;
    status.elapsed_seconds += dt;
    while status.shot < tour.shots.len() && tour.shot_seconds >= tour.shots[status.shot].seconds {
        tour.shot_seconds -= tour.shots[status.shot].seconds;
        status.shot += 1;
    }
    let Some(shot) = tour.shots.get(status.shot) else {
        status.phase = "finished".into();
        status.label = "Tour complete / R to replay".into();
        return;
    };
    status.label.clone_from(&shot.label);
    status.shot_elapsed_seconds = tour.shot_seconds / settings.speed;
    let pose = shot.pose((tour.shot_seconds / shot.seconds) as f32);
    let blend = if shot.cut || tour.frames.is_empty() {
        1.
    } else {
        1. - (-time.delta_secs() * 8.).exp()
    };
    camera.translation = pose.translation;
    camera.rotation = camera.rotation.slerp(pose.rotation, blend);
    if status.elapsed_seconds >= tour.next_trace {
        tour.trace.push(
            serde_json::json!({"seconds":status.elapsed_seconds,"shot":status.shot,
            "position":camera.translation.to_array(),"rotation":camera.rotation.to_array()}),
        );
        tour.next_trace = status.elapsed_seconds + 1.;
    }
}

fn conceal(
    state: Res<State<GameState>>,
    mut hud: Query<&mut Visibility, With<crate::hud::PlayerOverlay>>,
) {
    if **state == GameState::Playing {
        for mut visibility in &mut hud {
            *visibility = Visibility::Hidden;
        }
    }
}

fn record(
    time: Res<Time<Real>>,
    telemetry: Res<telemetry::Telemetry>,
    settings: Res<Settings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut tour: ResMut<Tour>,
    mut status: ResMut<TourStatus>,
) {
    let samples = telemetry.drain();
    if status.phase == "running" && !status.paused {
        if tour.discard_interval {
            tour.discard_interval = false;
            return;
        }
        if tour.frames.len() < 1_000_000 {
            tour.frames.push(time.delta_secs_f64() * 1000.);
        }
        tour.telemetry.extend(samples);
    }
    if status.phase == "finished" && !tour.saved {
        save(&settings, &mut tour, &mut status, windows.single().ok());
    }
}

fn save_on_exit(
    mut exits: MessageReader<AppExit>,
    mut closing: MessageReader<WindowCloseRequested>,
    settings: Res<Settings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut tour: ResMut<Tour>,
    mut status: ResMut<TourStatus>,
) {
    if (exits.read().next().is_some() || closing.read().next().is_some()) && !tour.saved {
        save(&settings, &mut tour, &mut status, windows.single().ok());
    }
}

fn save(settings: &Settings, tour: &mut Tour, status: &mut TourStatus, window: Option<&Window>) {
    let mut sorted = tour.frames.clone();
    sorted.sort_unstable_by(f64::total_cmp);
    let elapsed_ms: f64 = sorted.iter().sum();
    let percentile = |p: f64| {
        sorted
            .get(((sorted.len().saturating_sub(1)) as f64 * p).round() as usize)
            .copied()
    };
    let report = serde_json::json!({
        "completed": status.phase == "finished", "phase":status.phase, "speed":settings.speed,
        "profile":if cfg!(debug_assertions) {"development"} else {"release"},
        "dev_tools":cfg!(feature="dev-tools"), "executable":std::env::current_exe().ok(),
        "physical_resolution":window.map(|w|[w.physical_width(),w.physical_height()]),
        "logical_cpus":std::thread::available_parallelism().map_or(1,usize::from),
        "fps":if elapsed_ms>0. {Some(sorted.len() as f64*1000./elapsed_ms)} else {None},
        "p95_ms":percentile(0.95), "p99_ms":percentile(0.99), "frames":tour.frames.len(),
        "sample_seconds":elapsed_ms/1000., "frame_times_ms":tour.frames,
        "average_cpu_percent":telemetry::average(&tour.telemetry, |s|Some(s.cpu_percent)),
        "average_working_set_mib":telemetry::average(&tour.telemetry, |s|Some(s.working_set_mib)),
        "average_private_mib":telemetry::average(&tour.telemetry, |s|Some(s.private_mib)),
        "average_gpu_busiest_engine_percent":telemetry::average(&tour.telemetry, |s|s.gpu_percent),
        "average_dedicated_gpu_mib":telemetry::average(&tour.telemetry, |s|s.dedicated_gpu_mib),
        "average_system_cpu_percent":telemetry::average(&tour.telemetry, |s|s.system_cpu_percent),
        "resource_samples":tour.telemetry, "camera_trace":tour.trace,
        "route":tour.shots.iter().map(|s|serde_json::json!({"label":s.label,"seconds":s.seconds/settings.speed,
            "cut":s.cut,"path":s.path.iter().map(|p|p.to_array()).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "limitations":["Visual scenery workload: enemies and player movement disabled; doors open and generators powered.",
            "Frame times are application wall intervals, including the visible overlay, not GPU timestamps.",
            "CPU is process time divided by logical CPU count; RAM is resident working set and private committed bytes.",
            "GPU is this process's busiest Windows GPU engine, sampled about once a second; unavailable counters are null.",
            "PowerShell/CIM sampling runs outside the measured game process and adds system load.",
            "Navigation follows source travel links; disconnected areas use explicit camera cuts.",
            "Pause intervals are excluded; loading and scene setup are excluded."]
    });
    let result = serde_json::to_vec_pretty(&report)
        .map_err(std::io::Error::other)
        .and_then(|bytes| std::fs::write(settings.output.join("flythrough.json"), bytes));
    if let Err(error) = result {
        status.report_error = format!("Report save failed: {error}");
    }
    tour.saved = true;
}
