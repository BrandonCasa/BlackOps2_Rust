use super::special::tile_period;
use super::*;

fn model_uv_bounds(
    catalog: &Catalog<'_>,
    name: &str,
    nonnegative: bool,
) -> Result<([[f32; 2]; 2], usize)> {
    let mut bounds = [[f32::INFINITY; 2], [f32::NEG_INFINITY; 2]];
    let mut source_triangles = 0;
    for (model, zone, root) in catalog.definitions(5) {
        let streams = &catalog.zones[zone].1.streams;
        let materials = crate::fastfile::Address::decode(streams.u32(root.add(36)?)?)?;
        let mut uses = false;
        for index in 0..streams.u8(root.add(6)?)? as usize {
            if catalog
                .pointer(zone, materials.add(index * 4)?, 6)?
                .0
                .eq_ignore_ascii_case(name)
            {
                uses = true;
                break;
            }
        }
        if !uses {
            continue;
        }
        for lod in geometry::model(catalog, model, zone, root)? {
            for surface in lod
                .iter()
                .filter(|surface| surface.material.eq_ignore_ascii_case(name))
            {
                if nonnegative && surface.uv.iter().flatten().any(|value| *value < 0.) {
                    continue;
                }
                source_triangles += surface.indices.len() / 3;
                for uv in &surface.uv {
                    for axis in 0..2 {
                        bounds[0][axis] = bounds[0][axis].min(uv[axis]);
                        bounds[1][axis] = bounds[1][axis].max(uv[axis]);
                    }
                }
            }
        }
    }
    if source_triangles == 0 {
        return Err(format!("Nonperiodic representative {name} model geometry is missing in the requested source UV domain").into());
    }
    Ok((bounds, source_triangles))
}

