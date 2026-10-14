use super::unlit::unlit_color;
use super::*;

pub(super) fn authored_unlit_uv(
    source: &texture::Image,
    scale: [f32; 2],
    offset: [f32; 2],
    origin: [f32; 2],
    rotation: f32,
    sampler: u8,
) -> Result<texture::Image> {
    if scale == [1.; 2] && offset == [0.; 2] && rotation == 0. {
        return Ok(source.clone());
    }
    if source.depth != 1
        || source.faces != 1
        || source.levels.is_empty()
        || !scale
            .into_iter()
            .chain(offset)
            .chain(origin)
            .chain([rotation])
            .all(f32::is_finite)
    {
        return Err("Invalid authored single-layer UV input".into());
    }
    let mut image = source.clone();
    image.format = 19;
    image.levels.clear();
    let (sine, cosine) = (rotation * f32::from_bits(0x40c90fda)).sin_cos();
    for level in 0..source.levels.len() {
        let (width, height) = (
            (source.width >> level).max(1),
            (source.height >> level).max(1),
        );
        let pixels = texture::pixels(source, level)?;
        let mut bytes = Vec::with_capacity(width * height * 8);
        for y in 0..height {
            for x in 0..width {
                let uv = [
                    (x as f32 + 0.5) / width as f32,
                    (y as f32 + 0.5) / height as f32,
                ];
                let centered: [f32; 2] = std::array::from_fn(|axis| {
                    (uv[axis] - origin[axis] + offset[axis]) * scale[axis]
                });
                let uv = [
                    cosine * centered[0] + sine * centered[1] + origin[0],
                    -sine * centered[0] + cosine * centered[1] + origin[1],
                ];
                let pixel = texture::sample(&pixels, width, height, uv, sampler);
                for value in pixel {
                    bytes.extend(half::f16::from_f32(value).to_le_bytes());
                }
            }
        }
        image.levels.push(bytes);
    }
    Ok(image)
}

