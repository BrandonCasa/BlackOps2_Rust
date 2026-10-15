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

fn time_frames(motion: &[Motion], recipe: &Recipe, bounds: [[f32; 2]; 2]) -> Result<usize> {
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
    let frames = time_frames(&motion, &recipe, bounds)?;
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
            .map(|(motion, image)| motion.scale[axis].abs() * image.width.max(image.height) as f32)
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

pub(super) fn preflight(
    mode: &str,
    native: &Value,
    surfaces: &[Surface],
    images: &BTreeMap<String, texture::Image>,
) -> Result<u64> {
    let plan = plan(mode, native, surfaces, images)?;
    let texels = plan.dimensions[0] as u64 * plan.dimensions[1] as u64;
    let bytes = texels * plan.frames as u64 * plan.states.len() as u64 * 4 / 3;
    println!(
        "Prepared animation {}: {} x {} texels, {} time frames, {} source states, approximately {:.2} GiB of BC7 frames",
        native["name"],
        plan.dimensions[0],
        plan.dimensions[1],
        plan.frames,
        plan.states.len(),
        bytes as f64 / 1073741824.
    );
    Ok(bytes)
}

pub(super) fn bake(
    mode: &str,
    native: &Value,
    surfaces: &[Surface],
    images: &BTreeMap<String, texture::Image>,
    converted: &mut BTreeMap<String, Value>,
    pack: &mut Pack,
) -> Result<Vec<Surface>> {
    let Plan {
        mut recipe,
        motion,
        bounds,
        frames,
        states,
        periodic,
        dimensions,
    } = plan(mode, native, surfaces, images)?;
    let base = format!("prepared/materials/{}/script", converted.len());
    let sampler = if periodic { 20 } else { 116 };
    let mut remapped = surfaces.to_vec();
    let [width, height] = dimensions;
    let samples: Vec<gpu::Sample> = (0..if mode == "normal_script" {
        width * height
    } else {
        0
    })
        .map(|index| {
            let mut sample = [0.; 20];
            sample[0] = bounds[0][0]
                + (bounds[1][0] - bounds[0][0]) * ((index % width) as f32 + 0.5) / width as f32;
            sample[1] = bounds[0][1]
                + (bounds[1][1] - bounds[0][1]) * ((index / width) as f32 + 0.5) / height as f32;
            sample[8..12].fill(1.);
            sample
        })
        .collect();
    for surface in &mut remapped {
        if !periodic {
            for uv in &mut surface.uv {
                for axis in 0..2 {
                    uv[axis] = (uv[axis] - bounds[0][axis])
                        / (bounds[1][axis] - bounds[0][axis]).max(f32::EPSILON);
                }
            }
        }
        for color in &mut surface.colors {
            if !matches!(mode, "normal_script" | "normal_script_unlit" | "zfeather") {
                *color = [255; 4];
            } else if mode != "zfeather" {
                color[3] = 255;
            }
        }
    }
    println!(
        "Baking native script sequence {}: {frames} time frames at 20 Hz, {} source state banks, {:?} texels",
        native["name"],
        states.len(),
        dimensions
    );
    let baker = gpu::Baker::new(&recipe)?;
    let mut normals = if mode == "normal_script" {
        Some(
            baker
                .sample(&samples)?
                .into_iter()
                .map(|pixel| pixel[1])
                .collect(),
        )
    } else {
        None
    };
    drop(samples);
    let started = std::time::Instant::now();
    let mut first = None;
    let mut animation = Vec::new();
    for (bank, state) in states.iter().copied().enumerate() {
        for frame in 0..frames {
            let seconds = frame as f32 / 20.;
            let transforms: Vec<_> = motion
                .iter()
                .map(|motion| motion.transform(seconds))
                .collect();
            recipe.special_parms[1] = state;
            baker.set_script_frame(&transforms, state)?;
            let uri = format!("{base}/frames/{}", bank * frames + frame);
            let outputs = [
                baker.script_frame(width, height, bounds)?,
                normals.take().unwrap_or_default(),
                Vec::new(),
                Vec::new(),
            ];
            let value = store(native, &uri, width, height, outputs, sampler, 0, pack)?;
            let surface = &value["extras"]["surface"];
            animation.push(json!({"channels":surface["channels"],"standard":surface["standard"]}));
            if first.is_none() {
                first = Some(value);
            }
            if frame % 100 == 0 {
                println!(
                    "Baked {} state {}/{}, time frame {}/{}",
                    native["name"],
                    bank + 1,
                    states.len(),
                    frame + 1,
                    frames
                );
                println!(
                    "Sequence bake elapsed {:.1} seconds",
                    started.elapsed().as_secs_f64()
                );
            }
        }
    }
    let identity = native["name"]
        .as_str()
        .ok_or("Baked material has no name")?
        .to_ascii_lowercase();
    let mut value = first.ok_or("Baked material has no frames")?;
    let surface = &mut value["extras"]["surface"];
    surface["identity"] = json!(identity);
    if frames * states.len() > 1 {
        surface["animation"] = json!({"driver":"native_layers","fps":20.,"time_frames":frames,"states":states,"frames":animation});
    }
    surface["approximations"].as_array_mut().unwrap().push(json!("Native layer color, coverage and UV motion precomposed at 20 Hz; loop seam closes within one eighth source texel. Mapped script values select the nearest baked state. Vertex tint and view falloff use a single prepared frame texture. View parallax uses the head-on bake; shared GGX replaces native reflection probes"));
    value["name"] = json!(identity);
    converted.insert(identity, value);
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
        _ => return Err("Unknown baked effect mode".into()),
    };
    let falloff = if matches!(code, 2..=5) {
        crate::material::contract::literal(native, 3185247991)?[0]
    } else {
        0.
    };
    surface["baked_effect"] = json!({"mode":code,"falloff_power":falloff});
    let alpha_blend = matches!(mode, "single" | "blend" | "layer_blend");
    let channels = surface["channels"]
        .as_array_mut()
        .ok_or("Baked script channels are missing")?;
    let color = channels
        .iter()
        .find(|channel| channel["role"] == "color")
        .cloned()
        .ok_or("Baked script color is missing")?;
    if !alpha_blend {
        channels.retain(|channel| channel["role"] != "color");
    }
    let mut emissive = color;
    emissive["role"] = json!("emissive");
    channels.push(emissive);
    let boost = if matches!(mode, "opaque" | "blend" | "add" | "zfeather") {
        crate::material::contract::literal(native, 3173127725)?[0]
    } else {
        1.
    };
    let emission = surface["standard"]["emissive"]
        .as_array()
        .ok_or("Missing baked emission factor")?
        .iter()
        .map(|value| value.as_f64().unwrap() * f64::from(boost * boost))
        .collect::<Vec<_>>();
    surface["standard"]["emissive"] = json!(emission);
    if !matches!(mode, "single" | "normal_script_unlit") {
        let gloss = crate::material::contract::literal(native, 3432544439)?[0];
        let amount = crate::material::contract::literal(native, 211931156)?[0]
            * (0.75 + 0.25 * gloss).clamp(0., 1.);
        surface["standard"]["roughness"] = json!((1. - gloss).clamp(0., 1.));
        surface["standard"]["reflectance"] = json!((amount.max(0.) / 0.16).sqrt());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_motion_rotates_centered_uv_before_adding_scaled_scroll() {
        let motion = Motion {
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
            let frames = time_frames(&motions, &initial, [[0., 0.], [1., 1.]]).unwrap();
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
            .set_script_frame(&transforms, recipe.special_parms[1])
            .unwrap();
        let shifted = baker.sample(&samples).unwrap();
        for axis in 0..4 {
            assert!((shifted[0][0][axis] - output[1][0][axis]).abs() < 0.00001);
        }
        baker.set_script_frame(&transforms, [1.; 4]).unwrap();
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
            let prepared = baker.script_frame(2, 1, [[0., 0.], [0.5, 1.]]).unwrap();
            for axis in 0..4 {
                assert!(
                    (prepared[0][axis] - result[0][0][axis]).abs() < 0.00001,
                    "prepared frame mode {mode}, channel {axis}"
                );
            }
            if mode == 8 {
                let prepared = baker.script_frame(513, 513, [[0., 0.], [1., 1.]]).unwrap();
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
    }
}
