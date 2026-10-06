//! Fail verification on permanent engine shader errors instead of timing missing draws.
use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_resource::{CachedPipelineState, PipelineCache},
    },
    shader::ShaderCacheError,
};
use std::sync::{Arc, Mutex};

#[derive(Resource, Default, Clone)]
pub(super) struct PipelineFailures(pub Arc<Mutex<Option<String>>>);

pub(super) fn install(app: &mut App) {
    let failures = PipelineFailures::default();
    app.insert_resource(failures.clone())
        .add_systems(Update, fail);
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(failures)
            .add_systems(Render, inspect.after(RenderSystems::Render));
    }
}

fn fail(
    mut commands: Commands,
    failures: Res<PipelineFailures>,
    options: Res<crate::assets::LaunchOptions>,
) {
    if let Ok(failure) = failures.0.lock()
        && let Some(failure) = failure.as_ref()
    {
        if let Some(output) = &options.verify {
            let _ = std::fs::write(
                output.join("pipeline-failure.json"),
                serde_json::json!({"passed": false, "error": failure}).to_string(),
            );
        }
        error!("Game verification failed: permanent rendering pipeline error: {failure}");
        commands.write_message(AppExit::error());
    }
}

fn inspect(cache: Res<PipelineCache>, failures: Res<PipelineFailures>) {
    let Ok(mut failure) = failures.0.lock() else {
        return;
    };
    if failure.is_some() {
        return;
    }
    for pipeline in cache.pipelines() {
        if let CachedPipelineState::Err(error) = &pipeline.state {
            if matches!(
                error,
                ShaderCacheError::ShaderNotLoaded(_)
                    | ShaderCacheError::ShaderImportNotYetAvailable
            ) {
                continue;
            }
            *failure = Some(error.to_string());
            return;
        }
    }
}
