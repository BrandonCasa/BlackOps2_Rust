//! Opt-in measurements of the shipping application with live simulation and UI.
//! Setup is controlled; movement, admission, animation, combat and rendering use
//! production systems. No captures or report serialization occur while sampling.
use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions, LoadFailure},
    gameplay::{GameStep, Session, Zombie},
    input::GameInput,
    life::{LifeState, PlayerLethalPolicy, Vitality},
    player::Player,
    weapons::WeaponInventory,
};
use bevy::prelude::*;
use std::time::Instant;

const WARMUP_SECONDS: f64 = 10.;
// Visit the entire camera sweep twice before measuring. First-use texture reads
// otherwise dominate a short spawn warmup and change the path through time loss.
const MOVING_PREP_SECONDS: f32 = 36.;
const SAMPLE_SECONDS: f64 = 30.;
const SEED: u64 = 0xB02;
const COMBAT_ACTORS: u32 = 6;
const COMBAT_ACTOR_HEALTH: u32 = 1_000_000;

#[derive(Component)]
struct CombatPrepared;

#[derive(Resource)]
struct Run {
    started: Instant,
    ready: Option<Instant>,
    sample_start: Option<Instant>,
    previous: Option<Instant>,
    frames: Vec<f64>,
    ticks: u64,
    sample_ticks: u64,
    initial: Vec3,
    configured: bool,
    finished: bool,
    start_counts: serde_json::Value,
    living_range: [usize; 2],
    sample_shots: u32,
    sample_points: u32,
}

pub(super) fn install(app: &mut App) {
    app.insert_resource(Run {
        started: Instant::now(),
        ready: None,
        sample_start: None,
        previous: None,
        frames: Vec::with_capacity(18000),
        ticks: 0,
        sample_ticks: 0,
        initial: Vec3::ZERO,
        configured: false,
        finished: false,
        start_counts: serde_json::Value::Null,
        living_range: [usize::MAX, 0],
        sample_shots: 0,
        sample_points: 0,
    })
    .add_systems(Startup, setup)
    .add_systems(
        FixedUpdate,
        drive
            .in_set(GameStep::Movement)
            .before(crate::player::move_player),
    )
    .add_systems(Last, record);
}

fn setup(mut commands: Commands, options: Res<LaunchOptions>) {
    let Some(output) = options.verify.as_ref() else {
        commands.write_message(AppExit::error());
        return;
    };
    if !output.is_absolute() || output.starts_with(env!("CARGO_MANIFEST_DIR")) {
        error!("Release benchmark output must be an absolute external path");
        commands.write_message(AppExit::error());
        return;
    }
    if let Err(error) = std::fs::create_dir_all(output) {
        error!("Cannot create release benchmark output: {error}");
        commands.write_message(AppExit::error());
    }
}

