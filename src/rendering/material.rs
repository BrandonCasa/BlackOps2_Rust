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
    thermal_material: Option<String>,
    camera_region: String,
    game_flags: Vec<String>,
    pub(crate) surface_type_bits: u32,
    pub(crate) sort_key: u8,
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

    pub(crate) fn thermal_material(&self) -> Option<&str> {
        self.thermal_material
            .as_deref()
            .filter(|name| !name.is_empty())
    }

    /// Actual +0x6c override selected by004127d0 for MMS actors. Absence alone
    /// selects the engine's infrared_white material; unsupported inputs fail.
    pub(crate) fn sonar_material(
        &self,
        server: &AssetServer,
        inputs: &std::collections::BTreeMap<String, MaterialDefinition>,
    ) -> Result<Option<super::native_sonar::SonarMaterial>, String> {
        use super::native_sonar::{SonarMaterial, SonarProgram};
        let Some(name) = self.thermal_material() else {
            return Ok(None);
        };
        let source = inputs
            .get(name)
            .ok_or_else(|| format!("Load required original thermalMaterial {name}"))?;
        let program = match source.technique_set.as_str() {
            "mc_sw4_3d_thermal_w9wzw265" => SonarProgram::Thermal,
            "mc_sw4_3d_thermal_char_17f49w63" => SonarProgram::ThermalChar,
            other => {
                return Err(format!(
                    "Port original MMS thermal shader {other} before binding {name}"
                ));
            }
        };
        let emissive = source.constant_hash("Emissive_Amount", 2875883048)?;
        let detail = source.constant_hash("Detail_Amount", 871442466)?;
        Ok(Some(SonarMaterial {
            program,
            constant: Vec4::new(emissive.x, detail.x, 0., 0.),
            textures: vec![
                source.texture_key(server, "Thermal_Map", "colorMap", Some(70768136))?,
                source.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?,
                source.texture_key(server, "Thermal_Gradient", "colorMap", Some(414530162))?,
            ],
            state: source.state_bits_for(3, "disabled")?,
        }))
    }

    pub(crate) fn technique_set(&self) -> &str {
        &self.technique_set
    }

    pub(crate) fn has_camera_pass(&self) -> bool {
        self.camera_region != "none"
    }

    pub(crate) fn receives_marks(&self) -> bool {
        self.sort_key != 24 && !self.game_flags.iter().any(|flag| flag == "4")
    }

    pub(crate) fn mark_lighting(&self) -> (bool, bool) {
        (
            self.game_flags.iter().any(|flag| flag == "2"),
            self.game_flags.iter().any(|flag| flag == "10"),
        )
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
                "mc_sw4_3d_viewmodel_unlit_60466j88"
                    | "mc_sw4_3d_viewmodel_unlit_8216051u"
                    | "mc_sw4_3d_viewmodel_unlit_alpha_85z6j5w5"
                    | "mc_sw4_3d_viewmodel_unlit_j299w86e"
            ) {
                use super::native_model::NativeModelUnlitProgram;
                let program = match family {
                    "mc_sw4_3d_viewmodel_unlit_60466j88" => NativeModelUnlitProgram::ViewmodelAdd,
                    "mc_sw4_3d_viewmodel_unlit_8216051u" => {
                        NativeModelUnlitProgram::ViewmodelSqrtAlpha
                    }
                    "mc_sw4_3d_viewmodel_unlit_alpha_85z6j5w5" => {
                        NativeModelUnlitProgram::ViewmodelAlpha
                    }
                    _ => NativeModelUnlitProgram::ViewmodelOpaque,
                };
                let (color_map, color_sampler) =
                    self.texture_key(server, "Color_Map", "colorMap", Some(3053961470))?;
                let alpha = if matches!(
                    program,
                    NativeModelUnlitProgram::ViewmodelSqrtAlpha
                        | NativeModelUnlitProgram::ViewmodelAlpha
                ) {
                    "gt0"
                } else {
                    "disabled"
                };
                return Ok((
                    NativeModelMaterial::Unlit {
                        base: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        program,
                        scale_rgb: self.constant_hash("Emissive_Amount", 2875883048)?,
                    },
                    self.state_bits_for(3, alpha)?,
                ));
            }
            if family == "mc_sw4_3d_sonar_attachment_lens2_78904485" {
                use super::native_model::NativeSonarConstants;
                let scatter = self.constant("ScatterSize")?;
                let enemies = self.constant("EnemiesColor")?;
                let background = self.constant("BackgroundColor")?;
                let edge = self.constant("EdgeColor")?;
                return Ok((
                    NativeModelMaterial::Sonar {
                        textures: [
                            self.texture_key(
                                server,
                                "StaticNoiseMap",
                                "colorMap",
                                Some(1231620122),
                            )?,
                            self.texture(server, "Lookup", "colorMap")?,
                        ],
                        constants: NativeSonarConstants {
                            source: [
                                Vec4::new(
                                    self.constant("ScatterAmount")?.x,
                                    scatter.x,
                                    scatter.y,
                                    self.constant("MinDepth")?.x,
                                ),
                                Vec4::new(
                                    self.constant("MaxDepth")?.x,
                                    enemies.x,
                                    enemies.y,
                                    enemies.z,
                                ),
                                Vec4::new(
                                    self.constant("MaxPulseDepth")?.x,
                                    self.constant("SceneNoise")?.x,
                                    self.constant("LineNoise")?.x,
                                    self.constant("hdrAmount")?.x,
                                ),
                                Vec4::new(
                                    background.x,
                                    background.y,
                                    background.z,
                                    self.constant("BackgroundNoise")?.x,
                                ),
                                Vec4::new(
                                    self.constant("PulseInterval")?.x,
                                    self.constant("PulseTime")?.x,
                                    self.constant("EdgeSize")?.x,
                                    self.constant("EdgeIntensity")?.x,
                                ),
                                Vec4::new(
                                    self.constant("EdgeMinDist")?.x,
                                    self.constant("EdgeMaxDist")?.x,
                                    self.constant_hash("Player_Color_Multiplier", 2216979909)?.x,
                                    self.constant_hash("Pulse_Lookup_Scale", 1915875877)?.x,
                                ),
                                Vec4::new(
                                    self.constant_hash("Player_Lookup_Scale", 476534793)?.x,
                                    self.constant_hash("Outline_Lookup_Scale", 1235725306)?.x,
                                    self.constant_hash("Min_Player_Intensity", 857136406)?.x,
                                    0.,
                                ),
                                self.constant("PulseColor")?,
                                Vec4::new(
                                    edge.x,
                                    edge.y,
                                    edge.z,
                                    self.constant_hash("Edge_Color_Multiplier", 693037141)?.x,
                                ),
                                Vec4::new(
                                    self.constant_hash("Pulse_Color_Multiplier", 1531143913)?.x,
                                    0.,
                                    0.,
                                    0.,
                                ),
                            ],
                        },
                    },
                    self.state_bits_for(3, "disabled")?,
                ));
            }
            if family == "mc_sw4_3d_reticle_ui3d_q49ew30e" {
                use super::native_model::NativeReticleConstants;
                let glow = self.constant("Glow_Color")?;
                return Ok((
                    NativeModelMaterial::Ui3dReticle {
                        noise: self.texture_key(
                            server,
                            "Detail_Map",
                            "colorMap",
                            Some(1436567410),
                        )?,
                        constants: NativeReticleConstants {
                            source: [
                                Vec4::new(
                                    glow.x,
                                    glow.y,
                                    glow.z,
                                    self.constant_hash("Emissive_Amount", 2875883048)?.x,
                                ),
                                Vec4::new(
                                    self.constant("Noise_Scale")?.x,
                                    self.constant_hash("Glow_Falloff", 746727556)?.x,
                                    self.constant_hash("Color_Map_Noise", 3994589023)?.x,
                                    0.,
                                ),
                                self.constant_hash("Reticle_Color", 701365634)?,
                                self.constant_hash("Reticle_Alt_Color", 3943878052)?,
                                self.constant_hash("Glow_Alt_Color", 2900146391)?,
                            ],
                        },
                    },
                    self.state_bits_for(3, "disabled")?,
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_reticle_dynamic_9820jqfj"
                    | "mc_sw4_3d_reticle_dynamic_2colors_5f888f0w"
                    | "mc_sw4_3d_reticle_static_35866wj0"
            ) {
                use super::native_model::{NativeReticleConstants, NativeReticleProgram};
                let two = family == "mc_sw4_3d_reticle_dynamic_2colors_5f888f0w";
                let glow = self.constant("Glow_Color")?;
                let reticle = self.constant_hash("Reticle_Color", 701365634)?;
                return Ok((
                    NativeModelMaterial::Reticle {
                        program: if two {
                            NativeReticleProgram::DynamicTwoColor
                        } else if family == "mc_sw4_3d_reticle_static_35866wj0" {
                            NativeReticleProgram::Static
                        } else {
                            NativeReticleProgram::Dynamic
                        },
                        textures: [
                            self.texture_key(server, "Detail_Map", "colorMap", Some(1436567410))?,
                            self.texture_key(server, "Color_Map", "colorMap", Some(3053961470))?,
                        ],
                        constants: NativeReticleConstants {
                            source: [
                                Vec4::new(
                                    glow.x,
                                    glow.y,
                                    glow.z,
                                    self.constant_hash("Emissive_Amount", 2875883048)?.x,
                                ),
                                Vec4::new(
                                    self.constant("Noise_Scale")?.x,
                                    self.constant_hash("Color_Map_Scale", 4000797241)?.x,
                                    self.constant_hash("Glow_Falloff", 746727556)?.x,
                                    self.constant_hash("Color_Map_Noise", 3994589023)?.x,
                                ),
                                Vec4::new(
                                    self.constant("Background")?.x,
                                    reticle.x,
                                    reticle.y,
                                    reticle.z,
                                ),
                                if two {
                                    self.constant_hash("Reticle_Alt_Color", 3943878052)?
                                } else {
                                    Vec4::ZERO
                                },
                                if two {
                                    self.constant_hash("Glow_Alt_Color", 2900146391)?
                                } else {
                                    Vec4::ZERO
                                },
                            ],
                        },
                    },
                    self.state_bits_for(3, "disabled")?,
                ));
            }
            if family == "mc_sw4_3d_optic_lens_4w47028j" {
                return Ok((
                    NativeModelMaterial::OpticLens {
                        textures: [
                            self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?,
                            self.texture_key(server, "Color_Map", "colorMap", Some(3053961470))?,
                            self.texture_key(
                                server,
                                "Specular_Color_Map",
                                "specularMap",
                                Some(2940931118),
                            )?,
                            self.texture_key(
                                server,
                                "Specular_Gloss_Map",
                                "colorMap",
                                Some(3548237271),
                            )?,
                        ],
                        probe,
                        probe_sampler: code_sampler(MipmapFilterMode::Linear),
                        controls: Vec4::new(
                            self.constant_hash("Specular_Amount", 1307013020)?.x,
                            self.constant_hash("Reflection_Amount", 211931156)?.x,
                            self.constant_hash("Specular_Decay_Threshold", 994574754)?.x,
                            0.,
                        ),
                    },
                    self.world_state("gt0")?,
                ));
            }
            if family == "mc_sw4_3d_unlit_1layer_nofog_91f00f80" {
                let scale = self.constant("Layer1Scale")?;
                let offset = self.constant("Layer1Offset")?;
                let scroll = self.constant("Layer1Scroll")?;
                let origin = self.constant("Layer1Origin")?;
                let rotation = self.constant("Layer1Rotation")?;
                return Ok((
                    NativeModelMaterial::SingleLayer {
                        layer: self.texture(server, "Layer1Map", "colorMap")?,
                        probe,
                        probe_sampler: code_sampler(MipmapFilterMode::Linear),
                        constants: super::native_model::NativeModelSingleLayerConstants {
                            source: [
                                Vec4::new(scale.x, scale.y, offset.x, offset.y),
                                Vec4::new(
                                    scroll.x,
                                    scroll.y,
                                    self.constant("Layer1Alpha")?.x,
                                    self.constant("hdrAmount")?.x,
                                ),
                                Vec4::new(rotation.x, rotation.y, origin.x, origin.y),
                                Vec4::new(
                                    self.constant("Layer1Depth")?.x,
                                    self.constant_hash("Gloss_Amount", 3432544439)?.x,
                                    self.constant_hash("Reflection_Amount", 211931156)?.x,
                                    0.,
                                ),
                            ],
                        },
                    },
                    self.world_state("gt0")?,
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_scrolling_alpha2_distort_z58wq263"
                    | "mc_sw4_3d_scrolling_alpha2_distort_qwj8139f"
                    | "mc_sw4_3d_scrolling_falloff_zfeather_69456263"
                    | "mc_sw4_3d_scrolling_falloff_2ff9qe6w"
                    | "mc_sw4_3d_zm_elec_disc_w323fq8j"
                    | "mc_sw4_3d_light_sphere_46j58w7u"
                    | "mc_sw4_3d_portal_e96z3z92"
                    | "mc_sw4_3d_zmb_sky_pillar_on_6821fw2w"
                    | "mc_sw4_3d_zm_sphere_elec_9j108eq6"
                    | "mc_sw4_3d_zm_sphere_elec_535e0f02"
                    | "mc_sw4_3d_zm_ice_script_static_blizzard_45q67691"
                    | "mc_sw4_3d_zm_ice_script_static_877zz4j0"
                    | "mc_sw4_3d_zmb_sphere_tendril_j0598qze"
                    | "mc_sw4_3d_water_on_surface_reveal_77320z68"
                    | "mc_sw4_3d_unlit_scroll_jq69qew7"
            ) {
                use super::native_model::{NativeEffectModelConstants, NativeEffectModelProgram};
                let (program, textures, constants) = match family {
                    "mc_sw4_3d_scrolling_alpha2_distort_z58wq263" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_color_scale = self.constant("ColorScale")?;
                        source[1].x = source_color_scale.x;
                        source[1].y = source_color_scale.y;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[1].z = source_color_speed.x;
                        source[1].w = source_color_speed.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[2].x = source_hdr_amount.x;
                        let source_distort_amount =
                            self.constant_hash("DistortAmount", 2693350095)?;
                        source[2].w = source_distort_amount.x;
                        let source_distortion_scale = self.constant("distortionScale")?;
                        source[3].x = source_distortion_scale.x;
                        source[3].y = source_distortion_scale.y;
                        let source_distortion_speed =
                            self.constant_hash("DistortionSpeed", 4084488332)?;
                        source[3].z = source_distortion_speed.x;
                        source[3].w = source_distortion_speed.y;
                        (
                            NativeEffectModelProgram::DistortOpaque,
                            vec![self.texture(server, "colorMap", "colorMap")?],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_scrolling_alpha2_distort_qwj8139f" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_alpha_scale = self.constant("AlphaScale")?;
                        source[0].x = source_alpha_scale.x;
                        source[0].y = source_alpha_scale.y;
                        let source_alpha_speed = self.constant("AlphaSpeed")?;
                        source[0].z = source_alpha_speed.x;
                        source[0].w = source_alpha_speed.y;
                        let source_color_scale = self.constant("ColorScale")?;
                        source[1].x = source_color_scale.x;
                        source[1].y = source_color_scale.y;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[1].z = source_color_speed.x;
                        source[1].w = source_color_speed.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[2].x = source_hdr_amount.x;
                        let source_falloff_power = self.constant("FalloffPower")?;
                        source[2].y = source_falloff_power.x;
                        let source_falloff_boost = self.constant("FalloffBoost")?;
                        source[2].z = source_falloff_boost.x;
                        let source_distort_amount =
                            self.constant_hash("DistortAmount", 2693350095)?;
                        source[2].w = source_distort_amount.x;
                        let source_distortion_scale = self.constant("distortionScale")?;
                        source[3].x = source_distortion_scale.x;
                        source[3].y = source_distortion_scale.y;
                        let source_distortion_speed =
                            self.constant_hash("DistortionSpeed", 4084488332)?;
                        source[3].z = source_distortion_speed.x;
                        source[3].w = source_distortion_speed.y;
                        let source_feather_kill = self.constant("FeatherKill")?;
                        source[4].x = source_feather_kill.x;
                        let source_fade_factor = self.constant("FadeFactor")?;
                        source[4].y = source_fade_factor.x;
                        (
                            NativeEffectModelProgram::DistortAlpha,
                            vec![
                                self.texture(server, "colorMap", "colorMap")?,
                                self.texture_key(server, "AlphaMap", "colorMap", Some(170923560))?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_scrolling_falloff_zfeather_69456263" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_color_scale = self.constant("ColorScale")?;
                        source[0].x = source_color_scale.x;
                        source[0].y = source_color_scale.y;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[0].z = source_color_speed.x;
                        source[0].w = source_color_speed.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[1].x = source_hdr_amount.x;
                        let source_falloff_power = self.constant("FalloffPower")?;
                        source[1].y = source_falloff_power.x;
                        let source_falloff_boost = self.constant("FalloffBoost")?;
                        source[1].z = source_falloff_boost.x;
                        let source_feather_kill = self.constant("FeatherKill")?;
                        source[1].w = source_feather_kill.x;
                        let source_fade_factor = self.constant("FadeFactor")?;
                        source[2].x = source_fade_factor.x;
                        (
                            NativeEffectModelProgram::ScrollingFeather,
                            vec![self.texture(server, "colorMap", "colorMap")?],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_scrolling_falloff_2ff9qe6w" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_color_scale = self.constant("ColorScale")?;
                        source[0].x = source_color_scale.x;
                        source[0].y = source_color_scale.y;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[0].z = source_color_speed.x;
                        source[0].w = source_color_speed.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[1].x = source_hdr_amount.x;
                        let source_falloff_power = self.constant("FalloffPower")?;
                        source[1].y = source_falloff_power.x;
                        let source_falloff_boost = self.constant("FalloffBoost")?;
                        source[1].z = source_falloff_boost.x;
                        (
                            NativeEffectModelProgram::ScrollingFalloff,
                            vec![self.texture(server, "colorMap", "colorMap")?],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_zm_elec_disc_w323fq8j" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_alpha_scale = self.constant("AlphaScale")?;
                        source[0].x = source_alpha_scale.x;
                        source[0].y = source_alpha_scale.y;
                        let source_alpha_speed = self.constant("AlphaSpeed")?;
                        source[0].z = source_alpha_speed.x;
                        source[0].w = source_alpha_speed.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[1].x = source_hdr_amount.x;
                        let source_alpha_falloff_power =
                            self.constant_hash("AlphaFalloffPower", 42483651)?;
                        source[1].y = source_alpha_falloff_power.x;
                        let source_alpha_falloff_boost =
                            self.constant_hash("AlphaFalloffBoost", 68285721)?;
                        source[1].z = source_alpha_falloff_boost.x;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[2].x = source_color_speed.x;
                        source[2].y = source_color_speed.y;
                        let source_color_scale = self.constant("ColorScale")?;
                        source[2].z = source_color_scale.x;
                        source[2].w = source_color_scale.y;
                        let source_fade_factor = self.constant("FadeFactor")?;
                        source[3].x = source_fade_factor.x;
                        let source_distortion_speed =
                            self.constant_hash("DistortionSpeed", 4084488332)?;
                        source[3].y = source_distortion_speed.x;
                        source[3].z = source_distortion_speed.y;
                        let source_distortion_scale = self.constant("distortionScale")?;
                        source[4].x = source_distortion_scale.x;
                        source[4].y = source_distortion_scale.y;
                        let source_distortion_amount =
                            self.constant_hash("DistortionAmount", 1260895079)?;
                        source[4].z = source_distortion_amount.x;
                        let source_color_offset = self.constant("ColorOffset")?;
                        source[5].x = source_color_offset.x;
                        source[5].y = source_color_offset.y;
                        let source_color = self.constant("Color")?;
                        source[6].x = source_color.x;
                        source[6].y = source_color.y;
                        source[6].z = source_color.z;
                        source[6].w = source_color.w;
                        let source_distortion2_speed =
                            self.constant_hash("Distortion2Speed", 3544024158)?;
                        source[7].x = source_distortion2_speed.x;
                        source[7].y = source_distortion2_speed.y;
                        let source_distortion2_scale =
                            self.constant_hash("Distortion2Scale", 3543472193)?;
                        source[7].z = source_distortion2_scale.x;
                        source[7].w = source_distortion2_scale.y;
                        let source_distortion2_amount =
                            self.constant_hash("Distortion2Amount", 1663396853)?;
                        source[8].x = source_distortion2_amount.x;
                        let source_color2_speed = self.constant("Color2Speed")?;
                        source[8].y = source_color2_speed.x;
                        source[8].z = source_color2_speed.y;
                        let source_color2_scale = self.constant("Color2Scale")?;
                        source[9].x = source_color2_scale.x;
                        source[9].y = source_color2_scale.y;
                        let source_color2_amount = self.constant("Color2Amount")?;
                        source[9].z = source_color2_amount.x;
                        let source_depth_fade = self.constant("DepthFade")?;
                        source[9].w = source_depth_fade.x;
                        let source_fade_min = self.constant("FadeMin")?;
                        source[10].x = source_fade_min.x;
                        let source_fade_max = self.constant("FadeMax")?;
                        source[10].y = source_fade_max.x;
                        let source_color_offset2 = self.constant("ColorOffset2")?;
                        source[10].z = source_color_offset2.x;
                        source[10].w = source_color_offset2.y;
                        (
                            NativeEffectModelProgram::ElectricDisc,
                            vec![
                                self.texture_key(
                                    server,
                                    "DistortionMap2",
                                    "colorMap",
                                    Some(3897646725),
                                )?,
                                self.texture(server, "colorMap2", "colorMap")?,
                                self.texture_key(
                                    server,
                                    "DistortionMap",
                                    "colorMap",
                                    Some(2981422039),
                                )?,
                                self.texture(server, "colorMap", "colorMap")?,
                                self.texture_key(server, "AlphaMap", "colorMap", Some(170923560))?,
                                self.texture_key(
                                    server,
                                    "AlphaMap3",
                                    "colorMap",
                                    Some(1345510171),
                                )?,
                                self.texture_key(
                                    server,
                                    "AlphaMap2",
                                    "colorMap",
                                    Some(1345510170),
                                )?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_light_sphere_46j58w7u" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_fov = self.constant("FOV")?;
                        source[0].x = source_fov.x;
                        let source_depth_scale = self.constant("DepthScale")?;
                        source[0].y = source_depth_scale.x;
                        let source_tint = self.constant("Tint")?;
                        source[1].x = source_tint.x;
                        source[1].y = source_tint.y;
                        source[1].z = source_tint.z;
                        (
                            NativeEffectModelProgram::LightSphere,
                            vec![],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_portal_e96z3z92" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_layer1_scale = self.constant("Layer1Scale")?;
                        source[0].x = source_layer1_scale.x;
                        source[0].y = source_layer1_scale.y;
                        let source_layer1_offset = self.constant("Layer1Offset")?;
                        source[0].z = source_layer1_offset.x;
                        source[0].w = source_layer1_offset.y;
                        let source_layer1_scroll = self.constant("Layer1Scroll")?;
                        source[1].x = source_layer1_scroll.x;
                        source[1].y = source_layer1_scroll.y;
                        let source_layer2_scale = self.constant("Layer2Scale")?;
                        source[1].z = source_layer2_scale.x;
                        source[1].w = source_layer2_scale.y;
                        let source_layer2_scroll = self.constant("Layer2Scroll")?;
                        source[2].x = source_layer2_scroll.x;
                        source[2].y = source_layer2_scroll.y;
                        let source_layer3_scale = self.constant("Layer3Scale")?;
                        source[2].z = source_layer3_scale.x;
                        source[2].w = source_layer3_scale.y;
                        let source_layer3_offset = self.constant("Layer3Offset")?;
                        source[3].x = source_layer3_offset.x;
                        source[3].y = source_layer3_offset.y;
                        let source_layer3_scroll = self.constant("Layer3Scroll")?;
                        source[3].z = source_layer3_scroll.x;
                        source[3].w = source_layer3_scroll.y;
                        let source_layer1_alpha = self.constant("Layer1Alpha")?;
                        source[4].x = source_layer1_alpha.x;
                        let source_layer2_alpha = self.constant("Layer2Alpha")?;
                        source[4].y = source_layer2_alpha.x;
                        let source_layer3_alpha = self.constant("Layer3Alpha")?;
                        source[4].z = source_layer3_alpha.x;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[4].w = source_hdr_amount.x;
                        let source_layer2_depth = self.constant("Layer2Depth")?;
                        source[5].x = source_layer2_depth.x;
                        let source_layer3_depth = self.constant("Layer3Depth")?;
                        source[5].y = source_layer3_depth.x;
                        let source_layer4_scale = self.constant("Layer4Scale")?;
                        source[5].z = source_layer4_scale.x;
                        source[5].w = source_layer4_scale.y;
                        let source_layer4_offset = self.constant("Layer4Offset")?;
                        source[6].x = source_layer4_offset.x;
                        source[6].y = source_layer4_offset.y;
                        let source_layer4_alpha = self.constant("Layer4Alpha")?;
                        source[6].z = source_layer4_alpha.x;
                        let source_layer4_depth = self.constant("Layer4Depth")?;
                        source[6].w = source_layer4_depth.x;
                        let source_color = self.constant("Color")?;
                        source[7].x = source_color.x;
                        source[7].y = source_color.y;
                        source[7].z = source_color.z;
                        source[7].w = source_color.w;
                        let source_distortion_speed =
                            self.constant_hash("DistortionSpeed", 4084488332)?;
                        source[8].x = source_distortion_speed.x;
                        source[8].y = source_distortion_speed.y;
                        let source_distortion_scale = self.constant("distortionScale")?;
                        source[8].z = source_distortion_scale.x;
                        source[8].w = source_distortion_scale.y;
                        let source_distortion_amount =
                            self.constant_hash("DistortionAmount", 1260895079)?;
                        source[9].x = source_distortion_amount.x;
                        let source_sin_max = self.constant("SinMax")?;
                        source[9].y = source_sin_max.x;
                        let source_sin_min = self.constant("SinMin")?;
                        source[9].z = source_sin_min.x;
                        let source_sin_div = self.constant("SinDiv")?;
                        source[9].w = source_sin_div.x;
                        let source_sin_mult = self.constant("SinMult")?;
                        source[10].x = source_sin_mult.x;
                        let source_distortion_amount3 =
                            self.constant_hash("DistortionAmount3", 2954831988)?;
                        source[10].y = source_distortion_amount3.x;
                        let source_distortion_speed3 =
                            self.constant_hash("DistortionSpeed3", 1644128831)?;
                        source[10].z = source_distortion_speed3.x;
                        source[10].w = source_distortion_speed3.y;
                        let source_distortion_scale3 =
                            self.constant_hash("DistortionScale3", 1659407168)?;
                        source[11].x = source_distortion_scale3.x;
                        source[11].y = source_distortion_scale3.y;
                        let source_flag_speed = self.constant("FlagSpeed")?;
                        source[11].z = source_flag_speed.x;
                        (
                            NativeEffectModelProgram::Portal,
                            vec![
                                self.texture(server, "Layer4Map", "colorMap")?,
                                self.texture_key(
                                    server,
                                    "DistortionMap3",
                                    "colorMap",
                                    Some(3897646724),
                                )?,
                                self.texture(server, "Layer3Map", "colorMap")?,
                                self.texture(server, "Layer2Map", "colorMap")?,
                                self.texture_key(
                                    server,
                                    "DistortionMap",
                                    "colorMap",
                                    Some(2981422039),
                                )?,
                                self.texture(server, "Layer1Map", "colorMap")?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_zmb_sky_pillar_on_6821fw2w" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_color_scale = self.constant("ColorScale")?;
                        source[0].x = source_color_scale.x;
                        source[0].y = source_color_scale.y;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[0].z = source_color_speed.x;
                        source[0].w = source_color_speed.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[1].x = source_hdr_amount.x;
                        let source_reveal_scale = self.constant("RevealScale")?;
                        source[1].y = source_reveal_scale.x;
                        source[1].z = source_reveal_scale.y;
                        let source_color = self.constant("Color")?;
                        source[2].x = source_color.x;
                        source[2].y = source_color.y;
                        source[2].z = source_color.z;
                        source[2].w = source_color.w;
                        let source_alpha_falloff_power =
                            self.constant_hash("AlphaFalloffPower", 42483651)?;
                        source[3].x = source_alpha_falloff_power.x;
                        let source_alpha_falloff_boost =
                            self.constant_hash("AlphaFalloffBoost", 68285721)?;
                        source[3].y = source_alpha_falloff_boost.x;
                        let source_fade_factor = self.constant("FadeFactor")?;
                        source[3].z = source_fade_factor.x;
                        let source_distortion_amount =
                            self.constant_hash("DistortionAmount", 1260895079)?;
                        source[3].w = source_distortion_amount.x;
                        let source_distortion_speed =
                            self.constant_hash("DistortionSpeed", 4084488332)?;
                        source[4].x = source_distortion_speed.x;
                        source[4].y = source_distortion_speed.y;
                        let source_distortion_scale = self.constant("distortionScale")?;
                        source[4].z = source_distortion_scale.x;
                        source[4].w = source_distortion_scale.y;
                        let source_depth_fade = self.constant("DepthFade")?;
                        source[5].x = source_depth_fade.x;
                        let source_fade_min = self.constant("FadeMin")?;
                        source[5].y = source_fade_min.x;
                        let source_fade_max = self.constant("FadeMax")?;
                        source[5].z = source_fade_max.x;
                        (
                            NativeEffectModelProgram::SkyPillar,
                            vec![
                                self.texture_key(
                                    server,
                                    "DistortionMap",
                                    "colorMap",
                                    Some(2981422039),
                                )?,
                                self.texture(server, "colorMap", "colorMap")?,
                                self.texture_key(server, "AlphaMap", "colorMap", Some(170923560))?,
                                self.texture_key(server, "RevealMap", "colorMap", Some(886589909))?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_zm_sphere_elec_9j108eq6" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[0].x = source_hdr_amount.x;
                        let source_alpha_falloff_power =
                            self.constant_hash("AlphaFalloffPower", 42483651)?;
                        source[0].y = source_alpha_falloff_power.x;
                        let source_alpha_falloff_boost =
                            self.constant_hash("AlphaFalloffBoost", 68285721)?;
                        source[0].z = source_alpha_falloff_boost.x;
                        let source_fade_factor = self.constant("FadeFactor")?;
                        source[0].w = source_fade_factor.x;
                        let source_color = self.constant("Color")?;
                        source[1].x = source_color.x;
                        source[1].y = source_color.y;
                        source[1].z = source_color.z;
                        source[1].w = source_color.w;
                        let source_distortion2_speed =
                            self.constant_hash("Distortion2Speed", 3544024158)?;
                        source[2].x = source_distortion2_speed.x;
                        source[2].y = source_distortion2_speed.y;
                        let source_distortion2_scale =
                            self.constant_hash("Distortion2Scale", 3543472193)?;
                        source[2].z = source_distortion2_scale.x;
                        source[2].w = source_distortion2_scale.y;
                        let source_distortion2_amount =
                            self.constant_hash("Distortion2Amount", 1663396853)?;
                        source[3].x = source_distortion2_amount.x;
                        let source_color2_speed = self.constant("Color2Speed")?;
                        source[3].y = source_color2_speed.x;
                        source[3].z = source_color2_speed.y;
                        let source_color2_scale = self.constant("Color2Scale")?;
                        source[4].x = source_color2_scale.x;
                        source[4].y = source_color2_scale.y;
                        let source_color2_amount = self.constant("Color2Amount")?;
                        source[4].z = source_color2_amount.x;
                        let source_reflection_amount = self.constant("ReflectionAmount")?;
                        source[4].w = source_reflection_amount.x;
                        (
                            NativeEffectModelProgram::ElectricSphere,
                            vec![
                                self.texture_key(
                                    server,
                                    "DistortionMap2",
                                    "colorMap",
                                    Some(3897646725),
                                )?,
                                self.texture(server, "colorMap2", "colorMap")?,
                                self.texture_key(
                                    server,
                                    "AlphaMap2",
                                    "colorMap",
                                    Some(1345510170),
                                )?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_zm_sphere_elec_535e0f02" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[0].x = source_hdr_amount.x;
                        let source_alpha_falloff_power =
                            self.constant_hash("AlphaFalloffPower", 42483651)?;
                        source[0].y = source_alpha_falloff_power.x;
                        let source_alpha_falloff_boost =
                            self.constant_hash("AlphaFalloffBoost", 68285721)?;
                        source[0].z = source_alpha_falloff_boost.x;
                        let source_fade_factor = self.constant("FadeFactor")?;
                        source[0].w = source_fade_factor.x;
                        let source_color = self.constant("Color")?;
                        source[1].x = source_color.x;
                        source[1].y = source_color.y;
                        source[1].z = source_color.z;
                        source[1].w = source_color.w;
                        let source_distortion2_speed =
                            self.constant_hash("Distortion2Speed", 3544024158)?;
                        source[2].x = source_distortion2_speed.x;
                        source[2].y = source_distortion2_speed.y;
                        let source_distortion2_scale =
                            self.constant_hash("Distortion2Scale", 3543472193)?;
                        source[2].z = source_distortion2_scale.x;
                        source[2].w = source_distortion2_scale.y;
                        let source_distortion2_amount =
                            self.constant_hash("Distortion2Amount", 1663396853)?;
                        source[3].x = source_distortion2_amount.x;
                        let source_color2_speed = self.constant("Color2Speed")?;
                        source[3].y = source_color2_speed.x;
                        source[3].z = source_color2_speed.y;
                        let source_color2_scale = self.constant("Color2Scale")?;
                        source[4].x = source_color2_scale.x;
                        source[4].y = source_color2_scale.y;
                        let source_color2_amount = self.constant("Color2Amount")?;
                        source[4].z = source_color2_amount.x;
                        let source_reflection_amount = self.constant("ReflectionAmount")?;
                        source[4].w = source_reflection_amount.x;
                        (
                            NativeEffectModelProgram::ElectricSphereAlpha,
                            vec![
                                self.texture_key(
                                    server,
                                    "DistortionMap2",
                                    "colorMap",
                                    Some(3897646725),
                                )?,
                                self.texture(server, "colorMap2", "colorMap")?,
                                self.texture_key(
                                    server,
                                    "AlphaMap2",
                                    "colorMap",
                                    Some(1345510170),
                                )?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_zm_ice_script_static_blizzard_45q67691" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_alpha_scale = self.constant("AlphaScale")?;
                        source[0].x = source_alpha_scale.x;
                        source[0].y = source_alpha_scale.y;
                        let source_alpha_speed = self.constant("AlphaSpeed")?;
                        source[0].z = source_alpha_speed.x;
                        source[0].w = source_alpha_speed.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[1].x = source_hdr_amount.x;
                        let source_alpha_falloff_power =
                            self.constant_hash("AlphaFalloffPower", 42483651)?;
                        source[1].y = source_alpha_falloff_power.x;
                        let source_alpha_falloff_boost =
                            self.constant_hash("AlphaFalloffBoost", 68285721)?;
                        source[1].z = source_alpha_falloff_boost.x;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[2].x = source_color_speed.x;
                        source[2].y = source_color_speed.y;
                        let source_color_scale = self.constant("ColorScale")?;
                        source[2].z = source_color_scale.x;
                        source[2].w = source_color_scale.y;
                        let source_fade_factor = self.constant("FadeFactor")?;
                        source[3].x = source_fade_factor.x;
                        let source_reflection_amount = self.constant("ReflectionAmount")?;
                        source[3].y = source_reflection_amount.x;
                        let source_normal_scale = self.constant("NormalScale")?;
                        source[3].z = source_normal_scale.x;
                        source[3].w = source_normal_scale.y;
                        let source_normal_height = self.constant("NormalHeight")?;
                        source[4].x = source_normal_height.x;
                        source[4].y = source_normal_height.y;
                        let source_distortion_speed =
                            self.constant_hash("DistortionSpeed", 4084488332)?;
                        source[4].z = source_distortion_speed.x;
                        source[4].w = source_distortion_speed.y;
                        let source_distortion_scale = self.constant("distortionScale")?;
                        source[5].x = source_distortion_scale.x;
                        source[5].y = source_distortion_scale.y;
                        let source_distortion_amount =
                            self.constant_hash("DistortionAmount", 1260895079)?;
                        source[5].z = source_distortion_amount.x;
                        (
                            NativeEffectModelProgram::IceBlizzard,
                            vec![
                                self.texture(server, "normalMap", "normalMap")?,
                                self.texture_key(
                                    server,
                                    "DistortionMap",
                                    "colorMap",
                                    Some(2981422039),
                                )?,
                                self.texture(server, "colorMap", "colorMap")?,
                                self.texture_key(server, "AlphaMap", "colorMap", Some(170923560))?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_zm_ice_script_static_877zz4j0" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_alpha_scale = self.constant("AlphaScale")?;
                        source[0].x = source_alpha_scale.x;
                        source[0].y = source_alpha_scale.y;
                        let source_alpha_speed = self.constant("AlphaSpeed")?;
                        source[0].z = source_alpha_speed.x;
                        source[0].w = source_alpha_speed.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[1].x = source_hdr_amount.x;
                        let source_alpha_falloff_power =
                            self.constant_hash("AlphaFalloffPower", 42483651)?;
                        source[1].y = source_alpha_falloff_power.x;
                        let source_alpha_falloff_boost =
                            self.constant_hash("AlphaFalloffBoost", 68285721)?;
                        source[1].z = source_alpha_falloff_boost.x;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[2].x = source_color_speed.x;
                        source[2].y = source_color_speed.y;
                        let source_color_scale = self.constant("ColorScale")?;
                        source[2].z = source_color_scale.x;
                        source[2].w = source_color_scale.y;
                        let source_reflection_amount = self.constant("ReflectionAmount")?;
                        source[3].x = source_reflection_amount.x;
                        let source_normal_scale = self.constant("NormalScale")?;
                        source[3].y = source_normal_scale.x;
                        source[3].z = source_normal_scale.y;
                        let source_normal_height = self.constant("NormalHeight")?;
                        source[4].x = source_normal_height.x;
                        source[4].y = source_normal_height.y;
                        (
                            NativeEffectModelProgram::IceStatic,
                            vec![
                                self.texture(server, "normalMap", "normalMap")?,
                                self.texture(server, "colorMap", "colorMap")?,
                                self.texture_key(server, "AlphaMap", "colorMap", Some(170923560))?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_zmb_sphere_tendril_j0598qze" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_alpha_scale = self.constant("AlphaScale")?;
                        source[0].x = source_alpha_scale.x;
                        source[0].y = source_alpha_scale.y;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[0].z = source_hdr_amount.x;
                        let source_color_speed = self.constant("ColorSpeed")?;
                        source[1].x = source_color_speed.x;
                        source[1].y = source_color_speed.y;
                        let source_color_scale = self.constant("ColorScale")?;
                        source[1].z = source_color_scale.x;
                        source[1].w = source_color_scale.y;
                        let source_alpha_offset = self.constant("AlphaOffset")?;
                        source[2].x = source_alpha_offset.x;
                        source[2].y = source_alpha_offset.y;
                        let source_color = self.constant("Color")?;
                        source[3].x = source_color.x;
                        source[3].y = source_color.y;
                        source[3].z = source_color.z;
                        source[3].w = source_color.w;
                        let source_color_offset = self.constant("ColorOffset")?;
                        source[4].x = source_color_offset.x;
                        source[4].y = source_color_offset.y;
                        (
                            NativeEffectModelProgram::Tendril,
                            vec![
                                self.texture(server, "colorMap", "colorMap")?,
                                self.texture_key(
                                    server,
                                    "AlphaMap2",
                                    "colorMap",
                                    Some(1345510170),
                                )?,
                                self.texture_key(server, "AlphaMap", "colorMap", Some(170923560))?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_water_on_surface_reveal_77320z68" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_water_speed = self.constant("WaterSpeed")?;
                        source[0].x = source_water_speed.x;
                        let source_leading_edge_brightness =
                            self.constant_hash("Leading_Edge_Brightness", 3228582790)?;
                        source[0].y = source_leading_edge_brightness.x;
                        let source_state_2_leading_edge_color =
                            self.constant_hash("State_2_Leading_Edge_Color", 348546551)?;
                        source[1].x = source_state_2_leading_edge_color.x;
                        source[1].y = source_state_2_leading_edge_color.y;
                        source[1].z = source_state_2_leading_edge_color.z;
                        let source_state_2_alteration =
                            self.constant_hash("State_2_Alteration", 1288488438)?;
                        source[2].x = source_state_2_alteration.x;
                        source[2].y = source_state_2_alteration.y;
                        source[2].z = source_state_2_alteration.z;
                        let source_reflection_multiplier =
                            self.constant_hash("ReflectionMultiplier", 111253676)?;
                        source[2].w = source_reflection_multiplier.x;
                        let source_water_multiplier =
                            self.constant_hash("WaterMultiplier", 1116993630)?;
                        source[3].x = source_water_multiplier.x;
                        let source_water_scale = self.constant("WaterScale")?;
                        source[3].y = source_water_scale.x;
                        source[3].z = source_water_scale.y;
                        (
                            NativeEffectModelProgram::WaterReveal,
                            vec![
                                self.texture_key(
                                    server,
                                    "Temporal_Map",
                                    "colorMap",
                                    Some(2782436735),
                                )?,
                                self.texture(server, "normalMap", "normalMap")?,
                                self.texture_key(server, "Warp_Map", "colorMap", Some(3827006327))?,
                                self.texture(server, "NormalDetailMap", "normalMap")?,
                                self.texture(server, "colorMap", "colorMap")?,
                                self.texture(server, "SpecularAndGloss", "specularMap")?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    "mc_sw4_3d_unlit_scroll_jq69qew7" => {
                        let mut source = [Vec4::ZERO; 12];
                        let source_cycle_direction =
                            self.constant_hash("Cycle_Direction", 3048967178)?;
                        source[0].x = source_cycle_direction.x;
                        source[0].y = source_cycle_direction.y;
                        let source_cycle_scale = self.constant("Cycle_Scale")?;
                        source[0].z = source_cycle_scale.x;
                        let source_reflection_amount =
                            self.constant_hash("Reflection_Amount", 211931156)?;
                        source[0].w = source_reflection_amount.x;
                        let source_hdr_amount = self.constant("hdrAmount")?;
                        source[1].x = source_hdr_amount.x;
                        (
                            NativeEffectModelProgram::OrbScroll,
                            vec![
                                self.texture(server, "colorMap", "colorMap")?,
                                self.texture_key(
                                    server,
                                    "Cycle_Map",
                                    "colorMap",
                                    Some(3445507347),
                                )?,
                            ],
                            NativeEffectModelConstants { source },
                        )
                    }
                    _ => unreachable!("selected original effect-model program"),
                };
                let (slot, alpha_test) = match program {
                    NativeEffectModelProgram::DistortOpaque => (3, "disabled"),
                    NativeEffectModelProgram::DistortAlpha => (3, "gt0"),
                    NativeEffectModelProgram::ScrollingFeather => (3, "gt0"),
                    NativeEffectModelProgram::ScrollingFalloff => (3, "gt0"),
                    NativeEffectModelProgram::ElectricDisc => (3, "disabled"),
                    NativeEffectModelProgram::LightSphere => (3, "disabled"),
                    NativeEffectModelProgram::Portal => (3, "disabled"),
                    NativeEffectModelProgram::SkyPillar => (3, "disabled"),
                    NativeEffectModelProgram::ElectricSphere => (4, "disabled"),
                    NativeEffectModelProgram::ElectricSphereAlpha => (4, "gt0"),
                    NativeEffectModelProgram::IceBlizzard => (4, "disabled"),
                    NativeEffectModelProgram::IceStatic => (4, "disabled"),
                    NativeEffectModelProgram::Tendril => (3, "disabled"),
                    NativeEffectModelProgram::WaterReveal => (4, "disabled"),
                    NativeEffectModelProgram::OrbScroll => (4, "gt0"),
                };
                return Ok((
                    NativeModelMaterial::Effect {
                        program,
                        textures,
                        constants,
                        probe: program
                            .probe()
                            .then_some((probe, code_sampler(MipmapFilterMode::Linear))),
                    },
                    self.state_bits_for(slot, alpha_test)?,
                ));
            }
            if family == "mc_sw4_3d_player_icon_jef08712" {
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "NormalDetailMap", "normalMap")?;
                let (specular_map, specular_sampler) =
                    self.texture_key(server, "Specular_Map", "specularMap", Some(3532398796))?;
                return Ok((
                    NativeModelMaterial::TransparentPhong {
                        player_icon: true,
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
                        controls: Vec4::new(
                            self.constant("ReflectionAmount")?.x,
                            self.constant_hash("Specular_Amount", 1307013020)?.x,
                            self.constant_hash("Detail_Normal_Tile", 2210134934)?.x,
                            0.,
                        ),
                    },
                    self.world_state("gt0")?,
                ));
            }
            if family == "mc_sw4_3d_phong_simple_emissive_flicker_9j5f25q3" {
                let (color_map, color_sampler) =
                    self.texture_key(server, "DiffuseAndGloss", "colorMap", Some(3574481249))?;
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let (flicker_map, flicker_sampler) =
                    self.texture_key(server, "FlickerMap", "colorMap", Some(23340576))?;
                let (emissive_map, emissive_sampler) =
                    self.texture_key(server, "EmissiveMap", "colorMap", Some(878789447))?;
                return Ok((
                    NativeModelMaterial::Flicker {
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        flicker_map,
                        flicker_sampler,
                        emissive_map,
                        emissive_sampler,
                        constants: super::native_model::NativeFlickerConstants {
                            controls: Vec4::new(
                                self.constant("hdrAmount")?.x,
                                self.constant("FlickerSpeed")?.x,
                                self.constant("FlickerMin")?.x,
                                self.constant("FlickerMax")?.x,
                            ),
                            map_column: Vec4::new(
                                self.constant_hash("FlickerMapCol", 1271696416)?.x,
                                0.,
                                0.,
                                0.,
                            ),
                        },
                    },
                    self.lit_state()?,
                ));
            }
            if family == "mc_sw4_3d_clan_tag_fq631zzu" {
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap", "normalMap")?;
                let (detail_map, detail_sampler) =
                    self.texture(server, "NormalDetailMap", "normalMap")?;
                let size = self.constant("Char_Size")?;
                let constants = super::native_model::NativeClanTagConstants {
                    controls: Vec4::new(
                        self.constant("ReflectionAmount")?.x,
                        self.constant_hash("Detail_Normal_Tile", 2210134934)?.x,
                        self.constant_hash("Specular_Amount", 1307013020)?.x,
                        self.constant("Row_Chars_")?.x,
                    ),
                    atlas: Vec4::new(
                        size.x,
                        size.y,
                        self.constant("DDXScale")?.x,
                        self.constant("DDYScale")?.x,
                    ),
                    // 0052c1d0 initializes weaponParam1 to zero without a tag.
                    characters: Vec4::ZERO,
                };
                return Ok((
                    NativeModelMaterial::ClanTag {
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        detail_map,
                        detail_sampler,
                        constants,
                    },
                    self.world_state("gt0")?,
                ));
            }

            if family == "mc_sw4_3d_model_unlit_cheap_zombie_eyes_jq3e7eqw" {
                if self.camera_region != "emissiveOpaque" {
                    return Err(format!(
                        "Zombie eyes require original emissiveOpaque, got {}",
                        self.camera_region
                    ));
                }
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                return Ok((
                    NativeModelMaterial::ZombieEyes(NativeModelColorTexture {
                        color_map,
                        color_sampler,
                    }),
                    self.state_bits_for(3, "disabled")?,
                ));
            }

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
                    | "mc_sw4_3d_wpn_zmb_staff_3layer_94jz2ff5"
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
                let staff = family == "mc_sw4_3d_wpn_zmb_staff_3layer_94jz2ff5";
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
                let hdr_amount = if staff {
                    self.constant_hash("Emissive_Amount_Off", 996647064)?
                } else {
                    self.constant("hdrAmount")?
                };
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
                    Vec4::new(
                        warp.x,
                        warp.y,
                        self.constant("NormalHeight")?.x,
                        if staff {
                            self.constant_hash("Charge_Level", 963960723)?.x
                        } else {
                            0.
                        },
                    )
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
                if staff {
                    let (normal_map, normal_sampler) =
                        self.texture(server, "normalMap", "normalMap")?;
                    let (layer1_map, layer1_sampler) =
                        self.texture(server, "Layer1Map", "colorMap")?;
                    let (layer2_map, layer2_sampler) =
                        self.texture(server, "Layer2Map", "colorMap")?;
                    let (layer3_map, layer3_sampler) =
                        self.texture_key(server, "Layer3MapOn", "colorMap", Some(3424736909))?;
                    let off =
                        self.texture_key(server, "Layer3Map_Off", "colorMap", Some(1507458012))?;
                    return Ok((
                        NativeModelMaterial::Staff3Layer {
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
                            off,
                            constants,
                            emission_on: self.constant_hash("Emissive_Amount_On", 811104566)?,
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
                "mc_unlitdecalblend_multiply_34w7q3jw" | "mc_unlitdecalblend_add_fzw1z66f"
            ) {
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let base = NativeModelColorTexture {
                    color_map,
                    color_sampler,
                };
                return Ok((
                    if family == "mc_unlitdecalblend_add_fzw1z66f" {
                        NativeModelMaterial::DecalAdd(base)
                    } else {
                        NativeModelMaterial::DecalMultiply(base)
                    },
                    self.state_bits_for(3, "disabled")?,
                ));
            }
            if matches!(
                family,
                "mc_unlit_replace_3j970129"
                    | "mc_unlit_multiply_97923qz1"
                    | "mc_unlit_blend_588ewzff"
                    | "mc_unlit_add_ww82zw88"
            ) {
                use super::native_model::NativeModelUnlitProgram;
                let program = match family {
                    "mc_unlit_replace_3j970129" => NativeModelUnlitProgram::Replace,
                    "mc_unlit_multiply_97923qz1" => NativeModelUnlitProgram::Multiply,
                    "mc_unlit_blend_588ewzff" => NativeModelUnlitProgram::Blend,
                    "mc_unlit_add_ww82zw88" => NativeModelUnlitProgram::Add,
                    _ => unreachable!("selected original unlit family"),
                };
                let state = self.state_bits_for(
                    3,
                    if program == NativeModelUnlitProgram::Blend {
                        "gt0"
                    } else {
                        "disabled"
                    },
                )?;
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let scale_rgb = if program == NativeModelUnlitProgram::Multiply {
                    Vec4::ZERO
                } else {
                    self.constant("scaleRGB")?
                };
                return Ok((
                    NativeModelMaterial::Unlit {
                        base: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        program,
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
            if family == "mc_sw4_3d_viewmodel_transparent_z9z0z75f" {
                let (color_map, color_sampler) =
                    self.texture_key(server, "Color_Map", "colorMap", Some(3053961470))?;
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "SpecularAndGloss", "specularMap")?;
                return Ok((
                    NativeModelMaterial::TransparentPhong {
                        player_icon: false,
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
                        controls: Vec4::new(
                            self.constant_hash("Reflection_Amount", 211931156)?.x,
                            self.constant_hash("Specular_Amount", 1307013020)?.x,
                            1.,
                            0.,
                        ),
                    },
                    self.world_state("gt0")?,
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_char_cloth_reveal2_9z738w66" | "mc_sw4_3d_char_cloth_reveal2_727ej010"
            ) {
                use super::native_model::NativeClothRevealConstants;
                let (color_map, color_sampler) =
                    self.texture_key(server, "Diffuse_Map", "colorMap", Some(4030327853))?;
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "SpecularAndGloss", "specularMap")?;
                let (temporal_map, temporal_sampler) =
                    self.texture_key(server, "Temporal_Map", "colorMap", Some(2782436735))?;
                let edge1 = self.constant_hash("State_1_Leading_Edge_Color", 3218137492)?;
                let edge2 = self.constant_hash("State_2_Leading_Edge_Color", 348546551)?;
                let alter1 = self.constant_hash("State_1_Alteration", 3105072277)?;
                let alter2 = self.constant_hash("State_2_Alteration", 1288488438)?;
                let alpha_test = family == "mc_sw4_3d_char_cloth_reveal2_727ej010";
                return Ok((
                    NativeModelMaterial::ClothReveal {
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
                        temporal_map,
                        temporal_sampler,
                        alpha_test,
                        constants: NativeClothRevealConstants {
                            source: [
                                Vec4::new(
                                    self.constant_hash("Leading_Edge_Brightness", 3228582790)?.x,
                                    edge1.x,
                                    edge1.y,
                                    edge1.z,
                                ),
                                edge2,
                                alter1,
                                alter2
                                    .truncate()
                                    .extend(self.constant_hash("Specular_Amount", 1307013020)?.x),
                            ],
                        },
                    },
                    self.world_state(if alpha_test { "ge128" } else { "disabled" })?,
                ));
            }
            if family == "mc_objective_z4q1qe94" {
                use super::native_model::NativeSourceCharacterConstants;
                if self.camera_region != "emissiveOpaque" {
                    return Err("Objective requires original emissiveOpaque region".into());
                }
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let mut source = [Vec4::ZERO; 5];
                source[0] = self.constant("colorObjMin")?;
                source[1] = self.constant("colorObjMax")?;
                source[2] = Vec4::new(
                    self.constant("colorObjMinBaseBlend")?.x,
                    self.constant("colorObjMaxBaseBlend")?.x,
                    0.,
                    0.,
                );
                return Ok((
                    NativeModelMaterial::Objective {
                        base: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        constants: NativeSourceCharacterConstants { source },
                    },
                    self.state_bits_for(2, "disabled")?,
                ));
            }
            if family == "mc_projecteddecal_27zq5445" {
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                return Ok((
                    NativeModelMaterial::ProjectedDecal(NativeModelColorTexture {
                        color_map,
                        color_sampler,
                    }),
                    self.world_state("gt0")?,
                ));
            }
            if family == "mc_sw4_3d_char_eye_cornea_2eww29wu" {
                use super::native_model::NativeSourceCharacterConstants;
                let mut source = [Vec4::ZERO; 5];
                source[0] = self
                    .constant_hash("Fill_Direction", 2912016085)?
                    .truncate()
                    .extend(self.constant_hash("Hightlight_1_Size", 287662256)?.x);
                source[1] = Vec4::new(
                    self.constant_hash("Highlight_1_Sharpness", 2675429970)?.x,
                    self.constant_hash("Highlight_1_Brightness", 3598703080)?.x,
                    0.,
                    0.,
                );
                source[2] = self
                    .constant_hash("Fill_Direction2", 1607250247)?
                    .truncate()
                    .extend(self.constant_hash("Highlight_2_Size", 3136287143)?.x);
                source[3] = Vec4::new(
                    self.constant_hash("Highlight_2_Sharpness", 3466487505)?.x,
                    self.constant_hash("Highlight_2_Brightness", 606829963)?.x,
                    self.constant("OverallBrightness")?.x,
                    0.,
                );
                return Ok((
                    NativeModelMaterial::Cornea {
                        normal: self.texture_key(
                            server,
                            "Surface_Normal_Map",
                            "normalMap",
                            Some(188291226),
                        )?,
                        mask: self.texture(server, "Mask", "colorMap")?,
                        constants: NativeSourceCharacterConstants { source },
                    },
                    self.world_state("gt0")?,
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_phong_emissive_fritz_61728967"
                    | "mc_sw4_3d_phong_emissive_fritz_76q5wqjw"
            ) {
                use super::native_model::{NativeSourceCharacterConstants, SourceCharacterProgram};
                let cutout = family == "mc_sw4_3d_phong_emissive_fritz_76q5wqjw";
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let speed = self.constant("SparkleSpeed")?;
                let mut source = [Vec4::ZERO; 5];
                source[0] = Vec4::new(
                    self.constant("ReflectionAmount")?.x,
                    self.constant("SpecularAmount")?.x,
                    self.constant("hdrAmount")?.x,
                    0.,
                );
                source[1] = Vec4::new(
                    speed.x,
                    speed.y,
                    self.constant("SparkleScale")?.x,
                    self.constant("WarpAmount")?.x,
                );
                source[2] = Vec4::new(
                    self.constant("WarpScale")?.x,
                    self.constant("LumaBoost")?.x,
                    0.,
                    0.,
                );
                return Ok((
                    NativeModelMaterial::SourceCharacter {
                        program: SourceCharacterProgram::Fritz { cutout },
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        specular: self.texture(server, "SpecularAndGloss", "specularMap")?,
                        emission: Some(self.texture_key(
                            server,
                            "Glow_Map",
                            "colorMap",
                            Some(1825069840),
                        )?),
                        temporal: Some(self.texture(server, "WarpMap", "colorMap")?),
                        reveal: Some(self.texture(server, "SparkleMap", "colorMap")?),
                        constants: NativeSourceCharacterConstants { source },
                    },
                    self.lit_state_with_alpha(if cutout { "ge128" } else { "disabled" })?,
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_char_skin_emberglow_7q60f126" | "mc_sw4_3d_glass_emissive_jf185j5e"
            ) {
                use super::native_model::{NativeSourceCharacterConstants, SourceCharacterProgram};
                let ember = family == "mc_sw4_3d_char_skin_emberglow_7q60f126";
                let program = if ember {
                    SourceCharacterProgram::Ember
                } else {
                    SourceCharacterProgram::Glass
                };
                let (color_map, color_sampler) = if ember {
                    self.texture_key(server, "Diffuse_Map", "colorMap", Some(4030327853))?
                } else {
                    self.texture(server, "colorMap", "colorMap")?
                };
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let mut source = [Vec4::ZERO; 5];
                let (emission, temporal, reveal) = if ember {
                    source[0] = Vec4::new(
                        self.constant_hash("Reflection_Amount", 211931156)?.x,
                        self.constant_hash("Specular_Amount", 1307013020)?.x,
                        self.constant_hash("Diffuse_Normal_Height_Facing", 1012612089)?
                            .x,
                        self.constant("Heat_Scale")?.x,
                    );
                    let heat = self.constant_hash("Heat_Direction", 2542495778)?;
                    let ember = self.constant_hash("Ember_Direction", 4131458919)?;
                    source[1] = Vec4::new(heat.x, heat.y, ember.x, ember.y);
                    source[2] = Vec4::new(
                        self.constant("Ember_Scale")?.x,
                        self.constant("Flicker_Max")?.x,
                        self.constant("Flicker_Min")?.x,
                        self.constant("Seed_Value")?.x,
                    );
                    source[3].x = self.constant_hash("Emissiver_Amount", 227002554)?.x;
                    (
                        Some(self.texture_key(server, "Heat_Map", "colorMap", Some(3837378363))?),
                        Some(self.texture_key(
                            server,
                            "Ember_Map",
                            "colorMap",
                            Some(3617419774),
                        )?),
                        Some(self.texture_key(
                            server,
                            "EmberGlow_Reveal_Map",
                            "colorMap",
                            Some(690691323),
                        )?),
                    )
                } else {
                    source[0] = Vec4::new(
                        self.constant("ReflectionAmount")?.x,
                        self.constant("SpecularAmount")?.x,
                        self.constant_hash("EmissiveDepth", 3526985878)?.x,
                        self.constant("hdrAmount")?.x,
                    );
                    (None, None, None)
                };
                return Ok((
                    NativeModelMaterial::SourceCharacter {
                        program,
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        specular: self.texture(server, "SpecularAndGloss", "specularMap")?,
                        emission,
                        temporal,
                        reveal,
                        constants: NativeSourceCharacterConstants { source },
                    },
                    if ember {
                        self.lit_state()?
                    } else {
                        self.world_state("gt0")?
                    },
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_char_hair_287j114q"
                    | "mc_sw4_3d_char_skin_3e77jj31"
                    | "mc_sw4_3d_char_cloth_emissive_5ef7226z"
                    | "mc_sw4_3d_char_skin_emissive_189j7w84"
                    | "mc_sw4_3d_char_cloth_reveal2_emissive_5644z720"
                    | "mc_sw4_3d_char_cloth_reveal2_emissive_f88qewf4"
            ) {
                use super::native_model::{NativeSourceCharacterConstants, SourceCharacterProgram};
                let program = match family {
                    "mc_sw4_3d_char_hair_287j114q" => SourceCharacterProgram::Hair,
                    "mc_sw4_3d_char_skin_3e77jj31" => SourceCharacterProgram::SkinCutout,
                    "mc_sw4_3d_char_cloth_emissive_5ef7226z" => {
                        SourceCharacterProgram::ClothEmissive
                    }
                    "mc_sw4_3d_char_skin_emissive_189j7w84" => SourceCharacterProgram::SkinEmissive,
                    "mc_sw4_3d_char_cloth_reveal2_emissive_5644z720" => {
                        SourceCharacterProgram::RevealEmissive { cutout: false }
                    }
                    _ => SourceCharacterProgram::RevealEmissive { cutout: true },
                };
                let state = match program {
                    SourceCharacterProgram::Hair => self.world_state("gt0")?,
                    SourceCharacterProgram::SkinCutout
                    | SourceCharacterProgram::RevealEmissive { cutout: true } => {
                        self.lit_state_with_alpha("ge128")?
                    }
                    _ => self.lit_state()?,
                };
                let (color_map, color_sampler) = if program == SourceCharacterProgram::Hair {
                    self.texture(server, "colorMap", "colorMap")?
                } else {
                    self.texture_key(server, "Diffuse_Map", "colorMap", Some(4030327853))?
                };
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let emission = if matches!(
                    program,
                    SourceCharacterProgram::Hair | SourceCharacterProgram::SkinCutout
                ) {
                    None
                } else {
                    Some(self.texture_key(server, "Emissive_Map", "colorMap", Some(3229466552))?)
                };
                let temporal = if matches!(program, SourceCharacterProgram::RevealEmissive { .. }) {
                    Some(self.texture_key(server, "Temporal_Map", "colorMap", Some(2782436735))?)
                } else {
                    None
                };
                let mut source = [Vec4::ZERO; 5];
                match program {
                    SourceCharacterProgram::Ember
                    | SourceCharacterProgram::Glass
                    | SourceCharacterProgram::Fritz { .. } => unreachable!(),
                    SourceCharacterProgram::Hair => {
                        source[0].x = self.constant("ReflectionAmount")?.x
                    }
                    SourceCharacterProgram::SkinCutout | SourceCharacterProgram::SkinEmissive => {
                        source[0] = Vec4::new(
                            self.constant_hash("Reflection_Amount", 211931156)?.x,
                            self.constant_hash("Specular_Amount", 1307013020)?.x,
                            self.constant_hash("Diffuse_Normal_Height_Facing", 1012612089)?
                                .x,
                            if program == SourceCharacterProgram::SkinEmissive {
                                self.constant_hash("Emissive_Amount", 2875883048)?.x
                            } else {
                                0.
                            },
                        );
                    }
                    SourceCharacterProgram::ClothEmissive => {
                        source[0] = Vec4::new(
                            1.,
                            1.,
                            1.,
                            self.constant_hash("Emissive_Amount", 2875883048)?.x,
                        )
                    }
                    SourceCharacterProgram::RevealEmissive { .. } => {
                        let edge1 = self.constant_hash("State_1_Leading_Edge_Color", 3218137492)?;
                        source[0] = Vec4::new(
                            self.constant_hash("Leading_Edge_Brightness", 3228582790)?.x,
                            edge1.x,
                            edge1.y,
                            edge1.z,
                        );
                        source[1] = self.constant_hash("State_2_Leading_Edge_Color", 348546551)?;
                        source[2] = self.constant_hash("State_1_Alteration", 3105072277)?;
                        source[3] = self
                            .constant_hash("State_2_Alteration", 1288488438)?
                            .truncate()
                            .extend(self.constant_hash("Diffuse_Emissive_Level", 2251066627)?.x);
                        source[4] = Vec4::new(
                            self.constant_hash("Diffuse_Emissive_Speed", 2226647986)?.x,
                            self.constant_hash("Specular_Amount", 1307013020)?.x,
                            0.,
                            0.,
                        );
                    }
                }
                return Ok((
                    NativeModelMaterial::SourceCharacter {
                        program,
                        base: NativeModelTextures {
                            color_map,
                            color_sampler,
                            normal_map,
                            normal_sampler,
                            reflection_probe: probe,
                            reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                        },
                        specular: self.texture(server, "SpecularAndGloss", "specularMap")?,
                        emission,
                        temporal,
                        reveal: None,
                        constants: NativeSourceCharacterConstants { source },
                    },
                    state,
                ));
            }
            if matches!(
                family,
                "mc_sw4_3d_char_cloth_4z8fq5wu" | "mc_sw4_3d_char_skin_j92387z3"
            ) {
                let (color_map, color_sampler) =
                    self.texture_key(server, "Diffuse_Map", "colorMap", Some(4030327853))?;
                let (normal_map, normal_sampler) =
                    self.texture_key(server, "Normal_Map", "normalMap", Some(2485960688))?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "SpecularAndGloss", "specularMap")?;
                return Ok((
                    NativeModelMaterial::Character {
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
                        skin_controls: if family == "mc_sw4_3d_char_skin_j92387z3" {
                            Some(Vec4::new(
                                self.constant_hash("Reflection_Amount", 211931156)?.x,
                                self.constant_hash("Specular_Amount", 1307013020)?.x,
                                self.constant_hash("Diffuse_Normal_Height_Facing", 1012612089)?
                                    .x,
                                0.,
                            ))
                        } else {
                            None
                        },
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
                    | "mc_sw4_3d_phong_emissive_script_multi_72q2fj2u"
            ) {
                use super::native_model::{EmissiveProgram, NativeModelEmissiveConstants};
                let transparent = family == "mc_sw4_3d_phong_emissive_alcatraz_8q551153";
                let scripted_glow = family == "mc_sw4_3d_emissive_tomb_qww536q2";
                let multi_alpha = family == "mc_sw4_3d_phong_emissive_script_multi_j05zz52w";
                let multi =
                    multi_alpha || family == "mc_sw4_3d_phong_emissive_script_multi_72q2fj2u";
                let state = if multi_alpha {
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
                            EmissiveProgram::ScriptMulti {
                                alpha_test: multi_alpha,
                            }
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
            if matches!(
                family,
                "mc_lit_sm_r0c0n0s0o0_3z86zq2z" | "mc_lit_sm_r0c0n0s0o0_909j993q"
            ) {
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
                            hero_lighting: family == "mc_lit_sm_r0c0n0s0o0_909j993q",
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
            // These LIT programs have source-specific lighting-matrix or detail inputs.
            if matches!(
                family,
                "mc_lit_sm_b0c0n0s0_68f25z0u"
                    | "mc_lit_sm_b0c0s0_q94f27qw"
                    | "mc_lit_sm_r0c0n0s0_ze50j7jz"
                    | "mc_lit_sm_t0c0n0s0_f933q84u"
                    | "mc_lit_sm_r0c0n0s0_7j414q0z"
                    | "mc_lit_sm_r0c0d0n0s0_qwq4q449"
                    | "mc_lit_sm_t0c0d0n0s0_68eww765"
            ) {
                use super::native_model::SourceLitProgram;
                let program = match family {
                    "mc_lit_sm_b0c0n0s0_68f25z0u" => SourceLitProgram::BlendedHero,
                    "mc_lit_sm_b0c0s0_q94f27qw" => SourceLitProgram::GeometricHero,
                    "mc_lit_sm_r0c0n0s0_ze50j7jz" => SourceLitProgram::NormalHero,
                    "mc_lit_sm_t0c0n0s0_f933q84u" => SourceLitProgram::CutoutHero,
                    "mc_lit_sm_r0c0n0s0_7j414q0z" => SourceLitProgram::ColorDetail,
                    "mc_lit_sm_t0c0d0n0s0_68eww765" => SourceLitProgram::CutoutNormalDetail,
                    _ => SourceLitProgram::NormalDetail,
                };
                let state = match program {
                    SourceLitProgram::BlendedHero | SourceLitProgram::GeometricHero => {
                        self.world_state("gt0")?
                    }
                    SourceLitProgram::CutoutHero | SourceLitProgram::CutoutNormalDetail => {
                        self.lit_state_with_alpha("ge128")?
                    }
                    _ => self.lit_state()?,
                };
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let normal = if program == SourceLitProgram::GeometricHero {
                    None
                } else {
                    Some(self.texture(server, "normalMap", "normalMap")?)
                };
                let detail = match program {
                    SourceLitProgram::ColorDetail => {
                        Some(self.texture(server, "colorDetailMap", "color10Map")?)
                    }
                    SourceLitProgram::NormalDetail | SourceLitProgram::CutoutNormalDetail => {
                        Some(self.texture(server, "detailMap", "normalMap")?)
                    }
                    _ => None,
                };
                return Ok((
                    NativeModelMaterial::SourceLit {
                        program,
                        color: NativeModelColorTexture {
                            color_map,
                            color_sampler,
                        },
                        normal,
                        specular: self.texture(server, "specularMap", "specularMap")?,
                        probe: (probe, code_sampler(MipmapFilterMode::Linear)),
                        detail,
                        occlusion: self.constant("occlusionAmount")?,
                        // Base weapon setup 0052c1d0 supplies zero weaponParam2;
                        // only the normal-detail program binds material detailScale.
                        detail_parameters: if matches!(
                            program,
                            SourceLitProgram::NormalDetail | SourceLitProgram::CutoutNormalDetail
                        ) {
                            self.constant("detailScale")?
                        } else {
                            Vec4::ZERO
                        },
                    },
                    state,
                ));
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
            if matches!(
                family,
                "mc_lit_sm_b0c0n0s0_91j5z9q9" | "mc_lit_sm_b0c0n0s0_21fz2e61"
            ) {
                let state = self.world_state("gt0")?;
                let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
                let (normal_map, normal_sampler) =
                    self.texture(server, "normalMap", "normalMap")?;
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap", "specularMap")?;
                return Ok((
                    NativeModelMaterial::NormalSpecular {
                        program: super::native_model::NormalSpecularProgram::Blended {
                            unattenuated_reflection: family == "mc_lit_sm_b0c0n0s0_21fz2e61",
                        },
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
                        occlusion_amount: self.constant("occlusionAmount")?,
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
                "wpc_lit_sm_t0c0_2w3887e6"
                    | "wpc_lit_sm_b0c0_6j30z67q"
                    // World marks select the wc material from FxElemMarkVisuals.
                    // Its LIT technique uses the same VS bfa6d365 / PS 212e3fab.
                    | "wc_lit_sm_b0c0_jj25624z"
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
                    | "wc_lit_sm_b0c0n0_73wq744q"
                    | "wc_lit_sm_b0c0s0_12947e33"
                    | "wc_lit_sm_b0c0n0s0_w16w6e35"
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
                | "wpc_lit_sm_b0c0n0s0v0_efz5w56f"
                | "wc_lit_sm_b0c0n0_73wq744q"
                | "wc_lit_sm_b0c0s0_12947e33"
                | "wc_lit_sm_b0c0n0s0_w16w6e35" => "gt0",
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
                "wpc_lit_sm_r0c0s0_qfqz7850"
                    | "wpc_lit_sm_b0c0s0_3f3q946z"
                    | "wc_lit_sm_b0c0s0_12947e33"
                    | "lit_sm_r0c0s0_b1c1"
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
                    | "wc_lit_sm_b0c0n0_73wq744q"
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
                        | "wc_lit_sm_b0c0n0_73wq744q"
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
                "wpc_lit_sm_r0c0n0s0_1zzj1138"
                    | "wpc_lit_sm_t0c0n0s0_weef63f3"
                    | "wc_lit_sm_b0c0n0s0_w16w6e35"
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
        if !matches!(
            self.camera_region.as_str(),
            "litOpaque" | "litQuasiOpaque" | "litTrans"
        ) {
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
        let factor = |name: &str| match name {
            "zero" => Ok(BlendFactor::Zero),
            "one" => Ok(BlendFactor::One),
            "srccolor" => Ok(BlendFactor::Src),
            "invsrccolor" => Ok(BlendFactor::OneMinusSrc),
            "srcalpha" => Ok(BlendFactor::SrcAlpha),
            "invsrcalpha" => Ok(BlendFactor::OneMinusSrcAlpha),
            "destalpha" => Ok(BlendFactor::DstAlpha),
            "invdestalpha" => Ok(BlendFactor::OneMinusDstAlpha),
            "destcolor" => Ok(BlendFactor::Dst),
            "invdestcolor" => Ok(BlendFactor::OneMinusDst),
            other => Err(format!("Unsupported original blend factor {other}")),
        };
        let component = |operation: &str, source: &str, destination: &str| {
            let operation = match operation {
                "disabled" if source == "one" && destination == "zero" => BlendOperation::Add,
                "add" => BlendOperation::Add,
                "subtract" => BlendOperation::Subtract,
                "revsubtract" => BlendOperation::ReverseSubtract,
                "min" => BlendOperation::Min,
                "max" => BlendOperation::Max,
                other => return Err(format!("Unsupported original blend operation {other}")),
            };
            Ok(BlendComponent {
                src_factor: factor(source)?,
                dst_factor: factor(destination)?,
                operation,
            })
        };
        let blend = if state.blend_op_rgb == "disabled" && state.blend_op_alpha == "disabled" {
            None
        } else {
            Some(BlendState {
                color: component(
                    &state.blend_op_rgb,
                    &state.src_blend_rgb,
                    &state.dst_blend_rgb,
                )?,
                alpha: component(
                    &state.blend_op_alpha,
                    &state.src_blend_alpha,
                    &state.dst_blend_alpha,
                )?,
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires staged original Origins material inputs"]
    fn exported_mms_lens_binds_original_lookup_and_pulse_constants() {
        use super::super::native_model::NativeModelMaterial;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>();
        let definition = MaterialDefinition::read(Path::new(
            "target/assets/materials/mc/mtl_t6_attach_optic_mms_display.json",
        ))
        .unwrap();
        let (material, state) = definition
            .model(app.world().resource::<AssetServer>(), Handle::default())
            .unwrap();
        let NativeModelMaterial::Sonar {
            textures,
            constants,
        } = material
        else {
            panic!("MMS must require its source color/depth producer")
        };
        assert!(
            textures[0]
                .0
                .path()
                .unwrap()
                .path()
                .to_string_lossy()
                .ends_with("noise2.dds")
        );
        assert!(
            textures[1]
                .0
                .path()
                .unwrap()
                .path()
                .to_string_lossy()
                .ends_with("~sonar_attachment_lookup_puls~4187f661.dds")
        );
        assert_eq!(constants.source[0], Vec4::new(0., 0.5, 0.5, 60.));
        assert_eq!(constants.source[2], Vec4::new(1250., 0., 0., 1.25));
        assert_eq!(constants.source[4].xy(), Vec2::new(1.5, 1.1));
        assert_eq!(constants.source[5], Vec4::new(10., 1000., 7., 3.));
        assert_eq!(constants.source[6], Vec4::new(2., 2., 0., 0.));
        assert_eq!(constants.source[9].x, 1.5);
        assert!(!state.depth_write);
        assert_eq!(state.blend.unwrap().color.src_factor, BlendFactor::One);
        assert_eq!(state.blend.unwrap().color.dst_factor, BlendFactor::One);
    }

    #[test]
    #[ignore = "requires staged original Origins material inputs"]
    fn exported_rangefinder_ui3d_material_binds_real_target_contract() {
        use super::super::native_model::NativeModelMaterial;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>();
        let definition = MaterialDefinition::read(Path::new(
            "target/assets/materials/mc/mtl_t6_attach_optic_rangefinder_reticle.json",
        ))
        .unwrap();
        let (material, state) = definition
            .model(app.world().resource::<AssetServer>(), Handle::default())
            .unwrap();
        let NativeModelMaterial::Ui3dReticle { noise, constants } = material else {
            panic!("UI3D must require its generated target")
        };
        assert!(
            noise
                .0
                .path()
                .unwrap()
                .path()
                .to_string_lossy()
                .ends_with("mtl_t6_attach_optic_reticle_noise.dds")
        );
        assert_eq!(constants.source[0].w, 15.);
        assert_eq!(constants.source[1], Vec4::new(20., 3., 0.5, 0.));
        assert_eq!(constants.source[3].x, 1.);
        assert_eq!(constants.source[4], Vec4::new(1., 0., 0., 1.));
        assert!(!state.depth_write);
        assert_eq!(state.blend.unwrap().color.src_factor, BlendFactor::One);
        assert_eq!(state.blend.unwrap().color.dst_factor, BlendFactor::One);
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_objective_and_projected_model_materials_bind() {
        use super::super::native_model::NativeModelMaterial;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let objective = MaterialDefinition::read(
            &root.join("materials/mc/mtl_t6_wpn_zmb_shield_dlc4_red.json"),
        )
        .unwrap();
        let (material, state) = objective.model(server, Handle::default()).unwrap();
        let NativeModelMaterial::Objective { base, constants } = material else {
            panic!("Shield requires its source objective program");
        };
        assert_eq!(
            base.color_map.path().unwrap().path(),
            Path::new("images/t6_wpn_zmb_shield_dlc4_col.dds")
        );
        assert_eq!(
            constants.source[0],
            objective.constant("colorObjMin").unwrap()
        );
        assert_eq!(
            constants.source[1],
            objective.constant("colorObjMax").unwrap()
        );
        assert_eq!(constants.source[2], Vec4::new(1., 0.25, 0., 0.));
        assert!(state.depth_write);
        assert!(state.blend.is_none());
        let projected =
            MaterialDefinition::read(&root.join("materials/mc/mtl_fx_decal_character_blood.json"))
                .unwrap();
        let (material, state) = projected.model(server, Handle::default()).unwrap();
        let NativeModelMaterial::ProjectedDecal(base) = material else {
            panic!("Character blood requires depth projection");
        };
        assert_eq!(
            base.color_map.path().unwrap().path(),
            Path::new("images/~-gfx_decal_character_blood_c.dds")
        );
        assert_eq!(base.color_sampler.address_mode_u, AddressMode::ClampToEdge);
        assert_eq!(base.color_sampler.address_mode_v, AddressMode::ClampToEdge);
        assert!(state.blend.is_some());
        assert!(!state.depth_write);
        assert_eq!(state.cull_mode, None);
        assert_eq!(state.polygon_offset, 1);
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_source_cornea_model_binds() {
        use super::super::native_model::NativeModelMaterial;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let definition =
            MaterialDefinition::read(&root.join("materials/mc/mtl_gen_eye_cornea.json")).unwrap();
        let (material, state) = definition
            .model(app.world().resource::<AssetServer>(), Handle::default())
            .unwrap();
        let NativeModelMaterial::Cornea {
            normal,
            mask,
            constants,
        } = material
        else {
            panic!("Cornea requires native highlights and current camera axes");
        };
        assert_eq!(
            normal.0.path().unwrap().path(),
            Path::new("images/c_gen_eye_n.dds")
        );
        assert_eq!(
            mask.0.path().unwrap().path(),
            Path::new("images/c_gen_eye_color_a.dds")
        );
        assert_eq!(
            constants.source[0].truncate(),
            definition
                .constant_hash("Fill_Direction", 2912016085)
                .unwrap()
                .truncate()
        );
        assert_eq!(
            constants.source[2].truncate(),
            definition
                .constant_hash("Fill_Direction2", 1607250247)
                .unwrap()
                .truncate()
        );
        assert_eq!(constants.source[1].x, 0.);
        assert_eq!(constants.source[3].x, -2.880000114440918);
        assert_eq!(constants.source[3].z, 0.8899999856948853);
        assert_eq!(normal.1.mipmap_filter, MipmapFilterMode::Nearest);
        assert!(state.blend.is_some());
        assert!(!state.depth_write);
        assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_source_fritz_model_variants_bind() {
        use super::super::native_model::{NativeModelMaterial, SourceCharacterProgram};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        for (name, cutout) in [
            ("mtl_p6_zm_tm_monolith_ring", false),
            ("mtl_p6_zm_al_vending_three_gun_on", true),
            ("mtl_p6_zm_al_vending_three_gun_logo_on", true),
        ] {
            let definition =
                MaterialDefinition::read(&root.join(format!("materials/mc/{name}.json"))).unwrap();
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            let NativeModelMaterial::SourceCharacter {
                program,
                emission: Some((glow, _)),
                temporal: Some((warp, _)),
                reveal: Some((sparkle, _)),
                constants,
                ..
            } = material
            else {
                panic!("Fritz requires its separate glow, warp and sparkle inputs");
            };
            assert_eq!(program, SourceCharacterProgram::Fritz { cutout });
            assert_eq!(
                constants.source[0].z,
                definition.constant("hdrAmount").unwrap().x
            );
            assert_eq!(
                constants.source[1].truncate().truncate(),
                definition
                    .constant("SparkleSpeed")
                    .unwrap()
                    .truncate()
                    .truncate()
            );
            assert_eq!(
                constants.source[1].z,
                definition.constant("SparkleScale").unwrap().x
            );
            assert_eq!(
                constants.source[1].w,
                definition.constant("WarpAmount").unwrap().x
            );
            assert_eq!(
                constants.source[2].x,
                definition.constant("WarpScale").unwrap().x
            );
            assert_eq!(
                constants.source[2].y,
                definition.constant("LumaBoost").unwrap().x
            );
            for (image, source) in [
                (
                    &glow,
                    definition
                        .textures
                        .iter()
                        .find(|t| t.name_hash == Some(1825069840))
                        .unwrap(),
                ),
                (
                    &warp,
                    definition
                        .textures
                        .iter()
                        .find(|t| t.name.as_deref() == Some("WarpMap"))
                        .unwrap(),
                ),
                (
                    &sparkle,
                    definition
                        .textures
                        .iter()
                        .find(|t| t.name.as_deref() == Some("SparkleMap"))
                        .unwrap(),
                ),
            ] {
                let expected =
                    PathBuf::from(format!("images/{}.dds", source.image.replace('*', "_")));
                assert_eq!(image.path().unwrap().path(), expected);
                assert!(root.join(expected).is_file());
            }
            assert!(state.depth_write);
            assert!(state.blend.is_none());
            assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
        }
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_source_effect_model_materials_bind() {
        use super::super::native_model::NativeModelMaterial;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let mut counts = [0usize; 15];
        for entry in std::fs::read_dir(root.join("materials/mc")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|s| s != "json") {
                continue;
            }
            let definition = MaterialDefinition::read(&path).unwrap();
            let index = match definition.technique_set() {
                "mc_sw4_3d_scrolling_alpha2_distort_z58wq263" => 0,
                "mc_sw4_3d_scrolling_alpha2_distort_qwj8139f" => 1,
                "mc_sw4_3d_scrolling_falloff_zfeather_69456263" => 2,
                "mc_sw4_3d_scrolling_falloff_2ff9qe6w" => 3,
                "mc_sw4_3d_zm_elec_disc_w323fq8j" => 4,
                "mc_sw4_3d_light_sphere_46j58w7u" => 5,
                "mc_sw4_3d_portal_e96z3z92" => 6,
                "mc_sw4_3d_zmb_sky_pillar_on_6821fw2w" => 7,
                "mc_sw4_3d_zm_sphere_elec_9j108eq6" => 8,
                "mc_sw4_3d_zm_sphere_elec_535e0f02" => 9,
                "mc_sw4_3d_zm_ice_script_static_blizzard_45q67691" => 10,
                "mc_sw4_3d_zm_ice_script_static_877zz4j0" => 11,
                "mc_sw4_3d_zmb_sphere_tendril_j0598qze" => 12,
                "mc_sw4_3d_water_on_surface_reveal_77320z68" => 13,
                "mc_sw4_3d_unlit_scroll_jq69qew7" => 14,
                _ => continue,
            };
            let (material, state) = definition
                .model(app.world().resource::<AssetServer>(), Handle::default())
                .unwrap();
            let NativeModelMaterial::Effect {
                program,
                textures,
                constants,
                ..
            } = material
            else {
                panic!("exact effect-model program required");
            };
            assert_eq!(program as usize, index);
            assert!(!textures.is_empty() || index == 5);
            assert!(constants.source.iter().all(|value| value.is_finite()));
            assert!(
                textures
                    .iter()
                    .all(|(image, _)| root.join(image.path().unwrap().path()).is_file())
            );
            assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
            counts[index] += 1;
        }
        assert_eq!(counts, [1, 2, 5, 4, 1, 5, 4, 1, 1, 1, 2, 3, 3, 1, 4]);
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_model_additive_decal_binds() {
        use super::super::native_model::NativeModelMaterial;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let definition =
            MaterialDefinition::read(&root.join("materials/mc/mtl_p6_zm_tm_challenge_glow.json"))
                .unwrap();
        let (material, state) = definition
            .model(app.world().resource::<AssetServer>(), Handle::default())
            .unwrap();
        assert!(matches!(material, NativeModelMaterial::DecalAdd(_)));
        assert_eq!(state.polygon_offset, 1);
        assert_eq!(state.cull_mode, Some(Face::Back));
        assert!(!state.depth_write);
        assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
        assert_eq!(
            state.blend.unwrap(),
            BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::Zero,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add
                },
            }
        );
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_source_character_model_variants_bind() {
        use super::super::native_model::{NativeModelMaterial, SourceCharacterProgram};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let families = [
            (
                "mc_sw4_3d_char_skin_emberglow_7q60f126",
                SourceCharacterProgram::Ember,
                4,
            ),
            (
                "mc_sw4_3d_glass_emissive_jf185j5e",
                SourceCharacterProgram::Glass,
                1,
            ),
            (
                "mc_sw4_3d_char_hair_287j114q",
                SourceCharacterProgram::Hair,
                4,
            ),
            (
                "mc_sw4_3d_char_skin_3e77jj31",
                SourceCharacterProgram::SkinCutout,
                1,
            ),
            (
                "mc_sw4_3d_char_cloth_emissive_5ef7226z",
                SourceCharacterProgram::ClothEmissive,
                3,
            ),
            (
                "mc_sw4_3d_char_skin_emissive_189j7w84",
                SourceCharacterProgram::SkinEmissive,
                1,
            ),
            (
                "mc_sw4_3d_char_cloth_reveal2_emissive_5644z720",
                SourceCharacterProgram::RevealEmissive { cutout: false },
                2,
            ),
            (
                "mc_sw4_3d_char_cloth_reveal2_emissive_f88qewf4",
                SourceCharacterProgram::RevealEmissive { cutout: true },
                1,
            ),
        ];
        let mut counts = [0; 8];
        for entry in std::fs::read_dir(root.join("materials/mc")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let definition = MaterialDefinition::read(&path).unwrap();
            let Some(index) = families
                .iter()
                .position(|(family, _, _)| *family == definition.technique_set())
            else {
                continue;
            };
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            let NativeModelMaterial::SourceCharacter {
                program,
                base,
                specular,
                emission,
                temporal,
                constants,
                reveal,
            } = material
            else {
                panic!("{} lost its original character program", path.display());
            };
            assert_eq!(program, families[index].1);
            let assert_image = |image: &Handle<Image>, name: Option<&str>, hash: Option<u32>| {
                let input = definition
                    .textures
                    .iter()
                    .find(|t| match hash {
                        Some(hash) => t.name_hash == Some(hash),
                        None => t.name.as_deref() == name,
                    })
                    .unwrap();
                let expected =
                    PathBuf::from(format!("images/{}.dds", input.image.replace('*', "_")));
                assert_eq!(image.path().unwrap().path(), expected);
                assert!(root.join(expected).is_file());
            };
            assert_image(&base.normal_map, None, Some(2485960688));
            assert_image(
                &base.color_map,
                Some("colorMap"),
                if matches!(
                    program,
                    SourceCharacterProgram::Hair | SourceCharacterProgram::Glass
                ) {
                    None
                } else {
                    Some(4030327853)
                },
            );
            assert_image(&specular.0, Some("SpecularAndGloss"), None);
            if let Some((image, _)) = &emission {
                assert_image(
                    image,
                    None,
                    Some(if program == SourceCharacterProgram::Ember {
                        3837378363
                    } else {
                        3229466552
                    }),
                );
            }
            if let Some((image, _)) = &temporal {
                assert_image(
                    image,
                    None,
                    Some(if program == SourceCharacterProgram::Ember {
                        3617419774
                    } else {
                        2782436735
                    }),
                );
            }
            assert_eq!(
                emission.is_some(),
                !matches!(
                    program,
                    SourceCharacterProgram::Hair
                        | SourceCharacterProgram::SkinCutout
                        | SourceCharacterProgram::Glass
                )
            );
            assert_eq!(
                temporal.is_some(),
                matches!(
                    program,
                    SourceCharacterProgram::RevealEmissive { .. } | SourceCharacterProgram::Ember
                )
            );
            match program {
                SourceCharacterProgram::Fritz { .. } => unreachable!(),
                SourceCharacterProgram::Hair => assert_eq!(
                    constants.source[0].x,
                    definition.constant("ReflectionAmount").unwrap().x
                ),
                SourceCharacterProgram::SkinCutout | SourceCharacterProgram::SkinEmissive => {
                    assert_eq!(
                        constants.source[0].x,
                        definition
                            .constant_hash("Reflection_Amount", 211931156)
                            .unwrap()
                            .x
                    );
                    assert_eq!(
                        constants.source[0].y,
                        definition
                            .constant_hash("Specular_Amount", 1307013020)
                            .unwrap()
                            .x
                    );
                    assert_eq!(
                        constants.source[0].z,
                        definition
                            .constant_hash("Diffuse_Normal_Height_Facing", 1012612089)
                            .unwrap()
                            .x
                    );
                }
                SourceCharacterProgram::ClothEmissive => {
                    assert_eq!(constants.source[0].truncate(), Vec3::ONE)
                }
                SourceCharacterProgram::Ember => {
                    assert_eq!(
                        constants.source[2].w,
                        definition.constant("Seed_Value").unwrap().x
                    );
                    assert_eq!(
                        constants.source[2].y,
                        definition.constant("Flicker_Max").unwrap().x
                    );
                    assert_eq!(
                        constants.source[2].z,
                        definition.constant("Flicker_Min").unwrap().x
                    );
                    assert_eq!(
                        constants.source[3].x,
                        definition
                            .constant_hash("Emissiver_Amount", 227002554)
                            .unwrap()
                            .x
                    );
                    assert_image(&reveal.as_ref().unwrap().0, None, Some(690691323));
                }
                SourceCharacterProgram::Glass => {
                    assert_eq!(
                        constants.source[0].z,
                        definition
                            .constant_hash("EmissiveDepth", 3526985878)
                            .unwrap()
                            .x
                    );
                    assert_eq!(
                        constants.source[0].w,
                        definition.constant("hdrAmount").unwrap().x
                    );
                    assert_eq!(
                        constants.source[0].x,
                        definition.constant("ReflectionAmount").unwrap().x
                    );
                }
                SourceCharacterProgram::RevealEmissive { .. } => {
                    assert_eq!(
                        constants.source[4].x,
                        definition
                            .constant_hash("Diffuse_Emissive_Speed", 2226647986)
                            .unwrap()
                            .x
                    );
                    assert_eq!(
                        constants.source[3].w,
                        definition
                            .constant_hash("Diffuse_Emissive_Level", 2251066627)
                            .unwrap()
                            .x
                    );
                }
            }
            if matches!(
                program,
                SourceCharacterProgram::ClothEmissive | SourceCharacterProgram::SkinEmissive
            ) {
                assert_eq!(
                    constants.source[0].w,
                    definition
                        .constant_hash("Emissive_Amount", 2875883048)
                        .unwrap()
                        .x
                );
            }
            assert_eq!(
                state.blend.is_some(),
                matches!(
                    program,
                    SourceCharacterProgram::Hair | SourceCharacterProgram::Glass
                )
            );
            assert_eq!(
                state.depth_write,
                !matches!(
                    program,
                    SourceCharacterProgram::Hair | SourceCharacterProgram::Glass
                )
            );
            assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
            counts[index] += 1;
        }
        assert_eq!(counts, families.map(|(_, _, count)| count));
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_source_lit_model_variants_bind() {
        use super::super::native_model::{NativeModelMaterial, SourceLitProgram};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let families = [
            (
                "mc_lit_sm_b0c0n0s0_68f25z0u",
                SourceLitProgram::BlendedHero,
                2,
            ),
            (
                "mc_lit_sm_b0c0s0_q94f27qw",
                SourceLitProgram::GeometricHero,
                1,
            ),
            (
                "mc_lit_sm_r0c0n0s0_ze50j7jz",
                SourceLitProgram::NormalHero,
                4,
            ),
            (
                "mc_lit_sm_t0c0n0s0_f933q84u",
                SourceLitProgram::CutoutHero,
                1,
            ),
            (
                "mc_lit_sm_r0c0n0s0_7j414q0z",
                SourceLitProgram::ColorDetail,
                3,
            ),
            (
                "mc_lit_sm_r0c0d0n0s0_qwq4q449",
                SourceLitProgram::NormalDetail,
                1,
            ),
            (
                "mc_lit_sm_t0c0d0n0s0_68eww765",
                SourceLitProgram::CutoutNormalDetail,
                1,
            ),
        ];
        let mut counts = [0; 7];
        for entry in std::fs::read_dir(root.join("materials/mc")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let definition = MaterialDefinition::read(&path).unwrap();
            let Some(index) = families
                .iter()
                .position(|(family, _, _)| *family == definition.technique_set())
            else {
                continue;
            };
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            let NativeModelMaterial::SourceLit {
                program,
                color,
                normal,
                specular,
                detail,
                occlusion,
                detail_parameters,
                ..
            } = material
            else {
                panic!("{} lost its source-specific LIT program", path.display());
            };
            assert_eq!(program, families[index].1);
            assert_eq!(occlusion, definition.constant("occlusionAmount").unwrap());
            let assert_image = |handle: &Handle<Image>, name: &str| {
                let input = definition
                    .textures
                    .iter()
                    .find(|t| t.name.as_deref() == Some(name))
                    .unwrap();
                let expected =
                    PathBuf::from(format!("images/{}.dds", input.image.replace('*', "_")));
                assert_eq!(handle.path().unwrap().path(), expected);
                assert!(root.join(expected).is_file());
            };
            assert_image(&color.color_map, "colorMap");
            assert_image(&specular.0, "specularMap");
            if let Some((normal_map, _)) = &normal {
                assert_image(normal_map, "normalMap");
            }
            assert_eq!(normal.is_none(), program == SourceLitProgram::GeometricHero);
            if let Some((detail_map, _)) = &detail {
                assert_image(
                    detail_map,
                    if program == SourceLitProgram::ColorDetail {
                        "colorDetailMap"
                    } else {
                        "detailMap"
                    },
                );
            }
            assert_eq!(
                detail.is_some(),
                matches!(
                    program,
                    SourceLitProgram::ColorDetail
                        | SourceLitProgram::NormalDetail
                        | SourceLitProgram::CutoutNormalDetail
                )
            );
            assert_eq!(
                detail_parameters,
                if matches!(
                    program,
                    SourceLitProgram::NormalDetail | SourceLitProgram::CutoutNormalDetail
                ) {
                    definition.constant("detailScale").unwrap()
                } else {
                    Vec4::ZERO
                }
            );
            let blended = matches!(
                program,
                SourceLitProgram::BlendedHero | SourceLitProgram::GeometricHero
            );
            assert_eq!(state.blend.is_some(), blended);
            assert_eq!(state.depth_write, !blended);
            assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
            counts[index] += 1;
        }
        assert_eq!(counts, families.map(|(_, _, count)| count));
    }

    #[test]
    #[ignore = "requires exported source materials"]
    fn exported_weapon_authored_reticle_materials_bind() {
        use super::super::native_model::NativeModelMaterial;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        for relative in [
            "materials/mc/mtl_t6_attach_optic_rangefinder_reticle2.json",
            "materials/mc/mtl_t6_attach_optic_acog_reticle.json",
            "materials/mc/mtl_t6_attach_optic_holo_reticle.json",
            "materials/mc/mtl_t6_attach_optic_m32_reticle.json",
            "materials/mc/mtl_t6_attach_optic_reflex_reticle.json",
            "materials/mc/mtl_t6_attach_optic_mms_reticle.json",
        ] {
            let definition = MaterialDefinition::read(&root.join(relative)).unwrap();
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            let NativeModelMaterial::Reticle {
                textures,
                constants,
                ..
            } = material
            else {
                panic!("{relative}: wrong program");
            };
            assert_ne!(textures[0].0, textures[1].0);
            assert_eq!(
                constants.source[0].w,
                definition
                    .constant_hash("Emissive_Amount", 2875883048)
                    .unwrap()
                    .x
            );
            assert_eq!(
                constants.source[1].y,
                definition
                    .constant_hash("Color_Map_Scale", 4000797241)
                    .unwrap()
                    .x
            );
            assert_eq!(
                constants.source[2].x,
                definition.constant("Background").unwrap().x
            );
            assert!(!state.depth_write);
            assert_eq!(
                state.blend.unwrap().color.dst_factor,
                BlendFactor::OneMinusSrcAlpha
            );
        }
    }

    #[test]
    #[ignore = "requires exported source materials"]
    fn exported_weapon_optic_and_single_layer_materials_bind() {
        use super::super::native_model::NativeModelMaterial;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        for relative in [
            "materials/mc/mtl_t6_attach_optic_acog_lens.json",
            "materials/mc/mtl_t6_attach_optic_aimpoint_lens.json",
            "materials/mc/mtl_t6_attach_optic_holo_lens.json",
            "materials/mc/mtl_t6_attach_optic_m32_lens.json",
            "materials/mc/mtl_t6_attach_optic_mms_lens.json",
            "materials/mc/mtl_t6_attach_optic_rangefinder_lens.json",
            "materials/mc/mtl_t6_attach_optic_reflex_lens.json",
            "materials/mc/mtl_t6_wpn_sniper_dsr50_scope_lens.json",
            "materials/mc/mtl_p6_zm_tm_runes.json",
        ] {
            let definition = MaterialDefinition::read(&root.join(relative)).unwrap();
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            match material {
                NativeModelMaterial::OpticLens {
                    textures, controls, ..
                } => {
                    assert_eq!(
                        controls.x,
                        definition
                            .constant_hash("Specular_Amount", 1307013020)
                            .unwrap()
                            .x
                    );
                    assert_eq!(
                        controls.y,
                        definition
                            .constant_hash("Reflection_Amount", 211931156)
                            .unwrap()
                            .x
                    );
                    assert_eq!(
                        controls.z,
                        definition
                            .constant_hash("Specular_Decay_Threshold", 994574754)
                            .unwrap()
                            .x
                    );
                    assert_ne!(textures[2].0, textures[3].0);
                }
                NativeModelMaterial::SingleLayer { constants, .. } => {
                    assert_eq!(
                        constants.source[1].w,
                        definition.constant("hdrAmount").unwrap().x
                    );
                    assert_eq!(
                        constants.source[1].z,
                        definition.constant("Layer1Alpha").unwrap().x
                    );
                    assert_eq!(
                        constants.source[3].x,
                        definition.constant("Layer1Depth").unwrap().x
                    );
                }
                _ => panic!("{relative}: wrong program"),
            }
            assert!(state.blend.is_some());
        }
    }

    #[test]
    #[ignore = "requires exported source materials"]
    fn exported_weapon_staff_materials_bind() {
        use super::super::native_model::NativeModelMaterial;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        for relative in [
            "materials/mc/mtl_t6_wpn_zmb_staff_crystal_air.json",
            "materials/mc/mtl_t6_wpn_zmb_staff_crystal_bolt.json",
            "materials/mc/mtl_t6_wpn_zmb_staff_crystal_fire.json",
            "materials/mc/mtl_t6_wpn_zmb_staff_crystal_water.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_air.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_air3.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_bolt.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_bolt3.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_fire.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_fire1.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_fire3.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_water.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_crystal_water3.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_pwr_srce_air.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_pwr_srce_bolt.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_pwr_srce_fire.json",
            "materials/mc/mtl_t6_wpn_zmb_upg_pwr_srce_water.json",
        ] {
            let definition = MaterialDefinition::read(&root.join(relative)).unwrap();
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            let NativeModelMaterial::Staff3Layer {
                textures,
                off,
                constants,
                emission_on,
            } = material
            else {
                panic!("{relative}: wrong program");
            };
            assert_eq!(
                constants.source[10].w,
                definition
                    .constant_hash("Charge_Level", 963960723)
                    .unwrap()
                    .x
            );
            assert_eq!(
                constants.source[9].z,
                definition
                    .constant_hash("Emissive_Amount_Off", 996647064)
                    .unwrap()
                    .x
            );
            assert_eq!(
                emission_on,
                definition
                    .constant_hash("Emissive_Amount_On", 811104566)
                    .unwrap()
            );
            assert_ne!(textures.layer3_map, off.0);
            assert_eq!(constants.source[5].x, 0.); // unused original Layer3Alpha
            assert!(state.depth_write);
            assert!(state.blend.is_none());
        }
    }

    #[test]
    #[ignore = "requires exported source materials"]
    fn exported_weapon_viewmodel_unlit_materials_bind() {
        use super::super::native_model::{NativeModelMaterial, NativeModelUnlitProgram};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let paths = [
            "materials/mc/mat_veh_t6_dlc_zm_drone_unlit_vm.json",
            "materials/mc/mat_veh_t6_dlc_zm_drone_water_vm.json",
            "materials/mc/mtl_t6_attach_optic_acog_crosshair.json",
            "materials/mc/mtl_t6_attach_optic_acog_shade.json",
            "materials/mc/mtl_t6_attach_zmb_mc96_reticle.json",
            "materials/mc/mtl_t6_attach_tritium_grn_glo.json",
            "materials/mc/mtl_t6_attach_tritium_red_glo.json",
            "materials/mc/mtl_t6_attach_tritium_red_txt.json",
        ];
        for relative in paths {
            let path = root.join(relative);
            let definition = MaterialDefinition::read(&path).unwrap();
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            let NativeModelMaterial::Unlit {
                base,
                program,
                scale_rgb,
            } = material
            else {
                panic!("{relative}: wrong program");
            };
            assert_eq!(
                scale_rgb,
                definition
                    .constant_hash("Emissive_Amount", 2875883048)
                    .unwrap()
            );
            assert!(
                base.color_map
                    .path()
                    .unwrap()
                    .path()
                    .to_string_lossy()
                    .contains("images/")
            );
            assert!(!state.depth_write);
            if matches!(program, NativeModelUnlitProgram::ViewmodelOpaque) {
                assert_eq!(state.polygon_offset, 1);
                assert_eq!(
                    state.blend.unwrap().color.src_factor,
                    BlendFactor::OneMinusDst
                );
            }
        }
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_model_unlit_blend_materials_bind() {
        use super::super::native_model::{NativeModelMaterial, NativeModelUnlitProgram};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        for (name, additive, scale, offset, cull) in [
            ("mat_veh_t6_dlc_zm_drone_water", false, 50., 0, None),
            ("missing_fx", false, 8., 1, Some(Face::Back)),
            (
                "mat_veh_t6_dlc_zm_drone_unlit",
                true,
                50.,
                0,
                Some(Face::Back),
            ),
            (
                "mtl_t6_attach_trinium_sight_green",
                true,
                4.,
                1,
                Some(Face::Back),
            ),
        ] {
            let definition =
                MaterialDefinition::read(&root.join(format!("materials/mc/{name}.json"))).unwrap();
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            let NativeModelMaterial::Unlit {
                base,
                program,
                scale_rgb,
            } = material
            else {
                panic!("{name} requires the original color-only unlit stream");
            };
            assert!(
                program
                    == if additive {
                        NativeModelUnlitProgram::Add
                    } else {
                        NativeModelUnlitProgram::Blend
                    }
            );
            assert_eq!(scale_rgb, Vec4::new(scale, 0., 0., 1.));
            assert_eq!(definition.sort_key, 40);
            assert_eq!(definition.camera_region, "emissiveTrans");
            assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
            assert!(!state.depth_write);
            assert_eq!(state.cull_mode, cull);
            assert_eq!(state.polygon_offset, offset);
            assert_eq!(state.color_write, ColorWrites::ALL);
            let source = &definition.textures[0];
            let expected = PathBuf::from(format!("images/{}.dds", source.image.replace('*', "_")));
            assert_eq!(base.color_map.path().unwrap().path(), expected);
            assert!(root.join(expected).is_file());
            assert_eq!(
                base.color_sampler.address_mode_u,
                if source.sampler_state.clamp_u {
                    AddressMode::ClampToEdge
                } else {
                    AddressMode::Repeat
                }
            );
            assert_eq!(
                state.blend.unwrap(),
                BlendState {
                    color: BlendComponent {
                        src_factor: if additive {
                            BlendFactor::One
                        } else {
                            BlendFactor::SrcAlpha
                        },
                        dst_factor: if additive {
                            BlendFactor::One
                        } else {
                            BlendFactor::OneMinusSrcAlpha
                        },
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: if additive {
                            BlendFactor::Zero
                        } else {
                            BlendFactor::OneMinusDstAlpha
                        },
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                }
            );
        }
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_model_glass_materials_bind() {
        use super::super::native_model::{NativeModelMaterial, NormalSpecularProgram};

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        let mut counts = [0; 2];
        let mut names = std::collections::BTreeSet::new();
        for entry in std::fs::read_dir(root.join("materials/mc")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let definition = MaterialDefinition::read(&path).unwrap();
            let reflection = match definition.technique_set() {
                "mc_lit_sm_b0c0n0s0_91j5z9q9" => false,
                "mc_lit_sm_b0c0n0s0_21fz2e61" => true,
                _ => continue,
            };
            counts[usize::from(reflection)] += 1;
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            names.insert(name.clone());
            let (material, state) = definition.model(server, Handle::default()).unwrap();
            let NativeModelMaterial::NormalSpecular {
                program,
                base,
                specular_map,
                occlusion_amount,
                ..
            } = material
            else {
                panic!("{name} must retain normal, specular, model atlas and reflection inputs");
            };
            assert!(matches!(program, NormalSpecularProgram::Blended {
                unattenuated_reflection,
            } if unattenuated_reflection == reflection));
            assert_eq!(occlusion_amount.y, 1.0);
            assert_eq!(occlusion_amount.z, 1.0);
            for (semantic, handle) in [
                ("colorMap", &base.color_map),
                ("normalMap", &base.normal_map),
                ("specularMap", &specular_map),
            ] {
                let source = definition
                    .textures
                    .iter()
                    .find(|t| t.semantic == semantic)
                    .unwrap();
                let expected =
                    PathBuf::from(format!("images/{}.dds", source.image.replace('*', "_")));
                assert_eq!(handle.path().unwrap().path(), expected);
                assert!(root.join(expected).is_file());
            }
            assert!(definition.world_pass() == super::super::native::NativeWorldPass::Translucent);
            assert!(!state.depth_write);
            assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
            assert_eq!(state.color_write, ColorWrites::ALL);
            assert_eq!(
                state.cull_mode,
                if name == "gfx_impact_glass01_riotshield" {
                    None
                } else {
                    Some(Face::Back)
                }
            );
            assert_eq!(
                state.polygon_offset,
                if matches!(name.as_str(), "dec_glass02" | "com_glass_shattered_outside") {
                    1
                } else if name.starts_with("gfx_impact_") && name != "gfx_impact_glass01_riotshield"
                {
                    2
                } else {
                    0
                }
            );
            assert_eq!(
                state.blend.unwrap(),
                BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrcAlpha,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::OneMinusDstAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                }
            );
        }
        assert_eq!(
            counts,
            [12, 1],
            "check both complete exported shader families"
        );
        for name in [
            "com_glass_shattered_outside",
            "mtl_zombie_vending_sleight_glass",
            "mtl_p6_zm_tm_staff_projectile_ice",
            "dec_glass02",
        ] {
            assert!(
                names.contains(name),
                "missing startup glass or adjacent reflection variant {name}"
            );
        }
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_model_multiply_decal_binds() {
        use super::super::native_model::NativeModelMaterial;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        let definition =
            MaterialDefinition::read(&root.join("materials/mc/zm_tm_decal_quaternary.json"))
                .unwrap();
        let (material, state) = definition
            .model(app.world().resource::<AssetServer>(), Handle::default())
            .unwrap();
        assert!(matches!(material, NativeModelMaterial::DecalMultiply(_)));
        assert_eq!(state.polygon_offset, 1);
        assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
        assert!(!state.depth_write);
        assert_eq!(
            state.blend.unwrap(),
            BlendState {
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
            }
        );
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_world_decal_materials_bind() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let catalog: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("fx_catalog.json")).unwrap()).unwrap();
        // FxElemMarkVisuals has model material first, world material second.
        let visual = &catalog["effects"]["explosions/fx_grenadeexp_default"]["elements"][2];
        assert_eq!(visual["type"], 11);
        let name = visual["visuals"][0]["decal_materials"][1].as_str().unwrap();
        assert_eq!(name, "wc/gfx_crater_blacktop");
        let mut names = std::collections::BTreeSet::new();
        for effect in catalog["effects"].as_object().unwrap().values() {
            for element in effect["elements"].as_array().unwrap() {
                if element["type"] != 11 {
                    continue;
                }
                for visual in element["visuals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|v| !v.is_null())
                {
                    names.insert(
                        visual["decal_materials"][1]
                            .as_str()
                            .unwrap()
                            .trim_start_matches(','),
                    );
                }
            }
        }
        assert_eq!(
            names.len(),
            27,
            "Check the full exported world decal material inventory"
        );
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin {
                file_path: root.to_string_lossy().into_owned(),
                ..default()
            },
        ))
        .init_asset::<Image>();
        for name in names {
            let definition =
                MaterialDefinition::read(&root.join(format!("materials/{name}.json"))).unwrap();
            let (material, state) = definition
                .world(
                    app.world().resource::<AssetServer>(),
                    Some(Handle::default()),
                    Handle::default(),
                )
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(matches!(
                material,
                NativeWorldMaterial::GeometricDiffuse { .. }
                    | NativeWorldMaterial::Diffuse(_)
                    | NativeWorldMaterial::GeometricSpecular(_)
                    | NativeWorldMaterial::NormalSpecular { .. }
            ));
            assert!(matches!(
                definition.world_pass(),
                super::super::native::NativeWorldPass::Translucent
            ));
            let source = &definition.state_bits[definition.state_bits_entry[4] as usize];
            assert_eq!(
                state.polygon_offset,
                if source.polygon_offset == "offset2" {
                    2
                } else {
                    0
                }
            );
            assert_eq!(
                state.cull_mode,
                if source.cull_face == "none" {
                    None
                } else {
                    Some(Face::Back)
                }
            );
            assert!(!state.depth_write);
            assert_eq!(state.depth_compare, CompareFunction::GreaterEqual);
            assert_eq!(
                state.blend.unwrap().color,
                BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                }
            );
            for texture in &definition.textures {
                assert!(
                    root.join(format!("images/{}.dds", texture.image.replace('*', "_")))
                        .is_file(),
                    "{name}: missing {}",
                    texture.image
                );
            }
        }
        println!("Bound all 27 original world decal materials and checked their image files");
    }
    #[test]
    #[ignore = "requires exported original source assets"]
    fn exported_mms_actor_thermal_links_preserve_inputs_and_depth_state() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
        let server = app.world().resource::<AssetServer>();
        for (base, family, emissive, detail) in [
            (
                "mtl_c_zom_tomb_german_head_1_u",
                super::super::native_sonar::SonarProgram::Thermal,
                3.,
                0.30000001192092896,
            ),
            (
                "mtl_c_gen_insidemouth",
                super::super::native_sonar::SonarProgram::ThermalChar,
                3.,
                0.3700000047683716,
            ),
            (
                "mtl_gen_eye_iris_blue",
                super::super::native_sonar::SonarProgram::ThermalChar,
                4.010000228881836,
                0.3700000047683716,
            ),
        ] {
            let root = Path::new("target/assets/materials");
            let source = MaterialDefinition::read(&root.join(format!("mc/{base}.json"))).unwrap();
            let name = source.thermal_material().unwrap();
            let thermal = MaterialDefinition::read(&root.join(format!("{name}.json"))).unwrap();
            let inputs = std::collections::BTreeMap::from([(name.to_owned(), thermal)]);
            let material = source.sonar_material(server, &inputs).unwrap().unwrap();
            assert!(material.program == family);
            assert_eq!(material.constant, Vec4::new(emissive, detail, 0., 0.));
            assert_eq!(material.textures.len(), 3);
            assert!(material.state.depth_write);
            assert_eq!(material.state.depth_compare, CompareFunction::GreaterEqual);
            assert_eq!(material.state.front_face, FrontFace::Cw);
            let blend = material.state.blend.unwrap();
            assert_eq!(blend.color.src_factor, BlendFactor::SrcAlpha);
            assert_eq!(blend.color.dst_factor, BlendFactor::OneMinusSrcAlpha);
            assert_eq!(blend.alpha.operation, BlendOperation::Max);
            assert!(
                material.textures[0]
                    .0
                    .path()
                    .unwrap()
                    .path()
                    .to_string_lossy()
                    .contains("_ir.dds")
            );
            assert!(
                material.textures[1]
                    .0
                    .path()
                    .unwrap()
                    .path()
                    .to_string_lossy()
                    .ends_with("_n.dds")
            );
            assert!(
                material.textures[2]
                    .0
                    .path()
                    .unwrap()
                    .path()
                    .to_string_lossy()
                    .contains("thermal_gradient")
            );
            assert!(material.textures[2].1.address_mode_u == AddressMode::ClampToEdge);
        }
    }
}
