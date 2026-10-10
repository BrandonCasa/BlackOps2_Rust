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
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Clone)]
struct PipelineReadiness(Arc<AtomicUsize>);
impl Default for PipelineReadiness {
    fn default() -> Self {
        Self(Arc::new(AtomicUsize::new(usize::MAX)))
    }
}

#[derive(Resource, Default, Clone)]
pub(super) struct PipelineFailures(pub Arc<Mutex<Option<RenderFailure>>>);

#[derive(Clone, serde::Serialize)]
pub(super) struct RenderFailure {
    target: String,
    error: String,
}

pub(crate) fn error_layer(app: &mut App) -> Option<BoxedLayer> {
    if app
        .world()
        .resource::<crate::assets::LaunchOptions>()
        .verify
        .is_none()
        && app
            .world()
            .resource::<crate::assets::LaunchOptions>()
            .validation_remote
            .is_none()
    {
        return None;
    }
    app.init_resource::<PipelineReadiness>()
        .init_resource::<PipelineFailures>();
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
    app.init_resource::<PipelineReadiness>()
        .init_resource::<PipelineFailures>()
        .add_systems(Update, fail);
    let failures = app.world().resource::<PipelineFailures>().clone();
    if app
        .world()
        .resource::<crate::assets::LaunchOptions>()
        .release_benchmark
    {
        return;
    }
    let readiness = app.world().resource::<PipelineReadiness>().clone();
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(readiness)
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

fn inspect(
    cache: Res<PipelineCache>,
    failures: Res<PipelineFailures>,
    readiness: Res<PipelineReadiness>,
) {
    readiness.0.store(
        cache
            .pipelines()
            .filter(|p| !matches!(p.state, CachedPipelineState::Ok(_)))
            .count(),
        Ordering::Relaxed,
    );
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

#[cfg(all(debug_assertions, feature = "dev-dynamic"))]
pub(super) fn snapshot(world: &World) -> serde_json::Value {
    serde_json::json!({"pending":world.resource::<PipelineReadiness>().0.load(Ordering::Relaxed),"error":world.resource::<PipelineFailures>().0.lock().ok().and_then(|v|v.clone())})
}
