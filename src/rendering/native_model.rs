//! Original T6 static-model LIT draws, using each source material's program.
//!
//! NativeWorldPlugin owns geometry upload and common scene/object extraction.
//! This renderer supplies the original model-lighting atlas and cb3 constants
//! for visible placements, then draws PACKED32 geometry in the opaque phase.
use super::animated_models::{AnimatedModel, DynamicLightingOrigin};
use super::native_effect::{
    ExtractedEffectView, NativeFxFrame, PreparedEffectView, effect_scene_layout,
};
use super::native_weapon_view::{NativeModelDrawMatrices, NativeWeaponOwners, draw_constants};
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
        extract_component::ExtractComponentPlugin,
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
pub(crate) struct NativeModelSingleLayerConstants {
    pub source: [Vec4; 4],
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
pub(crate) struct NativeClanTagConstants {
    pub controls: Vec4,
    pub atlas: Vec4,
    pub characters: Vec4,
}

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeClothRevealConstants {
    pub source: [Vec4; 4],
}

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeFlickerConstants {
    pub controls: Vec4,
    pub map_column: Vec4,
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
    ScriptMulti { alpha_test: bool },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NormalSpecularProgram {
    Grid,
    VertexLit,
    Scrolling,
    Blended { unattenuated_reflection: bool },
}

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeSourceCharacterConstants {
    pub source: [Vec4; 5],
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum SourceCharacterProgram {
    Ember,
    Glass,
    Fritz { cutout: bool },
    Hair,
    SkinCutout,
    ClothEmissive,
    SkinEmissive,
    RevealEmissive { cutout: bool },
}

impl SourceCharacterProgram {
    fn extra_textures(self) -> usize {
        match self {
            Self::Ember | Self::Fritz { .. } => 3,
            Self::Glass | Self::Hair | Self::SkinCutout => 0,
            Self::ClothEmissive | Self::SkinEmissive => 1,
            Self::RevealEmissive { .. } => 2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum SourceLitProgram {
    BlendedHero,
    GeometricHero,
    NormalHero,
    CutoutHero,
    ColorDetail,
    NormalDetail,
    CutoutNormalDetail,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NativeModelUnlitProgram {
    Replace,
    Multiply,
    Blend,
    Add,
    ViewmodelAdd,
    ViewmodelSqrtAlpha,
    ViewmodelAlpha,
    ViewmodelOpaque,
}

/// Original weaponParam2 and selected color-detail texture. Base weapon setup
/// 0052c1d0 supplies zero; 0052b7c0 overrides XY/scale/enable for selected camo.
#[derive(Clone)]
pub(crate) struct NativeWeaponDetail {
    pub map: Handle<Image>,
    pub sampler: SamplerDescriptor<'static>,
    pub parameters: Vec4,
    pub hero_lighting: bool,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NativeReticleProgram {
    Dynamic,
    DynamicTwoColor,
    Static,
    Ui3d,
}
#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeReticleConstants {
    pub source: [Vec4; 5],
}

#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeSonarConstants {
    pub source: [Vec4; 10],
}
#[derive(Clone, Copy, PartialEq, ShaderType)]
struct NativeSonarCodeView {
    z_near: Vec4,
    target_size: Vec4,
    viewport: Vec4,
}
struct ModelSonarGroup {
    key: [TextureViewId; 3],
    source: NativeSonarCodeView,
    frame: UniformBuffer<NativeSonarCodeView>,
    group: BindGroup,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub(crate) enum NativeEffectModelProgram {
    DistortOpaque,
    DistortAlpha,
    ScrollingFeather,
    ScrollingFalloff,
    ElectricDisc,
    LightSphere,
    Portal,
    SkyPillar,
    ElectricSphere,
    ElectricSphereAlpha,
    IceBlizzard,
    IceStatic,
    Tendril,
    WaterReveal,
    OrbScroll,
}
impl NativeEffectModelProgram {
    const ALL: [Self; 15] = [
        Self::DistortOpaque,
        Self::DistortAlpha,
        Self::ScrollingFeather,
        Self::ScrollingFalloff,
        Self::ElectricDisc,
        Self::LightSphere,
        Self::Portal,
        Self::SkyPillar,
        Self::ElectricSphere,
        Self::ElectricSphereAlpha,
        Self::IceBlizzard,
        Self::IceStatic,
        Self::Tendril,
        Self::WaterReveal,
        Self::OrbScroll,
    ];
    fn texture_count(self) -> usize {
        match self {
            Self::DistortOpaque => 1,
            Self::DistortAlpha => 2,
            Self::ScrollingFeather => 1,
            Self::ScrollingFalloff => 1,
            Self::ElectricDisc => 7,
            Self::LightSphere => 0,
            Self::Portal => 6,
            Self::SkyPillar => 4,
            Self::ElectricSphere => 3,
            Self::ElectricSphereAlpha => 3,
            Self::IceBlizzard => 4,
            Self::IceStatic => 3,
            Self::Tendril => 3,
            Self::WaterReveal => 6,
            Self::OrbScroll => 2,
        }
    }
    pub(crate) fn probe(self) -> bool {
        matches!(
            self,
            Self::ElectricSphere
                | Self::ElectricSphereAlpha
                | Self::IceBlizzard
                | Self::IceStatic
                | Self::WaterReveal
                | Self::OrbScroll
        )
    }
    fn depth(self) -> bool {
        matches!(
            self,
            Self::DistortAlpha
                | Self::ScrollingFeather
                | Self::ElectricDisc
                | Self::LightSphere
                | Self::SkyPillar
                | Self::ElectricSphere
                | Self::ElectricSphereAlpha
                | Self::IceBlizzard
        )
    }
    fn atlas_owner(self) -> bool {
        matches!(
            self,
            Self::ElectricSphere
                | Self::ElectricSphereAlpha
                | Self::IceBlizzard
                | Self::IceStatic
                | Self::WaterReveal
                | Self::OrbScroll
        )
    }
    fn atlas_texture(self) -> bool {
        matches!(self, Self::WaterReveal)
    }
    fn scripts(self) -> bool {
        matches!(self, Self::SkyPillar | Self::WaterReveal)
    }
}
#[derive(Clone, Copy, ShaderType)]
pub(crate) struct NativeEffectModelConstants {
    pub source: [Vec4; 12],
}

#[derive(Asset, TypePath, Clone)]
pub(crate) enum NativeModelMaterial {
    Objective {
        base: NativeModelColorTexture,
        constants: NativeSourceCharacterConstants,
    },
    ProjectedDecal(NativeModelColorTexture),
    Cornea {
        normal: (Handle<Image>, SamplerDescriptor<'static>),
        mask: (Handle<Image>, SamplerDescriptor<'static>),
        constants: NativeSourceCharacterConstants,
    },
    Effect {
        program: NativeEffectModelProgram,
        textures: Vec<(Handle<Image>, SamplerDescriptor<'static>)>,
        probe: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
        constants: NativeEffectModelConstants,
    },
    SourceCharacter {
        program: SourceCharacterProgram,
        base: NativeModelTextures,
        specular: (Handle<Image>, SamplerDescriptor<'static>),
        emission: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
        temporal: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
        reveal: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
        constants: NativeSourceCharacterConstants,
    },
    SourceLit {
        program: SourceLitProgram,
        color: NativeModelColorTexture,
        normal: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
        specular: (Handle<Image>, SamplerDescriptor<'static>),
        probe: (Handle<Image>, SamplerDescriptor<'static>),
        detail: Option<(Handle<Image>, SamplerDescriptor<'static>)>,
        occlusion: Vec4,
        detail_parameters: Vec4,
    },
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
        program: NativeModelUnlitProgram,
        scale_rgb: Vec4,
    },
    DecalMultiply(NativeModelColorTexture),
    DecalAdd(NativeModelColorTexture),
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
    Ui3dReticle {
        noise: (Handle<Image>, SamplerDescriptor<'static>),
        constants: NativeReticleConstants,
    },
    Sonar {
        textures: [(Handle<Image>, SamplerDescriptor<'static>); 2],
        constants: NativeSonarConstants,
    },
    Reticle {
        program: NativeReticleProgram,
        textures: [(Handle<Image>, SamplerDescriptor<'static>); 2],
        constants: NativeReticleConstants,
    },
    OpticLens {
        textures: [(Handle<Image>, SamplerDescriptor<'static>); 4],
        probe: Handle<Image>,
        probe_sampler: SamplerDescriptor<'static>,
        controls: Vec4,
    },
    SingleLayer {
        layer: (Handle<Image>, SamplerDescriptor<'static>),
        probe: Handle<Image>,
        probe_sampler: SamplerDescriptor<'static>,
        constants: NativeModelSingleLayerConstants,
    },
    Staff3Layer {
        textures: NativeModelThreeLayerTextures,
        off: (Handle<Image>, SamplerDescriptor<'static>),
        constants: NativeModelParallaxConstants,
        emission_on: Vec4,
    },
    Crystal3Layer {
        vertex_lit: bool,
        textures: NativeModelThreeLayerTextures,
        constants: NativeModelParallaxConstants,
    },
    #[expect(
        dead_code,
        reason = "native camo rendering is retained until weapon camo selection supplies materials"
    )]
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
    Flicker {
        base: NativeModelTextures,
        flicker_map: Handle<Image>,
        flicker_sampler: SamplerDescriptor<'static>,
        emissive_map: Handle<Image>,
        emissive_sampler: SamplerDescriptor<'static>,
        constants: NativeFlickerConstants,
    },
    ClanTag {
        base: NativeModelTextures,
        detail_map: Handle<Image>,
        detail_sampler: SamplerDescriptor<'static>,
        constants: NativeClanTagConstants,
    },
    TransparentPhong {
        player_icon: bool,
        base: NativeModelTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        controls: Vec4,
    },
    Character {
        base: NativeModelTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        skin_controls: Option<Vec4>,
    },
    ClothReveal {
        base: NativeModelTextures,
        specular_map: Handle<Image>,
        specular_sampler: SamplerDescriptor<'static>,
        temporal_map: Handle<Image>,
        temporal_sampler: SamplerDescriptor<'static>,
        constants: NativeClothRevealConstants,
        alpha_test: bool,
    },
    ZombieEyes(NativeModelColorTexture),
}

impl NativeModelMaterial {
    pub(crate) fn images(&self) -> impl Iterator<Item = &Handle<Image>> {
        self.textures().map(|(_, image, _)| image)
    }

    fn textures(&self) -> impl Iterator<Item = (u32, &Handle<Image>, &SamplerDescriptor<'static>)> {
        let bindings = match self {
            Self::Objective { base, .. } | Self::ProjectedDecal(base) => [
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
            Self::Cornea { normal, mask, .. } => [
                Some((0, &normal.0, &normal.1)),
                Some((2, &mask.0, &mask.1)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::Effect {
                textures, probe, ..
            } => std::array::from_fn(|index| {
                if index == 8 {
                    probe.as_ref().map(|(image, sampler)| (28, image, sampler))
                } else {
                    textures
                        .get(index)
                        .map(|(image, sampler)| (index as u32 * 2, image, sampler))
                }
            }),
            Self::SourceCharacter {
                base,
                specular,
                emission,
                temporal,
                reveal,
                ..
            } => [
                Some((0, &base.normal_map, &base.normal_sampler)),
                Some((2, &base.color_map, &base.color_sampler)),
                Some((4, &specular.0, &specular.1)),
                emission
                    .as_ref()
                    .map(|(image, sampler)| (6, image, sampler)),
                Some((8, &base.reflection_probe, &base.reflection_sampler)),
                temporal
                    .as_ref()
                    .map(|(image, sampler)| (10, image, sampler)),
                reveal.as_ref().map(|(image, sampler)| (27, image, sampler)),
                None,
                None,
            ],
            Self::SourceLit {
                color,
                normal,
                specular,
                probe,
                detail,
                ..
            } => [
                Some((0, &color.color_map, &color.color_sampler)),
                normal.as_ref().map(|(image, sampler)| (2, image, sampler)),
                Some((4, &probe.0, &probe.1)),
                Some((6, &specular.0, &specular.1)),
                detail.as_ref().map(|(image, sampler)| (10, image, sampler)),
                None,
                None,
                None,
                None,
            ],
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
            Self::Unlit { base, .. } | Self::DecalMultiply(base) | Self::DecalAdd(base) => [
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
            Self::Ui3dReticle { noise, .. } => [
                Some((0, &noise.0, &noise.1)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::Sonar { textures, .. } => [
                Some((0, &textures[0].0, &textures[0].1)),
                Some((2, &textures[1].0, &textures[1].1)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::Reticle { textures, .. } => [
                Some((0, &textures[0].0, &textures[0].1)),
                Some((2, &textures[1].0, &textures[1].1)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::OpticLens {
                textures,
                probe,
                probe_sampler,
                ..
            } => [
                Some((0, &textures[0].0, &textures[0].1)),
                Some((2, &textures[1].0, &textures[1].1)),
                Some((4, &textures[2].0, &textures[2].1)),
                Some((6, &textures[3].0, &textures[3].1)),
                Some((8, probe, probe_sampler)),
                None,
                None,
                None,
                None,
            ],
            Self::SingleLayer {
                layer,
                probe,
                probe_sampler,
                ..
            } => [
                Some((0, &layer.0, &layer.1)),
                Some((8, probe, probe_sampler)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            Self::Staff3Layer { textures, off, .. } => [
                Some((0, &textures.normal_map, &textures.normal_sampler)),
                Some((2, &textures.layer1_map, &textures.layer1_sampler)),
                Some((4, &textures.layer2_map, &textures.layer2_sampler)),
                Some((6, &textures.layer3_map, &textures.layer3_sampler)),
                Some((8, &textures.reflection_probe, &textures.reflection_sampler)),
                Some((10, &off.0, &off.1)),
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
            Self::Flicker {
                base,
                flicker_map,
                flicker_sampler,
                emissive_map,
                emissive_sampler,
                ..
            } => [
                Some((0, &base.normal_map, &base.normal_sampler)),
                Some((2, &base.color_map, &base.color_sampler)),
                Some((4, flicker_map, flicker_sampler)),
                Some((6, emissive_map, emissive_sampler)),
                Some((8, &base.reflection_probe, &base.reflection_sampler)),
                None,
                None,
                None,
                None,
            ],
            Self::ClanTag {
                base,
                detail_map,
                detail_sampler,
                ..
            } => [
                Some((0, &base.normal_map, &base.normal_sampler)),
                Some((2, detail_map, detail_sampler)),
                Some((4, &base.color_map, &base.color_sampler)),
                Some((8, &base.reflection_probe, &base.reflection_sampler)),
                None,
                None,
                None,
                None,
                None,
            ],
            Self::ClothReveal {
                base,
                specular_map,
                specular_sampler,
                temporal_map,
                temporal_sampler,
                ..
            } => [
                Some((0, &base.normal_map, &base.normal_sampler)),
                Some((2, &base.color_map, &base.color_sampler)),
                Some((4, specular_map, specular_sampler)),
                Some((6, temporal_map, temporal_sampler)),
                Some((8, &base.reflection_probe, &base.reflection_sampler)),
                None,
                None,
                None,
                None,
            ],
            Self::Character {
                base,
                specular_map,
                specular_sampler,
                ..
            }
            | Self::TransparentPhong {
                base,
                specular_map,
                specular_sampler,
                ..
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
            Self::Objective { .. } => NativeModelFamily::Objective,
            Self::ProjectedDecal(_) => NativeModelFamily::ProjectedDecal,
            Self::Cornea { .. } => NativeModelFamily::Cornea,
            Self::Effect { program, .. } => NativeModelFamily::Effect(*program),
            Self::SourceCharacter { program, .. } => NativeModelFamily::SourceCharacter(*program),
            Self::SourceLit { program, .. } => NativeModelFamily::SourceLit(*program),
            Self::SmokePillar { .. } => NativeModelFamily::SmokePillar,
            Self::Flag { detail, .. } => NativeModelFamily::Flag(detail.is_some()),
            Self::Canopy { quasi, .. } => NativeModelFamily::Canopy(*quasi),
            Self::RevealDiffuse { .. } => NativeModelFamily::RevealDiffuse,
            Self::MappedOcclusion { detail, .. } => NativeModelFamily::MappedOcclusion {
                detail: detail.is_some(),
                hero_lighting: detail.as_ref().is_some_and(|detail| detail.hero_lighting),
            },
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
            Self::Unlit { program, .. } => NativeModelFamily::Unlit(*program),
            Self::DecalMultiply(_) => NativeModelFamily::DecalMultiply,
            Self::DecalAdd(_) => NativeModelFamily::DecalAdd,
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
            Self::Ui3dReticle { .. } => NativeModelFamily::Reticle(NativeReticleProgram::Ui3d),
            Self::Sonar { .. } => NativeModelFamily::Sonar,
            Self::Reticle { program, .. } => NativeModelFamily::Reticle(*program),
            Self::OpticLens { .. } => NativeModelFamily::OpticLens,
            Self::SingleLayer { .. } => NativeModelFamily::SingleLayer,
            Self::Staff3Layer { .. } => NativeModelFamily::Staff3Layer,
            Self::Camo3Layer { .. } => NativeModelFamily::Camo3Layer,
            Self::PhongEmissive {
                program,
                vertex_lit,
                ..
            } => NativeModelFamily::PhongEmissive {
                program: *program,
                vertex_lit: *vertex_lit,
            },
            Self::Flicker { .. } => NativeModelFamily::Flicker,
            Self::ClanTag { .. } => NativeModelFamily::ClanTag,
            Self::Character { skin_controls, .. } => NativeModelFamily::Character {
                skin: skin_controls.is_some(),
            },
            Self::TransparentPhong { player_icon, .. } => NativeModelFamily::TransparentPhong {
                player_icon: *player_icon,
            },
            Self::ClothReveal { alpha_test, .. } => NativeModelFamily::ClothReveal(*alpha_test),
            Self::ZombieEyes(_) => NativeModelFamily::ZombieEyes,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum NativeModelFamily {
    Objective,
    ProjectedDecal,
    Cornea,
    Effect(NativeEffectModelProgram),
    SourceCharacter(SourceCharacterProgram),
    SourceLit(SourceLitProgram),
    SmokePillar,
    Flag(bool),
    Canopy(bool),
    RevealDiffuse,
    MappedOcclusion {
        detail: bool,
        hero_lighting: bool,
    },
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
    Unlit(NativeModelUnlitProgram),
    DecalMultiply,
    DecalAdd,
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
    Reticle(NativeReticleProgram),
    Sonar,
    OpticLens,
    SingleLayer,
    Staff3Layer,
    Crystal3Layer(bool),
    Camo3Layer,
    PhongEmissive {
        program: EmissiveProgram,
        vertex_lit: bool,
    },
    Flicker,
    ClanTag,
    Character {
        skin: bool,
    },
    TransparentPhong {
        player_icon: bool,
    },
    ClothReveal(bool),
    ZombieEyes,
}

impl NativeModelFamily {
    fn draw_matrices(self) -> bool {
        matches!(
            self,
            Self::Reticle(_) | Self::Sonar | Self::Objective | Self::ProjectedDecal
        )
    }

    fn geometric(self) -> bool {
        matches!(
            self,
            Self::Objective
                | Self::ProjectedDecal
                | Self::SourceLit(SourceLitProgram::GeometricHero)
                | Self::GeometricDiffuse(_)
                | Self::GeometricSpecular { .. }
                | Self::Unlit(_)
                | Self::DecalMultiply
                | Self::DecalAdd
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
            Self::Cornea
                | Self::SourceCharacter(_)
                | Self::OpticLens
                | Self::SingleLayer
                | Self::Staff3Layer
                | Self::Crystal3Layer(_)
                | Self::Unlit3Layer { .. }
                | Self::Feather3Layer
                | Self::FourLayers { .. }
                | Self::Camo3Layer
                | Self::Character { .. }
                | Self::ClothReveal(_)
                | Self::ClanTag
                | Self::TransparentPhong { .. }
                | Self::Flicker
                | Self::PhongEmissive { .. }
                | Self::TileBlend { .. }
        )
    }

    fn view_constants(self) -> bool {
        if matches!(self, Self::Effect(_)) {
            return true;
        }
        matches!(self, Self::SourceLit(_))
            || self.scripted_vertex()
            || matches!(
                self,
                Self::MappedOcclusion {
                    hero_lighting: true,
                    ..
                }
            )
    }

    fn script_inputs(self) -> bool {
        if let Self::Effect(program) = self {
            return program.scripts();
        }
        matches!(
            self,
            Self::SourceCharacter(_)
                | Self::SingleLayer
                | Self::Staff3Layer
                | Self::Crystal3Layer(_)
                | Self::Unlit3Layer { .. }
                | Self::Feather3Layer
                | Self::FourLayers { .. }
                | Self::PhongEmissive {
                    program: EmissiveProgram::ScriptedGlow | EmissiveProgram::ScriptMulti { .. },
                    ..
                }
                | Self::Character { .. }
                | Self::ClothReveal(_)
                | Self::ZombieEyes
                | Self::Flicker
        )
    }

    fn depth(self) -> bool {
        if let Self::Effect(program) = self {
            return program.depth();
        }
        matches!(
            self,
            Self::ProjectedDecal | Self::Feather3Layer | Self::SmokePillar
        )
    }

    fn atlas(self) -> bool {
        if let Self::Effect(program) = self {
            return program.atlas_owner();
        }
        !matches!(
            self,
            Self::Objective
                | Self::Reticle(_)
                | Self::Sonar
                | Self::ZombieEyes
                | Self::Unlit(_)
                | Self::DecalMultiply
                | Self::DecalAdd
                | Self::Feather3Layer
                | Self::SmokePillar
        )
    }

    fn atlas_texture(self) -> bool {
        if let Self::Effect(program) = self {
            return program.atlas_texture();
        }
        !matches!(
            self,
            Self::Objective
                | Self::Reticle(_)
                | Self::Sonar
                | Self::ZombieEyes
                | Self::Canopy(_)
                | Self::SmokePillar
                | Self::SingleLayer
                | Self::Staff3Layer
                | Self::Crystal3Layer(_)
                | Self::Unlit3Layer { .. }
                | Self::Feather3Layer
                | Self::Camo3Layer
                | Self::Unlit(_)
                | Self::DecalMultiply
                | Self::DecalAdd
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
    colors_index: usize,
    visibility: u8,
}

impl SyncComponent<RenderApp> for NativeModelSurface {
    type Target = ExtractedNativeModelSurface;
}

fn extract_model_surfaces(
    mut commands: Commands,
    surfaces: Extract<
        Query<
            (RenderEntity, &NativeModelSurface),
            Or<(Changed<NativeModelSurface>, Changed<RenderEntity>)>,
        >,
    >,
    entities: Extract<Query<RenderEntity>>,
    mut previous_len: Local<usize>,
) {
    if surfaces.is_empty() {
        return;
    }
    let mut extracted = Vec::with_capacity(*previous_len);
    for (entity, surface) in &surfaces {
        let geometry = match &surface.geometry {
            ModelGeometry::Static(handle) => ExtractedModelGeometry::Static(handle.id()),
            ModelGeometry::Animated {
                model,
                instance,
                lod,
                surface,
            } => {
                let Ok(instance) = entities.get(*instance) else {
                    commands
                        .entity(entity)
                        .remove::<ExtractedNativeModelSurface>();
                    continue;
                };
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
                ExtractComponentPlugin::<DynamicLightingOrigin>::default(),
                RenderAssetPlugin::<PreparedModelMaterial, GpuImage>::default(),
            ));
        load_shader_library!(app, "native_model.wesl");
        load_shader_library!(app, "native_model_variants.wesl");
        load_shader_library!(app, "native_source_lit_model.wesl");
        load_shader_library!(app, "native_scripted_model.wesl");
        load_shader_library!(app, "native_character_model.wesl");
        load_shader_library!(app, "native_weapon_model_staff.wesl");
        load_shader_library!(app, "native_weapon_model_optic.wesl");
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
        embedded_asset!(
            app,
            "native_effect_model_scrolling_alpha2_distort_z58wq263.wesl"
        );
        embedded_asset!(
            app,
            "native_effect_model_scrolling_alpha2_distort_qwj8139f.wesl"
        );
        embedded_asset!(
            app,
            "native_effect_model_scrolling_falloff_zfeather_69456263.wesl"
        );
        embedded_asset!(app, "native_effect_model_scrolling_falloff_2ff9qe6w.wesl");
        embedded_asset!(app, "native_effect_model_zm_elec_disc_w323fq8j.wesl");
        embedded_asset!(app, "native_effect_model_light_sphere_46j58w7u.wesl");
        embedded_asset!(app, "native_effect_model_portal_e96z3z92.wesl");
        embedded_asset!(app, "native_effect_model_zmb_sky_pillar_on_6821fw2w.wesl");
        embedded_asset!(app, "native_effect_model_zm_sphere_elec_9j108eq6.wesl");
        embedded_asset!(app, "native_effect_model_zm_sphere_elec_535e0f02.wesl");
        embedded_asset!(
            app,
            "native_effect_model_zm_ice_script_static_blizzard_45q67691.wesl"
        );
        embedded_asset!(
            app,
            "native_effect_model_zm_ice_script_static_877zz4j0.wesl"
        );
        embedded_asset!(app, "native_effect_model_zmb_sphere_tendril_j0598qze.wesl");
        embedded_asset!(
            app,
            "native_effect_model_water_on_surface_reveal_77320z68.wesl"
        );
        embedded_asset!(app, "native_effect_model_unlit_scroll_jq69qew7.wesl");
        let effect_shaders = [
            load_embedded_asset!(
                app,
                "native_effect_model_scrolling_alpha2_distort_z58wq263.wesl"
            ),
            load_embedded_asset!(
                app,
                "native_effect_model_scrolling_alpha2_distort_qwj8139f.wesl"
            ),
            load_embedded_asset!(
                app,
                "native_effect_model_scrolling_falloff_zfeather_69456263.wesl"
            ),
            load_embedded_asset!(app, "native_effect_model_scrolling_falloff_2ff9qe6w.wesl"),
            load_embedded_asset!(app, "native_effect_model_zm_elec_disc_w323fq8j.wesl"),
            load_embedded_asset!(app, "native_effect_model_light_sphere_46j58w7u.wesl"),
            load_embedded_asset!(app, "native_effect_model_portal_e96z3z92.wesl"),
            load_embedded_asset!(app, "native_effect_model_zmb_sky_pillar_on_6821fw2w.wesl"),
            load_embedded_asset!(app, "native_effect_model_zm_sphere_elec_9j108eq6.wesl"),
            load_embedded_asset!(app, "native_effect_model_zm_sphere_elec_535e0f02.wesl"),
            load_embedded_asset!(
                app,
                "native_effect_model_zm_ice_script_static_blizzard_45q67691.wesl"
            ),
            load_embedded_asset!(
                app,
                "native_effect_model_zm_ice_script_static_877zz4j0.wesl"
            ),
            load_embedded_asset!(app, "native_effect_model_zmb_sphere_tendril_j0598qze.wesl"),
            load_embedded_asset!(
                app,
                "native_effect_model_water_on_surface_reveal_77320z68.wesl"
            ),
            load_embedded_asset!(app, "native_effect_model_unlit_scroll_jq69qew7.wesl"),
        ];

        load_shader_library!(app, "native_weapon_model_code.wesl");
        load_shader_library!(app, "native_model_draw.wesl");
        embedded_asset!(app, "native_objective_model_entry.wesl");
        let objective_shader = load_embedded_asset!(app, "native_objective_model_entry.wesl");
        embedded_asset!(app, "native_projected_model_entry.wesl");
        let projected_shader = load_embedded_asset!(app, "native_projected_model_entry.wesl");
        embedded_asset!(app, "native_weapon_model_reticle_entry.wesl");
        let reticle_shader = load_embedded_asset!(app, "native_weapon_model_reticle_entry.wesl");
        embedded_asset!(app, "native_weapon_model_sonar_entry.wesl");
        let sonar_shader = load_embedded_asset!(app, "native_weapon_model_sonar_entry.wesl");
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
                reticle_shader,
                sonar_shader,
                objective_shader,
                projected_shader,
                effect_shaders,
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
                    prepare_model_groups
                        .after(super::native_rangefinder_ui::RangefinderUiSystems::Prepare)
                        .after(super::native_sonar::PrepareSonarViews)
                        .in_set(RenderSystems::PrepareBindGroups),
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
    reticle_shader: Handle<Shader>,
    sonar_shader: Handle<Shader>,
    objective_shader: Handle<Shader>,
    projected_shader: Handle<Shader>,
    effect_shaders: [Handle<Shader>; 15],
    effect_layouts: [BindGroupLayoutDescriptor; 15],
    unlit_scale_layout: BindGroupLayoutDescriptor,
    tile_layouts: [BindGroupLayoutDescriptor; 2],
    scene_layout: BindGroupLayoutDescriptor,
    scripted_scene_layout: BindGroupLayoutDescriptor,
    object_layout: BindGroupLayoutDescriptor,
    draw_matrix_object_layout: BindGroupLayoutDescriptor,
    reticle_material_layout: BindGroupLayoutDescriptor,
    ui3d_material_layout: BindGroupLayoutDescriptor,
    ui3d_view_layout: BindGroupLayoutDescriptor,
    sonar_material_layout: BindGroupLayoutDescriptor,
    sonar_view_layout: BindGroupLayoutDescriptor,
    scripted_object_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    diffuse_material_layout: BindGroupLayoutDescriptor,
    detail_material_layout: BindGroupLayoutDescriptor,
    geometric_material_layout: BindGroupLayoutDescriptor,
    geometric_specular_material_layout: BindGroupLayoutDescriptor,
    source_lit_layouts: [BindGroupLayoutDescriptor; 3],
    source_character_layouts: [BindGroupLayoutDescriptor; 4],
    cornea_layout: BindGroupLayoutDescriptor,
    objective_layout: BindGroupLayoutDescriptor,
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
    staff_material_layout: BindGroupLayoutDescriptor,
    optics_material_layout: BindGroupLayoutDescriptor,
    single_layer_layout: BindGroupLayoutDescriptor,
    disc_material_layout: BindGroupLayoutDescriptor,
    cloth_material_layout: BindGroupLayoutDescriptor,
    cloth_reveal_layout: BindGroupLayoutDescriptor,
    phong_controls_layout: BindGroupLayoutDescriptor,
    clan_tag_layout: BindGroupLayoutDescriptor,
    flicker_layout: BindGroupLayoutDescriptor,
    lighting_layout: BindGroupLayoutDescriptor,
}

impl ModelPipeline {
    fn new(
        shader: Handle<Shader>,
        scripted_shader: Handle<Shader>,
        tile_shader: Handle<Shader>,
        unlit_shader: Handle<Shader>,
        plinth_shader: Handle<Shader>,
        reticle_shader: Handle<Shader>,
        sonar_shader: Handle<Shader>,
        objective_shader: Handle<Shader>,
        projected_shader: Handle<Shader>,
        effect_shaders: [Handle<Shader>; 15],
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
        let mut staff_entries = camo_entries.clone();
        staff_entries.retain(|entry| entry.binding != 14);
        staff_entries.push(uniform_buffer::<Vec4>(false).build(23, ShaderStages::FRAGMENT));
        let cloth_entries: Vec<_> = textures
            .iter()
            .copied()
            .filter(|entry| entry.binding != 6 && entry.binding != 7)
            .collect();
        let mut disc_entries = textures.to_vec();
        disc_entries.push(
            uniform_buffer::<NativeModelEmissiveConstants>(false).build(13, ShaderStages::FRAGMENT),
        );
        let mut optics_entries = textures.to_vec();
        optics_entries.push(uniform_buffer::<Vec4>(false).build(24, ShaderStages::FRAGMENT));
        let mut single_entries: Vec<_> = textures
            .iter()
            .copied()
            .filter(|entry| matches!(entry.binding, 0 | 1 | 8 | 9))
            .collect();
        single_entries.push(
            uniform_buffer::<NativeModelSingleLayerConstants>(false)
                .build(25, ShaderStages::VERTEX_FRAGMENT),
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
        let effect_layouts = NativeEffectModelProgram::ALL.map(|program| {
            let mut entries = Vec::new();
            for index in 0..program.texture_count() as u32 {
                entries.push(
                    texture_2d(TextureSampleType::Float { filterable: true })
                        .build(index * 2, ShaderStages::FRAGMENT),
                );
                entries.push(
                    sampler(SamplerBindingType::Filtering)
                        .build(index * 2 + 1, ShaderStages::FRAGMENT),
                );
            }
            if program.probe() {
                entries.push(
                    texture_cube(TextureSampleType::Float { filterable: true })
                        .build(28, ShaderStages::FRAGMENT),
                );
                entries
                    .push(sampler(SamplerBindingType::Filtering).build(29, ShaderStages::FRAGMENT));
            }
            entries.push(
                uniform_buffer::<NativeEffectModelConstants>(false)
                    .build(31, ShaderStages::VERTEX_FRAGMENT),
            );
            BindGroupLayoutDescriptor::new("original source effect-model inputs", &entries)
        });
        Self {
            effect_shaders,
            effect_layouts,
            shader,
            scripted_shader,
            tile_shader,
            unlit_shader,
            plinth_shader,
            reticle_shader,
            sonar_shader,
            objective_shader,
            projected_shader,
            unlit_scale_layout: BindGroupLayoutDescriptor::new(
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
            draw_matrix_object_layout: BindGroupLayoutDescriptor::new(
                "native weapon model source view",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        (0, uniform_buffer::<NativeModelObject>(true)),
                        (2, uniform_buffer::<NativeModelDrawMatrices>(true)),
                    ),
                ),
            ),
            sonar_material_layout: BindGroupLayoutDescriptor::new(
                "native MMS authored noise and lookup",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::FRAGMENT,
                    (
                        (0, texture_2d(TextureSampleType::Float { filterable: true })),
                        (1, sampler(SamplerBindingType::Filtering)),
                        (2, texture_2d(TextureSampleType::Float { filterable: true })),
                        (3, sampler(SamplerBindingType::Filtering)),
                        (26, uniform_buffer::<NativeSonarConstants>(false)),
                    ),
                ),
            ),
            sonar_view_layout: BindGroupLayoutDescriptor::new(
                "native MMS scene and generated sonar depth",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: false }),
                        sampler(SamplerBindingType::NonFiltering),
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        bevy::render::render_resource::binding_types::texture_2d(
                            TextureSampleType::Depth,
                        ),
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<NativeSonarCodeView>(false),
                    ),
                ),
            ),
            ui3d_material_layout: BindGroupLayoutDescriptor::new(
                "native UI3D reticle authored noise",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::FRAGMENT,
                    (
                        (0, texture_2d(TextureSampleType::Float { filterable: true })),
                        (1, sampler(SamplerBindingType::Filtering)),
                        (26, uniform_buffer::<NativeReticleConstants>(false)),
                    ),
                ),
            ),
            ui3d_view_layout: BindGroupLayoutDescriptor::new(
                "native UI3D generated window5",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                    ),
                ),
            ),
            reticle_material_layout: BindGroupLayoutDescriptor::new(
                "native reticle material",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        (0, texture_2d(TextureSampleType::Float { filterable: true })),
                        (1, sampler(SamplerBindingType::Filtering)),
                        (2, texture_2d(TextureSampleType::Float { filterable: true })),
                        (3, sampler(SamplerBindingType::Filtering)),
                        (26, uniform_buffer::<NativeReticleConstants>(false)),
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
            objective_layout: BindGroupLayoutDescriptor::new(
                "native objective inputs",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::FRAGMENT,
                    (
                        (0, texture_2d(TextureSampleType::Float { filterable: true })),
                        (1, sampler(SamplerBindingType::Filtering)),
                        (26, uniform_buffer::<NativeSourceCharacterConstants>(false)),
                    ),
                ),
            ),
            cornea_layout: BindGroupLayoutDescriptor::new(
                "native cornea inputs",
                &BindGroupLayoutEntries::with_indices(
                    ShaderStages::VERTEX_FRAGMENT,
                    (
                        (0, texture_2d(TextureSampleType::Float { filterable: true })),
                        (1, sampler(SamplerBindingType::Filtering)),
                        (2, texture_2d(TextureSampleType::Float { filterable: true })),
                        (3, sampler(SamplerBindingType::Filtering)),
                        (26, uniform_buffer::<NativeSourceCharacterConstants>(false)),
                    ),
                ),
            ),
            source_character_layouts: [0, 1, 2, 3].map(|extras| {
                let mut entries = cloth_entries.clone();
                entries.push(
                    uniform_buffer::<NativeSourceCharacterConstants>(false)
                        .build(26, ShaderStages::VERTEX_FRAGMENT),
                );
                for binding in [6, 10, 27].into_iter().take(extras) {
                    entries.push(
                        texture_2d(TextureSampleType::Float { filterable: true })
                            .build(binding, ShaderStages::FRAGMENT),
                    );
                    entries.push(
                        sampler(SamplerBindingType::Filtering)
                            .build(binding + 1, ShaderStages::FRAGMENT),
                    );
                }
                BindGroupLayoutDescriptor::new("native source character material", &entries)
            }),
            source_lit_layouts: [0, 1, 2].map(|kind| {
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
                if kind != 0 {
                    entries.extend_from_slice(&BindGroupLayoutEntries::with_indices(
                        ShaderStages::FRAGMENT,
                        (
                            (2, texture_2d(TextureSampleType::Float { filterable: true })),
                            (3, sampler(SamplerBindingType::Filtering)),
                        ),
                    ));
                }
                if kind == 2 {
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
                BindGroupLayoutDescriptor::new("native source-lit model inputs", &entries)
            }),
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
            optics_material_layout: BindGroupLayoutDescriptor::new(
                "native optic lens",
                &optics_entries,
            ),
            single_layer_layout: BindGroupLayoutDescriptor::new(
                "native single parallax layer",
                &single_entries,
            ),
            staff_material_layout: BindGroupLayoutDescriptor::new(
                "native staff charge layers",
                &staff_entries,
            ),
            camo_material_layout: BindGroupLayoutDescriptor::new(
                "native selected weapon camo original three layers",
                &camo_entries,
            ),
            flicker_layout: {
                let mut entries = textures.to_vec();
                entries.push(
                    uniform_buffer::<NativeFlickerConstants>(false)
                        .build(20, ShaderStages::FRAGMENT),
                );
                BindGroupLayoutDescriptor::new("native emissive flicker inputs", &entries)
            },
            clan_tag_layout: {
                let mut entries = cloth_entries.clone();
                entries.push(
                    uniform_buffer::<NativeClanTagConstants>(false)
                        .build(19, ShaderStages::FRAGMENT),
                );
                BindGroupLayoutDescriptor::new("native clan tag atlas inputs", &entries)
            },
            cloth_reveal_layout: {
                let mut entries = textures.to_vec();
                entries.push(
                    uniform_buffer::<NativeClothRevealConstants>(false)
                        .build(22, ShaderStages::FRAGMENT),
                );
                BindGroupLayoutDescriptor::new("native cloth reveal inputs", &entries)
            },
            phong_controls_layout: {
                let mut entries = cloth_entries.clone();
                entries.push(uniform_buffer::<Vec4>(false).build(21, ShaderStages::FRAGMENT));
                BindGroupLayoutDescriptor::new("native Phong material controls", &entries)
            },
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
            NativeModelFamily::Effect(program) => &self.effect_layouts[program as usize],
            NativeModelFamily::Flag(detail) => &self.flag_layouts[usize::from(detail)],
            NativeModelFamily::Canopy(_) => &self.canopy_layout,
            NativeModelFamily::RevealDiffuse => &self.reveal_layout,
            NativeModelFamily::MappedOcclusion { detail, .. } => {
                &self.mapped_occlusion_layouts[usize::from(detail)]
            }
            NativeModelFamily::FourLayers { .. } => &self.four_layer_layout,
            NativeModelFamily::DetailDiffuse(_) => &self.detail_material_layout,
            NativeModelFamily::Unlit(NativeModelUnlitProgram::Multiply)
            | NativeModelFamily::DecalMultiply
            | NativeModelFamily::DecalAdd => &self.geometric_material_layout,
            NativeModelFamily::Unlit(_) => &self.unlit_scale_layout,
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
            NativeModelFamily::Objective => &self.objective_layout,
            NativeModelFamily::ProjectedDecal => &self.geometric_material_layout,
            NativeModelFamily::Cornea => &self.cornea_layout,
            NativeModelFamily::SourceCharacter(program) => {
                &self.source_character_layouts[program.extra_textures()]
            }
            NativeModelFamily::SourceLit(program) => {
                &self.source_lit_layouts[match program {
                    SourceLitProgram::GeometricHero => 0,
                    SourceLitProgram::ColorDetail
                    | SourceLitProgram::NormalDetail
                    | SourceLitProgram::CutoutNormalDetail => 2,
                    _ => 1,
                }]
            }
            NativeModelFamily::NormalSpecular(_) | NativeModelFamily::AlphaSpecular(_) => {
                &self.specular_material_layout
            }
            NativeModelFamily::SmokePillar => &self.smoke_material_layout,
            NativeModelFamily::Feather3Layer => &self.feather_material_layout,
            NativeModelFamily::Unlit3Layer { .. } => &self.falloff_material_layout,
            NativeModelFamily::Crystal3Layer(_) => &self.crystal_material_layout,
            NativeModelFamily::Sonar => &self.sonar_material_layout,
            NativeModelFamily::Reticle(NativeReticleProgram::Ui3d) => &self.ui3d_material_layout,
            NativeModelFamily::Reticle(_) => &self.reticle_material_layout,
            NativeModelFamily::OpticLens => &self.optics_material_layout,
            NativeModelFamily::SingleLayer => &self.single_layer_layout,
            NativeModelFamily::Staff3Layer => &self.staff_material_layout,
            NativeModelFamily::Camo3Layer => &self.camo_material_layout,
            NativeModelFamily::PhongEmissive { .. } => &self.disc_material_layout,
            NativeModelFamily::ClanTag => &self.clan_tag_layout,
            NativeModelFamily::Flicker => &self.flicker_layout,
            NativeModelFamily::ClothReveal(_) => &self.cloth_reveal_layout,
            NativeModelFamily::Character { skin: false } => &self.cloth_material_layout,
            NativeModelFamily::Character { skin: true }
            | NativeModelFamily::TransparentPhong { .. } => &self.phong_controls_layout,
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
        if key.family == NativeModelFamily::ProjectedDecal {
            attributes.retain(|attribute| attribute.shader_location == 0);
        } else if key.family == NativeModelFamily::Objective {
            attributes.retain(|attribute| matches!(attribute.shader_location, 0 | 2 | 3));
        } else if matches!(
            key.family,
            NativeModelFamily::Unlit(_)
                | NativeModelFamily::DecalMultiply
                | NativeModelFamily::DecalAdd
                | NativeModelFamily::SmokePillar
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
            (NativeModelFamily::Objective, false) => "fragment_objective",
            (NativeModelFamily::Objective, true) => "fragment_objective_srgb",
            (NativeModelFamily::ProjectedDecal, false) => "fragment_projected",
            (NativeModelFamily::ProjectedDecal, true) => "fragment_projected_srgb",
            (NativeModelFamily::Cornea, false) => "fragment_source_cornea",
            (NativeModelFamily::Cornea, true) => "fragment_source_cornea_srgb",
            (
                NativeModelFamily::SourceCharacter(SourceCharacterProgram::Fritz { cutout: false }),
                false,
            ) => "fragment_source_fritz",
            (
                NativeModelFamily::SourceCharacter(SourceCharacterProgram::Fritz { cutout: false }),
                true,
            ) => "fragment_source_fritz_srgb",
            (
                NativeModelFamily::SourceCharacter(SourceCharacterProgram::Fritz { cutout: true }),
                false,
            ) => "fragment_source_fritz_cutout",
            (
                NativeModelFamily::SourceCharacter(SourceCharacterProgram::Fritz { cutout: true }),
                true,
            ) => "fragment_source_fritz_cutout_srgb",
            (NativeModelFamily::Effect(_), false) => "fragment",
            (NativeModelFamily::Effect(_), true) => "fragment_srgb",
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::Ember), false) => {
                "fragment_source_ember"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::Ember), true) => {
                "fragment_source_ember_srgb"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::Glass), false) => {
                "fragment_source_glass"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::Glass), true) => {
                "fragment_source_glass_srgb"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::Hair), false) => {
                "fragment_source_hair"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::Hair), true) => {
                "fragment_source_hair_srgb"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::SkinCutout), false) => {
                "fragment_source_skin_cutout"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::SkinCutout), true) => {
                "fragment_source_skin_cutout_srgb"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::ClothEmissive), false) => {
                "fragment_source_cloth_emission"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::ClothEmissive), true) => {
                "fragment_source_cloth_emission_srgb"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::SkinEmissive), false) => {
                "fragment_source_skin_emission"
            }
            (NativeModelFamily::SourceCharacter(SourceCharacterProgram::SkinEmissive), true) => {
                "fragment_source_skin_emission_srgb"
            }
            (
                NativeModelFamily::SourceCharacter(SourceCharacterProgram::RevealEmissive {
                    cutout: false,
                }),
                false,
            ) => "fragment_source_reveal_emission",
            (
                NativeModelFamily::SourceCharacter(SourceCharacterProgram::RevealEmissive {
                    cutout: false,
                }),
                true,
            ) => "fragment_source_reveal_emission_srgb",
            (
                NativeModelFamily::SourceCharacter(SourceCharacterProgram::RevealEmissive {
                    cutout: true,
                }),
                false,
            ) => "fragment_source_reveal_emission_cutout",
            (
                NativeModelFamily::SourceCharacter(SourceCharacterProgram::RevealEmissive {
                    cutout: true,
                }),
                true,
            ) => "fragment_source_reveal_emission_cutout_srgb",
            (NativeModelFamily::SourceLit(SourceLitProgram::BlendedHero), false) => {
                "fragment_source_blended_hero"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::BlendedHero), true) => {
                "fragment_source_blended_hero_srgb"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::GeometricHero), false) => {
                "fragment_source_geometric_hero"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::GeometricHero), true) => {
                "fragment_source_geometric_hero_srgb"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::NormalHero), false) => {
                "fragment_source_normal_hero"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::NormalHero), true) => {
                "fragment_source_normal_hero_srgb"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::CutoutHero), false) => {
                "fragment_source_cutout_hero"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::CutoutHero), true) => {
                "fragment_source_cutout_hero_srgb"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::ColorDetail), false) => {
                "fragment_source_color_detail"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::ColorDetail), true) => {
                "fragment_source_color_detail_srgb"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::CutoutNormalDetail), false) => {
                "fragment_source_normal_detail_cutout"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::CutoutNormalDetail), true) => {
                "fragment_source_normal_detail_cutout_srgb"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::NormalDetail), false) => {
                "fragment_source_normal_detail"
            }
            (NativeModelFamily::SourceLit(SourceLitProgram::NormalDetail), true) => {
                "fragment_source_normal_detail_srgb"
            }
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
            (
                NativeModelFamily::MappedOcclusion {
                    detail: true,
                    hero_lighting: false,
                },
                false,
            ) => "fragment_weapon_detail",
            (
                NativeModelFamily::MappedOcclusion {
                    detail: true,
                    hero_lighting: false,
                },
                true,
            ) => "fragment_weapon_detail_srgb",
            (NativeModelFamily::MappedOcclusion { detail: false, .. }, false) => {
                "fragment_mapped_occlusion"
            }
            (NativeModelFamily::MappedOcclusion { detail: false, .. }, true) => {
                "fragment_mapped_occlusion_srgb"
            }
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
            (
                NativeModelFamily::NormalSpecular(NormalSpecularProgram::Blended {
                    unattenuated_reflection: false,
                }),
                false,
            ) => "fragment_specular_blended",
            (
                NativeModelFamily::NormalSpecular(NormalSpecularProgram::Blended {
                    unattenuated_reflection: false,
                }),
                true,
            ) => "fragment_specular_blended_srgb",
            (
                NativeModelFamily::NormalSpecular(NormalSpecularProgram::Blended {
                    unattenuated_reflection: true,
                }),
                false,
            ) => "fragment_specular_blended_reflection",
            (
                NativeModelFamily::NormalSpecular(NormalSpecularProgram::Blended {
                    unattenuated_reflection: true,
                }),
                true,
            ) => "fragment_specular_blended_reflection_srgb",
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
            (NativeModelFamily::Reticle(NativeReticleProgram::Dynamic), false) => {
                "fragment_reticle_dynamic"
            }
            (NativeModelFamily::Reticle(NativeReticleProgram::Dynamic), true) => {
                "fragment_reticle_dynamic_srgb"
            }
            (NativeModelFamily::Reticle(NativeReticleProgram::DynamicTwoColor), false) => {
                "fragment_reticle_two_color"
            }
            (NativeModelFamily::Reticle(NativeReticleProgram::DynamicTwoColor), true) => {
                "fragment_reticle_two_color_srgb"
            }
            (NativeModelFamily::Sonar, false) => "fragment_sonar",
            (NativeModelFamily::Sonar, true) => "fragment_sonar_srgb",
            (NativeModelFamily::Reticle(NativeReticleProgram::Ui3d), false) => {
                "fragment_reticle_ui3d"
            }
            (NativeModelFamily::Reticle(NativeReticleProgram::Ui3d), true) => {
                "fragment_reticle_ui3d_srgb"
            }
            (NativeModelFamily::Reticle(NativeReticleProgram::Static), false) => {
                "fragment_reticle_static"
            }
            (NativeModelFamily::Reticle(NativeReticleProgram::Static), true) => {
                "fragment_reticle_static_srgb"
            }
            (NativeModelFamily::OpticLens, false) => "fragment_optic",
            (NativeModelFamily::OpticLens, true) => "fragment_optic_srgb",
            (NativeModelFamily::SingleLayer, false) => "fragment_single",
            (NativeModelFamily::SingleLayer, true) => "fragment_single_srgb",
            (NativeModelFamily::Staff3Layer, false) => "fragment_staff",
            (NativeModelFamily::Staff3Layer, true) => "fragment_staff_srgb",
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
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::Replace), false) => {
                "fragment_replace"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::Replace), true) => {
                "fragment_replace_srgb"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::Multiply), false) => {
                "fragment_multiply"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::Multiply), true) => {
                "fragment_multiply_srgb"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::Blend), false) => "fragment_blend",
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::Blend), true) => {
                "fragment_blend_srgb"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::ViewmodelAdd), false) => {
                "fragment_viewmodel_add"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::ViewmodelAdd), true) => {
                "fragment_viewmodel_add_srgb"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::ViewmodelSqrtAlpha), false) => {
                "fragment_viewmodel_sqrt_alpha"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::ViewmodelSqrtAlpha), true) => {
                "fragment_viewmodel_sqrt_alpha_srgb"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::ViewmodelAlpha), false) => {
                "fragment_viewmodel_alpha"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::ViewmodelAlpha), true) => {
                "fragment_viewmodel_alpha_srgb"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::ViewmodelOpaque), false) => {
                "fragment_viewmodel_opaque"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::ViewmodelOpaque), true) => {
                "fragment_viewmodel_opaque_srgb"
            }
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::Add), false) => "fragment_add",
            (NativeModelFamily::Unlit(NativeModelUnlitProgram::Add), true) => "fragment_add_srgb",
            (NativeModelFamily::DecalMultiply, false) => "fragment_decal_multiply",
            (NativeModelFamily::DecalMultiply, true) => "fragment_decal_multiply_srgb",
            (NativeModelFamily::DecalAdd, false) => "fragment_decal_add",
            (NativeModelFamily::DecalAdd, true) => "fragment_decal_add_srgb",
            (NativeModelFamily::DetailDiffuse(_), false) => "fragment_detail_diffuse",
            (NativeModelFamily::DetailDiffuse(_), true) => "fragment_detail_diffuse_srgb",
            (NativeModelFamily::FourLayers { fog: false, .. }, false) => "fragment_four_nofog",
            (NativeModelFamily::FourLayers { fog: false, .. }, true) => "fragment_four_nofog_srgb",
            (NativeModelFamily::FourLayers { fog: true, .. }, false) => "fragment_four_fog",
            (NativeModelFamily::FourLayers { fog: true, .. }, true) => "fragment_four_fog_srgb",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::ScriptMulti { alpha_test: true },
                    ..
                },
                false,
            ) => "fragment_emissive_multi",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::ScriptMulti { alpha_test: true },
                    ..
                },
                true,
            ) => "fragment_emissive_multi_srgb",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::ScriptMulti { alpha_test: false },
                    ..
                },
                false,
            ) => "fragment_emissive_multi_opaque",
            (
                NativeModelFamily::PhongEmissive {
                    program: EmissiveProgram::ScriptMulti { alpha_test: false },
                    ..
                },
                true,
            ) => "fragment_emissive_multi_opaque_srgb",
            (NativeModelFamily::ClanTag, false) => "fragment_clan_tag",
            (NativeModelFamily::Flicker, false) => "fragment_flicker",
            (NativeModelFamily::Flicker, true) => "fragment_flicker_srgb",
            (NativeModelFamily::ClanTag, true) => "fragment_clan_tag_srgb",
            (NativeModelFamily::ClothReveal(false), false) => "fragment_cloth_reveal",
            (NativeModelFamily::ClothReveal(false), true) => "fragment_cloth_reveal_srgb",
            (NativeModelFamily::ClothReveal(true), false) => "fragment_cloth_reveal_cutout",
            (NativeModelFamily::ClothReveal(true), true) => "fragment_cloth_reveal_cutout_srgb",
            (NativeModelFamily::Character { skin: false }, false) => "fragment_cloth",
            (NativeModelFamily::Character { skin: false }, true) => "fragment_cloth_srgb",
            (NativeModelFamily::Character { skin: true }, false) => "fragment_skin",
            (NativeModelFamily::Character { skin: true }, true) => "fragment_skin_srgb",
            (NativeModelFamily::TransparentPhong { player_icon: false }, false) => {
                "fragment_viewmodel_transparent"
            }
            (NativeModelFamily::TransparentPhong { player_icon: false }, true) => {
                "fragment_viewmodel_transparent_srgb"
            }
            (
                NativeModelFamily::MappedOcclusion {
                    detail: true,
                    hero_lighting: true,
                },
                false,
            ) => "fragment_weapon_detail_hero",
            (
                NativeModelFamily::MappedOcclusion {
                    detail: true,
                    hero_lighting: true,
                },
                true,
            ) => "fragment_weapon_detail_hero_srgb",
            (NativeModelFamily::TransparentPhong { player_icon: true }, false) => {
                "fragment_player_icon"
            }
            (NativeModelFamily::TransparentPhong { player_icon: true }, true) => {
                "fragment_player_icon_srgb"
            }
            (NativeModelFamily::ZombieEyes, false) => "fragment_eyes",
            (NativeModelFamily::ZombieEyes, true) => "fragment_eyes_srgb",
        };
        let shader = if key.family == NativeModelFamily::Objective {
            &self.objective_shader
        } else if key.family == NativeModelFamily::ProjectedDecal {
            &self.projected_shader
        } else if key.family == NativeModelFamily::Sonar {
            &self.sonar_shader
        } else if matches!(key.family, NativeModelFamily::Reticle(_)) {
            &self.reticle_shader
        } else if let NativeModelFamily::Effect(program) = key.family {
            &self.effect_shaders[program as usize]
        } else if key.family.depth() {
            &self.plinth_shader
        } else if matches!(
            key.family,
            NativeModelFamily::Unlit(_)
                | NativeModelFamily::DecalMultiply
                | NativeModelFamily::DecalAdd
        ) {
            &self.unlit_shader
        } else if matches!(key.family, NativeModelFamily::TileBlend { .. }) {
            &self.tile_shader
        } else if key.family.scripted_vertex() || key.family == NativeModelFamily::ZombieEyes {
            &self.scripted_shader
        } else {
            &self.shader
        };
        let mut layout = vec![
            if key.family == NativeModelFamily::ProjectedDecal {
                effect_scene_layout(key.depth_filtering)
            } else if key.family.view_constants() {
                self.scripted_scene_layout.clone()
            } else {
                self.scene_layout.clone()
            },
            if key.family.draw_matrices() {
                self.draw_matrix_object_layout.clone()
            } else if key.family.script_inputs() {
                self.scripted_object_layout.clone()
            } else {
                self.object_layout.clone()
            },
            self.material_layout(key.family).clone(),
        ];
        if key.family == NativeModelFamily::Sonar {
            layout.push(self.sonar_view_layout.clone());
        } else if key.family == NativeModelFamily::Reticle(NativeReticleProgram::Ui3d) {
            layout.push(self.ui3d_view_layout.clone());
        } else if key.family.atlas_texture() {
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
                    if key.family == NativeModelFamily::Objective {
                        "vertex_objective"
                    } else if key.family == NativeModelFamily::ProjectedDecal {
                        "vertex_projected"
                    } else if key.family == NativeModelFamily::Cornea {
                        "vertex_source_cornea"
                    } else if matches!(key.family, NativeModelFamily::Effect(_)) {
                        "vertex"
                    } else if key.family == NativeModelFamily::Sonar {
                        "vertex_sonar"
                    } else if let NativeModelFamily::Reticle(program) = key.family {
                        if matches!(
                            program,
                            NativeReticleProgram::Static | NativeReticleProgram::Ui3d
                        ) {
                            "vertex_reticle_static"
                        } else {
                            "vertex_reticle_dynamic"
                        }
                    } else if key.family
                        == NativeModelFamily::SourceCharacter(SourceCharacterProgram::Ember)
                    {
                        "vertex_source_ember"
                    } else if key.family
                        == NativeModelFamily::SourceCharacter(SourceCharacterProgram::Glass)
                    {
                        "vertex_source_glass"
                    } else if key.family == NativeModelFamily::SmokePillar {
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
                    } else if key.family == NativeModelFamily::DecalAdd {
                        "vertex_no_fog"
                    } else if matches!(
                        key.family,
                        NativeModelFamily::Unlit(_) | NativeModelFamily::DecalMultiply
                    ) {
                        "vertex_unlit"
                    } else if key.family == NativeModelFamily::SingleLayer {
                        "vertex_single"
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
                        NativeModelFamily::Staff3Layer
                            | NativeModelFamily::Crystal3Layer(_)
                            | NativeModelFamily::Camo3Layer
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
        for ((binding, handle, descriptor), image) in textures.into_iter().zip(gpu_images) {
            assert!(
                !image.texture_descriptor.format.is_srgb(),
                "native model binding {binding} image {:?} has {:?}; original encoded UNORM texture reads are required",
                handle.path(),
                image.texture_descriptor.format
            );
            assert!(
                descriptor.compare.is_none(),
                "native model material inputs require ordinary source samplers"
            );
            views.push((binding, &image.texture_view));
            samplers.push(device.create_sampler(descriptor));
        }
        let mut effect_uniform = match &source {
            NativeModelMaterial::Effect { constants, .. } => Some(UniformBuffer::from(*constants)),
            _ => None,
        };
        if let Some(uniform) = &mut effect_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut unlit_uniform = match &source {
            NativeModelMaterial::Unlit {
                program,
                scale_rgb: value,
                ..
            } if *program != NativeModelUnlitProgram::Multiply => {
                Some(UniformBuffer::from(NativeUnlitScale { scale_rgb: *value }))
            }
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
            NativeModelMaterial::SourceLit { occlusion, .. } => {
                Some(UniformBuffer::from(NativeModelOcclusion {
                    occlusion_amount: *occlusion,
                }))
            }
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
            | NativeModelMaterial::Camo3Layer { constants, .. }
            | NativeModelMaterial::Staff3Layer { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut parallax_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut weapon_detail_uniform = match &source {
            NativeModelMaterial::SourceLit {
                detail: Some(_),
                detail_parameters,
                ..
            } => Some(UniformBuffer::from(*detail_parameters)),
            NativeModelMaterial::MappedOcclusion {
                detail: Some(detail),
                ..
            } => Some(UniformBuffer::from(detail.parameters)),
            _ => None,
        };
        if let Some(uniform) = &mut weapon_detail_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut source_character_uniform = match &source {
            NativeModelMaterial::SourceCharacter { constants, .. }
            | NativeModelMaterial::Objective { constants, .. }
            | NativeModelMaterial::Cornea { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut source_character_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut cloth_reveal_uniform = match &source {
            NativeModelMaterial::ClothReveal { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut cloth_reveal_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut phong_controls_uniform = match &source {
            NativeModelMaterial::Character {
                skin_controls: Some(controls),
                ..
            }
            | NativeModelMaterial::TransparentPhong { controls, .. } => {
                Some(UniformBuffer::from(*controls))
            }
            _ => None,
        };
        if let Some(uniform) = &mut phong_controls_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut flicker_uniform = match &source {
            NativeModelMaterial::Flicker { constants, .. } => Some(UniformBuffer::from(*constants)),
            _ => None,
        };
        if let Some(uniform) = &mut flicker_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut clan_tag_uniform = match &source {
            NativeModelMaterial::ClanTag { constants, .. } => Some(UniformBuffer::from(*constants)),
            _ => None,
        };
        if let Some(uniform) = &mut clan_tag_uniform {
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
        let mut sonar_uniform = match &source {
            NativeModelMaterial::Sonar { constants, .. } => Some(UniformBuffer::from(*constants)),
            _ => None,
        };
        if let Some(uniform) = &mut sonar_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut reticle_uniform = match &source {
            NativeModelMaterial::Reticle { constants, .. }
            | NativeModelMaterial::Ui3dReticle { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut reticle_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut optics_uniform = match &source {
            NativeModelMaterial::OpticLens { controls, .. } => Some(UniformBuffer::from(*controls)),
            _ => None,
        };
        if let Some(uniform) = &mut optics_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut single_uniform = match &source {
            NativeModelMaterial::SingleLayer { constants, .. } => {
                Some(UniformBuffer::from(*constants))
            }
            _ => None,
        };
        if let Some(uniform) = &mut single_uniform {
            uniform.write_buffer(device, queue);
        }
        let mut staff_uniform = match &source {
            NativeModelMaterial::Staff3Layer { emission_on, .. } => {
                Some(UniformBuffer::from(*emission_on))
            }
            _ => None,
        };
        if let Some(uniform) = &mut staff_uniform {
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
        if let Some(uniform) = &effect_uniform {
            entries.push(BindGroupEntry {
                binding: 31,
                resource: uniform
                    .binding()
                    .expect("source effect-model constants upload"),
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
        if let Some(uniform) = &sonar_uniform {
            entries.push(BindGroupEntry {
                binding: 26,
                resource: uniform.binding().expect("original MMS lens constants"),
            });
        }
        if let Some(uniform) = &reticle_uniform {
            entries.push(BindGroupEntry {
                binding: 26,
                resource: uniform.binding().expect("original reticle constants"),
            });
        }
        if let Some(uniform) = &optics_uniform {
            entries.push(BindGroupEntry {
                binding: 24,
                resource: uniform.binding().expect("original optic controls"),
            });
        }
        if let Some(uniform) = &single_uniform {
            entries.push(BindGroupEntry {
                binding: 25,
                resource: uniform.binding().expect("original single-layer constants"),
            });
        }
        if let Some(uniform) = &staff_uniform {
            entries.push(BindGroupEntry {
                binding: 23,
                resource: uniform.binding().expect("original staff on emission"),
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
        if let Some(uniform) = &source_character_uniform {
            entries.push(BindGroupEntry {
                binding: 26,
                resource: uniform
                    .binding()
                    .expect("original source character constants"),
            });
        }
        if let Some(uniform) = &cloth_reveal_uniform {
            entries.push(BindGroupEntry {
                binding: 22,
                resource: uniform.binding().expect("original cloth reveal constants"),
            });
        }
        if let Some(uniform) = &phong_controls_uniform {
            entries.push(BindGroupEntry {
                binding: 21,
                resource: uniform.binding().expect("original Phong material controls"),
            });
        }
        if let Some(uniform) = &flicker_uniform {
            entries.push(BindGroupEntry {
                binding: 20,
                resource: uniform.binding().expect("original flicker constants"),
            });
        }
        if let Some(uniform) = &clan_tag_uniform {
            entries.push(BindGroupEntry {
                binding: 19,
                resource: uniform.binding().expect("original clan tag constants"),
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
    draw_matrices: DynamicUniformBuffer<NativeModelDrawMatrices>,
    draw_matrix_offsets: HashMap<(RetainedViewEntity, MainEntity), u32>,
    scene_offsets: HashMap<RetainedViewEntity, u32>,
    scripted_scene_offsets: HashMap<RetainedViewEntity, u32>,
    object_offsets: HashMap<(MainEntity, bool), u32>,
    script_offsets: HashMap<MainEntity, u32>,
}

#[derive(Resource, Default)]
struct ModelScriptOwners(HashMap<MainEntity, NativeModelScriptInputs>);

fn extract_script_owners(
    mut owners: ResMut<ModelScriptOwners>,
    source: Extract<
        Query<
            (Entity, &crate::presentation::cues::CueParameters),
            Changed<crate::presentation::cues::CueParameters>,
        >,
    >,
    mut removed: Extract<RemovedComponents<crate::presentation::cues::CueParameters>>,
) {
    for entity in removed.read() {
        owners.0.remove(&MainEntity::from(entity));
    }
    for (entity, inputs) in &source {
        let vectors = std::array::from_fn(|slot| {
            if inputs.script_available & (1 << slot) != 0 {
                Vec4::from_array(inputs.script_vectors[slot])
            } else {
                Vec4::ZERO
            }
        });
        owners
            .0
            .insert(entity.into(), NativeModelScriptInputs { vectors });
    }
}

#[derive(Resource, Default)]
struct ModelBindGroups {
    scene: Option<(BufferId, BindGroup)>,
    scripted_scene: Option<((BufferId, BufferId), BindGroup)>,
    object: Option<(BufferId, BindGroup)>,
    draw_matrices: Option<((BufferId, BufferId), BindGroup)>,
    scripted_object: Option<((BufferId, BufferId), BindGroup)>,
    lighting: Option<(TextureViewId, BindGroup)>,
    ui3d: Option<(TextureViewId, BindGroup)>,
    sonar: HashMap<RetainedViewEntity, ModelSonarGroup>,
}

fn prepare_model_groups(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<ModelPipeline>,
    mut uniforms: ResMut<ModelUniforms>,
    atlas: Option<Res<ModelLightingAtlas>>,
    mut groups: ResMut<ModelBindGroups>,
    ui3d: Option<Res<super::native_rangefinder_ui::PreparedRangefinderUi>>,
    images: Res<RenderAssets<GpuImage>>,
    sonar_views: Query<(
        &ExtractedView,
        &ViewTarget,
        &NativeFxFrame,
        &ExtractedEffectView,
        Option<&super::native_sonar::PreparedSonarView>,
    )>,
) {
    groups.sonar.retain(|view, _| {
        sonar_views
            .iter()
            .any(|(source, _, _, _, sonar)| source.retained_view_entity == *view && sonar.is_some())
    });
    for (view, target, frame, depth, sonar) in &sonar_views {
        let (Some(sonar), Some(float_z)) = (sonar, images.get(depth.float_z)) else {
            continue;
        };
        assert!(
            !depth.filtering,
            "native MMS floatZ requires source point sampler0x61"
        );
        let size = target.main_texture().size();
        let source = NativeSonarCodeView {
            z_near: frame.z_near,
            target_size: Vec4::new(
                size.width as f32,
                size.height as f32,
                1. / size.width as f32,
                1. / size.height as f32,
            ),
            viewport: sonar.viewport,
        };
        let key = [
            float_z.texture_view.id(),
            sonar.color.id(),
            sonar.depth.id(),
        ];
        if let Some(prepared) = groups.sonar.get_mut(&view.retained_view_entity) {
            if prepared.key == key {
                if prepared.source != source {
                    prepared.source = source;
                    prepared.frame.set(source);
                    prepared.frame.write_buffer(&device, &queue);
                }
                continue;
            }
        }
        let mut frame = UniformBuffer::from(source);
        frame.write_buffer(&device, &queue);
        let float_sampler = device.create_sampler(&depth.sampler);
        let group = device.create_bind_group(
            "native MMS lens generated code inputs",
            &cache.get_bind_group_layout(&pipeline.sonar_view_layout),
            &BindGroupEntries::sequential((
                &float_z.texture_view,
                &float_sampler,
                &sonar.color,
                &sonar.sampler,
                &sonar.depth,
                &sonar.sampler,
                frame.binding().expect("MMS view upload"),
            )),
        );
        groups.sonar.insert(
            view.retained_view_entity,
            ModelSonarGroup {
                key,
                source,
                frame,
                group,
            },
        );
    }
    if let Some(ui3d) = ui3d {
        if groups
            .ui3d
            .as_ref()
            .is_none_or(|(id, _)| *id != ui3d.view.id())
        {
            groups.ui3d = Some((
                ui3d.view.id(),
                device.create_bind_group(
                    "native rangefinder generated UI3D",
                    &cache.get_bind_group_layout(&pipeline.ui3d_view_layout),
                    &BindGroupEntries::sequential((&ui3d.view, &ui3d.sampler)),
                ),
            ));
        }
    } else {
        groups.ui3d = None;
    }
    if uniforms.objects.is_empty() {
        groups.scene = None;
        groups.scripted_scene = None;
        groups.object = None;
        groups.scripted_object = None;
        groups.draw_matrices = None;
        groups.lighting = None;
        return;
    }
    uniforms.scenes.write_buffer(&device, &queue);
    uniforms.objects.write_buffer(&device, &queue);
    uniforms.scripted_scenes.write_buffer(&device, &queue);
    uniforms.scripts.write_buffer(&device, &queue);
    uniforms.draw_matrices.write_buffer(&device, &queue);
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
    if let (Some(objects), Some(draw_matrices)) =
        (uniforms.objects.buffer(), uniforms.draw_matrices.buffer())
    {
        let key = (objects.id(), draw_matrices.id());
        if groups
            .draw_matrices
            .as_ref()
            .is_none_or(|(current, _)| *current != key)
        {
            groups.draw_matrices = Some((
                key,
                device.create_bind_group(
                    "native model source view matrices",
                    &cache.get_bind_group_layout(&pipeline.draw_matrix_object_layout),
                    &BindGroupEntries::with_indices((
                        (0, uniforms.objects.binding().unwrap()),
                        (2, uniforms.draw_matrices.binding().unwrap()),
                    )),
                ),
            ));
        }
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
        ResMut<super::dlss::NativeMotion>,
        Res<RenderAssets<PreparedNativeGeometry>>,
        Res<RenderAssets<PreparedAnimatedModel>>,
        Query<&PreparedSkin>,
        Query<&DynamicLightingOrigin>,
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
    weapon_owners: Res<NativeWeaponOwners>,
    mut uniforms: ResMut<ModelUniforms>,
    mut queued: Local<HashMap<RetainedViewEntity, Vec<MainEntity>>>,
    mut ordered: ResMut<NativeOrderedDraws>,
) {
    let (mut motion, geometries, animated, skins, origins) = geometry_inputs;
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
    uniforms.draw_matrices.clear();
    uniforms.draw_matrix_offsets.clear();
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
            if material.family.view_constants()
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
                    let lighting = if let Some(instance) = surface.geometry.instance() {
                        let Ok(origin) = origins.get(instance) else {
                            continue;
                        };
                        atlas
                            .touch_dynamic_grid(surface.placement, origin.0, frame.0)
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
            if material.family.draw_matrices()
                && !uniforms
                    .draw_matrix_offsets
                    .contains_key(&(view.retained_view_entity, surface.placement))
            {
                let inputs = weapon_owners
                    .0
                    .get(&surface.placement)
                    .copied()
                    .unwrap_or_default();
                let offset = uniforms.draw_matrices.push(&draw_constants(
                    source_object,
                    scene,
                    UVec2::new(
                        target.main_texture().width(),
                        target.main_texture().height(),
                    ),
                    inputs,
                ));
                uniforms
                    .draw_matrix_offsets
                    .insert((view.retained_view_entity, surface.placement), offset);
            }
            let pipeline = variants.specialize(
                &cache,
                &pipeline,
                ModelPipelineKey {
                    target: super::frame::NATIVE_COLOR_FORMAT,
                    samples: msaa.samples(),
                    state: surface.state,
                    family: material.family,
                    depth_filtering: material.family.depth()
                        && effect_view
                            .expect("plinth glow requires source floatZ view")
                            .filtering,
                },
            );
            motion.queue(
                &cache,
                entity,
                source_object,
                vertices,
                indices,
                offset,
                surface.first_index,
                surface.index_count,
                32,
                surface.geometry.instance(),
                surface.state,
                surface.pass == NativeWorldPass::EmissiveOpaque
                    || matches!(
                        material.family,
                        NativeModelFamily::Effect(_)
                            | NativeModelFamily::Flicker
                            | NativeModelFamily::ClanTag
                            | NativeModelFamily::Flag(_)
                            | NativeModelFamily::Canopy(_)
                            | NativeModelFamily::SmokePillar
                            | NativeModelFamily::NormalSpecular(NormalSpecularProgram::Scrolling)
                            | NativeModelFamily::SourceCharacter(
                                SourceCharacterProgram::Ember
                                    | SourceCharacterProgram::RevealEmissive { .. }
                            )
                            | NativeModelFamily::Staff3Layer
                            | NativeModelFamily::Crystal3Layer(_)
                            | NativeModelFamily::Camo3Layer
                    ),
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
    if material.family == NativeModelFamily::ProjectedDecal {
        let Some((depth, frame)) = depth else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(0, &depth.bind_group, &[frame.index()]);
    } else if material.family.view_constants() {
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
    if material.family.draw_matrices() {
        let (Some((_, matrices)), Some(&offset)) = (
            &groups.draw_matrices,
            uniforms
                .draw_matrix_offsets
                .get(&(view.retained_view_entity, surface.placement)),
        ) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(1, matrices, &[object_offset, offset]);
    } else if material.family.script_inputs() {
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
    if material.family == NativeModelFamily::Sonar {
        let Some(sonar) = groups.sonar.get(&view.retained_view_entity) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(3, &sonar.group, &[]);
    } else if material.family == NativeModelFamily::Reticle(NativeReticleProgram::Ui3d) {
        let Some((_, ui3d)) = &groups.ui3d else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(3, ui3d, &[]);
    } else if material.family.atlas_texture() {
        let Some((_, lighting)) = &groups.lighting else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(3, lighting, &[]);
    }
    if material.family.depth() && material.family != NativeModelFamily::ProjectedDecal {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objective_and_projected_model_pipelines_keep_source_resource_contracts() {
        let pipeline = ModelPipeline::new(
            default(),
            default(),
            default(),
            default(),
            default(),
            default(),
            default(),
            default(),
            default(),
            std::array::from_fn(|_| default()),
        );
        let state = NativeWorldState {
            blend: None,
            alpha_test: false,
            depth_compare: CompareFunction::GreaterEqual,
            depth_write: true,
            polygon_offset: 0,
            cull_mode: Some(Face::Back),
            front_face: FrontFace::Cw,
            color_write: ColorWrites::ALL,
        };
        for (family, vertex, fragment, attributes, groups) in [
            (
                NativeModelFamily::Objective,
                "vertex_objective",
                "fragment_objective",
                vec![0, 2, 3],
                3,
            ),
            (
                NativeModelFamily::ProjectedDecal,
                "vertex_projected",
                "fragment_projected",
                vec![0],
                4,
            ),
        ] {
            let descriptor = pipeline.specialize(ModelPipelineKey {
                target: TextureFormat::Rgba8Unorm,
                samples: 1,
                state,
                family,
                depth_filtering: true,
            });
            assert_eq!(descriptor.layout.len(), groups);
            assert_eq!(descriptor.vertex.entry_point.as_deref(), Some(vertex));
            assert_eq!(
                descriptor.fragment.as_ref().unwrap().entry_point.as_deref(),
                Some(fragment)
            );
            assert_eq!(
                descriptor.vertex.buffers[0]
                    .attributes
                    .iter()
                    .map(|a| a.shader_location)
                    .collect::<Vec<_>>(),
                attributes
            );
        }
    }

    use bevy::{ecs::schedule::ScheduleLabel, render::extract_plugin::ExtractPlugin};

    #[test]
    fn model_extraction_retains_surfaces_and_updates_instance_lighting() {
        let mut app = App::new();
        app.add_plugins(ExtractPlugin::<RenderApp>::new(
            |_, _| {},
            Render::base_schedule,
            Render.intern(),
            RenderSystems::ExtractCommands.intern(),
            RenderSystems::PostCleanup.intern(),
        ));
        app.add_plugins((
            SyncComponentPlugin::<NativeModelSurface>::default(),
            ExtractComponentPlugin::<DynamicLightingOrigin>::default(),
        ));
        app.sub_app_mut(RenderApp)
            .init_resource::<ModelScriptOwners>()
            .add_systems(
                ExtractSchedule,
                (extract_model_surfaces, extract_script_owners),
            )
            .update_schedule = Some(Render.intern());

        let owner = app
            .world_mut()
            .spawn((
                DynamicLightingOrigin(Vec3::X),
                crate::presentation::cues::CueParameters {
                    script_vectors: [[2.; 4]; 4],
                    script_available: 1,
                    ..default()
                },
            ))
            .id();
        let surface = NativeModelSurface {
            pass: NativeWorldPass::Opaque,
            sort_key: (0, 0),
            vertex_lighting: None,
            geometry: ModelGeometry::Static(Handle::default()),
            material: Handle::default(),
            state: NativeWorldState {
                blend: None,
                alpha_test: false,
                depth_compare: CompareFunction::GreaterEqual,
                depth_write: true,
                polygon_offset: 0,
                cull_mode: None,
                front_face: FrontFace::Ccw,
                color_write: ColorWrites::ALL,
            },
            primary_byte_offset: 0,
            first_index: 0,
            index_count: 3,
            placement: owner,
            colors_index: 0,
            visibility: 255,
        };
        let static_surface = app.world_mut().spawn(surface.clone()).id();
        let animated_surface = app
            .world_mut()
            .spawn(NativeModelSurface {
                geometry: ModelGeometry::Animated {
                    model: Handle::default(),
                    instance: owner,
                    lod: 0,
                    surface: 0,
                },
                ..surface
            })
            .id();
        app.update();
        let render_owner = app.world().get::<RenderEntity>(owner).unwrap().id();
        let render_static = app
            .world()
            .get::<RenderEntity>(static_surface)
            .unwrap()
            .id();
        let render_animated = app
            .world()
            .get::<RenderEntity>(animated_surface)
            .unwrap()
            .id();
        let ticks = [render_static, render_animated].map(|entity| {
            app.sub_app(RenderApp)
                .world()
                .entity(entity)
                .get_ref::<ExtractedNativeModelSurface>()
                .unwrap()
                .last_changed()
        });
        assert_eq!(
            app.sub_app(RenderApp)
                .world()
                .resource::<ModelScriptOwners>()
                .0[&MainEntity::from(owner)]
                .vectors,
            [Vec4::splat(2.), Vec4::ZERO, Vec4::ZERO, Vec4::ZERO]
        );
        assert_eq!(
            app.sub_app(RenderApp)
                .world()
                .get::<ExtractedNativeModelSurface>(render_animated)
                .unwrap()
                .geometry
                .instance(),
            Some(render_owner)
        );

        app.update();
        app.world_mut()
            .get_mut::<DynamicLightingOrigin>(owner)
            .unwrap()
            .0 = Vec3::Y;
        app.world_mut()
            .get_mut::<crate::presentation::cues::CueParameters>(owner)
            .unwrap()
            .script_available = 2;
        app.update();
        let render_world = app.sub_app(RenderApp).world();
        assert_eq!(
            render_world.resource::<ModelScriptOwners>().0[&MainEntity::from(owner)].vectors,
            [Vec4::ZERO, Vec4::splat(2.), Vec4::ZERO, Vec4::ZERO]
        );
        assert_eq!(
            render_world
                .get::<DynamicLightingOrigin>(render_owner)
                .unwrap()
                .0,
            Vec3::Y
        );
        for (entity, tick) in [render_static, render_animated].into_iter().zip(ticks) {
            assert_eq!(
                render_world
                    .entity(entity)
                    .get_ref::<ExtractedNativeModelSurface>()
                    .unwrap()
                    .last_changed(),
                tick,
                "unchanged surfaces must survive extraction without reinsertion"
            );
        }

        app.world_mut()
            .get_mut::<NativeModelSurface>(static_surface)
            .unwrap()
            .first_index = 3;
        app.world_mut().entity_mut(owner).remove::<(
            DynamicLightingOrigin,
            crate::presentation::cues::CueParameters,
        )>();
        app.update();
        assert!(
            !app.sub_app(RenderApp)
                .world()
                .resource::<ModelScriptOwners>()
                .0
                .contains_key(&MainEntity::from(owner))
        );
        assert_eq!(
            app.sub_app(RenderApp)
                .world()
                .get::<ExtractedNativeModelSurface>(render_static)
                .unwrap()
                .first_index,
            3
        );
        assert!(
            app.sub_app(RenderApp)
                .world()
                .get::<DynamicLightingOrigin>(render_owner)
                .is_none()
        );
        app.world_mut()
            .entity_mut(owner)
            .insert(DynamicLightingOrigin(Vec3::Z));
        app.world_mut()
            .entity_mut(static_surface)
            .remove::<NativeModelSurface>();
        app.update();
        assert_eq!(
            app.sub_app(RenderApp)
                .world()
                .get::<DynamicLightingOrigin>(render_owner)
                .unwrap()
                .0,
            Vec3::Z
        );
        assert!(
            app.sub_app(RenderApp)
                .world()
                .get::<ExtractedNativeModelSurface>(render_static)
                .is_none()
        );
        app.world_mut().despawn(animated_surface);
        app.update();
        assert!(
            app.sub_app(RenderApp)
                .world()
                .get_entity(render_animated)
                .is_err()
        );
    }
}
