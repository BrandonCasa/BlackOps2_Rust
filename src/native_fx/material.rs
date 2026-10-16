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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the authorized local FX export"]
    fn exported_source_particle_materials_bind() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let catalog: crate::native_fx::Catalog =
            crate::origins::read(&root.join("fx_catalog.json"));
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
        let mut names = Vec::new();
        let mut counts = [0; 12];
        for name in catalog.materials.keys() {
            let definition: Definition = crate::origins::read(
                &root
                    .join("materials")
                    .join(format!("{}.json", name.replace('*', "_"))),
            );
            let (family, program) = match definition.technique_set.as_str() {
                "effect_842ee834" => (0, NativeEffectProgram::SimpleFog),
                "effect_wj9j7075" => (1, NativeEffectProgram::ZfeatherDissolve),
                "effect_32z8zqf9" => (2, NativeEffectProgram::SimpleFogDissolve),
                "effect_775wj8ww" => (3, NativeEffectProgram::EyeFeather),
                "effect_8f63534j" => (4, NativeEffectProgram::SimpleAdditive),
                "particlecloud_9337w7f6" => (5, NativeEffectProgram::CloudSimple),
                "particlecloud_w7046j83" => (6, NativeEffectProgram::CloudAdditive),
                "effect_eq3qqw5f" => (7, NativeEffectProgram::SimpleAdditive),
                "effect_1w3e5wf8" => (8, NativeEffectProgram::SimpleFeather),
                "effect_e3ej38qz" => (9, NativeEffectProgram::LocalAdditiveFeather),
                "effect_ze9wj511" => (10, NativeEffectProgram::AdditiveFeather),
                "effect_60e2624u" => (11, NativeEffectProgram::EyeAdditive),
                _ => continue,
            };
            let material = definition
                .bind(server)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert!(material.program == program, "{name}");
            let constant = |key| {
                definition
                    .constants
                    .iter()
                    .find(|v| v.name.as_deref() == Some(key))
                    .map(|v| Vec4::from_array(v.literal))
                    .unwrap_or(Vec4::ZERO)
            };
            assert_eq!(material.feather_parms, constant("featherParms"), "{name}");
            assert_eq!(
                material.alpha_dissolve_parms,
                constant("alphaDissolveParms"),
                "{name}"
            );
            assert_eq!(
                material.additive_parms[3],
                constant("eyeOffsetParms"),
                "{name}"
            );
            assert!(
                root.join(material.color_map.path().unwrap().path())
                    .is_file(),
                "{name}"
            );
            if name == "gfx_fxt_lensflare_stripe_white" {
                assert_eq!(material.state.depth_compare, CompareFunction::Always);
            } else {
                assert_eq!(
                    material.state.depth_compare,
                    CompareFunction::GreaterEqual,
                    "{name}"
                );
            }
            if name == "gfx_fxt_env_dust_mote_pcloud_add" {
                assert_eq!(material.state.blend.color.src_factor, BlendFactor::SrcAlpha);
                assert_eq!(material.state.blend.color.dst_factor, BlendFactor::One);
                assert_eq!(
                    material.state.blend.alpha.src_factor,
                    BlendFactor::OneMinusDstAlpha
                );
            }
            if matches!(family, 7..=10) {
                assert_eq!(
                    material.state.blend.color.src_factor,
                    BlendFactor::OneMinusDst
                );
                assert_eq!(material.state.blend.color.dst_factor, BlendFactor::One);
            }
            names.push(name.as_str());
            counts[family] += 1;
        }
        assert_eq!(counts, [12, 16, 8, 14, 11, 13, 12, 1, 1, 1, 1, 4]);
        assert!(names.contains(&"gfx_fxt_debris_gib_wood"));
        assert!(names.contains(&"gfx_fxt_bio_bloodburst"));
        assert!(names.contains(&"gfx_fxt_gas_flash_eo2_z2"));
        assert!(names.contains(&"gfx_fxt_bio_bloodburst_b_ds64"));
        assert!(names.contains(&"gfx_fxt_debris_fine_clump_ds128"));
        assert!(names.contains(&"gfx_fxt_spark_single"));
        assert!(names.contains(&"gfx_fxt_lensflare_stripe_white"));
        assert!(names.contains(&"gfx_fxt_env_dust_mote_pcloud_add"));
        eprintln!(
            "Bound all {} original fog, dissolve, eye-offset, spark, and cloud materials",
            names.len()
        );
    }

    #[test]
    #[ignore = "requires the authorized local FX export"]
    fn exported_robot_dust_footstep_and_warning_materials_bind() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let catalog: crate::native_fx::Catalog =
            crate::origins::read(&root.join("fx_catalog.json"));
        let warning = "maps/zombie_tomb/fx_tomb_foot_warning_light_red";
        assert_eq!(catalog.effects[warning].elements.len(), 5);
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
        let names: std::collections::BTreeSet<_> = catalog
            .effects
            .iter()
            .filter(|(name, _)| {
                name.as_str() == "maps/zombie_tomb/fx_tomb_robot_dust"
                    || name.as_str() == warning
                    || name.starts_with("maps/zombie_tomb/fx_tomb_robot_foot_step_")
            })
            .flat_map(|(_, effect)| &effect.elements)
            .flat_map(|e| &e.visuals)
            .flatten()
            .filter_map(|v| v.material.as_ref())
            .map(|name| name.trim_start_matches(','))
            .collect();
        assert!(!names.is_empty());
        for name in names {
            let definition: Definition = crate::origins::read(
                &root
                    .join("materials")
                    .join(format!("{}.json", name.replace('*', "_"))),
            );
            definition
                .bind(server)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
        }
    }
}

