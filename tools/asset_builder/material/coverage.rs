use super::*;

fn gaps(native: &BTreeMap<String, Value>, covered: impl Fn(&str, &Value) -> bool) -> Vec<String> {
    let mut families = BTreeMap::<&str, Vec<&str>>::new();
    for (name, material) in native {
        if !covered(name, material) {
            families
                .entry(material["technique_set"].as_str().unwrap())
                .or_default()
                .push(name);
        }
    }
    families
        .into_iter()
        .map(|(family, names)| {
            format!(
                "{family}: {} used materials, first {}",
                names.len(),
                names[0]
            )
        })
        .collect()
}

pub fn missing(
    native: &BTreeMap<String, Value>,
    converted: &BTreeMap<String, Value>,
) -> Vec<String> {
    let mut missing = gaps(native, |name, _| converted.contains_key(name));
    missing.extend(native.keys().filter_map(|name| {
        converted.get(name).and_then(|material| {
            material["extras"]["surface"]["pending_runtime"]
                .as_str()
                .map(|reason| format!("{name}: {reason}"))
        })
    }));
    missing
}

pub fn accept_initial_visual_approximations(
    native: &BTreeMap<String, Value>,
    converted: &mut BTreeMap<String, Value>,
) -> Result<Vec<String>> {
    let mut accepted = Vec::new();
    for (name, value) in converted {
        let Some(surface) = value
            .get_mut("extras")
            .and_then(|value| value.get_mut("surface"))
        else {
            continue;
        };
        let source_name = name.rsplit_once('#').and_then(|(source, variant)| {
            let index = variant
                .strip_prefix("atlas")
                .or_else(|| variant.strip_prefix("periodic"))?;
            (!index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())).then_some(source)
        });
        let Some(material) = native
            .get(name)
            .or_else(|| source_name.and_then(|source| native.get(source)))
        else {
            continue;
        };
        let Some(reason) = surface["pending_runtime"].as_str().map(str::to_owned) else {
            continue;
        };
        let family = material["technique_set"]
            .as_str()
            .ok_or("Initial material has no native family")?;
        let flag = ordinary_recipe(family).is_some_and(|recipe| recipe.flag);
        if !surface["source_flag_motion"].is_null() && !flag {
            continue;
        }
        if [
            "source_camo",
            "source_reveal",
            "source_layer_uv",
            "source_script_layers",
            "source_script_coverage",
            "source_color_matrix",
            "source_model_color_transform",
            "source_uv_animation",
            "source_effective_inputs",
            "source_fritz",
            "source_emission_threshold",
            "source_emission_modulation",
        ]
        .iter()
        .any(|field| !surface[*field].is_null())
        {
            continue;
        }
        let mut roles = vec!["color"];
        let mut shadow_only = false;
        let feature = if let Some(recipe) = ordinary_recipe(family) {
            let vertex_lighting = family.starts_with("mlv_")
                && surface["source_vertex_lighting"]["source"] == 5
                && surface["source_vertex_lighting"]["stage"] == "separate_indirect_lighting_terms";
            if !vertex_lighting
                && !flag
                && !matches!(
                    family,
                    "mc_lit_sm_b0c0n0s0_21fz2e61"
                        | "mc_lit_sm_b0c0n0s0o0_jf00fe4q"
                        | "mc_lit_sm_r0c0n0x0_53144qq1"
                        | "mc_lit_sm_r0c0_5137qq99"
                        | "mc_lit_sm_t0c0d0n0s0_68eww765"
                )
            {
                continue;
            }
            validate_ordinary(name, material, &recipe)?;
            if flag {
                let motion = &surface["source_flag_motion"];
                if !motion.is_object()
                    || motion["parameters"] != json!(contract::literal(material, 2651392336)?)
                    || motion["time"]["code_constant"] != 25
                    || motion["time"]["component"] != 3
                    || motion["fragment_vertex_alpha"] != 1.
                    || surface["source_coverage"]
                        != if recipe.coverage == 2 {
                            "texture_mask_without_vertex_alpha"
                        } else {
                            "gamma2_texture_premultiplied_without_vertex_alpha"
                        }
                {
                    continue;
                }
            }
            if recipe.normal {
                roles.push("normal");
            }
            if recipe.specular != 0 {
                roles.push("response");
            }
            if flag {
                "native_flag_motion_with_shared_pbr_lighting"
            } else if vertex_lighting {
                "native_vertex_lighting_to_shared_pbr"
            } else {
                "native_lighting_and_framebuffer_to_shared_pbr"
            }
        } else if let Some(recipe) = character::lookup(family) {
            use character::CharacterRecipe::*;
            let inputs: &[&str] = match recipe {
                Cloth | Skin | SkinMask | Hair | Transparent => {
                    roles.extend(["normal", "response"]);
                    &["source_color", "source_normal", "source_specular_gloss"]
                }
                Optic => {
                    roles.extend(["normal", "response"]);
                    &[
                        "source_color",
                        "source_normal",
                        "source_specular_color",
                        "source_gloss",
                    ]
                }
                Cornea => &["source_highlight_mask", "source_highlight_normal"],
                Canopy | CanopyShadow | CanopyBlend => &["source_color"],
                _ => continue,
            };
            recipe.validate(name, material)?;
            if inputs.iter().any(|role| {
                let input = &surface["source_inputs"][*role];
                input["uri"].as_str().is_none_or(str::is_empty)
                    || input["sampler"]
                        .as_u64()
                        .is_none_or(|sampler| sampler > 255)
                    || !matches!(
                        input["transfer"].as_str(),
                        Some("encoded_gamma2" | "linear")
                    )
            }) {
                continue;
            }
            "native_character_lighting_and_coverage_to_shared_pbr"
        } else if world::lookup(family) == Some(world::WorldRecipe::Water) {
            world::WorldRecipe::Water.validate(name, material)?;
            let water = &surface["source_water"];
            if water["consumer"] != "native_world_normals_and_palette_shared_pbr"
                || water["game_time_code"] != 25
                || water["game_time_component"] != 3
                || water["layers"]
                    .as_array()
                    .is_none_or(|layers| layers.len() != 4)
                || water["layers"].as_array().unwrap().iter().any(|layer| {
                    layer["uri"].as_str().is_none_or(str::is_empty)
                        || layer["sampler"] != 20
                        || layer["coordinates"] != "source_position_xy"
                        || layer["normal_space"] != "source_world_xy"
                })
            {
                continue;
            }
            roles.clear();
            roles.push("normal");
            "native_water_world_normals_and_palette_to_shared_pbr"
        } else if world::lookup(family) == Some(world::WorldRecipe::Shadow) {
            world::WorldRecipe::Shadow.validate(name, material)?;
            let shadow = &surface["source_shadow_only"];
            if shadow["visible_lit_pass"] != false
                || shadow["cast_shadows"] != true
                || shadow["depth_prepass"]["color_write"] != false
                || shadow["shadowmap"]["color_write"] != false
                || surface["presentation"]["visible"] != false
            {
                continue;
            }
            roles.clear();
            shadow_only = true;
            "native_shadow_only_geometry_with_solari_occlusion"
        } else if let Some(recipe) = single_unlit_recipe(family) {
            let mode = recipe.mode;
            if matches!(mode, "layer" | "layer_reflection") {
                continue;
            }
            validate_single_unlit(name, material, recipe)?;
            if mode == "eyes"
                && (surface["source_eye_luminance"]["prepared_unit_luminance"] != true
                    || surface["source_eye_luminance"]["code_constant"] != 171
                    || surface["source_eye_luminance"]["component"] != 1)
            {
                continue;
            }
            "native_unlit_fog_hdr_and_framebuffer"
        } else if let Some(recipe) = unlit_recipe(family) {
            let alpha = recipe.alpha;
            if !matches!(alpha, "add" | "multiply" | "decal_add" | "decal_multiply") {
                continue;
            }
            validate_unlit(name, material, recipe)?;
            if alpha == "add" {
                "native_additive_fog_attenuation"
            } else {
                if surface["source_input"]["uri"]
                    .as_str()
                    .is_none_or(str::is_empty)
                    || !surface["source_composition"].is_object()
                {
                    continue;
                }
                "native_unlit_fragment_composition_to_shared_hdr"
            }
        } else if matches!(
            family,
            "mc_sw4_3d_cod7_tile_blend_spec_8w40z341"
                | "mlv_sw4_3d_cod7_tile_blend_spec_w8wezqe6"
                | "mc_sw4_3d_cod7_tile_blend_q2q2zej6"
                | "mlv_sw4_3d_cod7_tile_blend_26zq1w22"
                | "mc_sw4_3d_phong_emissive_q0751958"
                | "mc_sw4_3d_phong_emissive_alcatraz_90892wq5"
                | "mc_sw4_3d_phong_emissive_alcatraz_8q551153"
                | "mlv_sw4_3d_phong_emissive_alcatraz_w1q1w09e"
                | "lit_sm_r0c0n0x0_t1c1n1"
        ) {
            crate::bake::validate_native(material)?;
            roles.push("normal");
            if family.contains("phong_emissive") {
                roles.extend(["response", "emissive"]);
            }
            if family.contains("tile_blend_spec") {
                roles.push("response");
            }
            "native_baked_lighting_and_framebuffer"
        } else {
            continue;
        };
        let Some(channels) = surface["channels"]
            .as_array()
            .filter(|channels| shadow_only || !channels.is_empty())
        else {
            continue;
        };
        let mut present = BTreeSet::new();
        if channels.iter().any(|channel| {
            channel["uri"].as_str().is_none_or(str::is_empty)
                || channel["layer"] != 0
                || !matches!(channel["color_space"].as_str(), Some("linear" | "srgb"))
                || channel["sampler"]
                    .as_u64()
                    .is_none_or(|sampler| sampler > 255)
                || channel["role"]
                    .as_str()
                    .is_none_or(|role| !present.insert(role.to_owned()))
        }) || roles.iter().any(|role| !present.contains(*role))
            || !surface["standard"].is_object()
        {
            continue;
        }
        if surface["version"] != 3
            || surface["identity"] != name.as_str()
            || surface["native"] != *material
        {
            return Err(format!(
                "Initial visual approximation has inconsistent native metadata: {name}"
            )
            .into());
        }
        let approximations = surface["approximations"].as_array_mut().ok_or_else(|| {
            format!("Initial visual approximation has no documented bake/PBR limits: {name}")
        })?;
        approximations.push(json!(reason));
        if shadow_only {
            approximations.push(json!("Source shadow-only geometry participates in opaque Solari occlusion while a dedicated camera-excluded render layer prevents color output; secondary ray hits absorb light instead of reflecting native tool artwork"));
        }
        surface["approximation_scope"] = json!("first_playable_solo");
        surface["accepted_visual_feature"] = json!(feature);
        surface.as_object_mut().unwrap().remove("pending_runtime");
        accepted.push(format!("{name}: {reason}"));
    }
    Ok(accepted)
}

pub fn unimplemented(native: &BTreeMap<String, Value>) -> Vec<String> {
    gaps(native, |_, material| {
        let family = material["technique_set"].as_str().unwrap();
        selected_technique(family).is_some()
    })
}
