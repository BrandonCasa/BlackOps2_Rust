use bevy::prelude::*;

use crate::assets::LaunchOptions;

#[derive(Resource, Default, serde::Serialize)]
pub(crate) struct RunEnvironment {
    pub continuous_window_updates: bool,
    pub display_sleep_inhibited: bool,
    pub system_sleep_inhibited: bool,
}

pub(crate) struct UnattendedRunPlugin;

impl Plugin for UnattendedRunPlugin {
    fn build(&self, app: &mut App) {
        let options = app.world().resource::<LaunchOptions>();
        let unattended =
            options.benchmark || options.verify.is_some() || options.validation_remote.is_some();
        app.insert_resource(RunEnvironment {
            continuous_window_updates: unattended,
            ..default()
        });
        if unattended {
            app.insert_resource(bevy::winit::WinitSettings::continuous());
            #[cfg(windows)]
            app.add_systems(PreStartup, inhibit_sleep);
        }
    }
}

#[cfg(windows)]
struct PowerGuard {
    _request: keepawake::KeepAwake,
}

#[cfg(windows)]
fn inhibit_sleep(world: &mut World) {
    match keepawake::Builder::default()
        .display(true)
        .idle(true)
        .create()
    {
        Ok(request) => {
            world.insert_non_send(PowerGuard { _request: request });
            let mut environment = world.resource_mut::<RunEnvironment>();
            environment.display_sleep_inhibited = true;
            environment.system_sleep_inhibited = true;
            if let Err(error) = write_environment(world) {
                eprintln!("Cannot write unattended power-request receipt: {error}");
                world.write_message(AppExit::error());
            }
        }
        Err(error) => {
            eprintln!("Unattended run cannot inhibit display/system sleep: {error}");
            if let Some(output) = world.resource::<LaunchOptions>().verify.as_ref()
                && !output.starts_with(env!("CARGO_MANIFEST_DIR"))
            {
                let failure = serde_json::json!({
                    "passed": false,
                    "reason": "display/system sleep inhibition failed",
                    "error": error.to_string(),
                });
                if let Err(write_error) = std::fs::create_dir_all(output)
                    .and_then(|()| std::fs::write(output.join("failure.json"), failure.to_string()))
                {
                    eprintln!("Cannot write unattended startup failure: {write_error}");
                }
            }
            world.write_message(AppExit::error());
        }
    }
}

#[cfg(windows)]
fn write_environment(world: &World) -> std::io::Result<()> {
    let Some(output) = world.resource::<LaunchOptions>().verify.as_ref() else {
        return Ok(());
    };
    if !output.is_absolute() || output.starts_with(env!("CARGO_MANIFEST_DIR")) {
        return Ok(());
    }
    let receipt =
        serde_json::to_vec(world.resource::<RunEnvironment>()).map_err(std::io::Error::other)?;
    std::fs::create_dir_all(output)?;
    std::fs::write(output.join("run-environment.json"), receipt)
}