impl Definition {
    fn bind(&self, server: &AssetServer) -> Result<NativeEffectMaterial, String> {
        if self.game != "t6" || self.kind != "material" || self.version != 1 {
            return Err("requires the original T6 material.v1 record".into());
        }
        let program = match self.technique_set.as_str() {
            "effect_w77q49e8" => NativeEffectProgram::Zfeather,
            "effect_wj9j7075" => NativeEffectProgram::ZfeatherDissolve,
            "effect_842ee834" => NativeEffectProgram::SimpleFog,
            "effect_32z8zqf9" => NativeEffectProgram::SimpleFogDissolve,
            "effect_jqj65q53" => NativeEffectProgram::Additive,
            "effect_5e098749" | "effect_ze9wj511" => NativeEffectProgram::AdditiveFeather,
            "effect_737e1je4" | "effect_e3ej38qz" => NativeEffectProgram::LocalAdditiveFeather,
            "effect_z096f2e2" => NativeEffectProgram::OutdoorFeather,
            "effect_1519efqw" => NativeEffectProgram::OutdoorFogFeather,
            // VS5656248b adds only unused fog outputs to the simple feather
            // vertex program; PS a0389359 is identical.
            "effect_50567j38" | "effect_z0z25860" | "effect_8wjj7fj0" | "effect_1w3e5wf8" => {
                NativeEffectProgram::SimpleFeather
            }
            // VS7c3a8ec2 differs from VS8b0718c3 only in unused TEXCOORD0.w;
            // both use PSa0389359, which consumes TEXCOORD0.xyz.
            "effect_ezz8jfwj" | "effect_775wj8ww" => NativeEffectProgram::EyeFeather,
            // Both families use the identical effectadd_ab4b60e3 technique:
            // VS255edc40 / PS8788956c; only their debug performance pass differs.
            "effect_910219f6" | "effect_f4225412" | "effect_60e2624u" => {
                NativeEffectProgram::EyeAdditive
            }
            "effect_8e1qf0qq" | "effect_eq3qqw5f" => NativeEffectProgram::SimpleAdditive,
            // VS58ec9b48 preserves vertex RGBA/UV and ordinary world projection;
            // PS8788956c is the existing premultiplied additive operation.
            "effect_8f63534j" => NativeEffectProgram::SimpleAdditive,
            "particlecloud_ew972295" => NativeEffectProgram::CloudRain,
            "particlecloud_j32z635q" => NativeEffectProgram::CloudSnow,
            "particlecloud_9337w7f6" => NativeEffectProgram::CloudSimple,
            "particlecloud_w7046j83" => NativeEffectProgram::CloudAdditive,
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
        // Shader identity does not determine blend state. Preserve the selected
        // material's factors, including straight-alpha additive dust and screen RGB.
        let factor = |field: &str| -> Result<BlendFactor, String> {
            match state.get(field).and_then(|v| v.as_str()) {
                Some("zero") => Ok(BlendFactor::Zero),
                Some("one") => Ok(BlendFactor::One),
                Some("srcalpha") => Ok(BlendFactor::SrcAlpha),
                Some("invsrcalpha") => Ok(BlendFactor::OneMinusSrcAlpha),
                Some("invdestalpha") => Ok(BlendFactor::OneMinusDstAlpha),
                Some("invdestcolor") => Ok(BlendFactor::OneMinusDst),
                other => Err(format!("port source {field}={other:?}")),
            }
        };
        let blend = BlendState {
            color: BlendComponent {
                src_factor: factor("srcBlendRgb")?,
                dst_factor: factor("dstBlendRgb")?,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: factor("srcBlendAlpha")?,
                dst_factor: factor("dstBlendAlpha")?,
                operation: BlendOperation::Add,
            },
        };
        let depth_compare = match state.get("depthTest").and_then(|v| v.as_str()) {
            // The source rasterizer and native attachment both use reverse Z.
            Some("less_equal") => CompareFunction::GreaterEqual,
            Some("disabled") => CompareFunction::Always,
            other => return Err(format!("port source depthTest={other:?}")),
        };
        for (field, expected) in [
            ("blendOpRgb", "add"),
            ("blendOpAlpha", "add"),
            (
                "alphaTest",
                if program.additive() {
                    "disabled"
                } else {
                    "gt0"
                },
            ),
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
                | NativeEffectProgram::ZfeatherDissolve
                | NativeEffectProgram::AdditiveFeather
                | NativeEffectProgram::LocalAdditiveFeather
                | NativeEffectProgram::OutdoorFeather
                | NativeEffectProgram::OutdoorFogFeather
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
        let alpha_dissolve_parms = if matches!(
            program,
            NativeEffectProgram::ZfeatherDissolve | NativeEffectProgram::SimpleFogDissolve
        ) {
            let mut values = self
                .constants
                .iter()
                .filter(|v| v.name.as_deref() == Some("alphaDissolveParms"));
            let value = values
                .next()
                .ok_or("required alphaDissolveParms is absent")?;
            if values.next().is_some() || !value.literal.iter().all(|v| v.is_finite()) {
                return Err("required alphaDissolveParms must be one finite source float4".into());
            }
            Vec4::from_array(value.literal)
        } else {
            Vec4::ZERO
        };
        Ok(NativeEffectMaterial {
            program,
            color_map: image(server, &texture.image),
            color_sampler: texture.sampler_state.descriptor()?,
            feather_parms,
            additive_parms,
            alpha_dissolve_parms,
            state: NativeEffectState {
                blend,
                depth_compare,
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
