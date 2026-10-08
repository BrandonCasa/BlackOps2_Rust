//! Fail verification on specialization and shader errors instead of timing missing draws.
use bevy::{
    log::{
        BoxedLayer,
        tracing::{Event, Subscriber, field::Visit},
        tracing_subscriber::{Layer, layer::Context},
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_resource::{CachedPipelineState, PipelineCache},
    },
    shader::ShaderCacheError,
};
use std::sync::{Arc, Mutex};

#[derive(Resource, Default, Clone)]
pub(super) struct PipelineFailures(pub Arc<Mutex<Option<RenderFailure>>>);

#[derive(Clone, serde::Serialize)]
pub(super) struct RenderFailure {
    target: String,
    error: String,
}

/// LogPlugin runs before the verification/render plugins. They must retain the
/// same resource so errors emitted during engine registration are not lost.
pub(crate) fn error_layer(app: &mut App) -> Option<BoxedLayer> {
    if app
        .world()
        .resource::<crate::assets::LaunchOptions>()
        .verify
        .is_none()
    {
        return None;
    }
    app.init_resource::<PipelineFailures>();
    Some(Box::new(MaterialErrors(
        app.world().resource::<PipelineFailures>().clone(),
    )))
}

struct MaterialErrors(PipelineFailures);

#[derive(Default)]
struct Message(String);

impl Visit for Message {
    fn record_debug(
        &mut self,
        field: &bevy::log::tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        }
    }
}

impl<S: Subscriber> Layer<S> for MaterialErrors {
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        // Specialization errors are logged before PipelineCache receives a
        // descriptor. These engine errors otherwise leave missing draws while
        // the cached-shader inspection still reports success.
        if *metadata.level() != bevy::log::Level::ERROR
            || !(metadata.target().starts_with("bevy_pbr::")
                || metadata.target().starts_with("bevy_render::")
                || metadata.target().starts_with("bevy_shader::"))
        {
            return;
        }
        let mut message = Message::default();
        event.record(&mut message);
        if let Ok(mut failure) = self.0.0.lock() {
            failure.get_or_insert_with(|| RenderFailure {
                target: metadata.target().into(),
                error: message.0,
            });
        }
    }
}

pub(super) fn install(app: &mut App) {
    if !cfg!(debug_assertions) {
        return;
    }
    app.init_resource::<PipelineFailures>()
        .add_systems(Update, fail);
    let failures = app.world().resource::<PipelineFailures>().clone();
    // Release frame measurements retain error reporting without walking the
    // entire pipeline cache on every render frame.
    if app
        .world()
        .resource::<crate::assets::LaunchOptions>()
        .release_benchmark
    {
        return;
    }
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
    let failure = failures.0.lock().ok().and_then(|failure| failure.clone());
    if let Some(failure) = failure {
        if let Some(output) = &options.verify {
            let _ = std::fs::write(
                output.join("pipeline-failure.json"),
                serde_json::json!({"passed": false, "target": failure.target, "error": failure.error}).to_string(),
            );
        }
        error!(
            "Game verification failed: rendering error from {}: {}",
            failure.target, failure.error
        );
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
            *failure = Some(RenderFailure {
                target: "bevy_render::render_resource::pipeline_cache".into(),
                error: error.to_string(),
            });
            return;
        }
    }
}
