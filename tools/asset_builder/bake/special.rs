use super::*;

pub(super) fn tile_period(recipe: &Recipe) -> Result<[usize; 2]> {
    let mut period = [1; 2];
    for axis in 0..2 {
        period[axis] = (1..=16)
            .find(|&period| {
                [recipe.parms[axis], recipe.parms[axis + 2]]
                    .into_iter()
                    .all(|scale| {
                        let scaled = scale as f64 * period as f64;
                        scaled.is_finite() && scaled.fract() == 0.
                    })
            })
            .ok_or(
                "Tile surface has no bounded exact repeat period for its authored micro scales",
            )?;
    }
    Ok(period)
}

pub fn prepare_special(
    native: &Value,
    images: &BTreeMap<String, texture::Image>,
    uri: &str,
    bounds: Option<[[f32; 2]; 2]>,
    pack: &mut Pack,
    shared: &mut BTreeMap<String, String>,
) -> Result<Value> {
    let reviewed = recipe(native, None)?;
    let mut sources = BTreeMap::new();
    let active = &native["techniques"]
        .as_array()
        .unwrap()
        .iter()
        .find(|technique| technique["index"] == 4)
        .unwrap()["passes"][0]["arguments"];
    for binding in native["textures"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|binding| {
            active
                .as_array()
                .unwrap()
                .iter()
                .any(|arg| arg["type"] == 2 && arg["value"] == binding["hash"])
        })
    {
        let name = binding["image"]
            .as_str()
            .ok_or("Special surface image name is missing")?
            .to_ascii_lowercase();
        let path = if let Some(path) = shared.get(&name) {
            path.clone()
        } else {
            let image = images
                .get(&name)
                .ok_or_else(|| format!("Special surface image {name} is missing"))?;
            let path = format!("prepared/material-inputs/{}/rgba.ktx2", shared.len());
            let encoded = texture::encode(image, false, "reflectance")?;
            texture::validate(&encoded)?;
            if pack
                .entries
                .insert(path.clone(), Arc::new(encoded))
                .is_some()
            {
                return Err("Special surface input URI already exists".into());
            }
            shared.insert(name.clone(), path.clone());
            path
        };
        sources.insert(
            binding["hash"].as_u64().unwrap().to_string(),
            json!({"uri":path,"sampler":binding["sampler"],"transfer":"source_encoded_rgba"}),
        );
    }
    let required: &[&str] = match reviewed.special {
        2 => &["172", "25"],
        16 => &["169", "25"],
        32 => &["169"],
        64 => &["172"],
        _ => &[],
    };
    if required
        .iter()
        .any(|key| native["code_constants"][*key].is_null())
    {
        let reason = match reviewed.special {
            2 => {
                "Water reveal requires native per-object scriptVector3 and gameTime inputs, wet-normal/refraction sampling, leading-edge emission, separate reflection amount and the linear model color transform; no initial state is invented"
            }
            16 => {
                "Native Fritz material requires per-object scriptVector0.x/y and gameTime.w for source warp, luma, UV-height sparkle, specular and glow evaluation; no initial state is invented"
            }
            32 => {
                "Native threshold emission requires per-object scriptVector0.z and its inclusive authored threshold; no initial state is invented"
            }
            _ => {
                "Native tomb emission requires per-object scriptVector3 RGBA modulation before source RGB-alpha emission multiplication; no initial state is invented"
            }
        };
        return Ok(
            json!({"name":native["name"],"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
            "extras":{"surface":{"version":3,"identity":native["name"],"native":native,"channels":[],"source_inputs":sources,
                "source_effective_inputs":required.iter().map(|key| key.parse::<u32>().unwrap()).collect::<Vec<_>>(),"pending_runtime":reason}}}),
        );
    }
    let recipe = recipe(native, Some(images))?;
    let (minimum, extent, sampler) = if recipe.special == 1 {
        let period = tile_period(&recipe)?;
        ([0.; 2], period.map(|value| value as f32), 20)
    } else if matches!(recipe.special, 2 | 16) {
        let bounds = bounds.ok_or_else(|| {
            format!(
                "Bound nonperiodic surface {} requires source-derived UV bounds",
                native["name"]
            )
        })?;
        if bounds
            .iter()
            .flatten()
            .any(|value| !value.is_finite() || recipe.special == 2 && *value < 0.)
            || (0..2).any(|axis| bounds[1][axis] <= bounds[0][axis])
        {
            return Err(format!("Bound nonperiodic surface {} UV domain {bounds:?} is outside the recovered source domain{}",native["name"],if recipe.special == 2 {": native log2 receives source UV directly and is undefined for negative coordinates"} else {""}).into());
        }
        (
            bounds[0],
            std::array::from_fn(|axis| bounds[1][axis] - bounds[0][axis]),
            116,
        )
    } else {
        ([0.; 2], [1.; 2], 20)
    };
    let dimensions: [usize; 2] = std::array::from_fn(|axis| {
        recipe
            .inputs
            .iter()
            .enumerate()
            .filter(|(index, _)| recipe.special != 8 || *index != 3)
            .map(|(index, input)| {
                let scale = if recipe.special == 1 {
                    match index {
                        2 | 4 | 6 => recipe.parms[axis].abs(),
                        3 | 5 | 7 => recipe.parms[axis + 2].abs(),
                        _ => 1.,
                    }
                } else if recipe.special == 16 {
                    match index {
                        4 => recipe.second_parms[3].abs(),
                        5 => recipe.parms[3].abs(),
                        _ => 1.,
                    }
                } else {
                    1.
                };
                ((if axis == 0 { input.width } else { input.height }) as f64
                    * scale as f64
                    * extent[axis] as f64)
                    .ceil() as usize
            })
            .max()
            .unwrap_or(0)
            .max(1)
    });
    if dimensions.into_iter().any(|size| size > 16_384) {
        return Err(
            "Special surface bake exceeds supported texture dimensions at source density".into(),
        );
    }
    let [width, height] = dimensions;
    let count = width
        .checked_mul(height)
        .ok_or("Special surface dimensions overflow")?;
    let baker = gpu::Baker::new(&recipe)?;
    let mut outputs: [Vec<[f32; 4]>; 4] = std::array::from_fn(|_| Vec::with_capacity(count));
    for start in (0..count).step_by(262_144) {
        let samples: Vec<gpu::Sample> = (start..(start + 262_144).min(count))
            .map(|index| {
                let uv = [
                    minimum[0] + ((index % width) as f32 + 0.5) / width as f32 * extent[0],
                    minimum[1] + ((index / width) as f32 + 0.5) / height as f32 * extent[1],
                ];
                let mut sample = [0.; 20];
                sample[..2].copy_from_slice(&uv);
                sample
            })
            .collect();
        for pixel in baker.sample(&samples)? {
            for (output, pixel) in outputs.iter_mut().zip(pixel) {
                output.push(pixel);
            }
        }
    }
    let emission = if recipe.special != 1 {
        let mut emission = vec![[0.; 4]; count];
        parallel_chunks_mut(&mut emission, 4096, |start, pixels| {
            for (offset, pixel) in pixels.iter_mut().enumerate() {
                let index = start + offset;
                let uv = [
                    minimum[0] + ((index % width) as f32 + 0.5) / width as f32 * extent[0],
                    minimum[1] + ((index / width) as f32 + 0.5) / height as f32 * extent[1],
                ];
                *pixel = recipe.emission_pixel(uv, outputs[2][index][3]);
            }
            Ok(())
        })?;
        Some(emission)
    } else {
        None
    };
    if recipe.special != 1 {
        for response in &mut outputs[2] {
            response[3] = 1.;
        }
    }
    let mut value = store(native, uri, width, height, outputs, sampler, 0, pack)?;
    let surface = &mut value["extras"]["surface"];
    surface["standard"]["metallic"] = json!(if recipe.base_specular { 1. } else { 0. });
    surface["standard"]["reflectance"] = json!(if recipe.base_specular { 0.5 } else { 0. });
    surface["standard"]["uv_transform"] = json!([
        1. / extent[0],
        0.,
        0.,
        1. / extent[1],
        -minimum[0] / extent[0],
        -minimum[1] / extent[1]
    ]);
    surface["source_vertex_color_transfer"] = json!("gamma2");
    surface["source_inputs"] = json!(sources);
    surface["bake"]["source_uv_min"] = json!(minimum);
    surface["bake"]["source_uv_extent"] = json!(extent);
    if matches!(recipe.special, 2 | 16) {
        surface["bake"]["layout"] = json!("bounded_source_uv_domain");
        surface["approximations"]
            .as_array_mut()
            .unwrap()
            .retain(|value| {
                !value
                    .as_str()
                    .is_some_and(|value| value.starts_with("Coarse atlas mip"))
            });
        surface["approximations"].as_array_mut().unwrap().push(json!("Nonperiodic source equations are sampled over supplied geometry UV bounds with clamped output; finite texel, edge and baked mip filtering remain approximations"));
    }
    surface["approximations"].as_array_mut().unwrap().push(json!("Vertex gamma-2 transfer is evaluated before interpolation; the native shader squares interpolated primary RGB. These agree for constant per-triangle RGB; varying tint requires geometry-dependent evaluation"));
    if recipe.special == 1 {
        surface["bake"]["layout"] = json!("exact_uv_repeat_period");
        surface["source_macro_occlusion"] = json!({"source_hash":3976875907_u32,"operation":"saturate(red*red+adjust)","adjust":recipe.second_parms[2],
            "consumers":if recipe.base_specular {json!(["indirect_diffuse","direct_specular","reflection_specular"])} else {json!(["indirect_diffuse"])}});
        surface["source_reflection_amount"] = json!(recipe.special_parms[0][0]);
        if recipe.second_parms[2] < 1. || recipe.base_specular && recipe.special_parms[0][0] != 1. {
            surface["pending_runtime"] = json!(
                "Native tile macro occlusion affects indirect diffuse and, in specular variants, direct and reflection specular separately; reflection_amount scales only reflection specular. Distinct runtime lighting consumers remain unconnected"
            );
        }
        surface["approximations"].as_array_mut().unwrap().push(json!("All active source maps repeat on the derived integer UV period, including fractional authored micro scales. Source bilinear texels are evaluated before finite baked mip filtering; vertex tint is retained separately with gamma-2 transfer"));
    } else {
        surface["standard"]["emissive"] = json!([1., 1., 1.]);
        let path = format!("{uri}/emissive.ktx2");
        let image = mip_chain(width, height, emission.unwrap(), "color")?;
        let srgb = image.format != 19;
        let encoded = texture::encode(&image, srgb, "emissive")?;
        texture::validate(&encoded)?;
        pack.entries.insert(path.clone(), Arc::new(encoded));
        surface["channels"].as_array_mut().unwrap().push(json!({"uri":path,"role":"emissive","layer":0,"color_space":if srgb {"srgb"} else {"linear"},"sampler":sampler}));
        if recipe.special == 2 {
            surface["source_effective_inputs"] = native["code_constants"].clone();
            surface["source_reflection_amount"] = json!(recipe.special_parms[0][3]);
            surface["source_model_color_transform"] = json!({"buffer":0,"rows":[114,115,116],"operation":"linear_diffuse_plus_specular_plus_emission_before_fog"});
            surface["pending_runtime"] = json!(
                "Water reveal prepared for explicitly bound script/time inputs; live per-object updates, wet UV sampling, separate reflection amount and the native linear model color transform remain unconnected"
            );
        } else if recipe.special == 8 {
            let source = &recipe.inputs[3];
            let lookup: Vec<f32> = (0..source.height)
                .map(|row| {
                    source.sample([
                        recipe.second_parms[0],
                        (row as f32 + 0.5) / source.height as f32,
                    ])[0]
                })
                .collect();
            if lookup.iter().any(|value| !value.is_finite()) {
                return Err("Native flicker lookup contains non-finite source texels".into());
            }
            surface["scalar_animation"] = json!({"time_mode":"native_elapsed","lookup":lookup,"sampler":source.sampler,
                "speed":recipe.parms[1],"minimum":recipe.parms[2],"maximum":recipe.parms[3],"script_vector":1,"script_component":0});
            surface["source_effective_inputs"] = json!([170, 25]);
            surface["source_flicker"] = json!({"script_constant":170,"component":0,"time_constant":25,"time_component":3,
                "lookup_hash":23340576,"lookup_column":recipe.second_parms[0],"lookup_time":"game_time_w_times_authored_speed",
                "lookup_rows":source.height,"lookup_transfer":"source_encoded_red","lookup_filter":"native_fixed_column_bilinear_then_vertical_sampler",
                "range":[recipe.parms[2],recipe.parms[3]],"hdr_amount":recipe.parms[0],"hdr_binding":"squared_in_static_emissive_texture",
                "script_binding":{"normal_view_base":[0.,0.,0.,0.],"override":"active_object_mapped_script_vector","restore":"normal_view_base"},
                "time_binding":{"source":"render_client_milliseconds","seconds_multiplier_bits":0x3a83126f_u32,"shader_start":"source_script_model_zero"},
                "operation":"lookup_range_times_saturate_one_minus_script_times_hdr_before_emission_square"});
        } else if matches!(recipe.special, 16 | 32 | 64) {
            surface["source_effective_inputs"] = native["code_constants"].clone();
            if recipe.special != 16 {
                surface["source_specular_parameters"] = json!({"reflection_amount":recipe.second_parms[0],"direct_specular_amount":recipe.second_parms[1]});
                if recipe.special == 64 {
                    surface["source_specular_parameters"]["specular_push"] = json!(recipe.parms[0]);
                    surface["source_specular_parameters"]["gloss_push"] = json!(recipe.parms[1]);
                }
            }
            let masked = if recipe.special == 16 {
                recipe.special_parms[0][1] != 0.
            } else {
                recipe.special == 32 && recipe.second_parms[3] != 0.
            };
            if recipe.special == 16 {
                surface["source_fritz"] = json!({"script_constant":169,"warp_component":0,"emission_component":1,"time_constant":25,"time_component":3,
                    "warp_scale":recipe.second_parms[3],"warp_amount":recipe.second_parms[2],"sparkle_scale":recipe.parms[3],"sparkle_speed":[recipe.second_parms[0],recipe.second_parms[1]],
                    "luma_boost":recipe.special_parms[0][0],"luma_weights":[0.3,0.59,0.11],"sparkle_vertical":"uv0.y_cubed_times_sparkle_alpha",
                    "glow_mix_base":"unwarped_color_rgb","glow_alpha":"warped_glow_alpha_when_script_x_positive","specular":"warped_luma_mix_preserving_original_gloss",
                    "reflection_amount":recipe.parms[0],"direct_specular_amount":recipe.parms[1],"hdr_amount":recipe.parms[2]});
                surface["pending_runtime"] = json!(
                    "Native Fritz surface prepared for explicit scriptVector0.x/y and gameTime.w; source scoped parameter generation and live nonperiodic warp/sparkle material updates remain unconnected"
                );
            } else if recipe.special == 32 {
                surface["source_emission_threshold"] = json!({"script_constant":169,"component":2,"threshold":recipe.second_parms[2],"comparison":"greater_than_or_equal","hdr_amount":recipe.parms[3]});
                surface["pending_runtime"] = json!(
                    "Native threshold emission prepared for explicit scriptVector0.z using its inclusive authored threshold; source scoped parameter generation and live emission updates remain unconnected"
                );
            } else {
                surface["source_emission_modulation"] = json!({"script_constant":172,"channels":"rgba","operation":"multiply_glow_rgba_then_rgb_times_alpha_times_hdr_before_square","hdr_amount":recipe.parms[3]});
                surface["pending_runtime"] = json!(
                    "Native tomb emission prepared for explicit scriptVector3 RGBA; source scoped parameter generation and live per-channel emission updates remain unconnected"
                );
            }
            if masked {
                surface["presentation"]["alpha"] = json!("mask");
                surface["presentation"]["cutoff"] = json!(128_f32 / 255.);
                surface["source_framebuffer_alpha"] = json!("texture_vertex_coverage_after_cutoff");
                let reason = surface["pending_runtime"].as_str().unwrap().to_owned();
                surface["pending_runtime"] = json!(format!(
                    "{reason}; the native masked pass also retains framebuffer coverage after its 128/255 cutoff"
                ));
            }
        } else {
            surface["source_emissive"] = json!({"map_hash":1825069840,"push":recipe.parms[2],"hdr_amount":recipe.parms[3],"operation":"square_rgb_times_alpha_times_push_squared_times_hdr"});
            surface["source_specular_parameters"] = json!({"specular_push":recipe.parms[0],"gloss_push":recipe.parms[1],"reflection_amount":recipe.second_parms[0],"direct_specular_amount":recipe.second_parms[1]});
            if recipe.second_parms[..2].iter().any(|value| *value != 1.) {
                surface["pending_runtime"] = json!(
                    "Native phong emission is baked, but ReflectionAmount and SpecularAmount scale reflection and direct specular separately; distinct runtime lighting consumers remain unconnected"
                );
            }
            if recipe.special_parms[0][0] != 0. {
                surface["presentation"]["alpha"] = json!("premultiplied");
                surface["source_fragment_alpha"] = json!("sqrt_texture_vertex_coverage");
                surface["pending_runtime"] = json!(
                    "Native translucent phong emission uses sqrt texture/vertex framebuffer alpha with coverage-independent specular/emission in gamma2 premultiplied composition; matching runtime composition remains unconnected"
                );
            }
        }
    }
    if native["technique_set"]
        .as_str()
        .is_some_and(|family| family.starts_with("mlv_"))
    {
        surface["source_vertex_lighting"] = json!({"source":5,"destination":6,"encoded_rgb_scale":32.,"transfer":"gamma2_before_vertex_interpolation",
            "diffuse":{"operation":"divide_by_grid_spherical_harmonics","buffer":3,"rows":[5,6,7],"minimum":0.1},
            "alpha":"sun_shadow_fallback","stage":"separate_indirect_lighting_terms"});
        if recipe.base_specular {
            surface["source_vertex_lighting"]["specular"] = json!({"operation":"divide_by_reflection_spherical_harmonics","buffer":3,"rows":[8,9,10],"minimum":0.1});
        }
        surface["pending_runtime"] = json!(
            "Native vertex-lit surface requires secondary RGB squared times32 divided by separate grid/reflection spherical-harmonic responses before interpolation, separate indirect lighting multipliers and secondary-alpha sun-shadow fallback; geometry and runtime consumers remain unconnected independently of material lighting amounts"
        );
    }
    println!(
        "Baked native special surface {}: {width}x{height}, source UV extent {extent:?}",
        native["name"]
    );
    Ok(value)
}
