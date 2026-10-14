use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Definition {
    programs: [&'static str; 4],
    flicker: bool,
    translucent: bool,
    mlv: bool,
    fritz: bool,
    multi: bool,
    tomb: bool,
    masked: bool,
}

pub(super) fn lookup(family: &str) -> Option<Definition> {
    Some(match family {
        "mc_sw4_3d_phong_emissive_fritz_76q5wqjw" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_phong_emissive_fritz_f19fb54c",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_phong_emissive_fritz_97e02993",
            ],
            fritz: true,
            masked: true,
            ..Default::default()
        },
        "mc_sw4_3d_phong_emissive_fritz_61728967" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_phong_emissive_fritz_c65881e3",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_phong_emissive_fritz_a0271d3c",
            ],
            fritz: true,
            ..Default::default()
        },
        "mc_sw4_3d_phong_emissive_script_multi_j05zz52w" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_phong_emissive_script_multi_edc347f5",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_phong_emissive_script_multi_bf233da0",
            ],
            multi: true,
            masked: true,
            ..Default::default()
        },
        "mc_sw4_3d_phong_emissive_script_multi_72q2fj2u" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_phong_emissive_script_multi_f502551a",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_phong_emissive_script_multi_a7e22f4f",
            ],
            multi: true,
            ..Default::default()
        },
        "mc_sw4_3d_emissive_tomb_qww536q2" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_emissive_tomb_88e8a7be",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_emissive_tomb_f126611f",
            ],
            tomb: true,
            ..Default::default()
        },
        "mc_sw4_3d_phong_simple_emissive_flicker_9j5f25q3" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_phong_simple_emissive_flicker_bb5863e8",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_phong_simple_emissive_flicker_cc44003b",
            ],
            flicker: true,
            ..Default::default()
        },
        "mc_sw4_3d_phong_emissive_alcatraz_8q551153" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_phong_emissive_alcatraz_d217360d",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_phong_emissive_alcatraz_498ceddd",
            ],
            translucent: true,
            ..Default::default()
        },
        "mlv_sw4_3d_phong_emissive_alcatraz_w1q1w09e" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_bb68573a",
                "sw4_3d_phong_emissive_alcatraz_4fa0bc6c",
                "sw4_3d_phong_emissive_alcatraz_b79ce21",
                "sw4_3d_phong_emissive_alcatraz_d43b67bc",
            ],
            mlv: true,
            ..Default::default()
        },
        "mc_sw4_3d_phong_emissive_q0751958" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_phong_emissive_alcatraz_431e7c81",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_phong_emissive_alcatraz_d885a751",
            ],
            ..Default::default()
        },
        "mc_sw4_3d_phong_emissive_alcatraz_90892wq5" => Definition {
            programs: [
                "sw4_3d_phong_emissive_alcatraz_b7d697d7",
                "sw4_3d_phong_emissive_alcatraz_431e7c81",
                "sw4_3d_phong_emissive_alcatraz_7c70ecc",
                "sw4_3d_phong_emissive_alcatraz_d885a751",
            ],
            ..Default::default()
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
        flicker,
        translucent,
        mlv,
        fritz,
        multi,
        tomb,
        masked,
    } = definition;
    let bindings = if fritz {
        vec![
            (2695565377_u32, 1281),
            (2485960688, 1536),
            (2351529600, 1026),
            (1825069840, 771),
            (1938096808, 516),
            (1346219310, 261),
        ]
    } else if flicker {
        vec![
            (3574481249, 1026),
            (2485960688, 769),
            (878789447, 515),
            (23340576, 256),
        ]
    } else {
        vec![
            (2695565377, 1025),
            (2485960688, 512),
            (2351529600, 770),
            (1825069840, 259),
        ]
    };
    let constants = if fritz {
        vec![
            (14836402_u32, 88, 4),
            (1020264075, 80, 4),
            (1418232170, 104, 4),
            (1418568053, 96, 8),
            (1599130352, 116, 4),
            (1732624492, 112, 4),
            (2044141368, 108, 4),
            (2832011523, 84, 4),
        ]
    } else if multi {
        vec![
            (14836402, 88, 4),
            (1020264075, 80, 4),
            (2158510423, 92, 4),
            (2832011523, 84, 4),
        ]
    } else if tomb {
        vec![
            (14836402, 88, 4),
            (604090542, 92, 4),
            (1020264075, 80, 4),
            (2832011523, 84, 4),
            (3662839941, 96, 4),
        ]
    } else if flicker {
        vec![
            (14836402, 80, 4),
            (23340584, 92, 4),
            (23340854, 88, 4),
            (1271696416, 96, 4),
            (3936631419, 84, 4),
        ]
    } else {
        vec![
            (14836402, 88, 4),
            (604090542, 92, 4),
            (1020264075, 80, 4),
            (2832011523, 84, 4),
            (3493630810, 96, 4),
            (3662839941, 100, 4),
        ]
    };
    let routes = [(0, 0), (1, 2), (2, 5), (3, 1), (4, 7), (5, 6)];
    for (offset, index) in [4_usize, 6].into_iter().enumerate() {
        let pass = crate::material::contract::pass(native, index)?;
        let state = native["state_entries"][index]
            .as_u64()
            .ok_or("Emissive selected state is missing")? as usize;
        if pass["vertex_shader"] != format!("pimp_shader_{}.hlsl", programs[offset * 2])
            || pass["pixel_shader"] != format!("pimp_shader_{}.hlsl", programs[offset * 2 + 1])
            || native["state_bits"][state]
                != if translucent {
                    json!([422084965_u32, 12])
                } else if masked {
                    json!([403869714_u32, 13])
                } else {
                    json!([403867666_u32, 13])
                }
        {
            return Err("Emissive selected programs or states differ from recovered source".into());
        }
        let actual = pass["vertex_routes"]
            .as_array()
            .ok_or("Emissive vertex routes are missing")?;
        let args = pass["arguments"]
            .as_array()
            .ok_or("Emissive arguments are missing")?;
        if actual.len() != if mlv { 6 } else { 5 }
            || actual
                .iter()
                .zip(routes)
                .any(|(route, (source, destination))| {
                    route["source"] != source || route["destination"] != destination
                })
            || args.iter().filter(|arg| arg["type"] == 2).count() != bindings.len()
            || bindings.iter().any(|&(hash, location)| {
                !args.iter().any(|arg| {
                    arg["type"] == 2
                        && arg["value"] == hash
                        && arg["location"] == location
                        && arg["buffer"] == 0
                        && arg["size"] == 1
                })
            })
            || args.iter().filter(|arg| arg["type"] == 6).count() != constants.len()
            || constants.iter().any(|&(hash, location, size)| {
                !args.iter().any(|arg| {
                    arg["type"] == 6
                        && arg["value"] == hash
                        && arg["location"] == location
                        && arg["buffer"] == 2
                        && arg["size"] == size
                })
            })
        {
            return Err(
                "Emissive active source bindings or routes differ from recovered source".into(),
            );
        }
        if flicker || fritz || multi || tomb {
            let mut inputs = vec![(
                if flicker {
                    170
                } else if tomb {
                    172
                } else {
                    169
                },
                1,
                if flicker {
                    384
                } else if tomb {
                    416
                } else {
                    368
                },
            )];
            if flicker || fritz {
                inputs.push((25, 2, 64));
            }
            for (code, buffer, location) in inputs {
                if !args.iter().any(|arg| {
                    arg["type"] == 5
                        && arg["value"] == 16777216_u32 + code
                        && arg["buffer"] == buffer
                        && arg["location"] == location
                        && arg["size"] == 16
                }) {
                    return Err("Emissive script/time bindings differ from recovered source".into());
                }
            }
        }
    }
    let literal = |hash| crate::material::contract::literal(native, hash);
    let constant = |hash| -> Result<f32> { Ok(literal(hash)?[0]) };
    for &(hash, _, _) in &constants {
        constant(hash)?;
    }
    let (parms, second_parms) = if fritz {
        let speed = literal(1418568053)?;
        (
            [
                constant(1020264075)?,
                constant(2832011523)?,
                constant(14836402)?,
                constant(1418232170)?,
            ],
            [
                speed[0],
                speed[1],
                constant(2044141368)?,
                constant(1732624492)?,
            ],
        )
    } else if multi {
        (
            [0., 0., 0., constant(14836402)?],
            [
                constant(1020264075)?,
                constant(2832011523)?,
                constant(2158510423)?,
                if masked { 1. } else { 0. },
            ],
        )
    } else if tomb {
        (
            [
                constant(604090542)?,
                constant(3662839941)?,
                0.,
                constant(14836402)?,
            ],
            [constant(1020264075)?, constant(2832011523)?, 0., 0.],
        )
    } else if flicker {
        (
            [
                constant(14836402)?,
                constant(3936631419)?,
                constant(23340854)?,
                constant(23340584)?,
            ],
            [constant(1271696416)?, 0., 0., 0.],
        )
    } else {
        (
            [
                constant(604090542)?,
                constant(3662839941)?,
                constant(3493630810)?,
                constant(14836402)?,
            ],
            [constant(1020264075)?, constant(2832011523)?, 0., 0.],
        )
    };
    let mut special_parms: [[f32; 4]; 3] = [[0.; 4]; 3];
    special_parms[0][0] = if fritz {
        constant(1599130352)?
    } else if translucent {
        1.
    } else {
        0.
    };
    if fritz {
        special_parms[0][1] = if masked { 1. } else { 0. };
    }
    if (fritz || multi || tomb) && images.is_some() {
        let mut inputs = vec![(1, if tomb { "172" } else { "169" })];
        if fritz {
            inputs.push((2, "25"));
        }
        for (slot, key) in inputs {
            let value = native["code_constants"][key].clone();
            if value.is_null() {
                return Err(format!("Emissive surface requires bound native code constant {key}; no initial state is invented").into());
            }
            special_parms[slot] = serde_json::from_value(value)?;
            if special_parms[slot].iter().any(|value| !value.is_finite()) {
                return Err("Emissive effective input is not finite".into());
            }
        }
    }
    let mut inputs = Vec::new();
    for &(hash, _) in &bindings {
        let binding = crate::material::contract::texture_binding(native, hash)?;
        let sampler = if fritz && hash == 1346219310 { 2 } else { 20 };
        if binding["sampler"] != sampler {
            return Err("Emissive source sampler differs from recovered source".into());
        }
        if let Some(images) = images {
            let name = binding["image"]
                .as_str()
                .ok_or("Emissive image name is missing")?
                .to_ascii_lowercase();
            inputs.push(Input::read(
                images
                    .get(&name)
                    .ok_or_else(|| format!("Emissive image {name} is missing"))?,
                sampler,
            )?);
        }
    }
    Ok(Recipe {
        special: if fritz {
            16
        } else if multi {
            32
        } else if tomb {
            64
        } else if flicker {
            8
        } else {
            4
        },
        special_parms,
        inputs,
        base_specular: true,
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
