use super::*;

pub(super) struct Motion {
    scale: [f32; 2],
    offset: [f32; 2],
    origin: [f32; 2],
    rotation: [f32; 2],
    scroll: [f32; 2],
    tau: f32,
}

impl Motion {
    fn transform(&self, seconds: f32) -> [f32; 6] {
        let angle = self.rotation[0] * (1. + self.rotation[1] * seconds) * self.tau;
        let (sine, cosine) = angle.sin_cos();
        let centered = std::array::from_fn::<_, 2, _>(|axis| {
            (self.offset[axis] - self.origin[axis]) * self.scale[axis]
        });
        [
            cosine * self.scale[0],
            sine * self.scale[1],
            -sine * self.scale[0],
            cosine * self.scale[1],
            cosine * centered[0]
                + sine * centered[1]
                + seconds * self.scroll[0] * self.scale[0]
                + self.origin[0],
            -sine * centered[0]
                + cosine * centered[1]
                + seconds * self.scroll[1] * self.scale[1]
                + self.origin[1],
        ]
    }

    fn animated(&self) -> bool {
        self.rotation[0] * self.rotation[1] != 0.
            || (0..2).any(|axis| self.scroll[axis] * self.scale[axis] != 0.)
    }

    fn density(&self, image: &Input, axis: usize) -> f32 {
        if self.rotation[0] * self.rotation[1] == 0. {
            let matrix = self.transform(0.);
            if matrix[axis] == 0. {
                return matrix[axis + 2].abs() * image.height as f32;
            }
            if matrix[axis + 2] == 0. {
                return matrix[axis].abs() * image.width as f32;
            }
        }
        self.scale[axis].abs() * image.width.max(image.height) as f32
    }
}

pub(super) fn read(
    mode: &str,
    native: &Value,
    images: Option<&BTreeMap<String, texture::Image>>,
) -> Result<Recipe> {
    read_frame(mode, native, images, 0.)
}

pub(super) fn read_frame(
    mode: &str,
    native: &Value,
    images: Option<&BTreeMap<String, texture::Image>>,
    seconds: f32,
) -> Result<Recipe> {
    if !seconds.is_finite() {
        return Err("Script-layer bake time is non-finite".into());
    }
    crate::material::validate_script_layers(native)?;
    let code = match mode {
        "single" => 0,
        "normal_script" | "normal_script_unlit" => 1,
        "opaque" => 2,
        "blend" => 3,
        "add" => 4,
        "zfeather" => 5,
        "layer_blend" => 6,
        "four_add" => 7,
        "four_opaque" => 8,
        _ => return Err("Unsupported baked script-layer mode".into()),
    };
    let count = if mode == "single" {
        1
    } else if code >= 7 {
        4
    } else {
        3
    };
    let literal = |hash: u32| -> Result<[f32; 4]> {
        let row = native["constants"]
            .as_array()
            .ok_or("Missing script-layer constants")?
            .iter()
            .find(|row| row["hash"] == hash)
            .ok_or("Missing script-layer parameter")?;
        Ok(serde_json::from_value(row["literal"].clone())?)
    };
    let parameters = [
        [
            3320325482, 3286424575, 3272022534, 1886964550, 2196952607, 3337236870,
        ],
        [
            3354717129, 128962428, 114353733, 1519946245, 3332229724, 3376302501,
        ],
        [
            3398313000, 1565193213, 1550647684, 1339484996, 475721245, 3410903492,
        ],
        [
            3736587663, 3836749946, 3813076419, 2389732611, 3048924506, 3762764771,
        ],
    ];
    let hashes = [3714339118_u32, 3714374925, 3714406764, 3714721355];
    let mut inputs = Vec::new();
    let mut alpha = [1.; 4];
    for index in 0..count {
        let [
            scale_hash,
            offset_hash,
            origin_hash,
            rotation_hash,
            scroll_hash,
            alpha_hash,
        ] = parameters[index];
        let scale = literal(scale_hash)?;
        let offset = literal(offset_hash)?;
        let origin = literal(origin_hash)?;
        let rotation = literal(rotation_hash)?;
        let scroll = literal(scroll_hash)?;
        let motion = Motion {
            scale: [scale[0], scale[1]],
            offset: [offset[0], offset[1]],
            origin: [origin[0], origin[1]],
            rotation: [rotation[0], rotation[1]],
            scroll: [scroll[0], scroll[1]],
            tau: if mode == "four_opaque" {
                1.
            } else {
                f32::from_bits(0x40c90fda)
            },
        };
        if !(index == 2 && matches!(mode, "opaque" | "normal_script" | "normal_script_unlit")
            || index == 3 && mode == "four_opaque")
        {
            alpha[index] = literal(alpha_hash)?[0];
        }
        let binding = native["textures"]
            .as_array()
            .ok_or("Missing script-layer textures")?
            .iter()
            .find(|row| row["hash"] == hashes[index])
            .ok_or("Missing script-layer texture")?;
        if let Some(images) = images {
            let name = binding["image"]
                .as_str()
                .ok_or("Missing script-layer image name")?
                .to_ascii_lowercase();
            let image = images
                .get(&name)
                .ok_or_else(|| format!("Missing script-layer image {name}"))?;
            let mut input = Input::read(
                image,
                binding["sampler"]
                    .as_u64()
                    .ok_or("Missing script-layer sampler")? as u8,
            )?;
            input.transform = motion.transform(seconds);
            inputs.push(input);
        }
    }
    let script: [f32; 4] = if native["code_constants"]["169"].is_null() {
        [0.; 4]
    } else {
        serde_json::from_value(native["code_constants"]["169"].clone())?
    };
    let mut parms = [0.; 4];
    let reflective_normal = mode == "normal_script";
    if reflective_normal {
        let scale = literal(1732624492)?;
        let height = literal(2065614284)?[0];
        parms = [
            scale[0],
            scale[1],
            f32::from_bits(0x40808102) * height,
            f32::from_bits(0xc0010204) * height,
        ];
        if let Some(images) = images {
            let binding = crate::material::contract::texture_binding(native, 1507003663)?;
            let image = images
                .get(
                    &binding["image"]
                        .as_str()
                        .ok_or("Missing reflection normal image")?
                        .to_ascii_lowercase(),
                )
                .ok_or("Missing source reflection normal")?;
            let mut input = Input::read(
                image,
                binding["sampler"]
                    .as_u64()
                    .ok_or("Missing reflection normal sampler")? as u8,
            )?;
            input.transform = [scale[0], 0., 0., scale[1], 0., 0.];
            inputs.push(input);
        }
    }
    Ok(Recipe {
        alpha_reveal: false,
        special: 127,
        special_parms: [
            alpha,
            script,
            [
                count as f32,
                code as f32,
                literal(14836402)?[0],
                reflective_normal as u8 as f32,
            ],
        ],
        inputs,
        base_specular: false,
        geometric_normal: false,
        second_additive: false,
        second_normal: 0,
        second_reveal: false,
        second_specular: false,
        second_parms: [0.; 4],
        third_multiply: false,
        layer_cutout: false,
        decal: false,
        multiply: false,
        additive: false,
        blend_color: false,
        reveal: false,
        layer_specular: false,
        base_alpha_response: false,
        parms,
    })
}

