use super::*;

#[derive(Clone, Copy)]
pub(super) struct FalloffRecipe {
    pub(super) index: usize,
    pub(super) vertex: &'static str,
    pub(super) pixel: &'static str,
    pub(super) mode: &'static str,
}

pub(super) fn falloff_recipe(family: &str) -> Option<FalloffRecipe> {
    Some(match family {
        "mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777" => FalloffRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_197b665e.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_997302f8.hlsl",
            mode: "add",
        },
        "mc_sw4_3d_unlit_3layer_nofog_falloff_5z943qe4" => FalloffRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_197b665e.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_597f540d.hlsl",
            mode: "opaque",
        },
        "mc_sw4_3d_unlit_3layer_nofog_falloff_5z909874" => FalloffRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_197b665e.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_c21aa953.hlsl",
            mode: "blend",
        },
        "mc_sw4_3d_unlit_3layer_nofog_falloff_zfeather_wq4jwzz6" => FalloffRecipe {
            index: 3,
            vertex: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_zfeather_a5140590.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_zfeather_4b540a08.hlsl",
            mode: "zfeather",
        },
        "mlv_sw4_3d_unlit_3layer_nofog_falloff_fjz6ze0u" => FalloffRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_5f626c64.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_df6a08c2.hlsl",
            mode: "add",
        },
        "mc_sw4_3d_unlit_3layer_nofog_qw71w785" => FalloffRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_3layer_nofog_falloff_197b665e.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_3layer_nofog_61f120bd.hlsl",
            mode: "layer_blend",
        },
        "mc_sw4_3d_unlit_4layer_nofog_eee768jw" => FalloffRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_4layer_nofog_cd11c014.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_4layer_nofog_8b288e65.hlsl",
            mode: "four_add",
        },
        "mlv_sw4_3d_unlit_4layer_nofog_2j0ezz67" => FalloffRecipe {
            index: 4,
            vertex: "pimp_shader_sw4_3d_unlit_4layer_nofog_3874c1d6.hlsl",
            pixel: "pimp_shader_sw4_3d_unlit_4layer_nofog_7e4d8fa7.hlsl",
            mode: "four_add",
        },
        _ => return None,
    })
}

