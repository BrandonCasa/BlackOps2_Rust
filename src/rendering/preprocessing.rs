use bevy::{
    pbr::RenderMeshInstances,
    prelude::*,
    render::{
        RenderApp, RenderStartup,
        batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

#[derive(Resource, Clone, Default)]
pub(crate) struct PreprocessingStatus(Arc<AtomicU8>);

impl PreprocessingStatus {
    pub(crate) fn report(&self) -> serde_json::Value {
        let mode = match self.0.load(Ordering::Relaxed) {
            1 => "cpu",
            2 => "gpu_preprocessing",
            3 => "gpu_culling",
            _ => "uninitialized",
        };
        serde_json::json!({"active_mode": mode})
    }
}

pub(super) struct PreprocessingPlugin;

impl Plugin for PreprocessingPlugin {
    fn build(&self, app: &mut App) {
        let status = PreprocessingStatus::default();
        app.insert_resource(status.clone());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.insert_resource(status);
            render.add_systems(RenderStartup, report.after(bevy::pbr::MeshPipelineSystems));
        }
    }
}

fn report(
    support: Res<GpuPreprocessingSupport>,
    instances: Res<RenderMeshInstances>,
    status: Res<PreprocessingStatus>,
) {
    let mode = match (&*instances, support.max_supported_mode) {
        (RenderMeshInstances::CpuBuilding(_), _) => 1,
        (_, GpuPreprocessingMode::Culling) => 3,
        (_, GpuPreprocessingMode::PreprocessingOnly) => 2,
        (_, GpuPreprocessingMode::None) => 1,
    };
    status.0.store(mode, Ordering::Relaxed);
    info!("Mesh preprocessing: {}", status.report());
}