fn motions(native: &Value, mode: &str, count: usize) -> Result<Vec<Motion>> {
    let hashes = [
        [3320325482, 3286424575, 3272022534, 1886964550, 2196952607],
        [3354717129, 128962428, 114353733, 1519946245, 3332229724],
        [3398313000, 1565193213, 1550647684, 1339484996, 475721245],
        [3736587663, 3836749946, 3813076419, 2389732611, 3048924506],
    ];
    let mut result: Vec<_> = hashes[..count]
        .iter()
        .map(|hashes| {
            let values = hashes
                .iter()
                .map(|hash| crate::material::contract::literal(native, *hash))
                .collect::<Result<Vec<_>>>()?;
            let pair = |index: usize| [values[index][0], values[index][1]];
            Ok(Motion {
                scale: pair(0),
                offset: pair(1),
                origin: pair(2),
                rotation: pair(3),
                scroll: pair(4),
                tau: if mode == "four_opaque" {
                    1.
                } else {
                    f32::from_bits(0x40c90fda)
                },
            })
        })
        .collect::<Result<_>>()?;
    if mode == "normal_script" {
        let scale = crate::material::contract::literal(native, 1732624492)?;
        result.push(Motion {
            scale: [scale[0], scale[1]],
            offset: [0.; 2],
            origin: [0.; 2],
            rotation: [0.; 2],
            scroll: [0.; 2],
            tau: 1.,
        });
    }
    Ok(result)
}