fn drive(
    mut commands: Commands,
    state: Res<State<GameState>>,
    options: Res<LaunchOptions>,
    game: Option<ResMut<GameDefinition>>,
    time: Res<Time<Fixed>>,
    mut run: ResMut<Run>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<
        (&mut Player, &Transform, &mut Vitality, &mut WeaponInventory),
        Without<Zombie>,
    >,
    mut enemies: Query<
        (Entity, &Transform, &mut Vitality, Option<&CombatPrepared>),
        (With<Zombie>, Without<Player>),
    >,
) {
    input.clear();
    if **state != GameState::Playing || run.finished {
        return;
    }
    let (Some(mut game), Ok((mut player, pose, mut health, mut inventory))) =
        (game, players.single_mut())
    else {
        return;
    };
    if !run.configured {
        // A repeatable sustained encounter, not a survival or progression claim.
        session.seed = SEED;
        session.round = if options.benchmark_combat { 8 } else { 1 };
        session.pending = if options.benchmark_combat {
            COMBAT_ACTORS
        } else {
            1000
        };
        session.intermission = None;
        session.spawn_timer = Timer::from_seconds(
            if options.benchmark_combat { 1. } else { 3600. },
            TimerMode::Once,
        );
        run.initial = pose.translation;
        if options.benchmark_combat {
            let Some(weapon) = game
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.mg08_zm")
            else {
                error!("Release combat workload requires the native MG08 definition");
                commands.write_message(AppExit::error());
                return;
            };
            let mut owned = WeaponInventory::default();
            if owned.grant(weapon).is_err() {
                commands.write_message(AppExit::error());
                return;
            }
            *inventory = owned;
            // Keep the production perk maximum reconciliation consistent with
            // this explicitly recorded benchmark-only starting health.
            game.player_health.maximum = 10000;
            *health = Vitality::human(
                10000,
                10000,
                1,
                PlayerLethalPolicy::SoloOrLastEligiblePlayer,
            )
            .expect("valid benchmark starting health");
        }
        run.configured = true;
        if !options.benchmark_combat {
            run.ready = Some(Instant::now());
            info!("Release benchmark ready; warming up for {WARMUP_SECONDS} wall seconds");
        }
    }
    if options.benchmark_combat {
        let mut living = 0;
        for (entity, _, mut health, prepared) in &mut enemies {
            if health.state() != LifeState::ActorAlive {
                continue;
            }
            living += 1;
            if prepared.is_none() {
                // Preserve the production actor, gait, navigation and damage path.
                // High health holds the cohort constant during sustained hits.
                *health = Vitality::ordinary_actor(COMBAT_ACTOR_HEALTH, COMBAT_ACTOR_HEALTH)
                    .expect("valid benchmark actor health");
                commands.entity(entity).insert(CombatPrepared);
            }
        }
        if run.ready.is_none() {
            if living != COMBAT_ACTORS || session.pending != 0 {
                return;
            }
            run.ready = Some(Instant::now());
            info!("Release combat cohort ready; warming up for {WARMUP_SECONDS} wall seconds");
        }
    }
    run.ticks += 1;
    let seconds = run.ticks as f32 * time.delta_secs();
    if !options.benchmark_combat {
        let sample_seconds = run.sample_start.map(|start| start.elapsed().as_secs_f32());
        let camera_seconds = sample_seconds.unwrap_or(seconds);
        // Look across the actual spawn surroundings while the ordinary renderer,
        // environment, first-person animation and HUD continue to run.
        player.yaw = game.yaw + camera_seconds * 0.35;
        player.pitch = (camera_seconds * 0.4).sin() * 0.2;
        if let Some(seconds) = sample_seconds {
            input.movement = Vec2::new((seconds * 0.8).sin(), (seconds * 0.8).cos()) * 0.18;
        }
        return;
    }
    let target = enemies
        .iter()
        .filter(|(_, _, vitality, _)| vitality.state() == LifeState::ActorAlive)
        .min_by(|(a, pa, _, _), (b, pb, _, _)| {
            pa.translation
                .distance_squared(pose.translation)
                .total_cmp(&pb.translation.distance_squared(pose.translation))
                .then_with(|| a.to_bits().cmp(&b.to_bits()))
        });
    if let Some((_, target, _, _)) = target {
        let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
        let delta = target.translation + Vec3::Y * 0.35 - eye;
        player.yaw = (-delta.x).atan2(-delta.z);
        player.pitch = delta.y.atan2(delta.xz().length());
        // Recoil still renders; compensate aim exactly as an ordinary input driver.
        player.yaw -= inventory.recoil.x;
        player.pitch -= inventory.recoil.y;
        input.ads = true;
        input.fire_held = true;
        input.fire = true;
        input.reload = inventory.active().is_some_and(|w| w.magazine == 0);
        input.movement = Vec2::new(
            (seconds * 0.7).sin() * 0.2,
            if delta.length() < 3. { -0.35 } else { 0. },
        );
    }
}

