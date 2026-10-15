//! Original particle shader families in Bevy's transparent 3D phase.
use super::native::NativeWorldScene;
use bevy::{
    asset::{RenderAssetUsages, embedded_asset, load_embedded_asset},
    camera::visibility::{self, VisibilityClass},
    core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d},
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{SystemParamItem, lifetimeless::SRes},
    },
    mesh::VertexBufferLayout,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::{
            AssetExtractionError, PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets,
        },
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_component::SyncComponent,
        texture::GpuImage,
        uniform::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        view::{ExtractedView, RenderVisibleEntities, ViewTarget},
    },
    shader::load_shader_library,
};

/// Required camera input. float_z must contain the original zNear/viewDepth
/// encoding; this renderer never substitutes a regular forward-Z depth image.
/// The caller supplies the recovered code sampler and original scene constants.
#[derive(Component, Clone)]
pub struct NativeEffectView {
    pub float_z: Handle<Image>,
    pub float_z_sampler: SamplerDescriptor<'static>,
    pub z_near: Vec4,
    pub render_target_size: Vec4,
}

#[derive(Component, Clone, Copy, ShaderType)]
pub struct NativeFxFrame {
    scene: NativeWorldScene,
    z_near: Vec4,
    render_target_size: Vec4,
}

#[derive(Component)]
pub struct ExtractedEffectView {
    float_z: AssetId<Image>,
    sampler: SamplerDescriptor<'static>,
    filtering: bool,
}

fn filtering(descriptor: &SamplerDescriptor<'_>) -> bool {
    descriptor.mag_filter == FilterMode::Linear
        || descriptor.min_filter == FilterMode::Linear
        || descriptor.mipmap_filter == MipmapFilterMode::Linear
}

impl SyncComponent<RenderApp> for NativeEffectView {
    type Target = (
        NativeFxFrame,
        ExtractedEffectView,
        PreparedEffectView,
        PreparedCloudView,
    );
}

impl ExtractComponent<RenderApp> for NativeEffectView {
    type QueryData = (&'static Self, &'static NativeWorldScene);
    type QueryFilter = ();
    type Out = (NativeFxFrame, ExtractedEffectView);

    fn extract_component((view, scene): QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        assert!(
            view.z_near.x.is_finite() && view.z_near.x > 0.0,
            "native zfeather requires the original positive zNear.x"
        );
        assert!(
            view.render_target_size.is_finite()
                && view.render_target_size.z > 0.0
                && view.render_target_size.w > 0.0,
            "native zfeather requires the original reciprocal render-target size"
        );
        assert!(
            view.float_z_sampler.compare.is_none(),
            "native floatZ requires an ordinary source sampler, not a comparison sampler"
        );
        Some((
            NativeFxFrame {
                scene: *scene,
                z_near: view.z_near,
                render_target_size: view.render_target_size,
            },
            ExtractedEffectView {
                float_z: view.float_z.id(),
                sampler: view.float_z_sampler.clone(),
                filtering: filtering(&view.float_z_sampler),
            },
        ))
    }
}

/// Eye-relative source axes/inches, updated by the entity's existing owner.
#[derive(Component, Clone, Copy, ShaderType, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct NativeEffectTransform {
    pub world_rows: [Vec4; 4],
}

/// Original absolute native-inch outdoor lookup. The world owns this resource;
/// no default or fabricated mask is substituted for a required source image.
#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub(crate) struct NativeOutdoorMap {
    pub image: Handle<Image>,
    pub lookup_rows: [Vec4; 4],
    pub sampler: SamplerDescriptor<'static>,
}

/// The two original weather vertex programs use source cloud constants rather
/// than a color/UV quad mesh. world_rows and outdoor_rows transform SNORM xyz.
#[derive(Component, Clone, Copy, ShaderType, ExtractComponent)]
#[extract_app(RenderApp)]
pub(crate) struct NativeCloudObject {
    pub world_rows: [Vec4; 4],
    pub outdoor_rows: [Vec4; 3],
    pub view_right: Vec4,
    pub view_up: Vec4,
    pub cloud_matrix: Vec4,
    pub velocity_world: Vec4,
    pub color: Vec4,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, TypePath)]
