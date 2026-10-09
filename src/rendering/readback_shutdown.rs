use bevy::{
    ecs::message::Messages,
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems, render_resource::PollType,
        renderer::RenderDevice,
    },
};
use std::time::Duration;

#[derive(Resource, Default)]
struct ExitRequested(bool);

pub(super) struct ReadbackShutdownPlugin;

impl Plugin for ReadbackShutdownPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<ExitRequested>()
                .add_systems(ExtractSchedule, extract_exit)
                .add_systems(Render, finish_readbacks.after(RenderSystems::Cleanup));
        }
    }
}

fn extract_exit(exits: Extract<Res<Messages<AppExit>>>, mut requested: ResMut<ExitRequested>) {
    requested.0 = !exits.is_empty();
}

fn finish_readbacks(requested: Res<ExitRequested>, device: Res<RenderDevice>) {
    if !requested.0 {
        return;
    }
    if let Err(error) = device.poll(PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(30)),
    }) {
        error!("GPU readback shutdown did not complete: {error}");
    }
}
