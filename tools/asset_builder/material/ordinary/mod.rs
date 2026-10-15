use super::*;

mod families;
mod pixels;
use families::RuntimeFeature;
pub(super) use families::{OrdinaryRecipe, ordinary_recipe};
pub(super) use pixels::bake;
#[cfg(test)]
pub(super) use pixels::camo_pixel;
pub(super) use pixels::metallic_workflow;
#[cfg(test)]
pub use pixels::pbr_pixel;

pub(super) fn validate_ordinary(
    key: &str,
    material: &Value,
    recipe: &OrdinaryRecipe,
) -> Result<()> {
    let OrdinaryRecipe {
        expected,
        normal,
        specular,
        coverage,
        detail,
        occlusion,
        camo,
        programs,
        reveal,
        flag,
        runtime,
        ..
    } = *recipe;
    let scrolling = matches!(runtime, RuntimeFeature::Scrolling);
    let family = material["technique_set"]
        .as_str()
        .ok_or("Missing native material technique")?;

    let pass = contract::pass(material, 4)?;
    let shader = pass["pixel_shader"]
        .as_str()
        .ok_or("Native pixel shader name is missing")?;
    if shader != expected {
        return Err(
            format!("Native technique {family} has an unreviewed pixel shader: {shader}").into(),
        );
    }
    if let Some([vertex, sun_vertex, sun_pixel]) = programs {
        let sun_pass = contract::pass(material, 6)?;
        if pass["vertex_shader"] != vertex
            || sun_pass["vertex_shader"] != sun_vertex
            || sun_pass["pixel_shader"] != sun_pixel
        {
            return Err(
                format!("Reviewed material {key} has unreviewed vertex or sun programs").into(),
            );
        }
        let lit_state = material["state_entries"][4]
            .as_u64()
            .ok_or("Reviewed lit state index is missing")? as usize;
        let sun_state = material["state_entries"][6]
            .as_u64()
            .ok_or("Reviewed sun state index is missing")? as usize;
        if material["state_bits"].get(lit_state).is_none()
            || material["state_bits"].get(sun_state).is_none()
            || material["state_bits"][lit_state] != material["state_bits"][sun_state]
        {
            return Err(
                format!("Reviewed material {key} has differing lit/sun render states").into(),
            );
        }
        let state = &material["state_bits"][lit_state];
        let rgb = state[0].as_u64().ok_or("Missing reviewed RGB state")?;
        let depth = state[1].as_u64().ok_or("Missing reviewed depth state")?;
        if (coverage == 1 && rgb & 0x7ff != 0x162)
            || (coverage != 1 && rgb >> 8 & 7 != 0)
            || (coverage == 1 && ![12, 28].contains(&depth))
            || (coverage != 1 && ![13, 29].contains(&depth))
        {
            return Err(
                format!("Reviewed material {key} has unreviewed coverage or depth state").into(),
            );
        }
        let mut routes = vec![
            json!({"source":0,"destination":0}),
            json!({"source":1,"destination":2}),
            json!({"source":2,"destination":5}),
            json!({"source":3,"destination":1}),
        ];
        if normal || flag {
            routes.push(json!({"source":4,"destination":7}));
        }
        if recipe.lightmap_routes {
            routes.push(json!({"source":5,"destination":6}));
        }
        let mut inputs = BTreeMap::from([(2695565377_u32, 0_u64)]);
        if normal {
            inputs.insert(1507003663, 257);
        }
        if specular == 1 {
            inputs.insert(
                887934131,
                if detail {
                    515
                } else if normal {
                    514
                } else {
                    257
                },
            );
        }
        if detail {
            inputs.insert(3948059469, if specular == 1 { 770 } else { 514 });
        }
        if occlusion != 0 {
            inputs.insert(1610742065, if camo { 1027 } else { 771 });
        }
        if camo {
            inputs.insert(3183121296, if occlusion != 0 { 772 } else { 771 });
        }
        for (pass_index, pass) in [pass, sun_pass].into_iter().enumerate() {
            if pass["vertex_routes"] != json!(routes) {
                return Err(format!("Reviewed material {key} has unreviewed vertex routes").into());
            }
            let arguments = pass["arguments"]
                .as_array()
                .ok_or("Reviewed pass arguments are missing")?;
            let active: BTreeMap<_, _> = arguments
                .iter()
                .filter(|argument| argument["type"] == 2)
                .map(|argument| {
                    Ok((
                        argument["value"]
                            .as_u64()
                            .ok_or("Missing shader image hash")? as u32,
                        argument["location"]
                            .as_u64()
                            .ok_or("Missing shader image location")?,
                    ))
                })
                .collect::<Result<_>>()?;
            let texture_arguments: Vec<_> = arguments
                .iter()
                .filter(|argument| argument["type"] == 2)
                .collect();
            if active != inputs
                || texture_arguments.len() != inputs.len()
                || texture_arguments
                    .iter()
                    .any(|argument| argument["size"] != 1 || argument["buffer"] != 0)
            {
                return Err(format!(
                    "Reviewed material {key} has unreviewed active texture bindings"
                )
                .into());
            }
            if camo
                && arguments
                    .iter()
                    .filter(|argument| contract::argument(argument, [5, 16777397, 1, 528, 16]))
                    .count()
                    != 1
            {
                return Err(format!(
                    "Reviewed camo material {key} lacks its native weaponParam2 binding"
                )
                .into());
            }
            if (scrolling || flag)
                && arguments
                    .iter()
                    .filter(|argument| contract::argument(argument, [3, 16777241, 2, 0, 16]))
                    .count()
                    != 1
            {
                return Err(format!(
                    "Reviewed animated material {key} lacks its native gameTime binding"
                )
                .into());
            }
            for argument in arguments
                .iter()
                .filter(|argument| matches!(argument["type"].as_u64(), Some(0 | 1 | 6 | 7 | 8)))
            {
                let hash = argument["value"]
                    .as_u64()
                    .ok_or("Missing material constant hash")?;
                let expected = match hash {
                    2418533825 => (672, 12),
                    2932833280 if reveal => (928, 16),
                    148072969 if detail => (848, 16),
                    2651392336 if flag => (656, 16),
                    411508820 if camo && occlusion != 0 && pass_index == 1 => (1008, 16),
                    _ => {
                        return Err(format!(
                            "Reviewed material {key} has an unreviewed active material constant"
                        )
                        .into());
                    }
                };
                let kind = if flag && hash == 2651392336 { 0 } else { 6 };
                if argument["type"] != kind
                    || argument["buffer"] != 1
                    || argument["location"] != expected.0
                    || argument["size"] != expected.1
                {
                    return Err(format!(
                        "Reviewed material {key} has an unreviewed active material constant"
                    )
                    .into());
                }
                let constant = material["constants"]
                    .as_array()
                    .ok_or("Missing native constant table")?
                    .iter()
                    .find(|constant| constant["hash"] == hash)
                    .ok_or("Missing bound material constant")?;
                if (0..expected.1 as usize / 4).any(|component| {
                    constant["literal"][component]
                        .as_f64()
                        .is_none_or(|value| !value.is_finite())
                }) {
                    return Err(
                        format!("Reviewed material {key} has invalid material constants").into(),
                    );
                }
            }
            if detail
                && arguments
                    .iter()
                    .filter(|argument| argument["type"] == 6 && argument["value"] == 148072969_u32)
                    .count()
                    != 1
            {
                return Err(format!("Reviewed material {key} lacks its bound detailScale").into());
            }
            if flag
                && arguments
                    .iter()
                    .filter(|argument| argument["type"] == 0 && argument["value"] == 2651392336_u32)
                    .count()
                    != 1
            {
                return Err(
                    format!("Reviewed flag material {key} lacks its bound flagParams").into(),
                );
            }
            if reveal {
                if arguments
                    .iter()
                    .filter(|argument| argument["type"] == 6 && argument["value"] == 2932833280_u32)
                    .count()
                    != 1
                {
                    return Err(format!(
                        "Reviewed reveal material {key} lacks its bound alphaRevealParms"
                    )
                    .into());
                }
                let constant = material["constants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|constant| constant["hash"] == 2932833280_u32)
                    .unwrap();
                let parameters: [f32; 4] = serde_json::from_value(constant["literal"].clone())?;
                if parameters[0] <= 0. || parameters[1] < 0. {
                    return Err(format!(
                        "Reviewed reveal material {key} has invalid reveal parameters"
                    )
                    .into());
                }
            }
            if camo
                && occlusion != 0
                && pass_index == 1
                && arguments
                    .iter()
                    .filter(|argument| argument["type"] == 6 && argument["value"] == 411508820_u32)
                    .count()
                    != 1
            {
                return Err(format!(
                    "Reviewed material {key} lacks its sun specular scale binding"
                )
                .into());
            }
        }
        for hash in inputs.keys() {
            let binding = contract::texture_binding(material, *hash)?;
            if !matches!(binding["sampler"].as_u64(), Some(20 | 244)) {
                return Err(format!(
                    "Reviewed material {key} has an unreviewed active sampler for {hash}"
                )
                .into());
            }
        }
    }
    Ok(())
}

pub(super) fn convert(
    key: &str,
    material: &Value,
    recipe: OrdinaryRecipe,
    images: &BTreeMap<String, texture::Image>,
    shared: &mut BTreeMap<(String, String), String>,
    pack: &mut Pack,
) -> Result<Value> {
    validate_ordinary(key, material, &recipe)?;
    let OrdinaryRecipe {
        normal,
        specular,
        coverage,
        detail,
        occlusion,
        camo,
        reveal,
        flag,
        ..
    } = recipe;

    let pass = contract::pass(material, 4)?;
    let mut channels = Vec::new();
    let camo_parameters = if camo && !material["code_constants"]["181"].is_null() {
        let value: [f32; 4] = serde_json::from_value(material["code_constants"]["181"].clone())?;
        if !value.iter().all(|component| component.is_finite()) {
            return Err(format!("Invalid camo parameters for {key}").into());
        }
        Some(value)
    } else {
        None
    };
    let mut source_camo = Value::Null;
    let mut source_inputs = BTreeMap::new();
    if camo || occlusion != 0 || detail || reveal {
        let mut inputs = Vec::new();
        if camo {
            inputs.extend([
                (2695565377_u32, "source_color"),
                (3183121296, "source_camo"),
            ]);
        }
        if reveal {
            inputs.push((2695565377, "source_color"));
        }
        if occlusion != 0 {
            inputs.push((1610742065, "source_occlusion"));
        }
        if detail {
            inputs.push((3948059469, "source_detail_normal"));
        }
        for (hash, role) in inputs {
            let binding = contract::texture_binding(material, hash)?;
            let name = binding["image"]
                .as_str()
                .ok_or("Missing source camo image")?
                .to_ascii_lowercase();
            let image = images
                .get(&name)
                .ok_or_else(|| format!("Missing source camo image {name}"))?;
            let channel_key = (name, role.into());
            let uri = if let Some(uri) = shared.get(&channel_key) {
                uri.clone()
            } else {
                let uri = store_image(
                    image,
                    format!("prepared/material-channels/{}/{role}.ktx2", shared.len()),
                    false,
                    role,
                    pack,
                )?;
                shared.insert(channel_key, uri.clone());
                uri
            };
            source_inputs.insert(role,json!({"uri":uri,"sampler":binding["sampler"],"transfer":if matches!(role,"source_color" | "source_camo") {"encoded_gamma2"} else {"linear"}}));
        }
        if camo {
            source_camo = json!({"inputs":{"source_color":source_inputs["source_color"],"source_camo":source_inputs["source_camo"]},
            "code_constant":181,"parameters":camo_parameters,"color_bias":if occlusion == 1 {"occlusion_blue"} else {"one"},
            "operation":"alpha_masked_gamma_color_and_specular","uv":"center_scale_offset"});
        }
    }
    let bindings = material["textures"].as_array().unwrap();
    let detail_input = if detail {
        let binding = bindings
            .iter()
            .find(|value| value["hash"] == 3948059469_u32)
            .ok_or("Missing bound detail normal")?;
        let constant = material["constants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["hash"] == 148072969_u32)
            .ok_or("Missing bound detailScale")?;
        let parameters: [f32; 4] = serde_json::from_value(constant["literal"].clone())?;
        let name = binding["image"].as_str().unwrap().to_ascii_lowercase();
        Some((
            images
                .get(&name)
                .ok_or_else(|| format!("Missing detail normal image {name}"))?,
            binding["sampler"].as_u64().unwrap() as u8,
            parameters,
        ))
    } else {
        None
    };
    let mut bindings = vec![(
        2695565377_u32,
        if specular == 1 {
            Vec::new()
        } else {
            vec!["color"]
        },
    )];
    if normal {
        bindings.push((1507003663, vec!["normal"]));
    }
    if specular == 1 {
        bindings.push((887934131, Vec::new()));
    }
    if specular == 2 {
        bindings.push((2695565377, vec!["response"]));
    }
    let mut native_specular = Value::Null;
    for (hash, roles) in bindings {
        let binding = contract::texture_binding(material, hash)?;
        if !pass["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|argument| argument["type"] == 2 && argument["value"] == hash)
        {
            return Err(format!(
                "Reviewed material {key} has no selected shader binding for {hash}"
            )
            .into());
        }
        let image_name = binding["image"]
            .as_str()
            .ok_or("Native image binding name is missing")?;
        let source = images
            .get(&image_name.to_ascii_lowercase())
            .ok_or_else(|| format!("Material {key} image {image_name} is missing"))?;
        for role in roles {
            let mut identity = image_name.to_ascii_lowercase();
            if role == "normal"
                && let Some((_, sampler, parameters)) = detail_input
            {
                let binding = material["textures"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|value| value["hash"] == 3948059469_u32)
                    .unwrap();
                identity.push_str(&format!(
                    ":{}:{sampler}:{:?}",
                    binding["image"].as_str().unwrap().to_ascii_lowercase(),
                    parameters.map(f32::to_bits)
                ));
            }
            let channel_key = (identity, role.to_owned());
            let srgb = role == "color";
            let uri = if let Some(uri) = shared.get(&channel_key) {
                uri.clone()
            } else {
                let image = bake(
                    source,
                    role,
                    if role == "normal" { detail_input } else { None },
                )?;
                let uri = store_image(
                    &image,
                    format!("prepared/material-channels/{}/{role}.ktx2", shared.len()),
                    srgb,
                    role,
                    pack,
                )?;
                shared.insert(channel_key, uri.clone());
                uri
            };
            channels.push(json!({"uri":uri,"role":role,"layer":0,"color_space":if srgb { "srgb" } else { "linear" },"sampler":binding["sampler"]}));
        }
        if hash == 887934131 || specular == 2 && hash == 2695565377 {
            let channel_key = (image_name.to_ascii_lowercase(), "native-specular".into());
            let uri = if let Some(uri) = shared.get(&channel_key) {
                uri.clone()
            } else {
                let uri = store_image(
                    source,
                    format!(
                        "prepared/material-channels/{}/native-specular.ktx2",
                        shared.len()
                    ),
                    false,
                    "specular_gloss",
                    pack,
                )?;
                shared.insert(channel_key, uri.clone());
                uri
            };
            native_specular = json!({"uri":uri,"gloss_channel":"alpha","specular_rgb":if specular == 2 { "constant_0.04" } else { "gamma2" }});
        }
    }
    if specular == 1 {
        let bindings = material["textures"].as_array().unwrap();
        let color = bindings
            .iter()
            .find(|value| value["hash"] == 2695565377_u32)
            .unwrap();
        let specular = bindings
            .iter()
            .find(|value| value["hash"] == 887934131_u32)
            .unwrap();
        let names = [
            color["image"].as_str().unwrap().to_ascii_lowercase(),
            specular["image"].as_str().unwrap().to_ascii_lowercase(),
        ];
        let samplers = [
            color["sampler"].as_u64().unwrap() as u8,
            specular["sampler"].as_u64().unwrap() as u8,
        ];
        let mut pair = format!("{}:{}:{}:{}", names[0], names[1], samplers[0], samplers[1]);
        let occlusion_input = if camo
            && occlusion != 0
            && camo_parameters.is_some_and(|parameters| parameters[3] != 0.)
        {
            let binding = bindings
                .iter()
                .find(|value| value["hash"] == 1610742065_u32)
                .ok_or("Missing bound camo occlusion")?;
            let name = binding["image"]
                .as_str()
                .ok_or("Missing bound camo occlusion image")?
                .to_ascii_lowercase();
            let sampler = binding["sampler"]
                .as_u64()
                .ok_or("Missing bound camo occlusion sampler")? as u8;
            pair.push_str(&format!(":occlusion:{name}:{sampler}"));
            Some((
                images
                    .get(&name)
                    .ok_or_else(|| format!("Missing bound camo occlusion image {name}"))?,
                sampler,
            ))
        } else {
            None
        };
        let camo_input =
            if let Some(parameters) = camo_parameters.filter(|parameters| parameters[3] != 0.) {
                let binding = bindings
                    .iter()
                    .find(|value| value["hash"] == 3183121296_u32)
                    .ok_or("Missing bound camo image")?;
                let name = binding["image"]
                    .as_str()
                    .ok_or("Missing bound camo image name")?
                    .to_ascii_lowercase();
                let sampler = binding["sampler"]
                    .as_u64()
                    .ok_or("Missing bound camo sampler")? as u8;
                pair.push_str(&format!(
                    ":{name}:{sampler}:{:?}",
                    parameters.map(f32::to_bits)
                ));
                Some((
                    images
                        .get(&name)
                        .ok_or_else(|| format!("Missing bound camo image {name}"))?,
                    sampler,
                    parameters,
                ))
            } else {
                None
            };
        if !shared.contains_key(&(pair.clone(), "color".into())) {
            let (base, response) = metallic_workflow(
                &images[&names[0]],
                &images[&names[1]],
                samplers,
                camo_input,
                occlusion_input,
            )?;
            for (role, image) in [("color", base), ("response", response)] {
                let path = format!("prepared/material-channels/{}/{role}.ktx2", shared.len());
                let path = store_image(&image, path, role == "color", role, pack)?;
                shared.insert((pair.clone(), role.into()), path);
            }
        }
        for role in ["color", "response"] {
            channels.push(
                json!({"uri":shared[&(pair.clone(),role.into())],"role":role,"layer":0,
            "color_space":if role == "color" {"srgb"} else {"linear"},"sampler":samplers[0]}),
            );
        }
    }
    let state_index = material["state_entries"][4]
        .as_u64()
        .ok_or("Missing native lit state index")? as usize;
    let state = material["state_bits"][state_index][0]
        .as_u64()
        .ok_or("Native lit state index exceeds state table")? as u32;
    let (source, destination, operation) = (state & 15, state >> 4 & 15, state >> 8 & 7);
    let alpha = if coverage == 2 && operation == 0 {
        "mask"
    } else if operation == 0 {
        "opaque"
    } else if (source, destination, operation) == (2, 6, 1) {
        "premultiplied"
    } else {
        return Err(format!("Unreviewed ordinary lit blend state for {key}: {state:#010x}").into());
    };
    let cull = match state >> 14 & 3 {
        1 => "none",
        2 => "back",
        3 => "front",
        _ => return Err(format!("Unknown native cull state for {key}").into()),
    };
    let mut converted = json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
    "extras":{"surface":{"version":3,"identity":key,"channels":channels,"native":material,
        "standard":{"roughness":1.,"metallic":if specular == 0 { 0. } else { 1. },"reflectance":if specular == 0 { 0. } else { 0.5 },"specular_tint":[1.,1.,1.],"base_color":[1.,1.,1.,1.]},
        "presentation":{"alpha":alpha,"cutoff":128./255.,"cull":cull,"fog":true,"visible":state & (1 << 27) != 0},
        "source_vertex_color_transfer":"gamma2","source_coverage":match coverage { 1 => "gamma2_premultiplied", 2 => "mask", _ => "opaque" },
        "native_specular":native_specular,"source_camo":source_camo,
        "approximations":["Khronos specular-glossiness to metallic-roughness fit for shared upstream PBR and Solari inputs", "Native Blinn-Phong gloss fitted to GGX perceptual roughness; original specular and gloss retained", "Slope reconstruction baked before filtering", "Native lighting-term multipliers retained for runtime integration", "Vertex gamma2 transfer evaluated before interpolation"]}}});
    if matches!(recipe.runtime, RuntimeFeature::Glass) {
        converted["extras"]["surface"]["source_fragment_alpha"] =
            json!("sqrt_texture_vertex_coverage");
        converted["extras"]["surface"]["pending_runtime"] = json!(
            "Native glass fragment uses sqrt coverage and coverage-independent specular before gamma2 premultiplied composition; the shared linear PBR consumer does not yet reproduce that blend"
        );
    }
    let surface = &mut converted["extras"]["surface"];
    if recipe.vertex_lighting {
        surface["source_vertex_lighting"] = json!({"source":5,"destination":6,
        "encoded_rgb_scale":32.,"transfer":"gamma2_before_vertex_interpolation",
        "diffuse":{"operation":"divide_by_grid_spherical_harmonics","buffer":3,"rows":[5,6,7],"minimum":0.1},
        "specular":{"operation":"divide_by_reflection_spherical_harmonics","buffer":3,"rows":[8,9,10],"minimum":0.1},
        "alpha":"sun_shadow_fallback","stage":"separate_indirect_lighting_terms"});
        surface["pending_runtime"] = json!(
            "Native vertex-lit material requires its secondary vertex RGB divided by separate grid/reflection spherical-harmonic responses before interpolation, separate indirect lighting multipliers and secondary-alpha sun-shadow fallback; geometry delivery and runtime lighting consumers remain unconnected"
        );
        if specular == 2 {
            surface["source_ambient_specular"] =
                json!({"gloss_scale":0.0275,"bias":0.015,"fresnel_cap_scale":0.475});
        }
        if coverage == 2 {
            surface["source_framebuffer_alpha"] = json!("texture_vertex_coverage_after_cutoff");
        }
        if coverage == 1 {
            surface["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
            surface["pending_runtime"] = json!(
                "Native translucent vertex-lit material requires separate vertex grid/probe multipliers and secondary-alpha shadow fallback, plus sqrt coverage and coverage-scaled fog in gamma2 premultiplied composition; geometry and runtime consumers remain unconnected"
            );
        }
        if matches!(recipe.runtime, RuntimeFeature::VertexDiffuse) {
            surface["source_vertex_alpha"] = json!("indirect_diffuse");
            surface["pending_runtime"] = json!(
                "Native vertex-lit cheap sandbag additionally attenuates indirect diffuse with primary vertex alpha while retaining opaque coverage and unattenuated sun lighting; separate vertex grid/probe and shadow-fallback consumers remain unconnected"
            );
        }
        if matches!(recipe.runtime, RuntimeFeature::VertexDiffuseSpecular) {
            surface["source_vertex_alpha"] = json!("indirect_diffuse_and_specular");
            surface["pending_runtime"] = json!(
                "Native vertex-lit sandbag multiplies separate grid and reflection lighting by primary vertex alpha while retaining opaque coverage and unattenuated sun lighting; secondary vertex lighting, shadow fallback and gloss-dependent ambient specular consumers remain unconnected"
            );
        }
    }
    if reveal {
        let parameters = &material["constants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["hash"] == 2932833280_u32)
            .unwrap()["literal"];
        surface["source_coverage"] = json!("gamma2_reveal_premultiplied");
        surface["source_reveal"] = json!({"input":source_inputs["source_color"],"parameters":parameters,
        "vertex_channel":"alpha","vertex_scale":f32::from_bits(0x3f7f7cee),"vertex_bias":f32::from_bits(0x3a83126f),
        "operation":"saturate_texture_alpha_between_vertex_power_thresholds"});
        surface["source_fragment_alpha"] = json!("sqrt_reveal_coverage");
        surface["pending_runtime"] = json!(
            "Native alpha reveal uses interpolated vertex alpha and authored power thresholds before coverage, sqrt fragment alpha and gamma2 premultiplied fog composition; geometry-dependent reveal baking and matching runtime composition remain unconnected"
        );
    }
    if let Some((_, _, parameters)) = detail_input {
        surface["source_detail_normal"] = json!({"input":source_inputs["source_detail_normal"],"parameters":parameters,
        "operation":"scaled_slope_add_before_normalization","uv":"multiply_xy"});
        surface["approximations"].as_array_mut().unwrap().push(json!("Detail normal slopes baked at base-normal resolution using a major-axis mip estimate before output filtering"));
    }
    if flag {
        let parameters = &material["constants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["hash"] == 2651392336_u32)
            .unwrap()["literal"];
        let phase_uv = [
            [0x4067e015_u32, 0x3e81b6e4].map(f32::from_bits),
            [0x41040141_u32, 0xbfba354d].map(f32::from_bits),
            [0x419c9f28_u32, 0x40a7dddc].map(f32::from_bits),
        ];
        let phase_time = [0x40b4f4ab_u32, 0x4116cbe4, 0x40f1463a].map(f32::from_bits);
        let normal_cosine_u = [0x41d96214_u32, 0x41c601e2, 0x420520e2].map(f32::from_bits);
        let normal_cosine_v = [0x3ff336eb_u32, 0xc08ba7f9, 0x410eafc8].map(f32::from_bits);
        surface["source_flag_motion"] = json!({"parameters":parameters,"parameter_hash":2651392336_u32,
        "time":{"code_constant":25,"component":3,"operation":"add_parameter_y_then_multiply_parameter_x"},
        "uv":"one_minus_uv0","vertex_alpha":"motion_amplitude_only","fragment_vertex_alpha":1.,
        "phase_uv":phase_uv,"phase_time":phase_time,
        "displacement_amplitude":[7.5,3.,f32::from_bits(0x3fd9999a)],"displacement_axis":"decoded_model_normal",
        "normal_cosine_u":normal_cosine_u,"normal_cosine_v":normal_cosine_v,
        "normal_derivative_scale":f32::from_bits(0x3ccccccd),
        "normal_operation":"add_vertex_alpha_scaled_cosine_derivatives_in_source_tangent_basis_then_normalize"});
        surface["source_coverage"] = json!(if coverage == 2 {
            "texture_mask_without_vertex_alpha"
        } else {
            "gamma2_texture_premultiplied_without_vertex_alpha"
        });
        if coverage == 2 {
            surface["source_framebuffer_alpha"] = json!("texture_coverage_after_cutoff");
            surface["pending_runtime"] = json!(
                "Native masked flag material requires gameTime.w-driven three-wave displacement and derivative-adjusted normals using vertex alpha, with fragment vertex alpha forced to one and texture alpha retained after cutoff; live geometry and framebuffer-alpha consumers remain unconnected"
            );
        } else {
            surface["source_fragment_alpha"] = json!("sqrt_texture_coverage_without_vertex_alpha");
            surface["pending_runtime"] = json!(
                "Native translucent flag material requires gameTime.w-driven three-wave displacement and derivative-adjusted normals using vertex alpha, with fragment vertex alpha forced to one, separate grid/reflection responses and sqrt texture coverage in gamma2 premultiplied composition; live geometry and matching lighting/composition remain unconnected"
            );
        }
    }
    if occlusion != 0 {
        surface["source_occlusion"] = json!({"input":source_inputs["source_occlusion"],
        "indirect_diffuse":if occlusion == 1 {"red"} else {"blue"},"indirect_specular":"green",
        "direct_diffuse":if occlusion == 1 {"none"} else {"green"},"direct_specular":"red"});
        if camo {
            surface["source_occlusion"]["sun_specular_scale"] =
                json!({"hash":411508820,"component":2});
            surface["pending_runtime"] = json!(
                "Native weapon camo needs live scoped weaponParam2/image overrides and separate red indirect-diffuse/direct-specular, green reflection and blue camo-bias occlusion consumers; packed source inputs and explicit-state conversion do not establish those runtime bindings"
            );
        } else {
            surface["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
            surface["pending_runtime"] = json!(
                "Native catwalk gamma2 premultiplied lighting writes sqrt coverage and applies distinct source occlusion channels to lighting terms; the shared PBR consumer does not yet reproduce that composition"
            );
        }
    }
    if recipe.color_matrix {
        surface["source_color_matrix"] =
            json!({"buffer":0,"rows":[114,115,116],"stage":"linear_lighting_before_fog"});
    }
    match recipe.runtime {
        RuntimeFeature::MaskedColorMatrix => {
            surface["source_framebuffer_alpha"] = json!("texture_vertex_coverage_after_cutoff");
            surface["pending_runtime"] = json!(
                "Native masked Ray Gun material applies the model color matrix to linear lighting before fog and retains texture times vertex framebuffer alpha after cutoff; both runtime consumers remain unconnected"
            );
        }
        RuntimeFeature::TranslucentColorMatrix => {
            surface["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
            surface["source_color_matrix"]["stage"] =
                json!("coverage_scaled_linear_lighting_before_fog");
            surface["pending_runtime"] = json!(
                "Native translucent model material applies coverage to diffuse plus specular before its affine color matrix, then coverage-scaled fog and sqrt fragment alpha in gamma2 premultiplied composition; matching runtime composition remains unconnected"
            );
            if key == "mc/mtl_c_zom_tomb_richtofen_vial_out"
                && material["technique_set"] == "mc_lit_sm_b0c0n0s0_68f25z0u"
            {
                surface["source_hero_lighting"] = surface["source_color_matrix"].take();
                surface
                    .as_object_mut()
                    .unwrap()
                    .remove("source_color_matrix");
                surface["source_hero_lighting"]["producer"] =
                    json!("native_position_dependent_hero_lighting");
                surface["presentation"]["alpha"] = json!("blend");
                surface["pending_runtime"] = json!(
                    "Native vial heroLightingR/G/B applies coverage-scaled diffuse/specular plus coverage-independent affine bias before sqrt/fog/HDR composition; actual source maps and texture-times-vertex coverage use straight linear PBR blending while hero affine lighting and native framebuffer composition are replaced by shared PBR lighting"
                );
            }
        }
        RuntimeFeature::ColorMatrix => {
            surface["pending_runtime"] = json!(
                "Native model color matrix must be applied to linear diffuse plus specular before fog; its runtime parameter binding is not connected"
            );
        }
        RuntimeFeature::AttachmentCamo => {
            if camo_parameters.is_none() {
                surface["pending_runtime"] = json!(
                    "Native attachment camo requires a bound per-object submission; unresolved world, dual-hand and alternate-weapon inputs prevent pack closure"
                );
            } else {
                surface["camo_binding"] = json!("prepared_native_input_state");
            }
        }
        RuntimeFeature::Scrolling => {
            surface["source_uv_animation"] =
                json!({"code_constant":25,"component":2,"translation":[0.,-1.]});
            surface["uv_animation"] =
                json!({"translation":[0.,-1.],"time_mode":"native_fractional_second"});
            surface["pending_runtime"] = json!(
                "Native tank-tread UV binding subtracts gameTime.z from V on every channel, but the source render-game milliseconds and per-object shader-start inputs still require a gameplay producer; explicit setup inputs do not establish native runtime clock delivery"
            );
        }
        RuntimeFeature::IndirectDiffuseSpecular => {
            surface["source_vertex_alpha"] = json!("indirect_diffuse_and_specular");
            surface["source_ambient_specular"] =
                json!({"gloss_scale":0.0275,"bias":0.015,"fresnel_cap_scale":0.475});
            surface["pending_runtime"] = json!(
                "Native sandbag vertex alpha attenuates indirect diffuse and specular without attenuating sun lighting; the source gloss-dependent ambient response also remains distinct from the shared direct-light PBR fit"
            );
        }
        RuntimeFeature::IndirectDiffuse => {
            surface["source_vertex_alpha"] = json!("indirect_diffuse");
            surface["pending_runtime"] = json!(
                "Native cheap sandbag vertex alpha attenuates indirect diffuse without changing coverage or sun lighting; the runtime lighting consumer is not connected"
            );
        }
        RuntimeFeature::MaskedDetail => {
            surface["source_framebuffer_alpha"] = json!("texture_vertex_coverage_after_cutoff");
            surface["pending_runtime"] = json!(
                "Native masked detail material writes texture times vertex coverage after the 128/255 cutoff; the shared masked consumer forces framebuffer alpha to one"
            );
        }
        RuntimeFeature::None
        | RuntimeFeature::Glass
        | RuntimeFeature::VertexDiffuse
        | RuntimeFeature::VertexDiffuseSpecular => (),
    }

    Ok(converted)
}
