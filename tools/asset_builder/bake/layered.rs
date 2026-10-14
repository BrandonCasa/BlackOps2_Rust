use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct Definition {
    programs: [&'static str; 2],
    sun: Option<[&'static str; 2]>,
    reviewed_bindings: bool,
    pub(super) output: Output,
    third_multiply: bool,
    layer_cutout: bool,
    second_reveal: bool,
    second_specular: bool,
    second_additive: bool,
    second_normal: u8,
    base_specular: bool,
    geometric_normal: bool,
    response_decal: bool,
    diffuse_reveal: bool,
    double_blend: bool,
    blend_color: bool,
    additive: bool,
    multiply: bool,
    base_alpha_reveal: bool,
    base_alpha_response: bool,
    reveal_decal: bool,
    decal: bool,
    layer_specular: bool,
    reveal: bool,
}

pub(super) fn lookup(family: &str) -> Option<Definition> {
    Some(match family {
        "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_94510929.hlsl",
                "pimp_shader_layer_lmap_60dd249f.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_1fefc3d1.hlsl",
                "pimp_shader_layer_lmap_12f22da9.hlsl",
            ]),
            reviewed_bindings: true,
            second_reveal: true,
            second_specular: true,
            second_normal: 2,
            base_alpha_response: true,
            decal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1_b2c2n2s2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_3250349a.hlsl",
                "pimp_shader_layer_lmap_a3337049.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_b9eefe62.hlsl",
                "pimp_shader_layer_lmap_d11c797f.hlsl",
            ]),
            reviewed_bindings: true,
            second_specular: true,
            second_normal: 1,
            blend_color: true,
            base_alpha_response: true,
            decal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3" => Definition {
            programs: [
                "pimp_shader_layer_lmap_aaeb8157.hlsl",
                "pimp_shader_layer_lmap_b81c762c.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_21554baf.hlsl",
                "pimp_shader_layer_lmap_ca337f1a.hlsl",
            ]),
            reviewed_bindings: true,
            third_multiply: true,
            second_additive: true,
            base_alpha_reveal: true,
            base_alpha_response: true,
            decal: true,
            layer_specular: true,
            reveal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_t1c1n1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_c9b52d6f.hlsl",
                "pimp_shader_layer_lmap_5d72fcf0.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_420be797.hlsl",
                "pimp_shader_layer_lmap_2f5df5c6.hlsl",
            ]),
            reviewed_bindings: true,
            layer_cutout: true,
            base_alpha_response: true,
            output: Output {
                base_alpha_response: true,
                layer_cutout: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_94510929.hlsl",
                "pimp_shader_layer_lmap_d29e97e0.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_1fefc3d1.hlsl",
                "pimp_shader_layer_lmap_a0b19ed6.hlsl",
            ]),
            reviewed_bindings: true,
            second_reveal: true,
            second_specular: true,
            second_normal: 2,
            base_alpha_response: true,
            decal: true,
            layer_specular: true,
            reveal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_94510929.hlsl",
                "pimp_shader_layer_lmap_cf9388f8.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_1fefc3d1.hlsl",
                "pimp_shader_layer_lmap_bdbc81ce.hlsl",
            ]),
            reviewed_bindings: true,
            second_reveal: true,
            second_specular: true,
            second_normal: 2,
            base_specular: true,
            base_alpha_response: true,
            decal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_94510929.hlsl",
                "pimp_shader_layer_lmap_7dd03b87.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_1fefc3d1.hlsl",
                "pimp_shader_layer_lmap_fff32b1.hlsl",
            ]),
            reviewed_bindings: true,
            second_reveal: true,
            second_specular: true,
            second_normal: 2,
            base_specular: true,
            base_alpha_response: true,
            decal: true,
            layer_specular: true,
            reveal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_3250349a.hlsl",
                "pimp_shader_layer_lmap_c6dc192c.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_b9eefe62.hlsl",
                "pimp_shader_layer_lmap_b4f3101a.hlsl",
            ]),
            reviewed_bindings: true,
            second_reveal: true,
            second_specular: true,
            second_normal: 1,
            blend_color: true,
            base_alpha_response: true,
            decal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_94510929.hlsl",
                "pimp_shader_layer_lmap_76b69a5.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_1fefc3d1.hlsl",
                "pimp_shader_layer_lmap_75446093.hlsl",
            ]),
            reviewed_bindings: true,
            second_reveal: true,
            second_normal: 2,
            base_alpha_response: true,
            decal: true,
            layer_specular: true,
            reveal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_a1c1_m2c2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_8b6985a0.hlsl",
                "pimp_shader_layer_lmap_ad25a73a.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_d74f58.hlsl",
                "pimp_shader_layer_lmap_df0aae0c.hlsl",
            ]),
            reviewed_bindings: true,
            additive: true,
            base_alpha_response: true,
            decal: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1_a2c2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_3250349a.hlsl",
                "pimp_shader_layer_lmap_f5127bac.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_b9eefe62.hlsl",
                "pimp_shader_layer_lmap_873d729a.hlsl",
            ]),
            reviewed_bindings: true,
            second_additive: true,
            base_alpha_response: true,
            decal: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_3250349a.hlsl",
                "pimp_shader_layer_lmap_4751c8d3.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_b9eefe62.hlsl",
                "pimp_shader_layer_lmap_357ec1e5.hlsl",
            ]),
            reviewed_bindings: true,
            second_additive: true,
            response_decal: true,
            base_alpha_reveal: true,
            base_alpha_response: true,
            decal: true,
            layer_specular: true,
            reveal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1_b2c2n2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_3250349a.hlsl",
                "pimp_shader_layer_lmap_76c68e0c.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_b9eefe62.hlsl",
                "pimp_shader_layer_lmap_4e9873a.hlsl",
            ]),
            reviewed_bindings: true,
            second_normal: 1,
            blend_color: true,
            base_alpha_response: true,
            decal: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1_b2c2n2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_94510929.hlsl",
                "pimp_shader_layer_lmap_d0c7b3bf.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_1fefc3d1.hlsl",
                "pimp_shader_layer_lmap_a2e8ba89.hlsl",
            ]),
            reviewed_bindings: true,
            second_normal: 2,
            base_alpha_response: true,
            decal: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0s0_b1c1n1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_c9b52d6f.hlsl",
                "pimp_shader_layer_lmap_6278286e.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_420be797.hlsl",
                "pimp_shader_layer_lmap_10572158.hlsl",
            ]),
            reviewed_bindings: true,
            base_specular: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0s0_b1c1n1s1v1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_c9b52d6f.hlsl",
                "pimp_shader_layer_lmap_d03b9b11.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_420be797.hlsl",
                "pimp_shader_layer_lmap_a2149227.hlsl",
            ]),
            reviewed_bindings: true,
            base_specular: true,
            layer_specular: true,
            reveal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1s1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_c9b52d6f.hlsl",
                "pimp_shader_layer_lmap_aa3a8509.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_420be797.hlsl",
                "pimp_shader_layer_lmap_d8158c3f.hlsl",
            ]),
            reviewed_bindings: true,
            base_alpha_response: true,
            layer_specular: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_3250349a.hlsl",
                "pimp_shader_layer_lmap_675e1822.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_b9eefe62.hlsl",
                "pimp_shader_layer_lmap_15711114.hlsl",
            ]),
            reviewed_bindings: true,
            response_decal: true,
            base_alpha_reveal: true,
            base_alpha_response: true,
            decal: true,
            layer_specular: true,
            reveal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0s0_b1c1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_f55271f1.hlsl",
                "pimp_shader_layer_lmap_5e9f74f0.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_7eecbb09.hlsl",
                "pimp_shader_layer_lmap_2cb07dc6.hlsl",
            ]),
            reviewed_bindings: true,
            base_specular: true,
            geometric_normal: true,
            blend_color: true,
            base_alpha_response: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1v1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_c9b52d6f.hlsl",
                "pimp_shader_layer_lmap_18793676.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_420be797.hlsl",
                "pimp_shader_layer_lmap_6a563f40.hlsl",
            ]),
            diffuse_reveal: true,
            base_alpha_response: true,
            reveal: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1_b2c2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_8b6985a0.hlsl",
                "pimp_shader_layer_lmap_cfff3f36.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_d74f58.hlsl",
                "pimp_shader_layer_lmap_bdd03600.hlsl",
            ]),
            double_blend: true,
            blend_color: true,
            base_alpha_response: true,
            decal: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_9bf34854.hlsl",
                "pimp_shader_layer_lmap_9f70e132.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_104d82ac.hlsl",
                "pimp_shader_layer_lmap_ed5fe804.hlsl",
            ]),
            blend_color: true,
            base_alpha_response: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_a1c1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_9bf34854.hlsl",
                "pimp_shader_layer_lmap_b50e886e.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_104d82ac.hlsl",
                "pimp_shader_layer_lmap_c7218158.hlsl",
            ]),
            additive: true,
            base_alpha_response: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_m1c1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_9bf34854.hlsl",
                "pimp_shader_layer_lmap_254af097.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_104d82ac.hlsl",
                "pimp_shader_layer_lmap_5765f9a1.hlsl",
            ]),
            multiply: true,
            base_alpha_response: true,
            output: Output {
                base_alpha_response: true,
                multiply_gloss: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0_m1c1_m2c2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_e2f44d32.hlsl",
                "pimp_shader_layer_lmap_61a9e8fe.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_694a87ca.hlsl",
                "pimp_shader_layer_lmap_1386e1c8.hlsl",
            ]),
            multiply: true,
            decal: true,
            ..Default::default()
        },
        "lit_sm_r0c0n0_m1c1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_f26e80c6.hlsl",
                "pimp_shader_layer_lmap_7982c7aa.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_79d04a3e.hlsl",
                "pimp_shader_layer_lmap_badce9c.hlsl",
            ]),
            multiply: true,
            ..Default::default()
        },
        "lit_sm_r0c0n0_b1c1n1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_a028e5fd.hlsl",
                "pimp_shader_layer_lmap_91feb334.hlsl",
            ],
            ..Default::default()
        },
        "lit_sm_r0c0n0_b1c1n1_m2c2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_b0b22809.hlsl",
                "pimp_shader_layer_lmap_89d59c60.hlsl",
            ],
            decal: true,
            ..Default::default()
        },
        "lit_sm_r0c0n0_b1c1n1s1v1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_c9b52d6f.hlsl",
                "pimp_shader_layer_lmap_4a20c8d9.hlsl",
            ],
            layer_specular: true,
            reveal: true,
            output: Output {
                layer_specular: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0_b1c1n1v1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_a028e5fd.hlsl",
                "pimp_shader_layer_lmap_44b1014b.hlsl",
            ],
            reveal: true,
            ..Default::default()
        },
        "lit_sm_r0c0n0_b1c1n1v1_m2c2" => Definition {
            programs: [
                "pimp_shader_layer_lmap_b0b22809.hlsl",
                "pimp_shader_layer_lmap_5c9a2e1f.hlsl",
            ],
            reveal_decal: true,
            decal: true,
            reveal: true,
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_c9b52d6f.hlsl",
                "pimp_shader_layer_lmap_cd368409.hlsl",
            ],
            base_alpha_response: true,
            output: Output {
                base_alpha_response: true,
                ..Default::default()
            },
            ..Default::default()
        },
        "lit_sm_r0c0n0x0_b1c1n1s1v1" => Definition {
            programs: [
                "pimp_shader_layer_lmap_c9b52d6f.hlsl",
                "pimp_shader_layer_lmap_7f753776.hlsl",
            ],
            sun: Some([
                "pimp_shader_layer_lmap_420be797.hlsl",
                "pimp_shader_layer_lmap_d5a3e40.hlsl",
            ]),
            base_alpha_reveal: true,
            base_alpha_response: true,
            layer_specular: true,
            reveal: true,
            output: Output {
                base_alpha_reveal: true,
                layer_specular: true,
                ..Default::default()
            },
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
        sun,
        reviewed_bindings,
        third_multiply,
        layer_cutout,
        second_reveal,
        second_specular,
        second_additive,
        second_normal,
        base_specular,
        geometric_normal,
        response_decal,
        diffuse_reveal,
        double_blend,
        blend_color,
        additive,
        multiply,
        base_alpha_reveal,
        base_alpha_response,
        reveal_decal,
        decal,
        layer_specular,
        reveal,
        ..
    } = definition;
    let [vertex, pixel] = programs;
    let pass = crate::material::contract::pass(native, 4)?;
    if pass["pixel_shader"] != pixel || pass["vertex_shader"] != vertex {
        return Err("Layered material selected programs differ from recovered source".into());
    }
    let state = native["state_entries"][4]
        .as_u64()
        .ok_or("Layered lit state is missing")? as usize;
    if native["state_bits"][state] != json!([403867666_u32, 13]) {
        return Err("Layered material has unreviewed render states".into());
    }
    if let Some([vertex, pixel]) = sun {
        let pass = crate::material::contract::pass(native, 6)?;
        let state = native["state_entries"][6]
            .as_u64()
            .ok_or("Layered sun state is missing")? as usize;
        if pass["vertex_shader"] != vertex
            || pass["pixel_shader"] != pixel
            || native["state_bits"][state] != json!([403867666_u32, 13])
        {
            return Err(
                "Layered sun programs or render states differ from recovered source".into(),
            );
        }
    }
    let reveal_parms = if reveal {
        if !pass["arguments"].as_array().unwrap().iter().any(|arg| {
            arg["type"] == 6
                && arg["value"] == 2294217777_u32
                && arg["buffer"] == 1
                && arg["location"] == 944
                && arg["size"] == 16
        }) {
            return Err(
                "Layered reveal parameter shader binding differs from recovered source".into(),
            );
        }
        let constant = native["constants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["hash"] == 2294217777_u32)
            .ok_or("Layered reveal parameters are missing")?;
        let values: [f32; 4] = serde_json::from_value(constant["literal"].clone())?;
        if !values.iter().all(|value| value.is_finite())
            || values[0] <= 0.
            || values[1] < 0.
            || (!second_reveal && !base_alpha_reveal && !diffuse_reveal && values[1] == 0.)
        {
            return Err("Layered reveal parameters are outside the recovered domain".into());
        }
        values
    } else {
        [0.; 4]
    };
    let second_parms = if second_reveal {
        let constant = native["constants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["hash"] == 2294217778_u32)
            .ok_or("Second-layer reveal parameters are missing")?;
        let values: [f32; 4] = serde_json::from_value(constant["literal"].clone())?;
        if !values.iter().all(|value| value.is_finite()) || values[0] <= 0. || values[1] < 0. {
            return Err("Second-layer reveal parameters are outside the recovered domain".into());
        }
        values
    } else {
        [0.; 4]
    };
    let expected = [
        (0, 0),
        (1, 2),
        (2, 5),
        (3, 1),
        (4, 7),
        (5, 6),
        (6, 8),
        (8, 10),
        (9, 11),
    ];
    let geometric_routes = [(0, 0), (1, 2), (2, 5), (3, 1), (5, 6), (6, 8)];
    let third_routes = [
        (0, 0),
        (1, 2),
        (2, 5),
        (3, 1),
        (4, 7),
        (5, 6),
        (6, 8),
        (7, 9),
        (8, 10),
    ];
    let expected = if third_multiply {
        &third_routes[..]
    } else if geometric_normal {
        &geometric_routes[..]
    } else if second_normal == 2 {
        &expected[..]
    } else if (multiply || additive || blend_color) && second_normal == 0 {
        &expected[..7]
    } else {
        &expected[..8]
    };
    let routes = pass["vertex_routes"]
        .as_array()
        .ok_or("Layered vertex routes are missing")?;
    if routes.len() != expected.len()
        || routes
            .iter()
            .zip(expected.iter().copied())
            .any(|(route, (source, destination))| {
                route["source"] != source || route["destination"] != destination
            })
    {
        return Err("Layered material vertex declaration differs from recovered source".into());
    }
    let mut inputs = Vec::new();
    let mut bindings = if geometric_normal {
        vec![(2695565377_u32, 20, 0), (3054311504, 20, 258)]
    } else if base_specular {
        vec![
            (2695565377, 20, 0),
            (1507003663, 20, 257),
            (3054311504, 20, 515),
            (2486480606, 20, 772),
        ]
    } else {
        vec![
            (2695565377_u32, 20, 0),
            (1507003663, 20, 257),
            (3054311504, if multiply { 10 } else { 20 }, 514),
        ]
    };
    if !base_specular && !multiply && !additive && !blend_color {
        bindings.push((2486480606, 20, if decal { 1027 } else { 771 }));
    }
    if decal {
        bindings.push((
            3054311507,
            if double_blend || second_additive || second_normal != 0 {
                20
            } else {
                10
            },
            if multiply || additive || blend_color {
                771
            } else if response_decal {
                773
            } else {
                772
            },
        ));
    }
    if second_normal != 0 {
        bindings.push((2486480605, 20, if second_normal == 1 { 1028 } else { 1285 }));
    }
    if layer_specular {
        bindings.push((
            3532022562,
            20,
            if base_specular {
                1285
            } else if response_decal {
                1284
            } else {
                1028
            },
        ));
    }
    if base_specular {
        bindings.push((887934131, 20, if geometric_normal { 513 } else { 1026 }));
    }
    if third_multiply {
        bindings = vec![
            (2695565377, 20, 0),
            (1507003663, 20, 257),
            (3054311504, 20, 514),
            (2486480606, 20, 1283),
            (3054311507, 20, 773),
            (3532022562, 20, 1540),
            (3054311506, 74, 1030),
        ];
    } else if second_reveal || second_specular {
        bindings = if second_normal == 1 {
            vec![
                (2695565377, 20, 0),
                (1507003663, 20, 257),
                (3054311504, 20, 514),
                (3054311507, 20, 771),
                (2486480605, 20, 1028),
                (3532022561, 20, 1285),
            ]
        } else if base_specular {
            let mut bindings = vec![
                (2695565377, 20, 0),
                (1507003663, 20, 257),
                (3054311504, 20, 515),
                (2486480606, 20, 1028),
                (3054311507, 20, if layer_specular { 774 } else { 773 }),
                (2486480605, 20, if layer_specular { 1287 } else { 1286 }),
            ];
            if layer_specular {
                bindings.push((3532022562, 20, 1797));
            }
            bindings.push((3532022561, 20, if layer_specular { 2056 } else { 1799 }));
            bindings.push((887934131, 20, 1538));
            bindings
        } else if !layer_specular {
            vec![
                (2695565377, 20, 0),
                (1507003663, 20, 257),
                (3054311504, 20, 514),
                (2486480606, 20, 1027),
                (3054311507, 20, 772),
                (2486480605, 20, 1285),
                (3532022561, 20, 1542),
            ]
        } else {
            let mut bindings = vec![
                (2695565377, 20, 0),
                (1507003663, 20, 257),
                (3054311504, 20, 514),
                (2486480606, 20, 1027),
                (3054311507, 20, 773),
                (2486480605, 20, 1286),
                (3532022562, 20, 1540),
            ];
            if second_specular {
                bindings.push((3532022561, 20, 1799));
            }
            bindings
        };
    }
    if reviewed_bindings {
        for index in [4, 6] {
            let pass = crate::material::contract::pass(native, index)?;
            let routes = pass["vertex_routes"]
                .as_array()
                .ok_or("Layered vertex routes are missing")?;
            let args = pass["arguments"]
                .as_array()
                .ok_or("Layered shader arguments are missing")?;
            if routes.len() != expected.len()
                || routes
                    .iter()
                    .zip(expected)
                    .any(|(route, &(source, destination))| {
                        route["source"] != source || route["destination"] != destination
                    })
                || args.iter().filter(|arg| arg["type"] == 2).count() != bindings.len()
                || bindings.iter().any(|&(hash, _, slot)| {
                    !args.iter().any(|arg| {
                        arg["type"] == 2 && arg["value"] == hash && arg["location"] == slot
                    })
                })
                || args.iter().filter(|arg| arg["type"] == 6).count()
                    != usize::from(reveal) + usize::from(second_reveal)
                || reveal
                    && !args.iter().any(|arg| {
                        arg["type"] == 6
                            && arg["value"] == 2294217777_u32
                            && arg["buffer"] == 1
                            && arg["location"] == 944
                            && arg["size"] == 16
                    })
                || second_reveal
                    && !args.iter().any(|arg| {
                        arg["type"] == 6
                            && arg["value"] == 2294217778_u32
                            && arg["buffer"] == 1
                            && arg["location"] == 960
                            && arg["size"] == 16
                    })
            {
                return Err("Layered response inputs differ from recovered source".into());
            }
        }
    }
    for (hash, expected_sampler, location) in bindings {
        let binding = native["textures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|binding| binding["hash"] == hash)
            .ok_or("Layered source texture binding is missing")?;
        let sampler = binding["sampler"]
            .as_u64()
            .and_then(|value| u8::try_from(value).ok())
            .ok_or("Layered sampler is not a byte")?;
        if (sampler != expected_sampler
            && !(((reveal_decal || response_decal) && hash == 3054311507
                || multiply && hash == 3054311504)
                && sampler == 74))
            || !pass["arguments"]
                .as_array()
                .unwrap()
                .iter()
                .any(|argument| {
                    argument["type"] == 2
                        && argument["value"] == hash
                        && argument["location"] == location
                })
        {
            return Err("Layered texture has an unreviewed sampler or shader slot".into());
        }
        let name = binding["image"]
            .as_str()
            .ok_or("Layered source image name is missing")?;
        if let Some(images) = images {
            let image = images
                .get(&name.to_ascii_lowercase())
                .ok_or("Layered source image is missing")?;
            println!("Layered input {name}: {}x{}", image.width, image.height);
            inputs.push(Input::read(image, sampler)?);
        }
    }
    Ok(Recipe {
        alpha_reveal: false,
        special: 0,
        special_parms: [[0.; 4]; 3],
        third_multiply,
        layer_cutout,
        second_reveal,
        second_specular,
        second_parms,
        second_additive,
        second_normal,
        base_specular,
        geometric_normal,
        inputs,
        decal,
        multiply,
        additive,
        blend_color,
        reveal,
        layer_specular,
        base_alpha_response,
        parms: reveal_parms,
    })
}
