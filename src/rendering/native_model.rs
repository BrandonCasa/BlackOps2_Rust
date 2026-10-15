//! Original T6 static-model LIT draws, using each source material's program.
//!
//! NativeWorldPlugin owns geometry upload and common scene/object extraction.
//! This renderer supplies the original model-lighting atlas and cb3 constants
//! for visible placements, then draws PACKED32 geometry in the opaque phase.
use super::animated_models::{AnimatedModel, DynamicLightingOrigin};
use super::native_effect::{
    ExtractedEffectView, NativeFxFrame, PreparedEffectView, effect_scene_layout,
};
use super::skinning::{PreparedAnimatedModel, PreparedSkin};
use super::{
    lighting::lighting_lookup_scale,
    model_lighting::ModelLightingAtlas,
    native::{
        NativeOrderedDraw, NativeOrderedDraws, NativeWorldGeometry, NativeWorldObject,
        NativeWorldPass, NativeWorldScene, NativeWorldState, PreparedNativeGeometry,
    },
};
use bevy::render::uniform::DynamicUniformIndex;
use bevy::{
    asset::{RenderAssetUsages, embedded_asset, load_embedded_asset},
    camera::visibility::{self, VisibilityClass},
    core_pipeline::core_3d::{Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey},
    diagnostic::FrameCount,
    ecs::{
        query::ROQueryItem,
        system::{
            SystemParam, SystemParamItem,
            lifetimeless::{SQuery, SRes},
        },
    },
    mesh::VertexBufferLayout,
    platform::collections::HashMap,
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
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
            binding_types::{sampler, texture_2d, texture_3d, texture_cube, uniform_buffer},
            *,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_component::{SyncComponent, SyncComponentPlugin},
        sync_world::{MainEntity, RenderEntity},
        texture::GpuImage,
        view::{ExtractedView, RenderVisibleEntities, RetainedViewEntity, ViewTarget},
    },
    shader::load_shader_library,
};

/// Construct only after resolving the exact native LIT technique identity.
/// The selected placement's authored reflectionProbeIndex owns the cube input.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GeometricProgram {
    Blended(bool),
    VertexLit,
    Diffuse,
    AlphaTest,
    VertexOcclusion(bool),
}

#[derive(Clone)]
pub(crate) struct NativeModelTextures {
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
    pub normal_map: Handle<Image>,
    pub normal_sampler: SamplerDescriptor<'static>,
    pub reflection_probe: Handle<Image>,
    pub reflection_sampler: SamplerDescriptor<'static>,
}

#[derive(Clone)]
pub(crate) struct NativeModelDiffuseTextures {
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
    pub normal_map: Handle<Image>,
    pub normal_sampler: SamplerDescriptor<'static>,
}

#[derive(Clone)]
pub(crate) struct NativeModelColorTexture {
    pub color_map: Handle<Image>,
    pub color_sampler: SamplerDescriptor<'static>,
}

#[derive(Clone)]
pub(crate) struct NativeModelThreeLayerTextures {
    pub normal_map: Handle<Image>,
    pub normal_sampler: SamplerDescriptor<'static>,
    pub layer1_map: Handle<Image>,
    pub layer1_sampler: SamplerDescriptor<'static>,
    pub layer2_map: Handle<Image>,
    pub layer2_sampler: SamplerDescriptor<'static>,
    pub layer3_map: Handle<Image>,
    pub layer3_sampler: SamplerDescriptor<'static>,
    pub reflection_probe: Handle<Image>,
    pub reflection_sampler: SamplerDescriptor<'static>,
}

// Source tile-blend controls, shared by the MC and MLV programs.
#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeModelTileConstants {
    pub scales: Vec4,
    pub heights_ao: Vec4,
    pub reflection: Vec4,
}

