//! Bind the original FX material record to its recovered shader family.
use std::{collections::BTreeMap, path::Path};

use bevy::{prelude::*, render::render_resource::*};
use serde::Deserialize;

use crate::rendering::{material::image, native_effect::*};

#[derive(Resource)]
pub(super) struct Materials(BTreeMap<String, Result<Handle<NativeEffectMaterial>, String>>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Definition {
    #[serde(rename = "_game")]
    game: String,
    #[serde(rename = "_type")]
    kind: String,
    #[serde(rename = "_version")]
    version: u32,
    technique_set: String,
    camera_region: String,
    state_bits_entry: Vec<i8>,
    state_bits: Vec<serde_json::Value>,
    textures: Vec<Texture>,
    constants: Vec<Constant>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Texture {
    name: Option<String>,
    semantic: String,
    image: String,
    sampler_state: SourceSampler,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceSampler {
    clamp_u: bool,
    clamp_v: bool,
    clamp_w: bool,
    filter: String,
    mip_map: String,
}

#[derive(Deserialize)]
struct Constant {
    name: Option<String>,
    literal: [f32; 4],
}

impl Materials {
    pub(super) fn load<'a>(
        root: &Path,
        names: impl Iterator<Item = &'a String>,
        server: &AssetServer,
        materials: &mut Assets<NativeEffectMaterial>,
    ) -> Self {
        Self(
            names
                .map(|name| {
                    // The unchanged SDK material_file_stem uses the same '*' spelling
                    // as image_path and preserves source mc/ and wc/ subdirectories.
                    let path = root
                        .join("materials")
                        .join(format!("{}.json", name.replace('*', "_")));
                    let definition: Definition = crate::origins::read(&path);
                    let result = definition
                        .bind(server)
                        .map(|material| materials.add(material))
                        .map_err(|error| {
                            format!("Original FX material {name} ({}): {error}", path.display())
                        });
                    (name.clone(), result)
                })
                .collect(),
        )
    }

    pub(super) fn get(&self, name: &str) -> Result<Handle<NativeEffectMaterial>, String> {
        self.0
            .get(name.trim_start_matches(','))
            .ok_or_else(|| format!("Required original FX material is missing: {name}"))?
            .clone()
    }
}

impl Definition {
    fn bind(&self, server: &AssetServer) -> Result<NativeEffectMaterial, String> {
        if self.game != "t6" || self.kind != "material" || self.version != 1 {
            return Err("requires the original T6 material.v1 record".into());
        }
        let program = match self.technique_set.as_str() {
            "effect_w77q49e8" => NativeEffectProgram::Zfeather,
            "effect_jqj65q53" => NativeEffectProgram::Additive,
            "effect_5e098749" => NativeEffectProgram::AdditiveFeather,
            "effect_737e1je4" => NativeEffectProgram::LocalAdditiveFeather,
            "effect_z096f2e2" => NativeEffectProgram::OutdoorFeather,
            "effect_50567j38" | "effect_z0z25860" => NativeEffectProgram::SimpleFeather,
            "effect_ezz8jfwj" => NativeEffectProgram::EyeFeather,
            "effect_910219f6" => NativeEffectProgram::EyeAdditive,
            "effect_8e1qf0qq" => NativeEffectProgram::SimpleAdditive,
            "particlecloud_ew972295" => NativeEffectProgram::CloudRain,
            "particlecloud_j32z635q" => NativeEffectProgram::CloudSnow,
            _ => {
                return Err(format!(
                    "port the original {} UNLIT/EMISSIVE programs before submitting this effect",
                    self.technique_set
                ));
            }
        };
        if self.camera_region != "emissiveFx" {
            return Err(format!(
                "requires original camera region {}",
                self.camera_region
            ));
        }
        let mut selected = None;
        for slot in [2, 3] {
            // MATERIAL_TECHNIQUE_UNLIT / EMISSIVE
            let index = self
                .state_bits_entry
                .get(slot)
                .copied()
                .filter(|v| *v >= 0)
                .ok_or_else(|| format!("required stateBitsEntry[{slot}] is absent"))?;
            let state = self.state_bits.get(index as usize).ok_or_else(|| {
                format!("stateBitsEntry[{slot}] references missing state {index}")
            })?;
            if selected.is_some_and(|previous| previous != state) {
                return Err("UNLIT and EMISSIVE states require separate passes".into());
            }
            selected = Some(state);
        }
        let state = selected.expect("two required source technique slots were selected");
        // Validate the original family contract; shader identity does not imply
        // blend identity (z0z25860 shares 50567j38 shaders but screens RGB).
        let additive = program.additive();
        let screen = self.technique_set == "effect_z0z25860";
        let blend = BlendState {
            color: BlendComponent {
                src_factor: if screen {
                    BlendFactor::OneMinusDst
                } else if additive {
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
        };
        for (field, expected) in [
            (
                "srcBlendRgb",
                if screen {
                    "invdestcolor"
                } else if additive {
                    "one"
                } else {
                    "srcalpha"
                },
            ),
            ("dstBlendRgb", if additive { "one" } else { "invsrcalpha" }),
            ("blendOpRgb", "add"),
            (
                "srcBlendAlpha",
                if additive { "zero" } else { "invdestalpha" },
            ),
            ("dstBlendAlpha", "one"),
            ("blendOpAlpha", "add"),
            ("alphaTest", if additive { "disabled" } else { "gt0" }),
            ("depthTest", "less_equal"),
            ("polygonOffset", "offset0"),
        ] {
            if state.get(field).and_then(|v| v.as_str()) != Some(expected) {
                return Err(format!(
                    "port source {field}={:?} before submitting this effect",
                    state.get(field)
                ));
            }
        }
        for (field, expected) in [
            ("colorWriteRgb", true),
            ("colorWriteAlpha", true),
            ("polymodeLine", false),
            ("depthWrite", false),
        ] {
            if state.get(field).and_then(|v| v.as_bool()) != Some(expected) {
                return Err(format!(
                    "port source {field}={:?} before submitting this effect",
                    state.get(field)
                ));
            }
        }
        let cull_mode = match state.get("cullFace").and_then(|v| v.as_str()) {
            Some("back") => Some(Face::Back),
            Some("none") => None,
            other => return Err(format!("port source cullFace={other:?}")),
        };
        let stencil_face = |value: &serde_json::Value| -> Result<StencilFaceState, String> {
            for field in ["pass", "fail", "zfail"] {
                if value.get(field).and_then(|v| v.as_str()) != Some("keep") {
                    return Err(format!("port original stencil operation {field}"));
                }
            }
            if value.get("func").and_then(|v| v.as_str()) != Some("equal") {
                return Err("port original stencil comparison".into());
            }
            Ok(StencilFaceState {
                compare: CompareFunction::Equal,
                fail_op: StencilOperation::Keep,
                depth_fail_op: StencilOperation::Keep,
                pass_op: StencilOperation::Keep,
            })
        };
        let stencil = match (state.get("stencilFront"), state.get("stencilBack")) {
            (None, None) => None,
            (Some(front), back) => {
                let front = stencil_face(front)?;
                // Original state construction mirrors front when back is absent.
                let back = back.map(stencil_face).transpose()?.unwrap_or(front);
                Some(NativeEffectStencil {
                    front,
                    back,
                    read_mask: 0xff,
                    write_mask: 0xff,
                    reference: 0,
                })
            }
            (None, Some(_)) => return Err("original back stencil has no front state".into()),
        };
        let mut textures = self
            .textures
            .iter()
            .filter(|v| v.name.as_deref() == Some("colorMap"));
        let texture = textures.next().ok_or("required colorMap is absent")?;
        if textures.next().is_some() || texture.semantic != "colorMap" || texture.image.is_empty() {
            return Err("required colorMap must have one original colorMap image".into());
        }
        let feather_parms = if matches!(
            program,
            NativeEffectProgram::Zfeather
                | NativeEffectProgram::AdditiveFeather
                | NativeEffectProgram::LocalAdditiveFeather
                | NativeEffectProgram::OutdoorFeather
                | NativeEffectProgram::SimpleFeather
                | NativeEffectProgram::EyeFeather
        ) {
            let mut constants = self
                .constants
                .iter()
                .filter(|v| v.name.as_deref() == Some("featherParms"));
            let constant = constants.next().ok_or("required featherParms is absent")?;
            if constants.next().is_some() || !constant.literal.iter().all(|v| v.is_finite()) {
                return Err("required featherParms must be one finite source float4".into());
            }
            Vec4::from_array(constant.literal)
        } else {
            Vec4::ZERO
        }; // Cloud programs consume no material constants.
        let mut additive_parms = [Vec4::ZERO; 4];
        if matches!(
            program,
            NativeEffectProgram::Additive
                | NativeEffectProgram::AdditiveFeather
                | NativeEffectProgram::LocalAdditiveFeather
                | NativeEffectProgram::EyeFeather
                | NativeEffectProgram::EyeAdditive
        ) {
            for (output, name) in additive_parms.iter_mut().zip([
                "falloffParms",
                "falloffBeginColor",
                "falloffEndColor",
                "eyeOffsetParms",
            ]) {
                if matches!(
                    program,
                    NativeEffectProgram::EyeFeather | NativeEffectProgram::EyeAdditive
                ) && name != "eyeOffsetParms"
                {
                    continue;
                }
                if name == "eyeOffsetParms"
                    && matches!(
                        program,
                        NativeEffectProgram::AdditiveFeather
                            | NativeEffectProgram::LocalAdditiveFeather
                    )
                {
                    continue;
                }
                let mut values = self
                    .constants
                    .iter()
                    .filter(|v| v.name.as_deref() == Some(name));
                let value = values
                    .next()
                    .ok_or_else(|| format!("required {name} is absent"))?;
                if values.next().is_some() || !value.literal.iter().all(|v| v.is_finite()) {
                    return Err(format!("required {name} must be one finite source float4"));
                }
                *output = Vec4::from_array(value.literal);
            }
        }
        Ok(NativeEffectMaterial {
            program,
            color_map: image(server, &texture.image),
            color_sampler: texture.sampler_state.descriptor()?,
            feather_parms,
            additive_parms,
            state: NativeEffectState {
                blend,
                cull_mode,
                front_face: FrontFace::Cw,
                stencil,
            },
        })
    }
}

impl SourceSampler {
    fn descriptor(&self) -> Result<SamplerDescriptor<'static>, String> {
        // Original 00432ba0 sampler table at default r_texFilterQuality = 1.
        let (filter, anisotropy_clamp) = match self.filter.as_str() {
            "disabled" | "nearest" => (FilterMode::Nearest, 1),
            "linear" => (FilterMode::Linear, 1),
            "aniso2x" | "aniso4x" => (FilterMode::Linear, 2),
            other => return Err(format!("unknown source sampler filter {other}")),
        };
        let mipmap_filter = match self.mip_map.as_str() {
            "disabled" | "nearest" => MipmapFilterMode::Nearest,
            "linear" => MipmapFilterMode::Linear,
            other => return Err(format!("unknown source sampler mipMap {other}")),
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
            mipmap_filter: if anisotropy_clamp > 1 {
                MipmapFilterMode::Linear
            } else {
                mipmap_filter
            },
            lod_min_clamp: 0.,
            lod_max_clamp: f32::MAX,
            compare: None,
            anisotropy_clamp,
            ..default()
        })
    }
}
