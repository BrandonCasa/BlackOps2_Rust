use bevy::{
    pbr::{MeshInputUniform, MeshPipeline, MeshUniform, RenderMeshInstances},
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        batching::gpu_preprocessing::{
            BatchedInstanceBuffers, GpuPreprocessingMode, GpuPreprocessingSupport,
            write_batched_instance_buffers,
        },
        render_resource::ShaderType,
        renderer::RenderDevice,
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
            render.add_systems(
                Render,
                reserve_mesh_uniforms
                    .in_set(RenderSystems::PrepareResourcesFlush)
                    .after(write_batched_instance_buffers::<MeshPipeline>),
            );
        }
    }
}

fn mesh_uniform_capacity(count: usize) -> usize {
    (count.max(1) * u64::from(MeshUniform::min_size()) as usize).div_ceil(size_of::<MeshUniform>())
}

fn reserve_mesh_uniforms(
    device: Res<RenderDevice>,
    buffers: Option<ResMut<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>>,
) {
    let Some(mut buffers) = buffers else {
        return;
    };
    for phase in buffers.phase_instance_buffers.values_mut() {
        let capacity = mesh_uniform_capacity(phase.data_buffer.len());
        phase.data_buffer.reserve(capacity, &device);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesh_uniform_allocations_cover_shader_stride_at_session_transitions() {
        let shader_stride = u64::from(MeshUniform::min_size()) as usize;
        for count in [0, 1, 2, 21, 22, 1000, 65536] {
            let bytes = mesh_uniform_capacity(count) * size_of::<MeshUniform>();
            let required = count.max(1) * shader_stride;
            assert!(bytes >= required);
            assert!(bytes - required < size_of::<MeshUniform>());
        }
        assert_eq!(shader_stride, 176);
    }
}