fn record(
    mut commands: Commands,
    mut run: ResMut<Run>,
    state: Res<State<GameState>>,
    options: Res<LaunchOptions>,
    failure: Res<LoadFailure>,
    session: Res<Session>,
    combat: Res<crate::damage::CombatHistory>,
    enemies: Query<&Vitality, With<Zombie>>,
    windows: Query<&Window>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    streaming: Res<crate::rendering::TextureStreamingStats>,
    audio: Res<crate::audio::AudioDecodeStats>,
    profile: Res<crate::rendering::profile::VisualProfile>,
    preprocessing: Res<crate::rendering::PreprocessingStatus>,
    environment: Res<crate::run_environment::RunEnvironment>,
) {
    if run.finished {
        return;
    }
    let Some(output) = options.verify.as_ref() else {
        return;
    };
    let failed = matches!(
        **state,
        GameState::GameOver | GameState::LoadError | GameState::Paused
    ) || run.started.elapsed().as_secs() > 900
        || streaming.errors > 0;
    if failed {
        let receipt = serde_json::json!({"passed":false, "state":format!("{:?}", **state),
            "load_failure": failure.0, "streaming": &*streaming,
            "elapsed_seconds": run.started.elapsed().as_secs_f64()});
        let _ = std::fs::write(output.join("failure.json"), receipt.to_string());
        run.finished = true;
        commands.write_message(AppExit::error());
        return;
    }
    let Some(ready) = run.ready else {
        return;
    };
    let now = Instant::now();
    if now.duration_since(ready).as_secs_f64() < WARMUP_SECONDS {
        return;
    }
    if run.sample_start.is_none()
        && !options.benchmark_combat
        && (run.ticks < (MOVING_PREP_SECONDS * 60.) as u64
            || streaming.pending_detail > 0
            || streaming.queued_reads > 0)
    {
        return;
    }
    let living = enemies
        .iter()
        .filter(|v| v.state() == LifeState::ActorAlive)
        .count();
    let Some(start) = run.sample_start else {
        run.sample_start = Some(now);
        run.previous = Some(now);
        run.sample_ticks = run.ticks;
        run.sample_shots = session.shots;
        run.sample_points = combat.awarded_points;
        run.living_range = [living, living];
        run.start_counts = serde_json::json!({"kills":session.kills,"shots":session.shots,
            "pending":session.pending,"living":living,"damage_awarded_points":combat.awarded_points});
        info!("Release benchmark sampling for {SAMPLE_SECONDS} wall seconds");
        return;
    };
    if let Some(previous) = run.previous.replace(now) {
        run.frames
            .push(now.duration_since(previous).as_secs_f64() * 1000.);
    }
    run.living_range[0] = run.living_range[0].min(living);
    run.living_range[1] = run.living_range[1].max(living);
    let elapsed = now.duration_since(start).as_secs_f64();
    if elapsed < SAMPLE_SECONDS {
        return;
    }
    let passed = !run.frames.is_empty()
        && (!options.benchmark_combat
            || (session.shots > run.sample_shots
                && combat.awarded_points > run.sample_points
                && run.living_range == [COMBAT_ACTORS as usize; 2]
                && session.pending == 0));
    let receipt = serde_json::json!({
        "passed": passed, "schema":1, "workload_revision":3,
        "workload":if options.benchmark_combat {"combat"} else {"moving"},
        "clock":"wall-clock intervals between completed main-world frames",
        "warmup_seconds":WARMUP_SECONDS,"target_sample_seconds":SAMPLE_SECONDS,
        "actual_warmup_wall_seconds":start.duration_since(ready).as_secs_f64(),
        "moving_path_clock":if options.benchmark_combat {None} else {Some("wall seconds since sampling begins")},
        "sample_wall_seconds":elapsed,"boundary_policy":"include the complete final frame crossing 30 seconds",
        "frame_ms":run.frames,"summary":super::performance::summary(&run.frames,true),
        "fixed_ticks":run.ticks-run.sample_ticks,"seed":SEED,"initial_position":run.initial.to_array(),
        "controlled_setup":{"round":if options.benchmark_combat {8} else {1},
            "pending":if options.benchmark_combat {COMBAT_ACTORS} else {1000},
            "combat_actor_health":if options.benchmark_combat {Some(COMBAT_ACTOR_HEALTH)} else {None},
            "warmup_begins":if options.benchmark_combat {"after complete cohort admission"} else {"two complete camera sweeps, no movement, zero pending texture reads"},
            "player_health":if options.benchmark_combat {10000} else {100},
            "weapon":if options.benchmark_combat {"weapon.mg08_zm"} else {"weapon.c96_zm"},
            "enemy_admission":"production navigation and population cap", "moving_spawns_disabled":!options.benchmark_combat},
        "start_counts":run.start_counts,"end_counts":{"kills":session.kills,"shots":session.shots,
            "pending":session.pending,"living":living,"spawn_cursor":session.spawn_cursor,
            "damage_awarded_points":combat.awarded_points},
        "living_min_max":run.living_range,
        "physical_resolution":windows.single().ok().map(|w|[w.resolution.physical_width(),w.resolution.physical_height()]),
        "adapter":adapter.as_ref().map(|a|format!("{:?}",&***a)),
        "vsync":false,"profile_tracy":cfg!(feature="profile-tracy"),"profile_chrome":cfg!(feature="profile-chrome"),
        "build_profile":if cfg!(debug_assertions) {"debug"} else {"release"},
        "logging_enabled":cfg!(debug_assertions),
        "render_diagnostics":false,"pipeline_cache_per_frame_inspection":false,
        "run_environment": &*environment,
        "antialiasing":profile.antialiasing,"preprocessing":preprocessing.report(),
        "shared_lighting":{"method":"native_raster","ray_tracing":false,"local_light_limit":profile.max_local_lights},
        "streaming":&*streaming,"audio_decode":&*audio,
    });
    match std::fs::write(output.join("performance.json"), receipt.to_string()) {
        Ok(()) if passed => {
            info!("Release benchmark completed: {}", receipt["summary"]);
            commands.write_message(AppExit::Success);
        }
        result => {
            error!("Release benchmark failed: passed={passed}, write={result:?}");
            commands.write_message(AppExit::error());
        }
    }
    run.finished = true;
}
