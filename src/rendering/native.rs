//! Static T6 lightmapped-world draws in Bevy's ordinary opaque pass.
use bevy::{
    asset::{RenderAssetUsages, embedded_asset, load_embedded_asset},
    camera::visibility::{self, VisibilityClass},
    core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey},
    ecs::{
        query::{QueryItem, ROQueryItem},
        system::{SystemParamItem, lifetimeless::SRes},
    },
    mesh::VertexBufferLayout,
    platform::collections::HashMap,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        batching::{
            gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
            sort_binned_render_phase,
        },
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::allocator::MeshSlabs,
        render_asset::{
            AssetExtractionError, PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets,
        },
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
            RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
            ViewBinnedRenderPhases,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, texture_cube, uniform_buffer},
            *,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_component::SyncComponent,
        sync_world::MainEntity,
        texture::GpuImage,
        uniform::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewTarget},
    },
    shader::load_shader_library,
};

/// Original world vertex streams and little-endian Uint16 indices, shared by
/// all surfaces in the source world. Surface offsets retain the native layout.
#[derive(Asset, TypePath, Clone)]
pub struct NativeWorldGeometry {
    pub primary: Vec<u8>,
    pub auxiliary: Vec<u8>,
    pub indices: Vec<u8>,
}

pub(crate) struct PreparedNativeGeometry {
    pub(crate) primary: Buffer,
    auxiliary: Option<Buffer>,
    pub(crate) indices: Buffer,
}

impl RenderAsset for PreparedNativeGeometry {
    type SourceAsset = NativeWorldGeometry;
    type Param = SRes<RenderDevice>;

    fn asset_usage(_: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }

    fn take_gpu_data(
        source: &mut Self::SourceAsset,
        _: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        if source.primary.is_empty() {
            return Err(AssetExtractionError::AlreadyExtracted);
        }
        Ok(NativeWorldGeometry {
            primary: std::mem::take(&mut source.primary),
            auxiliary: std::mem::take(&mut source.auxiliary),
            indices: std::mem::take(&mut source.indices),
        })
    }

    fn byte_len(source: &Self::SourceAsset) -> Option<usize> {
        Some(source.primary.len() + source.auxiliary.len() + source.indices.len())
    }

    fn prepare_asset(
        source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        device: &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        assert!(
            !source.primary.is_empty(),
            "native world primary stream is empty"
        );
        assert!(
            !source.indices.is_empty() && source.indices.len().is_multiple_of(2),
            "native world indices must contain complete Uint16 elements"
        );
        let upload = |label, contents, usage| {
            device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some(label),
                contents,
                usage,
            })
        };
        Ok(Self {
            primary: upload(
                "native world primary vertices",
                &source.primary,
                BufferUsages::VERTEX,
            ),
            auxiliary: (!source.auxiliary.is_empty()).then(|| {
                upload(
                    "native world auxiliary vertices",
                    &source.auxiliary,
                    BufferUsages::VERTEX,
                )
            }),
            indices: upload(
                "native world Uint16 indices",
                &source.indices,
                BufferUsages::INDEX,
            ),
        })
    }
}

/// Camera constants in the source shader's axes and units. Rows, not columns.
#[derive(Component, Clone, Copy, ShaderType, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct NativeWorldScene {
    pub view_projection_rows: [Vec4; 4],
    pub fog_color: Vec4,
    pub fog_consts: Vec4,
    pub fog_consts2: Vec4,
    pub sun_fog_dir: Vec4,
    pub sun_fog_color: Vec4,
    pub sun_fog: Vec4,
    pub hdr_control0: Vec4,
}

/// Source world-to-eye-relative transform and the surface's probe coefficients.
#[derive(Component, Clone, Copy, ShaderType, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct NativeWorldObject {
    pub world_rows: [Vec4; 4],
    pub reflection_lighting_sh0: Vec4,
    pub reflection_lighting_sh1: Vec4,
    pub reflection_lighting_sh2: Vec4,
}

