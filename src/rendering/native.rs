//! T6 lightmapped-world draws in opaque and source-ordered litTrans passes.
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
        camera::ExtractedCamera,
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
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        sync_component::SyncComponent,
        sync_world::MainEntity,
        texture::GpuImage,
        uniform::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        view::{
            ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewDepthStencilTexture,
            ViewTarget,
        },
    },
    shader::load_shader_library,
};

/// Original world vertex streams and little-endian Uint16 indices, shared by
/// all surfaces in the source world. Surface offsets retain the native layout.
#[derive(Asset, TypePath, Clone)]
pub struct NativeWorldGeometry {
    pub primary: std::sync::Arc<[u8]>,
    pub auxiliary: Vec<u8>,
    pub indices: std::sync::Arc<[u8]>,
    pub vertex_stride: u64,
}

pub(crate) struct PreparedNativeGeometry {
    pub(crate) primary: Buffer,
    auxiliary: Option<Buffer>,
    pub(crate) indices: Buffer,
    vertex_stride: u64,
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
            vertex_stride: source.vertex_stride,
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
            vertex_stride: source.vertex_stride,
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

/// 00416df0 packs the four view-selected type-5 lights into seven SoA vectors.
/// Positions are eye-relative source inches; falloffs are negative inverse radii.
#[derive(Clone, Copy, Default, ShaderType)]
pub struct NativeOmniLights {
    pub position_x: Vec4,
    pub position_y: Vec4,
    pub position_z: Vec4,
    pub falloffs: Vec4,
    pub reds: Vec4,
    pub greens: Vec4,
    pub blues: Vec4,
    pub enabled: Vec4,
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
    pub game_time: Vec4,
    pub sun_position: Vec4,
    pub sun_diffuse: Vec4,
    pub omni_lights: NativeOmniLights,
}

/// Source world-to-eye-relative transform and the surface's probe coefficients.
#[derive(Component, Clone, Copy, PartialEq, ShaderType, ExtractComponent)]
#[extract_app(RenderApp)]
pub struct NativeWorldObject {
    pub world_rows: [Vec4; 4],
    pub reflection_lighting_sh0: Vec4,
    pub reflection_lighting_sh1: Vec4,
    pub reflection_lighting_sh2: Vec4,
    pub color: Vec4,
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
pub struct NativeWorldSpecularLayerTextures {
    pub layer: NativeWorldLayerTextures,
    pub specular_map: Handle<Image>,
    pub specular_sampler: SamplerDescriptor<'static>,
}

#[derive(Clone)]
pub struct NativeWorldSpecularTextures {
    pub base: NativeWorldTextures,
    pub specular_map: Handle<Image>,
    pub specular_sampler: SamplerDescriptor<'static>,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NativeWorldPass {
    Opaque,
    EmissiveOpaque,
    Translucent,
    QuasiOpaque,
}

#[derive(Clone, Copy)]
pub enum NativeUnlitProgram {
    Replace,
    Add,
    DecalAdd,
    DecalMultiply,
}

#[derive(ShaderType)]
struct NativeUnlitConstants {
    scale_rgb: Vec4,
}

#[derive(Clone, Copy, ShaderType)]
pub struct NativeWorldParallaxLayer {
    pub scale_offset: Vec4,
    pub scroll_alpha_hdr: Vec4,
    pub rotation_origin: Vec4,
    pub depth: Vec4,
}

#[derive(Clone, Copy, ShaderType)]
pub struct NativeLavaConstants {
    pub layers: [NativeWorldParallaxLayer; 3],
}

#[derive(Clone, Copy, ShaderType)]
pub struct NativeEmberConstants {
    pub scale_direction_max: Vec4,
    pub min_seed: Vec4,
}

#[derive(Asset, TypePath, Clone)]
pub enum NativeWorldMaterial {
    Ember {
        base: NativeWorldTextures,
        super_map: Handle<Image>,
        super_sampler: SamplerDescriptor<'static>,
        ember_map: Handle<Image>,
        ember_sampler: SamplerDescriptor<'static>,
        constants: NativeEmberConstants,
    },
    Lava {
        layers: [(Handle<Image>, SamplerDescriptor<'static>); 3],
        constants: NativeLavaConstants,
    },
    Wallbuy {
        color_map: Handle<Image>,
        color_sampler: SamplerDescriptor<'static>,
        constants: NativeWorldParallaxLayer,
    },
    Unlit {
        program: NativeUnlitProgram,
        color_map: Handle<Image>,
        color_sampler: SamplerDescriptor<'static>,
        scale_rgb: Option<Vec4>,
    },
    GeometricGloss {
        color_map: Handle<Image>,
        color_sampler: SamplerDescriptor<'static>,
        lightmap_secondary: Handle<Image>,
        lightmap_sampler: SamplerDescriptor<'static>,
        reflection_probe: Handle<Image>,
        reflection_sampler: SamplerDescriptor<'static>,
    },
    GeometricDiffuse {
        color_map: Handle<Image>,
        color_sampler: SamplerDescriptor<'static>,
        lightmap_secondary: Handle<Image>,
        lightmap_sampler: SamplerDescriptor<'static>,
    },
    Lightmapped(NativeWorldTextures),
    Diffuse(NativeWorldDiffuseTextures),
    RevealDiffuse {
        base: NativeWorldDiffuseTextures,
        alpha_reveal_parms: Vec4,
    },
    NormalSpecular {
        base: NativeWorldTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
    },
    RevealNormalSpecular {
        base: NativeWorldTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        alpha_reveal_parms: Vec4,
    },
    GeometricSpecular(NativeWorldGeometricSpecularTextures),
    DiffuseColorMultiply {
        base: NativeWorldDiffuseTextures,
        color_map: Handle<Image>,
        color_sampler: SamplerDescriptor<'static>,
        second_layer: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
    },
    ColorMultiply(NativeWorldColorLayerTextures),
    ColorBlend(NativeWorldColorLayerTextures),
    ColorAdd(NativeWorldColorLayerTextures),
    Layered {
        hard_select: bool,
        base: NativeWorldTextures,
        layer: NativeWorldLayerTextures,
    },
    RevealLayered {
        base: NativeWorldTextures,
        layer: NativeWorldLayerTextures,
        alpha_reveal_parms1: Vec4,
    },
    SpecularLayered {
        base: NativeWorldTextures,
        layer: NativeWorldLayerTextures,
        layer_specular_map: Handle<Image>,
        layer_specular_sampler: SamplerDescriptor<'static>,
    },
    RevealSpecularLayered {
        base: NativeWorldTextures,
        layer: NativeWorldLayerTextures,
        layer_specular_map: Handle<Image>,
        layer_specular_sampler: SamplerDescriptor<'static>,
        alpha_reveal_parms1: Vec4,
    },
    DiffuseLayerMultiply {
        base: NativeWorldDiffuseTextures,
        layer1: NativeWorldLayerTextures,
        color_map2: Handle<Image>,
        color_sampler2: SamplerDescriptor<'static>,
        alpha_reveal_parms1: Option<Vec4>,
    },
    LayerAdd {
        base: NativeWorldTextures,
        layer1: NativeWorldLayerTextures,
        color_map2: Handle<Image>,
        color_sampler2: SamplerDescriptor<'static>,
    },
    RevealOverlay {
        additive: bool,
        multiply3: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
        base: NativeWorldTextures,
        layer1: NativeWorldSpecularLayerTextures,
        color_map2: Handle<Image>,
        color_sampler2: SamplerDescriptor<'static>,
        alpha_reveal_parms1: Vec4,
    },
    ColorOverlays {
        base: NativeWorldTextures,
        color_map1: Handle<Image>,
        color_sampler1: SamplerDescriptor<'static>,
        color_map2: Handle<Image>,
        color_sampler2: SamplerDescriptor<'static>,
        add_multiply: bool,
    },
    ColorThreeLayer {
        base: NativeWorldTextures,
        color_map1: Handle<Image>,
        color_sampler1: SamplerDescriptor<'static>,
        layer2: NativeWorldLayerTextures,
        specular2: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
        alpha_reveal_parms2: Option<Vec4>,
    },
    ThreeLayerNormals {
        base: NativeWorldTextures,
        layer1: NativeWorldLayerTextures,
        layer2: NativeWorldLayerTextures,
    },
    ThreeLayerReveal {
        base: NativeWorldTextures,
        layer1: NativeWorldLayerTextures,
        layer2: NativeWorldSpecularLayerTextures,
        alpha_reveal_parms2: Vec4,
    },
    ThreeLayerRetainedSpecular {
        base: NativeWorldTextures,
        layer1: NativeWorldSpecularLayerTextures,
        layer2: NativeWorldLayerTextures,
        alpha_reveal_parms1: Vec4,
        alpha_reveal_parms2: Vec4,
    },
    ThreeLayerDoubleReveal {
        base: NativeWorldTextures,
        layer1: NativeWorldSpecularLayerTextures,
        layer2: NativeWorldSpecularLayerTextures,
        alpha_reveal_parms1: Vec4,
        alpha_reveal_parms2: Vec4,
    },
    BaseSpecularLayered {
        base: NativeWorldSpecularTextures,
        layer: NativeWorldLayerTextures,
    },
    BaseSpecularRevealLayered {
        base: NativeWorldSpecularTextures,
        layer: NativeWorldSpecularLayerTextures,
        alpha_reveal_parms1: Vec4,
    },
    BaseSpecularThreeLayerReveal {
        base: NativeWorldSpecularTextures,
        layer1: NativeWorldLayerTextures,
        layer2: NativeWorldSpecularLayerTextures,
        alpha_reveal_parms2: Vec4,
    },
    BaseSpecularThreeLayerDoubleReveal {
        base: NativeWorldSpecularTextures,
        layer1: NativeWorldSpecularLayerTextures,
        layer2: NativeWorldSpecularLayerTextures,
        alpha_reveal_parms1: Vec4,
        alpha_reveal_parms2: Vec4,
    },
    DiffuseLayered {
        base: NativeWorldDiffuseTextures,
        layer: NativeWorldLayerTextures,
    },
    DiffuseRevealLayered {
        base: NativeWorldDiffuseTextures,
        layer: NativeWorldLayerTextures,
        alpha_reveal_parms1: Vec4,
    },
    NoBaseGlossRevealLayered {
        base: NativeWorldTextures,
        layer: NativeWorldSpecularLayerTextures,
        alpha_reveal_parms1: Vec4,
    },
    GeometricLayered {
        base: NativeWorldGeometricSpecularTextures,
        color_map: Handle<Image>,
        color_sampler: SamplerDescriptor<'static>,
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
        type Binding<'a> = (u32, &'a Handle<Image>, &'a SamplerDescriptor<'static>);
        type Bindings<'a> = [Option<Binding<'a>>; 11];
        fn base_bindings(base: &NativeWorldTextures) -> Bindings<'_> {
            let mut out = [None; 11];
            out[0] = Some((0, &base.color_map, &base.color_sampler));
            out[1] = Some((2, &base.normal_map, &base.normal_sampler));
            out[2] = Some((4, &base.lightmap_secondary, &base.lightmap_sampler));
            out[3] = Some((6, &base.reflection_probe, &base.reflection_sampler));
            out
        }
        fn diffuse_bindings(base: &NativeWorldDiffuseTextures) -> Bindings<'_> {
            let mut out = [None; 11];
            out[0] = Some((0, &base.color_map, &base.color_sampler));
            out[1] = Some((2, &base.normal_map, &base.normal_sampler));
            out[2] = Some((4, &base.lightmap_secondary, &base.lightmap_sampler));
            out
        }
        fn specular_bindings(base: &NativeWorldSpecularTextures) -> Bindings<'_> {
            let mut out = base_bindings(&base.base);
            out[10] = Some((22, &base.specular_map, &base.specular_sampler));
            out
        }
        fn layer_bindings<'a>(
            out: &mut Bindings<'a>,
            slot: usize,
            binding: u32,
            layer: &'a NativeWorldLayerTextures,
        ) {
            out[slot] = Some((binding, &layer.color_map, &layer.color_sampler));
            out[slot + 1] = Some((binding + 2, &layer.normal_map, &layer.normal_sampler));
        }
        fn specular_layer_bindings<'a>(
            out: &mut Bindings<'a>,
            slot: usize,
            binding: u32,
            layer: &'a NativeWorldSpecularLayerTextures,
        ) {
            layer_bindings(out, slot, binding, &layer.layer);
            out[slot + 2] = Some((binding + 4, &layer.specular_map, &layer.specular_sampler));
        }
        let mut out = match self {
            Self::Ember { base, .. } => base_bindings(base),
            Self::Lava { layers, .. } => {
                let mut out = [None; 11];
                for (slot, binding) in [0, 8, 10].into_iter().enumerate() {
                    out[slot] = Some((binding, &layers[slot].0, &layers[slot].1));
                }
                out
            }
            Self::Wallbuy {
                color_map,
                color_sampler,
                ..
            }
            | Self::Unlit {
                color_map,
                color_sampler,
                ..
            } => {
                let mut out = [None; 11];
                out[0] = Some((0, color_map, color_sampler));
                out
            }
            Self::GeometricGloss {
                color_map,
                color_sampler,
                lightmap_secondary,
                lightmap_sampler,
                reflection_probe,
                reflection_sampler,
            } => {
                let mut out = [None; 11];
                out[0] = Some((0, color_map, color_sampler));
                out[2] = Some((4, lightmap_secondary, lightmap_sampler));
                out[3] = Some((6, reflection_probe, reflection_sampler));
                out
            }
            Self::GeometricDiffuse {
                color_map,
                color_sampler,
                lightmap_secondary,
                lightmap_sampler,
            } => {
                let mut out = [None; 11];
                out[0] = Some((0, color_map, color_sampler));
                out[2] = Some((4, lightmap_secondary, lightmap_sampler));
                out
            }
            Self::Diffuse(base)
            | Self::RevealDiffuse { base, .. }
            | Self::DiffuseColorMultiply { base, .. }
            | Self::DiffuseLayerMultiply { base, .. }
            | Self::DiffuseLayered { base, .. }
            | Self::DiffuseRevealLayered { base, .. } => diffuse_bindings(base),
            Self::BaseSpecularLayered { base, .. }
            | Self::BaseSpecularRevealLayered { base, .. }
            | Self::BaseSpecularThreeLayerReveal { base, .. }
            | Self::BaseSpecularThreeLayerDoubleReveal { base, .. } => specular_bindings(base),
            Self::GeometricSpecular(base) | Self::GeometricLayered { base, .. } => {
                let mut out = [None; 11];
                out[0] = Some((0, &base.color_map, &base.color_sampler));
                out[2] = Some((4, &base.lightmap_secondary, &base.lightmap_sampler));
                out[3] = Some((6, &base.reflection_probe, &base.reflection_sampler));
                if matches!(self, Self::GeometricSpecular(_)) {
                    out[4] = Some((8, &base.specular_map, &base.specular_sampler));
                } else {
                    out[10] = Some((22, &base.specular_map, &base.specular_sampler));
                }
                out
            }
            Self::ColorMultiply(layer) | Self::ColorBlend(layer) | Self::ColorAdd(layer) => {
                base_bindings(&layer.base)
            }
            Self::Lightmapped(base)
            | Self::NormalSpecular { base, .. }
            | Self::RevealNormalSpecular { base, .. }
            | Self::Layered { base, .. }
            | Self::RevealLayered { base, .. }
            | Self::SpecularLayered { base, .. }
            | Self::RevealSpecularLayered { base, .. }
            | Self::LayerAdd { base, .. }
            | Self::RevealOverlay { base, .. }
            | Self::ColorOverlays { base, .. }
            | Self::ColorThreeLayer { base, .. }
            | Self::ThreeLayerNormals { base, .. }
            | Self::ThreeLayerReveal { base, .. }
            | Self::ThreeLayerRetainedSpecular { base, .. }
            | Self::ThreeLayerDoubleReveal { base, .. }
            | Self::NoBaseGlossRevealLayered { base, .. } => base_bindings(base),
        };
        match self {
            Self::Ember {
                super_map,
                super_sampler,
                ember_map,
                ember_sampler,
                ..
            } => {
                out[4] = Some((8, super_map, super_sampler));
                out[5] = Some((10, ember_map, ember_sampler));
            }
            Self::NormalSpecular {
                specular_map,
                specular_sampler,
                ..
            }
            | Self::RevealNormalSpecular {
                specular_map,
                specular_sampler,
                ..
            } => out[4] = Some((8, specular_map, specular_sampler)),
            Self::ColorMultiply(layer) | Self::ColorBlend(layer) | Self::ColorAdd(layer) => {
                out[4] = Some((8, &layer.color_map, &layer.color_sampler))
            }
            Self::DiffuseColorMultiply {
                color_map,
                color_sampler,
                second_layer,
                ..
            } => {
                out[4] = Some((8, color_map, color_sampler));
                if let Some((image, sampler)) = second_layer {
                    out[7] = Some((14, image, sampler));
                }
            }
            Self::GeometricLayered {
                color_map,
                color_sampler,
                ..
            } => out[4] = Some((8, color_map, color_sampler)),
            Self::Layered { layer, .. }
            | Self::RevealLayered { layer, .. }
            | Self::BaseSpecularLayered { layer, .. }
            | Self::DiffuseLayered { layer, .. }
            | Self::DiffuseRevealLayered { layer, .. } => layer_bindings(&mut out, 4, 8, layer),
            Self::SpecularLayered {
                layer,
                layer_specular_map,
                layer_specular_sampler,
                ..
            }
            | Self::RevealSpecularLayered {
                layer,
                layer_specular_map,
                layer_specular_sampler,
                ..
            } => {
                layer_bindings(&mut out, 4, 8, layer);
                out[6] = Some((12, layer_specular_map, layer_specular_sampler));
            }
            Self::BaseSpecularRevealLayered { layer, .. }
            | Self::NoBaseGlossRevealLayered { layer, .. } => {
                specular_layer_bindings(&mut out, 4, 8, layer)
            }
            Self::LayerAdd {
                layer1,
                color_map2,
                color_sampler2,
                ..
            }
            | Self::DiffuseLayerMultiply {
                layer1,
                color_map2,
                color_sampler2,
                ..
            } => {
                layer_bindings(&mut out, 4, 8, layer1);
                out[7] = Some((14, color_map2, color_sampler2));
            }
            Self::RevealOverlay {
                multiply3,
                layer1,
                color_map2,
                color_sampler2,
                ..
            } => {
                specular_layer_bindings(&mut out, 4, 8, layer1);
                out[7] = Some((14, color_map2, color_sampler2));
                if let Some((image, sampler)) = multiply3 {
                    out[8] = Some((26, image, sampler));
                }
            }
            Self::ColorOverlays {
                color_map1,
                color_sampler1,
                color_map2,
                color_sampler2,
                ..
            } => {
                out[4] = Some((8, color_map1, color_sampler1));
                out[7] = Some((14, color_map2, color_sampler2));
            }
            Self::ColorThreeLayer {
                color_map1,
                color_sampler1,
                layer2,
                specular2,
                ..
            } => {
                out[4] = Some((8, color_map1, color_sampler1));
                layer_bindings(&mut out, 7, 14, layer2);
                if let Some((image, sampler)) = specular2 {
                    out[9] = Some((18, image, sampler));
                }
            }
            Self::ThreeLayerNormals { layer1, layer2, .. } => {
                layer_bindings(&mut out, 4, 8, layer1);
                layer_bindings(&mut out, 7, 14, layer2);
            }
            Self::ThreeLayerReveal { layer1, layer2, .. }
            | Self::BaseSpecularThreeLayerReveal { layer1, layer2, .. } => {
                layer_bindings(&mut out, 4, 8, layer1);
                specular_layer_bindings(&mut out, 7, 14, layer2);
            }
            Self::ThreeLayerRetainedSpecular { layer1, layer2, .. } => {
                specular_layer_bindings(&mut out, 4, 8, layer1);
                layer_bindings(&mut out, 7, 14, layer2);
            }
            Self::ThreeLayerDoubleReveal { layer1, layer2, .. }
            | Self::BaseSpecularThreeLayerDoubleReveal { layer1, layer2, .. } => {
                specular_layer_bindings(&mut out, 4, 8, layer1);
                specular_layer_bindings(&mut out, 7, 14, layer2);
            }
            Self::Lava { .. }
            | Self::Wallbuy { .. }
            | Self::Unlit { .. }
            | Self::GeometricGloss { .. }
            | Self::GeometricDiffuse { .. }
            | Self::Lightmapped(_)
            | Self::Diffuse(_)
            | Self::RevealDiffuse { .. }
            | Self::GeometricSpecular(_) => {}
        }
        out.into_iter().flatten()
    }

    fn family(&self) -> NativeWorldFamily {
        match self {
            Self::Ember { .. } => NativeWorldFamily::Ember,
            Self::Lava { .. } => NativeWorldFamily::Lava,
            Self::Wallbuy { .. } => NativeWorldFamily::Wallbuy,
            Self::Unlit { program, .. } => match program {
                NativeUnlitProgram::Replace => NativeWorldFamily::UnlitReplace,
                NativeUnlitProgram::Add => NativeWorldFamily::UnlitAdd,
                NativeUnlitProgram::DecalAdd => NativeWorldFamily::UnlitDecalAdd,
                NativeUnlitProgram::DecalMultiply => NativeWorldFamily::UnlitDecalMultiply,
            },
            Self::GeometricGloss { .. } => NativeWorldFamily::GeometricGloss,
            Self::GeometricDiffuse { .. } => NativeWorldFamily::GeometricDiffuse,
            Self::Lightmapped(_) => NativeWorldFamily::Lightmapped,
            Self::Diffuse(_) => NativeWorldFamily::Diffuse,
            Self::RevealDiffuse { .. } => NativeWorldFamily::RevealDiffuse,
            Self::NormalSpecular { .. } => NativeWorldFamily::NormalSpecular,
            Self::RevealNormalSpecular { .. } => NativeWorldFamily::RevealNormalSpecular,
            Self::GeometricSpecular(_) => NativeWorldFamily::GeometricSpecular,
            Self::DiffuseColorMultiply { second_layer, .. } => {
                if second_layer.is_some() {
                    NativeWorldFamily::DiffuseDoubleMultiply
                } else {
                    NativeWorldFamily::DiffuseColorMultiply
                }
            }
            Self::ColorMultiply(_) => NativeWorldFamily::ColorMultiply,
            Self::ColorBlend(_) => NativeWorldFamily::ColorBlend,
            Self::ColorAdd(_) => NativeWorldFamily::ColorAdd,
            Self::Layered { hard_select, .. } => {
                if *hard_select {
                    NativeWorldFamily::LayerSelected
                } else {
                    NativeWorldFamily::Layered
                }
            }
            Self::RevealLayered { .. } => NativeWorldFamily::RevealLayered,
            Self::SpecularLayered { .. } => NativeWorldFamily::SpecularLayered,
            Self::RevealSpecularLayered { .. } => NativeWorldFamily::RevealSpecularLayered,
            Self::DiffuseLayerMultiply {
                alpha_reveal_parms1,
                ..
            } => {
                if alpha_reveal_parms1.is_some() {
                    NativeWorldFamily::DiffuseRevealMultiply
                } else {
                    NativeWorldFamily::DiffuseLayerMultiply
                }
            }
            Self::LayerAdd { .. } => NativeWorldFamily::LayerAdd,
            Self::RevealOverlay {
                additive,
                multiply3,
                ..
            } => {
                if multiply3.is_some() {
                    NativeWorldFamily::FourLayerRevealAddMultiply
                } else if *additive {
                    NativeWorldFamily::RevealAdd
                } else {
                    NativeWorldFamily::RevealMultiply
                }
            }
            Self::ColorOverlays { add_multiply, .. } => {
                if *add_multiply {
                    NativeWorldFamily::ColorAddMultiply
                } else {
                    NativeWorldFamily::ColorDoubleBlend
                }
            }
            Self::ColorThreeLayer {
                alpha_reveal_parms2,
                specular2,
                ..
            } => {
                if alpha_reveal_parms2.is_some() {
                    NativeWorldFamily::ColorThreeLayerReveal
                } else if specular2.is_some() {
                    NativeWorldFamily::ColorThreeLayerSpecular
                } else {
                    NativeWorldFamily::ColorThreeLayerNormal
                }
            }
            Self::ThreeLayerNormals { .. } => NativeWorldFamily::ThreeLayerNormals,
            Self::ThreeLayerReveal { .. } => NativeWorldFamily::ThreeLayerReveal,
            Self::ThreeLayerRetainedSpecular { .. } => {
                NativeWorldFamily::ThreeLayerRetainedSpecular
            }
            Self::ThreeLayerDoubleReveal { .. } => NativeWorldFamily::ThreeLayerDoubleReveal,
            Self::BaseSpecularLayered { .. } => NativeWorldFamily::BaseSpecularLayered,
            Self::BaseSpecularRevealLayered { .. } => NativeWorldFamily::BaseSpecularRevealLayered,
            Self::BaseSpecularThreeLayerReveal { .. } => {
                NativeWorldFamily::BaseSpecularThreeLayerReveal
            }
            Self::BaseSpecularThreeLayerDoubleReveal { .. } => {
                NativeWorldFamily::BaseSpecularThreeLayerDoubleReveal
            }
            Self::DiffuseLayered { .. } => NativeWorldFamily::DiffuseLayered,
            Self::DiffuseRevealLayered { .. } => NativeWorldFamily::DiffuseRevealLayered,
            Self::NoBaseGlossRevealLayered { .. } => NativeWorldFamily::NoBaseGlossRevealLayered,
            Self::GeometricLayered { .. } => NativeWorldFamily::GeometricLayered,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum NativeWorldFamily {
    Ember,
    Lava,
    Wallbuy,
    UnlitReplace,
    UnlitAdd,
    UnlitDecalAdd,
    UnlitDecalMultiply,
    GeometricDiffuse,
    GeometricGloss,
    Lightmapped,
    Diffuse,
    RevealDiffuse,
    NormalSpecular,
    RevealNormalSpecular,
    GeometricSpecular,
    DiffuseColorMultiply,
    DiffuseDoubleMultiply,
    ColorMultiply,
    ColorBlend,
    ColorAdd,
    Layered,
    LayerSelected,
    RevealLayered,
    SpecularLayered,
    RevealSpecularLayered,
    DiffuseLayerMultiply,
    DiffuseRevealMultiply,
    LayerAdd,
    FourLayerRevealAddMultiply,
    RevealMultiply,
    RevealAdd,
    ColorDoubleBlend,
    ColorAddMultiply,
    ColorThreeLayerReveal,
    ColorThreeLayerSpecular,
    ColorThreeLayerNormal,
    ThreeLayerNormals,
    ThreeLayerReveal,
    ThreeLayerRetainedSpecular,
    ThreeLayerDoubleReveal,
    BaseSpecularLayered,
    BaseSpecularRevealLayered,
    BaseSpecularThreeLayerReveal,
    BaseSpecularThreeLayerDoubleReveal,
    DiffuseLayered,
    DiffuseRevealLayered,
    NoBaseGlossRevealLayered,
    GeometricLayered,
}

impl NativeWorldFamily {
    fn auxiliary_stride(self) -> Option<u64> {
        match self {
            Self::LayerAdd
            | Self::DiffuseLayerMultiply
            | Self::DiffuseRevealMultiply
            | Self::RevealAdd
            | Self::RevealMultiply
            | Self::ColorThreeLayerNormal
            | Self::ColorThreeLayerReveal
            | Self::ColorThreeLayerSpecular => Some(12),
            Self::DiffuseColorMultiply
            | Self::ColorMultiply
            | Self::ColorBlend
            | Self::ColorAdd
            | Self::GeometricLayered => Some(4),
            Self::ColorDoubleBlend
            | Self::ColorAddMultiply
            | Self::DiffuseDoubleMultiply
            | Self::LayerSelected
            | Self::Layered
            | Self::RevealLayered
            | Self::SpecularLayered
            | Self::RevealSpecularLayered
            | Self::BaseSpecularLayered
            | Self::BaseSpecularRevealLayered
            | Self::DiffuseLayered
            | Self::DiffuseRevealLayered
            | Self::NoBaseGlossRevealLayered => Some(8),
            Self::FourLayerRevealAddMultiply
            | Self::ThreeLayerNormals
            | Self::ThreeLayerReveal
            | Self::ThreeLayerRetainedSpecular
            | Self::ThreeLayerDoubleReveal
            | Self::BaseSpecularThreeLayerReveal
            | Self::BaseSpecularThreeLayerDoubleReveal => Some(16),
            _ => None,
        }
    }

    fn reveal1_binding(self) -> Option<u32> {
        match self {
            Self::FourLayerRevealAddMultiply => Some(24),
            Self::DiffuseRevealMultiply => Some(24),
            Self::RevealLayered | Self::RevealSpecularLayered => Some(14),
            Self::RevealAdd
            | Self::RevealMultiply
            | Self::BaseSpecularRevealLayered
            | Self::RevealNormalSpecular
            | Self::RevealDiffuse
            | Self::DiffuseRevealLayered
            | Self::NoBaseGlossRevealLayered => Some(24),
            _ => None,
        }
    }
}

#[derive(ShaderType)]
struct NativeWorldRevealMaterial {
    alpha_reveal_parms1: Vec4,
}

#[derive(ShaderType)]
struct NativeWorldThreeLayerRevealMaterial {
    alpha_reveal_parms2: Vec4,
}

#[derive(ShaderType)]
struct NativeWorldThreeLayerDoubleRevealMaterial {
    alpha_reveal_parms1: Vec4,
    alpha_reveal_parms2: Vec4,
}

/// Values decoded from this technique's original state bits. The camera's
/// depth convention must agree with depth_compare and its depth clear value.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeWorldState {
    pub blend: Option<BlendState>,
    pub alpha_test: bool,
    pub depth_compare: CompareFunction,
    pub depth_write: bool,
    pub polygon_offset: u8,
    pub cull_mode: Option<Face>,
    pub front_face: FrontFace,
    pub color_write: ColorWrites,
}

impl NativeWorldState {
    pub(crate) fn depth_bias(self) -> DepthBiasState {
        // 0046baa0 negates the logical offsets for the original reverse-Z rasterizer.
        DepthBiasState {
            constant: i32::from(self.polygon_offset) * 256,
            slope_scale: f32::from(self.polygon_offset),
            clamp: 0.0,
        }
    }
}

#[derive(Component, Clone)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<NativeWorldSurface>)]
pub struct NativeWorldSurface {
    pub pass: NativeWorldPass,
    pub sort_key: u64,
    pub source_surface_index: u32,
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
    pub pass: NativeWorldPass,
    pub sort_key: u64,
    pub source_surface_index: u32,
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
            pass: surface.pass,
            sort_key: surface.sort_key,
            source_surface_index: surface.source_surface_index,
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
        load_shader_library!(app, "native_world_wallbuy.wesl");
        load_shader_library!(app, "native_world_lava.wesl");
        load_shader_library!(app, "native_world_ember.wesl");
        load_shader_library!(app, "native_world_layers.wesl");
        load_shader_library!(app, "native_world_plain.wesl");
        load_shader_library!(app, "native_world_color_layers.wesl");
        load_shader_library!(app, "native_world_three_layers.wesl");
        load_shader_library!(app, "native_world_variants.wesl");
        embedded_asset!(app, "native_entry.wesl");
        embedded_asset!(app, "native_layer_entry.wesl");
        embedded_asset!(app, "native_plain_entry.wesl");
        embedded_asset!(app, "native_color_layer_entry.wesl");
        embedded_asset!(app, "native_three_layer_entry.wesl");
        embedded_asset!(app, "native_variants_entry.wesl");
        embedded_asset!(app, "native_unlit_entry.wesl");
        embedded_asset!(app, "native_ember_entry.wesl");
        let shader = load_embedded_asset!(app, "native_entry.wesl");
        let layer_shader = load_embedded_asset!(app, "native_layer_entry.wesl");
        let plain_shader = load_embedded_asset!(app, "native_plain_entry.wesl");
        let color_layer_shader = load_embedded_asset!(app, "native_color_layer_entry.wesl");
        let three_layer_shader = load_embedded_asset!(app, "native_three_layer_entry.wesl");
        let variants_shader = load_embedded_asset!(app, "native_variants_entry.wesl");
        let unlit_shader = load_embedded_asset!(app, "native_unlit_entry.wesl");
        let ember_shader = load_embedded_asset!(app, "native_ember_entry.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(NativePipeline::new(
                shader,
                layer_shader,
                plain_shader,
                color_layer_shader,
                three_layer_shader,
                variants_shader,
                unlit_shader,
                ember_shader,
            ))
            .insert_resource(GpuPreprocessingSupport {
                max_supported_mode: GpuPreprocessingMode::None,
            })
            .init_resource::<SpecializedRenderPipelines<NativePipeline>>()
            .init_resource::<NativeUniformGroups>()
            .init_resource::<NativeOrderedDraws>()
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
pub(crate) struct NativePipeline {
    shader: Handle<Shader>,
    layer_shader: Handle<Shader>,
    plain_shader: Handle<Shader>,
    color_layer_shader: Handle<Shader>,
    three_layer_shader: Handle<Shader>,
    variants_shader: Handle<Shader>,
    unlit_shader: Handle<Shader>,
    ember_shader: Handle<Shader>,
    scene_layout: BindGroupLayoutDescriptor,
    object_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    diffuse_material_layout: BindGroupLayoutDescriptor,
    extra_texture_material_layout: BindGroupLayoutDescriptor,
    geometric_specular_material_layout: BindGroupLayoutDescriptor,
    layer_material_layout: BindGroupLayoutDescriptor,
    reveal_material_layout: BindGroupLayoutDescriptor,
    reveal_specular_material_layout: BindGroupLayoutDescriptor,
    three_layer_material_layout: BindGroupLayoutDescriptor,
    three_layer_double_reveal_material_layout: BindGroupLayoutDescriptor,
    variant_material_layouts: HashMap<NativeWorldFamily, BindGroupLayoutDescriptor>,
}

impl NativePipeline {
    fn new(
        shader: Handle<Shader>,
        layer_shader: Handle<Shader>,
        plain_shader: Handle<Shader>,
        color_layer_shader: Handle<Shader>,
        three_layer_shader: Handle<Shader>,
        variants_shader: Handle<Shader>,
        unlit_shader: Handle<Shader>,
        ember_shader: Handle<Shader>,
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
        entries.retain(|entry| entry.binding != 14);
        for binding in [14, 16, 18] {
            entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (
                        binding,
                        texture_2d(TextureSampleType::Float { filterable: true }),
                    ),
                    (binding + 1, sampler(SamplerBindingType::Filtering)),
                ),
            ));
        }
        let mut three_layer_entries = entries.clone();
        three_layer_entries.retain(|entry| entry.binding != 12 && entry.binding != 13);
        three_layer_entries.push(
            uniform_buffer::<NativeWorldThreeLayerRevealMaterial>(false)
                .build(20, ShaderStages::FRAGMENT),
        );
        let three_layer_material_layout = BindGroupLayoutDescriptor::new(
            "native three-layer reveal material",
            &three_layer_entries,
        );
        entries.push(
            uniform_buffer::<NativeWorldThreeLayerDoubleRevealMaterial>(false)
                .build(21, ShaderStages::FRAGMENT),
        );
        let three_layer_double_reveal_material_layout =
            BindGroupLayoutDescriptor::new("native three-layer double reveal material", &entries);
        let variant_material_layouts = [
            (
                NativeWorldFamily::ColorThreeLayerReveal,
                &[0, 2, 4, 6, 8, 14, 16, 18][..],
            ),
            (
                NativeWorldFamily::ColorThreeLayerSpecular,
                &[0, 2, 4, 6, 8, 14, 16, 18][..],
            ),
            (
                NativeWorldFamily::ColorThreeLayerNormal,
                &[0, 2, 4, 6, 8, 14, 16][..],
            ),
            (
                NativeWorldFamily::RevealMultiply,
                &[0, 2, 4, 6, 8, 10, 12, 14][..],
            ),
            (
                NativeWorldFamily::RevealAdd,
                &[0, 2, 4, 6, 8, 10, 12, 14][..],
            ),
            (
                NativeWorldFamily::DiffuseLayerMultiply,
                &[0, 2, 4, 8, 10, 14][..],
            ),
            (
                NativeWorldFamily::DiffuseRevealMultiply,
                &[0, 2, 4, 8, 10, 14][..],
            ),
            (NativeWorldFamily::LayerAdd, &[0, 2, 4, 6, 8, 10, 14][..]),
            (
                NativeWorldFamily::ThreeLayerNormals,
                &[0, 2, 4, 6, 8, 10, 14, 16][..],
            ),
            (
                NativeWorldFamily::ThreeLayerRetainedSpecular,
                &[0, 2, 4, 6, 8, 10, 12, 14, 16][..],
            ),
            (
                NativeWorldFamily::ColorDoubleBlend,
                &[0, 2, 4, 6, 8, 14][..],
            ),
            (
                NativeWorldFamily::ColorAddMultiply,
                &[0, 2, 4, 6, 8, 14][..],
            ),
            (
                NativeWorldFamily::FourLayerRevealAddMultiply,
                &[0, 2, 4, 6, 8, 10, 12, 14, 26][..],
            ),
            (NativeWorldFamily::GeometricGloss, &[0, 4, 6][..]),
            (NativeWorldFamily::Ember, &[0, 2, 4, 6, 8, 10][..]),
            (NativeWorldFamily::Lava, &[0, 8, 10][..]),
            (NativeWorldFamily::Wallbuy, &[0][..]),
            (NativeWorldFamily::UnlitReplace, &[0][..]),
            (NativeWorldFamily::UnlitAdd, &[0][..]),
            (NativeWorldFamily::UnlitDecalAdd, &[0][..]),
            (NativeWorldFamily::UnlitDecalMultiply, &[0][..]),
            (NativeWorldFamily::RevealDiffuse, &[0, 2, 4][..]),
            (
                NativeWorldFamily::RevealNormalSpecular,
                &[0, 2, 4, 6, 8][..],
            ),
            (NativeWorldFamily::GeometricDiffuse, &[0, 4][..]),
            (
                NativeWorldFamily::BaseSpecularLayered,
                &[0, 2, 4, 6, 8, 10, 22][..],
            ),
            (
                NativeWorldFamily::BaseSpecularRevealLayered,
                &[0, 2, 4, 6, 8, 10, 12, 22][..],
            ),
            (
                NativeWorldFamily::BaseSpecularThreeLayerReveal,
                &[0, 2, 4, 6, 8, 10, 14, 16, 18, 22][..],
            ),
            (
                NativeWorldFamily::BaseSpecularThreeLayerDoubleReveal,
                &[0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 22][..],
            ),
            (
                NativeWorldFamily::SpecularLayered,
                &[0, 2, 4, 6, 8, 10, 12][..],
            ),
            (NativeWorldFamily::DiffuseColorMultiply, &[0, 2, 4, 8][..]),
            (
                NativeWorldFamily::DiffuseDoubleMultiply,
                &[0, 2, 4, 8, 14][..],
            ),
            (NativeWorldFamily::DiffuseLayered, &[0, 2, 4, 8, 10][..]),
            (
                NativeWorldFamily::DiffuseRevealLayered,
                &[0, 2, 4, 8, 10][..],
            ),
            (
                NativeWorldFamily::NoBaseGlossRevealLayered,
                &[0, 2, 4, 6, 8, 10, 12][..],
            ),
            (NativeWorldFamily::GeometricLayered, &[0, 4, 6, 8, 22][..]),
        ]
        .into_iter()
        .map(|(family, texture_bindings)| {
            let mut entries = Vec::with_capacity(texture_bindings.len() * 2 + 1);
            for &binding in texture_bindings {
                let texture = if binding == 6 {
                    texture_cube(TextureSampleType::Float { filterable: true })
                } else {
                    texture_2d(TextureSampleType::Float { filterable: true })
                };
                entries.push(texture.build(binding, ShaderStages::FRAGMENT));
                entries.push(
                    sampler(SamplerBindingType::Filtering)
                        .build(binding + 1, ShaderStages::FRAGMENT),
                );
            }
            if matches!(
                family,
                NativeWorldFamily::UnlitAdd | NativeWorldFamily::UnlitReplace
            ) {
                entries.push(
                    uniform_buffer::<NativeUnlitConstants>(false).build(2, ShaderStages::FRAGMENT),
                );
            }
            if family == NativeWorldFamily::Wallbuy {
                entries.push(
                    uniform_buffer::<NativeWorldParallaxLayer>(false)
                        .build(3, ShaderStages::VERTEX_FRAGMENT),
                );
            }
            if family == NativeWorldFamily::Lava {
                entries.push(
                    uniform_buffer::<NativeLavaConstants>(false).build(12, ShaderStages::FRAGMENT),
                );
            }
            if family == NativeWorldFamily::Ember {
                entries.push(
                    uniform_buffer::<NativeEmberConstants>(false)
                        .build(12, ShaderStages::VERTEX_FRAGMENT),
                );
            }
            if let Some(binding) = family.reveal1_binding() {
                entries.push(
                    uniform_buffer::<NativeWorldRevealMaterial>(false)
                        .build(binding, ShaderStages::FRAGMENT),
                );
            }
            match family {
                NativeWorldFamily::ColorThreeLayerReveal
                | NativeWorldFamily::BaseSpecularThreeLayerReveal => entries.push(
                    uniform_buffer::<NativeWorldThreeLayerRevealMaterial>(false)
                        .build(20, ShaderStages::FRAGMENT),
                ),
                NativeWorldFamily::ThreeLayerRetainedSpecular
                | NativeWorldFamily::BaseSpecularThreeLayerDoubleReveal => entries.push(
                    uniform_buffer::<NativeWorldThreeLayerDoubleRevealMaterial>(false)
                        .build(21, ShaderStages::FRAGMENT),
                ),
                _ => {}
            }
            (
                family,
                BindGroupLayoutDescriptor::new("native world variant material", &entries),
            )
        })
        .collect();
        Self {
            shader,
            layer_shader,
            plain_shader,
            color_layer_shader,
            three_layer_shader,
            variants_shader,
            unlit_shader,
            ember_shader,
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
            three_layer_material_layout,
            three_layer_double_reveal_material_layout,
            variant_material_layouts,
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
            NativeWorldFamily::LayerSelected | NativeWorldFamily::Layered => {
                &self.layer_material_layout
            }
            NativeWorldFamily::RevealLayered => &self.reveal_material_layout,
            NativeWorldFamily::RevealSpecularLayered => &self.reveal_specular_material_layout,
            NativeWorldFamily::ThreeLayerReveal => &self.three_layer_material_layout,
            NativeWorldFamily::ThreeLayerDoubleReveal => {
                &self.three_layer_double_reveal_material_layout
            }
            NativeWorldFamily::Ember
            | NativeWorldFamily::Lava
            | NativeWorldFamily::Wallbuy
            | NativeWorldFamily::UnlitReplace
            | NativeWorldFamily::UnlitAdd
            | NativeWorldFamily::UnlitDecalAdd
            | NativeWorldFamily::UnlitDecalMultiply
            | NativeWorldFamily::RevealDiffuse
            | NativeWorldFamily::RevealNormalSpecular
            | NativeWorldFamily::GeometricGloss
            | NativeWorldFamily::GeometricDiffuse
            | NativeWorldFamily::SpecularLayered
            | NativeWorldFamily::BaseSpecularLayered
            | NativeWorldFamily::BaseSpecularRevealLayered
            | NativeWorldFamily::DiffuseLayerMultiply
            | NativeWorldFamily::DiffuseRevealMultiply
            | NativeWorldFamily::FourLayerRevealAddMultiply
            | NativeWorldFamily::ColorDoubleBlend
            | NativeWorldFamily::ColorAddMultiply
            | NativeWorldFamily::ThreeLayerRetainedSpecular
            | NativeWorldFamily::ThreeLayerNormals
            | NativeWorldFamily::LayerAdd
            | NativeWorldFamily::RevealAdd
            | NativeWorldFamily::RevealMultiply
            | NativeWorldFamily::ColorThreeLayerNormal
            | NativeWorldFamily::ColorThreeLayerSpecular
            | NativeWorldFamily::ColorThreeLayerReveal
            | NativeWorldFamily::BaseSpecularThreeLayerReveal
            | NativeWorldFamily::BaseSpecularThreeLayerDoubleReveal
            | NativeWorldFamily::DiffuseDoubleMultiply
            | NativeWorldFamily::DiffuseColorMultiply
            | NativeWorldFamily::DiffuseLayered
            | NativeWorldFamily::DiffuseRevealLayered
            | NativeWorldFamily::NoBaseGlossRevealLayered
            | NativeWorldFamily::GeometricLayered => &self.variant_material_layouts[&family],
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct NativePipelineKey {
    target: TextureFormat,
    samples: u32,
    state: NativeWorldState,
    family: NativeWorldFamily,
    vertex_stride: u64,
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
        // Original world marks (005bf460) use float2 UVs/lightmap UVs,
        // rather than the packed coordinates in GfxWorld's 36-byte stream.
        if key.vertex_stride == 44 {
            vertex_layout.array_stride = 44;
            vertex_layout.attributes[2].format = VertexFormat::Float32x2;
            vertex_layout.attributes[3].offset = 28;
            vertex_layout.attributes[4].offset = 32;
            vertex_layout.attributes[5].offset = 36;
            vertex_layout.attributes[5].format = VertexFormat::Float32x2;
        } else {
            assert_eq!(key.vertex_stride, 36, "Original world vertex layout");
        }
        if matches!(
            key.family,
            NativeWorldFamily::GeometricGloss
                | NativeWorldFamily::GeometricDiffuse
                | NativeWorldFamily::GeometricSpecular
                | NativeWorldFamily::GeometricLayered
        ) {
            vertex_layout
                .attributes
                .retain(|attribute| attribute.shader_location != 4);
            vertex_layout.attributes[0].format = VertexFormat::Float32x3;
        }
        if matches!(
            key.family,
            NativeWorldFamily::UnlitReplace
                | NativeWorldFamily::UnlitAdd
                | NativeWorldFamily::UnlitDecalAdd
                | NativeWorldFamily::UnlitDecalMultiply
        ) {
            vertex_layout
                .attributes
                .retain(|attribute| attribute.shader_location <= 2);
            vertex_layout.attributes[0].format = VertexFormat::Float32x3;
        }
        let mut buffers = vec![vertex_layout];
        if let Some(stride) = key.family.auxiliary_stride() {
            let mut attributes = vec![VertexAttribute {
                format: if stride == 16
                    || stride == 12
                    || matches!(
                        key.family,
                        NativeWorldFamily::DiffuseDoubleMultiply
                            | NativeWorldFamily::ColorDoubleBlend
                            | NativeWorldFamily::ColorAddMultiply
                    ) {
                    VertexFormat::Float16x4
                } else {
                    VertexFormat::Float16x2
                },
                offset: 0,
                shader_location: 6,
            }];
            if stride == 8
                && !matches!(
                    key.family,
                    NativeWorldFamily::DiffuseDoubleMultiply
                        | NativeWorldFamily::ColorDoubleBlend
                        | NativeWorldFamily::ColorAddMultiply
                )
            {
                attributes.push(VertexAttribute {
                    format: VertexFormat::Unorm8x4,
                    offset: 4,
                    shader_location: 7,
                });
            }
            if stride == 12 {
                attributes.push(VertexAttribute {
                    format: VertexFormat::Unorm8x4,
                    offset: 8,
                    shader_location: 7,
                });
            }
            if stride == 16 {
                attributes.extend([
                    VertexAttribute {
                        format: if key.family == NativeWorldFamily::FourLayerRevealAddMultiply {
                            VertexFormat::Float16x2
                        } else {
                            VertexFormat::Unorm8x4
                        },
                        offset: 8,
                        shader_location: 7,
                    },
                    VertexAttribute {
                        format: VertexFormat::Unorm8x4,
                        offset: 12,
                        shader_location: 8,
                    },
                ]);
            }
            buffers.push(VertexBufferLayout {
                array_stride: stride,
                step_mode: VertexStepMode::Vertex,
                attributes,
            });
        }
        let (shader, vertex) = match key.family {
            NativeWorldFamily::Ember => (&self.ember_shader, "vertex"),
            NativeWorldFamily::FourLayerRevealAddMultiply => {
                (&self.variants_shader, "vertex_four_layer")
            }
            NativeWorldFamily::ColorDoubleBlend | NativeWorldFamily::ColorAddMultiply => {
                (&self.variants_shader, "vertex_color_overlays")
            }
            NativeWorldFamily::DiffuseLayerMultiply | NativeWorldFamily::DiffuseRevealMultiply => {
                (&self.variants_shader, "vertex_diffuse_three")
            }
            NativeWorldFamily::Lava => (&self.unlit_shader, "vertex_lava"),
            NativeWorldFamily::Wallbuy => (&self.unlit_shader, "vertex_wallbuy"),
            NativeWorldFamily::UnlitDecalAdd => (&self.unlit_shader, "vertex_no_fog"),
            NativeWorldFamily::UnlitReplace
            | NativeWorldFamily::UnlitAdd
            | NativeWorldFamily::UnlitDecalMultiply => (&self.unlit_shader, "vertex_fog"),
            NativeWorldFamily::GeometricDiffuse => (&self.plain_shader, "vertex_geometric_diffuse"),
            NativeWorldFamily::Lightmapped => (&self.shader, "vertex"),
            NativeWorldFamily::Diffuse | NativeWorldFamily::RevealDiffuse => {
                (&self.plain_shader, "vertex_diffuse")
            }
            NativeWorldFamily::NormalSpecular | NativeWorldFamily::RevealNormalSpecular => {
                (&self.plain_shader, "vertex_specular")
            }
            NativeWorldFamily::GeometricGloss | NativeWorldFamily::GeometricSpecular => {
                (&self.plain_shader, "vertex_geometric_specular")
            }
            NativeWorldFamily::ColorMultiply
            | NativeWorldFamily::ColorBlend
            | NativeWorldFamily::ColorAdd => (&self.color_layer_shader, "vertex"),
            NativeWorldFamily::LayerSelected
            | NativeWorldFamily::Layered
            | NativeWorldFamily::RevealLayered
            | NativeWorldFamily::SpecularLayered
            | NativeWorldFamily::RevealSpecularLayered => (&self.layer_shader, "vertex"),
            NativeWorldFamily::ThreeLayerReveal | NativeWorldFamily::ThreeLayerDoubleReveal => {
                (&self.three_layer_shader, "vertex")
            }
            NativeWorldFamily::BaseSpecularLayered
            | NativeWorldFamily::BaseSpecularRevealLayered
            | NativeWorldFamily::NoBaseGlossRevealLayered => {
                (&self.variants_shader, "vertex_layer")
            }
            NativeWorldFamily::LayerAdd
            | NativeWorldFamily::RevealAdd
            | NativeWorldFamily::RevealMultiply
            | NativeWorldFamily::ColorThreeLayerNormal
            | NativeWorldFamily::ColorThreeLayerSpecular
            | NativeWorldFamily::ColorThreeLayerReveal => {
                (&self.variants_shader, "vertex_color_three")
            }
            NativeWorldFamily::ThreeLayerRetainedSpecular
            | NativeWorldFamily::ThreeLayerNormals
            | NativeWorldFamily::BaseSpecularThreeLayerReveal
            | NativeWorldFamily::BaseSpecularThreeLayerDoubleReveal => {
                (&self.variants_shader, "vertex_three")
            }
            NativeWorldFamily::DiffuseLayered | NativeWorldFamily::DiffuseRevealLayered => {
                (&self.variants_shader, "vertex_diffuse_layer")
            }
            NativeWorldFamily::DiffuseDoubleMultiply => {
                (&self.variants_shader, "vertex_diffuse_double_color")
            }
            NativeWorldFamily::DiffuseColorMultiply => {
                (&self.variants_shader, "vertex_diffuse_color")
            }
            NativeWorldFamily::GeometricLayered => {
                (&self.variants_shader, "vertex_geometric_layer")
            }
        };
        let fragment = match key.family {
            NativeWorldFamily::FourLayerRevealAddMultiply => "fragment_four_layer",
            NativeWorldFamily::LayerSelected => "fragment_selected",
            NativeWorldFamily::ColorDoubleBlend => "fragment_color_double_blend",
            NativeWorldFamily::ColorAddMultiply => "fragment_color_add_multiply",
            NativeWorldFamily::Ember => "fragment",
            NativeWorldFamily::Lava => "fragment_lava",
            NativeWorldFamily::Wallbuy => "fragment_wallbuy",
            NativeWorldFamily::UnlitReplace => "fragment_replace",
            NativeWorldFamily::UnlitAdd => "fragment_add",
            NativeWorldFamily::UnlitDecalAdd => "fragment_decal_add",
            NativeWorldFamily::UnlitDecalMultiply => "fragment_decal_multiply",
            NativeWorldFamily::GeometricGloss => "fragment_geometric_gloss",
            NativeWorldFamily::GeometricDiffuse => "fragment_geometric_diffuse",
            NativeWorldFamily::Lightmapped
            | NativeWorldFamily::Layered
            | NativeWorldFamily::ThreeLayerReveal => "fragment",
            NativeWorldFamily::Diffuse => "fragment_diffuse",
            NativeWorldFamily::RevealDiffuse => "fragment_diffuse_reveal",
            NativeWorldFamily::NormalSpecular => "fragment_specular",
            NativeWorldFamily::RevealNormalSpecular => "fragment_specular_reveal",
            NativeWorldFamily::GeometricSpecular => "fragment_geometric_specular",
            NativeWorldFamily::DiffuseDoubleMultiply => "fragment_diffuse_double_multiply",
            NativeWorldFamily::DiffuseColorMultiply => "fragment_diffuse_multiply",
            NativeWorldFamily::ColorMultiply => "fragment_multiply",
            NativeWorldFamily::ColorBlend => "fragment_blend",
            NativeWorldFamily::ColorAdd => "fragment_add",
            NativeWorldFamily::ThreeLayerDoubleReveal => "fragment_double_reveal",
            NativeWorldFamily::RevealLayered => "fragment_reveal",
            NativeWorldFamily::SpecularLayered => "fragment_specular",
            NativeWorldFamily::RevealSpecularLayered => "fragment_reveal_specular",
            NativeWorldFamily::BaseSpecularLayered => "fragment_base_specular_layer",
            NativeWorldFamily::BaseSpecularRevealLayered => "fragment_base_specular_reveal_layer",
            NativeWorldFamily::ColorThreeLayerReveal => "fragment_color_three_reveal",
            NativeWorldFamily::ColorThreeLayerSpecular => "fragment_color_three_specular",
            NativeWorldFamily::DiffuseLayerMultiply => "fragment_diffuse_layer_multiply",
            NativeWorldFamily::DiffuseRevealMultiply => "fragment_diffuse_reveal_multiply",
            NativeWorldFamily::ThreeLayerRetainedSpecular => "fragment_retained_specular",
            NativeWorldFamily::ThreeLayerNormals => "fragment_three_normals",
            NativeWorldFamily::LayerAdd => "fragment_layer_add",
            NativeWorldFamily::RevealAdd => "fragment_reveal_add",
            NativeWorldFamily::RevealMultiply => "fragment_reveal_multiply",
            NativeWorldFamily::ColorThreeLayerNormal => "fragment_color_three_normal",
            NativeWorldFamily::BaseSpecularThreeLayerReveal => {
                "fragment_base_specular_three_layer_reveal"
            }
            NativeWorldFamily::BaseSpecularThreeLayerDoubleReveal => {
                "fragment_base_specular_three_layer_double_reveal"
            }
            NativeWorldFamily::DiffuseLayered => "fragment_diffuse_layer",
            NativeWorldFamily::DiffuseRevealLayered => "fragment_diffuse_reveal_layer",
            NativeWorldFamily::NoBaseGlossRevealLayered => "fragment_no_base_gloss_reveal_layer",
            NativeWorldFamily::GeometricLayered => "fragment_geometric_layer",
        };
        let fragment = match (key.family, key.state.alpha_test) {
            (NativeWorldFamily::Diffuse, true) => "fragment_diffuse_cutout",
            (NativeWorldFamily::NormalSpecular, true) => "fragment_specular_cutout",
            _ => fragment,
        };
        let fragment = match (key.family, key.state.blend.is_some()) {
            (NativeWorldFamily::Diffuse, true) => "fragment_diffuse_blend",
            (NativeWorldFamily::NormalSpecular, true) => "fragment_specular_blend",
            (NativeWorldFamily::GeometricDiffuse, true) => "fragment_geometric_diffuse_blend",
            (NativeWorldFamily::GeometricSpecular, true) => "fragment_geometric_specular_blend",
            _ => fragment,
        };
        let fragment = if key.target.is_srgb() {
            format!("{fragment}_srgb")
        } else {
            fragment.to_owned()
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
                    blend: key.state.blend,
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
                bias: key.state.depth_bias(),
            }),
            multisample: MultisampleState {
                count: key.samples,
                ..default()
            },
            ..default()
        }
    }
}

