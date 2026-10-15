use super::*;

#[derive(Clone, Copy)]
pub(super) struct UnlitRecipe {
    pub(super) pixel: &'static str,
    pub(super) alpha: &'static str,
}

pub(super) fn unlit_recipe(family: &str) -> Option<UnlitRecipe> {
    match family {
        "mc_unlit_replace_3j970129" | "wpc_unlit_replace_4688792e" => Some(UnlitRecipe {
            pixel: "pimp_shader_unlit_49c3292c.hlsl",
            alpha: "opaque",
        }),
        "mc_unlit_add_ww82zw88" | "wpc_unlit_add_8ez3wzw3" => Some(UnlitRecipe {
            pixel: "pimp_shader_unlit_4b1d2e0b.hlsl",
            alpha: "add",
        }),
        "mc_unlit_blend_588ewzff" => Some(UnlitRecipe {
            pixel: "pimp_shader_unlit_4ea12045.hlsl",
            alpha: "blend",
        }),
        "mc_unlit_multiply_97923qz1" => Some(UnlitRecipe {
            pixel: "pimp_shader_unlit_4f14dd58.hlsl",
            alpha: "multiply",
        }),
        "mc_unlitdecalblend_add_fzw1z66f" | "wpc_unlitdecalblend_add_j26wq580" => {
            Some(UnlitRecipe {
                pixel: "pimp_shader_unlitdecalblend_62b62074.hlsl",
                alpha: "decal_add",
            })
        }
        "mc_unlitdecalblend_multiply_34w7q3jw" | "wpc_unlitdecalblend_multiply_35079164" => {
            Some(UnlitRecipe {
                pixel: "pimp_shader_unlitdecalblend_5c3f6d9c.hlsl",
                alpha: "decal_multiply",
            })
        }
        _ => None,
    }
}

pub(super) fn validate_unlit(key: &str, material: &Value, recipe: UnlitRecipe) -> Result<f32> {
    let UnlitRecipe { pixel, alpha } = recipe;
    let pass = contract::pass(material, 2)?;
    let decal = matches!(alpha, "decal_add" | "decal_multiply");
    let scaled = !decal && alpha != "multiply";
    let vertex = if decal {
        pixel
    } else {
        "pimp_shader_unlit_4876d431.hlsl"
    };
    if pass["pixel_shader"] != pixel
        || pass["vertex_shader"] != vertex
        || pass["vertex_routes"]
            != json!([{"source":0,"destination":0},{"source":1,"destination":2},{"source":2,"destination":5}])
    {
        return Err(
            format!("Unlit material {key} has unreviewed programs or vertex routes").into(),
        );
    }
    let arguments = pass["arguments"]
        .as_array()
        .ok_or("Missing unlit arguments")?;
    let active: Vec<_> = arguments
        .iter()
        .filter(|argument| matches!(argument["type"].as_u64(), Some(0 | 1 | 2 | 6 | 7 | 8)))
        .collect();
    if active.len() != (if scaled { 2 } else { 1 })
        || !active
            .iter()
            .any(|argument| contract::argument(argument, [2, 2695565377, 0, 0, 1]))
        || scaled
            && !active
                .iter()
                .any(|argument| contract::argument(argument, [6, 3799286735, 2, 16, 4]))
    {
        return Err(format!("Unlit material {key} has unreviewed active bindings").into());
    }
    let texture = contract::texture_binding(material, 2695565377_u32)?;
    let sampler = texture["sampler"].as_u64();
    if !(if decal {
        matches!(sampler, Some(10 | 74))
    } else {
        matches!(sampler, Some(20 | 244))
    }) {
        return Err(format!("Unlit material {key} has an unreviewed sampler").into());
    }
    let scale = if scaled {
        let constant = material["constants"]
            .as_array()
            .ok_or("Missing unlit constants")?
            .iter()
            .find(|constant| constant["hash"] == 3799286735_u32)
            .ok_or("Missing bound scaleRGB")?;
        constant["literal"][0]
            .as_f64()
            .ok_or("Missing scaleRGB value")? as f32
    } else {
        1.
    };
    if !scale.is_finite() || scale < 0. {
        return Err(format!("Unlit material {key} has invalid scaleRGB").into());
    }
    let index = material["state_entries"][2]
        .as_u64()
        .ok_or("Missing native unlit state index")? as usize;
    let state = material["state_bits"][index][0]
        .as_u64()
        .ok_or("Missing native unlit state")?;
    let depth = material["state_bits"][index][1]
        .as_u64()
        .ok_or("Missing native unlit depth state")?;
    let (rgb, coverage) = match alpha {
        "opaque" => (0x12, 0x12),
        "add" | "decal_add" => (0x122, 0x121),
        "blend" => (0x165, 0x128),
        "multiply" | "decal_multiply" => (0x131, 0x128),
        _ => unreachable!(),
    };
    if state & 0x7ff != rgb
        || state >> 16 & 0x7ff != coverage
        || depth & !0x30 != if alpha == "opaque" { 13 } else { 12 }
    {
        return Err(format!("Unlit material {key} has unreviewed blend or depth state").into());
    }
    Ok(scale)
}

