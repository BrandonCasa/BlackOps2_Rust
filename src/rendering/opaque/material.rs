//! Compatibility with the pinned native extended-material slab lookup.
use super::OpaqueSurface;
use bevy::{
    asset::{UntypedAssetId, VisitAssetDependencies},
    ecs::system::SystemParamItem,
    material::OpaqueRendererMethod,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{ExtendedMaterial, MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
    shader::ShaderRef,
};

type Native = ExtendedMaterial<StandardMaterial, OpaqueSurface>;

#[derive(TypePath, Debug, Clone, Deref, DerefMut)]
pub(in crate::rendering) struct OpaqueMaterial(pub Native);
impl Asset for OpaqueMaterial {}
impl VisitAssetDependencies for OpaqueMaterial {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        self.base.visit_dependencies(visit);
        self.extension.visit_dependencies(visit);
    }
}

impl AsBindGroup for OpaqueMaterial {
    type Data = <Native as AsBindGroup>::Data;
    type Param = <Native as AsBindGroup>::Param;
    fn bindless_slot_count() -> Option<BindlessSlabResourceLimit> {
        Native::bindless_slot_count()
    }
    fn bindless_supported(device: &RenderDevice) -> bool {
        Native::bindless_supported(device)
    }
    fn label() -> &'static str {
        Native::label()
    }
    fn bind_group_data(&self) -> Self::Data {
        self.0.bind_group_data()
    }
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        param: &mut SystemParamItem<'_, '_, Self::Param>,
        force_no_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        self.0
            .unprepared_bind_group(layout, device, param, force_no_bindless)
    }
    fn bind_group_layout_entries(
        device: &RenderDevice,
        force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry> {
        Native::bind_group_layout_entries(device, force_no_bindless)
    }
    fn bindless_descriptor() -> Option<BindlessDescriptor> {
        let mut descriptor = Native::bindless_descriptor()?;
        // Bevy 0.19.1 MaterialBindlessSlab::get_bindless_index_table compares
        // the sought index to each table in the opposite binary-search order.
        // With ascending ExtendedMaterial tables it misses the base range,
        // leaking texture references on free. Descending ranges match that
        // pinned lookup. Table binding numbers, shader layouts and resources
        // remain native; remove this compatibility after an upstream repair.
        let mut tables = descriptor.index_tables.into_owned();
        tables.sort_by(|a, b| b.indices.start.cmp(&a.indices.start));
        descriptor.index_tables = tables.into();
        Some(descriptor)
    }
}

impl Material for OpaqueMaterial {
    fn vertex_shader() -> ShaderRef {
        Native::vertex_shader()
    }
    fn fragment_shader() -> ShaderRef {
        Native::fragment_shader()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.0.alpha_mode()
    }
    fn opaque_render_method(&self) -> OpaqueRendererMethod {
        self.0.opaque_render_method()
    }
    fn depth_bias(&self) -> f32 {
        self.0.depth_bias()
    }
    fn reads_view_transmission_texture(&self) -> bool {
        self.0.reads_view_transmission_texture()
    }
    fn enable_prepass() -> bool {
        Native::enable_prepass()
    }
    fn enable_shadows() -> bool {
        Native::enable_shadows()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        Native::prepass_vertex_shader()
    }
    fn prepass_fragment_shader() -> ShaderRef {
        Native::prepass_fragment_shader()
    }
    fn deferred_vertex_shader() -> ShaderRef {
        Native::deferred_vertex_shader()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        Native::deferred_fragment_shader()
    }
    fn specialize(
        pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        Native::specialize(
            pipeline,
            descriptor,
            layout,
            MaterialPipelineKey {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data,
            },
        )
    }
}