pub(super) fn convert_single_unlit(
    key: &str,
    native: &Value,
    recipe: SingleUnlitRecipe,
    images: &BTreeMap<String, texture::Image>,
    shared: &mut BTreeMap<(String, String), String>,
    pack: &mut Pack,
) -> Result<Value> {
    let (index, hash, mode) = validate_single_unlit(key, native, recipe)?;
    let layered = index == 4;
    let binding = native["textures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|binding| binding["hash"] == hash)
        .unwrap();
    let name = binding["image"]
        .as_str()
        .ok_or("Missing single-layer unlit source image name")?
        .to_ascii_lowercase();
    let source = images
        .get(&name)
        .ok_or_else(|| format!("Missing single-layer unlit image {name}"))?;
    let constants = native["constants"].as_array().unwrap();
    let constant = |hash: u32| -> Result<[f32; 4]> {
        let row = constants
            .iter()
            .find(|constant| constant["hash"] == hash)
            .ok_or("Missing bound single-layer image parameter")?;
        Ok(serde_json::from_value(row["literal"].clone())?)
    };
    let eyes = mode == "eyes";
    let eye_state: Option<[f32; 4]> = if eyes && !native["code_constants"]["171"].is_null() {
        let state: [f32; 4] = serde_json::from_value(native["code_constants"]["171"].clone())?;
        if state.iter().any(|value| !value.is_finite()) {
            return Err("Non-finite native eye luminance input".into());
        }
        Some(state)
    } else {
        None
    };
    let amount = if eyes {
        eye_state.unwrap_or([0.; 4])[1]
    } else {
        constant(if layered { 14836402 } else { 2875883048 })?[0]
    };
    let uv = if layered {
        Some([
            constant(3320325482)?,
            constant(3286424575)?,
            constant(3272022534)?,
            constant(1886964550)?,
        ])
    } else {
        None
    };
    let image = if let Some([scale, offset, origin, rotation]) = uv {
        authored_unlit_uv(
            source,
            [scale[0], scale[1]],
            [offset[0], offset[1]],
            [origin[0], origin[1]],
            rotation[0],
            20,
        )?
    } else {
        source.clone()
    };
    let mut image = image;
    if eyes {
        image.levels.truncate(1);
    }
    let operation = if layered {
        "source_layer"
    } else {
        match mode {
            "add" => "source_vm_add",
            "blend" => "source_vm_blend",
            "alpha" => "source_vm_alpha",
            _ => "source_vm",
        }
    };
    let prepared_amount = if eyes { 1. } else { amount };
    let image = unlit_color(&image, prepared_amount * prepared_amount, operation)?;
    let linear = image.format == 19;
    let channel_key = (
        name.clone(),
        format!(
            "single-unlit:{operation}:{}:{uv:?}:lod0-unit={eyes}",
            prepared_amount.to_bits()
        ),
    );
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
    let source_key = (name, "source_color".into());
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
    let selected = native["state_entries"][index].as_u64().unwrap() as usize;
    let state = native["state_bits"][selected][0].as_u64().unwrap();
    let cull = match state >> 14 & 3 {
        1 => "none",
        2 => "back",
        3 => "front",
        _ => return Err("Unknown single-layer unlit cull state".into()),
    };
    let alpha = if layered || mode == "alpha" {
        "blend"
    } else if eyes {
        "opaque"
    } else {
        mode
    };
    let mut surface = json!({"version":3,"identity":key,"native":native,
        "channels":[{"uri":uri,"role":"color","layer":0,"color_space":if linear {"linear"} else {"srgb"},"sampler":20}],
        "source_input":{"uri":source_uri,"sampler":20,"color_space":"native_encoded"},
        "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
        "presentation":{"alpha":alpha,"cull":cull,"fog":mode != "layer_reflection","fog_mode":"color","visible":state & (1<<27) == 0},
        "source_vertex_color_transfer":"unused","source_coverage":if layered {"script_scaled_sqrt_coverage"} else {"source_viewmodel_emissive"},
        "source_emissive":{"amount":amount,"rgb":if layered || mode == "add" {"square(texture_rgb*texture_alpha*amount)"} else {"square(texture_rgb*amount)"},"texture_alpha_in_rgb":layered || mode == "add",
            "output_transfer":if layered {"sqrt_after_scene_hdr_scale"} else {"linear_without_gamma2_sqrt"}},
        "source_blend":{"rgb":[state&15,state>>4&15,state>>8&7],"alpha":[state>>16&15,state>>20&15,state>>24&7]},
        "approximations":["Static authored UV terms resampled at original mip resolution; dynamic parallax/time terms retained for runtime integration",
            "Native source color equations retained separately from unresolved framebuffer transfer and destination-alpha composition"]});
    if layered {
        surface["source_layer_uv"] = json!({"scale":constant(3320325482)?,"offset":constant(3286424575)?,
            "origin":constant(3272022534)?,"rotation":constant(1886964550)?,"scroll":constant(2196952607)?,
            "depth":constant(3342767871)?[0],"depth_epsilon":f32::from_bits(0x3727c5ac),"game_time_code":25,"game_time_component":3,
            "operation":"scale_centered_uv_plus_scaled_offset_and_positive_facing_view_parallax_then_rotate_then_add_scaled_time_scroll_and_origin",
            "rotation_operation":"rotation_x_times_one_plus_rotation_y_times_game_time_w","rotation_tau":f32::from_bits(0x40c90fda)});
        surface["source_script_coverage"] = json!({"code_constant":169,"component":0,"layer_alpha":constant(3337236870)?[0],
            "operation":"sqrt(texture_alpha*layer_alpha*(1-saturate(scriptVector0.x)))"});
        if mode == "layer_reflection" {
            surface["source_reflection"] = json!({"amount":constant(211931156)?[0],"gloss":constant(3432544439)?[0],
                "lod":"four_times_one_minus_gloss","weight":"saturate_three_quarters_plus_quarter_gloss_times_reflection_amount",
                "probe_decode":"rgb_divided_by_alpha_plus_epsilon","response":"separate_vertex_spherical_harmonics"});
            surface["pending_runtime"] = json!(
                "Native no-fog rune layer requires view parallax including the depth epsilon, gameTime UV controls, live scriptVector0 alpha, separate view-dependent reflection/SH response and sqrt coverage in gamma2 composition; source consumers remain unconnected"
            );
        } else {
            surface["pending_runtime"] = json!(
                "Native wallbuy layer requires view parallax including the depth epsilon, gameTime UV controls and live scriptVector0 alpha, followed by source fog and sqrt coverage in gamma2 framebuffer composition; matching runtime consumers remain unconnected"
            );
        }
    } else {
        surface["source_fragment_alpha"] = json!(if mode == "blend" {
            "sqrt_texture_coverage"
        } else if mode == "alpha" {
            "texture_coverage"
        } else {
            "one"
        });
        surface["pending_runtime"] = json!(match mode {
            "screen" =>
                "Native tritium emission writes squared linear source RGB without the gamma2 sqrt, then blends with inverse destination color plus destination; source fog, screen blending and destination-alpha composition remain unconnected",
            "blend" =>
                "Native translucent viewmodel emission writes squared source RGB without gamma2 sqrt and sqrt texture alpha with source-alpha blending; matching source fog and framebuffer composition remain unconnected",
            "alpha" =>
                "Native viewmodel alpha emission writes squared source RGB and unchanged texture alpha with source-alpha blending; matching source fog and linear-output framebuffer composition remain unconnected",
            "eyes" =>
                "Native cheap eyes retain unit-luminance squared LOD-zero texture and live scriptVector2.y squared material factor; source fog, scene HDR control0 and gamma2 framebuffer composition remain unconnected",
            _ =>
                "Native additive viewmodel emission squares texture-alpha-premultiplied source RGB, writes alpha one, and adds source fog color before ONE/ONE blending; matching source fog and framebuffer composition remain unconnected",
        });
        if eyes {
            let factor = amount * amount;
            if !factor.is_finite() {
                return Err("Native eye luminance square is non-finite".into());
            }
            surface["standard"]["base_color"] = json!([factor, factor, factor, 1.]);
            surface["source_eye_luminance"] = json!({"code_constant":171,"component":1,"state":eye_state,"base_state":[0.,0.,0.,0.],
                "base_state_origin":"normal_view_zero_template_preserved_by_type2_constant_copy","lod":0.,"operation":"square_texture_rgb_times_luminance",
                "prepared_unit_luminance":true,"runtime_operation":"square_scriptVector2_y",
                "active_override_scope":"actor_scriptVector2_mapping_replaces_base_state"});
            surface["source_emissive"]["output_transfer"] =
                json!("sqrt_after_source_fog_and_scene_hdr_control0");
        }
    }
    Ok(
        json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},"extras":{"surface":surface}}),
    )
}

