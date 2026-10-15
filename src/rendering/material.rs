//! Original T6 material inputs for the world LIT programs ported to WESL.
use std::path::{Path, PathBuf};

use bevy::{
    asset::{AssetPath, RenderAssetUsages},
    image::ImageLoaderSettings,
    prelude::*,
    render::render_resource::{
        AddressMode, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites,
        CompareFunction, Face, FilterMode, FrontFace, MipmapFilterMode, SamplerDescriptor,
    },
};
use serde::Deserialize;

use super::native::{
    NativeUnlitProgram, NativeWorldColorLayerTextures, NativeWorldDiffuseTextures,
    NativeWorldGeometricSpecularTextures, NativeWorldLayerTextures, NativeWorldMaterial,
    NativeWorldSpecularLayerTextures, NativeWorldSpecularTextures, NativeWorldState,
    NativeWorldTextures,
};

#[derive(Deserialize, TypePath)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MaterialDefinition {
    #[serde(rename = "_game")]
    game: String,
    #[serde(rename = "_type")]
    kind: String,
    #[serde(rename = "_version")]
    version: u32,
    technique_set: String,
    camera_region: String,
    game_flags: Vec<String>,
    state_bits_entry: Vec<i8>,
    state_bits: Vec<StateDefinition>,
    textures: Vec<TextureDefinition>,
    constants: Vec<ConstantDefinition>,
    #[serde(skip)]
    path: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StateDefinition {
    src_blend_rgb: String,
    dst_blend_rgb: String,
    src_blend_alpha: String,
    dst_blend_alpha: String,
    blend_op_rgb: String,
    blend_op_alpha: String,
    alpha_test: String,
    cull_face: String,
    color_write_rgb: bool,
    color_write_alpha: bool,
    polymode_line: bool,
    depth_write: bool,
    depth_test: String,
    polygon_offset: String,
    stencil_front: Option<serde_json::Value>,
    stencil_back: Option<serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextureDefinition {
    name: Option<String>,
    name_hash: Option<u32>,
    semantic: String,
    image: String,
    sampler_state: SamplerDefinition,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SamplerDefinition {
    clamp_u: bool,
    clamp_v: bool,
    clamp_w: bool,
    filter: String,
    mip_map: String,
}

#[derive(Deserialize)]
struct ConstantDefinition {
    name: Option<String>,
    #[serde(rename = "nameHash")]
    name_hash: Option<u32>,
    literal: [f32; 4],
}

impl MaterialDefinition {
    pub(crate) fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("Read original material {}: {error}", path.display()))?;
        Self::from_bytes(path, &bytes)
    }

    pub(crate) fn from_bytes(path: &Path, bytes: &[u8]) -> Result<Self, String> {
        let mut material: Self = serde_json::from_slice(bytes)
            .map_err(|error| format!("Parse original material {}: {error}", path.display()))?;
        if material.game != "t6" || material.kind != "material" || material.version != 1 {
            return Err(format!(
                "Material {} requires the original T6 material.v1 export",
                path.display()
            ));
        }
        material.path = path.to_owned();
        Ok(material)
    }

    pub(crate) fn technique_set(&self) -> &str {
        &self.technique_set
    }

    pub(crate) fn casts_shadow(&self) -> bool {
        // Material.info.gameFlags bit 0x40, decoded by the T6 exporter.
        self.game_flags.iter().any(|flag| flag == "CASTS_SHADOW")
    }

    pub(crate) fn world_shadow_only(&self) -> bool {
        if self.technique_set != "wpc_shadowcaster_wj6w5j60"
            || self.camera_region != "none"
            || !self.casts_shadow()
        {
            return false;
        }
        // Technique slot1 is build-shadowmap-depth, not the stencil-tested
        // camera z-prepass in slot0. Only this recovered state is supported.
        let Some(state) = self
            .state_bits_entry
            .get(1)
            .and_then(|&index| usize::try_from(index).ok())
            .and_then(|index| self.state_bits.get(index))
        else {
            return false;
        };
        state.alpha_test == "disabled"
            && state.cull_face == "back"
            && !state.color_write_rgb
            && !state.color_write_alpha
            && !state.polymode_line
            && state.depth_write
            && state.depth_test == "less_equal"
            && state.polygon_offset == "offsetShadowmap"
            && state.blend_op_rgb == "disabled"
            && state.blend_op_alpha == "disabled"
            && state.stencil_front.is_none()
            && state.stencil_back.is_none()
    }

    pub(crate) fn world_debug_only(&self) -> bool {
        // Original wpc_default.techset exposes only UNLIT (2) and SOLID
        // WIREFRAME (32). Its cameraRegion=none has no gameplay draw, depth,
        // or shadow technique. Do not apply this to shadowcaster materials.
        self.technique_set == "wpc_default"
            && self.camera_region == "none"
            && self.state_bits_entry.len() == 36
            && self
                .state_bits_entry
                .iter()
                .enumerate()
                .all(|(slot, &state)| {
                    if matches!(slot, 2 | 32) {
                        state >= 0 && (state as usize) < self.state_bits.len()
                    } else {
                        state == -1
                    }
                })
    }

    pub(crate) fn model(
        &self,
        server: &AssetServer,
        probe: Handle<Image>,
    ) -> Result<(super::native_model::NativeModelMaterial, NativeWorldState), String> {
        use super::native_model::{
            NativeModelColorTexture, NativeModelDiffuseTextures, NativeModelMaterial,
            NativeModelTextures,
        };
        let bind = || {
            let family = self.technique_set();
            if matches!(
                family,
                "mc_sw4_3d_cod7_tile_blend_spec_8w40z341"
                    | "mlv_sw4_3d_cod7_tile_blend_spec_w8wezqe6"
                    | "mc_sw4_3d_cod7_tile_blend_q2q2zej6"
                    | "mlv_sw4_3d_cod7_tile_blend_26zq1w22"
            ) {
                use super::native_model::{NativeModelTileConstants, NativeModelTileSpecular};
                let state = self.lit_state()?;
                let textures = [
                    self.texture_key(server, "Macro_AO_Mix_Map", "colorMap", Some(3976875907))?,
                    self.texture_key(server, "Macro_NormalMap", "normalMap", Some(329745474))?,
                    self.texture_key(server, "Micro_1_NormalMap", "normalMap", Some(3069501348))?,
                    self.texture_key(server, "Micro_2_NormalMap", "normalMap", Some(3400726183))?,
                    self.texture_key(server, "Micro_1_ColorMap", "colorMap", Some(3580385162))?,
                    self.texture_key(server, "Micro_2_ColorMap", "colorMap", Some(2643103145))?,
                ];
                let specular = if family.contains("tile_blend_spec_") {
                    Some(NativeModelTileSpecular {
                        layers: [
                            self.texture_key(
                                server,
                                "Micro1SpecularAndGloss",
                                "specularMap",
                                Some(830013099),
                            )?,
                            self.texture_key(
                                server,
                                "Micro2SpecularAndGloss",
                                "specularMap",
                                Some(2415611304),
                            )?,
                        ],
                        probe,
                        probe_sampler: code_sampler(MipmapFilterMode::Linear),
                    })
                } else {
                    None
                };
                let scale1 = self.constant_hash("Micro_1_Scale", 404288371)?;
                let scale2 = self.constant_hash("Micro_2_Scale", 2106384112)?;
                let constants = NativeModelTileConstants {
                    scales: Vec4::new(scale1.x, scale1.y, scale2.x, scale2.y),
                    heights_ao: Vec4::new(
                        self.constant_hash("Micro_1_Height", 4005921460)?.x,
                        self.constant_hash("Macro_Height", 523422290)?.x,
                        self.constant_hash("AO_Diffuse_Adj", 2569221807)?.x,
                        self.constant_hash("Micro_2_Height", 4266698135)?.x,
                    ),
                    reflection: if specular.is_some() {
                        self.constant_hash("reflection_amount", 211931156)?
                    } else {
                        Vec4::ZERO
                    },
                };
                return Ok((
                    NativeModelMaterial::TileBlend {
                        textures,
                        specular,
                        constants,
                        vertex_lit: family.starts_with("mlv_"),
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_unlit_3layer_normal_script_q0zez5ju"
                    | "mlv_sw4_3d_unlit_3layer_normal_script_766fz50u"
                    | "mc_sw4_3d_unlit_3layer_nofog_falloff_5z943qe4"
                    | "mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777"
                    | "mc_sw4_3d_unlit_3layer_nofog_falloff_5z909874"
                    | "mc_sw4_3d_unlit_3layer_nofog_qw71w785"
                    | "mlv_sw4_3d_unlit_3layer_nofog_falloff_fjz6ze0u"
                    | "mc_sw4_3d_unlit_3layer_nofog_falloff_zfeather_wq4jwzz6"
                    | "mlv_sw4_3d_unlit_3layer_nofog_falloff_zfeather_z0zw1090"
            ) {
                use super::native_model::{
                    NativeModelParallaxConstants, NativeModelThreeLayerTextures,
                };
                let no_falloff = family == "mc_sw4_3d_unlit_3layer_nofog_qw71w785";
                let falloff = family.contains("_nofog_falloff_");
                let feather = family.contains("_zfeather_");
                let alpha_falloff = family == "mc_sw4_3d_unlit_3layer_nofog_falloff_5z909874";
                let coverage = (falloff || no_falloff)
                    && family != "mc_sw4_3d_unlit_3layer_nofog_falloff_5z943qe4";
                let state = if feather {
                    if self.camera_region != "emissiveTrans" {
                        return Err(format!(
                            "Plinth glow requires source emissiveTrans, got {}",
                            self.camera_region
                        ));
                    }
                    self.state_bits_for(3, "disabled")?
                } else if alpha_falloff || no_falloff {
                    self.world_state("gt0")?
                } else {
                    self.world_state("disabled")?
                };
                let layer3_alpha = if coverage {
                    self.constant("Layer3Alpha")?
                } else {
                    Vec4::ZERO
                };
                let layer1_scale = self.constant("Layer1Scale")?;
                let layer1_offset = self.constant("Layer1Offset")?;
                let layer1_scroll = self.constant("Layer1Scroll")?;
                let layer2_scale = self.constant("Layer2Scale")?;
                let layer2_scroll = self.constant("Layer2Scroll")?;
                let layer2_offset = self.constant("Layer2Offset")?;
                let layer3_scale = self.constant("Layer3Scale")?;
                let layer3_offset = self.constant("Layer3Offset")?;
                let layer3_scroll = self.constant("Layer3Scroll")?;
                let layer1_alpha = self.constant("Layer1Alpha")?;
                let layer2_alpha = self.constant("Layer2Alpha")?;
                let layer1_rotation = self.constant("Layer1Rotation")?;
                let layer2_rotation = self.constant("Layer2Rotation")?;
                let layer3_rotation = self.constant("Layer3Rotation")?;
                let layer3_origin = self.constant("Layer3Origin")?;
                let layer2_origin = self.constant("Layer2Origin")?;
                let layer1_origin = self.constant("Layer1Origin")?;
                let layer1_depth = self.constant("Layer1Depth")?;
                let layer2_depth = self.constant("Layer2Depth")?;
                let layer3_depth = self.constant("Layer3Depth")?;
                let gloss_amount = if feather {
                    Vec4::ZERO
                } else {
                    self.constant_hash("Gloss_Amount", 3432544439)?
                };
                let hdr_amount = self.constant("hdrAmount")?;
                let reflection_amount = if feather {
                    Vec4::ZERO
                } else {
                    self.constant_hash("Reflection_Amount", 211931156)?
                };
                let last = if feather {
                    Vec4::new(self.constant("FadeFactor")?.x, 0., 0., 0.)
                } else if no_falloff {
                    Vec4::ZERO
                } else if falloff {
                    Vec4::new(
                        self.constant("FalloffPower")?.x,
                        self.constant("FalloffBoost")?.x,
                        0.,
                        0.,
                    )
                } else {
                    let warp = self.constant("WarpScale")?;
                    Vec4::new(warp.x, warp.y, self.constant("NormalHeight")?.x, 0.)
                };
                let constants = NativeModelParallaxConstants {
                    source: [
                        Vec4::new(
                            layer1_scale.x,
                            layer1_scale.y,
                            layer1_offset.x,
                            layer1_offset.y,
                        ),
                        Vec4::new(
                            layer1_scroll.x,
                            layer1_scroll.y,
                            layer2_scale.x,
                            layer2_scale.y,
                        ),
                        Vec4::new(
                            layer2_scroll.x,
                            layer2_scroll.y,
                            layer2_offset.x,
                            layer2_offset.y,
                        ),
                        Vec4::new(
                            layer3_scale.x,
                            layer3_scale.y,
                            layer3_offset.x,
                            layer3_offset.y,
                        ),
                        Vec4::new(
                            layer3_scroll.x,
                            layer3_scroll.y,
                            layer1_alpha.x,
                            layer2_alpha.x,
                        ),
                        Vec4::new(layer3_alpha.x, layer1_rotation.x, layer1_rotation.y, 0.),
                        Vec4::new(
                            layer2_rotation.x,
                            layer2_rotation.y,
                            layer3_rotation.x,
                            layer3_rotation.y,
                        ),
                        Vec4::new(
                            layer3_origin.x,
                            layer3_origin.y,
                            layer2_origin.x,
                            layer2_origin.y,
                        ),
                        Vec4::new(
                            layer1_origin.x,
                            layer1_origin.y,
                            layer1_depth.x,
                            layer2_depth.x,
                        ),
                        if feather {
                            Vec4::new(
                                layer3_depth.x,
                                hdr_amount.x,
                                self.constant("FalloffPower")?.x,
                                self.constant("FalloffBoost")?.x,
                            )
                        } else {
                            Vec4::new(
                                layer3_depth.x,
                                gloss_amount.x,
                                hdr_amount.x,
                                reflection_amount.x,
                            )
                        },
                        last,
                    ],
                };
                if feather {
                    return Ok((
                        NativeModelMaterial::Feather3Layer {
                            layers: [
                                self.texture(server, "Layer1Map", "colorMap")?,
                                self.texture(server, "Layer2Map", "colorMap")?,
                                self.texture(server, "Layer3Map", "colorMap")?,
                            ],
                            constants,
                        },
                        state,
                    ));
                }
                if falloff || no_falloff {
                    return Ok((
                        NativeModelMaterial::Unlit3Layer {
                            layers: [
                                self.texture(server, "Layer1Map", "colorMap")?,
                                self.texture(server, "Layer2Map", "colorMap")?,
                                self.texture(server, "Layer3Map", "colorMap")?,
                            ],
                            probe,
                            probe_sampler: code_sampler(MipmapFilterMode::Linear),
                            constants,
                            vertex_lit: family.starts_with("mlv_"),
                            program: if no_falloff {
                                super::native_model::Unlit3LayerProgram::AlphaNoFalloff
                            } else if alpha_falloff {
                                super::native_model::Unlit3LayerProgram::Alpha
                            } else if coverage {
                                super::native_model::Unlit3LayerProgram::Coverage
                            } else {
                                super::native_model::Unlit3LayerProgram::Opaque
                            },
                        },
                        state,
                    ));
                }
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap", "normalMap")?;
                let (layer1_map, layer1_sampler) = self.texture(server, "Layer1Map", "colorMap")?;
                let (layer2_map, layer2_sampler) = self.texture(server, "Layer2Map", "colorMap")?;
                let (layer3_map, layer3_sampler) = self.texture(server, "Layer3Map", "colorMap")?;
                return Ok((
                    NativeModelMaterial::Crystal3Layer {
                        vertex_lit: family.starts_with("mlv_"),
                        constants,
                        textures: NativeModelThreeLayerTextures {
                            normal_map,
                            normal_sampler,
                            layer1_map,
                            layer1_sampler,
                            layer2_map,
                            layer2_sampler,
                            layer3_map,
                            layer3_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "mc_unlit_replace_3j970129" | "mc_unlit_multiply_97923qz1"
            ) {
                let state = self.state_bits_for(3, "disabled")?;
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let scale_rgb = if family == "mc_unlit_replace_3j970129" {
                    Some(self.constant("scaleRGB")?)
                } else {
                    None
                };
                return Ok((
                    NativeModelMaterial::Unlit {
                        base: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        scale_rgb,
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "mc_lit_sm_r0c0d0n0_33ffej1u" | "mlv_lit_sm_r0c0d0n0_w65788j0"
            ) {
                let state = self.lit_state()?;
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap", "normalMap")?;
                let (detail_map, detail_sampler) =
                    self.texture(server, "detailMap", "normalMap")?;
                return Ok((
                    NativeModelMaterial::DetailDiffuse {
                        base: NativeModelDiffuseTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                        },
                        detail_map,
                        detail_sampler,
                        detail_scale: self.constant("detailScale")?,
                        vertex_lit: family.starts_with("mlv_"),
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_unlit_4layer_nofog_eee768jw"
                    | "mlv_sw4_3d_unlit_4layer_nofog_2j0ezz67"
                    | "mc_sw4_3d_unlit_4layer_jw0z1973"
            ) {
                use super::native_model::NativeModelFourLayerConstants;
                let fog = family == "mc_sw4_3d_unlit_4layer_jw0z1973";
                let state = self.world_state("disabled")?;
                let layer1_scale = self.constant("Layer1Scale")?;
                let layer1_offset = self.constant("Layer1Offset")?;
                let layer1_scroll = self.constant("Layer1Scroll")?;
                let layer2_scale = self.constant("Layer2Scale")?;
                let layer2_scroll = self.constant("Layer2Scroll")?;
                let layer2_offset = self.constant("Layer2Offset")?;
                let layer3_scale = self.constant("Layer3Scale")?;
                let layer3_offset = self.constant("Layer3Offset")?;
                let layer3_scroll = self.constant("Layer3Scroll")?;
                let layer1_alpha = self.constant("Layer1Alpha")?;
                let layer2_alpha = self.constant("Layer2Alpha")?;
                let layer3_alpha = self.constant("Layer3Alpha")?;
                let hdr_amount = self.constant("hdrAmount")?;
                let layer1_rotation = self.constant("Layer1Rotation")?;
                let layer2_rotation = self.constant("Layer2Rotation")?;
                let layer3_rotation = self.constant("Layer3Rotation")?;
                let layer3_origin = self.constant("Layer3Origin")?;
                let layer2_origin = self.constant("Layer2Origin")?;
                let layer1_origin = self.constant("Layer1Origin")?;
                let layer1_depth = self.constant("Layer1Depth")?;
                let layer2_depth = self.constant("Layer2Depth")?;
                let layer3_depth = self.constant("Layer3Depth")?;
                let gloss_amount = self.constant_hash("Gloss_Amount", 3432544439)?;
                let reflection_amount = self.constant_hash("Reflection_Amount", 211931156)?;
                let layer4_scale = self.constant("Layer4Scale")?;
                let layer4_offset = self.constant("Layer4Offset")?;
                let layer4_scroll = self.constant("Layer4Scroll")?;
                let layer4_origin = self.constant("Layer4Origin")?;
                let layer4_alpha = if fog {
                    Vec4::ZERO
                } else {
                    self.constant("Layer4Alpha")?
                };
                let layer4_depth = self.constant("Layer4Depth")?;
                let layer4_rotation = self.constant("Layer4Rotation")?;
                let constants = NativeModelFourLayerConstants {
                    source: [
                        Vec4::new(
                            layer1_scale.x,
                            layer1_scale.y,
                            layer1_offset.x,
                            layer1_offset.y,
                        ),
                        Vec4::new(
                            layer1_scroll.x,
                            layer1_scroll.y,
                            layer2_scale.x,
                            layer2_scale.y,
                        ),
                        Vec4::new(
                            layer2_scroll.x,
                            layer2_scroll.y,
                            layer2_offset.x,
                            layer2_offset.y,
                        ),
                        Vec4::new(
                            layer3_scale.x,
                            layer3_scale.y,
                            layer3_offset.x,
                            layer3_offset.y,
                        ),
                        Vec4::new(
                            layer3_scroll.x,
                            layer3_scroll.y,
                            layer1_alpha.x,
                            layer2_alpha.x,
                        ),
                        Vec4::new(
                            layer3_alpha.x,
                            hdr_amount.x,
                            layer1_rotation.x,
                            layer1_rotation.y,
                        ),
                        Vec4::new(
                            layer2_rotation.x,
                            layer2_rotation.y,
                            layer3_rotation.x,
                            layer3_rotation.y,
                        ),
                        Vec4::new(
                            layer3_origin.x,
                            layer3_origin.y,
                            layer2_origin.x,
                            layer2_origin.y,
                        ),
                        Vec4::new(
                            layer1_origin.x,
                            layer1_origin.y,
                            layer1_depth.x,
                            layer2_depth.x,
                        ),
                        Vec4::new(layer3_depth.x, gloss_amount.x, reflection_amount.x, 0.),
                        Vec4::new(
                            layer4_scale.x,
                            layer4_scale.y,
                            layer4_offset.x,
                            layer4_offset.y,
                        ),
                        Vec4::new(
                            layer4_scroll.x,
                            layer4_scroll.y,
                            layer4_origin.x,
                            layer4_origin.y,
                        ),
                        Vec4::new(
                            layer4_alpha.x,
                            layer4_depth.x,
                            layer4_rotation.x,
                            layer4_rotation.y,
                        ),
                    ],
                };
                let layers = [
                    self.texture(server, "Layer1Map", "colorMap")?,
                    self.texture(server, "Layer2Map", "colorMap")?,
                    self.texture(server, "Layer3Map", "colorMap")?,
                    self.texture(server, "Layer4Map", "colorMap")?,
                ];
                return Ok((
                    NativeModelMaterial::FourLayers {
                        layers,
                        probe,
                        probe_sampler: code_sampler(MipmapFilterMode::Linear),
                        constants,
                        vertex_lit: family.starts_with("mlv_"),
                        fog,
                    },
                    state,
                ));
            }
            if family == "mc_sw4_3d_char_cloth_4z8fq5wu" {
                let (color_map, color_sampler) =
                    self.texture_key(server, "Diffuse_Map", "colorMap", Some(4030327853))?;
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "SpecularAndGloss", "specularMap")?;
                return Ok((
                    NativeModelMaterial::CharCloth {
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        specular_map,
                        specular_sampler,
                    },
                    self.lit_state()?,
                ));
            }
            // Shared VSb7d697d7. Emissive tomb uses PS88e8a7be and scriptVector3
            // instead of the Alcatraz pixel program's Emissive_Push constant.
            if matches!(
                family,
                "mc_sw4_3d_phong_emissive_alcatraz_90892wq5"
                    | "mc_sw4_3d_phong_emissive_q0751958"
                    | "mlv_sw4_3d_phong_emissive_alcatraz_w1q1w09e"
                    | "mc_sw4_3d_phong_emissive_alcatraz_8q551153"
                    | "mc_sw4_3d_emissive_tomb_qww536q2"
                    | "mc_sw4_3d_phong_emissive_script_multi_j05zz52w"
            ) {
                use super::native_model::{EmissiveProgram, NativeModelEmissiveConstants};
                let transparent = family == "mc_sw4_3d_phong_emissive_alcatraz_8q551153";
                let scripted_glow = family == "mc_sw4_3d_emissive_tomb_qww536q2";
                let multi = family == "mc_sw4_3d_phong_emissive_script_multi_j05zz52w";
                let state = if multi {
                    self.lit_state_with_alpha("ge128")?
                } else if transparent {
                    self.world_state("gt0")?
                } else {
                    self.lit_state()?
                };
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "SpecularAndGloss", "specularMap")?;
                let (glow_map, glow_sampler) =
                    self.texture_key(server, "Glow_Map", "colorMap", Some(1825069840))?;
                let constants = NativeModelEmissiveConstants {
                    controls: Vec4::new(
                        self.constant("ReflectionAmount")?.x,
                        self.constant("SpecularAmount")?.x,
                        self.constant("hdrAmount")?.x,
                        if multi {
                            self.constant("Threshold")?.x
                        } else {
                            self.constant_hash("Specular_Push", 604090542)?.x
                        },
                    ),
                    gloss_push: Vec4::new(
                        if multi {
                            0.
                        } else {
                            self.constant("Gloss_Push")?.x
                        },
                        if scripted_glow || multi {
                            0.
                        } else {
                            self.constant_hash("Emissive_Push", 3493630810)?.x
                        },
                        0.,
                        0.,
                    ),
                };
                return Ok((
                    NativeModelMaterial::PhongEmissive {
                        program: if multi {
                            EmissiveProgram::ScriptMulti
                        } else if scripted_glow {
                            EmissiveProgram::ScriptedGlow
                        } else if transparent {
                            EmissiveProgram::Transparent
                        } else {
                            EmissiveProgram::Alcatraz
                        },
                        vertex_lit: family == "mlv_sw4_3d_phong_emissive_alcatraz_w1q1w09e",
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        specular_map,
                        specular_sampler,
                        glow_map,
                        glow_sampler,
                        constants,
                    },
                    state,
                ));
            }

            if family == "mc_sw4_3d_zmb_smoke_pillar_cheap_w6w7e738" {
                if self.camera_region != "emissiveTrans" {
                    return Err("Smoke pillar requires original emissiveTrans region".into());
                }
                let alpha_scale = self.constant("AlphaScale")?;
                let color_speed = self.constant("ColorSpeed")?;
                let color_scale = self.constant("ColorScale")?;
                let alpha_speed = self.constant("AlphaSpeed")?;
                let minus_scale = self.constant_hash("ColorScaleMinus", 1621481545)?;
                let minus_speed = self.constant_hash("ColorSpeedMinus", 1638779318)?;
                let mask_scale = self.constant_hash("AlphaMaskScale", 504449656)?;
                let mask_offset = self.constant_hash("AlphaMaskOffset", 3599644333)?;
                return Ok((
                    NativeModelMaterial::SmokePillar {
                        textures: [
                            self.texture_key(server, "AlphaMap", "colorMap", Some(170923560))?,
                            self.texture_key(server, "AlphaMap2", "colorMap", Some(1345510170))?,
                            self.texture_key(
                                server,
                                "AlphaMap2Mask",
                                "colorMap",
                                Some(2515352718),
                            )?,
                            self.texture_key(
                                server,
                                "AlphaMapMinus",
                                "colorMap",
                                Some(2564398276),
                            )?,
                            self.texture(server, "colorMap", "colorMap")?,
                            self.texture_key(
                                server,
                                "ColorMapMinus",
                                "colorMap",
                                Some(2816785293),
                            )?,
                        ],
                        constants: super::native_model::NativeModelSmokeConstants {
                            source: [
                                Vec4::new(
                                    alpha_scale.x,
                                    alpha_scale.y,
                                    self.constant("hdrAmount")?.x,
                                    0.,
                                ),
                                Vec4::new(
                                    color_speed.x,
                                    color_speed.y,
                                    color_scale.x,
                                    color_scale.y,
                                ),
                                Vec4::new(
                                    self.constant("FadeFactor")?.x,
                                    alpha_speed.x,
                                    alpha_speed.y,
                                    0.,
                                ),
                                self.constant("Color")?,
                                Vec4::new(
                                    minus_scale.x,
                                    minus_scale.y,
                                    minus_speed.x,
                                    minus_speed.y,
                                ),
                                Vec4::new(mask_scale.x, mask_scale.y, mask_offset.x, mask_offset.y),
                            ],
                        },
                    },
                    self.state_bits_for(3, "disabled")?,
                ));
            }
            if matches!(
                family,
                "mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj" | "mc_lit_sm_flag_b0c0s0_6406ez95"
            ) {
                let detailed = family == "mc_lit_sm_flag_t0c0d0n0s0_w75eqzzj";
                let state = if detailed {
                    self.lit_state_with_alpha("ge128")?
                } else {
                    self.world_state("gt0")?
                };
                let detail = if detailed {
                    let (normal_map, normal_sampler) =
                        self.texture(server, "normalMap", "normalMap")?;
                    let (detail_map, detail_sampler) =
                        self.texture(server, "detailMap", "normalMap")?;
                    Some(super::native_model::NativeModelFlagDetail {
                        normal_map,
                        normal_sampler,
                        detail_map,
                        detail_sampler,
                        scale: self.constant("detailScale")?,
                    })
                } else {
                    None
                };
                return Ok((
                    NativeModelMaterial::Flag {
                        textures: [
                            self.texture(server, "colorMap", "colorMap")?,
                            self.texture(server, "specularMap", "specularMap")?,
                            (probe, code_sampler(MipmapFilterMode::Linear)),
                        ],
                        detail,
                        occlusion_amount: self.constant("occlusionAmount")?,
                        params: self.constant("flagParams")?,
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "mc_treecanopy_3z378q8f"
                    | "mc_treecanopy_sm_68zw237j"
                    | "mc_treecanopy_sm_j9ff996z"
            ) {
                let quasi = family == "mc_treecanopy_sm_j9ff996z";
                let state = if quasi {
                    if self.camera_region != "litQuasiOpaque" {
                        return Err("Source blended canopy requires litQuasiOpaque".into());
                    }
                    self.state_bits_for(4, "gt0")?
                } else {
                    self.lit_state_with_alpha("ge128")?
                };
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                return Ok((
                    NativeModelMaterial::Canopy {
                        quasi,
                        base: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        constants: super::native_model::NativeModelCanopyConstants {
                            lighting: self.constant("treeCanopyLightingParms")?,
                            tint: self.constant("colorTint")?,
                        },
                    },
                    state,
                ));
            }
            if family == "mc_lit_sm_r0c0n0s0o0_3z86zq2z" {
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap", "normalMap")?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap", "specularMap")?;
                let (occlusion_map, occlusion_sampler) =
                    self.texture(server, "occlusionMap", "occlusionMap")?;
                let (map, sampler) = self.texture(server, "colorDetailMap", "color10Map")?;
                return Ok((
                    NativeModelMaterial::MappedOcclusion {
                        detail: Some(super::native_model::NativeWeaponDetail {
                            map,
                            sampler,
                            parameters: Vec4::ZERO,
                        }),
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        specular_map,
                        specular_sampler,
                        occlusion_map,
                        occlusion_sampler,
                    },
                    self.lit_state()?,
                ));
            }
            if matches!(
                family,
                "mc_lit_sm_b0c0n0s0o0_jf00fe4q" | "mc_lit_sm_b0c0n0v0_3559e8jj"
            ) {
                let state = self.world_state("gt0")?;
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap", "normalMap")?;
                let material = if family == "mc_lit_sm_b0c0n0v0_3559e8jj" {
                    NativeModelMaterial::RevealDiffuse {
                        base: NativeModelDiffuseTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                        },
                        reveal: self.constant("alphaRevealParms")?,
                    }
                } else {
                    let (specular_map, specular_sampler) =
                        self.texture(server, "specularMap", "specularMap")?;
                    let (occlusion_map, occlusion_sampler) =
                        self.texture(server, "occlusionMap", "occlusionMap")?;
                    NativeModelMaterial::MappedOcclusion {
                        detail: None,
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        specular_map,
                        specular_sampler,
                        occlusion_map,
                        occlusion_sampler,
                    }
                };
                return Ok((material, state));
            }
            if matches!(
                family,
                "mc_lit_sm_b0c0_fw0jf955"
                    | "mlv_lit_sm_b0c0_50077370"
                    | "mc_lit_sm_b0c0n0_67fqj7jf"
                    | "mlv_lit_sm_b0c0n0_322zqw73"
            ) {
                let state = self.world_state("gt0")?;
                let vertex_lit = family.starts_with("mlv_");
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let material = if matches!(
                    family,
                    "mc_lit_sm_b0c0_fw0jf955" | "mlv_lit_sm_b0c0_50077370"
                ) {
                    NativeModelMaterial::GeometricDiffuse {
                        base: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        program: super::native_model::GeometricProgram::Blended(vertex_lit),
                    }
                } else {
                    let (normal_map, normal_sampler) =
                        self.texture(server, "normalMap", "normalMap")?;
                    NativeModelMaterial::BlendedDiffuse(
                        NativeModelDiffuseTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                        },
                        vertex_lit,
                    )
                };
                return Ok((material, state));
            }
            if matches!(
                family,
                "mc_lit_sm_r0c0s0_986ezzjq"
                    | "mc_lit_sm_b0c0s0_805w75j2"
                    | "mlv_lit_sm_b0c0s0_41731e52"
            ) {
                let transparent = family != "mc_lit_sm_r0c0s0_986ezzjq";
                let state = if transparent {
                    self.world_state("gt0")?
                } else {
                    self.lit_state()?
                };
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap", "specularMap")?;
                return Ok((
                    NativeModelMaterial::GeometricSpecular {
                        base: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        specular_map,
                        specular_sampler,
                        reflection_probe: probe,
                        reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        occlusion_amount: self.constant("occlusionAmount")?,
                        vertex_lit: family.starts_with("mlv_"),
                        transparent,
                    },
                    state,
                ));
            }
            if !matches!(
                family,
                "mc_lit_sm_r0c0n0x0_q361191u"
                    | "mlv_lit_sm_r0c0n0x0_619829wj"
                    | "mlv_lit_sm_r0c0_j60qqj0e"
                    | "mlv_lit_sm_r0c0_475q06w3"
                    | "mlv_lit_sm_r0c0n0_3qf192ew"
                    | "mlv_lit_sm_r0c0n0s0_wqqj839w"
                    | "mlv_lit_sm_t0c0n0s0_9jfj95q5"
                    | "mc_lit_sm_r0c0n0x0_53144qq1"
                    | "mlv_lit_sm_r0c0n0x0_8f47740f"
                    | "mc_lit_sm_r0c0n0_2z223015"
                    | "mc_lit_sm_t0c0n0_9qf6e4qj"
                    | "mc_lit_sm_r0c0_77e21qq8"
                    | "mc_lit_sm_t0c0_90wz6fe2"
                    | "mc_lit_sm_r0c0_5137qq99"
                    | "mc_lit_sm_r0c0n0s0_zqq1fze7"
                    | "mc_lit_sm_r0c0n0s0_4w3wqfe5"
                    | "mc_lit_sm_t0c0n0s0_47jq982j"
            ) {
                return Err(format!("Port original model LIT techniqueSet {}", family));
            }
            // These original cutout programs implement the exact ge128 fused
            // discard. Match the material's logical state to that program.
            let alpha_test = if matches!(
                family,
                "mc_lit_sm_t0c0n0_9qf6e4qj"
                    | "mlv_lit_sm_t0c0n0s0_9jfj95q5"
                    | "mc_lit_sm_t0c0n0s0_47jq982j"
                    | "mc_lit_sm_t0c0_90wz6fe2"
            ) {
                "ge128"
            } else {
                "disabled"
            };
            let state = self.lit_state_with_alpha(alpha_test)?;
            let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
            if matches!(
                family,
                "mc_lit_sm_r0c0_77e21qq8"
                    | "mc_lit_sm_t0c0_90wz6fe2"
                    | "mc_lit_sm_r0c0_5137qq99"
                    | "mlv_lit_sm_r0c0_j60qqj0e"
                    | "mlv_lit_sm_r0c0_475q06w3"
            ) {
                use super::native_model::GeometricProgram;
                return Ok((
                    NativeModelMaterial::GeometricDiffuse {
                        base: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        program: match family {
                            "mlv_lit_sm_r0c0_j60qqj0e" => GeometricProgram::VertexLit,
                            "mc_lit_sm_t0c0_90wz6fe2" => GeometricProgram::AlphaTest,
                            "mc_lit_sm_r0c0_5137qq99" | "mlv_lit_sm_r0c0_475q06w3" => {
                                GeometricProgram::VertexOcclusion(family.starts_with("mlv_"))
                            }
                            _ => GeometricProgram::Diffuse,
                        },
                    },
                    state,
                ));
            }
            let (normal_map, normal_sampler) = self.texture(server, "normalMap", "normalMap")?;
            if matches!(
                family,
                "mc_lit_sm_r0c0n0_2z223015"
                    | "mc_lit_sm_t0c0n0_9qf6e4qj"
                    | "mlv_lit_sm_r0c0n0_3qf192ew"
            ) {
                let textures = NativeModelDiffuseTextures {
                    color_map,
                    color_sampler,
                    normal_map,
                    normal_sampler,
                };
                return Ok((
                    if family == "mc_lit_sm_t0c0n0_9qf6e4qj" {
                        NativeModelMaterial::AlphaDiffuse(textures)
                    } else {
                        NativeModelMaterial::Diffuse(
                            textures,
                            family == "mlv_lit_sm_r0c0n0_3qf192ew",
                        )
                    },
                    state,
                ));
            }
            let base = NativeModelTextures {
                color_map,
                color_sampler,
                normal_map,
                normal_sampler,
                reflection_probe: probe,
                reflection_sampler: code_sampler(MipmapFilterMode::Linear),
            };
            let material = match family {
                "mc_lit_sm_r0c0n0x0_q361191u" => NativeModelMaterial::ImplicitSpecular(base),
                "mlv_lit_sm_r0c0n0x0_619829wj" => NativeModelMaterial::VertexLitSpecular(base),
                "mc_lit_sm_r0c0n0x0_53144qq1" | "mlv_lit_sm_r0c0n0x0_8f47740f" => {
                    NativeModelMaterial::VertexOcclusion(base, family.starts_with("mlv_"))
                }
                "mc_lit_sm_r0c0n0s0_zqq1fze7"
                | "mc_lit_sm_r0c0n0s0_4w3wqfe5"
                | "mc_lit_sm_t0c0n0s0_47jq982j"
                | "mlv_lit_sm_r0c0n0s0_wqqj839w"
                | "mlv_lit_sm_t0c0n0s0_9jfj95q5" => {
                    let (specular_map, specular_sampler) =
                        self.texture(server, "specularMap", "specularMap")?;
                    let occlusion_amount = self.constant("occlusionAmount")?;
                    if matches!(
                        family,
                        "mc_lit_sm_t0c0n0s0_47jq982j" | "mlv_lit_sm_t0c0n0s0_9jfj95q5"
                    ) {
                        NativeModelMaterial::AlphaSpecular {
                            vertex_lit: family.starts_with("mlv_"),
                            base,
                            specular_map,
                            specular_sampler,
                            occlusion_amount,
                        }
                    } else {
                        NativeModelMaterial::NormalSpecular {
                            program: if family == "mc_lit_sm_r0c0n0s0_4w3wqfe5" {
                                super::native_model::NormalSpecularProgram::Scrolling
                            } else if family.starts_with("mlv_") {
                                super::native_model::NormalSpecularProgram::VertexLit
                            } else {
                                super::native_model::NormalSpecularProgram::Grid
                            },
                            base,
                            specular_map,
                            specular_sampler,
                            occlusion_amount,
                        }
                    }
                }
                _ => unreachable!("selected original model family"),
            };
            Ok((material, state))
        };
        bind().map_err(|error| format!("Material {}: {error}", self.path.display()))
    }

    pub(crate) fn water(
        &self,
        server: &AssetServer,
        lightmap: Handle<Image>,
        probe: Handle<Image>,
    ) -> Result<(super::native_water::NativeWaterMaterial, NativeWorldState), String> {
        use super::native_water::{NativeWaterConstants, NativeWaterMaterial};
        let bind = || {
            if self.technique_set() != "wpc_cod7water_we876z2f" {
                return Err(format!(
                    "Port original water techniqueSet {}",
                    self.technique_set()
                ));
            }
            let state = self.lit_state()?;
            let (normal_map00, normal_sampler00) =
                self.texture_key(server, "normalMap00", "normalMap", Some(449481359))?;
            let (normal_map01, normal_sampler01) =
                self.texture_key(server, "normalMap01", "normalMap", Some(449481358))?;
            Ok((
                NativeWaterMaterial {
                    constants: NativeWaterConstants {
                        normal_position_scale: self
                            .constant_hash("waterNormalPositionScale", 3341333671)?,
                        lo_normal_scroll: self.constant_hash("waterLoNormalScroll", 1855396200)?,
                        hi_normal_scroll: self.constant_hash("waterHiNormalScroll", 27397098)?,
                        normal_scroll_speed: self
                            .constant_hash("waterNormalScrollSpeed", 323802060)?,
                        control0: self.constant_hash("waterControlVar0", 2815841959)?,
                        control1: self.constant_hash("waterControlVar1", 2815841958)?,
                        color_n: self.constant("waterColorN")?,
                        color_s: self.constant("waterColorS")?,
                        normal_scale: self.constant_hash("waterNormalScale", 2251460126)?,
                    },
                    normal_map00,
                    normal_sampler00,
                    normal_map01,
                    normal_sampler01,
                    lightmap_secondary: lightmap,
                    lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                    reflection_probe: probe,
                    reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                },
                state,
            ))
        };
        bind().map_err(|error| format!("Material {}: {error}", self.path.display()))
    }

    pub(crate) fn world(
        &self,
        server: &AssetServer,
        lightmap: Option<Handle<Image>>,
        probe: Handle<Image>,
    ) -> Result<(NativeWorldMaterial, NativeWorldState), String> {
        let bind = || {
            let family = self.technique_set();
            let unlit = match family {
                "wpc_unlit_replace_4688792e" => Some(NativeUnlitProgram::Replace),
                "wpc_unlit_add_8ez3wzw3" => Some(NativeUnlitProgram::Add),
                "wpc_unlitdecalblend_add_j26wq580" => Some(NativeUnlitProgram::DecalAdd),
                "wpc_unlitdecalblend_multiply_35079164" => Some(NativeUnlitProgram::DecalMultiply),
                _ => None,
            };
            if family == "wpc_sw4_3d_unlit_1layer_8691j77q" {
                let state = self.world_state("gt0")?;
                let (color_map, color_sampler) = self.texture(server, "Layer1Map", "colorMap")?;
                return Ok((
                    NativeWorldMaterial::Wallbuy {
                        color_map,
                        color_sampler,
                        constants: self.parallax_layer(1, self.constant("hdrAmount")?.x)?,
                    },
                    state,
                ));
            }
            if family == "wpc_sw4_3d_unlit_3layer_normal_script_q1z1f6w1" {
                let state = self.world_state("disabled")?;
                return Ok((
                    NativeWorldMaterial::Lava {
                        layers: [
                            self.texture(server, "Layer1Map", "colorMap")?,
                            self.texture(server, "Layer2Map", "colorMap")?,
                            self.texture(server, "Layer3Map", "colorMap")?,
                        ],
                        constants: super::native::NativeLavaConstants {
                            layers: [
                                self.parallax_layer(1, self.constant("hdrAmount")?.x)?,
                                self.parallax_layer(2, 0.0)?,
                                self.parallax_layer(3, 0.0)?,
                            ],
                        },
                    },
                    state,
                ));
            }
            if let Some(program) = unlit {
                let region = if matches!(program, NativeUnlitProgram::Replace) {
                    "emissiveOpaque"
                } else {
                    "emissiveTrans"
                };
                if self.camera_region != region {
                    return Err(format!("Expected {region} for {family}"));
                }
                let state = self.state_bits_for(3, "disabled")?;
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let scale_rgb = match program {
                    NativeUnlitProgram::Replace | NativeUnlitProgram::Add => {
                        Some(self.constant("scaleRGB")?)
                    }
                    _ => None,
                };
                return Ok((
                    NativeWorldMaterial::Unlit {
                        program,
                        color_map,
                        color_sampler,
                        scale_rgb,
                    },
                    state,
                ));
            }
            let lightmap = lightmap.ok_or_else(|| {
                format!(
                    "Port the original {family} lighting path; surface has no directional lightmap"
                )
            })?;
            if family == "wpc_sw4_3d_burning_embers_nuketown_74j6971w" {
                let state = self.world_state("gt0")?;
                let (color_map, color_sampler) =
                    self.texture_key(server, "Diffuse_Map", "colorMap", Some(4030327853))?;
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let (super_map, super_sampler) =
                    self.texture_key(server, "Embers_Super_Map", "colorMap", Some(2799170835))?;
                let (ember_map, ember_sampler) =
                    self.texture_key(server, "Ember_Map", "colorMap", Some(3617419774))?;
                let direction = self.constant_hash("Ember_Direction", 4131458919)?;
                return Ok((
                    NativeWorldMaterial::Ember {
                        base: NativeWorldTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            lightmap_secondary: lightmap,
                            lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        super_map,
                        super_sampler,
                        ember_map,
                        ember_sampler,
                        constants: super::native::NativeEmberConstants {
                            scale_direction_max: Vec4::new(
                                self.constant("Ember_Scale")?.x,
                                direction.x,
                                direction.y,
                                self.constant("Flicker_Max")?.x,
                            ),
                            min_seed: Vec4::new(
                                self.constant("Flicker_Min")?.x,
                                self.constant("Seed_Value")?.x,
                                0.0,
                                0.0,
                            ),
                        },
                    },
                    state,
                ));
            }
            if family == "wpc_lit_sm_r0c0x0_5j63fj97" {
                let state = self.world_state("disabled")?;
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                return Ok((
                    NativeWorldMaterial::GeometricGloss {
                        color_map,
                        color_sampler,
                        lightmap_secondary: lightmap,
                        lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                        reflection_probe: probe,
                        reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "wpc_lit_sm_t0c0_2w3887e6" | "wpc_lit_sm_b0c0_6j30z67q"
            ) {
                let state = self.world_state(if family == "wpc_lit_sm_t0c0_2w3887e6" {
                    "ge128"
                } else {
                    "gt0"
                })?;
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                return Ok((
                    NativeWorldMaterial::GeometricDiffuse {
                        color_map,
                        color_sampler,
                        lightmap_secondary: lightmap,
                        lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                    },
                    state,
                ));
            }
            if !matches!(
                family,
                "wpc_lit_sm_r0c0n0x0_f488034e"
                    | "wpc_lit_sm_r0c0n0_80fe30wz"
                    | "wpc_lit_sm_r0c0n0s0_1zzj1138"
                    | "wpc_lit_sm_t0c0n0_9ew0jz24"
                    | "wpc_lit_sm_t0c0n0s0_weef63f3"
                    | "wpc_lit_sm_b0c0n0_19f3588f"
                    | "wpc_lit_sm_b0c0n0v0_ew347fe3"
                    | "wpc_lit_sm_b0c0s0_3f3q946z"
                    | "wpc_lit_sm_b0c0n0s0v0_efz5w56f"
                    | "wpc_lit_sm_r0c0s0_qfqz7850"
                    | "lit_sm_r0c0n0x0_m1c1"
                    | "lit_sm_r0c0n0x0_b1c1_b2c2"
                    | "lit_sm_r0c0n0x0_a1c1_m2c2"
                    | "lit_sm_r0c0n0x0_b1c1"
                    | "lit_sm_r0c0n0x0_a1c1"
                    | "lit_sm_r0c0n0x0_b1c1n1_a2c2"
                    | "lit_sm_r0c0n0x0_t1c1n1"
                    | "lit_sm_r0c0n0x0_b1c1n1"
                    | "lit_sm_r0c0n0x0_b1c1n1v1"
                    | "lit_sm_r0c0n0x0_b1c1n1s1"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1"
                    | "lit_sm_r0c0n0x0_b1c1_b2c2n2"
                    | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2"
                    | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0x0_b1c1n1_b2c2n2"
                    | "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0s0_b1c1n1"
                    | "lit_sm_r0c0n0s0_b1c1n1s1v1"
                    | "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0_b1c1n1s1v1"
                    | "lit_sm_r0c0n0_m1c1_m2c2"
                    | "lit_sm_r0c0n0_m1c1"
                    | "lit_sm_r0c0n0_b1c1n1_m2c2"
                    | "lit_sm_r0c0n0_b1c1n1v1_m2c2"
                    | "lit_sm_r0c0n0_b1c1n1"
                    | "lit_sm_r0c0n0_b1c1n1v1"
                    | "lit_sm_r0c0s0_b1c1"
            ) {
                return Err(format!(
                    "Port the original LIT technique for techniqueSet {family} before binding this world material"
                ));
            }
            let state = self.world_state(match family {
                "wpc_lit_sm_t0c0n0_9ew0jz24" | "wpc_lit_sm_t0c0n0s0_weef63f3" => "ge128",
                "wpc_lit_sm_b0c0n0_19f3588f"
                | "wpc_lit_sm_b0c0n0v0_ew347fe3"
                | "wpc_lit_sm_b0c0s0_3f3q946z"
                | "wpc_lit_sm_b0c0n0s0v0_efz5w56f" => "gt0",
                _ => "disabled",
            })?;
            let reveal = match family {
                "lit_sm_r0c0n0x0_b1c1n1v1"
                | "lit_sm_r0c0n0x0_b1c1n1s1v1"
                | "lit_sm_r0c0n0s0_b1c1n1s1v1"
                | "lit_sm_r0c0n0_b1c1n1s1v1"
                | "lit_sm_r0c0n0_b1c1n1v1" => Some(self.constant("alphaRevealParms1")?),
                _ => None,
            };
            let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
            if matches!(
                family,
                "wpc_lit_sm_r0c0s0_qfqz7850" | "wpc_lit_sm_b0c0s0_3f3q946z" | "lit_sm_r0c0s0_b1c1"
            ) {
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap", "specularMap")?;
                let base = NativeWorldGeometricSpecularTextures {
                    color_map,
                    color_sampler,
                    specular_map,
                    specular_sampler,
                    lightmap_secondary: lightmap,
                    lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                    reflection_probe: probe,
                    reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                };
                let material = if family == "lit_sm_r0c0s0_b1c1" {
                    let (color_map, color_sampler) =
                        self.texture(server, "colorMap1", "colorMap")?;
                    NativeWorldMaterial::GeometricLayered {
                        base,
                        color_map,
                        color_sampler,
                    }
                } else {
                    NativeWorldMaterial::GeometricSpecular(base)
                };
                return Ok((material, state));
            }
            let (normal_map, normal_sampler) = self.texture(server, "normalMap", "normalMap")?;
            if matches!(
                family,
                "wpc_lit_sm_r0c0n0_80fe30wz"
                    | "wpc_lit_sm_b0c0n0_19f3588f"
                    | "wpc_lit_sm_b0c0n0v0_ew347fe3"
                    | "wpc_lit_sm_t0c0n0_9ew0jz24"
                    | "lit_sm_r0c0n0_m1c1_m2c2"
                    | "lit_sm_r0c0n0_m1c1"
                    | "lit_sm_r0c0n0_b1c1n1_m2c2"
                    | "lit_sm_r0c0n0_b1c1n1v1_m2c2"
                    | "lit_sm_r0c0n0_b1c1n1"
                    | "lit_sm_r0c0n0_b1c1n1v1"
            ) {
                let base = NativeWorldDiffuseTextures {
                    color_map,
                    color_sampler,
                    normal_map,
                    normal_sampler,
                    lightmap_secondary: lightmap,
                    lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                };
                if matches!(
                    family,
                    "lit_sm_r0c0n0_b1c1n1_m2c2" | "lit_sm_r0c0n0_b1c1n1v1_m2c2"
                ) {
                    let (color_map, color_sampler) =
                        self.texture(server, "colorMap1", "colorMap")?;
                    let (normal_map, normal_sampler) =
                        self.texture(server, "normalMap1", "normalMap")?;
                    let (color_map2, color_sampler2) =
                        self.texture(server, "colorMap2", "colorMap")?;
                    return Ok((
                        NativeWorldMaterial::DiffuseLayerMultiply {
                            base,
                            layer1: NativeWorldLayerTextures {
                                color_map,
                                color_sampler,
                                normal_map,
                                normal_sampler,
                            },
                            color_map2,
                            color_sampler2,
                            alpha_reveal_parms1: if family.contains("v1") {
                                Some(self.constant("alphaRevealParms1")?)
                            } else {
                                None
                            },
                        },
                        state,
                    ));
                }
                if matches!(family, "lit_sm_r0c0n0_m1c1" | "lit_sm_r0c0n0_m1c1_m2c2") {
                    let (color_map, color_sampler) =
                        self.texture(server, "colorMap1", "colorMap")?;
                    return Ok((
                        NativeWorldMaterial::DiffuseColorMultiply {
                            second_layer: if family == "lit_sm_r0c0n0_m1c1_m2c2" {
                                Some(self.texture(server, "colorMap2", "colorMap")?)
                            } else {
                                None
                            },
                            base,
                            color_map,
                            color_sampler,
                        },
                        state,
                    ));
                }
                if matches!(
                    family,
                    "wpc_lit_sm_r0c0n0_80fe30wz"
                        | "wpc_lit_sm_t0c0n0_9ew0jz24"
                        | "wpc_lit_sm_b0c0n0_19f3588f"
                        | "wpc_lit_sm_b0c0n0v0_ew347fe3"
                ) {
                    if family == "wpc_lit_sm_b0c0n0v0_ew347fe3" {
                        return Ok((
                            NativeWorldMaterial::RevealDiffuse {
                                base,
                                alpha_reveal_parms: self.constant("alphaRevealParms")?,
                            },
                            state,
                        ));
                    }
                    return Ok((NativeWorldMaterial::Diffuse(base), state));
                }
                let (color_map, color_sampler) = self.texture(server, "colorMap1", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap1", "normalMap")?;
                let layer = NativeWorldLayerTextures {
                    color_map,
                    color_sampler,
                    normal_map,
                    normal_sampler,
                };
                let material = match reveal {
                    Some(alpha_reveal_parms1) => NativeWorldMaterial::DiffuseRevealLayered {
                        base,
                        layer,
                        alpha_reveal_parms1,
                    },
                    None => NativeWorldMaterial::DiffuseLayered { base, layer },
                };
                return Ok((material, state));
            }
            let base = NativeWorldTextures {
                color_map,
                color_sampler,
                normal_map,
                normal_sampler,
                lightmap_secondary: lightmap,
                lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                reflection_probe: probe,
                reflection_sampler: code_sampler(MipmapFilterMode::Linear),
            };
            if family == "wpc_lit_sm_b0c0n0s0v0_efz5w56f" {
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap", "specularMap")?;
                return Ok((
                    NativeWorldMaterial::RevealNormalSpecular {
                        base,
                        specular_map,
                        specular_sampler,
                        alpha_reveal_parms: self.constant("alphaRevealParms")?,
                    },
                    state,
                ));
            }
            if family == "wpc_lit_sm_r0c0n0x0_f488034e" {
                return Ok((NativeWorldMaterial::Lightmapped(base), state));
            }
            if matches!(
                family,
                "wpc_lit_sm_r0c0n0s0_1zzj1138" | "wpc_lit_sm_t0c0n0s0_weef63f3"
            ) {
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap", "specularMap")?;
                return Ok((
                    NativeWorldMaterial::NormalSpecular {
                        base,
                        specular_map,
                        specular_sampler,
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "lit_sm_r0c0n0x0_b1c1_b2c2" | "lit_sm_r0c0n0x0_a1c1_m2c2"
            ) {
                let (color_map1, color_sampler1) = self.texture(server, "colorMap1", "colorMap")?;
                let (color_map2, color_sampler2) = self.texture(server, "colorMap2", "colorMap")?;
                return Ok((
                    NativeWorldMaterial::ColorOverlays {
                        base,
                        color_map1,
                        color_sampler1,
                        color_map2,
                        color_sampler2,
                        add_multiply: family == "lit_sm_r0c0n0x0_a1c1_m2c2",
                    },
                    state,
                ));
            }
            if family == "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2" {
                let layer = |index: u8| -> Result<NativeWorldLayerTextures, String> {
                    let (color_map, color_sampler) =
                        self.texture(server, &format!("colorMap{index}"), "colorMap")?;
                    let (normal_map, normal_sampler) =
                        self.texture(server, &format!("normalMap{index}"), "normalMap")?;
                    Ok(NativeWorldLayerTextures {
                        color_map,
                        color_sampler,
                        normal_map,
                        normal_sampler,
                    })
                };
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap1", "specularMap")?;
                return Ok((
                    NativeWorldMaterial::ThreeLayerRetainedSpecular {
                        base,
                        layer1: NativeWorldSpecularLayerTextures {
                            layer: layer(1)?,
                            specular_map,
                            specular_sampler,
                        },
                        layer2: layer(2)?,
                        alpha_reveal_parms1: self.constant("alphaRevealParms1")?,
                        alpha_reveal_parms2: self.constant("alphaRevealParms2")?,
                    },
                    state,
                ));
            }
            if family == "lit_sm_r0c0n0x0_b1c1n1_b2c2n2" {
                let layer = |index: u8| -> Result<NativeWorldLayerTextures, String> {
                    let (color_map, color_sampler) =
                        self.texture(server, &format!("colorMap{index}"), "colorMap")?;
                    let (normal_map, normal_sampler) =
                        self.texture(server, &format!("normalMap{index}"), "normalMap")?;
                    Ok(NativeWorldLayerTextures {
                        color_map,
                        color_sampler,
                        normal_map,
                        normal_sampler,
                    })
                };
                return Ok((
                    NativeWorldMaterial::ThreeLayerNormals {
                        base,
                        layer1: layer(1)?,
                        layer2: layer(2)?,
                    },
                    state,
                ));
            }
            if family == "lit_sm_r0c0n0x0_b1c1n1_a2c2" {
                let (color_map, color_sampler) = self.texture(server, "colorMap1", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap1", "normalMap")?;
                let (color_map2, color_sampler2) = self.texture(server, "colorMap2", "colorMap")?;
                return Ok((
                    NativeWorldMaterial::LayerAdd {
                        base,
                        layer1: NativeWorldLayerTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                        },
                        color_map2,
                        color_sampler2,
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2"
            ) {
                let (color_map, color_sampler) = self.texture(server, "colorMap1", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap1", "normalMap")?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap1", "specularMap")?;
                let (color_map2, color_sampler2) = self.texture(server, "colorMap2", "colorMap")?;
                return Ok((
                    NativeWorldMaterial::RevealOverlay {
                        additive: family != "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2",
                        multiply3: if family.ends_with("m3c3") {
                            Some(self.texture(server, "colorMap3", "colorMap")?)
                        } else {
                            None
                        },
                        base,
                        layer1: NativeWorldSpecularLayerTextures {
                            layer: NativeWorldLayerTextures {
                                color_map,
                                color_sampler,
                                normal_map,
                                normal_sampler,
                            },
                            specular_map,
                            specular_sampler,
                        },
                        color_map2,
                        color_sampler2,
                        alpha_reveal_parms1: self.constant("alphaRevealParms1")?,
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "lit_sm_r0c0n0x0_b1c1_b2c2n2"
                    | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2"
                    | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2"
            ) {
                let (color_map1, color_sampler1) = self.texture(server, "colorMap1", "colorMap")?;
                let (color_map, color_sampler) = self.texture(server, "colorMap2", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap2", "normalMap")?;
                return Ok((
                    NativeWorldMaterial::ColorThreeLayer {
                        base,
                        color_map1,
                        color_sampler1,
                        layer2: NativeWorldLayerTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                        },
                        specular2: if family.ends_with("n2") {
                            None
                        } else {
                            Some(self.texture(server, "specularMap2", "specularMap")?)
                        },
                        alpha_reveal_parms2: if family.ends_with("v2") {
                            Some(self.constant("alphaRevealParms2")?)
                        } else {
                            None
                        },
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2"
            ) {
                let layer = |index: u8| -> Result<NativeWorldLayerTextures, String> {
                    let (color_map, color_sampler) =
                        self.texture(server, &format!("colorMap{index}"), "colorMap")?;
                    let (normal_map, normal_sampler) =
                        self.texture(server, &format!("normalMap{index}"), "normalMap")?;
                    Ok(NativeWorldLayerTextures {
                        color_map,
                        color_sampler,
                        normal_map,
                        normal_sampler,
                    })
                };
                let layer1 = layer(1)?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap2", "specularMap")?;
                let layer2 = NativeWorldSpecularLayerTextures {
                    layer: layer(2)?,
                    specular_map,
                    specular_sampler,
                };
                let alpha_reveal_parms2 = self.constant("alphaRevealParms2")?;
                let material = match family {
                    "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2" => NativeWorldMaterial::ThreeLayerReveal {
                        base,
                        layer1,
                        layer2,
                        alpha_reveal_parms2,
                    },
                    "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2" => {
                        let (specular_map, specular_sampler) =
                            self.texture(server, "specularMap", "specularMap")?;
                        NativeWorldMaterial::BaseSpecularThreeLayerReveal {
                            base: NativeWorldSpecularTextures {
                                base,
                                specular_map,
                                specular_sampler,
                            },
                            layer1,
                            layer2,
                            alpha_reveal_parms2,
                        }
                    }
                    _ => {
                        let (specular_map, specular_sampler) =
                            self.texture(server, "specularMap1", "specularMap")?;
                        let layer1 = NativeWorldSpecularLayerTextures {
                            layer: layer1,
                            specular_map,
                            specular_sampler,
                        };
                        let alpha_reveal_parms1 = self.constant("alphaRevealParms1")?;
                        if family == "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2" {
                            let (specular_map, specular_sampler) =
                                self.texture(server, "specularMap", "specularMap")?;
                            NativeWorldMaterial::BaseSpecularThreeLayerDoubleReveal {
                                base: NativeWorldSpecularTextures {
                                    base,
                                    specular_map,
                                    specular_sampler,
                                },
                                layer1,
                                layer2,
                                alpha_reveal_parms1,
                                alpha_reveal_parms2,
                            }
                        } else {
                            NativeWorldMaterial::ThreeLayerDoubleReveal {
                                base,
                                layer1,
                                layer2,
                                alpha_reveal_parms1,
                                alpha_reveal_parms2,
                            }
                        }
                    }
                };
                return Ok((material, state));
            }
            let (color_map, color_sampler) = self.texture(server, "colorMap1", "colorMap")?;
            if matches!(
                family,
                "lit_sm_r0c0n0x0_m1c1" | "lit_sm_r0c0n0x0_b1c1" | "lit_sm_r0c0n0x0_a1c1"
            ) {
                let layer = NativeWorldColorLayerTextures {
                    base,
                    color_map,
                    color_sampler,
                };
                let material = match family {
                    "lit_sm_r0c0n0x0_m1c1" => NativeWorldMaterial::ColorMultiply(layer),
                    "lit_sm_r0c0n0x0_b1c1" => NativeWorldMaterial::ColorBlend(layer),
                    "lit_sm_r0c0n0x0_a1c1" => NativeWorldMaterial::ColorAdd(layer),
                    _ => unreachable!("selected original color-layer family"),
                };
                return Ok((material, state));
            }
            let (normal_map, normal_sampler) = self.texture(server, "normalMap1", "normalMap")?;
            let layer = NativeWorldLayerTextures {
                color_map,
                color_sampler,
                normal_map,
                normal_sampler,
            };
            let material = match (family, reveal) {
                ("lit_sm_r0c0n0x0_b1c1n1", None) | ("lit_sm_r0c0n0x0_t1c1n1", None) => {
                    NativeWorldMaterial::Layered {
                        base,
                        layer,
                        hard_select: family == "lit_sm_r0c0n0x0_t1c1n1",
                    }
                }
                ("lit_sm_r0c0n0x0_b1c1n1v1", Some(alpha_reveal_parms1)) => {
                    NativeWorldMaterial::RevealLayered {
                        base,
                        layer,
                        alpha_reveal_parms1,
                    }
                }
                ("lit_sm_r0c0n0x0_b1c1n1s1", None) => {
                    let (layer_specular_map, layer_specular_sampler) =
                        self.texture(server, "specularMap1", "specularMap")?;
                    NativeWorldMaterial::SpecularLayered {
                        base,
                        layer,
                        layer_specular_map,
                        layer_specular_sampler,
                    }
                }
                ("lit_sm_r0c0n0x0_b1c1n1s1v1", Some(alpha_reveal_parms1)) => {
                    let (layer_specular_map, layer_specular_sampler) =
                        self.texture(server, "specularMap1", "specularMap")?;
                    NativeWorldMaterial::RevealSpecularLayered {
                        base,
                        layer,
                        layer_specular_map,
                        layer_specular_sampler,
                        alpha_reveal_parms1,
                    }
                }
                ("lit_sm_r0c0n0s0_b1c1n1", None) => {
                    let (specular_map, specular_sampler) =
                        self.texture(server, "specularMap", "specularMap")?;
                    NativeWorldMaterial::BaseSpecularLayered {
                        base: NativeWorldSpecularTextures {
                            base,
                            specular_map,
                            specular_sampler,
                        },
                        layer,
                    }
                }
                ("lit_sm_r0c0n0s0_b1c1n1s1v1", Some(alpha_reveal_parms1)) => {
                    let (specular_map, specular_sampler) =
                        self.texture(server, "specularMap", "specularMap")?;
                    let base = NativeWorldSpecularTextures {
                        base,
                        specular_map,
                        specular_sampler,
                    };
                    let (specular_map, specular_sampler) =
                        self.texture(server, "specularMap1", "specularMap")?;
                    NativeWorldMaterial::BaseSpecularRevealLayered {
                        base,
                        layer: NativeWorldSpecularLayerTextures {
                            layer,
                            specular_map,
                            specular_sampler,
                        },
                        alpha_reveal_parms1,
                    }
                }
                ("lit_sm_r0c0n0_b1c1n1s1v1", Some(alpha_reveal_parms1)) => {
                    let (specular_map, specular_sampler) =
                        self.texture(server, "specularMap1", "specularMap")?;
                    NativeWorldMaterial::NoBaseGlossRevealLayered {
                        base,
                        layer: NativeWorldSpecularLayerTextures {
                            layer,
                            specular_map,
                            specular_sampler,
                        },
                        alpha_reveal_parms1,
                    }
                }
                _ => unreachable!("world family and its required constant were selected together"),
            };
            Ok((material, state))
        };
        bind().map_err(|error| format!("Material {}: {error}", self.path.display()))
    }

    pub(crate) fn world_pass(&self) -> super::native::NativeWorldPass {
        use super::native::NativeWorldPass;
        match self.camera_region.as_str() {
            "emissiveOpaque" => NativeWorldPass::EmissiveOpaque,
            "litQuasiOpaque" => NativeWorldPass::QuasiOpaque,
            "litTrans" | "emissiveTrans" => NativeWorldPass::Translucent,
            _ => NativeWorldPass::Opaque,
        }
    }

    fn world_state(&self, alpha_test: &str) -> Result<NativeWorldState, String> {
        if !matches!(self.camera_region.as_str(), "litOpaque" | "litTrans") {
            return Err(format!(
                "Unsupported world camera region {}",
                self.camera_region
            ));
        }
        self.lit_state_bits(alpha_test)
    }

    fn lit_state(&self) -> Result<NativeWorldState, String> {
        self.lit_state_with_alpha("disabled")
    }

    fn lit_state_with_alpha(&self, alpha_test: &str) -> Result<NativeWorldState, String> {
        if self.camera_region != "litOpaque" {
            return Err(format!(
                "Add the original {} camera-region pass before binding this material",
                self.camera_region
            ));
        }
        let state = self.lit_state_bits(alpha_test)?;
        if state.blend.is_some() {
            return Err(
                "This model/water pipeline does not implement the original blend state".to_owned(),
            );
        }
        Ok(state)
    }

    fn lit_state_bits(&self, alpha_test: &str) -> Result<NativeWorldState, String> {
        self.state_bits_for(4, alpha_test)
    }

    fn state_bits_for(
        &self,
        technique: usize,
        alpha_test: &str,
    ) -> Result<NativeWorldState, String> {
        // T6 MATERIAL_TECHNIQUE_LIT = 4; stateBitsEntry indexes the state table,
        // independently of the technique's position in that table.
        let index = self
            .state_bits_entry
            .get(technique)
            .copied()
            .filter(|index| *index >= 0)
            .ok_or_else(|| format!("Original stateBitsEntry[{technique}] has no state"))?;
        let state = self.state_bits.get(index as usize).ok_or_else(|| {
            format!("Original stateBitsEntry[{technique}] references missing stateBits[{index}]")
        })?;
        let blend = match (state.blend_op_rgb.as_str(), state.blend_op_alpha.as_str()) {
            ("disabled", "disabled") => None,
            ("add", "add")
                if matches!(state.src_blend_rgb.as_str(), "one" | "srcalpha")
                    && state.dst_blend_rgb == "invsrcalpha"
                    && state.src_blend_alpha == "invdestalpha"
                    && state.dst_blend_alpha == "one" =>
            {
                Some(BlendState {
                    color: BlendComponent {
                        src_factor: if state.src_blend_rgb == "srcalpha" {
                            BlendFactor::SrcAlpha
                        } else {
                            BlendFactor::One
                        },
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::OneMinusDstAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                })
            }
            ("add", "add")
                if state.src_blend_rgb == "one"
                    && state.dst_blend_rgb == "one"
                    && matches!(state.src_blend_alpha.as_str(), "zero" | "invdestalpha")
                    && state.dst_blend_alpha == "one" =>
            {
                Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: if state.src_blend_alpha == "zero" {
                            BlendFactor::Zero
                        } else {
                            BlendFactor::OneMinusDstAlpha
                        },
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                })
            }
            ("add", "add")
                if state.src_blend_rgb == "zero"
                    && state.dst_blend_rgb == "srccolor"
                    && state.src_blend_alpha == "invdestalpha"
                    && state.dst_blend_alpha == "one" =>
            {
                Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::Src,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::OneMinusDstAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                })
            }
            _ => return Err("Unsupported original material blend factors/operations".to_owned()),
        };
        if state.alpha_test != alpha_test {
            return Err(format!(
                "Original alphaTest {} does not match this WESL program's {} test",
                state.alpha_test, alpha_test
            ));
        }
        if state.polymode_line {
            return Err(format!(
                "Add original raster state polymodeLine={}, polygonOffset={} to the native pipeline",
                state.polymode_line, state.polygon_offset
            ));
        }
        let polygon_offset = match state.polygon_offset.as_str() {
            "offset0" => 0,
            "offset1" => 1,
            "offset2" => 2,
            other => return Err(format!("Unsupported original polygonOffset {other}")),
        };
        if state.stencil_front.is_some() || state.stencil_back.is_some() {
            return Err(
                "Add this material's original stencil state to the native pipeline".to_owned(),
            );
        }
        // Original D3D table00c5e228 translates the logical material depth
        // states to [Always, Greater, Equal, GreaterEqual]. The native camera
        // uses reverse Z and R's 00425750 clear sets depth to zero.
        let depth_compare = match state.depth_test.as_str() {
            "always" => CompareFunction::Always,
            "less" => CompareFunction::Greater,
            "equal" => CompareFunction::Equal,
            "less_equal" => CompareFunction::GreaterEqual,
            other => {
                return Err(format!(
                    "Add original depthTest {other} to the native pipeline"
                ));
            }
        };
        let cull_mode = match state.cull_face.as_str() {
            "none" => None,
            "back" => Some(Face::Back),
            "front" => Some(Face::Front),
            other => return Err(format!("Unknown original cullFace {other}")),
        };
        let mut color_write = ColorWrites::empty();
        if state.color_write_rgb {
            color_write |= ColorWrites::RED | ColorWrites::GREEN | ColorWrites::BLUE;
        }
        if state.color_write_alpha {
            color_write |= ColorWrites::ALPHA;
        }
        Ok(NativeWorldState {
            blend,
            alpha_test: alpha_test == "ge128",
            depth_compare,
            depth_write: state.depth_write,
            polygon_offset,
            cull_mode,
            // Original packed world vertices and Uint16 indices retain D3D's
            // clockwise front-face convention; no CPU winding conversion.
            front_face: FrontFace::Cw,
            color_write,
        })
    }

    fn texture(
        &self,
        server: &AssetServer,
        name: &str,
        semantic: &str,
    ) -> Result<(Handle<Image>, SamplerDescriptor<'static>), String> {
        self.texture_key(server, name, semantic, None)
    }

    fn texture_key(
        &self,
        server: &AssetServer,
        name: &str,
        semantic: &str,
        hash: Option<u32>,
    ) -> Result<(Handle<Image>, SamplerDescriptor<'static>), String> {
        let mut matches = self.textures.iter().filter(|texture| match hash {
            Some(hash) => texture.name_hash == Some(hash),
            None => texture.name.as_deref() == Some(name),
        });
        let texture = matches
            .next()
            .ok_or_else(|| format!("Export the required original texture slot {name}"))?;
        if matches.next().is_some() {
            return Err(format!("Original texture slot {name} is duplicated"));
        }
        if texture.image.is_empty() || texture.semantic != semantic {
            return Err(format!(
                "Original texture slot {name} requires an image with semantic {semantic}"
            ));
        }
        let sampler = texture
            .sampler_state
            .descriptor()
            .map_err(|error| format!("Texture {name}: {error}"))?;
        Ok((image(server, &texture.image), sampler))
    }

    fn parallax_layer(
        &self,
        layer: usize,
        hdr: f32,
    ) -> Result<super::native::NativeWorldParallaxLayer, String> {
        let read = |suffix| self.constant(&format!("Layer{layer}{suffix}"));
        let scale = read("Scale")?;
        let offset = read("Offset")?;
        let scroll = read("Scroll")?;
        let rotation = read("Rotation")?;
        let origin = read("Origin")?;
        Ok(super::native::NativeWorldParallaxLayer {
            scale_offset: Vec4::new(scale.x, scale.y, offset.x, offset.y),
            scroll_alpha_hdr: Vec4::new(scroll.x, scroll.y, read("Alpha")?.x, hdr),
            rotation_origin: Vec4::new(rotation.x, rotation.y, origin.x, origin.y),
            depth: read("Depth")?,
        })
    }

    fn constant(&self, name: &str) -> Result<Vec4, String> {
        self.constant_key(name, None)
    }

    fn constant_hash(&self, name: &str, hash: u32) -> Result<Vec4, String> {
        self.constant_key(name, Some(hash))
    }

    fn constant_key(&self, name: &str, hash: Option<u32>) -> Result<Vec4, String> {
        let mut matches = self.constants.iter().filter(|constant| match hash {
            Some(hash) => constant.name_hash == Some(hash),
            None => constant.name.as_deref() == Some(name),
        });
        let constant = matches
            .next()
            .ok_or_else(|| format!("Export the required original constant {name}"))?;
        if matches.next().is_some() || !constant.literal.iter().all(|value| value.is_finite()) {
            return Err(format!(
                "Original constant {name} must be one finite float4"
            ));
        }
        Ok(Vec4::from_array(constant.literal))
    }
}

impl SamplerDefinition {
    fn descriptor(&self) -> Result<SamplerDescriptor<'static>, String> {
        // R_RegisterRendererDvars (0041f2b0) defaults r_texFilterQuality to 1.
        // R's native sampler table (00432ba0) then maps material aniso2x and
        // aniso4x to D3D11_FILTER_ANISOTROPIC (0x55), MaxAnisotropy = 1 << 1.
        let (filter, anisotropy_clamp) = match self.filter.as_str() {
            "disabled" | "nearest" => (FilterMode::Nearest, 1),
            "linear" => (FilterMode::Linear, 1),
            "aniso2x" | "aniso4x" => (FilterMode::Linear, 2),
            other => return Err(format!("Unknown original sampler filter {other}")),
        };
        let mipmap_filter = match self.mip_map.as_str() {
            "disabled" | "nearest" => MipmapFilterMode::Nearest,
            "linear" => MipmapFilterMode::Linear,
            other => return Err(format!("Unknown original sampler mipMap {other}")),
        };
        // The native quality table selects anisotropic filtering for both
        // mipMap values of an anisotropic sampler, including a linear mip filter.
        let mipmap_filter = if anisotropy_clamp > 1 {
            MipmapFilterMode::Linear
        } else {
            mipmap_filter
        };
        let address = |clamp| {
            if clamp {
                AddressMode::ClampToEdge
            } else {
                AddressMode::Repeat
            }
        };
        Ok(SamplerDescriptor {
            address_mode_u: address(self.clamp_u),
            address_mode_v: address(self.clamp_v),
            address_mode_w: address(self.clamp_w),
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
            compare: None,
            anisotropy_clamp,
            ..default()
        })
    }
}

pub(crate) fn code_sampler(mipmap_filter: MipmapFilterMode) -> SamplerDescriptor<'static> {
    // R's world LIT setup (00475300) binds source sampler 0x62 at s13 and
    // 0x72 at s15. At quality 1 they resolve through 00432ba0 to min/mag
    // linear with nearest/linear mip filtering respectively. Both clamp UV,
    // repeat W, use no comparison, expose every mip and have zero mip bias.
    SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::Repeat,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter,
        lod_min_clamp: 0.0,
        lod_max_clamp: f32::MAX,
        compare: None,
        anisotropy_clamp: 1,
        ..default()
    }
}

pub(crate) fn image(server: &AssetServer, name: &str) -> Handle<Image> {
    // Unchanged SDK image_path spelling. A PathBuf preserves literal '#' and
    // '?' in source image names instead of parsing them as asset labels.
    let path = AssetPath::from(PathBuf::from(format!(
        "images/{}.dds",
        name.replace('*', "_")
    )));
    server
        .load_builder()
        .with_settings(|settings: &mut ImageLoaderSettings| {
            settings.is_srgb = false;
            settings.asset_usage = RenderAssetUsages::RENDER_WORLD;
        })
        .load(path)
}