pub enum NativeEffectProgram {
    Zfeather,
    CloudRain,
    CloudSnow,
}

impl NativeEffectProgram {
    fn cloud(self) -> bool {
        self != Self::Zfeather
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, TypePath)]
pub enum NativeEffectVertexLayout {
    Packed32,
    ParticleCloud8,
}

impl NativeEffectVertexLayout {
    fn stride(self) -> u64 {
        match self {
            Self::Packed32 => 32,
            Self::ParticleCloud8 => 8,
        }
    }
}

/// Source stencil operations share the native scene's depth/stencil attachment.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeEffectStencil {
    pub front: StencilFaceState,
    pub back: StencilFaceState,
    pub read_mask: u32,
    pub write_mask: u32,
    pub reference: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeEffectState {
    pub cull_mode: Option<Face>,
    pub front_face: FrontFace,
    pub stencil: Option<NativeEffectStencil>,
}

/// The owner resolves the exact original technique before constructing an asset.
#[derive(Asset, TypePath, Clone)]
pub struct NativeEffectMaterial {
    pub program: NativeEffectProgram,
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
    pub feather_parms: Vec4,
    pub state: NativeEffectState,
}

#[derive(ShaderType)]
struct NativeEffectConstants {
    feather_parms: Vec4,
}

/// Native PACKED32 vertices and little-endian Uint16 indices. Modified CPU
/// vectors are moved into extraction. Refill both vectors for the next update;
/// the prepared GPU buffers keep their allocation while capacity is sufficient.
#[derive(Asset, TypePath, Clone)]
pub struct NativeEffectGeometry {
    pub layout: NativeEffectVertexLayout,
    pub vertices: Vec<u8>,
    pub indices: Vec<u8>,
}

#[derive(Component, Clone)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<NativeEffectDraw>)]
pub struct NativeEffectDraw {
    pub geometry: Handle<NativeEffectGeometry>,
    pub material: Handle<NativeEffectMaterial>,
    pub vertex_byte_offset: u64,
    pub first_index: u32,
    pub index_count: u32,
    /// In the engine's world coordinates, for ordinary transparent depth sort.
    pub sort_center: Vec3,
}

#[derive(Component)]
pub struct ExtractedEffectDraw {
    geometry: AssetId<NativeEffectGeometry>,
    material: AssetId<NativeEffectMaterial>,
    vertex_byte_offset: u64,
    first_index: u32,
    index_count: u32,
    sort_center: Vec3,
}

impl SyncComponent<RenderApp> for NativeEffectDraw {
    type Target = ExtractedEffectDraw;
}

impl ExtractComponent<RenderApp> for NativeEffectDraw {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = ExtractedEffectDraw;

    fn extract_component(draw: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(ExtractedEffectDraw {
            geometry: draw.geometry.id(),
            material: draw.material.id(),
            vertex_byte_offset: draw.vertex_byte_offset,
            first_index: draw.first_index,
            index_count: draw.index_count,
            sort_center: draw.sort_center,
        })
    }
}

pub struct NativeEffectPlugin;