/// All four original inputs are required, with original sampler addressing.
#[derive(Clone)]
pub struct NativeWorldTextures {
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
    pub normal_map: Handle<Image>,
    pub normal_sampler: SamplerDescriptor<'static>,
    pub lightmap_secondary: Handle<Image>,
    pub lightmap_sampler: SamplerDescriptor<'static>,
    pub reflection_probe: Handle<Image>,
    pub reflection_sampler: SamplerDescriptor<'static>,
}

#[derive(Clone)]
pub struct NativeWorldLayerTextures {
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
    pub normal_map: Handle<Image>,
    pub normal_sampler: SamplerDescriptor<'static>,
}

#[derive(Clone)]
pub struct NativeWorldColorLayerTextures {
    pub base: NativeWorldTextures,
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
}

/// Diffuse-only LIT consumes no reflection probe or reflection coefficients.
#[derive(Clone)]
pub struct NativeWorldDiffuseTextures {
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
    pub normal_map: Handle<Image>,
    pub normal_sampler: SamplerDescriptor<'static>,
    pub lightmap_secondary: Handle<Image>,
    pub lightmap_sampler: SamplerDescriptor<'static>,
}

/// Geometric-normal specular LIT consumes no normal texture or tangent.
#[derive(Clone)]
pub struct NativeWorldGeometricSpecularTextures {
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
    pub specular_map: Handle<Image>,
    pub specular_sampler: SamplerDescriptor<'static>,
    pub lightmap_secondary: Handle<Image>,
    pub lightmap_sampler: SamplerDescriptor<'static>,
    pub reflection_probe: Handle<Image>,
    pub reflection_sampler: SamplerDescriptor<'static>,
}

#[derive(Asset, TypePath, Clone)]
pub enum NativeWorldMaterial {
    Lightmapped(NativeWorldTextures),
    Diffuse(NativeWorldDiffuseTextures),
    NormalSpecular {
        base: NativeWorldTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
    },
    GeometricSpecular(NativeWorldGeometricSpecularTextures),
    ColorMultiply(NativeWorldColorLayerTextures),
    ColorBlend(NativeWorldColorLayerTextures),
    ColorAdd(NativeWorldColorLayerTextures),
    Layered {
        base: NativeWorldTextures,
        layer: NativeWorldLayerTextures,
    },
    RevealLayered {
        base: NativeWorldTextures,
        layer: NativeWorldLayerTextures,
        alpha_reveal_parms1: Vec4,
    },
    RevealSpecularLayered {
        base: NativeWorldTextures,
        layer: NativeWorldLayerTextures,
        layer_specular_map: Handle<Image>,
        layer_specular_sampler: SamplerDescriptor<'static>,
        alpha_reveal_parms1: Vec4,
    },
}

impl NativeWorldMaterial {
    pub(crate) fn images(&self) -> impl Iterator<Item = &Handle<Image>> {
        self.textures().map(|(_, image, _)| image)
    }

    pub(crate) fn auxiliary_stride(&self) -> Option<u64> {
        self.family().auxiliary_stride()
    }

