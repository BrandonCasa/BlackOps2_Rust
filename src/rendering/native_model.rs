//! Original mc_lit_sm_r0c0n0x0_q361191u static-model LIT draws.
//!
//! NativeWorldPlugin owns geometry upload and common scene/object extraction.
//! This renderer supplies the original model-lighting atlas and cb3 constants
//! for visible placements, then draws PACKED32 geometry in the opaque phase.
use super::{
    lighting::lighting_lookup_scale,
    model_lighting::ModelLightingAtlas,
    native::{
        NativeWorldGeometry, NativeWorldObject, NativeWorldScene, NativeWorldState,
        PreparedNativeGeometry,
    },
};
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    camera::visibility::{self, VisibilityClass},
    core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey},
    diagnostic::FrameCount,
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{SystemParamItem, lifetimeless::SRes},
    },
    mesh::VertexBufferLayout,
    platform::collections::HashMap,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::allocator::MeshSlabs,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
            RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
            ViewBinnedRenderPhases,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, texture_3d, texture_cube, uniform_buffer},
            *,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_component::SyncComponent,
        sync_world::MainEntity,
        texture::GpuImage,
        view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewTarget},
    },
    shader::load_shader_library,
};

/// Construct only after resolving the exact native LIT technique identity.
/// The selected placement's authored reflectionProbeIndex owns the cube input.
#[derive(Asset, TypePath, Clone)]
pub(crate) struct NativeModelMaterial {
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
    pub normal_map: Handle<Image>,
    pub normal_sampler: SamplerDescriptor<'static>,
    pub reflection_probe: Handle<Image>,
    pub reflection_sampler: SamplerDescriptor<'static>,
}

impl NativeModelMaterial {
    pub(crate) fn images(&self) -> impl Iterator<Item = &Handle<Image>> {
        [&self.color_map, &self.normal_map, &self.reflection_probe].into_iter()
    }
}

#[derive(Component, Clone)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<NativeModelSurface>)]
pub(crate) struct NativeModelSurface {
    pub geometry: Handle<NativeWorldGeometry>,
    pub material: Handle<NativeModelMaterial>,
    pub state: NativeWorldState,
    pub primary_byte_offset: u64,
    pub first_index: u32,
    pub index_count: u32,
    /// Main-world placement owner, shared by every surface and LOD of this model.
    pub placement: Entity,
    pub colors_index: usize,
    pub visibility: u8,
}

#[derive(Component)]
pub(crate) struct ExtractedNativeModelSurface {
    geometry: AssetId<NativeWorldGeometry>,
    material: AssetId<NativeModelMaterial>,
    state: NativeWorldState,
    primary_byte_offset: u64,
    first_index: u32,
    index_count: u32,
    placement: MainEntity,
    colors_index: usize,
    visibility: u8,
}

impl SyncComponent<RenderApp> for NativeModelSurface {
    type Target = ExtractedNativeModelSurface;
}

impl ExtractComponent<RenderApp> for NativeModelSurface {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = ExtractedNativeModelSurface;

    fn extract_component(surface: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(ExtractedNativeModelSurface {
            geometry: surface.geometry.id(),
            material: surface.material.id(),
            state: surface.state,
            primary_byte_offset: surface.primary_byte_offset,
            first_index: surface.first_index,
            index_count: surface.index_count,
            placement: surface.placement.into(),
            colors_index: surface.colors_index,
            visibility: surface.visibility,
        })
    }
}

#[derive(ShaderType)]
struct NativeModelScene {
    world: NativeWorldScene,
    lighting_lookup_scale: Vec4,
}

/// Original cb3 model layout: 176 bytes before dynamic-offset alignment.
#[derive(ShaderType)]
struct NativeModelObject {
    world_rows: [Vec4; 4],
    grid_lighting_coords_and_vis: Vec4,
    grid_lighting_sh0: Vec4,
    grid_lighting_sh1: Vec4,
    grid_lighting_sh2: Vec4,
    reflection_lighting_sh0: Vec4,
    reflection_lighting_sh1: Vec4,
    reflection_lighting_sh2: Vec4,
}

pub(crate) struct NativeModelPlugin;