#[derive(Clone, Copy)]
pub(super) struct SingleUnlitRecipe {
    pub(super) index: usize,
    pub(super) vertex: &'static str,
    pub(super) pixel: &'static str,
    pub(super) image: u32,
    pub(super) mode: &'static str,
}

pub(super) fn single_unlit_recipe(family: &str) -> Option<SingleUnlitRecipe> {
    Some(match family {
        "wpc_sw4_3d_unlit_1layer_8691j77q" => SingleUnlitRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_1layer_20d9eb7d.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_1layer_c5daea2d.hlsl",
            image: 3714339118,
            mode: "layer",
        },
        "mc_sw4_3d_unlit_1layer_nofog_91f00f80" => SingleUnlitRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_1layer_nofog_39bf2c98.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_1layer_nofog_b5cdb07a.hlsl",
            image: 3714339118,
            mode: "layer_reflection",
        },
        "mc_sw4_3d_viewmodel_unlit_j299w86e" => SingleUnlitRecipe {
            index: 3,
            vertex: "pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl",
            pixel: "pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl",
            image: 3053961470,
            mode: "screen",
        },
        "mc_sw4_3d_viewmodel_unlit_8216051u" => SingleUnlitRecipe {
            index: 3,
            vertex: "pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl",
            pixel: "pimp_shader_sw4_3d_viewmodel_unlit_657ff8d2.hlsl",
            image: 3053961470,
            mode: "blend",
        },
        "mc_sw4_3d_viewmodel_unlit_60466j88" => SingleUnlitRecipe {
            index: 3,
            vertex: "pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl",
            pixel: "pimp_shader_sw4_3d_viewmodel_unlit_ff86912b.hlsl",
            image: 3053961470,
            mode: "add",
        },
        "mc_sw4_3d_viewmodel_unlit_alpha_85z6j5w5" => SingleUnlitRecipe {
            index: 3,
            vertex: "pimp_shader_sw4_3d_viewmodel_unlit_89d1b67c.hlsl",
            pixel: "pimp_shader_sw4_3d_viewmodel_unlit_alpha_161b096b.hlsl",
            image: 3053961470,
            mode: "alpha",
        },
        "mc_sw4_3d_model_unlit_cheap_zombie_eyes_jq3e7eqw" => SingleUnlitRecipe {
            index: 3,
            vertex: "pimp_shader_sw4_3d_model_unlit_cheap_zombie_eyes_6a8c6a7c.hlsl",
            pixel: "pimp_shader_sw4_3d_model_unlit_cheap_zombie_eyes_ee6b6abd.hlsl",
            image: 2695565377,
            mode: "eyes",
        },
        _ => return None,
    })
}

