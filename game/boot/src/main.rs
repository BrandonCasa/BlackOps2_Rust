#![forbid(unsafe_code)]

use std::{ffi::OsStr, time::Duration};

use bevy::{app::ScheduleRunnerPlugin, prelude::*};

// These values bound a development check. They are not original-game timing.
const SMOKE_UPDATES: u32 = 3;
const SMOKE_PAUSE: Duration = Duration::from_millis(1);

fn main() -> AppExit {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match arguments.as_slice() {
        [] => smoke_app().run(),
        [argument] if argument == OsStr::new("--smoke") => smoke_app().run(),
        [argument] if argument == OsStr::new("--help") || argument == OsStr::new("-h") => {
            print_help();
            AppExit::Success
        }
        _ => {
            eprintln!("Unsupported arguments. Only the headless smoke check is available.");
            print_help();
            AppExit::from_code(2)
        }
    }
}

fn print_help() {
    println!("Usage: bo2-boot [--smoke | --help]");
    println!("Default: run a bounded headless Bevy lifecycle check and exit.");
}

fn smoke_app() -> App {
    let mut app = bo2_core::headless_app(ScheduleRunnerPlugin::run_loop(SMOKE_PAUSE));
    app.add_plugins(SmokeCheckPlugin);
    app
}

struct SmokeCheckPlugin;

impl Plugin for SmokeCheckPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SmokeProgress>()
            .add_systems(Startup, initialize_smoke)
            .add_systems(Update, advance_smoke)
            .add_systems(Last, finish_smoke);
    }
}

#[derive(Resource, Default)]
struct SmokeProgress {
    startup_runs: u32,
    updates: u32,
}

/// Synthetic component used only to verify deferred spawn and query scheduling.
#[derive(Component, Default)]
struct SmokeProbe {
    updates: u32,
}

fn initialize_smoke(mut commands: Commands, mut progress: ResMut<SmokeProgress>) {
    progress.startup_runs += 1;
    commands.spawn(SmokeProbe::default());
    println!("{}: headless Bevy smoke", bo2_core::PROJECT_NAME);
    println!("Fidelity: UNKNOWN. No gameplay or assets are loaded.");
}

fn advance_smoke(mut progress: ResMut<SmokeProgress>, mut probes: Query<&mut SmokeProbe>) {
    progress.updates += 1;
    for mut probe in &mut probes {
        probe.updates += 1;
    }
}

fn finish_smoke(
    progress: Res<SmokeProgress>,
    probes: Query<&SmokeProbe>,
    mut exit: MessageWriter<AppExit>,
) {
    if progress.updates < SMOKE_UPDATES {
        return;
    }

    let probe_updates = probes.single().ok().map(|probe| probe.updates);
    if progress.startup_runs == 1
        && progress.updates == SMOKE_UPDATES
        && probe_updates == Some(SMOKE_UPDATES)
    {
        println!(
            "Smoke: PASS (startup={}, updates={}, probe_updates={}).",
            progress.startup_runs, progress.updates, SMOKE_UPDATES
        );
        exit.write(AppExit::Success);
    } else {
        eprintln!(
            "Smoke: FAIL (startup={}, updates={}, probe_updates={probe_updates:?}).",
            progress.startup_runs, progress.updates
        );
        exit.write(AppExit::error());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_spawn_is_updated_before_successful_exit() {
        let mut app = smoke_app();
        app.finish();
        app.cleanup();

        for _ in 1..SMOKE_UPDATES {
            app.update();
            assert_eq!(app.should_exit(), None);
        }
        app.update();

        assert_eq!(app.should_exit(), Some(AppExit::Success));
        let progress = app.world().resource::<SmokeProgress>();
        assert_eq!(progress.startup_runs, 1);
        assert_eq!(progress.updates, SMOKE_UPDATES);
        let mut probes = app.world_mut().query::<&SmokeProbe>();
        assert_eq!(probes.single(app.world()).unwrap().updates, SMOKE_UPDATES);
    }

    #[test]
    fn missing_synthetic_entity_reports_failure_instead_of_panicking() {
        let mut app = smoke_app();
        app.finish();
        app.cleanup();
        app.update();
        let mut probes = app.world_mut().query_filtered::<Entity, With<SmokeProbe>>();
        let probe = probes.single(app.world()).unwrap();
        assert!(app.world_mut().despawn(probe));
        for _ in 1..SMOKE_UPDATES {
            app.update();
        }

        assert_eq!(app.should_exit(), Some(AppExit::error()));
    }
}