impl Plugin for NativeModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<NativeModelMaterial>().add_plugins((
            ExtractComponentPlugin::<NativeModelSurface>::default(),
            RenderAssetPlugin::<PreparedModelMaterial, GpuImage>::default(),
        ));
        load_shader_library!(app, "native_model.wesl");
        embedded_asset!(app, "native_model_entry.wesl");
        let shader = load_embedded_asset!(app, "native_model_entry.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(ModelPipeline::new(shader))
            .init_resource::<SpecializedRenderPipelines<ModelPipeline>>()
            .init_resource::<ModelUniforms>()
            .init_resource::<ModelBindGroups>()
            .add_render_command::<Opaque3d, (SetItemPipeline, DrawNativeModel)>()
            .add_systems(
                Render,
                (
                    queue_models.in_set(RenderSystems::Queue),
                    prepare_model_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Resource)]
struct ModelPipeline {
    shader: Handle<Shader>,
    scene_layout: BindGroupLayoutDescriptor,
    object_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    lighting_layout: BindGroupLayoutDescriptor,
}

impl ModelPipeline {
    fn new(shader: Handle<Shader>) -> Self {
        Self {
            shader,
            scene_layout: BindGroupLayoutDescriptor::new(
                "native model scene",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX_FRAGMENT,
                    uniform_buffer::<NativeModelScene>(true),
                ),
            ),
            object_layout: BindGroupLayoutDescriptor::new(
                "native model object",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX,
                    uniform_buffer::<NativeModelObject>(true),
                ),
            ),
            material_layout: BindGroupLayoutDescriptor::new(
                "native model textures",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_cube(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
            lighting_layout: BindGroupLayoutDescriptor::new(
                "native model lighting atlas",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_3d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ModelPipelineKey {
    target: TextureFormat,
    samples: u32,
    state: NativeWorldState,
}

impl SpecializedRenderPipeline for ModelPipeline {
    type Key = ModelPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("T6 original light-probe static model".into()),
            layout: vec![
                self.scene_layout.clone(),
                self.object_layout.clone(),
                self.material_layout.clone(),
                self.lighting_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                buffers: vec![VertexBufferLayout {
                    array_stride: 32,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Unorm8x4,
                            offset: 16,
                            shader_location: 1,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float16x2,
                            offset: 20,
                            shader_location: 2,
                        },
                        VertexAttribute {
                            format: VertexFormat::Unorm10_10_10_2,
                            offset: 24,
                            shader_location: 3,
                        },
                        VertexAttribute {
                            format: VertexFormat::Unorm10_10_10_2,
                            offset: 28,
                            shader_location: 4,
                        },
                    ],
                }],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                entry_point: Some(
                    if key.target.is_srgb() {
                        "fragment_srgb"
                    } else {
                        "fragment"
                    }
                    .into(),
                ),
                targets: vec![Some(ColorTargetState {
                    format: key.target,
                    blend: None,
                    write_mask: key.state.color_write,
                })],
                ..default()
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: key.state.front_face,
                cull_mode: key.state.cull_mode,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32FloatStencil8,
                depth_write_enabled: Some(key.state.depth_write),
                depth_compare: Some(key.state.depth_compare),
                stencil: default(),
                bias: default(),
            }),
            multisample: MultisampleState {
                count: key.samples,
                ..default()
            },
            ..default()
        }
    }
}

struct PreparedModelMaterial {
    bind_group: BindGroup,
}

impl RenderAsset for PreparedModelMaterial {
    type SourceAsset = NativeModelMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<PipelineCache>,
        SRes<ModelPipeline>,
        SRes<RenderAssets<GpuImage>>,
    );

    fn prepare_asset(
        source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (device, cache, pipeline, images): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let (Some(color), Some(normal), Some(probe)) = (
            images.get(source.color_map.id()),
            images.get(source.normal_map.id()),
            images.get(source.reflection_probe.id()),
        ) else {
            return Err(PrepareAssetError::RetryNextUpdate(source));
        };
        for image in [color, normal, probe] {
            assert!(
                !image.texture_descriptor.format.is_srgb(),
                "native model inputs require original encoded UNORM texture reads"
            );
        }
        assert!(
            source.color_sampler.compare.is_none()
                && source.normal_sampler.compare.is_none()
                && source.reflection_sampler.compare.is_none(),
            "native model material inputs require ordinary source samplers"
        );
        let color_sampler = device.create_sampler(&source.color_sampler);
        let normal_sampler = device.create_sampler(&source.normal_sampler);
        let reflection_sampler = device.create_sampler(&source.reflection_sampler);
        let bind_group = device.create_bind_group(
            "native model material",
            &cache.get_bind_group_layout(&pipeline.material_layout),
            &BindGroupEntries::sequential((
                &color.texture_view,
                &color_sampler,
                &normal.texture_view,
                &normal_sampler,
                &probe.texture_view,
                &reflection_sampler,
            )),
        );
        Ok(Self { bind_group })
    }
}