pub(crate) struct PreparedNativeMaterial {
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
            NativeWorldMaterial::RevealDiffuse {
                alpha_reveal_parms, ..
            } => Some(*alpha_reveal_parms),
            NativeWorldMaterial::RevealNormalSpecular {
                alpha_reveal_parms, ..
            } => Some(*alpha_reveal_parms),
            NativeWorldMaterial::DiffuseLayerMultiply {
                alpha_reveal_parms1: Some(alpha_reveal_parms1),
                ..
            }
            | NativeWorldMaterial::RevealOverlay {
                alpha_reveal_parms1,
                ..
            }
            | NativeWorldMaterial::RevealLayered {
                alpha_reveal_parms1,
                ..
            }
            | NativeWorldMaterial::RevealSpecularLayered {
                alpha_reveal_parms1,
                ..
            }
            | NativeWorldMaterial::BaseSpecularRevealLayered {
                alpha_reveal_parms1,
                ..
            }
            | NativeWorldMaterial::DiffuseRevealLayered {
                alpha_reveal_parms1,
                ..
            }
            | NativeWorldMaterial::NoBaseGlossRevealLayered {
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
        let mut three_layer_uniform = match &source {
            NativeWorldMaterial::ThreeLayerReveal {
                alpha_reveal_parms2,
                ..
            }
            | NativeWorldMaterial::ColorThreeLayer {
                alpha_reveal_parms2: Some(alpha_reveal_parms2),
                ..
            }
            | NativeWorldMaterial::BaseSpecularThreeLayerReveal {
                alpha_reveal_parms2,
                ..
            } => Some(UniformBuffer::from(NativeWorldThreeLayerRevealMaterial {
                alpha_reveal_parms2: *alpha_reveal_parms2,
            })),
            _ => None,
        };
        if let Some(uniform) = &mut three_layer_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut double_reveal_uniform = match &source {
            NativeWorldMaterial::ThreeLayerRetainedSpecular {
                alpha_reveal_parms1,
                alpha_reveal_parms2,
                ..
            }
            | NativeWorldMaterial::ThreeLayerDoubleReveal {
                alpha_reveal_parms1,
                alpha_reveal_parms2,
                ..
            }
            | NativeWorldMaterial::BaseSpecularThreeLayerDoubleReveal {
                alpha_reveal_parms1,
                alpha_reveal_parms2,
                ..
            } => Some(UniformBuffer::from(
                NativeWorldThreeLayerDoubleRevealMaterial {
                    alpha_reveal_parms1: *alpha_reveal_parms1,
                    alpha_reveal_parms2: *alpha_reveal_parms2,
                },
            )),
            _ => None,
        };
        if let Some(uniform) = &mut double_reveal_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut unlit_uniform = match &source {
            NativeWorldMaterial::Unlit {
                scale_rgb: Some(scale_rgb),
                ..
            } => Some(UniformBuffer::from(NativeUnlitConstants {
                scale_rgb: *scale_rgb,
            })),
            _ => None,
        };
        if let Some(uniform) = &mut unlit_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut entries = Vec::with_capacity(views.len() * 2 + usize::from(reveal.is_some()));
        let mut ember_uniform = match &source {
            NativeWorldMaterial::Ember { constants, .. } => Some(UniformBuffer::from(*constants)),
            _ => None,
        };
        if let Some(uniform) = &mut ember_uniform {
            uniform.write_buffer(device, queue);
            entries.push(BindGroupEntry {
                binding: 12,
                resource: uniform.binding().expect("original ember constants"),
            });
        }
        let mut lava_uniform = match &source {
            NativeWorldMaterial::Lava { constants, .. } => Some(UniformBuffer::from(*constants)),
            _ => None,
        };
        if let Some(uniform) = &mut lava_uniform {
            uniform.write_buffer(device, queue);
            entries.push(BindGroupEntry {
                binding: 12,
                resource: uniform.binding().expect("original lava layer constants"),
            });
        }
        let mut wallbuy_uniform = match &source {
            NativeWorldMaterial::Wallbuy { constants, .. } => Some(UniformBuffer::from(*constants)),
            _ => None,
        };
        if let Some(uniform) = &mut wallbuy_uniform {
            uniform.write_buffer(device, queue);
            entries.push(BindGroupEntry {
                binding: 3,
                resource: uniform.binding().expect("original wallbuy constants"),
            });
        }
        if let Some(uniform) = &unlit_uniform {
            entries.push(BindGroupEntry {
                binding: 2,
                resource: uniform.binding().expect("original unlit scaleRGB"),
            });
        }
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
                binding: family
                    .reveal1_binding()
                    .expect("source layer1 reveal binding"),
                resource: uniform.binding().expect("uploaded native reveal constants"),
            });
        }
        if let Some(uniform) = &three_layer_uniform {
            entries.push(BindGroupEntry {
                binding: 20,
                resource: uniform
                    .binding()
                    .expect("uploaded source layer2 reveal constants"),
            });
        }
        if let Some(uniform) = &double_reveal_uniform {
            entries.push(BindGroupEntry {
                binding: 21,
                resource: uniform
                    .binding()
                    .expect("uploaded source layer1/layer2 reveal constants"),
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
pub(crate) struct NativeUniformGroups {
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

pub(crate) fn queue_native_world(
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
    mut translucent: ResMut<NativeOrderedDraws>,
) {
    translucent.0.retain(|(view, _), draws| {
        draws.clear();
        views
            .iter()
            .any(|(candidate, _, _, _)| candidate.retained_view_entity == *view)
    });
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
            let Some(geometry) = geometries.get(surface.geometry) else {
                continue;
            };
            let Some(material) = materials.get(surface.material) else {
                continue;
            };
            let key = NativePipelineKey {
                target: target.main_texture_format(),
                samples: msaa.samples(),
                state: surface.state,
                family: material.family,
                vertex_stride: geometry.vertex_stride,
            };
            let pipeline = variants.specialize(&cache, &pipeline, key);
            if surface.pass != NativeWorldPass::Opaque {
                translucent
                    .0
                    .entry((view.retained_view_entity, surface.pass))
                    .or_default()
                    .push(NativeOrderedDraw {
                        entity,
                        model: false,
                        pipeline,
                        sort_key: (surface.sort_key, surface.source_surface_index),
                    });
                continue;
            }
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

#[derive(Resource, Default)]
pub(crate) struct NativeOrderedDraws(
    pub(crate) HashMap<(RetainedViewEntity, NativeWorldPass), Vec<NativeOrderedDraw>>,
);

pub(crate) struct NativeOrderedDraw {
    pub(crate) entity: Entity,
    pub(crate) model: bool,
    pub(crate) pipeline: CachedRenderPipelineId,
    pub(crate) sort_key: (u64, u32),
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
        draw_surface(
            pass,
            surface,
            material,
            geometry,
            scene_group,
            object_group,
            scene.index(),
            object.index(),
        );
        RenderCommandResult::Success
    }
}

fn draw_surface<'w>(
    pass: &mut TrackedRenderPass<'w>,
    surface: &ExtractedNativeSurface,
    material: &'w PreparedNativeMaterial,
    geometry: &'w PreparedNativeGeometry,
    scene_group: &'w BindGroup,
    object_group: &'w BindGroup,
    scene_offset: u32,
    object_offset: u32,
) {
    pass.set_bind_group(0, scene_group, &[scene_offset]);
    pass.set_bind_group(1, object_group, &[object_offset]);
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
}

/// 004189a0 collects BSP litTrans in list7, merges lists7..9 and19..21,
/// then004438b0 sorts packed keys.00424ff0 draws that list after post-resolve
/// water/floatZ, before the later emissive effects. Keep source order, not
/// camera-distance sorting (including alpha-tested decals with blending off).
pub(crate) fn render_ordered<const PASS: u8>(
    view: ViewQuery<(
        &ExtractedView,
        &ViewTarget,
        &ViewDepthStencilTexture,
        &ExtractedCamera,
        Option<&bevy::camera::MainPassResolutionOverride>,
        &DynamicUniformIndex<NativeWorldScene>,
        Option<(
            &super::native_effect::PreparedEffectView,
            &DynamicUniformIndex<super::native_effect::NativeFxFrame>,
        )>,
    )>,
    draws: Res<NativeOrderedDraws>,
    cache: Res<PipelineCache>,
    groups: Res<NativeUniformGroups>,
    materials: Res<RenderAssets<PreparedNativeMaterial>>,
    geometries: Res<RenderAssets<PreparedNativeGeometry>>,
    surfaces: Query<(
        &ExtractedNativeSurface,
        &DynamicUniformIndex<NativeWorldObject>,
    )>,
    models: super::native_model::ModelOrderedRenderer,
    mut ctx: RenderContext,
) {
    let (stage, label) = match PASS {
        x if x == NativeWorldPass::QuasiOpaque as u8 => {
            (NativeWorldPass::QuasiOpaque, "T6 quasi-opaque")
        }
        x if x == NativeWorldPass::EmissiveOpaque as u8 => {
            (NativeWorldPass::EmissiveOpaque, "T6 emissiveOpaque")
        }
        x if x == NativeWorldPass::Translucent as u8 => {
            (NativeWorldPass::Translucent, "T6 translucent")
        }
        _ => unreachable!("ordered renderer requires an ordered source stage"),
    };
    let (view, target, depth, camera, resolution, scene, effect_depth) = view.into_inner();
    let Some(draws) = draws
        .0
        .get(&(view.retained_view_entity, stage))
        .filter(|draws| !draws.is_empty())
    else {
        return;
    };
    let (Some((_, scene_group)), Some((_, object_group))) = (&groups.scene, &groups.object) else {
        return;
    };
    let attachments = [Some(target.get_color_attachment())];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some(label),
        color_attachments: &attachments,
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some(viewport) = bevy::camera::Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution) {
        pass.set_camera_viewport(&viewport);
    }
    for draw in draws {
        if draw.model {
            if let Some(pipeline) = cache.get_render_pipeline(draw.pipeline) {
                pass.set_render_pipeline(pipeline);
                models.draw(draw.entity, view, effect_depth, &mut pass);
            }
            continue;
        }
        let (Some(pipeline), Ok((surface, object))) = (
            cache.get_render_pipeline(draw.pipeline),
            surfaces.get(draw.entity),
        ) else {
            continue;
        };
        let (Some(material), Some(geometry)) = (
            materials.get(surface.material),
            geometries.get(surface.geometry),
        ) else {
            continue;
        };
        pass.set_render_pipeline(pipeline);
        draw_surface(
            &mut pass,
            surface,
            material,
            geometry,
            scene_group,
            object_group,
            scene.index(),
            object.index(),
        );
    }
}

/// Shader gameTime: periodic sine/cosine, fractional seconds, total seconds.
/// Keep world/model and water programs on the same simulation clock encoding.
pub(crate) fn shader_game_time(seconds: f32) -> Vec4 {
    let fraction = seconds - seconds.floor();
    let (sin, cos) = (fraction * std::f32::consts::TAU).sin_cos();
    Vec4::new(sin, cos, fraction, seconds)
}

#[cfg(test)]
mod clock_tests {
    use super::shader_game_time;

    #[test]
    fn scrolling_time_wraps_without_losing_total_time() {
        for (seconds, fraction) in [
            (0., 0.),
            (0.25, 0.25),
            (1., 0.),
            (7.75, 0.75),
            (-0.25, 0.75),
        ] {
            let time = shader_game_time(seconds);
            assert_eq!(time.z, fraction);
            assert_eq!(time.w, seconds);
            assert!((time.x * time.x + time.y * time.y - 1.).abs() < 1e-6);
        }
    }
}