    /// The material's actual GPU inputs, with the same bindings used by the
    /// entry shaders. Loading readiness and GPU preparation share this list.
    fn textures(&self) -> impl Iterator<Item = (u32, &Handle<Image>, &SamplerDescriptor<'static>)> {
        fn base_bindings(
            base: &NativeWorldTextures,
        ) -> [Option<(u32, &Handle<Image>, &SamplerDescriptor<'static>)>; 7] {
            [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((2, &base.normal_map, &base.normal_sampler)),
                Some((4, &base.lightmap_secondary, &base.lightmap_sampler)),
                Some((6, &base.reflection_probe, &base.reflection_sampler)),
                None,
                None,
                None,
            ]
        }
        let bindings = match self {
            Self::Lightmapped(base) => base_bindings(base),
            Self::Diffuse(base) => [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((2, &base.normal_map, &base.normal_sampler)),
                Some((4, &base.lightmap_secondary, &base.lightmap_sampler)),
                None,
                None,
                None,
                None,
            ],
            Self::NormalSpecular {
                base,
                specular_map,
                specular_sampler,
            } => {
                let mut bindings = base_bindings(base);
                bindings[4] = Some((8, specular_map, specular_sampler));
                bindings
            }
            Self::GeometricSpecular(base) => [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((4, &base.lightmap_secondary, &base.lightmap_sampler)),
                Some((6, &base.reflection_probe, &base.reflection_sampler)),
                Some((8, &base.specular_map, &base.specular_sampler)),
                None,
                None,
                None,
            ],
            Self::ColorMultiply(layer) | Self::ColorBlend(layer) | Self::ColorAdd(layer) => {
                let mut bindings = base_bindings(&layer.base);
                bindings[4] = Some((8, &layer.color_map, &layer.color_sampler));
                bindings
            }
            Self::Layered { base, layer } | Self::RevealLayered { base, layer, .. } => {
                let mut bindings = base_bindings(base);
                bindings[4] = Some((8, &layer.color_map, &layer.color_sampler));
                bindings[5] = Some((10, &layer.normal_map, &layer.normal_sampler));
                bindings
            }
            Self::RevealSpecularLayered {
                base,
                layer,
                layer_specular_map,
                layer_specular_sampler,
                ..
            } => {
                let mut bindings = base_bindings(base);
                bindings[4] = Some((8, &layer.color_map, &layer.color_sampler));
                bindings[5] = Some((10, &layer.normal_map, &layer.normal_sampler));
                bindings[6] = Some((12, layer_specular_map, layer_specular_sampler));
                bindings
            }
        };
        bindings.into_iter().flatten()
    }

    fn family(&self) -> NativeWorldFamily {
        match self {
            Self::Lightmapped(_) => NativeWorldFamily::Lightmapped,
            Self::Diffuse(_) => NativeWorldFamily::Diffuse,
            Self::NormalSpecular { .. } => NativeWorldFamily::NormalSpecular,
            Self::GeometricSpecular(_) => NativeWorldFamily::GeometricSpecular,
            Self::ColorMultiply(_) => NativeWorldFamily::ColorMultiply,
            Self::ColorBlend(_) => NativeWorldFamily::ColorBlend,
            Self::ColorAdd(_) => NativeWorldFamily::ColorAdd,
            Self::Layered { .. } => NativeWorldFamily::Layered,
            Self::RevealLayered { .. } => NativeWorldFamily::RevealLayered,
            Self::RevealSpecularLayered { .. } => NativeWorldFamily::RevealSpecularLayered,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum NativeWorldFamily {
    Lightmapped,
    Diffuse,
    NormalSpecular,
    GeometricSpecular,
    ColorMultiply,
    ColorBlend,
    ColorAdd,
    Layered,
    RevealLayered,
    RevealSpecularLayered,
}

impl NativeWorldFamily {
    fn auxiliary_stride(self) -> Option<u64> {
        match self {
            Self::ColorMultiply | Self::ColorBlend | Self::ColorAdd => Some(4),
            Self::Layered | Self::RevealLayered | Self::RevealSpecularLayered => Some(8),
            _ => None,
        }
    }
}

#[derive(ShaderType)]
struct NativeWorldRevealMaterial {
    alpha_reveal_parms1: Vec4,
}

/// Values decoded from this technique's original state bits. The camera's
/// depth convention must agree with depth_compare and its depth clear value.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeWorldState {
    pub depth_compare: CompareFunction,
    pub depth_write: bool,
    pub cull_mode: Option<Face>,
    pub front_face: FrontFace,
    pub color_write: ColorWrites,
}

#[derive(Component, Clone)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<NativeWorldSurface>)]
pub struct NativeWorldSurface {
    pub geometry: Handle<NativeWorldGeometry>,
    pub material: Handle<NativeWorldMaterial>,
    pub state: NativeWorldState,
    pub primary_byte_offset: u64,
    pub auxiliary_byte_offset: u64,
    pub first_index: u32,
    pub index_count: u32,
}

#[derive(Component)]
pub struct ExtractedNativeSurface {
    geometry: AssetId<NativeWorldGeometry>,
    material: AssetId<NativeWorldMaterial>,
    state: NativeWorldState,
    primary_byte_offset: u64,
    auxiliary_byte_offset: u64,
    first_index: u32,
    index_count: u32,
}

impl SyncComponent<RenderApp> for NativeWorldSurface {
    type Target = ExtractedNativeSurface;
}

impl ExtractComponent<RenderApp> for NativeWorldSurface {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = ExtractedNativeSurface;

    fn extract_component(surface: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(ExtractedNativeSurface {
            geometry: surface.geometry.id(),
            material: surface.material.id(),
            state: surface.state,
            primary_byte_offset: surface.primary_byte_offset,
            auxiliary_byte_offset: surface.auxiliary_byte_offset,
            first_index: surface.first_index,
            index_count: surface.index_count,
        })
    }
}

/// Attach only after the original material inputs and camera constants exist.
pub struct NativeWorldPlugin;

impl Plugin for NativeWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<NativeWorldGeometry>()
            .init_asset::<NativeWorldMaterial>()
            .add_plugins((
                ExtractComponentPlugin::<NativeWorldScene>::default(),
                ExtractComponentPlugin::<NativeWorldObject>::default(),
                ExtractComponentPlugin::<NativeWorldSurface>::default(),
                UniformComponentPlugin::<NativeWorldScene>::default(),
                UniformComponentPlugin::<NativeWorldObject>::default(),
                RenderAssetPlugin::<PreparedNativeMaterial, GpuImage>::default(),
                RenderAssetPlugin::<PreparedNativeGeometry>::default(),
            ));
        load_shader_library!(app, "native_world.wesl");
        load_shader_library!(app, "native_world_layers.wesl");
        load_shader_library!(app, "native_world_plain.wesl");
        load_shader_library!(app, "native_world_color_layers.wesl");
        embedded_asset!(app, "native_entry.wesl");
        embedded_asset!(app, "native_layer_entry.wesl");
        embedded_asset!(app, "native_plain_entry.wesl");
        embedded_asset!(app, "native_color_layer_entry.wesl");
        let shader = load_embedded_asset!(app, "native_entry.wesl");
        let layer_shader = load_embedded_asset!(app, "native_layer_entry.wesl");
        let plain_shader = load_embedded_asset!(app, "native_plain_entry.wesl");
        let color_layer_shader = load_embedded_asset!(app, "native_color_layer_entry.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(NativePipeline::new(
                shader,
                layer_shader,
                plain_shader,
                color_layer_shader,
            ))
            .insert_resource(GpuPreprocessingSupport {
                max_supported_mode: GpuPreprocessingMode::None,
            })
            .init_resource::<SpecializedRenderPipelines<NativePipeline>>()
            .init_resource::<NativeUniformGroups>()
            .add_render_command::<Opaque3d, (SetItemPipeline, DrawNativeWorld)>()
            .add_systems(
                Render,
                (
                    queue_native_world.in_set(RenderSystems::Queue),
                    prepare_uniform_groups.in_set(RenderSystems::PrepareBindGroups),
                    sort_binned_render_phase::<Opaque3d>.in_set(RenderSystems::PhaseSort),
                ),
            );
    }
}