pub(super) fn validate_falloff(
    key: &str,
    material: &Value,
    recipe: FalloffRecipe,
) -> Result<(usize, &'static str)> {
    let family = material["technique_set"]
        .as_str()
        .ok_or("Missing three-layer falloff family")?;
    let FalloffRecipe {
        index,
        vertex,
        pixel,
        mode,
    } = recipe;
    let feather = mode == "zfeather";
    let four = mode == "four_add";
    let mut routes = vec![json!({"source":0,"destination":0})];
    if feather {
        routes.push(json!({"source":1,"destination":2}));
    }
    routes.extend([
        json!({"source":2,"destination":5}),
        json!({"source":3,"destination":1}),
        json!({"source":4,"destination":7}),
    ]);
    if family.starts_with("mlv_") {
        routes.push(json!({"source":5,"destination":6}));
    }
    let mut constants = BTreeMap::from([
        (3320325482_u32, (80, 8)),
        (3286424575, (88, 8)),
        (2196952607, (96, 8)),
        (3354717129, (104, 8)),
        (3332229724, (112, 8)),
        (128962428, (120, 8)),
        (3398313000, (128, 8)),
        (1565193213, (136, 8)),
        (475721245, (144, 8)),
        (3337236870, (152, 4)),
        (3376302501, (156, 4)),
        (1886964550, (164, 8)),
        (1519946245, (176, 8)),
        (1339484996, (184, 8)),
        (1550647684, (192, 8)),
        (114353733, (200, 8)),
        (3272022534, (208, 8)),
        (3342767871, (216, 4)),
        (3379389788, (220, 4)),
        (3416155453, (224, 4)),
    ]);
    if mode != "opaque" {
        constants.insert(3410903492, (160, 4));
    }
    if feather {
        constants.extend([
            (14836402, (228, 4)),
            (3185247991, (232, 4)),
            (3173127725, (236, 4)),
            (3371644427, (240, 4)),
        ]);
    } else if four {
        constants.insert(1886964550, (168, 8));
        constants.extend([
            (3432544439, (228, 4)),
            (14836402, (164, 4)),
            (211931156, (232, 4)),
            (3736587663, (240, 8)),
            (3836749946, (248, 8)),
            (3048924506, (256, 8)),
            (3813076419, (264, 8)),
            (3762764771, (272, 4)),
            (3756660378, (276, 4)),
            (2389732611, (280, 8)),
        ]);
    } else {
        constants.extend([
            (3432544439, (228, 4)),
            (14836402, (232, 4)),
            (211931156, (236, 4)),
        ]);
        if mode != "layer_blend" {
            constants.extend([(3185247991, (240, 4)), (3173127725, (244, 4))]);
        }
    }
    let mut arguments = vec![
        (3, 67109077_u32, 3, 0, 64),
        (3, 67109093, 0, 576, 64),
        (5, 16777385, 1, 368, 16),
        (if four { 3 } else { 5 }, 16777241, 2, 64, 16),
    ];
    if feather {
        arguments.extend([(4, 18, 0, 259, 1), (5, 16777233, 0, 1520, 16)]);
    } else {
        arguments.extend([
            (3, 16777286, 3, 80, 16),
            (3, 16777287, 3, 96, 16),
            (3, 16777288, 3, 112, 16),
        ]);
    }
    let mut hashes = vec![3714339118_u32, 3714374925, 3714406764];
    if four {
        hashes.push(3714721355);
    }
    let locations = if four {
        vec![259, 514, 769, 1024]
    } else if feather {
        vec![514, 769, 1024]
    } else {
        vec![258, 513, 768]
    };
    arguments.extend(
        hashes
            .iter()
            .copied()
            .zip(locations)
            .map(|(hash, location)| (2, hash, 0, location, 1)),
    );
    arguments.extend(constants.iter().map(|(&hash, &(location, size))| {
        let pixel = matches!(
            hash,
            14836402 | 211931156 | 3337236870 | 3376302501 | 3410903492 | 3432544439 | 3762764771
        );
        (if four && !pixel { 0 } else { 6 }, hash, 2, location, size)
    }));
    let mut state = match mode {
        "opaque" => [403867666_u32, 13],
        "blend" | "layer_blend" => [422084965, 12],
        "zfeather" => [421628194, 12],
        _ => [422086946, 12],
    };
    if family == "mc_sw4_3d_unlit_3layer_nofog_falloff_w840w777"
        && material["name"] == "mc/mtl_p6_zm_tm_pap_glow_effect"
    {
        state[0] = 422070562;
    }
    let vortex = four
        && matches!(
            material["name"].as_str(),
            Some("mc/mtl_zm_tm_elm_portal_vortex" | "mlv/mtl_zm_tm_elm_portal_vortex")
        );
    if vortex {
        state = [422070562, 28];
    }
    for selected in if feather { vec![3] } else { vec![4, 6] } {
        let pass = contract::pass(material, selected)?;
        if pass["vertex_shader"] != vertex
            || pass["pixel_shader"] != pixel
            || pass["vertex_routes"] != json!(routes)
        {
            return Err(format!(
                "Three-layer falloff material {key} has unreviewed programs or vertex routes"
            )
            .into());
        }
        let state_index = material["state_entries"][selected]
            .as_u64()
            .ok_or("Missing three-layer falloff state index")? as usize;
        if material["state_bits"][state_index] != json!(state) {
            return Err(
                format!("Three-layer falloff material {key} has unreviewed render state").into(),
            );
        }
        let active = pass["arguments"]
            .as_array()
            .ok_or("Missing three-layer falloff arguments")?;
        if active.len() != arguments.len()
            || arguments
                .iter()
                .any(|&(kind, value, buffer, location, size)| {
                    active
                        .iter()
                        .filter(|argument| {
                            argument["type"] == kind
                                && argument["value"] == value
                                && argument["buffer"] == buffer
                                && argument["location"] == location
                                && argument["size"] == size
                        })
                        .count()
                        != 1
                })
        {
            return Err(format!(
                "Three-layer falloff material {key} has unreviewed active arguments"
            )
            .into());
        }
    }
    for (&hash, &(_, size)) in &constants {
        let constant = material["constants"]
            .as_array()
            .ok_or("Missing three-layer falloff constants")?
            .iter()
            .find(|constant| constant["hash"] == hash)
            .ok_or("Missing bound three-layer falloff constant")?;
        if (0..size as usize / 4).any(|component| {
            constant["literal"][component]
                .as_f64()
                .is_none_or(|value| !value.is_finite())
        }) {
            return Err(format!(
                "Three-layer falloff material {key} has non-finite constant {hash}"
            )
            .into());
        }
    }
    for &hash in &hashes {
        let binding = material["textures"]
            .as_array()
            .ok_or("Missing three-layer falloff images")?
            .iter()
            .find(|binding| binding["hash"] == hash)
            .ok_or("Missing active three-layer falloff image")?;
        if if four {
            binding["sampler"] != if vortex { 244 } else { 20 }
        } else {
            !matches!(binding["sampler"].as_u64(), Some(20 | 84))
                || (hash == hashes[0] && binding["sampler"] != 20)
        } {
            return Err(
                format!("Three-layer falloff material {key} has unreviewed sampler").into(),
            );
        }
    }
    Ok((index, mode))
}