impl Plugin for NativeEffectPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<NativeEffectGeometry>()
            .init_asset::<NativeEffectMaterial>()
            .add_plugins((
                ExtractComponentPlugin::<NativeEffectView>::default(),
                ExtractComponentPlugin::<NativeEffectTransform>::default(),
                ExtractComponentPlugin::<NativeEffectDraw>::default(),
                ExtractComponentPlugin::<NativeCloudObject>::default(),
                ExtractResourcePlugin::<NativeOutdoorMap>::default(),
                UniformComponentPlugin::<NativeFxFrame>::default(),
                UniformComponentPlugin::<NativeEffectTransform>::default(),
                UniformComponentPlugin::<NativeCloudObject>::default(),
                RenderAssetPlugin::<PreparedEffectGeometry>::default(),
                RenderAssetPlugin::<PreparedEffectMaterial, GpuImage>::default(),
            ));
        load_shader_library!(app, "native_fx.wesl");
        load_shader_library!(app, "native_cloud.wesl");
        embedded_asset!(app, "native_fx_entry.wesl");
        let shader = load_embedded_asset!(app, "native_fx_entry.wesl");
        embedded_asset!(app, "native_cloud_entry.wesl");
        let cloud_shader = load_embedded_asset!(app, "native_cloud_entry.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(EffectPipeline::new(shader, cloud_shader))
            .init_resource::<SpecializedRenderPipelines<EffectPipeline>>()
            .init_resource::<EffectObjectGroup>()
            .add_render_command::<Transparent3d, (SetItemPipeline, DrawNativeEffect)>()
            .add_systems(
                Render,
                (
                    queue_effects.in_set(RenderSystems::Queue),
                    prepare_effect_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

struct PreparedEffectGeometry {
    layout: NativeEffectVertexLayout,
    vertices: Buffer,
    indices: Buffer,
    vertex_bytes: u64,
    index_count: u32,
}

impl RenderAsset for PreparedEffectGeometry {
    type SourceAsset = NativeEffectGeometry;
    type Param = (SRes<RenderDevice>, SRes<RenderQueue>);

    fn asset_usage(_: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }

    fn take_gpu_data(
        source: &mut Self::SourceAsset,
        _: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        if source.vertices.is_empty() {
            return Err(AssetExtractionError::AlreadyExtracted);
        }
        Ok(NativeEffectGeometry {
            layout: source.layout,
            vertices: std::mem::take(&mut source.vertices),
            indices: std::mem::take(&mut source.indices),
        })
    }

    fn byte_len(source: &Self::SourceAsset) -> Option<usize> {
        Some(source.vertices.len() + source.indices.len().next_multiple_of(4))
    }

    fn prepare_asset(
        mut source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (device, queue): &mut SystemParamItem<Self::Param>,
        previous: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        assert!(
            !source.vertices.is_empty()
                && source
                    .vertices
                    .len()
                    .is_multiple_of(source.layout.stride() as usize),
            "native effect vertices must preserve their complete source declaration"
        );
        assert!(
            !source.indices.is_empty() && source.indices.len().is_multiple_of(2),
            "native zfeather indices must contain complete Uint16 elements"
        );
        let vertex_bytes = source.vertices.len() as u64;
        let index_count = u32::try_from(source.indices.len() / 2)
            .expect("native zfeather index count exceeds Uint32");
        // Queue writes require four-byte alignment; this padding is outside all
        // source draw ranges and never becomes an additional source index.
        source
            .indices
            .resize(source.indices.len().next_multiple_of(4), 0);
        let upload = |label, bytes: &[u8], usage, previous: Option<&Buffer>| {
            if let Some(buffer) = previous.filter(|buffer| buffer.size() >= bytes.len() as u64) {
                queue.write_buffer(buffer, 0, bytes);
                buffer.clone()
            } else {
                device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some(label),
                    contents: bytes,
                    usage: usage | BufferUsages::COPY_DST,
                })
            }
        };
        Ok(Self {
            layout: source.layout,
            vertices: upload(
                "native effect vertices",
                &source.vertices,
                BufferUsages::VERTEX,
                previous.map(|previous| &previous.vertices),
            ),
            indices: upload(
                "native effect indices",
                &source.indices,
                BufferUsages::INDEX,
                previous.map(|previous| &previous.indices),
            ),
            vertex_bytes,
            index_count,
        })
    }
}

#[derive(Resource)]
struct EffectPipeline {
    shader: Handle<Shader>,
    cloud_shader: Handle<Shader>,
    scene_layouts: [BindGroupLayoutDescriptor; 2],
    object_layout: BindGroupLayoutDescriptor,
    cloud_object_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    cloud_material_layout: BindGroupLayoutDescriptor,
}

impl EffectPipeline {
    fn new(shader: Handle<Shader>, cloud_shader: Handle<Shader>) -> Self {
        let scene_layouts = [false, true].map(|filtering| {
            BindGroupLayoutDescriptor::new(
                "native effect scene",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        uniform_buffer::<NativeFxFrame>(true),
                        texture_2d(TextureSampleType::Float {
                            filterable: filtering,
                        }),
                        sampler(if filtering {
                            SamplerBindingType::Filtering
                        } else {
                            SamplerBindingType::NonFiltering
                        }),
                    ),
                ),
            )
        });
        Self {
            shader,
            cloud_shader,
            scene_layouts,
            object_layout: BindGroupLayoutDescriptor::new(
                "native effect transform",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX_FRAGMENT,
                    uniform_buffer::<NativeEffectTransform>(true),
                ),
            ),
            cloud_object_layout: BindGroupLayoutDescriptor::new(
                "native cloud constants",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX_FRAGMENT,
                    uniform_buffer::<NativeCloudObject>(true),
                ),
            ),
            material_layout: BindGroupLayoutDescriptor::new(
                "native effect material",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<NativeEffectConstants>(false),
                    ),
                ),
            ),
            cloud_material_layout: BindGroupLayoutDescriptor::new(
                "native cloud material",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct EffectPipelineKey {
    program: NativeEffectProgram,
    target: TextureFormat,
    samples: u32,
    depth_filtering: bool,
    state: NativeEffectState,
}

impl SpecializedRenderPipeline for EffectPipeline {
    type Key = EffectPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        assert!(
            !key.target.is_srgb(),
            "native zfeather requires encoded RGB blending in a non-sRGB main attachment"
        );
        let stencil = key
            .state
            .stencil
            .map(|stencil| StencilState {
                front: stencil.front,
                back: stencil.back,
                read_mask: stencil.read_mask,
                write_mask: stencil.write_mask,
            })
            .unwrap_or_default();
        let cloud = key.program.cloud();
        let shader = if cloud {
            &self.cloud_shader
        } else {
            &self.shader
        };
        let vertex_entry = match key.program {
            NativeEffectProgram::Zfeather => "vertex",
            NativeEffectProgram::CloudRain => "vertex_rain",
            NativeEffectProgram::CloudSnow => "vertex_snow",
        };
        let vertex_layout = if cloud {
            VertexBufferLayout {
                array_stride: 8,
                step_mode: VertexStepMode::Vertex,
                attributes: vec![VertexAttribute {
                    format: VertexFormat::Snorm16x4,
                    offset: 0,
                    shader_location: 0,
                }],
            }
        } else {
            VertexBufferLayout {
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
                ],
            }
        };
        RenderPipelineDescriptor {
            label: Some("T6 source particle effect".into()),
            layout: vec![
                self.scene_layouts[usize::from(key.depth_filtering)].clone(),
                if cloud {
                    self.cloud_object_layout.clone()
                } else {
                    self.object_layout.clone()
                },
                if cloud {
                    self.cloud_material_layout.clone()
                } else {
                    self.material_layout.clone()
                },
            ],
            vertex: VertexState {
                shader: shader.clone(),
                entry_point: Some(vertex_entry.into()),
                buffers: vec![vertex_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: key.target,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::OneMinusDstAlpha,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                cull_mode: key.state.cull_mode,
                front_face: key.state.front_face,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: super::frame::NATIVE_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil,
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

struct PreparedEffectMaterial {
    program: NativeEffectProgram,
    bind_group: BindGroup,
    state: NativeEffectState,
}

impl RenderAsset for PreparedEffectMaterial {
    type SourceAsset = NativeEffectMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<PipelineCache>,
        SRes<EffectPipeline>,
        SRes<RenderAssets<GpuImage>>,
    );

    fn prepare_asset(
        source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (device, queue, cache, pipeline, images): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let Some(image) = images.get(source.color_map.id()) else {
            return Err(PrepareAssetError::RetryNextUpdate(source));
        };
        assert!(
            !image.texture_descriptor.format.is_srgb(),
            "native zfeather color input must preserve original encoded texture reads"
        );
        assert!(
            source.color_sampler.compare.is_none(),
            "native zfeather colorMap requires an ordinary material sampler"
        );
        assert!(
            source.feather_parms.is_finite(),
            "native zfeather material constants must be finite"
        );
        let sampler = device.create_sampler(&source.color_sampler);
        let bind_group = if source.program.cloud() {
            device.create_bind_group(
                "native cloud material",
                &cache.get_bind_group_layout(&pipeline.cloud_material_layout),
                &BindGroupEntries::sequential((&image.texture_view, &sampler)),
            )
        } else {
            let mut constants = UniformBuffer::from(NativeEffectConstants {
                feather_parms: source.feather_parms,
            });
            constants.write_buffer(device, queue);
            device.create_bind_group(
                "native effect material",
                &cache.get_bind_group_layout(&pipeline.material_layout),
                &BindGroupEntries::sequential((
                    &image.texture_view,
                    &sampler,
                    constants
                        .binding()
                        .expect("uploaded native effect material constants"),
                )),
            )
        };
        Ok(Self {
            program: source.program,
            bind_group,
            state: source.state,
        })
    }
}

#[derive(Resource, Default)]
struct EffectObjectGroup {
    packed: Option<(BufferId, BindGroup)>,
    cloud: Option<(BufferId, BindGroup)>,
}

#[derive(Component)]
pub struct PreparedEffectView {
    frame_buffer: BufferId,
    depth_view: TextureViewId,
    sampler: SamplerDescriptor<'static>,
    bind_group: BindGroup,
}

#[derive(Component)]
pub struct PreparedCloudView {
    frame_buffer: BufferId,
    outdoor_view: TextureViewId,
    sampler: SamplerDescriptor<'static>,
    bind_group: BindGroup,
}

fn prepare_effect_groups(
    mut commands: Commands,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    pipeline: Res<EffectPipeline>,
    frames: Res<ComponentUniforms<NativeFxFrame>>,
    objects: Res<ComponentUniforms<NativeEffectTransform>>,
    clouds: Res<ComponentUniforms<NativeCloudObject>>,
    outdoor: Option<Res<NativeOutdoorMap>>,
    images: Res<RenderAssets<GpuImage>>,
    views: Query<(
        Entity,
        &ExtractedEffectView,
        Option<&PreparedEffectView>,
        Option<&PreparedCloudView>,
    )>,
    mut object_group: ResMut<EffectObjectGroup>,
) {
    if let (Some(buffer), Some(binding)) = (objects.buffer(), objects.binding()) {
        if object_group
            .packed
            .as_ref()
            .is_none_or(|(id, _)| *id != buffer.id())
        {
            object_group.packed = Some((
                buffer.id(),
                device.create_bind_group(
                    "native effect transforms",
                    &cache.get_bind_group_layout(&pipeline.object_layout),
                    &BindGroupEntries::single(binding),
                ),
            ));
        }
    }
    if let (Some(buffer), Some(binding)) = (clouds.buffer(), clouds.binding()) {
        if object_group
            .cloud
            .as_ref()
            .is_none_or(|(id, _)| *id != buffer.id())
        {
            object_group.cloud = Some((
                buffer.id(),
                device.create_bind_group(
                    "native cloud constants",
                    &cache.get_bind_group_layout(&pipeline.cloud_object_layout),
                    &BindGroupEntries::single(binding),
                ),
            ));
        }
    }
    let (Some(buffer), Some(binding)) = (frames.buffer(), frames.binding()) else {
        return;
    };
    for (entity, view, prepared, prepared_cloud) in &views {
        if let Some((outdoor, image)) = outdoor
            .as_ref()
            .and_then(|outdoor| images.get(outdoor.image.id()).map(|image| (outdoor, image)))
        {
            assert!(
                !image.texture_descriptor.format.is_srgb(),
                "original outdoor height requires UNORM numeric reads"
            );
            if !prepared_cloud.is_some_and(|prepared| {
                prepared.frame_buffer == buffer.id()
                    && prepared.outdoor_view == image.texture_view.id()
                    && prepared.sampler == outdoor.sampler
            }) {
                let sampler = device.create_sampler(&outdoor.sampler);
                let bind_group = device.create_bind_group(
                    "native cloud scene and outdoor map",
                    &cache.get_bind_group_layout(
                        &pipeline.scene_layouts[usize::from(filtering(&outdoor.sampler))],
                    ),
                    &BindGroupEntries::sequential((binding.clone(), &image.texture_view, &sampler)),
                );
                commands.entity(entity).insert(PreparedCloudView {
                    frame_buffer: buffer.id(),
                    outdoor_view: image.texture_view.id(),
                    sampler: outdoor.sampler.clone(),
                    bind_group,
                });
            }
        } else {
            commands.entity(entity).remove::<PreparedCloudView>();
        }
        let Some(image) = images.get(view.float_z) else {
            commands.entity(entity).remove::<PreparedEffectView>();
            continue;
        };
        assert!(
            image.texture_descriptor.sample_count == 1,
            "native zfeather requires the resolved single-sample floatZ image"
        );
        assert!(
            !image.texture_descriptor.format.is_srgb(),
            "native floatZ must preserve original numeric depth reads"
        );
        let sample_type = image
            .texture_descriptor
            .format
            .sample_type(Some(TextureAspect::All), Some(device.features()));
        assert!(
            matches!(sample_type, Some(TextureSampleType::Float { filterable })
            if filterable || !view.filtering),
            "native floatZ image format must support its original code sampler"
        );
        if prepared.is_some_and(|prepared| {
            prepared.frame_buffer == buffer.id()
                && prepared.depth_view == image.texture_view.id()
                && prepared.sampler == view.sampler
        }) {
            continue;
        }
        let sampler = device.create_sampler(&view.sampler);
        let bind_group = device.create_bind_group(
            "native effect scene and floatZ",
            &cache.get_bind_group_layout(&pipeline.scene_layouts[usize::from(view.filtering)]),
            &BindGroupEntries::sequential((binding.clone(), &image.texture_view, &sampler)),
        );
        commands.entity(entity).insert(PreparedEffectView {
            frame_buffer: buffer.id(),
            depth_view: image.texture_view.id(),
            sampler: view.sampler.clone(),
            bind_group,
        });
    }
}

fn queue_effects(
    pipeline: Res<EffectPipeline>,
    cache: Res<PipelineCache>,
    mut variants: ResMut<SpecializedRenderPipelines<EffectPipeline>>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    geometries: Res<RenderAssets<PreparedEffectGeometry>>,
    materials: Res<RenderAssets<PreparedEffectMaterial>>,
    images: Res<RenderAssets<GpuImage>>,
    views: Query<
        (
            &ExtractedView,
            &ExtractedEffectView,
            &RenderVisibleEntities,
            &ViewTarget,
            &Msaa,
            Option<&PreparedCloudView>,
        ),
        With<NativeWorldScene>,
    >,
    draws: Query<&ExtractedEffectDraw>,
) {
    let draw_function = draw_functions
        .read()
        .id::<(SetItemPipeline, DrawNativeEffect)>();
    for (view, effect_view, visible, target, msaa, cloud_view) in &views {
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(visible) = visible.get::<NativeEffectDraw>() else {
            continue;
        };
        for (&entity, &main_entity) in visible.iter_visible() {
            let Ok(draw) = draws.get(entity) else {
                continue;
            };
            if draw.index_count == 0 {
                continue;
            }
            let (Some(geometry), Some(material)) =
                (geometries.get(draw.geometry), materials.get(draw.material))
            else {
                continue;
            };
            let cloud = material.program.cloud();
            if (cloud && cloud_view.is_none())
                || (!cloud && images.get(effect_view.float_z).is_none())
            {
                continue;
            }
            assert!(
                geometry.layout
                    == if cloud {
                        NativeEffectVertexLayout::ParticleCloud8
                    } else {
                        NativeEffectVertexLayout::Packed32
                    },
                "original effect program requires its own original vertex declaration"
            );
            assert!(
                draw.vertex_byte_offset
                    .is_multiple_of(geometry.layout.stride())
                    && draw.vertex_byte_offset < geometry.vertex_bytes,
                "native effect draw must bind a complete source vertex range"
            );
            assert!(
                draw.index_count.is_multiple_of(3)
                    && draw
                        .first_index
                        .checked_add(draw.index_count)
                        .is_some_and(|end| end <= geometry.index_count),
                "native effect draw must reference complete triangles inside its index buffer"
            );
            let pipeline = variants.specialize(
                &cache,
                &pipeline,
                EffectPipelineKey {
                    program: material.program,
                    target: target.main_texture_format(),
                    samples: msaa.samples(),
                    depth_filtering: if cloud {
                        filtering(&cloud_view.expect("cloud view selected").sampler)
                    } else {
                        effect_view.filtering
                    },
                    state: material.state,
                },
            );
            phase.add_transient(Transparent3d {
                sorting_info: TransparentSortingInfo3d::Sorted {
                    mesh_center: draw.sort_center,
                    depth_bias: 0.0,
                },
                distance: 0.0,
                pipeline,
                entity: (entity, main_entity),
                draw_function,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

struct DrawNativeEffect;

impl<P: PhaseItem> RenderCommand<P> for DrawNativeEffect {
    type Param = (
        SRes<EffectObjectGroup>,
        SRes<RenderAssets<PreparedEffectMaterial>>,
        SRes<RenderAssets<PreparedEffectGeometry>>,
    );
    type ViewQuery = (
        &'static DynamicUniformIndex<NativeFxFrame>,
        Option<&'static PreparedEffectView>,
        Option<&'static PreparedCloudView>,
    );
    type ItemQuery = (
        &'static ExtractedEffectDraw,
        Option<&'static DynamicUniformIndex<NativeEffectTransform>>,
        Option<&'static DynamicUniformIndex<NativeCloudObject>>,
    );

    fn render<'w>(
        _: &P,
        (frame, view, cloud_view): ROQueryItem<'w, '_, Self::ViewQuery>,
        item: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (objects, materials, geometries): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some((draw, object, cloud_object)) = item else {
            return RenderCommandResult::Skip;
        };
        let (objects, materials, geometries) = (
            objects.into_inner(),
            materials.into_inner(),
            geometries.into_inner(),
        );
        let (Some(material), Some(geometry)) =
            (materials.get(draw.material), geometries.get(draw.geometry))
        else {
            return RenderCommandResult::Skip;
        };
        if material.program.cloud() {
            let (Some(view), Some((_, group)), Some(object)) =
                (cloud_view, &objects.cloud, cloud_object)
            else {
                return RenderCommandResult::Skip;
            };
            pass.set_bind_group(0, &view.bind_group, &[frame.index()]);
            pass.set_bind_group(1, group, &[object.index()]);
        } else {
            let (Some(view), Some((_, group)), Some(object)) = (view, &objects.packed, object)
            else {
                return RenderCommandResult::Skip;
            };
            pass.set_bind_group(0, &view.bind_group, &[frame.index()]);
            pass.set_bind_group(1, group, &[object.index()]);
        }
        pass.set_bind_group(2, &material.bind_group, &[]);
        if let Some(stencil) = material.state.stencil {
            pass.set_stencil_reference(stencil.reference);
        }
        pass.set_vertex_buffer(0, geometry.vertices.slice(draw.vertex_byte_offset..));
        pass.set_index_buffer(geometry.indices.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(
            draw.first_index..draw.first_index + draw.index_count,
            0,
            0..1,
        );
        RenderCommandResult::Success
    }
}