#[derive(Clone)]
pub(crate) struct NativeModelTileSpecular {
    pub layers: [(Handle<Image>, SamplerDescriptor<'static>); 2],
    pub probe: Handle<Image>,
    pub probe_sampler: SamplerDescriptor<'static>,
}

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeModelFourLayerConstants {
    pub source: [Vec4; 13],
}

/// Bound material components at original cb2[5..15], with original packing.
/// The loader supplies every used component from that material's own constants.
#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeModelParallaxConstants {
    pub source: [Vec4; 11],
}

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeModelEmissiveConstants {
    pub controls: Vec4,
    pub gloss_push: Vec4,
}

#[derive(Clone, Copy, ShaderType)]
struct NativeModelCamoParameters {
    weapon_param6: Vec4,
}

#[derive(ShaderType)]
struct NativeModelScriptedScene {
    sun_position: Vec4,
    sun_diffuse: Vec4,
    game_time: Vec4,
    hero_lighting: [Vec4; 3],
}

/// GPU values extracted from the gameplay actor/rig owner's CueParameters.
/// Default source inputs are zeroed by 0045b7e0, then copied by 004189a0.
#[derive(Default, Clone, Copy, ShaderType)]
struct NativeModelScriptInputs {
    vectors: [Vec4; 4],
}

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeModelSmokeConstants {
    pub source: [Vec4; 6],
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Unlit3LayerProgram {
    Opaque,
    Coverage,
    Alpha,
    AlphaNoFalloff,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum EmissiveProgram {
    Alcatraz,
    Transparent,
    ScriptedGlow,
    ScriptMulti,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NormalSpecularProgram {
    Grid,
    VertexLit,
    Scrolling,
}

/// Original weaponParam2 and selected color-detail texture. Base weapon setup
/// 0052c1d0 supplies zero; 0052b7c0 overrides XY/scale/enable for selected camo.
#[derive(Clone)]
pub(crate) struct NativeWeaponDetail {
    pub map: Handle<Image>,
    pub sampler: SamplerDescriptor<'static>,
    pub parameters: Vec4,
}

#[derive(Clone)]
pub(crate) struct NativeModelFlagDetail {
    pub normal_map: Handle<Image>,
    pub normal_sampler: SamplerDescriptor<'static>,
    pub detail_map: Handle<Image>,
    pub detail_sampler: SamplerDescriptor<'static>,
    pub scale: Vec4,
}
#[derive(ShaderType)]
struct NativeModelFlagParams {
    params: Vec4,
}

#[derive(Asset, TypePath, Clone)]
pub(crate) enum NativeModelMaterial {
    SmokePillar {
        textures: [(Handle<Image>, SamplerDescriptor<'static>); 6],
        constants: NativeModelSmokeConstants,
    },
    Flag {
        textures: [(Handle<Image>, SamplerDescriptor<'static>); 3],
        detail: Option<NativeModelFlagDetail>,
        occlusion_amount: Vec4,
        params: Vec4,
    },
    Canopy {
        quasi: bool,
        base: NativeModelColorTexture,
        constants: NativeModelCanopyConstants,
    },
    RevealDiffuse {
        base: NativeModelDiffuseTextures,
        reveal: Vec4,
    },
    MappedOcclusion {
        detail: Option<NativeWeaponDetail>,
        base: NativeModelTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        occlusion_map: Handle<Image>,
        occlusion_sampler: SamplerDescriptor<'static>,
    },
    Feather3Layer {
        layers: [(Handle<Image>, SamplerDescriptor<'static>); 3],
        constants: NativeModelParallaxConstants,
    },
    Unlit3Layer {
        layers: [(Handle<Image>, SamplerDescriptor<'static>); 3],
        probe: Handle<Image>,
        probe_sampler: SamplerDescriptor<'static>,
        constants: NativeModelParallaxConstants,
        vertex_lit: bool,
        program: Unlit3LayerProgram,
    },
    FourLayers {
        layers: [(Handle<Image>, SamplerDescriptor<'static>); 4],
        probe: Handle<Image>,
        probe_sampler: SamplerDescriptor<'static>,
        constants: NativeModelFourLayerConstants,
        vertex_lit: bool,
        fog: bool,
    },
    DetailDiffuse {
        base: NativeModelDiffuseTextures,
        detail_map: Handle<Image>,
        detail_sampler: SamplerDescriptor<'static>,
        detail_scale: Vec4,
        vertex_lit: bool,
    },
    Unlit {
        base: NativeModelColorTexture,
        scale_rgb: Option<Vec4>,
    },
    TileBlend {
        // Macro AO/mix, macro normal, micro normals 1/2, micro colors 1/2.
        textures: [(Handle<Image>, SamplerDescriptor<'static>); 6],
        specular: Option<NativeModelTileSpecular>,
        constants: NativeModelTileConstants,
        vertex_lit: bool,
    },
    ImplicitSpecular(NativeModelTextures),
    VertexLitSpecular(NativeModelTextures),
    VertexOcclusion(NativeModelTextures, bool),
    Diffuse(NativeModelDiffuseTextures, bool),
    BlendedDiffuse(NativeModelDiffuseTextures, bool),
    AlphaDiffuse(NativeModelDiffuseTextures),
    GeometricDiffuse {
        base: NativeModelColorTexture,
        program: GeometricProgram,
    },
    GeometricSpecular {
        base: NativeModelColorTexture,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        reflection_probe: Handle<Image>,
        reflection_sampler: SamplerDescriptor<'static>,
        occlusion_amount: Vec4,
        vertex_lit: bool,
        transparent: bool,
    },
    NormalSpecular {
        program: NormalSpecularProgram,
        base: NativeModelTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        occlusion_amount: Vec4,
    },
    AlphaSpecular {
        vertex_lit: bool,
        base: NativeModelTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        occlusion_amount: Vec4,
    },
    Crystal3Layer {
        vertex_lit: bool,
        textures: NativeModelThreeLayerTextures,
        constants: NativeModelParallaxConstants,
    },
    Camo3Layer {
        textures: NativeModelThreeLayerTextures,
        constants: NativeModelParallaxConstants,
        weapon_normal_map: Handle<Image>,
        weapon_normal_sampler: SamplerDescriptor<'static>,
        weapon_param6: Vec4,
    },
    PhongEmissive {
        program: EmissiveProgram,
        vertex_lit: bool,
        base: NativeModelTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        glow_map: Handle<Image>,
        glow_sampler: SamplerDescriptor<'static>,
        constants: NativeModelEmissiveConstants,
    },
    CharCloth {
        base: NativeModelTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
    },
    ZombieEyes(NativeModelColorTexture),
}

impl NativeModelMaterial {
    pub(crate) fn images(&self) -> impl Iterator<Item = &Handle<Image>> {
        self.textures().map(|(_, image, _)| image)
    }

    fn textures(&self) -> impl Iterator<Item = (u32, &Handle<Image>, &SamplerDescriptor<'static>)> {
        let bindings = match self {
            Self::SmokePillar { textures, .. } => [
                Some((0, &textures[0].0, &textures[0].1)),
                Some((2, &textures[1].0, &textures[1].1)),
                Some((4, &textures[2].0, &textures[2].1)),
                Some((6, &textures[3].0, &textures[3].1)),
                Some((8, &textures[4].0, &textures[4].1)),
                Some((10, &textures[5].0, &textures[5].1)),
                None,
                None,
                None,
            ],
            Self::Flag {
                textures, detail, ..
            } => [
                Some((0, &textures[0].0, &textures[0].1)),
                Some((6, &textures[1].0, &textures[1].1)),
                Some((4, &textures[2].0, &textures[2].1)),
                detail
                    .as_ref()
                    .map(|d| (2, &d.normal_map, &d.normal_sampler)),
                detail
                    .as_ref()
                    .map(|d| (10, &d.detail_map, &d.detail_sampler)),
                None,
                None,
                None,
                None,
            ],
            Self::Canopy { base, .. } => [
                Some((0, &base.color_map, &base.color_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::MappedOcclusion {
                detail,
                base,
                specular_map,
                specular_sampler,
                occlusion_map,
                occlusion_sampler,
            } => [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((2, &base.normal_map, &base.normal_sampler)),
                Some((4, &base.reflection_probe, &base.reflection_sampler)),
                Some((6, specular_map, specular_sampler)),
                Some((14, occlusion_map, occlusion_sampler)),
                detail
                    .as_ref()
                    .map(|detail| (10, &detail.map, &detail.sampler)),
                None,
                None,
                None,
            ],
            Self::Feather3Layer { layers, .. } => [
                Some((0, &layers[0].0, &layers[0].1)),
                Some((2, &layers[1].0, &layers[1].1)),
                Some((4, &layers[2].0, &layers[2].1)),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::Unlit3Layer {
                layers,
                probe,
                probe_sampler,
                ..
            } => [
                Some((0, &layers[0].0, &layers[0].1)),
                Some((2, &layers[1].0, &layers[1].1)),
                Some((4, &layers[2].0, &layers[2].1)),
                Some((8, probe, probe_sampler)),
                None,
                None,
                None,
                None,
                None,
            ],
            Self::FourLayers {
                layers,
                probe,
                probe_sampler,
                ..
            } => [
                Some((0, &layers[0].0, &layers[0].1)),
                Some((2, &layers[1].0, &layers[1].1)),
                Some((4, &layers[2].0, &layers[2].1)),
                Some((6, &layers[3].0, &layers[3].1)),
                Some((8, probe, probe_sampler)),
                None,
                None,
                None,
                None,
            ],
            Self::DetailDiffuse {
                base,
                detail_map,
                detail_sampler,
                ..
            } => [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((2, &base.normal_map, &base.normal_sampler)),
                Some((10, detail_map, detail_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::Unlit { base, .. } => [
                Some((0, &base.color_map, &base.color_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::TileBlend {
                textures, specular, ..
            } => [
                Some((0, &textures[0].0, &textures[0].1)),
                Some((2, &textures[1].0, &textures[1].1)),
                Some((4, &textures[2].0, &textures[2].1)),
                Some((6, &textures[3].0, &textures[3].1)),
                Some((8, &textures[4].0, &textures[4].1)),
                Some((10, &textures[5].0, &textures[5].1)),
                specular
                    .as_ref()
                    .map(|v| (12, &v.layers[0].0, &v.layers[0].1)),
                specular
                    .as_ref()
                    .map(|v| (14, &v.layers[1].0, &v.layers[1].1)),
                specular.as_ref().map(|v| (16, &v.probe, &v.probe_sampler)),
            ],
            Self::ImplicitSpecular(base)
            | Self::VertexLitSpecular(base)
            | Self::VertexOcclusion(base, _) => [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((2, &base.normal_map, &base.normal_sampler)),
                Some((4, &base.reflection_probe, &base.reflection_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::RevealDiffuse { base, .. }
            | Self::Diffuse(base, _)
            | Self::BlendedDiffuse(base, _)
            | Self::AlphaDiffuse(base) => [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((2, &base.normal_map, &base.normal_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::GeometricDiffuse { base, .. } => [
                Some((0, &base.color_map, &base.color_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::GeometricSpecular {
                base,
                specular_map,
                specular_sampler,
                reflection_probe,
                reflection_sampler,
                ..
            } => [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((4, reflection_probe, reflection_sampler)),
                Some((6, specular_map, specular_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::NormalSpecular {
                base,
                specular_map,
                specular_sampler,
                ..
            }
            | Self::AlphaSpecular {
                base,
                specular_map,
                specular_sampler,
                ..
            } => [
                Some((0, &base.color_map, &base.color_sampler)),
                Some((2, &base.normal_map, &base.normal_sampler)),
                Some((4, &base.reflection_probe, &base.reflection_sampler)),
                Some((6, specular_map, specular_sampler)),
                None,
                None,
                None,
                None,
                None,
            ],
            Self::Crystal3Layer { textures, .. } => [
                Some((0, &textures.normal_map, &textures.normal_sampler)),
                Some((2, &textures.layer1_map, &textures.layer1_sampler)),
                Some((4, &textures.layer2_map, &textures.layer2_sampler)),
                Some((6, &textures.layer3_map, &textures.layer3_sampler)),
                Some((8, &textures.reflection_probe, &textures.reflection_sampler)),
                None,
                None,
                None,
                None,
            ],
            Self::Camo3Layer {
                textures,
                weapon_normal_map,
                weapon_normal_sampler,
                ..
            } => [
                Some((0, &textures.normal_map, &textures.normal_sampler)),
                Some((2, &textures.layer1_map, &textures.layer1_sampler)),
                Some((4, &textures.layer2_map, &textures.layer2_sampler)),
                Some((6, &textures.layer3_map, &textures.layer3_sampler)),
                Some((8, &textures.reflection_probe, &textures.reflection_sampler)),
                Some((10, weapon_normal_map, weapon_normal_sampler)),
                None,
                None,
                None,
            ],
            Self::PhongEmissive {
                base,
                specular_map,
                specular_sampler,
                glow_map,
                glow_sampler,
                ..
            } => [
                Some((0, &base.normal_map, &base.normal_sampler)),
                Some((2, &base.color_map, &base.color_sampler)),
                Some((4, specular_map, specular_sampler)),
                Some((6, glow_map, glow_sampler)),
                Some((8, &base.reflection_probe, &base.reflection_sampler)),
                None,
                None,
                None,
                None,
            ],
            Self::CharCloth {
                base,
                specular_map,
                specular_sampler,
            } => [
                Some((0, &base.normal_map, &base.normal_sampler)),
                Some((2, &base.color_map, &base.color_sampler)),
                Some((4, specular_map, specular_sampler)),
                Some((8, &base.reflection_probe, &base.reflection_sampler)),
                None,
                None,
                None,
                None,
                None,
            ],
            Self::ZombieEyes(base) => [
                Some((0, &base.color_map, &base.color_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
        };
        bindings.into_iter().flatten()
    }

    fn family(&self) -> NativeModelFamily {
        match self {
            Self::SmokePillar { .. } => NativeModelFamily::SmokePillar,
            Self::Flag { detail, .. } => NativeModelFamily::Flag(detail.is_some()),
            Self::Canopy { quasi, .. } => NativeModelFamily::Canopy(*quasi),
            Self::RevealDiffuse { .. } => NativeModelFamily::RevealDiffuse,
            Self::MappedOcclusion { detail, .. } => {
                NativeModelFamily::MappedOcclusion(detail.is_some())
            }
            Self::Feather3Layer { .. } => NativeModelFamily::Feather3Layer,
            Self::Unlit3Layer {
                vertex_lit,
                program,
                ..
            } => NativeModelFamily::Unlit3Layer {
                vertex_lit: *vertex_lit,
                program: *program,
            },
            Self::FourLayers {
                vertex_lit, fog, ..
            } => NativeModelFamily::FourLayers {
                vertex_lit: *vertex_lit,
                fog: *fog,
            },
            Self::DetailDiffuse { vertex_lit, .. } => NativeModelFamily::DetailDiffuse(*vertex_lit),
            Self::Unlit { scale_rgb, .. } => NativeModelFamily::Unlit {
                replace: scale_rgb.is_some(),
            },
            Self::TileBlend {
                specular,
                vertex_lit,
                ..
            } => NativeModelFamily::TileBlend {
                specular: specular.is_some(),
                vertex_lit: *vertex_lit,
            },
            Self::ImplicitSpecular(_) => NativeModelFamily::ImplicitSpecular,
            Self::VertexLitSpecular(_) => NativeModelFamily::VertexLitSpecular,
            Self::VertexOcclusion(_, vertex_lit) => NativeModelFamily::VertexOcclusion(*vertex_lit),
            Self::Diffuse(_, vertex_lit) => NativeModelFamily::Diffuse(*vertex_lit),
            Self::BlendedDiffuse(_, vertex_lit) => NativeModelFamily::BlendedDiffuse(*vertex_lit),
            Self::AlphaDiffuse(_) => NativeModelFamily::AlphaDiffuse,
            Self::GeometricDiffuse { program, .. } => NativeModelFamily::GeometricDiffuse(*program),
            Self::GeometricSpecular {
                vertex_lit,
                transparent,
                ..
            } => NativeModelFamily::GeometricSpecular {
                vertex_lit: *vertex_lit,
                transparent: *transparent,
            },
            Self::NormalSpecular { program, .. } => NativeModelFamily::NormalSpecular(*program),
            Self::AlphaSpecular { vertex_lit, .. } => NativeModelFamily::AlphaSpecular(*vertex_lit),
            Self::Crystal3Layer { vertex_lit, .. } => NativeModelFamily::Crystal3Layer(*vertex_lit),
            Self::Camo3Layer { .. } => NativeModelFamily::Camo3Layer,
            Self::PhongEmissive {
                program,
                vertex_lit,
                ..
            } => NativeModelFamily::PhongEmissive {
                program: *program,
                vertex_lit: *vertex_lit,
            },
            Self::CharCloth { .. } => NativeModelFamily::CharCloth,
            Self::ZombieEyes(_) => NativeModelFamily::ZombieEyes,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum NativeModelFamily {
    SmokePillar,
    Flag(bool),
    Canopy(bool),
    RevealDiffuse,
    MappedOcclusion(bool),
    Feather3Layer,
    Unlit3Layer {
        vertex_lit: bool,
        program: Unlit3LayerProgram,
    },
    FourLayers {
        vertex_lit: bool,
        fog: bool,
    },
    DetailDiffuse(bool),
    Unlit {
        replace: bool,
    },
    TileBlend {
        specular: bool,
        vertex_lit: bool,
    },
    GeometricSpecular {
        vertex_lit: bool,
        transparent: bool,
    },
    ImplicitSpecular,
    VertexLitSpecular,
    VertexOcclusion(bool),
    Diffuse(bool),
    BlendedDiffuse(bool),
    AlphaDiffuse,
    GeometricDiffuse(GeometricProgram),
    NormalSpecular(NormalSpecularProgram),
    AlphaSpecular(bool),
    Crystal3Layer(bool),
    Camo3Layer,
    PhongEmissive {
        program: EmissiveProgram,
        vertex_lit: bool,
    },
    CharCloth,
    ZombieEyes,
}

impl NativeModelFamily {
    fn geometric(self) -> bool {
        matches!(
            self,
            Self::GeometricDiffuse(_)
                | Self::GeometricSpecular { .. }
                | Self::Unlit { .. }
                | Self::Canopy(_)
                | Self::SmokePillar
        )
    }

    fn vertex_lit(self) -> bool {
        matches!(
            self,
            Self::GeometricSpecular {
                vertex_lit: true,
                ..
            } | Self::VertexLitSpecular
                | Self::GeometricDiffuse(GeometricProgram::VertexLit)
                | Self::PhongEmissive {
                    vertex_lit: true,
                    ..
                }
                | Self::TileBlend {
                    vertex_lit: true,
                    ..
                }
                | Self::Crystal3Layer(true)
                | Self::Unlit3Layer {
                    vertex_lit: true,
                    ..
                }
                | Self::VertexOcclusion(true)
                | Self::GeometricDiffuse(GeometricProgram::VertexOcclusion(true))
                | Self::DetailDiffuse(true)
                | Self::FourLayers {
                    vertex_lit: true,
                    ..
                }
                | Self::BlendedDiffuse(true)
                | Self::GeometricDiffuse(GeometricProgram::Blended(true))
                | Self::Diffuse(true)
                | Self::NormalSpecular(NormalSpecularProgram::VertexLit)
                | Self::AlphaSpecular(true)
        )
    }

    fn scripted_vertex(self) -> bool {
        matches!(
            self,
            Self::Crystal3Layer(_)
                | Self::Unlit3Layer { .. }
                | Self::Feather3Layer
                | Self::FourLayers { .. }
                | Self::Camo3Layer
                | Self::CharCloth
                | Self::PhongEmissive { .. }
                | Self::TileBlend { .. }
        )
    }

    fn script_inputs(self) -> bool {
        matches!(
            self,
            Self::Crystal3Layer(_)
                | Self::Unlit3Layer { .. }
                | Self::Feather3Layer
                | Self::FourLayers { .. }
                | Self::PhongEmissive {
                    program: EmissiveProgram::ScriptedGlow | EmissiveProgram::ScriptMulti,
                    ..
                }
                | Self::CharCloth
                | Self::ZombieEyes
        )
    }

    fn depth(self) -> bool {
        matches!(self, Self::Feather3Layer | Self::SmokePillar)
    }

    fn atlas(self) -> bool {
        !matches!(
            self,
            Self::ZombieEyes | Self::Unlit { .. } | Self::Feather3Layer | Self::SmokePillar
        )
    }

    fn atlas_texture(self) -> bool {
        !matches!(
            self,
            Self::ZombieEyes
                | Self::Canopy(_)
                | Self::SmokePillar
                | Self::Crystal3Layer(_)
                | Self::Unlit3Layer { .. }
                | Self::Feather3Layer
                | Self::Camo3Layer
                | Self::Unlit { .. }
                | Self::FourLayers { .. }
        )
    }
}

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeModelCanopyConstants {
    pub lighting: Vec4,
    pub tint: Vec4,
}

#[derive(ShaderType)]
struct NativeUnlitScale {
    scale_rgb: Vec4,
}

#[derive(ShaderType)]
struct NativeModelDetailScale {
    detail_scale: Vec4,
}

#[derive(ShaderType)]
struct NativeModelReveal {
    reveal: Vec4,
}

#[derive(ShaderType)]
struct NativeModelOcclusion {
    occlusion_amount: Vec4,
}

/// Static source streams or one animated instance's compact skinned LOD.
#[derive(Clone)]
pub(crate) enum ModelGeometry {
    Static(Handle<NativeWorldGeometry>),
    Animated {
        model: Handle<AnimatedModel>,
        instance: Entity,
        lod: usize,
        surface: usize,
    },
}

#[derive(Clone, Copy)]
enum ExtractedModelGeometry {
    Static(AssetId<NativeWorldGeometry>),
    Animated {
        model: AssetId<AnimatedModel>,
        instance: Entity,
        lod: usize,
        surface: usize,
    },
}

impl ExtractedModelGeometry {
    fn asset_id(self) -> bevy::asset::UntypedAssetId {
        match self {
            Self::Static(asset) => asset.untyped(),
            Self::Animated { model, .. } => model.untyped(),
        }
    }

    fn instance(self) -> Option<Entity> {
        match self {
            Self::Static(_) => None,
            Self::Animated { instance, .. } => Some(instance),
        }
    }

    fn buffers<'a>(
        self,
        static_models: &'a RenderAssets<PreparedNativeGeometry>,
        animated_models: &'a RenderAssets<PreparedAnimatedModel>,
        skin: Option<&'a PreparedSkin>,
        static_offset: u64,
    ) -> Option<(&'a Buffer, &'a Buffer, u64, bool)> {
        match self {
            Self::Static(id) => {
                let model = static_models.get(id)?;
                Some((&model.primary, &model.indices, static_offset, true))
            }
            Self::Animated {
                model,
                lod,
                surface,
                ..
            } => {
                let model = animated_models.get(model)?;
                let skin = skin?;
                if !skin.matches(model, lod) {
                    return None;
                }
                Some((
                    &skin.vertices,
                    &model.indices,
                    *model.lods.get(lod)?.surface_offsets.get(surface)?,
                    skin.ready,
                ))
            }
        }
    }
}

#[derive(Component, Clone)]
#[require(VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<NativeModelSurface>)]
pub(crate) struct NativeModelSurface {
    pub pass: NativeWorldPass,
    pub sort_key: (u64, u32),
    pub vertex_lighting: Option<(Handle<ModelVertexLighting>, u64)>,
    pub geometry: ModelGeometry,
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
    pass: NativeWorldPass,
    sort_key: (u64, u32),
    vertex_lighting: Option<(AssetId<ModelVertexLighting>, u64)>,
    geometry: ExtractedModelGeometry,
    material: AssetId<NativeModelMaterial>,
    state: NativeWorldState,
    primary_byte_offset: u64,
    first_index: u32,
    index_count: u32,
    placement: MainEntity,
    dynamic_lighting_origin: Option<Vec3>,
    colors_index: usize,
    visibility: u8,
}

impl SyncComponent<RenderApp> for NativeModelSurface {
    type Target = ExtractedNativeModelSurface;
}

fn extract_model_surfaces(
    mut commands: Commands,
    surfaces: Extract<Query<(RenderEntity, &NativeModelSurface)>>,
    entities: Extract<Query<(RenderEntity, &DynamicLightingOrigin)>>,
    mut previous_len: Local<usize>,
) {
    let mut extracted = Vec::with_capacity(*previous_len);
    for (entity, surface) in &surfaces {
        let mut dynamic_lighting_origin = None;
        let geometry = match &surface.geometry {
            ModelGeometry::Static(handle) => ExtractedModelGeometry::Static(handle.id()),
            ModelGeometry::Animated {
                model,
                instance,
                lod,
                surface,
            } => {
                let Ok((instance, origin)) = entities.get(*instance) else {
                    commands
                        .entity(entity)
                        .remove::<ExtractedNativeModelSurface>();
                    continue;
                };
                dynamic_lighting_origin = Some(origin.0);
                ExtractedModelGeometry::Animated {
                    model: model.id(),
                    instance: instance.into(),
                    lod: *lod,
                    surface: *surface,
                }
            }
        };
        extracted.push((
            entity,
            ExtractedNativeModelSurface {
                pass: surface.pass,
                sort_key: surface.sort_key,
                vertex_lighting: surface
                    .vertex_lighting
                    .as_ref()
                    .map(|(handle, offset)| (handle.id(), *offset)),
                geometry,
                material: surface.material.id(),
                state: surface.state,
                primary_byte_offset: surface.primary_byte_offset,
                first_index: surface.first_index,
                index_count: surface.index_count,
                placement: surface.placement.into(),
                dynamic_lighting_origin,
                colors_index: surface.colors_index,
                visibility: surface.visibility,
            },
        ));
    }
    *previous_len = extracted.len();
    commands.try_insert_batch(extracted);
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
        app.init_asset::<ModelVertexLighting>()
            .init_asset::<NativeModelMaterial>()
            .add_plugins((
                RenderAssetPlugin::<PreparedModelVertexLighting>::default(),
                SyncComponentPlugin::<NativeModelSurface>::default(),
                RenderAssetPlugin::<PreparedModelMaterial, GpuImage>::default(),
            ));
        load_shader_library!(app, "native_model.wesl");
        load_shader_library!(app, "native_model_variants.wesl");
        load_shader_library!(app, "native_scripted_model.wesl");
        load_shader_library!(app, "native_camo.wesl");
        embedded_asset!(app, "native_model_entry.wesl");
        embedded_asset!(app, "native_skinning.wesl");
        embedded_asset!(app, "native_scripted_model_entry.wesl");
        let shader = load_embedded_asset!(app, "native_model_entry.wesl");
        let scripted_shader = load_embedded_asset!(app, "native_scripted_model_entry.wesl");
        embedded_asset!(app, "native_tile_model_entry.wesl");
        let tile_shader = load_embedded_asset!(app, "native_tile_model_entry.wesl");
        embedded_asset!(app, "native_unlit_model_entry.wesl");
        let unlit_shader = load_embedded_asset!(app, "native_unlit_model_entry.wesl");
        embedded_asset!(app, "native_plinth_model_entry.wesl");
        let plinth_shader = load_embedded_asset!(app, "native_plinth_model_entry.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(ModelPipeline::new(
                shader,
                scripted_shader,
                tile_shader,
                unlit_shader,
                plinth_shader,
            ))
            .init_resource::<SpecializedRenderPipelines<ModelPipeline>>()
            .init_resource::<ModelUniforms>()
            .init_resource::<ModelBindGroups>()
            .init_resource::<ModelScriptOwners>()
            .add_systems(
                ExtractSchedule,
                (extract_script_owners, extract_model_surfaces),
            )
            .add_render_command::<Opaque3d, (SetItemPipeline, DrawNativeModel)>()
            .add_systems(
                Render,
                (
                    queue_models
                        .after(super::native::queue_native_world)
                        .in_set(RenderSystems::Queue),
                    prepare_model_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Resource)]
struct ModelPipeline {
    shader: Handle<Shader>,
    scripted_shader: Handle<Shader>,
    tile_shader: Handle<Shader>,
    unlit_shader: Handle<Shader>,
    plinth_shader: Handle<Shader>,
    unlit_replace_layout: BindGroupLayoutDescriptor,
    tile_layouts: [BindGroupLayoutDescriptor; 2],
    scene_layout: BindGroupLayoutDescriptor,
    scripted_scene_layout: BindGroupLayoutDescriptor,
    object_layout: BindGroupLayoutDescriptor,
    scripted_object_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    diffuse_material_layout: BindGroupLayoutDescriptor,
    detail_material_layout: BindGroupLayoutDescriptor,
    geometric_material_layout: BindGroupLayoutDescriptor,
    geometric_specular_material_layout: BindGroupLayoutDescriptor,
    specular_material_layout: BindGroupLayoutDescriptor,
    mapped_occlusion_layouts: [BindGroupLayoutDescriptor; 2],
    reveal_layout: BindGroupLayoutDescriptor,
    canopy_layout: BindGroupLayoutDescriptor,
    flag_layouts: [BindGroupLayoutDescriptor; 2],
    crystal_material_layout: BindGroupLayoutDescriptor,
    falloff_material_layout: BindGroupLayoutDescriptor,
    feather_material_layout: BindGroupLayoutDescriptor,
    smoke_material_layout: BindGroupLayoutDescriptor,
    four_layer_layout: BindGroupLayoutDescriptor,
    camo_material_layout: BindGroupLayoutDescriptor,
    disc_material_layout: BindGroupLayoutDescriptor,
    cloth_material_layout: BindGroupLayoutDescriptor,
    lighting_layout: BindGroupLayoutDescriptor,
}

impl ModelPipeline {
    fn new(
        shader: Handle<Shader>,
        scripted_shader: Handle<Shader>,
        tile_shader: Handle<Shader>,
        unlit_shader: Handle<Shader>,
        plinth_shader: Handle<Shader>,
    ) -> Self {
        let textures = BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_cube(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        );
        let mapped_occlusion_entries = BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (0, texture_2d(TextureSampleType::Float { filterable: true })),
                (1, sampler(SamplerBindingType::Filtering)),
                (2, texture_2d(TextureSampleType::Float { filterable: true })),
                (3, sampler(SamplerBindingType::Filtering)),
                (
                    4,
                    texture_cube(TextureSampleType::Float { filterable: true }),
                ),
                (5, sampler(SamplerBindingType::Filtering)),
                (6, texture_2d(TextureSampleType::Float { filterable: true })),
                (7, sampler(SamplerBindingType::Filtering)),
                (
                    14,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (15, sampler(SamplerBindingType::Filtering)),
            ),
        );
        let reveal_entries = BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (0, texture_2d(TextureSampleType::Float { filterable: true })),
                (1, sampler(SamplerBindingType::Filtering)),
                (2, texture_2d(TextureSampleType::Float { filterable: true })),
                (3, sampler(SamplerBindingType::Filtering)),
                (13, uniform_buffer::<NativeModelReveal>(false)),
            ),
        );
        let flag_layouts = [false, true].map(|detail| {
            let mut entries = BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, texture_2d(TextureSampleType::Float { filterable: true })),
                    (1, sampler(SamplerBindingType::Filtering)),
                    (
                        4,
                        texture_cube(TextureSampleType::Float { filterable: true }),
                    ),
                    (5, sampler(SamplerBindingType::Filtering)),
                    (6, texture_2d(TextureSampleType::Float { filterable: true })),
                    (7, sampler(SamplerBindingType::Filtering)),
                    (8, uniform_buffer::<NativeModelOcclusion>(false)),
                ),
            )
            .to_vec();
            entries.push(
                uniform_buffer::<NativeModelFlagParams>(false).build(17, ShaderStages::VERTEX),
            );
            if detail {
                entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
                    ShaderStages::FRAGMENT,
                    (
                        (2, texture_2d(TextureSampleType::Float { filterable: true })),
                        (3, sampler(SamplerBindingType::Filtering)),
                        (
                            10,
                            texture_2d(TextureSampleType::Float { filterable: true }),
                        ),
                        (11, sampler(SamplerBindingType::Filtering)),
                        (12, uniform_buffer::<NativeModelDetailScale>(false)),
                    ),
                ));
            }
            BindGroupLayoutDescriptor::new("native moving cloth", &entries)
        });
        let mut smoke_entries = Vec::with_capacity(13);
        for binding in (0..12).step_by(2) {
            smoke_entries.push(
                texture_2d(TextureSampleType::Float { filterable: true })
                    .build(binding, ShaderStages::FRAGMENT),
            );
            smoke_entries.push(
                sampler(SamplerBindingType::Filtering).build(binding + 1, ShaderStages::FRAGMENT),
            );
        }
        smoke_entries.push(
            uniform_buffer::<NativeModelSmokeConstants>(false).build(13, ShaderStages::FRAGMENT),
        );
        let mut four_entries = textures.to_vec();
        four_entries.push(
            uniform_buffer::<NativeModelFourLayerConstants>(false)
                .build(15, ShaderStages::VERTEX_FRAGMENT),
        );
        let mut crystal_entries = textures.to_vec();
        crystal_entries.push(
            uniform_buffer::<NativeModelParallaxConstants>(false).build(12, ShaderStages::FRAGMENT),
        );
        let falloff_entries: Vec<_> = crystal_entries
            .iter()
            .copied()
            .filter(|entry| entry.binding != 6 && entry.binding != 7)
            .collect();
        let feather_entries: Vec<_> = falloff_entries
            .iter()
            .copied()
            .filter(|entry| entry.binding != 8 && entry.binding != 9)
            .collect();
        let mut camo_entries = crystal_entries.clone();
        camo_entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (
                    10,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (11, sampler(SamplerBindingType::Filtering)),
                (14, uniform_buffer::<NativeModelCamoParameters>(false)),
            ),
        ));
        let cloth_entries: Vec<_> = textures
            .iter()
            .copied()
            .filter(|entry| entry.binding != 6 && entry.binding != 7)
            .collect();
        let mut disc_entries = textures.to_vec();
        disc_entries.push(
            uniform_buffer::<NativeModelEmissiveConstants>(false).build(13, ShaderStages::FRAGMENT),
        );
        let tile_layouts = [false, true].map(|specular| {
            let mut entries = Vec::new();
            for index in 0..if specular { 8 } else { 6 } {
                entries.push(
                    texture_2d(TextureSampleType::Float { filterable: true })
                        .build(index * 2, ShaderStages::FRAGMENT),
                );
                entries.push(
                    sampler(SamplerBindingType::Filtering)
                        .build(index * 2 + 1, ShaderStages::FRAGMENT),
                );
            }
            if specular {
                entries.push(
                    texture_cube(TextureSampleType::Float { filterable: true })
                        .build(16, ShaderStages::FRAGMENT),
                );
                entries
                    .push(sampler(SamplerBindingType::Filtering).build(17, ShaderStages::FRAGMENT));
            }
            entries.push(
                uniform_buffer::<NativeModelTileConstants>(false)
                    .build(18, ShaderStages::VERTEX_FRAGMENT),
            );
            BindGroupLayoutDescriptor::new("original model tile blend", &entries)
        });
        Self {
            shader,
            scripted_shader,
            tile_shader,
            unlit_shader,
            plinth_shader,
            unlit_replace_layout: BindGroupLayoutDescriptor::new(
                "native unlit model color and scale",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<NativeUnlitScale>(false),
                    ),
                ),
            ),
            tile_layouts,
            scene_layout: BindGroupLayoutDescriptor::new(
                "native model scene",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX_FRAGMENT,
                    uniform_buffer::<NativeModelScene>(true),
                ),
            ),
            scripted_scene_layout: BindGroupLayoutDescriptor::new(
                "native scripted model scene",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        uniform_buffer::<NativeModelScene>(true),
                        uniform_buffer::<NativeModelScriptedScene>(true),
                    ),
                ),
            ),
            object_layout: BindGroupLayoutDescriptor::new(
                "native model object",
                &BindGroupLayoutEntries::single(
                    ShaderStages::VERTEX,
                    uniform_buffer::<NativeModelObject>(true),
                ),
            ),
            scripted_object_layout: BindGroupLayoutDescriptor::new(
                "native model object with source script vectors",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        (0, uniform_buffer::<NativeModelObject>(true)),
                        (1, uniform_buffer::<NativeModelScriptInputs>(true)),
                    ),
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
            detail_material_layout: BindGroupLayoutDescriptor::new(
                "native model detail normal",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::FRAGMENT,
                    (
                        (0, texture_2d(TextureSampleType::Float { filterable: true })),
                        (1, sampler(SamplerBindingType::Filtering)),
                        (2, texture_2d(TextureSampleType::Float { filterable: true })),
                        (3, sampler(SamplerBindingType::Filtering)),
                        (
                            10,
                            texture_2d(TextureSampleType::Float { filterable: true }),
                        ),
                        (11, sampler(SamplerBindingType::Filtering)),
                        (12, uniform_buffer::<NativeModelDetailScale>(false)),
                    ),
                ),
            ),
            diffuse_material_layout: BindGroupLayoutDescriptor::new(
                "native model diffuse textures",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
            geometric_material_layout: BindGroupLayoutDescriptor::new(
                "native model geometric diffuse texture",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
            geometric_specular_material_layout: BindGroupLayoutDescriptor::new(
                "native geometric specular textures and occlusion",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::FRAGMENT,
                    (
                        (0, texture_2d(TextureSampleType::Float { filterable: true })),
                        (1, sampler(SamplerBindingType::Filtering)),
                        (
                            4,
                            texture_cube(TextureSampleType::Float { filterable: true }),
                        ),
                        (5, sampler(SamplerBindingType::Filtering)),
                        (6, texture_2d(TextureSampleType::Float { filterable: true })),
                        (7, sampler(SamplerBindingType::Filtering)),
                        (8, uniform_buffer::<NativeModelOcclusion>(false)),
                    ),
                ),
            ),
            mapped_occlusion_layouts: [false, true].map(|detail| {
                let mut entries = mapped_occlusion_entries.to_vec();
                if detail {
                    entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
                        ShaderStages::FRAGMENT,
                        (
                            (
                                10,
                                texture_2d(TextureSampleType::Float { filterable: true }),
                            ),
                            (11, sampler(SamplerBindingType::Filtering)),
                            (18, uniform_buffer::<Vec4>(false)),
                        ),
                    ));
                }
                BindGroupLayoutDescriptor::new("native model occlusion/detail inputs", &entries)
            }),
            flag_layouts,
            canopy_layout: BindGroupLayoutDescriptor::new(
                "native canopy",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        (0, texture_2d(TextureSampleType::Float { filterable: true })),
                        (1, sampler(SamplerBindingType::Filtering)),
                        (16, uniform_buffer::<NativeModelCanopyConstants>(false)),
                    ),
                ),
            ),
            reveal_layout: BindGroupLayoutDescriptor::new("model alpha reveal", &reveal_entries),
            specular_material_layout: BindGroupLayoutDescriptor::new(
                "native model explicit specular textures and occlusion",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_cube(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<NativeModelOcclusion>(false),
                    ),
                ),
            ),
            four_layer_layout: BindGroupLayoutDescriptor::new(
                "native four-layer model",
                &four_entries,
            ),
            smoke_material_layout: BindGroupLayoutDescriptor::new(
                "original smoke pillar inputs",
                &smoke_entries,
            ),
            feather_material_layout: BindGroupLayoutDescriptor::new(
                "native plinth layers",
                &feather_entries,
            ),
            falloff_material_layout: BindGroupLayoutDescriptor::new(
                "native three-layer falloff",
                &falloff_entries,
            ),
            crystal_material_layout: BindGroupLayoutDescriptor::new(
                "native crystal original three layers",
                &crystal_entries,
            ),
            camo_material_layout: BindGroupLayoutDescriptor::new(
                "native selected weapon camo original three layers",
                &camo_entries,
            ),
            cloth_material_layout: BindGroupLayoutDescriptor::new(
                "native character cloth inputs",
                &cloth_entries,
            ),
            disc_material_layout: BindGroupLayoutDescriptor::new(
                "native crypt disc emissive tomb inputs",
                &disc_entries,
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

    fn material_layout(&self, family: NativeModelFamily) -> &BindGroupLayoutDescriptor {
        match family {
            NativeModelFamily::Flag(detail) => &self.flag_layouts[usize::from(detail)],
            NativeModelFamily::Canopy(_) => &self.canopy_layout,
            NativeModelFamily::RevealDiffuse => &self.reveal_layout,
            NativeModelFamily::MappedOcclusion(detail) => {
                &self.mapped_occlusion_layouts[usize::from(detail)]
            }
            NativeModelFamily::FourLayers { .. } => &self.four_layer_layout,
            NativeModelFamily::DetailDiffuse(_) => &self.detail_material_layout,
            NativeModelFamily::Unlit { replace: true } => &self.unlit_replace_layout,
            NativeModelFamily::Unlit { replace: false } => &self.geometric_material_layout,
            NativeModelFamily::TileBlend { specular, .. } => {
                &self.tile_layouts[usize::from(specular)]
            }
            NativeModelFamily::ImplicitSpecular
            | NativeModelFamily::VertexLitSpecular
            | NativeModelFamily::VertexOcclusion(_) => &self.material_layout,
            NativeModelFamily::Diffuse(_)
            | NativeModelFamily::BlendedDiffuse(_)
            | NativeModelFamily::AlphaDiffuse => &self.diffuse_material_layout,
            NativeModelFamily::GeometricSpecular { .. } => &self.geometric_specular_material_layout,
            NativeModelFamily::GeometricDiffuse(_) => &self.geometric_material_layout,
            NativeModelFamily::NormalSpecular(_) | NativeModelFamily::AlphaSpecular(_) => {
                &self.specular_material_layout
            }
            NativeModelFamily::SmokePillar => &self.smoke_material_layout,
            NativeModelFamily::Feather3Layer => &self.feather_material_layout,
            NativeModelFamily::Unlit3Layer { .. } => &self.falloff_material_layout,
            NativeModelFamily::Crystal3Layer(_) => &self.crystal_material_layout,
            NativeModelFamily::Camo3Layer => &self.camo_material_layout,
            NativeModelFamily::PhongEmissive { .. } => &self.disc_material_layout,
            NativeModelFamily::CharCloth => &self.cloth_material_layout,
            NativeModelFamily::ZombieEyes => &self.geometric_material_layout,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct ModelPipelineKey {
    target: TextureFormat,
    samples: u32,
    state: NativeWorldState,
    family: NativeModelFamily,
    depth_filtering: bool,
}

impl SpecializedRenderPipeline for ModelPipeline {
    type Key = ModelPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut attributes = vec![
            VertexAttribute {
                format: if key.family.geometric() || key.family == NativeModelFamily::ZombieEyes {
                    VertexFormat::Float32x3
                } else {
                    VertexFormat::Float32x4
                },
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
        ];
        if matches!(
            key.family,
            NativeModelFamily::Unlit { .. } | NativeModelFamily::SmokePillar
        ) {
            attributes.retain(|attribute| attribute.shader_location <= 2);
        } else if key.family == NativeModelFamily::ZombieEyes {
            attributes.retain(|attribute| matches!(attribute.shader_location, 0 | 2));
        } else if !key.family.geometric() {
            attributes.push(VertexAttribute {
                format: VertexFormat::Unorm10_10_10_2,
                offset: 28,
                shader_location: 4,
            });
        }
        let fragment = match (key.family, key.target.is_srgb()) {
            (NativeModelFamily::SmokePillar, false) => "fragment_smoke",
            (NativeModelFamily::SmokePillar, true) => "fragment_smoke_srgb",
            (NativeModelFamily::Flag(true), false) => "fragment_flag_detail",
            (NativeModelFamily::Flag(true), true) => "fragment_flag_detail_srgb",
            (NativeModelFamily::Flag(false), false) => "fragment_flag_web",
            (NativeModelFamily::Flag(false), true) => "fragment_flag_web_srgb",
            (NativeModelFamily::Canopy(true), false) => "fragment_canopy_quasi",
            (NativeModelFamily::Canopy(true), true) => "fragment_canopy_quasi_srgb",
            (NativeModelFamily::Canopy(false), false) => "fragment_canopy",
            (NativeModelFamily::Canopy(false), true) => "fragment_canopy_srgb",
            (NativeModelFamily::RevealDiffuse, false) => "fragment_reveal",
            (NativeModelFamily::RevealDiffuse, true) => "fragment_reveal_srgb",
            (NativeModelFamily::MappedOcclusion(true), false) => "fragment_weapon_detail",
            (NativeModelFamily::MappedOcclusion(true), true) => "fragment_weapon_detail_srgb",
            (NativeModelFamily::MappedOcclusion(false), false) => "fragment_mapped_occlusion",
            (NativeModelFamily::MappedOcclusion(false), true) => "fragment_mapped_occlusion_srgb",
            (NativeModelFamily::Feather3Layer, false) => "fragment",
            (NativeModelFamily::Feather3Layer, true) => "fragment_srgb",
            (
                NativeModelFamily::GeometricSpecular {
                    transparent: false, ..
                },
                false,
            ) => "fragment_geometric_specular",
            (
                NativeModelFamily::GeometricSpecular {
                    transparent: false, ..
                },
                true,
            ) => "fragment_geometric_specular_srgb",
            (
                NativeModelFamily::GeometricSpecular {
                    transparent: true, ..
                },
                false,
            ) => "fragment_geometric_specular_blended",
            (
                NativeModelFamily::GeometricSpecular {
                    transparent: true, ..
                },
                true,
            ) => "fragment_geometric_specular_blended_srgb",

            (NativeModelFamily::ImplicitSpecular | NativeModelFamily::VertexLitSpecular, false) => {
                "fragment"
            }
            (NativeModelFamily::ImplicitSpecular | NativeModelFamily::VertexLitSpecular, true) => {
                "fragment_srgb"
            }
            (NativeModelFamily::VertexOcclusion(_), false) => "fragment_vertex_occlusion",
            (NativeModelFamily::VertexOcclusion(_), true) => "fragment_vertex_occlusion_srgb",
            (NativeModelFamily::BlendedDiffuse(_), false) => "fragment_blended_diffuse",
            (NativeModelFamily::BlendedDiffuse(_), true) => "fragment_blended_diffuse_srgb",
            (NativeModelFamily::GeometricDiffuse(GeometricProgram::Blended(_)), false) => {
                "fragment_blended_geometric"
            }
            (NativeModelFamily::GeometricDiffuse(GeometricProgram::Blended(_)), true) => {
                "fragment_blended_geometric_srgb"
            }
            (NativeModelFamily::Diffuse(_), false) => "fragment_diffuse",
            (NativeModelFamily::Diffuse(_), true) => "fragment_diffuse_srgb",
            (NativeModelFamily::AlphaDiffuse, false) => "fragment_alpha_diffuse",
            (NativeModelFamily::AlphaDiffuse, true) => "fragment_alpha_diffuse_srgb",
            (
                NativeModelFamily::GeometricDiffuse(
                    GeometricProgram::Diffuse | GeometricProgram::VertexLit,
                ),
                false,
            ) => "fragment_geometric_diffuse",
            (
                NativeModelFamily::GeometricDiffuse(
                    GeometricProgram::Diffuse | GeometricProgram::VertexLit,
                ),
                true,
            ) => "fragment_geometric_diffuse_srgb",
            (NativeModelFamily::GeometricDiffuse(GeometricProgram::AlphaTest), false) => {
                "fragment_geometric_alpha"
            }
            (NativeModelFamily::GeometricDiffuse(GeometricProgram::AlphaTest), true) => {
                "fragment_geometric_alpha_srgb"
            }
            (NativeModelFamily::GeometricDiffuse(GeometricProgram::VertexOcclusion(_)), false) => {
                "fragment_geometric_occlusion"
            }
            (NativeModelFamily::GeometricDiffuse(GeometricProgram::VertexOcclusion(_)), true) => {
                "fragment_geometric_occlusion_srgb"
            }
            (NativeModelFamily::NormalSpecular(_), false) => "fragment_specular",
            (NativeModelFamily::NormalSpecular(_), true) => "fragment_specular_srgb",
            (NativeModelFamily::AlphaSpecular(_), false) => "fragment_alpha_specular",
            (NativeModelFamily::AlphaSpecular(_), true) => "fragment_alpha_specular_srgb",
            (
                NativeModelFamily::Unlit3Layer {
                    program: Unlit3LayerProgram::Opaque,
                    ..
                },
                false,
            ) => "fragment_falloff",
            (
                NativeModelFamily::Unlit3Layer {
                    program: Unlit3LayerProgram::Opaque,
                    ..
                },
                true,
            ) => "fragment_falloff_srgb",
            (
                NativeModelFamily::Unlit3Layer {
                    program: Unlit3LayerProgram::Coverage,
                    ..
                },
                false,
            ) => "fragment_falloff_coverage",
            (
                NativeModelFamily::Unlit3Layer {
                    program: Unlit3LayerProgram::Coverage,
                    ..
                },
                true,
            ) => "fragment_falloff_coverage_srgb",
            (
                NativeModelFamily::Unlit3Layer {
                    program: Unlit3LayerProgram::Alpha,
                    ..
                },
                false,
            ) => "fragment_falloff_alpha",
            (
                NativeModelFamily::Unlit3Layer {
                    program: Unlit3LayerProgram::Alpha,
                    ..
                },
                true,
            ) => "fragment_falloff_alpha_srgb",
            (
                NativeModelFamily::Unlit3Layer {
                    program: Unlit3LayerProgram::AlphaNoFalloff,
                    ..
                },
                false,
            ) => "fragment_three_alpha",
            (
                NativeModelFamily::Unlit3Layer {
                    program: Unlit3LayerProgram::AlphaNoFalloff,
                    ..
                },
                true,
            ) => "fragment_three_alpha_srgb",
            (NativeModelFamily::Crystal3Layer(_), false) => "fragment_crystal",
            (NativeModelFamily::Crystal3Layer(_), true) => "fragment_crystal_srgb",
            (NativeModelFamily::Camo3Layer, false) => "fragment_camo",
            (NativeModelFamily::Camo3Layer, true) => "fragment_camo_srgb",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::ScriptedGlow,
                    ..
                },
                false,
            ) => "fragment_disc",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::ScriptedGlow,
                    ..
                },
                true,
            ) => "fragment_disc_srgb",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::Alcatraz,
                    ..
                },
                false,
            ) => "fragment_emissive",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::Alcatraz,
                    ..
                },
                true,
            ) => "fragment_emissive_srgb",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::Transparent,
                    ..
                },
                false,
            ) => "fragment_emissive_transparent",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::Transparent,
                    ..
                },
                true,
            ) => "fragment_emissive_transparent_srgb",
            (
                NativeModelFamily::TileBlend {
                    specular: false, ..
                },
                false,
            ) => "fragment_diffuse",
            (
                NativeModelFamily::TileBlend {
                    specular: false, ..
                },
                true,
            ) => "fragment_diffuse_srgb",
            (NativeModelFamily::TileBlend { specular: true, .. }, false) => "fragment_specular",
            (NativeModelFamily::TileBlend { specular: true, .. }, true) => "fragment_specular_srgb",
            (NativeModelFamily::Unlit { replace: true }, false) => "fragment_replace",
            (NativeModelFamily::Unlit { replace: true }, true) => "fragment_replace_srgb",
            (NativeModelFamily::Unlit { replace: false }, false) => "fragment_multiply",
            (NativeModelFamily::Unlit { replace: false }, true) => "fragment_multiply_srgb",
            (NativeModelFamily::DetailDiffuse(_), false) => "fragment_detail_diffuse",
            (NativeModelFamily::DetailDiffuse(_), true) => "fragment_detail_diffuse_srgb",
            (NativeModelFamily::FourLayers { fog: false, .. }, false) => "fragment_four_nofog",
            (NativeModelFamily::FourLayers { fog: false, .. }, true) => "fragment_four_nofog_srgb",
            (NativeModelFamily::FourLayers { fog: true, .. }, false) => "fragment_four_fog",
            (NativeModelFamily::FourLayers { fog: true, .. }, true) => "fragment_four_fog_srgb",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::ScriptMulti,
                    ..
                },
                false,
            ) => "fragment_emissive_multi",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::ScriptMulti,
                    ..
                },
                true,
            ) => "fragment_emissive_multi_srgb",
            (NativeModelFamily::CharCloth, false) => "fragment_cloth",
            (NativeModelFamily::CharCloth, true) => "fragment_cloth_srgb",
            (NativeModelFamily::ZombieEyes, false) => "fragment_eyes",
            (NativeModelFamily::ZombieEyes, true) => "fragment_eyes_srgb",
        };
        let shader = if key.family.depth() {
            &self.plinth_shader
        } else if matches!(key.family, NativeModelFamily::Unlit { .. }) {
            &self.unlit_shader
        } else if matches!(key.family, NativeModelFamily::TileBlend { .. }) {
            &self.tile_shader
        } else if key.family.scripted_vertex() || key.family == NativeModelFamily::ZombieEyes {
            &self.scripted_shader
        } else {
            &self.shader
        };
        let mut layout = vec![
            if key.family.scripted_vertex() {
                self.scripted_scene_layout.clone()
            } else {
                self.scene_layout.clone()
            },
            if key.family.script_inputs() {
                self.scripted_object_layout.clone()
            } else {
                self.object_layout.clone()
            },
            self.material_layout(key.family).clone(),
        ];
        if key.family.atlas_texture() {
            layout.push(self.lighting_layout.clone());
        } else if key.family.depth() {
            layout.push(effect_scene_layout(key.depth_filtering));
        }
        let mut buffers = vec![VertexBufferLayout {
            array_stride: 32,
            step_mode: VertexStepMode::Vertex,
            attributes,
        }];
        if key.family.vertex_lit() {
            buffers.push(VertexBufferLayout {
                array_stride: 4,
                step_mode: VertexStepMode::Vertex,
                attributes: vec![VertexAttribute {
                    format: VertexFormat::Unorm8x4,
                    offset: 0,
                    shader_location: 5,
                }],
            });
        }
        RenderPipelineDescriptor {
            label: Some("T6 original light-probe static model".into()),
            layout,
            vertex: VertexState {
                shader: shader.clone(),
                entry_point: Some(
                    if key.family == NativeModelFamily::SmokePillar {
                        "vertex_smoke"
                    } else if matches!(key.family, NativeModelFamily::Flag(_)) {
                        "vertex_flag"
                    } else if matches!(key.family, NativeModelFamily::Canopy(_)) {
                        "vertex_canopy"
                    } else if key.family == NativeModelFamily::Feather3Layer {
                        "vertex"
                    } else if let NativeModelFamily::Unlit3Layer { vertex_lit, .. } = key.family {
                        if vertex_lit {
                            "vertex_falloff_mlv"
                        } else {
                            "vertex_falloff"
                        }
                    } else if let NativeModelFamily::FourLayers { vertex_lit, fog } = key.family {
                        if fog {
                            "vertex_four_fog"
                        } else if vertex_lit {
                            "vertex_four_mlv"
                        } else {
                            "vertex_four_nofog"
                        }
                    } else if matches!(key.family, NativeModelFamily::Unlit { .. }) {
                        "vertex_unlit"
                    } else if key.family == NativeModelFamily::Crystal3Layer(true) {
                        "vertex_crystal_mlv"
                    } else if key.family.geometric() && key.family.vertex_lit() {
                        "vertex_geometric_mlv"
                    } else if key.family.vertex_lit() && key.family.scripted_vertex() {
                        "vertex_scripted_mlv"
                    } else if key.family.vertex_lit() {
                        "vertex_mlv"
                    } else if key.family == NativeModelFamily::ZombieEyes {
                        "vertex_eyes"
                    } else if matches!(
                        key.family,
                        NativeModelFamily::Crystal3Layer(_) | NativeModelFamily::Camo3Layer
                    ) {
                        "vertex_parallax"
                    } else if key.family.scripted_vertex() {
                        "vertex_scripted"
                    } else if key.family
                        == NativeModelFamily::NormalSpecular(NormalSpecularProgram::Scrolling)
                    {
                        "vertex_scrolling"
                    } else if key.family.geometric() {
                        "vertex_geometric"
                    } else {
                        "vertex"
                    }
                    .into(),
                ),
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
                format: TextureFormat::Depth32FloatStencil8,
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

struct PreparedModelMaterial {
    bind_group: BindGroup,
    family: NativeModelFamily,
}

impl RenderAsset for PreparedModelMaterial {
    type SourceAsset = NativeModelMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderQueue>,
        SRes<PipelineCache>,
        SRes<ModelPipeline>,
        SRes<RenderAssets<GpuImage>>,
    );

    fn prepare_asset(
        source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        (device, queue, cache, pipeline, images): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let family = source.family();
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
        for ((binding, _, descriptor), image) in textures.into_iter().zip(gpu_images) {
            assert!(
                !image.texture_descriptor.format.is_srgb(),
                "native model inputs require original encoded UNORM texture reads"
            );
            assert!(
                descriptor.compare.is_none(),
                "native model material inputs require ordinary source samplers"
            );
            views.push((binding, &image.texture_view));
            samplers.push(device.create_sampler(descriptor));
        }
        let mut unlit_uniform = match &source {
            NativeModelMaterial::Unlit {
                scale_rgb: Some(value),
                ..
            } => Some(UniformBuffer::from(NativeUnlitScale { scale_rgb: *value })),
            _ => None,
        };
        if let Some(uniform) = &mut unlit_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut canopy_uniform = match &source {
            NativeModelMaterial::Canopy { constants, .. } => Some(UniformBuffer::from(*constants)),
            _ => None,
        };
        if let Some(uniform) = &mut canopy_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut reveal_uniform = match &source {
            NativeModelMaterial::RevealDiffuse { reveal, .. } => {
                Some(UniformBuffer::from(NativeModelReveal { reveal: *reveal }))
            }
            _ => None,
        };
        if let Some(uniform) = &mut reveal_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut detail_uniform = match &source {
            NativeModelMaterial::Flag {
                detail: Some(detail),
                ..
            } => Some(UniformBuffer::from(NativeModelDetailScale {
                detail_scale: detail.scale,
            })),
            NativeModelMaterial::DetailDiffuse { detail_scale, .. } => {
                Some(UniformBuffer::from(NativeModelDetailScale {
                    detail_scale: *detail_scale,
                }))
            }
            _ => None,
        };
        if let Some(uniform) = &mut detail_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut four_uniform = match &source {
            NativeModelMaterial::FourLayers { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut four_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut tile_uniform = match &source {
            NativeModelMaterial::TileBlend { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut tile_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut smoke_uniform = match &source {
            NativeModelMaterial::SmokePillar { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut smoke_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut flag_uniform = match &source {
            NativeModelMaterial::Flag { params, .. } => {
                Some(UniformBuffer::from(NativeModelFlagParams {
                    params: *params,
                }))
            }
            _ => None,
        };
        if let Some(uniform) = &mut flag_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut occlusion_uniform = match &source {
            NativeModelMaterial::Flag {
                occlusion_amount, ..
            } => Some(UniformBuffer::from(NativeModelOcclusion {
                occlusion_amount: *occlusion_amount,
            })),
            NativeModelMaterial::GeometricSpecular {
                occlusion_amount, ..
            }
            | NativeModelMaterial::NormalSpecular {
                occlusion_amount, ..
            }
            | NativeModelMaterial::AlphaSpecular {
                occlusion_amount, ..
            } => Some(UniformBuffer::from(NativeModelOcclusion {
                occlusion_amount: *occlusion_amount,
            })),
            _ => None,
        };
        if let Some(uniform) = &mut occlusion_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut parallax_uniform = match &source {
            NativeModelMaterial::Feather3Layer { constants, .. }
            | NativeModelMaterial::Unlit3Layer { constants, .. }
            | NativeModelMaterial::Crystal3Layer { constants, .. }
            | NativeModelMaterial::Camo3Layer { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut parallax_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut weapon_detail_uniform = match &source {
            NativeModelMaterial::MappedOcclusion {
                detail: Some(detail),
                ..
            } => Some(UniformBuffer::from(detail.parameters)),
            _ => None,
        };
        if let Some(uniform) = &mut weapon_detail_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut disc_uniform = match &source {
            NativeModelMaterial::PhongEmissive { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut disc_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut camo_uniform = match &source {
            NativeModelMaterial::Camo3Layer { weapon_param6, .. } => {
                assert!(
                    weapon_param6.is_finite() && weapon_param6.x != 0. && weapon_param6.y != 0.,
                    "selected original camo requires its actual delivered weaponParam6"
                );
                Some(UniformBuffer::from(NativeModelCamoParameters {
                    weapon_param6: *weapon_param6,
                }))
            }
            _ => None,
        };
        if let Some(uniform) = &mut camo_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut entries =
            Vec::with_capacity(views.len() * 2 + usize::from(occlusion_uniform.is_some()));
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
        if let Some(uniform) = &unlit_uniform {
            entries.push(BindGroupEntry {
                binding: 2,
                resource: uniform.binding().expect("unlit scale upload"),
            });
        }
        if let Some(uniform) = &detail_uniform {
            entries.push(BindGroupEntry {
                binding: 12,
                resource: uniform.binding().expect("detail scale upload"),
            });
        }
        if let Some(uniform) = &four_uniform {
            entries.push(BindGroupEntry {
                binding: 15,
                resource: uniform.binding().expect("four-layer controls upload"),
            });
        }
        if let Some(uniform) = &tile_uniform {
            entries.push(BindGroupEntry {
                binding: 18,
                resource: uniform.binding().expect("tile constants upload"),
            });
        }
        if let Some(uniform) = &smoke_uniform {
            entries.push(BindGroupEntry {
                binding: 13,
                resource: uniform.binding().expect("uploaded smoke controls"),
            });
        }
        if let Some(uniform) = &flag_uniform {
            entries.push(BindGroupEntry {
                binding: 17,
                resource: uniform.binding().expect("uploaded cloth parameters"),
            });
        }
        if let Some(uniform) = &canopy_uniform {
            entries.push(BindGroupEntry {
                binding: 16,
                resource: uniform.binding().expect("uploaded canopy constants"),
            });
        }
        if let Some(uniform) = &reveal_uniform {
            entries.push(BindGroupEntry {
                binding: 13,
                resource: uniform.binding().expect("uploaded model reveal"),
            });
        }
        if let Some(uniform) = &occlusion_uniform {
            entries.push(BindGroupEntry {
                binding: 8,
                resource: uniform
                    .binding()
                    .expect("uploaded original model occlusionAmount"),
            });
        }
        if let Some(uniform) = &parallax_uniform {
            entries.push(BindGroupEntry {
                binding: 12,
                resource: uniform.binding().expect("original parallax constants"),
            });
        }
        if let Some(uniform) = &disc_uniform {
            entries.push(BindGroupEntry {
                binding: 13,
                resource: uniform.binding().expect("original crypt disc constants"),
            });
        }
        if let Some(uniform) = &camo_uniform {
            entries.push(BindGroupEntry {
                binding: 14,
                resource: uniform.binding().expect("original selected weaponParam6"),
            });
        }
        if let Some(uniform) = &weapon_detail_uniform {
            entries.push(BindGroupEntry {
                binding: 18,
                resource: uniform.binding().expect("original weaponParam2"),
            });
        }
        let bind_group = device.create_bind_group(
            "native model material",
            &cache.get_bind_group_layout(pipeline.material_layout(family)),
            &entries,
        );
        Ok(Self { bind_group, family })
    }
}

#[derive(Resource, Default)]
struct ModelUniforms {
    scenes: DynamicUniformBuffer<NativeModelScene>,
    scripted_scenes: DynamicUniformBuffer<NativeModelScriptedScene>,
    objects: DynamicUniformBuffer<NativeModelObject>,
    scripts: DynamicUniformBuffer<NativeModelScriptInputs>,
    scene_offsets: HashMap<RetainedViewEntity, u32>,
    scripted_scene_offsets: HashMap<RetainedViewEntity, u32>,
    object_offsets: HashMap<(MainEntity, bool), u32>,
    script_offsets: HashMap<MainEntity, u32>,
}

#[derive(Resource, Default)]
struct ModelScriptOwners(HashMap<MainEntity, NativeModelScriptInputs>);

fn extract_script_owners(
    mut owners: ResMut<ModelScriptOwners>,
    source: Extract<Query<(Entity, &crate::presentation::cues::CueParameters)>>,
) {
    owners.0.clear();
    owners.0.extend(source.iter().map(|(entity, inputs)| {
        let vectors = std::array::from_fn(|slot| {
            if inputs.script_available & (1 << slot) != 0 {
                Vec4::from_array(inputs.script_vectors[slot])
            } else {
                Vec4::ZERO
            }
        });
        (entity.into(), NativeModelScriptInputs { vectors })
    }));
}

#[derive(Resource, Default)]
struct ModelBindGroups {
    scene: Option<(BufferId, BindGroup)>,
    scripted_scene: Option<((BufferId, BufferId), BindGroup)>,
    object: Option<(BufferId, BindGroup)>,
    scripted_object: Option<((BufferId, BufferId), BindGroup)>,
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
    if uniforms.objects.is_empty() {
        groups.scene = None;
        groups.scripted_scene = None;
        groups.object = None;
        groups.scripted_object = None;
        groups.lighting = None;
        return;
    }
    uniforms.scenes.write_buffer(&device, &queue);
    uniforms.objects.write_buffer(&device, &queue);
    uniforms.scripted_scenes.write_buffer(&device, &queue);
    uniforms.scripts.write_buffer(&device, &queue);
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
    if let Some(atlas) = atlas {
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
    } else {
        groups.lighting = None;
    }
    if uniforms.scripted_scenes.is_empty() {
        groups.scripted_scene = None;
    } else {
        let key = (
            scene_buffer.id(),
            uniforms
                .scripted_scenes
                .buffer()
                .expect("scripted scene upload")
                .id(),
        );
        if groups
            .scripted_scene
            .as_ref()
            .is_none_or(|(id, _)| *id != key)
        {
            groups.scripted_scene = Some((
                key,
                device.create_bind_group(
                    "original scripted model scene uniforms",
                    &cache.get_bind_group_layout(&pipeline.scripted_scene_layout),
                    &BindGroupEntries::sequential((
                        uniforms.scenes.binding().expect("model scene upload"),
                        uniforms
                            .scripted_scenes
                            .binding()
                            .expect("scripted scene upload"),
                    )),
                ),
            ));
        }
    }
    if uniforms.scripts.is_empty() {
        groups.scripted_object = None;
    } else {
        let key = (
            object_buffer.id(),
            uniforms.scripts.buffer().expect("script input upload").id(),
        );
        if groups
            .scripted_object
            .as_ref()
            .is_none_or(|(id, _)| *id != key)
        {
            groups.scripted_object = Some((
                key,
                device.create_bind_group(
                    "original model object and script vectors",
                    &cache.get_bind_group_layout(&pipeline.scripted_object_layout),
                    &BindGroupEntries::sequential((
                        uniforms.objects.binding().expect("model object upload"),
                        uniforms.scripts.binding().expect("script input upload"),
                    )),
                ),
            ));
        }
    }
}

fn queue_models(
    pipeline: Res<ModelPipeline>,
    cache: Res<PipelineCache>,
    mut variants: ResMut<SpecializedRenderPipelines<ModelPipeline>>,
    mut phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    geometry_inputs: (
        Res<RenderAssets<PreparedNativeGeometry>>,
        Res<RenderAssets<PreparedAnimatedModel>>,
        Query<&PreparedSkin>,
    ),
    materials: Res<RenderAssets<PreparedModelMaterial>>,
    views: Query<(
        &ExtractedView,
        &RenderVisibleEntities,
        &ViewTarget,
        &Msaa,
        &NativeWorldScene,
        Option<&ExtractedEffectView>,
    )>,
    surfaces: Query<(&ExtractedNativeModelSurface, &NativeWorldObject)>,
    atlas: Option<ResMut<ModelLightingAtlas>>,
    frame: Res<FrameCount>,
    script_owners: Res<ModelScriptOwners>,
    mut uniforms: ResMut<ModelUniforms>,
    mut queued: Local<HashMap<RetainedViewEntity, Vec<MainEntity>>>,
    mut ordered: ResMut<NativeOrderedDraws>,
) {
    let (geometries, animated, skins) = geometry_inputs;
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
    uniforms.scripted_scenes.clear();
    uniforms.objects.clear();
    uniforms.scripts.clear();
    uniforms.scene_offsets.clear();
    uniforms.scripted_scene_offsets.clear();
    uniforms.object_offsets.clear();
    uniforms.script_offsets.clear();
    let mut atlas = atlas;
    let draw_function = draw_functions
        .read()
        .id::<(SetItemPipeline, DrawNativeModel)>();
    for (view, visible, target, msaa, scene, effect_view) in &views {
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
            let (Some((vertices, indices, offset, _)), Some(material)) = (
                surface.geometry.buffers(
                    &geometries,
                    &animated,
                    surface.geometry.instance().and_then(|e| skins.get(e).ok()),
                    surface.primary_byte_offset,
                ),
                materials.get(surface.material),
            ) else {
                continue;
            };
            assert!(
                offset.is_multiple_of(32) && offset < vertices.size(),
                "native static model requires its complete PACKED32 vertex range"
            );
            assert!(
                surface.index_count.is_multiple_of(3)
                    && surface
                        .first_index
                        .checked_add(surface.index_count)
                        .is_some_and(|end| u64::from(end) * 2 <= indices.size()),
                "native static model draw must reference complete source Uint16 triangles"
            );
            if material.family.scripted_vertex()
                && !uniforms
                    .scripted_scene_offsets
                    .contains_key(&view.retained_view_entity)
            {
                let offset = uniforms.scripted_scenes.push(&NativeModelScriptedScene {
                    sun_position: scene.sun_position,
                    sun_diffuse: scene.sun_diffuse,
                    game_time: scene.game_time,
                    // 00416380 uses fixed 6500 K and r_heroLightScale, whose
                    // shipped default is (1,1,1). 0074d390 quantizes the neutral
                    // color-temperature matrix to identity at 1/1024 precision.
                    hero_lighting: [Vec4::X, Vec4::Y, Vec4::Z],
                });
                uniforms
                    .scripted_scene_offsets
                    .insert(view.retained_view_entity, offset);
            }
            if material.family.script_inputs()
                && !uniforms.script_offsets.contains_key(&surface.placement)
            {
                let source = script_owners
                    .0
                    .get(&surface.placement)
                    .copied()
                    .unwrap_or_default();
                let offset = uniforms.scripts.push(&source);
                uniforms.script_offsets.insert(surface.placement, offset);
            }
            let object_key = (surface.placement, material.family.atlas());
            if !uniforms.object_offsets.contains_key(&object_key) {
                let lighting = if material.family.atlas() {
                    let atlas = atlas.as_deref_mut().expect("original model lighting atlas");
                    let lighting = if let Some(origin) = surface.dynamic_lighting_origin {
                        atlas
                            .touch_dynamic_grid(surface.placement, origin, frame.0)
                            .map(|(lighting, _)| lighting)
                    } else {
                        atlas
                            .touch(
                                surface.placement,
                                surface.colors_index,
                                surface.visibility,
                                frame.0,
                            )
                            .ok()
                    };
                    let Some(lighting) = lighting else {
                        continue;
                    };
                    Some(lighting)
                } else {
                    None
                };
                let offset = uniforms.objects.push(&NativeModelObject {
                    world_rows: source_object.world_rows,
                    // The eye VS/PS consume only world_rows. Its declared but
                    // unused lighting fields need no atlas slot or substitute.
                    grid_lighting_coords_and_vis: lighting
                        .as_ref()
                        .map_or(Vec4::ZERO, |light| light.grid_coords_and_vis),
                    grid_lighting_sh0: lighting.as_ref().map_or(Vec4::ZERO, |light| light.sh[0]),
                    grid_lighting_sh1: lighting.as_ref().map_or(Vec4::ZERO, |light| light.sh[1]),
                    grid_lighting_sh2: lighting.as_ref().map_or(Vec4::ZERO, |light| light.sh[2]),
                    reflection_lighting_sh0: source_object.reflection_lighting_sh0,
                    reflection_lighting_sh1: source_object.reflection_lighting_sh1,
                    reflection_lighting_sh2: source_object.reflection_lighting_sh2,
                });
                uniforms.object_offsets.insert(object_key, offset);
            }
            let pipeline = variants.specialize(
                &cache,
                &pipeline,
                ModelPipelineKey {
                    target: target.main_texture_format(),
                    samples: msaa.samples(),
                    state: surface.state,
                    family: material.family,
                    depth_filtering: material.family.depth()
                        && effect_view
                            .expect("plinth glow requires source floatZ view")
                            .filtering,
                },
            );
            if surface.pass != NativeWorldPass::Opaque {
                ordered
                    .0
                    .entry((view.retained_view_entity, surface.pass))
                    .or_default()
                    .push(NativeOrderedDraw {
                        entity,
                        model: true,
                        pipeline,
                        sort_key: surface.sort_key,
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
                    asset_id: surface.geometry.asset_id(),
                },
                (entity, main_entity),
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
            );
            queued.push(main_entity);
        }
    }
    for draws in ordered.0.values_mut() {
        draws.sort_unstable_by_key(|draw| draw.sort_key);
    }
}

struct DrawNativeModel;

impl<P: PhaseItem> RenderCommand<P> for DrawNativeModel {
    type Param = (
        SRes<ModelUniforms>,
        SRes<ModelBindGroups>,
        SRes<RenderAssets<PreparedModelMaterial>>,
        SRes<RenderAssets<PreparedNativeGeometry>>,
        SRes<RenderAssets<PreparedAnimatedModel>>,
        SQuery<&'static PreparedSkin>,
        SRes<RenderAssets<PreparedModelVertexLighting>>,
    );
    type ViewQuery = (
        &'static ExtractedView,
        Option<(
            &'static PreparedEffectView,
            &'static DynamicUniformIndex<NativeFxFrame>,
        )>,
    );
    type ItemQuery = &'static ExtractedNativeModelSurface;

    fn render<'w>(
        _: &P,
        (view, depth): ROQueryItem<'w, '_, Self::ViewQuery>,
        surface: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (uniforms, groups, materials, geometries, animated, skins, lighting): SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
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
        draw_model(
            surface,
            view,
            uniforms,
            groups,
            materials,
            geometries,
            animated.into_inner(),
            surface
                .geometry
                .instance()
                .and_then(|e| skins.get_inner(e).ok()),
            lighting.into_inner(),
            depth,
            pass,
        )
    }
}

/// One authored lighting stream for all static placements. Geometry stays shared.
#[derive(Asset, TypePath, Clone)]
pub(crate) struct ModelVertexLighting(pub Vec<u8>);

struct PreparedModelVertexLighting(Buffer);

impl RenderAsset for PreparedModelVertexLighting {
    type SourceAsset = ModelVertexLighting;
    type Param = SRes<RenderDevice>;

    fn asset_usage(_: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::RENDER_WORLD
    }

    fn take_gpu_data(
        source: &mut Self::SourceAsset,
        _: Option<&Self>,
    ) -> Result<Self::SourceAsset, AssetExtractionError> {
        if source.0.is_empty() {
            return Err(AssetExtractionError::AlreadyExtracted);
        }
        Ok(ModelVertexLighting(std::mem::take(&mut source.0)))
    }

    fn byte_len(source: &Self::SourceAsset) -> Option<usize> {
        Some(source.0.len())
    }

    fn prepare_asset(
        source: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        device: &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        assert!(!source.0.is_empty() && source.0.len().is_multiple_of(4));
        Ok(Self(device.create_buffer_with_data(
            &BufferInitDescriptor {
                label: Some("Original static model vertex lighting"),
                contents: &source.0,
                usage: BufferUsages::VERTEX,
            },
        )))
    }
}

fn draw_model<'w>(
    surface: &ExtractedNativeModelSurface,
    view: &ExtractedView,
    uniforms: &'w ModelUniforms,
    groups: &'w ModelBindGroups,
    materials: &'w RenderAssets<PreparedModelMaterial>,
    geometries: &'w RenderAssets<PreparedNativeGeometry>,
    animated: &'w RenderAssets<PreparedAnimatedModel>,
    skin: Option<&'w PreparedSkin>,
    lighting: &'w RenderAssets<PreparedModelVertexLighting>,
    depth: Option<(
        &'w PreparedEffectView,
        &'w DynamicUniformIndex<NativeFxFrame>,
    )>,
    pass: &mut TrackedRenderPass<'w>,
) -> RenderCommandResult {
    let Some(material) = materials.get(surface.material) else {
        return RenderCommandResult::Skip;
    };
    let (
        Some((_, scene)),
        Some((_, object)),
        Some((vertices, indices, vertex_offset, true)),
        Some(&scene_offset),
        Some(&object_offset),
    ) = (
        &groups.scene,
        &groups.object,
        surface
            .geometry
            .buffers(geometries, animated, skin, surface.primary_byte_offset),
        uniforms.scene_offsets.get(&view.retained_view_entity),
        uniforms
            .object_offsets
            .get(&(surface.placement, material.family.atlas())),
    )
    else {
        return RenderCommandResult::Skip;
    };
    if material.family.scripted_vertex() {
        let (Some((_, scripted_scene)), Some(&offset)) = (
            &groups.scripted_scene,
            uniforms
                .scripted_scene_offsets
                .get(&view.retained_view_entity),
        ) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(0, scripted_scene, &[scene_offset, offset]);
    } else {
        pass.set_bind_group(0, scene, &[scene_offset]);
    }
    if material.family.script_inputs() {
        let (Some((_, scripted_object)), Some(&offset)) = (
            &groups.scripted_object,
            uniforms.script_offsets.get(&surface.placement),
        ) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(1, scripted_object, &[object_offset, offset]);
    } else {
        pass.set_bind_group(1, object, &[object_offset]);
    }
    pass.set_bind_group(2, &material.bind_group, &[]);
    if material.family.atlas_texture() {
        let Some((_, lighting)) = &groups.lighting else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(3, lighting, &[]);
    }
    if material.family.depth() {
        let Some((depth, frame)) = depth else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(3, &depth.bind_group, &[frame.index()]);
    }
    if material.family.vertex_lit() {
        let Some((handle, offset)) = surface.vertex_lighting else {
            return RenderCommandResult::Skip;
        };
        let Some(stream) = lighting.get(handle) else {
            return RenderCommandResult::Skip;
        };
        pass.set_vertex_buffer(1, stream.0.slice(offset..));
    }
    pass.set_vertex_buffer(0, vertices.slice(vertex_offset..));
    pass.set_index_buffer(indices.slice(..), IndexFormat::Uint16);
    pass.draw_indexed(
        surface.first_index..surface.first_index + surface.index_count,
        0,
        0..1,
    );
    RenderCommandResult::Success
}

#[derive(SystemParam)]
pub(crate) struct ModelOrderedRenderer<'w, 's> {
    surfaces: Query<'w, 's, &'static ExtractedNativeModelSurface>,
    uniforms: Res<'w, ModelUniforms>,
    groups: Res<'w, ModelBindGroups>,
    materials: Res<'w, RenderAssets<PreparedModelMaterial>>,
    geometries: Res<'w, RenderAssets<PreparedNativeGeometry>>,
    animated: Res<'w, RenderAssets<PreparedAnimatedModel>>,
    skins: Query<'w, 's, &'static PreparedSkin>,
    lighting: Res<'w, RenderAssets<PreparedModelVertexLighting>>,
}

impl ModelOrderedRenderer<'_, '_> {
    pub(crate) fn draw<'a>(
        &'a self,
        entity: Entity,
        view: &ExtractedView,
        depth: Option<(
            &'a PreparedEffectView,
            &'a DynamicUniformIndex<NativeFxFrame>,
        )>,
        pass: &mut TrackedRenderPass<'a>,
    ) {
        if let Ok(surface) = self.surfaces.get(entity) {
            draw_model(
                surface,
                view,
                &self.uniforms,
                &self.groups,
                &self.materials,
                &self.geometries,
                &self.animated,
                surface
                    .geometry
                    .instance()
                    .and_then(|e| self.skins.get(e).ok()),
                &self.lighting,
                depth,
                pass,
            );
        }
    }
}