fn loop_samples(motion: &[Motion], recipe: &Recipe, bounds: [[f32; 2]; 2]) -> Result<usize> {
    if !motion.iter().any(Motion::animated) {
        return Ok(1);
    }
    let mut rates = Vec::new();
    for (motion, input) in motion.iter().zip(&recipe.inputs) {
        let radius = (0..2)
            .map(|axis| {
                (bounds[0][axis] - motion.origin[axis] + motion.offset[axis])
                    .abs()
                    .max((bounds[1][axis] - motion.origin[axis] + motion.offset[axis]).abs())
                    * motion.scale[axis].abs()
            })
            .sum::<f32>() as f64;
        let rotation =
            f64::from(motion.rotation[0]) * f64::from(motion.rotation[1]) * f64::from(motion.tau)
                / std::f64::consts::TAU;
        if rotation != 0. {
            rates.push((
                rotation,
                radius.max(1.) * input.width.max(input.height) as f64 * std::f64::consts::TAU,
            ));
        }
        for axis in 0..2 {
            let scroll = f64::from(motion.scroll[axis]) * f64::from(motion.scale[axis]);
            if scroll == 0. {
                continue;
            }
            if input.sampler & (32 << axis) != 0 {
                return Err(
                    "A moving clamped script-layer axis has no repeating baked sequence".into(),
                );
            }
            rates.push((
                scroll,
                if axis == 0 { input.width } else { input.height } as f64,
            ));
        }
    }
    for frames in 1..=2_000_000 {
        let seconds = frames as f64 / 20.;
        if rates.iter().all(|(rate, texels)| {
            let phase = rate * seconds;
            phase.round() != 0. && (phase - phase.round()).abs() * texels <= 0.125
        }) {
            return Ok(frames);
        }
    }
    Err("Authored script-layer motion does not close within 100000 seconds at one eighth of a source texel".into())
}

struct Plan {
    recipe: Recipe,
    motion: Vec<Motion>,
    bounds: [[f32; 2]; 2],
    frames: usize,
    states: Vec<[f32; 4]>,
    periodic: bool,
    dimensions: [usize; 2],
}

fn plan(
    mode: &str,
    native: &Value,
    surfaces: &[Surface],
    images: &BTreeMap<String, texture::Image>,
) -> Result<Plan> {
    let recipe = read(mode, native, Some(images))?;
    let count = recipe.special_parms[2][0] as usize;
    let motion = motions(native, mode, count)?;
    let mut bounds = [[f32::INFINITY; 2], [f32::NEG_INFINITY; 2]];
    for uv in surfaces.iter().flat_map(|surface| &surface.uv) {
        for axis in 0..2 {
            bounds[0][axis] = bounds[0][axis].min(uv[axis]);
            bounds[1][axis] = bounds[1][axis].max(uv[axis]);
        }
    }
    if !bounds.iter().flatten().all(|value| value.is_finite()) {
        return Err("Script-layer bake has no finite source UV domain".into());
    }
    let frames = loop_samples(&motion, &recipe, bounds)?;
    let states: Vec<[f32; 4]> = if native["baked_script_states"].is_null() {
        vec![recipe.special_parms[1]]
    } else {
        serde_json::from_value(native["baked_script_states"].clone())?
    };
    if states.is_empty() || states.iter().flatten().any(|value| !value.is_finite()) {
        return Err("Invalid baked native script states".into());
    }
    let periodic = motion.iter().zip(&recipe.inputs).all(|(motion, image)| {
        let transform = motion.transform(0.);
        motion.rotation[0] * motion.rotation[1] == 0.
            && transform[..4]
                .iter()
                .all(|value| (*value - value.round()).abs() < 0.000001)
            && (0..2).all(|axis| image.sampler & (32 << axis) == 0 || motion.scale[axis] == 0.)
    });
    if periodic {
        bounds = [[0., 0.], [1., 1.]];
    }
    let dimensions: [usize; 2] = std::array::from_fn(|axis| {
        let density = motion
            .iter()
            .zip(&recipe.inputs)
            .map(|(motion, image)| motion.density(image, axis))
            .fold(0_f32, f32::max);
        (((bounds[1][axis] - bounds[0][axis]) * density).ceil() as usize)
            .max(4)
            .next_multiple_of(4)
    });
    if dimensions.iter().any(|value| *value > 16384) {
        return Err(format!("Baked script-layer source UV domain requires {dimensions:?}, exceeding the runtime texture limit").into());
    }
    Ok(Plan {
        recipe,
        motion,
        bounds,
        frames,
        states,
        periodic,
        dimensions,
    })
}

fn state_weights(state: f32) -> [f64; 5] {
    std::array::from_fn(|index| {
        (0..5)
            .filter(|other| *other != index)
            .map(|other| (state as f64 * 4. - other as f64) / (index as f64 - other as f64))
            .product()
    })
}

