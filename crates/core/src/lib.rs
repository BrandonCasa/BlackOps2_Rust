#![forbid(unsafe_code)]

//! Shared engine composition. Original-game systems require separate evidence.

use bevy::{app::ScheduleRunnerPlugin, prelude::*};

/// The project name displayed by the boot executable.
pub const PROJECT_NAME: &str = "Black Ops II Zombies: Origins in Rust";

/// Compose Bevy's minimal headless application with the caller's runner policy.
///
/// Bevy owns the world, schedules, task pools, time and lifecycle. Assembly adds
/// its own plugins to the returned app; this layer contains no map or smoke logic.
/// The runner's pacing is an application choice, not a gameplay simulation rate.
pub fn headless_app(runner: ScheduleRunnerPlugin) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(runner));
    app
}