#[derive(Resource, Default)]
struct ModelUniforms {
    scenes: DynamicUniformBuffer<NativeModelScene>,
    objects: DynamicUniformBuffer<NativeModelObject>,
    scene_offsets: HashMap<RetainedViewEntity, u32>,
    object_offsets: HashMap<MainEntity, u32>,
}

#[derive(Resource, Default)]
struct ModelBindGroups {
    scene: Option<(BufferId, BindGroup)>,
    object: Option<(BufferId, BindGroup)>,
    lighting: Option<(TextureViewId, BindGroup)>,
}

fn prepare_model_groups(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<ModelPipeline>,
    mut uniforms: ResMut<ModelUniforms>,
    atlas: Option<Res<ModelLightingAtlas>>,
    mut groups: ResMut<ModelBindGroups>,
) {
    let Some(atlas) = atlas.filter(|_| !uniforms.objects.is_empty()) else {
        groups.scene = None;
        groups.object = None;
        groups.lighting = None;
        return;
    };
    uniforms.scenes.write_buffer(&device, &queue);
    uniforms.objects.write_buffer(&device, &queue);
    let scene_buffer = uniforms
        .scenes
        .buffer()
        .expect("uploaded native model scenes");
    if groups
        .scene
        .as_ref()
        .is_none_or(|(id, _)| *id != scene_buffer.id())
    {
        groups.scene = Some((
            scene_buffer.id(),
            device.create_bind_group(
                "native model scene uniforms",
                &cache.get_bind_group_layout(&pipeline.scene_layout),
                &BindGroupEntries::single(
                    uniforms
                        .scenes
                        .binding()
                        .expect("uploaded native model scene binding"),
                ),
            ),
        ));
    }
    let object_buffer = uniforms
        .objects
        .buffer()
        .expect("uploaded native model objects");
    if groups
        .object
        .as_ref()
        .is_none_or(|(id, _)| *id != object_buffer.id())
    {
        groups.object = Some((
            object_buffer.id(),
            device.create_bind_group(
                "native model object uniforms",
                &cache.get_bind_group_layout(&pipeline.object_layout),
                &BindGroupEntries::single(
                    uniforms
                        .objects
                        .binding()
                        .expect("uploaded native model object binding"),
                ),
            ),
        ));
    }
    if groups
        .lighting
        .as_ref()
        .is_none_or(|(id, _)| *id != atlas.view.id())
    {
        groups.lighting = Some((
            atlas.view.id(),
            device.create_bind_group(
                "native model atlas",
                &cache.get_bind_group_layout(&pipeline.lighting_layout),
                &BindGroupEntries::sequential((&atlas.view, &atlas.sampler)),
            ),
        ));
    }
}