pub(super) fn bake(
    mode: &str,
    native: &Value,
    surfaces: &[Surface],
    images: &BTreeMap<String, texture::Image>,
    converted: &mut BTreeMap<String, Value>,
    pack: &mut Pack,
) -> Result<Vec<Surface>> {
    let identity = native["name"]
        .as_str()
        .ok_or("Hybrid material has no source name")?
        .to_ascii_lowercase();
    let primary = converted
        .get(&identity)
        .ok_or("Hybrid primary material is absent")?["extras"]["surface"]
        .clone();
    let mut geometry = surfaces.to_vec();
    for surface in &mut geometry {
        surface.lightmap_uv = surface.uv.clone();
    }
    if !matches!(
        mode,
        "normal_script" | "normal_script_unlit" | "opaque" | "four_opaque"
    ) {
        let surface = &mut converted.get_mut(&identity).unwrap()["extras"]["surface"];
        surface.as_object_mut().unwrap().remove("pending_runtime");
        surface["source_script_layers"]["consumer"] = json!("native_hybrid_layers");
        surface["hybrid"] = json!({"ray_visible":false});
        return Ok(geometry);
    }
    let Plan {
        mut recipe,
        motion,
        bounds,
        frames,
        states,
        periodic,
        dimensions,
    } = plan(mode, native, surfaces, images)?;
    let [width, height] = dimensions;
    let base = format!("prepared/materials/{}/average", converted.len());
    let tint = matches!(mode, "normal_script" | "normal_script_unlit");
    let first_color = geometry[0].colors[0];
    let atlas = tint
        && geometry
            .iter()
            .flat_map(|surface| &surface.colors)
            .any(|color| color[..3] != first_color[..3]);
    let mut banks = BTreeMap::<String, Vec<Value>>::new();
    let mut materials = BTreeMap::<String, Value>::new();
    let mut remapped = Vec::new();
    let mut integrals = BTreeMap::<[u32; 3], Vec<[Vec<[f32; 4]>; 2]>>::new();
    println!(
        "Integrating static ray average {}: {} source-loop samples at 20 Hz, {} script states, {:?} texels; transparent effects excluded",
        native["name"],
        frames,
        states.len(),
        dimensions
    );
    let source_baker = gpu::Baker::new(&recipe)?;
    for (bank, state) in states.iter().copied().enumerate() {
        recipe = read(mode, native, Some(images))?;
        recipe.special_parms[1] = state;
        let integrate = |state| {
            source_baker.average(
                width,
                height,
                bounds,
                frames,
                state,
                |sample| {
                    let seconds = if frames == 1 {
                        0.
                    } else {
                        (sample as f32 + 0.5) / 20.
                    };
                    motion
                        .iter()
                        .map(|motion| motion.transform(seconds))
                        .collect()
                },
                false,
            )
        };
        let [mut colors, normals] = if states.len() > 5 && tint && (0. ..=1.).contains(&state[0]) {
            let key = [state[1].to_bits(), state[2].to_bits(), state[3].to_bits()];
            if !integrals.contains_key(&key) {
                let mut basis = Vec::new();
                for index in 0..5 {
                    let mut anchor = state;
                    anchor[0] = index as f32 / 4.;
                    basis.push(integrate(anchor)?);
                }
                integrals.insert(key, basis);
            }
            let basis = &integrals[&key];
            let weights = state_weights(state[0]);
            let colors = (0..width * height)
                .map(|pixel| {
                    std::array::from_fn(|channel| {
                        (0..5)
                            .map(|index| weights[index] * basis[index][0][pixel][channel] as f64)
                            .sum::<f64>()
                            .max(0.) as f32
                    })
                })
                .collect();
            [colors, basis[0][1].clone()]
        } else {
            integrate(state)?
        };
        if tint && !atlas {
            let tint = first_color.map(|value| value as f32 / 255.);
            for pixel in &mut colors {
                for axis in 0..3 {
                    pixel[axis] *= tint[axis] * tint[axis];
                }
            }
        }
        let mut current = BTreeMap::new();
        let uri = format!("{base}/states/{bank}");
        let pieces = if atlas {
            let span = [bounds[1][0] - bounds[0][0], bounds[1][1] - bounds[0][1]];
            let transform = [
                1. / span[0].max(f32::EPSILON),
                0.,
                0.,
                1. / span[1].max(f32::EPSILON),
                -bounds[0][0] / span[0].max(f32::EPSILON),
                -bounds[0][1] / span[1].max(f32::EPSILON),
            ];
            recipe.inputs = vec![Input {
                transform,
                width,
                height,
                sampler: if periodic { 20 } else { 116 },
                pixels: colors,
            }];
            if mode == "normal_script" {
                recipe.inputs.push(Input {
                    transform,
                    width,
                    height,
                    sampler: if periodic { 20 } else { 116 },
                    pixels: normals,
                });
            }
            recipe.special_parms[2][3] = if mode == "normal_script" { 3. } else { 2. };
            let baker = gpu::Baker::new(&recipe)?;
            let geometry = geometry
                .iter()
                .map(Surface::unindex)
                .collect::<Result<Vec<_>>>()?;
            layered(native, &geometry, &recipe, &baker, &uri, &mut current, pack)?
        } else {
            let value = store(
                native,
                &format!("{uri}/0"),
                width,
                height,
                [
                    colors,
                    if mode == "normal_script" {
                        normals
                    } else {
                        Vec::new()
                    },
                    Vec::new(),
                    Vec::new(),
                ],
                if periodic { 20 } else { 116 },
                0,
                pack,
            )?;
            current.insert(identity.clone(), value);
            let mut pieces = geometry.clone();
            if !periodic {
                for surface in &mut pieces {
                    for uv in &mut surface.uv {
                        for axis in 0..2 {
                            uv[axis] = (uv[axis] - bounds[0][axis])
                                / (bounds[1][axis] - bounds[0][axis]).max(f32::EPSILON);
                        }
                    }
                }
            }
            pieces
        };
        let identities: std::collections::BTreeSet<_> = pieces
            .iter()
            .map(|surface| surface.material.clone())
            .collect();
        for identity in identities {
            let value = current
                .remove(&identity)
                .ok_or("Static ray average page material is absent")?;
            let surface = &value["extras"]["surface"];
            banks
                .entry(identity.clone())
                .or_default()
                .push(json!({"channels":surface["channels"],"standard":surface["standard"]}));
            materials.entry(identity).or_insert(value);
        }
        if bank == 0 {
            remapped = pieces;
        }
        println!(
            "Integrated ray state {}/{} for {}",
            bank + 1,
            states.len(),
            native["name"]
        );
    }
    for (name, mut material) in materials {
        let surface = &mut material["extras"]["surface"];
        for (key, value) in primary.as_object().unwrap() {
            if key.starts_with("source_") {
                surface[key] = value.clone();
            }
        }
        surface.as_object_mut().unwrap().remove("pending_runtime");
        surface["source_script_layers"]["consumer"] = json!("native_hybrid_layers");
        surface["hybrid"] = json!({"ray_visible":true,"ray_average":{"source_loop_samples":frames,"sample_hz":20.,
            "loop_seconds":if frames == 1 {0.} else {frames as f64 / 20.},"color_space":"linear_radiance",
            "integration":"uniform_midpoint_with_compensated_summation","primary_uv_channel":1}});
        if states.len() > 1 {
            surface["static_states"] =
                json!({"states":states,"surfaces":banks.remove(&name).unwrap()});
        }
        surface["approximations"].as_array_mut().unwrap().push(json!("Secondary rays use a finite temporal integral of source linear radiance over the authored repeat period at 20 Hz. Primary visibility retains source animation and view dependence. Native reflection probes use the shared GGX fit"));
        if name != identity && !converted.contains_key(&identity) {
            let mut alias = material.clone();
            alias["name"] = json!(identity);
            alias["extras"]["surface"]["identity"] = json!(identity);
            converted.insert(identity.clone(), alias);
        }
        converted.insert(name, material);
    }
    if let Some(first) = remapped
        .first()
        .and_then(|surface| converted.get(&surface.material))
        .cloned()
    {
        let mut alias = first;
        alias["name"] = json!(identity);
        alias["extras"]["surface"]["identity"] = json!(identity);
        converted.insert(identity, alias);
    }
    Ok(remapped)
}