pub(super) fn validate_single_unlit(
    key: &str,
    material: &Value,
    recipe: SingleUnlitRecipe,
) -> Result<(usize, u32, &'static str)> {
    let SingleUnlitRecipe {
        index,
        vertex,
        pixel,
        image,
        mode,
    } = recipe;
    let layered = index == 4;
    let eyes = mode == "eyes";
    let mut routes = vec![
        json!({"source":0,"destination":0}),
        json!({"source":2,"destination":5}),
    ];
    if layered {
        routes.extend([
            json!({"source":3,"destination":1}),
            json!({"source":4,"destination":7}),
        ]);
    }
    let mut constants = BTreeMap::new();
    if layered {
        constants.extend([
            (1886964550_u32, (0, 112, 8)),
            (2196952607, (0, 96, 8)),
            (3272022534, (0, 120, 8)),
            (3286424575, (0, 88, 8)),
            (3320325482, (0, 80, 8)),
            (3342767871, (0, 128, 4)),
            (14836402, (6, 108, 4)),
            (3337236870, (6, 104, 4)),
        ]);
        if mode == "layer_reflection" {
            constants.extend([(211931156, (6, 136, 4)), (3432544439, (6, 132, 4))]);
        }
    } else if !eyes {
        constants.insert(2875883048, (6, 80, 4));
    }
    let indices = if layered { vec![4, 6] } else { vec![3] };
    let state_index = material["state_entries"][index]
        .as_u64()
        .ok_or("Missing single-layer unlit state index")? as usize;
    let state = &material["state_bits"][state_index];
    let rgb = state[0]
        .as_u64()
        .ok_or("Missing single-layer unlit RGB state")?;
    let depth = state[1]
        .as_u64()
        .ok_or("Missing single-layer unlit depth state")?;
    let (blend, alpha) = match mode {
        "screen" => (0x12a, 0x121),
        "add" => (0x122, 0x121),
        "eyes" => (0x12, 0x12),
        _ => (0x165, 0x128),
    };
    if rgb & 0x7ff != blend
        || rgb >> 16 & 0x7ff != alpha
        || depth & !0x30 != if eyes { 13 } else { 12 }
    {
        return Err(format!(
            "Single-layer unlit material {key} has unreviewed blend or depth state"
        )
        .into());
    }
    for index in indices {
        let pass = contract::pass(material, index)?;
        if pass["vertex_shader"] != vertex
            || pass["pixel_shader"] != pixel
            || pass["vertex_routes"] != json!(routes)
        {
            return Err(format!(
                "Single-layer unlit material {key} has unreviewed programs or vertex routes"
            )
            .into());
        }
        let selected = material["state_entries"][index]
            .as_u64()
            .ok_or("Missing selected single-layer unlit state index")?
            as usize;
        if material["state_bits"][selected] != *state {
            return Err(format!(
                "Single-layer unlit material {key} has differing selected render states"
            )
            .into());
        }
        let arguments = pass["arguments"]
            .as_array()
            .ok_or("Missing selected single-layer unlit arguments")?;
        if matches!(mode, "eyes" | "alpha") {
            let expected_state = if eyes {
                json!([403867666_u32, 13])
            } else {
                json!([422084965_u32, 12])
            };
            if arguments.len() != 4
                || !arguments.iter().any(|argument| {
                    argument["type"] == 3
                        && argument["value"] == 67109077_u32
                        && argument["buffer"] == 3
                        && argument["location"] == 0
                        && argument["size"] == 64
                })
                || !arguments.iter().any(|argument| {
                    argument["type"] == 3
                        && argument["value"] == 67109093_u32
                        && argument["buffer"] == 0
                        && argument["location"] == 576
                        && argument["size"] == 64
                })
                || *state != expected_state
            {
                return Err(format!(
                    "Selected eye/alpha material {key} has unreviewed matrices or render state"
                )
                .into());
            }
        }
        let images: Vec<_> = arguments
            .iter()
            .filter(|argument| argument["type"] == 2)
            .collect();
        if images.len() != 1
            || images[0]["value"] != image
            || images[0]["buffer"] != 0
            || images[0]["location"] != 256
            || images[0]["size"] != 1
        {
            return Err(
                format!("Single-layer unlit material {key} has unreviewed image bindings").into(),
            );
        }
        let active: Vec<_> = arguments
            .iter()
            .filter(|argument| matches!(argument["type"].as_u64(), Some(0 | 1 | 6 | 7 | 8)))
            .collect();
        if active.len() != constants.len() {
            return Err(format!(
                "Single-layer unlit material {key} has unreviewed material argument count"
            )
            .into());
        }
        for (&hash, &(kind, location, size)) in &constants {
            if active
                .iter()
                .filter(|argument| {
                    argument["type"] == kind
                        && argument["value"] == hash
                        && argument["buffer"] == 2
                        && argument["location"] == location
                        && argument["size"] == size
                })
                .count()
                != 1
            {
                return Err(format!(
                    "Single-layer unlit material {key} has unreviewed constant {hash}"
                )
                .into());
            }
            let constant = material["constants"]
                .as_array()
                .ok_or("Missing single-layer unlit constants")?
                .iter()
                .find(|constant| constant["hash"] == hash)
                .ok_or("Missing bound single-layer unlit constant")?;
            if (0..size as usize / 4).any(|component| {
                constant["literal"][component]
                    .as_f64()
                    .is_none_or(|value| !value.is_finite())
            }) {
                return Err(format!(
                    "Single-layer unlit material {key} has non-finite constant {hash}"
                )
                .into());
            }
        }
        if layered
            && (!arguments.iter().any(|argument| {
                argument["type"] == 3
                    && argument["value"] == 16777241_u32
                    && argument["buffer"] == 2
                    && argument["location"] == 64
                    && argument["size"] == 16
            }) || !arguments.iter().any(|argument| {
                argument["type"] == 5
                    && argument["value"] == 16777385_u32
                    && argument["buffer"] == 1
                    && argument["location"] == 368
                    && argument["size"] == 16
            }))
        {
            return Err(format!(
                "Single-layer unlit material {key} lacks reviewed gameTime or scriptVector0 binding"
            )
            .into());
        }
        if eyes
            && (arguments.len() != 4
                || arguments
                    .iter()
                    .filter(|argument| {
                        argument["type"] == 5
                            && argument["value"] == 16777387_u32
                            && argument["buffer"] == 1
                            && argument["location"] == 400
                            && argument["size"] == 16
                    })
                    .count()
                    != 1)
        {
            return Err(format!(
                "Cheap eye material {key} lacks its exact scriptVector2 luminance binding"
            )
            .into());
        }
    }
    let texture = material["textures"]
        .as_array()
        .ok_or("Missing single-layer unlit images")?
        .iter()
        .find(|binding| binding["hash"] == image)
        .ok_or("Missing single-layer unlit source image")?;
    if texture["sampler"] != 20 {
        return Err(format!("Single-layer unlit material {key} has unreviewed sampler").into());
    }
    Ok((index, image, mode))
}