#[derive(Clone, Copy, Debug)]
pub(super) struct UnlitLayer {
    pub(super) scale: [f32; 2],
    pub(super) offset: [f32; 2],
    pub(super) origin: [f32; 2],
    pub(super) rotation: [f32; 2],
    pub(super) scroll: [f32; 2],
    pub(super) depth: f32,
    pub(super) alpha: Option<f32>,
}

pub(super) fn falloff_pixel(
    samples: [[f32; 4]; 3],
    alpha: [f32; 2],
    last_alpha: Option<f32>,
    script: [f32; 4],
    mode: &str,
) -> [f32; 4] {
    let weights: [f32; 2] = std::array::from_fn(|layer| {
        samples[layer][3] * alpha[layer] * (1. - script[layer].clamp(0., 1.))
    });
    let coverage = if mode == "opaque" {
        1.
    } else {
        ((samples[2][3] * last_alpha.unwrap() * (1. - script[2].clamp(0., 1.)) + weights[1])
            .clamp(0., 1.)
            + weights[0])
            .clamp(0., 1.)
    };
    let mut color: [f32; 3] =
        std::array::from_fn(|component| samples[2][component] * samples[2][3]);
    for layer in [1, 0] {
        for component in 0..3 {
            color[component] += weights[layer] * (samples[layer][component] - color[component]);
        }
    }
    if matches!(mode, "add" | "zfeather") {
        for component in &mut color {
            *component *= coverage;
        }
    }
    [
        color[0],
        color[1],
        color[2],
        if mode == "blend" { coverage.sqrt() } else { 1. },
    ]
}

pub(super) fn four_layer_pixel(
    samples: [[f32; 4]; 4],
    alpha: [f32; 4],
    script: [f32; 4],
) -> [f32; 4] {
    let weights: [f32; 4] = std::array::from_fn(|layer| {
        samples[layer][3] * alpha[layer] * (1. - script[layer].clamp(0., 1.))
    });
    let mut color: [f32; 3] =
        std::array::from_fn(|component| samples[3][component] * samples[3][3]);
    let mut coverage = weights[3];
    for layer in [2, 1, 0] {
        for component in 0..3 {
            color[component] += weights[layer] * (samples[layer][component] - color[component]);
        }
        coverage = (coverage + weights[layer]).clamp(0., 1.);
    }
    [
        color[0] * coverage,
        color[1] * coverage,
        color[2] * coverage,
        1.,
    ]
}

