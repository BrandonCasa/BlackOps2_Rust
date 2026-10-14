use super::*;

#[derive(Clone, Copy)]
pub(super) struct Definition {
    programs: [&'static str; 4],
    water: bool,
    pub(super) specular: bool,
    vertex_lighting: bool,
}

pub(super) fn lookup(family: &str) -> Option<Definition> {
    Some(match family {
        "mc_sw4_3d_cod7_tile_blend_q2q2zej6" => Definition {
            programs: [
                "sw4_3d_cod7_tile_blend_3e199cfe",
                "sw4_3d_cod7_tile_blend_e88a8c75",
                "sw4_3d_cod7_tile_blend_ff4d7872",
                "sw4_3d_cod7_tile_blend_2d911f96",
            ],
            water: false,
            specular: false,
            vertex_lighting: false,
        },
        "mlv_sw4_3d_cod7_tile_blend_26zq1w22" => Definition {
            programs: [
                "sw4_3d_cod7_tile_blend_e11f113c",
                "sw4_3d_cod7_tile_blend_378c01b7",
                "sw4_3d_cod7_tile_blend_204bf5b0",
                "sw4_3d_cod7_tile_blend_f2979254",
            ],
            water: false,
            specular: false,
            vertex_lighting: true,
        },
        "mc_sw4_3d_cod7_tile_blend_spec_8w40z341" => Definition {
            programs: [
                "sw4_3d_cod7_tile_blend_spec_6bd58c9b",
                "sw4_3d_cod7_tile_blend_spec_d5cd3eb7",
                "sw4_3d_cod7_tile_blend_spec_ff8a6704",
                "sw4_3d_cod7_tile_blend_spec_b3b2a268",
            ],
            water: false,
            specular: true,
            vertex_lighting: false,
        },
        "mlv_sw4_3d_cod7_tile_blend_spec_w8wezqe6" => Definition {
            programs: [
                "sw4_3d_cod7_tile_blend_spec_e51efca4",
                "sw4_3d_cod7_tile_blend_spec_5b064e88",
                "sw4_3d_cod7_tile_blend_spec_7141173b",
                "sw4_3d_cod7_tile_blend_spec_3d79d257",
            ],
            water: false,
            specular: true,
            vertex_lighting: true,
        },
        "mc_sw4_3d_water_on_surface_reveal_77320z68" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_water_on_surface_reveal_f3ca08bb",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_water_on_surface_reveal_6851d36b",
            ],
            water: true,
            specular: true,
            vertex_lighting: false,
        },
        _ => return None,
    })
}

