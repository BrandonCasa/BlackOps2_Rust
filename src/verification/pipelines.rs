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
#[derive(Resource, Default, Clone)]
struct MeshConsistency(Arc<Mutex<serde_json::Value>>);
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
    let mesh_consistency = app
        .world()
        .resource::<crate::assets::LaunchOptions>()
        .validation_remote
        .is_some()
        .then(MeshConsistency::default);
    if let Some(consistency) = &mesh_consistency {
        app.insert_resource(consistency.clone());
    }
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(readiness)
            .insert_resource(failures)
            .add_systems(Render, inspect.after(RenderSystems::Render));
        if let Some(consistency) = mesh_consistency {
            render
                .insert_resource(consistency)
                .add_systems(Render, inspect_meshes.after(RenderSystems::Render));
        }
    }
}

fn inspect_meshes(
    consistency: Res<MeshConsistency>,
    instances: Res<bevy::pbr::RenderMeshInstances>,
    skins: Res<bevy::pbr::SkinUniforms>,
    allocator: Res<bevy::render::mesh::allocator::MeshAllocator>,
    buffers: Option<
        Res<
            bevy::render::batching::gpu_preprocessing::BatchedInstanceBuffers<
                bevy::pbr::MeshUniform,
                bevy::pbr::MeshInputUniform,
            >,
        >,
    >,
) {
    let Some(buffers) = buffers else { return };
    let mut checked = 0;
    let mut mismatches = Vec::new();
    for &entity in skins.all_skins() {
        let Some(queue) = instances.render_mesh_queue_data(entity) else {
            continue;
        };
        let Some(input) = buffers
            .current_input_buffer
            .get(queue.current_uniform_index.0)
        else {
            continue;
        };
        let Some(asset) = instances.mesh_asset_id(entity) else {
            continue;
        };
        let Some(vertices) = allocator.mesh_vertex_slice(&asset) else {
            continue;
        };
        let indices = allocator.mesh_index_slice(&asset);
        let expected_skin = skins.skin_index(entity);
        checked += 1;
        if input.first_vertex_index != vertices.range.start
            || indices
                .as_ref()
                .is_some_and(|i| input.first_index_index != i.range.start)
            || expected_skin != Some(input.current_skin_index)
        {
            mismatches.push(serde_json::json!({"entity":entity.to_bits(),
                "vertex":input.first_vertex_index,"expected_vertex":vertices.range.start,
                "index":input.first_index_index,"expected_index":indices.map(|i|i.range.start),
                "skin":input.current_skin_index,"expected_skin":expected_skin}));
        }
    }
    if let Ok(mut report) = consistency.0.lock() {
        let frames = report["frames"].as_u64().unwrap_or(0) + 1;
        let mismatch_frames =
            report["mismatch_frames"].as_u64().unwrap_or(0) + u64::from(!mismatches.is_empty());
        let first = report
            .get("first_mismatches")
            .cloned()
            .filter(|v| !v.is_null())
            .unwrap_or_else(|| {
                if mismatches.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::json!(mismatches)
                }
            });
        *report = serde_json::json!({"frames":frames,"checked":checked,"mismatch_frames":mismatch_frames,
            "mismatches":mismatches,"first_mismatches":first});
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

#[cfg(all(debug_assertions, feature = "dev-tools"))]
pub(super) fn snapshot(world: &World) -> serde_json::Value {
    serde_json::json!({"pending":world.resource::<PipelineReadiness>().0.load(Ordering::Relaxed),"error":world.resource::<PipelineFailures>().0.lock().ok().and_then(|v|v.clone()),
        "mesh_consistency":world.get_resource::<MeshConsistency>().and_then(|r|r.0.lock().ok().map(|v|v.clone()))})
}