pub(super) fn falloff_image<const N: usize>(
    sources: [&texture::Image; N],
    samplers: [u8; N],
    layers: [UnlitLayer; N],
    script: [f32; 4],
    amount: f32,
    mode: &str,
) -> Result<texture::Image> {
    if sources.iter().any(|source| {
        source.depth != 1
            || source.faces != 1
            || source.width == 0
            || source.height == 0
            || source.levels.is_empty()
    }) {
        return Err("Three-layer falloff input is not a valid 2D image".into());
    }
    let (width, height) = (
        sources.iter().map(|image| image.width).max().unwrap(),
        sources.iter().map(|image| image.height).max().unwrap(),
    );
    let mut output = texture::Image {
        width,
        height,
        depth: 1,
        faces: 1,
        format: 19,
        flags: 0,
        levels: Vec::new(),
    };
    let decoded = sources
        .iter()
        .map(|source| {
            (0..source.levels.len())
                .map(|level| texture::pixels(source, level))
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let rotation = layers.map(|layer| (layer.rotation[0] * f32::from_bits(0x40c90fda)).sin_cos());
    for level in 0..sources
        .iter()
        .map(|source| source.levels.len())
        .max()
        .unwrap()
    {
        let (w, h) = ((width >> level).max(1), (height >> level).max(1));
        let mut bytes = Vec::with_capacity(w * h * 8);
        for y in 0..h {
            for x in 0..w {
                let uv = [(x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32];
                let samples: [[f32; 4]; N] = std::array::from_fn(|index| {
                    let source = sources[index];
                    let layer = layers[index];
                    let (sine, cosine) = rotation[index];
                    let centered: [f32; 2] = std::array::from_fn(|axis| {
                        (uv[axis] - layer.origin[axis] + layer.offset[axis]) * layer.scale[axis]
                    });
                    let uv = [
                        cosine * centered[0] + sine * centered[1] + layer.origin[0],
                        -sine * centered[0] + cosine * centered[1] + layer.origin[1],
                    ];
                    let bias = width.max(height).ilog2() - source.width.max(source.height).ilog2();
                    let lod = level
                        .saturating_sub(bias as usize)
                        .min(source.levels.len() - 1);
                    texture::sample(
                        &decoded[index][lod],
                        (source.width >> lod).max(1),
                        (source.height >> lod).max(1),
                        uv,
                        samplers[index],
                    )
                });
                let mut pixel = if mode == "four_add" {
                    four_layer_pixel(
                        [samples[0], samples[1], samples[2], samples[3]],
                        [
                            layers[0].alpha.unwrap(),
                            layers[1].alpha.unwrap(),
                            layers[2].alpha.unwrap(),
                            layers[3].alpha.unwrap(),
                        ],
                        script,
                    )
                } else {
                    falloff_pixel(
                        [samples[0], samples[1], samples[2]],
                        [layers[0].alpha.unwrap(), layers[1].alpha.unwrap()],
                        layers[2].alpha,
                        script,
                        if mode == "layer_blend" { "blend" } else { mode },
                    )
                };
                for component in &mut pixel[..3] {
                    *component = (*component * amount).powi(2);
                }
                if pixel
                    .iter()
                    .any(|value| !value.is_finite() || !(0. ..=65504.).contains(value))
                    || pixel[3] > 1.
                {
                    return Err(
                        "Three-layer falloff color exceeds the finite half-float/coverage domain"
                            .into(),
                    );
                }
                for component in pixel {
                    bytes.extend(half::f16::from_f32(component).to_le_bytes());
                }
            }
        }
        output.levels.push(bytes);
    }
    Ok(output)
}

pub(super) fn convert_falloff(
    key: &str,
    native: &Value,
    recipe: FalloffRecipe,
    images: &BTreeMap<String, texture::Image>,
    shared: &mut BTreeMap<(String, String), String>,
    pack: &mut Pack,
) -> Result<Value> {
    let (index, mode) = validate_falloff(key, native, recipe)?;
    let four = mode == "four_add";
    let plain = four || mode == "layer_blend";
    let constant = |hash: u32| -> Result<[f32; 4]> {
        let row = native["constants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|constant| constant["hash"] == hash)
            .ok_or("Missing bound three-layer falloff parameter")?;
        Ok(serde_json::from_value(row["literal"].clone())?)
    };
    let vec2 = |hash| -> Result<[f32; 2]> {
        let value = constant(hash)?;
        Ok([value[0], value[1]])
    };
    let mut parameter_hashes = vec![
        [
            3320325482, 3286424575, 3272022534, 1886964550, 2196952607, 3342767871, 3337236870,
        ],
        [
            3354717129, 128962428, 114353733, 1519946245, 3332229724, 3379389788, 3376302501,
        ],
        [
            3398313000, 1565193213, 1550647684, 1339484996, 475721245, 3416155453, 3410903492,
        ],
    ];
    if four {
        parameter_hashes.push([
            3736587663, 3836749946, 3813076419, 2389732611, 3048924506, 3756660378, 3762764771,
        ]);
    }
    let layers = parameter_hashes
        .into_iter()
        .enumerate()
        .map(|(index, hashes)| {
            Ok(UnlitLayer {
                scale: vec2(hashes[0])?,
                offset: vec2(hashes[1])?,
                origin: vec2(hashes[2])?,
                rotation: vec2(hashes[3])?,
                scroll: vec2(hashes[4])?,
                depth: constant(hashes[5])?[0],
                alpha: if index == 2 && mode == "opaque" {
                    None
                } else {
                    Some(constant(hashes[6])?[0])
                },
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let mut sources = Vec::new();
    let mut samplers = Vec::new();
    let mut source_layers = Vec::new();
    let mut names = Vec::new();
    let mut hashes = vec![3714339118_u32, 3714374925, 3714406764];
    if four {
        hashes.push(3714721355);
    }
    for (index, hash) in hashes.into_iter().enumerate() {
        let binding = native["textures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|binding| binding["hash"] == hash)
            .unwrap();
        let name = binding["image"]
            .as_str()
            .ok_or("Missing three-layer falloff source image name")?
            .to_ascii_lowercase();
        let source = images
            .get(&name)
            .ok_or_else(|| format!("Missing three-layer falloff image {name}"))?;
        let sampler = binding["sampler"].as_u64().unwrap() as u8;
        let source_key = (name.clone(), "source_color".into());
        let uri = if let Some(uri) = shared.get(&source_key) {
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
        let layer = layers[index];
        source_layers.push(json!({"uri":uri,"layer":index,"sampler":sampler,"color_space":"native_encoded",
            "scale":layer.scale,"offset":layer.offset,"origin":layer.origin,"rotation":layer.rotation,"scroll":layer.scroll,"depth":layer.depth,"alpha":layer.alpha}));
        sources.push(source);
        samplers.push(sampler);
        names.push(name);
    }
    let script = if native["code_constants"]["169"].is_null() {
        if plain { Some([0.; 4]) } else { None }
    } else {
        let state: [f32; 4] = serde_json::from_value(native["code_constants"]["169"].clone())?;
        if state.iter().any(|value| !value.is_finite()) {
            return Err("Non-finite source scriptVector0 state".into());
        }
        Some(state)
    };
    let amount = constant(14836402)?[0];
    let mut channels = Vec::new();
    if let Some(script) = script {
        let color_key = (
            names.join("|"),
            format!(
                "falloff:{mode}:{layers:?}:{script:?}:{}:{samplers:?}",
                amount.to_bits()
            ),
        );
        let uri = if let Some(uri) = shared.get(&color_key) {
            uri.clone()
        } else {
            let image = if four {
                falloff_image(
                    [sources[0], sources[1], sources[2], sources[3]],
                    [samplers[0], samplers[1], samplers[2], samplers[3]],
                    [layers[0], layers[1], layers[2], layers[3]],
                    script,
                    amount,
                    mode,
                )?
            } else {
                falloff_image(
                    [sources[0], sources[1], sources[2]],
                    [samplers[0], samplers[1], samplers[2]],
                    [layers[0], layers[1], layers[2]],
                    script,
                    amount,
                    mode,
                )?
            };
            let uri = store_image(
                &image,
                format!("prepared/material-channels/{}/color.ktx2", shared.len()),
                false,
                "color",
                pack,
            )?;
            shared.insert(color_key, uri.clone());
            uri
        };
        channels
            .push(json!({"uri":uri,"role":"color","layer":0,"color_space":"linear","sampler":20}));
    }
    let selected = native["state_entries"][index].as_u64().unwrap() as usize;
    let state = native["state_bits"][selected][0].as_u64().unwrap();
    let cull = match state >> 14 & 3 {
        1 => "none",
        2 => "back",
        3 => "front",
        _ => return Err("Unknown three-layer falloff cull state".into()),
    };
    let mut surface = json!({"version":3,"identity":key,"native":native,"channels":channels,"source_layers":source_layers,
        "standard":{"unlit":true,"roughness":1.,"metallic":0.,"reflectance":0.,"base_color":[1.,1.,1.,1.]},
        "presentation":{"alpha":if mode == "zfeather" || four {"add"} else if mode == "layer_blend" {"blend"} else {mode},"cull":cull,"fog":false,"visible":state & (1<<27) == 0},
        "source_script_layers":{"code_constant":169,"state":script,"layer_order":if four {vec![3,2,1,0]} else {vec![2,1,0]},
            "weights":"texture_alpha_times_layer_alpha_times_one_minus_saturated_script_component",
            "rgb":if four {"layer4_rgb_times_layer4_texture_alpha_then_mix_layer3_then_layer2_then_layer1_without_saturating_mix_weights"} else {"layer3_rgb_times_layer3_texture_alpha_then_mix_layer2_then_layer1_without_saturating_mix_weights"},
            "coverage":if mode == "opaque" {"unused"} else {"add_each_preceding_layer_weight_and_saturate_after_each_step"},
            "layer3_alpha_affects_rgb":four,"layer3_alpha_bound":mode != "opaque","last_layer_alpha_affects_rgb":false,
            "rgb_times_coverage":matches!(mode,"add" | "zfeather" | "four_add"),
            "fragment_alpha":if matches!(mode,"blend" | "layer_blend") {"sqrt_coverage"} else {"one"}},
        "source_layer_uv":{"game_time_code":25,"game_time_component":3,"view_parallax":"negative_view_tangent_xy_divided_by_positive_view_normal_z_else_zero",
            "depth_epsilon":if four {f32::from_bits(0x3727c5ac)} else {0.},"evaluation":if four {"vertex_then_interpolate"} else {"fragment"},
            "operation":"scale_centered_uv_plus_scaled_offset_and_scaled_view_parallax_then_rotate_then_add_scaled_time_scroll_and_origin",
            "rotation_operation":"rotation_x_times_one_plus_rotation_y_times_game_time_w","rotation_tau":f32::from_bits(0x40c90fda)},
        "source_emissive":{"amount":amount,"rgb":if plain {"square_layer_color_times_hdr_amount"} else {"square_layer_color_times_view_falloff_times_hdr_amount"},"output_transfer":"sqrt_after_reflection_and_scene_hdr_control0"},
        "source_blend":{"rgb":[state&15,state>>4&15,state>>8&7],"alpha":[state>>16&15,state>>20&15,state>>24&7]},
        "approximations":["Explicit scriptVector0 states evaluate the three-layer color and coverage with static authored UV terms at source mip resolution",
            "Live view parallax, time, view falloff, scene lighting and framebuffer terms remain separate from the converted texture stage"]});
    if !plain {
        surface["source_falloff"] = json!({"power":constant(3185247991)?[0],"boost":constant(3173127725)?[0],
        "operation":"pow_saturate_dot_negative_normalized_view_position_and_normalized_normal_times_boost"});
    }
    if plain {
        surface["source_script_layers"]["base_state"] = json!([0., 0., 0., 0.]);
        surface["source_script_layers"]["base_state_origin"] =
            json!("normal_view_zero_template_preserved_by_type2_constant_copy");
        surface["source_script_layers"]["active_override_scope"] =
            json!("per_object_mapped_scriptVector0_replaces_base_state");
    }
    if mode == "zfeather" {
        surface["source_vertex_color_transfer"] =
            json!("linear_primary_vertex_rgb_and_squared_primary_vertex_alpha");
        surface["source_depth_fade"] = json!({"code_image":18,"near_plane_code":17,"fade_factor":constant(3371644427)?[0],
            "depth_epsilon":f32::from_bits(0x322bcc77),"fade_epsilon":f32::from_bits(0x38d1b717),
            "operation":"saturate_near_plane_divided_by_max_abs_scene_depth_minus_clip_w_divided_by_max_fade_factor",
            "rgb":"layer_color_times_coverage_times_vertex_rgb_times_vertex_alpha_squared_times_depth_fade"});
        surface["pending_runtime"] = json!(
            "Native three-layer feather requires the live per-object scriptVector0 producer, native clock/offset UV controls, view parallax and falloff, primary vertex RGB/alpha, scene depth/near-plane/clip-w fade, global HDR control0 and native additive/destination-alpha gamma2 framebuffer composition"
        );
    } else {
        surface["source_vertex_color_transfer"] = json!("unused_primary_color");
        surface["source_reflection"] = json!({"amount":constant(211931156)?[0],"gloss":constant(3432544439)?[0],"lod":"four_times_one_minus_gloss",
            "weight":"saturate_three_quarters_plus_quarter_gloss_times_reflection_amount","probe_decode":"rgb_divided_by_alpha_plus_epsilon",
            "decode_epsilon":f32::from_bits(0x358637bd),"response":"separate_grid_and_reflection_spherical_harmonics_ratio",
            "secondary_vertex_alpha_red_bias":if native["technique_set"].as_str().unwrap().starts_with("mlv_") {0.00001} else {0.}});
        surface["source_reflection"]["active"] = json!(constant(211931156)?[0] != 0.);
        surface["pending_runtime"] = json!(if four {
            "Native four-layer no-fog surface retains per-object scriptVector0 overrides, native clock UV rotation/scroll, vertex-evaluated positive-facing parallax including its depth epsilon, global HDR control0 and native additive/destination-alpha gamma2 composition; nonzero reflection requires its separate view-dependent SH consumer"
        } else if plain {
            "Native three-layer no-fog surface retains per-object scriptVector0 overrides, native clock UV rotation, nonzero view-dependent reflection/SH response, global HDR control0 and native source-alpha/destination-alpha gamma2 composition; authored zero layer depths leave parallax inactive"
        } else {
            "Native three-layer falloff requires the live per-object scriptVector0 producer, native clock/offset UV controls, view parallax and falloff, view-dependent reflection/SH response, global HDR control0 and native blend/destination-alpha gamma2 framebuffer composition"
        });
    }
    surface["source_script_layers"]["composed_color_available"] = json!(script.is_some());
    if four && native["state_bits"][selected][1] == 28 {
        let pending = surface["pending_runtime"].as_str().unwrap().to_owned();
        surface["pending_runtime"] = json!(format!(
            "{pending}; native polygon-offset mode1 remains unconnected"
        ));
    }
    Ok(
        json!({"name":key,"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},"extras":{"surface":surface}}),
    )
}
