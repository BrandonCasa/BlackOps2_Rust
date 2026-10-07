//! Bevy owns instance extraction, compute preprocessing and device fallback.
use crate::assets::MeshPreprocessing;
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
        let requested = app
            .world()
            .resource::<crate::assets::LaunchOptions>()
            .mesh_preprocessing;
        let status = PreprocessingStatus::default();
        app.insert_resource(status.clone());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.insert_resource(status);
            render
                .insert_resource(RequestedMode(requested))
                .add_systems(
                    RenderStartup,
                    configure
                        .after(bevy::render::init_gpu_resource::<GpuPreprocessingSupport>)
                        .before(bevy::pbr::MeshPipelineSystems),
                );
            render.add_systems(RenderStartup, report.after(bevy::pbr::MeshPipelineSystems));
        }
    }
}

#[derive(Resource)]
struct RequestedMode(MeshPreprocessing);

fn configure(mut support: ResMut<GpuPreprocessingSupport>, requested: Res<RequestedMode>) {
    // The explicit CPU comparison must use CPU phase batches as well as CPU
    // instance extraction; changing only PbrPlugin would leave them mismatched.
    support.max_supported_mode = support.min(match requested.0 {
        MeshPreprocessing::Cpu => GpuPreprocessingMode::None,
        // Diagnostic comparison: keep visibility/direct batching while moving
        // mesh-uniform expansion to compute.
        MeshPreprocessing::Gpu => GpuPreprocessingMode::PreprocessingOnly,
        // Native indirect batching is the default on supported devices.
        // CPU visibility admission bounds the work lists. Native GPU frustum
        // culling/indirect batching retains the preprocessing/CPU fallback;
        // matched ordinary workloads rejected two-phase occlusion overhead.
        MeshPreprocessing::GpuCulling => GpuPreprocessingMode::Culling,
    });
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
