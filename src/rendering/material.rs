//! Original T6 material inputs for the world LIT programs ported to WESL.
use std::path::{Path, PathBuf};

use bevy::{
    asset::{AssetPath, RenderAssetUsages},
    image::ImageLoaderSettings,
    prelude::*,
    render::render_resource::{
        AddressMode, ColorWrites, CompareFunction, Face, FilterMode, FrontFace, MipmapFilterMode,
        SamplerDescriptor,
    },
};
use serde::Deserialize;

use super::native::{
    NativeWorldColorLayerTextures, NativeWorldDiffuseTextures,
    NativeWorldGeometricSpecularTextures, NativeWorldLayerTextures, NativeWorldMaterial,
    NativeWorldSpecularLayerTextures, NativeWorldState, NativeWorldTextures,
};

#[derive(Deserialize)]
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
    literal: [f32; 4],
}

impl MaterialDefinition {
    pub(crate) fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path)
            .map_err(|error| format!("Read original material {}: {error}", path.display()))?;
        let mut material: Self = serde_json::from_slice(&bytes)
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

    pub(crate) fn model(
        &self,
        server: &AssetServer,
        probe: Handle<Image>,
    ) -> Result<(super::native_model::NativeModelMaterial, NativeWorldState), String> {
        let bind = || {
            if self.technique_set() != "mc_lit_sm_r0c0n0x0_q361191u" {
                return Err(format!(
                    "Port original model LIT techniqueSet {}",
                    self.technique_set()
                ));
            }
            let state = self.lit_state()?;
            let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
            let (normal_map, normal_sampler) = self.texture(server, "normalMap", "normalMap")?;
            Ok((
                super::native_model::NativeModelMaterial {
                    color_map,
                    color_sampler,
                    normal_map,
                    normal_sampler,
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
        lightmap: Handle<Image>,
        probe: Handle<Image>,
    ) -> Result<(NativeWorldMaterial, NativeWorldState), String> {
        let bind = || {
            let family = self.technique_set();
            if !matches!(
                family,
                "wpc_lit_sm_r0c0n0x0_f488034e"
                    | "wpc_lit_sm_r0c0n0_80fe30wz"
                    | "wpc_lit_sm_r0c0n0s0_1zzj1138"
                    | "wpc_lit_sm_r0c0s0_qfqz7850"
                    | "lit_sm_r0c0n0x0_m1c1"
                    | "lit_sm_r0c0n0x0_b1c1"
                    | "lit_sm_r0c0n0x0_a1c1"
                    | "lit_sm_r0c0n0x0_b1c1n1"
                    | "lit_sm_r0c0n0x0_b1c1n1v1"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1"
                    | "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2"
                    | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2"
            ) {
                return Err(format!(
                    "Port the original LIT technique for techniqueSet {family} before binding this world material"
                ));
            }
            let state = self.lit_state()?;
            let reveal = match family {
                "lit_sm_r0c0n0x0_b1c1n1v1" | "lit_sm_r0c0n0x0_b1c1n1s1v1" => {
                    Some(self.constant("alphaRevealParms1")?)
                }
                _ => None,
            };
            let (color_map, color_sampler) = self.texture(server, "colorMap", "colorMap")?;
            if family == "wpc_lit_sm_r0c0s0_qfqz7850" {
                let (specular_map, specular_sampler) =
                    self.texture(server, "specularMap", "specularMap")?;
                return Ok((
                    NativeWorldMaterial::GeometricSpecular(NativeWorldGeometricSpecularTextures {
                        color_map,
                        color_sampler,
                        specular_map,
                        specular_sampler,
                        lightmap_secondary: lightmap,
                        lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                        reflection_probe: probe,
                        reflection_sampler: code_sampler(MipmapFilterMode::Linear),
                    }),
                    state,
                ));
            }
            let (normal_map, normal_sampler) = self.texture(server, "normalMap", "normalMap")?;
            if family == "wpc_lit_sm_r0c0n0_80fe30wz" {
                return Ok((
                    NativeWorldMaterial::Diffuse(NativeWorldDiffuseTextures {
                        color_map,
                        color_sampler,
                        normal_map,
                        normal_sampler,
                        lightmap_secondary: lightmap,
                        lightmap_sampler: code_sampler(MipmapFilterMode::Nearest),
                    }),
                    state,
                ));
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
            if family == "wpc_lit_sm_r0c0n0x0_f488034e" {
                return Ok((NativeWorldMaterial::Lightmapped(base), state));
            }
            if family == "wpc_lit_sm_r0c0n0s0_1zzj1138" {
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
                "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2"
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
                let material = if family == "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2" {
                    NativeWorldMaterial::ThreeLayerReveal {
                        base,
                        layer1,
                        layer2,
                        alpha_reveal_parms2,
                    }
                } else {
                    let (specular_map, specular_sampler) =
                        self.texture(server, "specularMap1", "specularMap")?;
                    NativeWorldMaterial::ThreeLayerDoubleReveal {
                        base,
                        layer1: NativeWorldSpecularLayerTextures {
                            layer: layer1,
                            specular_map,
                            specular_sampler,
                        },
                        layer2,
                        alpha_reveal_parms1: self.constant("alphaRevealParms1")?,
                        alpha_reveal_parms2,
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
                ("lit_sm_r0c0n0x0_b1c1n1", None) => NativeWorldMaterial::Layered { base, layer },
                ("lit_sm_r0c0n0x0_b1c1n1v1", Some(alpha_reveal_parms1)) => {
                    NativeWorldMaterial::RevealLayered {
                        base,
                        layer,
                        alpha_reveal_parms1,
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
                _ => unreachable!("world family and its required constant were selected together"),
            };
            Ok((material, state))
        };
        bind().map_err(|error| format!("Material {}: {error}", self.path.display()))
    }

    fn lit_state(&self) -> Result<NativeWorldState, String> {
        if self.camera_region != "litOpaque" {
            return Err(format!(
                "Add the original {} camera-region pass before binding this material",
                self.camera_region
            ));
        }
        // T6 MATERIAL_TECHNIQUE_LIT = 4; stateBitsEntry indexes the state table,
        // independently of the technique's position in that table.
        let index = self
            .state_bits_entry
            .get(4)
            .copied()
            .filter(|index| *index >= 0)
            .ok_or_else(|| "Original stateBitsEntry[4] has no LIT state".to_owned())?;
        let state = self.state_bits.get(index as usize).ok_or_else(|| {
            format!("Original stateBitsEntry[4] references missing stateBits[{index}]")
        })?;
        if state.blend_op_rgb != "disabled" || state.blend_op_alpha != "disabled" {
            return Err(
                "Add this material's original blend state to the native pipeline".to_owned(),
            );
        }
        if state.alpha_test != "disabled" {
            return Err(format!(
                "Port original alphaTest {} into this material's WESL program",
                state.alpha_test
            ));
        }
        if state.polymode_line || state.polygon_offset != "offset0" {
            return Err(format!(
                "Add original raster state polymodeLine={}, polygonOffset={} to the native pipeline",
                state.polymode_line, state.polygon_offset
            ));
        }
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
            depth_compare,
            depth_write: state.depth_write,
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
        let mut matches = self
            .textures
            .iter()
            .filter(|texture| texture.name.as_deref() == Some(name));
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

    fn constant(&self, name: &str) -> Result<Vec4, String> {
        let mut matches = self
            .constants
            .iter()
            .filter(|constant| constant.name.as_deref() == Some(name));
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

fn code_sampler(mipmap_filter: MipmapFilterMode) -> SamplerDescriptor<'static> {
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
