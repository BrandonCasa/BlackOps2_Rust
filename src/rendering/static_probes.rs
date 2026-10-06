//! Filter immutable retained cubes once with Bevy, then retain its output images.
use bevy::{
    light::{EnvironmentMapLight, GeneratedEnvironmentMapLight},
    pbr::generate::{GeneratorBindGroups, GeneratorPipelines, filtering_system},
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems, render_resource::PipelineCache, sync_world::MainEntity,
    },
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

#[derive(Resource, Default, Clone)]
struct FilteredProbes(Arc<Mutex<HashSet<Entity>>>);

pub(super) struct StaticProbePlugin;
impl Plugin for StaticProbePlugin {
    fn build(&self, app: &mut App) {
        let feedback = FilteredProbes::default();
        app.insert_resource(feedback.clone())
            .add_systems(Update, freeze);
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.insert_resource(feedback).add_systems(
                Render,
                filtered
                    .after(filtering_system)
                    .before(RenderSystems::Render),
            );
        }
    }
}

fn filtered(
    probes: Query<&MainEntity, With<GeneratorBindGroups>>,
    pipelines: Option<Res<GeneratorPipelines>>,
    cache: Res<PipelineCache>,
    feedback: Res<FilteredProbes>,
) {
    let Some(pipelines) = pipelines else { return };
    if [
        pipelines.copy,
        pipelines.downsample_first,
        pipelines.downsample_second,
        pipelines.radiance,
        pipelines.irradiance,
    ]
    .into_iter()
    .any(|id| cache.get_compute_pipeline(id).is_none())
    {
        return;
    }
    // All generator pipelines and per-cube bindings were ready for Bevy's
    // preceding filter systems. Their writes are ordered before subsequent draws.
    if let Ok(mut completed) = feedback.0.lock() {
        completed.extend(probes.iter().map(|entity| entity.id()));
    }
}

fn freeze(
    mut commands: Commands,
    probes: Query<
        Entity,
        (
            With<super::opaque::ProbeIndex>,
            With<GeneratedEnvironmentMapLight>,
            With<EnvironmentMapLight>,
        ),
    >,
    feedback: Res<FilteredProbes>,
) {
    let Ok(completed) = feedback.0.lock() else {
        return;
    };
    for entity in &probes {
        if completed.contains(&entity) {
            // SyncComponentPlugin removes the generator's render-world target.
            // EnvironmentMapLight and material handles keep the filtered images.
            commands
                .entity(entity)
                .remove::<GeneratedEnvironmentMapLight>();
            info!("Retained probe {entity:?} filtered once and frozen");
        }
    }
}