pub(super) fn unlit_color(
    source: &texture::Image,
    scale: f32,
    alpha: &str,
) -> Result<texture::Image> {
    if source.depth != 1
        || source.faces != 1
        || source.width == 0
        || source.height == 0
        || source.levels.is_empty()
    {
        return Err("Unlit color input is not a valid 2D image".into());
    }
    let levels = (0..source.levels.len())
        .map(|level| -> Result<Vec<[f32; 4]>> {
            texture::pixels(source, level)?
                .into_iter()
                .map(|pixel| {
                    let coverage = if alpha == "blend" { pixel[3] } else { 1. };
                    let decal_weight =
                        pixel[3] * (pixel[0] * 0.25 + pixel[1] * 0.5 + pixel[2] * 0.25);
                    let value: [f32; 4] = std::array::from_fn(|c| {
                        if c == 3 {
                            if matches!(alpha, "add" | "source_vm" | "source_vm_add") {
                                1.
                            } else if alpha == "source_vm_blend" {
                                pixel[3].sqrt()
                            } else if alpha == "decal_add" {
                                decal_weight
                            } else {
                                pixel[3]
                            }
                        } else {
                            match alpha {
                                "decal_add" => (pixel[c] * decal_weight).powi(2),
                                "decal_multiply" => (1. + pixel[3] * (pixel[c] - 1.)).powi(2),
                                "multiply" => pixel[c].powi(4),
                                "source_layer" | "source_vm_add" => {
                                    (pixel[c] * pixel[3]).powi(2) * scale
                                }
                                _ => pixel[c] * pixel[c] * scale * coverage,
                            }
                        }
                    });
                    if value
                        .iter()
                        .any(|value| !value.is_finite() || !(0. ..=65504.).contains(value))
                        || value[3] > 1.
                    {
                        return Err(
                            "Unlit converted color exceeds the finite half-float/coverage domain"
                                .into(),
                        );
                    }
                    Ok(value)
                })
                .collect()
        })
        .collect::<Result<Vec<_>>>()?;
    let hdr = levels
        .iter()
        .flatten()
        .any(|pixel| pixel[..3].iter().any(|&value| value > 1.));
    let mut image = source.clone();
    image.format = if hdr { 19 } else { 1 };
    image.levels = levels
        .into_iter()
        .map(|pixels| {
            pixels
                .into_iter()
                .flat_map(|pixel| {
                    if hdr {
                        pixel
                            .into_iter()
                            .flat_map(|value| half::f16::from_f32(value).to_le_bytes())
                            .collect::<Vec<_>>()
                    } else {
                        pixel
                            .into_iter()
                            .enumerate()
                            .map(|(c, value)| {
                                let value = if c == 3 {
                                    value
                                } else if value <= 0.0031308 {
                                    value * 12.92
                                } else {
                                    1.055 * value.powf(1. / 2.4) - 0.055
                                };
                                (value * 255.).round() as u8
                            })
                            .collect()
                    }
                })
                .collect()
        })
        .collect();
    Ok(image)
}