pub(super) fn finish(native: &Value, mode: &str, material: &mut Value) -> Result<()> {
    let index = crate::material::validate_script_layers(native)?;
    let selected = native["state_entries"][index]
        .as_u64()
        .ok_or("Missing script-layer state")? as usize;
    let state = native["state_bits"][selected][0]
        .as_u64()
        .ok_or("Missing script-layer state bits")?;
    let surface = &mut material["extras"]["surface"];
    surface["standard"]["unlit"] = json!(false);
    surface["standard"]["emissive_exposure_weight"] = json!(1.);
    surface["standard"]["base_color"] = json!([0., 0., 0., 1.]);
    let alpha_blend = matches!(mode, "single" | "blend" | "layer_blend");
    let channels = surface["channels"]
        .as_array_mut()
        .ok_or("Baked script channels are missing")?;
    let color = channels
        .iter()
        .find(|channel| channel["role"] == "color" || channel["role"] == "emissive")
        .cloned()
        .ok_or("Baked script color is missing")?;
    if !alpha_blend {
        channels.retain(|channel| channel["role"] != "color");
    }
    channels.retain(|channel| channel["role"] != "emissive");
    let mut emissive = color;
    emissive["role"] = json!("emissive");
    channels.push(emissive);
    factors(native, mode, &mut surface["standard"])?;
    surface["presentation"] = json!({
        "alpha": match mode { "single" | "blend" | "layer_blend" => "blend", "add" | "zfeather" | "four_add" => "add", _ => "opaque" },
        "cull": match state >> 14 & 3 { 1 => "none", 2 => "back", 3 => "front", _ => return Err("Unknown baked script-layer cull state".into()) },
        "fog": matches!(mode, "normal_script" | "normal_script_unlit" | "four_opaque"),
        "visible": state & (1 << 27) != 0,
    });
    surface["bake"]["script_layers"] = json!(true);
    surface["bake"]["script_state"] = if native["code_constants"]["169"].is_null() {
        json!([0., 0., 0., 0.])
    } else {
        native["code_constants"]["169"].clone()
    };
    Ok(())
}