#[derive(Resource)]
struct NativePipeline {
    shader: Handle<Shader>,
    layer_shader: Handle<Shader>,
    plain_shader: Handle<Shader>,
    color_layer_shader: Handle<Shader>,
    scene_layout: BindGroupLayoutDescriptor,
    object_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    diffuse_material_layout: BindGroupLayoutDescriptor,
    extra_texture_material_layout: BindGroupLayoutDescriptor,
    geometric_specular_material_layout: BindGroupLayoutDescriptor,
    layer_material_layout: BindGroupLayoutDescriptor,
    reveal_material_layout: BindGroupLayoutDescriptor,
    reveal_specular_material_layout: BindGroupLayoutDescriptor,
}

impl NativePipeline {
    fn new(
        shader: Handle<Shader>,
        layer_shader: Handle<Shader>,
        plain_shader: Handle<Shader>,
        color_layer_shader: Handle<Shader>,
    ) -> Self {
        let mut entries = BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_cube(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        )
        .to_vec();
        let material_layout = BindGroupLayoutDescriptor::new("native world textures", &entries);
        let diffuse_material_layout =
            BindGroupLayoutDescriptor::new("native diffuse world textures", &entries[..6]);
        let mut specular_entries = entries.clone();
        specular_entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (8, texture_2d(TextureSampleType::Float { filterable: true })),
                (9, sampler(SamplerBindingType::Filtering)),
            ),
        ));
        let extra_texture_material_layout =
            BindGroupLayoutDescriptor::new("native world with extra texture", &specular_entries);
        // This source program has no normal texture. Keep its actual bindings
        // without allocating a replacement texture for the unused 2/3 slots.
        specular_entries.retain(|entry| entry.binding != 2 && entry.binding != 3);
        let geometric_specular_material_layout = BindGroupLayoutDescriptor::new(
            "native geometric specular world textures",
            &specular_entries,
        );
        entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (8, texture_2d(TextureSampleType::Float { filterable: true })),
                (9, sampler(SamplerBindingType::Filtering)),
                (
                    10,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (11, sampler(SamplerBindingType::Filtering)),
            ),
        ));
        let layer_material_layout =
            BindGroupLayoutDescriptor::new("native layered world textures", &entries);
        let mut reveal_entries = entries.clone();
        reveal_entries.push(
            uniform_buffer::<NativeWorldRevealMaterial>(false).build(14, ShaderStages::FRAGMENT),
        );
        let reveal_material_layout =
            BindGroupLayoutDescriptor::new("native reveal world material", &reveal_entries);
        entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (
                    12,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (13, sampler(SamplerBindingType::Filtering)),
                (14, uniform_buffer::<NativeWorldRevealMaterial>(false)),
            ),
        ));
        let reveal_specular_material_layout =
            BindGroupLayoutDescriptor::new("native reveal specular world material", &entries);
        Self {
            shader,
            layer_shader,
            plain_shader,
            color_layer_shader,
            scene_layout: BindGroupLayoutDescriptor::new(
                "native world scene",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX_FRAGMENT,
                    uniform_buffer::<NativeWorldScene>(true),
                ),
            ),
            object_layout: BindGroupLayoutDescriptor::new(
                "native world object",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX,
                    uniform_buffer::<NativeWorldObject>(true),
                ),
            ),
            material_layout,
            diffuse_material_layout,
            extra_texture_material_layout,
            geometric_specular_material_layout,
            layer_material_layout,
            reveal_material_layout,
            reveal_specular_material_layout,
        }
    }

    fn material_layout(&self, family: NativeWorldFamily) -> &BindGroupLayoutDescriptor {
        match family {
            NativeWorldFamily::Lightmapped => &self.material_layout,
            NativeWorldFamily::Diffuse => &self.diffuse_material_layout,
            NativeWorldFamily::NormalSpecular
            | NativeWorldFamily::ColorMultiply
            | NativeWorldFamily::ColorBlend
            | NativeWorldFamily::ColorAdd => &self.extra_texture_material_layout,
            NativeWorldFamily::GeometricSpecular => &self.geometric_specular_material_layout,
            NativeWorldFamily::Layered => &self.layer_material_layout,
            NativeWorldFamily::RevealLayered => &self.reveal_material_layout,
            NativeWorldFamily::RevealSpecularLayered => &self.reveal_specular_material_layout,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct NativePipelineKey {
    target: TextureFormat,
    samples: u32,
    state: NativeWorldState,
    family: NativeWorldFamily,
}

impl SpecializedRenderPipeline for NativePipeline {
    type Key = NativePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut vertex_layout = VertexBufferLayout {
            array_stride: 36,
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
                VertexAttribute {
                    format: VertexFormat::Unorm16x2,
                    offset: 32,
                    shader_location: 5,
                },
            ],
        };
        if key.family == NativeWorldFamily::GeometricSpecular {
            vertex_layout
                .attributes
                .retain(|attribute| attribute.shader_location != 4);
        }
        let mut buffers = vec![vertex_layout];
        if let Some(stride) = key.family.auxiliary_stride() {
            let mut attributes = vec![VertexAttribute {
                format: VertexFormat::Float16x2,
                offset: 0,
                shader_location: 6,
            }];
            if stride == 8 {
                attributes.push(VertexAttribute {
                    format: VertexFormat::Unorm8x4,
                    offset: 4,
                    shader_location: 7,
                });
            }
            buffers.push(VertexBufferLayout {
                array_stride: stride,
                step_mode: VertexStepMode::Vertex,
                attributes,
            });
        }
        let (shader, vertex) = match key.family {
            NativeWorldFamily::Lightmapped => (&self.shader, "vertex"),
            NativeWorldFamily::Diffuse => (&self.plain_shader, "vertex_diffuse"),
            NativeWorldFamily::NormalSpecular => (&self.plain_shader, "vertex_specular"),
            NativeWorldFamily::GeometricSpecular => {
                (&self.plain_shader, "vertex_geometric_specular")
            }
            NativeWorldFamily::ColorMultiply
            | NativeWorldFamily::ColorBlend
            | NativeWorldFamily::ColorAdd => (&self.color_layer_shader, "vertex"),
            NativeWorldFamily::Layered
            | NativeWorldFamily::RevealLayered
            | NativeWorldFamily::RevealSpecularLayered => (&self.layer_shader, "vertex"),
        };
        let fragment = match (key.family, key.target.is_srgb()) {
            (NativeWorldFamily::Diffuse, false) => "fragment_diffuse",
            (NativeWorldFamily::Diffuse, true) => "fragment_diffuse_srgb",
            (NativeWorldFamily::NormalSpecular, false) => "fragment_specular",
            (NativeWorldFamily::NormalSpecular, true) => "fragment_specular_srgb",
            (NativeWorldFamily::GeometricSpecular, false) => "fragment_geometric_specular",
            (NativeWorldFamily::GeometricSpecular, true) => "fragment_geometric_specular_srgb",
            (NativeWorldFamily::ColorMultiply, false) => "fragment_multiply",
            (NativeWorldFamily::ColorMultiply, true) => "fragment_multiply_srgb",
            (NativeWorldFamily::ColorBlend, false) => "fragment_blend",
            (NativeWorldFamily::ColorBlend, true) => "fragment_blend_srgb",
            (NativeWorldFamily::ColorAdd, false) => "fragment_add",
            (NativeWorldFamily::ColorAdd, true) => "fragment_add_srgb",
            (NativeWorldFamily::RevealLayered, false) => "fragment_reveal",
            (NativeWorldFamily::RevealLayered, true) => "fragment_reveal_srgb",
            (NativeWorldFamily::RevealSpecularLayered, false) => "fragment_reveal_specular",
            (NativeWorldFamily::RevealSpecularLayered, true) => "fragment_reveal_specular_srgb",
            (_, false) => "fragment",
            (_, true) => "fragment_srgb",
        };
        RenderPipelineDescriptor {
            label: Some("T6 lightmapped world".into()),
            layout: vec![
                self.scene_layout.clone(),
                self.object_layout.clone(),
                self.material_layout(key.family).clone(),
            ],
            vertex: VertexState {
                shader: shader.clone(),
                entry_point: Some(vertex.into()),
                buffers,
                ..default()
            },
            fragment: Some(FragmentState {
                shader: shader.clone(),
                entry_point: Some(fragment.into()),
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
                format: super::frame::NATIVE_DEPTH_FORMAT,
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

struct PreparedNativeMaterial {
    bind_group: BindGroup,
    family: NativeWorldFamily,
}

impl RenderAsset for PreparedNativeMaterial {
    type SourceAsset = NativeWorldMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<PipelineCache>,
        SRes<NativePipeline>,
        SRes<RenderAssets<GpuImage>>,
    );

    fn prepare_asset(
        source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (device, queue, cache, pipeline, images): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let family = source.family();
        let reveal = match &source {
            NativeWorldMaterial::RevealLayered {
                alpha_reveal_parms1,
                ..
            }
            | NativeWorldMaterial::RevealSpecularLayered {
                alpha_reveal_parms1,
                ..
            } => Some(*alpha_reveal_parms1),
            _ => None,
        };
        let textures: Vec<_> = source.textures().collect();
        let Some(gpu_images): Option<Vec<_>> = textures
            .iter()
            .map(|(_, handle, _)| images.get(handle.id()))
            .collect()
        else {
            drop(textures);
            return Err(PrepareAssetError::RetryNextUpdate(source));
        };
        let mut views = Vec::with_capacity(textures.len());
        let mut samplers = Vec::with_capacity(textures.len());
        for ((binding, _, sampler), image) in textures.into_iter().zip(gpu_images) {
            assert!(
                !image.texture_descriptor.format.is_srgb(),
                "native world inputs must preserve original UNORM shader reads"
            );
            views.push((binding, &image.texture_view));
            samplers.push(device.create_sampler(sampler));
        }
        let mut reveal_uniform = reveal.map(|alpha_reveal_parms1| {
            UniformBuffer::from(NativeWorldRevealMaterial {
                alpha_reveal_parms1,
            })
        });
        if let Some(uniform) = &mut reveal_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut entries = Vec::with_capacity(views.len() * 2 + usize::from(reveal.is_some()));
        for ((binding, view), sampler) in views.into_iter().zip(&samplers) {
            entries.push(BindGroupEntry {
                binding,
                resource: BindingResource::TextureView(view),
            });
            entries.push(BindGroupEntry {
                binding: binding + 1,
                resource: BindingResource::Sampler(sampler),
            });
        }
        if let Some(uniform) = &reveal_uniform {
            entries.push(BindGroupEntry {
                binding: 14,
                resource: uniform.binding().expect("uploaded native reveal constants"),
            });
        }
        let bind_group = device.create_bind_group(
            "native world material",
            &cache.get_bind_group_layout(pipeline.material_layout(family)),
            &entries,
        );
        Ok(Self { bind_group, family })
    }
}

#[derive(Resource, Default)]
struct NativeUniformGroups {
    scene: Option<(BufferId, BindGroup)>,
    object: Option<(BufferId, BindGroup)>,
}

fn prepare_uniform_groups(
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    pipeline: Res<NativePipeline>,
    scenes: Res<ComponentUniforms<NativeWorldScene>>,
    objects: Res<ComponentUniforms<NativeWorldObject>>,
    mut groups: ResMut<NativeUniformGroups>,
) {
    if let (Some(buffer), Some(binding)) = (scenes.buffer(), scenes.binding()) {
        if groups
            .scene
            .as_ref()
            .is_none_or(|(id, _)| *id != buffer.id())
        {
            groups.scene = Some((
                buffer.id(),
                device.create_bind_group(
                    "native scene uniforms",
                    &cache.get_bind_group_layout(&pipeline.scene_layout),
                    &BindGroupEntries::single(binding),
                ),
            ));
        }
    }
    if let (Some(buffer), Some(binding)) = (objects.buffer(), objects.binding()) {
        if groups
            .object
            .as_ref()
            .is_none_or(|(id, _)| *id != buffer.id())
        {
            groups.object = Some((
                buffer.id(),
                device.create_bind_group(
                    "native object uniforms",
                    &cache.get_bind_group_layout(&pipeline.object_layout),
                    &BindGroupEntries::single(binding),
                ),
            ));
        }
    }
}

fn queue_native_world(
    pipeline: Res<NativePipeline>,
    cache: Res<PipelineCache>,
    mut variants: ResMut<SpecializedRenderPipelines<NativePipeline>>,
    mut phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    geometries: Res<RenderAssets<PreparedNativeGeometry>>,
    materials: Res<RenderAssets<PreparedNativeMaterial>>,
    views: Query<
        (&ExtractedView, &RenderVisibleEntities, &ViewTarget, &Msaa),
        With<NativeWorldScene>,
    >,
    surfaces: Query<&ExtractedNativeSurface, With<NativeWorldObject>>,
    mut queued: Local<HashMap<RetainedViewEntity, Vec<MainEntity>>>,
) {
    // Remove only this renderer's previous membership. This also handles a view
    // losing its native scene constants or a surface becoming invisible.
    queued.retain(|view, entities| {
        let Some(phase) = phases.get_mut(view) else {
            return false;
        };
        for entity in entities.drain(..) {
            phase.remove(entity);
        }
        true
    });
    let draw_function = draw_functions
        .read()
        .id::<(SetItemPipeline, DrawNativeWorld)>();
    for (view, visible, target, msaa) in &views {
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(visible) = visible.get::<NativeWorldSurface>() else {
            continue;
        };
        let queued = queued.entry(view.retained_view_entity).or_default();
        for (&entity, &main_entity) in visible.iter_visible() {
            let Ok(surface) = surfaces.get(entity) else {
                continue;
            };
            if geometries.get(surface.geometry).is_none() {
                continue;
            }
            let Some(material) = materials.get(surface.material) else {
                continue;
            };
            let key = NativePipelineKey {
                target: target.main_texture_format(),
                samples: msaa.samples(),
                state: surface.state,
                family: material.family,
            };
            let pipeline = variants.specialize(&cache, &pipeline, key);
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

struct DrawNativeWorld;

impl<P: PhaseItem> RenderCommand<P> for DrawNativeWorld {
    type Param = (
        SRes<NativeUniformGroups>,
        SRes<RenderAssets<PreparedNativeMaterial>>,
        SRes<RenderAssets<PreparedNativeGeometry>>,
    );
    type ViewQuery = &'static DynamicUniformIndex<NativeWorldScene>;
    type ItemQuery = (
        &'static ExtractedNativeSurface,
        &'static DynamicUniformIndex<NativeWorldObject>,
    );

    fn render<'w>(
        _: &P,
        scene: ROQueryItem<'w, '_, Self::ViewQuery>,
        item: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (groups, materials, geometries): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some((surface, object)) = item else {
            return RenderCommandResult::Skip;
        };
        let groups = groups.into_inner();
        let materials = materials.into_inner();
        let geometries = geometries.into_inner();
        let (Some((_, scene_group)), Some((_, object_group)), Some(material), Some(geometry)) = (
            &groups.scene,
            &groups.object,
            materials.get(surface.material),
            geometries.get(surface.geometry),
        ) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(0, scene_group, &[scene.index()]);
        pass.set_bind_group(1, object_group, &[object.index()]);
        pass.set_bind_group(2, &material.bind_group, &[]);
        pass.set_vertex_buffer(0, geometry.primary.slice(surface.primary_byte_offset..));
        if material.family.auxiliary_stride().is_some() {
            let auxiliary = geometry
                .auxiliary
                .as_ref()
                .expect("native layered world requires the original auxiliary vertex stream");
            pass.set_vertex_buffer(1, auxiliary.slice(surface.auxiliary_byte_offset..));
        }
        pass.set_index_buffer(geometry.indices.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(
            surface.first_index..surface.first_index + surface.index_count,
            0,
            0..1,
        );
        RenderCommandResult::Success
    }
}