pub(super) fn convert_unlit(
    key: &str,
    native: &Value,
    recipe: UnlitRecipe,
    images: &BTreeMap<String, texture::Image>,
    shared: &mut BTreeMap<(String, String), String>,
    pack: &mut Pack,
) -> Result<Value> {
    let UnlitRecipe { pixel: _, alpha } = recipe;
    let scale = validate_unlit(key, native, recipe)?;
    let binding = contract::texture_binding(native, 2695565377_u32)?;
    let name = binding["image"]
        .as_str()
        .ok_or("Missing unlit image name")?
        .to_ascii_lowercase();
    let source = images
        .get(&name)
        .ok_or_else(|| format!("Missing unlit color image {name} for {key}"))?;
    let image = unlit_color(source, scale, alpha)?;
    let linear = image.format == 19;
    let channel_key = (name, format!("unlit-{}-{alpha}", scale.to_bits()));
    let uri = if let Some(uri) = shared.get(&channel_key) {
        uri.clone()
    } else {
        let uri = store_image(
            &image,
            format!("prepared/material-channels/{}/color.ktx2", shared.len()),
            !linear,
            "color",
            pack,
        )?;
        shared.insert(channel_key, uri.clone());
        uri
    };
    let index = native["state_entries"][2].as_u64().unwrap() as usize;
    let state = native["state_bits"][index][0].as_u64().unwrap();
    let cull = match state >> 14 & 3 {
        1 => "none",
        2 => "back",
        3 => "front",
        _ => return Err("Unknown unlit cull state".into()),
    };
    let composed = matches!(alpha, "multiply" | "decal_add" | "decal_multiply");
    let blend = match alpha {
        "decal_add" => "add",
        "decal_multiply" => "multiply",
        _ => alpha,
    };
    let mut converted = json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
        "extras":{"surface":{"version":3,"identity":key,"native":native,
            "channels":[{"uri":uri,"role":"color","layer":0,"color_space":if linear {"linear"} else {"srgb"},"sampler":binding["sampler"]}],
            "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
            "presentation":{"alpha":blend,"cull":cull,"fog":alpha != "decal_add","fog_mode":if alpha == "add" || composed {"attenuation"} else {"color"},"visible":state & (1<<27) != 0},
            "source_vertex_color_transfer":if composed {"native_fragment_composition"} else if alpha == "blend" {"gamma2_alpha"} else {"gamma2"},
            "source_coverage":if composed {alpha} else if alpha == "add" {"unlit_add"} else if alpha == "blend" {"unlit_blend"} else {"opaque"},
            "pending_runtime":if alpha == "add" {json!("Native ONE/ONE addition requires fog attenuation without adding fog color")} else {Value::Null},
            "approximations":["Vertex gamma2 transfer evaluated before interpolation", "Source gamma2 framebuffer composition represented in the shared linear HDR pipeline", "Source polygon-offset modes retained for runtime integration"]}}});
    if composed {
        let source_key = (
            binding["image"].as_str().unwrap().to_ascii_lowercase(),
            "source_color".into(),
        );
        let source_uri = if let Some(uri) = shared.get(&source_key) {
            uri.clone()
        } else {
            let uri = store_image(
                source,
                format!(
                    "prepared/material-channels/{}/source-color.ktx2",
                    shared.len()
                ),
                false,
                "source_color",
                pack,
            )?;
            shared.insert(source_key, uri.clone());
            uri
        };
        let surface = &mut converted["extras"]["surface"];
        surface["source_input"] =
            json!({"uri":source_uri,"sampler":binding["sampler"],"color_space":"native_encoded"});
        surface["source_composition"] = match alpha {
            "decal_add" => json!({"color":"texture_rgb_times_interpolated_vertex_rgb",
                "coverage":"texture_alpha_times_interpolated_vertex_alpha_times_color_luminance",
                "luminance_weights":[0.25,0.5,0.25],"rgb":"color_times_coverage","alpha":"coverage","fog":false}),
            "decal_multiply" => json!({"color":"texture_rgb_times_interpolated_vertex_rgb",
                "coverage":"texture_alpha_times_interpolated_vertex_alpha_times_fog_transmittance",
                "rgb":"one_plus_coverage_times_color_minus_one","alpha":"coverage","fog_neutral_rgb":[1.,1.,1.]}),
            _ => json!({"color":"square_texture_rgb_times_interpolated_vertex_rgb",
                "coverage":"texture_alpha_times_interpolated_vertex_alpha",
                "rgb":"one_plus_fog_transmittance_times_color_minus_one","alpha":"coverage","fog_neutral_rgb":[1.,1.,1.]}),
        };
        surface["approximations"] = json!([
            "Static texture terms evaluated for unit vertex tint and no fog; raw inputs retained for source fragment composition",
            "Native encoded framebuffer factors squared for shared linear HDR storage; exact destination-alpha and vertex/fog composition remain pending",
            "Source polygon-offset modes retained for runtime integration"
        ]);
        surface["pending_runtime"] = json!(match alpha {
            "decal_add" =>
                "Native additive decal computes coverage from interpolated vertex-tinted RGB luminance and texture/vertex alpha, then adds encoded color times that coverage without fog; source fragment tint, destination-alpha and gamma2 framebuffer composition remain unconnected",
            "decal_multiply" =>
                "Native multiplicative decal interpolates its encoded texture/vertex color toward white using texture alpha times vertex alpha and fog transmittance, then multiplies the destination; source fragment tint, neutral-white fog and destination-alpha composition remain unconnected",
            _ =>
                "Native unlit multiplication squares interpolated texture/vertex RGB before fog interpolates the encoded multiplier toward white; source fragment tint, neutral-white fog and destination-alpha composition remain unconnected",
        });
    }
    Ok(converted)
}