fn queue_models(
    pipeline: Res<ModelPipeline>,
    cache: Res<PipelineCache>,
    mut variants: ResMut<SpecializedRenderPipelines<ModelPipeline>>,
    mut phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    geometries: Res<RenderAssets<PreparedNativeGeometry>>,
    materials: Res<RenderAssets<PreparedModelMaterial>>,
    views: Query<(
        &ExtractedView,
        &RenderVisibleEntities,
        &ViewTarget,
        &Msaa,
        &NativeWorldScene,
    )>,
    surfaces: Query<(&ExtractedNativeModelSurface, &NativeWorldObject)>,
    atlas: Option<ResMut<ModelLightingAtlas>>,
    frame: Res<FrameCount>,
    mut uniforms: ResMut<ModelUniforms>,
    mut queued: Local<HashMap<RetainedViewEntity, Vec<MainEntity>>>,
) {
    queued.retain(|view, entities| {
        let Some(phase) = phases.get_mut(view) else {
            return false;
        };
        for entity in entities.drain(..) {
            phase.remove(entity);
        }
        true
    });
    uniforms.scenes.clear();
    uniforms.objects.clear();
    uniforms.scene_offsets.clear();
    uniforms.object_offsets.clear();
    let Some(mut atlas) = atlas else {
        return;
    };
    let draw_function = draw_functions
        .read()
        .id::<(SetItemPipeline, DrawNativeModel)>();
    for (view, visible, target, msaa, scene) in &views {
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(visible) = visible.get::<NativeModelSurface>() else {
            continue;
        };
        let scene_offset = uniforms.scenes.push(&NativeModelScene {
            world: *scene,
            lighting_lookup_scale: lighting_lookup_scale(),
        });
        uniforms
            .scene_offsets
            .insert(view.retained_view_entity, scene_offset);
        let queued = queued.entry(view.retained_view_entity).or_default();
        for (&entity, &main_entity) in visible.iter_visible() {
            let Ok((surface, source_object)) = surfaces.get(entity) else {
                continue;
            };
            if surface.index_count == 0 {
                continue;
            }
            let (Some(geometry), Some(_)) = (
                geometries.get(surface.geometry),
                materials.get(surface.material),
            ) else {
                continue;
            };
            assert!(
                surface.primary_byte_offset.is_multiple_of(32)
                    && surface.primary_byte_offset < geometry.primary.size(),
                "native static model requires its complete PACKED32 vertex range"
            );
            assert!(
                surface.index_count.is_multiple_of(3)
                    && surface
                        .first_index
                        .checked_add(surface.index_count)
                        .is_some_and(|end| u64::from(end) * 2 <= geometry.indices.size()),
                "native static model draw must reference complete source Uint16 triangles"
            );
            if !uniforms.object_offsets.contains_key(&surface.placement) {
                let Ok(lighting) = atlas.touch(
                    surface.placement,
                    surface.colors_index,
                    surface.visibility,
                    frame.0,
                ) else {
                    // Native slot exhaustion rejects this draw. No alternate
                    // ambient color or another placement's slot is substituted.
                    continue;
                };
                let offset = uniforms.objects.push(&NativeModelObject {
                    world_rows: source_object.world_rows,
                    grid_lighting_coords_and_vis: lighting.grid_coords_and_vis,
                    grid_lighting_sh0: lighting.sh[0],
                    grid_lighting_sh1: lighting.sh[1],
                    grid_lighting_sh2: lighting.sh[2],
                    reflection_lighting_sh0: source_object.reflection_lighting_sh0,
                    reflection_lighting_sh1: source_object.reflection_lighting_sh1,
                    reflection_lighting_sh2: source_object.reflection_lighting_sh2,
                });
                uniforms.object_offsets.insert(surface.placement, offset);
            }
            let pipeline = variants.specialize(
                &cache,
                &pipeline,
                ModelPipelineKey {
                    target: target.main_texture_format(),
                    samples: msaa.samples(),
                    state: surface.state,
                },
            );
            phase.add(
                Opaque3dBatchSetKey {
                    pipeline,
                    draw_function,
                    material_bind_group_index: None,
                    slabs: MeshSlabs::default(),
                    lightmap_slab: None,
                },
                Opaque3dBinKey {
                    asset_id: surface.geometry.untyped(),
                },
                (entity, main_entity),
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
            );
            queued.push(main_entity);
        }
    }
}

struct DrawNativeModel;

impl<P: PhaseItem> RenderCommand<P> for DrawNativeModel {
    type Param = (
        SRes<ModelUniforms>,
        SRes<ModelBindGroups>,
        SRes<RenderAssets<PreparedModelMaterial>>,
        SRes<RenderAssets<PreparedNativeGeometry>>,
    );
    type ViewQuery = &'static ExtractedView;
    type ItemQuery = &'static ExtractedNativeModelSurface;

    fn render<'w>(
        _: &P,
        view: ROQueryItem<'w, '_, Self::ViewQuery>,
        surface: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (uniforms, groups, materials, geometries): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(surface) = surface else {
            return RenderCommandResult::Skip;
        };
        let (uniforms, groups, materials, geometries) = (
            uniforms.into_inner(),
            groups.into_inner(),
            materials.into_inner(),
            geometries.into_inner(),
        );
        let (
            Some((_, scene)),
            Some((_, object)),
            Some((_, lighting)),
            Some(material),
            Some(geometry),
            Some(&scene_offset),
            Some(&object_offset),
        ) = (
            &groups.scene,
            &groups.object,
            &groups.lighting,
            materials.get(surface.material),
            geometries.get(surface.geometry),
            uniforms.scene_offsets.get(&view.retained_view_entity),
            uniforms.object_offsets.get(&surface.placement),
        )
        else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(0, scene, &[scene_offset]);
        pass.set_bind_group(1, object, &[object_offset]);
        pass.set_bind_group(2, &material.bind_group, &[]);
        pass.set_bind_group(3, lighting, &[]);
        pass.set_vertex_buffer(0, geometry.primary.slice(surface.primary_byte_offset..));
        pass.set_index_buffer(geometry.indices.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(
            surface.first_index..surface.first_index + surface.index_count,
            0,
            0..1,
        );
        RenderCommandResult::Success
    }
}