pub(super) fn read(
    definition: Definition,
    native: &Value,
    images: Option<&BTreeMap<String, texture::Image>>,
) -> Result<Recipe> {
    let Definition {
        programs,
        water,
        specular,
        vertex_lighting,
    } = definition;
    let bindings = if water {
        vec![
            (2782436735_u32, 256),
            (1507003663, 513),
            (3827006327, 770),
            (1837877086, 1027),
            (2695565377, 1284),
            (2351529600, 1541),
        ]
    } else if specular {
        vec![
            (3976875907, 256),
            (329745474, 1025),
            (3400726183, 515),
            (3069501348, 770),
            (2643103145, 1542),
            (3580385162, 2052),
            (2415611304, 1287),
            (830013099, 1797),
        ]
    } else {
        vec![
            (3976875907, 256),
            (329745474, 1025),
            (3400726183, 515),
            (3069501348, 770),
            (2643103145, 1285),
            (3580385162, 1540),
        ]
    };
    let constants = if water {
        vec![
            (111253676_u32, 124, 4),
            (348546551, 96, 12),
            (1116993630, 128, 4),
            (1288488438, 112, 12),
            (3070667917, 132, 8),
            (3071481490, 80, 4),
            (3228582790, 84, 4),
        ]
    } else {
        let mut values = vec![
            (523422290, 92, 4),
            (2569221807, 96, 4),
            (4005921460, 88, 4),
            (4266698135, 100, 4),
        ];
        if specular {
            values.push((211931156, 112, 4));
        }
        values
    };
    let expected_routes = [(0, 0), (1, 2), (2, 5), (3, 1), (4, 7), (5, 6)];
    let route_count = if vertex_lighting { 6 } else { 5 };
    for (offset, index) in [4_usize, 6].into_iter().enumerate() {
        let pass = crate::material::contract::pass(native, index)?;
        let state = native["state_entries"][index]
            .as_u64()
            .ok_or("Special surface state is missing")? as usize;
        if pass["vertex_shader"] != format!("pimp_shader_{}.hlsl", programs[offset * 2])
            || pass["pixel_shader"] != format!("pimp_shader_{}.hlsl", programs[offset * 2 + 1])
            || native["state_bits"][state] != json!([403867666_u32, 13])
        {
            return Err(
                "Special surface selected programs or states differ from recovered source".into(),
            );
        }
        let routes = pass["vertex_routes"]
            .as_array()
            .ok_or("Special surface routes are missing")?;
        let args = pass["arguments"]
            .as_array()
            .ok_or("Special surface arguments are missing")?;
        if routes.len() != route_count
            || routes
                .iter()
                .zip(expected_routes)
                .any(|(route, (source, destination))| {
                    route["source"] != source || route["destination"] != destination
                })
            || args.iter().filter(|arg| arg["type"] == 2).count() != bindings.len()
            || bindings.iter().any(|&(hash, slot)| {
                !args.iter().any(|arg| {
                    arg["type"] == 2
                        && arg["value"] == hash
                        && arg["location"] == slot
                        && arg["buffer"] == 0
                        && arg["size"] == 1
                })
            })
            || args.iter().filter(|arg| arg["type"] == 6).count() != constants.len()
            || constants.iter().any(|&(hash, location, size)| {
                !args.iter().any(|arg| {
                    arg["type"] == 6
                        && arg["value"] == hash
                        && arg["buffer"] == 2
                        && arg["location"] == location
                        && arg["size"] == size
                })
            })
        {
            return Err(
                "Special surface active bindings or vertex routes differ from recovered source"
                    .into(),
            );
        }
        if water {
            for (code, buffer, location) in [(172, 1, 416), (25, 2, 64)] {
                if !args.iter().any(|arg| {
                    arg["type"] == 5
                        && arg["value"] == 16777216_u32 + code
                        && arg["buffer"] == buffer
                        && arg["location"] == location
                        && arg["size"] == 16
                }) {
                    return Err(
                        "Water reveal script/time binding differs from recovered source".into(),
                    );
                }
            }
        } else if args.iter().filter(|arg| arg["type"] == 0).count() != 2
            || [(404288371_u32, 80), (2106384112, 104)]
                .into_iter()
                .any(|(hash, location)| {
                    !args.iter().any(|arg| {
                        arg["type"] == 0
                            && arg["value"] == hash
                            && arg["buffer"] == 2
                            && arg["location"] == location
                            && arg["size"] == 8
                    })
                })
        {
            return Err(
                "Tile surface micro-coordinate scale binding differs from recovered source".into(),
            );
        }
    }
    let constant = |hash| crate::material::contract::literal(native, hash);
    for &(hash, _, _) in &constants {
        constant(hash)?;
    }
    let (parms, second_parms, mut special_parms) = if water {
        let scale = constant(3070667917)?;
        if scale[0] == 0. || scale[1] == 0. {
            return Err("Water reveal scale has a zero divisor".into());
        }
        let edge = constant(348546551)?;
        let alteration = constant(1288488438)?;
        (
            [
                constant(3071481490)?[0],
                constant(1116993630)?[0],
                scale[0],
                scale[1],
            ],
            [edge[0], edge[1], edge[2], constant(3228582790)?[0]],
            [
                [
                    alteration[0],
                    alteration[1],
                    alteration[2],
                    constant(111253676)?[0],
                ],
                [0.; 4],
                [0.; 4],
            ],
        )
    } else {
        let first = constant(404288371)?;
        let second = constant(2106384112)?;
        (
            [first[0], first[1], second[0], second[1]],
            [
                constant(4005921460)?[0],
                constant(523422290)?[0],
                constant(2569221807)?[0],
                constant(4266698135)?[0],
            ],
            [
                [
                    if specular {
                        constant(211931156)?[0]
                    } else {
                        0.
                    },
                    0.,
                    0.,
                    0.,
                ],
                [0.; 4],
                [0.; 4],
            ],
        )
    };
    if water && images.is_some() {
        for (slot, code) in [(1, 172), (2, 25)] {
            let value = native["code_constants"][code.to_string()].clone();
            if value.is_null() {
                return Err(format!("Water reveal requires bound native code constant {code}; no initial state is invented").into());
            }
            special_parms[slot] = serde_json::from_value(value)?;
            if special_parms[slot].iter().any(|value| !value.is_finite()) {
                return Err("Water reveal effective input is not finite".into());
            }
        }
    }
    let mut inputs = Vec::new();
    for &(hash, _) in &bindings {
        let binding = crate::material::contract::texture_binding(native, hash)?;
        let sampler = if water && hash == 3827006327 { 10 } else { 20 };
        if binding["sampler"] != sampler {
            return Err("Special surface source sampler differs from recovered source".into());
        }
        if let Some(images) = images {
            let name = binding["image"]
                .as_str()
                .ok_or("Special surface image name is missing")?
                .to_ascii_lowercase();
            inputs.push(Input::read(
                images
                    .get(&name)
                    .ok_or_else(|| format!("Special surface image {name} is missing"))?,
                sampler,
            )?);
        }
    }
    Ok(Recipe {
        special: if water { 2 } else { 1 },
        special_parms,
        inputs,
        base_specular: specular,
        geometric_normal: false,
        second_additive: false,
        second_normal: 0,
        second_reveal: false,
        second_specular: false,
        second_parms,
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