pub fn check_special_response(
    catalog: &Catalog<'_>,
    selected: &BTreeMap<String, Value>,
    images: &BTreeMap<String, texture::Image>,
    pack: &mut Pack,
) -> Result<Vec<String>> {
    let native_lookups: std::collections::BTreeSet<_> = selected
        .values()
        .filter(|native| {
            native["technique_set"] == "mc_sw4_3d_phong_simple_emissive_flicker_9j5f25q3"
        })
        .map(|native| {
            native["textures"]
                .as_array()
                .unwrap()
                .iter()
                .find(|binding| binding["hash"] == 23340576_u32)
                .unwrap()["image"]
                .as_str()
                .unwrap()
                .to_ascii_lowercase()
        })
        .collect();
    let images: BTreeMap<_, _> = images
        .iter()
        .map(|(name, image)| {
            let level = if native_lookups.contains(name) {
                0
            } else {
                (0..image.levels.len())
                    .find(|&level| {
                        (image.width >> level).max(1) <= 64 && (image.height >> level).max(1) <= 64
                    })
                    .unwrap_or(image.levels.len() - 1)
            };
            (
                name.clone(),
                texture::Image {
                    width: (image.width >> level).max(1),
                    height: (image.height >> level).max(1),
                    depth: (image.depth >> level).max(1),
                    faces: image.faces,
                    format: image.format,
                    flags: image.flags,
                    levels: image.levels[level..].to_vec(),
                },
            )
        })
        .collect();
    let mut shared = BTreeMap::new();
    let mut converted = BTreeMap::new();
    let mut maximum = 0_f32;
    let mut fractional = 0;
    for (name, native) in selected {
        validate_native(native)?;
        let uri = format!("prepared/special-check/{}/material", converted.len());
        let value = prepare_special(native, &images, &uri, None, pack, &mut shared)?;
        for input in value["extras"]["surface"]["source_inputs"]
            .as_object()
            .ok_or("Special source inputs were not retained")?
            .values()
        {
            texture::validate(
                pack.get(
                    input["uri"]
                        .as_str()
                        .ok_or("Special source input URI is missing")?,
                )?,
            )?;
        }
        if native["technique_set"] == "mc_sw4_3d_water_on_surface_reveal_77320z68" {
            if value["extras"]["surface"]["pending_runtime"].is_null() {
                return Err("Unbound water was silently assigned an initial state".into());
            }
            let (all_bounds, all_triangles) = model_uv_bounds(catalog, name, false)?;
            let (bounds, source_triangles) = if all_bounds.iter().flatten().any(|value| *value < 0.)
            {
                let mut bound = native.clone();
                bound["code_constants"] = json!({"172":[0.,0.,0.5,0.],"25":[0.,0.,0.,1.25]});
                let error = prepare_special(
                    &bound,
                    &images,
                    &format!("{uri}/negative-domain"),
                    Some(all_bounds),
                    pack,
                    &mut shared,
                )
                .unwrap_err();
                if !error.to_string().contains("native log2") {
                    return Err(format!("Water {name} negative source domain failed for an unrelated reason: {error}").into());
                }
                println!(
                    "Water {name} retains an unsupported negative-UV source log2 domain across {all_triangles} source triangles/all LODs: {all_bounds:?}"
                );
                model_uv_bounds(catalog, name, true)?
            } else {
                (all_bounds, all_triangles)
            };
            for (index, (state, time)) in
                [(0.2, 0.), (0.5, 1.25), (0.8, 3.)].into_iter().enumerate()
            {
                let mut bound = native.clone();
                bound["code_constants"] = json!({"172":[0.,0.,state,0.],"25":[0.,0.,0.,time]});
                let value = prepare_special(
                    &bound,
                    &images,
                    &format!("{uri}/state{index}"),
                    Some(bounds),
                    pack,
                    &mut shared,
                )?;
                if value["extras"]["surface"]["channels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|channel| channel["role"] == "emissive")
                    .count()
                    != 1
                {
                    return Err("Bound water leading-edge emission is missing".into());
                }
                if value["extras"]["surface"]["standard"]["emissive"] != json!([1., 1., 1.]) {
                    return Err(format!(
                        "Water {name} emission texture has no neutral StandardMaterial multiplier"
                    )
                    .into());
                }
                let recipe = recipe(&bound, Some(&images))?;
                let baker = gpu::Baker::new(&recipe)?;
                let samples: Vec<gpu::Sample> = (0..33)
                    .map(|i| {
                        let mut sample = [0.; 20];
                        sample[0] = 0.125 + (i % 7) as f32 / 8.;
                        sample[1] = 0.125 + (i % 5) as f32 / 6.;
                        sample
                    })
                    .collect();
                for (sample, pixel) in samples.iter().zip(baker.sample(&samples)?) {
                    let expected = recipe.sample_special([sample[0], sample[1]])?;
                    for (actual, expected) in pixel
                        .into_iter()
                        .flatten()
                        .zip(expected.into_iter().flatten())
                    {
                        let error = (actual - expected).abs();
                        maximum = maximum.max(error);
                        if error > 0.0001 {
                            return Err(format!("Bound water CPU/GPU mismatch: {error}").into());
                        }
                    }
                }
                for channel in value["extras"]["surface"]["channels"].as_array().unwrap() {
                    texture::validate(pack.get(channel["uri"].as_str().unwrap())?)?;
                }
            }
            println!(
                "Checked water effective-input states against {source_triangles} source model triangles with nonnegative UV, bounds {bounds:?}; full geometry and runtime gates retained"
            );
        } else if emissive_family(native["technique_set"].as_str().unwrap()) {
            let kind = recipe(native, None)?.special;
            let controlled = matches!(kind, 16 | 32 | 64);
            let bounds = if kind == 16 {
                let (bounds, triangles) = model_uv_bounds(catalog, name, false)?;
                println!(
                    "Checked Fritz nonperiodic UV domain against {triangles} source triangles/all LODs: {bounds:?}"
                );
                Some(bounds)
            } else {
                None
            };
            let variants = if kind == 16 {
                [
                    (-0.25, 1.),
                    (0., 0.),
                    (0., 1.),
                    (0.0001, 1.),
                    (0.5, 0.5),
                    (1., 1.),
                    (1.25, 1.),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (warp, amount))| {
                    let mut bound = native.clone();
                    bound["code_constants"] =
                        json!({"169":[warp,amount,0.,0.],"25":[0.,0.,0.,index as f32*1.25]});
                    bound
                })
                .collect()
            } else if kind == 32 {
                let threshold = native["constants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|constant| constant["hash"] == 2158510423_u32)
                    .unwrap()["literal"][0]
                    .as_f64()
                    .unwrap() as f32;
                [
                    f32::from_bits(threshold.to_bits() - 1),
                    threshold,
                    f32::from_bits(threshold.to_bits() + 1),
                ]
                .into_iter()
                .map(|state| {
                    let mut bound = native.clone();
                    bound["code_constants"] = json!({"169":[0.,0.,state,0.]});
                    bound
                })
                .collect()
            } else if kind == 64 {
                [[0.; 4], [1.; 4], [2., 0.5, 1., 0.25], [-1., 1., -0.5, 1.]]
                    .into_iter()
                    .map(|rgba| {
                        let mut bound = native.clone();
                        bound["code_constants"] = json!({"172":rgba});
                        bound
                    })
                    .collect()
            } else {
                vec![native.clone()]
            };
            if controlled && value["extras"]["surface"]["pending_runtime"].is_null() {
                return Err("Scripted emission received an invented initial state".into());
            }
            for (index, bound) in variants.into_iter().enumerate() {
                let state = if controlled {
                    prepare_special(
                        &bound,
                        &images,
                        &format!("{uri}/state{index}"),
                        bounds,
                        pack,
                        &mut shared,
                    )?
                } else {
                    value.clone()
                };
                let surface = &state["extras"]["surface"];
                if surface["channels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|channel| channel["role"] == "emissive")
                    .count()
                    != 1
                {
                    return Err("Native phong/flicker emission channel is missing".into());
                }
                if surface["standard"]["emissive"] != json!([1., 1., 1.]) {
                    return Err(format!("Phong/flicker {name} emission texture has no neutral StandardMaterial multiplier").into());
                }
                let recipe = recipe(&bound, Some(&images))?;
                let baker = gpu::Baker::new(&recipe)?;
                let samples: Vec<gpu::Sample> = (0..33)
                    .map(|i| {
                        let mut sample = [0.; 20];
                        sample[0] = (i % 7) as f32 / 4. - 0.25;
                        sample[1] = (i % 5) as f32 / 3. - 0.25;
                        sample
                    })
                    .collect();
                for (sample, pixel) in samples.iter().zip(baker.sample(&samples)?) {
                    let expected = recipe.sample_special([sample[0], sample[1]])?;
                    for (actual, expected) in pixel
                        .into_iter()
                        .flatten()
                        .zip(expected.into_iter().flatten())
                    {
                        let error = (actual - expected).abs();
                        maximum = maximum.max(error);
                        if error > 0.0001 {
                            return Err(format!("Phong/flicker CPU/GPU mismatch: {error}").into());
                        }
                    }
                }
                for channel in surface["channels"].as_array().unwrap() {
                    texture::validate(pack.get(channel["uri"].as_str().unwrap())?)?;
                }
                texture::validate(
                    pack.get(
                        surface["native_specular"]["uri"]
                            .as_str()
                            .ok_or("Phong effective specular/gloss is missing")?,
                    )?,
                )?;
                if native["technique_set"]
                    .as_str()
                    .unwrap()
                    .starts_with("mlv_")
                    && (surface["source_vertex_lighting"]["source"] != 5
                        || surface["pending_runtime"].is_null())
                {
                    return Err("Neutral phong lighting amounts bypassed required secondary vertex lighting".into());
                }
                if recipe.special == 8 {
                    let animation = &surface["scalar_animation"];
                    if !surface["pending_runtime"].is_null()
                        || animation["time_mode"] != "native_elapsed"
                        || animation["speed"] != json!(recipe.parms[1])
                        || animation["minimum"] != json!(recipe.parms[2])
                        || animation["maximum"] != json!(recipe.parms[3])
                        || animation["script_vector"] != 1
                        || animation["script_component"] != 0
                        || animation["sampler"] != recipe.inputs[3].sampler
                        || surface["source_flicker"]["script_binding"]["normal_view_base"]
                            != json!([0., 0., 0., 0.])
                    {
                        return Err(format!("Native flicker {name} live source clock/script binding is missing or changed").into());
                    }
                    let lookup: Vec<f32> = serde_json::from_value(animation["lookup"].clone())?;
                    if lookup.len() != recipe.inputs[3].height {
                        return Err(format!(
                            "Native flicker {name} lookup reduced the source row count"
                        )
                        .into());
                    }
                    let pixels: Vec<_> = lookup.into_iter().map(|value| [value; 4]).collect();
                    let period = if recipe.parms[1] == 0. {
                        1.
                    } else {
                        1. / recipe.parms[1]
                    };
                    for time in [
                        -1.,
                        0.,
                        1.25,
                        period - 0.0001,
                        period,
                        period + 0.0001,
                        period * 3. + 1.25,
                    ] {
                        let coordinate = time * recipe.parms[1];
                        let original =
                            recipe.inputs[3].sample([recipe.second_parms[0], coordinate])[0];
                        let packed = texture::sample(
                            &pixels,
                            1,
                            pixels.len(),
                            [0.5, coordinate],
                            recipe.inputs[3].sampler,
                        )[0];
                        if (original - packed).abs() > 0.00001 {
                            return Err(format!("Native flicker {name} fixed-column source sampling changed at elapsed {time}").into());
                        }
                        for script in [-0.5_f32, 0., 0.5, 1., 1.5] {
                            let factor = (recipe.parms[2]
                                + (recipe.parms[3] - recipe.parms[2]) * packed)
                                * (1. - script).clamp(0., 1.);
                            let source = (recipe.parms[2]
                                + (recipe.parms[3] - recipe.parms[2]) * original)
                                * (1. - script).clamp(0., 1.)
                                * recipe.parms[0];
                            for uv in [[0.125, 0.375], [0.625, 0.875]] {
                                let base =
                                    recipe.emission_pixel(uv, recipe.parms[0] * recipe.parms[0]);
                                let expected = recipe.emission_pixel(uv, source * source);
                                for channel in 0..3 {
                                    let actual = base[channel] * (factor * factor);
                                    if (actual - expected[channel]).abs()
                                        > 0.00003 * expected[channel].abs().max(1.)
                                    {
                                        return Err(format!("Native flicker {name} static HDR and live factor differ from source emission").into());
                                    }
                                    if script >= 1. && actual != 0. {
                                        return Err(format!("Native flicker {name} scripted off state retains emission").into());
                                    }
                                }
                            }
                        }
                    }
                }
                if recipe.special == 32 {
                    let expected = if index == 0 {
                        0.
                    } else {
                        recipe.parms[3] * recipe.parms[3]
                    };
                    if recipe.sample_special([0.375, 0.625])?[2][3] != expected {
                        return Err(
                            "Scripted emission threshold is not inclusive at the authored value"
                                .into(),
                        );
                    }
                }
                if controlled && surface["pending_runtime"].is_null() {
                    return Err(
                        "Explicit scripted state bypassed its live source-input runtime gate"
                            .into(),
                    );
                }
                if matches!(recipe.special, 16 | 32)
                    && native["state_bits"][native["state_entries"][4].as_u64().unwrap() as usize]
                        [0]
                        == 403869714_u32
                    && (surface["presentation"]["alpha"] != "mask"
                        || surface["presentation"]["cutoff"] != json!(128_f32 / 255.)
                        || surface["source_framebuffer_alpha"].is_null())
                {
                    return Err(
                        "Native scripted mask lost its coverage/cutoff/framebuffer semantics"
                            .into(),
                    );
                }
            }
        } else {
            let recipe = recipe(native, Some(&images))?;
            let period = tile_period(&recipe)?;
            if period != [1, 1] {
                fractional += 1;
            }
            if value["extras"]["surface"]["standard"]["uv_transform"]
                != json!([1. / period[0] as f32, 0., 0., 1. / period[1] as f32, 0., 0.])
            {
                return Err("Tile repeat period is not bound to the prepared UV transform".into());
            }
            let baker = gpu::Baker::new(&recipe)?;
            let coordinates = [
                [0.125, 0.375],
                [period[0] as f32 + 0.125, period[1] as f32 + 0.375],
                [period[0] as f32 - 1. / 1024., period[1] as f32 - 1. / 1024.],
                [period[0] as f32 + 1. / 1024., period[1] as f32 + 1. / 1024.],
            ];
            let samples: Vec<gpu::Sample> = coordinates
                .into_iter()
                .map(|uv| {
                    let mut sample = [0.; 20];
                    sample[..2].copy_from_slice(&uv);
                    sample
                })
                .collect();
            let pixels = baker.sample(&samples)?;
            for (sample, pixel) in samples.iter().zip(&pixels) {
                let expected = recipe.sample_special([sample[0], sample[1]])?;
                for (&actual, expected) in
                    pixel.iter().flatten().zip(expected.into_iter().flatten())
                {
                    let error = (actual - expected).abs();
                    maximum = maximum.max(error);
                    if error > 0.00003 {
                        return Err(format!("Tile CPU/GPU mismatch: {error}").into());
                    }
                }
            }
            if pixels[0]
                .iter()
                .flatten()
                .zip(pixels[1].iter().flatten())
                .any(|(a, b)| (a - b).abs() > 0.00003)
            {
                return Err("Authored tile sampling fails its derived repeat-period seam".into());
            }
            for channel in value["extras"]["surface"]["channels"].as_array().unwrap() {
                texture::validate(pack.get(channel["uri"].as_str().unwrap())?)?;
            }
            if recipe.base_specular {
                texture::validate(
                    pack.get(
                        value["extras"]["surface"]["native_specular"]["uri"]
                            .as_str()
                            .ok_or("Tile native response is missing")?,
                    )?,
                )?;
            }
            if native["technique_set"]
                .as_str()
                .unwrap()
                .starts_with("mlv_")
            {
                let mut neutral = native.clone();
                for constant in neutral["constants"].as_array_mut().unwrap() {
                    if constant["hash"] == 2569221807_u32 || constant["hash"] == 211931156_u32 {
                        constant["literal"][0] = json!(1.);
                    }
                }
                let neutral = prepare_special(
                    &neutral,
                    &images,
                    &format!("{uri}/neutral-lighting"),
                    None,
                    pack,
                    &mut shared,
                )?;
                let surface = &neutral["extras"]["surface"];
                if surface["source_vertex_lighting"]["source"] != 5
                    || surface["source_vertex_lighting"]["alpha"] != "sun_shadow_fallback"
                    || surface["pending_runtime"]
                        .as_str()
                        .is_none_or(|reason| !reason.contains("secondary RGB"))
                {
                    return Err("Neutral tile occlusion/reflection incorrectly bypassed required secondary vertex lighting".into());
                }
            }
        }
        converted.insert(name.clone(), value);
    }
    if fractional == 0 {
        return Err("Special representative batch omitted the fractional tile-period edge".into());
    }
    println!(
        "Checked {} special representatives at existing source mips <=64px, full native uniform flicker lookups, {fractional} fractional-repeat edges and three explicitly bound water states; maximum CPU/GPU error {maximum}; {} raw input encodings shared",
        selected.len(),
        shared.len()
    );
    Ok(crate::material::missing(selected, &converted))
}

pub fn check_response_geometry(
    catalog: &Catalog<'_>,
    materials: &BTreeMap<String, Value>,
    images: &BTreeMap<String, texture::Image>,
    pack: &mut Pack,
) -> Result<Vec<String>> {
    let recipes = materials
        .iter()
        .map(|(name, native)| Ok((name.clone(), recipe(native, Some(images))?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let mut selected = BTreeMap::<String, (usize, Surface)>::new();
    let mut triangles = 0;
    let mut vertices = 0;
    for (_, zone, root) in catalog.definitions(17) {
        for surface in geometry::world(catalog, zone, root)? {
            let key = surface.material.to_ascii_lowercase();
            let Some(recipe) = recipes.get(&key) else {
                continue;
            };
            let surface = surface.unindex()?;
            for index in 0..surface.positions.len() {
                recipe.auxiliary(&surface, index)?;
                vertices += 1;
            }
            triangles += surface.indices.len() / 3;
            for triangle in 0..surface.indices.len() / 3 {
                let first = triangle * 3;
                let spread = (1..3)
                    .map(|i| {
                        surface.colors[first + i][1].abs_diff(surface.colors[first][1]) as usize
                            + surface.colors[first + i][2].abs_diff(surface.colors[first][2])
                                as usize
                    })
                    .sum::<usize>();
                if selected.get(&key).is_none_or(|&(score, _)| spread > score) {
                    selected.insert(
                        key.clone(),
                        (spread, surface.select_triangles(&[triangle])?),
                    );
                }
            }
        }
    }
    if selected.len() != materials.len() {
        return Err("Representative layered geometry is missing".into());
    }
    let mut converted = BTreeMap::new();
    let mut surfaces = Vec::new();
    for (name, (_, surface)) in selected {
        let recipe = &recipes[&name];
        let baker = gpu::Baker::new(recipe)?;
        let uri = format!("prepared/materials/{}/representative", converted.len());
        surfaces.extend(layered(
            &materials[&name],
            &[surface],
            recipe,
            &baker,
            &uri,
            &mut converted,
            pack,
        )?);
    }
    let mut scene = crate::gltf::Scene::new("layered-response");
    let indices = scene.materials(surfaces.iter(), &converted)?;
    let mesh = scene.mesh("layered-response", surfaces.iter(), &indices, false)?;
    scene.node(json!({"name":"layered-response","mesh":mesh}));
    scene.write("prepared/material.gltf", pack)?;
    for material in converted.values() {
        for channel in material["extras"]["surface"]["channels"]
            .as_array()
            .unwrap()
        {
            texture::validate(pack.get(channel["uri"].as_str().unwrap())?)?;
        }
        texture::validate(
            pack.get(
                material["extras"]["surface"]["native_specular"]["uri"]
                    .as_str()
                    .ok_or("Native specular channel is missing")?,
            )?,
        )?;
    }
    println!(
        "Validated {vertices} representative-material source vertices and {triangles} triangles; baked/exported {} source triangles with KTX2 and native-specular bindings",
        surfaces
            .iter()
            .map(|surface| surface.indices.len() / 3)
            .sum::<usize>()
    );
    Ok(crate::material::missing(materials, &converted))
}