fn factors(native: &Value, mode: &str, standard: &mut Value) -> Result<()> {
    let boost = if matches!(mode, "opaque" | "blend" | "add" | "zfeather") {
        crate::material::contract::literal(native, 3173127725)?[0]
    } else {
        1.
    };
    let scale = standard["baked_emission_scale"]
        .as_f64()
        .ok_or("Missing baked emission scale")?;
    let angular = if mode == "opaque" {
        let power = crate::material::contract::literal(native, 3185247991)?[0];
        if power < 0. || !power.is_finite() {
            return Err("Invalid source angular falloff exponent".into());
        }
        1. / (1. + f64::from(power))
    } else {
        1.
    };
    let emission = scale * f64::from(boost * boost) * angular;
    standard["emissive"] = json!([emission, emission, emission]);
    if crate::material::script_layer_has_reflection(native["technique_set"].as_str().unwrap()) {
        let gloss = crate::material::contract::literal(native, 3432544439)?[0];
        let amount = crate::material::contract::literal(native, 211931156)?[0]
            * (0.75 + 0.25 * gloss).clamp(0., 1.);
        standard["roughness"] = json!((1. - gloss).clamp(0., 1.));
        standard["reflectance"] = json!((amount.max(0.) / 0.16).sqrt());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_motion_rotates_centered_uv_before_adding_scaled_scroll() {
        let mut motion = Motion {
            scale: [2., 3.],
            offset: [0.1, 0.2],
            origin: [0.5, 0.5],
            rotation: [0.125, 1.],
            scroll: [0.25, -0.5],
            tau: f32::from_bits(0x40c90fda),
        };
        let transform = motion.transform(1.);
        let uv = [0.75, 0.8];
        let result = [
            transform[0] * uv[0] + transform[1] * uv[1] + transform[4],
            transform[2] * uv[0] + transform[3] * uv[1] + transform[5],
        ];
        assert!((result[0] - 2.5).abs() < 0.00001);
        assert!((result[1] + 1.7).abs() < 0.00001);
        let image = Input {
            transform: [1., 0., 0., 1., 0., 0.],
            width: 512,
            height: 256,
            sampler: 20,
            pixels: Vec::new(),
        };
        assert_eq!(motion.density(&image, 0), 1024.);
        assert_eq!(motion.density(&image, 1), 1536.);
        motion.rotation = [0.; 2];
        assert_eq!(motion.density(&image, 0), 1024.);
        assert_eq!(motion.density(&image, 1), 768.);
    }

    #[test]
    #[ignore = "requires the authorized Steam installation"]
    fn installed_layers_preserve_static_wallbuy_and_require_temporal_lava_and_generator_bakes() {
        let zones = crate::source::read_zones(&crate::source::installation().unwrap()).unwrap();
        let catalog = crate::catalog::Catalog::new(&zones);
        let names = [
            "wpc/zm_tm_wallbuy_01",
            "wpc/zm_tm_rock_lava_molten",
            "mc/mtl_p6_zm_tm_generator_115_vat",
            "mc/mtl_p6_zm_tm_generator_115_vat_spiral",
            "mc/mtl_zm_tm_elm_portal_vortex",
            "mc/mtl_p6_zm_tm_elm_plinth_glow_blue",
            "mlv/mtl_p6_zm_tm_elm_plinth_glow_blue",
            "mlv/mtl_p6_zm_tm_elm_plinth_glow_purple",
            "mlv/mtl_p6_zm_tm_elm_plinth_glow_red",
            "mlv/mtl_p6_zm_tm_elm_plinth_glow_yellow",
        ]
        .map(str::to_owned)
        .into_iter()
        .collect();
        let materials = crate::material::read_used(&catalog, &names).unwrap();
        let required = crate::material::inputs(&materials, &catalog).unwrap();
        let mut pack = Pack::default();
        let (_, images) = crate::images::convert(
            &crate::source::installation().unwrap(),
            &catalog,
            &mut pack,
            Some(&required),
        )
        .unwrap();
        let samples: Vec<gpu::Sample> = (0..256)
            .map(|index| {
                let mut sample = [0.; 20];
                sample[0] = (index % 16) as f32 / 16. + 0.03125;
                sample[1] = (index / 16) as f32 / 16. + 0.03125;
                sample[8..12].fill(1.);
                sample
            })
            .collect();
        for (name, native) in materials {
            let mode =
                crate::material::script_layer_mode(native["technique_set"].as_str().unwrap())
                    .unwrap();
            let initial = read_frame(mode, &native, Some(&images), 0.).unwrap();
            let motions = motions(&native, mode, initial.special_parms[2][0] as usize).unwrap();
            let frames = loop_samples(&motions, &initial, [[0., 0.], [1., 1.]]).unwrap();
            assert_eq!(frames == 1, name == "wpc/zm_tm_wallbuy_01");
            println!("{name}: {frames} baked time frames");
            let later = read_frame(mode, &native, Some(&images), 45.).unwrap();
            let first = gpu::Baker::new(&initial).unwrap().sample(&samples).unwrap();
            let last = gpu::Baker::new(&later).unwrap().sample(&samples).unwrap();
            let changed = first
                .iter()
                .zip(&last)
                .filter(|(a, b)| a[0].iter().zip(b[0]).any(|(a, b)| (*a - b).abs() > 0.00001))
                .count();
            assert_eq!(
                changed == 0,
                name == "wpc/zm_tm_wallbuy_01",
                "{name}: {changed} changing samples"
            );
            println!("{name}: {changed}/256 samples change after 45 source seconds");
            let mut encoded = Vec::new();
            for (index, pixels) in [&first, &last].into_iter().enumerate() {
                let outputs =
                    std::array::from_fn(|role| pixels.iter().map(|pixel| pixel[role]).collect());
                let value = store(
                    &native,
                    &format!("prepared/frames/{name}/{index}"),
                    16,
                    16,
                    outputs,
                    20,
                    0,
                    &mut pack,
                )
                .unwrap();
                let channels = value["extras"]["surface"]["channels"].as_array().unwrap();
                assert!(
                    channels
                        .iter()
                        .all(|channel| channel["uri"] == channels[0]["uri"])
                );
                let uri = channels[0]["uri"].as_str().unwrap();
                encoded.push(pack.get(uri).unwrap().to_vec());
            }
            assert_eq!(encoded[0] == encoded[1], name == "wpc/zm_tm_wallbuy_01");
            for row in native["constants"].as_array().unwrap() {
                if matches!(
                    row["hash"].as_u64(),
                    Some(
                        1886964550 | 2196952607 | 1519946245 | 3332229724 | 1339484996 | 475721245
                    )
                ) {
                    println!("{name} motion {}: {}", row["hash"], row["literal"]);
                }
            }
        }
    }

    #[test]
    fn gpu_bakes_native_script_weights_after_authored_uv_transform_and_wrap() {
        let first = [[0.8, 0.2, 0.1, 0.25], [0.1, 0.9, 0.2, 0.5]];
        let input = |pixel| Input {
            transform: [1., 0., 0., 1., 0., 0.],
            width: 1,
            height: 1,
            sampler: 1,
            pixels: vec![pixel],
        };
        let mut recipe = Recipe {
            alpha_reveal: false,
            special: 127,
            special_parms: [[0.75, 0.5, 1., 1.], [0.5, 0.25, 0., 0.], [3., 6., 2., 0.]],
            inputs: vec![
                Input {
                    transform: [2., 0., 0., 1., -0.25, 0.],
                    width: 2,
                    height: 1,
                    sampler: 1,
                    pixels: first.to_vec(),
                },
                input([0.1, 0.3, 0.7, 0.6]),
                input([0.4, 0.5, 0.9, 0.8]),
            ],
            base_specular: false,
            geometric_normal: false,
            second_additive: false,
            second_normal: 0,
            second_reveal: false,
            second_specular: false,
            second_parms: [0.; 4],
            third_multiply: false,
            layer_cutout: false,
            decal: false,
            multiply: false,
            additive: false,
            blend_color: false,
            reveal: false,
            layer_specular: false,
            base_alpha_response: false,
            parms: [0.; 4],
        };
        let samples: Vec<gpu::Sample> = [0.125, 0.375, 0.875]
            .into_iter()
            .map(|u| {
                let mut sample = [0.; 20];
                sample[0] = u;
                sample[1] = 0.5;
                sample
            })
            .collect();
        let baker = gpu::Baker::new(&recipe).unwrap();
        let output = baker.sample(&samples).unwrap();
        for (index, pixel) in output.iter().enumerate() {
            let source = first[usize::from(index != 0)];
            let weight = source[3] * 0.75 * 0.5;
            let second = [0.2705, 0.3775, 0.7155];
            for axis in 0..3 {
                let expected =
                    ((second[axis] + weight * (source[axis] - second[axis])) * 2.).powi(2);
                assert!((pixel[0][axis] - expected).abs() < 0.00001);
            }
            assert_eq!(pixel[0][3], 1.);
            assert_eq!(pixel[1], [0., 0., 1., 1.]);
        }
        let mut transforms: Vec<_> = recipe.inputs.iter().map(|input| input.transform).collect();
        transforms[0][4] = 0.25;
        baker
            .set_script_sample(&transforms, recipe.special_parms[1])
            .unwrap();
        let shifted = baker.sample(&samples).unwrap();
        for axis in 0..4 {
            assert!((shifted[0][0][axis] - output[1][0][axis]).abs() < 0.00001);
        }
        baker.set_script_sample(&transforms, [1.; 4]).unwrap();
        let hidden = baker.sample(&samples).unwrap();
        for pixel in hidden {
            assert_eq!(pixel[0][3], 0.);
        }
        recipe.inputs.push(input([0.7, 0.3, 0.1, 0.8]));
        let mut white = samples[0];
        white[8..12].fill(1.);
        let mut tinted = white;
        tinted[8..12].copy_from_slice(&[-1., 0.5, -0.7, -0.5]);
        for mode in 0..=8 {
            recipe.special_parms[2][0] = if mode == 0 {
                1.
            } else if mode >= 7 {
                4.
            } else {
                3.
            };
            recipe.special_parms[2][1] = mode as f32;
            let baker = gpu::Baker::new(&recipe).unwrap();
            let result = baker.sample(&[white, tinted]).unwrap();
            assert_eq!(
                result[0] == result[1],
                !matches!(mode, 1 | 5),
                "native vertex-color mode {mode}"
            );
            let prepared = baker
                .average(
                    2,
                    1,
                    [[0., 0.], [0.5, 1.]],
                    1,
                    recipe.special_parms[1],
                    |_| recipe.inputs.iter().map(|input| input.transform).collect(),
                    matches!(mode, 0 | 3 | 6),
                )
                .unwrap()[0]
                .clone();
            for axis in 0..4 {
                assert!(
                    (prepared[0][axis] - result[0][0][axis]).abs() < 0.00001,
                    "prepared frame mode {mode}, channel {axis}"
                );
            }
            if mode == 8 {
                let prepared = baker
                    .average(
                        513,
                        513,
                        [[0., 0.], [1., 1.]],
                        1,
                        recipe.special_parms[1],
                        |_| recipe.inputs.iter().map(|input| input.transform).collect(),
                        false,
                    )
                    .unwrap()[0]
                    .clone();
                let samples: Vec<_> = [0, 262_143, 262_144, 263_168]
                    .into_iter()
                    .map(|index| {
                        let mut sample = white;
                        sample[0] = ((index % 513) as f32 + 0.5) / 513.;
                        sample[1] = ((index / 513) as f32 + 0.5) / 513.;
                        sample
                    })
                    .collect();
                let reference = baker.sample(&samples).unwrap();
                for (index, expected) in [0, 262_143, 262_144, 263_168].into_iter().zip(reference) {
                    for axis in 0..4 {
                        assert!((prepared[index][axis] - expected[0][axis]).abs() < 0.00001);
                    }
                }
            }
        }
        recipe.inputs = vec![Input {
            transform: [1., 0., 0., 1., 0., 0.],
            width: 2,
            height: 1,
            sampler: 1,
            pixels: vec![[0.1, 0.1, 0.1, 1.], [0.9, 0.9, 0.9, 1.]],
        }];
        recipe.special_parms = [[1.; 4], [0.; 4], [1., 0., 1., 0.]];
        let baker = gpu::Baker::new(&recipe).unwrap();
        let mean = baker
            .average(
                1,
                1,
                [[0., 0.], [0.5, 1.]],
                2,
                [0.; 4],
                |index| vec![[1., 0., 0., 1., index as f32 * 0.5, 0.]],
                true,
            )
            .unwrap();
        assert!((mean[0][0][0] - 0.41).abs() < 0.000001);
        for degree in 0..=4 {
            let weights = state_weights(0.37);
            let value: f64 = (0..5)
                .map(|index| weights[index] * (index as f64 / 4.).powi(degree))
                .sum();
            assert!((value - 0.37_f32.powi(degree) as f64).abs() < 0.000001);
        }
    }
}
